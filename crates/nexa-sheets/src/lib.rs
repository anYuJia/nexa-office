#![forbid(unsafe_code)]

//! Native sparse spreadsheet engine for Nexa Sheets.
//!
//! The workbook model is independent from Slint and stores only populated cells,
//! explicit row/column metadata and bounded viewport state. XLSX I/O builds on the
//! shared Nexa OOXML/OPC layer.

mod formula;
mod model;
mod xlsx;

pub use formula::{FormulaError, evaluate_formula};
pub use model::{
    Cell, CellAddress, CellAlignment, CellFormat, CellRange, CellValue, FreezePane, MAX_COLUMNS,
    MAX_ROWS, ViewportCell, Workbook, WorkbookError, Worksheet,
};
pub use xlsx::{XlsxError, XlsxWorkbook, open_xlsx, save_xlsx, save_xlsx_atomic};
