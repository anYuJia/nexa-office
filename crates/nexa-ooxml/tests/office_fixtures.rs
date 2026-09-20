use nexa_ooxml::{
    ContentTypeMap, ContentTypeRule, LazyZipPackage, OfficePackageKind, Package, PartName,
    Relationship, RelationshipId, RelationshipTarget, write_owned_package,
};
use std::io::Cursor;

const OFFICE_DOCUMENT_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";

const WORD_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml";
const EXCEL_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml";
const POWERPOINT_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml";

#[test]
fn recognizes_minimal_docx_package() {
    assert_kind(
        "/word/document.xml",
        WORD_CONTENT_TYPE,
        b"<w:document/>",
        OfficePackageKind::Document,
    );
}

#[test]
fn recognizes_minimal_xlsx_package() {
    assert_kind(
        "/xl/workbook.xml",
        EXCEL_CONTENT_TYPE,
        b"<workbook/>",
        OfficePackageKind::Workbook,
    );
}

#[test]
fn recognizes_minimal_pptx_package() {
    assert_kind(
        "/ppt/presentation.xml",
        POWERPOINT_CONTENT_TYPE,
        b"<p:presentation/>",
        OfficePackageKind::Presentation,
    );
}

#[test]
fn validated_repack_keeps_kind_and_unknown_bytes_for_all_office_families() {
    for (main_part, content_type, body, expected_kind) in [
        (
            "/word/document.xml",
            WORD_CONTENT_TYPE,
            b"<w:document/>".as_slice(),
            OfficePackageKind::Document,
        ),
        (
            "/xl/workbook.xml",
            EXCEL_CONTENT_TYPE,
            b"<workbook/>".as_slice(),
            OfficePackageKind::Workbook,
        ),
        (
            "/ppt/presentation.xml",
            POWERPOINT_CONTENT_TYPE,
            b"<p:presentation/>".as_slice(),
            OfficePackageKind::Presentation,
        ),
    ] {
        let original = build_package(main_part, content_type, body);
        let mut lazy = LazyZipPackage::open(Cursor::new(original)).unwrap();

        let repacked = lazy
            .repack_validated(Cursor::new(Vec::new()))
            .unwrap()
            .into_inner();
        let mut reopened = LazyZipPackage::open(Cursor::new(repacked)).unwrap();

        assert_eq!(
            reopened.office_package_info().unwrap().kind(),
            expected_kind
        );
        assert_eq!(
            reopened
                .read_part(&PartName::new("/customXml/preserve.bin").unwrap())
                .unwrap(),
            [0, 1, 2, 255, 17, 42]
        );
    }
}

fn assert_kind(main_part: &str, content_type: &str, body: &[u8], expected_kind: OfficePackageKind) {
    let bytes = build_package(main_part, content_type, body);
    let mut package = LazyZipPackage::open(Cursor::new(bytes)).unwrap();
    let info = package.office_package_info().unwrap();

    assert_eq!(info.kind(), expected_kind);
    assert_eq!(info.main_part().as_str(), main_part);
    assert_eq!(info.content_type(), content_type);
}

fn build_package(main_part: &str, content_type: &str, body: &[u8]) -> Vec<u8> {
    let main_part = PartName::new(main_part).unwrap();
    let mut content_types = ContentTypeMap::default();
    content_types.insert(ContentTypeRule::Default {
        extension: "xml".into(),
        content_type: "application/xml".into(),
    });
    content_types.insert(ContentTypeRule::Default {
        extension: "rels".into(),
        content_type: "application/vnd.openxmlformats-package.relationships+xml".into(),
    });
    content_types.insert(ContentTypeRule::Default {
        extension: "bin".into(),
        content_type: "application/octet-stream".into(),
    });
    content_types.insert(ContentTypeRule::Override {
        part_name: main_part.clone(),
        content_type: content_type.into(),
    });

    let mut package = Package::new(content_types);
    package
        .insert_part(main_part.clone(), body.to_vec())
        .unwrap();
    package
        .insert_part(
            PartName::new("/customXml/preserve.bin").unwrap(),
            vec![0, 1, 2, 255, 17, 42],
        )
        .unwrap();
    package
        .package_relationships_mut()
        .insert(Relationship {
            id: RelationshipId::new("rId1"),
            relationship_type: OFFICE_DOCUMENT_RELATIONSHIP.into(),
            target: RelationshipTarget::Internal(main_part),
        })
        .unwrap();

    write_owned_package(&package, Cursor::new(Vec::new()))
        .unwrap()
        .into_inner()
}
