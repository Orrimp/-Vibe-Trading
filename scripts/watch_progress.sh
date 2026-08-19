#!/usr/bin/env bash
# watch_progress.sh — one-screen status for long-running work in this repo.
#
# Designed to be polled by `watch`, so it MUST stay fast (<1s): it reads git,
# the process table, and log tails. It never invokes cargo.
#
#   watch -n 5 -c bash scripts/watch_progress.sh          # colour, 5s
#   bash scripts/watch_progress.sh --gates                # + anchors/spec-lint (slower, ~2s)
#
# macOS without `watch`:  brew install watch
#   ...or the builtin fallback:  bash scripts/watch_progress.sh --loop 5
set -uo pipefail
cd "$(dirname "$0")/.." || exit 1

SCRATCH="${CLAUDE_SCRATCH:-/private/tmp/claude-502/-Users-Vitaliy-Schreibmann-Projects-Privat-trading-trading/362d2a09-04ba-4ea6-a7c1-07605f6e187a/scratchpad}"
B=$'\033[1m'; D=$'\033[2m'; G=$'\033[32m'; Y=$'\033[33m'; R=$'\033[31m'; N=$'\033[0m'

if [ "${1:-}" = "--loop" ]; then
  while true; do clear; bash "$0" "${3:-}"; sleep "${2:-5}"; done; exit 0
fi

hr() { printf "${D}%s${N}\n" "────────────────────────────────────────────────────────────────"; }

printf "${B}trading — progress${N}   ${D}%s${N}\n" "$(date '+%F %T')"
hr

# ── 1. running jobs ──────────────────────────────────────────────────────────
printf "${B}JOBS${N}\n"
jobs_found=0
while read -r et cmd; do
  [ -z "${et:-}" ] && continue
  short=$(printf '%s' "$cmd" | sed -E 's#.*/target/[^/]*/deps/##; s#^/[^ ]*/##' | cut -c1-58)
  printf "  ${Y}%-12s${N} %s\n" "$et" "$short"; jobs_found=1
done < <(ps -eo etime,command 2>/dev/null \
          | grep -E 'cargo (test|build|clippy|check)|param_robustness_sweep|threshold_sweep|target/[^ ]*/deps/' \
          | grep -v grep | awk '{et=$1; $1=""; print et, substr($0,2)}' | head -6)
[ "$jobs_found" = 0 ] && printf "  ${D}idle — no cargo/sweep running${N}\n"
# `docker ps` costs ~4s on this box — opt in with --docker, keep the poll fast.
case " $* " in *" --docker "*)
  dk=$(docker ps -q 2>/dev/null | wc -l | tr -d ' ')
  [ "${dk:-0}" != "0" ] && printf "  ${Y}docker${N}       %s container(s)\n" "$dk" ;;
esac
hr

# ── 2. story 1-25 CRITICAL burn-down ─────────────────────────────────────────
printf "${B}STORY 1-25 — eight CRITICALs${N}\n"
# Capture ONCE into variables, then match with `case`. Do NOT pipe into `grep -q`
# here: grep exits on first match, git dies of SIGPIPE (141), and `set -o pipefail`
# reports that as the pipeline's status — so a SUCCESSFUL match reads as failure.
# (Same pipeline-exit-status trap as bug-log #84, which disabled the CI anchor gate.)
# ONLY `fix(...)` subjects count as a fix. `disclose(...)`, `plan(...)`,
# `measure(...)` mention the same ids and must NOT read as done — an earlier
# version marked #76 "pushed" off `disclose(1-21): two CRITICALs ...`.
_subjects=$(git log --format=%s -80 2>/dev/null | grep '^fix' || true)
_codediff=$(git diff HEAD -- crates/ scripts/ 2>/dev/null)
for spec in "67:engine guard + per-symbol fill" "75:score/accrual channel split" \
            "71:side-aware exposure cap" "76:residual direction" \
            "69:portfolio cap inert" "72:settlement cadence" \
            "73:funding dedup" "68:drift axis"; do
  id="${spec%%:*}"; desc="${spec#*:}"
  # pushed: the id appears in a commit SUBJECT (e.g. "fix(1-25,#67): ...")
  # local:  the id appears in an uncommitted diff of CODE — a docs/ mention is
  #         NOT evidence of a fix (the bug-log names every id, which made an
  #         earlier version of this script report #68 as done).
  case "$_subjects" in
    *"#${id})"*|*"#${id}:"*|*"#${id},"*)
      printf "  ${G}✔ pushed ${N} #%-3s %s\n" "$id" "$desc" ;;
    *)
      case "$_codediff" in
        *"#${id}"*) printf "  ${Y}◐ local  ${N} #%-3s %s\n" "$id" "$desc" ;;
        *)          printf "  ${D}○ todo   ${N} #%-3s %s${N}\n" "$id" "$desc" ;;
      esac ;;
  esac
done
hr

# ── 3. git ───────────────────────────────────────────────────────────────────
printf "${B}GIT${N}\n"
un=$(git status --porcelain 2>/dev/null | wc -l | tr -d ' ')
ah=$(git log --oneline @{u}..HEAD 2>/dev/null | wc -l | tr -d ' ')
printf "  uncommitted %s   unpushed %s\n" "${un:-?}" "${ah:-?}"
printf "  ${D}HEAD %s${N}\n" "$(git log --oneline -1 2>/dev/null | cut -c1-58)"
hr

# ── 4. latest log tails ──────────────────────────────────────────────────────
printf "${B}LATEST RESULTS${N}\n"
found=0
for f in $(ls -t "$SCRATCH"/*.log 2>/dev/null | head -3); do
  line=$(grep -E "^test result|BUILD EXIT|CLIPPY_EXIT|^error|FAILED" "$f" 2>/dev/null | tail -1)
  [ -z "$line" ] && line=$(tail -1 "$f" 2>/dev/null)
  col="$D"; case "$line" in *"0 failed"*|*"EXIT: 0"*|*"EXIT=0"*) col="$G";; *FAILED*|*error*) col="$R";; esac
  printf "  %-26s ${col}%s${N}\n" "$(basename "$f" .log | cut -c1-26)" "$(printf '%s' "$line" | cut -c1-46)"
  found=1
done
[ "$found" = 0 ] && printf "  ${D}(no logs yet)${N}\n"

# ── 5. gates (opt-in — costs ~2s) ────────────────────────────────────────────
if [ "${1:-}" = "--gates" ]; then
  hr; printf "${B}GATES${N}\n"
  a=$(bash scripts/verify_anchors.sh 2>&1 | tail -1)
  s=$(python3 scripts/spec_lint.py 2>&1 | tail -1)
  case "$a" in *PASS*) printf "  ${G}%s${N}\n" "$a";; *) printf "  ${R}%s${N}\n" "$a";; esac
  case "$s" in *PASS*) printf "  ${G}%s${N}\n" "$s";; *) printf "  ${R}%s${N}\n" "$s";; esac
fi
