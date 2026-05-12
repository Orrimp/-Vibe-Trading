#!/usr/bin/env bash
# scripts/orch_cockpit_on_screen.sh — temporarily patch cockpit.rs to default
# to a non-Home screen, build, launch in background.
#
# Why this exists: in the chart-canvas-overhaul + bootstrap sessions I
# repeatedly patched `crates/ui/src/bin/cockpit.rs:158` from `Screen::Home`
# to `Screen::Charts`, built, launched, captured, then reverted. The dance
# was ~10 lines of sed + cargo + revert each time, and the revert step
# occasionally got skipped under interruption. This script encodes the
# pattern with guaranteed cleanup via a state file.
#
# Usage:
#   scripts/orch_cockpit_on_screen.sh <screen>
#
# Where <screen> is one of:
#   Home, Debug, Strategies, Risk, Audit, Charts, Control
#
# Effects:
#   1. Stash the current cockpit.rs:158 line in /tmp/orch-cockpit-state/
#      (or skip if state file already present — means a previous invocation
#      didn't clean up; refuse to layer patches).
#   2. Patch cockpit.rs:158 to `Screen::<arg>`.
#   3. cargo build --release -p ui --bin cockpit --features fixtures.
#   4. Launch ./target/release/cockpit in background, log to
#      /tmp/orch-cockpit-state/cockpit.log, write PID to
#      /tmp/orch-cockpit-state/cockpit.pid.
#   5. Sleep 6s (cockpit boot + initial render).
#   6. Print PID + log path + a reminder to run orch_cockpit_off.sh.
#
# Companion: scripts/orch_cockpit_off.sh kills the process + reverts.
#
# Exit codes:
#   0 — running, ready for screenshot/cursor automation
#   2 — usage error or state-file conflict (prior invocation didn't clean up)
#   3 — build or launch failed

set -euo pipefail

VALID_SCREENS=(Home Debug Strategies Risk Audit Charts Control)
SCREEN="${1:-}"
STATE_DIR="/tmp/orch-cockpit-state"
STATE_FILE="${STATE_DIR}/original-line.txt"
PID_FILE="${STATE_DIR}/cockpit.pid"
LOG_FILE="${STATE_DIR}/cockpit.log"
COCKPIT_RS="crates/ui/src/bin/cockpit.rs"
PATCH_LINE_PATTERN="cockpit.current_screen = Screen::"

if [[ -z "$SCREEN" ]]; then
    echo "usage: $0 <screen>" >&2
    echo "valid screens: ${VALID_SCREENS[*]}" >&2
    exit 2
fi

valid=0
for s in "${VALID_SCREENS[@]}"; do
    [[ "$s" == "$SCREEN" ]] && valid=1 && break
done
if (( !valid )); then
    echo "orch_cockpit_on_screen: invalid screen '$SCREEN'" >&2
    echo "valid: ${VALID_SCREENS[*]}" >&2
    exit 2
fi

cd "$(dirname "$0")/.."

if [[ ! -f "$COCKPIT_RS" ]]; then
    echo "orch_cockpit_on_screen: $COCKPIT_RS not found" >&2
    exit 2
fi

# Refuse to re-patch if a state file is present (means prior invocation
# didn't clean up). Operator runs orch_cockpit_off.sh first.
if [[ -f "$STATE_FILE" ]]; then
    echo "orch_cockpit_on_screen: state file already present at $STATE_FILE" >&2
    echo "Run scripts/orch_cockpit_off.sh first to revert the prior patch." >&2
    exit 2
fi

mkdir -p "$STATE_DIR"

# Find the line, stash it, replace with the new screen.
original_line="$(grep -n "$PATCH_LINE_PATTERN" "$COCKPIT_RS" | head -1)"
if [[ -z "$original_line" ]]; then
    echo "orch_cockpit_on_screen: no '$PATCH_LINE_PATTERN' line in $COCKPIT_RS" >&2
    exit 3
fi
echo "$original_line" > "$STATE_FILE"

# sed -i.bak in-place edit; keep .bak to double-defend revert path.
sed -i.bak "s|cockpit.current_screen = Screen::[A-Za-z]*;|cockpit.current_screen = Screen::${SCREEN};|" "$COCKPIT_RS"

echo "patched $COCKPIT_RS to default to Screen::${SCREEN}"

# Kill any prior cockpit instance just in case.
pkill -f "target/release/cockpit" 2>/dev/null || true
sleep 1

# Build.
echo "building..."
if ! cargo build --release -p ui --bin cockpit --features fixtures > "$LOG_FILE" 2>&1; then
    echo "orch_cockpit_on_screen: build failed; see $LOG_FILE" >&2
    # Revert before exiting.
    mv "${COCKPIT_RS}.bak" "$COCKPIT_RS"
    rm -f "$STATE_FILE"
    exit 3
fi

# Launch in background.
./target/release/cockpit >> "$LOG_FILE" 2>&1 &
PID=$!
echo "$PID" > "$PID_FILE"
echo "launched cockpit pid=$PID, log=$LOG_FILE"

# Sleep for boot + first paint.
sleep 6

# Confirm it's still running.
if ! kill -0 "$PID" 2>/dev/null; then
    echo "orch_cockpit_on_screen: cockpit exited within 6s; see $LOG_FILE" >&2
    mv "${COCKPIT_RS}.bak" "$COCKPIT_RS"
    rm -f "$STATE_FILE" "$PID_FILE"
    exit 3
fi

echo "cockpit running on Screen::${SCREEN}"
echo "next steps:"
echo "  - capture: screencapture -x out.png"
echo "  - hover:   scripts/orch_hover_screenshot.sh <x> <y> out.png"
echo "  - cleanup: scripts/orch_cockpit_off.sh"
