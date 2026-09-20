use crate::{PartName, PartNameError};
use std::{error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelationshipId(String);

impl RelationshipId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetMode {
    Internal,
    External,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelationshipTarget {
    Internal(PartName),
    External(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relationship {
    pub id: RelationshipId,
    pub relationship_type: String,
    pub target: RelationshipTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelationshipTargetError {
    InvalidPartName(PartNameError),
    InvalidRelationshipPartName,
    EscapesPackageRoot,
    EmptyTarget,
    QueryOrFragment,
}

impl fmt::Display for RelationshipTargetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPartName(error) => write!(f, "invalid relationship target: {error}"),
            Self::InvalidRelationshipPartName => f.write_str("invalid OPC relationship part name"),
            Self::EscapesPackageRoot => f.write_str("relationship target escapes package root"),
            Self::EmptyTarget => f.write_str("relationship target is empty"),
            Self::QueryOrFragment => {
                f.write_str("internal relationship target contains query or fragment")
            }
        }
    }
}

impl Error for RelationshipTargetError {}

impl From<PartNameError> for RelationshipTargetError {
    fn from(value: PartNameError) -> Self {
        Self::InvalidPartName(value)
    }
}

/// Return the OPC relationships part name for a package or source part.
pub fn relationship_part_name(source: Option<&PartName>) -> Result<PartName, PartNameError> {
    match source {
        None => PartName::new("/_rels/.rels"),
        Some(source) => {
            let path = source.as_str();
            let slash = path.rfind('/').expect("validated part name");
            let directory = &path[..=slash];
            let file_name = &path[slash + 1..];

            PartName::new(format!("{directory}_rels/{file_name}.rels"))
        }
    }
}

/// Recover the source part represented by an OPC relationships part.
///
/// `/_rels/.rels` is package-level and therefore maps to `None`.
pub fn source_part_from_relationship_part(
    relationship_part: &PartName,
) -> Result<Option<PartName>, RelationshipTargetError> {
    if relationship_part.as_str() == "/_rels/.rels" {
        return Ok(None);
    }

    let path = relationship_part.as_str();
    let Some((directory, file_name)) = path.rsplit_once("/_rels/") else {
        return Err(RelationshipTargetError::InvalidRelationshipPartName);
    };
    let Some(source_file) = file_name.strip_suffix(".rels") else {
        return Err(RelationshipTargetError::InvalidRelationshipPartName);
    };
    if source_file.is_empty() {
        return Err(RelationshipTargetError::InvalidRelationshipPartName);
    }

    PartName::new(format!("{directory}/{source_file}"))
        .map(Some)
        .map_err(RelationshipTargetError::InvalidPartName)
}

/// Resolve an internal OPC relationship target relative to its source part.
///
/// Package-level relationships pass `None` as the source.
pub fn resolve_internal_target(
    source: Option<&PartName>,
    target: &str,
) -> Result<PartName, RelationshipTargetError> {
    if target.is_empty() {
        return Err(RelationshipTargetError::EmptyTarget);
    }
    if target.contains(['?', '#']) {
        return Err(RelationshipTargetError::QueryOrFragment);
    }
    if target.starts_with('/') {
        return Ok(PartName::new(target.to_owned())?);
    }

    let base = source.map_or("/", PartName::parent_path);
    let mut segments: Vec<&str> = base
        .trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();

    for segment in target.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                if segments.pop().is_none() {
                    return Err(RelationshipTargetError::EscapesPackageRoot);
                }
            }
            value => segments.push(value),
        }
    }

    if segments.is_empty() {
        return Err(RelationshipTargetError::EmptyTarget);
    }

    PartName::new(format!("/{}", segments.join("/"))).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_package_relationship_part() {
        let rels = relationship_part_name(None).unwrap();
        assert_eq!(rels.as_str(), "/_rels/.rels");
        assert_eq!(source_part_from_relationship_part(&rels).unwrap(), None);
    }

    #[test]
    fn maps_part_relationship_part_bidirectionally() {
        let source = PartName::new("/word/document.xml").unwrap();
        let rels = relationship_part_name(Some(&source)).unwrap();

        assert_eq!(rels.as_str(), "/word/_rels/document.xml.rels");
        assert_eq!(
            source_part_from_relationship_part(&rels).unwrap(),
            Some(source)
        );
    }

    #[test]
    fn rejects_non_relationship_part_name() {
        let part = PartName::new("/word/document.xml").unwrap();
        assert_eq!(
            source_part_from_relationship_part(&part),
            Err(RelationshipTargetError::InvalidRelationshipPartName)
        );
    }

    #[test]
    fn resolves_part_relative_target() {
        let source = PartName::new("/word/document.xml").unwrap();
        let target = resolve_internal_target(Some(&source), "media/image1.png").unwrap();

        assert_eq!(target.as_str(), "/word/media/image1.png");
    }

    #[test]
    fn resolves_parent_segments_without_leaving_package() {
        let source = PartName::new("/ppt/slides/slide1.xml").unwrap();
        let target =
            resolve_internal_target(Some(&source), "../slideLayouts/slideLayout1.xml").unwrap();

        assert_eq!(target.as_str(), "/ppt/slideLayouts/slideLayout1.xml");
    }

    #[test]
    fn package_relationship_is_relative_to_root() {
        let target = resolve_internal_target(None, "word/document.xml").unwrap();
        assert_eq!(target.as_str(), "/word/document.xml");
    }

    #[test]
    fn rejects_root_escape() {
        let source = PartName::new("/word/document.xml").unwrap();
        assert_eq!(
            resolve_internal_target(Some(&source), "../../evil.xml"),
            Err(RelationshipTargetError::EscapesPackageRoot)
        );
    }
}
