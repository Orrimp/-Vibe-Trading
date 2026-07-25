# Story 7.5: v26-forecast-bakeoff

Status: retired

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the v2.6 forecast bake-off + retirement decision (phase 4 of 4) closing the DL programme,
so that the measured dead-ends stay on the record so they are never re-litigated.

## Acceptance Criteria

1. **Given** the repo history at `v26-forecast-bakeoff`'s landing commits (`git log -- spec/v1/v26-forecast-bakeoff`), **when** the recorded verification for `v26-forecast-bakeoff` is replayed (tests, reports under `evidence/v1/v26-forecast-bakeoff/reports/` where present, render proofs where UI-facing), **then** the shipped behaviour holds: the v2.6 forecast bake-off + retirement decision (phase 4 of 4) closing the DL programme.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [x] `v26-forecast-bakeoff` 2.6.0 - the base feature (deprecated)

## Dev Notes

- Source feature folder: `spec/v1/v26-forecast-bakeoff/` - frontmatter status **`deprecated`** (verbatim), version `2.6.0`, updated `2026-06-17`.
- Status mapping: `deprecated` -> `retired` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- Disposition: deprecated (superseded / measured NO-GO) — treated as retired.
- CHANGELOG index: CHANGELOG § Retired research lines.
- Provenance: `git log -- spec/v1/v26-forecast-bakeoff` (full narrative); reports under `evidence/v1/v26-forecast-bakeoff/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-V26-BAKEOFF-001` (state=`deprecated`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 7 (Retired Research Lines (measured-and-retired bets))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
