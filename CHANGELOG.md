# Changelog

All notable Nexa Office changes are recorded here. Nexa Office follows semantic-versioned release labels; pre-1.0 builds may change unsupported or explicitly experimental behavior.

## [Unreleased]

## [0.1.0-alpha.1] — Candidate

First Alpha candidate of the native Nexa Office product baseline.

### Added

- Native Rust/Slint shell for Windows, macOS and Linux.
- DOCX, XLSX and PPTX semantic editing subsets.
- OOXML/OPC preservation and compatibility blocking.
- Sparse, virtualized Sheets engine and deterministic supported-formula recalculation.
- Native Slides editing model.
- Simplified Chinese / English / System language modes.
- Recent files, native dialogs, printing handoff, clipboard integration and crash recovery.
- Platform packaging, file associations, release metadata and SHA-256 manifests.
- Cross-platform CI, interoperability, performance and release-hardening gates.

### Changed

- Product-wide UI/UX was tightened for desktop density and accessibility.
- Native integration failure prefixes are localized in Simplified Chinese.
- Release tooling now distinguishes prerelease labels from platform installer version fields.

### Known limitations

See `docs/KNOWN_LIMITATIONS.md` and `docs/SUPPORT_MATRIX.md`. This Alpha is not intended for production-critical documents.
