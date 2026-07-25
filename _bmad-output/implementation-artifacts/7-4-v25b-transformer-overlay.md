# Story 7.4: v25b-transformer-overlay

Status: retired

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: spec/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the vanilla decoder-only Transformer overlay (phase 3 of 4) - null; deprecated,
so that the measured dead-ends stay on the record so they are never re-litigated.

## Acceptance Criteria

1. **Given** the repo history at `v25b-transformer-overlay`'s landing commits (`git log -- spec/v1/v25b-transformer-overlay`), **when** the recorded verification for `v25b-transformer-overlay` is replayed (tests, reports under `spec/v1/v25b-transformer-overlay/reports/` where present, render proofs where UI-facing), **then** the shipped behaviour holds: the vanilla decoder-only Transformer overlay (phase 3 of 4) - null; deprecated.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [x] `v25b-transformer-overlay` 2.5.2 - the base feature (deprecated)

## Dev Notes

- Source feature folder: `spec/v1/v25b-transformer-overlay/` - frontmatter status **`deprecated`** (verbatim), version `2.5.2`, updated `2026-06-17`.
- Status mapping: `deprecated` -> `retired` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- Disposition: deprecated (superseded / measured NO-GO) — treated as retired.
- CHANGELOG index: CHANGELOG § Retired research lines.
- Provenance: `git log -- spec/v1/v25b-transformer-overlay` (full narrative); reports under `spec/v1/v25b-transformer-overlay/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-V25B-TRANSFORMER-001` (state=`deprecated`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 7 (Retired Research Lines (measured-and-retired bets))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
