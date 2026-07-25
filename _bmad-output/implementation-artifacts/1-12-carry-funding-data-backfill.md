# Story 1.12: carry-funding-data-backfill

Status: review

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: spec/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the funding-rate data backfill feeding the carry strategy,
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

## Acceptance Criteria

1. **Given** the built-and-verified state frozen at frontmatter `dev-done` (2026-06-17 spec compression), **when** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived, **then** the delivered behaviour stands as recorded: the funding-rate data backfill feeding the carry strategy.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [ ] `carry-funding-data-backfill` 0.1.0 - the base feature (dev-done)

## Dev Notes

- Source feature folder: `spec/v1/carry-funding-data-backfill/` - frontmatter status **`dev-done`** (verbatim), version `0.1.0`, updated `2026-06-17`.
- Status mapping: `dev-done` -> `review` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: no CHANGELOG line (dev-done; never flipped to shipped).
- Provenance: `git log -- spec/v1/carry-funding-data-backfill` (full narrative); reports under `spec/v1/carry-funding-data-backfill/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-CARRY-FUNDING-DATA-BACKFILL-001` (state=`dev-done`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 1 (Strategy & Backtest Engine (v0-v5 ladder + robustness program))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
