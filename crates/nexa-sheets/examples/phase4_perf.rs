use nexa_sheets::{
    CellAddress, CellFormat, CellRange, FreezePane, XlsxWorkbook, open_xlsx, save_xlsx,
};
use std::{io::Cursor, time::Instant};

const POPULATED_ROWS: u32 = 8_000;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let build_start = Instant::now();
    let mut xlsx = XlsxWorkbook::blank();

    {
        let workbook = xlsx.workbook_mut();
        let sheet = workbook.sheet_mut(0).expect("blank workbook has Sheet1");
        sheet.set_input(CellAddress::parse_a1("A1")?, "Item");
        sheet.set_input(CellAddress::parse_a1("B1")?, "Qty");
        sheet.set_input(CellAddress::parse_a1("C1")?, "Price");
        sheet.set_input(CellAddress::parse_a1("D1")?, "Total");
        sheet.set_format(
            CellAddress::parse_a1("A1")?,
            CellFormat {
                bold: true,
                fill_rgb: Some("EAF2FF".into()),
                ..CellFormat::default()
            },
        );
        sheet.set_freeze_pane(Some(FreezePane {
            rows: 1,
            columns: 1,
        }));

        for index in 0..POPULATED_ROWS {
            let row = index + 2;
            sheet.set_input(
                CellAddress::new(row - 1, 0)?,
                &format!("SKU-{index:05}"),
            );
            sheet.set_input(
                CellAddress::new(row - 1, 1)?,
                &(index % 17 + 1).to_string(),
            );
            sheet.set_input(
                CellAddress::new(row - 1, 2)?,
                &format!("{:.2}", 1.25 + f64::from(index % 31)),
            );
            sheet.set_input(
                CellAddress::new(row - 1, 3)?,
                &format!("=B{row}*C{row}"),
            );
        }

        sheet.set_input(
            CellAddress::parse_a1("XFD1048576")?,
            "sparse-tail",
        );
        sheet.set_auto_filter(Some(CellRange::parse_a1(&format!(
            "A1:D{}",
            POPULATED_ROWS + 1
        ))?));
        workbook.recalculate();
    }

    let build_us = build_start.elapsed().as_micros();
    let model_bytes = xlsx.workbook().estimated_bytes();
    let populated_cells = xlsx.workbook().sheet(0).unwrap().cells().len();

    let viewport_start = Instant::now();
    let viewport = xlsx
        .workbook()
        .sheet(0)
        .unwrap()
        .viewport(900_000, 24, 0, 10);
    let viewport_us = viewport_start.elapsed().as_micros();

    let recalc_start = Instant::now();
    xlsx.workbook_mut().recalculate();
    let recalc_us = recalc_start.elapsed().as_micros();

    let save_start = Instant::now();
    let bytes = save_xlsx(&mut xlsx, Cursor::new(Vec::new()))?.into_inner();
    let save_us = save_start.elapsed().as_micros();

    let reopen_start = Instant::now();
    let reopened = open_xlsx(Cursor::new(bytes.clone()))?;
    let reopen_us = reopen_start.elapsed().as_micros();

    let tail = reopened
        .workbook()
        .sheet(0)
        .unwrap()
        .cell(CellAddress::parse_a1("XFD1048576")?)
        .map(nexa_sheets::Cell::display_text);
    if tail.as_deref() != Some("sparse-tail") {
        return Err("sparse tail cell did not survive XLSX round-trip".into());
    }

    let first_total = reopened
        .workbook()
        .sheet(0)
        .unwrap()
        .cell(CellAddress::parse_a1("D2")?)
        .and_then(|cell| cell.cached_number);
    if first_total != Some(1.25) {
        return Err(format!("formula result did not survive: {first_total:?}").into());
    }

    println!("theoretical_rows={}", nexa_sheets::MAX_ROWS);
    println!("theoretical_columns={}", nexa_sheets::MAX_COLUMNS);
    println!("populated_rows={POPULATED_ROWS}");
    println!("populated_cells={populated_cells}");
    println!("viewport_cells={}", viewport.len());
    println!("estimated_model_bytes={model_bytes}");
    println!("xlsx_bytes={}", bytes.len());
    println!("build_us={build_us}");
    println!("viewport_us={viewport_us}");
    println!("recalc_us={recalc_us}");
    println!("save_us={save_us}");
    println!("reopen_us={reopen_us}");

    Ok(())
}
