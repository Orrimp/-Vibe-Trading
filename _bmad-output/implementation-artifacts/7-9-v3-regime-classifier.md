# Story 7.9: v3-regime-classifier

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the v3 regime classifier (predict regime label, not mu) - foreclosed when the OHLCV channel was exhausted,
so that the measured dead-ends stay on the record so they are never re-litigated.

## Acceptance Criteria

1. **Given** the repo history at `v3-regime-classifier`'s landing commits (`git log -- spec/v1/v3-regime-classifier`), **when** the recorded verification for `v3-regime-classifier` is replayed (tests, reports under `evidence/v1/v3-regime-classifier/reports/` where present, render proofs where UI-facing), **then** the shipped behaviour holds: the v3 regime classifier (predict regime label, not mu) - foreclosed when the OHLCV channel was exhausted.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [x] `v3-regime-classifier` 0.1.0 - the base feature (shipped)

## Dev Notes

- Source feature folder: `spec/v1/v3-regime-classifier/` - frontmatter status **`shipped`** (verbatim), version `0.1.0`, updated `2026-06-17`.
- Status mapping: `shipped` -> `done` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Retired research lines.
- Provenance: `git log -- spec/v1/v3-regime-classifier` (full narrative); reports under `evidence/v1/v3-regime-classifier/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-V3-REGIME-CLASSIFIER-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 7 (Retired Research Lines (measured-and-retired bets))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
