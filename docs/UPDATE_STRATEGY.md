# Update Strategy

Nexa Office does not ship a mandatory background updater.

## Principles

- updates must never require an always-running background service;
- Office files and user settings remain usable when an update check is unavailable;
- update metadata must be fetched only on explicit user action or a future opt-in setting;
- binaries must be signed/notarized by the release environment before distribution;
- update packages must be validated before replacing an installed binary;
- rollback must remain possible by reinstalling the previous signed package.

## Platform delivery

- Windows: signed MSI or equivalent native package.
- macOS: signed and notarized application package.
- Linux: distribution package where available plus a portable archive.

The checked-in package builders are reproducible inputs. Signing keys, Apple credentials, certificates and store credentials are never committed.

## Version policy

Until the project reaches its published 1.0 support matrix, releases use pre-1.0 semantic versions. A release may not be called stable merely because a package can be produced.
