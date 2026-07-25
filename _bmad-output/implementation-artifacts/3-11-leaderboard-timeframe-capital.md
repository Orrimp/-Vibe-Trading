# Story 3.11: leaderboard-timeframe-capital

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: spec/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the bake-off tune knobs: H1/H4/D1 timeframe resampling (may change ranking) and starting-capital control (does not - and the UI says so),
so that the guided EUR-200 journey stays honest, reproducible, and robustness-gated end to end.

## Acceptance Criteria

1. **Given** the repo history at `leaderboard-timeframe-capital`'s landing commits (`git log -- spec/v1/leaderboard-timeframe-capital`), **when** the recorded verification for `leaderboard-timeframe-capital` is replayed (tests, reports under `spec/v1/leaderboard-timeframe-capital/reports/` where present, render proofs where UI-facing), **then** the shipped behaviour holds: the bake-off tune knobs: H1/H4/D1 timeframe resampling (may change ranking) and starting-capital control (does not - and the UI says so).
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [x] `leaderboard-timeframe-capital` 0.1.0 - the base feature (shipped)

## Dev Notes

- Source feature folder: `spec/v1/leaderboard-timeframe-capital/` - frontmatter status **`shipped`** (verbatim), version `0.1.0`, updated `2026-06-26`.
- Status mapping: `shipped` -> `done` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Advisor — `**leaderboard-timeframe-capital**`.
- Provenance: `git log -- spec/v1/leaderboard-timeframe-capital` (full narrative); reports under `spec/v1/leaderboard-timeframe-capital/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: none — known trace-coverage gap (spec audit 2026-07-06); no `[[req]]` row in `spec/trace.toml`
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 3 (Advisor MVP (F1-F9 + EUR-FX + dynamic data + PIT discipline))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
