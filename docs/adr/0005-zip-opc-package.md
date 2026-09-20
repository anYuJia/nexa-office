# ADR 0005 — ZIP / OPC package layer

Status: Proposed

## Context

DOCX, XLSX and PPTX are ZIP-based OPC packages. The package layer sits on every open/save path and is therefore both a security boundary and a memory/performance boundary.

## Decision

Build a Nexa-owned OPC abstraction over a narrowly selected Rust ZIP implementation.

The ZIP crate itself is not selected in Phase 1. Phase 2 must compare candidates against the package requirements before acceptance.

The OPC layer owns:

- part names;
- content types;
- relationships;
- package entry metadata;
- lazy part access;
- bounded decompression;
- atomic package writing;
- preservation of unknown parts.

## Requirements

- reject path traversal;
- defend against decompression bombs;
- avoid expanding every package member eagerly;
- support streaming reads/writes where useful;
- preserve unrelated unknown entries;
- deterministic enough output for stable tests;
- useful corruption/error reporting;
- compatible licensing and manageable unsafe surface.

## Save contract

Saving must write to a temporary destination and only replace the prior valid file after successful completion where the platform permits atomic replacement.

## Revisit conditions

Accept a ZIP implementation after Phase 2 corpus, fuzz and performance results.
