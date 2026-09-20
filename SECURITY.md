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

Until a dedicated private security advisory/contact process is configured, contact the repository owner through an appropriate private GitHub channel.

A vulnerability fix should include a minimized regression fixture/test when it can be shared safely.

## Supported versions

No production release exists yet. During pre-alpha development, only the current main branch receives security fixes.

This policy will be revised before the first public alpha.
