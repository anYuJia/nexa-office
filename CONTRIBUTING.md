# Contributing to Nexa Office

Thank you for contributing.

Nexa Office is intentionally strict about data integrity, performance and architecture. Please read:

- AGENTS.md
- docs/PROJECT_CHARTER.md
- docs/ARCHITECTURE.md
- docs/ENGINEERING_GATES.md
- docs/TESTING.md
- docs/CODE_STANDARDS.md
- docs/PERFORMANCE.md

before substantial implementation work.

## Workflow

1. Create a focused branch.
2. Keep the change scoped to one coherent concern.
3. Add tests with behavior changes.
4. Run the relevant local gates.
5. Include benchmark evidence for performance-sensitive code.
6. Open a pull request explaining behavior, tests and known limitations.

## Architecture changes

Create an ADR under docs/adr/ before or with changes that introduce or replace foundational technology.

Do not casually replace:

- GUI toolkit;
- renderer;
- text shaping stack;
- XML/ZIP stack;
- async runtime;
- persistence/indexing strategy;
- plugin architecture;
- licensing model.

## File-format changes

For DOCX/XLSX/PPTX work, include a minimized redistributable fixture or generated test case when practical.

A fix for corruption/data loss must include a regression test.

## Performance

If the change touches startup, parsing, layout, rendering, scrolling, font handling, caching, formula evaluation, open/save or package size, include before/after measurements once the relevant benchmark exists.

## Pull request description

A good PR states:

- What changed?
- Why?
- What is intentionally out of scope?
- Which tests were run?
- What are the benchmark deltas?
- Are there compatibility or round-trip implications?

## Security

Do not open a public issue with exploit details for a serious unpublished vulnerability once a private reporting channel is available. Until then, avoid posting sensitive proof-of-concept document contents publicly and contact the repository owner directly.
