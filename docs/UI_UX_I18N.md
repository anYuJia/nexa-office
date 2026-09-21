# UI / UX and Localization

Status: active product baseline after Phase 3.

## Product direction

Nexa Office uses a native Slint shell and intentionally avoids a browser-style desktop layout. The interface should remain quiet, information-dense, and desktop-native:

- reduce decorative cards and oversized empty surfaces;
- use separators, typography and spacing for hierarchy;
- keep the primary document surface visually dominant;
- avoid continuous animation or redraw that would weaken idle-resource targets;
- do not add a WebView or JavaScript runtime to solve UI problems.

## Responsive behavior

Current desktop breakpoints:

- default window: 1240 × 800;
- supported minimum: 840 × 560;
- compact navigation is a persisted user preference and never depends on a self-referential window-layout binding;
- enabling compact navigation also switches the Docs toolbar and tools rail to their dense presentation;
- the document and spreadsheet workspaces keep bounded, clipped editing surfaces at the supported minimum size;
- document paper width adapts to the remaining workspace.

Responsive behavior must preserve access to every editing command. A compact visual label may use a symbol, but the accessibility label must remain descriptive.

## Localization

Supported preferences:

- System;
- 简体中文 (`zh-CN`);
- English (`en`).

The preference is persisted locally through `AppSettings`.

Rules:

1. User-visible shell and Docs state must not mix Chinese and English when Simplified Chinese is selected.
2. File names and paths are never translated.
3. Error prefixes and command feedback are localized; low-level format errors may retain technical terms when no stable translation exists.
4. CJK text uses platform font fallback. Nexa does not bundle a large CJK font only for UI chrome because that would conflict with package-size goals.
5. New UI strings require both Chinese and English forms in the same change until a dedicated catalog replaces the current lightweight bilingual layer.

## Accessibility

Custom controls expose Slint accessibility metadata:

- buttons use `accessible-role: button` and descriptive labels;
- switches expose checked state;
- navigation and main/document regions are named;
- symbol-only compact buttons retain full text accessibility names;
- default actions match pointer activation.

## Visual baseline

- neutral light surfaces with restrained blue emphasis;
- no heavy shadows or glass effects;
- 1 px separators instead of nested cards where possible;
- compact 9–13 px metadata typography, 13–15 px working text, larger page titles only for top-level surfaces;
- document canvas stays white against a subtle neutral workspace;
- state color is reserved for selection, writable/safe state, warnings and blocked saves.

## Quality gates

UI/localization changes must keep the existing gates green:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```

Native and Docs performance smoke remain mandatory. UI polish is not accepted if it materially regresses idle CPU, memory, launch behavior or editor responsiveness.
