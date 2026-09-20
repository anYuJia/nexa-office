#![forbid(unsafe_code)]

use std::path::PathBuf;

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

/// Commands emitted by the application shell.
///
/// Phase 1 deliberately keeps these commands small. File-format and editor-specific
/// commands belong in their future domain crates rather than the UI layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppCommand {
    GoHome,
    New(EditorKind),
    OpenFile(PathBuf),
}

/// Small, UI-independent application state used by the Phase 1 shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppState {
    active_editor: Option<EditorKind>,
    status: String,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            active_editor: None,
            status: "Native shell ready".to_owned(),
        }
    }
}

impl AppState {
    pub fn apply(&mut self, command: AppCommand) {
        match command {
            AppCommand::GoHome => {
                self.active_editor = None;
                self.status = "Home".to_owned();
            }
            AppCommand::New(editor) => {
                self.active_editor = Some(editor);
                self.status = format!("{} engine is not enabled in Phase 1", editor.label());
            }
            AppCommand::OpenFile(path) => {
                self.active_editor = None;
                self.status = match path.file_name().and_then(|name| name.to_str()) {
                    Some(name) => format!("Open pipeline reserved for: {name}"),
                    None => "Open pipeline reserved for selected file".to_owned(),
                };
            }
        }
    }

    #[must_use]
    pub const fn active_editor(&self) -> Option<EditorKind> {
        self.active_editor
    }

    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }
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
    fn returning_home_clears_active_editor() {
        let mut state = AppState::default();
        state.apply(AppCommand::New(EditorKind::Sheets));

        state.apply(AppCommand::GoHome);

        assert_eq!(state.active_editor(), None);
        assert_eq!(state.status(), "Home");
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
}
