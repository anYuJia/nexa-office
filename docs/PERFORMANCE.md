# Performance Budgets

Performance is part of Nexa Office's product contract.

These values are engineering targets and release gates once the corresponding feature exists. Early prototypes must record measurements even when they do not yet pass the final target.

## Measurement principles

Do not compare incomparable metrics.

Preferred memory metrics:

- Windows: Private Bytes / private committed memory, plus Working Set as secondary context.
- Linux: PSS and USS/private, with RSS as secondary context.
- macOS: physical footprint / resident/private measurements using a documented repeatable tool.

Every benchmark report records:

- commit;
- build profile;
- OS/version;
- CPU/architecture;
- RAM;
- GPU where relevant;
- document fixture;
- cold/warm state;
- metric/tool.

Release decisions use pinned reference machines. Shared CI runners are trend indicators, not authoritative absolute-memory measurements.

## Idle CPU

After the UI settles with no active work:

- target: approximately 0% application CPU;
- hard release gate: less than 0.5% average of one logical core over a sustained 30-second idle sample, excluding known OS instrumentation noise.

There MUST NOT be a permanent 60 FPS render loop.

## Memory budgets

Initial release targets for private/owned application memory:

| Scenario | Target | Hard budget |
| --- | ---: | ---: |
| Start screen, settled | <= 40 MiB | <= 60 MiB |
| Blank Docs document | <= 70 MiB | <= 100 MiB |
| Typical 20-page DOCX | <= 100 MiB | <= 150 MiB |
| Typical XLSX workbook | <= 120 MiB | <= 180 MiB |
| Typical PPTX deck | <= 120 MiB | <= 180 MiB |
| Large 1,000,000-row sparse XLSX navigation | <= 180 MiB | <= 250 MiB |

These budgets exclude unavoidable shared OS graphics-driver pages when the platform cannot attribute them fairly, but benchmark reports must still disclose the full observed process footprint.

A feature MUST NOT meet the budget by silently disabling expected functionality.

## Startup

Reference target from process launch to responsive start screen:

- warm p50: <= 300 ms;
- cold p50: <= 600 ms;
- cold p95: <= 1,000 ms.

Startup MUST avoid eager initialization of Docs/Sheets/Slides engines that are not needed for the start screen.

## Open latency

Representative local files on reference SSD:

### DOCX

- small/simple: target < 300 ms to first editable view;
- typical 20-page: target < 700 ms;
- large documents: progressive usability preferred over blocking full-document work.

### XLSX

- small workbook: target < 400 ms to visible sheet;
- large workbook: show first useful viewport without materializing the full grid.

### PPTX

- small deck: target < 400 ms to first slide;
- large deck: thumbnails/content may populate progressively.

These are user-perceived readiness targets, not necessarily full-background-completion times.

## Interaction latency

### Text editing

- normal keystroke-to-visible-update p95: < 16 ms where layout impact is local;
- expensive reflow p95: < 50 ms for ordinary documents;
- long reflow must be cancelable/progressive rather than freezing the UI.

### Scrolling

Target:

- 60 Hz-capable smoothness on reference hardware;
- no allocations proportional to total spreadsheet dimensions per scroll event;
- frame p95 < 16.7 ms for ordinary visible regions where platform refresh is 60 Hz.

### Slide switch

- cached/near slide: target < 50 ms;
- uncached ordinary slide: target < 150 ms to useful render.

## Save latency

For typical local documents:

- target < 500 ms for small files;
- target < 1.5 s for ordinary documents.

Large files may take longer, but saving should expose progress/cancellation policy where appropriate and MUST protect the previous valid file.

## Binary and package size

Initial goals:

- core executable and required native libraries should remain intentionally small;
- desktop installer target: <= 80 MiB for a base language build;
- hard review required for any single dependency/asset change that increases release package size by >5% or >5 MiB, whichever is smaller.

Fonts, dictionaries, templates and optional language packs SHOULD be modular where possible.

## Cache rules

Every cache must define:

- key;
- value;
- maximum size/count or eviction policy;
- invalidation rule;
- measurement/debug visibility.

Unbounded caches are forbidden.

Suggested design:

- Docs: visible pages plus bounded near-page cache.
- Sheets: visible/near-visible render data; semantic cell storage remains separate.
- Slides: active slide plus bounded neighboring/full-resolution cache; thumbnails lower-resolution.

## Large-file behavior

Nexa Office should degrade progressively, not catastrophically.

Rules:

- streaming ZIP/XML where practical;
- bounded decompression;
- sparse workbook representation;
- no per-empty-cell allocation;
- lazy images/media decode when possible;
- avoid keeping both full compressed and full expanded copies without reason;
- cancel obsolete layout/render work.

## Regression thresholds

For a stable benchmark against accepted main:

- >5% memory regression: investigate and normally block.
- >10% latency regression: investigate and normally block.
- >5% binary/package regression: investigate and normally block.

Noise must be characterized with repeated samples before declaring a regression.

An intentional regression requires:

- measured numbers;
- user benefit;
- alternatives considered;
- explicit review;
- updated baseline only after acceptance.

## Benchmark output

Performance reports should include a compact before/after table and raw artifacts where practical.

Never publish invented or single-run numbers as reliable conclusions.
