# Phase 3 Status

**Status: Complete — Nexa Docs MVP engineering baseline**

Phase 3 establishes the first native DOCX semantic/editing workflow on top of the Phase 2 OOXML / OPC package foundation.

## Completed

- dedicated UI-independent `nexa-docs` crate;
- semantic document model for paragraphs, runs, run formatting, tables, sections, headers/footers, numbering, styles and inline-image relationships;
- caret/selection editing commands;
- insert/delete/split paragraph operations;
- bold, italic and underline editing;
- bounded undo/redo history with a 4 MiB default history budget;
- case-sensitive and case-insensitive search plus replace-all;
- section-aware pagination with long-paragraph page splitting;
- WordprocessingML read/write path for the Phase 3 supported subset;
- DOCX open → edit → save → reopen semantic regression tests;
- compatibility reporting that blocks saves when unsupported constructs would be lost;
- package preservation through the Phase 2 OPC boundary;
- raw-copy preservation for unchanged compressed Parts during selective ZIP rewrite;
- failure-safe atomic DOCX saves;
- native Slint Docs workspace;
- New/Open/Save/Save As application session;
- paragraph navigation and paragraph insertion;
- formatting, search/replace and undo/redo controls in the Docs workspace;
- command-line `.docx` open handoff;
- pinned external DOCX compatibility corpus;
- exact 20-page performance fixture;
- full application memory smoke with the 20-page DOCX open;
- Windows/macOS/Linux strict fmt/check/Clippy/tests;
- Docs, OOXML and native performance regression gates.

## Acceptance evidence

| Gate | Status |
| --- | --- |
| Common pinned real-world DOCX samples open and paginate without crashing | ✅ |
| Generated supported DOCX edits save and reopen semantically | ✅ |
| Unsupported destructive constructs block save | ✅ |
| Unchanged opaque package Parts remain preserved by package layer | ✅ |
| Search / replace / undo / redo regression tests | ✅ |
| Long paragraph and explicit page-break pagination tests | ✅ |
| Exact 20-page fixture enforced in CI | ✅ |
| 20-page Docs semantic performance smoke | ✅ |
| Full Nexa Office + 20-page DOCX memory smoke | ✅ |
| Windows fmt/check/Clippy/tests | ✅ |
| macOS fmt/check/Clippy/tests | ✅ |
| Linux fmt/check/Clippy/tests | ✅ |
| External DOCX corpus smoke | ✅ |
| Native shell performance smoke | ✅ |
| OOXML quality/performance gates | ✅ |

## Compatibility boundary

Phase 3 is a safe MVP, not a claim of complete Microsoft Word compatibility.

The pinned external corpus currently opens and paginates successfully, but its three samples contain WordprocessingML constructs that the Phase 3 writer does not promise to reproduce. They therefore correctly report `can_save=false` instead of allowing a destructive rewrite.

Current corpus snapshot:

| Fixture | Paragraphs | Pages | Compatibility issues | Writable |
| --- | ---: | ---: | ---: | --- |
| `test.docx` | 2 | 1 | 2 | no |
| `having-images.docx` | 5 | 1 | 4 | no |
| `blk-inner-content.docx` | 6 | 1 | 2 | no |

Supported generated fixtures remain writable and are covered by save/reopen semantic tests.

## Deliberately deferred

The following are not represented as completed Word-fidelity features:

- final WYSIWYG paginated document canvas;
- production font shaping/fallback and exact Word line breaking;
- advanced floating objects/anchors and broad DrawingML editing;
- comments, tracked changes, fields, equations and other advanced WordprocessingML;
- dedicated header/footer editing UI;
- image insertion/manipulation UI;
- PDF export and printing.

The roadmap described PDF/print as "when feasible"; it is deferred until the document renderer/text stack and native product-integration phases can support it without weakening correctness or memory budgets.

These items belong primarily to Phase 6 fidelity hardening and Phase 7 native product integration.

## Phase 4 boundary

Phase 4 may build Nexa Sheets on the accepted Phase 2 package layer and must preserve the performance rules established by Phase 1-3.

Docs compatibility improvements may continue in parallel, but Phase 4 must not weaken the DOCX corruption safeguards or native-shell performance gates.
