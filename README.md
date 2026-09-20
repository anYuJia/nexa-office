# Nexa Office

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

Nexa Office has completed **Phase 2 — OOXML / OPC Foundation** and is entering **Phase 3 — Nexa Docs MVP**.

The native Rust/Slint shell remains performance-gated. The shared OOXML package layer now provides bounded ZIP/XML parsing, DOCX/XLSX/PPTX package detection, package-level round-trip preservation, unknown opaque Part preservation, fuzz boundaries, and failure-safe saving.

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
