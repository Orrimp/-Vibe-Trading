# Story 4.7: advisor-drawdown-control-overlay

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the drawdown-control sizing overlay (high-water-mark restart, CPPI-style 20% floor) with day-1 divergence e2e (P1-3),
so that the recommendation surface earns credibility by measurement instead of asserting alpha.

## Acceptance Criteria

1. **Given** the repo history at `advisor-drawdown-control-overlay`'s landing commits (`git log -- spec/v2/advisor-drawdown-control-overlay`), **when** the recorded verification for `advisor-drawdown-control-overlay` is replayed (tests, reports under `evidence/v2/advisor-drawdown-control-overlay/reports/` where present, render proofs where UI-facing), **then** the shipped behaviour holds: the drawdown-control sizing overlay (high-water-mark restart, CPPI-style 20% floor) with day-1 divergence e2e (P1-3).
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [x] `advisor-drawdown-control-overlay` v2.0.0-phase-2c - the base feature (shipped)

## Dev Notes

- Source feature folder: `spec/v2/advisor-drawdown-control-overlay/` - frontmatter status **`shipped`** (verbatim), version `v2.0.0-phase-2c`, updated `2026-07-01`.
- Status mapping: `shipped` -> `done` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § v2 — research-driven credibility & honesty tranche.
- Provenance: `git log -- spec/v2/advisor-drawdown-control-overlay` (full narrative); reports under `evidence/v2/advisor-drawdown-control-overlay/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-V2-P1-3-DRAWDOWN-OVERLAY-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 4 (v2 Research-Driven Credibility Tranche)
- Shared phase-2C tester-report umbrella (`spec/v2/phase-2c-overlays/`) is folded under story 4.5 (advisor-vol-estimator).

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
