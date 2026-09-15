#!/usr/bin/env bash
# codegraph_bench — measure CodeGraph's Rust caller recall and precision on THIS repo.
#
# Ground truth is gt.py: a small Rust lexer, independent of both CodeGraph and grep.
# It skips comments, strings, char literals and #[attributes], and tracks brackets,
# so it knows each call's enclosing fn and whether the call sits inside a macro.
# metrics.py reads CodeGraph's SQLite index directly. Its `calls` edges were checked
# to match `codegraph callers` exactly, apart from crate-root `pub use` re-exports,
# which the CLI also lists and which are not calls.
#
# The index is deterministic (three re-indexes of one commit gave identical edge sets),
# so repeating a run changes nothing; re-run after an upgrade or a large code change.
#
# Usage:  scripts/codegraph_bench/run.sh [LABEL=path/to/codegraph.db ...]
#   no args        measures the installed index (.codegraph/codegraph.db, COPIED first — never written)
#   LABEL=db args  add further indexes side by side, e.g. one built by another CodeGraph version
set -euo pipefail
root=$(git rev-parse --show-toplevel); cd "$root"
here="$root/scripts/codegraph_bench"
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
for s in "" -wal -shm; do
  if [ -f ".codegraph/codegraph.db$s" ]; then cp ".codegraph/codegraph.db$s" "$tmp/installed.db$s"; fi
done
if [ ! -f "$tmp/installed.db" ]; then echo "no .codegraph/codegraph.db — run 'codegraph init' first" >&2; exit 1; fi
ver=$(codegraph --version 2>/dev/null || echo unknown)
python3 "$here/gt.py" "$root" "$tmp/gt.json" crates
python3 "$here/metrics.py" "$tmp/gt.json" "$root" "installed-$ver=$tmp/installed.db" "$@"
