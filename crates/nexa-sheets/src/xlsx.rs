use crate::{
    Cell, CellAddress, CellAlignment, CellFormat, CellRange, CellValue, FreezePane, Workbook,
    WorkbookError, Worksheet,
};
use nexa_ooxml::{
    AtomicSaveError, ContentTypeMap, ContentTypeRule, InteropReport, LazyZipPackage,
    OfficePackageKind, Package, PackageError, PartName, Relationship, RelationshipId,
    RelationshipSet, RelationshipTarget, ZipPackageError, audit_package_rewrite_risks,
    save_package_atomic, write_owned_package,
};
use quick_xml::{
    XmlVersion,
    events::{BytesStart, Event},
    reader::Reader,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
    io::{Read, Seek, Write},
    path::Path,
};

const WORKBOOK_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml";
const WORKSHEET_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml";
const STYLES_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml";

const OFFICE_DOCUMENT_REL: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";
const WORKSHEET_REL: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet";
const STYLES_REL: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles";
const SHARED_STRINGS_REL: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings";

#[derive(Debug, Clone)]
pub struct XlsxWorkbook {
    workbook: Workbook,
    package: Package,
    workbook_part: PartName,
    sheet_parts: Vec<PartName>,
    sheet_relationship_ids: Vec<String>,
    styles_part: PartName,
    compatibility_issues: Vec<String>,
    interop_report: InteropReport,
}

impl XlsxWorkbook {
    #[must_use]
    pub fn blank() -> Self {
        let workbook_part = PartName::new("/xl/workbook.xml").expect("constant part");
        let sheet_part = PartName::new("/xl/worksheets/sheet1.xml").expect("constant part");
        let styles_part = PartName::new("/xl/styles.xml").expect("constant part");

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
            part_name: workbook_part.clone(),
            content_type: WORKBOOK_CONTENT_TYPE.into(),
        });
        content_types.insert(ContentTypeRule::Override {
            part_name: sheet_part.clone(),
            content_type: WORKSHEET_CONTENT_TYPE.into(),
        });
        content_types.insert(ContentTypeRule::Override {
            part_name: styles_part.clone(),
            content_type: STYLES_CONTENT_TYPE.into(),
        });

        let mut package = Package::new(content_types);
        package
            .insert_part(workbook_part.clone(), Vec::new())
            .expect("workbook content type exists");
        package
            .insert_part(sheet_part.clone(), Vec::new())
            .expect("worksheet content type exists");
        package
            .insert_part(styles_part.clone(), Vec::new())
            .expect("styles content type exists");

        package
            .package_relationships_mut()
            .insert(Relationship {
                id: RelationshipId::new("rId1"),
                relationship_type: OFFICE_DOCUMENT_REL.into(),
                target: RelationshipTarget::Internal(workbook_part.clone()),
            })
            .expect("unique package relationship");

        let workbook_relationships = package.relationships_mut(workbook_part.clone());
        workbook_relationships
            .insert(Relationship {
                id: RelationshipId::new("rIdSheet1"),
                relationship_type: WORKSHEET_REL.into(),
                target: RelationshipTarget::Internal(sheet_part.clone()),
            })
            .expect("unique sheet relationship");
        workbook_relationships
            .insert(Relationship {
                id: RelationshipId::new("rIdStyles"),
                relationship_type: STYLES_REL.into(),
                target: RelationshipTarget::Internal(styles_part.clone()),
            })
            .expect("unique styles relationship");

        let mut result = Self {
            workbook: Workbook::blank(),
            package,
            workbook_part,
            sheet_parts: vec![sheet_part],
            sheet_relationship_ids: vec!["rIdSheet1".into()],
            styles_part,
            compatibility_issues: Vec::new(),
            interop_report: InteropReport::default(),
        };
        result.sync_package().expect("blank XLSX serialization");
        result
    }

    #[must_use]
    pub fn workbook(&self) -> &Workbook {
        &self.workbook
    }

    pub fn workbook_mut(&mut self) -> &mut Workbook {
        &mut self.workbook
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
    pub fn interop_report(&self) -> &InteropReport {
        &self.interop_report
    }

    #[must_use]
    pub fn can_save(&self) -> bool {
        self.compatibility_issues.is_empty() && self.interop_report.can_rewrite_safely()
    }

    #[must_use]
    pub fn compatibility_issue_count(&self) -> usize {
        self.compatibility_issues.len() + self.interop_report.blocker_count()
    }

    fn sync_package(&mut self) -> Result<(), XlsxError> {
        if !self.can_save() {
            return Err(XlsxError::SaveBlocked(self.compatibility_issue_count()));
        }

        self.ensure_sheet_parts()?;
        let (styles, style_ids) = collect_styles(&self.workbook);

        self.package.replace_part(
            &self.workbook_part,
            write_workbook_xml(&self.workbook, &self.sheet_relationship_ids),
        )?;
        self.package
            .replace_part(&self.styles_part, write_styles_xml(&styles))?;

        for (index, sheet) in self.workbook.sheets().iter().enumerate() {
            let part = self
                .sheet_parts
                .get(index)
                .ok_or(XlsxError::MissingWorksheet(index))?;
            self.package
                .replace_part(part, write_worksheet_xml(sheet, &style_ids))?;
        }
        Ok(())
    }

    fn ensure_sheet_parts(&mut self) -> Result<(), XlsxError> {
        while self.sheet_parts.len() < self.workbook.sheets().len() {
            let index = self.sheet_parts.len();
            let part = PartName::new(format!("/xl/worksheets/sheet{}.xml", index + 1))?;
            self.package
                .content_types_mut()
                .insert(ContentTypeRule::Override {
                    part_name: part.clone(),
                    content_type: WORKSHEET_CONTENT_TYPE.into(),
                });
            if self.package.part(&part).is_none() {
                self.package.insert_part(part.clone(), Vec::new())?;
            }

            let relationship_id = next_sheet_relationship_id(
                self.package
                    .relationships(&self.workbook_part)
                    .cloned()
                    .unwrap_or_default(),
                index + 1,
            );
            self.package
                .relationships_mut(self.workbook_part.clone())
                .insert(Relationship {
                    id: RelationshipId::new(&relationship_id),
                    relationship_type: WORKSHEET_REL.into(),
                    target: RelationshipTarget::Internal(part.clone()),
                })?;

            self.sheet_parts.push(part);
            self.sheet_relationship_ids.push(relationship_id);
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum XlsxError {
    Zip(ZipPackageError),
    AtomicSave(AtomicSaveError),
    Package(PackageError),
    PartName(nexa_ooxml::PartNameError),
    Workbook(WorkbookError),
    WrongOfficeKind,
    MissingWorkbookPart,
    MissingWorksheet(usize),
    MissingRelationship(String),
    Xml(String),
    SaveBlocked(usize),
}

impl fmt::Display for XlsxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zip(error) => write!(f, "{error}"),
            Self::AtomicSave(error) => write!(f, "{error}"),
            Self::Package(error) => write!(f, "{error}"),
            Self::PartName(error) => write!(f, "{error}"),
            Self::Workbook(error) => write!(f, "{error}"),
            Self::WrongOfficeKind => f.write_str("package is not an XLSX workbook"),
            Self::MissingWorkbookPart => f.write_str("XLSX workbook part is missing"),
            Self::MissingWorksheet(index) => write!(f, "XLSX worksheet is missing: {index}"),
            Self::MissingRelationship(id) => {
                write!(f, "XLSX relationship target is missing: {id}")
            }
            Self::Xml(message) => write!(f, "SpreadsheetML error: {message}"),
            Self::SaveBlocked(count) => write!(
                f,
                "saving is blocked because {count} unsupported spreadsheet construct(s) could be lost"
            ),
        }
    }
}

impl Error for XlsxError {}

impl From<ZipPackageError> for XlsxError {
    fn from(value: ZipPackageError) -> Self {
        Self::Zip(value)
    }
}
impl From<AtomicSaveError> for XlsxError {
    fn from(value: AtomicSaveError) -> Self {
        Self::AtomicSave(value)
    }
}
impl From<PackageError> for XlsxError {
    fn from(value: PackageError) -> Self {
        Self::Package(value)
    }
}
impl From<nexa_ooxml::PartNameError> for XlsxError {
    fn from(value: nexa_ooxml::PartNameError) -> Self {
        Self::PartName(value)
    }
}
impl From<WorkbookError> for XlsxError {
    fn from(value: WorkbookError) -> Self {
        Self::Workbook(value)
    }
}

pub fn open_xlsx<R: Read + Seek>(reader: R) -> Result<XlsxWorkbook, XlsxError> {
    let mut lazy = LazyZipPackage::open(reader)?;
    let info = lazy
        .office_package_info()
        .map_err(|error| XlsxError::Xml(error.to_string()))?;
    if info.kind() != OfficePackageKind::Workbook {
        return Err(XlsxError::WrongOfficeKind);
    }

    let workbook_part = info.main_part().clone();
    let package = lazy.load_owned_package()?;
    let workbook_bytes = package
        .part(&workbook_part)
        .ok_or(XlsxError::MissingWorkbookPart)?
        .bytes();

    let relationships = package
        .relationships(&workbook_part)
        .cloned()
        .unwrap_or_default();

    let mut issues = Vec::new();
    let workbook_sheets = parse_workbook_xml(workbook_bytes, &mut issues)?;

    let styles_part = relationship_target_by_type(&relationships, STYLES_REL)
        .unwrap_or_else(|| PartName::new("/xl/styles.xml").expect("constant part"));
    let styles = if let Some(part) = package.part(&styles_part) {
        parse_styles_xml(part.bytes(), &mut issues)?
    } else {
        vec![CellFormat::default()]
    };

    let shared_strings =
        if let Some(part_name) = relationship_target_by_type(&relationships, SHARED_STRINGS_REL) {
            package
                .part(&part_name)
                .map(|part| parse_shared_strings_xml(part.bytes()))
                .transpose()?
                .unwrap_or_default()
        } else {
            Vec::new()
        };

    let mut sheets = Vec::new();
    let mut sheet_parts = Vec::new();
    let mut sheet_relationship_ids = Vec::new();

    for (name, relationship_id) in workbook_sheets {
        let target = relationship_target_by_id(&relationships, &relationship_id)
            .ok_or_else(|| XlsxError::MissingRelationship(relationship_id.clone()))?;
        let bytes = package
            .part(&target)
            .ok_or_else(|| XlsxError::MissingRelationship(relationship_id.clone()))?
            .bytes();
        sheets.push(parse_worksheet_xml(
            bytes,
            name,
            &shared_strings,
            &styles,
            &mut issues,
        )?);
        sheet_parts.push(target);
        sheet_relationship_ids.push(relationship_id);
    }

    if sheets.is_empty() {
        return Err(XlsxError::Xml("workbook contains no worksheets".into()));
    }

    let mut workbook = Workbook::new(sheets)?;
    workbook.recalculate();

    for relationship in relationships.iter() {
        let known = relationship.relationship_type == WORKSHEET_REL
            || relationship.relationship_type == STYLES_REL
            || relationship.relationship_type == SHARED_STRINGS_REL
            || relationship.relationship_type.ends_with("/theme");
        if !known {
            push_issue(
                &mut issues,
                format!(
                    "unsupported workbook relationship: {}",
                    relationship.relationship_type
                ),
            );
        }
    }

    let interop_report = audit_package_rewrite_risks(&package);

    Ok(XlsxWorkbook {
        workbook,
        package,
        workbook_part,
        sheet_parts,
        sheet_relationship_ids,
        styles_part,
        compatibility_issues: issues,
        interop_report,
    })
}

pub fn save_xlsx<W: Write + Seek>(workbook: &mut XlsxWorkbook, writer: W) -> Result<W, XlsxError> {
    workbook.sync_package()?;
    write_owned_package(&workbook.package, writer).map_err(XlsxError::Zip)
}

pub fn save_xlsx_atomic(workbook: &mut XlsxWorkbook, destination: &Path) -> Result<(), XlsxError> {
    workbook.sync_package()?;
    save_package_atomic(&workbook.package, destination)?;
    Ok(())
}

fn parse_workbook_xml(
    input: &[u8],
    issues: &mut Vec<String>,
) -> Result<Vec<(String, String)>, XlsxError> {
    let mut reader = Reader::from_reader(input);
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut sheets = Vec::new();
    let mut root_seen = false;

    loop {
        let event = reader
            .read_event_into(&mut buffer)
            .map_err(|error| XlsxError::Xml(error.to_string()))?;
        match event {
            Event::Start(element) | Event::Empty(element) => {
                let name = local_name(&element);
                if !root_seen {
                    if name != "workbook" {
                        return Err(XlsxError::Xml(format!(
                            "unexpected workbook root element: {name}"
                        )));
                    }
                    root_seen = true;
                    buffer.clear();
                    continue;
                }
                match name.as_str() {
                    "sheet" => {
                        let sheet_name = attr(&element, "name").unwrap_or_else(|| "Sheet".into());
                        let relationship_id = attr(&element, "id").ok_or_else(|| {
                            XlsxError::Xml("worksheet is missing relationship id".into())
                        })?;
                        sheets.push((sheet_name, relationship_id));
                    }
                    "fileVersion" | "workbookPr" | "bookViews" | "workbookView" | "sheets"
                    | "calcPr" | "extLst" => {}
                    "definedNames" | "definedName" | "externalReferences" | "externalReference"
                    | "pivotCaches" | "pivotCache" => {
                        push_issue(issues, format!("unsupported workbook element: {name}"));
                    }
                    _ => {}
                }
            }
            Event::DocType(_) => {
                return Err(XlsxError::Xml(
                    "DOCTYPE is not allowed in SpreadsheetML workbook".into(),
                ));
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }
    Ok(sheets)
}

#[derive(Default)]
struct ParsedCell {
    address: Option<CellAddress>,
    cell_type: String,
    style_index: usize,
    formula: Option<String>,
    value: String,
    inline_text: String,
}

fn parse_worksheet_xml(
    input: &[u8],
    sheet_name: String,
    shared_strings: &[String],
    styles: &[CellFormat],
    issues: &mut Vec<String>,
) -> Result<Worksheet, XlsxError> {
    let mut reader = Reader::from_reader(input);
    reader.config_mut().trim_text(false);
    let mut buffer = Vec::new();
    let mut sheet = Worksheet::new(sheet_name)?;
    let mut current_cell: Option<ParsedCell> = None;
    let mut in_formula = false;
    let mut in_value = false;
    let mut in_inline_text = false;
    let mut root_seen = false;

    loop {
        let event = reader
            .read_event_into(&mut buffer)
            .map_err(|error| XlsxError::Xml(error.to_string()))?;
        match event {
            Event::Start(element) => {
                let name = local_name(&element);
                if !root_seen {
                    if name != "worksheet" {
                        return Err(XlsxError::Xml(format!(
                            "unexpected worksheet root element: {name}"
                        )));
                    }
                    root_seen = true;
                    buffer.clear();
                    continue;
                }
                match name.as_str() {
                    "c" => {
                        let address = attr(&element, "r")
                            .map(|value| CellAddress::parse_a1(&value))
                            .transpose()?;
                        current_cell = Some(ParsedCell {
                            address,
                            cell_type: attr(&element, "t").unwrap_or_default(),
                            style_index: attr(&element, "s")
                                .and_then(|value| value.parse().ok())
                                .unwrap_or(0),
                            ..ParsedCell::default()
                        });
                    }
                    "f" => {
                        if attr(&element, "t").is_some_and(|kind| kind != "normal") {
                            push_issue(
                                issues,
                                "shared/array/data-table formulas are not safely editable".into(),
                            );
                        }
                        in_formula = true;
                    }
                    "v" => in_value = true,
                    "t" if current_cell.is_some() => in_inline_text = true,
                    "row" => apply_row_metadata(&element, &mut sheet),
                    "col" => apply_column_metadata(&element, &mut sheet),
                    "mergeCell" => apply_merge(&element, &mut sheet, issues),
                    "pane" => apply_pane(&element, &mut sheet),
                    "autoFilter" => apply_auto_filter(&element, &mut sheet),
                    "worksheet" | "dimension" | "sheetViews" | "sheetView" | "selection"
                    | "sheetFormatPr" | "cols" | "sheetData" | "is" | "mergeCells"
                    | "pageMargins" | "pageSetup" | "printOptions" | "headerFooter" | "sheetPr"
                    | "outlinePr" | "extLst" => {}
                    "conditionalFormatting"
                    | "dataValidations"
                    | "dataValidation"
                    | "drawing"
                    | "legacyDrawing"
                    | "hyperlinks"
                    | "hyperlink"
                    | "tableParts"
                    | "tablePart"
                    | "sheetProtection"
                    | "oleObjects"
                    | "controls" => {
                        push_issue(issues, format!("unsupported worksheet element: {name}"));
                    }
                    _ => {}
                }
            }
            Event::Empty(element) => {
                let name = local_name(&element);
                match name.as_str() {
                    "c" => {
                        if let Some(address) =
                            attr(&element, "r").and_then(|value| CellAddress::parse_a1(&value).ok())
                        {
                            let style_index = attr(&element, "s")
                                .and_then(|value| value.parse().ok())
                                .unwrap_or(0);
                            let format = styles.get(style_index).cloned().unwrap_or_else(|| {
                                push_issue(
                                    issues,
                                    format!("unknown cell style index: {style_index}"),
                                );
                                CellFormat::default()
                            });
                            sheet.set_cell(
                                address,
                                Cell {
                                    format,
                                    ..Cell::default()
                                },
                            );
                        }
                    }
                    "row" => apply_row_metadata(&element, &mut sheet),
                    "col" => apply_column_metadata(&element, &mut sheet),
                    "mergeCell" => apply_merge(&element, &mut sheet, issues),
                    "pane" => apply_pane(&element, &mut sheet),
                    "autoFilter" => apply_auto_filter(&element, &mut sheet),
                    "dimension" | "selection" | "pageMargins" | "pageSetup" | "printOptions"
                    | "sheetFormatPr" => {}
                    "conditionalFormatting"
                    | "dataValidations"
                    | "dataValidation"
                    | "drawing"
                    | "legacyDrawing"
                    | "hyperlinks"
                    | "hyperlink"
                    | "tableParts"
                    | "tablePart"
                    | "sheetProtection"
                    | "oleObjects"
                    | "controls" => {
                        push_issue(issues, format!("unsupported worksheet element: {name}"));
                    }
                    _ => {}
                }
            }
            Event::Text(text) => {
                let decoded = text.xml_content(XmlVersion::Implicit1_0).into_owned();
                if let Some(cell) = &mut current_cell {
                    if in_formula {
                        cell.formula.push_or_insert(decoded);
                    } else if in_value {
                        cell.value.push_str(&decoded);
                    } else if in_inline_text {
                        cell.inline_text.push_str(&decoded);
                    }
                }
            }
            Event::End(element) => match element.local_name().as_ref() {
                "f" => in_formula = false,
                "v" => in_value = false,
                "t" => in_inline_text = false,
                "c" => {
                    if let Some(parsed) = current_cell.take()
                        && let Some(address) = parsed.address
                    {
                        let format = styles.get(parsed.style_index).cloned().unwrap_or_else(|| {
                            push_issue(
                                issues,
                                format!("unknown cell style index: {}", parsed.style_index),
                            );
                            CellFormat::default()
                        });
                        let mut cell = parsed_cell_value(parsed, shared_strings, issues);
                        cell.format = format;
                        sheet.set_cell(address, cell);
                    }
                }
                _ => {}
            },
            Event::DocType(_) => {
                return Err(XlsxError::Xml(
                    "DOCTYPE is not allowed in SpreadsheetML worksheet".into(),
                ));
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }

    Ok(sheet)
}

trait OptionStringExt {
    fn push_or_insert(&mut self, value: String);
}

impl OptionStringExt for Option<String> {
    fn push_or_insert(&mut self, value: String) {
        if let Some(existing) = self {
            existing.push_str(&value);
        } else {
            *self = Some(value);
        }
    }
}

fn parsed_cell_value(
    parsed: ParsedCell,
    shared_strings: &[String],
    issues: &mut Vec<String>,
) -> Cell {
    let cached_number = parsed
        .formula
        .as_ref()
        .and_then(|_| parsed.value.parse::<f64>().ok());

    let value = match parsed.cell_type.as_str() {
        "s" => parsed
            .value
            .parse::<usize>()
            .ok()
            .and_then(|index| shared_strings.get(index))
            .cloned()
            .map(CellValue::Text)
            .unwrap_or_else(|| {
                push_issue(issues, "invalid shared-string cell reference".into());
                CellValue::Empty
            }),
        "inlineStr" => CellValue::Text(parsed.inline_text),
        "b" => CellValue::Bool(parsed.value == "1" || parsed.value.eq_ignore_ascii_case("true")),
        "str" => CellValue::Text(parsed.value),
        "e" => CellValue::Error(parsed.value),
        _ => {
            if parsed.value.is_empty() {
                CellValue::Empty
            } else if let Ok(number) = parsed.value.parse::<f64>() {
                CellValue::Number(number)
            } else {
                CellValue::Text(parsed.value)
            }
        }
    };

    Cell {
        value,
        formula: parsed
            .formula
            .map(|value| value.trim_start_matches('=').to_owned()),
        cached_number,
        format: CellFormat::default(),
    }
}

fn parse_shared_strings_xml(input: &[u8]) -> Result<Vec<String>, XlsxError> {
    let mut reader = Reader::from_reader(input);
    reader.config_mut().trim_text(false);
    let mut buffer = Vec::new();
    let mut values = Vec::new();
    let mut current = String::new();
    let mut in_item = false;
    let mut in_text = false;

    loop {
        match reader
            .read_event_into(&mut buffer)
            .map_err(|error| XlsxError::Xml(error.to_string()))?
        {
            Event::Start(element) => match local_name(&element).as_str() {
                "si" => {
                    current.clear();
                    in_item = true;
                }
                "t" if in_item => in_text = true,
                _ => {}
            },
            Event::Text(text) if in_text => {
                current.push_str(&text.xml_content(XmlVersion::Implicit1_0));
            }
            Event::End(element) => match element.local_name().as_ref() {
                "t" => in_text = false,
                "si" => {
                    values.push(current.clone());
                    in_item = false;
                }
                _ => {}
            },
            Event::DocType(_) => {
                return Err(XlsxError::Xml(
                    "DOCTYPE is not allowed in SpreadsheetML shared strings".into(),
                ));
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }
    Ok(values)
}

#[derive(Debug, Clone, Copy, Default)]
struct ParsedFont {
    bold: bool,
    italic: bool,
}

fn parse_styles_xml(input: &[u8], issues: &mut Vec<String>) -> Result<Vec<CellFormat>, XlsxError> {
    let mut reader = Reader::from_reader(input);
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut fonts = Vec::new();
    let mut fills: Vec<Option<String>> = Vec::new();
    let mut formats = Vec::new();

    let mut in_fonts = false;
    let mut in_font = false;
    let mut current_font = ParsedFont::default();
    let mut in_fills = false;
    let mut in_fill = false;
    let mut current_fill = None;
    let mut in_cell_xfs = false;
    let mut current_xf: Option<(usize, usize, CellAlignment, u32)> = None;

    loop {
        match reader
            .read_event_into(&mut buffer)
            .map_err(|error| XlsxError::Xml(error.to_string()))?
        {
            Event::Start(element) => {
                let name = local_name(&element);
                match name.as_str() {
                    "fonts" => in_fonts = true,
                    "font" if in_fonts => {
                        in_font = true;
                        current_font = ParsedFont::default();
                    }
                    "fills" => in_fills = true,
                    "fill" if in_fills => {
                        in_fill = true;
                        current_fill = None;
                    }
                    "fgColor" if in_fill => {
                        current_fill = attr(&element, "rgb").and_then(normalize_rgb);
                    }
                    "cellXfs" => in_cell_xfs = true,
                    "xf" if in_cell_xfs => {
                        current_xf = Some((
                            attr(&element, "fontId")
                                .and_then(|value| value.parse().ok())
                                .unwrap_or(0),
                            attr(&element, "fillId")
                                .and_then(|value| value.parse().ok())
                                .unwrap_or(0),
                            CellAlignment::General,
                            attr(&element, "numFmtId")
                                .and_then(|value| value.parse().ok())
                                .unwrap_or(0),
                        ));
                    }
                    "alignment" if current_xf.is_some() => {
                        if let Some((_, _, alignment, _)) = &mut current_xf {
                            *alignment = match attr(&element, "horizontal").as_deref() {
                                Some("left") => CellAlignment::Left,
                                Some("center") => CellAlignment::Center,
                                Some("right") => CellAlignment::Right,
                                _ => CellAlignment::General,
                            };
                        }
                    }
                    "b" if in_font => current_font.bold = true,
                    "i" if in_font => current_font.italic = true,
                    _ => {}
                }
            }
            Event::Empty(element) => {
                let name = local_name(&element);
                match name.as_str() {
                    "b" if in_font => current_font.bold = true,
                    "i" if in_font => current_font.italic = true,
                    "fgColor" if in_fill => {
                        current_fill = attr(&element, "rgb").and_then(normalize_rgb);
                    }
                    "alignment" if current_xf.is_some() => {
                        if let Some((_, _, alignment, _)) = &mut current_xf {
                            *alignment = match attr(&element, "horizontal").as_deref() {
                                Some("left") => CellAlignment::Left,
                                Some("center") => CellAlignment::Center,
                                Some("right") => CellAlignment::Right,
                                _ => CellAlignment::General,
                            };
                        }
                    }
                    "xf" if in_cell_xfs => {
                        let font_id = attr(&element, "fontId")
                            .and_then(|value| value.parse().ok())
                            .unwrap_or(0);
                        let fill_id = attr(&element, "fillId")
                            .and_then(|value| value.parse().ok())
                            .unwrap_or(0);
                        let num_fmt_id = attr(&element, "numFmtId")
                            .and_then(|value| value.parse().ok())
                            .unwrap_or(0);
                        formats.push(build_format(
                            font_id,
                            fill_id,
                            CellAlignment::General,
                            num_fmt_id,
                            &fonts,
                            &fills,
                            issues,
                        ));
                    }
                    _ => {}
                }
            }
            Event::End(element) => match element.local_name().as_ref() {
                "font" if in_font => {
                    fonts.push(current_font);
                    in_font = false;
                }
                "fonts" => in_fonts = false,
                "fill" if in_fill => {
                    fills.push(current_fill.take());
                    in_fill = false;
                }
                "fills" => in_fills = false,
                "xf" if in_cell_xfs => {
                    if let Some((font_id, fill_id, alignment, num_fmt_id)) = current_xf.take() {
                        formats.push(build_format(
                            font_id, fill_id, alignment, num_fmt_id, &fonts, &fills, issues,
                        ));
                    }
                }
                "cellXfs" => in_cell_xfs = false,
                _ => {}
            },
            Event::DocType(_) => {
                return Err(XlsxError::Xml(
                    "DOCTYPE is not allowed in SpreadsheetML styles".into(),
                ));
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }

    if formats.is_empty() {
        formats.push(CellFormat::default());
    }
    Ok(formats)
}

fn build_format(
    font_id: usize,
    fill_id: usize,
    alignment: CellAlignment,
    num_fmt_id: u32,
    fonts: &[ParsedFont],
    fills: &[Option<String>],
    issues: &mut Vec<String>,
) -> CellFormat {
    if num_fmt_id != 0 {
        push_issue(
            issues,
            format!("number format {num_fmt_id} is displayed but not safely editable"),
        );
    }
    let font = fonts.get(font_id).copied().unwrap_or_default();
    CellFormat {
        bold: font.bold,
        italic: font.italic,
        fill_rgb: fills.get(fill_id).cloned().flatten(),
        alignment,
    }
}

fn apply_row_metadata(element: &BytesStart<'_>, sheet: &mut Worksheet) {
    let Some(row) = attr(element, "r")
        .and_then(|value| value.parse::<u32>().ok())
        .and_then(|value| value.checked_sub(1))
    else {
        return;
    };
    if let Some(height) = attr(element, "ht").and_then(|value| value.parse().ok()) {
        let _ = sheet.set_row_height(row, Some(height));
    }
    if attr(element, "hidden").is_some_and(|value| value == "1" || value == "true") {
        sheet.set_hidden_row(row, true);
    }
}

fn apply_column_metadata(element: &BytesStart<'_>, sheet: &mut Worksheet) {
    let Some(min) = attr(element, "min")
        .and_then(|value| value.parse::<u32>().ok())
        .and_then(|value| value.checked_sub(1))
    else {
        return;
    };
    let max = attr(element, "max")
        .and_then(|value| value.parse::<u32>().ok())
        .and_then(|value| value.checked_sub(1))
        .unwrap_or(min);
    let Some(width) = attr(element, "width").and_then(|value| value.parse().ok()) else {
        return;
    };
    for column in min..=max.min(crate::MAX_COLUMNS - 1) {
        let _ = sheet.set_column_width(column, Some(width));
    }
}

fn apply_merge(element: &BytesStart<'_>, sheet: &mut Worksheet, issues: &mut Vec<String>) {
    if let Some(range) = attr(element, "ref").and_then(|value| CellRange::parse_a1(&value).ok())
        && sheet.merge(range).is_err()
    {
        push_issue(
            issues,
            format!("overlapping merged range: {}", range.to_a1()),
        );
    }
}

fn apply_pane(element: &BytesStart<'_>, sheet: &mut Worksheet) {
    if !attr(element, "state").is_some_and(|value| value == "frozen" || value == "frozenSplit") {
        return;
    }
    let columns = attr(element, "xSplit")
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.0) as u32;
    let rows = attr(element, "ySplit")
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.0) as u32;
    sheet.set_freeze_pane(Some(FreezePane { rows, columns }));
}

fn apply_auto_filter(element: &BytesStart<'_>, sheet: &mut Worksheet) {
    if let Some(range) = attr(element, "ref").and_then(|value| CellRange::parse_a1(&value).ok()) {
        sheet.set_auto_filter(Some(range));
    }
}

fn collect_styles(workbook: &Workbook) -> (Vec<CellFormat>, BTreeMap<CellFormat, u32>) {
    let mut unique = BTreeSet::new();
    unique.insert(CellFormat::default());
    for sheet in workbook.sheets() {
        for cell in sheet.cells().values() {
            unique.insert(cell.format.clone());
        }
    }

    let mut styles = vec![CellFormat::default()];
    styles.extend(
        unique
            .into_iter()
            .filter(|format| *format != CellFormat::default()),
    );

    let ids = styles
        .iter()
        .cloned()
        .enumerate()
        .map(|(index, format)| (format, index as u32))
        .collect();
    (styles, ids)
}

fn write_workbook_xml(workbook: &Workbook, relationship_ids: &[String]) -> Vec<u8> {
    let mut output = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><bookViews><workbookView activeTab=""#,
    );
    output.push_str(&workbook.active_sheet().to_string());
    output.push_str(r#""/></bookViews><sheets>"#);
    for (index, sheet) in workbook.sheets().iter().enumerate() {
        output.push_str("<sheet");
        push_attr(&mut output, "name", sheet.name());
        push_attr(&mut output, "sheetId", &(index + 1).to_string());
        push_attr(
            &mut output,
            "r:id",
            relationship_ids
                .get(index)
                .map_or("rIdMissing", String::as_str),
        );
        output.push_str("/>");
    }
    output.push_str(r#"</sheets><calcPr calcMode="auto" fullCalcOnLoad="1"/></workbook>"#);
    output.into_bytes()
}

fn write_worksheet_xml(sheet: &Worksheet, style_ids: &BTreeMap<CellFormat, u32>) -> Vec<u8> {
    let mut output = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
    );
    let dimension = sheet
        .used_range()
        .map_or_else(|| "A1".into(), CellRange::to_a1);
    output.push_str("<dimension");
    push_attr(&mut output, "ref", &dimension);
    output.push_str("/>");

    if let Some(pane) = sheet.freeze_pane() {
        output.push_str(r#"<sheetViews><sheetView workbookViewId="0"><pane"#);
        if pane.columns > 0 {
            push_attr(&mut output, "xSplit", &pane.columns.to_string());
        }
        if pane.rows > 0 {
            push_attr(&mut output, "ySplit", &pane.rows.to_string());
        }
        let top_left = CellAddress {
            row: pane.rows,
            column: pane.columns,
        }
        .to_a1();
        push_attr(&mut output, "topLeftCell", &top_left);
        push_attr(&mut output, "state", "frozen");
        output.push_str(r#" activePane="bottomRight"/></sheetView></sheetViews>"#);
    } else {
        output.push_str(r#"<sheetViews><sheetView workbookViewId="0"/></sheetViews>"#);
    }
    output.push_str(r#"<sheetFormatPr defaultRowHeight="15"/>"#);

    if !sheet.column_widths().is_empty() {
        output.push_str("<cols>");
        for (column, width) in sheet.column_widths() {
            output.push_str("<col");
            push_attr(&mut output, "min", &(column + 1).to_string());
            push_attr(&mut output, "max", &(column + 1).to_string());
            push_attr(&mut output, "width", &width.to_string());
            push_attr(&mut output, "customWidth", "1");
            output.push_str("/>");
        }
        output.push_str("</cols>");
    }

    let mut rows = BTreeSet::new();
    for address in sheet.cells().keys() {
        rows.insert(address.row);
    }
    rows.extend(sheet.row_heights().keys().copied());
    rows.extend(sheet.hidden_rows().iter().copied());

    output.push_str("<sheetData>");
    for row in rows {
        output.push_str("<row");
        push_attr(&mut output, "r", &(row + 1).to_string());
        if let Some(height) = sheet.row_heights().get(&row) {
            push_attr(&mut output, "ht", &height.to_string());
            push_attr(&mut output, "customHeight", "1");
        }
        if sheet.hidden_rows().contains(&row) {
            push_attr(&mut output, "hidden", "1");
        }
        output.push('>');

        for (address, cell) in sheet.cells().range(
            CellAddress { row, column: 0 }..=CellAddress {
                row,
                column: crate::MAX_COLUMNS - 1,
            },
        ) {
            write_cell(&mut output, *address, cell, style_ids);
        }
        output.push_str("</row>");
    }
    output.push_str("</sheetData>");

    if !sheet.merged_ranges().is_empty() {
        output.push_str("<mergeCells");
        push_attr(
            &mut output,
            "count",
            &sheet.merged_ranges().len().to_string(),
        );
        output.push('>');
        for range in sheet.merged_ranges() {
            output.push_str("<mergeCell");
            push_attr(&mut output, "ref", &range.to_a1());
            output.push_str("/>");
        }
        output.push_str("</mergeCells>");
    }

    if let Some(filter) = sheet.auto_filter() {
        output.push_str("<autoFilter");
        push_attr(&mut output, "ref", &filter.to_a1());
        output.push_str("/>");
    }

    output.push_str(r#"<pageMargins left="0.7" right="0.7" top="0.75" bottom="0.75" header="0.3" footer="0.3"/></worksheet>"#);
    output.into_bytes()
}

fn write_cell(
    output: &mut String,
    address: CellAddress,
    cell: &Cell,
    style_ids: &BTreeMap<CellFormat, u32>,
) {
    if cell.is_semantically_empty() {
        return;
    }

    output.push_str("<c");
    push_attr(output, "r", &address.to_a1());
    let style_id = style_ids.get(&cell.format).copied().unwrap_or(0);
    if style_id != 0 {
        push_attr(output, "s", &style_id.to_string());
    }

    if let Some(formula) = &cell.formula {
        output.push('>');
        output.push_str("<f>");
        escape_text(output, formula.trim_start_matches('='));
        output.push_str("</f>");
        if let Some(cached) = cell.cached_number {
            output.push_str("<v>");
            output.push_str(&cached.to_string());
            output.push_str("</v>");
        }
        output.push_str("</c>");
        return;
    }

    match &cell.value {
        CellValue::Empty => output.push_str("/>"),
        CellValue::Number(value) => {
            output.push_str("><v>");
            output.push_str(&value.to_string());
            output.push_str("</v></c>");
        }
        CellValue::Bool(value) => {
            push_attr(output, "t", "b");
            output.push_str("><v>");
            output.push_str(if *value { "1" } else { "0" });
            output.push_str("</v></c>");
        }
        CellValue::Text(value) => {
            push_attr(output, "t", "inlineStr");
            output.push_str(r#"><is><t xml:space="preserve">"#);
            escape_text(output, value);
            output.push_str("</t></is></c>");
        }
        CellValue::Error(value) => {
            push_attr(output, "t", "e");
            output.push_str("><v>");
            escape_text(output, value);
            output.push_str("</v></c>");
        }
    }
}

fn write_styles_xml(styles: &[CellFormat]) -> Vec<u8> {
    let mut fills = Vec::<String>::new();
    for style in styles {
        if let Some(fill) = &style.fill_rgb
            && !fills.iter().any(|existing| existing == fill)
        {
            fills.push(fill.clone());
        }
    }

    let mut output = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
    );

    output.push_str("<fonts");
    push_attr(&mut output, "count", &styles.len().to_string());
    output.push('>');
    for style in styles {
        output.push_str("<font><sz val=\"11\"/><name val=\"Calibri\"/>");
        if style.bold {
            output.push_str("<b/>");
        }
        if style.italic {
            output.push_str("<i/>");
        }
        output.push_str("</font>");
    }
    output.push_str("</fonts>");

    output.push_str("<fills");
    push_attr(&mut output, "count", &(fills.len() + 2).to_string());
    output.push_str(r#"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill>"#);
    for fill in &fills {
        output.push_str(r#"<fill><patternFill patternType="solid"><fgColor"#);
        push_attr(
            &mut output,
            "rgb",
            &format!("FF{}", fill.trim_start_matches('#')),
        );
        output.push_str(r#"/><bgColor indexed="64"/></patternFill></fill>"#);
    }
    output.push_str("</fills>");

    output.push_str(r#"<borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>"#);

    output.push_str("<cellXfs");
    push_attr(&mut output, "count", &styles.len().to_string());
    output.push('>');
    for (index, style) in styles.iter().enumerate() {
        output.push_str("<xf");
        push_attr(&mut output, "numFmtId", "0");
        push_attr(&mut output, "fontId", &index.to_string());
        let fill_id = style
            .fill_rgb
            .as_ref()
            .and_then(|fill| fills.iter().position(|candidate| candidate == fill))
            .map_or(0, |position| position + 2);
        push_attr(&mut output, "fillId", &fill_id.to_string());
        push_attr(&mut output, "borderId", "0");
        push_attr(&mut output, "xfId", "0");
        if style != &CellFormat::default() {
            push_attr(&mut output, "applyFont", "1");
            if fill_id != 0 {
                push_attr(&mut output, "applyFill", "1");
            }
        }
        if style.alignment == CellAlignment::General {
            output.push_str("/>");
        } else {
            push_attr(&mut output, "applyAlignment", "1");
            output.push_str("><alignment");
            push_attr(
                &mut output,
                "horizontal",
                match style.alignment {
                    CellAlignment::General => "general",
                    CellAlignment::Left => "left",
                    CellAlignment::Center => "center",
                    CellAlignment::Right => "right",
                },
            );
            output.push_str("/></xf>");
        }
    }
    output.push_str("</cellXfs>");
    output.push_str(r#"<cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles><dxfs count="0"/><tableStyles count="0" defaultTableStyle="TableStyleMedium2" defaultPivotStyle="PivotStyleLight16"/></styleSheet>"#);
    output.into_bytes()
}

fn relationship_target_by_type(
    relationships: &RelationshipSet,
    relationship_type: &str,
) -> Option<PartName> {
    relationships.iter().find_map(|relationship| {
        if relationship.relationship_type != relationship_type {
            return None;
        }
        match &relationship.target {
            RelationshipTarget::Internal(target) => Some(target.clone()),
            RelationshipTarget::External(_) => None,
        }
    })
}

fn relationship_target_by_id(
    relationships: &RelationshipSet,
    relationship_id: &str,
) -> Option<PartName> {
    relationships
        .get(&RelationshipId::new(relationship_id))
        .and_then(|relationship| match &relationship.target {
            RelationshipTarget::Internal(target) => Some(target.clone()),
            RelationshipTarget::External(_) => None,
        })
}

fn next_sheet_relationship_id(relationships: RelationshipSet, hint: usize) -> String {
    for suffix in hint..hint + 100_000 {
        let candidate = format!("rIdNexaSheet{suffix}");
        if relationships
            .get(&RelationshipId::new(&candidate))
            .is_none()
        {
            return candidate;
        }
    }
    format!("rIdNexaSheet{hint}")
}

fn local_name(element: &BytesStart<'_>) -> String {
    element.local_name().as_ref().to_owned()
}

fn attr(element: &BytesStart<'_>, name: &str) -> Option<String> {
    for attribute in element.attributes().flatten() {
        if attribute.key.local_name().as_ref() == name
            && let Ok(value) = attribute.normalized_value(XmlVersion::Implicit1_0)
        {
            return Some(value.into_owned());
        }
    }
    None
}

fn normalize_rgb(value: String) -> Option<String> {
    let value = value.trim_start_matches('#');
    let value = if value.len() == 8 { &value[2..] } else { value };
    if value.len() == 6 && value.chars().all(|character| character.is_ascii_hexdigit()) {
        Some(value.to_ascii_uppercase())
    } else {
        None
    }
}

fn push_issue(issues: &mut Vec<String>, issue: String) {
    if !issues.iter().any(|existing| existing == &issue) {
        issues.push(issue);
    }
}

fn push_attr(output: &mut String, name: &str, value: &str) {
    output.push(' ');
    output.push_str(name);
    output.push_str("=\"");
    escape_attr(output, value);
    output.push('"');
}

fn escape_attr(output: &mut String, value: &str) {
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            _ => output.push(character),
        }
    }
}

fn escape_text(output: &mut String, value: &str) {
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            _ => output.push(character),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn blank_xlsx_round_trips_values_formulas_formatting_and_layout() {
        let mut xlsx = XlsxWorkbook::blank();
        let workbook = xlsx.workbook_mut();
        workbook
            .set_cell_input(0, CellAddress::parse_a1("A1").unwrap(), "收入")
            .unwrap();
        workbook
            .set_cell_input(0, CellAddress::parse_a1("B1").unwrap(), "10")
            .unwrap();
        workbook
            .set_cell_input(0, CellAddress::parse_a1("B2").unwrap(), "20")
            .unwrap();
        workbook
            .set_cell_input(0, CellAddress::parse_a1("B3").unwrap(), "=SUM(B1:B2)")
            .unwrap();

        let sheet = workbook.sheet_mut(0).unwrap();
        sheet.set_format(
            CellAddress::parse_a1("A1").unwrap(),
            CellFormat {
                bold: true,
                fill_rgb: Some("EAF2FF".into()),
                alignment: CellAlignment::Center,
                ..CellFormat::default()
            },
        );
        sheet.merge(CellRange::parse_a1("A4:B4").unwrap()).unwrap();
        sheet.set_freeze_pane(Some(FreezePane {
            rows: 1,
            columns: 1,
        }));

        let bytes = save_xlsx(&mut xlsx, Cursor::new(Vec::new()))
            .unwrap()
            .into_inner();
        let reopened = open_xlsx(Cursor::new(bytes)).unwrap();

        assert!(reopened.can_save());
        let sheet = reopened.workbook().sheet(0).unwrap();
        assert_eq!(
            sheet
                .cell(CellAddress::parse_a1("B3").unwrap())
                .unwrap()
                .cached_number,
            Some(30.0)
        );
        assert!(
            sheet
                .cell(CellAddress::parse_a1("A1").unwrap())
                .unwrap()
                .format
                .bold
        );
        assert_eq!(
            sheet.freeze_pane(),
            Some(FreezePane {
                rows: 1,
                columns: 1
            })
        );
        assert_eq!(sheet.merged_ranges().len(), 1);
    }

    #[test]
    fn adding_sheet_expands_package_without_materializing_grid() {
        let mut xlsx = XlsxWorkbook::blank();
        xlsx.workbook_mut().add_sheet("第二张").unwrap();

        let bytes = save_xlsx(&mut xlsx, Cursor::new(Vec::new()))
            .unwrap()
            .into_inner();
        let reopened = open_xlsx(Cursor::new(bytes)).unwrap();

        assert_eq!(reopened.workbook().sheets().len(), 2);
        assert_eq!(reopened.workbook().sheets()[1].name(), "第二张");
    }
}
