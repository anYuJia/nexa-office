# Migration and Compatibility Notes

Nexa Office currently uses local, file-based settings and recovery data. No cloud account or server-side migration is required.

## Application settings

Settings are stored per-user in the platform application-data directory. Unknown future settings should be ignored rather than making older versions unusable.

A release must not require deleting user settings merely to start successfully. When a settings schema becomes incompatible, migrate explicitly or fall back safely to defaults while preserving the previous file for recovery.

## Recent files

Recent-file entries are paths only. Missing files are tolerated; they do not block startup.

## Recovery files

Recovery snapshots are not a durable archival format. A new version should either:

- open a compatible recovery snapshot using the same document engine contract; or
- leave an incompatible snapshot untouched and explain that it cannot be restored automatically.

A release must not silently delete an unknown recovery snapshot before successful recovery or an explicit user decision.

## Office documents

The primary compatibility boundary is the Office file itself, not an internal Nexa project format.

Upgrade testing therefore focuses on:

1. open a representative DOCX/XLSX/PPTX with the previous release;
2. save a controlled edit;
3. open the result with the candidate release;
4. save/reopen again;
5. verify supported semantics and preserved opaque package parts.

## Rollback

Because Nexa uses standard OOXML files, rollback should remain possible by reinstalling a previous signed package. Files saved by a newer version may contain Office structures that an older Nexa version does not semantically edit; the older version must preserve or block unsafe rewrites according to its compatibility policy.
