#!/usr/bin/env python3
from __future__ import annotations

import os
import signal
import subprocess
import sys
import time
from pathlib import Path


WARMUP_SECONDS = int(os.environ.get("NEXA_SOAK_WARMUP_SECONDS", "15"))
SAMPLE_SECONDS = int(os.environ.get("NEXA_SOAK_SAMPLE_SECONDS", "60"))
MAX_GROWTH_MIB = float(os.environ.get("NEXA_SOAK_MAX_GROWTH_MIB", "25"))


def rss_bytes(pid: int) -> int:
    status = Path(f"/proc/{pid}/status").read_text(encoding="utf-8")
    for line in status.splitlines():
        if line.startswith("VmRSS:"):
            return int(line.split()[1]) * 1024
    raise RuntimeError("VmRSS not found")


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: long_session_smoke.py <nexa-office-binary>", file=sys.stderr)
        return 2

    binary = Path(sys.argv[1]).resolve()
    if not binary.is_file():
        print(f"missing binary: {binary}", file=sys.stderr)
        return 2

    process = subprocess.Popen([str(binary)])
    samples: list[int] = []
    try:
        deadline = time.monotonic() + WARMUP_SECONDS
        while time.monotonic() < deadline:
            if process.poll() is not None:
                print(f"application exited during warmup with {process.returncode}", file=sys.stderr)
                return 1
            time.sleep(1)

        start = rss_bytes(process.pid)
        deadline = time.monotonic() + SAMPLE_SECONDS
        while time.monotonic() < deadline:
            if process.poll() is not None:
                print(f"application exited during soak with {process.returncode}", file=sys.stderr)
                return 1
            samples.append(rss_bytes(process.pid))
            time.sleep(1)

        end = samples[-1]
        peak = max(samples)
        growth = max(0, end - start)
        growth_mib = growth / (1024 * 1024)
        print(f"start_rss_bytes={start}")
        print(f"end_rss_bytes={end}")
        print(f"peak_rss_bytes={peak}")
        print(f"growth_mib={growth_mib:.2f}")
        if growth_mib > MAX_GROWTH_MIB:
            print(
                f"RSS growth {growth_mib:.2f} MiB exceeds {MAX_GROWTH_MIB:.2f} MiB smoke gate",
                file=sys.stderr,
            )
            return 1
        return 0
    finally:
        if process.poll() is None:
            process.send_signal(signal.SIGTERM)
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)


if __name__ == "__main__":
    raise SystemExit(main())
