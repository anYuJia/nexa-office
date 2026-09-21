# Release Checklist

A release candidate is not publishable until every required item below is either passed or explicitly marked not applicable with rationale.

## Source and version

- [ ] release commit is on protected `main`;
- [ ] working tree is reproducible from a fresh checkout;
- [ ] Cargo workspace version matches the release tag;
- [ ] package metadata uses the same version;
- [ ] release notes describe user-visible changes and known limitations.

## Correctness

- [ ] Windows/macOS/Linux CI green;
- [ ] Interoperability Gate green;
- [ ] Docs corpus and performance gates green;
- [ ] Sheets performance gate green;
- [ ] Slides performance gate green;
- [ ] Native performance and integration gates green;
- [ ] Release Hardening Gate green;
- [ ] no known P0 issue;
- [ ] every accepted P1 has an explicit release-blocking decision.

## Stability

- [ ] automated bounded soak smoke passes;
- [ ] 2+ hour interactive editing soak completed on at least one reference machine;
- [ ] repeated open/close and file-switch tests show a stable memory floor;
- [ ] recovery smoke verifies unsaved state can be reopened;
- [ ] installer install/uninstall smoke passes on each release platform.

## Compatibility

- [ ] representative DOCX/XLSX/PPTX corpus open/edit/save/reopen run completed;
- [ ] unsupported destructive cases remain blocked;
- [ ] support matrix and known limitations reviewed for accuracy;
- [ ] migration/compatibility notes updated.

## Security

- [ ] dependency inventory generated;
- [ ] untrusted-file regression tests green;
- [ ] security reporting instructions valid;
- [ ] no secrets are embedded in artifacts;
- [ ] release artifacts are signed/notarized where required by the platform;
- [ ] artifact SHA-256 manifest generated and verified.

## Distribution

- [ ] Windows MSI and portable ZIP produced;
- [ ] macOS PKG and application ZIP produced;
- [ ] Linux DEB and portable TAR.GZ produced;
- [ ] package size stays inside the release budget;
- [ ] artifacts are uploaded from CI rather than rebuilt manually;
- [ ] checksums and build metadata accompany the release.

## 1.0-only review

- [ ] final license/contribution model selected;
- [ ] stable document contracts approved;
- [ ] published support matrix reviewed;
- [ ] security response process exercised;
- [ ] reference-hardware performance results meet sustained targets;
- [ ] upgrade/rollback path rehearsed.
