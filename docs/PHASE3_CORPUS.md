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
