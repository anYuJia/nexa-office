use nexa_docs::{
    Alignment, Block, CellProperties, DocsEditor, DocxDocument, ListReference, Paragraph,
    ParagraphProperties, Paginator, Run, RunContent, RunProperties, Table, TableCell,
    TableProperties, TableRow, TextPosition, open_docx, save_docx,
};
use std::{
    io::Cursor,
    time::Instant,
};

const PAGE_COUNT: usize = 20;
const BODY_PARAGRAPHS_PER_PAGE: usize = 12;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let build_start = Instant::now();
    let mut docx = build_fixture();
    let build_us = build_start.elapsed().as_micros();

    let document = docx.document();
    let paragraphs = document.paragraph_count();
    let estimated_model_bytes = document.estimated_bytes();

    let layout_start = Instant::now();
    let layout = Paginator::default().layout(document);
    let layout_us = layout_start.elapsed().as_micros();

    let save_start = Instant::now();
    let initial_bytes = save_docx(&mut docx, Cursor::new(Vec::new()))?.into_inner();
    let initial_save_us = save_start.elapsed().as_micros();

    let open_start = Instant::now();
    let mut reopened = open_docx(Cursor::new(initial_bytes.clone()))?;
    let reopen_us = open_start.elapsed().as_micros();

    let edit_start = Instant::now();
    let mut editor = DocsEditor::new(reopened.document().clone());
    let first_len = editor
        .document()
        .paragraph(0)
        .map_or(0, nexa_docs::Paragraph::logical_len);
    editor.set_caret(TextPosition {
        paragraph: 0,
        offset: first_len,
    })?;
    editor.insert_text(" · edited by Nexa")?;
    let matches = editor.find_all(
        "Nexa",
        nexa_docs::SearchOptions {
            case_sensitive: false,
        },
    )?;
    let edit_us = edit_start.elapsed().as_micros();

    reopened.replace_document(editor.document().clone());

    let edited_save_start = Instant::now();
    let edited_bytes = save_docx(&mut reopened, Cursor::new(Vec::new()))?.into_inner();
    let edited_save_us = edited_save_start.elapsed().as_micros();

    let verify_start = Instant::now();
    let verified = open_docx(Cursor::new(edited_bytes.clone()))?;
    let verify_us = verify_start.elapsed().as_micros();

    if !verified.document().plain_text().contains("edited by Nexa") {
        return Err("edited text did not survive DOCX save/reopen".into());
    }

    println!("fixture_requested_pages={PAGE_COUNT}");
    println!("fixture_body_paragraphs_per_page={BODY_PARAGRAPHS_PER_PAGE}");
    println!("paragraphs={paragraphs}");
    println!("layout_pages={}", layout.page_count());
    println!("estimated_model_bytes={estimated_model_bytes}");
    println!("initial_docx_bytes={}", initial_bytes.len());
    println!("edited_docx_bytes={}", edited_bytes.len());
    println!("search_matches={}", matches.len());
    println!("build_us={build_us}");
    println!("layout_us={layout_us}");
    println!("initial_save_us={initial_save_us}");
    println!("reopen_us={reopen_us}");
    println!("edit_us={edit_us}");
    println!("edited_save_us={edited_save_us}");
    println!("verify_reopen_us={verify_us}");

    Ok(())
}

fn build_fixture() -> DocxDocument {
    let mut docx = DocxDocument::blank();
    let document = docx.document_mut();
    document.blocks.clear();

    for page in 0..PAGE_COUNT {
        document.blocks.push(Block::Paragraph(Paragraph {
            runs: vec![Run {
                content: RunContent::Text(format!("Nexa Docs performance page {}", page + 1)),
                properties: RunProperties {
                    bold: true,
                    font_size_half_points: Some(32),
                    ..RunProperties::default()
                },
            }],
            properties: ParagraphProperties {
                alignment: Alignment::Center,
                page_break_before: page > 0,
                spacing_after_twips: 240,
                ..ParagraphProperties::default()
            },
        }));

        for paragraph_index in 0..BODY_PARAGRAPHS_PER_PAGE {
            let body = format!(
                "Page {} paragraph {}. Nexa Office keeps document semantics native and bounded.                  This representative paragraph includes Latin text, 中文文本, numbers 12345,                  punctuation, and enough words to exercise wrapping and pagination.",
                page + 1,
                paragraph_index + 1
            );
            document.blocks.push(Block::Paragraph(Paragraph {
                runs: vec![
                    Run {
                        content: RunContent::Text(body),
                        properties: RunProperties::default(),
                    },
                    Run {
                        content: RunContent::Text(" Bold segment.".into()),
                        properties: RunProperties {
                            bold: true,
                            ..RunProperties::default()
                        },
                    },
                    Run {
                        content: RunContent::Text(" Italic segment.".into()),
                        properties: RunProperties {
                            italic: true,
                            ..RunProperties::default()
                        },
                    },
                ],
                properties: ParagraphProperties {
                    list: (paragraph_index % 4 == 0).then_some(ListReference {
                        numbering_id: 1,
                        level: 0,
                    }),
                    spacing_after_twips: 80,
                    ..ParagraphProperties::default()
                },
            }));
        }

        document.blocks.push(Block::Table(Table {
            rows: (0..3)
                .map(|row| TableRow {
                    cells: (0..3)
                        .map(|column| TableCell {
                            blocks: vec![Block::Paragraph(Paragraph {
                                runs: vec![Run::text(format!(
                                    "P{} R{} C{}",
                                    page + 1,
                                    row + 1,
                                    column + 1
                                ))],
                                properties: ParagraphProperties::default(),
                            })],
                            properties: CellProperties {
                                width_twips: Some(2400),
                                grid_span: 1,
                                vertical_merge: false,
                                shading: None,
                            },
                        })
                        .collect(),
                    exact_height_twips: None,
                })
                .collect(),
            properties: TableProperties {
                style_id: None,
                width_twips: Some(7200),
            },
        }));
    }

    docx
}
