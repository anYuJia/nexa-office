use std::{
    fs,
    io,
    path::{Path, PathBuf},
};

const MAX_RECENT_FILES: usize = 8;

pub fn load(path: &Path) -> io::Result<Vec<PathBuf>> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(contents
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(PathBuf::from)
            .take(MAX_RECENT_FILES)
            .collect()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error),
    }
}

pub fn save(path: &Path, recent_files: &[PathBuf]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut body = String::new();
    for file in recent_files.iter().take(MAX_RECENT_FILES) {
        let value = file.to_string_lossy();
        if !value.contains(['\n', '\r']) {
            body.push_str(&value);
            body.push('\n');
        }
    }
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, body)?;
    fs::rename(temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        process,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn missing_recent_store_is_empty_and_round_trips() {
        let path = unique_path();
        assert!(load(&path).unwrap().is_empty());

        let recent = vec![
            PathBuf::from("/tmp/one.docx"),
            PathBuf::from("/tmp/two.xlsx"),
        ];
        save(&path, &recent).unwrap();
        assert_eq!(load(&path).unwrap(), recent);
        let _ = fs::remove_file(path);
    }

    fn unique_path() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should follow Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("nexa-recent-{}-{nonce}.txt", process::id()))
    }
}
