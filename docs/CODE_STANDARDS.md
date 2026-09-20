# Code Standards

## Language

Rust is the default language.

The workspace should use a pinned stable toolchain and a single Rust edition. The exact MSRV/toolchain is recorded in repository configuration once code is initialized.

Platform FFI or native libraries are permitted only behind narrow abstractions.

## Formatting and linting

Required:

- rustfmt;
- Clippy with repository-agreed warnings;
- no blanket suppression of important lints.

Prefer local, documented allow attributes when a lint is intentionally inapplicable.

Warnings in CI should be treated as failures for project-owned Rust code once the workspace is established.

## Error handling

Use Result for recoverable failures.

Rules:

- no unwrap/expect on untrusted or normal fallible paths;
- retain source/context when crossing layers;
- domain errors should be typed where callers need to branch;
- user-facing messages are created near the UI/application boundary;
- parser errors should identify part/path/context without leaking sensitive contents.

Panics are reserved for programmer invariants that cannot be triggered by malformed documents or normal user action.

## API design

Prefer domain types:

- Twips, Emu, Points, RowIndex, ColumnIndex, PartName, RelationshipId

instead of interchangeable integers/strings.

Avoid ambiguous boolean parameters.

Prefer builders/options structs once a function has multiple independent options.

Public APIs should expose ownership/lifetime intentionally rather than cloning for convenience.

## Ownership and allocation

Memory is a product constraint.

Prefer:

- borrowing;
- slices;
- Arc only when shared ownership is real;
- compact enums/IDs;
- sparse structures for sparse sheets;
- arenas only when lifetime and reclamation are clear;
- bounded caches.

Avoid:

- cloning complete document subtrees;
- String where an interned ID or borrowed str is sufficient;
- HashMap per tiny element when a compact representation works;
- per-cell widget/state objects for invisible spreadsheet regions.

Optimization still requires measurement; do not introduce unreadable micro-optimizations without evidence.

## Collections

Choose collection types based on workload.

Deterministic serialization/testing may prefer ordered structures.

Hash-based structures should not accidentally make saved output nondeterministic when deterministic ordering is feasible.

## Concurrency

Use concurrency for measurable work, not architecture fashion.

Rules:

- bounded worker count;
- cancellation;
- no detached tasks that own critical document state indefinitely;
- no blocking heavy I/O on the UI thread;
- document mutation uses a clear serialized command/state path.

If an async runtime is adopted, it requires an ADR because it affects binary size, task model and dependencies.

## Logging

Logs are diagnostic, not a data dump.

Never log:

- document body text by default;
- credentials/tokens;
- full personal file paths in telemetry;
- embedded document metadata unnecessarily.

Local debug logs may include sanitized identifiers and timing.

## File and module structure

Each crate/module should have one clear responsibility.

Avoid catch-all files named utils.rs containing unrelated functionality.

Prefer feature/domain modules over technical buckets when that improves ownership.

Cyclic crate dependencies are forbidden.

## Documentation

Document:

- invariants;
- non-obvious algorithms;
- unsafe contracts;
- file-format quirks;
- performance-sensitive assumptions;
- public APIs whose behavior is not obvious.

Comments explain why, not line-by-line syntax.

## Dependencies

Before adding a crate, evaluate:

- necessity;
- maintenance;
- license;
- transitive tree;
- feature flags/default features;
- unsafe use;
- binary size;
- runtime cost.

Use default-features = false when default features drag in functionality the project does not need and disabling them is supported.

Duplicate major versions of heavy dependencies should be avoided.

## Serialization and XML

Do not depend on XML field order unless the schema requires it.

Namespaces must be handled semantically.

Unknown attributes/elements should be preserved when the round-trip model promises preservation.

Avoid building full DOM trees for giant parts unless a domain requirement justifies it.

## Numeric and unit handling

Office formats mix units and coordinate systems.

Centralize conversions.

Use checked/saturating arithmetic deliberately; do not rely on release-mode overflow behavior.

Floating-point comparisons in layout tests use explicit tolerances.

## Unsafe and FFI

FFI boundaries:

- validate pointers/lengths;
- convert ownership exactly once;
- isolate platform-specific code;
- document thread requirements;
- return safe Rust abstractions.

Every unsafe block needs a SAFETY explanation.

## UI standards

UI code should:

- use shared tokens for spacing/typography;
- support keyboard operation;
- avoid hard-coded platform-specific assumptions;
- respect high-DPI;
- support accessibility semantics;
- avoid continuous animation timers when idle.

Do not put OOXML parsing or document business logic in UI components.

## Commit style

Use concise Conventional-Commit-like prefixes where useful:

- feat:
- fix:
- perf:
- refactor:
- test:
- docs:
- build:
- ci:

Commit subjects describe intent, not file names.

## Review checklist

Reviewers should ask:

- Can malformed input trigger this path?
- Can this lose document content?
- Does this allocate based on untrusted size?
- Does this scale with document size or visible size?
- Is a cache bounded?
- Is this on the UI thread?
- Is there a test proving the contract?
- Is the new dependency/abstraction necessary?
- Did performance change?
