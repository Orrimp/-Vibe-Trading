#!/usr/bin/env bash
# scripts/orch_supplement_log.sh — append a verbatim-output section to a
# test-run log.
#
# Why this exists: when the test-runner sub-agent's sandbox denies certain
# bash invocations (verify_anchors.sh, check_no_clocks_in_ui_tests.sh, etc.),
# the orchestrator must re-run them and append verbatim output to the test-
# runner's log so the evaluator has one document to read. In the bootstrap
# session I wrote that heredoc inline (~80 lines). This script makes it a
# one-liner per command.
#
# Usage:
#   scripts/orch_supplement_log.sh <log-path> <section-title> -- <command> [args...]
#
# Example:
#   scripts/orch_supplement_log.sh test-run.log "verify_anchors" -- bash scripts/verify_anchors.sh
#
# Appended block format (matches the existing supplement convention):
#
#   ### <section-title>  (appended by orchestrator at <UTC>)
#   command: <command> <args>
#   verbatim stdout+stderr:
#   ```
#   <output>
#   ```
#   exit: <code>
#
# Exits non-zero only on argument errors; the underlying command's exit code
# is recorded in the log but does NOT propagate (use $? from inside if you
# want propagation).

set -uo pipefail

if [[ "$#" -lt 4 ]]; then
    echo "usage: $0 <log-path> <section-title> -- <command> [args...]" >&2
    exit 2
fi

log_path="$1"
section_title="$2"
shift 2

if [[ "${1:-}" != "--" ]]; then
    echo "orch_supplement_log: missing '--' separator before command" >&2
    exit 2
fi
shift

if [[ "$#" -lt 1 ]]; then
    echo "orch_supplement_log: no command supplied after '--'" >&2
    exit 2
fi

# Ensure log dir exists.
log_dir="$(dirname "$log_path")"
mkdir -p "$log_dir"

# Build a human-readable command echo (quote args with spaces).
cmd_echo=""
for arg in "$@"; do
    if [[ "$arg" == *" "* ]]; then
        cmd_echo+=" \"$arg\""
    else
        cmd_echo+=" $arg"
    fi
done
cmd_echo="${cmd_echo# }"  # strip leading space

ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

# Run the command and capture both streams + exit code.
out="$(set +e; "$@" 2>&1; echo "__ORCH_EXIT__=$?")"
exit_code="${out##*__ORCH_EXIT__=}"
out="${out%__ORCH_EXIT__=*}"
# Strip the trailing newline before the exit marker.
out="${out%$'\n'}"

# Append the section.
{
    echo ""
    echo "### ${section_title}  (appended by orchestrator at ${ts})"
    echo "command: ${cmd_echo}"
    echo "verbatim stdout+stderr:"
    echo '```'
    echo "$out"
    echo '```'
    echo "exit: ${exit_code}"
} >> "$log_path"

echo "appended section '${section_title}' to ${log_path} (cmd exit ${exit_code})"
