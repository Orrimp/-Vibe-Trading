#!/usr/bin/env bash
# Wave B canonical re-emission script for v5 v0.3.0 full-path wiring.
# Runs 12 scenarios that are newly wired (Pairs, TCN, PatchTST, GARCH)
# + 2 Group A SMA scenarios with --force-synthetic-bars (Q1=(a)).
# All run under canonical config { latency_ms_min: 30, latency_ms_max: 80, slippage_bps: 8 }.
#
# Usage:
#   bash scripts/wave_b_emit.sh 2>&1 | tee /tmp/v5-v030-wave-b.log
#
# NOTE: Realdata scenarios require REVISION.toml pinned parquet cache.
# They will fail gracefully if the cache is not present.

set -euo pipefail

WORKSPACE="$(cd "$(dirname "$0")/.." && pwd)"
BINARY="$WORKSPACE/target/release/backtest"
REPORTS_DIR="$WORKSPACE/spec/v5-latency-slippage-sim-v0.3.0-full-path-wiring/reports"
SEED="0xC0FFEE"
SIM_FLAGS="--sim-latency-ms-min 30 --sim-latency-ms-max 80 --sim-slippage-bps 8"
LOG="/tmp/v5-v030-wave-b.log"

if [[ ! -f "$BINARY" ]]; then
  echo "ERROR: Binary not found at $BINARY. Run: cargo build -p backtest --release" >&2
  exit 1
fi

mkdir -p "$REPORTS_DIR"

run_scenario() {
  local scenario="$1"
  local extra_flags="${2:-}"
  echo ""
  echo "=== $(date +%H:%M:%S) Running: $scenario $extra_flags ==="
  "$BINARY" \
    --scenario "$scenario" \
    --seed "$SEED" \
    --reports-dir "$REPORTS_DIR" \
    $SIM_FLAGS \
    $extra_flags \
    2>&1 | tail -5
  echo "--- done: $scenario ---"
}

echo "=== Wave B canonical re-emission: $(date) ===" | tee -a "$LOG"
echo "Config: latency_ms_min=30, latency_ms_max=80, slippage_bps=8" | tee -a "$LOG"
echo "Reports dir: $REPORTS_DIR" | tee -a "$LOG"

# --- Group A: SMA scenarios with --force-synthetic-bars (Q1=(a)) ---
echo ""
echo "## Group A: SMA synthetic (--force-synthetic-bars)" | tee -a "$LOG"
run_scenario "btc-2023-1m-sma-cross"          "--force-synthetic-bars" 2>&1 | tee -a "$LOG"
run_scenario "btc-2023-1m-sma-baseline-refresh" "--force-synthetic-bars" 2>&1 | tee -a "$LOG"

# --- Group B: Composed strategies (momentum, SMA, composed -- already wired in v0.1.0) ---
echo ""
echo "## Group B: Composed strategies (synthetic)" | tee -a "$LOG"
run_scenario "btc-2023-1m-macd-trend"          "" 2>&1 | tee -a "$LOG"
run_scenario "btc-2023-1m-rsi-reversion"       "" 2>&1 | tee -a "$LOG"
run_scenario "btc-2023-1m-bbands-mean-revert"  "" 2>&1 | tee -a "$LOG"

# --- Group C: Momentum (cross-sectional, synthetic) ---
echo ""
echo "## Group C: Momentum scenarios (synthetic)" | tee -a "$LOG"
run_scenario "top10-2023-1h-momentum" "" 2>&1 | tee -a "$LOG"
run_scenario "top10-2024-h1-momentum" "" 2>&1 | tee -a "$LOG"

# --- Group D: Pairs (newly wired -- synthetic) ---
echo ""
echo "## Group D: Pairs scenarios (newly wired, synthetic)" | tee -a "$LOG"
run_scenario "pairs-2023-zscore-mr"    "" 2>&1 | tee -a "$LOG"
run_scenario "pairs-2024-h1-zscore-mr" "" 2>&1 | tee -a "$LOG"

# --- Group E: TCN overlay (synthetic, newly wired) ---
echo ""
echo "## Group E: TCN overlay (synthetic, newly wired)" | tee -a "$LOG"
run_scenario "top10-2023-fy-tcn-overlay"         "" 2>&1 | tee -a "$LOG"
run_scenario "top10-2024-fy-tcn-overlay"         "" 2>&1 | tee -a "$LOG"
run_scenario "top10-2023-fy-tcn-overlay-weights" "" 2>&1 | tee -a "$LOG"
run_scenario "top10-2024-fy-tcn-overlay-weights" "" 2>&1 | tee -a "$LOG"

echo ""
echo "=== Synthetic scenarios complete: $(date) ===" | tee -a "$LOG"
echo "=== Realdata scenarios below require --features realdata parquet cache ===" | tee -a "$LOG"

# --- Group F: TCN overlay realdata (newly wired) ---
echo ""
echo "## Group F: TCN overlay realdata (newly wired)" | tee -a "$LOG"
run_scenario "top10-2023-fy-tcn-overlay-realdata"         "--features realdata" 2>&1 | tee -a "$LOG" || echo "SKIP: realdata not available" | tee -a "$LOG"
run_scenario "top10-2024-fy-tcn-overlay-realdata"         "--features realdata" 2>&1 | tee -a "$LOG" || echo "SKIP: realdata not available" | tee -a "$LOG"
run_scenario "top10-2023-fy-tcn-overlay-weights-realdata" "--features realdata" 2>&1 | tee -a "$LOG" || echo "SKIP: realdata not available" | tee -a "$LOG"
run_scenario "top10-2024-fy-tcn-overlay-weights-realdata" "--features realdata" 2>&1 | tee -a "$LOG" || echo "SKIP: realdata not available" | tee -a "$LOG"

# --- Group G: PatchTST overlay realdata (newly wired) ---
echo ""
echo "## Group G: PatchTST overlay realdata (newly wired)" | tee -a "$LOG"
run_scenario "top10-2023-fy-patchtst-overlay-realdata" "--features realdata" 2>&1 | tee -a "$LOG" || echo "SKIP: realdata not available" | tee -a "$LOG"

# --- Group H: GARCH vol target realdata (newly wired) ---
echo ""
echo "## Group H: GARCH vol target realdata (newly wired)" | tee -a "$LOG"
run_scenario "top10-2023-fy-vol-target-overlay-realdata" "--features realdata" 2>&1 | tee -a "$LOG" || echo "SKIP: realdata not available" | tee -a "$LOG"

echo ""
echo "=== Wave B complete: $(date) ===" | tee -a "$LOG"
echo "Reports in: $REPORTS_DIR" | tee -a "$LOG"
ls -la "$REPORTS_DIR/" | tee -a "$LOG"
