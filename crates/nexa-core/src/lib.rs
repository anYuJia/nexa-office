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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppLanguage {
    System,
    English,
    SimplifiedChinese,
}

impl AppLanguage {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::English => "en",
            Self::SimplifiedChinese => "zh-CN",
        }
    }

    #[must_use]
    pub fn from_code(value: &str) -> Self {
        match value.trim() {
            "en" | "en-US" | "en-GB" => Self::English,
            "zh" | "zh-CN" | "zh-Hans" => Self::SimplifiedChinese,
            _ => Self::System,
        }
    }
}

/// Small settings set used while Phase 1 establishes the native shell.
///
/// The format intentionally remains simple and dependency-free. Phase 1 settings
/// affect shell behavior immediately instead of pretending future editor features exist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppSettings {
    show_status_bar: bool,
    compact_navigation: bool,
    language: AppLanguage,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            show_status_bar: true,
            compact_navigation: false,
            language: AppLanguage::System,
        }
    }
}

impl AppSettings {
    #[must_use]
    pub const fn show_status_bar(&self) -> bool {
        self.show_status_bar
    }

    #[must_use]
    pub const fn compact_navigation(&self) -> bool {
        self.compact_navigation
    }

    #[must_use]
    pub const fn language(&self) -> AppLanguage {
        self.language
    }

    #[must_use]
    pub fn encode(&self) -> String {
        format!(
            "show_status_bar={}\ncompact_navigation={}\nlanguage={}\n",
            self.show_status_bar,
            self.compact_navigation,
            self.language.code()
        )
    }

    #[must_use]
    pub fn decode(input: &str) -> Self {
        let mut settings = Self::default();

        for line in input.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };

            match key.trim() {
                "show_status_bar" => {
                    if let Ok(value) = value.trim().parse::<bool>() {
                        settings.show_status_bar = value;
                    }
                }
                "compact_navigation" => {
                    if let Ok(value) = value.trim().parse::<bool>() {
                        settings.compact_navigation = value;
                    }
                }
                "language" => settings.language = AppLanguage::from_code(value),
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
    SetShowStatusBar(bool),
    SetCompactNavigation(bool),
    SetLanguage(AppLanguage),
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
                self.status = match editor {
                    EditorKind::Docs => "New Docs document".to_owned(),
                    EditorKind::Sheets => "New Sheets workbook".to_owned(),
                    EditorKind::Slides => "New Slides presentation".to_owned(),
                };
            }
            AppCommand::OpenFile(path) => {
                self.page = AppPage::Home;
                self.active_editor = editor_for_path(&path);
                self.remember_file(path.clone());
                self.status = match file_name(&path) {
                    Some(name) => format!("Opened {name}"),
                    None => "Opened selected file".to_owned(),
                };
            }
            AppCommand::SetShowStatusBar(value) => {
                self.settings.show_status_bar = value;
                self.status = if value {
                    "Status bar shown"
                } else {
                    "Status bar hidden"
                }
                .to_owned();
            }
            AppCommand::SetCompactNavigation(value) => {
                self.settings.compact_navigation = value;
                self.status = if value {
                    "Compact navigation enabled"
                } else {
                    "Compact navigation disabled"
                }
                .to_owned();
            }
            AppCommand::SetLanguage(value) => {
                self.settings.language = value;
                self.status = "Language preference updated".to_owned();
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

    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
    }
}

fn editor_for_path(path: &Path) -> Option<EditorKind> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("docx") => Some(EditorKind::Docs),
        Some("xlsx") => Some(EditorKind::Sheets),
        Some("pptx") => Some(EditorKind::Slides),
        _ => None,
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
        assert_eq!(state.status(), "New Docs document");
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

        assert_eq!(state.status(), "Opened quarterly.xlsx");
        assert_eq!(state.active_editor(), Some(EditorKind::Sheets));
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
        let source =
            "show_status_bar=false\nfuture_key=42\ncompact_navigation=true\nlanguage=zh-CN\n";

        let settings = AppSettings::decode(source);

        assert!(!settings.show_status_bar());
        assert!(settings.compact_navigation());
        assert_eq!(settings.language(), AppLanguage::SimplifiedChinese);
        assert_eq!(AppSettings::decode(&settings.encode()), settings);
    }

    #[test]
    fn language_setting_round_trips_and_defaults_safely() {
        assert_eq!(AppSettings::default().language(), AppLanguage::System);
        assert_eq!(
            AppSettings::decode("language=en\n").language(),
            AppLanguage::English
        );
        assert_eq!(
            AppSettings::decode("language=unknown\n").language(),
            AppLanguage::System
        );
    }

    #[test]
    fn settings_commands_update_only_the_requested_value() {
        let mut state = AppState::default();

        state.apply(AppCommand::SetCompactNavigation(true));

        assert!(state.settings().compact_navigation());
        assert!(state.settings().show_status_bar());
    }
}
