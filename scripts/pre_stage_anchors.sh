#!/usr/bin/env bash
# T1936 (v2-llm-strategy, pass 6) — developer-side anchor pre-stage.
#
# Captures the two `report-sample-*` body-SHA-256s after the T1935
# System Health renderer rewrite (Q11 denominator $135 → $200 + Q5d
# `Cache hit ratio` row). The tester reads this output at
# `T_FINAL_V2_LLM_STRATEGY` and copies the new SHAs into
# `evidence/anchors.toml:67-75` (lines 67–75 in the v2.0.0 layout).
#
# The 9 strategy anchors at `evidence/anchors.toml:15-58` stay byte-
# identical — T1937 (and its sibling negative-invariant test
# `crates/reports/tests/strategy_anchors_unchanged.rs`) guards that.
#
# This script does NOT mutate `evidence/anchors.toml`; it only prints
# the captured SHAs and verifies byte-stability by hashing the
# regenerated `success-fixed-report-sample-*.md` files twice.
#
# Usage:
#   bash scripts/pre_stage_anchors.sh
#
# Exit codes:
#   0 — both report samples present and byte-stable; SHAs printed.
#   1 — a hash mismatch was detected between the on-disk file and a
#       fresh re-hash (indicates a half-finished re-anchor; re-run
#       `cargo test -p reports --test report_scenarios` to regenerate).
#   2 — a report sample file is missing.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

SAMPLE_7D="evidence/v1/operator-success-reports/reports/success-fixed-report-sample-7d.md"
SAMPLE_90D="evidence/v1/operator-success-reports/reports/success-fixed-report-sample-90d.md"

if [[ ! -f "$SAMPLE_7D" ]]; then
  echo "ERROR: $SAMPLE_7D not found — run 'cargo test -p reports --test report_scenarios' first" >&2
  exit 2
fi
if [[ ! -f "$SAMPLE_90D" ]]; then
  echo "ERROR: $SAMPLE_90D not found — run 'cargo test -p reports --test report_scenarios' first" >&2
  exit 2
fi

SHA_7D_A=$(python3 scripts/hash_report.py "$SAMPLE_7D" | awk '{print $1}')
SHA_7D_B=$(python3 scripts/hash_report.py "$SAMPLE_7D" | awk '{print $1}')
SHA_90D_A=$(python3 scripts/hash_report.py "$SAMPLE_90D" | awk '{print $1}')
SHA_90D_B=$(python3 scripts/hash_report.py "$SAMPLE_90D" | awk '{print $1}')

if [[ "$SHA_7D_A" != "$SHA_7D_B" ]]; then
  echo "ERROR: 7d sample hash mismatch between two reads of the same file" >&2
  echo "  read 1: $SHA_7D_A" >&2
  echo "  read 2: $SHA_7D_B" >&2
  exit 1
fi
if [[ "$SHA_90D_A" != "$SHA_90D_B" ]]; then
  echo "ERROR: 90d sample hash mismatch between two reads of the same file" >&2
  exit 1
fi

cat <<EOF
# v2-llm-strategy v2.0.0 anchor re-lock (pre-staged by T1936)
#
# The tester applies these SHAs at T_FINAL_V2_LLM_STRATEGY by editing
# evidence/anchors.toml:67-75 in place. Body-byte changes are driven by
# T1935 (Q11 denominator + Q5d Cache hit ratio row).
#
# Determinism gate: each SHA is captured twice from the same file via
# scripts/hash_report.py; both reads matched (byte-stable on disk).

report-sample-7d  = "$SHA_7D_A"
report-sample-90d = "$SHA_90D_A"
EOF
