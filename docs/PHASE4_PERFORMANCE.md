# Phase 4 — Nexa Sheets Performance

This page records the Phase 4 shared-runner trend baseline. It is a regression signal, not an end-user hardware promise.

## Fixture

The release benchmark builds a sparse workbook with:

- theoretical XLSX bounds: 1,048,576 rows × 16,384 columns;
- 8,000 populated data rows;
- 32,005 populated cells;
- formulas in the Total column;
- formatting and a frozen first row/column;
- an auto-filter range;
- one populated tail cell at XFD1048576;
- a bounded 24 × 10 viewport sampled around row 900,001.

The tail cell proves that the workbook can address the full XLSX range without materializing intermediate rows.

## Recorded GitHub-hosted Ubuntu sample

| Measurement | Sample |
| --- | ---: |
| Populated cells | 32,005 |
| Virtual viewport cells | 240 |
| Estimated semantic model | 3,742,392 bytes (~3.57 MiB) |
| Generated XLSX | 232,157 bytes |
| Workbook construction | 28,470 µs |
| Far-row viewport extraction | 266 µs |
| Recalculation | 8,568 µs |
| XLSX save | 33,953 µs |
| XLSX reopen | 52,921 µs |
| Benchmark peak RSS | 28,656 KiB (~28.0 MiB) |

## What this validates

- memory scales with populated semantic state, not the million-row theoretical dimension;
- viewport work remains bounded to the requested visible region;
- formulas recalculate deterministically before serialization;
- a sparse tail cell survives save/reopen;
- the XLSX path remains inside a low-memory native process budget.

## Gate philosophy

CI intentionally uses gross-regression limits rather than treating one shared-runner sample as a guaranteed product number. A change that materially increases sparse-model memory, viewport latency, recalculation time, save/reopen time or process RSS must be investigated before merge.
