# Phase 2 Status

Phase 2 establishes the shared OOXML / OPC package foundation used by DOCX, XLSX, and PPTX.

## Batch 1 — dependency-free OPC invariants

Implemented:

- dedicated `nexa-ooxml` crate with no UI dependency;
- canonical OPC `PartName` validation;
- rejection of path traversal, backslashes, empty segments, fragments, and control characters;
- relationship target resolution relative to package root or source part;
- explicit rejection of relationship targets that escape the package root;
- compact content-type defaults/overrides with override precedence;
- package resource-budget model for entry count, per-part size, total decompressed size, and compression ratio;
- unit tests for common Word/Excel/PowerPoint path patterns and hostile inputs.

This first batch deliberately adds no ZIP/XML crate yet. The repository keeps `cargo --locked` intact while the adapters are evaluated separately.

## Next batches

1. streaming XML adapter and parsing limits;
2. ZIP adapter with lazy entry reads and decompression enforcement;
3. `[Content_Types].xml` and `.rels` parsers/writers;
4. OPC package graph and unknown-part preservation;
5. deterministic package rewrite / round-trip fixture harness;
6. DOCX, XLSX, and PPTX package-level smoke fixtures;
7. malformed package corpus and fuzz targets;
8. performance measurements before accepting ADR 0004 / 0005.
