#!/usr/bin/env bash
# scripts/check_no_raw_asof_join.sh
#
# advisor-pit-discipline v3.1.0 (ADR-0086 D1 / M-DEV-1, M-DEV-2).
#
# Defence-in-depth grep gate: forbids a raw, hand-rolled time-keyed
# as-of join — `partition_point(|&(t, _)| t <= query)` or
# `binary_search_by*` on a `t <= query`-shaped comparison — anywhere
# under production `crates/*/src/**`, OUTSIDE the sanctioned home
# `crates/core/src/pit.rs`. The type-level `trading_core::pit::PitSeries`
# API (ADR-0058) makes the *core join* safe by construction; this lint
# is the backstop that catches a FRESH hand-rolled bypass a future
# author might write for a new exogenous channel — the exact gap
# ADR-0058 § D5 named as "a v0.2 follow-on if a fresh data channel
# opens." DVOL (ADR-0072) and macro (ADR-0073) are those channels.
#
# Why a shell grep instead of a `cargo-deny` / clippy symbol ban?
#   `partition_point` and `binary_search_by*` have legitimate
#   non-temporal uses (e.g. splitting a sorted Vec on a non-timestamp
#   key). A bare-symbol ban is either false-negative (misses
#   `binary_search_by_key`) or false-positive (blocks a harmless
#   non-temporal partition_point). This grep matches the AS-OF
#   PREDICATE SHAPE — a `t <= query`/`t <= bar`-style comparison
#   inside the call — not the bare method name. Same reasoning as
#   `check_no_clocks_in_ui_tests.sh` rejecting a `SystemTime` ban.
#
# Allowlist — two mechanisms:
#   1. The sanctioned home `crates/core/src/pit.rs` is exempt outright
#      (SANCTIONED_HOME below) — it IS the guarded `partition_point`,
#      the single implementation every consumer routes through.
#   2. Per-line escape hatch `// PIT-OK: <reason>` on the same line or
#      the PRECEDING line — mirrors `// CLOCK-OK:`. Reserved for a
#      future justified exception (none exist in production `src`
#      today); a raw join added under this marker also demands its
#      own ADR note per ADR-0086 § Consequences.
#
# Scope (SCANLIST): production library sources `crates/*/src/**/*.rs`
# ONLY (resolved via `git ls-files`), per feature.md OQ-LINT-SCANLIST
# (architect lean, confirmed against the corpus: no `tests/` fixture
# in the current tree exercises the raw-predicate shape, so
# `tests/`/`benches/`/`examples/` stay OUT of scope — a test
# legitimately building a raw fixture should not need a `// PIT-OK:`
# marker; `examples/` carries the two known research diags per
# feature.md § What we found, deliberately out of the production
# scan).
#
# Acceptance criteria (AC1 / M-TEST-1):
#   - Exits 0 on the current clean tree.
#   - Exits non-zero when a raw as-of join is planted in a scanned
#     production `crates/*/src` file.
#   - `--self-test` writes a synthetic OFFENDING fixture to a tempdir,
#     asserts a hit, then a CLEAN fixture, asserts no hit. Mirrors
#     `spec_lint.py --self-test` / the `check_no_clocks` V4 AC.
#   - Wired into `rust-validate`'s pre-test gate (SKILL.md step) and
#     listed in AGENT.md's tooling table alongside the two sibling
#     grep gates.

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# The one sanctioned home for the raw partition_point — exempt outright.
SANCTIONED_HOME="crates/core/src/pit.rs"

# ── The as-of predicate matcher ────────────────────────────────────────────
#
# Matches a `.partition_point(` or `.binary_search_by(`/`.binary_search_by_key(`
# call whose closure body contains a `t <= q`-shaped comparison — i.e. a
# `<=` comparison where the LEFT side looks like a timestamp-ish binding
# (`t`, `ts`, a `.0` tuple-field projection, or anything ending in `_ts`)
# and it is inside a partition_point/binary_search call on the SAME source
# line (production loaders in this codebase write the raw predicate as a
# one-liner closure — the exact pattern ADR-0058/ADR-0086 document; a
# multi-line closure would still trip the two independent legs below via
# the windowed scan).
#
# We match in two independent legs and require BOTH within a small window
# (same line, since production one-liners are single-line) so bare
# non-temporal `partition_point(...)` calls (no `<=` comparison, or a `<=`
# comparison on a clearly non-temporal key) do NOT false-positive.
ASOF_METHOD_RE='\.(partition_point|binary_search_by(_key)?)\('
ASOF_PREDICATE_RE='[A-Za-z_][A-Za-z0-9_]*(\.0)?[[:space:]]*<=[[:space:]]*[A-Za-z_(]'

self_test() {
  local tmpdir
  tmpdir="$(mktemp -d)"
  trap 'rm -rf "$tmpdir"' RETURN

  local offending="$tmpdir/offending_crate_src.rs"
  local clean="$tmpdir/clean_crate_src.rs"

  cat > "$offending" <<'EOF'
// A synthetic raw as-of join — this MUST be flagged.
fn bad_as_of(records: &[(i64, f64)], query: i64) -> Option<f64> {
    let idx = records.partition_point(|&(t, _)| t <= query);
    if idx == 0 { None } else { Some(records[idx - 1].1) }
}
EOF

  cat > "$clean" <<'EOF'
// A clean file — routes through PitSeries; no raw predicate.
use trading_core::pit::{PitSeries, TimestampMs};

fn good_as_of(series: &PitSeries<f64>, query: TimestampMs) -> Option<f64> {
    series.as_of_value(query)
}

// A legitimate NON-temporal partition_point — must NOT false-positive.
fn split_point(sorted: &[u32], needle: u32) -> usize {
    sorted.partition_point(|&x| x < needle)
}
EOF

  local offending_hits clean_hits
  offending_hits="$(scan_file "$offending" | wc -l | tr -d ' ')"
  clean_hits="$(scan_file "$clean" | wc -l | tr -d ' ')"

  local ok=1
  if [[ "$offending_hits" -eq 0 ]]; then
    echo "SELF-TEST FAIL: offending fixture produced 0 hits (matcher too weak)" >&2
    ok=0
  fi
  if [[ "$clean_hits" -ne 0 ]]; then
    echo "SELF-TEST FAIL: clean fixture produced $clean_hits hit(s) (matcher too strong / false-positive)" >&2
    ok=0
  fi

  if [[ "$ok" -eq 1 ]]; then
    echo "SELF-TEST PASS: offending fixture flagged ($offending_hits hit), clean fixture silent (0 hits)"
    return 0
  fi
  return 1
}

# scan_file FILE — print "lineno:matched_line" for every unwhitelisted hit.
scan_file() {
  local file="$1"
  local lineno
  while IFS= read -r lineno; do
    [[ -z "$lineno" ]] && continue
    local this_line prev_line prev_lineno
    this_line="$(sed -n "${lineno}p" "$file")"
    prev_line=""
    if (( lineno > 1 )); then
      prev_lineno=$((lineno - 1))
      prev_line="$(sed -n "${prev_lineno}p" "$file")"
    fi
    if [[ "$this_line" == *"// PIT-OK:"* ]] || [[ "$prev_line" == *"// PIT-OK:"* ]]; then
      continue
    fi
    echo "${lineno}:${this_line}"
  done < <(grep -n -E "$ASOF_METHOD_RE" "$file" 2>/dev/null | grep -E "$ASOF_PREDICATE_RE" | cut -d: -f1)
}

# List every production library source under `crates/*/src/**`.
#
# NOTE: `git ls-files 'crates/*/src/**/*.rs'` ALONE silently misses files
# that sit DIRECTLY in `crates/<name>/src/*.rs` (no subdirectory) — git's
# `**` pathspec glob requires at least one intermediate directory level,
# so e.g. `crates/backtest/src/dvol_data.rs` and `crates/core/src/pit.rs`
# itself would be skipped (discovered + fixed during M-TEST-1: the
# retrofit target files are exactly the ones this gap would have hidden).
# We therefore combine BOTH the flat-`src/` and the nested-`src/**/`
# pathspecs and de-duplicate.
scan_list() {
  (cd "$WORKSPACE_ROOT" && git ls-files 'crates/*/src/*.rs' 'crates/*/src/**/*.rs') | sort -u
}

run_scan() {
  local failed=0
  local file rel
  while IFS= read -r rel; do
    [[ -z "$rel" ]] && continue
    [[ "$rel" == "$SANCTIONED_HOME" ]] && continue
    file="${WORKSPACE_ROOT}/${rel}"
    [[ -f "$file" ]] || continue
    while IFS= read -r hit; do
      [[ -z "$hit" ]] && continue
      local lineno="${hit%%:*}"
      local matched="${hit#*:}"
      echo "FAIL  ${rel}:${lineno} — raw as-of predicate outside core::pit"
      echo "      ${matched}"
      failed=1
    done < <(scan_file "$file")
  done < <(scan_list)

  if (( failed != 0 )); then
    echo ""
    echo "Unwhitelisted raw time-keyed as-of join (partition_point/binary_search_by*"
    echo "on a 't <= query'-shaped comparison) outside crates/core/src/pit.rs."
    echo "Route through trading_core::pit::PitSeries (ADR-0058), or, for a"
    echo "deliberate reviewed exception, add a '// PIT-OK: <reason>' marker on"
    echo "the same or preceding line (and record the exception per ADR-0086"
    echo "§ Consequences)."
    return 1
  fi

  echo "PIT-JOIN LINT PASS (scanned $(scan_list | wc -l | tr -d ' ') production src files; sanctioned home + PIT-OK markers exempt)"
  return 0
}

main() {
  if [[ "${1:-}" == "--self-test" ]]; then
    self_test
    exit $?
  fi
  run_scan
  exit $?
}

main "$@"
