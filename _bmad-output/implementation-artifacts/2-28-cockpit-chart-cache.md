# Story 2.28: cockpit-chart-cache

Status: retired

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the chart canvas::Cache hover-smoothness measure - Phase 1 MEASURE returned NO-GO (deprecated by measurement),
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

## Acceptance Criteria

1. **Given** the repo history at `cockpit-chart-cache`'s landing commits (`git log -- spec/v1/cockpit-chart-cache`), **when** the recorded verification for `cockpit-chart-cache` is replayed (tests, reports under `evidence/v1/cockpit-chart-cache/reports/` where present, render proofs where UI-facing), **then** the shipped behaviour holds: the chart canvas::Cache hover-smoothness measure - Phase 1 MEASURE returned NO-GO (deprecated by measurement).
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [x] `cockpit-chart-cache` n/a - the base feature (deprecated)

## Dev Notes

- Source feature folder: `spec/v1/cockpit-chart-cache/` - frontmatter status **`deprecated`** (verbatim), version `n/a`, updated `2026-06-17`.
- Status mapping: `deprecated` -> `retired` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- Disposition: deprecated (superseded / measured NO-GO) — treated as retired.
- CHANGELOG index: no CHANGELOG line (measured NO-GO; deprecated).
- Provenance: `git log -- spec/v1/cockpit-chart-cache` (full narrative); reports under `evidence/v1/cockpit-chart-cache/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-COCKPIT-CHARTCACHE-001` (state=`deprecated`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 2 (Cockpit & UI (Lumen shell, Live, Lab, charts, quality gates))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
