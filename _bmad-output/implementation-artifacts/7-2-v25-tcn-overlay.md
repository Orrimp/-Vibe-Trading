# Story 7.2: v25-tcn-overlay

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: spec/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the TCN forecast overlay (phase 1 of 4) + its alpha-investigation / recalibrate / threshold-tuning / horizon-bump sub-studies - no +0.10 Sharpe delta; line retired,
so that the measured dead-ends stay on the record so they are never re-litigated.

## Acceptance Criteria

1. **Given** the repo history at `v25-tcn-overlay`'s landing commits (`git log -- spec/v1/v25-tcn-overlay`), **when** the recorded verification for `v25-tcn-overlay` is replayed (tests, reports under `spec/v1/v25-tcn-overlay/reports/` where present, render proofs where UI-facing), **then** the shipped behaviour holds: the TCN forecast overlay (phase 1 of 4) + its alpha-investigation / recalibrate / threshold-tuning / horizon-bump sub-studies - no +0.10 Sharpe delta; line retired.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [x] `v25-tcn-overlay` 2.5.0 - the base feature (shipped)
- [x] Folded iteration `v25-tcn-alpha-investigation` (0.3.0, frontmatter `shipped`): v2.5 — TCN alpha-verdict investigation - carries `REQ-V25-TCN-ALPHA-001` (provenance: `git log -- spec/v1/v25-tcn-alpha-investigation`)
- [x] Folded iteration `v25-tcn-recalibrate` (0.1.0, frontmatter `shipped`): v2.5 — TCN σ_train recalibration (metadata-only fix) - carries `REQ-V25-TCN-RECALIBRATE-001` (provenance: `git log -- spec/v1/v25-tcn-recalibrate`)
- [x] Folded iteration `v25-tcn-threshold-tuning` (0.1.0, frontmatter `shipped`): v2.5 — TCN threshold tuning (cheap τ × ε sweep over recalibrated checkpoints) - carries `REQ-V25-TCN-THRESHOLD-TUNING-001` (provenance: `git log -- spec/v1/v25-tcn-threshold-tuning`)
- [x] Folded iteration `v25-tcn-horizon-bump-or-retire` (0.1.0, frontmatter `shipped`): v2.5 — TCN horizon-bump or retire (scope-decision-grade) - carries `REQ-V25-TCN-HORIZON-BUMP-OR-RETIRE-001` (provenance: `git log -- spec/v1/v25-tcn-horizon-bump-or-retire`)

## Dev Notes

- Source feature folder: `spec/v1/v25-tcn-overlay/` - frontmatter status **`shipped`** (verbatim), version `2.5.0`, updated `2026-06-17`.
- Status mapping: `shipped` -> `done` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Retired research lines.
- Provenance: `git log -- spec/v1/v25-tcn-overlay` (full narrative); reports under `spec/v1/v25-tcn-overlay/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-V25-TCN-001` (state=`shipped`) · `REQ-V25-TCN-ALPHA-001` (state=`shipped`) · `REQ-V25-TCN-RECALIBRATE-001` (state=`shipped`) · `REQ-V25-TCN-THRESHOLD-TUNING-001` (state=`shipped`) · `REQ-V25-TCN-HORIZON-BUMP-OR-RETIRE-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 7 (Retired Research Lines (measured-and-retired bets))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
