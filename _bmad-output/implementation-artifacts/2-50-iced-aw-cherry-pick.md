# Story 2.50: iced-aw-cherry-pick

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: spec/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the iced_aw cherry-pick bridge used during the native-widget migration,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

## Acceptance Criteria

1. **Given** the repo history at `iced-aw-cherry-pick`'s landing commits (`git log -- spec/v1/iced-aw-cherry-pick`), **when** the recorded verification for `iced-aw-cherry-pick` is replayed (tests, reports under `spec/v1/iced-aw-cherry-pick/reports/` where present, render proofs where UI-facing), **then** the shipped behaviour holds: the iced_aw cherry-pick bridge used during the native-widget migration.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [x] `iced-aw-cherry-pick` 1.0.0 - the base feature (shipped)

## Dev Notes

- Source feature folder: `spec/v1/iced-aw-cherry-pick/` - frontmatter status **`shipped`** (verbatim), version `1.0.0`, updated `2026-06-17`.
- Status mapping: `shipped` -> `done` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Cockpit & UI › iced platform & UI quality gates.
- Provenance: `git log -- spec/v1/iced-aw-cherry-pick` (full narrative); reports under `spec/v1/iced-aw-cherry-pick/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-ICED-AW-001` (state=`shipped`) · `REQ-ICED-AW-002` (state=`shipped`) · `REQ-ICED-AW-003` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 2 (Cockpit & UI (Lumen shell, Live, Lab, charts, quality gates))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
