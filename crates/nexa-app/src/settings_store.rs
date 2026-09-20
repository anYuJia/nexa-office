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

    match fs::rename(&temporary, path) {
        Ok(()) => Ok(()),
        Err(error) if cfg!(target_os = "windows") && error.kind() == ErrorKind::AlreadyExists => {
            fs::remove_file(path)?;
            fs::rename(temporary, path)
        }
        Err(error) => {
            let _ = fs::remove_file(temporary);
            Err(error)
        }
    }
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

        assert!(settings.reopen_last_session());
        assert!(settings.autosave_enabled());
    }

    #[test]
    fn saved_settings_can_be_loaded_again() {
        let path = unique_test_path("round-trip");
        let settings = AppSettings::decode("reopen_last_session=false\nautosave_enabled=false\n");

        save(&path, &settings).expect("settings should save");
        let loaded = load(&path).expect("settings should load");

        assert_eq!(loaded, settings);

        let _ = fs::remove_file(&path);
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
