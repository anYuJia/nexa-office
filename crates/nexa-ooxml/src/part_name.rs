use std::{error::Error, fmt};

/// Canonical OPC part name.
///
/// Nexa stores part names with a leading slash. ZIP entry names are derived by
/// stripping that slash at the package adapter boundary.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartName(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartNameError {
    MissingLeadingSlash,
    EmptySegment,
    DotSegment,
    Backslash,
    QueryOrFragment,
    RootOnly,
    ControlCharacter,
}

impl fmt::Display for PartNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::MissingLeadingSlash => "OPC part name must begin with '/'",
            Self::EmptySegment => "OPC part name must not contain an empty path segment",
            Self::DotSegment => "OPC part name must not contain '.' or '..' segments",
            Self::Backslash => "OPC part name must use '/' rather than '\\'",
            Self::QueryOrFragment => "OPC part name must not contain query or fragment syntax",
            Self::RootOnly => "'/' identifies the package root, not a part",
            Self::ControlCharacter => "OPC part name must not contain control characters",
        };
        f.write_str(message)
    }
}

impl Error for PartNameError {}

impl PartName {
    pub fn new(value: impl Into<String>) -> Result<Self, PartNameError> {
        let value = value.into();
        validate(&value)?;
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn zip_entry_name(&self) -> &str {
        self.0.strip_prefix('/').expect("validated part name")
    }

    #[must_use]
    pub fn extension(&self) -> Option<&str> {
        let file = self.0.rsplit('/').next()?;
        let (_, extension) = file.rsplit_once('.')?;
        (!extension.is_empty()).then_some(extension)
    }

    #[must_use]
    pub fn parent_path(&self) -> &str {
        let last_slash = self.0.rfind('/').expect("validated part name");
        if last_slash == 0 {
            "/"
        } else {
            &self.0[..last_slash + 1]
        }
    }
}

impl fmt::Display for PartName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn validate(value: &str) -> Result<(), PartNameError> {
    if !value.starts_with('/') {
        return Err(PartNameError::MissingLeadingSlash);
    }
    if value == "/" {
        return Err(PartNameError::RootOnly);
    }
    if value.contains('\\') {
        return Err(PartNameError::Backslash);
    }
    if value.contains(['?', '#']) {
        return Err(PartNameError::QueryOrFragment);
    }
    if value.chars().any(char::is_control) {
        return Err(PartNameError::ControlCharacter);
    }

    for segment in value[1..].split('/') {
        if segment.is_empty() {
            return Err(PartNameError::EmptySegment);
        }
        if matches!(segment, "." | "..") {
            return Err(PartNameError::DotSegment);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_common_ooxml_part_names() {
        let part = PartName::new("/word/document.xml").unwrap();
        assert_eq!(part.zip_entry_name(), "word/document.xml");
        assert_eq!(part.extension(), Some("xml"));
        assert_eq!(part.parent_path(), "/word/");
    }

    #[test]
    fn root_level_part_has_root_parent() {
        let part = PartName::new("/docProps.xml").unwrap();
        assert_eq!(part.parent_path(), "/");
    }

    #[test]
    fn rejects_traversal_and_non_opc_path_forms() {
        for value in [
            "word/document.xml",
            "/word/../document.xml",
            "/word//document.xml",
            "/word\\document.xml",
            "/word/document.xml#x",
            "/",
        ] {
            assert!(PartName::new(value).is_err(), "{value} should be rejected");
        }
    }
}
