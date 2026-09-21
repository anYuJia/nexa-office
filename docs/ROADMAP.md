# Roadmap

This roadmap uses planning windows, not promises. A phase exits only when its acceptance gates pass.

## Phase 0 — Engineering Constitution

Planning window: 1-2 days.

Deliverables:

- project charter;
- architecture boundaries;
- AI rules;
- code standards;
- testing strategy;
- performance budgets;
- contribution and merge gates.

Exit criteria:

- all documents linked from README exist;
- rules are internally consistent;
- no implementation work is required to interpret core constraints.

## Phase 1 — Native Shell and Measurement Baseline

Planning window: 1-2 weeks.

Scope:

- Rust workspace;
- native application shell;
- window lifecycle;
- menus/commands;
- settings skeleton;
- logging/diagnostics;
- CI;
- benchmark harness;
- memory/startup measurement scripts;
- initial ADRs for GUI/render/text/XML/ZIP stack.

Exit criteria:

- application launches on at least Windows, macOS and Linux CI/build targets where practical;
- idle app performs no continuous redraw;
- startup and idle-memory baselines are recorded;
- formatting, linting and tests are mandatory CI checks;
- no browser/WebView dependency exists.

## Phase 2 — OOXML / OPC Foundation

Status: **Completed**.

Planning window: 2-4 weeks.

Scope:

- ZIP package abstraction;
- content types;
- relationships;
- namespace-aware XML layer;
- part graph;
- bounded decompression;
- unknown-part preservation;
- fixture harness;
- semantic/package comparison tools.

Exit criteria:

- open/rewrite of representative packages does not lose unrelated parts;
- malformed ZIP/XML fixtures fail safely;
- fuzz targets exist for package/XML boundaries;
- package open/save benchmarks fit current budgets;
- at least one DOCX, XLSX and PPTX fixture can round-trip at package level.

## Phase 3 — Nexa Docs MVP

Status: **Completed**.

Planning window: 6-10 weeks.

Scope:

- paragraphs/runs;
- styles;
- lists;
- sections;
- page layout;
- tables;
- images;
- headers/footers;
- selection/caret/edit commands;
- undo/redo;
- search/replace;
- DOCX open/save;
- PDF/print path when feasible.

Exit criteria:

- common real-world DOCX corpus opens without crashes;
- edits save and reopen semantically;
- unknown content is preserved where supported by the package layer;
- typing, scrolling and pagination meet latency gates;
- ordinary documents stay inside memory budgets;
- destructive compatibility issues are documented and blocked from release.

## Phase 4 — Nexa Sheets MVP

Status: **Completed**.

Planning window: 6-10 weeks.

Scope:

- workbook/sheet model;
- sparse cell storage;
- viewport virtualization;
- values/formulas;
- formatting;
- merged cells;
- rows/columns;
- sort/filter;
- freeze panes;
- recalculation core;
- XLSX open/save.

Exit criteria:

- one-million-row worksheets can be opened/navigated without creating one widget/object per row;
- viewport scrolling stays responsive;
- supported formulas recalculate deterministically;
- edit/save/reopen tests pass across the fixture corpus;
- memory scales primarily with populated/active data, not theoretical sheet dimensions.

## Phase 5 — Nexa Slides MVP

Status: **Completed**.

Planning window: 5-8 weeks.

Scope:

- presentation model;
- themes/masters;
- text;
- images;
- basic shapes;
- tables;
- z-order;
- slide thumbnails;
- editing commands;
- PPTX open/save;
- export/print foundation.

Exit criteria:

- representative PPTX corpus opens and renders consistently;
- current/near-current slide caching is bounded;
- save/reopen round-trip tests pass for supported features;
- idle presentation does not continuously redraw.

## Phase 6 — Fidelity and Interoperability Hardening

Status: **In progress**.

Planning window: 8-12 weeks, then ongoing.

Scope:

- real-world compatibility corpus;
- fonts/fallback;
- CJK/bidi/emoji;
- charts;
- advanced layout;
- comments/notes;
- hyperlinks;
- object anchoring;
- edge-case OOXML;
- export fidelity.

Exit criteria:

- no known high-severity data-loss bug in supported workflows;
- compatibility regression suite is stable;
- top interoperability failures are categorized and tracked;
- round-trip score improves without performance-budget regressions.

## Phase 7 — Native Product Integration

Planning window: 4-6 weeks.

Scope:

- installers;
- file associations;
- recent files;
- native dialogs;
- clipboard;
- drag/drop;
- accessibility;
- high-DPI;
- printing;
- crash recovery;
- settings;
- update strategy;
- localization foundation.

Exit criteria:

- platform-native workflows validated on supported OS targets;
- install/uninstall is clean;
- recovery path is tested;
- accessibility baseline is met;
- package-size and idle-resource gates pass.

## Phase 8 — Alpha / Beta / 1.0 Hardening

No fixed duration.

Alpha requires:

- Docs/Sheets/Slides core workflows usable;
- no critical corruption issue in known corpus;
- CI and perf gates enforced;
- crash reporting strategy decided without mandatory telemetry.

Beta requires:

- broad corpus testing;
- installer/update testing;
- long-session leak tests;
- migration/compatibility notes;
- release checklist rehearsed.

1.0 requires:

- stable document contracts;
- published support matrix;
- known limitations documented;
- reproducible release builds;
- security response process;
- sustained performance targets on reference hardware.

## Scheduling rule

Quality gates outrank schedule.

A phase may overlap with the next only when shared foundations are stable and the overlap does not bypass the previous phase's exit criteria.
