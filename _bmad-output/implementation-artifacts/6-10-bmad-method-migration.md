# Story 6.10: bmad-method-migration

Status: in-progress

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the operator-ratified full migration to BMAD-METHOD v6.10.0 (7 phases; Phase 0 install + Phase 1 planning docs landed; THIS story - Phase 2 retro epics/stories/sprint-status - is the live work; Phases 3-5c pending),
so that the gates, ledgers, and process infrastructure keep the repo honest without manual vigilance.

## Acceptance Criteria

1. **Given** the ratified migration plan and the Phase 0/1 commits on main, **when** Phases 2-5c execute (epics/stories, corpus move + anchor base-swap, knowledge move, personas, lint re-founding, docs cutover) with gates green at every commit, **then** spec/ is retired with zero guarantees dropped: verify_anchors 119/119, re-founded spec_lint PASS, trace ledger authoritative at its new path.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [x] Phase 0 - install BMAD-METHOD v6.10.0 (additive; commit `5582a74`)
- [x] Phase 1 - planning docs (PRD.md + PRD-addendum.md + architecture.md under `_bmad-output/planning-artifacts/`)
- [x] Phase 2 - retro epics + stories + sprint-status (THIS story; `_bmad-output/` only, gates untouched)
- [x] Phase 3 - move report corpus -> `evidence/`, base-swap anchors (must hold 119/119 in the same commit; commit `452ce02`)
- [x] Phase 4 - move knowledge -> `docs/` (`git mv` spec/{dev-notes,runbooks,design,ui-design-principles.md} -> docs/, spec/architecture/adr -> `_bmad-output/planning-artifacts/architecture/decisions/`, atomic per AD-18; 211 renames, 0 deletions; `bash scripts/verify_anchors.sh` -> 119/119; `python3 scripts/spec_lint.py` -> PASS(0); `python3 scripts/adr_registry_check.py --self-test` -> OK, `--pre-commit` -> exit 0; `cargo test -p ui --lib` -> 617 passed)
- [ ] Phase 5a - persona customization TOMLs; 5b - re-found lints + retire `spec/`; 5c - docs cutover

## Dev Notes

- Forward story - no `spec/<slug>/` feature folder exists (see References).
- CHANGELOG index: not yet in CHANGELOG (live work; plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).

### References

- Trace: none — known trace-coverage gap (spec audit 2026-07-06); no `[[req]]` row in `spec/trace.toml`
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 6 (Remediation, Infra & Governance (P0-P8, lints, BMAD migration))
- Plan: `docs/dev-notes/bmad-migration-plan-2026-07-24.md` (ratified 2026-07-24, all 7 decisions resolved).

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
