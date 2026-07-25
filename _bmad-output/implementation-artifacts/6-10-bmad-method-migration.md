# Story 6.10: bmad-method-migration

Status: in-progress

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the operator-ratified full migration to BMAD-METHOD v6.10.0 (7 phases; Phases 0-4 landed (install, planning docs, retro epics/stories/sprint-status — THIS story — evidence/ corpus move, docs/ knowledge move); Phase 5a persona customization TOMLs + the two flagged path-sweep debts landed; Phase 5b re-founded the governance-lint triad onto the story/trace layout and retired `spec/` entirely; Phase 5c pending),
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
- [x] Phase 5a - persona customization TOMLs (`_bmad/custom/bmad-agent-{analyst,architect,dev,ux-designer,tech-writer}.toml` + `bmad-code-review.toml` + `bmad-sprint-status.toml`, all `tomllib`-valid and `resolve_customization.py`-verified; presenter mapped to `bmad-agent-tech-writer` per this pass's dispatch, a deliberate delta from this plan's own § 6 table which named `bmad-agent-pm` — noted in the tech-writer override's header; no-BMAD-twin charter notes for spec-auditor/ui-debugger/researcher at `_bmad/custom/{spec-auditor,ui-debugger,researcher}-charter.md`) + the two flagged path-sweep debts (`.claude/skills/*/SKILL.md` reports/dev-notes path refs, 11 files fixed, `spec-update` given a Phase-5c deprecation header per ratified D5; `_bmad-output/implementation-artifacts/*.md` provenance pointers, 402 substitutions across 141 files, scripted + sample-verified); `bash scripts/verify_anchors.sh` -> 119/119; `python3 scripts/spec_lint.py` -> PASS(0); `python3 scripts/adr_registry_check.py --pre-commit` -> exit 0 (commit pending — orchestrator commits per this project's git-authority contract)
- [x] Phase 5b - re-found lints + retire `spec/` (155 feature folders + product.md/architecture.md/architecture/*.md/backlog.md/bug-log.md/trace.toml disposed per plan: 291 pure `git mv` renames [`spec/archive/`→`docs/archive/`; `spec/{v1,v2,v3,bare,v5-*}/**`→`docs/archive/pre-bmad-spec/**` co-located with feature.md/tasks.md/decomp.md/dev-notes; 15 `presentations/` dirs extracted→`evidence/**/presentations/` sibling to `reports/`] + 6 rename+edit [`trace.toml`→`_bmad-output/planning-artifacts/trace.toml` content-rewritten (495 path substitutions + 3 pre-existing ADR-0082 state/status drifts found+fixed: carry-strategy tester-done→retired, simple-strategies-realdata shipped→presenter-done, cockpit-cross-platform dev-done→in-progress — each PRIOR-preserved in a trace.toml comment]; `bug-log.md`→`docs/dev-notes/bug-log.md`; 4 presentation decks with `../feature.md` cross-root links] + 60 content-only edits [34 ADRs + 6 dev-notes + 3 runbooks + ui-design-principles.md + 5 evidence screenshot READMEs + 2 residual functional-code path leftovers Phase 4 missed (`crates/backtest/examples/passive_baseline_equity.rs` DEFAULT_OUT_DIR, `crates/ui/tests/{layout_invariants.rs,fixtures/mod.rs}` fixture strings) + 7 scripts]; NEW `_bmad-output/planning-artifacts/backlog.md` authored (forward-Queue-only, per plan D-row); `spec/` directory gone (0 deletions — everything archived or repointed). `scripts/spec_lint.py` fully re-founded onto the story/trace layout (story↔trace bridge via the REQ-id embedded in each story's `### References` line, NOT slug-string matching — story filenames sanitize dots/add disambiguating prefixes; CATEGORIES re-keyed: `orphan-story`, `story-done-no-tests` [faithfully narrow-scoped to match the pre-existing bare-folder-only behavior — verified empirically still a no-op], `status-drift` [re-founded: full status-vocabulary mapping, not just the shipped/done terminal case], `story-done-trace-drift`, `story-done-changelog-missing` [+6 new CHANGELOG_ROLLUP_ALLOWLIST entries discovered: v3-llm-forecaster's "v3 LLM-forecaster" prose-spacing variant + 5 lumen sub-phases covered only by the parent rollup line]); `scripts/{spec_brief,precheck,queue_staleness_check,operator_ledger_check,adr_registry_check,check_no_secrets_in_llm_artifacts}` repointed (`queue_staleness_check.py` also relaxed `## Active` to optional — the new backlog.md is Queue-only by design). Gates: `bash scripts/verify_anchors.sh` -> 119/119; `python3 scripts/spec_lint.py` -> PASS (0 violations); `python3 scripts/spec_lint.py --self-test` -> 3/3 PASS (status-drift, story-done-trace-drift, story-done-changelog-missing) + a real-tree negative control (mutated THIS file's sibling `1-2-v05-composed-strategies.md` `Status: done`→`review`, observed exactly 1 `status-drift` FAIL, restored verbatim, re-confirmed PASS(0)); `python3 scripts/adr_registry_check.py --self-test` -> OK (5/5), bare run -> exit 0; `python3 scripts/queue_staleness_check.py --self-test` -> all cases PASS, live run -> exit 0 (silent, clean); `cargo build --workspace` -> Finished (8m09s); `cargo test -p reports --lib` -> 103 passed, 0 failed (incl. `parse::tests::all_anchored_reports_parse_ok`); `cargo test -p ui --lib` -> 617 passed, 0 failed; `cargo test -p ui --test layout_invariants` -> 11 passed, 0 failed. `git status` census: 291 R + 6 RM + 60 M + 1 untracked (the new backlog.md), 0 deletions, byte-frozen `evidence/**` report bodies untouched (verify_anchors 119/119 proves it; the only evidence/ diffs are NEW `presentations/` content + 5 non-anchored screenshot-manifest README.md edits). Remaining `spec/` string hits: `.claude/skills/*/SKILL.md` (~41, explicitly Phase 5c scope per the plan's own phasing — Phase 5a already did a partial pass there) + historical/frozen prose inside `docs/archive/**` (byte-irrelevant, skip-listed by the lint's own `iter_tree_md`) + 2 KNOWN_FROZEN_DEAD_LINKS-allowlisted dead links inside 2 byte-immutable anchored `evidence/v3/advisor-corpus-expansion/reports/backtest-2026-07-10-*.md` bodies (pre-existing pattern, cannot edit without breaking the anchor SHA, allowlist entry added).
- [ ] Phase 5c - docs cutover

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
