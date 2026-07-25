# Story 1.3: v1-cross-sectional-momentum

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the cross-sectional top-N momentum strategy - the first multi-symbol real-edge candidate,
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

## Acceptance Criteria

1. **Given** the repo history at `v1-cross-sectional-momentum`'s landing commits (`git log -- spec/v1/v1-cross-sectional-momentum`), **when** the recorded verification for `v1-cross-sectional-momentum` is replayed (tests, reports under `evidence/v1/v1-cross-sectional-momentum/reports/` where present, render proofs where UI-facing), **then** the shipped behaviour holds: the cross-sectional top-N momentum strategy - the first multi-symbol real-edge candidate.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [x] `v1-cross-sectional-momentum` 1.0.0 - the base feature (shipped)

## Dev Notes

- Source feature folder: `spec/v1/v1-cross-sectional-momentum/` - frontmatter status **`shipped`** (verbatim), version `1.0.0`, updated `2026-06-17`.
- Status mapping: `shipped` -> `done` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Strategy — `**v1**` (Cross-sectional top-N momentum).
- Provenance: `git log -- spec/v1/v1-cross-sectional-momentum` (full narrative); reports under `evidence/v1/v1-cross-sectional-momentum/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-V1-MOMENTUM-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 1 (Strategy & Backtest Engine (v0-v5 ladder + robustness program))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
