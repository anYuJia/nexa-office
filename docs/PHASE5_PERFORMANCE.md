# Phase 5 Slides Performance

Phase 5 adds a release-mode presentation benchmark and CI regression gate.

## Fixture

The benchmark creates:

- 180 slides
- more than 2,000 semantic slide elements
- text-heavy slides with bounded element vectors
- periodic 4 × 4 tables
- native PPTX save and reopen
- full semantic slide/element traversal after reopen

This intentionally tests a medium/large deck without creating one UI widget per semantic element outside the currently visible Slides workspace.

## Shared-runner regression gates

The GitHub-hosted Ubuntu smoke currently enforces:

- exactly 180 reopened slides
- more than 2,000 semantic elements
- reopened element count equals the original count
- generated PPTX below 32 MiB
- save below 5 s
- reopen below 5 s
- semantic traversal below 500 ms
- benchmark peak RSS below 128 MiB

These are gross regression gates for shared CI hardware, not end-user reference-machine claims.

## Design rules

- slide models are plain Rust data rather than widget trees
- the shell only projects current-slide summaries into Slint
- no continuous animation/redraw is required while idle
- imported unknown content is preserved at package level where possible and unsafe rewrites are blocked
