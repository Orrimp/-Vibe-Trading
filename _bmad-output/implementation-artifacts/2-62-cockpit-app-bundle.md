# Story 2.62: cockpit-app-bundle

Status: backlog

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: spec/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want macOS .app packaging for dock + cmd-tab + Spotlight icons (candidate; not built),
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

## Acceptance Criteria

1. **Given** the recorded brief in `spec/cockpit-app-bundle/feature.md`, **when** the operator schedules the work (post do-not-build-register check), **then** the story delivers: macOS .app packaging for dock + cmd-tab + Spotlight icons (candidate; not built).
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [ ] `cockpit-app-bundle` 0.1.0 - the base feature (candidate)

## Dev Notes

- Source feature folder: `spec/cockpit-app-bundle/` - frontmatter status **`candidate`** (verbatim), version `0.1.0`, updated `2026-05-11`.
- Status mapping: `candidate` -> `backlog` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Deferred / not built (by decision).
- Provenance: `git log -- spec/cockpit-app-bundle` (full narrative); reports under `spec/cockpit-app-bundle/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-COCKPIT-BUNDLE-001` (state=`candidate`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 2 (Cockpit & UI (Lumen shell, Live, Lab, charts, quality gates))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
