# Story 6.9: cockpit-cross-platform

Status: in-progress

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the cockpit on Linux/Windows: source shipped + macOS-verified; the 3-OS CI matrix ACTIVATED 2026-07-10 (P7) - the run-2 shakeout is the open in-progress work,
so that the gates, ledgers, and process infrastructure keep the repo honest without manual vigilance.

## Acceptance Criteria

1. **Given** the activated 3-OS CI matrix (ci.yml live on push/PR) with run-2 shakeout reds open, **when** the shakeout fixes land (fix-forward per the operator direction), **then** the Linux/Windows lanes go green and the story flips to done - until then it is honestly in-progress.
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

- [ ] `cockpit-cross-platform` 0.1.0 - the base feature (in-progress)

## Dev Notes

- Source feature folder: `spec/cockpit-cross-platform/` - frontmatter status **`in-progress`** (verbatim), version `0.1.0`, updated `2026-06-15`.
- Status mapping: `in-progress` -> `in-progress` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Deferred / not built (by decision) — superseded: CI activated 2026-07-10 (PRD §14 assumption).
- Provenance: `git log -- spec/cockpit-cross-platform` (full narrative); reports under `evidence/cockpit-cross-platform/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-COCKPIT-CROSS-PLATFORM-001` (state=`dev-done`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 6 (Remediation, Infra & Governance (P0-P8, lints, BMAD migration))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List
