# Story 2.9: ui-rethink-phase-b-lab-run

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: spec/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the Lab Run button + run plumbing (UI rethink Phase B),
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

## Acceptance Criteria

1. **Given** the repo history at `ui-rethink-phase-b-lab-run`'s landing commits (`git log -- spec/v1/ui-rethink-phase-b-lab-run`), **when** the recorded verification for `ui-rethink-phase-b-lab-run` is replayed (tests, reports under `spec/v1/ui-rethink-phase-b-lab-run/reports/` where present, render proofs where UI-facing), **then** the shipped behaviour holds: the Lab Run button + run plumbing (UI rethink Phase B).
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [x] `ui-rethink-phase-b-lab-run` 0.2.0 - the base feature (shipped)

## Dev Notes

- Source feature folder: `spec/v1/ui-rethink-phase-b-lab-run/` - frontmatter status **`shipped`** (verbatim), version `0.2.0`, updated `2026-06-17`.
- Status mapping: `shipped` -> `done` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Cockpit & UI › Shell, navigation & design system.
- Provenance: `git log -- spec/v1/ui-rethink-phase-b-lab-run` (full narrative); reports under `spec/v1/ui-rethink-phase-b-lab-run/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-UI-RETHINK-PHASE-B-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 2 (Cockpit & UI (Lumen shell, Live, Lab, charts, quality gates))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
