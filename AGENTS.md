# AGENTS.md

This file is the mandatory operating contract for AI coding agents working in Nexa Office.

The words MUST, MUST NOT, SHOULD and MAY are normative. When rules conflict, use this order:

1. Security and data-integrity requirements.
2. This file.
3. docs/ENGINEERING_GATES.md and docs/PERFORMANCE.md.
4. docs/ARCHITECTURE.md.
5. Task-specific requirements.
6. Local implementation preference.

## 1. Product invariants

Nexa Office MUST remain a native, low-memory desktop application.

Agents MUST NOT introduce any of the following into the production editor runtime:

- Chromium, Electron or CEF.
- Tauri or an embedded WebView as the editor shell.
- A JavaScript runtime required for core editing.
- A permanent local HTTP server for application internals.
- Background daemons that survive after the application exits.

Rust is the default implementation language. FFI is allowed only when a native library provides a clear interoperability or platform advantage and the boundary is documented.

DOCX, XLSX and PPTX are first-class formats. A change that silently corrupts, drops or rewrites unsupported document content is a correctness failure even if the visible test document looks acceptable.

## 2. Before changing code

An agent MUST:

- Read this file and the relevant documents under docs/.
- Inspect the existing implementation and tests before editing.
- Identify which architectural layer owns the behavior.
- Reuse existing abstractions before creating another parallel abstraction.
- Keep the change as small as reasonably possible.

An agent MUST NOT perform broad rewrites merely to make code look cleaner.

If a task exposes an architectural gap, prefer a small isolated implementation plus an ADR proposal over silently inventing a new architecture.

## 3. Dependency rules

Adding a production dependency requires all of:

- A concrete capability that is not reasonably provided by std or existing dependencies.
- License compatibility.
- Active maintenance or a documented reason to accept the risk.
- No hidden browser/runtime/server dependency.
- A measured or bounded impact on binary size, startup and memory for foundational dependencies.

Foundational dependencies such as GUI, renderer, shaping, XML, ZIP, font, image and async runtimes require an Architecture Decision Record before adoption or replacement.

Agents MUST NOT add dependencies only to save a few lines of code.

## 4. Correctness rules

Production library code MUST avoid panic-driven control flow.

- Do not use unwrap() or expect() on user-controlled, file-controlled or IPC-controlled data.
- Validate lengths, indexes, dimensions and decompressed sizes before allocation.
- Treat OOXML/XML/ZIP input as untrusted.
- Use checked arithmetic for sizes and offsets where overflow is plausible.
- Saves MUST be atomic where the platform permits it.
- Failed imports or exports MUST return actionable errors, not partially written output.

Unsupported OOXML should be preserved for round-trip whenever technically possible. If it cannot be preserved, the loss MUST be explicit and covered by a test.

## 5. Unsafe Rust

unsafe is exceptional, not normal.

Every unsafe block MUST:

- Be contained behind a safe API.
- Have a SAFETY comment stating the invariant.
- Have tests covering the boundary.
- Be justified by platform FFI or measured performance need.

Large unsafe subsystems require an ADR and review. Agents MUST NOT use unsafe to bypass borrow-checker design problems.

## 6. Performance discipline

Performance is a release requirement.

Agents MUST NOT:

- Materialize an entire large workbook merely to render a viewport.
- Build UI objects for off-screen spreadsheet cells.
- Rasterize pages/slides that are not visible or near-visible without a bounded cache reason.
- Add unbounded caches.
- Add unbounded channels or task queues.
- Spawn one thread per document element, page, sheet, slide or cell.
- change benchmarks or thresholds merely to make a regression pass.

Hot-path allocations, clones and string conversions SHOULD be justified by measurement.

Any change that touches parsing, layout, rendering, scrolling, startup, open/save, font handling or caching MUST run the relevant benchmark before merge once that benchmark exists.

## 7. Testing rules

Agents MUST add or update tests for behavior changes.

Agents MUST NOT:

- Delete a valid failing test to make CI pass.
- Add blanket ignore attributes without a tracked reason.
- weaken assertions without explaining the changed contract.
- replace real parser/serializer tests with mocks.
- commit generated fuzz crashes without reducing them into regression cases when practical.

Parser and serializer bugs require a regression fixture.

Document round-trip changes require semantic and package-level verification, not only screenshot comparison.

## 8. Code-quality rules

All Rust code MUST pass the repository formatting and lint gates.

Public APIs require documentation when their contract is not obvious.

Prefer:

- explicit domain types over primitive soup;
- borrowed data over clones in hot paths;
- Result-based errors with context;
- bounded concurrency;
- deterministic output;
- small crates with clear ownership.

Avoid:

- global mutable state;
- hidden singletons;
- giant cross-domain utility modules;
- bool parameters whose meaning is unclear;
- speculative generic abstractions;
- cyclic crate dependencies.

## 9. UI rules

The UI layer MUST NOT become the document engine.

Business rules, OOXML parsing, document mutation, layout and serialization belong in engine crates.

The UI should consume typed state and commands.

Accessibility, keyboard operation, high-DPI behavior and platform-native text input are requirements, not optional polish.

Visual effects MUST NOT create permanent high CPU/GPU usage when the application is idle.

## 10. AI-specific prohibitions

AI agents MUST NOT:

- fabricate benchmark numbers or claim tests were run when they were not;
- commit secrets, tokens, machine paths or personal data;
- introduce fake data into production paths;
- leave TODO stubs presented as completed functionality;
- duplicate large chunks of code instead of understanding the abstraction;
- mass-format unrelated files;
- change license headers or licensing policy without explicit instruction;
- silently alter performance budgets or project gates.

When a tool or environment prevents verification, report exactly what was not verified.

## 11. Change size and history

Prefer one coherent concern per commit.

A feature PR SHOULD include:

- implementation;
- tests;
- benchmark evidence when performance-sensitive;
- documentation/ADR updates when contracts change.

Generated files and large binary fixtures MUST be minimized. Large test corpora belong in a deliberate fixture strategy rather than casual commits.

## 12. Completion report

At the end of an implementation task, an AI agent SHOULD report:

- what changed;
- relevant files/modules;
- tests and checks actually executed;
- benchmark deltas if applicable;
- known limitations or follow-up items.

Never describe a task as complete if a required gate is known to be failing.
