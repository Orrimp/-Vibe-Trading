# Story 7.8: v3-volatility-forecaster-rebaseline

Status: retired

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: spec/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the v3 volatility-forecaster re-baseline pass confirming the retirement verdict,
so that the measured dead-ends stay on the record so they are never re-litigated.

## Acceptance Criteria

1. **Given** the repo history at `v3-volatility-forecaster-rebaseline`'s landing commits (`git log -- spec/v1/v3-volatility-forecaster-rebaseline`), **when** the recorded verification for `v3-volatility-forecaster-rebaseline` is replayed (tests, reports under `spec/v1/v3-volatility-forecaster-rebaseline/reports/` where present, render proofs where UI-facing), **then** the shipped behaviour holds: the v3 volatility-forecaster re-baseline pass confirming the retirement verdict.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [x] `v3-volatility-forecaster-rebaseline` 0.1.0 - the base feature (retired)

## Dev Notes

- Source feature folder: `spec/v1/v3-volatility-forecaster-rebaseline/` - frontmatter status **`retired`** (verbatim), version `0.1.0`, updated `2026-06-17`.
- Status mapping: `retired` -> `retired` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- Disposition: retired research/measure line — code + evidence retained, not deleted.
- CHANGELOG index: CHANGELOG § Retired research lines (the `v3 volatility forecaster (+ noop-fix + re-baseline)` line).
- Provenance: `git log -- spec/v1/v3-volatility-forecaster-rebaseline` (full narrative); reports under `spec/v1/v3-volatility-forecaster-rebaseline/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-V3-VOL-FORECASTER-REBASELINE-001` (state=`retired`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 7 (Retired Research Lines (measured-and-retired bets))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
