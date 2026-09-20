use crate::{ContentTypeMap, LazyZipPackage, PartName, RelationshipTarget, ZipPackageError};
use std::{
    error::Error,
    fmt,
    io::{Read, Seek},
};

const WORD_DOCUMENT_MAIN: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml";
const WORD_MACRO_MAIN: &str = "application/vnd.ms-word.document.macroEnabled.main+xml";
const EXCEL_WORKBOOK_MAIN: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml";
const EXCEL_MACRO_MAIN: &str = "application/vnd.ms-excel.sheet.macroEnabled.main+xml";
const POWERPOINT_PRESENTATION_MAIN: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml";
const POWERPOINT_MACRO_MAIN: &str =
    "application/vnd.ms-powerpoint.presentation.macroEnabled.main+xml";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfficePackageKind {
    Document,
    Workbook,
    Presentation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficePackageInfo {
    kind: OfficePackageKind,
    main_part: PartName,
    content_type: String,
}

impl OfficePackageInfo {
    #[must_use]
    pub const fn kind(&self) -> OfficePackageKind {
        self.kind
    }

    #[must_use]
    pub fn main_part(&self) -> &PartName {
        &self.main_part
    }

    #[must_use]
    pub fn content_type(&self) -> &str {
        &self.content_type
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OfficePackageError {
    Zip(ZipPackageError),
    MissingOfficeDocumentRelationship,
    ExternalOfficeDocumentRelationship,
    MissingMainPartContentType(String),
    UnsupportedMainPartContentType(String),
}

impl fmt::Display for OfficePackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zip(error) => write!(f, "{error}"),
            Self::MissingOfficeDocumentRelationship => {
                f.write_str("package has no officeDocument root relationship")
            }
            Self::ExternalOfficeDocumentRelationship => {
                f.write_str("officeDocument root relationship must be internal")
            }
            Self::MissingMainPartContentType(part) => {
                write!(f, "main Office part has no content type: {part}")
            }
            Self::UnsupportedMainPartContentType(content_type) => {
                write!(
                    f,
                    "unsupported Office main-part content type: {content_type}"
                )
            }
        }
    }
}

impl Error for OfficePackageError {}

impl From<ZipPackageError> for OfficePackageError {
    fn from(value: ZipPackageError) -> Self {
        Self::Zip(value)
    }
}

impl<R: Read + Seek> LazyZipPackage<R> {
    pub fn office_package_info(&mut self) -> Result<OfficePackageInfo, OfficePackageError> {
        let content_types = self.read_content_types()?;
        let relationships = self.read_relationships(None)?;

        let office_document = relationships
            .iter()
            .find(|relationship| relationship.relationship_type.ends_with("/officeDocument"))
            .ok_or(OfficePackageError::MissingOfficeDocumentRelationship)?;

        let RelationshipTarget::Internal(main_part) = &office_document.target else {
            return Err(OfficePackageError::ExternalOfficeDocumentRelationship);
        };

        office_package_info(&content_types, main_part)
    }
}

fn office_package_info(
    content_types: &ContentTypeMap,
    main_part: &PartName,
) -> Result<OfficePackageInfo, OfficePackageError> {
    let content_type = content_types
        .content_type_for(main_part)
        .ok_or_else(|| OfficePackageError::MissingMainPartContentType(main_part.to_string()))?;

    let kind = match content_type {
        WORD_DOCUMENT_MAIN | WORD_MACRO_MAIN => OfficePackageKind::Document,
        EXCEL_WORKBOOK_MAIN | EXCEL_MACRO_MAIN => OfficePackageKind::Workbook,
        POWERPOINT_PRESENTATION_MAIN | POWERPOINT_MACRO_MAIN => OfficePackageKind::Presentation,
        other => {
            return Err(OfficePackageError::UnsupportedMainPartContentType(
                other.to_owned(),
            ));
        }
    };

    Ok(OfficePackageInfo {
        kind,
        main_part: main_part.clone(),
        content_type: content_type.to_owned(),
    })
}
