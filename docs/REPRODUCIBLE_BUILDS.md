# Reproducible Release Builds

Nexa Office release artifacts are built by CI from a pinned source commit and locked Rust dependency graph.

## Inputs

- exact Git commit;
- checked-in `Cargo.lock`;
- pinned Rust toolchain;
- release Cargo profile;
- checked-in packaging metadata and scripts;
- platform runner image documented by the workflow.

## Outputs

Every release build produces:

- native platform package;
- portable archive;
- SHA-256 checksum file;
- build metadata JSON;
- Cargo dependency inventory.

The metadata records version, commit, target OS/architecture and Rust/Cargo versions.

## Reproducibility scope

Cross-machine package bytes may still differ because platform packaging tools can embed timestamps or platform metadata. The reproducibility contract therefore has two levels:

1. **source reproducibility** — a fresh checkout of the same commit and lockfile produces the same program semantics and dependency graph;
2. **artifact traceability** — every published binary is tied to its source commit and accompanied by hashes and build metadata.

A future byte-for-byte reproducible packaging initiative may tighten this contract where platform tooling permits it.


## Prerelease version mapping

Nexa distinguishes the semantic **release label** from platform package version fields.

For example, the release label `0.1.0-alpha.1` is built from workspace version `0.1.0`:

- artifact names and build metadata use `0.1.0-alpha.1`;
- Windows MSI product version uses numeric `0.1.0`;
- macOS bundle/package version uses numeric `0.1.0`;
- Debian package metadata uses `0.1.0~alpha.1` so the Alpha sorts before stable `0.1.0`.

The release verifier accepts either an exact workspace version or a semantic prerelease whose base is exactly the workspace version. This keeps stable tags strict while allowing Alpha/Beta/RC artifacts without violating platform installer version rules.
