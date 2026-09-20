#!/usr/bin/env bash
set -euo pipefail

binary="${1:-target/release/nexa-office}"
settle_seconds="${SETTLE_SECONDS:-30}"
cpu_sample_seconds="${CPU_SAMPLE_SECONDS:-10}"
max_private_kb="${MAX_PRIVATE_KB:-61440}"
max_idle_cpu_percent="${MAX_IDLE_CPU_PERCENT:-2.0}"

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

rss_kb="$(awk '/^Rss:/ { print $2 }' "/proc/$pid/smaps_rollup")"
pss_kb="$(awk '/^Pss:/ { print $2 }' "/proc/$pid/smaps_rollup")"
private_clean_kb="$(awk '/^Private_Clean:/ { print $2 }' "/proc/$pid/smaps_rollup")"
private_dirty_kb="$(awk '/^Private_Dirty:/ { print $2 }' "/proc/$pid/smaps_rollup")"
private_total_kb="$((private_clean_kb + private_dirty_kb))"

echo "pid=$pid"
echo "settle_seconds=$settle_seconds"
printf "Rss:              %8d kB\n" "$rss_kb"
printf "Pss:              %8d kB\n" "$pss_kb"
printf "Private_Clean:    %8d kB\n" "$private_clean_kb"
printf "Private_Dirty:    %8d kB\n" "$private_dirty_kb"
printf "Private_Total:    %8d kB\n" "$private_total_kb"

clock_ticks="$(getconf CLK_TCK)"
cpu_start="$(awk '{ print $14 + $15 }' "/proc/$pid/stat")"
sleep "$cpu_sample_seconds"
cpu_end="$(awk '{ print $14 + $15 }' "/proc/$pid/stat")"

idle_cpu_percent="$(awk -v start="$cpu_start" -v end="$cpu_end" -v hz="$clock_ticks" -v seconds="$cpu_sample_seconds" 'BEGIN { printf "%.3f", ((end - start) / hz / seconds) * 100 }')"

echo "idle_cpu_sample_seconds=$cpu_sample_seconds"
echo "idle_cpu_percent_one_core=$idle_cpu_percent"
ps -p "$pid" -o pid=,pcpu=,rss=,etime=,comm=

if (( private_total_kb > max_private_kb )); then
  echo "Gross memory regression: private memory ${private_total_kb} kB exceeds CI smoke limit ${max_private_kb} kB" >&2
  exit 4
fi

if ! awk -v actual="$idle_cpu_percent" -v limit="$max_idle_cpu_percent" 'BEGIN { exit !(actual <= limit) }'; then
  echo "Gross idle-CPU regression: ${idle_cpu_percent}% exceeds CI smoke limit ${max_idle_cpu_percent}%" >&2
  exit 5
fi

echo "CI smoke gates passed: private <= ${max_private_kb} kB and idle CPU <= ${max_idle_cpu_percent}%."
echo "Note: CI runners provide trend data only; reference-machine baselines require repeated dedicated runs."
