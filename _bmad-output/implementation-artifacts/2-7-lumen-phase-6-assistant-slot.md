# Story 2.7: lumen-phase-6-assistant-slot

Status: backlog

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the reserved Lumen Phase 6 Assistant slot (forward-compat reservation only, gated on a v2 LLM assistant),
so that the cockpit shows the true system state and every UI claim is provable at the rendered-pixel layer.

## Acceptance Criteria

1. **Given** the recorded brief in `spec/lumen-design-adoption/phase-6-assistant-slot/feature.md`, **when** the operator schedules the work (post do-not-build-register check), **then** the story delivers: the reserved Lumen Phase 6 Assistant slot (forward-compat reservation only, gated on a v2 LLM assistant).
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [ ] `lumen-phase-6-assistant-slot` 2.5.0 - the base feature (reserved)

## Dev Notes

- Source feature folder: `spec/lumen-design-adoption/phase-6-assistant-slot/` - frontmatter status **`reserved`** (verbatim), version `2.5.0`, updated `2026-05-04`.
- Status mapping: `reserved` -> `backlog` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- Disposition: reserved — forward-compat reservation only (gated on a v2 LLM assistant); deliberately not built.
- CHANGELOG index: CHANGELOG § Cockpit & UI › Shell, navigation & design system (reservation noted on the `lumen-design-adoption` line).
- Provenance: `git log -- spec/lumen-design-adoption/phase-6-assistant-slot` (full narrative); reports under `evidence/lumen-design-adoption/phase-6-assistant-slot/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: none — known trace-coverage gap (spec audit 2026-07-06); no `[[req]]` row in `spec/trace.toml`
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 2 (Cockpit & UI (Lumen shell, Live, Lab, charts, quality gates))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
