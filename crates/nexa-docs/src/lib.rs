#![forbid(unsafe_code)]

//! Native semantic document engine for Nexa Docs.
//!
//! The semantic model is independent from the UI and renderer. WordprocessingML
//! parsing/serialization, editing, layout and package preservation build on these
//! types without leaking widget state into the document graph.

mod docx;
mod editor;
mod layout;
mod model;

pub use docx::{DocxDocument, DocxError, open_docx, save_docx, save_docx_atomic};
pub use editor::{
    DocsEditor, EditError, HistoryLimits, SearchMatch, SearchOptions, Selection, TextPosition,
};
pub use layout::{DocumentLayout, LayoutBlock, LayoutConfig, PageLayout, Paginator};
pub use model::{
    AbstractList, Alignment, Block, CellProperties, CompatibilityIssue, CompatibilityReport,
    CompatibilitySeverity, Document, HeaderFooter, InlineImage, ListFormat, ListLevel,
    ListReference, ModelError, Numbering, NumberingInstance, Orientation, PageMargins, Paragraph,
    ParagraphProperties, RgbColor, Run, RunContent, RunProperties, Section, SectionProperties,
    Style, StyleKind, StyleSheet, Table, TableCell, TableProperties, TableRow,
};
