#!/usr/bin/env bash
# callers.sh — a COMPLETE caller census for a Rust symbol: CodeGraph ∪ grep.
#
# WHY: on this repo CodeGraph's caller lists are a LOWER BOUND
# (docs/dev-notes/codegraph.md § Correction 2026-09-14). It does not resolve
#   (1) a callee named through a module or crate name — `risk::size_portfolio_target(`,
#       `agent::spawn_aggregator(`, `frame::panel(` after `use crate::...::frame`; only
#       full `crate::` / `super::` paths mostly resolve; and
#   (2) any call inside a macro invocation — `format!("{}", w.file_slug())`,
#       `assert_eq!(x.slug(), ..)`.
# Census (scripts/codegraph_bench/run.sh, 2026-09-15): 71% of call sites found overall, 81% in
# production code; 9% of called functions are reported as having no callers at all.
# Measured on 1.1.0 AND on 1.6.0 (latest, 2026-08-26): both persist. Those are
# exactly where this repo's production callers live — CodeGraph alone reports
# `size_portfolio_target` as having no production caller.
#
# So any "no callers" / dead-code / reachability claim goes through this. It lists
# every grep call site and says whether CodeGraph knows the enclosing function.
# `✗` lines are the ones to READ before concluding anything. Grep has false
# positives too (string literals, same-named methods on other types), which is
# why they are labelled candidates, not callers.
#
# Usage:  scripts/callers.sh <symbol> [path...]     (default path: crates/; paths are repo-root-relative)
set -uo pipefail
# Run from the repo root, so the default path and CodeGraph's index resolve from any
# subdirectory. Path arguments are therefore repo-root-relative.
root=$(git rev-parse --show-toplevel 2>/dev/null) && cd "$root"
sym="${1:?usage: scripts/callers.sh <symbol> [path...]}"; shift
if [ "$#" -gt 0 ]; then paths=("$@"); else paths=(crates/); fi
strip() { sed $'s/\x1b\\[[0-9;]*m//g'; }

# CodeGraph's view: "file:line" of each enclosing caller function.
cg=""; cg_note=""
if command -v codegraph >/dev/null 2>&1; then
  cg_raw=$(codegraph callers "$sym" 2>&1 | strip)
  case "$cg_raw" in
    *"not initialized"*) cg_note="codegraph index missing here — grep-only census";;
    *) cg=$(printf '%s\n' "$cg_raw" | grep -oE '[A-Za-z0-9_./-]+\.rs:[0-9]+' | sort -u);;
  esac
else
  cg_note="codegraph not installed — grep-only census"
fi

# grep's view: every textual call site, minus definitions and full-line comments.
hits=$(grep -rnE "(^|[^A-Za-z0-9_])${sym}[[:space:]]*\(" --include='*.rs' "${paths[@]}" 2>/dev/null \
       | grep -vE "fn[[:space:]]+${sym}([^A-Za-z0-9_]|$)" \
       | grep -vE '^[^:]+:[0-9]+:[[:space:]]*//')

printf 'callers of `%s` — CodeGraph ∪ grep  (%s)\n' "$sym" "${paths[*]}"
[ -n "$cg_note" ] && printf '  NOTE: %s\n' "$cg_note"

total=0; covered=0; missed=0
while IFS= read -r h; do
  [ -z "$h" ] && continue
  file=${h%%:*}; rest=${h#*:}; line=${rest%%:*}; code=${rest#*:}
  # Enclosing fn = nearest preceding `fn <ident>` line; attributes and doc
  # comments are siblings in the syntax tree, so CodeGraph reports this line.
  encl=$(awk -v L="$line" 'NR>L{exit} !/^[[:space:]]*\/\// && /(^|[^A-Za-z0-9_])fn[[:space:]]+[A-Za-z_]/ {last=NR} END{print last+0}' "$file")
  total=$((total+1))
  if printf '%s\n' "$cg" | grep -qxF "${file}:${encl}"; then
    covered=$((covered+1)); mark="✓"; why=""
  else
    missed=$((missed+1)); mark="✗"; why=""
    case "$code" in
      *\"*"$sym"*\"*) why="  [inside a string literal?]";;
      *"::$sym"*)     why="  [path-qualified call]";;
      *"!("*)         why="  [inside a macro?]";;
    esac
  fi
  printf '  %s  %s:%s  (fn @%s)%s\n' "$mark" "$file" "$line" "$encl" "$why"
done <<< "$hits"

printf '\nsummary: %d call site(s) · %d in functions CodeGraph reports · %d grep-only' "$total" "$covered" "$missed"
[ "$missed" -gt 0 ] && printf '  ->  read the ✗ lines before any "no callers" claim'
printf '\n'
