# Support Matrix

This matrix defines the current tested product contract. It is intentionally narrower than the full OOXML standards.

## Desktop platforms

| Platform | Architecture | CI/build status | Distribution baseline |
| --- | --- | --- | --- |
| Windows 11 | x86_64 | Required | MSI + portable ZIP |
| macOS | Apple Silicon | Required | PKG + application ZIP |
| Ubuntu Linux | x86_64 | Required | DEB + portable TAR.GZ |

Other operating systems or architectures may work but are not part of the current release gate until they have repeatable build, package and smoke coverage.

## File formats

| Format | Open | Edit | Save | Compatibility policy |
| --- | --- | --- | --- | --- |
| DOCX | Supported subset | Supported subset | Yes | Unsafe semantic rewrites are blocked |
| XLSX | Supported subset | Supported subset | Yes | Sparse model; unsupported destructive rewrites are blocked |
| PPTX | Supported subset | Supported subset | Yes | Unsupported destructive rewrites are blocked |
| DOC / XLS / PPT | No | No | No | Convert externally to OOXML first |
| ODT / ODS / ODP | No | No | No | Not in current product contract |
| PDF | No semantic editing | No | No | Printing/export foundation only |

## Language and text baseline

The shell is available in:

- System language mode;
- Simplified Chinese;
- English.

Document text regression coverage includes Latin, Simplified Chinese, emoji, Arabic and Hebrew at the save/reopen boundary. Advanced typography and complete font-fallback parity with Microsoft Office are not claimed.

## Network and telemetry

Nexa Office has no required browser runtime and no mandatory telemetry service. Core local editing, opening and saving are designed to work without a network connection.

## Compatibility rule

A file opening successfully does not imply that every feature inside it is editable. When Nexa cannot safely own a rewrite, the application preserves opaque package data where possible or blocks the destructive save path.
