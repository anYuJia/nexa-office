use nexa_ooxml::PartName;
use std::{collections::BTreeMap, error::Error, fmt, ops::Range};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RgbColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    #[default]
    Portrait,
    Landscape,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RunProperties {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub color: Option<RgbColor>,
    pub font_family: Option<String>,
    pub font_size_half_points: Option<u16>,
    pub style_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineImage {
    pub relationship_id: String,
    pub part_name: Option<PartName>,
    pub width_emu: u64,
    pub height_emu: u64,
    pub alt_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunContent {
    Text(String),
    Tab,
    LineBreak,
    Image(InlineImage),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub content: RunContent,
    pub properties: RunProperties,
}

impl Run {
    #[must_use]
    pub fn text(value: impl Into<String>) -> Self {
        Self {
            content: RunContent::Text(value.into()),
            properties: RunProperties::default(),
        }
    }

    #[must_use]
    pub fn logical_len(&self) -> usize {
        match &self.content {
            RunContent::Text(text) => text.chars().count(),
            RunContent::Tab | RunContent::LineBreak | RunContent::Image(_) => 1,
        }
    }

    fn append_plain_text(&self, output: &mut String) {
        match &self.content {
            RunContent::Text(text) => output.push_str(text),
            RunContent::Tab => output.push('\t'),
            RunContent::LineBreak => output.push('\n'),
            RunContent::Image(_) => output.push('\u{fffc}'),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListReference {
    pub numbering_id: u32,
    pub level: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParagraphProperties {
    pub style_id: Option<String>,
    pub alignment: Alignment,
    pub list: Option<ListReference>,
    pub spacing_before_twips: u32,
    pub spacing_after_twips: u32,
    pub line_spacing_twips: Option<u32>,
    pub indent_left_twips: i32,
    pub indent_right_twips: i32,
    pub first_line_twips: i32,
    pub keep_next: bool,
    pub keep_lines: bool,
    pub page_break_before: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paragraph {
    pub runs: Vec<Run>,
    pub properties: ParagraphProperties,
}

impl Default for Paragraph {
    fn default() -> Self {
        Self {
            runs: vec![Run::text("")],
            properties: ParagraphProperties::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelError {
    InvalidRange {
        start: usize,
        end: usize,
        len: usize,
    },
    InvalidParagraphIndex(usize),
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRange { start, end, len } => {
                write!(f, "invalid text range {start}..{end} for length {len}")
            }
            Self::InvalidParagraphIndex(index) => write!(f, "invalid paragraph index: {index}"),
        }
    }
}

impl Error for ModelError {}

impl Paragraph {
    #[must_use]
    pub fn plain_text(&self) -> String {
        let mut output = String::new();
        for run in &self.runs {
            run.append_plain_text(&mut output);
        }
        output
    }

    #[must_use]
    pub fn logical_len(&self) -> usize {
        self.runs.iter().map(Run::logical_len).sum()
    }

    pub fn replace_range(
        &mut self,
        range: Range<usize>,
        replacement: &str,
    ) -> Result<(), ModelError> {
        let len = self.logical_len();
        validate_range(&range, len)?;

        let (before, tail) = split_runs_at(&self.runs, range.start);
        let (_, after) = split_runs_at(&tail, range.end - range.start);
        let props = before
            .last()
            .or_else(|| after.first())
            .map(|run| run.properties.clone())
            .unwrap_or_default();

        let mut runs = before;
        if !replacement.is_empty() {
            runs.push(Run {
                content: RunContent::Text(replacement.to_owned()),
                properties: props,
            });
        }
        runs.extend(after);
        self.runs = normalize_runs(runs);
        Ok(())
    }

    pub fn split_at(&mut self, offset: usize) -> Result<Paragraph, ModelError> {
        let len = self.logical_len();
        if offset > len {
            return Err(ModelError::InvalidRange {
                start: offset,
                end: offset,
                len,
            });
        }
        let (before, after) = split_runs_at(&self.runs, offset);
        self.runs = normalize_runs(before);
        Ok(Paragraph {
            runs: normalize_runs(after),
            properties: self.properties.clone(),
        })
    }

    pub fn set_range_properties(
        &mut self,
        range: Range<usize>,
        update: impl Fn(&mut RunProperties),
    ) -> Result<(), ModelError> {
        let len = self.logical_len();
        validate_range(&range, len)?;
        let (before, tail) = split_runs_at(&self.runs, range.start);
        let (mut selected, after) = split_runs_at(&tail, range.end - range.start);
        for run in &mut selected {
            update(&mut run.properties);
        }
        let mut runs = before;
        runs.extend(selected);
        runs.extend(after);
        self.runs = normalize_runs(runs);
        Ok(())
    }

    #[must_use]
    pub fn estimated_bytes(&self) -> usize {
        self.runs
            .iter()
            .map(|run| {
                let content = match &run.content {
                    RunContent::Text(text) => text.capacity(),
                    RunContent::Image(image) => {
                        image.relationship_id.capacity()
                            + image.alt_text.as_ref().map_or(0, String::capacity)
                    }
                    RunContent::Tab | RunContent::LineBreak => 0,
                };
                content
                    + run
                        .properties
                        .font_family
                        .as_ref()
                        .map_or(0, String::capacity)
                    + run.properties.style_id.as_ref().map_or(0, String::capacity)
                    + std::mem::size_of::<Run>()
            })
            .sum()
    }
}

fn validate_range(range: &Range<usize>, len: usize) -> Result<(), ModelError> {
    if range.start > range.end || range.end > len {
        return Err(ModelError::InvalidRange {
            start: range.start,
            end: range.end,
            len,
        });
    }
    Ok(())
}

fn split_runs_at(runs: &[Run], offset: usize) -> (Vec<Run>, Vec<Run>) {
    let mut before = Vec::new();
    let mut after = Vec::new();
    let mut position = 0usize;

    for run in runs {
        let len = run.logical_len();
        let end = position.saturating_add(len);

        if end <= offset {
            before.push(run.clone());
        } else if position >= offset {
            after.push(run.clone());
        } else {
            match &run.content {
                RunContent::Text(text) => {
                    let local = offset - position;
                    let byte_index = if local == 0 {
                        0
                    } else {
                        text.char_indices()
                            .nth(local)
                            .map_or(text.len(), |(index, _)| index)
                    };
                    if byte_index > 0 {
                        before.push(Run {
                            content: RunContent::Text(text[..byte_index].to_owned()),
                            properties: run.properties.clone(),
                        });
                    }
                    if byte_index < text.len() {
                        after.push(Run {
                            content: RunContent::Text(text[byte_index..].to_owned()),
                            properties: run.properties.clone(),
                        });
                    }
                }
                RunContent::Tab | RunContent::LineBreak | RunContent::Image(_) => {
                    after.push(run.clone())
                }
            }
        }

        position = end;
    }

    (before, after)
}

fn normalize_runs(runs: Vec<Run>) -> Vec<Run> {
    let mut output: Vec<Run> = Vec::with_capacity(runs.len());
    for run in runs {
        if matches!(&run.content, RunContent::Text(text) if text.is_empty()) && !output.is_empty() {
            continue;
        }
        if let Some(previous) = output.last_mut() {
            if previous.properties == run.properties {
                if let (RunContent::Text(left), RunContent::Text(right)) =
                    (&mut previous.content, &run.content)
                {
                    left.push_str(right);
                    continue;
                }
            }
        }
        output.push(run);
    }
    if output.is_empty() {
        output.push(Run::text(""));
    }
    output
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CellProperties {
    pub width_twips: Option<u32>,
    pub grid_span: u16,
    pub vertical_merge: bool,
    pub shading: Option<RgbColor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TableCell {
    pub blocks: Vec<Block>,
    pub properties: CellProperties,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
    pub exact_height_twips: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TableProperties {
    pub style_id: Option<String>,
    pub width_twips: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Table {
    pub rows: Vec<TableRow>,
    pub properties: TableProperties,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Paragraph(Paragraph),
    Table(Table),
}

impl Block {
    #[must_use]
    pub fn estimated_bytes(&self) -> usize {
        match self {
            Self::Paragraph(paragraph) => paragraph.estimated_bytes(),
            Self::Table(table) => table
                .rows
                .iter()
                .flat_map(|row| &row.cells)
                .flat_map(|cell| &cell.blocks)
                .map(Self::estimated_bytes)
                .sum(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleKind {
    Paragraph,
    Character,
    Table,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Style {
    pub id: String,
    pub name: String,
    pub kind: StyleKind,
    pub based_on: Option<String>,
    pub paragraph: ParagraphProperties,
    pub run: RunProperties,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StyleSheet {
    pub styles: BTreeMap<String, Style>,
    pub default_paragraph_style: Option<String>,
}

impl StyleSheet {
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Style> {
        self.styles.get(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListFormat {
    Decimal,
    LowerLetter,
    UpperLetter,
    LowerRoman,
    UpperRoman,
    Bullet,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListLevel {
    pub level: u8,
    pub start: u32,
    pub format: ListFormat,
    pub text: String,
    pub paragraph: ParagraphProperties,
    pub run: RunProperties,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AbstractList {
    pub id: u32,
    pub levels: BTreeMap<u8, ListLevel>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumberingInstance {
    pub id: u32,
    pub abstract_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Numbering {
    pub abstract_lists: BTreeMap<u32, AbstractList>,
    pub instances: BTreeMap<u32, NumberingInstance>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageMargins {
    pub top_twips: u32,
    pub right_twips: u32,
    pub bottom_twips: u32,
    pub left_twips: u32,
    pub header_twips: u32,
    pub footer_twips: u32,
}

impl Default for PageMargins {
    fn default() -> Self {
        Self {
            top_twips: 1440,
            right_twips: 1440,
            bottom_twips: 1440,
            left_twips: 1440,
            header_twips: 720,
            footer_twips: 720,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionProperties {
    pub page_width_twips: u32,
    pub page_height_twips: u32,
    pub orientation: Orientation,
    pub margins: PageMargins,
    pub header_default: Option<String>,
    pub footer_default: Option<String>,
}

impl Default for SectionProperties {
    fn default() -> Self {
        Self {
            page_width_twips: 12_240,
            page_height_twips: 15_840,
            orientation: Orientation::Portrait,
            margins: PageMargins::default(),
            header_default: None,
            footer_default: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub start_block: usize,
    pub properties: SectionProperties,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HeaderFooter {
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilitySeverity {
    Informational,
    SaveBlocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityIssue {
    pub part: String,
    pub element: String,
    pub detail: String,
    pub severity: CompatibilitySeverity,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CompatibilityReport {
    pub issues: Vec<CompatibilityIssue>,
}

impl CompatibilityReport {
    #[must_use]
    pub fn can_save(&self) -> bool {
        !self
            .issues
            .iter()
            .any(|issue| issue.severity == CompatibilitySeverity::SaveBlocked)
    }

    pub fn block_save(
        &mut self,
        part: impl Into<String>,
        element: impl Into<String>,
        detail: impl Into<String>,
    ) {
        self.issues.push(CompatibilityIssue {
            part: part.into(),
            element: element.into(),
            detail: detail.into(),
            severity: CompatibilitySeverity::SaveBlocked,
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub blocks: Vec<Block>,
    pub styles: StyleSheet,
    pub numbering: Numbering,
    pub sections: Vec<Section>,
    pub headers: BTreeMap<String, HeaderFooter>,
    pub footers: BTreeMap<String, HeaderFooter>,
    pub compatibility: CompatibilityReport,
}

impl Default for Document {
    fn default() -> Self {
        Self::blank()
    }
}

impl Document {
    #[must_use]
    pub fn blank() -> Self {
        Self {
            blocks: vec![Block::Paragraph(Paragraph::default())],
            styles: StyleSheet::default(),
            numbering: Numbering::default(),
            sections: vec![Section {
                start_block: 0,
                properties: SectionProperties::default(),
            }],
            headers: BTreeMap::new(),
            footers: BTreeMap::new(),
            compatibility: CompatibilityReport::default(),
        }
    }

    #[must_use]
    pub fn paragraph_count(&self) -> usize {
        count_paragraphs(&self.blocks)
    }

    #[must_use]
    pub fn paragraph(&self, ordinal: usize) -> Option<&Paragraph> {
        let mut target = ordinal;
        paragraph_ref(&self.blocks, &mut target)
    }

    pub fn paragraph_mut(&mut self, ordinal: usize) -> Option<&mut Paragraph> {
        let mut target = ordinal;
        paragraph_mut(&mut self.blocks, &mut target)
    }

    pub fn replace_paragraph(
        &mut self,
        ordinal: usize,
        replacement: Paragraph,
    ) -> Result<(), ModelError> {
        let target = self
            .paragraph_mut(ordinal)
            .ok_or(ModelError::InvalidParagraphIndex(ordinal))?;
        *target = replacement;
        Ok(())
    }

    pub fn insert_paragraph_after(
        &mut self,
        ordinal: usize,
        paragraph: Paragraph,
    ) -> Result<(), ModelError> {
        let mut target = ordinal;
        if insert_after(&mut self.blocks, &mut target, paragraph) {
            Ok(())
        } else {
            Err(ModelError::InvalidParagraphIndex(ordinal))
        }
    }

    pub fn remove_paragraph(&mut self, ordinal: usize) -> Result<Paragraph, ModelError> {
        let mut target = ordinal;
        remove_paragraph(&mut self.blocks, &mut target)
            .ok_or(ModelError::InvalidParagraphIndex(ordinal))
    }

    #[must_use]
    pub fn plain_text(&self) -> String {
        let mut output = Vec::with_capacity(self.paragraph_count());
        collect_text(&self.blocks, &mut output);
        output.join("\n")
    }

    #[must_use]
    pub fn estimated_bytes(&self) -> usize {
        self.blocks.iter().map(Block::estimated_bytes).sum()
    }
}

fn count_paragraphs(blocks: &[Block]) -> usize {
    blocks
        .iter()
        .map(|block| match block {
            Block::Paragraph(_) => 1,
            Block::Table(table) => table
                .rows
                .iter()
                .flat_map(|row| &row.cells)
                .map(|cell| count_paragraphs(&cell.blocks))
                .sum(),
        })
        .sum()
}

fn paragraph_ref<'a>(blocks: &'a [Block], target: &mut usize) -> Option<&'a Paragraph> {
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => {
                if *target == 0 {
                    return Some(paragraph);
                }
                *target -= 1;
            }
            Block::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        if let Some(value) = paragraph_ref(&cell.blocks, target) {
                            return Some(value);
                        }
                    }
                }
            }
        }
    }
    None
}

fn paragraph_mut<'a>(blocks: &'a mut [Block], target: &mut usize) -> Option<&'a mut Paragraph> {
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => {
                if *target == 0 {
                    return Some(paragraph);
                }
                *target -= 1;
            }
            Block::Table(table) => {
                for row in &mut table.rows {
                    for cell in &mut row.cells {
                        if let Some(value) = paragraph_mut(&mut cell.blocks, target) {
                            return Some(value);
                        }
                    }
                }
            }
        }
    }
    None
}

fn insert_after(blocks: &mut Vec<Block>, target: &mut usize, paragraph: Paragraph) -> bool {
    let mut index = 0usize;
    while index < blocks.len() {
        match &mut blocks[index] {
            Block::Paragraph(_) => {
                if *target == 0 {
                    blocks.insert(index + 1, Block::Paragraph(paragraph));
                    return true;
                }
                *target -= 1;
            }
            Block::Table(table) => {
                for row in &mut table.rows {
                    for cell in &mut row.cells {
                        if insert_after(&mut cell.blocks, target, paragraph.clone()) {
                            return true;
                        }
                    }
                }
            }
        }
        index += 1;
    }
    false
}

fn remove_paragraph(blocks: &mut Vec<Block>, target: &mut usize) -> Option<Paragraph> {
    let mut index = 0usize;
    while index < blocks.len() {
        match &mut blocks[index] {
            Block::Paragraph(_) => {
                if *target == 0 {
                    let Block::Paragraph(value) = blocks.remove(index) else {
                        unreachable!()
                    };
                    return Some(value);
                }
                *target -= 1;
            }
            Block::Table(table) => {
                for row in &mut table.rows {
                    for cell in &mut row.cells {
                        if let Some(value) = remove_paragraph(&mut cell.blocks, target) {
                            return Some(value);
                        }
                    }
                }
            }
        }
        index += 1;
    }
    None
}

fn collect_text(blocks: &[Block], output: &mut Vec<String>) {
    for block in blocks {
        match block {
            Block::Paragraph(paragraph) => output.push(paragraph.plain_text()),
            Block::Table(table) => {
                for row in &table.rows {
                    for cell in &row.cells {
                        collect_text(&cell.blocks, output);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_edit_keeps_surrounding_formatting() {
        let bold = RunProperties {
            bold: true,
            ..RunProperties::default()
        };
        let italic = RunProperties {
            italic: true,
            ..RunProperties::default()
        };
        let mut paragraph = Paragraph {
            runs: vec![
                Run {
                    content: RunContent::Text("Hello".into()),
                    properties: bold,
                },
                Run {
                    content: RunContent::Text(" world".into()),
                    properties: italic,
                },
            ],
            properties: ParagraphProperties::default(),
        };

        paragraph.replace_range(3..8, "p!").unwrap();

        assert_eq!(paragraph.plain_text(), "Help!rld");
        assert!(paragraph.runs.first().unwrap().properties.bold);
        assert!(paragraph.runs.last().unwrap().properties.italic);
    }

    #[test]
    fn table_cell_paragraphs_participate_in_document_navigation() {
        let mut document = Document::blank();
        document.blocks.push(Block::Table(Table {
            rows: vec![TableRow {
                cells: vec![TableCell {
                    blocks: vec![Block::Paragraph(Paragraph {
                        runs: vec![Run::text("cell")],
                        properties: ParagraphProperties::default(),
                    })],
                    properties: CellProperties::default(),
                }],
                exact_height_twips: None,
            }],
            properties: TableProperties::default(),
        }));

        assert_eq!(document.paragraph_count(), 2);
        assert_eq!(document.paragraph(1).unwrap().plain_text(), "cell");
    }
}
