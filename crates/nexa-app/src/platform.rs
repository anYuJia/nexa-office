use std::{env, path::PathBuf};

#[derive(Debug, Clone, Copy)]
pub struct PlatformInfo {
    pub os: &'static str,
    pub architecture: &'static str,
    pub build_profile: &'static str,
    pub renderer: &'static str,
    pub version: &'static str,
}

impl PlatformInfo {
    #[must_use]
    pub fn current() -> Self {
        Self {
            os: env::consts::OS,
            architecture: env::consts::ARCH,
            build_profile: if cfg!(debug_assertions) {
                "Debug"
            } else {
                "Release"
            },
            renderer: "Slint software renderer",
            version: env!("CARGO_PKG_VERSION"),
        }
    }
}

#[must_use]
pub fn settings_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        return env::var_os("APPDATA")
            .map(PathBuf::from)
            .map(|path| path.join("Nexa Office").join("settings.conf"));
    }

    #[cfg(target_os = "macos")]
    {
        return env::var_os("HOME").map(PathBuf::from).map(|path| {
            path.join("Library")
                .join("Application Support")
                .join("Nexa Office")
                .join("settings.conf")
        });
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(path) = env::var_os("XDG_CONFIG_HOME") {
            return Some(PathBuf::from(path).join("nexa-office").join("settings.conf"));
        }

        return env::var_os("HOME")
            .map(PathBuf::from)
            .map(|path| path.join(".config").join("nexa-office").join("settings.conf"));
    }

    #[allow(unreachable_code)]
    None
}
