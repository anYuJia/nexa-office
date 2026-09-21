#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import os
import platform
import subprocess
import sys
from pathlib import Path


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def command_output(*args: str) -> str:
    return subprocess.check_output(args, text=True).strip()


def main() -> int:
    if len(sys.argv) < 3:
        print("usage: artifact_manifest.py <version> <artifact> [artifact...]", file=sys.stderr)
        return 2

    version = sys.argv[1]
    artifacts = [Path(value) for value in sys.argv[2:]]
    missing = [str(path) for path in artifacts if not path.is_file()]
    if missing:
        print("missing artifacts: " + ", ".join(missing), file=sys.stderr)
        return 2

    out = Path("target/package")
    out.mkdir(parents=True, exist_ok=True)

    commit = os.environ.get("GITHUB_SHA") or command_output("git", "rev-parse", "HEAD")
    metadata = {
        "version": version,
        "commit": commit,
        "platform": platform.platform(),
        "machine": platform.machine(),
        "rustc": command_output("rustc", "--version"),
        "cargo": command_output("cargo", "--version"),
        "artifacts": [
            {
                "name": path.name,
                "bytes": path.stat().st_size,
                "sha256": sha256(path),
            }
            for path in artifacts
        ],
    }

    metadata_path = out / "build-metadata.json"
    metadata_path.write_text(json.dumps(metadata, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    checksum_path = out / "SHA256SUMS"
    checksum_path.write_text(
        "".join(f"{item['sha256']}  {item['name']}\n" for item in metadata["artifacts"]),
        encoding="utf-8",
    )

    cargo_metadata = json.loads(command_output("cargo", "metadata", "--locked", "--format-version", "1"))
    dependencies = sorted(
        {
            (package["name"], package["version"], package.get("source") or "workspace")
            for package in cargo_metadata["packages"]
        }
    )
    inventory = {
        "commit": commit,
        "packages": [
            {"name": name, "version": version_value, "source": source}
            for name, version_value, source in dependencies
        ],
    }
    (out / "dependency-inventory.json").write_text(
        json.dumps(inventory, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )

    print(metadata_path)
    print(checksum_path)
    print(out / "dependency-inventory.json")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
