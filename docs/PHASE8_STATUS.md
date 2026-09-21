# Phase 8 — Alpha / Beta / 1.0 Hardening

Status: **Completed — release-hardening engineering baseline**.

Phase 8 converts the completed native product baseline into a release-ready engineering process. Completing this phase does not itself declare a 1.0 release.

## Implemented on this branch

- build-time Windows manifest embedding;
- package version injection for Windows, macOS and Linux;
- deterministic release input contract using Cargo.lock and the pinned Rust toolchain;
- three-platform installer and portable-package builders;
- release artifact SHA-256 manifests;
- build metadata with source commit, Rust/Cargo versions, target platform and artifact sizes;
- Cargo dependency inventory for every release build;
- support matrix;
- known limitations;
- migration/compatibility notes;
- privacy-preserving crash-reporting strategy;
- release checklist;
- reproducible-build/traceability contract;
- automated bounded long-session RSS growth smoke;
- three-platform installer-build validation in the Release Hardening Gate;
- tag/manual Release Artifacts workflow.

## Release levels

### Alpha readiness

Alpha requires the existing Docs / Sheets / Slides workflows, data-integrity gates, performance gates, interoperability gate and recovery behavior to remain green. The crash-reporting decision is explicitly local-first and telemetry-free by default.

### Beta readiness

Beta additionally requires broad corpus testing, real package builds, bounded soak validation, migration notes and a rehearsed release checklist. Automated CI provides a bounded soak smoke; the release checklist still requires a 2+ hour interactive reference-machine soak.

### 1.0 readiness

1.0 additionally requires publisher decisions and release-environment evidence that source control cannot supply by itself:

- final project license/contribution model;
- signing/notarization credentials;
- final stable version/tag;
- reference-hardware sustained performance evidence;
- a rehearsed security-response and upgrade/rollback process.

These are explicit release gates rather than hidden TODOs.

## Exit validation

The implementation branch exits after the final HEAD passes:

- existing Windows/macOS/Linux CI;
- OOXML/interoperability/document performance gates;
- Native Integration and Native Performance gates;
- Release Hardening Gate with real package builds on all three platforms.

Formal Alpha, Beta or 1.0 publication remains a separate release decision using `docs/RELEASE_CHECKLIST.md`.


## Publication boundary

Completing Phase 8 means the repository now contains the engineering controls needed to build and validate Alpha/Beta/1.0 candidates. It does **not** claim that Nexa Office 1.0 has been published.

A public Alpha, Beta or 1.0 release still requires the publisher-controlled checklist items that cannot be satisfied by source code alone: final version/tag choice, signing/notarization credentials, final license/contribution decision, reference-hardware interactive soak evidence and the actual release publication step.
