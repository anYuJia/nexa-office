# ADR 0004 — OOXML XML parser

Status: Accepted

## Context

OOXML packages contain many XML parts, including potentially very large worksheet and shared-string documents. Building a generic DOM for every part conflicts with Nexa's memory goals and unnecessarily expands the attack surface for untrusted documents.

## Decision

Use **quick-xml 0.42.x** as Nexa's native Rust streaming/event XML layer.

The dependency is pinned with default features disabled. OPC metadata is parsed from events into compact domain structures. Large future document parts follow the same streaming-first rule unless a dedicated owned representation is justified by a semantic engine.

Phase 2 implements bounded readers and writers for:

- `[Content_Types].xml`;
- package-level relationships;
- part-level relationships.

## Security and resource rules

- no external entity or network resolution;
- DOCTYPE is rejected for OPC metadata;
- input byte limits are checked before parsing;
- nesting depth is bounded;
- attributes per element are bounded;
- malformed input returns structured errors;
- relationship targets are resolved through package-root traversal checks;
- XML attribute escaping is handled by the metadata writers;
- duplicate relationship IDs are rejected.

## Evidence

Phase 2 includes:

- reader/writer round-trip tests for Content Types and Relationships;
- deterministic malformed-XML mutation corpus tests;
- a cargo-fuzz target for OPC metadata;
- DOCX, XLSX and PPTX package-level round-trip fixtures;
- Windows, macOS and Linux strict CI;
- release-mode OOXML package performance smoke.

## Consequences

Benefits:

- native Rust parser;
- streaming, low-allocation architecture;
- no generic XML DOM dependency;
- shared XML policy across all Office families;
- explicit resource limits and structured failures.

Costs:

- semantic engines must explicitly model the OOXML they understand;
- preserving unknown XML *inside a semantically edited XML part* remains the semantic engine's responsibility;
- streaming parsers require more deliberate state machines than DOM traversal.

## Revisit conditions

Reopen this ADR if quick-xml introduces an unacceptable security, interoperability or performance regression, or if a required Office feature cannot be implemented safely with the streaming model.
