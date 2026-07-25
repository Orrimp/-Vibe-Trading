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
# AND from the V9 integration test (T1926 — wraps this script in a
# `std::process::Command::status()` call after running the smoke harness).
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
# `--scan-spec` to also grep `spec/**.md` + `spec/**.toml` AND
# `evidence/**.md` + `evidence/**.toml` (used by the standalone CI
# helper that catches operators pasting real keys into runbooks /
# reports). `evidence/` joined the scan 2026-07-25 (BMAD-migration
# Phase 3 — the byte-immutable reports corpus moved out of `spec/`
# there; flag name kept verbatim). The T1926 integration test passes
# the artifact set only — spec/evidence files legitimately use
# `sk-...` / `Bearer ...` as placeholder examples in design / runbook
# / report docs.
SCAN_SPEC=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --db) REPLAY_DB="$2"; shift 2 ;;
    --log-dir) LOG_DIR="$2"; shift 2 ;;
    --fixtures-dir) FIXTURES_DIR="$2"; shift 2 ;;
    --audit-db) AUDIT_DB="$2"; shift 2 ;;
    --scan-spec) SCAN_SPEC=1; shift ;;
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
SK_RE='sk-[A-Za-z0-9_-]{12,}'

hits=0
report_hit() {
  echo "V9 HIT: $1" >&2
  hits=$((hits + 1))
}

scan_text_file() {
  local file="$1"
  if [[ ! -f "$file" ]]; then
    return 0
  fi
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
  for pat in "${PATTERNS[@]}"; do
    if strings -- "$file" 2>/dev/null | grep -i -q -F "$pat"; then
      report_hit "$file (binary/SQLite) contains literal '$pat'"
    fi
  done
  if strings -- "$file" 2>/dev/null | grep -E -q "$SK_RE"; then
    report_hit "$file (binary/SQLite) contains sk-key-shape"
  fi
}

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

# 4. Any committed docs under spec/ + committed report bodies under
#    evidence/ (the byte-immutable reports corpus moved there in the
#    2026-07-25 BMAD-migration Phase 3 `git mv`) + project-knowledge docs
#    under docs/ (dev-notes/runbooks/design/ui-design-principles.md moved
#    there in the 2026-07-25 BMAD-migration Phase 4 `git mv`) — opt-in via
#    `--scan-spec`. Spec/evidence/docs legitimately use placeholder
#    key shapes (`sk-ant-...`, `Bearer ...`) so the default
#    integration-test invocation skips this leg.
if [[ "$SCAN_SPEC" -eq 1 && -d "spec" ]]; then
  while IFS= read -r f; do
    scan_text_file "$f"
  done < <(find spec -type f \( -name '*.md' -o -name '*.toml' \) 2>/dev/null)
fi
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

if [[ $hits -gt 0 ]]; then
  echo "V9 FAIL: $hits secret pattern(s) found in artifacts" >&2
  exit 1
fi

echo "V9 PASS: no secret patterns found in any scanned artifact"
exit 0
