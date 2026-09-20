# Phase 1 Performance Baseline

This file records measured Phase 1 shell baselines.

The budgets remain authoritative in [PERFORMANCE.md](PERFORMANCE.md). Numbers are added here only after they are measured from a release build on a documented machine.

## Required baseline states

1. Process launch to responsive start screen.
2. Settled start screen for at least 30 seconds.
3. Idle CPU over 30 seconds.
4. Private/owned memory.
5. Release executable/package size.

## Current status

| Platform | Commit | Startup | Idle private/PSS/footprint | Idle CPU | Binary/package | Status |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| macOS | — | — | — | — | — | pending reference run |
| Windows | — | — | — | — | — | pending reference run |
| Linux | — | — | — | — | — | pending reference run |

No numbers are intentionally guessed. Phase 1 is not performance-accepted until actual reference-machine results are recorded.

## Measurement rules

Build:

```bash
cargo build --release --locked -p nexa-app
```

Run the release executable and allow the start screen to settle before collecting memory/CPU.

Use the platform metric defined by [PERFORMANCE.md](PERFORMANCE.md):

- Linux: PSS + USS/private; RSS only as context.
- Windows: Private Bytes/private committed; Working Set only as context.
- macOS: physical footprint/resident/private using a documented repeatable tool.

Record:

- hardware;
- OS;
- architecture;
- commit SHA;
- exact command/tool;
- at least 3 samples for startup and memory;
- p50 (and p95 when enough samples exist).

## Acceptance

Phase 1 target:

- settled start screen target <= 40 MiB;
- hard budget <= 60 MiB;
- idle CPU hard gate < 0.5% of one logical core averaged over 30 seconds;
- warm startup target <= 300 ms;
- cold p95 <= 1,000 ms.

If the shell itself cannot approach these limits, Phase 2 must not normalize the regression. The GUI feature set/backend must be revisited first.
