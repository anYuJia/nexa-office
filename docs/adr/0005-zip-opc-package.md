# ADR 0005 — ZIP / OPC package layer

Status: Accepted

## Context

DOCX, XLSX and PPTX are ZIP-based OPC packages. The package layer sits on every open/save path and is therefore both a security boundary and a memory/performance boundary.

## Decision

Use **zip 8.6.x** behind a Nexa-owned OPC abstraction.

ZIP default features are disabled. Nexa enables only `deflate-flate2-zlib-rs`, supporting Stored and Deflate OOXML packages without pulling AES, Bzip2, LZMA, Zstd, Deflate64, Zopfli or unrelated format features into the core package path.

Document engines depend on Nexa OPC types, not directly on the ZIP crate.

## Package architecture

The layer provides:

- canonical `PartName` validation;
- Content Types and Relationships;
- package and part relationship graphs;
- lazy central-directory indexing;
- on-demand part decompression;
- bounded decompression;
- Office family/main-part detection;
- owned package graph for deliberate rewrites;
- validated repack;
- unknown opaque Part preservation;
- deterministic metadata writers;
- package comparison/diff support;
- failure-safe save-to-temporary followed by replacement.

## Security requirements implemented

- reject package-root traversal and path aliases;
- reject overlapping file data;
- reject encrypted entries;
- reject symbolic-link entries;
- reject unsupported compression methods;
- enforce entry-count limits;
- enforce per-Part and aggregate declared uncompressed limits;
- enforce compression-ratio limits;
- re-check actual bytes during selected-Part decompression;
- use checked/saturating arithmetic at resource boundaries.

## Save contract

Nexa serializes a complete package to a sibling temporary file and syncs it before replacing the destination.

On Unix-like platforms, replacement uses same-directory rename semantics. On Windows, the implementation uses a backup/restore fallback so a failed replacement does not intentionally discard the previous valid file.

## Evidence

Phase 2 includes:

- generated minimal DOCX, XLSX and PPTX fixtures;
- package-level family detection for all three;
- unknown binary Part preservation through validated repack;
- owned package read/write/read tests;
- malformed package and deterministic XML mutation tests;
- cargo-fuzz targets for XML and ZIP/package boundaries;
- Windows, macOS and Linux quality gates;
- release-mode OOXML performance smoke.

Recorded shared-runner sample:

- 132 ZIP entries;
- ~8.4 MiB package;
- central-directory indexing ~272 µs;
- Office family identification ~46 µs;
- selected main-Part read ~4 µs;
- validated full repack ~243 ms;
- complete benchmark peak RSS ~18.8 MiB.

These measurements are CI trend evidence, not reference-hardware product claims.

## Consequences

Benefits:

- no browser or managed runtime;
- only ZIP features Nexa needs;
- package indexing does not require eager full decompression;
- one package layer for DOCX/XLSX/PPTX;
- preservation path for unsupported opaque Parts.

Costs:

- unsupported ZIP compression/encryption is rejected rather than silently converted;
- an owned package rewrite deliberately materializes loaded Parts;
- preserving unknown XML *inside a semantically edited XML Part* remains the relevant semantic engine's responsibility.

## Revisit conditions

Reopen this ADR if interoperability requires another compression method, the ZIP crate changes its security/performance profile, or measurements show that this adapter cannot meet Office open/save budgets.
