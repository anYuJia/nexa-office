#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
from pathlib import Path

SEMVER = re.compile(
    r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)"
    r"(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?"
    r"(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$"
)


def workspace_version() -> str:
    cargo = Path("Cargo.toml").read_text(encoding="utf-8")
    match = re.search(r'(?m)^version\s*=\s*"([^"]+)"', cargo)
    if not match:
        raise RuntimeError("workspace version not found")
    return match.group(1)


def main() -> int:
    if len(sys.argv) == 2 and sys.argv[1] == "--workspace":
        try:
            print(workspace_version())
        except RuntimeError as error:
            print(error, file=sys.stderr)
            return 2
        return 0

    if len(sys.argv) != 2:
        print("usage: verify_version.py <release-version> | --workspace", file=sys.stderr)
        return 2

    expected = sys.argv[1].removeprefix("v")
    if not SEMVER.fullmatch(expected):
        print(f"invalid semantic version: {expected}", file=sys.stderr)
        return 2

    try:
        actual = workspace_version()
    except RuntimeError as error:
        print(error, file=sys.stderr)
        return 2

    valid = expected == actual or expected.startswith(f"{actual}-")
    if not valid:
        print(
            f"version mismatch: release={expected} workspace={actual}; "
            "a prerelease must use the workspace version as its base",
            file=sys.stderr,
        )
        return 1

    print(f"release_version={expected}")
    print(f"workspace_version={actual}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
