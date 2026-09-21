# Phase 7 — Native Product Integration

Status: **In progress**.

Phase 7 turns the native Docs / Sheets / Slides engines into a desktop product without introducing a browser runtime or a heavyweight integration framework.

## Implemented

- persisted bounded recent-file history;
- recent-file UI on the start screen;
- native open and Save As dialogs:
  - macOS via system AppleScript dialogs;
  - Windows via native WinForms dialogs through PowerShell;
  - Linux via Zenity where available;
- native print handoff:
  - CUPS `lp` on macOS/Linux;
  - Windows shell Print verb;
- native clipboard path copy;
- crash-recovery snapshots for DOCX / XLSX / PPTX:
  - five-second write throttling;
  - only writable semantic state is snapshotted;
  - original path metadata is preserved separately;
  - latest recovery is restored on next launch;
  - clean saves/open operations clear stale recovery state;
- platform packaging metadata and Office file associations for DOCX / XLSX / PPTX;
- macOS Retina/high-resolution bundle metadata;
- Windows PerMonitorV2 DPI manifest;
- Linux desktop/MIME integration;
- responsive navigation restored across Docs / Sheets / Slides.

## Product integration policy

Nexa Office keeps native integration outside the semantic document engines. OS dialogs, printing, clipboard and packaging must not create a dependency from Docs / Sheets / Slides into platform UI code.

No Chromium, WebView, Electron, Tauri or required JavaScript runtime is permitted for these workflows.

## Recovery contract

Recovery snapshots are best-effort protection for unsaved semantic state, not a replacement for normal Save.

- recovery files live in the local Nexa application-data directory;
- snapshots are bounded to one current file per editor family;
- snapshots use the same safe atomic Office writers as ordinary files;
- unsupported destructive rewrites are never bypassed to create recovery data;
- successfully saved or freshly opened files clear stale recovery for that editor family.

## Packaging baseline

Checked-in platform metadata lives under `packaging/`.

- Linux: desktop entry, MIME metadata and package staging script;
- macOS: application bundle metadata and package script;
- Windows: DPI manifest, WiX source and package script.

Release signing/notarization requires publisher credentials and therefore remains a release-environment responsibility rather than checked-in secrets.

## Remaining exit validation

- three-platform Rust quality;
- native integration unit tests;
- packaging metadata validation;
- package-size gate;
- existing Docs / Sheets / Slides / OOXML / interoperability performance gates.

Phase 7 is complete only when the latest branch HEAD passes those gates.
