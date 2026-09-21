use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

const SNAPSHOT_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryKind {
    Docs,
    Sheets,
    Slides,
}

impl RecoveryKind {
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Docs => "docx",
            Self::Sheets => "xlsx",
            Self::Slides => "pptx",
        }
    }

    const fn stem(self) -> &'static str {
        match self {
            Self::Docs => "docs",
            Self::Sheets => "sheets",
            Self::Slides => "slides",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryCandidate {
    pub kind: RecoveryKind,
    pub snapshot: PathBuf,
    pub original: Option<PathBuf>,
}

pub fn latest_candidate(root: &Path) -> io::Result<Option<RecoveryCandidate>> {
    let mut best: Option<(SystemTime, RecoveryCandidate)> = None;
    for kind in [
        RecoveryKind::Docs,
        RecoveryKind::Sheets,
        RecoveryKind::Slides,
    ] {
        let Some(value) = candidate(root, kind)? else {
            continue;
        };
        let modified = fs::metadata(&value.snapshot)
            .and_then(|metadata| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        if best.as_ref().is_none_or(|(current, _)| modified > *current) {
            best = Some((modified, value));
        }
    }
    Ok(best.map(|(_, value)| value))
}

pub fn candidate(root: &Path, kind: RecoveryKind) -> io::Result<Option<RecoveryCandidate>> {
    let snapshot = snapshot_path(root, kind);
    if !snapshot.exists() {
        return Ok(None);
    }
    let metadata = metadata_path(root, kind);
    let original = match fs::read_to_string(metadata) {
        Ok(value) if !value.trim().is_empty() => Some(PathBuf::from(value.trim())),
        Ok(_) => None,
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    Ok(Some(RecoveryCandidate {
        kind,
        snapshot,
        original,
    }))
}

pub fn snapshot_path(root: &Path, kind: RecoveryKind) -> PathBuf {
    root.join(format!("{}.recovery.{}", kind.stem(), kind.extension()))
}

pub fn should_snapshot(root: &Path, kind: RecoveryKind) -> bool {
    let path = snapshot_path(root, kind);
    let Ok(metadata) = fs::metadata(path) else {
        return true;
    };
    let Ok(modified) = metadata.modified() else {
        return true;
    };
    SystemTime::now()
        .duration_since(modified)
        .map_or(true, |age| age >= SNAPSHOT_INTERVAL)
}

pub fn record(root: &Path, kind: RecoveryKind, original: Option<&Path>) -> io::Result<()> {
    fs::create_dir_all(root)?;
    let metadata = metadata_path(root, kind);
    fs::write(
        metadata,
        original
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default(),
    )
}

pub fn clear(root: &Path, kind: RecoveryKind) -> io::Result<()> {
    for path in [snapshot_path(root, kind), metadata_path(root, kind)] {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn metadata_path(root: &Path, kind: RecoveryKind) -> PathBuf {
    root.join(format!("{}.recovery.meta", kind.stem()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        process,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn recovery_metadata_round_trips_and_clears() {
        let root = unique_directory();
        fs::create_dir_all(&root).unwrap();
        let snapshot = snapshot_path(&root, RecoveryKind::Docs);
        fs::write(&snapshot, b"fixture").unwrap();
        record(
            &root,
            RecoveryKind::Docs,
            Some(Path::new("/tmp/original.docx")),
        )
        .unwrap();

        let value = candidate(&root, RecoveryKind::Docs).unwrap().unwrap();
        assert_eq!(value.snapshot, snapshot);
        assert_eq!(value.original, Some(PathBuf::from("/tmp/original.docx")));

        clear(&root, RecoveryKind::Docs).unwrap();
        assert!(candidate(&root, RecoveryKind::Docs).unwrap().is_none());
        let _ = fs::remove_dir(root);
    }

    fn unique_directory() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should follow Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("nexa-recovery-{}-{nonce}", process::id()))
    }
}
