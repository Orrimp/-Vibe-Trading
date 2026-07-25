# Story 2.15: cockpit-live-dashboard-wiring

Status: review

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: spec/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the Live equity curve + KPI strip fed from the live agent,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

## Acceptance Criteria

1. **Given** the built-and-verified state frozen at frontmatter `presenter-done` (2026-06-17 spec compression), **when** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived, **then** the delivered behaviour stands as recorded: the Live equity curve + KPI strip fed from the live agent.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [ ] `cockpit-live-dashboard-wiring` 0.1.2 - the base feature (presenter-done)

## Dev Notes

- Source feature folder: `spec/v1/cockpit-live-dashboard-wiring/` - frontmatter status **`presenter-done`** (verbatim), version `0.1.2`, updated `2026-06-17`.
- Status mapping: `presenter-done` -> `review` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Cockpit & UI › Live cockpit & dashboards.
- Provenance: `git log -- spec/v1/cockpit-live-dashboard-wiring` (full narrative); reports under `spec/v1/cockpit-live-dashboard-wiring/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-COCKPIT-LIVE-DASHBOARD-001` (state=`tester-done`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 2 (Cockpit & UI (Lumen shell, Live, Lab, charts, quality gates))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
