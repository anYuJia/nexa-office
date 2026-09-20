# Phase 1 Status

Phase 1 builds the native application shell and makes resource usage measurable before document-engine work begins.

## Implemented

- Rust 1.98.1 workspace with a committed lockfile.
- Slint 1.18 native shell with no Chromium, Electron, Tauri, CEF, WebView, or JavaScript runtime.
- Lightweight software renderer selected after measured FemtoVG memory regression.
- UI-independent application state and command routing.
- Home, Diagnostics, and Settings surfaces.
- Local, dependency-free settings persistence on Windows, macOS, and Linux.
- Settings that affect the shell immediately: status-bar visibility and compact navigation.
- Bounded recent-file state with path privacy in user-facing status text.
- Command-line file handoff into the future open pipeline.
- Runtime diagnostics for platform, architecture, build, renderer, and shell initialization time.
- Windows/macOS/Linux format, check, Clippy, and unit-test CI gates.
- Native Linux release-launch smoke with memory, idle-CPU, and binary-size measurements.
- Gross-regression CI limits for memory, idle CPU, and release binary size.
- ADRs for GUI, document renderer boundary, text shaping, XML, and ZIP/OPC.

## Measured CI trend

On the GitHub-hosted Ubuntu/Xvfb smoke environment, the software-renderer shell has remained around:

- PSS: ~13 MiB;
- private memory: ~11 MiB;
- idle CPU: 0.000% in the sampled idle window;
- release binary: ~12.6-12.8 MiB.

These are trend measurements from a shared runner, not end-user performance claims.

## Remaining Phase 1 acceptance work

- Record repeatable startup p50/p95 on dedicated reference hardware.
- Record dedicated Windows, macOS, and Linux memory/idle-CPU baselines.
- Validate native text input, clipboard, DPI, focus, and accessibility behavior interactively on supported platforms.
- Configure repository-side main branch protection/required checks when administrative access is available.

Phase 2 should not weaken the shell budgets to make new dependencies fit. If OOXML infrastructure causes a large regression, the dependency or loading strategy must be revisited.
