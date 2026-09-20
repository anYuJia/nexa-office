use crate::{ContentTypeMap, PartName, Relationship, RelationshipId};
use std::collections::BTreeMap;
use std::{error::Error, fmt};

/// Opaque package part payload.
///
/// Semantic DOCX/XLSX/PPTX crates may parse selected parts, but the OPC layer keeps
/// the original bytes available so unsupported content can survive a rewrite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    name: PartName,
    content_type: String,
    bytes: Vec<u8>,
}

impl Part {
    #[must_use]
    pub fn new(name: PartName, content_type: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            name,
            content_type: content_type.into(),
            bytes,
        }
    }

    #[must_use]
    pub fn name(&self) -> &PartName {
        &self.name
    }

    #[must_use]
    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RelationshipSet {
    entries: BTreeMap<RelationshipId, Relationship>,
}

impl RelationshipSet {
    pub fn insert(&mut self, relationship: Relationship) -> Result<(), PackageError> {
        if self.entries.contains_key(&relationship.id) {
            return Err(PackageError::DuplicateRelationshipId(
                relationship.id.as_str().to_owned(),
            ));
        }

        self.entries.insert(relationship.id.clone(), relationship);
        Ok(())
    }

    #[must_use]
    pub fn get(&self, id: &RelationshipId) -> Option<&Relationship> {
        self.entries.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Relationship> {
        self.entries.values()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// In-memory OPC package graph.
///
/// This is intentionally not the ZIP representation. ZIP adapters populate this graph
/// lazily/selectively, and serializers decide which opaque bytes can be copied through.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Package {
    content_types: ContentTypeMap,
    parts: BTreeMap<PartName, Part>,
    package_relationships: RelationshipSet,
    part_relationships: BTreeMap<PartName, RelationshipSet>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageError {
    DuplicatePart(String),
    DuplicateRelationshipId(String),
    MissingContentType(String),
}

impl fmt::Display for PackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicatePart(name) => write!(f, "duplicate OPC part: {name}"),
            Self::DuplicateRelationshipId(id) => {
                write!(f, "duplicate relationship identifier: {id}")
            }
            Self::MissingContentType(name) => {
                write!(f, "no content type registered for OPC part: {name}")
            }
        }
    }
}

impl Error for PackageError {}

impl Package {
    #[must_use]
    pub fn new(content_types: ContentTypeMap) -> Self {
        Self {
            content_types,
            ..Self::default()
        }
    }

    pub fn insert_part(&mut self, name: PartName, bytes: Vec<u8>) -> Result<(), PackageError> {
        if self.parts.contains_key(&name) {
            return Err(PackageError::DuplicatePart(name.to_string()));
        }

        let content_type = self
            .content_types
            .content_type_for(&name)
            .ok_or_else(|| PackageError::MissingContentType(name.to_string()))?
            .to_owned();

        self.parts
            .insert(name.clone(), Part::new(name, content_type, bytes));
        Ok(())
    }

    #[must_use]
    pub fn part(&self, name: &PartName) -> Option<&Part> {
        self.parts.get(name)
    }

    pub fn remove_part(&mut self, name: &PartName) -> Option<Part> {
        self.part_relationships.remove(name);
        self.parts.remove(name)
    }

    pub fn parts(&self) -> impl Iterator<Item = &Part> {
        self.parts.values()
    }

    #[must_use]
    pub fn content_types(&self) -> &ContentTypeMap {
        &self.content_types
    }

    pub fn package_relationships_mut(&mut self) -> &mut RelationshipSet {
        &mut self.package_relationships
    }

    #[must_use]
    pub fn package_relationships(&self) -> &RelationshipSet {
        &self.package_relationships
    }

    pub fn relationships_mut(&mut self, source: PartName) -> &mut RelationshipSet {
        self.part_relationships.entry(source).or_default()
    }

    #[must_use]
    pub fn relationships(&self, source: &PartName) -> Option<&RelationshipSet> {
        self.part_relationships.get(source)
    }

    pub fn part_relationships(&self) -> impl Iterator<Item = (&PartName, &RelationshipSet)> {
        self.part_relationships.iter()
    }

    #[must_use]
    pub fn part_count(&self) -> usize {
        self.parts.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ContentTypeRule, RelationshipTarget};

    fn content_types() -> ContentTypeMap {
        let mut map = ContentTypeMap::default();
        map.insert(ContentTypeRule::Default {
            extension: "xml".into(),
            content_type: "application/xml".into(),
        });
        map.insert(ContentTypeRule::Default {
            extension: "bin".into(),
            content_type: "application/octet-stream".into(),
        });
        map
    }

    #[test]
    fn opaque_unknown_part_bytes_are_preserved_exactly() {
        let mut package = Package::new(content_types());
        let name = PartName::new("/customXml/item1.bin").unwrap();
        let original = vec![0, 255, 42, 0, 17];

        package.insert_part(name.clone(), original.clone()).unwrap();

        assert_eq!(package.part(&name).unwrap().bytes(), original);
    }

    #[test]
    fn duplicate_parts_are_rejected() {
        let mut package = Package::new(content_types());
        let name = PartName::new("/word/document.xml").unwrap();

        package
            .insert_part(name.clone(), b"first".to_vec())
            .unwrap();

        assert_eq!(
            package.insert_part(name.clone(), b"second".to_vec()),
            Err(PackageError::DuplicatePart(name.to_string()))
        );
    }

    #[test]
    fn missing_content_type_is_rejected() {
        let mut package = Package::new(content_types());
        let name = PartName::new("/word/media/image1.png").unwrap();

        assert_eq!(
            package.insert_part(name.clone(), vec![1, 2, 3]),
            Err(PackageError::MissingContentType(name.to_string()))
        );
    }

    #[test]
    fn relationship_ids_are_unique_within_a_set() {
        let mut relationships = RelationshipSet::default();
        let id = RelationshipId::new("rId1");

        relationships
            .insert(Relationship {
                id: id.clone(),
                relationship_type: "type-a".into(),
                target: RelationshipTarget::Internal(PartName::new("/word/styles.xml").unwrap()),
            })
            .unwrap();

        assert_eq!(
            relationships.insert(Relationship {
                id: id.clone(),
                relationship_type: "type-b".into(),
                target: RelationshipTarget::External("https://example.invalid".into()),
            }),
            Err(PackageError::DuplicateRelationshipId("rId1".into()))
        );
    }

    #[test]
    fn removing_part_removes_only_its_relationship_set() {
        let mut package = Package::new(content_types());
        let document = PartName::new("/word/document.xml").unwrap();
        let styles = PartName::new("/word/styles.xml").unwrap();

        package.insert_part(document.clone(), vec![1]).unwrap();
        package.insert_part(styles.clone(), vec![2]).unwrap();
        package
            .relationships_mut(document.clone())
            .insert(Relationship {
                id: RelationshipId::new("rId1"),
                relationship_type: "styles".into(),
                target: RelationshipTarget::Internal(styles.clone()),
            })
            .unwrap();

        let removed = package.remove_part(&document).unwrap();

        assert_eq!(removed.name(), &document);
        assert!(package.relationships(&document).is_none());
        assert!(package.part(&styles).is_some());
    }
}
