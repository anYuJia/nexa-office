#!/usr/bin/env bash
set -euo pipefail

binary="${1:-target/release/nexa-office}"
settle_seconds="${SETTLE_SECONDS:-30}"

if [[ ! -x "$binary" ]]; then
  echo "Binary not found or not executable: $binary" >&2
  echo "Run: cargo build --release --locked -p nexa-app" >&2
  exit 2
fi

"$binary" &
pid=$!
trap 'kill "$pid" 2>/dev/null || true' EXIT

sleep "$settle_seconds"

if [[ ! -r "/proc/$pid/smaps_rollup" ]]; then
  echo "smaps_rollup unavailable for PID $pid" >&2
  exit 3
fi

echo "pid=$pid"
grep -E '^(Rss|Pss|Private_Clean|Private_Dirty):' "/proc/$pid/smaps_rollup"
ps -p "$pid" -o pid=,pcpu=,rss=,etime=,comm=

echo "Note: repeat samples and use PSS/private metrics for the recorded baseline."
