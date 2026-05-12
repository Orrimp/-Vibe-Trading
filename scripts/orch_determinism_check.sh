#!/usr/bin/env bash
# scripts/orch_determinism_check.sh — run a cargo test twice, byte-diff the
# named output files.
#
# Why this exists: ui-test-harness-bootstrap H1 falsifier ("tiny-skia CPU
# determinism holds across two runs") was a recurring pattern: run the
# visual_snapshots test, shasum the baselines, run it again, shasum again,
# diff. This script generalizes it — any test + any output-file glob.
#
# Usage:
#   scripts/orch_determinism_check.sh -p <crate> --test <test-name> -- <baseline-glob>
#
# Example:
#   scripts/orch_determinism_check.sh -p ui --test visual_snapshots -- 'crates/ui/tests/visual-baselines/*.png'
#
# Exit codes:
#   0 — byte-identical across two runs (determinism holds)
#   1 — byte-diff detected (determinism falsified — log the diff)
#   2 — usage error / cargo test failed

set -uo pipefail

if [[ "$#" -lt 5 ]]; then
    echo "usage: $0 -p <crate> --test <test-name> -- <baseline-glob>" >&2
    exit 2
fi

crate=""
test_name=""
glob=""
state="flags"
while (( "$#" )); do
    case "$1" in
        -p) crate="$2"; shift 2;;
        --test) test_name="$2"; shift 2;;
        --) state="glob"; shift;;
        *)
            if [[ "$state" == "glob" ]]; then
                glob="$1"
                shift
            else
                echo "orch_determinism_check: unknown flag '$1'" >&2
                exit 2
            fi
            ;;
    esac
done

if [[ -z "$crate" ]] || [[ -z "$test_name" ]] || [[ -z "$glob" ]]; then
    echo "orch_determinism_check: -p, --test, and baseline-glob all required" >&2
    exit 2
fi

cd "$(dirname "$0")/.."

echo "--- run 1: cargo test -p $crate --test $test_name ---"
if ! cargo test -p "$crate" --test "$test_name" 2>&1 | tail -3; then
    echo "orch_determinism_check: run 1 cargo test failed" >&2
    exit 2
fi

# Note: glob expansion happens here in the shell.
# shellcheck disable=SC2086
shas_1="$(shasum -a 256 $glob 2>&1 | sort)"
echo ""
echo "SHA-256 after run 1:"
echo "$shas_1"

echo ""
echo "--- run 2: cargo test -p $crate --test $test_name ---"
if ! cargo test -p "$crate" --test "$test_name" 2>&1 | tail -3; then
    echo "orch_determinism_check: run 2 cargo test failed" >&2
    exit 2
fi

# shellcheck disable=SC2086
shas_2="$(shasum -a 256 $glob 2>&1 | sort)"
echo ""
echo "SHA-256 after run 2:"
echo "$shas_2"

echo ""
echo "--- diff ---"
if [[ "$shas_1" == "$shas_2" ]]; then
    echo "DETERMINISM PASS  (byte-identical across two runs)"
    exit 0
fi

echo "DETERMINISM FAIL  (byte-diff detected)"
diff <(echo "$shas_1") <(echo "$shas_2") || true
exit 1
