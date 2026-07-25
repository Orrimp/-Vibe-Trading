# Story 6.2: operator-success-reports

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want auto-generated "is this working?" operator reports (equity, Sharpe/Sortino/drawdown, attribution, system health),
so that the gates, ledgers, and process infrastructure keep the repo honest without manual vigilance.

## Acceptance Criteria

1. **Given** the repo history at `operator-success-reports`'s landing commits (`git log -- spec/v1/operator-success-reports`), **when** the recorded verification for `operator-success-reports` is replayed (tests, reports under `evidence/v1/operator-success-reports/reports/` where present, render proofs where UI-facing), **then** the shipped behaviour holds: auto-generated "is this working?" operator reports (equity, Sharpe/Sortino/drawdown, attribution, system health).
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [x] `operator-success-reports` 1.7.0 - the base feature (shipped)

## Dev Notes

- Source feature folder: `spec/v1/operator-success-reports/` - frontmatter status **`shipped`** (verbatim), version `1.7.0`, updated `2026-06-17`.
- Status mapping: `shipped` -> `done` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Core infrastructure.
- Provenance: `git log -- spec/v1/operator-success-reports` (full narrative); reports under `evidence/v1/operator-success-reports/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-OPERATOR-REPORTS-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 6 (Remediation, Infra & Governance (P0-P8, lints, BMAD migration))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
