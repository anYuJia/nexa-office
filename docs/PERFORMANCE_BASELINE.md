# Phase 1 Performance Baseline

This file records measured Phase 1 shell baselines.

The budgets remain authoritative in [PERFORMANCE.md](PERFORMANCE.md). Reference numbers are added only after they are measured from a release build on a documented machine.

## Required baseline states

1. Process launch to responsive start screen.
2. Settled start screen for at least 30 seconds.
3. Idle CPU over a sustained sample.
4. Private/owned memory.
5. Release executable/package size.

## Current reference status

| Platform | Commit | Startup | Idle private/PSS/footprint | Idle CPU | Binary/package | Status |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| macOS | — | — | — | — | — | pending reference run |
| Windows | — | — | — | — | — | pending reference run |
| Linux | — | — | — | — | — | pending reference run |

No reference-machine numbers are guessed. Shared CI measurements below are trend evidence, not substitutes for reference-machine acceptance.

## CI native smoke trend

GitHub-hosted Ubuntu + Xvfb is used to detect large regressions and validate that the release shell launches natively.

| Shell renderer | Commit | Settle | PSS | Private | Idle CPU | Release binary |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| FemtoVG | d9b5dbcf | 8 s | ~136.7 MiB | ~133.7 MiB | not isolated | ~12.4 MiB |
| Software | 673c75de | 30 s | ~13.0 MiB | ~11.0 MiB | 0.000% / one core | ~12.6 MiB |\n| Software + Home/Diagnostics/Settings | 44604b15 | 30 s | ~13.0 MiB | ~11.1 MiB | 0.000% / one core | ~12.75 MiB |

The renderer change reduced the CI smoke PSS by roughly an order of magnitude. These figures are environment-specific and MUST NOT be advertised as end-user hardware measurements.

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

The current software-renderer CI smoke is comfortably below the memory hard budget, but Phase 1 still requires dedicated Windows/macOS/Linux reference runs before final performance acceptance.
