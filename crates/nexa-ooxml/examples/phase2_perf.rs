use nexa_ooxml::{
    ContentTypeMap, ContentTypeRule, LazyZipPackage, Package, PartName, Relationship,
    RelationshipId, RelationshipTarget, write_owned_package,
};
use std::{
    fs::{self, File},
    io::Read,
    path::PathBuf,
    process,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

const OFFICE_DOCUMENT_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";
const WORD_CONTENT_TYPE: &str =
    "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml";
const LARGE_PART_BYTES: usize = 8 * 1024 * 1024;
const XML_PARTS: usize = 128;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_path = temporary_path("source");
    let repacked_path = temporary_path("repacked");

    build_fixture(&source_path)?;
    let source_zip_bytes = fs::metadata(&source_path)?.len();

    let start = Instant::now();
    let source = File::open(&source_path)?;
    let mut package = LazyZipPackage::open(source)?;
    let index_elapsed = start.elapsed();

    let start = Instant::now();
    let info = package.office_package_info()?;
    let identify_elapsed = start.elapsed();

    let main_part = info.main_part().clone();
    let start = Instant::now();
    let main_bytes = package.read_part(&main_part)?;
    let selected_read_elapsed = start.elapsed();

    let start = Instant::now();
    let repacked = File::create(&repacked_path)?;
    let repacked = package.repack_validated(repacked)?;
    repacked.sync_all()?;
    let repack_elapsed = start.elapsed();
    let repacked_zip_bytes = fs::metadata(&repacked_path)?.len();

    println!("fixture_xml_parts={XML_PARTS}");
    println!("fixture_large_part_bytes={LARGE_PART_BYTES}");
    println!("source_zip_bytes={source_zip_bytes}");
    println!("indexed_entries={}", package.entries().count());
    println!("declared_uncompressed_bytes={}", package.usage().total_uncompressed);
    println!("office_kind={:?}", info.kind());
    println!("main_part={}", main_part.as_str());
    println!("main_part_bytes={}", main_bytes.len());
    println!("index_us={}", index_elapsed.as_micros());
    println!("identify_us={}", identify_elapsed.as_micros());
    println!("selected_read_us={}", selected_read_elapsed.as_micros());
    println!("validated_repack_us={}", repack_elapsed.as_micros());
    println!("repacked_zip_bytes={repacked_zip_bytes}");

    validate_repacked(&repacked_path)?;

    let _ = fs::remove_file(source_path);
    let _ = fs::remove_file(repacked_path);
    Ok(())
}

fn build_fixture(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let main_part = PartName::new("/word/document.xml")?;
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
        content_type: WORD_CONTENT_TYPE.into(),
    });

    let mut package = Package::new(content_types);
    package.insert_part(
        main_part.clone(),
        br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Nexa</w:t></w:r></w:p></w:body></w:document>"#
            .to_vec(),
    )?;

    for index in 0..XML_PARTS {
        package.insert_part(
            PartName::new(format!("/customXml/item-{index}.xml"))?,
            format!("<item id=\"{index}\"><value>Nexa Office</value></item>").into_bytes(),
        )?;
    }

    package.insert_part(
        PartName::new("/customXml/large.bin")?,
        deterministic_bytes(LARGE_PART_BYTES),
    )?;

    package
        .package_relationships_mut()
        .insert(Relationship {
            id: RelationshipId::new("rId1"),
            relationship_type: OFFICE_DOCUMENT_RELATIONSHIP.into(),
            target: RelationshipTarget::Internal(main_part),
        })?;

    let file = File::create(path)?;
    let file = write_owned_package(&package, file)?;
    file.sync_all()?;
    Ok(())
}

fn validate_repacked(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    File::open(path)?.read_to_end(&mut bytes)?;
    let cursor = std::io::Cursor::new(bytes);
    let mut package = LazyZipPackage::open(cursor)?;

    let info = package.office_package_info()?;
    let large = package.read_part(&PartName::new("/customXml/large.bin")?)?;

    if info.main_part().as_str() != "/word/document.xml" || large.len() != LARGE_PART_BYTES {
        return Err("validated repack changed the package contract".into());
    }

    Ok(())
}

fn deterministic_bytes(size: usize) -> Vec<u8> {
    let mut state = 0x4d59_5df4_d0f3_3173u64;
    let mut output = Vec::with_capacity(size);

    for _ in 0..size {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        output.push(state as u8);
    }

    output
}

fn temporary_path(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    std::env::temp_dir().join(format!(
        "nexa-ooxml-phase2-{label}-{}-{nonce}.zip",
        process::id()
    ))
}
