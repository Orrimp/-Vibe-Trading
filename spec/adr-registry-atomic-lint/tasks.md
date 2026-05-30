---
slug: adr-registry-atomic-lint
status: arch-done
owner: developer
updated: 2026-05-30
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

## M-T1 — Architect (DONE 2026-05-30)

- [x] T-ADR-T1.1 — Ratify Q-ADR-WHEN (pre-commit hook) + Q-ADR-AMEND (always bump) per bundle Q-HYG-EMIT — _accept: § Design § Operator-decide ratifications records both on Recommended DURABLE path + Q-ADR-STATUS-ENUM RATIFIED after live grep_
- [x] T-ADR-T1.2 — Lock script path + invocation contract — _accept: D-ADR-1 documents `scripts/adr_registry_check.py` with `--pre-commit` (default) / `--self-test` / reserved `--ci`; exit codes 0/1/≥2 per R1.5; `def main(argv)->int` sibling convention_
- [x] T-ADR-T1.3 — Lock `git diff` semantics per Q-ADR-WHEN ratification — _accept: D-ADR-2 locks `git diff --cached --name-only --diff-filter=ACMR -- 'spec/architecture/adr/*.md'` (list-form, no shell=True) + README-staged proxy for (b) + graceful-skip + git-unavailable fail-closed exit 2_
- [x] T-ADR-T1.4 — Resolve Q-ADR-STATUS-ENUM — _accept: D-ADR-4 locks `STATUS_ENUM = {accepted, proposed, superseded, deprecated}` (matches README § Format line 22); "withdrawn" deferred to v0.2.0; live grep confirms zero out-of-enum status + "no withdrawn ADR in tree" assertion holds_
- [x] T-ADR-T1.5 — Author architect.md § ADR registry atomic-write amendment SPEC — _accept: D-ADR-6 gives the exact 1-paragraph mechanical-enforcement note (append after line 92) + the `deprecated` micro-fix on line 82; brief OWNS this per bundle K4 ownership-table; no sibling-section drift_
- [x] T-ADR-T1.6 — Pre-draw self-test cases — _accept: § Design § Self-test lists 5 cases (missing-row / updated-not-bumped / status-out-of-enum / exclude-rule on TEMPLATE.md+README.md / clean) + git-seam injection for case 2_
- [x] T-ADR-T1.7 — Falsification probe P-ADR-1 spec'd — _accept: § Design § P-ADR-1 self-contained recipe — create `9999-probe.md` (no README row) → assert exit 1 names ADR-9999; delete → assert exit 0; Cleanup confirms zero git delta_
- [x] T-ADR-T1.8 — Frontmatter flipped — _accept: feature.md + tasks.md status: draft → arch-done, owner: analyst → developer; trace.toml arch column updated + state → arch-done_

## M-DEV — Developer (single wave; ~0.5 day; architect-ratified)

- [ ] T-ADR-D1 — Author `scripts/adr_registry_check.py` per § Design D-ADR-1 — _accept: `#!/usr/bin/env python3` + executable bit; `def main(argv: list[str]) -> int` + `raise SystemExit(main(sys.argv[1:]))` sibling convention; argparse `--pre-commit` (default, also bare) / `--self-test` / reserved `--ci` (exit-2 not-implemented); `REPO_ROOT = Path(__file__).resolve().parent.parent`; runs from repo root; exits 0 on clean main; ≤ 150 LoC per H2_
- [ ] T-ADR-D2 — Implement ADR file discovery + frontmatter parse per D-ADR-4 — _accept: globs `spec/architecture/adr/[0-9][0-9][0-9][0-9]-*.md` (structurally excludes README.md + TEMPLATE.md per R1.3); ADR number from filename `^(\d{4})-`; frontmatter via `^---\n.*?\n---\n` (re.DOTALL, from hash_report.py) + `^status:\s*(\S+)` line-regex — NO yaml dep_
- [ ] T-ADR-D3 — Implement invariant (a) registry-row check per D-ADR-3 — _accept: README parser uses `^\|\s*(\d{4})\s*\|` row-regex anchored to `## Registry` section (SHOULD); builds registered-ID set; every discovered ADR number ∈ set or emits `(a) registry-row-missing`_
- [ ] T-ADR-D4 — Implement invariant (b) updated-bump check per D-ADR-2 — _accept: `_staged_adr_files()` runs `git diff --cached --name-only --diff-filter=ACMR -- 'spec/architecture/adr/*.md'` (list-form subprocess, cwd=REPO_ROOT); `_readme_staged()` checks README path in staged set; if any ADR staged AND README not staged → one `(b)` drift; graceful-skip when zero ADR staged; git-unavailable → exit 2 fail-closed; functions factored as seams for self-test injection_
- [ ] T-ADR-D5 — Implement invariant (c) status enum check per D-ADR-4 — _accept: module-level `STATUS_ENUM = frozenset({"accepted","proposed","superseded","deprecated"})`; per-ADR `status:` ∈ enum or emits `(c) status-out-of-enum`; missing-status frontmatter → (c) drift with observed "no status: frontmatter" (NOT crash)_
- [ ] T-ADR-D6 — Implement markdown-table emit per D-ADR-5 + bundle Q-HYG-EMIT — _accept: stderr; header `adr-registry-check: <N> drift(s) detected`; 4 cols (invariant | file | observed | expected); rows sorted by (invariant-letter, file); clean run zero output_
- [ ] T-ADR-D7 — Author self-test per D-ADR-6 self-test list (R4.1) — _accept: inline `--self-test` flag (single-file; no scripts/tests/ dir needed); 5 cases (missing-row / updated-not-bumped / status-out-of-enum / exclude TEMPLATE.md+README.md / clean); case 2 injects fake staged-file list via the seam (no real git index); sub-1-s_
- [ ] T-ADR-D8 — Run falsification probe P-ADR-1 per § Design recipe — _accept: create `spec/architecture/adr/9999-probe.md` (no README row) → exit 1 + `(a)` row naming ADR-9999; `rm` probe → exit 0; `git status spec/architecture/adr/` clean after cleanup_
- [ ] T-ADR-D9 — Verify R-NR contract — _accept: `git diff spec/architecture/adr/` shows zero edits from script run (READ-ONLY, no auto-fix); `bash scripts/verify_anchors.sh` all-PASS byte-identical_
- [ ] T-ADR-D10 — Amend architect.md § ADR registry per D-ADR-6 (K4-owned) — _accept: append the verbatim 1-paragraph "Mechanical enforcement." note AFTER line 92 (before `## Style`); update line 82 status-list parenthetical to add `/ deprecated`; ONLY this owned section touched — no AGENT.md, no ledger (sibling-owned)_
- [ ] T-ADR-D11 — Update `spec/trace.toml` `REQ-ADR-REGISTRY-ATOMIC-LINT-001` row — _accept: `crates = []`; `tests = ["scripts/adr_registry_check.py"]` (inline `--self-test`); `arch` column gains the D-clause refs; `state = "dev-done"`_
- [ ] T-ADR-D12 — Dev-side gates — _accept: `python3 scripts/adr_registry_check.py --self-test` all-pass; smoke `python3 scripts/adr_registry_check.py --pre-commit` exits 0 on current main (registry is CLEAN per § Pre-existing-debt findings); spec-lint zero new violations; no Rust → cargo fmt N/A but confirm zero crates/ delta_

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
