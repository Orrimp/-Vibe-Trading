#!/usr/bin/env bash
# scripts/orch_probe_tcc.sh — probe macOS TCC permissions effective for THIS process tree.
#
# Why this exists: in the chart-canvas-overhaul session I conflated
# Accessibility, Automation (Apple Events → System Events), and Screen
# Recording multiple times. Each is a SEPARATE TCC bucket and a separate
# grant. This script tells me what's actually effective right now.
#
# Usage:
#   scripts/orch_probe_tcc.sh         # one-line status
#   scripts/orch_probe_tcc.sh -v      # verbose (include test invocations)
#
# Output (one-line):
#   screen-recording=YES accessibility=NO automation-system-events=YES
#
# Exit codes:
#   0 — at least one capability is granted
#   1 — all three denied
#   2 — usage error / not macOS

set -uo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "orch_probe_tcc: macOS-only" >&2
    exit 2
fi

verbose=0
if [[ "${1:-}" == "-v" ]]; then
    verbose=1
fi

# 1. Screen Recording — try `screencapture -x` to a temp file.
#    A denied process either fails or produces a black/empty frame; we
#    detect by file size + exit code. Faster than the OSAStatus path.
screen_recording="UNKNOWN"
tmp_png="$(mktemp -t orch-tcc-XXXXXX).png"
if screencapture -x -t png "$tmp_png" 2>/dev/null && [[ -s "$tmp_png" ]]; then
    # File exists and has non-zero size — likely granted.
    # Black-frame detection: a fully-black PNG from a denied capture is
    # typically <50KB regardless of resolution. Production captures are
    # 100KB+ on any Retina display.
    sz="$(stat -f%z "$tmp_png" 2>/dev/null || stat -c%s "$tmp_png")"
    if (( sz > 50000 )); then
        screen_recording="YES"
    else
        screen_recording="DENIED"
    fi
else
    screen_recording="DENIED"
fi
rm -f "$tmp_png"

# 2. Accessibility — query `process is trusted`. AppleScript shim:
#    `tell application "System Events" to get name` requires Automation;
#    that's a different bucket. For pure Accessibility, the canonical
#    probe is AXIsProcessTrusted() via Swift, but we approximate by
#    attempting a benign UI event (which fails -1719 without Accessibility).
accessibility="UNKNOWN"
ax_out="$(osascript -e 'tell application "System Events" to keystroke ""' 2>&1 || true)"
if [[ "$ax_out" == *"-1719"* ]] || [[ "$ax_out" == *"-25211"* ]]; then
    accessibility="DENIED"
elif [[ "$ax_out" == *"-1743"* ]]; then
    # -1743 = Automation denied (different bucket); Accessibility status
    # unknown but the keystroke would fail Automation first. Treat as
    # "blocked-by-automation" — Accessibility might be granted, but you
    # can't reach it through AppleScript without Automation.
    accessibility="BLOCKED-BY-AUTOMATION"
else
    # Empty stderr or success ⇒ Accessibility appears reachable.
    accessibility="YES"
fi

# 3. Automation → System Events specifically.
automation_se="UNKNOWN"
au_out="$(osascript -e 'tell application "System Events" to get name' 2>&1 || true)"
if [[ "$au_out" == *"-1743"* ]]; then
    automation_se="DENIED"
elif [[ "$au_out" == "System Events"* ]] || [[ "$au_out" == *"System Events"* ]]; then
    automation_se="YES"
else
    automation_se="UNKNOWN"
fi

# Compose status line.
status_line="screen-recording=${screen_recording} accessibility=${accessibility} automation-system-events=${automation_se}"
echo "$status_line"

if (( verbose )); then
    echo ""
    echo "--- verbose probe output ---"
    echo "AppleScript probe 1 (keystroke ''): $ax_out"
    echo "AppleScript probe 2 (get name):    $au_out"
fi

# Exit code: 0 if any granted, 1 if all denied.
if [[ "$screen_recording" == "YES" ]] || [[ "$accessibility" == "YES" ]] || [[ "$automation_se" == "YES" ]]; then
    exit 0
fi
exit 1
