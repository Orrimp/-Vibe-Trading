#!/usr/bin/env bash
# Mechanical enforcement of the presenter agent's "approval boxes ship
# UN-TICKED" rule.  Use this after the presenter writes a presentation
# file; it fails non-zero if ANY approval-block checkbox is pre-ticked
# `[x]`.  The operator is the only one who ticks.
#
# Background: the presenter agent's first two production fires both
# shipped with `[x] Approved — ship` pre-ticked, despite the agent
# definition having been doc-hardened after the first incident.  The
# agent self-claims to verify but doesn't.  This script is the gate
# that doesn't take the agent's word for it.
#
# Usage:
#   scripts/check_presentation.sh <path-to-presentation.md>
#
# Exit codes:
#   0  — clean, all approval boxes UN-ticked
#   1  — at least one approval box is pre-ticked `[x]`
#   2  — usage error (missing arg or file not found)
#
# Wired into:
#   - .claude/skills/present-results/SKILL.md (post-write step; the deck seam —
#     tech-writer persona since the BMAD migration; the retired presenter agent
#     definition is archived at docs/archive/pre-bmad-agents/presenter.md)

set -euo pipefail

if [[ "$#" -lt 1 ]]; then
    echo "usage: $0 <presentation.md>" >&2
    exit 2
fi

target="$1"
if [[ ! -f "$target" ]]; then
    echo "FAIL  not a file: $target" >&2
    exit 2
fi

# Match any of the three approval-block checkbox lines pre-ticked.
# The exact strings the presenter is supposed to write (UN-ticked):
#   - [ ] Approved — ship
#   - [ ] Approve with notes (notes below)
#   - [ ] Reject — _add reason below_
#
# A pre-tick replaces `[ ]` with `[x]` (lower-case is what the agent
# emits; we also catch `[X]` defensively).
shopt -s nocasematch || true

violations=$(grep -nE '^- \[(x|X)\] (Approved|Approve with notes|Reject)' "$target" || true)

if [[ -n "$violations" ]]; then
    printf 'FAIL  presenter pre-ticked an approval box in:\n      %s\n\n' "$target" >&2
    printf '%s\n' "$violations" >&2
    printf '\n      The operator is the only one who ticks.  Reset the box\n' >&2
    printf '      to `- [ ]` and re-run this gate.\n' >&2
    exit 1
fi

echo "PRESENTATION CHECK PASS  ($target — approval block UN-ticked)"
exit 0
