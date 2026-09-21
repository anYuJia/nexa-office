use crate::{
    Image, Presentation, PresentationError, Rect, RgbColor, Shape, ShapeKind, Slide, SlideElement,
    Table, TableCell, TextBox, TextStyle,
};
use nexa_ooxml::{
    AtomicSaveError, ContentTypeMap, ContentTypeRule, LazyZipPackage, OfficePackageKind, Package,
    PackageError, PartName, Relationship, RelationshipId, RelationshipSet, RelationshipTarget,
    ZipPackageError, save_package_atomic, write_owned_package,
};
use quick_xml::{events::Event, reader::Reader};
use std::{
    collections::BTreeMap,
    error::Error,
    fmt,
    io::{Read, Seek, Write},
    path::Path,
};

const EMU_PER_INCH: f64 = 914_400.0;

const PRESENTATION_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml";
const SLIDE_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.slide+xml";
const MASTER_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml";
const LAYOUT_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml";
const THEME_CONTENT_TYPE: &str = "application/vnd.openxmlformats-officedocument.theme+xml";

const OFFICE_DOCUMENT_REL: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";
const SLIDE_REL: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide";
const MASTER_REL: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster";
const LAYOUT_REL: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout";
const THEME_REL: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme";

#[derive(Debug, Clone)]
pub struct PptxPresentation {
    presentation: Presentation,
    package: Package,
    presentation_part: PartName,
    slide_parts: Vec<PartName>,
    slide_relationship_ids: Vec<String>,
    compatibility_issues: Vec<String>,
}

impl PptxPresentation {
    #[must_use]
    pub fn blank() -> Self {
        let presentation_part =
            PartName::new("/ppt/presentation.xml").expect("constant presentation part");
        let slide_part = PartName::new("/ppt/slides/slide1.xml").expect("constant slide part");
        let master_part =
            PartName::new("/ppt/slideMasters/slideMaster1.xml").expect("constant master part");
        let layout_part =
            PartName::new("/ppt/slideLayouts/slideLayout1.xml").expect("constant layout part");
        let theme_part = PartName::new("/ppt/theme/theme1.xml").expect("constant theme part");

        let mut content_types = ContentTypeMap::default();
        content_types.insert(ContentTypeRule::Default {
            extension: "xml".into(),
            content_type: "application/xml".into(),
        });
        content_types.insert(ContentTypeRule::Default {
            extension: "rels".into(),
            content_type: "application/vnd.openxmlformats-package.relationships+xml".into(),
        });
        for (part, content_type) in [
            (&presentation_part, PRESENTATION_CONTENT_TYPE),
            (&slide_part, SLIDE_CONTENT_TYPE),
            (&master_part, MASTER_CONTENT_TYPE),
            (&layout_part, LAYOUT_CONTENT_TYPE),
            (&theme_part, THEME_CONTENT_TYPE),
        ] {
            content_types.insert(ContentTypeRule::Override {
                part_name: part.clone(),
                content_type: content_type.into(),
            });
        }

        let mut package = Package::new(content_types);
        for part in [
            &presentation_part,
            &slide_part,
            &master_part,
            &layout_part,
            &theme_part,
        ] {
            package
                .insert_part(part.clone(), Vec::new())
                .expect("content type exists");
        }

        package
            .package_relationships_mut()
            .insert(Relationship {
                id: RelationshipId::new("rId1"),
                relationship_type: OFFICE_DOCUMENT_REL.into(),
                target: RelationshipTarget::Internal(presentation_part.clone()),
            })
            .expect("root relationship");

        {
            let relationships = package.relationships_mut(presentation_part.clone());
            relationships
                .insert(Relationship {
                    id: RelationshipId::new("rIdMaster1"),
                    relationship_type: MASTER_REL.into(),
                    target: RelationshipTarget::Internal(master_part.clone()),
                })
                .expect("master relationship");
            relationships
                .insert(Relationship {
                    id: RelationshipId::new("rIdSlide1"),
                    relationship_type: SLIDE_REL.into(),
                    target: RelationshipTarget::Internal(slide_part.clone()),
                })
                .expect("slide relationship");
        }

        package
            .relationships_mut(slide_part.clone())
            .insert(Relationship {
                id: RelationshipId::new("rIdLayout"),
                relationship_type: LAYOUT_REL.into(),
                target: RelationshipTarget::Internal(layout_part.clone()),
            })
            .expect("layout relationship");

        {
            let relationships = package.relationships_mut(master_part.clone());
            relationships
                .insert(Relationship {
                    id: RelationshipId::new("rIdLayout1"),
                    relationship_type: LAYOUT_REL.into(),
                    target: RelationshipTarget::Internal(layout_part.clone()),
                })
                .expect("master layout relationship");
            relationships
                .insert(Relationship {
                    id: RelationshipId::new("rIdTheme1"),
                    relationship_type: THEME_REL.into(),
                    target: RelationshipTarget::Internal(theme_part.clone()),
                })
                .expect("theme relationship");
        }

        package
            .relationships_mut(layout_part.clone())
            .insert(Relationship {
                id: RelationshipId::new("rIdMaster"),
                relationship_type: MASTER_REL.into(),
                target: RelationshipTarget::Internal(master_part.clone()),
            })
            .expect("layout master relationship");

        let mut result = Self {
            presentation: Presentation::blank(),
            package,
            presentation_part,
            slide_parts: vec![slide_part],
            slide_relationship_ids: vec!["rIdSlide1".into()],
            compatibility_issues: Vec::new(),
        };
        result
            .package
            .replace_part(&master_part, write_master_xml())
            .expect("blank master");
        result
            .package
            .replace_part(&layout_part, write_layout_xml())
            .expect("blank layout");
        result
            .package
            .replace_part(&theme_part, write_theme_xml())
            .expect("blank theme");
        result.sync_package().expect("blank PPTX serialization");
        result
    }

    #[must_use]
    pub fn presentation(&self) -> &Presentation {
        &self.presentation
    }

    pub fn presentation_mut(&mut self) -> &mut Presentation {
        &mut self.presentation
    }

    #[must_use]
    pub fn package(&self) -> &Package {
        &self.package
    }

    #[must_use]
    pub fn compatibility_issues(&self) -> &[String] {
        &self.compatibility_issues
    }

    #[must_use]
    pub fn can_save(&self) -> bool {
        self.compatibility_issues.is_empty()
    }

    fn sync_package(&mut self) -> Result<(), PptxError> {
        if !self.can_save() {
            return Err(PptxError::SaveBlocked(self.compatibility_issues.len()));
        }
        self.ensure_slide_parts()?;
        self.package.replace_part(
            &self.presentation_part,
            write_presentation_xml(
                &self.presentation,
                &self.slide_relationship_ids,
            ),
        )?;
        for (index, slide) in self.presentation.slides().iter().enumerate() {
            let part = self
                .slide_parts
                .get(index)
                .ok_or(PptxError::MissingSlide(index))?;
            self.package.replace_part(part, write_slide_xml(slide))?;
        }
        Ok(())
    }

    fn ensure_slide_parts(&mut self) -> Result<(), PptxError> {
        while self.slide_parts.len() < self.presentation.slides().len() {
            let index = self.slide_parts.len();
            let part = PartName::new(format!("/ppt/slides/slide{}.xml", index + 1))?;
            self.package
                .content_types_mut()
                .insert(ContentTypeRule::Override {
                    part_name: part.clone(),
                    content_type: SLIDE_CONTENT_TYPE.into(),
                });
            if self.package.part(&part).is_none() {
                self.package.insert_part(part.clone(), Vec::new())?;
            }
            let relationship_id = format!("rIdSlide{}", index + 1);
            self.package
                .relationships_mut(self.presentation_part.clone())
                .insert(Relationship {
                    id: RelationshipId::new(&relationship_id),
                    relationship_type: SLIDE_REL.into(),
                    target: RelationshipTarget::Internal(part.clone()),
                })?;
            if let Some(layout_target) = first_relationship_target_by_suffix(
                self.package
                    .relationships(&self.slide_parts[0])
                    .cloned()
                    .unwrap_or_default(),
                "/slideLayout",
            ) {
                self.package
                    .relationships_mut(part.clone())
                    .insert(Relationship {
                        id: RelationshipId::new("rIdLayout"),
                        relationship_type: LAYOUT_REL.into(),
                        target: RelationshipTarget::Internal(layout_target),
                    })?;
            }
            self.slide_parts.push(part);
            self.slide_relationship_ids.push(relationship_id);
        }
        Ok(())
    }
}

impl Default for PptxPresentation {
    fn default() -> Self {
        Self::blank()
    }
}

#[derive(Debug)]
pub enum PptxError {
    Zip(ZipPackageError),
    AtomicSave(AtomicSaveError),
    Package(PackageError),
    PartName(nexa_ooxml::PartNameError),
    Presentation(PresentationError),
    WrongOfficeKind,
    MissingPresentationPart,
    MissingSlide(usize),
    MissingRelationship(String),
    Xml(String),
    SaveBlocked(usize),
}

impl fmt::Display for PptxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zip(error) => write!(f, "{error}"),
            Self::AtomicSave(error) => write!(f, "{error}"),
            Self::Package(error) => write!(f, "{error}"),
            Self::PartName(error) => write!(f, "{error}"),
            Self::Presentation(error) => write!(f, "{error}"),
            Self::WrongOfficeKind => f.write_str("package is not a PPTX presentation"),
            Self::MissingPresentationPart => f.write_str("PPTX presentation part is missing"),
            Self::MissingSlide(index) => write!(f, "PPTX slide is missing: {index}"),
            Self::MissingRelationship(id) => write!(f, "PPTX relationship target is missing: {id}"),
            Self::Xml(message) => write!(f, "PresentationML error: {message}"),
            Self::SaveBlocked(count) => write!(
                f,
                "saving is blocked because {count} unsupported presentation construct(s) could be lost"
            ),
        }
    }
}

impl Error for PptxError {}

impl From<ZipPackageError> for PptxError {
    fn from(value: ZipPackageError) -> Self {
        Self::Zip(value)
    }
}

impl From<AtomicSaveError> for PptxError {
    fn from(value: AtomicSaveError) -> Self {
        Self::AtomicSave(value)
    }
}

impl From<PackageError> for PptxError {
    fn from(value: PackageError) -> Self {
        Self::Package(value)
    }
}

impl From<nexa_ooxml::PartNameError> for PptxError {
    fn from(value: nexa_ooxml::PartNameError) -> Self {
        Self::PartName(value)
    }
}

impl From<PresentationError> for PptxError {
    fn from(value: PresentationError) -> Self {
        Self::Presentation(value)
    }
}

pub fn open_pptx<R: Read + Seek>(reader: R) -> Result<PptxPresentation, PptxError> {
    let mut lazy = LazyZipPackage::open(reader)?;
    let info = lazy
        .office_package_info()
        .map_err(|error| PptxError::Xml(error.to_string()))?;
    if info.kind() != OfficePackageKind::Presentation {
        return Err(PptxError::WrongOfficeKind);
    }

    let presentation_part = info.main_part().clone();
    let package = lazy.load_owned_package()?;
    let presentation_bytes = package
        .part(&presentation_part)
        .ok_or(PptxError::MissingPresentationPart)?
        .bytes();

    let relationships = package
        .relationships(&presentation_part)
        .cloned()
        .unwrap_or_default();
    let (slide_ids, width_inches, height_inches) = parse_presentation_xml(presentation_bytes)?;

    let mut slides = Vec::with_capacity(slide_ids.len().max(1));
    let mut slide_parts = Vec::with_capacity(slide_ids.len());
    let mut compatibility_issues = Vec::new();

    for (index, relationship_id) in slide_ids.iter().enumerate() {
        let target = relationship_target(&relationships, relationship_id)
            .ok_or_else(|| PptxError::MissingRelationship(relationship_id.clone()))?;
        let bytes = package
            .part(&target)
            .ok_or(PptxError::MissingSlide(index))?
            .bytes();
        let slide_relationships = package.relationships(&target).cloned().unwrap_or_default();
        let (slide, issues) = parse_slide_xml(bytes, &slide_relationships, index)?;
        slides.push(slide);
        slide_parts.push(target);
        compatibility_issues.extend(issues);
    }

    if slides.is_empty() {
        slides.push(Slide::blank(0));
    }
    let mut presentation = Presentation::blank();
    presentation.replace_slides(slides)?;
    presentation.width_inches = width_inches;
    presentation.height_inches = height_inches;
    presentation.set_active_slide(0)?;

    Ok(PptxPresentation {
        presentation,
        package,
        presentation_part,
        slide_parts,
        slide_relationship_ids: slide_ids,
        compatibility_issues,
    })
}

pub fn save_pptx<W: Write + Seek>(
    presentation: &mut PptxPresentation,
    writer: W,
) -> Result<W, PptxError> {
    presentation.sync_package()?;
    write_owned_package(&presentation.package, writer).map_err(PptxError::Zip)
}

pub fn save_pptx_atomic(
    presentation: &mut PptxPresentation,
    destination: &Path,
) -> Result<(), PptxError> {
    presentation.sync_package()?;
    save_package_atomic(&presentation.package, destination)?;
    Ok(())
}

fn parse_presentation_xml(bytes: &[u8]) -> Result<(Vec<String>, f64, f64), PptxError> {
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(true);
    let mut slide_ids = Vec::new();
    let mut width = 13.333;
    let mut height = 7.5;

    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) | Ok(Event::Empty(event)) => {
                let event_name = event.name();
                let name = local_name(event_name.as_ref());
                if name == b"sldId" {
                    for attribute in event.attributes().flatten() {
                        if local_name(attribute.key.as_ref()) == b"id" {
                            let value = String::from_utf8_lossy(attribute.value.as_ref()).into_owned();
                            if value.starts_with("rId") {
                                slide_ids.push(value);
                            }
                        }
                    }
                } else if name == b"sldSz" {
                    let mut cx = None;
                    let mut cy = None;
                    for attribute in event.attributes().flatten() {
                        match local_name(attribute.key.as_ref()) {
                            b"cx" => cx = parse_f64(attribute.value.as_ref()),
                            b"cy" => cy = parse_f64(attribute.value.as_ref()),
                            _ => {}
                        }
                    }
                    if let (Some(cx), Some(cy)) = (cx, cy) {
                        width = cx / EMU_PER_INCH;
                        height = cy / EMU_PER_INCH;
                    }
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => return Err(PptxError::Xml(error.to_string())),
        }
    }
    Ok((slide_ids, width, height))
}

fn parse_slide_xml(
    bytes: &[u8],
    relationships: &RelationshipSet,
    index: usize,
) -> Result<(Slide, Vec<String>), PptxError> {
    let xml = String::from_utf8_lossy(bytes);
    let mut issues = Vec::new();
    for marker in [
        "<p:cxnSp", "<p:grpSp", "<p:contentPart", "<p:oleObj", "<p:transition",
        "<p:timing", "<c:chart", "<a:videoFile", "<a:audioFile",
    ] {
        if xml.contains(marker) {
            issues.push(format!("slide {} contains unsupported {}", index + 1, marker));
        }
    }

    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().trim_text(false);
    let mut slide = Slide::blank(index);
    let mut current_kind: Option<&'static str> = None;
    let mut text = String::new();
    let mut bounds = Rect::default();
    let mut shape_kind = ShapeKind::Rectangle;
    let mut image_relationship = String::new();
    let mut image_alt = String::new();
    let mut in_text = false;
    let mut table_depth = 0_usize;
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut current_cell = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(event)) => {
                let event_name = event.name();
                let name = local_name(event_name.as_ref());
                match name {
                    b"sp" => {
                        current_kind = Some("shape");
                        text.clear();
                        bounds = Rect::default();
                        shape_kind = ShapeKind::Rectangle;
                    }
                    b"pic" => {
                        current_kind = Some("image");
                        bounds = Rect::default();
                        image_relationship.clear();
                        image_alt.clear();
                    }
                    b"graphicFrame" => {
                        current_kind = Some("table");
                        table_depth = 0;
                        table_rows.clear();
                        bounds = Rect::default();
                    }
                    b"tbl" if current_kind == Some("table") => table_depth += 1,
                    b"tr" if table_depth > 0 => current_row.clear(),
                    b"tc" if table_depth > 0 => current_cell.clear(),
                    b"t" => in_text = true,
                    b"off" => {
                        let (x, y) = parse_xy(&event);
                        if let Some(x) = x {
                            bounds.x = x / EMU_PER_INCH;
                        }
                        if let Some(y) = y {
                            bounds.y = y / EMU_PER_INCH;
                        }
                    }
                    b"ext" => {
                        let (cx, cy) = parse_cxcy(&event);
                        if let Some(cx) = cx {
                            bounds.width = cx / EMU_PER_INCH;
                        }
                        if let Some(cy) = cy {
                            bounds.height = cy / EMU_PER_INCH;
                        }
                    }
                    b"prstGeom" => {
                        for attribute in event.attributes().flatten() {
                            if local_name(attribute.key.as_ref()) == b"prst" {
                                shape_kind = match attribute.value.as_ref() {
                                    b"roundRect" => ShapeKind::RoundedRectangle,
                                    b"ellipse" => ShapeKind::Ellipse,
                                    b"line" => ShapeKind::Line,
                                    _ => ShapeKind::Rectangle,
                                };
                            }
                        }
                    }
                    b"blip" => {
                        for attribute in event.attributes().flatten() {
                            if local_name(attribute.key.as_ref()) == b"embed" {
                                image_relationship =
                                    String::from_utf8_lossy(attribute.value.as_ref()).into_owned();
                            }
                        }
                    }
                    b"cNvPr" if current_kind == Some("image") => {
                        for attribute in event.attributes().flatten() {
                            if local_name(attribute.key.as_ref()) == b"descr" {
                                image_alt =
                                    String::from_utf8_lossy(attribute.value.as_ref()).into_owned();
                            }
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(event)) => {
                let event_name = event.name();
                let name = local_name(event_name.as_ref());
                if name == b"off" {
                    let (x, y) = parse_xy(&event);
                    if let Some(x) = x {
                        bounds.x = x / EMU_PER_INCH;
                    }
                    if let Some(y) = y {
                        bounds.y = y / EMU_PER_INCH;
                    }
                } else if name == b"ext" {
                    let (cx, cy) = parse_cxcy(&event);
                    if let Some(cx) = cx {
                        bounds.width = cx / EMU_PER_INCH;
                    }
                    if let Some(cy) = cy {
                        bounds.height = cy / EMU_PER_INCH;
                    }
                } else if name == b"blip" {
                    for attribute in event.attributes().flatten() {
                        if local_name(attribute.key.as_ref()) == b"embed" {
                            image_relationship =
                                String::from_utf8_lossy(attribute.value.as_ref()).into_owned();
                        }
                    }
                }
            }
            Ok(Event::Text(event)) if in_text => {
                let value = event.decode().map_err(|error| PptxError::Xml(error.to_string()))?;
                if table_depth > 0 {
                    current_cell.push_str(&value);
                } else {
                    text.push_str(&value);
                }
            }
            Ok(Event::End(event)) => {
                let event_name = event.name();
                let name = local_name(event_name.as_ref());
                match name {
                    b"t" => in_text = false,
                    b"tc" if table_depth > 0 => current_row.push(current_cell.clone()),
                    b"tr" if table_depth > 0 => table_rows.push(current_row.clone()),
                    b"tbl" if table_depth > 0 => table_depth = table_depth.saturating_sub(1),
                    b"sp" if current_kind == Some("shape") => {
                        let is_text_only = xml.contains("<p:txBody") && text.len() > 0;
                        if is_text_only && shape_kind == ShapeKind::Rectangle {
                            slide.elements.push(SlideElement::TextBox(TextBox {
                                bounds,
                                text: text.clone(),
                                style: TextStyle::default(),
                            }));
                        } else {
                            slide.elements.push(SlideElement::Shape(Shape {
                                bounds,
                                kind: shape_kind,
                                fill: None,
                                line: None,
                                text: text.clone(),
                                text_style: TextStyle::default(),
                            }));
                        }
                        current_kind = None;
                    }
                    b"pic" if current_kind == Some("image") => {
                        if !image_relationship.is_empty()
                            && relationship_target(relationships, &image_relationship).is_some()
                        {
                            slide.elements.push(SlideElement::Image(Image {
                                bounds,
                                relationship_id: image_relationship.clone(),
                                alt_text: image_alt.clone(),
                            }));
                        } else {
                            issues.push(format!(
                                "slide {} image relationship could not be resolved",
                                index + 1
                            ));
                        }
                        current_kind = None;
                    }
                    b"graphicFrame" if current_kind == Some("table") => {
                        if !table_rows.is_empty() {
                            let rows = table_rows.len();
                            let columns = table_rows.iter().map(Vec::len).max().unwrap_or(1);
                            let mut cells = Vec::with_capacity(rows * columns);
                            for row in &table_rows {
                                for column in 0..columns {
                                    cells.push(TableCell {
                                        text: row.get(column).cloned().unwrap_or_default(),
                                        bold: false,
                                    });
                                }
                            }
                            slide.elements.push(SlideElement::Table(Table {
                                bounds,
                                rows,
                                columns,
                                cells,
                            }));
                        } else {
                            issues.push(format!("slide {} has unsupported graphic frame", index + 1));
                        }
                        current_kind = None;
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => return Err(PptxError::Xml(error.to_string())),
        }
    }

    Ok((slide, issues))
}

fn write_presentation_xml(presentation: &Presentation, relationship_ids: &[String]) -> Vec<u8> {
    let slide_ids = relationship_ids
        .iter()
        .enumerate()
        .map(|(index, relationship_id)| {
            format!(
                r#"<p:sldId id="{}" r:id="{}"/>"#,
                256 + index,
                escape_xml(relationship_id)
            )
        })
        .collect::<String>();
    let cx = (presentation.width_inches * EMU_PER_INCH).round() as u64;
    let cy = (presentation.height_inches * EMU_PER_INCH).round() as u64;
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:sldMasterIdLst><p:sldMasterId id="2147483648" r:id="rIdMaster1"/></p:sldMasterIdLst>
<p:sldIdLst>{slide_ids}</p:sldIdLst>
<p:sldSz cx="{cx}" cy="{cy}" type="screen16x9"/>
<p:notesSz cx="6858000" cy="9144000"/>
</p:presentation>"#
    )
    .into_bytes()
}

fn write_slide_xml(slide: &Slide) -> Vec<u8> {
    let elements = slide
        .elements
        .iter()
        .enumerate()
        .map(|(index, element)| write_element_xml(index + 2, element))
        .collect::<String>();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
<p:cSld><p:spTree>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
<p:grpSpPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/><a:chOff x="0" y="0"/><a:chExt cx="0" cy="0"/></a:xfrm></p:grpSpPr>
{elements}
</p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>
</p:sld>"#
    )
    .into_bytes()
}

fn write_element_xml(id: usize, element: &SlideElement) -> String {
    match element {
        SlideElement::TextBox(value) => {
            let (x, y, cx, cy) = rect_emu(value.bounds);
            format!(
                r#"<p:sp><p:nvSpPr><p:cNvPr id="{id}" name="Text {id}"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom><a:noFill/><a:ln><a:noFill/></a:ln></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang="en-US" sz="{}" b="{}" i="{}"><a:solidFill><a:srgbClr val="{}"/></a:solidFill></a:rPr><a:t>{}</a:t></a:r><a:endParaRPr/></a:p></p:txBody></p:sp>"#,
                (value.style.font_size_pt * 100.0).round() as u64,
                if value.style.bold { 1 } else { 0 },
                if value.style.italic { 1 } else { 0 },
                value.style.color.to_hex(),
                escape_xml(&value.text),
            )
        }
        SlideElement::Shape(value) => {
            let (x, y, cx, cy) = rect_emu(value.bounds);
            let preset = match value.kind {
                ShapeKind::Rectangle => "rect",
                ShapeKind::RoundedRectangle => "roundRect",
                ShapeKind::Ellipse => "ellipse",
                ShapeKind::Line => "line",
            };
            let fill = value.fill.map_or_else(
                || "<a:noFill/>".into(),
                |color| format!("<a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill>", color.to_hex()),
            );
            let line = value.line.map_or_else(
                || "<a:ln><a:noFill/></a:ln>".into(),
                |color| format!("<a:ln><a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill></a:ln>", color.to_hex()),
            );
            format!(
                r#"<p:sp><p:nvSpPr><p:cNvPr id="{id}" name="Shape {id}"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm><a:prstGeom prst="{preset}"><a:avLst/></a:prstGeom>{fill}{line}</p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang="en-US" sz="{}"/><a:t>{}</a:t></a:r><a:endParaRPr/></a:p></p:txBody></p:sp>"#,
                (value.text_style.font_size_pt * 100.0).round() as u64,
                escape_xml(&value.text),
            )
        }
        SlideElement::Image(value) => {
            let (x, y, cx, cy) = rect_emu(value.bounds);
            format!(
                r#"<p:pic><p:nvPicPr><p:cNvPr id="{id}" name="Image {id}" descr="{}"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed="{}"/><a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr><a:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="{cx}" cy="{cy}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr></p:pic>"#,
                escape_xml(&value.alt_text),
                escape_xml(&value.relationship_id),
            )
        }
        SlideElement::Table(value) => write_table_xml(id, value),
    }
}

fn write_table_xml(id: usize, table: &Table) -> String {
    let (x, y, cx, cy) = rect_emu(table.bounds);
    let grid_width = if table.columns == 0 {
        cx
    } else {
        cx / table.columns as u64
    };
    let grid = (0..table.columns)
        .map(|_| format!(r#"<a:gridCol w="{grid_width}"/>"#))
        .collect::<String>();
    let row_height = if table.rows == 0 {
        cy
    } else {
        cy / table.rows as u64
    };
    let rows = (0..table.rows)
        .map(|row| {
            let cells = (0..table.columns)
                .map(|column| {
                    let cell = table.cell(row, column);
                    let text = cell.map_or("", |cell| cell.text.as_str());
                    let bold = cell.is_some_and(|cell| cell.bold);
                    format!(
                        r#"<a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr b="{}"/><a:t>{}</a:t></a:r><a:endParaRPr/></a:p></a:txBody><a:tcPr/></a:tc>"#,
                        if bold { 1 } else { 0 },
                        escape_xml(text)
                    )
                })
                .collect::<String>();
            format!(r#"<a:tr h="{row_height}">{cells}</a:tr>"#)
        })
        .collect::<String>();

    format!(
        r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="{id}" name="Table {id}"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="{x}" y="{y}"/><a:ext cx="{cx}" cy="{cy}"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr firstRow="1" bandRow="1"/><a:tblGrid>{grid}</a:tblGrid>{rows}</a:tbl></a:graphicData></a:graphic></p:graphicFrame>"#
    )
}

fn write_master_xml() -> Vec<u8> {
    br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld><p:sldLayoutIdLst><p:sldLayoutId id="1" r:id="rIdLayout1"/></p:sldLayoutIdLst><p:txStyles><p:titleStyle/><p:bodyStyle/><p:otherStyle/></p:txStyles></p:sldMaster>"#.to_vec()
}

fn write_layout_xml() -> Vec<u8> {
    br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" type="blank" preserve="1"><p:cSld name="Blank"><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:sldLayout>"#.to_vec()
}

fn write_theme_xml() -> Vec<u8> {
    br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Nexa"><a:themeElements><a:clrScheme name="Nexa"><a:dk1><a:srgbClr val="101828"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="344054"/></a:dk2><a:lt2><a:srgbClr val="F2F4F7"/></a:lt2><a:accent1><a:srgbClr val="2563EB"/></a:accent1><a:accent2><a:srgbClr val="7A70FF"/></a:accent2><a:accent3><a:srgbClr val="12B76A"/></a:accent3><a:accent4><a:srgbClr val="F79009"/></a:accent4><a:accent5><a:srgbClr val="6172F3"/></a:accent5><a:accent6><a:srgbClr val="EE46BC"/></a:accent6><a:hlink><a:srgbClr val="175CD3"/></a:hlink><a:folHlink><a:srgbClr val="6941C6"/></a:folHlink></a:clrScheme><a:fontScheme name="Nexa"><a:majorFont><a:latin typeface="Aptos Display"/><a:ea typeface=""/><a:cs typeface=""/></a:majorFont><a:minorFont><a:latin typeface="Aptos"/><a:ea typeface=""/><a:cs typeface=""/></a:minorFont></a:fontScheme><a:fmtScheme name="Nexa"><a:fillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:fillStyleLst><a:lnStyleLst><a:ln w="9525"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln></a:lnStyleLst><a:effectStyleLst><a:effectStyle><a:effectLst/></a:effectStyle></a:effectStyleLst><a:bgFillStyleLst><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:bgFillStyleLst></a:fmtScheme></a:themeElements></a:theme>"#.to_vec()
}

fn relationship_target(relationships: &RelationshipSet, id: &str) -> Option<PartName> {
    relationships.iter().find_map(|relationship| {
        if relationship.id.as_str() != id {
            return None;
        }
        match &relationship.target {
            RelationshipTarget::Internal(target) => Some(target.clone()),
            RelationshipTarget::External(_) => None,
        }
    })
}

fn first_relationship_target_by_suffix(
    relationships: RelationshipSet,
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

fn parse_xy(event: &quick_xml::events::BytesStart<'_>) -> (Option<f64>, Option<f64>) {
    let mut x = None;
    let mut y = None;
    for attribute in event.attributes().flatten() {
        match local_name(attribute.key.as_ref()) {
            b"x" => x = parse_f64(attribute.value.as_ref()),
            b"y" => y = parse_f64(attribute.value.as_ref()),
            _ => {}
        }
    }
    (x, y)
}

fn parse_cxcy(event: &quick_xml::events::BytesStart<'_>) -> (Option<f64>, Option<f64>) {
    let mut cx = None;
    let mut cy = None;
    for attribute in event.attributes().flatten() {
        match local_name(attribute.key.as_ref()) {
            b"cx" => cx = parse_f64(attribute.value.as_ref()),
            b"cy" => cy = parse_f64(attribute.value.as_ref()),
            _ => {}
        }
    }
    (cx, cy)
}

fn parse_f64(bytes: &[u8]) -> Option<f64> {
    std::str::from_utf8(bytes).ok()?.parse().ok()
}

fn local_name(name: &[u8]) -> &[u8] {
    name.rsplit(|byte| *byte == b':').next().unwrap_or(name)
}

fn rect_emu(rect: Rect) -> (u64, u64, u64, u64) {
    (
        (rect.x.max(0.0) * EMU_PER_INCH).round() as u64,
        (rect.y.max(0.0) * EMU_PER_INCH).round() as u64,
        (rect.width.max(0.01) * EMU_PER_INCH).round() as u64,
        (rect.height.max(0.01) * EMU_PER_INCH).round() as u64,
    )
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn blank_round_trip_preserves_slide_text_and_table() {
        let mut pptx = PptxPresentation::blank();
        let slide = pptx.presentation_mut().slide_mut(0).unwrap();
        slide.add_text_box("Nexa Slides");
        let table_index = slide.add_table(2, 2).unwrap();
        let SlideElement::Table(table) = &mut slide.elements[table_index] else {
            panic!("table expected");
        };
        table.cell_mut(1, 1).unwrap().text = "42".into();

        let cursor = save_pptx(&mut pptx, Cursor::new(Vec::new())).unwrap();
        let reopened = open_pptx(Cursor::new(cursor.into_inner())).unwrap();

        assert_eq!(reopened.presentation().slides().len(), 1);
        let slide = reopened.presentation().slide(0).unwrap();
        assert!(slide.elements.iter().any(|element| element.text().contains("Nexa Slides")));
        assert!(slide.elements.iter().any(|element| element.text().contains("42")));
    }

    #[test]
    fn adding_slides_materializes_parts_on_save() {
        let mut pptx = PptxPresentation::blank();
        pptx.presentation_mut().add_slide();
        pptx.presentation_mut().add_slide();
        let cursor = save_pptx(&mut pptx, Cursor::new(Vec::new())).unwrap();
        let reopened = open_pptx(Cursor::new(cursor.into_inner())).unwrap();
        assert_eq!(reopened.presentation().slides().len(), 3);
    }

    #[test]
    fn xml_escape_covers_text_content() {
        assert_eq!(escape_xml("A&B<\"x\">"), "A&amp;B&lt;&quot;x&quot;&gt;");
    }
}
