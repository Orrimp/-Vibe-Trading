---
slug: adr-registry-atomic-lint
status: draft
owner: analyst
updated: 2026-05-29
---

# Tasks — adr-registry-atomic-lint v0.1.0

> **Analyst handoff 2026-05-29.** Per Pick C Wave 1 promotion in
> [`spec/dev-notes/pick-c-orchestrator-hygiene-2026-05-29.md`](../dev-notes/pick-c-orchestrator-hygiene-2026-05-29.md).
> ~0.5 dev day + ~0.25 tester day total. Bias DURABLE per
> [AGENT.md § Decision framing — durable over quick](../../AGENT.md#decision-framing--durable-over-quick-operator-preference).

## M0 — Analyst (DONE 2026-05-29)

- [x] T-ADR-M0.1 — Feature brief authored — _accept: feature.md R1-R4 + R-NR (7 clauses) + K1-K4 + H1-H4 + Q-ADR-WHEN + Q-ADR-AMEND + pre-drawn 4-cell verdict tree_
- [x] T-ADR-M0.2 — Bundle direction dev-note authored — _accept: `spec/dev-notes/pick-c-orchestrator-hygiene-2026-05-29.md` ships with trio framing + Q-HYG-EMIT_
- [x] T-ADR-M0.3 — Backlog Active row appended under § Process / tooling — _accept: PROMOTED Queue → Active 2026-05-29 annotation_
- [x] T-ADR-M0.4 — Trace row `REQ-ADR-REGISTRY-ATOMIC-LINT-001` opened `proposed` — _accept: appended at EOF spec/trace.toml_

## M-T1 — Architect (PENDING)

- [ ] T-ADR-T1.1 — Ratify Q-ADR-WHEN (pre-commit hook) + Q-ADR-AMEND (always bump) per bundle Q-HYG-EMIT — _accept: § Design § Operator-decide ratifications records both on Recommended DURABLE path; fast-skip expected_
- [ ] T-ADR-T1.2 — Lock script path + invocation contract — _accept: D-ADR-1 documents `scripts/adr_registry_check.py` with `python3 scripts/adr_registry_check.py --pre-commit` invocation; exit codes 0/1/≥2 per R1.5_
- [ ] T-ADR-T1.3 — Lock `git diff` semantics per Q-ADR-WHEN ratification — _accept: D-ADR-2 documents the exact `git diff --cached --name-only -- 'spec/architecture/adr/*'` invocation; falls back gracefully on missing staged changes_
- [ ] T-ADR-T1.4 — Resolve Q-ADR-STATUS-ENUM open carve-out — _accept: D-ADR-3 documents the v0.1.0 status enum (analyst recommends `{accepted, proposed, superseded, deprecated}`); "withdrawn" handling deferred to v0.2.0 with explicit "no withdrawn ADRs in tree at v0.1.0" assertion_
- [ ] T-ADR-T1.5 — Author architect.md § ADR registry atomic-write amendment — _accept: 1-line invocation example added under existing § contract (codified 2026-05-29); brief OWNS this amendment per bundle K4 ownership-table; no architect.md sibling-section drift_
- [ ] T-ADR-T1.6 — Pre-draw self-test cases — _accept: D-ADR-4 lists ≥ 4 self-test cases (missing row / updated-not-bumped / status-out-of-enum / exclude-rule on TEMPLATE.md)_
- [ ] T-ADR-T1.7 — Falsification probe P-ADR-1 spec'd — _accept: § Design § Falsification probe P-ADR-1 includes (a) stage an unregistered ADR file (synthetic 0099-test.md), assert lint exits 1; (b) un-stage + assert exit 0_
- [ ] T-ADR-T1.8 — Frontmatter flipped — _accept: feature.md + tasks.md status: draft → arch-done, owner: analyst → developer_

## M-DEV — Developer (single wave; ~0.5 day; architect-ratified)

- [ ] T-ADR-D1 — Author `scripts/adr_registry_check.py` per § Design D-ADR-1 — _accept: script runs `python3 scripts/adr_registry_check.py --pre-commit` from repo root; exits 0 on clean main; ≤ 150 LoC per H2_
- [ ] T-ADR-D2 — Implement ADR file discovery + frontmatter parse — _accept: globs `spec/architecture/adr/[0-9][0-9][0-9][0-9]-*.md`; reads each frontmatter; skips TEMPLATE.md + README.md per R1.3_
- [ ] T-ADR-D3 — Implement invariant (a) registry-row check per R1.2 — _accept: parses README.md ## Registry table; asserts every ADR file has a corresponding row (match by ADR number)_
- [ ] T-ADR-D4 — Implement invariant (b) updated-bump check per R1.2 — _accept: invokes `git diff` per Q-ADR-WHEN ratification; if any ADR file in scope was modified, asserts README.md frontmatter `updated:` field was also modified in the same diff_
- [ ] T-ADR-D5 — Implement invariant (c) status enum check per R1.2 — _accept: per-ADR frontmatter `status:` matches v0.1.0 enum from D-ADR-3_
- [ ] T-ADR-D6 — Implement markdown-table emit per R2.1 + bundle Q-HYG-EMIT — _accept: stderr emit on drift; markdown table with invariant + file + observed + expected columns_
- [ ] T-ADR-D7 — Author self-test per R4.1 — _accept: `scripts/tests/test_adr_registry_check.py` OR inline `--self-test` flag; ≥ 4 cases pass per T-ADR-T1.6; sub-1-s wall-clock_
- [ ] T-ADR-D8 — Run falsification probe P-ADR-1 — _accept: synthetic unregistered ADR → exit 1 + diff message; un-stage → exit 0_
- [ ] T-ADR-D9 — Verify R-NR contract — _accept: `git diff spec/architecture/adr/` shows zero edits from script run (READ-ONLY); anchors all-PASS_
- [ ] T-ADR-D10 — Amend architect.md § ADR registry atomic-write contract with invocation example per T-ADR-T1.5 — _accept: 1-line addition under existing § contract; no overlap with bundle siblings_
- [ ] T-ADR-D11 — Update `spec/trace.toml` `REQ-ADR-REGISTRY-ATOMIC-LINT-001` row — _accept: `crates = []`; `tests` lists self-test path; `state = "dev-done"`_
- [ ] T-ADR-D12 — Dev-side gates — _accept: cargo fmt clean (no Rust); spec-lint zero new violations; smoke `python3 scripts/adr_registry_check.py --pre-commit` exits 0 on current main_

## M-FINAL — Tester

- [ ] T-ADR-FINAL.1 — Run `python3 scripts/adr_registry_check.py --pre-commit` on current main — _accept: exit code 0; zero output (assumes current main has clean registry; if not, surface as test finding)_
- [ ] T-ADR-FINAL.2 — Inject synthetic drift cases (one per invariant a/b/c) in scratch worktree; run script; revert — _accept: exit code 1 for each; markdown table identifies the right invariant + file_
- [ ] T-ADR-FINAL.3 — Run self-test per R4.2 — _accept: all-PASS; sub-1-s_
- [ ] T-ADR-FINAL.4 — Verify R-NR.5 anchors — _accept: `bash scripts/verify_anchors.sh` → all-PASS byte-identical_
- [ ] T-ADR-FINAL.5 — Verify bundle dialect — _accept: diff message matches bundle Q-HYG-EMIT contract (markdown table); cross-checked against sibling scripts if shipped_
- [ ] T-ADR-FINAL.6 — Write test-final report — _accept: `spec/adr-registry-atomic-lint/reports/test-20260529-v0.1.0-adr-registry-atomic-lint.md` VERDICT → PASS or FAIL_

## M-PRESENT — Presenter

- [ ] T-ADR-P1 — Deck `spec/adr-registry-atomic-lint/presentations/adr-registry-atomic-lint-<date>.md` — _accept: trio framing recap; per-invariant explanation; pre-commit hook install recipe (if Q-ADR-WHEN=(a) ratified); sample drift detection_

## Notes

- **Anchor contract**: all-PASS byte-identical pre/post. Script
  addition only.
- **Bundle ownership**: this feature is the CHEAPEST pillar of Pick C
  Wave 1 (~0.5 dev day; sibling `queue-staleness-reconciliation`
  is ~1d, `operator-ledger-schema-lint` is ~0.5d). PARALLEL-SAFE.
- **architect.md amendment ownership**: this brief OWNS the
  architect.md § ADR registry atomic-write contract invocation-
  example amendment per bundle K4 ownership-table.
- **Per-cycle benefit (Rank 5 in process-tooling-survey)**: MEDIUM
  — high-frequency surface (every ADR-touching commit); preventive
  vs the 3× reactive cleanup cycles paid in 3 weeks.
