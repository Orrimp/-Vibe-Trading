# Story 3.4: advisor-dynamic-data

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want on-demand Binance fetch for any coin + window outside the pinned corpus (git-ignored cache, anchor-safe by construction),
so that the guided EUR-200 journey stays honest, reproducible, and robustness-gated end to end.

## Acceptance Criteria

1. **Given** the repo history at `advisor-dynamic-data`'s landing commits (`git log -- spec/v1/advisor-dynamic-data`), **when** the recorded verification for `advisor-dynamic-data` is replayed (tests, reports under `evidence/v1/advisor-dynamic-data/reports/` where present, render proofs where UI-facing), **then** the shipped behaviour holds: on-demand Binance fetch for any coin + window outside the pinned corpus (git-ignored cache, anchor-safe by construction).
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [x] `advisor-dynamic-data` 0.1.0 - the base feature (shipped)

## Dev Notes

- Source feature folder: `spec/v1/advisor-dynamic-data/` - frontmatter status **`shipped`** (verbatim), version `0.1.0`, updated `2026-06-22`.
- Status mapping: `shipped` -> `done` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Advisor — `**advisor-dynamic-data**`.
- Provenance: `git log -- spec/v1/advisor-dynamic-data` (full narrative); reports under `evidence/v1/advisor-dynamic-data/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-ADVISOR-DYNAMIC-DATA-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 3 (Advisor MVP (F1-F9 + EUR-FX + dynamic data + PIT discipline))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
