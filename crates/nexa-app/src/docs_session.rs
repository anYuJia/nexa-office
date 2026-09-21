use nexa_docs::{
    DocxDocument, DocxError, DocsEditor, EditError, Paginator, SearchOptions, Selection,
    TextPosition, open_docx, save_docx_atomic,
};
use std::{
    error::Error,
    fmt,
    fs::File,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum DocsSessionError {
    Io(std::io::Error),
    Docx(DocxError),
    Edit(EditError),
    MissingSavePath,
    EmptyPath,
    UnsupportedExtension,
    MissingParagraph(usize),
}

impl fmt::Display for DocsSessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "document I/O error: {error}"),
            Self::Docx(error) => write!(f, "{error}"),
            Self::Edit(error) => write!(f, "{error}"),
            Self::MissingSavePath => f.write_str("choose a DOCX path before saving"),
            Self::EmptyPath => f.write_str("document path is empty"),
            Self::UnsupportedExtension => f.write_str("Phase 3 Docs opens and saves .docx files"),
            Self::MissingParagraph(index) => write!(f, "document paragraph {index} does not exist"),
        }
    }
}

impl Error for DocsSessionError {}

impl From<std::io::Error> for DocsSessionError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<DocxError> for DocsSessionError {
    fn from(value: DocxError) -> Self {
        Self::Docx(value)
    }
}

impl From<EditError> for DocsSessionError {
    fn from(value: EditError) -> Self {
        Self::Edit(value)
    }
}

#[derive(Debug)]
pub struct DocsSession {
    docx: DocxDocument,
    editor: DocsEditor,
    path: Option<PathBuf>,
    dirty: bool,
    current_paragraph: usize,
}

impl DocsSession {
    #[must_use]
    pub fn blank() -> Self {
        let docx = DocxDocument::blank();
        let editor = DocsEditor::new(docx.document().clone());
        Self {
            docx,
            editor,
            path: None,
            dirty: false,
            current_paragraph: 0,
        }
    }

    pub fn open(path: impl Into<PathBuf>) -> Result<Self, DocsSessionError> {
        let path = path.into();
        validate_docx_path(&path)?;
        let file = File::open(&path)?;
        let docx = open_docx(file)?;
        let editor = DocsEditor::new(docx.document().clone());

        Ok(Self {
            docx,
            editor,
            path: Some(path),
            dirty: false,
            current_paragraph: 0,
        })
    }

    #[must_use]
    pub fn title(&self) -> String {
        self.path
            .as_deref()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled.docx")
            .to_owned()
    }

    #[must_use]
    pub fn path_text(&self) -> String {
        self.path
            .as_deref()
            .map_or_else(String::new, |path| path.to_string_lossy().into_owned())
    }

    #[must_use]
    pub const fn is_dirty(&self) -> bool {
        self.dirty
    }

    #[must_use]
    pub fn can_save(&self) -> bool {
        self.editor.document().compatibility.can_save()
    }

    #[must_use]
    pub fn compatibility_issue_count(&self) -> usize {
        self.editor.document().compatibility.issues.len()
    }

    #[must_use]
    pub fn paragraph_count(&self) -> usize {
        self.editor.document().paragraph_count()
    }

    #[must_use]
    pub const fn current_paragraph_index(&self) -> usize {
        self.current_paragraph
    }

    #[must_use]
    pub fn current_paragraph_text(&self) -> String {
        self.editor
            .document()
            .paragraph(self.current_paragraph)
            .map_or_else(String::new, |paragraph| paragraph.plain_text())
    }

    #[must_use]
    pub fn page_count(&self) -> usize {
        Paginator::default().layout(self.editor.document()).page_count()
    }

    #[must_use]
    pub fn undo_depth(&self) -> usize {
        self.editor.undo_depth()
    }

    #[must_use]
    pub fn current_bold(&self) -> bool {
        self.current_run_flag(|properties| properties.bold)
    }

    #[must_use]
    pub fn current_italic(&self) -> bool {
        self.current_run_flag(|properties| properties.italic)
    }

    #[must_use]
    pub fn current_underline(&self) -> bool {
        self.current_run_flag(|properties| properties.underline)
    }

    pub fn set_current_paragraph_text(&mut self, text: &str) -> Result<(), DocsSessionError> {
        let len = self.current_paragraph_len()?;
        self.editor.set_selection(Selection {
            anchor: TextPosition {
                paragraph: self.current_paragraph,
                offset: 0,
            },
            active: TextPosition {
                paragraph: self.current_paragraph,
                offset: len,
            },
        })?;
        self.editor.insert_text(text)?;
        self.dirty = true;
        Ok(())
    }

    pub fn previous_paragraph(&mut self) {
        self.current_paragraph = self.current_paragraph.saturating_sub(1);
    }

    pub fn next_paragraph(&mut self) {
        let last = self.paragraph_count().saturating_sub(1);
        self.current_paragraph = self.current_paragraph.saturating_add(1).min(last);
    }

    pub fn toggle_bold(&mut self) -> Result<(), DocsSessionError> {
        self.select_current_paragraph()?;
        self.editor.toggle_bold()?;
        self.dirty = true;
        Ok(())
    }

    pub fn toggle_italic(&mut self) -> Result<(), DocsSessionError> {
        self.select_current_paragraph()?;
        self.editor.toggle_italic()?;
        self.dirty = true;
        Ok(())
    }

    pub fn toggle_underline(&mut self) -> Result<(), DocsSessionError> {
        self.select_current_paragraph()?;
        self.editor.toggle_underline()?;
        self.dirty = true;
        Ok(())
    }

    pub fn search_count(&self, query: &str) -> Result<usize, DocsSessionError> {
        Ok(self
            .editor
            .find_all(
                query,
                SearchOptions {
                    case_sensitive: false,
                },
            )?
            .len())
    }

    pub fn replace_all(
        &mut self,
        query: &str,
        replacement: &str,
    ) -> Result<usize, DocsSessionError> {
        let count = self.editor.replace_all(
            query,
            replacement,
            SearchOptions {
                case_sensitive: false,
            },
        )?;
        if count > 0 {
            self.dirty = true;
            self.current_paragraph = self
                .current_paragraph
                .min(self.paragraph_count().saturating_sub(1));
        }
        Ok(count)
    }

    pub fn undo(&mut self) -> Result<bool, DocsSessionError> {
        let changed = self.editor.undo()?;
        if changed {
            self.dirty = true;
            self.current_paragraph = self
                .editor
                .selection()
                .active
                .paragraph
                .min(self.paragraph_count().saturating_sub(1));
        }
        Ok(changed)
    }

    pub fn redo(&mut self) -> Result<bool, DocsSessionError> {
        let changed = self.editor.redo()?;
        if changed {
            self.dirty = true;
            self.current_paragraph = self
                .editor
                .selection()
                .active
                .paragraph
                .min(self.paragraph_count().saturating_sub(1));
        }
        Ok(changed)
    }

    pub fn save(&mut self) -> Result<(), DocsSessionError> {
        let path = self.path.clone().ok_or(DocsSessionError::MissingSavePath)?;
        self.save_to(path)
    }

    pub fn save_as(&mut self, path: impl Into<PathBuf>) -> Result<(), DocsSessionError> {
        self.save_to(path.into())
    }

    fn save_to(&mut self, path: PathBuf) -> Result<(), DocsSessionError> {
        validate_docx_path(&path)?;
        self.docx.replace_document(self.editor.document().clone());
        save_docx_atomic(&mut self.docx, &path)?;
        self.path = Some(path);
        self.dirty = false;
        Ok(())
    }

    fn current_paragraph_len(&self) -> Result<usize, DocsSessionError> {
        self.editor
            .document()
            .paragraph(self.current_paragraph)
            .map(|paragraph| paragraph.logical_len())
            .ok_or(DocsSessionError::MissingParagraph(self.current_paragraph))
    }

    fn select_current_paragraph(&mut self) -> Result<(), DocsSessionError> {
        let len = self.current_paragraph_len()?;
        self.editor.set_selection(Selection {
            anchor: TextPosition {
                paragraph: self.current_paragraph,
                offset: 0,
            },
            active: TextPosition {
                paragraph: self.current_paragraph,
                offset: len,
            },
        })?;
        Ok(())
    }

    fn current_run_flag(&self, read: impl Fn(&nexa_docs::RunProperties) -> bool) -> bool {
        self.editor
            .document()
            .paragraph(self.current_paragraph)
            .and_then(|paragraph| paragraph.runs.first())
            .is_some_and(|run| read(&run.properties))
    }
}

fn validate_docx_path(path: &Path) -> Result<(), DocsSessionError> {
    if path.as_os_str().is_empty() {
        return Err(DocsSessionError::EmptyPath);
    }
    if !path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("docx"))
    {
        return Err(DocsSessionError::UnsupportedExtension);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        process,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn blank_session_edits_saves_and_reopens() {
        let directory = unique_test_directory();
        let path = directory.join("phase3.docx");
        let mut session = DocsSession::blank();

        session.set_current_paragraph_text("Nexa Docs").unwrap();
        session.toggle_bold().unwrap();
        session.save_as(path.clone()).unwrap();

        assert!(!session.is_dirty());
        let reopened = DocsSession::open(path.clone()).unwrap();
        assert_eq!(reopened.current_paragraph_text(), "Nexa Docs");
        assert!(reopened.current_bold());

        let _ = fs::remove_file(path);
        let _ = fs::remove_dir(directory);
    }

    #[test]
    fn search_replace_and_undo_use_docs_editor_history() {
        let mut session = DocsSession::blank();
        session
            .set_current_paragraph_text("alpha beta alpha")
            .unwrap();

        assert_eq!(session.search_count("ALPHA").unwrap(), 2);
        assert_eq!(session.replace_all("alpha", "x").unwrap(), 2);
        assert_eq!(session.current_paragraph_text(), "x beta x");
        assert!(session.undo().unwrap());
        assert_eq!(session.current_paragraph_text(), "alpha beta alpha");
    }

    #[test]
    fn save_without_destination_is_rejected() {
        let mut session = DocsSession::blank();
        assert!(matches!(
            session.save(),
            Err(DocsSessionError::MissingSavePath)
        ));
    }

    fn unique_test_directory() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should follow Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("nexa-docs-session-{}-{nonce}", process::id()))
    }
}
