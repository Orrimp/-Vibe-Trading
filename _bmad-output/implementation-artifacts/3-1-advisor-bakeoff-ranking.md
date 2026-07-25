# Story 3.1: advisor-bakeoff-ranking

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the strategy bake-off + ranking engine (run_bakeoff -> BakeoffReport; Fragile-ineligible -> Sharpe -> return -> drawdown -> id; buy-and-hold always the benchmark arm; structured Recommendation) plus the Leaderboard/guided-input surfaces (F1+F2, with F3 and the leaderboard-inspect iterations landing on the same folder),
so that the guided EUR-200 journey stays honest, reproducible, and robustness-gated end to end.

## Acceptance Criteria

1. **Given** the repo history at `advisor-bakeoff-ranking`'s landing commits (`git log -- spec/v1/advisor-bakeoff-ranking`), **when** the recorded verification for `advisor-bakeoff-ranking` is replayed (tests, reports under `evidence/v1/advisor-bakeoff-ranking/reports/` where present, render proofs where UI-facing), **then** the shipped behaviour holds: the strategy bake-off + ranking engine (run_bakeoff -> BakeoffReport; Fragile-ineligible -> Sharpe -> return -> drawdown -> id; buy-and-hold always the benchmark arm; structured Recommendation) plus the Leaderboard/guided-input surfaces (F1+F2, with F3 and the leaderboard-inspect iterations landing on the same folder).
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [x] `advisor-bakeoff-ranking` 0.1.0 - the base feature (shipped)

## Dev Notes

- Source feature folder: `spec/v1/advisor-bakeoff-ranking/` - frontmatter status **`shipped`** (verbatim), version `0.1.0`, updated `2026-06-22`.
- Status mapping: `shipped` -> `done` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Advisor — `**advisor-bakeoff F1+F2**` (+ F3, progress/polish, leaderboard-inspect lines).
- Provenance: `git log -- spec/v1/advisor-bakeoff-ranking` (full narrative); reports under `evidence/v1/advisor-bakeoff-ranking/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-ADVISOR-BAKEOFF-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 3 (Advisor MVP (F1-F9 + EUR-FX + dynamic data + PIT discipline))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
