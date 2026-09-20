# ADR 0004 — OOXML XML parser

Status: Proposed

## Context

OOXML packages contain many XML parts, including very large worksheet and shared-string documents. Building a full DOM for every part conflicts with Nexa's memory goals.

Input is untrusted, namespaces matter, and round-trip preservation may require retaining unknown content.

## Decision

Use a streaming/event-oriented XML architecture for large OOXML parts.

quick-xml is the leading Rust candidate, but the dependency is not added until Phase 2 validates:

- namespace handling;
- bounded allocation;
- entity/security behavior;
- error quality;
- unknown-element preservation strategy;
- large worksheet throughput;
- serialization control.

Small metadata parts may be represented as compact owned structures after parsing. Large parts should not become generic DOM trees by default.

## Security requirements

- no external entity/network resolution;
- explicit size/depth limits where needed;
- checked numeric conversions;
- malformed input must return structured errors;
- parser state must not panic on file-controlled data.

## Revisit conditions

Accept after Phase 2 parser benchmarks and malformed-corpus tests.
