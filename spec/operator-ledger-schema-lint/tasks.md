---
slug: operator-ledger-schema-lint
status: draft
owner: analyst
updated: 2026-05-29
---

# Tasks — operator-ledger-schema-lint v0.1.0

> **Analyst handoff 2026-05-29.** Per Pick C Wave 1 promotion in
> [`spec/dev-notes/pick-c-orchestrator-hygiene-2026-05-29.md`](../dev-notes/pick-c-orchestrator-hygiene-2026-05-29.md).
> ~0.5 dev day + ~0.25 tester day total. Bias DURABLE per
> [AGENT.md § Decision framing — durable over quick](../../AGENT.md#decision-framing--durable-over-quick-operator-preference).

## M0 — Analyst (DONE 2026-05-29)

- [x] T-LED-M0.1 — Feature brief authored — _accept: feature.md R1-R4 + R-NR (7 clauses) + K1-K4 + H1-H4 + Q-LED-WHEN + Q-LED-NOTE + pre-drawn 4-cell verdict tree_
- [x] T-LED-M0.2 — Bundle direction dev-note authored — _accept: `spec/dev-notes/pick-c-orchestrator-hygiene-2026-05-29.md` ships with trio framing + Q-HYG-EMIT_
- [x] T-LED-M0.3 — Backlog Active row appended under § Process / tooling — _accept: PROMOTED Queue → Active 2026-05-29 annotation_
- [x] T-LED-M0.4 — Trace row `REQ-OPERATOR-LEDGER-SCHEMA-LINT-001` opened `proposed` — _accept: appended at EOF spec/trace.toml_

## M-T1 — Architect (PENDING)

- [ ] T-LED-T1.1 — Ratify Q-LED-WHEN (session pre-flight) + Q-LED-NOTE (require dev-note) per bundle Q-HYG-EMIT — _accept: § Design § Operator-decide ratifications records both on Recommended DURABLE path; fast-skip expected_
- [ ] T-LED-T1.2 — Lock script path + invocation contract — _accept: D-LED-1 documents `scripts/operator_ledger_check.py` with `python3 scripts/operator_ledger_check.py` invocation; exit codes 0/1/≥2 per R1.5_
- [ ] T-LED-T1.3 — Lock SCHEMA constant per K2 mitigation — _accept: D-LED-2 documents the module-top `SCHEMA = {...}` dict with per-table column lists + required columns + optional columns. Per-table parse shape ratified (handles markdown-formatted cells per K1)_
- [ ] T-LED-T1.4 — Lock STALE_FAILED_DAYS = 7 per K3 default — _accept: D-LED-3 documents the named constant with rationale; v0.1.x patch path documented_
- [ ] T-LED-T1.5 — Author ledger frontmatter amendment per R3.3 — _accept: 1-line "validated by `scripts/operator_ledger_check.py`" note added under existing ledger frontmatter; brief OWNS the ledger frontmatter touch per bundle K4 ownership-table; no row body modifications_
- [ ] T-LED-T1.6 — Author AGENT.md cross-reference per R3.4 — _accept: 1-line cross-reference from AGENT.md to the ledger; architect picks the exact section (analyst recommends new "## Pending operator verifications" subsection OR cross-link under Queue pre-flight)_
- [ ] T-LED-T1.7 — Pre-draw self-test cases — _accept: D-LED-4 lists ≥ 5 self-test cases (clean / schema enum / stale at day 8 / not-stale at day 1 / missing completion date / Bug #64 D.1.1 regression-case verbatim parse)_
- [ ] T-LED-T1.8 — Falsification probe P-LED-1 spec'd — _accept: § Design § Falsification probe P-LED-1 includes (a) inject synthetic stale-FAILED 8-day-old row into mock ledger, assert lint exits 1 with escalation message; (b) revert + assert exit 0_
- [ ] T-LED-T1.9 — Frontmatter flipped — _accept: feature.md + tasks.md status: draft → arch-done, owner: analyst → developer_

## M-DEV — Developer (single wave; ~0.5 day; architect-ratified)

- [ ] T-LED-D1 — Author `scripts/operator_ledger_check.py` per § Design D-LED-1 — _accept: script runs `python3 scripts/operator_ledger_check.py` from repo root; exits 0 on clean ledger; ≤ 200 LoC per H2_
- [ ] T-LED-D2 — Implement markdown-table parser per D-LED-2 — _accept: handles pipe-separated rows, escaped pipes (`\|`), markdown-formatted cells (strip bold/italic/links before enum match); parses all 3 tables (Pending / Done / Cancelled); regression-tests against the verbatim Bug #64 D.1.1 row_
- [ ] T-LED-D3 — Implement schema check per R1.4 (a) + (b) + (c) — _accept: per-row column count matches per-table schema; status enum normalizes to {pending, FAILED, done, cancelled}; date parses ISO_
- [ ] T-LED-D4 — Implement stale-FAILED escalation per R1.4 (e) — _accept: today's date - `Date surfaced` ≥ STALE_FAILED_DAYS (= 7) AND status = FAILED → escalation row; within-window FAILED emits soft-warning per R2.5_
- [ ] T-LED-D5 — Implement Q-LED-NOTE check per R1.4 (f) — _accept: FAILED rows require `spec/dev-notes/.*\.md` path match in Notes cell; missing citation → exit 1_
- [ ] T-LED-D6 — Implement done-row completion-date check per R1.4 (d) — _accept: Done table rows have ISO date in `Completed` column; missing → exit 1_
- [ ] T-LED-D7 — Implement markdown-table emit per R2.1 + bundle Q-HYG-EMIT — _accept: stderr emit on issue; markdown table with issue / row / observed / expected / action columns_
- [ ] T-LED-D8 — Author self-test per R4.1 — _accept: `scripts/tests/test_operator_ledger_check.py` OR inline `--self-test` flag; ≥ 5 cases pass per T-LED-T1.7; sub-1-s_
- [ ] T-LED-D9 — Run falsification probe P-LED-1 — _accept: synthetic 8-day-old stale-FAILED → exit 1 + escalation message; revert → exit 0_
- [ ] T-LED-D10 — Verify R-NR contract — _accept: `git diff spec/dev-notes/operator-side-pending-ledger.md` shows ONLY the frontmatter R3.3 amendment, zero row body modifications; anchors all-PASS_
- [ ] T-LED-D11 — Apply ledger frontmatter R3.3 amendment — _accept: 1-line "validated by `scripts/operator_ledger_check.py`" added to frontmatter; ledger rows byte-identical_
- [ ] T-LED-D12 — Amend AGENT.md per R3.4 cross-reference — _accept: 1-line cross-reference added per T-LED-T1.6; no overlap with bundle siblings' AGENT.md / architect.md amendments_
- [ ] T-LED-D13 — Update `spec/trace.toml` `REQ-OPERATOR-LEDGER-SCHEMA-LINT-001` row — _accept: `crates = []`; `tests` lists self-test path; `state = "dev-done"`_
- [ ] T-LED-D14 — Dev-side gates — _accept: cargo fmt clean (no Rust); spec-lint zero new violations; smoke `python3 scripts/operator_ledger_check.py` exits 0 on current main ledger_

## M-FINAL — Tester

- [ ] T-LED-FINAL.1 — Run `python3 scripts/operator_ledger_check.py` on current main — _accept: per the current ledger state (1 FAILED row at 1 day age + 4 done rows), exit code 0 with optional within-window soft-warning for Bug #64 D.1.1; OR exit 1 if Q-LED-NOTE fires on a missing citation (tester surfaces as finding)_
- [ ] T-LED-FINAL.2 — Inject synthetic 8-day-old stale-FAILED row in scratch worktree; run script; revert — _accept: exit 1; markdown table identifies the row + escalation issue class_
- [ ] T-LED-FINAL.3 — Inject schema-enum violation (`status: "blocked"`) in scratch; run; revert — _accept: exit 1; identifies schema-status-enum issue class_
- [ ] T-LED-FINAL.4 — Run self-test per R4.2 — _accept: all-PASS; sub-1-s_
- [ ] T-LED-FINAL.5 — Verify R-NR.4 anchors — _accept: `bash scripts/verify_anchors.sh` → all-PASS byte-identical_
- [ ] T-LED-FINAL.6 — Verify bundle dialect — _accept: diff message matches bundle Q-HYG-EMIT contract (markdown table); cross-checked against sibling scripts if shipped_
- [ ] T-LED-FINAL.7 — Write test-final report — _accept: `spec/operator-ledger-schema-lint/reports/test-20260529-v0.1.0-operator-ledger-schema-lint.md` VERDICT → PASS or FAIL_

## M-PRESENT — Presenter

- [ ] T-LED-P1 — Deck `spec/operator-ledger-schema-lint/presentations/operator-ledger-schema-lint-<date>.md` — _accept: trio framing recap; ledger schema table; stale-FAILED escalation contract; sample detection (the verbatim Bug #64 D.1.1 row regression test); operator-decide-ready_

## Notes

- **Anchor contract**: all-PASS byte-identical pre/post. Script
  addition + 1-line ledger frontmatter amendment + 1-line AGENT.md
  cross-reference; zero anchored-report touches; zero Rust touches.
- **Bundle ownership**: this feature is one of two CHEAPER pillars
  of Pick C Wave 1 (~0.5 dev day; sibling
  `queue-staleness-reconciliation` is ~1d, `adr-registry-atomic-lint`
  is ~0.5d). PARALLEL-SAFE with both siblings.
- **Ledger ownership**: this brief OWNS the ledger frontmatter
  amendment (R3.3) AND the AGENT.md cross-reference (R3.4) per
  bundle K4 ownership-table. Siblings own their AGENT.md / architect.md
  amendments; no overlap.
- **Append-only contract**: the lint preserves the ledger's
  existing append-only contract — READ-ONLY on row bodies; only
  frontmatter touch is the R3.3 amendment.
- **Per-cycle benefit (Rank 5 in process-tooling-survey)**: MEDIUM
  — preventive stale-FAILED escalation at every session start;
  consolidates the chronic carry-over class (Bug #64 visual-verify,
  Yahoo bulk fetch, toast-queue smoke tests).
