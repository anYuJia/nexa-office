use std::{error::Error, fmt};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RgbColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl RgbColor {
    #[must_use]
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }

    #[must_use]
    pub fn to_hex(self) -> String {
        format!("{:02X}{:02X}{:02X}", self.red, self.green, self.blue)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Default for Rect {
    fn default() -> Self {
        Self {
            x: 1.0,
            y: 1.0,
            width: 4.0,
            height: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ShapeKind {
    #[default]
    Rectangle,
    RoundedRectangle,
    Ellipse,
    Line,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    pub font_size_pt: f64,
    pub bold: bool,
    pub italic: bool,
    pub color: RgbColor,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_size_pt: 24.0,
            bold: false,
            italic: false,
            color: RgbColor::new(16, 24, 40),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextBox {
    pub bounds: Rect,
    pub text: String,
    pub style: TextStyle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    pub bounds: Rect,
    pub kind: ShapeKind,
    pub fill: Option<RgbColor>,
    pub line: Option<RgbColor>,
    pub text: String,
    pub text_style: TextStyle,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    pub bounds: Rect,
    pub relationship_id: String,
    pub alt_text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableCell {
    pub text: String,
    pub bold: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub bounds: Rect,
    pub rows: usize,
    pub columns: usize,
    pub cells: Vec<TableCell>,
}

impl Table {
    #[must_use]
    pub fn cell(&self, row: usize, column: usize) -> Option<&TableCell> {
        if row >= self.rows || column >= self.columns {
            return None;
        }
        self.cells.get(row * self.columns + column)
    }

    pub fn cell_mut(&mut self, row: usize, column: usize) -> Option<&mut TableCell> {
        if row >= self.rows || column >= self.columns {
            return None;
        }
        self.cells.get_mut(row * self.columns + column)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SlideElement {
    TextBox(TextBox),
    Shape(Shape),
    Image(Image),
    Table(Table),
}

impl SlideElement {
    #[must_use]
    pub fn bounds(&self) -> Rect {
        match self {
            Self::TextBox(value) => value.bounds,
            Self::Shape(value) => value.bounds,
            Self::Image(value) => value.bounds,
            Self::Table(value) => value.bounds,
        }
    }

    pub fn set_bounds(&mut self, bounds: Rect) {
        match self {
            Self::TextBox(value) => value.bounds = bounds,
            Self::Shape(value) => value.bounds = bounds,
            Self::Image(value) => value.bounds = bounds,
            Self::Table(value) => value.bounds = bounds,
        }
    }

    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::TextBox(_) => "Text",
            Self::Shape(_) => "Shape",
            Self::Image(_) => "Image",
            Self::Table(_) => "Table",
        }
    }

    #[must_use]
    pub fn text(&self) -> String {
        match self {
            Self::TextBox(value) => value.text.clone(),
            Self::Shape(value) => value.text.clone(),
            Self::Image(value) => value.alt_text.clone(),
            Self::Table(value) => value
                .cells
                .iter()
                .map(|cell| cell.text.as_str())
                .collect::<Vec<_>>()
                .join(" | "),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Slide {
    pub name: String,
    pub elements: Vec<SlideElement>,
    pub notes: String,
}

impl Slide {
    #[must_use]
    pub fn blank(index: usize) -> Self {
        Self {
            name: format!("Slide {}", index + 1),
            elements: Vec::new(),
            notes: String::new(),
        }
    }

    pub fn add_text_box(&mut self, text: impl Into<String>) -> usize {
        self.elements.push(SlideElement::TextBox(TextBox {
            bounds: Rect::default(),
            text: text.into(),
            style: TextStyle::default(),
        }));
        self.elements.len() - 1
    }

    pub fn add_shape(&mut self, kind: ShapeKind) -> usize {
        self.elements.push(SlideElement::Shape(Shape {
            bounds: Rect::default(),
            kind,
            fill: Some(RgbColor::new(238, 244, 255)),
            line: Some(RgbColor::new(37, 99, 235)),
            text: String::new(),
            text_style: TextStyle::default(),
        }));
        self.elements.len() - 1
    }

    pub fn add_table(&mut self, rows: usize, columns: usize) -> Result<usize, PresentationError> {
        if rows == 0 || columns == 0 || rows > 64 || columns > 64 {
            return Err(PresentationError::InvalidTableSize { rows, columns });
        }
        let cells = vec![
            TableCell {
                text: String::new(),
                bold: false,
            };
            rows * columns
        ];
        self.elements.push(SlideElement::Table(Table {
            bounds: Rect {
                width: 6.0,
                height: 3.0,
                ..Rect::default()
            },
            rows,
            columns,
            cells,
        }));
        Ok(self.elements.len() - 1)
    }

    pub fn delete_element(&mut self, index: usize) -> Result<(), PresentationError> {
        if index >= self.elements.len() {
            return Err(PresentationError::MissingElement(index));
        }
        self.elements.remove(index);
        Ok(())
    }

    pub fn move_element(&mut self, index: usize, delta: isize) -> Result<usize, PresentationError> {
        if index >= self.elements.len() {
            return Err(PresentationError::MissingElement(index));
        }
        let next = if delta.is_negative() {
            index.saturating_sub(delta.unsigned_abs())
        } else {
            (index + delta as usize).min(self.elements.len().saturating_sub(1))
        };
        if next != index {
            let element = self.elements.remove(index);
            self.elements.insert(next, element);
        }
        Ok(next)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Presentation {
    slides: Vec<Slide>,
    active_slide: usize,
    pub width_inches: f64,
    pub height_inches: f64,
}

impl Presentation {
    #[must_use]
    pub fn blank() -> Self {
        Self {
            slides: vec![Slide::blank(0)],
            active_slide: 0,
            width_inches: 13.333,
            height_inches: 7.5,
        }
    }

    #[must_use]
    pub fn slides(&self) -> &[Slide] {
        &self.slides
    }

    pub fn slides_mut(&mut self) -> &mut [Slide] {
        &mut self.slides
    }

    pub fn replace_slides(&mut self, slides: Vec<Slide>) -> Result<(), PresentationError> {
        if slides.is_empty() {
            return Err(PresentationError::MissingSlide(0));
        }
        self.slides = slides;
        self.active_slide = self.active_slide.min(self.slides.len() - 1);
        Ok(())
    }

    #[must_use]
    pub const fn active_slide(&self) -> usize {
        self.active_slide
    }

    pub fn set_active_slide(&mut self, index: usize) -> Result<(), PresentationError> {
        if index >= self.slides.len() {
            return Err(PresentationError::MissingSlide(index));
        }
        self.active_slide = index;
        Ok(())
    }

    pub fn add_slide(&mut self) -> usize {
        let index = self.slides.len();
        self.slides.push(Slide::blank(index));
        self.active_slide = index;
        index
    }

    pub fn duplicate_slide(&mut self, index: usize) -> Result<usize, PresentationError> {
        let slide = self
            .slides
            .get(index)
            .cloned()
            .ok_or(PresentationError::MissingSlide(index))?;
        let next = index + 1;
        self.slides.insert(next, slide);
        self.active_slide = next;
        Ok(next)
    }

    pub fn delete_slide(&mut self, index: usize) -> Result<(), PresentationError> {
        if self.slides.len() == 1 {
            return Err(PresentationError::CannotDeleteLastSlide);
        }
        if index >= self.slides.len() {
            return Err(PresentationError::MissingSlide(index));
        }
        self.slides.remove(index);
        self.active_slide = self.active_slide.min(self.slides.len() - 1);
        Ok(())
    }

    #[must_use]
    pub fn slide(&self, index: usize) -> Option<&Slide> {
        self.slides.get(index)
    }

    pub fn slide_mut(&mut self, index: usize) -> Option<&mut Slide> {
        self.slides.get_mut(index)
    }
}

impl Default for Presentation {
    fn default() -> Self {
        Self::blank()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresentationError {
    MissingSlide(usize),
    MissingElement(usize),
    CannotDeleteLastSlide,
    InvalidTableSize { rows: usize, columns: usize },
}

impl fmt::Display for PresentationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSlide(index) => write!(f, "slide {index} does not exist"),
            Self::MissingElement(index) => write!(f, "slide element {index} does not exist"),
            Self::CannotDeleteLastSlide => {
                f.write_str("a presentation must contain at least one slide")
            }
            Self::InvalidTableSize { rows, columns } => {
                write!(f, "invalid table size: {rows}×{columns}")
            }
        }
    }
}

impl Error for PresentationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn z_order_move_is_bounded() {
        let mut slide = Slide::blank(0);
        slide.add_text_box("A");
        slide.add_text_box("B");
        assert_eq!(slide.move_element(0, 1).unwrap(), 1);
        assert_eq!(slide.elements[1].text(), "A");
        assert_eq!(slide.move_element(1, 99).unwrap(), 1);
    }

    #[test]
    fn presentation_never_deletes_last_slide() {
        let mut presentation = Presentation::blank();
        assert_eq!(
            presentation.delete_slide(0),
            Err(PresentationError::CannotDeleteLastSlide)
        );
        presentation.add_slide();
        presentation.delete_slide(0).unwrap();
        assert_eq!(presentation.slides().len(), 1);
    }

    #[test]
    fn table_cells_are_row_major() {
        let mut slide = Slide::blank(0);
        let index = slide.add_table(2, 3).unwrap();
        let SlideElement::Table(table) = &mut slide.elements[index] else {
            panic!("table expected");
        };
        table.cell_mut(1, 2).unwrap().text = "done".into();
        assert_eq!(table.cell(1, 2).unwrap().text, "done");
    }
}
