use crate::formula::evaluate_formula;
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

pub const MAX_ROWS: u32 = 1_048_576;
pub const MAX_COLUMNS: u32 = 16_384;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CellAddress {
    pub row: u32,
    pub column: u32,
}

impl CellAddress {
    pub fn new(row: u32, column: u32) -> Result<Self, WorkbookError> {
        if row >= MAX_ROWS || column >= MAX_COLUMNS {
            return Err(WorkbookError::AddressOutOfRange { row, column });
        }
        Ok(Self { row, column })
    }

    pub fn parse_a1(value: &str) -> Result<Self, WorkbookError> {
        let value = value.trim().replace('$', "");
        if value.is_empty() {
            return Err(WorkbookError::InvalidAddress(value));
        }

        let split = value
            .find(|character: char| character.is_ascii_digit())
            .ok_or_else(|| WorkbookError::InvalidAddress(value.clone()))?;
        if split == 0 || split == value.len() {
            return Err(WorkbookError::InvalidAddress(value));
        }

        let (letters, digits) = value.split_at(split);
        if !letters
            .chars()
            .all(|character| character.is_ascii_alphabetic())
            || !digits.chars().all(|character| character.is_ascii_digit())
        {
            return Err(WorkbookError::InvalidAddress(value));
        }

        let mut column = 0_u32;
        for character in letters.bytes() {
            column = column
                .checked_mul(26)
                .and_then(|value| {
                    value.checked_add(u32::from(character.to_ascii_uppercase() - b'A') + 1)
                })
                .ok_or_else(|| WorkbookError::InvalidAddress(value.clone()))?;
        }

        let row = digits
            .parse::<u32>()
            .map_err(|_| WorkbookError::InvalidAddress(value.clone()))?;
        if row == 0 || column == 0 {
            return Err(WorkbookError::InvalidAddress(value));
        }

        Self::new(row - 1, column - 1)
    }

    #[must_use]
    pub fn to_a1(self) -> String {
        let mut column = self.column + 1;
        let mut letters = Vec::new();
        while column > 0 {
            let remainder = (column - 1) % 26;
            letters.push((b'A' + remainder as u8) as char);
            column = (column - 1) / 26;
        }
        letters.reverse();
        format!(
            "{}{}",
            letters.into_iter().collect::<String>(),
            self.row + 1
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CellRange {
    pub start: CellAddress,
    pub end: CellAddress,
}

impl CellRange {
    pub fn new(start: CellAddress, end: CellAddress) -> Self {
        Self {
            start: CellAddress {
                row: start.row.min(end.row),
                column: start.column.min(end.column),
            },
            end: CellAddress {
                row: start.row.max(end.row),
                column: start.column.max(end.column),
            },
        }
    }

    pub fn parse_a1(value: &str) -> Result<Self, WorkbookError> {
        if let Some((start, end)) = value.split_once(':') {
            Ok(Self::new(
                CellAddress::parse_a1(start)?,
                CellAddress::parse_a1(end)?,
            ))
        } else {
            let address = CellAddress::parse_a1(value)?;
            Ok(Self::new(address, address))
        }
    }

    #[must_use]
    pub fn to_a1(self) -> String {
        if self.start == self.end {
            self.start.to_a1()
        } else {
            format!("{}:{}", self.start.to_a1(), self.end.to_a1())
        }
    }

    #[must_use]
    pub fn contains(self, address: CellAddress) -> bool {
        address.row >= self.start.row
            && address.row <= self.end.row
            && address.column >= self.start.column
            && address.column <= self.end.column
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum CellValue {
    #[default]
    Empty,
    Number(f64),
    Text(String),
    Bool(bool),
    Error(String),
}

impl CellValue {
    #[must_use]
    pub fn display_text(&self) -> String {
        match self {
            Self::Empty => String::new(),
            Self::Number(value) => format_number(*value),
            Self::Text(value) => value.clone(),
            Self::Bool(value) => {
                if *value {
                    "TRUE".into()
                } else {
                    "FALSE".into()
                }
            }
            Self::Error(value) => value.clone(),
        }
    }

    #[must_use]
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            Self::Bool(value) => Some(if *value { 1.0 } else { 0.0 }),
            Self::Text(value) => value.parse().ok(),
            Self::Empty | Self::Error(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum CellAlignment {
    #[default]
    General,
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct CellFormat {
    pub bold: bool,
    pub italic: bool,
    pub fill_rgb: Option<String>,
    pub alignment: CellAlignment,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Cell {
    pub value: CellValue,
    pub formula: Option<String>,
    pub cached_number: Option<f64>,
    pub format: CellFormat,
}

impl Cell {
    #[must_use]
    pub fn display_text(&self) -> String {
        if self.formula.is_some() {
            return self
                .cached_number
                .map(format_number)
                .unwrap_or_else(|| "#CALC!".into());
        }
        self.value.display_text()
    }

    #[must_use]
    pub fn is_semantically_empty(&self) -> bool {
        self.formula.is_none()
            && matches!(self.value, CellValue::Empty)
            && self.format == CellFormat::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreezePane {
    pub rows: u32,
    pub columns: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ViewportCell {
    pub address: CellAddress,
    pub text: String,
    pub formula: Option<String>,
    pub format: CellFormat,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Worksheet {
    name: String,
    cells: BTreeMap<CellAddress, Cell>,
    merged_ranges: Vec<CellRange>,
    freeze_pane: Option<FreezePane>,
    row_heights: BTreeMap<u32, f64>,
    column_widths: BTreeMap<u32, f64>,
    hidden_rows: BTreeSet<u32>,
    auto_filter: Option<CellRange>,
}

impl Worksheet {
    pub fn new(name: impl Into<String>) -> Result<Self, WorkbookError> {
        let name = name.into();
        validate_sheet_name(&name)?;
        Ok(Self {
            name,
            cells: BTreeMap::new(),
            merged_ranges: Vec::new(),
            freeze_pane: None,
            row_heights: BTreeMap::new(),
            column_widths: BTreeMap::new(),
            hidden_rows: BTreeSet::new(),
            auto_filter: None,
        })
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn rename(&mut self, name: impl Into<String>) -> Result<(), WorkbookError> {
        let name = name.into();
        validate_sheet_name(&name)?;
        self.name = name;
        Ok(())
    }

    #[must_use]
    pub fn cells(&self) -> &BTreeMap<CellAddress, Cell> {
        &self.cells
    }

    #[must_use]
    pub fn cell(&self, address: CellAddress) -> Option<&Cell> {
        self.cells.get(&address)
    }

    pub fn set_cell(&mut self, address: CellAddress, cell: Cell) {
        if cell.is_semantically_empty() {
            self.cells.remove(&address);
        } else {
            self.cells.insert(address, cell);
        }
    }

    pub fn set_input(&mut self, address: CellAddress, input: &str) {
        let current_format = self
            .cells
            .get(&address)
            .map(|cell| cell.format.clone())
            .unwrap_or_default();

        let input = input.trim();
        let cell = if let Some(formula) = input.strip_prefix('=') {
            Cell {
                formula: Some(formula.trim().to_owned()),
                format: current_format,
                ..Cell::default()
            }
        } else if input.is_empty() {
            Cell {
                format: current_format,
                ..Cell::default()
            }
        } else if input.eq_ignore_ascii_case("true") || input.eq_ignore_ascii_case("false") {
            Cell {
                value: CellValue::Bool(input.eq_ignore_ascii_case("true")),
                format: current_format,
                ..Cell::default()
            }
        } else if let Ok(number) = input.parse::<f64>() {
            Cell {
                value: CellValue::Number(number),
                format: current_format,
                ..Cell::default()
            }
        } else {
            Cell {
                value: CellValue::Text(input.to_owned()),
                format: current_format,
                ..Cell::default()
            }
        };
        self.set_cell(address, cell);
    }

    pub fn set_format(&mut self, address: CellAddress, format: CellFormat) {
        let mut cell = self.cells.remove(&address).unwrap_or_default();
        cell.format = format;
        self.set_cell(address, cell);
    }

    pub fn merge(&mut self, range: CellRange) -> Result<(), WorkbookError> {
        if self
            .merged_ranges
            .iter()
            .any(|existing| ranges_overlap(*existing, range))
        {
            return Err(WorkbookError::OverlappingMerge(range.to_a1()));
        }
        self.merged_ranges.push(range);
        self.merged_ranges.sort();
        Ok(())
    }

    pub fn unmerge(&mut self, range: CellRange) {
        self.merged_ranges.retain(|existing| *existing != range);
    }

    #[must_use]
    pub fn merged_ranges(&self) -> &[CellRange] {
        &self.merged_ranges
    }

    pub fn set_freeze_pane(&mut self, pane: Option<FreezePane>) {
        self.freeze_pane = pane.filter(|value| value.rows > 0 || value.columns > 0);
    }

    #[must_use]
    pub const fn freeze_pane(&self) -> Option<FreezePane> {
        self.freeze_pane
    }

    pub fn set_row_height(&mut self, row: u32, height: Option<f64>) -> Result<(), WorkbookError> {
        if row >= MAX_ROWS {
            return Err(WorkbookError::AddressOutOfRange { row, column: 0 });
        }
        if let Some(height) = height {
            self.row_heights.insert(row, height.clamp(2.0, 409.0));
        } else {
            self.row_heights.remove(&row);
        }
        Ok(())
    }

    pub fn set_column_width(
        &mut self,
        column: u32,
        width: Option<f64>,
    ) -> Result<(), WorkbookError> {
        if column >= MAX_COLUMNS {
            return Err(WorkbookError::AddressOutOfRange { row: 0, column });
        }
        if let Some(width) = width {
            self.column_widths.insert(column, width.clamp(0.1, 255.0));
        } else {
            self.column_widths.remove(&column);
        }
        Ok(())
    }

    #[must_use]
    pub fn row_heights(&self) -> &BTreeMap<u32, f64> {
        &self.row_heights
    }

    #[must_use]
    pub fn column_widths(&self) -> &BTreeMap<u32, f64> {
        &self.column_widths
    }

    #[must_use]
    pub fn hidden_rows(&self) -> &BTreeSet<u32> {
        &self.hidden_rows
    }

    pub fn set_hidden_row(&mut self, row: u32, hidden: bool) {
        if hidden {
            self.hidden_rows.insert(row);
        } else {
            self.hidden_rows.remove(&row);
        }
    }

    #[must_use]
    pub const fn auto_filter(&self) -> Option<CellRange> {
        self.auto_filter
    }

    pub fn set_auto_filter(&mut self, range: Option<CellRange>) {
        self.auto_filter = range;
    }

    pub fn filter_equals(
        &mut self,
        range: CellRange,
        column: u32,
        expected: &str,
    ) -> Result<usize, WorkbookError> {
        if column < range.start.column || column > range.end.column {
            return Err(WorkbookError::FilterColumnOutsideRange);
        }
        self.auto_filter = Some(range);
        let mut hidden = 0;
        for row in range.start.row.saturating_add(1)..=range.end.row {
            let address = CellAddress { row, column };
            let matches = self
                .cells
                .get(&address)
                .map(Cell::display_text)
                .is_some_and(|value| value == expected);
            if matches {
                self.hidden_rows.remove(&row);
            } else {
                self.hidden_rows.insert(row);
                hidden += 1;
            }
        }
        Ok(hidden)
    }

    pub fn clear_filter(&mut self) {
        if let Some(range) = self.auto_filter {
            for row in range.start.row.saturating_add(1)..=range.end.row {
                self.hidden_rows.remove(&row);
            }
        }
        self.auto_filter = None;
    }

    pub fn sort_rows(
        &mut self,
        range: CellRange,
        key_column: u32,
        ascending: bool,
    ) -> Result<(), WorkbookError> {
        if key_column < range.start.column || key_column > range.end.column {
            return Err(WorkbookError::SortColumnOutsideRange);
        }
        if range.start.row >= range.end.row {
            return Ok(());
        }

        let first_data_row = range.start.row + 1;
        let mut rows = Vec::new();
        for row in first_data_row..=range.end.row {
            let key = self
                .cells
                .get(&CellAddress {
                    row,
                    column: key_column,
                })
                .cloned()
                .unwrap_or_default();
            let mut values = Vec::new();
            for column in range.start.column..=range.end.column {
                values.push(
                    self.cells
                        .get(&CellAddress { row, column })
                        .cloned()
                        .unwrap_or_default(),
                );
            }
            rows.push((key, values));
        }

        rows.sort_by(|(left, _), (right, _)| compare_cells(left, right));
        if !ascending {
            rows.reverse();
        }

        for row in first_data_row..=range.end.row {
            for column in range.start.column..=range.end.column {
                self.cells.remove(&CellAddress { row, column });
            }
        }

        for (offset, (_, values)) in rows.into_iter().enumerate() {
            let row = first_data_row + offset as u32;
            for (index, cell) in values.into_iter().enumerate() {
                let column = range.start.column + index as u32;
                self.set_cell(CellAddress { row, column }, cell);
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn used_range(&self) -> Option<CellRange> {
        let mut iter = self.cells.keys();
        let first = *iter.next()?;
        let mut start = first;
        let mut end = first;
        for address in iter {
            start.row = start.row.min(address.row);
            start.column = start.column.min(address.column);
            end.row = end.row.max(address.row);
            end.column = end.column.max(address.column);
        }
        Some(CellRange { start, end })
    }

    #[must_use]
    pub fn viewport(
        &self,
        start_row: u32,
        row_count: u32,
        start_column: u32,
        column_count: u32,
    ) -> Vec<ViewportCell> {
        let rows = row_count.min(256);
        let columns = column_count.min(64);
        let mut output = Vec::with_capacity((rows * columns) as usize);
        for row in start_row..start_row.saturating_add(rows).min(MAX_ROWS) {
            for column in start_column..start_column.saturating_add(columns).min(MAX_COLUMNS) {
                let address = CellAddress { row, column };
                let cell = self.cells.get(&address).cloned().unwrap_or_default();
                output.push(ViewportCell {
                    address,
                    text: cell.display_text(),
                    formula: cell.formula,
                    format: cell.format,
                });
            }
        }
        output
    }

    #[must_use]
    pub fn estimated_bytes(&self) -> usize {
        let cell_bytes = self
            .cells
            .values()
            .map(|cell| {
                std::mem::size_of::<CellAddress>()
                    + std::mem::size_of::<Cell>()
                    + cell.formula.as_ref().map_or(0, String::capacity)
                    + match &cell.value {
                        CellValue::Text(value) | CellValue::Error(value) => value.capacity(),
                        _ => 0,
                    }
                    + cell.format.fill_rgb.as_ref().map_or(0, String::capacity)
            })
            .sum::<usize>();
        self.name.capacity()
            + cell_bytes
            + self.merged_ranges.capacity() * std::mem::size_of::<CellRange>()
            + self.row_heights.len() * (std::mem::size_of::<u32>() + std::mem::size_of::<f64>())
            + self.column_widths.len() * (std::mem::size_of::<u32>() + std::mem::size_of::<f64>())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Workbook {
    sheets: Vec<Worksheet>,
    active_sheet: usize,
}

impl Workbook {
    pub fn blank() -> Self {
        Self {
            sheets: vec![Worksheet::new("Sheet1").expect("constant sheet name")],
            active_sheet: 0,
        }
    }

    pub fn new(sheets: Vec<Worksheet>) -> Result<Self, WorkbookError> {
        if sheets.is_empty() {
            return Err(WorkbookError::NoSheets);
        }
        ensure_unique_names(&sheets)?;
        Ok(Self {
            sheets,
            active_sheet: 0,
        })
    }

    #[must_use]
    pub fn sheets(&self) -> &[Worksheet] {
        &self.sheets
    }

    pub fn sheets_mut(&mut self) -> &mut [Worksheet] {
        &mut self.sheets
    }

    #[must_use]
    pub fn sheet(&self, index: usize) -> Option<&Worksheet> {
        self.sheets.get(index)
    }

    pub fn sheet_mut(&mut self, index: usize) -> Option<&mut Worksheet> {
        self.sheets.get_mut(index)
    }

    pub fn add_sheet(&mut self, name: impl Into<String>) -> Result<usize, WorkbookError> {
        let sheet = Worksheet::new(name)?;
        if self
            .sheets
            .iter()
            .any(|existing| existing.name.eq_ignore_ascii_case(sheet.name()))
        {
            return Err(WorkbookError::DuplicateSheetName(sheet.name().to_owned()));
        }
        self.sheets.push(sheet);
        Ok(self.sheets.len() - 1)
    }

    #[must_use]
    pub const fn active_sheet(&self) -> usize {
        self.active_sheet
    }

    pub fn set_active_sheet(&mut self, index: usize) -> Result<(), WorkbookError> {
        if index >= self.sheets.len() {
            return Err(WorkbookError::MissingSheet(index));
        }
        self.active_sheet = index;
        Ok(())
    }

    pub fn set_cell_input(
        &mut self,
        sheet: usize,
        address: CellAddress,
        input: &str,
    ) -> Result<(), WorkbookError> {
        self.sheet_mut(sheet)
            .ok_or(WorkbookError::MissingSheet(sheet))?
            .set_input(address, input);
        self.recalculate();
        Ok(())
    }

    pub fn recalculate(&mut self) {
        let snapshot = self.clone();
        for sheet_index in 0..self.sheets.len() {
            let formulas: Vec<(CellAddress, String)> = self.sheets[sheet_index]
                .cells
                .iter()
                .filter_map(|(address, cell)| {
                    cell.formula
                        .as_ref()
                        .map(|formula| (*address, formula.clone()))
                })
                .collect();

            for (address, formula) in formulas {
                let mut stack = BTreeSet::new();
                if let Ok(value) =
                    evaluate_formula(&snapshot, sheet_index, address, &formula, &mut stack)
                    && let Some(cell) = self.sheets[sheet_index].cells.get_mut(&address)
                {
                    cell.cached_number = Some(value);
                }
            }
        }
    }

    #[must_use]
    pub fn estimated_bytes(&self) -> usize {
        self.sheets.iter().map(Worksheet::estimated_bytes).sum()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkbookError {
    AddressOutOfRange { row: u32, column: u32 },
    InvalidAddress(String),
    InvalidSheetName(String),
    DuplicateSheetName(String),
    MissingSheet(usize),
    NoSheets,
    OverlappingMerge(String),
    SortColumnOutsideRange,
    FilterColumnOutsideRange,
}

impl fmt::Display for WorkbookError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AddressOutOfRange { row, column } => {
                write!(
                    f,
                    "cell address outside XLSX bounds: row={row}, column={column}"
                )
            }
            Self::InvalidAddress(value) => write!(f, "invalid A1 cell address: {value}"),
            Self::InvalidSheetName(value) => write!(f, "invalid worksheet name: {value}"),
            Self::DuplicateSheetName(value) => write!(f, "duplicate worksheet name: {value}"),
            Self::MissingSheet(index) => write!(f, "worksheet index does not exist: {index}"),
            Self::NoSheets => f.write_str("workbook must contain at least one worksheet"),
            Self::OverlappingMerge(value) => {
                write!(f, "merged range overlaps an existing merge: {value}")
            }
            Self::SortColumnOutsideRange => {
                f.write_str("sort key column is outside the selected range")
            }
            Self::FilterColumnOutsideRange => {
                f.write_str("filter column is outside the selected range")
            }
        }
    }
}

impl Error for WorkbookError {}

fn validate_sheet_name(name: &str) -> Result<(), WorkbookError> {
    if name.is_empty()
        || name.chars().count() > 31
        || name
            .chars()
            .any(|character| matches!(character, ':' | '\\' | '/' | '?' | '*' | '[' | ']'))
        || name.starts_with('\'')
        || name.ends_with('\'')
    {
        return Err(WorkbookError::InvalidSheetName(name.to_owned()));
    }
    Ok(())
}

fn ensure_unique_names(sheets: &[Worksheet]) -> Result<(), WorkbookError> {
    for (index, sheet) in sheets.iter().enumerate() {
        if sheets[..index]
            .iter()
            .any(|existing| existing.name.eq_ignore_ascii_case(sheet.name()))
        {
            return Err(WorkbookError::DuplicateSheetName(sheet.name.clone()));
        }
    }
    Ok(())
}

fn ranges_overlap(left: CellRange, right: CellRange) -> bool {
    left.start.row <= right.end.row
        && left.end.row >= right.start.row
        && left.start.column <= right.end.column
        && left.end.column >= right.start.column
}

fn compare_cells(left: &Cell, right: &Cell) -> Ordering {
    match (
        left.cached_number.or_else(|| left.value.as_number()),
        right.cached_number.or_else(|| right.value.as_number()),
    ) {
        (Some(left), Some(right)) => left.partial_cmp(&right).unwrap_or(Ordering::Equal),
        _ => left
            .display_text()
            .to_lowercase()
            .cmp(&right.display_text().to_lowercase()),
    }
}

fn format_number(value: f64) -> String {
    if value.fract() == 0.0 && value.is_finite() {
        format!("{value:.0}")
    } else {
        let mut output = format!("{value:.10}");
        while output.contains('.') && output.ends_with('0') {
            output.pop();
        }
        if output.ends_with('.') {
            output.pop();
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a1_addresses_round_trip_at_excel_edges() {
        for value in ["A1", "Z9", "AA10", "XFD1048576"] {
            let address = CellAddress::parse_a1(value).unwrap();
            assert_eq!(address.to_a1(), value);
        }
        assert!(CellAddress::parse_a1("XFE1").is_err());
        assert!(CellAddress::parse_a1("A1048577").is_err());
    }

    #[test]
    fn sparse_sheet_does_not_materialize_theoretical_rows() {
        let mut sheet = Worksheet::new("Data").unwrap();
        sheet.set_input(CellAddress::parse_a1("A1").unwrap(), "header");
        sheet.set_input(CellAddress::parse_a1("XFD1048576").unwrap(), "tail");

        assert_eq!(sheet.cells().len(), 2);
        assert!(sheet.estimated_bytes() < 16 * 1024);
    }

    #[test]
    fn viewport_is_bounded_and_sparse() {
        let mut sheet = Worksheet::new("Data").unwrap();
        sheet.set_input(CellAddress::parse_a1("B2").unwrap(), "42");
        let viewport = sheet.viewport(0, 24, 0, 10);

        assert_eq!(viewport.len(), 240);
        assert_eq!(
            viewport
                .iter()
                .find(|cell| cell.address == CellAddress::parse_a1("B2").unwrap())
                .unwrap()
                .text,
            "42"
        );
    }

    #[test]
    fn filter_and_sort_operate_on_sparse_rows() {
        let mut sheet = Worksheet::new("Data").unwrap();
        for (row, name, score) in [(2, "b", 2), (3, "a", 3), (4, "c", 1)] {
            sheet.set_input(CellAddress::new(row - 1, 0).unwrap(), name);
            sheet.set_input(CellAddress::new(row - 1, 1).unwrap(), &score.to_string());
        }
        let range = CellRange::parse_a1("A1:B4").unwrap();
        sheet.sort_rows(range, 1, true).unwrap();

        assert_eq!(
            sheet
                .cell(CellAddress::parse_a1("A2").unwrap())
                .unwrap()
                .display_text(),
            "c"
        );

        let hidden = sheet.filter_equals(range, 0, "a").unwrap();
        assert_eq!(hidden, 2);
        assert_eq!(sheet.hidden_rows().len(), 2);
        sheet.clear_filter();
        assert!(sheet.hidden_rows().is_empty());
    }
}
