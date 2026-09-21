use nexa_sheets::{
    CellAddress, CellAlignment, CellFormat, CellRange, FreezePane, XlsxWorkbook, open_xlsx,
    save_xlsx,
};
use std::io::Cursor;

#[test]
fn values_formula_and_unicode_fixture_round_trip() {
    let mut xlsx = XlsxWorkbook::blank();
    let workbook = xlsx.workbook_mut();
    workbook.set_cell_input(0, CellAddress::parse_a1("A1").unwrap(), "中文").unwrap();
    workbook.set_cell_input(0, CellAddress::parse_a1("B1").unwrap(), "12.5").unwrap();
    workbook.set_cell_input(0, CellAddress::parse_a1("C1").unwrap(), "=B1*2").unwrap();

    let bytes = save_xlsx(&mut xlsx, Cursor::new(Vec::new()))
        .unwrap()
        .into_inner();
    let reopened = open_xlsx(Cursor::new(bytes)).unwrap();
    let sheet = reopened.workbook().sheet(0).unwrap();

    assert_eq!(
        sheet
            .cell(CellAddress::parse_a1("A1").unwrap())
            .unwrap()
            .display_text(),
        "中文"
    );
    assert_eq!(
        sheet
            .cell(CellAddress::parse_a1("C1").unwrap())
            .unwrap()
            .cached_number,
        Some(25.0)
    );
}

#[test]
fn layout_and_format_fixture_round_trip() {
    let mut xlsx = XlsxWorkbook::blank();
    let sheet = xlsx.workbook_mut().sheet_mut(0).unwrap();
    sheet.set_input(CellAddress::parse_a1("A1").unwrap(), "Header");
    sheet.set_format(
        CellAddress::parse_a1("A1").unwrap(),
        CellFormat {
            bold: true,
            italic: true,
            fill_rgb: Some("DDEBFF".into()),
            alignment: CellAlignment::Center,
        },
    );
    sheet.merge(CellRange::parse_a1("A2:C2").unwrap()).unwrap();
    sheet.set_freeze_pane(Some(FreezePane {
        rows: 1,
        columns: 1,
    }));
    sheet.set_column_width(0, Some(22.0)).unwrap();
    sheet.set_row_height(0, Some(24.0)).unwrap();

    let bytes = save_xlsx(&mut xlsx, Cursor::new(Vec::new()))
        .unwrap()
        .into_inner();
    let reopened = open_xlsx(Cursor::new(bytes)).unwrap();
    assert!(reopened.can_save());

    let sheet = reopened.workbook().sheet(0).unwrap();
    let format = &sheet
        .cell(CellAddress::parse_a1("A1").unwrap())
        .unwrap()
        .format;
    assert!(format.bold);
    assert!(format.italic);
    assert_eq!(format.fill_rgb.as_deref(), Some("DDEBFF"));
    assert_eq!(format.alignment, CellAlignment::Center);
    assert_eq!(sheet.merged_ranges(), &[CellRange::parse_a1("A2:C2").unwrap()]);
    assert_eq!(
        sheet.freeze_pane(),
        Some(FreezePane {
            rows: 1,
            columns: 1
        })
    );
}

#[test]
fn million_row_sparse_fixture_stays_sparse_after_round_trip() {
    let mut xlsx = XlsxWorkbook::blank();
    let sheet = xlsx.workbook_mut().sheet_mut(0).unwrap();
    sheet.set_input(CellAddress::parse_a1("A1").unwrap(), "start");
    sheet.set_input(CellAddress::parse_a1("A1048576").unwrap(), "end");

    let bytes = save_xlsx(&mut xlsx, Cursor::new(Vec::new()))
        .unwrap()
        .into_inner();
    assert!(bytes.len() < 64 * 1024);

    let reopened = open_xlsx(Cursor::new(bytes)).unwrap();
    let sheet = reopened.workbook().sheet(0).unwrap();
    assert_eq!(sheet.cells().len(), 2);
    assert_eq!(
        sheet
            .cell(CellAddress::parse_a1("A1048576").unwrap())
            .unwrap()
            .display_text(),
        "end"
    );
}
