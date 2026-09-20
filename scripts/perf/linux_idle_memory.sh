#!/usr/bin/env bash
set -euo pipefail

binary="${1:-target/release/nexa-office}"
settle_seconds="${SETTLE_SECONDS:-30}"
cpu_sample_seconds="${CPU_SAMPLE_SECONDS:-10}"

if [[ ! -x "$binary" ]]; then
  echo "Binary not found or not executable: $binary" >&2
  echo "Run: cargo build --release --locked -p nexa-app" >&2
  exit 2
fi

"$binary" &
pid=$!
trap 'kill "$pid" 2>/dev/null || true' EXIT

sleep "$settle_seconds"

if [[ ! -r "/proc/$pid/smaps_rollup" || ! -r "/proc/$pid/stat" ]]; then
  echo "proc metrics unavailable for PID $pid" >&2
  exit 3
fi

echo "pid=$pid"
echo "settle_seconds=$settle_seconds"

awk '
  /^(Rss|Pss|Private_Clean|Private_Dirty):/ { print }
  /^Private_Clean:/ { private_clean=$2 }
  /^Private_Dirty:/ { private_dirty=$2 }
  END { printf "Private_Total:    %8d kB\n", private_clean + private_dirty }
' "/proc/$pid/smaps_rollup"

clock_ticks="$(getconf CLK_TCK)"
cpu_start="$(awk '{ print $14 + $15 }' "/proc/$pid/stat")"
sleep "$cpu_sample_seconds"
cpu_end="$(awk '{ print $14 + $15 }' "/proc/$pid/stat")"

idle_cpu_percent="$(awk -v start="$cpu_start" -v end="$cpu_end" -v hz="$clock_ticks" -v seconds="$cpu_sample_seconds"   'BEGIN { printf "%.3f", ((end - start) / hz / seconds) * 100 }')"

echo "idle_cpu_sample_seconds=$cpu_sample_seconds"
echo "idle_cpu_percent_one_core=$idle_cpu_percent"
ps -p "$pid" -o pid=,pcpu=,rss=,etime=,comm=

echo "Note: CI runners provide trend data only; reference-machine baselines require repeated dedicated runs."
