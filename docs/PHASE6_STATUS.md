# Phase 6 — Fidelity and Interoperability Hardening

Status: **Completed**.

Phase 6 hardens the three native Office engines against silent fidelity loss. The goal is not to claim support for every Office feature; the goal is to make unsupported content explicit, preserve opaque package data where safe, and block semantic rewrites when Nexa cannot reproduce a construct faithfully.

## Completed scope

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

## Exit criteria

Phase 6 exits with a conservative interoperability contract:

- unsupported high-risk semantic rewrites are blocked rather than silently losing content;
- opaque OOXML Parts remain preserved when Nexa does not own their rewrite;
- DOCX, XLSX and PPTX share the same structured interoperability audit;
- CJK, emoji and RTL save/reopen regressions are covered for all three editor families;
- the dedicated Interoperability Gate is green together with all existing quality, corpus and performance gates;
- known unsupported areas are explicitly categorized instead of being treated as supported.

The following capabilities remain valid post-Phase-6 expansion work, not blockers for this phase exit: native hyperlink editing, semantic comments/notes, native chart semantics, richer bidi shaping, advanced object anchoring and export-fidelity scoring. Until those are implemented, the compatibility policy continues to preserve or block safely.
