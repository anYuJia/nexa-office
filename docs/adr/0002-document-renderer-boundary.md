# ADR 0002 — Document renderer boundary

Status: Proposed

## Context

The application shell needs a renderer today, while the future Docs/Sheets/Slides surfaces need much stronger guarantees around large scenes, partial invalidation, text integration, GPU memory and deterministic layout.

Binding the document engines directly to the GUI toolkit renderer would make later performance work expensive.

## Decision

Keep the document renderer behind a Nexa-owned scene/render boundary.

Phase 1 uses Slint + FemtoVG only for the application shell. This does **not** select FemtoVG as the permanent document renderer.

Vello/wgpu is the leading native Rust candidate for the document surface, but adoption requires a benchmark prototype before this ADR can become Accepted.

The engine must not expose Slint widget types in document/layout crates.

## Evaluation criteria

A renderer candidate must be measured for:

- idle memory and GPU allocation;
- first render and steady frame latency;
- dirty-region/partial redraw behavior;
- large text-heavy pages;
- thousands of spreadsheet primitives;
- slide shapes/images/transforms;
- high-DPI behavior;
- software/fallback behavior;
- macOS Metal, Windows GPU and Linux compatibility;
- integration cost with the chosen text stack.

## Consequences

Phase 1 can build a real native shell without prematurely locking the core editor to one graphics implementation.

There is a deliberate abstraction cost, but it protects the long-term memory/performance goal.

## Revisit conditions

Accept a concrete renderer only after the prototype benchmark and text integration tests exist.
