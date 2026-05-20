#!/usr/bin/env bash
# bench_idle_cpu.sh — Idle-CPU sampler for Phase D+ T-F6 / H3 gate.
#
# Usage:
#   bash scripts/bench_idle_cpu.sh <pid> [seconds=60]
#
# Samples the given PID's CPU% once per second using macOS `top` in
# non-interactive mode. Outputs N lines of "<wall-secs> <cpu_pct>" to stdout
# so the test report can compute the median.
#
# The tester invokes N=3 runs and reports median-of-medians:
#   for r in 1 2 3; do
#     bash scripts/bench_idle_cpu.sh "$cockpit_pid" 60 > /tmp/cpu_run_${r}.txt
#     median=$(awk '{print $2}' /tmp/cpu_run_${r}.txt | sort -n \
#              | awk 'BEGIN{c=0} {a[c++]=$1} END{print a[int(c/2)]}')
#     echo "run_${r}_median: $median"
#   done
#
# Acceptance gate: median(N=3) <= 13.6% (H3, per feature.md R4.2).
#
# Architect-locked (ui-rethink-phase-d-trail-followup T-D-N11 / decomp.md § 1.3).
# Tooling: macOS `top -l 1 -n 0 -pid <pid> -stats cpu` (Q4 resolution (a)).
set -euo pipefail

pid="${1:?Usage: bench_idle_cpu.sh <pid> [seconds=60]}"
secs="${2:-60}"

# Validate that the PID is alive before starting.
if ! kill -0 "$pid" 2>/dev/null; then
    echo "ERROR: PID $pid is not running" >&2
    exit 1
fi

for ((i=0; i<secs; i++)); do
    # macOS `top` one-shot:
    #   -l 1        = one sample (non-interactive)
    #   -n 0        = show 0 processes in the default process list
    #   -pid <pid>  = show only this PID
    #   -stats cpu  = single column — CPU%
    #
    # The header line ("CPU") and the data line (the actual %) are both
    # output; we grep for the PID line which starts with the numeric PID.
    cpu=$(top -l 1 -n 0 -pid "$pid" -stats "pid,cpu" 2>/dev/null \
          | awk -v pid="$pid" '$1 == pid { print $2; exit }')
    printf '%d %s\n' "$i" "${cpu:-0.0}"
    sleep 1
done
