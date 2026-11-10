#!/usr/bin/env bash
# scripts/check_no_secrets_in_llm_artifacts.sh — V9 grep gate (T1926).
#
# Walks every artifact the `llm-smoke` binary writes — replay DB, run
# logs, fixtures, audit ledger — and asserts that NO common API-key
# pattern appears in any of them. Per Q3 = C resolution: redaction is
# a tracing-only cosmetic; production artifacts must not surface the
# prefix even with the masked-tail.
#
# Patterns scanned (lowercase grep so casing variations still match):
#
#   sk-                             — generic OpenAI / Anthropic / Cohere prefix
#   sk-ant-                         — Anthropic explicit prefix
#   bearer                          — `Authorization: Bearer ...` headers
#   anthropic-api-key               — Anthropic SDK env-var spelling
#   openai-api-key                  — OpenAI SDK env-var spelling
#   x-api-key                       — Anthropic auth header
#
# Synthetic V9 test keys (substring greps):
#
#   v9-secretkey-12345678           — used by the secrets test in CI
#   v9-openai-secretkey-87654321    — used by the secrets test in CI
#
# Exit codes:
#   0  — no hits in any artifact path.
#   1  — one or more patterns matched (stderr lists every hit).
#   2  — required artifact path missing (configuration / harness bug).
#
# The script is invocable standalone (`bash scripts/check_no_secrets_in_llm_artifacts.sh`)
# AND from the V9 integration test (T1926).
#
# THIS FILE IS THE SINGLE SOURCE OF TRUTH FOR THE PATTERN LIST.
# `crates/llm/tests/no_secrets_in_artifacts_test.rs` READS the `PATTERNS=(...)`
# array and the `SK_RE=` line out of this file at runtime and scans the smoke
# run's artifacts itself, in Rust, on every platform — so the test and CI cannot
# drift. Two consequences worth knowing before editing:
#
#   1. Keep the `PATTERNS=(` opener, one quoted pattern per line, and a closing
#      `)` on its own line. The test's parser asserts it found >= 8 patterns and
#      fails loudly rather than scanning a silently-empty list.
#   2. Changing `SK_RE` fails that test on purpose: the Rust side pins the
#      supported spelling (`SUPPORTED_SK_RE`) because its matcher is hand-rolled.
#      Update both together.
#
# On unix the test ALSO executes this script verbatim, so the find/grep/strings
# plumbing below stays exercised. It does not on windows-latest: `strings(1)` is
# binutils (absent from Git-for-Windows) and a `strings`-less run reports "no
# match" for every binary artifact — a gate that passes having scanned nothing.
#
# Usage:
#   scripts/check_no_secrets_in_llm_artifacts.sh \
#       [--db PATH] [--log-dir DIR] [--fixtures-dir DIR] [--audit-db PATH]

set -euo pipefail

# Defaults — operator can override via CLI flags for non-default
# layouts (e.g. CI puts everything under a per-job scratch dir).
REPLAY_DB="${REPLAY_DB:-data/llm-replay.db}"
LOG_DIR="${LOG_DIR:-target/logs}"
FIXTURES_DIR="${FIXTURES_DIR:-crates/llm/fixtures}"
AUDIT_DB="${AUDIT_DB:-data/audit.db}"
# By default the gate scans LLM-written artifacts only. Pass
# `--scan-spec` to also grep `evidence/**.md` + `evidence/**.toml` AND
# `docs/**.md` + `docs/**.toml` AND `_bmad-output/**.md` +
# `_bmad-output/**.toml` (used by the standalone CI helper that catches
# operators pasting real keys into runbooks / reports / stories).
# `evidence/` joined the scan 2026-07-25 (BMAD-migration Phase 3 — the
# byte-immutable reports corpus moved out of `spec/` there); `docs/` +
# `_bmad-output/` joined 2026-07-25 (BMAD-migration Phase 5b `spec/`
# retirement — flag name kept verbatim, the `spec/` leg is retired since
# the directory no longer exists). The T1926 integration test passes
# the artifact set only — these docs legitimately use `sk-...` /
# `Bearer ...` as placeholder examples in design / runbook / report / story
# docs.
SCAN_SPEC=0
# Non-vacuity floor (bug-log `#193` C). The gate used to print PASS without
# saying how many files it read, and every target is silently skippable: on a
# fresh CI checkout exactly ONE of the four default targets is tracked, so the
# AD-19 gate scanned one fixture database and passed. Default 1 rather than 0
# because "scanned nothing" must never be a pass — and 1 is what the T1926
# Rust harness's own invocation scans (`--db <file> --log-dir <empty tmpdir>
# --audit-db /dev/null --fixtures-dir <empty tmpdir>`), so the default does not
# break it. CI and the hook pass a real floor explicitly.
MIN_FILES="${MIN_FILES:-1}"
RUN_SELF_TEST=0
# Which paths the CALLER named. A default that is absent is skipped (a fresh
# checkout has no `target/logs`); a path someone typed and got wrong is the
# "configuration / harness bug" the header promises exit 2 for.
EXPLICIT_DB=0
EXPLICIT_LOG_DIR=0
EXPLICIT_FIXTURES=0
EXPLICIT_AUDIT_DB=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --db) REPLAY_DB="$2"; EXPLICIT_DB=1; shift 2 ;;
    --log-dir) LOG_DIR="$2"; EXPLICIT_LOG_DIR=1; shift 2 ;;
    --fixtures-dir) FIXTURES_DIR="$2"; EXPLICIT_FIXTURES=1; shift 2 ;;
    --audit-db) AUDIT_DB="$2"; EXPLICIT_AUDIT_DB=1; shift 2 ;;
    --scan-spec) SCAN_SPEC=1; shift ;;
    --min-files) MIN_FILES="$2"; shift 2 ;;
    --self-test) exec bash "$0" --__run-self-test ;;
    --__run-self-test) RUN_SELF_TEST=1; shift ;;
    -h|--help)
      sed -n 's/^# //p' "$0" | head -40
      exit 0
      ;;
    *)
      echo "unknown flag: $1" >&2
      exit 2
      ;;
  esac
done

# ---------------------------------------------------------------------------
# --self-test (bug-log `#193` B/C) — synthetic fixtures in a tmpdir.
# ---------------------------------------------------------------------------
#
# Until 2026-11-10 this was the ONLY gate in `.githooks/pre-commit` or the
# ci.yml governance block with no probe — which, for the gate enforcing AD-19
# ("no secrets in git"), is the worst possible one to leave unmeasured. Each
# pair below differs in exactly one dimension and expects opposite outcomes.
self_test() {
  local tmp probes=0 fired=0 out rc
  tmp="$(mktemp -d)"
  # shellcheck disable=SC2064
  trap "rm -rf '$tmp'" RETURN
  mkdir -p "$tmp/logs" "$tmp/fix" "$tmp/empty-logs" "$tmp/empty-fix"

  # $1 name, $2 expected rc, $3 expected substring (may be empty), $4.. flags
  _p() {
    local name="$1" want_rc="$2" want_txt="$3"; shift 3
    out="$(bash "$SELF" "$@" 2>&1)" && rc=0 || rc=$?
    probes=$((probes + 1))
    local ok=1
    [ "$rc" = "$want_rc" ] || ok=0
    if [ -n "$want_txt" ]; then
      case "$out" in *"$want_txt"*) ;; *) ok=0 ;; esac
    fi
    if [ "$ok" = 1 ]; then
      echo "  OK   $name (rc=$rc)"
      fired=$((fired + 1))
    else
      echo "  FAIL $name (rc=$rc, wanted $want_rc${want_txt:+ + \"$want_txt\"})"
      echo "       got: $(printf '%s' "$out" | head -2 | tr '\n' ' ')"
    fi
  }

  echo "check_no_secrets --self-test"

  # ── the `#193` A pair: a real key shape vs the English word that used to match
  printf 'Authorization: sk-ant-api03-ABCDEFGHIJKLMNOP\n' > "$tmp/logs/a.log"
  _p "S1 a real key shape FIRES" 1 "V9 HIT" \
     --db /dev/null --audit-db /dev/null --log-dir "$tmp/logs" --fixtures-dir "$tmp/empty-fix"

  printf 'the risk-level-overlay and risk-management-section notes\n' > "$tmp/logs/a.log"
  _p "S2 'risk-level' is SILENT (the #193 A fix)" 0 "V9 PASS" \
     --db /dev/null --audit-db /dev/null --log-dir "$tmp/logs" --fixtures-dir "$tmp/empty-fix"

  # ── a literal prefix still fires, so the fix did not disarm the literals
  printf 'leaked sk-ant- prefix here\n' > "$tmp/logs/a.log"
  _p "S3 a literal pattern FIRES" 1 "V9 HIT" \
     --db /dev/null --audit-db /dev/null --log-dir "$tmp/logs" --fixtures-dir "$tmp/empty-fix"

  # ── the floor pair: one clean file passes, ZERO files does not
  printf 'nothing to see\n' > "$tmp/logs/a.log"
  _p "S4 a clean file passes and reports its count" 0 "1 scanned file(s)" \
     --db /dev/null --audit-db /dev/null --log-dir "$tmp/logs" --fixtures-dir "$tmp/empty-fix"

  _p "S5 scanning ZERO files FAILS the floor" 1 "below the floor" \
     --db /dev/null --audit-db /dev/null --log-dir "$tmp/empty-logs" --fixtures-dir "$tmp/empty-fix"

  # ── the `-e` vs `-f` pair, which is what keeps the T1926 harness green
  _p "S6 an explicitly named MISSING path exits 2" 2 "V9 CONFIG ERROR" \
     --db "$tmp/nope.db" --audit-db /dev/null --log-dir "$tmp/logs" --fixtures-dir "$tmp/empty-fix"

  _p "S7 /dev/null is NOT a config error (exists, then skipped)" 0 "V9 PASS" \
     --db /dev/null --audit-db /dev/null --log-dir "$tmp/logs" --fixtures-dir "$tmp/empty-fix"

  # ── the floor is a floor, not a constant
  _p "S8 an unmeetable --min-files FAILS even on a clean scan" 1 "below the floor" \
     --min-files 9999 --db /dev/null --audit-db /dev/null --log-dir "$tmp/logs" --fixtures-dir "$tmp/empty-fix"

  echo ""
  if [ "$fired" = "$probes" ]; then
    echo "check_no_secrets --self-test: PASS ($fired/$probes probes)"
    return 0
  fi
  echo "check_no_secrets --self-test: FAIL ($fired/$probes probes)"
  return 1
}

# Patterns to grep for. Run grep -i (case-insensitive) against every
# binary or text artifact. SQLite DBs surface BLOB content as text
# under `strings`; we pipe `strings(1)` so SQLite-stored JSON in
# `response_json` is reachable even though the column is TEXT.
PATTERNS=(
  "sk-ant-"
  "sk-proj-"
  "bearer "
  "anthropic-api-key"
  "openai-api-key"
  "x-api-key"
  "V9-secretkey-12345678"
  "V9-OpenAI-secretkey-87654321"
)

# We DON'T grep for `sk-` standalone because Markdown / runbook docs
# legitimately use `sk-...` as a placeholder example. The narrower
# prefixes above are the real-world key shapes; the V9 test keys
# above are the unique substrings the test harness emits.

# Pattern: stand-alone `sk-` followed by at least 12 chars (real keys
# are 40+ chars). Match avoids `sk-...` placeholders.
#
# THE LEADING BOUNDARY IS LOAD-BEARING (bug-log `#193` A, 2026-11-10). Without
# it this was `sk-[A-Za-z0-9_-]{12,}`, which matches INSIDE ENGLISH WORDS: two
# of the three key-shape hits in the `--scan-spec` corpus were
# `ri`+`sk-management` inside a markdown anchor and `ri`+`sk-level` inside the
# phrase "risk-level overlay". Any word ending in `sk` before a hyphen matched —
# risk-, disk-, task-, desk-. An unanchored pattern over prose finds English,
# not secrets.
#
# `crates/llm/tests/no_secrets_in_artifacts_test.rs` parses this line at runtime
# and asserts it equals its `SUPPORTED_SK_RE`, because its matcher is
# hand-rolled: this spelling, that constant and its `sk_shape_hit` move
# together or the test fails on purpose.
SK_RE='(^|[^A-Za-z0-9_-])sk-[A-Za-z0-9_-]{12,}'

# Absolute path to this script, for the self-test's re-entrant probe calls.
SELF="$(cd "$(dirname "$0")" && pwd)/$(basename "$0")"

if [[ "$RUN_SELF_TEST" -eq 1 ]]; then
  self_test
  exit $?
fi

hits=0
scanned=0
report_hit() {
  echo "V9 HIT: $1" >&2
  hits=$((hits + 1))
}

# A path named EXPLICITLY on the command line that does not exist at all is the
# "configuration / harness bug" the header has always promised exit 2 for — and
# never delivered: before 2026-11-10 the only `exit 2` in this file was for an
# unknown flag, and a missing artifact exited 0 (bug-log `#193` C). `-e` and not
# `-f` on purpose: the T1926 harness passes `--audit-db /dev/null`, a character
# device, which EXISTS and is then skipped by the `-f` test below.
require_exists() {
  if [[ ! -e "$1" ]]; then
    echo "V9 CONFIG ERROR: $2 '$1' was named explicitly and does not exist" >&2
    exit 2
  fi
}

scan_text_file() {
  local file="$1"
  if [[ ! -f "$file" ]]; then
    return 0
  fi
  scanned=$((scanned + 1))
  for pat in "${PATTERNS[@]}"; do
    if grep -i -q -F "$pat" "$file" 2>/dev/null; then
      report_hit "$file contains literal '$pat'"
    fi
  done
  if grep -E -q "$SK_RE" "$file" 2>/dev/null; then
    report_hit "$file contains sk-key-shape"
  fi
}

scan_binary_file() {
  local file="$1"
  if [[ ! -f "$file" ]]; then
    return 0
  fi
  scanned=$((scanned + 1))
  for pat in "${PATTERNS[@]}"; do
    if strings -- "$file" 2>/dev/null | grep -i -q -F "$pat"; then
      report_hit "$file (binary/SQLite) contains literal '$pat'"
    fi
  done
  if strings -- "$file" 2>/dev/null | grep -E -q "$SK_RE"; then
    report_hit "$file (binary/SQLite) contains sk-key-shape"
  fi
}

# `require_exists` was added with the exit-2 path and NOT CALLED — the dangling
# half of the same change. Self-test probe S6 caught that, and then caught my fix
# for it too: the calls sat ABOVE the function definition, so bash reported
# `require_exists: command not found`. Production never noticed, because the bare
# invocation names no paths and short-circuits before the call — the probes
# exercise a path production does not. They belong HERE, after the definitions
# and immediately before the scan.
[[ "$EXPLICIT_DB" -eq 1 ]] && require_exists "$REPLAY_DB" "--db"
[[ "$EXPLICIT_AUDIT_DB" -eq 1 ]] && require_exists "$AUDIT_DB" "--audit-db"
[[ "$EXPLICIT_LOG_DIR" -eq 1 ]] && require_exists "$LOG_DIR" "--log-dir"
[[ "$EXPLICIT_FIXTURES" -eq 1 ]] && require_exists "$FIXTURES_DIR" "--fixtures-dir"
true  # the `&&` chains above must not set the script's exit status under `set -e`

# 1. Replay DB (runtime + fixture).
scan_binary_file "$REPLAY_DB"
if [[ -d "$FIXTURES_DIR" ]]; then
  while IFS= read -r f; do
    scan_binary_file "$f"
  done < <(find "$FIXTURES_DIR" -type f -name '*.db' 2>/dev/null)
fi

# 2. Logs.
if [[ -d "$LOG_DIR" ]]; then
  while IFS= read -r f; do
    scan_text_file "$f"
  done < <(find "$LOG_DIR" -type f 2>/dev/null)
fi

# 3. Audit ledger.
scan_binary_file "$AUDIT_DB"

# 4. Committed report bodies under evidence/ (the byte-immutable reports
#    corpus moved there in the 2026-07-25 BMAD-migration Phase 3 `git mv`)
#    + project-knowledge docs under docs/ (dev-notes/runbooks/design/
#    ui-design-principles.md moved there in the 2026-07-25 BMAD-migration
#    Phase 4 `git mv`, `docs/archive/` retired-spec content moved there in
#    Phase 5b) + BMAD planning/implementation artifacts under _bmad-output/
#    (PRD/architecture/ADRs/trace.toml/stories — Phase 5b) — opt-in via
#    `--scan-spec`. These legitimately use placeholder key shapes
#    (`sk-ant-...`, `Bearer ...`) so the default integration-test invocation
#    skips this leg. `spec/` itself is RETIRED (Phase 5b, 2026-07-25) — no
#    leg for it; the directory no longer exists.
if [[ "$SCAN_SPEC" -eq 1 && -d "evidence" ]]; then
  while IFS= read -r f; do
    scan_text_file "$f"
  done < <(find evidence -type f \( -name '*.md' -o -name '*.toml' \) 2>/dev/null)
fi
if [[ "$SCAN_SPEC" -eq 1 && -d "docs" ]]; then
  while IFS= read -r f; do
    scan_text_file "$f"
  done < <(find docs -type f \( -name '*.md' -o -name '*.toml' \) 2>/dev/null)
fi
if [[ "$SCAN_SPEC" -eq 1 && -d "_bmad-output" ]]; then
  while IFS= read -r f; do
    scan_text_file "$f"
  done < <(find _bmad-output -type f \( -name '*.md' -o -name '*.toml' \) 2>/dev/null)
fi

if [[ $hits -gt 0 ]]; then
  echo "V9 FAIL: $hits secret pattern(s) found in $scanned scanned file(s)" >&2
  exit 1
fi

# Measured, not declared. "No hits" over zero files is not a pass.
if [[ "$scanned" -lt "$MIN_FILES" ]]; then
  echo "V9 FAIL: scanned $scanned file(s), below the floor of $MIN_FILES — a clean" >&2
  echo "         result over nothing is not a pass. Check the artifact paths." >&2
  exit 1
fi

echo "V9 PASS: no secret patterns in $scanned scanned file(s) [floor $MIN_FILES] · ${#PATTERNS[@]} literals + 1 key-shape regex"
exit 0
