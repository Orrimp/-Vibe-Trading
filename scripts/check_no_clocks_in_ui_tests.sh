#!/usr/bin/env bash
# scripts/check_no_clocks_in_ui_tests.sh
#
# ui-test-harness-bootstrap v0.1 (Q7 resolution / T4031).
#
# Defence-in-depth grep gate: forbids non-deterministic clock/RNG calls
# in the UI rendering paths reachable from `iced_test::screenshot`.
# Exits non-zero on any unwhitelisted match.
#
# Allowed via `// CLOCK-OK: <reason>` marker on the same line or the
# preceding line. The chart widget's `local_offset_or_utc()` test
# override at `crates/ui/src/widgets/chart.rs:125-160` is the canonical
# example.
#
# Why a shell grep instead of a cargo-deny ban?
#   The `local_offset_or_utc()` override INTENTIONALLY references
#   `UtcOffset::current_local_offset` in a `#[cfg(test)]`-only
#   pseudo-stub — see the comment block at `chart.rs:125`. A
#   cargo-deny ban on `std::time::SystemTime` would either let that
#   slip through (false-negative) or block it (false-positive).
#   This grep is precise: paths + tokens + per-line whitelist marker.
#
# Acceptance criteria (V4):
#   - Exits 0 on the clean tree.
#   - Exits non-zero when a `SystemTime::now()` is injected.
#   - Listed in `rust-validate`'s pre-test gate.

set -euo pipefail

# Files to scan — the rendering / Charts-screen paths reachable from
# `iced_test::screenshot(&ui::test_support::program_from_cockpit(...))`.
# Extending the watchlist is one line per file.
WATCHLIST=(
  "crates/ui/src/widgets/chart.rs"
  "crates/ui/src/widgets/canvas_chart.rs"
  "crates/ui/src/screens/charts.rs"
  "crates/ui/src/test_support.rs"
  "crates/ui/tests/visual_snapshots.rs"
  "crates/ui/tests/chart_hover_grid_sweep.rs"
  "crates/ui/tests/fixtures/mod.rs"
  "crates/ui/tests/fixtures/visual_diff.rs"
)

# Forbidden patterns. Whitespace + capture covers most call shapes
# (`time::SystemTime::now()`, `std::time::Instant::now()`, etc.).
PATTERNS=(
  'SystemTime::now'
  'Instant::now'
  'thread_rng'
  'UtcOffset::current_local_offset'
)

WORKSPACE_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

failed=0
for file in "${WATCHLIST[@]}"; do
  abs="${WORKSPACE_ROOT}/${file}"
  if [[ ! -f "$abs" ]]; then
    # File doesn't exist yet — skip (some tests added incrementally).
    continue
  fi
  for pat in "${PATTERNS[@]}"; do
    # `grep -n` for each pattern.  Filter out lines AND the line
    # immediately preceding that carry the whitelist marker.
    while IFS=: read -r lineno match; do
      [[ -z "$lineno" ]] && continue
      this_line="$match"
      prev_line=""
      if (( lineno > 1 )); then
        prev_line="$(sed -n "${lineno}p" "$abs")"  # placeholder; below sed -n on lineno - 1
        prev_lineno=$((lineno - 1))
        prev_line="$(sed -n "${prev_lineno}p" "$abs")"
      fi
      if [[ "$this_line" == *"// CLOCK-OK:"* ]] || [[ "$prev_line" == *"// CLOCK-OK:"* ]]; then
        continue
      fi
      echo "FAIL  ${file}:${lineno} — unwhitelisted '${pat}'"
      echo "      ${this_line}"
      failed=1
    done < <(grep -n -E "${pat}" "$abs" || true)
  done
done

if (( failed != 0 )); then
  echo ""
  echo "Unwhitelisted clock/RNG calls reachable from the snapshot path."
  echo "Add a '// CLOCK-OK: <reason>' marker on the same line or"
  echo "the preceding line if the use is intentional."
  exit 1
fi

echo "CLOCKS PASS  (${#WATCHLIST[@]} files / ${#PATTERNS[@]} patterns)"
exit 0
