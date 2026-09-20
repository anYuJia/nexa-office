use crate::{Package, PartName, RelationshipSet};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackageDiff {
    pub added_parts: Vec<PartName>,
    pub removed_parts: Vec<PartName>,
    pub changed_parts: Vec<PartName>,
    pub content_types_changed: bool,
    pub package_relationships_changed: bool,
    pub part_relationships_changed: bool,
}

impl PackageDiff {
    #[must_use]
    pub fn is_equivalent(&self) -> bool {
        self.added_parts.is_empty()
            && self.removed_parts.is_empty()
            && self.changed_parts.is_empty()
            && !self.content_types_changed
            && !self.package_relationships_changed
            && !self.part_relationships_changed
    }
}

#[must_use]
pub fn compare_packages(left: &Package, right: &Package) -> PackageDiff {
    let left_parts: BTreeMap<PartName, (&str, &[u8])> = left
        .parts()
        .map(|part| (part.name().clone(), (part.content_type(), part.bytes())))
        .collect();
    let right_parts: BTreeMap<PartName, (&str, &[u8])> = right
        .parts()
        .map(|part| {
            (
                part.name().clone(),
                (part.content_type(), part.bytes()),
            )
        })
        .collect();

    let added_parts = right_parts
        .keys()
        .filter(|name| !left_parts.contains_key(*name))
        .cloned()
        .collect();
    let removed_parts = left_parts
        .keys()
        .filter(|name| !right_parts.contains_key(*name))
        .cloned()
        .collect();
    let changed_parts = left_parts
        .iter()
        .filter_map(|(name, value)| {
            right_parts
                .get(name)
                .is_some_and(|right_value| right_value != value)
                .then(|| name.clone())
        })
        .collect();

    let left_relationships = relationship_map(left);
    let right_relationships = relationship_map(right);

    PackageDiff {
        added_parts,
        removed_parts,
        changed_parts,
        content_types_changed: left.content_types() != right.content_types(),
        package_relationships_changed: left.package_relationships()
            != right.package_relationships(),
        part_relationships_changed: left_relationships != right_relationships,
    }
}

fn relationship_map(package: &Package) -> BTreeMap<PartName, RelationshipSet> {
    package
        .part_relationships()
        .map(|(source, relationships)| (source.clone(), relationships.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ContentTypeMap, ContentTypeRule};

    #[test]
    fn equal_packages_have_empty_diff() {
        let package = package_with_payload(b"same");

        assert!(compare_packages(&package, &package.clone()).is_equivalent());
    }

    #[test]
    fn changed_opaque_part_is_reported() {
        let left = package_with_payload(b"left");
        let right = package_with_payload(b"right");
        let diff = compare_packages(&left, &right);

        assert_eq!(
            diff.changed_parts,
            vec![PartName::new("/custom/data.bin").unwrap()]
        );
        assert!(!diff.is_equivalent());
    }

    fn package_with_payload(payload: &[u8]) -> Package {
        let part = PartName::new("/custom/data.bin").unwrap();
        let mut content_types = ContentTypeMap::default();
        content_types.insert(ContentTypeRule::Default {
            extension: "bin".into(),
            content_type: "application/octet-stream".into(),
        });

        let mut package = Package::new(content_types);
        package.insert_part(part, payload.to_vec()).unwrap();
        package
    }
}
