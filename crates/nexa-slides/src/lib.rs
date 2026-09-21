#![forbid(unsafe_code)]

//! Native semantic presentation engine for Nexa Slides.

mod model;
mod pptx;

pub use model::{
    Image, Presentation, PresentationError, Rect, RgbColor, Shape, ShapeKind, Slide, SlideElement,
    Table, TableCell, TextBox, TextStyle,
};
pub use pptx::{PptxError, PptxPresentation, open_pptx, save_pptx, save_pptx_atomic};
