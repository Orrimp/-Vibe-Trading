# Story 2.1: lumen-design-adoption

Status: in-progress

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: spec/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the Lumen design-system master roadmap governing the multi-screen sidebar-shell migration (phases 1-5 shipped; phase 6 reserved),
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

## Acceptance Criteria

1. **Given** the recorded brief in `spec/lumen-design-adoption/feature.md`, **when** the operator schedules the work (post do-not-build-register check), **then** the story delivers: the Lumen design-system master roadmap governing the multi-screen sidebar-shell migration (phases 1-5 shipped; phase 6 reserved).
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [ ] `lumen-design-adoption` 2.0.0 - the base feature (roadmap)

## Dev Notes

- Source feature folder: `spec/lumen-design-adoption/` - frontmatter status **`roadmap`** (verbatim), version `2.0.0`, updated `2026-05-04`.
- Status mapping: `roadmap` -> `in-progress` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- Disposition: master-roadmap umbrella — phases 1-5 shipped as their own stories, phase 6 reserved; the roadmap doc stays the open governing document.
- CHANGELOG index: CHANGELOG § Cockpit & UI › Shell, navigation & design system.
- Provenance: `git log -- spec/lumen-design-adoption` (full narrative); reports under `spec/lumen-design-adoption/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-LUMEN-DESIGN-001` (state=`roadmap`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 2 (Cockpit & UI (Lumen shell, Live, Lab, charts, quality gates))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
