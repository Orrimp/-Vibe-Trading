#!/usr/bin/env bash
# review_prep.sh — the mechanical first two minutes of a bmad-code-review.
#
# Usage:  bash scripts/review_prep.sh <story-key> [outdir]
#   e.g.  bash scripts/review_prep.sh 1-18-horizon-retest-robustness
#
# Does what the orchestrator did by hand for eight stories:
#   1. derives the feature slug from the story key
#   2. finds the commits that actually touched crates/ for that slug
#   3. writes a code-only diff (no PNGs) the review layers can be pointed at
#   4. prints the baseline gates (anchors + spec-lint) BEFORE any edit
#   5. prints the story's triad legs (trace state, CHANGELOG line, board row)
#
# Read docs/dev-notes/review-playbook.md for what to do with the output.
set -uo pipefail

STORY_KEY="${1:-}"
if [[ -z "$STORY_KEY" ]]; then
    echo "usage: bash scripts/review_prep.sh <story-key> [outdir]" >&2
    echo "  story keys: ls _bmad-output/implementation-artifacts/*.md" >&2
    exit 2
fi

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT" || exit 2

# Story key is <epic>-<n>-<slug>; the slug is what the old spec/ commits used.
SLUG="$(printf '%s' "$STORY_KEY" | sed -E 's/^[0-9]+-[0-9]+-//')"
OUTDIR="${2:-${TMPDIR:-/tmp}}"
DIFF="$OUTDIR/review-diff-${STORY_KEY}.patch"

STORY_FILE="_bmad-output/implementation-artifacts/${STORY_KEY}.md"
if [[ ! -f "$STORY_FILE" ]]; then
    echo "!! no story file at $STORY_FILE — check the key" >&2
    exit 2
fi

echo "== story:  $STORY_FILE"
echo "== slug:   $SLUG"
grep -m1 '^Status:' "$STORY_FILE" || true

# ── 1. candidate commits (implementation only: touched crates/) ───────────────
echo
echo "== candidate implementing commits (touched crates/):"
COMMITS=()
while IFS= read -r sha; do
    [[ -z "$sha" ]] && continue
    if git show --stat --format="" "$sha" -- 'crates/' | grep -q .; then
        COMMITS+=("$sha")
        git log -1 --format='   %h %s' "$sha"
    fi
done < <(git log --reverse --format=%H --all --grep="$SLUG" -i)

if [[ ${#COMMITS[@]} -eq 0 ]]; then
    echo "   (none found by slug grep — widen manually:"
    echo "    git log --oneline --all --grep='<keyword>' )"
fi

# ── 2. code-only diff ────────────────────────────────────────────────────────
: > "$DIFF"
for sha in "${COMMITS[@]}"; do
    git show "$sha" -- 'crates/' ':(exclude)*.png' >> "$DIFF"
done
echo
echo "== diff written: $DIFF  ($(wc -l < "$DIFF" | tr -d ' ') lines)"
echo "   point every review layer at this path (\"its contents ARE the diff\")."

# ── 3. baseline gates (BEFORE any edit — the anchor double-gate's first half) ─
echo
echo "== baseline gates (before any edit):"
bash scripts/verify_anchors.sh 2>/dev/null | tail -1
python3 scripts/spec_lint.py 2>/dev/null | tail -1

# ── 4. triad legs ────────────────────────────────────────────────────────────
echo
echo "== triad legs:"
REQ="$(grep -oE 'REQ-[A-Z0-9-]+' "$STORY_FILE" | head -1)"
if [[ -n "$REQ" ]]; then
    LINE="$(awk -v req="$REQ" '$0 ~ req {f=1} f && /^state/ {print NR": "substr($0,1,90); exit}' \
        _bmad-output/planning-artifacts/trace.toml)"
    echo "   trace  $REQ -> ${LINE:-<no state line found>}"
else
    echo "   trace  <no REQ id in the story's References block>"
fi
if grep -qi -- "$SLUG" CHANGELOG.md; then
    echo "   CHANGELOG: line PRESENT  ($(grep -n -i -m1 -- "$SLUG" CHANGELOG.md | cut -d: -f1))"
else
    echo "   CHANGELOG: line MISSING — the done-flip must add one (lint enforces it)"
fi
grep -m1 "  ${STORY_KEY}:" _bmad-output/implementation-artifacts/sprint-status.yaml \
    | sed 's/^/   board:/' || echo "   board: <row not found>"

# ── 5. reminders that cost real time when forgotten ──────────────────────────
cat <<'EOF'

== reminders (docs/dev-notes/review-playbook.md)
   - ACs live at the IMPLEMENTING commit: git show <sha>:spec/<slug>/feature.md
     (the feature.md at HEAD is a compressed stub — useless for an audit)
   - known-and-owned (#67 fills, #68 inert axis, #69 inert cap, PWSD, √8575):
     route with a verified chain, never re-report -> stories 1-24 / 1-25
   - mandatory probes: axis-execution, binding-limit, vacuity, skip-visibility,
     chain, identity-forge, seed-collision
   - infra reds that are NOT your diff: 62 visual baselines (font drift), 6-9 CI
   - re-run verify_anchors AFTER the last edit too (the double gate)
EOF
