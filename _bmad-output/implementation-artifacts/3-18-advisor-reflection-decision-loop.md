# Story 3.18: advisor-reflection-decision-loop

Status: ready-for-dev

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the C4 reflection decision-support memory surface for the advisor (the honest C4) - architecture done, build pending an operator decision,
so that the guided EUR-200 journey stays honest, reproducible, and robustness-gated end to end.

## Acceptance Criteria

1. **Given** the completed C4 architecture in spec/advisor-reflection-decision-loop/, **when** the operator green-lights the build (or parks it via the do-not-build check), **then** the story moves to dev with the arch-done design as its context - until then it stays ready-for-dev, honestly not built.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [ ] `advisor-reflection-decision-loop` 0.1.0 - the base feature (arch-done)

## Dev Notes

- Source feature folder: `spec/advisor-reflection-decision-loop/` - frontmatter status **`arch-done`** (verbatim), version `0.1.0`, updated `2026-06-26`.
- Status mapping: `arch-done` -> `ready-for-dev` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: no CHANGELOG line (arch-done; not built).
- Provenance: `git log -- spec/advisor-reflection-decision-loop` (full narrative); reports under `evidence/advisor-reflection-decision-loop/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-ADVISOR-REFLECTION-DECISION-LOOP-001` (state=`design-complete`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 3 (Advisor MVP (F1-F9 + EUR-FX + dynamic data + PIT discipline))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
