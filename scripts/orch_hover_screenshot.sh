#!/usr/bin/env bash
# scripts/orch_hover_screenshot.sh — move cursor to (x, y), wait for the
# focused window's hover state to settle, capture a screenshot.
#
# Why this exists: the most painful pattern from the chart-canvas-overhaul
# session — needed 4 cycles to get tooltip-on-hover captured. Each cycle
# was: write Swift, compile inline, sleep, screencapture, kill, revert
# cockpit patch. This script bundles it.
#
# Prerequisites:
#   1. macOS Screen Recording permission granted to the host (Terminal/
#      VS Code/etc). Probe with: scripts/orch_probe_tcc.sh
#   2. A cockpit running on the right screen — typically launched via
#      scripts/orch_cockpit_on_screen.sh Charts.
#   3. swift in $PATH (ships with Xcode CLT).
#
# Usage:
#   scripts/orch_hover_screenshot.sh <x> <y> <output.png> [hold-ms]
#
# Where:
#   <x> <y>        Cursor target in LOGICAL macOS screen coordinates.
#                  Note: Retina logical = framebuffer/scale_factor.
#   <output.png>   Where to write the captured image.
#   [hold-ms]      Optional. ms to hold cursor before capture (default
#                  1500). Increase if the tooltip is slow to render or
#                  if iced needs more time to drain the event queue.
#
# Effects:
#   1. Pre-check: cockpit running, screencapture works, swift available.
#   2. Dispatch CursorMoved via swift orch_cursor_move.swift (twice with
#      a 1-px jiggle — some iced versions ignore the second-identical-pos
#      CursorMoved as a no-op, so the jiggle ensures the queue isn't
#      coalesced).
#   3. sleep <hold-ms>.
#   4. screencapture -x <output.png>.
#   5. Print the output path + dimensions.
#
# Exit codes:
#   0 — captured
#   2 — usage / prerequisite missing
#   3 — capture failed

set -uo pipefail

if [[ "$#" -lt 3 ]]; then
    echo "usage: $0 <x> <y> <output.png> [hold-ms]" >&2
    exit 2
fi

X="$1"
Y="$2"
OUT="$3"
HOLD_MS="${4:-1500}"

cd "$(dirname "$0")/.."

# Pre-checks.
if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "orch_hover_screenshot: macOS-only" >&2
    exit 2
fi
if ! command -v swift >/dev/null; then
    echo "orch_hover_screenshot: 'swift' not in PATH (install Xcode CLT)" >&2
    exit 2
fi
if [[ ! -f scripts/orch_cursor_move.swift ]]; then
    echo "orch_hover_screenshot: scripts/orch_cursor_move.swift missing" >&2
    exit 2
fi
if ! pgrep -f "target/release/cockpit" >/dev/null; then
    echo "orch_hover_screenshot: WARNING — no cockpit running" >&2
    echo "  launch first via: scripts/orch_cockpit_on_screen.sh Charts" >&2
    # Continue anyway — operator may have a debug build running.
fi

# Dispatch CursorMoved twice with a 1-px jiggle.
swift scripts/orch_cursor_move.swift "$X" "$Y" >/dev/null
sleep 0.2
swift scripts/orch_cursor_move.swift "$(awk -v x="$X" 'BEGIN { print x+1 }')" "$Y" >/dev/null

# Hold.
sleep_seconds="$(awk -v ms="$HOLD_MS" 'BEGIN { printf "%.3f", ms/1000.0 }')"
sleep "$sleep_seconds"

# Capture.
mkdir -p "$(dirname "$OUT")"
if ! screencapture -x "$OUT"; then
    echo "orch_hover_screenshot: screencapture failed" >&2
    exit 3
fi

# Report dimensions for sanity.
if command -v file >/dev/null; then
    dims="$(file "$OUT" | sed -n 's/.*PNG image data, \([0-9]* x [0-9]*\).*/\1/p')"
    echo "captured -> $OUT  ($dims, cursor at ($X, $Y), held ${HOLD_MS}ms)"
else
    echo "captured -> $OUT"
fi
