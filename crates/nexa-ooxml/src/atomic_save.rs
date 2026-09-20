use crate::{Package, ZipPackageError, write_owned_package};
use std::{
    error::Error,
    fmt,
    fs::{self, File},
    io,
    path::{Path, PathBuf},
    process,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtomicSaveError {
    InvalidDestination,
    Io(String),
    Zip(ZipPackageError),
}

impl fmt::Display for AtomicSaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDestination => f.write_str("invalid package save destination"),
            Self::Io(message) => write!(f, "package save I/O error: {message}"),
            Self::Zip(error) => write!(f, "package serialization failed: {error}"),
        }
    }
}

impl Error for AtomicSaveError {}

impl From<io::Error> for AtomicSaveError {
    fn from(value: io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

impl From<ZipPackageError> for AtomicSaveError {
    fn from(value: ZipPackageError) -> Self {
        Self::Zip(value)
    }
}

pub fn save_package_atomic(package: &Package, destination: &Path) -> Result<(), AtomicSaveError> {
    let parent = destination
        .parent()
        .ok_or(AtomicSaveError::InvalidDestination)?;
    let file_name = destination
        .file_name()
        .ok_or(AtomicSaveError::InvalidDestination)?;

    fs::create_dir_all(parent)?;

    let temporary = sibling_path(parent, file_name, "tmp");
    let result = save_to_temporary(package, &temporary)
        .and_then(|()| replace_destination(&temporary, destination));

    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }

    result?;

    #[cfg(unix)]
    {
        File::open(parent)?.sync_all()?;
    }

    Ok(())
}

fn save_to_temporary(package: &Package, temporary: &Path) -> Result<(), AtomicSaveError> {
    let file = File::create(temporary)?;
    let file = write_owned_package(package, file)?;
    file.sync_all()?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn replace_destination(temporary: &Path, destination: &Path) -> Result<(), AtomicSaveError> {
    fs::rename(temporary, destination)?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn replace_destination(temporary: &Path, destination: &Path) -> Result<(), AtomicSaveError> {
    if !destination.exists() {
        fs::rename(temporary, destination)?;
        return Ok(());
    }

    let parent = destination
        .parent()
        .ok_or(AtomicSaveError::InvalidDestination)?;
    let file_name = destination
        .file_name()
        .ok_or(AtomicSaveError::InvalidDestination)?;
    let backup = sibling_path(parent, file_name, "bak");

    let _ = fs::remove_file(&backup);
    fs::rename(destination, &backup)?;

    match fs::rename(temporary, destination) {
        Ok(()) => {
            let _ = fs::remove_file(backup);
            Ok(())
        }
        Err(error) => {
            let _ = fs::rename(&backup, destination);
            Err(error.into())
        }
    }
}

fn sibling_path(parent: &Path, file_name: &std::ffi::OsStr, suffix: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let mut name = file_name.to_os_string();
    name.push(format!(".nexa-{}-{nonce}.{suffix}", process::id()));
    parent.join(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ContentTypeMap, ContentTypeRule, LazyZipPackage, PartName};
    use std::io::Cursor;

    #[test]
    fn saves_and_replaces_existing_package_without_losing_valid_output() {
        let directory = unique_test_directory();
        let destination = directory.join("document.docx");

        let first = package_with_payload(b"first");
        save_package_atomic(&first, &destination).unwrap();

        let second = package_with_payload(b"second");
        save_package_atomic(&second, &destination).unwrap();

        let bytes = fs::read(&destination).unwrap();
        let mut reopened = LazyZipPackage::open(Cursor::new(bytes)).unwrap();
        assert_eq!(
            reopened
                .read_part(&PartName::new("/word/document.xml").unwrap())
                .unwrap(),
            b"second"
        );

        let _ = fs::remove_file(&destination);
        let _ = fs::remove_dir(&directory);
    }

    fn package_with_payload(payload: &[u8]) -> Package {
        let document = PartName::new("/word/document.xml").unwrap();
        let mut content_types = ContentTypeMap::default();
        content_types.insert(ContentTypeRule::Default {
            extension: "xml".into(),
            content_type: "application/xml".into(),
        });
        content_types.insert(ContentTypeRule::Override {
            part_name: document.clone(),
            content_type:
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"
                    .into(),
        });

        let mut package = Package::new(content_types);
        package.insert_part(document, payload.to_vec()).unwrap();
        package
    }

    fn unique_test_directory() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should follow the Unix epoch")
            .as_nanos();

        std::env::temp_dir().join(format!("nexa-ooxml-atomic-save-{}-{nonce}", process::id()))
    }
}
