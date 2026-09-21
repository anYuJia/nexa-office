# Phase 4 — Nexa Sheets MVP

Status: **Completed**.

## Implemented

- UI-independent `nexa-sheets` Rust crate.
- Sparse worksheet storage bounded by populated cells, not XLSX theoretical dimensions.
- Full XLSX row/column address bounds: 1,048,576 × 16,384.
- A1 cell/range parsing and serialization.
- Bounded virtual viewport extraction.
- Values: empty, number, text, boolean and error.
- Formula core: arithmetic, references, ranges, SUM, AVERAGE, MIN and MAX.
- Circular-reference detection and deterministic recalculation.
- Basic cell formatting: bold, italic, fill and horizontal alignment.
- Merged cells.
- Row heights and column widths.
- Freeze panes.
- Sparse row sort and equality filter.
- Multi-sheet workbook model.
- XLSX open/save on the shared OOXML/OPC package layer.
- Shared strings on import and inline strings on write.
- Basic SpreadsheetML styles import/export.
- Compatibility blocking for constructs the current writer cannot safely reproduce.
- Atomic XLSX save.
- Native Slint Sheets workspace with a fixed-size virtualized grid.
- Simplified Chinese / English Sheets UI and command feedback.
- Generated fixture corpus and million-row sparse round-trip test.
- Dedicated release performance smoke.

## Deliberate safety boundary

Phase 4 does not silently rewrite spreadsheet constructs it cannot safely reproduce. Complex items such as conditional formatting, data validation, drawings, tables, sheet protection, external workbook relationships and unsupported number formats are surfaced through compatibility issues and block destructive save.

## Acceptance

Final acceptance requires and is enforced by CI:

- Windows, macOS and Linux workspace fmt/check/Clippy/tests;
- existing OOXML, Docs and native performance gates;
- the dedicated Sheets sparse-workbook performance gate;
- generated XLSX edit/save/reopen fixture tests;
- the one-million-row sparse fixture remaining sparse rather than materializing theoretical rows.

The final Phase 4 branch fixes the Sheets session borrow regression and removes Sheets-specific Slint width binding loops by keeping the rendered grid bounded and scrolling through logical columns instead.
