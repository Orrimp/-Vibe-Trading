# Story 4.1: advisor-overfitting-scorecard

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: spec/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the report-only overfitting scorecard (N_eff -> DSR -> MinBTL) beside every recommendation - additive to the FROZEN gate, never a veto (P0-1),
so that the recommendation surface earns credibility by measurement instead of asserting alpha.

## Acceptance Criteria

1. **Given** the repo history at `advisor-overfitting-scorecard`'s landing commits (`git log -- spec/v2/advisor-overfitting-scorecard`), **when** the recorded verification for `advisor-overfitting-scorecard` is replayed (tests, reports under `spec/v2/advisor-overfitting-scorecard/reports/` where present, render proofs where UI-facing), **then** the shipped behaviour holds: the report-only overfitting scorecard (N_eff -> DSR -> MinBTL) beside every recommendation - additive to the FROZEN gate, never a veto (P0-1).
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [x] `advisor-overfitting-scorecard` 0.1.0 - the base feature (shipped)

## Dev Notes

- Source feature folder: `spec/v2/advisor-overfitting-scorecard/` - frontmatter status **`shipped`** (verbatim), version `0.1.0`, updated `2026-06-30`.
- Status mapping: `shipped` -> `done` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § v2 — research-driven credibility & honesty tranche.
- Provenance: `git log -- spec/v2/advisor-overfitting-scorecard` (full narrative); reports under `spec/v2/advisor-overfitting-scorecard/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-V2-ANALYSIS-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 4 (v2 Research-Driven Credibility Tranche)

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
