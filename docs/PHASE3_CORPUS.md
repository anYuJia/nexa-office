# Phase 3 DOCX Compatibility Corpus

Phase 3 uses two fixture classes.

## Nexa-generated acceptance fixtures

The unit/integration suite generates DOCX packages covering:

- paragraphs and rich runs;
- styles and numbering;
- tables;
- sections and page geometry;
- image relationships;
- safe edit → save → reopen;
- compatibility blocking for unsupported destructive constructs.

## Pinned external corpus

CI also downloads three small DOCX files from the `python-openxml/python-docx` test corpus at commit
`e45454602b53e8e572b179ccf1c91093ec9f4ed7`:

- `test.docx`;
- `having-images.docx`;
- `blk-inner-content.docx`.

The upstream project is MIT-licensed. The files are downloaded during CI and are not vendored into the Nexa repository.

The corpus gate requires every file to open and paginate without crashing. A file is only rewritten when the compatibility report says Nexa can reproduce the parsed semantic content safely. Unsupported destructive constructs therefore remain a save blocker rather than being silently discarded.

This external corpus is intentionally small. It is a regression floor, not a claim of complete Microsoft Word compatibility.

## Current CI snapshot

Pinned commit: `e45454602b53e8e572b179ccf1c91093ec9f4ed7`.

| Fixture | Paragraphs | Pages | Compatibility issues | Writable |
| --- | ---: | ---: | ---: | --- |
| `test.docx` | 2 | 1 | 2 | no |
| `having-images.docx` | 5 | 1 | 4 | no |
| `blk-inner-content.docx` | 6 | 1 | 2 | no |

All three files open and paginate without crashing. They are currently intentionally non-writable because the compatibility detector found constructs outside the Phase 3 writer's safe reproduction set.

This is considered a successful safety outcome: Nexa may inspect documents outside its writable subset, but it must not silently rewrite them with data loss.
