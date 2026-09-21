use crate::{Block, Document, Paragraph, Run, RunContent, SectionProperties, Table};

const DEFAULT_FONT_SIZE_HALF_POINTS: u16 = 22;
const EMU_PER_TWIP: u64 = 635;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutConfig {
    pub default_font_size_half_points: u16,
    pub minimum_line_height_twips: u32,
    pub paragraph_minimum_height_twips: u32,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            default_font_size_half_points: DEFAULT_FONT_SIZE_HALF_POINTS,
            minimum_line_height_twips: 240,
            paragraph_minimum_height_twips: 240,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutBlock {
    pub source_block: usize,
    pub line_start: u32,
    pub line_count: u32,
    pub y_twips: u32,
    pub height_twips: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageLayout {
    pub section_index: usize,
    pub width_twips: u32,
    pub height_twips: u32,
    pub content_top_twips: u32,
    pub content_bottom_twips: u32,
    pub used_height_twips: u32,
    pub blocks: Vec<LayoutBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DocumentLayout {
    pub pages: Vec<PageLayout>,
}

impl DocumentLayout {
    #[must_use]
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Paginator {
    config: LayoutConfig,
}

impl Default for Paginator {
    fn default() -> Self {
        Self::new(LayoutConfig::default())
    }
}

impl Paginator {
    #[must_use]
    pub const fn new(config: LayoutConfig) -> Self {
        Self { config }
    }

    #[must_use]
    pub fn layout(&self, document: &Document) -> DocumentLayout {
        let mut pages = Vec::new();

        for (block_index, block) in document.blocks.iter().enumerate() {
            let (section_index, section) = section_for_block(document, block_index);
            match block {
                Block::Paragraph(paragraph) => {
                    self.layout_paragraph(
                        &mut pages,
                        block_index,
                        section_index,
                        section,
                        paragraph,
                    );
                }
                Block::Table(table) => {
                    self.layout_table(&mut pages, block_index, section_index, section, table);
                }
            }
        }

        if pages.is_empty() {
            let section = document
                .sections
                .first()
                .map_or_else(SectionProperties::default, |item| item.properties.clone());
            pages.push(new_page(0, &section));
        }

        DocumentLayout { pages }
    }

    fn layout_paragraph(
        &self,
        pages: &mut Vec<PageLayout>,
        block_index: usize,
        section_index: usize,
        section: &SectionProperties,
        paragraph: &Paragraph,
    ) {
        let measurement = self.measure_paragraph(paragraph, section);

        if paragraph.properties.page_break_before {
            ensure_fresh_page(pages, section_index, section);
        } else {
            ensure_page(pages, section_index, section);
        }

        let mut line_start = 0u32;
        let mut remaining_lines = measurement.line_count.max(1);
        let mut first_fragment = true;

        while remaining_lines > 0 {
            ensure_page(pages, section_index, section);
            let page = pages.last_mut().expect("page exists");
            let available = available_height(section).saturating_sub(page.used_height_twips);

            let before = if first_fragment {
                paragraph.properties.spacing_before_twips
            } else {
                0
            };
            let after = if remaining_lines == 1 {
                paragraph.properties.spacing_after_twips
            } else {
                0
            };

            let usable_for_lines = available.saturating_sub(before).saturating_sub(after);
            let mut lines_fit = usable_for_lines
                .checked_div(measurement.line_height_twips)
                .unwrap_or(remaining_lines);

            if lines_fit == 0 {
                if page.used_height_twips > 0 {
                    pages.push(new_page(section_index, section));
                    continue;
                }
                lines_fit = 1;
            }

            lines_fit = lines_fit.min(remaining_lines);
            let is_last = lines_fit == remaining_lines;
            let fragment_after = if is_last {
                paragraph.properties.spacing_after_twips
            } else {
                0
            };
            let fragment_before = if first_fragment {
                paragraph.properties.spacing_before_twips
            } else {
                0
            };
            let height = fragment_before
                .saturating_add(measurement.line_height_twips.saturating_mul(lines_fit))
                .saturating_add(fragment_after)
                .max(
                    self.config
                        .paragraph_minimum_height_twips
                        .min(available.max(1)),
                );

            let page = pages.last_mut().expect("page exists");
            let y = section
                .margins
                .top_twips
                .saturating_add(page.used_height_twips);
            page.blocks.push(LayoutBlock {
                source_block: block_index,
                line_start,
                line_count: lines_fit,
                y_twips: y,
                height_twips: height,
            });
            page.used_height_twips = page.used_height_twips.saturating_add(height);

            remaining_lines -= lines_fit;
            line_start = line_start.saturating_add(lines_fit);
            first_fragment = false;

            if remaining_lines > 0 {
                pages.push(new_page(section_index, section));
            }
        }
    }

    fn layout_table(
        &self,
        pages: &mut Vec<PageLayout>,
        block_index: usize,
        section_index: usize,
        section: &SectionProperties,
        table: &Table,
    ) {
        ensure_page(pages, section_index, section);

        let mut row_start = 0u32;
        for row in &table.rows {
            let row_height = self.measure_table_row(row, section);
            let page = pages.last_mut().expect("page exists");
            let available = available_height(section).saturating_sub(page.used_height_twips);

            if row_height > available && page.used_height_twips > 0 {
                pages.push(new_page(section_index, section));
            }

            let page = pages.last_mut().expect("page exists");
            let y = section
                .margins
                .top_twips
                .saturating_add(page.used_height_twips);
            let placed_height = row_height.min(available_height(section).max(1));
            page.blocks.push(LayoutBlock {
                source_block: block_index,
                line_start: row_start,
                line_count: 1,
                y_twips: y,
                height_twips: placed_height,
            });
            page.used_height_twips = page.used_height_twips.saturating_add(placed_height);
            row_start = row_start.saturating_add(1);
        }

        if table.rows.is_empty() {
            let page = pages.last_mut().expect("page exists");
            let height = self.config.paragraph_minimum_height_twips;
            let y = section
                .margins
                .top_twips
                .saturating_add(page.used_height_twips);
            page.blocks.push(LayoutBlock {
                source_block: block_index,
                line_start: 0,
                line_count: 1,
                y_twips: y,
                height_twips: height,
            });
            page.used_height_twips = page.used_height_twips.saturating_add(height);
        }
    }

    fn measure_paragraph(
        &self,
        paragraph: &Paragraph,
        section: &SectionProperties,
    ) -> ParagraphMeasurement {
        let content_width = available_width(section);
        let mut width = 0u32;
        let mut lines = 1u32;
        let mut line_height = self.config.minimum_line_height_twips;

        for run in &paragraph.runs {
            line_height = line_height.max(run_line_height(run, self.config));
            match &run.content {
                RunContent::LineBreak => {
                    lines = lines.saturating_add(1);
                    width = 0;
                }
                RunContent::Image(image) => {
                    let image_width =
                        u32::try_from(image.width_emu / EMU_PER_TWIP).unwrap_or(u32::MAX);
                    let image_height =
                        u32::try_from(image.height_emu / EMU_PER_TWIP).unwrap_or(u32::MAX);
                    if width > 0 && width.saturating_add(image_width) > content_width {
                        lines = lines.saturating_add(1);
                        width = 0;
                    }
                    width = width.saturating_add(image_width.min(content_width));
                    line_height = line_height.max(image_height);
                }
                RunContent::Tab => {
                    width = advance_width(width, 720, content_width, &mut lines);
                }
                RunContent::Text(text) => {
                    for character in text.chars() {
                        width = advance_width(
                            width,
                            estimated_character_width(character, run, self.config),
                            content_width,
                            &mut lines,
                        );
                    }
                }
            }
        }

        ParagraphMeasurement {
            line_count: lines,
            line_height_twips: paragraph
                .properties
                .line_spacing_twips
                .unwrap_or(line_height)
                .max(self.config.minimum_line_height_twips),
        }
    }

    fn measure_table_row(&self, row: &crate::TableRow, section: &SectionProperties) -> u32 {
        let mut height = row.exact_height_twips.unwrap_or(0);
        for cell in &row.cells {
            let mut cell_height = 0u32;
            for block in &cell.blocks {
                cell_height = cell_height.saturating_add(match block {
                    Block::Paragraph(paragraph) => {
                        let measured = self.measure_paragraph(paragraph, section);
                        paragraph
                            .properties
                            .spacing_before_twips
                            .saturating_add(
                                measured
                                    .line_height_twips
                                    .saturating_mul(measured.line_count),
                            )
                            .saturating_add(paragraph.properties.spacing_after_twips)
                    }
                    Block::Table(table) => table
                        .rows
                        .iter()
                        .map(|nested| self.measure_table_row(nested, section))
                        .fold(0u32, u32::saturating_add),
                });
            }
            height = height.max(cell_height);
        }
        height.max(self.config.paragraph_minimum_height_twips)
    }
}

#[derive(Debug, Clone, Copy)]
struct ParagraphMeasurement {
    line_count: u32,
    line_height_twips: u32,
}

fn ensure_page(pages: &mut Vec<PageLayout>, section_index: usize, section: &SectionProperties) {
    if pages
        .last()
        .is_none_or(|page| page.section_index != section_index)
    {
        pages.push(new_page(section_index, section));
    }
}

fn ensure_fresh_page(
    pages: &mut Vec<PageLayout>,
    section_index: usize,
    section: &SectionProperties,
) {
    if pages
        .last()
        .is_none_or(|page| page.section_index != section_index || page.used_height_twips > 0)
    {
        pages.push(new_page(section_index, section));
    }
}

fn new_page(section_index: usize, section: &SectionProperties) -> PageLayout {
    PageLayout {
        section_index,
        width_twips: section.page_width_twips,
        height_twips: section.page_height_twips,
        content_top_twips: section.margins.top_twips,
        content_bottom_twips: section
            .page_height_twips
            .saturating_sub(section.margins.bottom_twips),
        used_height_twips: 0,
        blocks: Vec::new(),
    }
}

fn section_for_block(document: &Document, block_index: usize) -> (usize, &SectionProperties) {
    let first = document.sections.first();
    let mut selected_index = 0usize;
    let mut selected = first.map(|item| &item.properties);

    for (index, section) in document.sections.iter().enumerate() {
        if section.start_block > block_index {
            break;
        }
        selected_index = index;
        selected = Some(&section.properties);
    }

    if let Some(section) = selected {
        (selected_index, section)
    } else {
        static DEFAULT: std::sync::LazyLock<SectionProperties> =
            std::sync::LazyLock::new(SectionProperties::default);
        (0, &DEFAULT)
    }
}

fn available_width(section: &SectionProperties) -> u32 {
    section
        .page_width_twips
        .saturating_sub(section.margins.left_twips)
        .saturating_sub(section.margins.right_twips)
        .max(1)
}

fn available_height(section: &SectionProperties) -> u32 {
    section
        .page_height_twips
        .saturating_sub(section.margins.top_twips)
        .saturating_sub(section.margins.bottom_twips)
        .max(1)
}

fn run_line_height(run: &Run, config: LayoutConfig) -> u32 {
    u32::from(
        run.properties
            .font_size_half_points
            .unwrap_or(config.default_font_size_half_points),
    )
    .saturating_mul(12)
    .max(config.minimum_line_height_twips)
}

fn estimated_character_width(character: char, run: &Run, config: LayoutConfig) -> u32 {
    let em_twips = u32::from(
        run.properties
            .font_size_half_points
            .unwrap_or(config.default_font_size_half_points),
    )
    .saturating_mul(10);

    if character == ' ' {
        em_twips / 3
    } else if is_wide_character(character) {
        em_twips
    } else {
        em_twips / 2
    }
    .max(1)
}

fn is_wide_character(character: char) -> bool {
    matches!(
        character as u32,
        0x1100..=0x115f
            | 0x2e80..=0xa4cf
            | 0xac00..=0xd7a3
            | 0xf900..=0xfaff
            | 0xfe10..=0xfe6f
            | 0xff00..=0xff60
            | 0x1f300..=0x1faff
    )
}

fn advance_width(current: u32, item: u32, limit: u32, lines: &mut u32) -> u32 {
    if current > 0 && current.saturating_add(item) > limit {
        *lines = lines.saturating_add(1);
        item.min(limit)
    } else {
        current.saturating_add(item).min(limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Block, Paragraph, ParagraphProperties, Run, Section};

    #[test]
    fn long_paragraph_is_split_across_pages() {
        let document = Document {
            blocks: vec![Block::Paragraph(Paragraph {
                runs: vec![Run::text("Nexa ".repeat(8000))],
                properties: ParagraphProperties::default(),
            })],
            ..Document::blank()
        };

        let layout = Paginator::default().layout(&document);

        assert!(layout.page_count() > 1);
        assert!(
            layout
                .pages
                .iter()
                .all(|page| page.used_height_twips <= 12_960)
        );
    }

    #[test]
    fn explicit_page_break_starts_new_page() {
        let second = ParagraphProperties {
            page_break_before: true,
            ..ParagraphProperties::default()
        };
        let document = Document {
            blocks: vec![
                Block::Paragraph(Paragraph {
                    runs: vec![Run::text("first")],
                    properties: ParagraphProperties::default(),
                }),
                Block::Paragraph(Paragraph {
                    runs: vec![Run::text("second")],
                    properties: second,
                }),
            ],
            ..Document::blank()
        };

        assert_eq!(Paginator::default().layout(&document).page_count(), 2);
    }

    #[test]
    fn section_geometry_starts_a_new_page() {
        let mut document = Document {
            blocks: vec![
                Block::Paragraph(Paragraph::default()),
                Block::Paragraph(Paragraph::default()),
            ],
            ..Document::blank()
        };
        document.sections.push(Section {
            start_block: 1,
            properties: SectionProperties {
                page_width_twips: 20_000,
                ..SectionProperties::default()
            },
        });

        let layout = Paginator::default().layout(&document);

        assert_eq!(layout.page_count(), 2);
        assert_eq!(layout.pages[1].width_twips, 20_000);
    }
}
