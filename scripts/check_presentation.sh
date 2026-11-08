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
#   scripts/check_presentation.sh --self-test
#
# Exit codes:
#   0  — clean: at least one approval box found, and all of them UN-ticked
#   1  — at least one approval box is pre-ticked `[x]`
#   2  — usage error (missing arg or file not found), OR no approval block at all
#
# bug-log #163's sibling, #164: until 2026-11-08 this script affirmed a property
# of a set it never proved non-empty. It grepped for PRE-TICKED boxes and printed
# PASS when it found none — so a deck with NO approval block at all, which is the
# likelier presenter failure than a pre-ticked one, certified clean. The count is
# now a precondition and it is printed, so a PASS states what it measured.
#
# Run `--self-test` to re-measure this gate's own teeth.
#
# Wired into:
#   - .claude/skills/present-results/SKILL.md (post-write step; the deck seam —
#     tech-writer persona since the BMAD migration; the retired presenter agent
#     definition is archived at docs/archive/pre-bmad-agents/presenter.md)

set -euo pipefail

# Deliberately WIDER than the violation regex below: it must match a box in
# EITHER state, so an off-template bullet cannot make the floor read zero and
# turn this gate back into the thing it was. `(\*\*)?` tolerates bolding.
BOX_RE='^[[:space:]]*[-*] \[[ xX]\][[:space:]]*(\*\*)?[[:space:]]*(Approved|Approve with notes|Reject)'
VIOLATION_RE='^- \[(x|X)\] (Approved|Approve with notes|Reject)'

count_boxes() { grep -cE "$BOX_RE" "$1" || true; }
find_violations() { grep -nE "$VIOLATION_RE" "$1" || true; }

self_test() {
    local tmpdir
    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' RETURN

    local no_block="$tmpdir/no-approval-block.md"
    local clean="$tmpdir/clean.md"
    local ticked="$tmpdir/pre-ticked.md"

    printf '# Deck\n\n## TL;DR\n\nShipped it.\n' > "$no_block"
    printf '# Deck\n\n## Approval\n\n- [ ] Approved \xe2\x80\x94 ship\n- [ ] Approve with notes (notes below)\n- [ ] Reject \xe2\x80\x94 _add reason below_\n' > "$clean"
    printf '# Deck\n\n## Approval\n\n- [x] Approved \xe2\x80\x94 ship\n- [ ] Reject \xe2\x80\x94 _add reason below_\n' > "$ticked"

    local rc=0

    # P1 — the floor: a deck with no approval block must NOT pass.
    if [[ "$(count_boxes "$no_block")" -ne 0 ]]; then
        echo "SELF-TEST FAIL: the no-block fixture counted boxes (BOX_RE too loose)" >&2
        rc=1
    else
        echo "  OK    floor: no-approval-block fixture counts 0 boxes -> exit 2"
    fi

    # P2 — a clean deck must count its boxes and have no violations.
    local n; n="$(count_boxes "$clean")"
    if [[ "$n" -ne 3 ]]; then
        echo "SELF-TEST FAIL: clean fixture counted $n boxes, expected 3 (BOX_RE too tight)" >&2
        rc=1
    elif [[ -n "$(find_violations "$clean")" ]]; then
        echo "SELF-TEST FAIL: clean fixture reported a violation (VIOLATION_RE too loose)" >&2
        rc=1
    else
        echo "  OK    control: clean fixture counts 3 boxes, 0 violations -> exit 0"
    fi

    # P3 — a pre-ticked deck must be caught, and still counted.
    if [[ -z "$(find_violations "$ticked")" ]]; then
        echo "SELF-TEST FAIL: pre-ticked fixture produced no violation (matcher too weak)" >&2
        rc=1
    elif [[ "$(count_boxes "$ticked")" -ne 2 ]]; then
        echo "SELF-TEST FAIL: pre-ticked fixture counted $(count_boxes "$ticked") boxes, expected 2" >&2
        rc=1
    else
        echo "  OK    pre-ticked fixture: 1 violation, 2 boxes counted -> exit 1"
    fi

    if [[ "$rc" -eq 0 ]]; then
        echo "PRESENTATION CHECK SELF-TEST PASS (3 probes)"
    else
        echo "PRESENTATION CHECK SELF-TEST FAIL" >&2
    fi
    return "$rc"
}

if [[ "${1:-}" == "--self-test" ]]; then
    self_test
    exit $?
fi

if [[ "$#" -lt 1 ]]; then
    echo "usage: $0 <presentation.md> | --self-test" >&2
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

# NON-VACUITY FLOOR, before anything else: a deck with no approval block cannot
# be certified "approval block UN-ticked".
boxes=$(count_boxes "$target")
if [[ "$boxes" -eq 0 ]]; then
    printf 'FAIL  no approval block found in:\n      %s\n\n' "$target" >&2
    printf '      Expected at least one line matching\n' >&2
    printf '        - [ ] Approved | Approve with notes | Reject\n\n' >&2
    printf '      A deck with no approval block cannot be certified UN-ticked.\n' >&2
    printf '      Until 2026-11-08 this script PASSED on exactly this input\n' >&2
    printf '      (bug-log #164).\n' >&2
    exit 2
fi

violations=$(find_violations "$target")

if [[ -n "$violations" ]]; then
    printf 'FAIL  presenter pre-ticked an approval box in:\n      %s\n\n' "$target" >&2
    printf '%s\n' "$violations" >&2
    printf '\n      The operator is the only one who ticks.  Reset the box\n' >&2
    printf '      to `- [ ]` and re-run this gate.\n' >&2
    exit 1
fi

echo "PRESENTATION CHECK PASS  ($target — $boxes approval box(es), all UN-ticked)"
exit 0
