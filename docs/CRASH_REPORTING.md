# Crash Reporting Strategy

Nexa Office does not require background telemetry or automatic crash upload.

## Default behavior

- crashes do not silently transmit documents, paths or user content;
- recovery snapshots stay local in the application-data directory;
- diagnostic output is local unless the user explicitly chooses to share it;
- no document body is included in a crash report by default.

## User-assisted reports

A useful report may include:

- Nexa version and commit/build identifier;
- operating system and architecture;
- steps to reproduce;
- sanitized error output;
- a minimized redistributable fixture when safe to share.

Users should not upload private Office documents merely to demonstrate a crash.

## Future opt-in reporting

Any future automated crash reporting must be:

- opt-in or explicitly enabled by the distributor;
- documented;
- data-minimized;
- transparent about endpoint and retention;
- capable of excluding document content and full local paths.

Mandatory telemetry is outside the current product contract.
