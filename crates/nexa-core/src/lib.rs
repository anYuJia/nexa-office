#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

const MAX_RECENT_FILES: usize = 8;

/// Top-level application pages owned by the shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppPage {
    Home,
    Diagnostics,
    Settings,
}

/// Editor surfaces exposed by the application shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorKind {
    Docs,
    Sheets,
    Slides,
}

impl EditorKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Docs => "Docs",
            Self::Sheets => "Sheets",
            Self::Slides => "Slides",
        }
    }
}

/// Small settings set used while Phase 1 establishes the native shell.
///
/// The format intentionally remains simple and dependency-free. A richer schema can
/// replace it later without leaking persistence concerns into the UI layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppSettings {
    reopen_last_session: bool,
    autosave_enabled: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            reopen_last_session: true,
            autosave_enabled: true,
        }
    }
}

impl AppSettings {
    #[must_use]
    pub const fn reopen_last_session(&self) -> bool {
        self.reopen_last_session
    }

    #[must_use]
    pub const fn autosave_enabled(&self) -> bool {
        self.autosave_enabled
    }

    #[must_use]
    pub fn encode(&self) -> String {
        format!(
            "reopen_last_session={}\nautosave_enabled={}\n",
            self.reopen_last_session, self.autosave_enabled
        )
    }

    #[must_use]
    pub fn decode(input: &str) -> Self {
        let mut settings = Self::default();

        for line in input.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };

            let parsed = match value.trim() {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            };

            match (key.trim(), parsed) {
                ("reopen_last_session", Some(value)) => settings.reopen_last_session = value,
                ("autosave_enabled", Some(value)) => settings.autosave_enabled = value,
                _ => {}
            }
        }

        settings
    }
}

/// Commands emitted by the application shell.
///
/// Phase 1 deliberately keeps these commands small. File-format and editor-specific
/// commands belong in their future domain crates rather than the UI layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppCommand {
    Navigate(AppPage),
    New(EditorKind),
    OpenFile(PathBuf),
    SetReopenLastSession(bool),
    SetAutosaveEnabled(bool),
}

/// UI-independent application state used by the Phase 1 shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppState {
    page: AppPage,
    active_editor: Option<EditorKind>,
    settings: AppSettings,
    recent_files: Vec<PathBuf>,
    status: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self::with_settings(AppSettings::default())
    }
}

impl AppState {
    #[must_use]
    pub fn with_settings(settings: AppSettings) -> Self {
        Self {
            page: AppPage::Home,
            active_editor: None,
            settings,
            recent_files: Vec::new(),
            status: "Native shell ready".to_owned(),
        }
    }

    pub fn apply(&mut self, command: AppCommand) {
        match command {
            AppCommand::Navigate(page) => {
                self.page = page;
                self.active_editor = None;
                self.status = match page {
                    AppPage::Home => "Home".to_owned(),
                    AppPage::Diagnostics => "Diagnostics".to_owned(),
                    AppPage::Settings => "Settings".to_owned(),
                };
            }
            AppCommand::New(editor) => {
                self.page = AppPage::Home;
                self.active_editor = Some(editor);
                self.status = format!("{} engine is not enabled in Phase 1", editor.label());
            }
            AppCommand::OpenFile(path) => {
                self.page = AppPage::Home;
                self.active_editor = None;
                self.remember_file(path.clone());
                self.status = match file_name(&path) {
                    Some(name) => format!("Open pipeline reserved for: {name}"),
                    None => "Open pipeline reserved for selected file".to_owned(),
                };
            }
            AppCommand::SetReopenLastSession(value) => {
                self.settings.reopen_last_session = value;
                self.status = if value {
                    "Reopen last session enabled"
                } else {
                    "Reopen last session disabled"
                }
                .to_owned();
            }
            AppCommand::SetAutosaveEnabled(value) => {
                self.settings.autosave_enabled = value;
                self.status = if value {
                    "Autosave enabled"
                } else {
                    "Autosave disabled"
                }
                .to_owned();
            }
        }
    }

    fn remember_file(&mut self, path: PathBuf) {
        self.recent_files.retain(|existing| existing != &path);
        self.recent_files.insert(0, path);
        self.recent_files.truncate(MAX_RECENT_FILES);
    }

    #[must_use]
    pub const fn page(&self) -> AppPage {
        self.page
    }

    #[must_use]
    pub const fn active_editor(&self) -> Option<EditorKind> {
        self.active_editor
    }

    #[must_use]
    pub const fn settings(&self) -> &AppSettings {
        &self.settings
    }

    #[must_use]
    pub fn recent_files(&self) -> &[PathBuf] {
        &self.recent_files
    }

    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }
}

fn file_name(path: &Path) -> Option<&str> {
    path.file_name().and_then(|name| name.to_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_editor_updates_state_without_ui_dependency() {
        let mut state = AppState::default();

        state.apply(AppCommand::New(EditorKind::Docs));

        assert_eq!(state.active_editor(), Some(EditorKind::Docs));
        assert_eq!(state.status(), "Docs engine is not enabled in Phase 1");
    }

    #[test]
    fn navigation_clears_active_editor() {
        let mut state = AppState::default();
        state.apply(AppCommand::New(EditorKind::Sheets));

        state.apply(AppCommand::Navigate(AppPage::Diagnostics));

        assert_eq!(state.page(), AppPage::Diagnostics);
        assert_eq!(state.active_editor(), None);
        assert_eq!(state.status(), "Diagnostics");
    }

    #[test]
    fn open_file_reports_only_the_file_name() {
        let mut state = AppState::default();

        state.apply(AppCommand::OpenFile(PathBuf::from(
            "/private/example/quarterly.xlsx",
        )));

        assert_eq!(state.status(), "Open pipeline reserved for: quarterly.xlsx");
        assert!(!state.status().contains("/private/example"));
    }

    #[test]
    fn recent_files_are_deduplicated_and_bounded() {
        let mut state = AppState::default();

        for index in 0..10 {
            state.apply(AppCommand::OpenFile(PathBuf::from(format!(
                "/tmp/document-{index}.docx"
            ))));
        }
        state.apply(AppCommand::OpenFile(PathBuf::from("/tmp/document-8.docx")));

        assert_eq!(state.recent_files().len(), MAX_RECENT_FILES);
        assert_eq!(
            state.recent_files().first(),
            Some(&PathBuf::from("/tmp/document-8.docx"))
        );
        assert_eq!(
            state
                .recent_files()
                .iter()
                .filter(|path| path.ends_with("document-8.docx"))
                .count(),
            1
        );
    }

    #[test]
    fn settings_round_trip_unknown_keys_safely() {
        let source = "reopen_last_session=false\nfuture_key=42\nautosave_enabled=true\n";

        let settings = AppSettings::decode(source);

        assert!(!settings.reopen_last_session());
        assert!(settings.autosave_enabled());
        assert_eq!(AppSettings::decode(&settings.encode()), settings);
    }

    #[test]
    fn settings_commands_update_only_the_requested_value() {
        let mut state = AppState::default();

        state.apply(AppCommand::SetAutosaveEnabled(false));

        assert!(!state.settings().autosave_enabled());
        assert!(state.settings().reopen_last_session());
    }
}
