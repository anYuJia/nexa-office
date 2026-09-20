# Project Charter

## Mission

Nexa Office is a native, lightweight office suite for desktop platforms.

Its purpose is to provide modern document, spreadsheet and presentation editing without embedding a browser engine and without accepting high memory usage as the normal cost of office software.

## Product priorities

When priorities conflict, use this order:

1. **Data integrity and correctness**
2. **Interoperability and round-trip safety**
3. **Memory and responsiveness**
4. **Usability and accessibility**
5. **Feature breadth**
6. **Visual polish**

A fast editor that damages a file is unacceptable. A feature-rich editor that requires a browser-sized runtime also misses the project goal.

## Non-negotiable constraints

- Native desktop runtime.
- Rust-first implementation.
- No Chromium/Electron/CEF.
- No Tauri/WebView editor architecture.
- No required JavaScript runtime for core editing.
- No mandatory account, cloud service or telemetry.
- No persistent background process after the app exits.
- Offline open/edit/save MUST remain a complete workflow.
- DOCX, XLSX and PPTX are first-class formats, not import-only demonstrations.
- Performance budgets are merge/release gates.

## Initial product surfaces

### Nexa Docs

Primary formats:

- DOCX
- ODT is a later interoperability target.
- Plain text/Markdown may be supported as secondary formats.

Core capabilities include rich text, paragraphs, styles, lists, tables, images, headers/footers, pagination, search/replace, print and PDF export.

### Nexa Sheets

Primary format:

- XLSX

Core capabilities include large-sheet viewport virtualization, formulas, formatting, merged cells, filters, sorting, freeze panes, charts over time, import/export fidelity and efficient recalculation.

### Nexa Slides

Primary format:

- PPTX

Core capabilities include slides, master/theme support, text, images, shapes, tables, ordering, transitions later, presenter/print/export workflows and round-trip fidelity.

## Explicit non-goals for early releases

The first product milestones do not require:

- browser/web edition;
- real-time collaboration;
- cloud storage;
- VBA execution;
- full Microsoft Office macro compatibility;
- every legacy binary Office format;
- every OOXML extension;
- an extension marketplace;
- AI features in the core editor.

These may be considered later only if they do not compromise the core architecture.

## Platform scope

The architecture MUST support:

- Windows
- macOS
- Linux

Platform support is accepted per release only after native input, font, DPI, window, clipboard, file-dialog, print and rendering behavior have been validated on that platform.

## Design principles

### Native by default

Use native or compiled Rust components. Do not emulate a browser application inside a desktop shell.

### Load only what the user needs

Opening a document MUST NOT imply eagerly constructing every render object in the file.

Examples:

- Docs should retain a compact document model and prioritize layout near the visible region.
- Sheets must virtualize cells and rendering.
- Slides should load/render current and near-current slides with bounded caches.

### Preserve before interpreting

Unknown OOXML content should be preserved for round-trip when possible instead of discarded because Nexa does not yet understand it.

### One application, modular engines

Docs, Sheets and Slides may share an application shell and OOXML infrastructure, but their document models and layout engines should remain domain-focused.

### No hidden idle work

With no active operation, Nexa Office should settle to near-zero CPU usage. Timers, animations, watchers and background indexing must be justified and bounded.

## Definition of success

Nexa Office becomes a credible WPS/Office alternative when it can:

- open common real-world DOCX/XLSX/PPTX files without destructive conversion;
- edit and save them with predictable round-trip behavior;
- remain responsive on large files through incremental/virtualized processing;
- stay materially lighter than browser-engine office applications;
- provide a modern, coherent native desktop experience;
- pass the quality and performance gates defined in this repository.

## Governance principle

Dates are planning tools. Gates are acceptance criteria.

No milestone is considered complete because its calendar window ended. It is complete only when its exit criteria pass.
