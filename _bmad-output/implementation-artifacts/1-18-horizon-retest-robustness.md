# Story 1.18: horizon-retest-robustness

Status: review

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: spec/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the coarser 4h/daily decision-cadence retest - the last untested OHLCV axis (verdict: FRAGILE),
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

## Acceptance Criteria

1. **Given** the built-and-verified state frozen at frontmatter `presenter-done` (2026-06-17 spec compression), **when** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived, **then** the delivered behaviour stands as recorded: the coarser 4h/daily decision-cadence retest - the last untested OHLCV axis (verdict: FRAGILE).
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [ ] `horizon-retest-robustness` 0.2.0 - the base feature (presenter-done)

## Dev Notes

- Source feature folder: `spec/v1/horizon-retest-robustness/` - frontmatter status **`presenter-done`** (verbatim), version `0.2.0`, updated `2026-06-17`.
- Status mapping: `presenter-done` -> `review` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Robustness program — CONCLUDED 2026-06-08 → ship passive.
- Provenance: `git log -- spec/v1/horizon-retest-robustness` (full narrative); reports under `spec/v1/horizon-retest-robustness/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-HORIZON-RETEST-ROBUSTNESS-001` (state=`tester-done`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 1 (Strategy & Backtest Engine (v0-v5 ladder + robustness program))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
