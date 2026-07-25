# Story 3.16: advisor-crossasset-macro-regime

Status: review

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: spec/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the macro risk-on/off probe (v0.macro_riskon over ^GSPC/DXY/^TNX) + the durable market-calendar layer - FRAGILE, the pre-registered null,
so that the guided EUR-200 journey stays honest, reproducible, and robustness-gated end to end.

## Acceptance Criteria

1. **Given** the built-and-verified state frozen at frontmatter `tester-done` (2026-06-17 spec compression), **when** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived, **then** the delivered behaviour stands as recorded: the macro risk-on/off probe (v0.macro_riskon over ^GSPC/DXY/^TNX) + the durable market-calendar layer - FRAGILE, the pre-registered null.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [ ] `advisor-crossasset-macro-regime` 0.1.0 - the base feature (tester-done)

## Dev Notes

- Source feature folder: `spec/v1/advisor-crossasset-macro-regime/` - frontmatter status **`tester-done`** (verbatim), version `0.1.0`, updated `2026-06-28`.
- Status mapping: `tester-done` -> `review` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Advisor — `**advisor-crossasset-macro-regime**`.
- Provenance: `git log -- spec/v1/advisor-crossasset-macro-regime` (full narrative); reports under `spec/v1/advisor-crossasset-macro-regime/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-ADVISOR-CROSSASSET-MACRO-REGIME-001` (state=`tested`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 3 (Advisor MVP (F1-F9 + EUR-FX + dynamic data + PIT discipline))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
