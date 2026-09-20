use nexa_ooxml::{
    ContentTypeMap, ContentTypeRule, LazyZipPackage, LimitViolation, OfficePackageError, Package,
    PackageLimits, PartName, Relationship, RelationshipId, RelationshipTarget, XmlLimits,
    ZipPackageError, parse_content_types_with_limits, parse_relationships_with_limits,
    write_owned_package,
};
use std::io::Cursor;

const OFFICE_DOCUMENT_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";

#[test]
fn rejects_package_that_exceeds_entry_budget() {
    let bytes = package_with_parts(4);
    let limits = PackageLimits {
        max_entries: 3,
        ..PackageLimits::default()
    };

    assert!(matches!(
        LazyZipPackage::new(Cursor::new(bytes), limits),
        Err(ZipPackageError::ResourceLimit(
            LimitViolation::TooManyEntries
        ))
    ));
}

#[test]
fn rejects_external_office_document_relationship() {
    let bytes = package_with_root_relationship(RelationshipTarget::External(
        "https://example.invalid/document.xml".into(),
    ));
    let mut package = LazyZipPackage::open(Cursor::new(bytes)).unwrap();

    assert_eq!(
        package.office_package_info(),
        Err(OfficePackageError::ExternalOfficeDocumentRelationship)
    );
}

#[test]
fn rejects_unknown_main_part_content_type() {
    let main_part = PartName::new("/vendor/main.xml").unwrap();
    let mut content_types = base_content_types();
    content_types.insert(ContentTypeRule::Override {
        part_name: main_part.clone(),
        content_type: "application/x-vendor-office".into(),
    });

    let mut package = Package::new(content_types);
    package.insert_part(main_part.clone(), b"<vendor/>".to_vec()).unwrap();
    package
        .package_relationships_mut()
        .insert(Relationship {
            id: RelationshipId::new("rId1"),
            relationship_type: OFFICE_DOCUMENT_RELATIONSHIP.into(),
            target: RelationshipTarget::Internal(main_part),
        })
        .unwrap();

    let bytes = write_owned_package(&package, Cursor::new(Vec::new()))
        .unwrap()
        .into_inner();
    let mut lazy = LazyZipPackage::open(Cursor::new(bytes)).unwrap();

    assert_eq!(
        lazy.office_package_info(),
        Err(OfficePackageError::UnsupportedMainPartContentType(
            "application/x-vendor-office".into()
        ))
    );
}

#[test]
fn bounded_xml_parsers_survive_deterministic_mutation_corpus() {
    let seeds: [&[u8]; 2] = [
        br#"<Types><Default Extension="xml" ContentType="application/xml"/></Types>"#,
        br#"<Relationships><Relationship Id="rId1" Type="x" Target="word/document.xml"/></Relationships>"#,
    ];
    let limits = XmlLimits {
        max_input_bytes: 4096,
        max_depth: 16,
        max_attributes_per_element: 16,
    };

    for seed in seeds {
        for index in 0..seed.len().min(128) {
            let mut mutation = seed.to_vec();
            mutation[index] ^= 0x5a;

            let _ = parse_content_types_with_limits(&mutation, limits);
            let _ = parse_relationships_with_limits(None, &mutation, limits);
        }
    }
}

fn package_with_parts(part_count: usize) -> Vec<u8> {
    let mut package = Package::new(base_content_types());

    for index in 0..part_count {
        package
            .insert_part(
                PartName::new(format!("/custom/item-{index}.xml")).unwrap(),
                format!("<item index=\"{index}\"/>").into_bytes(),
            )
            .unwrap();
    }

    write_owned_package(&package, Cursor::new(Vec::new()))
        .unwrap()
        .into_inner()
}

fn package_with_root_relationship(target: RelationshipTarget) -> Vec<u8> {
    let main_part = PartName::new("/word/document.xml").unwrap();
    let mut content_types = base_content_types();
    content_types.insert(ContentTypeRule::Override {
        part_name: main_part.clone(),
        content_type:
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"
                .into(),
    });

    let mut package = Package::new(content_types);
    package.insert_part(main_part, b"<w:document/>".to_vec()).unwrap();
    package
        .package_relationships_mut()
        .insert(Relationship {
            id: RelationshipId::new("rId1"),
            relationship_type: OFFICE_DOCUMENT_RELATIONSHIP.into(),
            target,
        })
        .unwrap();

    write_owned_package(&package, Cursor::new(Vec::new()))
        .unwrap()
        .into_inner()
}

fn base_content_types() -> ContentTypeMap {
    let mut content_types = ContentTypeMap::default();
    content_types.insert(ContentTypeRule::Default {
        extension: "xml".into(),
        content_type: "application/xml".into(),
    });
    content_types.insert(ContentTypeRule::Default {
        extension: "rels".into(),
        content_type: "application/vnd.openxmlformats-package.relationships+xml".into(),
    });
    content_types
}
