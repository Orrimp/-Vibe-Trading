#!/usr/bin/env bash
# scripts/orch_crop.sh — crop a PNG with (x, y, w, h) order.
#
# Why this exists: macOS `sips --cropOffset` takes (Y, X) order, not (X, Y).
# I (orchestrator) miscalled this twice in the chart-canvas-overhaul session.
# This wrapper enforces the conventional (X, Y, W, H) order.
#
# Usage:
#   scripts/orch_crop.sh <png-in> <x> <y> <w> <h> <png-out>
#
# Example:
#   scripts/orch_crop.sh diag.png 3300 250 900 500 legend-region.png
#
# Exits non-zero on any failure (set -euo pipefail).

set -euo pipefail

if [[ "$#" -ne 6 ]]; then
    echo "usage: $0 <png-in> <x> <y> <w> <h> <png-out>" >&2
    exit 2
fi

in="$1"
x="$2"
y="$3"
w="$4"
h="$5"
out="$6"

if [[ ! -f "$in" ]]; then
    echo "orch_crop: input not found: $in" >&2
    exit 2
fi

# sips --cropOffset is (offsetY offsetX); --cropToHeightWidth is (height width).
# We translate from (x,y,w,h) to sips's order here.
sips -c "$h" "$w" --cropOffset "$y" "$x" "$in" --out "$out" >/dev/null
echo "cropped $in [$x,$y,${w}x${h}] -> $out"
