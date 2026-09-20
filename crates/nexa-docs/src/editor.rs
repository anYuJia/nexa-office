use crate::{Document, ModelError, Paragraph, RunProperties};
use std::{
    collections::{BTreeMap, VecDeque},
    error::Error,
    fmt,
    ops::Range,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TextPosition {
    pub paragraph: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: TextPosition,
    pub active: TextPosition,
}

impl Selection {
    #[must_use]
    pub const fn caret(position: TextPosition) -> Self {
        Self {
            anchor: position,
            active: position,
        }
    }

    #[must_use]
    pub const fn is_caret(self) -> bool {
        self.anchor.paragraph == self.active.paragraph && self.anchor.offset == self.active.offset
    }

    #[must_use]
    pub fn ordered(self) -> (TextPosition, TextPosition) {
        if self.anchor <= self.active {
            (self.anchor, self.active)
        } else {
            (self.active, self.anchor)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HistoryLimits {
    pub max_entries: usize,
    pub max_estimated_bytes: usize,
}

impl Default for HistoryLimits {
    fn default() -> Self {
        Self {
            max_entries: 256,
            max_estimated_bytes: 4 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchOptions {
    pub case_sensitive: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            case_sensitive: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMatch {
    pub paragraph: usize,
    pub range: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    Model(ModelError),
    InvalidPosition(TextPosition),
    CrossParagraphSelection,
    EmptySearch,
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Model(error) => write!(f, "{error}"),
            Self::InvalidPosition(position) => write!(
                f,
                "invalid text position: paragraph {}, offset {}",
                position.paragraph, position.offset
            ),
            Self::CrossParagraphSelection => {
                f.write_str("command requires a selection within one paragraph")
            }
            Self::EmptySearch => f.write_str("search query is empty"),
        }
    }
}

impl Error for EditError {}

impl From<ModelError> for EditError {
    fn from(value: ModelError) -> Self {
        Self::Model(value)
    }
}

#[derive(Debug, Clone)]
enum HistoryChange {
    Paragraph {
        index: usize,
        before: Paragraph,
        after: Paragraph,
    },
    ParagraphSet {
        changes: Vec<(usize, Paragraph, Paragraph)>,
    },
    Split {
        index: usize,
        before: Paragraph,
        first: Paragraph,
        second: Paragraph,
    },
}

impl HistoryChange {
    fn estimated_bytes(&self) -> usize {
        match self {
            Self::Paragraph { before, after, .. } => {
                before.estimated_bytes().saturating_add(after.estimated_bytes())
            }
            Self::ParagraphSet { changes } => changes.iter().map(|(_, before, after)| {
                before.estimated_bytes().saturating_add(after.estimated_bytes())
            }).sum(),
            Self::Split {
                before,
                first,
                second,
                ..
            } => before
                .estimated_bytes()
                .saturating_add(first.estimated_bytes())
                .saturating_add(second.estimated_bytes()),
        }
    }

    fn apply_before(&self, document: &mut Document) -> Result<(), ModelError> {
        match self {
            Self::Paragraph { index, before, .. } => document.replace_paragraph(*index, before.clone()),
            Self::ParagraphSet { changes } => {
                for (index, before, _) in changes {
                    document.replace_paragraph(*index, before.clone())?;
                }
                Ok(())
            }
            Self::Split { index, before, .. } => {
                document.replace_paragraph(*index, before.clone())?;
                let _ = document.remove_paragraph(index + 1)?;
                Ok(())
            }
        }
    }

    fn apply_after(&self, document: &mut Document) -> Result<(), ModelError> {
        match self {
            Self::Paragraph { index, after, .. } => document.replace_paragraph(*index, after.clone()),
            Self::ParagraphSet { changes } => {
                for (index, _, after) in changes {
                    document.replace_paragraph(*index, after.clone())?;
                }
                Ok(())
            }
            Self::Split {
                index,
                first,
                second,
                ..
            } => {
                document.replace_paragraph(*index, first.clone())?;
                document.insert_paragraph_after(*index, second.clone())
            }
        }
    }
}

#[derive(Debug, Clone)]
struct HistoryEntry {
    change: HistoryChange,
    before_selection: Selection,
    after_selection: Selection,
    estimated_bytes: usize,
}

#[derive(Debug, Clone)]
struct History {
    undo: VecDeque<HistoryEntry>,
    redo: VecDeque<HistoryEntry>,
    limits: HistoryLimits,
    estimated_bytes: usize,
}

impl History {
    fn new(limits: HistoryLimits) -> Self {
        Self {
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            limits,
            estimated_bytes: 0,
        }
    }

    fn push(&mut self, entry: HistoryEntry) {
        self.redo.clear();

        if entry.estimated_bytes > self.limits.max_estimated_bytes {
            self.undo.clear();
            self.estimated_bytes = 0;
            return;
        }

        self.estimated_bytes = self.estimated_bytes.saturating_add(entry.estimated_bytes);
        self.undo.push_back(entry);

        while self.undo.len() > self.limits.max_entries
            || self.estimated_bytes > self.limits.max_estimated_bytes
        {
            if let Some(removed) = self.undo.pop_front() {
                self.estimated_bytes =
                    self.estimated_bytes.saturating_sub(removed.estimated_bytes);
            } else {
                break;
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct DocsEditor {
    document: Document,
    selection: Selection,
    history: History,
}

impl DocsEditor {
    #[must_use]
    pub fn new(document: Document) -> Self {
        Self::with_history_limits(document, HistoryLimits::default())
    }

    #[must_use]
    pub fn with_history_limits(document: Document, limits: HistoryLimits) -> Self {
        Self {
            document,
            selection: Selection::caret(TextPosition {
                paragraph: 0,
                offset: 0,
            }),
            history: History::new(limits),
        }
    }

    #[must_use]
    pub fn document(&self) -> &Document {
        &self.document
    }

    pub fn document_mut(&mut self) -> &mut Document {
        &mut self.document
    }

    #[must_use]
    pub const fn selection(&self) -> Selection {
        self.selection
    }

    pub fn set_selection(&mut self, selection: Selection) -> Result<(), EditError> {
        self.validate_position(selection.anchor)?;
        self.validate_position(selection.active)?;
        self.selection = selection;
        Ok(())
    }

    pub fn set_caret(&mut self, position: TextPosition) -> Result<(), EditError> {
        self.set_selection(Selection::caret(position))
    }

    pub fn insert_text(&mut self, text: &str) -> Result<(), EditError> {
        let (start, end) = self.selection.ordered();
        if start.paragraph != end.paragraph {
            return Err(EditError::CrossParagraphSelection);
        }

        let before = self
            .document
            .paragraph(start.paragraph)
            .ok_or(EditError::InvalidPosition(start))?
            .clone();
        let mut after = before.clone();
        after.replace_range(start.offset..end.offset, text)?;
        let new_offset = start.offset.saturating_add(text.chars().count());
        let after_selection = Selection::caret(TextPosition {
            paragraph: start.paragraph,
            offset: new_offset,
        });

        self.commit_change(
            HistoryChange::Paragraph {
                index: start.paragraph,
                before,
                after,
            },
            after_selection,
        )
    }

    pub fn delete_backward(&mut self) -> Result<(), EditError> {
        if !self.selection.is_caret() {
            return self.insert_text("");
        }
        let caret = self.selection.active;
        if caret.offset == 0 {
            return Ok(());
        }

        let before = self
            .document
            .paragraph(caret.paragraph)
            .ok_or(EditError::InvalidPosition(caret))?
            .clone();
        let mut after = before.clone();
        after.replace_range(caret.offset - 1..caret.offset, "")?;

        self.commit_change(
            HistoryChange::Paragraph {
                index: caret.paragraph,
                before,
                after,
            },
            Selection::caret(TextPosition {
                paragraph: caret.paragraph,
                offset: caret.offset - 1,
            }),
        )
    }

    pub fn insert_paragraph(&mut self) -> Result<(), EditError> {
        if !self.selection.is_caret() {
            return Err(EditError::CrossParagraphSelection);
        }

        let caret = self.selection.active;
        let before = self
            .document
            .paragraph(caret.paragraph)
            .ok_or(EditError::InvalidPosition(caret))?
            .clone();
        let mut first = before.clone();
        let second = first.split_at(caret.offset)?;

        self.commit_change(
            HistoryChange::Split {
                index: caret.paragraph,
                before,
                first,
                second,
            },
            Selection::caret(TextPosition {
                paragraph: caret.paragraph + 1,
                offset: 0,
            }),
        )
    }

    pub fn toggle_bold(&mut self) -> Result<(), EditError> {
        self.map_selected_runs(|properties| properties.bold = !properties.bold)
    }

    pub fn toggle_italic(&mut self) -> Result<(), EditError> {
        self.map_selected_runs(|properties| properties.italic = !properties.italic)
    }

    pub fn toggle_underline(&mut self) -> Result<(), EditError> {
        self.map_selected_runs(|properties| properties.underline = !properties.underline)
    }

    pub fn set_paragraph_style(&mut self, style_id: Option<String>) -> Result<(), EditError> {
        let index = self.selection.active.paragraph;
        let before = self
            .document
            .paragraph(index)
            .ok_or(EditError::InvalidPosition(self.selection.active))?
            .clone();
        let mut after = before.clone();
        after.properties.style_id = style_id;

        self.commit_change(
            HistoryChange::Paragraph {
                index,
                before,
                after,
            },
            self.selection,
        )
    }

    pub fn find_all(
        &self,
        query: &str,
        options: SearchOptions,
    ) -> Result<Vec<SearchMatch>, EditError> {
        if query.is_empty() {
            return Err(EditError::EmptySearch);
        }

        let mut matches = Vec::new();
        for paragraph_index in 0..self.document.paragraph_count() {
            let Some(paragraph) = self.document.paragraph(paragraph_index) else {
                continue;
            };
            let text = paragraph.plain_text();
            if options.case_sensitive {
                for (byte_start, _) in text.match_indices(query) {
                    let start = text[..byte_start].chars().count();
                    matches.push(SearchMatch {
                        paragraph: paragraph_index,
                        range: start..start + query.chars().count(),
                    });
                }
            } else {
                find_ascii_case_insensitive(&text, query, paragraph_index, &mut matches);
            }
        }
        Ok(matches)
    }

    pub fn replace_all(
        &mut self,
        query: &str,
        replacement: &str,
        options: SearchOptions,
    ) -> Result<usize, EditError> {
        let matches = self.find_all(query, options)?;
        if matches.is_empty() {
            return Ok(0);
        }

        let count = matches.len();
        let mut grouped: BTreeMap<usize, Vec<Range<usize>>> = BTreeMap::new();
        for found in matches {
            grouped.entry(found.paragraph).or_default().push(found.range);
        }

        let mut changes = Vec::with_capacity(grouped.len());
        for (index, mut ranges) in grouped {
            ranges.sort_by_key(|range| range.start);
            let before = self
                .document
                .paragraph(index)
                .ok_or(EditError::InvalidPosition(TextPosition {
                    paragraph: index,
                    offset: 0,
                }))?
                .clone();
            let mut after = before.clone();
            for range in ranges.into_iter().rev() {
                after.replace_range(range, replacement)?;
            }
            changes.push((index, before, after));
        }

        self.commit_change(
            HistoryChange::ParagraphSet { changes },
            Selection::caret(TextPosition {
                paragraph: 0,
                offset: 0,
            }),
        )?;

        Ok(count)
    }

    pub fn undo(&mut self) -> Result<bool, EditError> {
        let Some(entry) = self.history.undo.pop_back() else {
            return Ok(false);
        };
        entry.change.apply_before(&mut self.document)?;
        self.selection = entry.before_selection;
        self.history.estimated_bytes = self
            .history
            .estimated_bytes
            .saturating_sub(entry.estimated_bytes);
        self.history.redo.push_back(entry);
        Ok(true)
    }

    pub fn redo(&mut self) -> Result<bool, EditError> {
        let Some(entry) = self.history.redo.pop_back() else {
            return Ok(false);
        };
        entry.change.apply_after(&mut self.document)?;
        self.selection = entry.after_selection;
        self.history.estimated_bytes = self
            .history
            .estimated_bytes
            .saturating_add(entry.estimated_bytes);
        self.history.undo.push_back(entry);
        Ok(true)
    }

    #[must_use]
    pub fn undo_depth(&self) -> usize {
        self.history.undo.len()
    }

    fn map_selected_runs(
        &mut self,
        update: impl Fn(&mut RunProperties),
    ) -> Result<(), EditError> {
        let (start, end) = self.selection.ordered();
        if start.paragraph != end.paragraph {
            return Err(EditError::CrossParagraphSelection);
        }

        let before = self
            .document
            .paragraph(start.paragraph)
            .ok_or(EditError::InvalidPosition(start))?
            .clone();
        let mut after = before.clone();
        after.set_range_properties(start.offset..end.offset, update)?;

        self.commit_change(
            HistoryChange::Paragraph {
                index: start.paragraph,
                before,
                after,
            },
            self.selection,
        )
    }

    fn validate_position(&self, position: TextPosition) -> Result<(), EditError> {
        let paragraph = self
            .document
            .paragraph(position.paragraph)
            .ok_or(EditError::InvalidPosition(position))?;
        if position.offset > paragraph.logical_len() {
            return Err(EditError::InvalidPosition(position));
        }
        Ok(())
    }

    fn commit_change(
        &mut self,
        change: HistoryChange,
        after_selection: Selection,
    ) -> Result<(), EditError> {
        let before_selection = self.selection;
        change.apply_after(&mut self.document)?;
        let estimated_bytes = change.estimated_bytes();
        self.selection = after_selection;
        self.history.push(HistoryEntry {
            change,
            before_selection,
            after_selection,
            estimated_bytes,
        });
        Ok(())
    }
}

fn find_ascii_case_insensitive(
    text: &str,
    query: &str,
    paragraph: usize,
    output: &mut Vec<SearchMatch>,
) {
    let text_chars: Vec<char> = text.chars().collect();
    let query_chars: Vec<char> = query.chars().collect();
    if query_chars.is_empty() || query_chars.len() > text_chars.len() {
        return;
    }

    for start in 0..=text_chars.len() - query_chars.len() {
        if text_chars[start..start + query_chars.len()]
            .iter()
            .zip(&query_chars)
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
        {
            output.push(SearchMatch {
                paragraph,
                range: start..start + query_chars.len(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Block, ParagraphProperties, Run};

    fn document_with_text(text: &str) -> Document {
        Document {
            blocks: vec![Block::Paragraph(Paragraph {
                runs: vec![Run::text(text)],
                properties: ParagraphProperties::default(),
            })],
            ..Document::blank()
        }
    }

    #[test]
    fn typing_undo_and_redo_round_trip() {
        let mut editor = DocsEditor::new(document_with_text("Hello"));
        editor
            .set_caret(TextPosition {
                paragraph: 0,
                offset: 5,
            })
            .unwrap();
        editor.insert_text(" world").unwrap();

        assert_eq!(editor.document().plain_text(), "Hello world");
        assert!(editor.undo().unwrap());
        assert_eq!(editor.document().plain_text(), "Hello");
        assert!(editor.redo().unwrap());
        assert_eq!(editor.document().plain_text(), "Hello world");
    }

    #[test]
    fn paragraph_split_is_reversible() {
        let mut editor = DocsEditor::new(document_with_text("abcdef"));
        editor
            .set_caret(TextPosition {
                paragraph: 0,
                offset: 3,
            })
            .unwrap();

        editor.insert_paragraph().unwrap();
        assert_eq!(editor.document().plain_text(), "abc\ndef");

        assert!(editor.undo().unwrap());
        assert_eq!(editor.document().plain_text(), "abcdef");
    }

    #[test]
    fn replace_all_records_one_history_entry() {
        let mut editor = DocsEditor::new(document_with_text("one two one"));
        let replaced = editor
            .replace_all("one", "three", SearchOptions::default())
            .unwrap();

        assert_eq!(replaced, 2);
        assert_eq!(editor.document().plain_text(), "three two three");
        assert_eq!(editor.undo_depth(), 1);
    }

    #[test]
    fn history_is_bounded_by_estimated_bytes() {
        let mut editor = DocsEditor::with_history_limits(
            document_with_text("a"),
            HistoryLimits {
                max_entries: 100,
                max_estimated_bytes: 512,
            },
        );
        editor
            .set_caret(TextPosition {
                paragraph: 0,
                offset: 1,
            })
            .unwrap();

        for _ in 0..40 {
            editor.insert_text("x").unwrap();
        }

        assert!(editor.undo_depth() < 40);
    }
}
