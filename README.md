# Nexa Office

[简体中文](README.zh-CN.md) · English

> A native, lightweight office suite built with Rust.

Nexa Office is an experimental desktop productivity suite focused on three non-negotiable goals:

1. **Native and lightweight** — no Chromium, Electron, Tauri or WebView-based editor runtime.
2. **Office-grade interoperability** — DOCX, XLSX and PPTX are first-class formats, with round-trip fidelity treated as a correctness requirement.
3. **Performance by design** — startup time, idle CPU, memory, document latency and package size are release gates, not afterthoughts.

## Planned stack

- **Rust** for application and document engine code
- **Slint** for native application UI
- **Vello / wgpu** for document-surface rendering where appropriate
- **COSMIC Text / HarfRust** for text shaping/layout where appropriate
- Streaming ZIP/XML processing for OOXML
- No browser engine or JavaScript runtime in the core product

The exact dependency set is not frozen. New foundational dependencies require an architecture decision and benchmark evidence.

## Status

Nexa Office has completed **Phase 7 — Native Product Integration**. Docs, Sheets and Slides have native Rust semantic engines, shared interoperability safeguards, native Slint workspaces, recent-file persistence, crash recovery, native dialogs/printing handoff, Office file-association metadata and three-platform integration gates. **Phase 8 — Alpha / Beta / 1.0 Hardening** is now the active roadmap stage.

The native Rust/Slint application includes real Docs, Sheets and Slides workspaces. Docs provides safe DOCX editing, formatting, search/replace, undo/redo and pagination. Sheets provides a sparse XLSX-sized workbook model, a bounded virtualized grid, formulas and deterministic recalculation, formatting, merged cells, row/column sizing, sort/filter, freeze panes, multi-sheet editing and XLSX open/save. Slides provides a native multi-slide model, text boxes, basic shapes, tables, image relationship preservation, z-order editing, slide operations and PPTX open/save with compatibility blocking. The shell remains bilingual (System / 简体中文 / English), accessibility-aware and browser-runtime-free. Unsupported destructive rewrites remain blocked instead of silently losing Office content.

## Engineering documents

- [AGENTS.md](AGENTS.md) — mandatory rules for AI coding agents
- [Project Charter](docs/PROJECT_CHARTER.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Roadmap](docs/ROADMAP.md)
- [Engineering Gates](docs/ENGINEERING_GATES.md)
- [Testing Strategy](docs/TESTING.md)
- [Code Standards](docs/CODE_STANDARDS.md)
- [Performance Budgets](docs/PERFORMANCE.md)
- [Phase 1 Status](docs/PHASE1_STATUS.md)
- [Phase 1 Performance Baseline](docs/PERFORMANCE_BASELINE.md)
- [Phase 2 Status](docs/PHASE2_STATUS.md)
- [Phase 2 OOXML Performance](docs/PHASE2_PERFORMANCE.md)
- [Phase 3 Status](docs/PHASE3_STATUS.md)
- [Phase 3 Docs Performance](docs/PHASE3_PERFORMANCE.md)
- [Phase 3 DOCX Corpus](docs/PHASE3_CORPUS.md)
- [Phase 4 Status](docs/PHASE4_STATUS.md)
- [Phase 4 Sheets Performance](docs/PHASE4_PERFORMANCE.md)
- [Phase 5 Status](docs/PHASE5_STATUS.md)
- [Phase 5 Slides Performance](docs/PHASE5_PERFORMANCE.md)
- [Phase 6 Status](docs/PHASE6_STATUS.md)
- [Phase 7 Status](docs/PHASE7_STATUS.md)
- [Update Strategy](docs/UPDATE_STRATEGY.md)
- [Phase 8 Status](docs/PHASE8_STATUS.md)
- [Support Matrix](docs/SUPPORT_MATRIX.md)
- [Known Limitations](docs/KNOWN_LIMITATIONS.md)
- [Migration / Compatibility](docs/MIGRATION_COMPATIBILITY.md)
- [Release Checklist](docs/RELEASE_CHECKLIST.md)
- [Reproducible Builds](docs/REPRODUCIBLE_BUILDS.md)
- [Crash Reporting](docs/CRASH_REPORTING.md)
- [UI / UX and localization](docs/UI_UX_I18N.md)
- [Architecture Decisions](docs/adr/README.md)
- [Contribution Guide](CONTRIBUTING.md)

## Development

```bash
cargo run --locked -p nexa-app
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The application shell is native. Core editor code must not introduce Chromium, Electron, Tauri, CEF, WebView, or a required JavaScript runtime.

## Core principle

> Correctness first. Measured performance second. Features third.

A feature that corrupts files, silently loses unsupported OOXML, causes uncontrolled memory growth, or bypasses quality gates is not considered complete.

## License

License selection is intentionally deferred until the dependency and contribution model is finalized. Do not add third-party code with license obligations that would constrain the final project license without an explicit architecture decision.
