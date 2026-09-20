# Engineering Gates

These gates define what may enter main and what may ship.

## Branch policy

Once CI exists:

- main is always expected to be buildable.
- Feature work uses short-lived branches.
- Direct pushes to main should be disabled except emergency repository administration.
- Merge should use pull requests.
- Required status checks must pass before merge.
- Force-push to main should be disabled.
- Deleted/renamed tests and benchmark thresholds receive explicit review.

Until branch protection is technically configured, contributors and AI agents MUST behave as though these rules are already enforced.

## Required PR gates

A normal code PR must pass:

1. formatting;
2. compilation;
3. Clippy/lint policy;
4. unit tests;
5. relevant integration tests;
6. documentation checks where applicable;
7. dependency/license checks once configured;
8. performance regression checks for affected hot paths.

Parser/serializer changes additionally require fixture or round-trip tests.

Unsafe/FFI changes additionally require safety review.

Foundational dependency/architecture changes additionally require an ADR.

## Definition of Done

A task is not Done merely because code compiles.

Done means:

- implementation is complete for the stated scope;
- no known placeholder is presented as production functionality;
- tests cover the behavior;
- error paths are handled;
- docs/contracts are updated if changed;
- performance-sensitive code was measured;
- no gate is knowingly failing;
- known limitations are stated.

## Severity model

### P0 — Data loss / security / destructive corruption

Examples:

- saving destroys valid source content;
- path traversal or arbitrary write;
- decompression bomb causes uncontrolled allocation;
- unsafe memory corruption.

Rules:

- blocks release;
- blocks merge unless the PR is the fix;
- regression test required.

### P1 — Major correctness failure

Examples:

- common document cannot open;
- edit/save/reopen changes supported content incorrectly;
- formula result wrong;
- severe layout break in supported feature.

Rules:

- blocks milestone/release;
- regression test required.

### P2 — Functional or performance regression

Examples:

- supported shortcut broken;
- scrolling visibly regresses;
- memory exceeds budget;
- cold startup regresses beyond allowed threshold.

Rules:

- normally blocks merge if introduced by current change;
- may be tracked only when baseline already contains the issue and scope is explicit.

### P3 — polish / low-impact issue

May be tracked without blocking merge unless it violates accessibility or a documented contract.

## Performance gate

Performance-sensitive PRs compare against a pinned baseline.

Default regression policy:

- memory: no >5% regression on a stable benchmark without explicit justification;
- latency: no >10% regression on a stable benchmark without explicit justification;
- package/binary size: no >5% regression from a foundational dependency or asset change without review.

Absolute budgets in docs/PERFORMANCE.md still apply. Passing a relative comparison does not excuse exceeding an absolute release budget.

## Compatibility gate

For DOCX/XLSX/PPTX:

- opening MUST NOT rewrite the source file;
- save failures MUST preserve the previous valid file;
- supported content MUST survive open-edit-save-reopen;
- unrelated unknown package parts SHOULD survive round-trip;
- destructive losses MUST be classified and either prevented or explicitly blocked.

A screenshot alone is not proof of package correctness.

## Test gate

A new bug fix requires a regression test unless a deterministic automated test is genuinely impractical.

Flaky tests are defects.

A flaky test MUST be fixed, quarantined with a tracked issue and owner, or removed only if the behavior is no longer part of the product contract.

## Dependency gate

Production dependency changes require review of:

- purpose;
- alternatives;
- license;
- maintenance health;
- transitive dependencies;
- unsafe/FFI use;
- binary-size impact;
- startup/memory impact when relevant.

Do not merge a foundational runtime dependency solely because integration is convenient.

## Unsafe gate

New unsafe code requires:

- narrow boundary;
- SAFETY invariant;
- tests;
- rationale;
- Miri/sanitizer coverage where applicable.

Unsafe added for performance requires benchmark evidence.

## Release gate

A release candidate must pass:

- clean build from fresh checkout;
- full automated test suite;
- release-profile lint/check;
- representative interoperability corpus;
- startup and memory benchmark suite;
- open/edit/save benchmark suite;
- installer/package validation;
- malware/signing/notarization steps where platform requires;
- crash/recovery smoke tests;
- license/notice generation;
- release notes with known limitations.

No release is cut with a known P0.

## Emergency fixes

Emergency fixes may use an expedited review, but MUST NOT skip:

- compile/check;
- focused regression test;
- data-integrity validation;
- follow-up full CI.

An emergency is not permission to permanently bypass the engineering contract.
