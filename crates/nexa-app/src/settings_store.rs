use nexa_core::AppSettings;
use std::{
    fs,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
};

pub fn load(path: &Path) -> io::Result<AppSettings> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(AppSettings::decode(&contents)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(AppSettings::default()),
        Err(error) => Err(error),
    }
}

pub fn save(path: &Path, settings: &AppSettings) -> io::Result<()> {
    let Some(parent) = path.parent() else {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "settings path has no parent directory",
        ));
    };

    fs::create_dir_all(parent)?;

    let temporary = temporary_path(path);
    fs::write(&temporary, settings.encode())?;

    if let Err(error) = replace_file(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn replace_file(source: &Path, destination: &Path) -> io::Result<()> {
    if destination.exists() {
        fs::remove_file(destination)?;
    }
    fs::rename(source, destination)
}

#[cfg(not(target_os = "windows"))]
fn replace_file(source: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(source, destination)
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut temporary = path.as_os_str().to_os_string();
    temporary.push(".tmp");
    PathBuf::from(temporary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn missing_settings_file_uses_defaults() {
        let path = unique_test_path("missing");
        let settings = load(&path).expect("missing settings should not be an error");

        assert!(settings.show_status_bar());
        assert!(!settings.compact_navigation());
    }

    #[test]
    fn saved_settings_can_be_loaded_again() {
        let path = unique_test_path("round-trip");
        let settings = AppSettings::decode("show_status_bar=false\ncompact_navigation=true\n");

        save(&path, &settings).expect("settings should save");
        let loaded = load(&path).expect("settings should load");

        assert_eq!(loaded, settings);
        cleanup(&path);
    }

    #[test]
    fn existing_settings_can_be_replaced() {
        let path = unique_test_path("replace");
        let first = AppSettings::decode("show_status_bar=true\ncompact_navigation=false\n");
        let second = AppSettings::decode("show_status_bar=false\ncompact_navigation=true\n");

        save(&path, &first).expect("initial settings should save");
        save(&path, &second).expect("existing settings should be replaceable");

        assert_eq!(load(&path).expect("replaced settings should load"), second);
        cleanup(&path);
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_file(path);
        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir(parent);
        }
    }

    fn unique_test_path(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after UNIX epoch")
            .as_nanos();

        std::env::temp_dir()
            .join(format!("nexa-office-{label}-{nonce}"))
            .join("settings.conf")
    }
}
