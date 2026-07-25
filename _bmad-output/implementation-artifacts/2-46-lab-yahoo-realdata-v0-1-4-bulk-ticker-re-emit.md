# Story 2.46: lab-yahoo-realdata-v0.1.4-bulk-ticker-re-emit

Status: retired

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the v0.1.4 bulk-ticker re-emit (9 new tickers + ETH-daily redo) - retired,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

## Acceptance Criteria

1. **Given** the repo history at `lab-yahoo-realdata-v0.1.4-bulk-ticker-re-emit`'s landing commits (`git log -- spec/v1/lab-yahoo-realdata-v0.1.4-bulk-ticker-re-emit`), **when** the recorded verification for `lab-yahoo-realdata-v0.1.4-bulk-ticker-re-emit` is replayed (tests, reports under `evidence/v1/lab-yahoo-realdata-v0.1.4-bulk-ticker-re-emit/reports/` where present, render proofs where UI-facing), **then** the shipped behaviour holds: the v0.1.4 bulk-ticker re-emit (9 new tickers + ETH-daily redo) - retired.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [x] `lab-yahoo-realdata-v0.1.4-bulk-ticker-re-emit` 0.1.0 - the base feature (retired)

## Dev Notes

- Source feature folder: `spec/v1/lab-yahoo-realdata-v0.1.4-bulk-ticker-re-emit/` - frontmatter status **`retired`** (verbatim), version `0.1.0`, updated `2026-06-17`.
- Status mapping: `retired` -> `retired` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- Disposition: retired research/measure line — code + evidence retained, not deleted.
- CHANGELOG index: no CHANGELOG line (retired iteration).
- Provenance: `git log -- spec/v1/lab-yahoo-realdata-v0.1.4-bulk-ticker-re-emit` (full narrative); reports under `evidence/v1/lab-yahoo-realdata-v0.1.4-bulk-ticker-re-emit/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-LAB-YAHOO-REALDATA-V0-1-4-001` (state=`retired`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 2 (Cockpit & UI (Lumen shell, Live, Lab, charts, quality gates))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
