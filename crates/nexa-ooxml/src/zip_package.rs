use crate::{
    ContentTypeMap, LimitViolation, Package, PackageError, PackageLimits, PackageUsage, PartName,
    PartNameError, RelationshipSet, RelationshipTargetError, XmlParseError, parse_content_types,
    parse_relationships, relationship_part_name, source_part_from_relationship_part,
    write_content_types, write_relationships,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
    io::{Read, Seek, Write},
};
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageCompression {
    Stored,
    Deflated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZipEntryMetadata {
    part_name: PartName,
    index: usize,
    compressed_size: u64,
    uncompressed_size: u64,
    compression: PackageCompression,
}

impl ZipEntryMetadata {
    #[must_use]
    pub fn part_name(&self) -> &PartName {
        &self.part_name
    }

    #[must_use]
    pub const fn compressed_size(&self) -> u64 {
        self.compressed_size
    }

    #[must_use]
    pub const fn uncompressed_size(&self) -> u64 {
        self.uncompressed_size
    }

    #[must_use]
    pub const fn compression(&self) -> PackageCompression {
        self.compression
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZipPackageError {
    Archive(String),
    Io(String),
    InvalidPartName(PartNameError),
    ResourceLimit(LimitViolation),
    OverlappingEntries,
    EncryptedEntry(String),
    SymlinkEntry(String),
    UnsupportedCompression {
        part: String,
        method: String,
    },
    DuplicatePart(String),
    PartNotFound(String),
    PartReadLimitExceeded(String),
    SizeMismatch {
        part: String,
        expected: u64,
        actual: u64,
    },
    Xml(XmlParseError),
    Package(PackageError),
    RelationshipPath(RelationshipTargetError),
    Write(String),
}

impl fmt::Display for ZipPackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Archive(message) => write!(f, "invalid ZIP archive: {message}"),
            Self::Io(message) => write!(f, "ZIP I/O error: {message}"),
            Self::InvalidPartName(error) => write!(f, "invalid OPC ZIP entry name: {error}"),
            Self::ResourceLimit(error) => write!(f, "ZIP resource limit exceeded: {error:?}"),
            Self::OverlappingEntries => f.write_str("ZIP contains overlapping file data"),
            Self::EncryptedEntry(part) => write!(f, "encrypted ZIP entry is not supported: {part}"),
            Self::SymlinkEntry(part) => write!(f, "symbolic-link ZIP entry is not allowed: {part}"),
            Self::UnsupportedCompression { part, method } => {
                write!(f, "unsupported ZIP compression for {part}: {method}")
            }
            Self::DuplicatePart(part) => write!(f, "duplicate OPC ZIP part: {part}"),
            Self::PartNotFound(part) => write!(f, "OPC ZIP part not found: {part}"),
            Self::PartReadLimitExceeded(part) => {
                write!(f, "OPC ZIP part exceeded its read limit: {part}")
            }
            Self::SizeMismatch {
                part,
                expected,
                actual,
            } => write!(
                f,
                "OPC ZIP part size mismatch for {part}: expected {expected}, read {actual}"
            ),
            Self::Xml(error) => write!(f, "OPC metadata XML error: {error}"),
            Self::Package(error) => write!(f, "OPC package error: {error}"),
            Self::RelationshipPath(error) => write!(f, "OPC relationship path error: {error}"),
            Self::Write(message) => write!(f, "ZIP write error: {message}"),
        }
    }
}

impl Error for ZipPackageError {}

impl From<PartNameError> for ZipPackageError {
    fn from(value: PartNameError) -> Self {
        Self::InvalidPartName(value)
    }
}

impl From<LimitViolation> for ZipPackageError {
    fn from(value: LimitViolation) -> Self {
        Self::ResourceLimit(value)
    }
}

impl From<XmlParseError> for ZipPackageError {
    fn from(value: XmlParseError) -> Self {
        Self::Xml(value)
    }
}

impl From<PackageError> for ZipPackageError {
    fn from(value: PackageError) -> Self {
        Self::Package(value)
    }
}

impl From<RelationshipTargetError> for ZipPackageError {
    fn from(value: RelationshipTargetError) -> Self {
        Self::RelationshipPath(value)
    }
}

/// Lazy ZIP-backed OPC package.
///
/// Construction validates the central directory and records compact metadata only.
/// Individual file bodies are decompressed only when `read_part` is called.
pub struct LazyZipPackage<R: Read + Seek> {
    archive: ZipArchive<R>,
    entries: BTreeMap<PartName, ZipEntryMetadata>,
    usage: PackageUsage,
    limits: PackageLimits,
}

impl<R: Read + Seek> LazyZipPackage<R> {
    pub fn new(reader: R, limits: PackageLimits) -> Result<Self, ZipPackageError> {
        let mut archive =
            ZipArchive::new(reader).map_err(|error| ZipPackageError::Archive(error.to_string()))?;

        if archive
            .has_overlapping_files()
            .map_err(|error| ZipPackageError::Archive(error.to_string()))?
        {
            return Err(ZipPackageError::OverlappingEntries);
        }

        let mut entries = BTreeMap::new();
        let mut usage = PackageUsage::default();

        for index in 0..archive.len() {
            let metadata = {
                let file = archive
                    .by_index_raw(index)
                    .map_err(|error| ZipPackageError::Archive(error.to_string()))?;

                limits.check_entry(&mut usage, file.compressed_size(), file.size())?;

                if file.is_dir() {
                    continue;
                }

                let display_name = file.name().to_owned();

                if file.encrypted() {
                    return Err(ZipPackageError::EncryptedEntry(display_name));
                }
                if file.is_symlink() {
                    return Err(ZipPackageError::SymlinkEntry(display_name));
                }

                let compression = match file.compression() {
                    CompressionMethod::Stored => PackageCompression::Stored,
                    CompressionMethod::Deflated => PackageCompression::Deflated,
                    method => {
                        return Err(ZipPackageError::UnsupportedCompression {
                            part: display_name,
                            method: format!("{method:?}"),
                        });
                    }
                };

                let part_name = PartName::new(format!("/{}", file.name()))?;

                ZipEntryMetadata {
                    part_name,
                    index,
                    compressed_size: file.compressed_size(),
                    uncompressed_size: file.size(),
                    compression,
                }
            };

            if entries.contains_key(&metadata.part_name) {
                return Err(ZipPackageError::DuplicatePart(
                    metadata.part_name.to_string(),
                ));
            }

            entries.insert(metadata.part_name.clone(), metadata);
        }

        Ok(Self {
            archive,
            entries,
            usage,
            limits,
        })
    }

    pub fn open(reader: R) -> Result<Self, ZipPackageError> {
        Self::new(reader, PackageLimits::default())
    }

    pub fn entries(&self) -> impl Iterator<Item = &ZipEntryMetadata> {
        self.entries.values()
    }

    #[must_use]
    pub fn usage(&self) -> PackageUsage {
        self.usage
    }

    #[must_use]
    pub const fn limits(&self) -> PackageLimits {
        self.limits
    }

    #[must_use]
    pub fn contains_part(&self, part_name: &PartName) -> bool {
        self.entries.contains_key(part_name)
    }

    pub fn read_part(&mut self, part_name: &PartName) -> Result<Vec<u8>, ZipPackageError> {
        let metadata = self
            .entries
            .get(part_name)
            .cloned()
            .ok_or_else(|| ZipPackageError::PartNotFound(part_name.to_string()))?;

        let mut file = self
            .archive
            .by_index(metadata.index)
            .map_err(|error| ZipPackageError::Archive(error.to_string()))?;

        let capacity = metadata.uncompressed_size.min(8 * 1024 * 1024) as usize;
        let mut output = Vec::with_capacity(capacity);
        let read_limit = self.limits.max_single_part_uncompressed.saturating_add(1);

        (&mut file)
            .take(read_limit)
            .read_to_end(&mut output)
            .map_err(|error| ZipPackageError::Io(error.to_string()))?;

        let actual = output.len() as u64;
        if actual > self.limits.max_single_part_uncompressed {
            return Err(ZipPackageError::PartReadLimitExceeded(
                part_name.to_string(),
            ));
        }
        if actual != metadata.uncompressed_size {
            return Err(ZipPackageError::SizeMismatch {
                part: part_name.to_string(),
                expected: metadata.uncompressed_size,
                actual,
            });
        }

        Ok(output)
    }

    pub fn read_content_types(&mut self) -> Result<ContentTypeMap, ZipPackageError> {
        let part_name = PartName::new("/[Content_Types].xml")?;
        let bytes = self.read_part(&part_name)?;
        parse_content_types(&bytes).map_err(Into::into)
    }

    pub fn read_relationships(
        &mut self,
        source: Option<&PartName>,
    ) -> Result<RelationshipSet, ZipPackageError> {
        let relationship_part = relationship_part_name(source)?;
        let bytes = self.read_part(&relationship_part)?;
        parse_relationships(source, &bytes).map_err(Into::into)
    }

    pub fn load_owned_package(&mut self) -> Result<Package, ZipPackageError> {
        let content_types = self.read_content_types()?;
        let mut package = Package::new(content_types);
        let part_names: Vec<PartName> = self.entries.keys().cloned().collect();

        for part_name in part_names {
            if part_name.as_str() == "/[Content_Types].xml" {
                continue;
            }

            if is_relationship_part(&part_name) {
                let source = source_part_from_relationship_part(&part_name)?;
                let relationships = self.read_relationships(source.as_ref())?;

                match source {
                    Some(source) => {
                        *package.relationships_mut(source) = relationships;
                    }
                    None => {
                        *package.package_relationships_mut() = relationships;
                    }
                }
                continue;
            }

            let bytes = self.read_part(&part_name)?;
            package.insert_part(part_name, bytes)?;
        }

        Ok(package)
    }

    /// Rebuild a validated package while raw-copying unchanged compressed entries.
    ///
    /// Replacement bytes are compressed only for changed/new Parts. Unchanged Parts are
    /// copied from their already-validated raw ZIP representation without decompression.
    pub fn rewrite_with_replacements<W: Write + Seek>(
        &mut self,
        writer: W,
        replacements: &BTreeMap<PartName, Vec<u8>>,
        removals: &BTreeSet<PartName>,
    ) -> Result<W, ZipPackageError> {
        let entries: Vec<ZipEntryMetadata> = self.entries.values().cloned().collect();
        let mut output = ZipWriter::new(writer);
        let mut written = BTreeSet::new();

        for metadata in entries {
            if removals.contains(&metadata.part_name) {
                continue;
            }

            if let Some(bytes) = replacements.get(&metadata.part_name) {
                write_zip_entry(
                    &mut output,
                    metadata.part_name.zip_entry_name(),
                    bytes,
                    zip_options(metadata.compression),
                )?;
                written.insert(metadata.part_name.clone());
                continue;
            }

            let raw = self
                .archive
                .by_index_raw(metadata.index)
                .map_err(|error| ZipPackageError::Archive(error.to_string()))?;
            output
                .raw_copy_file(raw)
                .map_err(|error| ZipPackageError::Write(error.to_string()))?;
        }

        for (part_name, bytes) in replacements {
            if written.contains(part_name) || self.entries.contains_key(part_name) {
                continue;
            }

            write_zip_entry(
                &mut output,
                part_name.zip_entry_name(),
                bytes,
                zip_options(PackageCompression::Deflated),
            )?;
        }

        output
            .finish()
            .map_err(|error| ZipPackageError::Write(error.to_string()))
    }

    pub fn repack_validated<W: Write + Seek>(&mut self, writer: W) -> Result<W, ZipPackageError> {
        let entries: Vec<ZipEntryMetadata> = self.entries.values().cloned().collect();
        let mut output = ZipWriter::new(writer);

        for metadata in entries {
            let bytes = self.read_part(&metadata.part_name)?;
            let options = zip_options(metadata.compression);
            output
                .start_file(metadata.part_name.zip_entry_name(), options)
                .map_err(|error| ZipPackageError::Write(error.to_string()))?;
            output
                .write_all(&bytes)
                .map_err(|error| ZipPackageError::Write(error.to_string()))?;
        }

        output
            .finish()
            .map_err(|error| ZipPackageError::Write(error.to_string()))
    }
}

pub fn write_owned_package<W: Write + Seek>(
    package: &Package,
    writer: W,
) -> Result<W, ZipPackageError> {
    let mut output = ZipWriter::new(writer);
    let options = zip_options(PackageCompression::Deflated);

    write_zip_entry(
        &mut output,
        "[Content_Types].xml",
        &write_content_types(package.content_types()),
        options,
    )?;

    if !package.package_relationships().is_empty() {
        write_zip_entry(
            &mut output,
            "_rels/.rels",
            &write_relationships(None, package.package_relationships()),
            options,
        )?;
    }

    for part in package.parts() {
        write_zip_entry(
            &mut output,
            part.name().zip_entry_name(),
            part.bytes(),
            options,
        )?;
    }

    for (source, relationships) in package.part_relationships() {
        if relationships.is_empty() {
            continue;
        }

        let relationship_part = relationship_part_name(Some(source))?;
        write_zip_entry(
            &mut output,
            relationship_part.zip_entry_name(),
            &write_relationships(Some(source), relationships),
            options,
        )?;
    }

    output
        .finish()
        .map_err(|error| ZipPackageError::Write(error.to_string()))
}

fn is_relationship_part(part_name: &PartName) -> bool {
    part_name.as_str() == "/_rels/.rels"
        || (part_name.as_str().contains("/_rels/") && part_name.as_str().ends_with(".rels"))
}

fn zip_options(compression: PackageCompression) -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(match compression {
        PackageCompression::Stored => CompressionMethod::Stored,
        PackageCompression::Deflated => CompressionMethod::Deflated,
    })
}

fn write_zip_entry<W: Write + Seek>(
    writer: &mut ZipWriter<W>,
    name: &str,
    bytes: &[u8],
    options: SimpleFileOptions,
) -> Result<(), ZipPackageError> {
    writer
        .start_file(name, options)
        .map_err(|error| ZipPackageError::Write(error.to_string()))?;
    writer
        .write_all(bytes)
        .map_err(|error| ZipPackageError::Write(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    use zip::ZipWriter;

    const CONTENT_TYPES: &[u8] = br#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
      <Default Extension="xml" ContentType="application/xml"/>
      <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
      <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
    </Types>"#;

    const ROOT_RELS: &[u8] =
        br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
      <Relationship Id="rId1" Type="officeDocument" Target="word/document.xml"/>
    </Relationships>"#;

    #[test]
    fn indexes_package_without_eagerly_exposing_part_bodies() {
        let bytes = build_zip(&[
            (
                "[Content_Types].xml",
                CONTENT_TYPES,
                CompressionMethod::Stored,
            ),
            ("_rels/.rels", ROOT_RELS, CompressionMethod::Deflated),
            (
                "word/document.xml",
                b"<w:document/>",
                CompressionMethod::Deflated,
            ),
        ]);

        let package = LazyZipPackage::open(Cursor::new(bytes)).unwrap();

        assert_eq!(package.entries().count(), 3);
        assert!(package.contains_part(&PartName::new("/word/document.xml").unwrap()));
    }

    #[test]
    fn reads_selected_part_on_demand() {
        let bytes = build_zip(&[(
            "word/document.xml",
            b"<w:document>hello</w:document>",
            CompressionMethod::Deflated,
        )]);
        let mut package = LazyZipPackage::open(Cursor::new(bytes)).unwrap();
        let document = PartName::new("/word/document.xml").unwrap();

        assert_eq!(
            package.read_part(&document).unwrap(),
            b"<w:document>hello</w:document>"
        );
    }

    #[test]
    fn parses_content_types_directly_from_zip() {
        let bytes = build_zip(&[(
            "[Content_Types].xml",
            CONTENT_TYPES,
            CompressionMethod::Deflated,
        )]);
        let mut package = LazyZipPackage::open(Cursor::new(bytes)).unwrap();

        let content_types = package.read_content_types().unwrap();

        assert_eq!(
            content_types.content_type_for(&PartName::new("/word/document.xml").unwrap()),
            Some(
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"
            )
        );
    }

    #[test]
    fn parses_root_relationships_directly_from_zip() {
        let bytes = build_zip(&[("_rels/.rels", ROOT_RELS, CompressionMethod::Deflated)]);
        let mut package = LazyZipPackage::open(Cursor::new(bytes)).unwrap();

        let relationships = package.read_relationships(None).unwrap();

        let relationship = relationships
            .get(&crate::RelationshipId::new("rId1"))
            .unwrap();
        assert_eq!(
            relationship.target,
            crate::RelationshipTarget::Internal(PartName::new("/word/document.xml").unwrap())
        );
    }

    #[test]
    fn selective_rewrite_preserves_unmodified_opaque_part() {
        let unknown = [0, 255, 17, 42, 0, 9];
        let bytes = build_zip(&[
            (
                "[Content_Types].xml",
                CONTENT_TYPES,
                CompressionMethod::Deflated,
            ),
            (
                "word/document.xml",
                b"<w:document>old</w:document>",
                CompressionMethod::Deflated,
            ),
            ("customXml/item1.bin", &unknown, CompressionMethod::Stored),
        ]);
        let mut package = LazyZipPackage::open(Cursor::new(bytes)).unwrap();
        let mut replacements = BTreeMap::new();
        replacements.insert(
            PartName::new("/word/document.xml").unwrap(),
            b"<w:document>new</w:document>".to_vec(),
        );

        let rewritten = package
            .rewrite_with_replacements(Cursor::new(Vec::new()), &replacements, &BTreeSet::new())
            .unwrap()
            .into_inner();
        let mut reopened = LazyZipPackage::open(Cursor::new(rewritten)).unwrap();

        assert_eq!(
            reopened
                .read_part(&PartName::new("/word/document.xml").unwrap())
                .unwrap(),
            b"<w:document>new</w:document>"
        );
        assert_eq!(
            reopened
                .read_part(&PartName::new("/customXml/item1.bin").unwrap())
                .unwrap(),
            unknown
        );
    }

    #[test]
    fn validated_repack_preserves_unknown_part_bytes() {
        let unknown = [0, 255, 17, 42, 0, 9];
        let bytes = build_zip(&[
            (
                "[Content_Types].xml",
                CONTENT_TYPES,
                CompressionMethod::Deflated,
            ),
            ("_rels/.rels", ROOT_RELS, CompressionMethod::Deflated),
            (
                "word/document.xml",
                b"<w:document/>",
                CompressionMethod::Deflated,
            ),
            ("customXml/item1.bin", &unknown, CompressionMethod::Stored),
        ]);
        let mut package = LazyZipPackage::open(Cursor::new(bytes)).unwrap();

        let repacked = package
            .repack_validated(Cursor::new(Vec::new()))
            .unwrap()
            .into_inner();
        let mut reopened = LazyZipPackage::open(Cursor::new(repacked)).unwrap();

        assert_eq!(
            reopened
                .read_part(&PartName::new("/customXml/item1.bin").unwrap())
                .unwrap(),
            unknown
        );
    }

    #[test]
    fn owned_package_round_trip_preserves_parts_and_relationships() {
        let bytes = build_zip(&[
            (
                "[Content_Types].xml",
                CONTENT_TYPES,
                CompressionMethod::Deflated,
            ),
            ("_rels/.rels", ROOT_RELS, CompressionMethod::Deflated),
            (
                "word/document.xml",
                b"<w:document/>",
                CompressionMethod::Deflated,
            ),
        ]);
        let mut lazy = LazyZipPackage::open(Cursor::new(bytes)).unwrap();
        let owned = lazy.load_owned_package().unwrap();

        let rewritten = write_owned_package(&owned, Cursor::new(Vec::new()))
            .unwrap()
            .into_inner();
        let mut reopened = LazyZipPackage::open(Cursor::new(rewritten)).unwrap();

        assert_eq!(
            reopened
                .read_part(&PartName::new("/word/document.xml").unwrap())
                .unwrap(),
            b"<w:document/>"
        );
        assert_eq!(
            reopened
                .read_relationships(None)
                .unwrap()
                .get(&crate::RelationshipId::new("rId1"))
                .unwrap()
                .target,
            crate::RelationshipTarget::Internal(PartName::new("/word/document.xml").unwrap())
        );
    }

    #[test]
    fn rejects_suspicious_deflate_ratio_from_metadata() {
        let repeated = vec![b'A'; 64 * 1024];
        let bytes = build_zip(&[("word/document.xml", &repeated, CompressionMethod::Deflated)]);
        let limits = PackageLimits {
            max_compression_ratio: 2,
            ..PackageLimits::default()
        };

        assert!(matches!(
            LazyZipPackage::new(Cursor::new(bytes), limits),
            Err(ZipPackageError::ResourceLimit(
                LimitViolation::SuspiciousCompressionRatio
            ))
        ));
    }

    fn build_zip(entries: &[(&str, &[u8], CompressionMethod)]) -> Vec<u8> {
        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);

        for (name, bytes, compression) in entries {
            let options = SimpleFileOptions::default().compression_method(*compression);
            writer.start_file(*name, options).unwrap();
            writer.write_all(bytes).unwrap();
        }

        writer.finish().unwrap().into_inner()
    }
}
