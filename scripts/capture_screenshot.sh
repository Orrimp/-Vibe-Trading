#!/usr/bin/env bash
# Capture a screenshot of a Rust binary's window on macOS.
#
# Usage:
#   scripts/capture_screenshot.sh <binary> "<feature-args>" <output-path>
#
# Example:
#   scripts/capture_screenshot.sh cockpit "--features fixtures" \
#     spec/reports/screenshots/v0-paper-sma/tape-ready.png
#
# The script:
#   1. Launches `cargo run --release --bin <binary> <feature-args>` in the bg.
#   2. Sleeps 4s for the iced window to render.
#   3. Calls `screencapture -W <output>` (the operator clicks the window).
#   4. Falls back to `screencapture -x <output>` (full screen, no input)
#      if `-W` fails or there is no operator to click.
#   5. Kills the binary.
#
# macOS-only. On Linux, callers should fall back to the operator-instruction
# path documented in `.claude/skills/capture-screenshot/SKILL.md`.

set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "FAIL  capture_screenshot.sh: macOS-only. Caller should emit instruction block instead." >&2
    exit 2
fi

if [[ "$#" -lt 3 ]]; then
    echo "usage: $0 <binary> \"<feature-args>\" <output-path>" >&2
    exit 2
fi

binary="$1"
feature_args="$2"
output="$3"

mkdir -p "$(dirname "$output")"

# Launch in background; remember pid so we can clean up.
# shellcheck disable=SC2086
cargo run --release --bin "$binary" $feature_args >/tmp/"capture-$binary".log 2>&1 &
bin_pid="$!"

cleanup() {
    if kill -0 "$bin_pid" 2>/dev/null; then
        kill "$bin_pid" 2>/dev/null || true
        wait "$bin_pid" 2>/dev/null || true
    fi
}
trap cleanup EXIT

sleep 4

# Try window-pick first. Falls through to full-screen if no operator clicks
# within ~10s (screencapture -W blocks until a click; we timeout via `gtimeout`
# if installed, otherwise we go straight to full-screen).
if command -v gtimeout >/dev/null 2>&1; then
    if gtimeout 10s screencapture -W "$output"; then
        echo "captured (window) -> $output"
        exit 0
    fi
fi

if screencapture -x "$output"; then
    echo "captured (full screen) -> $output"
    exit 0
fi

echo "FAIL  screencapture failed; see /tmp/capture-$binary.log for binary output" >&2
exit 1
