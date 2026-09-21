#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
from pathlib import Path


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: verify_version.py <version>", file=sys.stderr)
        return 2

    expected = sys.argv[1].removeprefix("v")
    cargo = Path("Cargo.toml").read_text(encoding="utf-8")
    match = re.search(r'(?m)^version\s*=\s*"([^"]+)"', cargo)
    if not match:
        print("workspace version not found", file=sys.stderr)
        return 2
    actual = match.group(1)
    if actual != expected:
        print(f"version mismatch: tag={expected} workspace={actual}", file=sys.stderr)
        return 1
    print(f"version={actual}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
