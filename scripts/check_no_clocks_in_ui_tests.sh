#!/usr/bin/env bash
# scripts/check_no_clocks_in_ui_tests.sh
#
# ui-test-harness-bootstrap v0.1 (Q7 resolution / T4031).
#
# Defence-in-depth grep gate: forbids non-deterministic clock/RNG calls
# in the UI rendering paths reachable from `iced_test::screenshot`.
# Exits non-zero on any unwhitelisted match.
#
# Allowed via a `// CLOCK-OK: <reason>` marker on the same line, or anywhere
# in the contiguous comment block immediately above the line.
#
# Why a shell grep instead of a cargo-deny ban?
#   The `local_offset_or_utc()` override INTENTIONALLY references
#   `UtcOffset::current_local_offset` in a `#[cfg(test)]`-only pseudo-stub.
#   A cargo-deny ban on `std::time::SystemTime` would either let that slip
#   through (false-negative) or block it (false-positive). This grep is
#   precise: paths + tokens + per-line whitelist marker.
#
# ── REBUILT 2026-11-10 (bug-log `#190`) ───────────────────────────────────────
#
# It was RED, and nothing ran it. `rust-validate`'s SKILL.md lists it at step 1
# and no hook or CI step did, so the one place it surfaced was a skill people
# invoke by hand — which trains the reader to scroll past a red gate. Four
# defects, all of them in the shapes this repo's bug-log already names:
#
#   1. THE MARKER LOOKBACK WAS ONE LINE, and a reason long enough to WRAP
#      defeated it. `chart.rs:301`'s sanctioned production call carries
#      `// CLOCK-OK: production reads the OS-local offset; defensive` on :299
#      and `// UTC fallback if the lookup fails.` on :300 — so the immediately
#      preceding line is the continuation, which holds no marker, and the gate
#      failed the one call it was written to permit. The gate punished writing
#      a reason long enough to be useful. Now the lookback walks the contiguous
#      comment block and stops at the first line that is not a comment.
#   2. IT MATCHED PROSE. `chart.rs:251` and `:2289` are doc comments that
#      *mention* `current_local_offset()`; the gate read them as calls. Two of
#      its three failures were sentences. Comment-only lines are now skipped —
#      the subject is calls, and a line whose first non-space characters are
#      `//`, `/*` or `*` is not one.
#   3. A MISSING WATCHLIST FILE WAS SILENTLY SKIPPED (`continue`, with a
#      comment saying some tests are added incrementally). Measured:
#      `crates/ui/src/screens/charts.rs` was DELETED in `f3cb7f92`
#      (ui-rethink-phase-a-lab Wave 1) and the entry had been dead ever since,
#      so the gate scanned 7 files while reporting 8. A listed file that is
#      gone is now a FAILURE, because a rename is a decision and this gate is
#      the thing that should ask about it. The dead entry is removed here.
#   4. THE PASS LINE PRINTED THE ARRAY LENGTH, not what was scanned —
#      `(${#WATCHLIST[@]} files / ${#PATTERNS[@]} patterns)` is a count the
#      gate never measured, which is a named sub-shape of the cannot-fail rule.
#      It now prints files actually read, lines examined, and exemptions
#      honoured, so the success path is observable.
#
# NOT DONE HERE, and priced rather than left vague: the watchlist is still a
# hand-written list, which is the `#188` defect one level up. Widening it to a
# WALK of `crates/ui/src/widgets/` + `crates/ui/src/screens/` was measured —
# **5 non-comment hits in 5 files**, so the widening is cheap — but each hit
# needs a marker whose reason is TRUE, and only four of the five could be
# verified in that pass: `chart_build_probe.rs:98` (behind
# `#[cfg(feature = "chart-build-probe")]`, off by default),
# `cache_state_summary_badge.rs:126` (inside a `#[test]` fn asserting the label
# ignores mtime), `throttled_spinner.rs:230` (carries its own H-arch-9
# "Instant::now is test-unreachable" analysis: snapshot paths never deliver
# `RedrawRequested`, so the seed values stand) and `chart.rs:301` (already
# marked). The fifth, `activity_tape.rs:63`, is an `Instant::now()` age filter
# against `ACTIVITY_TICK_RENDER_FLOOR_MS` inside `view()`, and whether its
# FILTER OUTCOME is snapshot-invariant was not established — writing a marker
# that says it is would be inventing a reason, which is the `#186` defect.
# That analysis is the whole of the remaining work.
#
# Acceptance criteria (V4):
#   - Exits 0 on the clean tree.
#   - Exits non-zero when a `SystemTime::now()` is injected.
#   - Listed in `rust-validate`'s pre-test gate, in `.githooks/pre-commit`,
#     and in the ci.yml governance block.

set -euo pipefail

# Files to scan — the rendering / Charts-screen paths reachable from
# `iced_test::screenshot(&ui::test_support::program_from_cockpit(...))`.
# Extending the watchlist is one line per file. Every entry MUST exist.
WATCHLIST=(
  "crates/ui/src/widgets/chart.rs"
  "crates/ui/src/widgets/canvas_chart.rs"
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

# A line whose first non-space characters open a comment is PROSE, not a call.
is_comment_line() {
  [[ "$1" =~ ^[[:space:]]*(//|/\*|\*) ]]
}

# Walk upward from the line above `$2` through the contiguous run of comment
# lines and report whether any of them carries the marker. Stops at the first
# non-comment line, so a marker cannot leak across a blank line or a statement.
marker_in_block_above() {
  local file="$1" lineno="$2" i line
  i=$((lineno - 1))
  while (( i >= 1 )); do
    line="$(sed -n "${i}p" "$file")"
    is_comment_line "$line" || return 1
    [[ "$line" == *"// CLOCK-OK:"* ]] && return 0
    i=$((i - 1))
  done
  return 1
}

scan_tree() {
  local root="$1"
  local -i failed=0 scanned=0 examined=0 exempted=0
  local file abs pat lineno match

  for file in "${WATCHLIST[@]}"; do
    abs="${root}/${file}"
    if [[ ! -f "$abs" ]]; then
      # Defect 3: this used to be a silent `continue`.
      echo "FAIL  ${file} — watchlist entry does not exist"
      echo "      A renamed or deleted path is a DECISION; update the watchlist"
      echo "      (or remove the entry and say why) rather than scanning fewer files."
      failed=1
      continue
    fi
    scanned+=1
    for pat in "${PATTERNS[@]}"; do
      while IFS=: read -r lineno match; do
        [[ -z "$lineno" ]] && continue
        examined+=1
        # Defect 2: prose is not a call.
        if is_comment_line "$match"; then
          continue
        fi
        # Same line, or anywhere in the comment block directly above (defect 1).
        if [[ "$match" == *"// CLOCK-OK:"* ]] || marker_in_block_above "$abs" "$lineno"; then
          exempted+=1
          continue
        fi
        echo "FAIL  ${file}:${lineno} — unwhitelisted '${pat}'"
        echo "      ${match}"
        failed=1
      done < <(grep -n -E "${pat}" "$abs" || true)
    done
  done

  if (( failed != 0 )); then
    echo ""
    echo "Unwhitelisted clock/RNG calls reachable from the snapshot path."
    echo "Add a '// CLOCK-OK: <reason>' marker on the same line, or anywhere in"
    echo "the comment block immediately above it, if the use is intentional."
    return 1
  fi

  # Defect 4: measured, not declared.
  if (( scanned != ${#WATCHLIST[@]} )); then
    echo "FAIL  scanned ${scanned} file(s) but the watchlist declares ${#WATCHLIST[@]}"
    return 1
  fi
  echo "CLOCKS PASS  (${scanned}/${#WATCHLIST[@]} files read · ${#PATTERNS[@]} patterns · ${examined} matching line(s) examined · ${exempted} exemption(s) honoured)"
  return 0
}

# ---------------------------------------------------------------------------
# --self-test — synthetic fixtures in a tmpdir; never touches the real tree.
# ---------------------------------------------------------------------------
self_test() {
  # `local -i`, not `local`: with a string variable `probes+=1` CONCATENATES,
  # and this script's own defect 4 was a printed count that was never measured.
  # First run of this self-test reported "01111111111/01111111111 probes".
  local tmp
  local -i probes=0 fired=0
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN

  _probe() {
    # $1 = name, $2 = expected ("fire" | "silent"), $3 = file body
    local name="$1" expect="$2" body="$3" out rc verdict
    mkdir -p "$tmp/crates/ui/src/widgets" "$tmp/crates/ui/src" \
             "$tmp/crates/ui/tests/fixtures"
    # Every watchlist entry must exist or defect-3 fires and masks the probe.
    local f
    for f in "${WATCHLIST[@]}"; do
      mkdir -p "$tmp/$(dirname "$f")"
      : > "$tmp/$f"
    done
    printf '%s\n' "$body" > "$tmp/crates/ui/src/widgets/chart.rs"
    out="$(scan_tree "$tmp" 2>&1)" && rc=0 || rc=1
    probes+=1
    if [[ "$expect" == "fire" ]]; then
      (( rc != 0 )) && verdict="OK   " || verdict="FAIL "
      (( rc != 0 )) && fired+=1
    else
      (( rc == 0 )) && verdict="OK   " || verdict="FAIL "
      (( rc == 0 )) && fired+=1
    fi
    echo "  ${verdict}${name} (expected ${expect}, rc=${rc})"
  }

  echo "check_no_clocks --self-test"

  _probe "P1 a bare SystemTime::now fires" fire \
    'fn f() { let _ = SystemTime::now(); }'

  _probe "P2 the same call with a same-line marker is exempt" silent \
    'fn f() { let _ = SystemTime::now(); } // CLOCK-OK: probe'

  _probe "P3 a marker on the line directly above is exempt" silent \
    '// CLOCK-OK: probe
fn f() { let _ = SystemTime::now(); }'

  # Defect 1: this is the case that failed the real tree for months.
  _probe "P4 a marker at the TOP of a wrapped comment block is exempt" silent \
    '// CLOCK-OK: a reason long enough to wrap onto
// a second line, which is a GOOD reason
fn f() { let _ = SystemTime::now(); }'

  _probe "P5 a marker separated by a blank line does NOT exempt" fire \
    '// CLOCK-OK: too far

fn f() { let _ = SystemTime::now(); }'

  _probe "P6 a marker separated by a statement does NOT exempt" fire \
    '// CLOCK-OK: too far
let _ = 1;
fn f() { let _ = SystemTime::now(); }'

  # Defect 2: prose is not a call.
  _probe "P7 a mention inside a doc comment stays silent" silent \
    '/// Reads the offset via `SystemTime::now()` when asked.
fn f() {}'

  _probe "P8 a mention inside a block-comment body stays silent" silent \
    '/*
 * SystemTime::now is what this used to do.
 */
fn f() {}'

  _probe "P9 the control: a clean file is silent" silent \
    'fn f() { let _ = 1; }'

  # Defect 3: a missing watchlist entry must FAIL, not vanish.
  local out rc
  mkdir -p "$tmp/empty"
  out="$(scan_tree "$tmp/empty" 2>&1)" && rc=0 || rc=1
  probes+=1
  if (( rc != 0 )) && [[ "$out" == *"does not exist"* ]]; then
    echo "  OK   P10 a missing watchlist entry fails loudly (expected fire, rc=${rc})"
    fired+=1
  else
    echo "  FAIL P10 a missing watchlist entry fails loudly (expected fire, rc=${rc})"
  fi

  echo ""
  if (( fired == probes )); then
    echo "check_no_clocks --self-test: PASS (${fired}/${probes} probes)"
    return 0
  fi
  echo "check_no_clocks --self-test: FAIL (${fired}/${probes} probes)"
  return 1
}

case "${1-}" in
  --self-test) self_test ;;
  "")          scan_tree "$WORKSPACE_ROOT" ;;
  *)           echo "usage: $0 [--self-test]" >&2; exit 2 ;;
esac
