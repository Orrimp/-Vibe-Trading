# Story 2.52: ui-gallery-bin

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the widget-gallery binary,
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

## Acceptance Criteria

1. **Given** the repo history at `ui-gallery-bin`'s landing commits (`git log -- spec/v1/ui-gallery-bin`), **when** the recorded verification for `ui-gallery-bin` is replayed (tests, reports under `evidence/v1/ui-gallery-bin/reports/` where present, render proofs where UI-facing), **then** the shipped behaviour holds: the widget-gallery binary.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [x] `ui-gallery-bin` 0.1.0-partial-terminal - the base feature (shipped)

## Dev Notes

- Source feature folder: `spec/v1/ui-gallery-bin/` - frontmatter status **`shipped`** (verbatim), version `0.1.0-partial-terminal`, updated `2026-06-17`.
- Status mapping: `shipped` -> `done` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Cockpit & UI › iced platform & UI quality gates.
- Provenance: `git log -- spec/v1/ui-gallery-bin` (full narrative); reports under `evidence/v1/ui-gallery-bin/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: none — known trace-coverage gap (spec audit 2026-07-06); no `[[req]]` row in `spec/trace.toml`
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 2 (Cockpit & UI (Lumen shell, Live, Lab, charts, quality gates))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
