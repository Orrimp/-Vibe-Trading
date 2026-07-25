# Story 5.6: advisor-handoff-export

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: spec/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the deterministic plan export (plan-{coin}-{window}-{seed8}.md, operator-ratified golden-locked wording incl. the short unbounded-loss case) (P5),
so that the shipped product is provably done, with its thesis boundary honestly mapped.

## Acceptance Criteria

1. **Given** the repo history at `advisor-handoff-export`'s landing commits (`git log -- spec/v3/advisor-handoff-export`), **when** the recorded verification for `advisor-handoff-export` is replayed (tests, reports under `spec/v3/advisor-handoff-export/reports/` where present, render proofs where UI-facing), **then** the shipped behaviour holds: the deterministic plan export (plan-{coin}-{window}-{seed8}.md, operator-ratified golden-locked wording incl. the short unbounded-loss case) (P5).
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [x] `advisor-handoff-export` 3.4.0 - the base feature (shipped)

## Dev Notes

- Source feature folder: `spec/v3/advisor-handoff-export/` - frontmatter status **`shipped`** (verbatim), version `3.4.0`, updated `2026-07-10`.
- Status mapping: `shipped` -> `done` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Remediation P0–P8.
- Provenance: `git log -- spec/v3/advisor-handoff-export` (full narrative); reports under `spec/v3/advisor-handoff-export/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-V3-P5-HANDOFF-EXPORT-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 5 (v3 "Prove It's Done" Close-Out)

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
