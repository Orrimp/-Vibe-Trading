#!/usr/bin/env bash
# RETIRED 2026-11-24 (bug-log #228). This script deletes nothing and always exits 2.
#
# It used to "prune duplicate backtest reports": for every `scenario` + `sha256` row of
# evidence/anchors.toml it kept the first `evidence/**/reports/backtest-*-<scenario>.md`
# whose body matched THAT row and `rm -f`'d every other match. It was not namespace-aware,
# so a scenario's later rows lost the bodies its earlier row did not keep. Simulated over
# the committed tree (in memory, nothing deleted), a real run would have removed 155
# tracked files, 31 of them bodies `scripts/verify_anchors.sh` resolves, with 27 rows
# SKIPped; its `--dry-run` printed per row and deleted nothing between rows, so it was
# not a faithful preview. A "skip tracked files" guard is not a fix either: a freshly
# emitted ADR-0038 § D6.b body is UNTRACKED until committed, and it would have been
# deleted with no copy anywhere.
#
# Nothing replaces it. Removing a file from evidence/ is an operator decision, made by hand
# and committed. Bug-log #210 (ADR-0104) moves the `backtest` bin's default output out of
# `evidence/` into the git-ignored `backtest-runs/`, so a default run leaves nothing there
# to prune; a body emitted into the corpus with `--reports-dir` is deliberate.
# The old code is in git history (`git log -- scripts/prune_backtest_duplicates.sh`).
# Do not add a flag that re-enables deletion.
#
# Usage:
#   scripts/prune_backtest_duplicates.sh [anything]   # refuses: message on stderr, exit 2
#   scripts/prune_backtest_duplicates.sh --self-test  # proves the refusal in a temp tree
#
# `--self-test` (wired into ci.yml's "Governance — AD-2 anchors" step) copies this script
# into a temp tree holding an anchors.toml with two rows for one scenario and two reports
# with different bodies (the shape the old loop deleted from), runs the copy with no
# argument, `--dry-run` and an arbitrary argument, and asserts each exits 2 with the
# REFUSED line and leaves the tree byte-identical (names, sizes, mtimes, contents). A
# control first shows the snapshot sees a deletion, so the comparison can fail (#187).

set -euo pipefail

REFUSED_LINE="REFUSED (bug-log #228): scripts/prune_backtest_duplicates.sh is retired and deletes nothing."

refuse() {
    {
        echo "$REFUSED_LINE"
        echo "It rm -f'd committed anchored bodies of other anchor namespaces (155 tracked files on a"
        echo "simulated run, 31 of them bodies verify_anchors.sh resolves). Removing a file from"
        echo "evidence/ is an operator decision, made by hand and committed. Bug-log #210 (ADR-0104)"
        echo "moves the backtest CLI's default output out of evidence/, into git-ignored backtest-runs/."
    } >&2
    exit 2
}

# One line per entry under $1: relative path, kind, size, mtime (ns), sha256 of a file.
snapshot() {
    python3 -I - "$1" <<'PY'
import hashlib, os, sys
root = sys.argv[1]
out = []
for d, dirs, files in os.walk(root, onerror=lambda e: sys.exit(f"snapshot: {e}")):
    dirs.sort()
    for name in [""] + sorted(dirs) + sorted(files):
        p = os.path.join(d, name) if name else d
        st = os.lstat(p)
        digest = "-"
        if os.path.isfile(p) and not os.path.islink(p):
            with open(p, "rb") as fh:
                digest = hashlib.sha256(fh.read()).hexdigest()
        out.append(f"{os.path.relpath(p, root)}\t{oct(st.st_mode)}\t{st.st_size}\t{st.st_mtime_ns}\t{digest}")
print("\n".join(sorted(set(out))))
PY
}

self_test() {
    local tmp
    tmp="$(mktemp -d)"
    # shellcheck disable=SC2064
    trap "rm -rf '$tmp'" EXIT
    local tree="$tmp/tree"
    mkdir -p "$tree/scripts" "$tree/evidence/v1/f/reports" "$tree/evidence/f/reports"
    cp "$0" "$tree/scripts/prune_backtest_duplicates.sh"
    cat > "$tree/evidence/anchors.toml" <<'TOML'
[[anchor]]
scenario = "s"
sha256   = "1111111111111111111111111111111111111111111111111111111111111111"
version  = "a"

[[anchor]]
scenario = "s"
sha256   = "2222222222222222222222222222222222222222222222222222222222222222"
version  = "b"
TOML
    printf -- '---\nscenario: s\n---\nbody one\n' > "$tree/evidence/v1/f/reports/backtest-20260101-000000-s.md"
    printf -- '---\nscenario: s\n---\nbody two\n' > "$tree/evidence/f/reports/backtest-20261124-000000-s.md"

    # Control (#187): the snapshot sees a deletion, so an equal snapshot below means something.
    local ctl="$tmp/control"
    mkdir -p "$ctl/reports"
    printf 'x\n' > "$ctl/reports/backtest-1-s.md"
    local c1 c2
    c1="$(snapshot "$ctl")"
    rm -f "$ctl/reports/backtest-1-s.md"
    c2="$(snapshot "$ctl")"
    if [[ "$c1" == "$c2" ]]; then
        echo "prune self-test: FAIL — the snapshot did not see a deleted file (control)" >&2
        exit 1
    fi

    local before after rc err n=0
    before="$(snapshot "$tree")"
    for args in "" "--dry-run" "--apply"; do
        set +e
        # shellcheck disable=SC2086
        err="$(cd "$tree" && bash "$tree/scripts/prune_backtest_duplicates.sh" $args 2>&1 >/dev/null)"
        rc=$?
        set -e
        if [[ "$rc" -ne 2 ]]; then
            echo "prune self-test: FAIL — '${args:-<no argument>}' exited $rc, expected 2" >&2
            exit 1
        fi
        if [[ "$err" != *"$REFUSED_LINE"* ]]; then
            echo "prune self-test: FAIL — '${args:-<no argument>}' did not print the REFUSED line:" >&2
            echo "$err" >&2
            exit 1
        fi
        n=$((n + 1))
    done
    after="$(snapshot "$tree")"
    if [[ "$before" != "$after" ]]; then
        echo "prune self-test: FAIL — a refused run changed the tree:" >&2
        diff <(echo "$before") <(echo "$after") >&2 || true
        exit 1
    fi
    echo "prune self-test: PASS — $n invocations refused (exit 2), tree unchanged ($(echo "$before" | wc -l | tr -d ' ') entries); control saw a deletion"
}

case "${1:-}" in
    --self-test) self_test ;;
    *) refuse ;;
esac
