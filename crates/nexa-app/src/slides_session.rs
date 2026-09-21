use nexa_slides::{
    PptxError, PptxPresentation, PresentationError, ShapeKind, SlideElement, open_pptx,
    save_pptx_atomic,
};
use std::{
    error::Error,
    fmt,
    fs::File,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum SlidesSessionError {
    Io(std::io::Error),
    Pptx(PptxError),
    Presentation(PresentationError),
    MissingSavePath,
    EmptyPath,
    UnsupportedExtension,
    MissingElement(usize),
}

impl fmt::Display for SlidesSessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "presentation I/O error: {error}"),
            Self::Pptx(error) => write!(f, "{error}"),
            Self::Presentation(error) => write!(f, "{error}"),
            Self::MissingSavePath => f.write_str("choose a PPTX path before saving"),
            Self::EmptyPath => f.write_str("presentation path is empty"),
            Self::UnsupportedExtension => f.write_str("Nexa Slides opens and saves .pptx files"),
            Self::MissingElement(index) => write!(f, "slide element {index} does not exist"),
        }
    }
}

impl Error for SlidesSessionError {}

impl From<std::io::Error> for SlidesSessionError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<PptxError> for SlidesSessionError {
    fn from(value: PptxError) -> Self {
        Self::Pptx(value)
    }
}

impl From<PresentationError> for SlidesSessionError {
    fn from(value: PresentationError) -> Self {
        Self::Presentation(value)
    }
}

#[derive(Debug, Clone)]
pub struct SlideElementSummary {
    pub index: usize,
    pub kind: String,
    pub text: String,
}

#[derive(Debug)]
pub struct SlidesSession {
    pptx: PptxPresentation,
    path: Option<PathBuf>,
    dirty: bool,
    selected_element: Option<usize>,
}

impl SlidesSession {
    #[must_use]
    pub fn blank() -> Self {
        Self {
            pptx: PptxPresentation::blank(),
            path: None,
            dirty: false,
            selected_element: None,
        }
    }

    pub fn open(path: impl Into<PathBuf>) -> Result<Self, SlidesSessionError> {
        let path = path.into();
        validate_pptx_path(&path)?;
        let file = File::open(&path)?;
        let pptx = open_pptx(file)?;
        Ok(Self {
            pptx,
            path: Some(path),
            dirty: false,
            selected_element: None,
        })
    }

    #[must_use]
    pub fn title(&self) -> String {
        self.path
            .as_deref()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled.pptx")
            .to_owned()
    }

    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
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
        self.pptx.can_save()
    }

    #[must_use]
    pub fn compatibility_issue_count(&self) -> usize {
        self.pptx.compatibility_issue_count()
    }

    #[must_use]
    pub fn slide_count(&self) -> usize {
        self.pptx.presentation().slides().len()
    }

    #[must_use]
    pub fn current_slide_index(&self) -> usize {
        self.pptx.presentation().active_slide()
    }

    #[must_use]
    pub fn element_summaries(&self) -> Vec<SlideElementSummary> {
        self.pptx
            .presentation()
            .slide(self.current_slide_index())
            .map(|slide| {
                slide
                    .elements
                    .iter()
                    .enumerate()
                    .map(|(index, element)| SlideElementSummary {
                        index,
                        kind: element.label().to_owned(),
                        text: element.text(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    #[must_use]
    pub fn selected_text(&self) -> String {
        let Some(index) = self.selected_element else {
            return String::new();
        };
        self.pptx
            .presentation()
            .slide(self.current_slide_index())
            .and_then(|slide| slide.elements.get(index))
            .map_or_else(String::new, SlideElement::text)
    }

    pub fn previous_slide(&mut self) -> Result<(), SlidesSessionError> {
        let current = self.current_slide_index();
        if current > 0 {
            self.pptx.presentation_mut().set_active_slide(current - 1)?;
            self.selected_element = None;
        }
        Ok(())
    }

    pub fn next_slide(&mut self) -> Result<(), SlidesSessionError> {
        let current = self.current_slide_index();
        if current + 1 < self.slide_count() {
            self.pptx.presentation_mut().set_active_slide(current + 1)?;
            self.selected_element = None;
        }
        Ok(())
    }

    pub fn add_slide(&mut self) {
        self.pptx.presentation_mut().add_slide();
        self.selected_element = None;
        self.dirty = true;
    }

    pub fn duplicate_slide(&mut self) -> Result<(), SlidesSessionError> {
        let current = self.current_slide_index();
        self.pptx.presentation_mut().duplicate_slide(current)?;
        self.selected_element = None;
        self.dirty = true;
        Ok(())
    }

    pub fn delete_slide(&mut self) -> Result<(), SlidesSessionError> {
        let current = self.current_slide_index();
        self.pptx.presentation_mut().delete_slide(current)?;
        self.selected_element = None;
        self.dirty = true;
        Ok(())
    }

    pub fn add_text_box(&mut self) -> Result<(), SlidesSessionError> {
        let slide_index = self.current_slide_index();
        let slide = self
            .pptx
            .presentation_mut()
            .slide_mut(slide_index)
            .ok_or(PresentationError::MissingSlide(slide_index))?;
        let index = slide.add_text_box("Text");
        self.selected_element = Some(index);
        self.dirty = true;
        Ok(())
    }

    pub fn add_shape(&mut self) -> Result<(), SlidesSessionError> {
        let slide_index = self.current_slide_index();
        let slide = self
            .pptx
            .presentation_mut()
            .slide_mut(slide_index)
            .ok_or(PresentationError::MissingSlide(slide_index))?;
        let index = slide.add_shape(ShapeKind::RoundedRectangle);
        self.selected_element = Some(index);
        self.dirty = true;
        Ok(())
    }

    pub fn add_table(&mut self) -> Result<(), SlidesSessionError> {
        let slide_index = self.current_slide_index();
        let slide = self
            .pptx
            .presentation_mut()
            .slide_mut(slide_index)
            .ok_or(PresentationError::MissingSlide(slide_index))?;
        let index = slide.add_table(3, 3)?;
        self.selected_element = Some(index);
        self.dirty = true;
        Ok(())
    }

    pub fn select_element(&mut self, index: usize) -> Result<(), SlidesSessionError> {
        let exists = self
            .pptx
            .presentation()
            .slide(self.current_slide_index())
            .is_some_and(|slide| index < slide.elements.len());
        if !exists {
            return Err(SlidesSessionError::MissingElement(index));
        }
        self.selected_element = Some(index);
        Ok(())
    }

    pub fn edit_selected_text(&mut self, text: &str) -> Result<(), SlidesSessionError> {
        let index = self
            .selected_element
            .ok_or(SlidesSessionError::MissingElement(usize::MAX))?;
        let slide_index = self.current_slide_index();
        let slide = self
            .pptx
            .presentation_mut()
            .slide_mut(slide_index)
            .ok_or(PresentationError::MissingSlide(slide_index))?;
        let element = slide
            .elements
            .get_mut(index)
            .ok_or(SlidesSessionError::MissingElement(index))?;

        match element {
            SlideElement::TextBox(value) => value.text = text.to_owned(),
            SlideElement::Shape(value) => value.text = text.to_owned(),
            SlideElement::Image(value) => value.alt_text = text.to_owned(),
            SlideElement::Table(value) => {
                if let Some(cell) = value.cell_mut(0, 0) {
                    cell.text = text.to_owned();
                }
            }
        }
        self.dirty = true;
        Ok(())
    }

    pub fn delete_selected(&mut self) -> Result<(), SlidesSessionError> {
        let index = self
            .selected_element
            .ok_or(SlidesSessionError::MissingElement(usize::MAX))?;
        let slide_index = self.current_slide_index();
        self.pptx
            .presentation_mut()
            .slide_mut(slide_index)
            .ok_or(PresentationError::MissingSlide(slide_index))?
            .delete_element(index)?;
        self.selected_element = None;
        self.dirty = true;
        Ok(())
    }

    pub fn move_selected_forward(&mut self) -> Result<(), SlidesSessionError> {
        self.move_selected(1)
    }

    pub fn move_selected_backward(&mut self) -> Result<(), SlidesSessionError> {
        self.move_selected(-1)
    }

    pub fn move_selected(&mut self, delta: isize) -> Result<(), SlidesSessionError> {
        let index = self
            .selected_element
            .ok_or(SlidesSessionError::MissingElement(usize::MAX))?;
        let slide_index = self.current_slide_index();
        let next = self
            .pptx
            .presentation_mut()
            .slide_mut(slide_index)
            .ok_or(PresentationError::MissingSlide(slide_index))?
            .move_element(index, delta)?;
        self.selected_element = Some(next);
        self.dirty = true;
        Ok(())
    }

    pub fn save(&mut self) -> Result<(), SlidesSessionError> {
        let path = self
            .path
            .clone()
            .ok_or(SlidesSessionError::MissingSavePath)?;
        self.save_to(path)
    }

    pub fn save_as(&mut self, path: impl Into<PathBuf>) -> Result<(), SlidesSessionError> {
        self.save_to(path.into())
    }

    pub fn save_recovery_copy(&mut self, path: &Path) -> Result<(), SlidesSessionError> {
        validate_pptx_path(path)?;
        save_pptx_atomic(&mut self.pptx, path)?;
        Ok(())
    }

    pub fn open_recovery(
        snapshot: impl Into<PathBuf>,
        original: Option<PathBuf>,
    ) -> Result<Self, SlidesSessionError> {
        let mut session = Self::open(snapshot)?;
        session.path = original;
        session.dirty = true;
        Ok(session)
    }

    fn save_to(&mut self, path: PathBuf) -> Result<(), SlidesSessionError> {
        validate_pptx_path(&path)?;
        save_pptx_atomic(&mut self.pptx, &path)?;
        self.path = Some(path);
        self.dirty = false;
        Ok(())
    }
}

fn validate_pptx_path(path: &Path) -> Result<(), SlidesSessionError> {
    if path.as_os_str().is_empty() {
        return Err(SlidesSessionError::EmptyPath);
    }
    if !path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pptx"))
    {
        return Err(SlidesSessionError::UnsupportedExtension);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs, process,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn blank_edit_save_reopen_round_trip() {
        let directory = unique_test_directory();
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("slides.pptx");
        let mut session = SlidesSession::blank();
        session.add_text_box().unwrap();
        session.edit_selected_text("你好 Nexa Slides").unwrap();
        session.add_slide();
        session.add_shape().unwrap();
        session.edit_selected_text("Second").unwrap();
        session.save_as(path.clone()).unwrap();

        let reopened = SlidesSession::open(path.clone()).unwrap();
        assert_eq!(reopened.slide_count(), 2);
        assert!(!reopened.is_dirty());

        let _ = fs::remove_file(path);
        let _ = fs::remove_dir(directory);
    }

    #[test]
    fn slide_operations_keep_valid_active_index() {
        let mut session = SlidesSession::blank();
        session.add_slide();
        session.duplicate_slide().unwrap();
        assert_eq!(session.slide_count(), 3);
        session.delete_slide().unwrap();
        assert_eq!(session.slide_count(), 2);
        session.previous_slide().unwrap();
        assert!(session.current_slide_index() < session.slide_count());
    }

    fn unique_test_directory() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should follow Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("nexa-slides-session-{}-{nonce}", process::id()))
    }
}
