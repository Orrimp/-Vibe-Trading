# Story 3.3: advisor-forward-plan

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: spec/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the honest, conditional forward buy/sell plan (F6): IF/THEN rules faithful to the real TOMLs, not-a-prediction framing, configurable 1-30d horizon,
so that the guided EUR-200 journey stays honest, reproducible, and robustness-gated end to end.

## Acceptance Criteria

1. **Given** the repo history at `advisor-forward-plan`'s landing commits (`git log -- spec/v1/advisor-forward-plan`), **when** the recorded verification for `advisor-forward-plan` is replayed (tests, reports under `spec/v1/advisor-forward-plan/reports/` where present, render proofs where UI-facing), **then** the shipped behaviour holds: the honest, conditional forward buy/sell plan (F6): IF/THEN rules faithful to the real TOMLs, not-a-prediction framing, configurable 1-30d horizon.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [x] `advisor-forward-plan` 0.2.0 - the base feature (shipped)

## Dev Notes

- Source feature folder: `spec/v1/advisor-forward-plan/` - frontmatter status **`shipped`** (verbatim), version `0.2.0`, updated `2026-06-22`.
- Status mapping: `shipped` -> `done` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Advisor — `**advisor-forward-plan F6**`.
- Provenance: `git log -- spec/v1/advisor-forward-plan` (full narrative); reports under `spec/v1/advisor-forward-plan/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-ADVISOR-FORWARD-PLAN-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 3 (Advisor MVP (F1-F9 + EUR-FX + dynamic data + PIT discipline))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
