#!/usr/bin/env bash
# bug-log #111/#144/#95/#146 — regenerate the non-theta anchored scenarios whose lanes changed.
#
# Companion to run_surfaces.sh (which owns the 34 theta-surfaces). This script owns the 19
# scenarios produced by the `backtest` bin and its siblings.
#
# ── Why this exists ───────────────────────────────────────────────────────────
#
#   1. The DEFAULT report directory resolves INTO the anchored corpus. Every invocation here
#      passes --reports-dir explicitly, and the script refuses an out-dir inside evidence/.
#      That flag exists for exactly this reason ("re-running into a tempdir without touching
#      the anchored reports under evidence/").
#   2. Affectedness is MEASURED, not assumed. Some of these 19 come from other producers
#      (monte_carlo, forecast_distribution, sharpe_comparison) and may be untouched by the
#      lane changes. A scenario whose regenerated body is byte-identical to its anchored one
#      needs no re-lock, and the only honest way to know is to run it and diff.
#   3. Resumable: a scenario whose .md already exists in the out-dir is skipped, so a kill
#      costs only the in-flight run.
#
# ── Usage ─────────────────────────────────────────────────────────────────────
#
#   scripts/relock/run_lane_scenarios.sh --out-dir <dir> [--dry-run]
#
# Stop any time with Ctrl-C; re-run the same command to continue.
set -euo pipefail

# ── Host-toolchain workaround (bug-log #147) ──────────────────────────────────
#
# The Command Line Tools shipped a MacOSX27.0.sdk whose
# System/Library/Frameworks/Security.framework/Security.tbd is MALFORMED:
#   ld: tapi error: malformed file ... Security.tbd
# Nothing that links `-framework Security` builds against it — which is every binary in
# this workspace, in debug AND release, including ones that built fine on 2026-10-11.
#
# Pinning SDKROOT to the previous SDK links cleanly. Done here rather than by changing the
# host's `xcode-select`, because a re-lock driver has no business mutating the operator's
# toolchain. Override by setting SDKROOT yourself; remove this block once the CLT ships a
# sound 27.x SDK.
if [ -z "${SDKROOT:-}" ]; then
  for _sdk in /Library/Developer/CommandLineTools/SDKs/MacOSX26.5.sdk \
              /Library/Developer/CommandLineTools/SDKs/MacOSX26.sdk; do
    if [ -d "$_sdk" ]; then
      export SDKROOT="$_sdk"
      echo "SDKROOT pinned to $(basename "$_sdk") (bug-log #147: the 27.0 SDK's Security.tbd is malformed)"
      break
    fi
  done
fi


OUT=""; DRY=0
while [ $# -gt 0 ]; do
  case "$1" in
    --out-dir) OUT="${2:-}"; shift 2 ;;
    --dry-run) DRY=1; shift ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done
[ -n "$OUT" ] || { echo "--out-dir is REQUIRED (the default resolves into evidence/)" >&2; exit 2; }
case "$(cd "$(dirname "$OUT")" 2>/dev/null && pwd)/$(basename "$OUT")" in
  */evidence/*) echo "REFUSING: out-dir resolves inside evidence/" >&2; exit 2 ;;
esac
mkdir -p "$OUT"

SCENARIOS=(
  top10-2023-1h-momentum
  top10-2024-h1-momentum
  top10-2023-fy-momentum-realdata
  pairs-2023-zscore-mr
  pairs-2024-h1-zscore-mr
  top10-2023-fy-tcn-overlay
  top10-2024-fy-tcn-overlay
  top10-2023-fy-tcn-overlay-realdata
  top10-2024-fy-tcn-overlay-realdata
  top10-2023-fy-tcn-overlay-weights
  top10-2024-fy-tcn-overlay-weights
  top10-2023-fy-tcn-overlay-weights-realdata
  top10-2024-fy-tcn-overlay-weights-realdata
  top10-2023-fy-patchtst-overlay-realdata
  top10-2023-fy-vol-target-overlay-realdata
)

BIN=./target/release/backtest
echo "ensuring $BIN is current (release, features candle,realdata) …"
cargo build --release -p backtest --features candle,realdata --bin backtest

echo "scenarios: ${#SCENARIOS[@]} · out: $OUT"
echo "resumable — Ctrl-C any time, re-run the same command to continue"
echo
ran=0; skipped=0; failed=0
for i in "${!SCENARIOS[@]}"; do
  s="${SCENARIOS[$i]}"
  n=$((i+1))
  if ls "$OUT"/*-"$s".md >/dev/null 2>&1; then
    printf '[%2d/%2d] SKIP  %s\n' "$n" "${#SCENARIOS[@]}" "$s"; skipped=$((skipped+1)); continue
  fi
  if [ "$DRY" = 1 ]; then
    printf '[%2d/%2d] DRY   %s --scenario %s --seed 0xC0FFEE --reports-dir %s\n' "$n" "${#SCENARIOS[@]}" "$BIN" "$s" "$OUT"; continue
  fi
  printf '[%2d/%2d] RUN   %s\n' "$n" "${#SCENARIOS[@]}" "$s"
  start=$(date +%s)
  if nice -n 19 "$BIN" --scenario "$s" --seed 0xC0FFEE --reports-dir "$OUT" >"$OUT/$s.log" 2>&1; then
    printf '        done in %s min\n' "$(( ($(date +%s) - start) / 60 ))"; ran=$((ran+1))
  else
    printf '        FAILED (see %s/%s.log) — continuing\n' "$OUT" "$s"; failed=$((failed+1))
  fi
done
echo
echo "ran $ran · skipped $skipped · failed $failed · of ${#SCENARIOS[@]}"
