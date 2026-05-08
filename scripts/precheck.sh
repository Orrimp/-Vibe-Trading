#!/usr/bin/env bash
# Pre-flight checks before spawning the developer or accepting a tick.
#
# Catches:
#   1. Workspace package names that shadow Rust stdlib crates
#      (would break `cargo test --workspace --doc` on edition 2024).
#      Note: directory names may match — only the [package] `name = "..."`
#      field matters.
#   2. Untiked task summary for the slug under review.
#
# Usage:
#   scripts/precheck.sh                         # workspace-only
#   scripts/precheck.sh <feature-slug>          # also surface task ticks

set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
shadow_re='^name = "(core|std|alloc|test|proc_macro)"[[:space:]]*$'

# 1. Stdlib package-name clash — scan every crates/*/Cargo.toml [package] name.
clash="$(grep -RhE "$shadow_re" "$root/crates/" 2>/dev/null || true)"
if [[ -n "$clash" ]]; then
    echo "FAIL  workspace package name shadows a Rust stdlib crate:"
    echo "$clash" | sed 's/^/      /'
    echo "      Rename the [package] name (the directory may stay the same)."
    echo "      e.g. crates/core/Cargo.toml -> name = \"trading_core\""
    exit 1
fi

# 2. Optional slug task summary.
slug="${1:-}"
if [[ -n "$slug" ]]; then
    # Tasks live under per-feature folders. Lumen phases nest one level
    # deeper at spec/lumen-design-adoption/<phase>/tasks.md.
    f="$root/spec/$slug/tasks.md"
    if [[ ! -f "$f" ]]; then
        f="$root/spec/lumen-design-adoption/$slug/tasks.md"
    fi
    if [[ ! -f "$f" ]]; then
        echo "FAIL  no task file at spec/$slug/tasks.md (or spec/lumen-design-adoption/$slug/tasks.md)"
        exit 1
    fi
    open=$(grep -cE '^- \[ \]'   "$f" || true)
    done=$(grep -cE '^- \[x\]'   "$f" || true)
    final_done=$(grep -cE '^- \[x\][[:space:]]+\**T_FINAL' "$f" || true)
    final_open=$(grep -cE '^- \[ \][[:space:]]+\**T_FINAL' "$f" || true)
    echo "tasks for $slug: $done done / $open open  (T_FINAL: $final_done done, $final_open open)"
fi

echo "PRECHECK PASS"
