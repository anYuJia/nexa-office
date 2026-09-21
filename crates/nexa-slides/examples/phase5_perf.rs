use nexa_slides::{PptxPresentation, SlideElement, open_pptx, save_pptx};
use std::{
    io::Cursor,
    time::Instant,
};

fn main() {
    let mut pptx = PptxPresentation::blank();

    for slide_index in 0..180 {
        if slide_index > 0 {
            pptx.presentation_mut().add_slide();
        }
        let slide = pptx
            .presentation_mut()
            .slide_mut(slide_index)
            .expect("slide exists");

        for element_index in 0..12 {
            let index = slide.add_text_box(format!(
                "Slide {} · native text element {}",
                slide_index + 1,
                element_index + 1
            ));
            if let SlideElement::TextBox(text) = &mut slide.elements[index] {
                text.bounds.x = 0.7 + (element_index % 3) as f64 * 3.9;
                text.bounds.y = 0.7 + (element_index / 3) as f64 * 1.45;
                text.bounds.width = 3.4;
                text.bounds.height = 0.8;
            }
        }

        if slide_index % 5 == 0 {
            let table = slide.add_table(4, 4).expect("table");
            if let SlideElement::Table(table) = &mut slide.elements[table] {
                for row in 0..4 {
                    for column in 0..4 {
                        table.cell_mut(row, column).expect("cell").text =
                            format!("{}:{}", row + 1, column + 1);
                    }
                }
            }
        }
    }

    let element_count: usize = pptx
        .presentation()
        .slides()
        .iter()
        .map(|slide| slide.elements.len())
        .sum();
    let text_bytes: usize = pptx
        .presentation()
        .slides()
        .iter()
        .flat_map(|slide| slide.elements.iter())
        .map(|element| element.text().len())
        .sum();

    let save_started = Instant::now();
    let cursor = save_pptx(&mut pptx, Cursor::new(Vec::new())).expect("save");
    let save_us = save_started.elapsed().as_micros();
    let bytes = cursor.into_inner();

    let reopen_started = Instant::now();
    let reopened = open_pptx(Cursor::new(bytes.clone())).expect("reopen");
    let reopen_us = reopen_started.elapsed().as_micros();

    let scan_started = Instant::now();
    let scanned_elements: usize = reopened
        .presentation()
        .slides()
        .iter()
        .map(|slide| slide.elements.len())
        .sum();
    let scan_us = scan_started.elapsed().as_micros();

    println!("slides={}", reopened.presentation().slides().len());
    println!("elements={element_count}");
    println!("scanned_elements={scanned_elements}");
    println!("text_bytes={text_bytes}");
    println!("pptx_bytes={}", bytes.len());
    println!("save_us={save_us}");
    println!("reopen_us={reopen_us}");
    println!("scan_us={scan_us}");
}
