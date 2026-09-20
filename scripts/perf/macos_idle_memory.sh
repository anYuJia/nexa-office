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

echo "pid=$pid"
ps -p "$pid" -o pid=,pcpu=,rss=,etime=,comm=

if command -v footprint >/dev/null 2>&1; then
  echo
  echo "macOS footprint summary:"
  footprint "$pid" 2>/dev/null | head -80 || true
fi

echo "Record the exact macOS version and metric used with the baseline."
