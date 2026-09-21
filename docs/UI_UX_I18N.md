# UI / UX and Localization

Status: **Product baseline after Phase 8 release hardening.**

## Product direction

Nexa Office uses a native Slint shell and intentionally avoids browser-style desktop UI. The interface should feel like a compact desktop productivity application rather than a web dashboard:

- reduce decorative cards, oversized empty surfaces and repeated boxed panels;
- use separators, typography, spacing and restrained state color for hierarchy;
- keep the active Docs / Sheets / Slides editing surface visually dominant;
- make opening an existing Office file a first-class home-screen action;
- keep recent files one click away without exposing paths as the primary visual label;
- avoid continuous animation or redraw that would weaken idle-resource targets;
- never introduce a WebView or required JavaScript runtime to solve product-UI problems.

## Information architecture

The native shell exposes six stable surfaces:

1. Home;
2. Docs;
3. Sheets;
4. Slides;
5. Diagnostics;
6. Settings.

Home prioritizes:

- native **Open Office file**;
- creation of DOCX / XLSX / PPTX files;
- up to three recent Office files with file-type identity;
- compact product-health metadata.

Editor workspaces share the same hierarchy:

- document/workbook/presentation identity and dirty state at the top;
- primary editing commands in a single compact toolbar;
- secondary tools in a side rail or bottom strip;
- compatibility/safe-save state visually separated from ordinary metadata;
- native Save / Save As / Print commands remain available from the menu.

## Responsive behavior

Desktop baseline:

- preferred window: 1240 × 800;
- supported minimum: 840 × 560;
- navigation width is controlled by the persisted compact-navigation preference and does not participate in a self-referential layout loop;
- editor toolbars enter **dense workspace mode** automatically below 1080 px without changing the persisted navigation preference;
- dense mode may replace visible labels with compact symbols, but full accessibility labels remain descriptive;
- Docs paper, Sheets grid and Slides canvas remain the dominant flexible surfaces;
- secondary rails shrink before the primary editing surface does;
- every editing command must remain reachable at the supported minimum size.

## Localization

Supported preferences:

- System;
- 简体中文 (`zh-CN`);
- English (`en`).

The preference is persisted locally through `AppSettings`.

Localization rules:

1. Home, Docs, Sheets, Slides, Diagnostics, Settings, native integration feedback and compatibility state must not intentionally mix Chinese and English when Simplified Chinese is selected.
2. User file names, paths, formulas, cell addresses and Office technical identifiers are never translated.
3. Native dialog/clipboard/print/recovery failure prefixes are localized; low-level OS or format error details may remain technical.
4. Recent-file type badges remain format names (`DOCX`, `XLSX`, `PPTX`) in every locale.
5. CJK text relies on platform font fallback; Nexa does not bundle a large CJK font only for application chrome.
6. Text input stays on Slint's native TextEdit / LineEdit path so the platform input method remains responsible for IME composition.
7. New user-visible UI strings require both Chinese and English forms in the same change until a dedicated translation catalog replaces the lightweight bilingual layer.

## Chinese text and IME

Chinese adaptation is more than translated labels:

- long Chinese labels must fit the supported minimum window without forcing horizontal overflow;
- compact toolbars use symbols only when a full accessible name is retained;
- multiline Docs edits are converted into real semantic paragraphs instead of embedding newline characters into one WordprocessingML text run;
- file names and paths use elision rather than forced wrapping;
- Chinese compatibility warnings may wrap in side rails;
- input controls must remain native Slint widgets to preserve platform IME behavior.

## Recent files and native open flow

The home screen treats opening existing work as equal to creating new content:

- native Open is the first home action and is also available through `Ctrl/Cmd+O`;
- recent rows show file name, full path as secondary text and a DOCX/XLSX/PPTX badge;
- recent-file persistence is owned by the native integration layer;
- failed native dialogs, clipboard operations, recovery and dropped-data parsing surface localized status feedback.

## Accessibility

Custom controls expose Slint accessibility metadata:

- buttons use `accessible-role: button` and descriptive labels;
- switches expose checked state;
- navigation, main content and editor regions are named;
- compact symbol buttons retain full text accessibility names;
- recent-file rows expose their file path as an accessibility description;
- default actions match pointer activation.

Keyboard menus provide the desktop baseline for common commands:

- Open: `Ctrl/Cmd+O`;
- Save: `Ctrl/Cmd+S`;
- Save As: `Ctrl/Cmd+Shift+S`;
- Print: `Ctrl/Cmd+P`;
- Docs Undo/Redo: `Ctrl/Cmd+Z`, `Ctrl/Cmd+Shift+Z`;
- Docs Bold/Italic/Underline: `Ctrl/Cmd+B/I/U`.

## Visual baseline

- neutral light surfaces with restrained blue emphasis;
- no heavy shadows, glass effects or decorative gradients;
- 1 px separators instead of nested card stacks where possible;
- compact 9–13 px metadata typography and 13–15 px working text;
- large headings are reserved for top-level Home/Settings/Diagnostics surfaces;
- Docs uses a white paper surface against a neutral workspace;
- Sheets keeps grid lines light and selection contrast explicit;
- Slides keeps the 16:9 canvas visually dominant and element controls secondary;
- state color is reserved for selection, safe/writable state, warnings and blocked saves.

## Acceptance matrix

| Surface | English | Simplified Chinese | 840×560 | Accessibility names |
| --- | --- | --- | --- | --- |
| Home / recent files | required | required | required | required |
| Docs | required | required | required | required |
| Sheets | required | required | required | required |
| Slides | required | required | required | required |
| Diagnostics | required | required | required | required |
| Settings | required | required | required | required |
| Native open/save/print/recovery feedback | required | required | n/a | status text |

## Quality gates

UI/localization changes must keep the existing engineering gates green:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
```

In addition:

- Native Performance Smoke must remain green;
- Docs Performance Smoke must remain green;
- Native Integration Gate must remain green;
- release executable size must stay under the existing 25 MiB gross-regression gate;
- UI polish is rejected if it materially regresses settled memory, idle CPU, startup or editor performance.
