use nexa_slides::{PptxPresentation, ShapeKind, SlideElement, open_pptx, save_pptx};
use std::io::Cursor;

#[test]
fn generated_presentation_edits_save_and_reopen() {
    let mut pptx = PptxPresentation::blank();
    {
        let slide = pptx.presentation_mut().slide_mut(0).unwrap();
        slide.add_text_box("Quarterly review");
        let shape = slide.add_shape(ShapeKind::RoundedRectangle);
        let SlideElement::Shape(shape) = &mut slide.elements[shape] else {
            panic!("shape expected");
        };
        shape.text = "Launch".into();

        let table = slide.add_table(3, 2).unwrap();
        let SlideElement::Table(table) = &mut slide.elements[table] else {
            panic!("table expected");
        };
        table.cell_mut(0, 0).unwrap().text = "Metric".into();
        table.cell_mut(0, 1).unwrap().text = "Value".into();
        table.cell_mut(1, 0).unwrap().text = "Memory".into();
        table.cell_mut(1, 1).unwrap().text = "Low".into();
    }
    pptx.presentation_mut().add_slide();
    pptx.presentation_mut()
        .slide_mut(1)
        .unwrap()
        .add_text_box("第二页");

    let cursor = save_pptx(&mut pptx, Cursor::new(Vec::new())).unwrap();
    let reopened = open_pptx(Cursor::new(cursor.into_inner())).unwrap();

    assert_eq!(reopened.presentation().slides().len(), 2);
    assert!(
        reopened
            .presentation()
            .slide(0)
            .unwrap()
            .elements
            .iter()
            .any(|element| element.text().contains("Quarterly review"))
    );
    assert!(
        reopened
            .presentation()
            .slide(1)
            .unwrap()
            .elements
            .iter()
            .any(|element| element.text().contains("第二页"))
    );
}

#[test]
fn large_slide_deck_does_not_materialize_extra_ui_state() {
    let mut pptx = PptxPresentation::blank();
    for index in 1..500 {
        pptx.presentation_mut().add_slide();
        pptx.presentation_mut()
            .slide_mut(index)
            .unwrap()
            .add_text_box(index.to_string());
    }
    assert_eq!(pptx.presentation().slides().len(), 500);
    assert_eq!(
        pptx.presentation()
            .slides()
            .iter()
            .map(|slide| slide.elements.len())
            .sum::<usize>(),
        500
    );
}


#[test]
fn deleting_a_slide_rewrites_only_the_active_slide_relationship_sequence() {
    let mut pptx = PptxPresentation::blank();
    pptx
        .presentation_mut()
        .slide_mut(0)
        .unwrap()
        .add_text_box("first");
    pptx.presentation_mut().add_slide();
    pptx
        .presentation_mut()
        .slide_mut(1)
        .unwrap()
        .add_text_box("middle");
    pptx.presentation_mut().add_slide();
    pptx
        .presentation_mut()
        .slide_mut(2)
        .unwrap()
        .add_text_box("last");

    let cursor = save_pptx(&mut pptx, Cursor::new(Vec::new())).unwrap();
    let mut reopened = open_pptx(Cursor::new(cursor.into_inner())).unwrap();
    reopened.presentation_mut().delete_slide(1).unwrap();

    let cursor = save_pptx(&mut reopened, Cursor::new(Vec::new())).unwrap();
    let reopened = open_pptx(Cursor::new(cursor.into_inner())).unwrap();

    assert_eq!(reopened.presentation().slides().len(), 2);
    assert!(
        reopened
            .presentation()
            .slide(0)
            .unwrap()
            .elements
            .iter()
            .any(|element| element.text().contains("first"))
    );
    assert!(
        reopened
            .presentation()
            .slide(1)
            .unwrap()
            .elements
            .iter()
            .any(|element| element.text().contains("last"))
    );
}
