# Story 7.11: vol-killswitch-overlay-noop-fix

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the fix for the computed-but-unapplied vol kill-switch overlay - the precedent behind the day-1 baseline-equity-divergence e2e non-negotiable,
so that the measured dead-ends stay on the record so they are never re-litigated.

## Acceptance Criteria

1. **Given** the repo history at `vol-killswitch-overlay-noop-fix`'s landing commits (`git log -- spec/v1/vol-killswitch-overlay-noop-fix`), **when** the recorded verification for `vol-killswitch-overlay-noop-fix` is replayed (tests, reports under `evidence/v1/vol-killswitch-overlay-noop-fix/reports/` where present, render proofs where UI-facing), **then** the shipped behaviour holds: the fix for the computed-but-unapplied vol kill-switch overlay - the precedent behind the day-1 baseline-equity-divergence e2e non-negotiable.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [x] `vol-killswitch-overlay-noop-fix` 0.1.0 - the base feature (shipped)

## Dev Notes

- Source feature folder: `spec/v1/vol-killswitch-overlay-noop-fix/` - frontmatter status **`shipped`** (verbatim), version `0.1.0`, updated `2026-06-17`.
- Status mapping: `shipped` -> `done` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Retired research lines.
- Provenance: `git log -- spec/v1/vol-killswitch-overlay-noop-fix` (full narrative); reports under `evidence/v1/vol-killswitch-overlay-noop-fix/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-VOL-KILLSWITCH-NOOP-FIX-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 7 (Retired Research Lines (measured-and-retired bets))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
