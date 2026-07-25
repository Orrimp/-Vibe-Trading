# Story 1.11: binance-corpus-expansion

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: spec/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the wider down-market Binance corpus (data/binance/, ADR-0032 pins) underpinning the bear surveys,
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

## Acceptance Criteria

1. **Given** the repo history at `binance-corpus-expansion`'s landing commits (`git log -- spec/v1/binance-corpus-expansion`), **when** the recorded verification for `binance-corpus-expansion` is replayed (tests, reports under `spec/v1/binance-corpus-expansion/reports/` where present, render proofs where UI-facing), **then** the shipped behaviour holds: the wider down-market Binance corpus (data/binance/, ADR-0032 pins) underpinning the bear surveys.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [x] `binance-corpus-expansion` 0.1.0 - the base feature (shipped)

## Dev Notes

- Source feature folder: `spec/v1/binance-corpus-expansion/` - frontmatter status **`shipped`** (verbatim), version `0.1.0`, updated `2026-06-17`.
- Status mapping: `shipped` -> `done` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Robustness program — CONCLUDED 2026-06-08 → ship passive.
- Provenance: `git log -- spec/v1/binance-corpus-expansion` (full narrative); reports under `spec/v1/binance-corpus-expansion/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-BINANCE-CORPUS-EXPANSION-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 1 (Strategy & Backtest Engine (v0-v5 ladder + robustness program))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
