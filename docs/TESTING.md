# Testing Strategy

Nexa Office treats document parsing, serialization, layout and performance as correctness-critical systems.

## Testing pyramid

### Unit tests

Use for:

- domain types;
- parsers for focused XML fragments;
- formula functions;
- command/state transitions;
- layout primitives;
- cache policies;
- coordinate/unit conversions.

Unit tests should be deterministic and fast.

### Integration tests

Use for:

- OOXML package relationships;
- document open/edit/save;
- workbook recalculation;
- slide/theme resolution;
- font and image integration;
- platform bridges.

### Round-trip tests

Round-trip is a core test class.

Typical flow:

1. open fixture;
2. build semantic model;
3. apply zero or controlled edits;
4. save to a new package;
5. reopen;
6. compare expected semantics;
7. inspect preservation of unrelated package parts.

Byte-identical output is not generally required because ZIP/XML ordering may legitimately change. Semantic stability is required.

### Golden/render tests

Use for layout/rendering where semantic assertions are insufficient.

Golden tests should:

- render at fixed DPI/font environment;
- compare with a controlled tolerance;
- keep fixtures small;
- avoid turning every UI pixel into a brittle snapshot.

A visual golden never replaces semantic document assertions.

### Property tests

Good candidates:

- unit/EMU/twip conversions;
- range operations;
- cell references;
- style resolution;
- relationship identifiers;
- parse/serialize invariants;
- undo/redo properties.

### Fuzzing

Fuzz boundaries exposed to untrusted files:

- ZIP/OPC package parsing;
- XML tokenization/helpers;
- relationship/content-type parsing;
- DOCX/XLSX/PPTX part parsers;
- image metadata boundaries if custom parsing exists;
- formula parser.

Every reproducible crash should become a minimized regression case where practical.

### Performance tests

Benchmarks are required for:

- cold startup;
- idle memory;
- package open;
- document parse;
- layout;
- visible-page render;
- spreadsheet scrolling/viewport update;
- formula recalculation;
- slide switch;
- save/export;
- large-file memory.

Performance tests run on pinned reference environments for release decisions. CI microbenchmarks may detect trends but shared runners are not authoritative for absolute memory.

## Fixture classes

Maintain separate fixture groups:

- tiny synthetic cases;
- feature-specific cases;
- malformed/adversarial cases;
- real-world anonymized cases;
- large stress cases;
- regression cases.

Do not commit personal documents or proprietary customer data.

Fixtures must have clear provenance and redistributable licensing.

## Coverage policy

Coverage is a signal, not the goal.

Initial targets:

- shared OOXML/package/parsing code: at least 85% line coverage where measurable;
- serializer/relationship/content-type logic: at least 90%;
- core command/state logic: at least 85%;
- UI glue: no arbitrary numeric target if behavior is better covered by integration tests.

Critical branches involving corruption, bounds and save failure require direct tests regardless of aggregate coverage.

A PR MUST NOT add meaningless tests solely to raise coverage percentage.

## Parser test requirements

For every parser:

- valid minimal input;
- representative valid input;
- missing optional fields;
- unknown elements/attributes;
- invalid numeric/text values;
- extreme lengths/counts;
- namespace variation;
- truncated input;
- duplicated/conflicting metadata where relevant.

Input-controlled allocation MUST be tested against limits.

## Save test requirements

Save tests should verify:

- destination created successfully;
- original remains intact on failure;
- temporary file cleanup;
- reopened result matches supported semantic state;
- unknown preserved parts remain present where promised;
- embedded media relationships remain valid.

## Text/layout matrix

The text stack needs fixtures for:

- Latin;
- Simplified/Traditional Chinese;
- Japanese;
- Korean;
- Arabic/Hebrew bidi;
- emoji;
- combining marks;
- ligatures;
- variable fonts;
- fallback fonts;
- mixed-script paragraphs.

## Spreadsheet matrix

Include:

- sparse sheets;
- very large dimensions;
- formulas/chains;
- merged ranges;
- hidden rows/columns;
- freeze panes;
- shared strings;
- inline strings;
- dates/numbers;
- styles;
- cross-sheet references;
- malformed formulas.

## Presentation matrix

Include:

- theme/master/layout inheritance;
- grouped shapes;
- text boxes;
- images;
- tables;
- z-order;
- rotations/transforms;
- notes;
- missing media;
- unusual slide sizes.

## Leak/stability tests

Before beta:

- repeated open/close cycles;
- repeated file switch;
- repeated undo/redo;
- repeated slide switch;
- repeated sheet navigation;
- 2+ hour editing soak;
- memory floor after cache eviction.

A stable cache plateau is acceptable. Monotonic unbounded growth is not.

## CI tiers

### Tier 1 — every PR

- fmt;
- check;
- clippy;
- unit tests;
- focused integration tests;
- small fixture round-trip tests.

### Tier 2 — merge/nightly

- full integration suite;
- fuzz smoke runs;
- larger fixture corpus;
- render goldens;
- platform-specific tests.

### Tier 3 — release/performance

- reference-machine memory/startup;
- large stress fixtures;
- soak/leak tests;
- installer smoke tests;
- broad interoperability corpus.

## Test evidence

AI agents and contributors must report tests actually executed.

Do not write “all tests pass” when only a subset was run.
