# Phase 6 — Fidelity and Interoperability Hardening

Status: **In progress — shared fidelity gate established**.

Phase 6 hardens the three native Office engines against silent fidelity loss. The goal is not to claim support for every Office feature; the goal is to make unsupported content explicit, preserve opaque package data where safe, and block semantic rewrites when Nexa cannot reproduce a construct faithfully.

## Implemented in the Phase 6 baseline

- shared OOXML interoperability taxonomy;
- structured `InteropReport` available to DOCX, XLSX and PPTX engines;
- rewrite-risk scanning across package and Part relationships;
- high-risk detection for:
  - hyperlinks;
  - comments and threaded comments;
  - notes;
  - charts and drawings;
  - pivot data;
  - external workbook data;
  - embedded/OLE packages;
  - ActiveX;
  - VBA/macros;
  - SmartArt;
  - custom XML;
  - audio/video/media;
  - unknown external relationships;
- semantic-rewrite blockers are surfaced through the existing application compatibility counters;
- CJK + emoji + RTL save/reopen regression tests for DOCX, XLSX and PPTX;
- dedicated Interoperability Gate workflow.

## Policy

### Preserve opaque

An unsupported Part may remain in the OPC package when Nexa does not rewrite the semantic owner that references it. The shared package layer keeps those bytes intact.

### Block rewrite

Nexa blocks saving when a supported editor would rewrite an owning semantic Part and the unsupported relationship or object cannot be reproduced safely.

Examples include worksheet hyperlinks/drawings, document hyperlinks/charts, slide charts, embedded objects, ActiveX and macros.

This is intentionally conservative. A false-positive save block is preferable to silent data loss.

## Unicode fidelity

All three editor families now test a mixed string containing:

- Simplified Chinese;
- emoji outside the BMP;
- Arabic RTL text;
- Hebrew RTL text.

The test boundary is save → reopen, not only in-memory storage.

## Remaining Phase 6 work

- broaden real-world public corpus coverage;
- native hyperlink editing instead of save blocking;
- comments / notes semantic models;
- chart preservation and eventually native chart semantics;
- richer font fallback and explicit bidi layout validation;
- object anchoring fidelity;
- export / print fidelity scoring.

Phase 6 remains open until these areas have representative corpus coverage and no known high-severity silent-loss path remains in supported workflows.
