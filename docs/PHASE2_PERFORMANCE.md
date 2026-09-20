# Phase 2 OOXML Performance

Phase 2 performance validation focuses on the package foundation rather than editor rendering.

## CI fixture

The reproducible release-mode fixture contains:

- one minimal DOCX main Part;
- root Relationships and Content Types;
- 128 small XML Parts;
- one 8 MiB deterministic, poorly-compressible opaque binary Part.

The smoke measures ZIP indexing, Office family identification, selective Part reading, validated repack, and peak process RSS.

## Measured shared-runner result

Environment: GitHub-hosted Ubuntu, release build, commit `ce696f36`.

| Metric | Result |
| --- | ---: |
| Fixture ZIP size | 8,412,830 bytes |
| Indexed entries | 132 |
| Declared uncompressed bytes | 8,395,583 bytes |
| ZIP central-directory index | 272 µs |
| Office family identification | 46 µs |
| Selected 150-byte main-Part read | 4 µs |
| Validated full repack | 243,259 µs (~243 ms) |
| Repacked ZIP size | 8,412,830 bytes |
| Complete benchmark peak RSS | 19,252 KiB (~18.8 MiB) |

This is shared-runner trend evidence, not an end-user performance claim.

## CI regression gate

The OOXML smoke now fails above **64 MiB peak RSS** for the complete benchmark process.

The process includes fixture construction, compression, decompression, repack, allocator state, and ZIP/XML runtime overhead. The measured sample is substantially below the gate.

The stronger architecture invariant remains:

> Opening/indexing a package MUST NOT eagerly materialize every compressed Part.

Dedicated user-document open/save benchmarks will be added with the Docs, Sheets and Slides semantic engines.
