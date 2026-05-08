#!/usr/bin/env bash
# Prune duplicate backtest reports under spec/reports/, keeping ONE file
# per anchor scenario.
#
# Policy (option A + C):
# - For each scenario in spec/anchors.toml, the surviving file is the
#   *oldest* report on disk whose body-SHA matches the locked anchor.
#   The surviving filename's timestamp therefore records when the
#   current canonical body was first produced; idempotent re-runs do
#   not churn the timestamp.
# - Every other backtest-*-<scenario>.md is deleted: either it is a
#   redundant duplicate of the canonical body, or it is a stale run
#   produced before the anchor was updated. Audit trail lives in git.
# - Success-report scenarios (success/success-*-<scenario>.md) are
#   skipped — those filenames have no timestamp prefix and already
#   overwrite in place.
#
# Usage:
#   scripts/prune_backtest_duplicates.sh             # apply
#   scripts/prune_backtest_duplicates.sh --dry-run   # report only
#
# Invoked by the tester after verify_anchors.sh exits 0. Idempotent.

set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
anchors="$root/spec/anchors.toml"
hasher="$root/scripts/hash_report.py"
dry_run=0

if [[ "${1:-}" == "--dry-run" ]]; then
    dry_run=1
fi

[[ -f "$anchors" ]] || { echo "missing $anchors" >&2; exit 2; }
[[ -x "$hasher"  ]] || { echo "missing $hasher (chmod +x?)" >&2; exit 2; }

kept=0
removed=0
scenario=""

while IFS= read -r line; do
    if [[ "$line" =~ ^[[:space:]]*scenario[[:space:]]*=[[:space:]]*\"([^\"]+)\" ]]; then
        scenario="${BASH_REMATCH[1]}"
        continue
    fi
    if [[ "$line" =~ ^[[:space:]]*sha256[[:space:]]*=[[:space:]]*\"([a-f0-9]{64})\" ]]; then
        expected="${BASH_REMATCH[1]}"

        files=()
        while IFS= read -r f; do
            [[ -n "$f" ]] && files+=("$f")
        done < <(ls -1 "$root"/spec/reports/backtest-*-"$scenario".md 2>/dev/null | sort || true)

        if [[ "${#files[@]}" -eq 0 ]]; then
            continue
        fi

        keeper=""
        for f in "${files[@]}"; do
            actual="$(python3 "$hasher" "$f" | awk '{print $1}')"
            if [[ "$actual" == "$expected" ]]; then
                keeper="$f"
                break
            fi
        done

        if [[ -z "$keeper" ]]; then
            printf 'SKIP  %-36s  no file matches anchor (%s)\n' "$scenario" "$expected" >&2
            continue
        fi

        for f in "${files[@]}"; do
            if [[ "$f" == "$keeper" ]]; then
                kept=$((kept + 1))
                printf 'KEEP  %s\n' "${f#"$root"/}"
            else
                removed=$((removed + 1))
                if [[ "$dry_run" -eq 1 ]]; then
                    printf 'DRY   would remove  %s\n' "${f#"$root"/}"
                else
                    rm -f "$f"
                    printf 'RM    %s\n' "${f#"$root"/}"
                fi
            fi
        done
    fi
done < "$anchors"

echo "---"
if [[ "$dry_run" -eq 1 ]]; then
    echo "PRUNE DRY-RUN  kept=$kept  would-remove=$removed"
else
    echo "PRUNE PASS  kept=$kept  removed=$removed"
fi
