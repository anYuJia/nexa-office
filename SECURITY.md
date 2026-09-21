# Security Policy

Nexa Office processes complex files from untrusted sources. DOCX, XLSX and PPTX packages, ZIP entries, XML, images, fonts, hyperlinks and embedded metadata MUST be treated as untrusted input.

## Security principles

- Parsing must be memory-safe and bounded.
- File-controlled lengths/counts must not cause unchecked allocation.
- ZIP extraction must prevent path traversal.
- Decompression limits must prevent decompression bombs.
- XML parsing must not enable external entity/network resolution.
- Embedded relationships/links must not silently execute programs or access the network.
- Temporary files must use safe locations/names and correct permissions.
- Save/export must never overwrite the last valid file before successful completion.
- Unsafe Rust and FFI must follow AGENTS.md and the engineering gates.

## Threat classes to test

At minimum:

- malformed ZIP central directories;
- duplicate/conflicting ZIP entries;
- path traversal names;
- extreme compression ratios;
- oversized XML nodes/attributes/counts;
- deeply nested structures;
- integer overflow in dimensions/offsets;
- malformed relationship targets;
- malicious image/font metadata;
- external links and embedded objects;
- formula/resource abuse;
- repeated open/close resource exhaustion.

## Reporting

For serious unpublished vulnerabilities, avoid posting exploit details or private documents in a public issue.

Prefer GitHub's private vulnerability-reporting / Security Advisory flow when it is available for this repository. If that flow is unavailable, contact the repository owner through an appropriate private GitHub channel. Do not attach private Office documents unless they have been minimized and sanitized.

A vulnerability fix should include a minimized regression fixture/test when it can be shared safely.

## Supported versions

During pre-1.0 development, the current `main` branch is the primary supported security line. Once public prereleases exist, the newest prerelease may receive critical fixes when a safe backport is practical.

Old development snapshots and superseded prereleases are not promised security maintenance. The support policy is reviewed at each formal release using `docs/RELEASE_CHECKLIST.md`.
