# Phase 2 OOXML Performance

Phase 2 performance validation focuses on the package foundation rather than editor rendering.

## CI fixture

The reproducible release-mode fixture contains:

- one minimal DOCX main part;
- root relationships and content types;
- 128 small XML parts;
- one 8 MiB deterministic, poorly-compressible opaque binary part.

The smoke measures:

- ZIP central-directory indexing;
- Office family identification;
- selective main-part read;
- validated full-package repack;
- process peak RSS.

## Shared-runner gate

GitHub-hosted Ubuntu is a trend environment, not a reference machine.

The Phase 2 smoke uses a deliberately coarse peak RSS ceiling of **192 MiB** for the entire benchmark process. This includes fixture construction, compression, decompression, repack, allocator state, and ZIP/XML runtime overhead.

The important architecture invariant is stricter than the numeric smoke ceiling:

> Opening/indexing a package MUST NOT eagerly materialize every compressed part.

Actual measurements are recorded from the workflow artifact before Phase 2 is accepted.
