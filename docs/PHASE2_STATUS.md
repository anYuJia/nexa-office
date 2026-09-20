# Phase 2 Status

**Status: Complete**

Phase 2 establishes the shared OOXML / OPC package foundation used by DOCX, XLSX and PPTX.

## Completed

- dedicated UI-independent `nexa-ooxml` crate;
- canonical OPC Part names and relationship-Part mapping;
- path traversal, alias and package-root escape rejection;
- Content Types defaults/overrides;
- package and Part Relationships;
- bounded streaming XML metadata parser/writer;
- pinned `quick-xml 0.42.0`, default features disabled;
- DOCTYPE, depth, input-size and attribute-count controls;
- pinned `zip 8.6.0`, default features disabled;
- only `deflate-flate2-zlib-rs` enabled;
- lazy ZIP central-directory indexing;
- Stored/Deflate Part reading on demand;
- entry, per-Part, aggregate size and compression-ratio limits;
- overlapping-entry, encryption, symlink and unsupported-compression rejection;
- owned `Package` / `Part` / `RelationshipSet` graph;
- Office family/main-Part detection;
- DOCX/XLSX/PPTX generated fixture coverage;
- validated repack;
- unknown opaque Part preservation;
- package semantic comparison helper;
- deterministic malformed XML corpus;
- malformed package/resource-limit tests;
- cargo-fuzz targets for XML metadata and ZIP/package boundaries;
- failure-safe package save with temporary file + sync + replacement;
- Windows/macOS/Linux strict CI;
- OOXML release performance smoke.

## Acceptance evidence

| Gate | Status |
| --- | --- |
| Preserve unrelated opaque Parts across validated repack | ✅ |
| Content Types reader/writer round-trip | ✅ |
| Relationships reader/writer round-trip | ✅ |
| Malformed ZIP/XML fail safely | ✅ |
| XML fuzz target exists | ✅ |
| ZIP/package fuzz target exists | ✅ |
| DOCX package-level round-trip | ✅ |
| XLSX package-level round-trip | ✅ |
| PPTX package-level round-trip | ✅ |
| Package comparison tooling | ✅ |
| Failure-safe save regression test | ✅ |
| Windows fmt/check/Clippy/tests | ✅ |
| macOS fmt/check/Clippy/tests | ✅ |
| Linux fmt/check/Clippy/tests | ✅ |
| OOXML performance smoke | ✅ |
| XML ADR accepted | ✅ |
| ZIP/OPC ADR accepted | ✅ |

## Performance snapshot

The recorded 8.4 MiB / 132-entry release fixture measured approximately:

- 272 µs to index the ZIP central directory;
- 46 µs to identify the Office family/main Part;
- 4 µs to read the selected 150-byte main Part;
- 243 ms to perform a validated full repack;
- 18.8 MiB peak RSS for the entire benchmark process.

Shared CI now enforces a coarse 64 MiB peak-RSS regression ceiling for this fixture.

## Boundary for Phase 3

Phase 2 does **not** implement WordprocessingML layout/editing, spreadsheet semantics, or presentation rendering.

Phase 3 may build Nexa Docs semantics and layout on this package layer. It must not bypass the OPC security, preservation or performance boundaries established here.
