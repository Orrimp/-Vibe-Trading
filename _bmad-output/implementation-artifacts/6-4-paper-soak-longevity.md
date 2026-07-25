# Story 6.4: paper-soak-longevity

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the reflection-loop paper wiring (lesson card per closed trade) + the longevity soak evidence artifact + the 90-day soak runbook,
so that the gates, ledgers, and process infrastructure keep the repo honest without manual vigilance.

## Acceptance Criteria

1. **Given** the repo history at `paper-soak-longevity`'s landing commits (`git log -- spec/v1/paper-soak-longevity`), **when** the recorded verification for `paper-soak-longevity` is replayed (tests, reports under `evidence/v1/paper-soak-longevity/reports/` where present, render proofs where UI-facing), **then** the shipped behaviour holds: the reflection-loop paper wiring (lesson card per closed trade) + the longevity soak evidence artifact + the 90-day soak runbook.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [x] `paper-soak-longevity` 0.1.0 - the base feature (shipped)

## Dev Notes

- Source feature folder: `spec/v1/paper-soak-longevity/` - frontmatter status **`shipped`** (verbatim), version `0.1.0`, updated `2026-06-19`.
- Status mapping: `shipped` -> `done` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Core infrastructure.
- Provenance: `git log -- spec/v1/paper-soak-longevity` (full narrative); reports under `evidence/v1/paper-soak-longevity/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: none — known trace-coverage gap (spec audit 2026-07-06); no `[[req]]` row in `spec/trace.toml`
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 6 (Remediation, Infra & Governance (P0-P8, lints, BMAD migration))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
