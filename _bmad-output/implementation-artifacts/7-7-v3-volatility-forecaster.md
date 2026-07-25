# Story 7.7: v3-volatility-forecaster

Status: retired

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: spec/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the v3 GARCH-sigma volatility forecaster (predict sigma, not mu) + the noop-fix that exposed the computed-but-unapplied overlay - MODEL-BROKEN / NO-ALPHA; retired,
so that the measured dead-ends stay on the record so they are never re-litigated.

## Acceptance Criteria

1. **Given** the repo history at `v3-volatility-forecaster`'s landing commits (`git log -- spec/v1/v3-volatility-forecaster`), **when** the recorded verification for `v3-volatility-forecaster` is replayed (tests, reports under `spec/v1/v3-volatility-forecaster/reports/` where present, render proofs where UI-facing), **then** the shipped behaviour holds: the v3 GARCH-sigma volatility forecaster (predict sigma, not mu) + the noop-fix that exposed the computed-but-unapplied overlay - MODEL-BROKEN / NO-ALPHA; retired.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [x] `v3-volatility-forecaster` 0.1.0 - the base feature (retired)
- [x] Folded iteration `v3-volatility-forecaster-noop-fix` (0.1.0, frontmatter `shipped`): v3 volatility forecaster — no-op wire-up FIX - carries `REQ-V3-VOL-FORECASTER-NOOP-FIX-001` (provenance: `git log -- spec/v1/v3-volatility-forecaster-noop-fix`)

## Dev Notes

- Source feature folder: `spec/v1/v3-volatility-forecaster/` - frontmatter status **`retired`** (verbatim), version `0.1.0`, updated `2026-06-17`.
- Status mapping: `retired` -> `retired` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- Disposition: retired research/measure line — code + evidence retained, not deleted.
- CHANGELOG index: CHANGELOG § Retired research lines.
- Provenance: `git log -- spec/v1/v3-volatility-forecaster` (full narrative); reports under `spec/v1/v3-volatility-forecaster/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-V3-VOL-FORECASTER-001` (state=`retired`) · `REQ-V3-VOL-FORECASTER-NOOP-FIX-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 7 (Retired Research Lines (measured-and-retired bets))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
