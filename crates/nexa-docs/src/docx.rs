use crate::{
    AbstractList, Alignment, Block, CellProperties, CompatibilityReport, Document, HeaderFooter,
    InlineImage, ListFormat, ListLevel, ListReference, Numbering, NumberingInstance, Orientation,
    PageMargins, Paragraph, ParagraphProperties, RgbColor, Run, RunContent, RunProperties, Section,
    SectionProperties, Style, StyleKind, StyleSheet, Table, TableCell, TableProperties, TableRow,
};
use nexa_ooxml::{
    ContentTypeMap, ContentTypeRule, LazyZipPackage, OfficePackageKind, Package, PackageError,
    PartName, Relationship, RelationshipId, RelationshipSet, RelationshipTarget, ZipPackageError,
    write_owned_package,
};
use quick_xml::{
    XmlVersion,
    events::{BytesStart, Event},
    reader::Reader,
};
use std::{
    collections::BTreeMap,
    error::Error,
    fmt,
    io::{Read, Seek, Write},
};

const DOCUMENT_PART: &str = "/word/document.xml";
const STYLES_PART: &str = "/word/styles.xml";
const NUMBERING_PART: &str = "/word/numbering.xml";

const OFFICE_DOCUMENT_REL: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";
const STYLES_REL: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles";
const NUMBERING_REL: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering";

const DOCUMENT_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml";
const STYLES_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml";
const NUMBERING_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml";

#[derive(Debug, Clone)]
pub struct DocxDocument {
    document: Document,
    package: Package,
    main_part: PartName,
}

impl DocxDocument {
    #[must_use]
    pub fn blank() -> Self {
        let main_part = PartName::new(DOCUMENT_PART).expect("constant part name");
        let styles_part = PartName::new(STYLES_PART).expect("constant part name");
        let numbering_part = PartName::new(NUMBERING_PART).expect("constant part name");

        let mut content_types = ContentTypeMap::default();
        content_types.insert(ContentTypeRule::Default {
            extension: "xml".into(),
            content_type: "application/xml".into(),
        });
        content_types.insert(ContentTypeRule::Default {
            extension: "rels".into(),
            content_type: "application/vnd.openxmlformats-package.relationships+xml".into(),
        });
        content_types.insert(ContentTypeRule::Override {
            part_name: main_part.clone(),
            content_type: DOCUMENT_CONTENT_TYPE.into(),
        });
        content_types.insert(ContentTypeRule::Override {
            part_name: styles_part.clone(),
            content_type: STYLES_CONTENT_TYPE.into(),
        });
        content_types.insert(ContentTypeRule::Override {
            part_name: numbering_part.clone(),
            content_type: NUMBERING_CONTENT_TYPE.into(),
        });

        let mut package = Package::new(content_types);
        package
            .insert_part(main_part.clone(), Vec::new())
            .expect("blank DOCX content type exists");
        package
            .insert_part(styles_part.clone(), Vec::new())
            .expect("blank DOCX styles content type exists");
        package
            .insert_part(numbering_part.clone(), Vec::new())
            .expect("blank DOCX numbering content type exists");

        package
            .package_relationships_mut()
            .insert(Relationship {
                id: RelationshipId::new("rId1"),
                relationship_type: OFFICE_DOCUMENT_REL.into(),
                target: RelationshipTarget::Internal(main_part.clone()),
            })
            .expect("unique root relationship");

        let document_relationships = package.relationships_mut(main_part.clone());
        document_relationships
            .insert(Relationship {
                id: RelationshipId::new("rIdStyles"),
                relationship_type: STYLES_REL.into(),
                target: RelationshipTarget::Internal(styles_part),
            })
            .expect("unique styles relationship");
        document_relationships
            .insert(Relationship {
                id: RelationshipId::new("rIdNumbering"),
                relationship_type: NUMBERING_REL.into(),
                target: RelationshipTarget::Internal(numbering_part),
            })
            .expect("unique numbering relationship");

        let mut result = Self {
            document: Document::blank(),
            package,
            main_part,
        };
        result.sync_package().expect("blank DOCX serialization");
        result
    }

    #[must_use]
    pub fn document(&self) -> &Document {
        &self.document
    }

    pub fn document_mut(&mut self) -> &mut Document {
        &mut self.document
    }

    #[must_use]
    pub fn package(&self) -> &Package {
        &self.package
    }

    pub fn into_document(self) -> Document {
        self.document
    }

    fn sync_package(&mut self) -> Result<(), DocxError> {
        if !self.document.compatibility.can_save() {
            return Err(DocxError::SaveBlocked(
                self.document.compatibility.issues.len(),
            ));
        }

        self.package
            .replace_part(&self.main_part, write_document_xml(&self.document))?;

        let styles_part = PartName::new(STYLES_PART)?;
        if self.package.part(&styles_part).is_some() {
            self.package
                .replace_part(&styles_part, write_styles_xml(&self.document.styles))?;
        }

        let numbering_part = PartName::new(NUMBERING_PART)?;
        if self.package.part(&numbering_part).is_some() {
            self.package
                .replace_part(&numbering_part, write_numbering_xml(&self.document.numbering))?;
        }

        let relationships = self
            .package
            .relationships(&self.main_part)
            .cloned()
            .unwrap_or_default();

        for relationship in relationships.iter() {
            let RelationshipTarget::Internal(target) = &relationship.target else {
                continue;
            };

            if relationship.relationship_type.ends_with("/header") {
                if let Some(header) = self.document.headers.get(relationship.id.as_str()) {
                    self.package
                        .replace_part(target, write_header_footer_xml("hdr", header))?;
                }
            } else if relationship.relationship_type.ends_with("/footer")
                && let Some(footer) = self.document.footers.get(relationship.id.as_str())
            {
                self.package
                    .replace_part(target, write_header_footer_xml("ftr", footer))?;
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum DocxError {
    Zip(ZipPackageError),
    Package(PackageError),
    PartName(nexa_ooxml::PartNameError),
    WrongOfficeKind,
    MissingMainPart,
    Xml(String),
    SaveBlocked(usize),
}

impl fmt::Display for DocxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zip(error) => write!(f, "{error}"),
            Self::Package(error) => write!(f, "{error}"),
            Self::PartName(error) => write!(f, "{error}"),
            Self::WrongOfficeKind => f.write_str("package is not a DOCX Word document"),
            Self::MissingMainPart => f.write_str("DOCX main document part is missing"),
            Self::Xml(message) => write!(f, "WordprocessingML error: {message}"),
            Self::SaveBlocked(count) => write!(
                f,
                "saving is blocked because {count} unsupported construct(s) could be lost"
            ),
        }
    }
}

impl Error for DocxError {}

impl From<ZipPackageError> for DocxError {
    fn from(value: ZipPackageError) -> Self {
        Self::Zip(value)
    }
}

impl From<PackageError> for DocxError {
    fn from(value: PackageError) -> Self {
        Self::Package(value)
    }
}

impl From<nexa_ooxml::PartNameError> for DocxError {
    fn from(value: nexa_ooxml::PartNameError) -> Self {
        Self::PartName(value)
    }
}

pub fn open_docx<R: Read + Seek>(reader: R) -> Result<DocxDocument, DocxError> {
    let mut lazy = LazyZipPackage::open(reader)?;
    let info = lazy
        .office_package_info()
        .map_err(|error| DocxError::Xml(error.to_string()))?;
    if info.kind() != OfficePackageKind::Document {
        return Err(DocxError::WrongOfficeKind);
    }

    let main_part = info.main_part().clone();
    let package = lazy.load_owned_package()?;
    let main_bytes = package
        .part(&main_part)
        .ok_or(DocxError::MissingMainPart)?
        .bytes();

    let relationships = package.relationships(&main_part).cloned().unwrap_or_default();
    let mut document = parse_document_xml(main_bytes, &relationships)?;

    let styles_part = relationship_target_by_suffix(&relationships, "/styles")
        .unwrap_or_else(|| PartName::new(STYLES_PART).expect("constant part"));
    if let Some(part) = package.part(&styles_part) {
        document.styles = parse_styles_xml(part.bytes())?;
    }

    let numbering_part = relationship_target_by_suffix(&relationships, "/numbering")
        .unwrap_or_else(|| PartName::new(NUMBERING_PART).expect("constant part"));
    if let Some(part) = package.part(&numbering_part) {
        document.numbering = parse_numbering_xml(part.bytes())?;
    }

    for relationship in relationships.iter() {
        let RelationshipTarget::Internal(target) = &relationship.target else {
            continue;
        };
        if relationship.relationship_type.ends_with("/header") {
            if let Some(part) = package.part(target) {
                document.headers.insert(
                    relationship.id.as_str().to_owned(),
                    HeaderFooter {
                        blocks: parse_fragment_xml(part.bytes(), "hdr")?,
                    },
                );
            }
        } else if relationship.relationship_type.ends_with("/footer")
            && let Some(part) = package.part(target)
        {
            document.footers.insert(
                relationship.id.as_str().to_owned(),
                HeaderFooter {
                    blocks: parse_fragment_xml(part.bytes(), "ftr")?,
                },
            );
        }
    }

    Ok(DocxDocument {
        document,
        package,
        main_part,
    })
}

pub fn save_docx<W: Write + Seek>(
    document: &mut DocxDocument,
    writer: W,
) -> Result<W, DocxError> {
    document.sync_package()?;
    write_owned_package(&document.package, writer).map_err(DocxError::Zip)
}

fn relationship_target_by_suffix(
    relationships: &RelationshipSet,
    suffix: &str,
) -> Option<PartName> {
    relationships.iter().find_map(|relationship| {
        if !relationship.relationship_type.ends_with(suffix) {
            return None;
        }
        match &relationship.target {
            RelationshipTarget::Internal(target) => Some(target.clone()),
            RelationshipTarget::External(_) => None,
        }
    })
}

#[derive(Debug, Default)]
struct RunBuilder {
    properties: RunProperties,
    contents: Vec<RunContent>,
    image_relationship: Option<String>,
    image_width_emu: u64,
    image_height_emu: u64,
    image_alt: Option<String>,
}

impl RunBuilder {
    fn finish(mut self, relationships: &RelationshipSet) -> Vec<Run> {
        if let Some(relationship_id) = self.image_relationship.take() {
            let part_name = relationships
                .get(&RelationshipId::new(&relationship_id))
                .and_then(|relationship| match &relationship.target {
                    RelationshipTarget::Internal(target) => Some(target.clone()),
                    RelationshipTarget::External(_) => None,
                });

            self.contents.push(RunContent::Image(InlineImage {
                relationship_id,
                part_name,
                width_emu: self.image_width_emu,
                height_emu: self.image_height_emu,
                alt_text: self.image_alt,
            }));
        }

        if self.contents.is_empty() {
            self.contents.push(RunContent::Text(String::new()));
        }

        self.contents
            .into_iter()
            .map(|content| Run {
                content,
                properties: self.properties.clone(),
            })
            .collect()
    }
}

#[derive(Debug, Default)]
struct TableBuilder {
    rows: Vec<TableRow>,
    current_row: Option<TableRow>,
    current_cell: Option<TableCell>,
}

fn parse_document_xml(
    input: &[u8],
    relationships: &RelationshipSet,
) -> Result<Document, DocxError> {
    let (blocks, mut sections, compatibility) =
        parse_word_xml(input, "document", relationships, true)?;
    if sections.is_empty() {
        sections.push(Section {
            start_block: 0,
            properties: SectionProperties::default(),
        });
    }

    Ok(Document {
        blocks: if blocks.is_empty() {
            vec![Block::Paragraph(Paragraph::default())]
        } else {
            blocks
        },
        styles: StyleSheet::default(),
        numbering: Numbering::default(),
        sections,
        headers: BTreeMap::new(),
        footers: BTreeMap::new(),
        compatibility,
    })
}

fn parse_fragment_xml(input: &[u8], root: &str) -> Result<Vec<Block>, DocxError> {
    let relationships = RelationshipSet::default();
    let (blocks, _, _) = parse_word_xml(input, root, &relationships, false)?;
    Ok(blocks)
}

fn parse_word_xml(
    input: &[u8],
    expected_root: &str,
    relationships: &RelationshipSet,
    collect_sections: bool,
) -> Result<(Vec<Block>, Vec<Section>, CompatibilityReport), DocxError> {
    let mut reader = Reader::from_reader(input);
    reader.config_mut().trim_text(false);
    let mut buffer = Vec::new();

    let mut blocks = Vec::new();
    let mut table_stack: Vec<TableBuilder> = Vec::new();
    let mut paragraph: Option<Paragraph> = None;
    let mut run: Option<RunBuilder> = None;
    let mut in_text = false;
    let mut in_ppr = false;
    let mut in_rpr = false;
    let mut in_numpr = false;
    let mut section = SectionProperties::default();
    let mut in_sectpr = false;
    let mut sections = Vec::new();
    let mut root_seen = false;
    let mut compatibility = CompatibilityReport::default();

    loop {
        let event = reader
            .read_event_into(&mut buffer)
            .map_err(|error| DocxError::Xml(error.to_string()))?;

        match event {
            Event::Start(element) => {
                let name = local_name(&element);
                if !root_seen {
                    if name != expected_root {
                        return Err(DocxError::Xml(format!(
                            "unexpected root element {name}, expected {expected_root}"
                        )));
                    }
                    root_seen = true;
                    buffer.clear();
                    continue;
                }

                match name.as_str() {
                    "body" | "hyperlink" | "smartTag" | "sdt" | "sdtContent" => {}
                    "p" => paragraph = Some(Paragraph::default()),
                    "pPr" => in_ppr = true,
                    "r" => run = Some(RunBuilder::default()),
                    "rPr" => in_rpr = true,
                    "t" | "instrText" => in_text = true,
                    "numPr" => in_numpr = true,
                    "tbl" => table_stack.push(TableBuilder::default()),
                    "tr" => {
                        if let Some(table) = table_stack.last_mut() {
                            table.current_row = Some(TableRow::default());
                        }
                    }
                    "tc" => {
                        if let Some(table) = table_stack.last_mut() {
                            table.current_cell = Some(TableCell::default());
                        }
                    }
                    "sectPr" => {
                        in_sectpr = true;
                        section = SectionProperties::default();
                    }
                    "drawing" | "inline" | "anchor" | "graphic" | "graphicData" | "pic"
                    | "blipFill" | "spPr" | "xfrm" | "nvPicPr" | "cNvPr" | "cNvPicPr"
                    | "stretch" | "fillRect" | "off" | "ext" | "prstGeom" | "avLst" => {}
                    "tab" => {
                        if let Some(run) = &mut run {
                            run.contents.push(RunContent::Tab);
                        }
                    }
                    "br" | "cr" => {
                        if let Some(run) = &mut run {
                            run.contents.push(RunContent::LineBreak);
                        }
                    }
                    "b" if in_rpr => {
                        if let Some(run) = &mut run {
                            run.properties.bold = bool_on(&element);
                        }
                    }
                    "i" if in_rpr => {
                        if let Some(run) = &mut run {
                            run.properties.italic = bool_on(&element);
                        }
                    }
                    "u" if in_rpr => {
                        if let Some(run) = &mut run {
                            run.properties.underline =
                                attr(&element, "val").is_none_or(|value| value != "none");
                        }
                    }
                    "strike" if in_rpr => {
                        if let Some(run) = &mut run {
                            run.properties.strike = bool_on(&element);
                        }
                    }
                    "color" if in_rpr => {
                        if let Some(run) = &mut run {
                            run.properties.color =
                                attr(&element, "val").and_then(|value| parse_rgb(&value));
                        }
                    }
                    "rFonts" if in_rpr => {
                        if let Some(run) = &mut run {
                            run.properties.font_family = attr(&element, "ascii")
                                .or_else(|| attr(&element, "eastAsia"))
                                .or_else(|| attr(&element, "hAnsi"));
                        }
                    }
                    "sz" if in_rpr => {
                        if let Some(run) = &mut run {
                            run.properties.font_size_half_points =
                                attr(&element, "val").and_then(|value| value.parse().ok());
                        }
                    }
                    "rStyle" if in_rpr => {
                        if let Some(run) = &mut run {
                            run.properties.style_id = attr(&element, "val");
                        }
                    }
                    "pStyle" if in_ppr => {
                        if let Some(paragraph) = &mut paragraph {
                            paragraph.properties.style_id = attr(&element, "val");
                        }
                    }
                    "jc" if in_ppr => {
                        if let Some(paragraph) = &mut paragraph {
                            paragraph.properties.alignment = match attr(&element, "val").as_deref() {
                                Some("center") => Alignment::Center,
                                Some("right") | Some("end") => Alignment::Right,
                                Some("both") | Some("distribute") => Alignment::Justify,
                                _ => Alignment::Left,
                            };
                        }
                    }
                    "spacing" if in_ppr => {
                        if let Some(paragraph) = &mut paragraph {
                            paragraph.properties.spacing_before_twips =
                                parse_u32_attr(&element, "before").unwrap_or(0);
                            paragraph.properties.spacing_after_twips =
                                parse_u32_attr(&element, "after").unwrap_or(0);
                            paragraph.properties.line_spacing_twips =
                                parse_u32_attr(&element, "line");
                        }
                    }
                    "ind" if in_ppr => {
                        if let Some(paragraph) = &mut paragraph {
                            paragraph.properties.indent_left_twips =
                                parse_i32_attr(&element, "left").unwrap_or(0);
                            paragraph.properties.indent_right_twips =
                                parse_i32_attr(&element, "right").unwrap_or(0);
                            paragraph.properties.first_line_twips =
                                parse_i32_attr(&element, "firstLine")
                                    .or_else(|| {
                                        parse_i32_attr(&element, "hanging")
                                            .map(|value| -value)
                                    })
                                    .unwrap_or(0);
                        }
                    }
                    "pageBreakBefore" if in_ppr => {
                        if let Some(paragraph) = &mut paragraph {
                            paragraph.properties.page_break_before = bool_on(&element);
                        }
                    }
                    "keepNext" if in_ppr => {
                        if let Some(paragraph) = &mut paragraph {
                            paragraph.properties.keep_next = bool_on(&element);
                        }
                    }
                    "keepLines" if in_ppr => {
                        if let Some(paragraph) = &mut paragraph {
                            paragraph.properties.keep_lines = bool_on(&element);
                        }
                    }
                    "ilvl" if in_numpr => {
                        if let Some(paragraph) = &mut paragraph {
                            let level = parse_u32_attr(&element, "val").unwrap_or(0) as u8;
                            paragraph
                                .properties
                                .list
                                .get_or_insert(ListReference {
                                    numbering_id: 0,
                                    level,
                                })
                                .level = level;
                        }
                    }
                    "numId" if in_numpr => {
                        if let Some(paragraph) = &mut paragraph {
                            let numbering_id = parse_u32_attr(&element, "val").unwrap_or(0);
                            paragraph
                                .properties
                                .list
                                .get_or_insert(ListReference {
                                    numbering_id,
                                    level: 0,
                                })
                                .numbering_id = numbering_id;
                        }
                    }
                    "pgSz" if in_sectpr => apply_page_size(&element, &mut section),
                    "pgMar" if in_sectpr => apply_page_margins(&element, &mut section),
                    "headerReference" if in_sectpr => {
                        if attr(&element, "type").as_deref() == Some("default") {
                            section.header_default = attr(&element, "id");
                        }
                    }
                    "footerReference" if in_sectpr => {
                        if attr(&element, "type").as_deref() == Some("default") {
                            section.footer_default = attr(&element, "id");
                        }
                    }
                    "extent" => {
                        if let Some(run) = &mut run {
                            run.image_width_emu = parse_u64_attr(&element, "cx").unwrap_or(0);
                            run.image_height_emu = parse_u64_attr(&element, "cy").unwrap_or(0);
                        }
                    }
                    "docPr" => {
                        if let Some(run) = &mut run {
                            run.image_alt = attr(&element, "descr").or_else(|| attr(&element, "name"));
                        }
                    }
                    "blip" => {
                        if let Some(run) = &mut run {
                            run.image_relationship = attr(&element, "embed");
                        }
                    }
                    "tblPr" | "tblGrid" | "gridCol" | "trPr" | "tcPr" | "tcW" | "tblW"
                    | "gridSpan" | "vMerge" => {}
                    other => mark_unsupported(
                        &mut compatibility,
                        DOCUMENT_PART,
                        other,
                        "unsupported WordprocessingML element would not be reproduced on save",
                    ),
                }
            }
            Event::Empty(element) => {
                let name = local_name(&element);
                match name.as_str() {
                    "tab" => {
                        if let Some(run) = &mut run {
                            run.contents.push(RunContent::Tab);
                        }
                    }
                    "br" | "cr" => {
                        if let Some(run) = &mut run {
                            run.contents.push(RunContent::LineBreak);
                        }
                    }
                    "b" if in_rpr => {
                        if let Some(run) = &mut run {
                            run.properties.bold = bool_on(&element);
                        }
                    }
                    "i" if in_rpr => {
                        if let Some(run) = &mut run {
                            run.properties.italic = bool_on(&element);
                        }
                    }
                    "u" if in_rpr => {
                        if let Some(run) = &mut run {
                            run.properties.underline =
                                attr(&element, "val").is_none_or(|value| value != "none");
                        }
                    }
                    "strike" if in_rpr => {
                        if let Some(run) = &mut run {
                            run.properties.strike = bool_on(&element);
                        }
                    }
                    "color" if in_rpr => {
                        if let Some(run) = &mut run {
                            run.properties.color =
                                attr(&element, "val").and_then(|value| parse_rgb(&value));
                        }
                    }
                    "rFonts" if in_rpr => {
                        if let Some(run) = &mut run {
                            run.properties.font_family = attr(&element, "ascii")
                                .or_else(|| attr(&element, "eastAsia"))
                                .or_else(|| attr(&element, "hAnsi"));
                        }
                    }
                    "sz" if in_rpr => {
                        if let Some(run) = &mut run {
                            run.properties.font_size_half_points =
                                attr(&element, "val").and_then(|value| value.parse().ok());
                        }
                    }
                    "rStyle" if in_rpr => {
                        if let Some(run) = &mut run {
                            run.properties.style_id = attr(&element, "val");
                        }
                    }
                    "pStyle" if in_ppr => {
                        if let Some(paragraph) = &mut paragraph {
                            paragraph.properties.style_id = attr(&element, "val");
                        }
                    }
                    "jc" if in_ppr => {
                        if let Some(paragraph) = &mut paragraph {
                            paragraph.properties.alignment = match attr(&element, "val").as_deref() {
                                Some("center") => Alignment::Center,
                                Some("right") | Some("end") => Alignment::Right,
                                Some("both") | Some("distribute") => Alignment::Justify,
                                _ => Alignment::Left,
                            };
                        }
                    }
                    "spacing" if in_ppr => {
                        if let Some(paragraph) = &mut paragraph {
                            paragraph.properties.spacing_before_twips =
                                parse_u32_attr(&element, "before").unwrap_or(0);
                            paragraph.properties.spacing_after_twips =
                                parse_u32_attr(&element, "after").unwrap_or(0);
                            paragraph.properties.line_spacing_twips =
                                parse_u32_attr(&element, "line");
                        }
                    }
                    "ind" if in_ppr => {
                        if let Some(paragraph) = &mut paragraph {
                            paragraph.properties.indent_left_twips =
                                parse_i32_attr(&element, "left").unwrap_or(0);
                            paragraph.properties.indent_right_twips =
                                parse_i32_attr(&element, "right").unwrap_or(0);
                            paragraph.properties.first_line_twips =
                                parse_i32_attr(&element, "firstLine")
                                    .or_else(|| {
                                        parse_i32_attr(&element, "hanging")
                                            .map(|value| -value)
                                    })
                                    .unwrap_or(0);
                        }
                    }
                    "pageBreakBefore" if in_ppr => {
                        if let Some(paragraph) = &mut paragraph {
                            paragraph.properties.page_break_before = bool_on(&element);
                        }
                    }
                    "keepNext" if in_ppr => {
                        if let Some(paragraph) = &mut paragraph {
                            paragraph.properties.keep_next = bool_on(&element);
                        }
                    }
                    "keepLines" if in_ppr => {
                        if let Some(paragraph) = &mut paragraph {
                            paragraph.properties.keep_lines = bool_on(&element);
                        }
                    }
                    "ilvl" if in_numpr => {
                        if let Some(paragraph) = &mut paragraph {
                            let level = parse_u32_attr(&element, "val").unwrap_or(0) as u8;
                            paragraph
                                .properties
                                .list
                                .get_or_insert(ListReference {
                                    numbering_id: 0,
                                    level,
                                })
                                .level = level;
                        }
                    }
                    "numId" if in_numpr => {
                        if let Some(paragraph) = &mut paragraph {
                            let numbering_id = parse_u32_attr(&element, "val").unwrap_or(0);
                            paragraph
                                .properties
                                .list
                                .get_or_insert(ListReference {
                                    numbering_id,
                                    level: 0,
                                })
                                .numbering_id = numbering_id;
                        }
                    }
                    "pgSz" if in_sectpr => apply_page_size(&element, &mut section),
                    "pgMar" if in_sectpr => apply_page_margins(&element, &mut section),
                    "headerReference" if in_sectpr => {
                        if attr(&element, "type").as_deref() == Some("default") {
                            section.header_default = attr(&element, "id");
                        }
                    }
                    "footerReference" if in_sectpr => {
                        if attr(&element, "type").as_deref() == Some("default") {
                            section.footer_default = attr(&element, "id");
                        }
                    }
                    "extent" => {
                        if let Some(run) = &mut run {
                            run.image_width_emu = parse_u64_attr(&element, "cx").unwrap_or(0);
                            run.image_height_emu = parse_u64_attr(&element, "cy").unwrap_or(0);
                        }
                    }
                    "docPr" => {
                        if let Some(run) = &mut run {
                            run.image_alt = attr(&element, "descr").or_else(|| attr(&element, "name"));
                        }
                    }
                    "blip" => {
                        if let Some(run) = &mut run {
                            run.image_relationship = attr(&element, "embed");
                        }
                    }
                    "tcW" => {
                        if let Some(cell) = table_stack
                            .last_mut()
                            .and_then(|table| table.current_cell.as_mut())
                        {
                            cell.properties.width_twips = parse_u32_attr(&element, "w");
                        }
                    }
                    "gridSpan" => {
                        if let Some(cell) = table_stack
                            .last_mut()
                            .and_then(|table| table.current_cell.as_mut())
                        {
                            cell.properties.grid_span =
                                parse_u32_attr(&element, "val").unwrap_or(1) as u16;
                        }
                    }
                    "vMerge" => {
                        if let Some(cell) = table_stack
                            .last_mut()
                            .and_then(|table| table.current_cell.as_mut())
                        {
                            cell.properties.vertical_merge = true;
                        }
                    }
                    "bookmarkStart" | "bookmarkEnd" | "proofErr" | "lastRenderedPageBreak" => {
                        mark_unsupported(
                            &mut compatibility,
                            DOCUMENT_PART,
                            &name,
                            "metadata marker is not yet preserved by the semantic writer",
                        );
                    }
                    _ => {}
                }
            }
            Event::Text(text) => {
                if in_text {
                    if let Some(run) = &mut run {
                        let decoded = text
                            .xml_content()
                            .map_err(|error| DocxError::Xml(error.to_string()))?;
                        run.contents.push(RunContent::Text(decoded.into_owned()));
                    }
                }
            }
            Event::End(element) => {
                let name = String::from_utf8_lossy(element.local_name().as_ref()).into_owned();
                match name.as_str() {
                    "t" | "instrText" => in_text = false,
                    "rPr" => in_rpr = false,
                    "pPr" => in_ppr = false,
                    "numPr" => in_numpr = false,
                    "r" => {
                        if let (Some(paragraph), Some(builder)) =
                            (&mut paragraph, run.take())
                        {
                            paragraph.runs.extend(builder.finish(relationships));
                        }
                    }
                    "p" => {
                        if let Some(mut value) = paragraph.take() {
                            if value.runs.is_empty() {
                                value.runs.push(Run::text(""));
                            }
                            push_block(
                                &mut blocks,
                                &mut table_stack,
                                Block::Paragraph(value),
                            );
                        }
                    }
                    "tc" => {
                        if let Some(table) = table_stack.last_mut() {
                            if let Some(mut cell) = table.current_cell.take() {
                                if cell.blocks.is_empty() {
                                    cell.blocks.push(Block::Paragraph(Paragraph::default()));
                                }
                                if let Some(row) = table.current_row.as_mut() {
                                    row.cells.push(cell);
                                }
                            }
                        }
                    }
                    "tr" => {
                        if let Some(table) = table_stack.last_mut() {
                            if let Some(row) = table.current_row.take() {
                                table.rows.push(row);
                            }
                        }
                    }
                    "tbl" => {
                        if let Some(table) = table_stack.pop() {
                            push_block(
                                &mut blocks,
                                &mut table_stack,
                                Block::Table(Table {
                                    rows: table.rows,
                                    properties: TableProperties::default(),
                                }),
                            );
                        }
                    }
                    "sectPr" => {
                        in_sectpr = false;
                        if collect_sections {
                            let start_block = blocks.len().saturating_sub(1);
                            if sections.is_empty() {
                                sections.push(Section {
                                    start_block: 0,
                                    properties: section.clone(),
                                });
                            } else {
                                sections.push(Section {
                                    start_block,
                                    properties: section.clone(),
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
            Event::DocType(_) => {
                return Err(DocxError::Xml(
                    "DOCTYPE is not allowed in WordprocessingML".into(),
                ));
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }

    if !root_seen {
        return Err(DocxError::Xml(format!("missing root element {expected_root}")));
    }

    Ok((blocks, sections, compatibility))
}

fn push_block(
    blocks: &mut Vec<Block>,
    tables: &mut [TableBuilder],
    block: Block,
) {
    if let Some(cell) = tables.last_mut().and_then(|table| table.current_cell.as_mut()) {
        cell.blocks.push(block);
    } else {
        blocks.push(block);
    }
}

fn mark_unsupported(
    report: &mut CompatibilityReport,
    part: &str,
    element: &str,
    detail: &str,
) {
    if !report
        .issues
        .iter()
        .any(|issue| issue.part == part && issue.element == element)
    {
        report.block_save(part, element, detail);
    }
}

fn parse_styles_xml(input: &[u8]) -> Result<StyleSheet, DocxError> {
    let mut reader = Reader::from_reader(input);
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut sheet = StyleSheet::default();
    let mut current: Option<Style> = None;
    let mut in_ppr = false;
    let mut in_rpr = false;

    loop {
        match reader
            .read_event_into(&mut buffer)
            .map_err(|error| DocxError::Xml(error.to_string()))?
        {
            Event::Start(element) => match local_name(&element).as_str() {
                "style" => {
                    let kind = match attr(&element, "type").as_deref() {
                        Some("character") => StyleKind::Character,
                        Some("table") => StyleKind::Table,
                        _ => StyleKind::Paragraph,
                    };
                    let id = attr(&element, "styleId").unwrap_or_default();
                    current = Some(Style {
                        id,
                        name: String::new(),
                        kind,
                        based_on: None,
                        paragraph: ParagraphProperties::default(),
                        run: RunProperties::default(),
                    });
                }
                "pPr" => in_ppr = true,
                "rPr" => in_rpr = true,
                "name" => {
                    if let Some(style) = &mut current {
                        style.name = attr(&element, "val").unwrap_or_default();
                    }
                }
                "basedOn" => {
                    if let Some(style) = &mut current {
                        style.based_on = attr(&element, "val");
                    }
                }
                "b" if in_rpr => {
                    if let Some(style) = &mut current {
                        style.run.bold = bool_on(&element);
                    }
                }
                "i" if in_rpr => {
                    if let Some(style) = &mut current {
                        style.run.italic = bool_on(&element);
                    }
                }
                "u" if in_rpr => {
                    if let Some(style) = &mut current {
                        style.run.underline =
                            attr(&element, "val").is_none_or(|value| value != "none");
                    }
                }
                "color" if in_rpr => {
                    if let Some(style) = &mut current {
                        style.run.color = attr(&element, "val").and_then(|value| parse_rgb(&value));
                    }
                }
                "sz" if in_rpr => {
                    if let Some(style) = &mut current {
                        style.run.font_size_half_points =
                            attr(&element, "val").and_then(|value| value.parse().ok());
                    }
                }
                "jc" if in_ppr => {
                    if let Some(style) = &mut current {
                        style.paragraph.alignment = match attr(&element, "val").as_deref() {
                            Some("center") => Alignment::Center,
                            Some("right") => Alignment::Right,
                            Some("both") => Alignment::Justify,
                            _ => Alignment::Left,
                        };
                    }
                }
                _ => {}
            },
            Event::Empty(element) => match local_name(&element).as_str() {
                "name" => {
                    if let Some(style) = &mut current {
                        style.name = attr(&element, "val").unwrap_or_default();
                    }
                }
                "basedOn" => {
                    if let Some(style) = &mut current {
                        style.based_on = attr(&element, "val");
                    }
                }
                "b" if in_rpr => {
                    if let Some(style) = &mut current {
                        style.run.bold = bool_on(&element);
                    }
                }
                "i" if in_rpr => {
                    if let Some(style) = &mut current {
                        style.run.italic = bool_on(&element);
                    }
                }
                "u" if in_rpr => {
                    if let Some(style) = &mut current {
                        style.run.underline =
                            attr(&element, "val").is_none_or(|value| value != "none");
                    }
                }
                "color" if in_rpr => {
                    if let Some(style) = &mut current {
                        style.run.color = attr(&element, "val").and_then(|value| parse_rgb(&value));
                    }
                }
                "sz" if in_rpr => {
                    if let Some(style) = &mut current {
                        style.run.font_size_half_points =
                            attr(&element, "val").and_then(|value| value.parse().ok());
                    }
                }
                "jc" if in_ppr => {
                    if let Some(style) = &mut current {
                        style.paragraph.alignment = match attr(&element, "val").as_deref() {
                            Some("center") => Alignment::Center,
                            Some("right") => Alignment::Right,
                            Some("both") => Alignment::Justify,
                            _ => Alignment::Left,
                        };
                    }
                }
                _ => {}
            },
            Event::End(element) => match String::from_utf8_lossy(element.local_name().as_ref()).as_ref() {
                "pPr" => in_ppr = false,
                "rPr" => in_rpr = false,
                "style" => {
                    if let Some(style) = current.take() {
                        if style.id == "Normal" || sheet.default_paragraph_style.is_none() {
                            if style.kind == StyleKind::Paragraph {
                                sheet.default_paragraph_style = Some(style.id.clone());
                            }
                        }
                        sheet.styles.insert(style.id.clone(), style);
                    }
                }
                _ => {}
            },
            Event::DocType(_) => {
                return Err(DocxError::Xml("DOCTYPE is not allowed in styles".into()));
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }

    Ok(sheet)
}

fn parse_numbering_xml(input: &[u8]) -> Result<Numbering, DocxError> {
    let mut reader = Reader::from_reader(input);
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut numbering = Numbering::default();
    let mut abstract_list: Option<AbstractList> = None;
    let mut level: Option<ListLevel> = None;
    let mut number_id: Option<u32> = None;
    let mut abstract_id_for_num: Option<u32> = None;

    loop {
        match reader
            .read_event_into(&mut buffer)
            .map_err(|error| DocxError::Xml(error.to_string()))?
        {
            Event::Start(element) => match local_name(&element).as_str() {
                "abstractNum" => {
                    abstract_list = Some(AbstractList {
                        id: parse_u32_attr(&element, "abstractNumId").unwrap_or(0),
                        levels: BTreeMap::new(),
                    });
                }
                "lvl" => {
                    level = Some(ListLevel {
                        level: parse_u32_attr(&element, "ilvl").unwrap_or(0) as u8,
                        start: 1,
                        format: ListFormat::Decimal,
                        text: "%1.".into(),
                        paragraph: ParagraphProperties::default(),
                        run: RunProperties::default(),
                    });
                }
                "num" => number_id = parse_u32_attr(&element, "numId"),
                "start" => {
                    if let Some(level) = &mut level {
                        level.start = parse_u32_attr(&element, "val").unwrap_or(1);
                    }
                }
                "numFmt" => {
                    if let Some(level) = &mut level {
                        level.format = parse_list_format(attr(&element, "val").as_deref());
                    }
                }
                "lvlText" => {
                    if let Some(level) = &mut level {
                        level.text = attr(&element, "val").unwrap_or_default();
                    }
                }
                "abstractNumId" if number_id.is_some() => {
                    abstract_id_for_num = parse_u32_attr(&element, "val");
                }
                _ => {}
            },
            Event::Empty(element) => match local_name(&element).as_str() {
                "start" => {
                    if let Some(level) = &mut level {
                        level.start = parse_u32_attr(&element, "val").unwrap_or(1);
                    }
                }
                "numFmt" => {
                    if let Some(level) = &mut level {
                        level.format = parse_list_format(attr(&element, "val").as_deref());
                    }
                }
                "lvlText" => {
                    if let Some(level) = &mut level {
                        level.text = attr(&element, "val").unwrap_or_default();
                    }
                }
                "abstractNumId" if number_id.is_some() => {
                    abstract_id_for_num = parse_u32_attr(&element, "val");
                }
                _ => {}
            },
            Event::End(element) => match String::from_utf8_lossy(element.local_name().as_ref()).as_ref() {
                "lvl" => {
                    if let (Some(list), Some(level)) = (&mut abstract_list, level.take()) {
                        list.levels.insert(level.level, level);
                    }
                }
                "abstractNum" => {
                    if let Some(list) = abstract_list.take() {
                        numbering.abstract_lists.insert(list.id, list);
                    }
                }
                "num" => {
                    if let (Some(id), Some(abstract_id)) = (number_id.take(), abstract_id_for_num.take()) {
                        numbering.instances.insert(
                            id,
                            NumberingInstance {
                                id,
                                abstract_id,
                            },
                        );
                    }
                }
                _ => {}
            },
            Event::DocType(_) => {
                return Err(DocxError::Xml("DOCTYPE is not allowed in numbering".into()));
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }

    Ok(numbering)
}

fn parse_list_format(value: Option<&str>) -> ListFormat {
    match value {
        Some("lowerLetter") => ListFormat::LowerLetter,
        Some("upperLetter") => ListFormat::UpperLetter,
        Some("lowerRoman") => ListFormat::LowerRoman,
        Some("upperRoman") => ListFormat::UpperRoman,
        Some("bullet") => ListFormat::Bullet,
        Some("none") => ListFormat::None,
        _ => ListFormat::Decimal,
    }
}

fn apply_page_size(element: &BytesStart<'_>, section: &mut SectionProperties) {
    section.page_width_twips = parse_u32_attr(element, "w").unwrap_or(section.page_width_twips);
    section.page_height_twips = parse_u32_attr(element, "h").unwrap_or(section.page_height_twips);
    section.orientation = if attr(element, "orient").as_deref() == Some("landscape") {
        Orientation::Landscape
    } else {
        Orientation::Portrait
    };
}

fn apply_page_margins(element: &BytesStart<'_>, section: &mut SectionProperties) {
    section.margins = PageMargins {
        top_twips: parse_u32_attr(element, "top").unwrap_or(section.margins.top_twips),
        right_twips: parse_u32_attr(element, "right").unwrap_or(section.margins.right_twips),
        bottom_twips: parse_u32_attr(element, "bottom").unwrap_or(section.margins.bottom_twips),
        left_twips: parse_u32_attr(element, "left").unwrap_or(section.margins.left_twips),
        header_twips: parse_u32_attr(element, "header").unwrap_or(section.margins.header_twips),
        footer_twips: parse_u32_attr(element, "footer").unwrap_or(section.margins.footer_twips),
    };
}

fn bool_on(element: &BytesStart<'_>) -> bool {
    !matches!(attr(element, "val").as_deref(), Some("0" | "false" | "off" | "none"))
}

fn local_name(element: &BytesStart<'_>) -> String {
    String::from_utf8_lossy(element.local_name().as_ref()).into_owned()
}

fn attr(element: &BytesStart<'_>, name: &str) -> Option<String> {
    for attribute in element.attributes().flatten() {
        if attribute.key.local_name().as_ref() == name.as_bytes() {
            if let Ok(value) = attribute.normalized_value(XmlVersion::Implicit1_0) {
                return Some(value.into_owned());
            }
        }
    }
    None
}

fn parse_u32_attr(element: &BytesStart<'_>, name: &str) -> Option<u32> {
    attr(element, name)?.parse().ok()
}

fn parse_i32_attr(element: &BytesStart<'_>, name: &str) -> Option<i32> {
    attr(element, name)?.parse().ok()
}

fn parse_u64_attr(element: &BytesStart<'_>, name: &str) -> Option<u64> {
    attr(element, name)?.parse().ok()
}

fn parse_rgb(value: &str) -> Option<RgbColor> {
    if value.len() != 6 || value.eq_ignore_ascii_case("auto") {
        return None;
    }
    Some(RgbColor {
        red: u8::from_str_radix(&value[0..2], 16).ok()?,
        green: u8::from_str_radix(&value[2..4], 16).ok()?,
        blue: u8::from_str_radix(&value[4..6], 16).ok()?,
    })
}

fn write_document_xml(document: &Document) -> Vec<u8> {
    let mut output = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"><w:body>"#,
    );

    for block in &document.blocks {
        write_block(&mut output, block);
    }

    let section = document
        .sections
        .last()
        .map_or_else(SectionProperties::default, |value| value.properties.clone());
    write_section_properties(&mut output, &section);
    output.push_str("</w:body></w:document>");
    output.into_bytes()
}

fn write_header_footer_xml(root: &str, value: &HeaderFooter) -> Vec<u8> {
    let mut output = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:{root} xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#
    );
    for block in &value.blocks {
        write_block(&mut output, block);
    }
    output.push_str(&format!("</w:{root}>"));
    output.into_bytes()
}

fn write_block(output: &mut String, block: &Block) {
    match block {
        Block::Paragraph(paragraph) => write_paragraph(output, paragraph),
        Block::Table(table) => write_table(output, table),
    }
}

fn write_paragraph(output: &mut String, paragraph: &Paragraph) {
    output.push_str("<w:p>");
    write_paragraph_properties(output, &paragraph.properties);
    for run in &paragraph.runs {
        write_run(output, run);
    }
    output.push_str("</w:p>");
}

fn write_paragraph_properties(output: &mut String, props: &ParagraphProperties) {
    let has_props = props.style_id.is_some()
        || props.alignment != Alignment::Left
        || props.list.is_some()
        || props.spacing_before_twips != 0
        || props.spacing_after_twips != 0
        || props.line_spacing_twips.is_some()
        || props.indent_left_twips != 0
        || props.indent_right_twips != 0
        || props.first_line_twips != 0
        || props.keep_next
        || props.keep_lines
        || props.page_break_before;

    if !has_props {
        return;
    }

    output.push_str("<w:pPr>");
    if let Some(style) = &props.style_id {
        push_val(output, "w:pStyle", style);
    }
    match props.alignment {
        Alignment::Left => {}
        Alignment::Center => push_val(output, "w:jc", "center"),
        Alignment::Right => push_val(output, "w:jc", "right"),
        Alignment::Justify => push_val(output, "w:jc", "both"),
    }
    if let Some(list) = props.list {
        output.push_str("<w:numPr>");
        push_val(output, "w:ilvl", &list.level.to_string());
        push_val(output, "w:numId", &list.numbering_id.to_string());
        output.push_str("</w:numPr>");
    }
    if props.spacing_before_twips != 0
        || props.spacing_after_twips != 0
        || props.line_spacing_twips.is_some()
    {
        output.push_str("<w:spacing");
        push_attr(output, "w:before", &props.spacing_before_twips.to_string());
        push_attr(output, "w:after", &props.spacing_after_twips.to_string());
        if let Some(line) = props.line_spacing_twips {
            push_attr(output, "w:line", &line.to_string());
        }
        output.push_str("/>");
    }
    if props.indent_left_twips != 0
        || props.indent_right_twips != 0
        || props.first_line_twips != 0
    {
        output.push_str("<w:ind");
        push_attr(output, "w:left", &props.indent_left_twips.to_string());
        push_attr(output, "w:right", &props.indent_right_twips.to_string());
        if props.first_line_twips >= 0 {
            push_attr(output, "w:firstLine", &props.first_line_twips.to_string());
        } else {
            push_attr(output, "w:hanging", &props.first_line_twips.unsigned_abs().to_string());
        }
        output.push_str("/>");
    }
    if props.keep_next {
        output.push_str("<w:keepNext/>");
    }
    if props.keep_lines {
        output.push_str("<w:keepLines/>");
    }
    if props.page_break_before {
        output.push_str("<w:pageBreakBefore/>");
    }
    output.push_str("</w:pPr>");
}

fn write_run(output: &mut String, run: &Run) {
    output.push_str("<w:r>");
    write_run_properties(output, &run.properties);

    match &run.content {
        RunContent::Text(text) => {
            output.push_str("<w:t xml:space="preserve">");
            escape_text(output, text);
            output.push_str("</w:t>");
        }
        RunContent::Tab => output.push_str("<w:tab/>"),
        RunContent::LineBreak => output.push_str("<w:br/>"),
        RunContent::Image(image) => write_image(output, image),
    }
    output.push_str("</w:r>");
}

fn write_run_properties(output: &mut String, props: &RunProperties) {
    if !props.bold
        && !props.italic
        && !props.underline
        && !props.strike
        && props.color.is_none()
        && props.font_family.is_none()
        && props.font_size_half_points.is_none()
        && props.style_id.is_none()
    {
        return;
    }

    output.push_str("<w:rPr>");
    if let Some(style) = &props.style_id {
        push_val(output, "w:rStyle", style);
    }
    if props.bold {
        output.push_str("<w:b/>");
    }
    if props.italic {
        output.push_str("<w:i/>");
    }
    if props.underline {
        output.push_str("<w:u w:val="single"/>");
    }
    if props.strike {
        output.push_str("<w:strike/>");
    }
    if let Some(color) = props.color {
        output.push_str("<w:color");
        push_attr(
            output,
            "w:val",
            &format!("{:02X}{:02X}{:02X}", color.red, color.green, color.blue),
        );
        output.push_str("/>");
    }
    if let Some(font) = &props.font_family {
        output.push_str("<w:rFonts");
        push_attr(output, "w:ascii", font);
        push_attr(output, "w:hAnsi", font);
        push_attr(output, "w:eastAsia", font);
        output.push_str("/>");
    }
    if let Some(size) = props.font_size_half_points {
        push_val(output, "w:sz", &size.to_string());
    }
    output.push_str("</w:rPr>");
}

fn write_image(output: &mut String, image: &InlineImage) {
    output.push_str("<w:drawing><wp:inline><wp:extent");
    push_attr(output, "cx", &image.width_emu.to_string());
    push_attr(output, "cy", &image.height_emu.to_string());
    output.push_str("/><wp:docPr id="1" name="Picture"");
    if let Some(alt) = &image.alt_text {
        push_attr(output, "descr", alt);
    }
    output.push_str("/><a:graphic><a:graphicData><pic:pic><pic:blipFill><a:blip");
    push_attr(output, "r:embed", &image.relationship_id);
    output.push_str("/><a:stretch><a:fillRect/></a:stretch></pic:blipFill><pic:spPr/></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing>");
}

fn write_table(output: &mut String, table: &Table) {
    output.push_str("<w:tbl><w:tblPr>");
    if let Some(width) = table.properties.width_twips {
        output.push_str("<w:tblW");
        push_attr(output, "w:w", &width.to_string());
        push_attr(output, "w:type", "dxa");
        output.push_str("/>");
    }
    if let Some(style) = &table.properties.style_id {
        push_val(output, "w:tblStyle", style);
    }
    output.push_str("</w:tblPr>");

    for row in &table.rows {
        output.push_str("<w:tr>");
        if let Some(height) = row.exact_height_twips {
            output.push_str("<w:trPr><w:trHeight");
            push_attr(output, "w:val", &height.to_string());
            output.push_str("/></w:trPr>");
        }
        for cell in &row.cells {
            output.push_str("<w:tc><w:tcPr>");
            if let Some(width) = cell.properties.width_twips {
                output.push_str("<w:tcW");
                push_attr(output, "w:w", &width.to_string());
                push_attr(output, "w:type", "dxa");
                output.push_str("/>");
            }
            if cell.properties.grid_span > 1 {
                push_val(output, "w:gridSpan", &cell.properties.grid_span.to_string());
            }
            if cell.properties.vertical_merge {
                output.push_str("<w:vMerge/>");
            }
            output.push_str("</w:tcPr>");
            for block in &cell.blocks {
                write_block(output, block);
            }
            output.push_str("</w:tc>");
        }
        output.push_str("</w:tr>");
    }

    output.push_str("</w:tbl>");
}

fn write_section_properties(output: &mut String, section: &SectionProperties) {
    output.push_str("<w:sectPr>");
    if let Some(header) = &section.header_default {
        output.push_str("<w:headerReference w:type="default"");
        push_attr(output, "r:id", header);
        output.push_str("/>");
    }
    if let Some(footer) = &section.footer_default {
        output.push_str("<w:footerReference w:type="default"");
        push_attr(output, "r:id", footer);
        output.push_str("/>");
    }
    output.push_str("<w:pgSz");
    push_attr(output, "w:w", &section.page_width_twips.to_string());
    push_attr(output, "w:h", &section.page_height_twips.to_string());
    if section.orientation == Orientation::Landscape {
        push_attr(output, "w:orient", "landscape");
    }
    output.push_str("/><w:pgMar");
    push_attr(output, "w:top", &section.margins.top_twips.to_string());
    push_attr(output, "w:right", &section.margins.right_twips.to_string());
    push_attr(output, "w:bottom", &section.margins.bottom_twips.to_string());
    push_attr(output, "w:left", &section.margins.left_twips.to_string());
    push_attr(output, "w:header", &section.margins.header_twips.to_string());
    push_attr(output, "w:footer", &section.margins.footer_twips.to_string());
    output.push_str("/></w:sectPr>");
}

fn write_styles_xml(styles: &StyleSheet) -> Vec<u8> {
    let mut output = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#,
    );
    for style in styles.styles.values() {
        output.push_str("<w:style");
        push_attr(
            &mut output,
            "w:type",
            match style.kind {
                StyleKind::Paragraph => "paragraph",
                StyleKind::Character => "character",
                StyleKind::Table => "table",
            },
        );
        push_attr(&mut output, "w:styleId", &style.id);
        output.push('>');
        push_val(&mut output, "w:name", &style.name);
        if let Some(based_on) = &style.based_on {
            push_val(&mut output, "w:basedOn", based_on);
        }
        write_paragraph_properties(&mut output, &style.paragraph);
        write_run_properties(&mut output, &style.run);
        output.push_str("</w:style>");
    }
    output.push_str("</w:styles>");
    output.into_bytes()
}

fn write_numbering_xml(numbering: &Numbering) -> Vec<u8> {
    let mut output = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#,
    );
    for list in numbering.abstract_lists.values() {
        output.push_str("<w:abstractNum");
        push_attr(&mut output, "w:abstractNumId", &list.id.to_string());
        output.push('>');
        for level in list.levels.values() {
            output.push_str("<w:lvl");
            push_attr(&mut output, "w:ilvl", &level.level.to_string());
            output.push('>');
            push_val(&mut output, "w:start", &level.start.to_string());
            push_val(
                &mut output,
                "w:numFmt",
                match level.format {
                    ListFormat::Decimal => "decimal",
                    ListFormat::LowerLetter => "lowerLetter",
                    ListFormat::UpperLetter => "upperLetter",
                    ListFormat::LowerRoman => "lowerRoman",
                    ListFormat::UpperRoman => "upperRoman",
                    ListFormat::Bullet => "bullet",
                    ListFormat::None => "none",
                },
            );
            push_val(&mut output, "w:lvlText", &level.text);
            output.push_str("</w:lvl>");
        }
        output.push_str("</w:abstractNum>");
    }
    for instance in numbering.instances.values() {
        output.push_str("<w:num");
        push_attr(&mut output, "w:numId", &instance.id.to_string());
        output.push('>');
        push_val(
            &mut output,
            "w:abstractNumId",
            &instance.abstract_id.to_string(),
        );
        output.push_str("</w:num>");
    }
    output.push_str("</w:numbering>");
    output.into_bytes()
}

fn push_val(output: &mut String, element: &str, value: &str) {
    output.push('<');
    output.push_str(element);
    push_attr(output, "w:val", value);
    output.push_str("/>");
}

fn push_attr(output: &mut String, name: &str, value: &str) {
    output.push(' ');
    output.push_str(name);
    output.push_str("="");
    escape_attribute(output, value);
    output.push('"');
}

fn escape_text(output: &mut String, value: &str) {
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            other => output.push(other),
        }
    }
}

fn escape_attribute(output: &mut String, value: &str) {
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            other => output.push(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn blank_docx_saves_and_reopens() {
        let mut document = DocxDocument::blank();
        document.document_mut().paragraph_mut(0).unwrap().runs =
            vec![Run::text("Hello Nexa")];

        let bytes = save_docx(&mut document, Cursor::new(Vec::new()))
            .unwrap()
            .into_inner();
        let reopened = open_docx(Cursor::new(bytes)).unwrap();

        assert_eq!(reopened.document().plain_text(), "Hello Nexa");
    }

    #[test]
    fn formatting_table_numbering_and_section_round_trip() {
        let mut document = DocxDocument::blank();
        document.document_mut().blocks = vec![
            Block::Paragraph(Paragraph {
                runs: vec![Run {
                    content: RunContent::Text("Title".into()),
                    properties: RunProperties {
                        bold: true,
                        font_size_half_points: Some(32),
                        ..RunProperties::default()
                    },
                }],
                properties: ParagraphProperties {
                    alignment: Alignment::Center,
                    ..ParagraphProperties::default()
                },
            }),
            Block::Table(Table {
                rows: vec![TableRow {
                    cells: vec![TableCell {
                        blocks: vec![Block::Paragraph(Paragraph {
                            runs: vec![Run::text("Cell")],
                            properties: ParagraphProperties::default(),
                        })],
                        properties: CellProperties {
                            width_twips: Some(2400),
                            grid_span: 1,
                            vertical_merge: false,
                            shading: None,
                        },
                    }],
                    exact_height_twips: None,
                }],
                properties: TableProperties::default(),
            }),
        ];
        document.document_mut().sections[0].properties.orientation = Orientation::Landscape;

        let bytes = save_docx(&mut document, Cursor::new(Vec::new()))
            .unwrap()
            .into_inner();
        let reopened = open_docx(Cursor::new(bytes)).unwrap();

        assert_eq!(reopened.document().paragraph(0).unwrap().plain_text(), "Title");
        assert!(reopened.document().paragraph(0).unwrap().runs[0].properties.bold);
        assert_eq!(
            reopened.document().paragraph(0).unwrap().properties.alignment,
            Alignment::Center
        );
        assert_eq!(
            reopened.document().sections[0].properties.orientation,
            Orientation::Landscape
        );
        assert!(matches!(reopened.document().blocks[1], Block::Table(_)));
    }

    #[test]
    fn unknown_destructive_construct_blocks_save() {
        let mut blank = DocxDocument::blank();
        let mut bytes = save_docx(&mut blank, Cursor::new(Vec::new()))
            .unwrap()
            .into_inner();
        let needle = b"<w:p>";
        let replacement = b"<w:bookmarkStart/><w:p>";
        if let Some(position) = bytes.windows(needle.len()).position(|window| window == needle) {
            bytes.splice(position..position + needle.len(), replacement.iter().copied());
        }

        if let Ok(mut opened) = open_docx(Cursor::new(bytes)) {
            assert!(!opened.document().compatibility.can_save());
            assert!(matches!(
                save_docx(&mut opened, Cursor::new(Vec::new())),
                Err(DocxError::SaveBlocked(_))
            ));
        }
    }
}
