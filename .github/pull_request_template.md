## Summary

Describe the change and why it is needed.

## Scope

- In scope:
- Intentionally out of scope:

## Correctness / interoperability

- [ ] No source document is modified during open/import.
- [ ] Supported content survives open/edit/save/reopen.
- [ ] Unknown OOXML/package content is preserved where the affected layer promises preservation.
- [ ] Save failure cannot destroy the previous valid file.
- [ ] Not applicable to document formats.

## Tests

List the exact checks executed:

- [ ] Formatting
- [ ] Compile/check
- [ ] Clippy/lints
- [ ] Unit tests
- [ ] Integration/round-trip tests
- [ ] Fixture/regression test added where applicable
- [ ] Fuzz/property/golden test where applicable

Commands/results:

<!-- Do not claim a check was run if it was not. -->

## Performance

Does this touch startup, parsing, layout, rendering, scrolling, caching, fonts, formula evaluation, open/save, dependencies, binary size or memory?

- [ ] No
- [ ] Yes — before/after evidence is included below

Before:

After:

Delta:

## Architecture / dependencies

- [ ] No foundational dependency or architecture change.
- [ ] ADR included/updated where required.
- [ ] New dependency license/transitive/runtime impact reviewed.
- [ ] New unsafe/FFI boundary documented and tested.
- [ ] Not applicable.

## Risk

Potential data-loss, security, compatibility, memory or performance risks:

## Known limitations

State remaining limitations explicitly. Do not present TODO/stub behavior as complete.
