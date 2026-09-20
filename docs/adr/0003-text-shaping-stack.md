# ADR 0003 — Text shaping and font stack

Status: Proposed

## Context

Office fidelity depends on text shaping, font fallback and metrics. This includes CJK, bidi, emoji, combining marks, ligatures, variable fonts and platform font discovery.

A visually plausible Latin-only implementation is not sufficient.

## Decision

Evaluate a Rust-native stack centered on COSMIC Text / HarfRust concepts, while keeping Nexa's document model independent from any specific shaping library.

No text stack is Accepted in Phase 1 until the required script/fallback fixture matrix has been measured.

## Required prototype coverage

- Latin;
- Simplified and Traditional Chinese;
- Japanese;
- Korean;
- Arabic and Hebrew bidi;
- emoji and variation selectors;
- combining marks;
- ligatures;
- variable fonts;
- missing-font fallback;
- mixed-script paragraphs;
- high-DPI;
- stable metrics needed for pagination.

## Constraints

The shaping layer must expose compact glyph runs and metrics, not UI widgets.

Font caches must be bounded and observable.

Changing the shaping stack after document layout is built around it is expensive, so acceptance requires test evidence rather than preference.

## Revisit conditions

Accept or reject the candidate after the Phase 2/early Phase 3 text prototype.
