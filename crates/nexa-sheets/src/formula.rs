use crate::{CellAddress, CellRange, CellValue, Workbook};
use std::{collections::BTreeSet, error::Error, fmt};

#[derive(Debug, Clone, PartialEq)]
pub enum FormulaError {
    InvalidSyntax(String),
    DivisionByZero,
    CircularReference(String),
    MissingSheet(usize),
}

impl fmt::Display for FormulaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSyntax(value) => write!(f, "unsupported or invalid formula: {value}"),
            Self::DivisionByZero => f.write_str("formula division by zero"),
            Self::CircularReference(value) => write!(f, "circular formula reference at {value}"),
            Self::MissingSheet(index) => write!(f, "formula worksheet does not exist: {index}"),
        }
    }
}

impl Error for FormulaError {}

pub fn evaluate_formula(
    workbook: &Workbook,
    sheet_index: usize,
    address: CellAddress,
    formula: &str,
    stack: &mut BTreeSet<(usize, CellAddress)>,
) -> Result<f64, FormulaError> {
    if !stack.insert((sheet_index, address)) {
        return Err(FormulaError::CircularReference(address.to_a1()));
    }
    let result = Parser::new(workbook, sheet_index, formula, stack).parse();
    stack.remove(&(sheet_index, address));
    result
}

struct Parser<'a> {
    workbook: &'a Workbook,
    sheet_index: usize,
    source: &'a str,
    index: usize,
    stack: &'a mut BTreeSet<(usize, CellAddress)>,
}

impl<'a> Parser<'a> {
    fn new(
        workbook: &'a Workbook,
        sheet_index: usize,
        source: &'a str,
        stack: &'a mut BTreeSet<(usize, CellAddress)>,
    ) -> Self {
        Self {
            workbook,
            sheet_index,
            source: source.trim().trim_start_matches('='),
            index: 0,
            stack,
        }
    }

    fn parse(mut self) -> Result<f64, FormulaError> {
        let value = self.parse_expression()?;
        self.skip_space();
        if self.index != self.source.len() {
            return Err(self.invalid());
        }
        Ok(value)
    }

    fn parse_expression(&mut self) -> Result<f64, FormulaError> {
        let mut value = self.parse_term()?;
        loop {
            self.skip_space();
            if self.consume('+') {
                value += self.parse_term()?;
            } else if self.consume('-') {
                value -= self.parse_term()?;
            } else {
                return Ok(value);
            }
        }
    }

    fn parse_term(&mut self) -> Result<f64, FormulaError> {
        let mut value = self.parse_factor()?;
        loop {
            self.skip_space();
            if self.consume('*') {
                value *= self.parse_factor()?;
            } else if self.consume('/') {
                let divisor = self.parse_factor()?;
                if divisor == 0.0 {
                    return Err(FormulaError::DivisionByZero);
                }
                value /= divisor;
            } else {
                return Ok(value);
            }
        }
    }

    fn parse_factor(&mut self) -> Result<f64, FormulaError> {
        self.skip_space();
        if self.consume('+') {
            return self.parse_factor();
        }
        if self.consume('-') {
            return Ok(-self.parse_factor()?);
        }
        if self.consume('(') {
            let value = self.parse_expression()?;
            self.expect(')')?;
            return Ok(value);
        }
        if self
            .peek()
            .is_some_and(|value| value.is_ascii_digit() || value == '.')
        {
            return self.parse_number();
        }

        let start = self.index;
        let token = self.read_identifier_or_cell();
        if token.is_empty() {
            return Err(self.invalid());
        }

        self.skip_space();
        if self.consume('(') {
            return self.parse_function(&token);
        }

        self.index = start;
        let cell = self.read_cell_reference()?;
        self.cell_number(cell)
    }

    fn parse_function(&mut self, name: &str) -> Result<f64, FormulaError> {
        let mut values = Vec::new();
        self.skip_space();
        if self.consume(')') {
            return aggregate(name, &values).ok_or_else(|| self.invalid());
        }

        loop {
            self.skip_space();
            let save = self.index;
            if let Ok(start) = self.read_cell_reference() {
                self.skip_space();
                if self.consume(':') {
                    let end = self.read_cell_reference()?;
                    let range = CellRange::new(start, end);
                    for row in range.start.row..=range.end.row {
                        for column in range.start.column..=range.end.column {
                            values.push(self.cell_number(CellAddress { row, column })?);
                        }
                    }
                } else {
                    self.index = save;
                    values.push(self.parse_expression()?);
                }
            } else {
                self.index = save;
                values.push(self.parse_expression()?);
            }

            self.skip_space();
            if self.consume(')') {
                break;
            }
            self.expect(',')?;
        }

        aggregate(name, &values).ok_or_else(|| self.invalid())
    }

    fn cell_number(&mut self, address: CellAddress) -> Result<f64, FormulaError> {
        let sheet = self
            .workbook
            .sheet(self.sheet_index)
            .ok_or(FormulaError::MissingSheet(self.sheet_index))?;
        let Some(cell) = sheet.cell(address) else {
            return Ok(0.0);
        };
        if let Some(formula) = &cell.formula {
            return evaluate_formula(
                self.workbook,
                self.sheet_index,
                address,
                formula,
                self.stack,
            );
        }
        Ok(match &cell.value {
            CellValue::Number(value) => *value,
            CellValue::Bool(value) => {
                if *value {
                    1.0
                } else {
                    0.0
                }
            }
            CellValue::Text(value) => value.parse().unwrap_or(0.0),
            CellValue::Empty | CellValue::Error(_) => 0.0,
        })
    }

    fn parse_number(&mut self) -> Result<f64, FormulaError> {
        let start = self.index;
        while self.peek().is_some_and(|value| {
            value.is_ascii_digit() || matches!(value, '.' | 'e' | 'E' | '+' | '-')
        }) {
            let current = self.peek().unwrap();
            if matches!(current, '+' | '-')
                && self.index > start
                && !matches!(self.source.as_bytes()[self.index - 1] as char, 'e' | 'E')
            {
                break;
            }
            self.index += current.len_utf8();
        }
        self.source[start..self.index]
            .parse()
            .map_err(|_| self.invalid())
    }

    fn read_identifier_or_cell(&mut self) -> String {
        let start = self.index;
        while self.peek().is_some_and(|value| {
            value.is_ascii_alphanumeric() || value == '_' || value == '.' || value as u32 == 36
        }) {
            self.index += self.peek().unwrap().len_utf8();
        }
        self.source[start..self.index].to_owned()
    }

    fn read_cell_reference(&mut self) -> Result<CellAddress, FormulaError> {
        self.skip_space();
        let start = self.index;
        let token = self.read_identifier_or_cell();
        if token.is_empty() {
            self.index = start;
            return Err(self.invalid());
        }
        match CellAddress::parse_a1(&token) {
            Ok(address) => Ok(address),
            Err(_) => {
                self.index = start;
                Err(self.invalid())
            }
        }
    }

    fn expect(&mut self, expected: char) -> Result<(), FormulaError> {
        self.skip_space();
        if self.consume(expected) {
            Ok(())
        } else {
            Err(self.invalid())
        }
    }

    fn consume(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.index += expected.len_utf8();
            true
        } else {
            false
        }
    }

    fn skip_space(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.index += self.peek().unwrap().len_utf8();
        }
    }

    fn peek(&self) -> Option<char> {
        self.source[self.index..].chars().next()
    }

    fn invalid(&self) -> FormulaError {
        FormulaError::InvalidSyntax(self.source.to_owned())
    }
}

fn aggregate(name: &str, values: &[f64]) -> Option<f64> {
    match name.to_ascii_uppercase().as_str() {
        "SUM" => Some(values.iter().sum()),
        "AVERAGE" => {
            if values.is_empty() {
                Some(0.0)
            } else {
                Some(values.iter().sum::<f64>() / values.len() as f64)
            }
        }
        "MIN" => values.iter().copied().reduce(f64::min).or(Some(0.0)),
        "MAX" => values.iter().copied().reduce(f64::max).or(Some(0.0)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Workbook;

    #[test]
    fn arithmetic_references_ranges_and_functions_recalculate() {
        let mut workbook = Workbook::blank();
        workbook
            .set_cell_input(0, CellAddress::parse_a1("A1").unwrap(), "10")
            .unwrap();
        workbook
            .set_cell_input(0, CellAddress::parse_a1("A2").unwrap(), "20")
            .unwrap();
        workbook
            .set_cell_input(
                0,
                CellAddress::parse_a1("B1").unwrap(),
                "=SUM(A1:A2) * 2 + 5",
            )
            .unwrap();

        assert_eq!(
            workbook
                .sheet(0)
                .unwrap()
                .cell(CellAddress::parse_a1("B1").unwrap())
                .unwrap()
                .cached_number,
            Some(65.0)
        );
    }

    #[test]
    fn circular_references_are_rejected() {
        let mut workbook = Workbook::blank();
        workbook
            .sheet_mut(0)
            .unwrap()
            .set_input(CellAddress::parse_a1("A1").unwrap(), "=B1");
        workbook
            .sheet_mut(0)
            .unwrap()
            .set_input(CellAddress::parse_a1("B1").unwrap(), "=A1");

        let mut stack = BTreeSet::new();
        let error = evaluate_formula(
            &workbook,
            0,
            CellAddress::parse_a1("A1").unwrap(),
            "B1",
            &mut stack,
        )
        .unwrap_err();
        assert!(matches!(error, FormulaError::CircularReference(_)));
    }
}
