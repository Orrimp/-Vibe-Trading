#!/usr/bin/env bash
# scripts/orch_cockpit_off.sh — kill the running cockpit and revert the
# cockpit.rs patch from orch_cockpit_on_screen.sh.
#
# Usage:
#   scripts/orch_cockpit_off.sh
#
# Effects:
#   1. Kill the cockpit PID from /tmp/orch-cockpit-state/cockpit.pid.
#   2. Revert cockpit.rs from cockpit.rs.bak (created by sed -i.bak).
#   3. Remove the state directory.
#
# Idempotent — exits 0 if no state to clean up.

set -uo pipefail

STATE_DIR="/tmp/orch-cockpit-state"
STATE_FILE="${STATE_DIR}/original-line.txt"
PID_FILE="${STATE_DIR}/cockpit.pid"
COCKPIT_RS="crates/ui/src/bin/cockpit.rs"

cd "$(dirname "$0")/.."

# Kill cockpit by PID file (preferred) + pkill fallback.
if [[ -f "$PID_FILE" ]]; then
    PID="$(cat "$PID_FILE")"
    if kill -0 "$PID" 2>/dev/null; then
        kill "$PID" 2>/dev/null || true
        sleep 1
    fi
fi
pkill -f "target/release/cockpit" 2>/dev/null || true
sleep 1

# Revert cockpit.rs from .bak.
if [[ -f "${COCKPIT_RS}.bak" ]]; then
    mv "${COCKPIT_RS}.bak" "$COCKPIT_RS"
    echo "reverted ${COCKPIT_RS}"
fi

# Clean state dir.
if [[ -d "$STATE_DIR" ]]; then
    rm -rf "$STATE_DIR"
    echo "cleaned $STATE_DIR"
fi

# Verify no diff against git for cockpit.rs.
if git diff --quiet -- "$COCKPIT_RS" 2>/dev/null; then
    echo "cockpit.rs clean against git"
else
    echo "orch_cockpit_off: WARNING — ${COCKPIT_RS} still differs from git HEAD"
    echo "Run: git diff $COCKPIT_RS  to inspect"
    exit 1
fi
