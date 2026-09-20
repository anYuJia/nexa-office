# Architecture

This document defines the intended architecture boundaries for Nexa Office. Names may evolve, but dependency direction and product constraints should remain stable unless changed by an ADR.

## High-level model

Nexa Office is designed as a single native desktop application with modular Rust crates.

Expected layers:

- application shell;
- UI;
- editor commands/state;
- Docs/Sheets/Slides domain engines;
- layout and rendering;
- text shaping and fonts;
- shared OOXML/OPC infrastructure;
- platform integration;
- diagnostics/benchmark support.

The production editor MUST NOT require a browser engine, local web server or JavaScript runtime.

## Proposed workspace

The initial workspace should converge toward modules similar to:

- nexa-app — executable composition root
- nexa-ui — shell, menus, panels, dialogs, command palette
- nexa-platform — native OS integration
- nexa-core — shared domain primitives and commands
- nexa-ooxml — OPC, ZIP/XML, relationships, content types and shared OOXML types
- nexa-docs — document model and DOCX mapping
- nexa-docs-layout — pagination and document layout
- nexa-sheets — workbook model and XLSX mapping
- nexa-calc — formula/recalculation engine
- nexa-slides — presentation model and PPTX mapping
- nexa-render — renderer-facing scene primitives
- nexa-text — shaping, font resolution and text metrics
- nexa-assets — images and embedded media abstractions
- nexa-test-support — fixtures and semantic comparison helpers
- nexa-bench — reproducible benchmark harnesses

The exact crate split should be introduced only when there is real code to own.

## Dependency direction

Platform/UI layers depend on engine layers, never the inverse.

Domain engines may depend on shared OOXML, text and rendering abstractions.

Shared OOXML code MUST NOT import UI concepts.

Rendering MUST consume layout/scene information rather than mutate the document model directly.

Serialization MUST consume document state through explicit contracts and MUST NOT depend on widget state.

## Document model versus render model

Nexa Office distinguishes:

1. package/file representation;
2. semantic document model;
3. layout model;
4. render scene/cache;
5. UI interaction state.

They MUST NOT collapse into one giant object graph.

This separation allows:

- compact storage of non-visible content;
- partial/incremental layout;
- render-cache eviction;
- deterministic serialization;
- headless tests;
- lower memory usage.

## OOXML architecture

OOXML is ZIP + OPC relationships + XML parts plus application-specific schemas.

The shared layer should own:

- package entries;
- content types;
- part names;
- relationships;
- namespaces;
- XML helpers;
- shared media;
- themes;
- DrawingML primitives that genuinely cross formats.

DOCX/XLSX/PPTX-specific semantics belong in their respective domain crates.

### Round-trip preservation

Nexa should distinguish between:

- understood and editable data;
- understood but currently read-only data;
- unknown data preserved opaquely;
- unsupported data that cannot safely be retained.

Unknown XML/parts SHOULD be retained when possible. Serialization tests must prove that unrelated unknown content is not accidentally dropped.

## Parsing strategy

Input files are untrusted.

The parser should favor:

- streaming XML where practical;
- bounded decompression and allocation;
- lazy part loading;
- clear ownership of raw bytes;
- deterministic parsing;
- limits for pathological dimensions/counts.

Large files MUST NOT require a full duplicate in memory solely for convenience.

## Layout strategy

Layout is incremental.

### Docs

Prefer a compact semantic model plus page/flow layout that can be invalidated from a change point. Do not rebuild all pages after every keystroke.

### Sheets

Viewport virtualization is mandatory. Render and UI objects are created for visible/near-visible cells, not every populated cell.

Workbook data storage and formula dependency structures are separate from on-screen cell widgets.

### Slides

Only active and near-active slides require full-resolution render resources. Thumbnail and scene caches are bounded.

## Rendering

The application shell may use Slint.

The document surface may use an appropriate native renderer such as Vello/wgpu if benchmarks and integration justify it.

The renderer choice is an architectural dependency and requires an ADR before final adoption.

Rendering rules:

- event-driven redraw;
- dirty-region/invalidation awareness where practical;
- bounded GPU/CPU caches;
- no continuous animation loop while idle;
- graceful software/fallback strategy where required.

## Text and fonts

Text shaping, font fallback and layout are core correctness infrastructure.

Potential technologies such as COSMIC Text and HarfRust require validation against:

- mixed scripts;
- bidi;
- CJK;
- emoji;
- ligatures;
- variable fonts;
- font fallback;
- DPI scaling;
- printing/PDF metrics.

Text measurements used for editing and serialization/layout must be deterministic enough to avoid unstable reflow.

## Concurrency

Concurrency must be bounded.

Good candidates:

- document parsing;
- image decoding;
- page/slide pre-render;
- formula recalculation;
- background save preparation.

Rules:

- no thread per element;
- no unbounded task queues;
- cancellation for long-running work;
- document mutations serialized through a clear command/state mechanism;
- UI thread must not perform avoidable heavy I/O or parsing.

## Save model

Save operations should:

1. validate target state;
2. serialize to a temporary destination;
3. flush/close;
4. atomically replace the original when supported.

A failed save MUST NOT destroy the previous valid file.

Autosave/recovery, when introduced, must use a separate recovery artifact and explicit lifecycle.

## Error model

Library layers return structured errors.

User-facing messages are produced near the application layer.

Do not leak raw implementation errors directly to end users, but preserve detailed diagnostic context for logs.

## Observability

Production telemetry is not required.

Local diagnostics SHOULD support:

- startup phases;
- open/save timing;
- parser/layout/render timing;
- cache sizes;
- memory benchmark modes;
- trace export when explicitly enabled.

Diagnostics must be off or low-overhead by default.

## Architecture Decision Records

Create an ADR under docs/adr/ for decisions that are expensive to reverse, including:

- GUI toolkit;
- renderer;
- text shaping stack;
- async runtime;
- OOXML XML parser;
- ZIP implementation;
- database/indexing layer if any;
- FFI/native library;
- plugin model;
- license model.

An ADR should record context, decision, alternatives and consequences.
