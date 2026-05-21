#!/usr/bin/env bash
# Verify every entry in spec/anchors.toml against the latest matching
# backtest report under spec/reports/. Prints PASS/FAIL per scenario,
# exits non-zero on any mismatch or missing report.
#
# Usage: scripts/verify_anchors.sh
#
# This is the regression gate the tester MUST run before VERDICT -> PASS.
# It is also wired into the `verify-anchors` skill.

set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
anchors="$root/spec/anchors.toml"
hasher="$root/scripts/hash_report.py"

[[ -f "$anchors" ]] || { echo "missing $anchors" >&2; exit 2; }
[[ -x "$hasher"  ]] || { echo "missing $hasher (chmod +x?)" >&2; exit 2; }

fail=0
total=0
scenario=""

while IFS= read -r line; do
    if [[ "$line" =~ ^[[:space:]]*scenario[[:space:]]*=[[:space:]]*\"([^\"]+)\" ]]; then
        scenario="${BASH_REMATCH[1]}"
        continue
    fi
    if [[ "$line" =~ ^[[:space:]]*sha256[[:space:]]*=[[:space:]]*\"([a-f0-9]{64})\" ]]; then
        expected="${BASH_REMATCH[1]}"
        total=$((total + 1))
        # Resolve the latest report file for this scenario. Reports live
        # under per-feature folders:
        #     spec/<feature>/reports/backtest-<stamp>-<scenario>.md   (9 backtest)
        #     spec/<feature>/reports/success-<stamp>-<scenario>.md    (2 success, T816)
        #     spec/<feature>/reports/<scenario>-<stamp>.md            (investigation reports, T-T-1)
        # The script picks whichever pattern resolves first; all finds
        # are sorted independently and the lexicographically-largest
        # match wins (timestamp prefix → effectively "newest").
        #
        # Pattern 3 (investigation reports) requires a digit-only suffix
        # `<scenario>-[0-9]+.md` so that derivative report families with
        # extra word-segments before the timestamp (e.g.
        # `forecast-distribution-bs1-realdata-recalibrated-<date>.md`
        # shipped by v25-tcn-recalibrate) DON'T collide with the base
        # scenario's anchor when the base scenario name is a prefix of
        # the derivative's filename. Each derivative locks its own anchor
        # under its own scenario string.
        latest="$(find "$root"/spec -type f -path "*/reports/backtest-*-$scenario.md" 2>/dev/null | sort | tail -1 || true)"
        if [[ -z "$latest" ]]; then
            latest="$(find "$root"/spec -type f -path "*/reports/success-*-$scenario.md" 2>/dev/null | sort | tail -1 || true)"
        fi
        if [[ -z "$latest" ]]; then
            latest="$(find "$root"/spec -type f -path "*/reports/$scenario-*.md" 2>/dev/null \
                | grep -E "/reports/${scenario}-[0-9]+\.md$" \
                | sort | tail -1 || true)"
        fi
        if [[ -z "$latest" ]]; then
            printf 'MISS  %-36s  no report on disk\n' "$scenario"
            fail=1
            continue
        fi
        actual="$(python3 "$hasher" "$latest" | awk '{print $1}')"
        if [[ "$actual" == "$expected" ]]; then
            printf 'PASS  %-36s  %s\n' "$scenario" "$expected"
        else
            printf 'FAIL  %-36s\n      expected %s\n      actual   %s\n      file     %s\n' \
                "$scenario" "$expected" "$actual" "$latest"
            fail=1
        fi
    fi
done < "$anchors"

echo "---"
if [[ "$fail" -eq 0 ]]; then
    echo "ANCHORS PASS  ($total / $total)"
else
    echo "ANCHORS FAIL  (mismatches detected; route HANDOFF -> developer with body diff)"
fi
exit "$fail"
