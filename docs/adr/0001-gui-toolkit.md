# ADR 0001 — Native application GUI toolkit

Status: Accepted

## Context

Nexa Office requires a cross-platform native shell on Windows, macOS and Linux while preserving the project's primary constraint: no browser engine, WebView or JavaScript runtime in the editor.

The shell must support accessibility, keyboard input, high-DPI behavior and a path toward a custom document surface without forcing the document engine into the UI toolkit.

## Decision

Use **Slint 1.18.x** for the Phase 1 native application shell.

The initial production feature set is intentionally narrow:

- std
- winit backend
- FemtoVG renderer
- accessibility
- current compatibility feature

Default Slint features are disabled so Nexa does not automatically include additional renderers or system-tray functionality.

The document surface is not permanently bound to FemtoVG. Docs/Sheets/Slides rendering will be evaluated independently and may use Vello/wgpu or another native renderer through a documented boundary.

## Alternatives

### GPUI

Attractive Rust-native architecture and strong performance precedent. Rejected for the initial shell because Slint currently provides a clearer cross-platform product/UI toolkit boundary and accessibility path for this project.

### iced

Mature Rust-native option. Not selected because Nexa benefits from Slint's declarative UI model and explicit multi-renderer backend architecture.

### Qt

Very capable and mature, but introduces a significantly larger C++ runtime/deployment surface than desired for the initial low-memory target.

### Tauri / Electron / WebView

Rejected by product charter. They violate the no-browser-runtime constraint.

## Consequences

Benefits:

- native compiled UI;
- no Chromium/WebView runtime;
- cross-platform shell from one Rust workspace;
- UI remains separable from document engines;
- renderer can be revisited independently.

Costs/risks:

- Slint becomes a foundational dependency and must be benchmarked;
- platform-specific behavior still needs native validation;
- FemtoVG/OpenGL memory and driver behavior must be measured;
- renderer changes require an ADR and before/after benchmark.

## Evidence

At adoption time, Slint 1.18.0 supports the winit backend across Windows, macOS and Linux, offers explicit renderer selection, and permits disabling default features.

Phase 1 CI and reference-machine measurements are the acceptance evidence; this ADR does not assume the runtime meets the memory budget before it is measured.

## Revisit conditions

Reopen this decision if:

- the settled start-screen hard budget cannot be met after reasonable feature pruning;
- accessibility or native text-input requirements cannot be satisfied;
- platform stability is insufficient;
- another toolkit demonstrates materially better measured memory/startup with equivalent functionality.
