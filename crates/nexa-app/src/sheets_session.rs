use nexa_sheets::{
    CellAddress, CellFormat, FreezePane, WorkbookError, XlsxError, XlsxWorkbook, open_xlsx,
    save_xlsx_atomic,
};
use std::{
    array,
    error::Error,
    fmt,
    fs::File,
    path::{Path, PathBuf},
};

pub const VIEWPORT_ROWS: usize = 14;
pub const VIEWPORT_COLUMNS: usize = 10;

#[derive(Debug)]
pub enum SheetsSessionError {
    Io(std::io::Error),
    Xlsx(XlsxError),
    Workbook(WorkbookError),
    MissingSavePath,
    EmptyPath,
    UnsupportedExtension,
    MissingSheet(usize),
}

impl fmt::Display for SheetsSessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "spreadsheet I/O error: {error}"),
            Self::Xlsx(error) => write!(f, "{error}"),
            Self::Workbook(error) => write!(f, "{error}"),
            Self::MissingSavePath => f.write_str("choose an XLSX path before saving"),
            Self::EmptyPath => f.write_str("spreadsheet path is empty"),
            Self::UnsupportedExtension => f.write_str("Nexa Sheets opens and saves .xlsx files"),
            Self::MissingSheet(index) => write!(f, "worksheet {index} does not exist"),
        }
    }
}

impl Error for SheetsSessionError {}

impl From<std::io::Error> for SheetsSessionError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<XlsxError> for SheetsSessionError {
    fn from(value: XlsxError) -> Self {
        Self::Xlsx(value)
    }
}

impl From<WorkbookError> for SheetsSessionError {
    fn from(value: WorkbookError) -> Self {
        Self::Workbook(value)
    }
}

#[derive(Debug, Clone)]
pub struct SheetRowData {
    pub row: u32,
    pub row_label: String,
    pub cells: [String; VIEWPORT_COLUMNS],
}

#[derive(Debug)]
pub struct SheetsSession {
    xlsx: XlsxWorkbook,
    path: Option<PathBuf>,
    dirty: bool,
    active_sheet: usize,
    active_cell: CellAddress,
    viewport_row: u32,
    viewport_column: u32,
}

impl SheetsSession {
    #[must_use]
    pub fn blank() -> Self {
        Self {
            xlsx: XlsxWorkbook::blank(),
            path: None,
            dirty: false,
            active_sheet: 0,
            active_cell: CellAddress { row: 0, column: 0 },
            viewport_row: 0,
            viewport_column: 0,
        }
    }

    pub fn open(path: impl Into<PathBuf>) -> Result<Self, SheetsSessionError> {
        let path = path.into();
        validate_xlsx_path(&path)?;
        let file = File::open(&path)?;
        let xlsx = open_xlsx(file)?;
        let active_sheet = xlsx.workbook().active_sheet();

        Ok(Self {
            xlsx,
            path: Some(path),
            dirty: false,
            active_sheet,
            active_cell: CellAddress { row: 0, column: 0 },
            viewport_row: 0,
            viewport_column: 0,
        })
    }

    #[must_use]
    pub fn title(&self) -> String {
        self.path
            .as_deref()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled.xlsx")
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
        self.xlsx.can_save()
    }

    #[must_use]
    pub fn compatibility_issue_count(&self) -> usize {
        self.xlsx.compatibility_issues().len()
    }

    #[must_use]
    pub fn sheet_count(&self) -> usize {
        self.xlsx.workbook().sheets().len()
    }

    #[must_use]
    pub const fn active_sheet_index(&self) -> usize {
        self.active_sheet
    }

    #[must_use]
    pub fn sheet_name(&self) -> String {
        self.xlsx
            .workbook()
            .sheet(self.active_sheet)
            .map_or_else(|| "Sheet".into(), |sheet| sheet.name().to_owned())
    }

    #[must_use]
    pub const fn active_cell(&self) -> CellAddress {
        self.active_cell
    }

    #[must_use]
    pub fn active_address(&self) -> String {
        self.active_cell.to_a1()
    }

    #[must_use]
    pub fn active_input(&self) -> String {
        let Some(cell) = self
            .xlsx
            .workbook()
            .sheet(self.active_sheet)
            .and_then(|sheet| sheet.cell(self.active_cell))
        else {
            return String::new();
        };
        if let Some(formula) = &cell.formula {
            format!("={formula}")
        } else {
            cell.value.display_text()
        }
    }

    #[must_use]
    pub fn active_display_value(&self) -> String {
        self.xlsx
            .workbook()
            .sheet(self.active_sheet)
            .and_then(|sheet| sheet.cell(self.active_cell))
            .map_or_else(String::new, nexa_sheets::Cell::display_text)
    }

    #[must_use]
    pub fn active_bold(&self) -> bool {
        self.active_format().bold
    }

    #[must_use]
    pub fn active_italic(&self) -> bool {
        self.active_format().italic
    }

    #[must_use]
    pub const fn viewport_row(&self) -> u32 {
        self.viewport_row
    }

    #[must_use]
    pub const fn viewport_column(&self) -> u32 {
        self.viewport_column
    }

    #[must_use]
    pub fn viewport_header(&self) -> [String; VIEWPORT_COLUMNS] {
        array::from_fn(|index| {
            CellAddress {
                row: 0,
                column: self.viewport_column + index as u32,
            }
            .to_a1()
            .trim_end_matches('1')
            .to_owned()
        })
    }

    #[must_use]
    pub fn viewport_rows(&self) -> Vec<SheetRowData> {
        let Some(sheet) = self.xlsx.workbook().sheet(self.active_sheet) else {
            return Vec::new();
        };

        let mut rows = Vec::with_capacity(VIEWPORT_ROWS);
        let mut row = self.viewport_row;
        while row < nexa_sheets::MAX_ROWS && rows.len() < VIEWPORT_ROWS {
            if !sheet.hidden_rows().contains(&row) {
                let cells = array::from_fn(|index| {
                    let address = CellAddress {
                        row,
                        column: self.viewport_column + index as u32,
                    };
                    sheet
                        .cell(address)
                        .map_or_else(String::new, nexa_sheets::Cell::display_text)
                });
                rows.push(SheetRowData {
                    row,
                    row_label: (row + 1).to_string(),
                    cells,
                });
            }
            row += 1;
        }
        rows
    }

    pub fn select_cell(&mut self, row: u32, column: u32) -> Result<(), SheetsSessionError> {
        self.active_cell = CellAddress::new(row, column)?;
        self.ensure_active_visible();
        Ok(())
    }

    pub fn select_address(&mut self, value: &str) -> Result<(), SheetsSessionError> {
        self.active_cell = CellAddress::parse_a1(value)?;
        self.ensure_active_visible();
        Ok(())
    }

    pub fn set_active_input(&mut self, input: &str) -> Result<(), SheetsSessionError> {
        self.xlsx
            .workbook_mut()
            .set_cell_input(self.active_sheet, self.active_cell, input)?;
        self.dirty = true;
        Ok(())
    }

    pub fn toggle_bold(&mut self) -> Result<(), SheetsSessionError> {
        let mut format = self.active_format();
        format.bold = !format.bold;
        self.set_active_format(format)
    }

    pub fn toggle_italic(&mut self) -> Result<(), SheetsSessionError> {
        let mut format = self.active_format();
        format.italic = !format.italic;
        self.set_active_format(format)
    }

    pub fn toggle_fill(&mut self) -> Result<(), SheetsSessionError> {
        let mut format = self.active_format();
        format.fill_rgb = if format.fill_rgb.is_some() {
            None
        } else {
            Some("EAF2FF".into())
        };
        self.set_active_format(format)
    }

    pub fn scroll(&mut self, rows: i32, columns: i32) {
        self.viewport_row = offset_u32(
            self.viewport_row,
            rows,
            nexa_sheets::MAX_ROWS.saturating_sub(VIEWPORT_ROWS as u32),
        );
        self.viewport_column = offset_u32(
            self.viewport_column,
            columns,
            nexa_sheets::MAX_COLUMNS.saturating_sub(VIEWPORT_COLUMNS as u32),
        );
    }

    pub fn freeze_first_row(&mut self) -> Result<(), SheetsSessionError> {
        let pane = self
            .active_sheet_mut()?
            .freeze_pane()
            .unwrap_or(FreezePane {
                rows: 0,
                columns: 0,
            });
        self.active_sheet_mut()?.set_freeze_pane(Some(FreezePane {
            rows: if pane.rows == 0 { 1 } else { 0 },
            columns: pane.columns,
        }));
        self.dirty = true;
        Ok(())
    }

    pub fn freeze_first_column(&mut self) -> Result<(), SheetsSessionError> {
        let pane = self
            .active_sheet_mut()?
            .freeze_pane()
            .unwrap_or(FreezePane {
                rows: 0,
                columns: 0,
            });
        self.active_sheet_mut()?.set_freeze_pane(Some(FreezePane {
            rows: pane.rows,
            columns: if pane.columns == 0 { 1 } else { 0 },
        }));
        self.dirty = true;
        Ok(())
    }

    pub fn sort_active_column(&mut self, ascending: bool) -> Result<(), SheetsSessionError> {
        let active_column = self.active_cell.column;
        let Some(range) = self.active_sheet_mut()?.used_range() else {
            return Ok(());
        };
        if active_column < range.start.column || active_column > range.end.column {
            return Ok(());
        }
        self.active_sheet_mut()?
            .sort_rows(range, active_column, ascending)?;
        self.xlsx.workbook_mut().recalculate();
        self.dirty = true;
        Ok(())
    }

    pub fn filter_active_column_equals(
        &mut self,
        expected: &str,
    ) -> Result<usize, SheetsSessionError> {
        let active_column = self.active_cell.column;
        let Some(range) = self.active_sheet_mut()?.used_range() else {
            return Ok(0);
        };
        let hidden = self
            .active_sheet_mut()?
            .filter_equals(range, active_column, expected)?;
        self.dirty = true;
        Ok(hidden)
    }

    pub fn clear_filter(&mut self) -> Result<(), SheetsSessionError> {
        self.active_sheet_mut()?.clear_filter();
        self.dirty = true;
        Ok(())
    }

    pub fn add_sheet(&mut self) -> Result<(), SheetsSessionError> {
        let index = self.sheet_count() + 1;
        let name = format!("Sheet{index}");
        self.active_sheet = self.xlsx.workbook_mut().add_sheet(name)?;
        self.xlsx
            .workbook_mut()
            .set_active_sheet(self.active_sheet)?;
        self.active_cell = CellAddress { row: 0, column: 0 };
        self.viewport_row = 0;
        self.viewport_column = 0;
        self.dirty = true;
        Ok(())
    }

    pub fn save(&mut self) -> Result<(), SheetsSessionError> {
        let path = self
            .path
            .clone()
            .ok_or(SheetsSessionError::MissingSavePath)?;
        self.save_to(path)
    }

    pub fn save_as(&mut self, path: impl Into<PathBuf>) -> Result<(), SheetsSessionError> {
        self.save_to(path.into())
    }

    fn save_to(&mut self, path: PathBuf) -> Result<(), SheetsSessionError> {
        validate_xlsx_path(&path)?;
        save_xlsx_atomic(&mut self.xlsx, &path)?;
        self.path = Some(path);
        self.dirty = false;
        Ok(())
    }

    fn active_format(&self) -> CellFormat {
        self.xlsx
            .workbook()
            .sheet(self.active_sheet)
            .and_then(|sheet| sheet.cell(self.active_cell))
            .map(|cell| cell.format.clone())
            .unwrap_or_default()
    }

    fn set_active_format(&mut self, format: CellFormat) -> Result<(), SheetsSessionError> {
        let active_cell = self.active_cell;
        self.active_sheet_mut()?.set_format(active_cell, format);
        self.dirty = true;
        Ok(())
    }

    fn active_sheet_mut(&mut self) -> Result<&mut nexa_sheets::Worksheet, SheetsSessionError> {
        self.xlsx
            .workbook_mut()
            .sheet_mut(self.active_sheet)
            .ok_or(SheetsSessionError::MissingSheet(self.active_sheet))
    }

    fn ensure_active_visible(&mut self) {
        if self.active_cell.row < self.viewport_row {
            self.viewport_row = self.active_cell.row;
        } else if self.active_cell.row >= self.viewport_row + VIEWPORT_ROWS as u32 {
            self.viewport_row = self
                .active_cell
                .row
                .saturating_sub(VIEWPORT_ROWS as u32 - 1);
        }

        if self.active_cell.column < self.viewport_column {
            self.viewport_column = self.active_cell.column;
        } else if self.active_cell.column >= self.viewport_column + VIEWPORT_COLUMNS as u32 {
            self.viewport_column = self
                .active_cell
                .column
                .saturating_sub(VIEWPORT_COLUMNS as u32 - 1);
        }
    }
}

fn offset_u32(value: u32, offset: i32, max: u32) -> u32 {
    if offset.is_negative() {
        value.saturating_sub(offset.unsigned_abs())
    } else {
        value.saturating_add(offset as u32).min(max)
    }
}

fn validate_xlsx_path(path: &Path) -> Result<(), SheetsSessionError> {
    if path.as_os_str().is_empty() {
        return Err(SheetsSessionError::EmptyPath);
    }
    if !path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("xlsx"))
    {
        return Err(SheetsSessionError::UnsupportedExtension);
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
    fn blank_session_edits_formulas_scrolls_and_saves() {
        let directory = unique_test_directory();
        let path = directory.join("phase4.xlsx");
        let mut session = SheetsSession::blank();

        session.select_address("A1").unwrap();
        session.set_active_input("10").unwrap();
        session.select_address("A2").unwrap();
        session.set_active_input("20").unwrap();
        session.select_address("A3").unwrap();
        session.set_active_input("=SUM(A1:A2)").unwrap();
        assert_eq!(session.active_display_value(), "30");

        session.toggle_bold().unwrap();
        session.freeze_first_row().unwrap();
        session.scroll(100, 5);
        assert_eq!(session.viewport_row(), 100);
        assert_eq!(session.viewport_column(), 5);

        session.save_as(path.clone()).unwrap();
        let reopened = SheetsSession::open(path.clone()).unwrap();
        assert!(reopened.can_save());

        let _ = fs::remove_file(path);
        let _ = fs::remove_dir(directory);
    }

    #[test]
    fn viewport_rows_are_bounded() {
        let session = SheetsSession::blank();
        assert_eq!(session.viewport_rows().len(), VIEWPORT_ROWS);
        assert!(
            session
                .viewport_rows()
                .iter()
                .all(|row| row.cells.len() == VIEWPORT_COLUMNS)
        );
    }

    fn unique_test_directory() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should follow Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("nexa-sheets-session-{}-{nonce}", process::id()))
    }
}
