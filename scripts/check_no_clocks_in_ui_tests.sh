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
# WIDENED THE SAME DAY, after the analysis the first draft deferred. The
# watchlist is now a WALK of `crates/ui/src/widgets/` + `crates/ui/src/screens/`
# plus five named harness files — 76 files against a floor of 60. The five
# non-comment hits each carry a marker whose reason was verified individually,
# not asserted:
#
#   - `chart.rs:301` — the sanctioned production read, already marked.
#   - `chart_build_probe.rs:98` — the whole `impl` is behind
#     `#[cfg(feature = "chart-build-probe")]`, off by default and enabled by no
#     snapshot invocation.
#   - `cache_state_summary_badge.rs:126` — inside a `#[test]` fn whose assertion
#     IS that `populated_count == 0` makes the value irrelevant.
#   - `throttled_spinner.rs:234` — its own H-arch-9 analysis: snapshot paths
#     never deliver `RedrawRequested`, so the seed values stand.
#   - `activity_tape.rs:63` — the one the first draft could not verify, now
#     settled by reading the graph: `ActivityTape::apply` is the only mutator and
#     is called from exactly one site (`state.rs:4043`) inside a message handler
#     the harness never dispatches; the single gallery cell declares
#     `state: "empty"` and seeds `Cockpit::default()`; and no gallery cell
#     anywhere constructs an `ActivityEvent`. So `view` returns a zero-size
#     `Space` before `now` reaches anything drawn.
#
# That last one changed the finding rather than closing it. `activity_tape.rs`
# renders `format_elapsed(now.duration_since(state.started_at))` INTO THE PIXELS,
# so what keeps it out of the baselines is a CONDITION — that its only gallery
# cell is the empty state — and not a property of the call. The marker says so,
# and says what to do instead (a seeded, fixed `started_at`) when someone adds a
# populated cell. This gate is now the thing that will ask.
#
# Acceptance criteria (V4):
#   - Exits 0 on the clean tree.
#   - Exits non-zero when a `SystemTime::now()` is injected.
#   - Listed in `rust-validate`'s pre-test gate, in `.githooks/pre-commit`,
#     and in the ci.yml governance block.

set -euo pipefail

# WALKED, not listed. Until 2026-11-10 this was a hand-written list of eight
# files, one of which had not existed since `f3cb7f92` — the `#188` defect, in a
# gate. The subject is every rendering path reachable from
# `iced_test::screenshot(&ui::test_support::program_from_cockpit(...))`, which is
# the widget and screen trees plus the harness's own files, so that is what it
# now walks. Cost of the widening, measured before it landed: 5 non-comment hits
# in 5 files, each given a marker whose reason was individually verified
# (bug-log `#190`).
SCAN_DIRS=(
  "crates/ui/src/widgets"
  "crates/ui/src/screens"
)

# The harness's own files, which live outside those trees. Every entry MUST
# exist — these five are also what makes a mis-rooted run impossible to mistake
# for a clean one.
NAMED_FILES=(
  "crates/ui/src/test_support.rs"
  "crates/ui/tests/visual_snapshots.rs"
  "crates/ui/tests/chart_hover_grid_sweep.rs"
  "crates/ui/tests/fixtures/mod.rs"
  "crates/ui/tests/fixtures/visual_diff.rs"
)

# Non-vacuity floor. Measured 2026-11-10: 51 `widgets/*.rs` + 20 `screens/*.rs`
# + 5 named = 76. The floor's job is to make "the glob returned nothing" or a
# mis-rooted walk impossible to mistake for a pass — NOT to turn every deleted
# widget into a gate failure, which is why it sits below the measurement rather
# than on it. A drop to 59 means something structural moved and should be read,
# not bumped.
MIN_FILES=60

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

  # Resolve the subject: the walked trees, then the named harness files.
  local -a files=()
  local d
  for d in "${SCAN_DIRS[@]}"; do
    if [[ ! -d "${root}/${d}" ]]; then
      echo "FAIL  ${d} — scan directory does not exist"
      failed=1
      continue
    fi
    while IFS= read -r found; do
      files+=("${found#"${root}/"}")
    done < <(find "${root}/${d}" -name '*.rs' -type f | sort)
  done
  for file in "${NAMED_FILES[@]}"; do
    if [[ ! -f "${root}/${file}" ]]; then
      # Defect 3: this used to be a silent `continue`.
      echo "FAIL  ${file} — named harness file does not exist"
      echo "      A renamed or deleted path is a DECISION; update NAMED_FILES"
      echo "      (or remove the entry and say why) rather than scanning fewer files."
      failed=1
      continue
    fi
    files+=("$file")
  done

  for file in "${files[@]}"; do
    abs="${root}/${file}"
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
  if (( scanned < MIN_FILES )); then
    echo "FAIL  scanned ${scanned} file(s), below the declared floor of ${MIN_FILES}"
    echo "      Something structural moved. Read it before bumping the floor."
    return 1
  fi
  echo "CLOCKS PASS  (${scanned} file(s) read [floor ${MIN_FILES}] · ${#PATTERNS[@]} patterns · ${examined} matching line(s) examined · ${exempted} exemption(s) honoured)"
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
  # The fixtures are a handful of files, so the production floor would fire on
  # every probe and mask what each one is testing. P11 below pins the floor
  # itself, which is the only place it belongs.
  local -i MIN_FILES=1
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN

  _probe() {
    # $1 = name, $2 = expected ("fire" | "silent"), $3 = file body
    local name="$1" expect="$2" body="$3" out rc verdict
    mkdir -p "$tmp/crates/ui/src/widgets" "$tmp/crates/ui/src/screens" \
             "$tmp/crates/ui/src" "$tmp/crates/ui/tests/fixtures"
    # Every named file must exist or defect-3 fires and masks the probe.
    local f
    for f in "${NAMED_FILES[@]}"; do
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

  # Defect 4 / non-vacuity: the floor must fire when the walk collapses.
  local -i save_min=$MIN_FILES
  MIN_FILES=9999
  out="$(scan_tree "$tmp" 2>&1)" && rc=0 || rc=1
  MIN_FILES=$save_min
  probes+=1
  if (( rc != 0 )) && [[ "$out" == *"below the declared floor"* ]]; then
    echo "  OK   P11 a scan below the floor fails loudly (expected fire, rc=${rc})"
    fired+=1
  else
    echo "  FAIL P11 a scan below the floor fails loudly (expected fire, rc=${rc})"
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
