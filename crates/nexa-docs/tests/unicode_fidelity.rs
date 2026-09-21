use nexa_docs::{DocxDocument, Run, open_docx, save_docx};
use std::io::Cursor;

#[test]
fn cjk_emoji_and_rtl_text_round_trip_without_loss() {
    let sample = "中文办公 🚀 مرحبا שלום";
    let mut docx = DocxDocument::blank();
    docx.document_mut().paragraph_mut(0).unwrap().runs = vec![Run::text(sample)];

    let bytes = save_docx(&mut docx, Cursor::new(Vec::new()))
        .unwrap()
        .into_inner();
    let reopened = open_docx(Cursor::new(bytes)).unwrap();

    assert_eq!(reopened.document().plain_text(), sample);
    assert!(reopened.can_save());
}
