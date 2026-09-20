use crate::{
    ContentTypeMap, LimitViolation, PackageLimits, PackageUsage, PartName, PartNameError,
    RelationshipSet, XmlParseError, parse_content_types, parse_relationships,
    relationship_part_name,
};
use std::{
    collections::BTreeMap,
    error::Error,
    fmt,
    io::{Read, Seek},
};
use zip::{CompressionMethod, ZipArchive};

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
        let read_limit = self
            .limits
            .max_single_part_uncompressed
            .checked_add(1)
            .unwrap_or(u64::MAX);

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    use zip::{ZipWriter, write::SimpleFileOptions};

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
    fn rejects_duplicate_part_names() {
        let bytes = build_zip(&[
            ("word/document.xml", b"first", CompressionMethod::Stored),
            ("word/document.xml", b"second", CompressionMethod::Stored),
        ]);

        assert!(matches!(
            LazyZipPackage::open(Cursor::new(bytes)),
            Err(ZipPackageError::DuplicatePart(_))
        ));
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
