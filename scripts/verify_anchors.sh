#!/usr/bin/env bash
# Verify every entry in spec/anchors.toml against the matching backtest
# report under spec/reports/. Prints PASS/FAIL per scenario,
# exits non-zero on any mismatch or missing report.
#
# Usage: scripts/verify_anchors.sh
#
# This is the regression gate the tester MUST run before VERDICT -> PASS.
# It is also wired into the `verify-anchors` skill.
#
# v5 v0.2.0 amendment (ADR-0045 D2 / T-D-N5):
# When spec/anchors.toml carries two-namespace rows (noop-baseline + canonical),
# the `version` field disambiguates which report to verify against:
#   - version contains "+ noop-baseline": use the NEWEST matching report
#     OUTSIDE any canonical migration folder (sort | tail -1, excluding
#     spec/v5-latency-slippage-sim-v0.*.0-*/reports/).
#   - version contains "+ v5-realdata-medium-2026-05": use the NEWEST matching
#     report from canonical migration folders first (v0.3.0 preferred over v0.2.0),
#     then fall back to the global newest.
#   - all other versions: use the NEWEST matching report (legacy default).
#
# v5 v0.3.0 amendment (ADR-0047 D3 / T-D-N9):
# Canonical migration dirs expanded to include v0.3.0-full-path-wiring.
# The resolver first checks v0.3.0 dir, then v0.2.0 dir (newest wins).
#
# v5 v0.4.0 amendment (2026-05-28):
# Canonical migration dirs expanded to include v0.4.0-candle-feature-gated-re-emit.
# The resolver first checks v0.4.0 dir, then v0.3.0 dir, then v0.2.0 dir (newest wins).

set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
anchors="$root/spec/anchors.toml"
hasher="$root/scripts/hash_report.py"
migration_dir_v02="$root/spec/v5-latency-slippage-sim-v0.2.0-anchor-migration"
migration_dir_v03="$root/spec/v5-latency-slippage-sim-v0.3.0-full-path-wiring"
migration_dir_v04="$root/spec/v5-latency-slippage-sim-v0.4.0-candle-feature-gated-re-emit"
# Combined pattern for excluding all canonical dirs from noop-baseline search:
canonical_dirs_pattern="$root/spec/v5-latency-slippage-sim-v0"

[[ -f "$anchors" ]] || { echo "missing $anchors" >&2; exit 2; }
[[ -x "$hasher"  ]] || { echo "missing $hasher (chmod +x?)" >&2; exit 2; }

fail=0
total=0
scenario=""
version=""

while IFS= read -r line; do
    if [[ "$line" =~ ^[[:space:]]*scenario[[:space:]]*=[[:space:]]*\"([^\"]+)\" ]]; then
        scenario="${BASH_REMATCH[1]}"
        version=""   # reset version each time scenario resets
        continue
    fi
    if [[ "$line" =~ ^[[:space:]]*version[[:space:]]*=[[:space:]]*\"([^\"]+)\" ]]; then
        version="${BASH_REMATCH[1]}"
        continue
    fi
    if [[ "$line" =~ ^[[:space:]]*sha256[[:space:]]*=[[:space:]]*\"([a-f0-9]{64})\" ]]; then
        expected="${BASH_REMATCH[1]}"
        total=$((total + 1))
        # Resolve the report file for this scenario.
        # Reports live under per-feature folders:
        #     spec/<feature>/reports/backtest-<stamp>-<scenario>.md   (backtest)
        #     spec/<feature>/reports/success-<stamp>-<scenario>.md    (success, T816)
        #     spec/<feature>/reports/<scenario>-<stamp>.md            (investigation, T-T-1)
        #
        # Namespace-aware selection (v5 v0.2.0 ADR-0045 D2):
        #   noop-baseline version → NEWEST report OUTSIDE migration folder
        #   canonical version     → NEWEST report in migration folder first,
        #                           then fall back to global newest
        #   other versions        → NEWEST report (legacy default)

        latest=""

        if [[ "$version" == *"noop-baseline"* ]]; then
            # noop-baseline: find newest report OUTSIDE all canonical migration folders
            latest="$(find "$root"/spec -type f -path "*/reports/backtest-*-$scenario.md" \
                ! -path "${canonical_dirs_pattern}*" 2>/dev/null | sort | tail -1 || true)"
            if [[ -z "$latest" ]]; then
                latest="$(find "$root"/spec -type f -path "*/reports/success-*-$scenario.md" \
                    ! -path "${canonical_dirs_pattern}*" 2>/dev/null | sort | tail -1 || true)"
            fi
            if [[ -z "$latest" ]]; then
                latest="$(find "$root"/spec -type f -path "*/reports/$scenario-*.md" \
                    ! -path "${canonical_dirs_pattern}*" 2>/dev/null \
                    | grep -E "/reports/${scenario}-[0-9]+\.md$" \
                    | sort | tail -1 || true)"
            fi
        elif [[ "$version" == *"v5-realdata-medium-2026-05"* ]]; then
            # canonical: prefer v0.4.0 migration dir, then v0.3.0, then v0.2.0, then global newest
            latest="$(find "$migration_dir_v04" -type f -name "backtest-*-$scenario.md" \
                2>/dev/null | sort | tail -1 || true)"
            if [[ -z "$latest" ]]; then
                latest="$(find "$migration_dir_v03" -type f -name "backtest-*-$scenario.md" \
                    2>/dev/null | sort | tail -1 || true)"
            fi
            if [[ -z "$latest" ]]; then
                latest="$(find "$migration_dir_v02" -type f -name "backtest-*-$scenario.md" \
                    2>/dev/null | sort | tail -1 || true)"
            fi
            if [[ -z "$latest" ]]; then
                # canonical SHA = noop SHA for non-re-emittable scenarios
                latest="$(find "$root"/spec -type f -path "*/reports/backtest-*-$scenario.md" \
                    2>/dev/null | sort | tail -1 || true)"
            fi
            if [[ -z "$latest" ]]; then
                latest="$(find "$root"/spec -type f -path "*/reports/success-*-$scenario.md" \
                    2>/dev/null | sort | tail -1 || true)"
            fi
            if [[ -z "$latest" ]]; then
                latest="$(find "$root"/spec -type f -path "*/reports/$scenario-*.md" \
                    2>/dev/null \
                    | grep -E "/reports/${scenario}-[0-9]+\.md$" \
                    | sort | tail -1 || true)"
            fi
        else
            # Legacy default: newest matching report anywhere
            latest="$(find "$root"/spec -type f -path "*/reports/backtest-*-$scenario.md" \
                2>/dev/null | sort | tail -1 || true)"
            if [[ -z "$latest" ]]; then
                latest="$(find "$root"/spec -type f -path "*/reports/success-*-$scenario.md" \
                    2>/dev/null | sort | tail -1 || true)"
            fi
            if [[ -z "$latest" ]]; then
                latest="$(find "$root"/spec -type f -path "*/reports/$scenario-*.md" \
                    2>/dev/null \
                    | grep -E "/reports/${scenario}-[0-9]+\.md$" \
                    | sort | tail -1 || true)"
            fi
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
