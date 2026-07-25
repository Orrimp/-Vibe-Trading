# Story 2.53: ui-gallery-table-cell

Status: backlog

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the widget-gallery table-cell bounds fix (draft brief; not built),
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

## Acceptance Criteria

1. **Given** the recorded brief in `spec/ui-gallery-table-cell/feature.md`, **when** the operator schedules the work (post do-not-build-register check), **then** the story delivers: the widget-gallery table-cell bounds fix (draft brief; not built).
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [ ] `ui-gallery-table-cell` n/a - the base feature (draft)

## Dev Notes

- Source feature folder: `spec/ui-gallery-table-cell/` - frontmatter status **`draft`** (verbatim), version `n/a`, updated `2026-05-16`.
- Status mapping: `draft` -> `backlog` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Deferred / not built (by decision).
- Provenance: `git log -- spec/ui-gallery-table-cell` (full narrative); reports under `evidence/ui-gallery-table-cell/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: none — known trace-coverage gap (spec audit 2026-07-06); no `[[req]]` row in `spec/trace.toml`
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 2 (Cockpit & UI (Lumen shell, Live, Lab, charts, quality gates))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
