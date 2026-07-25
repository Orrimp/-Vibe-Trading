# Story 3.13: advisor-param-promotion

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: spec/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want promotion of a surviving (non-FRAGILE) tuned config into the forward plan + paper-run, with the tuned-rules honesty header,
so that the guided EUR-200 journey stays honest, reproducible, and robustness-gated end to end.

## Acceptance Criteria

1. **Given** the repo history at `advisor-param-promotion`'s landing commits (`git log -- spec/v1/advisor-param-promotion`), **when** the recorded verification for `advisor-param-promotion` is replayed (tests, reports under `spec/v1/advisor-param-promotion/reports/` where present, render proofs where UI-facing), **then** the shipped behaviour holds: promotion of a surviving (non-FRAGILE) tuned config into the forward plan + paper-run, with the tuned-rules honesty header.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [x] `advisor-param-promotion` 0.1.0 - the base feature (shipped)

## Dev Notes

- Source feature folder: `spec/v1/advisor-param-promotion/` - frontmatter status **`shipped`** (verbatim), version `0.1.0`, updated `2026-06-26`.
- Status mapping: `shipped` -> `done` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Advisor — `**advisor-param-promotion**`.
- Provenance: `git log -- spec/v1/advisor-param-promotion` (full narrative); reports under `spec/v1/advisor-param-promotion/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-ADVISOR-PARAM-PROMOTION-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 3 (Advisor MVP (F1-F9 + EUR-FX + dynamic data + PIT discipline))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
