# Known Limitations

Nexa Office is a native lightweight Office-compatible editor, not a byte-for-byte reimplementation of Microsoft Office.

## Cross-format limitations

- legacy binary Office formats (`.doc`, `.xls`, `.ppt`) are not supported;
- macros/VBA are not executed;
- ActiveX/OLE and other executable embedded content are not executed;
- complex unsupported semantic rewrites may be blocked to prevent silent data loss;
- byte-identical ZIP/XML output is not guaranteed after save;
- external links and embedded network resources are not automatically fetched.

## Docs

Current Docs support focuses on common paragraphs, runs, formatting, lists, sections, tables, images, headers/footers, search/replace, undo/redo and pagination.

Not all advanced Word layout constructs, fields, tracked changes, comments, floating-object anchoring, equations or desktop-publishing edge cases have native editing semantics.

## Sheets

Current Sheets supports sparse worksheets, common values/formulas, deterministic recalculation for the implemented formula set, formatting, merged cells, sort/filter, freeze panes and multiple sheets.

The complete Excel formula language, pivot tables, Power Query, external data connections, macros, advanced charts and every conditional-formatting rule are not claimed.

## Slides

Current Slides supports multi-slide presentations, text boxes, basic shapes, tables, image relationship preservation, z-order and common slide operations.

Advanced animations, transitions, SmartArt, charts, media playback, notes editing, master/theme parity and complex grouped transforms are not fully implemented.

## Native integration

- native file drag/drop is limited by the stable Slint Rust data-transfer API; plain-text paths and `file://` URIs are accepted when provided;
- Linux native dialogs use Zenity when available;
- printing hands files to the operating-system print path and does not promise Office-identical print layout;
- signing, notarization and publisher trust depend on release credentials and are not embedded in source control.

## Release policy

Any limitation that could cause destructive loss is treated differently from a missing feature: Nexa should preserve the opaque content or block the unsafe save rather than silently discard it.
