# Story 5.2: advisor-corpus-expansion

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the P2 corpus expansion (4 new pinned corpora + 2nd venue) + the ship-passive verdict re-run that era-qualified the thesis (efficiency migration; scorecard errata honored),
so that the shipped product is provably done, with its thesis boundary honestly mapped.

## Acceptance Criteria

1. **Given** the repo history at `advisor-corpus-expansion`'s landing commits (`git log -- spec/v3/advisor-corpus-expansion`), **when** the recorded verification for `advisor-corpus-expansion` is replayed (tests, reports under `evidence/v3/advisor-corpus-expansion/reports/` where present, render proofs where UI-facing), **then** the shipped behaviour holds: the P2 corpus expansion (4 new pinned corpora + 2nd venue) + the ship-passive verdict re-run that era-qualified the thesis (efficiency migration; scorecard errata honored).
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [x] `advisor-corpus-expansion` 3.3.1 - the base feature (shipped)

## Dev Notes

- Source feature folder: `spec/v3/advisor-corpus-expansion/` - frontmatter status **`shipped`** (verbatim), version `3.3.1`, updated `2026-07-10`.
- Status mapping: `shipped` -> `done` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § v3 — "prove it's done" close-out.
- Provenance: `git log -- spec/v3/advisor-corpus-expansion` (full narrative); reports under `evidence/v3/advisor-corpus-expansion/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-V3-P2-CORPUS-EXPANSION-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 5 (v3 "Prove It's Done" Close-Out)

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
