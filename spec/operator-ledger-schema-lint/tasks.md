---
slug: operator-ledger-schema-lint
status: dev-done
owner: tester
updated: 2026-05-30
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

## M-T1 — Architect (DONE 2026-05-30)

- [x] T-LED-T1.1 — Ratify Q-LED-WHEN (session pre-flight) + Q-LED-NOTE (require dev-note) per bundle Q-HYG-EMIT — _accept: § Design § Operator-decide ratifications records both (a)/(a) on Recommended DURABLE path; fast-skip honoured; Q-HYG-EMIT inherited (a) markdown table_
- [x] T-LED-T1.2 — Lock script path + invocation contract — _accept: D-LED-1 documents `scripts/operator_ledger_check.py` (PEP-723 stdlib header) with `python3 scripts/operator_ledger_check.py` invocation; exit codes 0/1/≥2 per R1.5; HARD vs SOFT split decided (stale-FAILED + missing-citation + enum/column/date/completion = HARD exit 1; within-window FAILED = SOFT exit 0 stdout)_
- [x] T-LED-T1.3 — Lock SCHEMA constant per K2 mitigation — _accept: D-LED-2 documents the module-top `SCHEMA = {...}` dict keyed by 3 table headings with ordered required columns + status_col/date_col/completion_col; heading-anchored table discovery (robust to F1 no-blank-line-before-next-heading); markdown-tolerant `normalize_cell` + escaped-pipe split; single-physical-line row assumption with `schema-row-truncated` guard (fragility flagged)_
- [x] T-LED-T1.4 — Lock STALE_FAILED_DAYS = 7 + status enum + `--today` lever — _accept: D-LED-3 `CANONICAL_STATUS` + canonical reconciliation table (`fix-in-flight` = NON-canonical churn state, HARD if live); D-LED-4 `STALE_FAILED_DAYS = 7` named constant + `--today YYYY-MM-DD` determinism override (REQUIRED by self-test/P-LED-1 in 2026 future-clock env)_
- [x] T-LED-T1.5 — Spec ledger frontmatter amendment per R3.3 — _accept: § Design `### R3.3` specifies the `validated_by:` frontmatter line + Changelog row; applied by DEVELOPER at T-LED-D11 in one commit; no row body bytes change_
- [x] T-LED-T1.6 — Decide + author AGENT.md cross-reference per R3.4 — _accept: § Design `### R3.4` DECIDES new `### Pending operator-verification ledger` subsection nested under `## Queue pre-flight reconciliation sweep` (appended after its body, before `## The vibe-coding loop`); collision-safe vs queue-staleness sibling's numbered-steps edit; exact subsection text drafted; applied by DEVELOPER at T-LED-D12_
- [x] T-LED-T1.7 — Pre-draw self-test cases — _accept: D-LED-7 lists 8 cases (clean / schema-enum / stale@8d / not-stale@1d soft / missing-completion-date / missing-devnote-citation / cancelled-table-exclusion / Bug #64 D.1.1 verbatim regression); inline `--self-test` flag chosen over `scripts/tests/` per H2 LoC budget_
- [x] T-LED-T1.8 — Falsification probe P-LED-1 spec'd — _accept: § Design § P-LED-1 — temp-fixture (NOT live edit) synthetic FAILED row dated 2026-05-20, no citation, `--today 2026-05-29` → exit 1 with BOTH `stale-failed` (9d) + `missing-devnote-citation`; negative control (citation + 1-day-old) → exit 0 soft_
- [x] T-LED-T1.9 — Frontmatter flipped — _accept: feature.md + tasks.md status: draft → arch-done, owner: analyst → developer; trace.toml arch column + state = arch-done_

## M-DEV — Developer (single wave; ~0.5 day; architect-ratified)

- [x] T-LED-D1 — Author `scripts/operator_ledger_check.py` per § Design D-LED-1 — file: `scripts/operator_ledger_check.py:1-350`. Test: `python3 scripts/operator_ledger_check.py --self-test`. Output: `self-test: 8 passed` (exit 0).
- [x] T-LED-D2 — Implement heading-anchored 3-table parser per D-LED-2 — file: `scripts/operator_ledger_check.py:100-165` (`parse_ledger`). Test: `python3 scripts/operator_ledger_check.py --self-test` (case 8 Bug #64 D.1.1 verbatim regression). Output: `self-test: 8 passed`.
- [x] T-LED-D3 — Implement schema check per R1.4 (a)+(b)+(c) + D-LED-3 — file: `scripts/operator_ledger_check.py:185-225` (`check_rows` pending-status block). Test: `python3 scripts/operator_ledger_check.py --self-test` (case 2 schema-status-enum). Output: `self-test: 8 passed`.
- [x] T-LED-D4 — Implement stale-FAILED per R1.4 (e) + D-LED-4 — file: `scripts/operator_ledger_check.py:225-260`. Test: `python3 scripts/operator_ledger_check.py --self-test` (cases 3 stale@8d + 4 not-stale@1d). Output: `self-test: 8 passed`.
- [x] T-LED-D5 — Implement Q-LED-NOTE check per R1.4 (f) + D-LED-5 — file: `scripts/operator_ledger_check.py:260-275`. Test: `python3 scripts/operator_ledger_check.py --self-test` (case 6 missing-devnote-citation). Output: `self-test: 8 passed`.
- [x] T-LED-D6 — Implement done-row completion-date check per R1.4 (d) + D-LED-5 — file: `scripts/operator_ledger_check.py:277-310`. Test: `python3 scripts/operator_ledger_check.py --self-test` (case 5 missing-completion-date, case 7 cancelled-exclusion). Output: `self-test: 8 passed`.
- [x] T-LED-D7 — Implement markdown-table emit per R2 + bundle Q-HYG-EMIT + D-LED-6 — file: `scripts/operator_ledger_check.py:325-345` (`format_hard_table`, `format_soft_block`). Test: P-LED-1 probe asserted stderr markdown table with 2 issues. Output: `P-LED-1 PASS: both stale-failed + missing-devnote-citation detected`.
- [x] T-LED-D8 — Author embedded `--self-test` per R4.1 + D-LED-7 — file: `scripts/operator_ledger_check.py:375-500` (`run_self_test`). Test: `python3 scripts/operator_ledger_check.py --self-test`. Output: `self-test: 8 passed` (exit 0).
- [x] T-LED-D9 — Run falsification probe P-LED-1 per § Design — temp fixture at `/tmp/` (deleted); `--today 2026-05-29` → exit 1 with BOTH `stale-failed` (9d) + `missing-devnote-citation`; negative control exit 0 + soft stdout line. Output: `P-LED-1 PASS`.
- [x] T-LED-D10 — Verify R-NR contract — `bash scripts/verify_anchors.sh` → `ANCHORS PASS (84 / 84)`. Table rows byte-identical (only frontmatter + Changelog row changed in ledger).
- [x] T-LED-D11 — Apply ledger frontmatter R3.3 amendment per § Design `### R3.3` — file: `spec/dev-notes/operator-side-pending-ledger.md:2-7` (frontmatter + updated:) + Changelog row appended. Test: `python3 scripts/operator_ledger_check.py --today 2026-05-30`. Output: `EXIT:0` (clean).
- [x] T-LED-D12 — Amend AGENT.md per § Design `### R3.4` — file: `AGENT.md` (after line ~477, before `## The vibe-coding loop`). Test: `grep -n "Pending operator-verification ledger" AGENT.md` confirms insertion. Output: line found.
- [ ] T-LED-D13 — Update `spec/trace.toml` `REQ-OPERATOR-LEDGER-SCHEMA-LINT-001` row — _accept: `crates = []`; `tests = ["scripts/operator_ledger_check.py"]` (inline `--self-test`, no separate test file); `state = "dev-done"` — NOTE: orchestrator owns trace.toml per SHARED-FILE DISCIPLINE; cite for orchestrator: crates=[], tests=["scripts/operator_ledger_check.py"], state="dev-done"_
- [x] T-LED-D14 — Dev-side gates — `python3 scripts/operator_ledger_check.py --today 2026-05-30` → exit 0 zero output; `python3 scripts/operator_ledger_check.py --self-test` → `self-test: 8 passed`; `bash scripts/verify_anchors.sh` → `ANCHORS PASS (84 / 84)`; stdlib-only confirmed (no pip deps).

## M-FINAL — Tester

- [ ] T-LED-FINAL.1 — Run `python3 scripts/operator_ledger_check.py` on current main — _accept: **CORRECTED per architect D-LED-8 / finding F1** — the live ledger is 0 pending / 7 done / 0 cancelled (the Bug #64 D.1.1 row is in the Done table with a 2026-05-29 completion date, NOT a FAILED Pending row). Expectation: **exit 0 with ZERO output** (no FAILED rows to escalate; all 7 Done rows carry ISO completion dates). Any non-clean exit is a FINDING (most likely a Done-row `Completed` cell failing ISO parse) — tester surfaces it. The "1 FAILED row" path is exercised by `--self-test` (cases 3/6) + P-LED-1, NOT by the live ledger._
- [ ] T-LED-FINAL.2 — Inject synthetic 8-day-old stale-FAILED row in scratch worktree; run script; revert — _accept: exit 1; markdown table identifies the row + escalation issue class_
- [ ] T-LED-FINAL.3 — Inject schema-enum violation (`status: "blocked"`) in scratch; run; revert — _accept: exit 1; identifies schema-status-enum issue class_
- [ ] T-LED-FINAL.4 — Run self-test per R4.2 — _accept: `python3 scripts/operator_ledger_check.py --self-test` (inline flag per architect D-LED-7, no separate test file) → `self-test: 8 passed` exit 0; sub-1-s_
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
- **Architect M-T1 ground-truth findings (read § Design F1/F2/F3
  before coding)**:
  - **F1** — the live Pending table is EMPTY (0 rows); the ledger is
    0 pending / 7 done / 0 cancelled. Do NOT author a "1 FAILED row"
    expectation; clean-exit-0 is current-main behaviour. Parser MUST
    handle an empty table immediately followed by the next `##`
    heading (no blank line on line 31→32).
  - **F2** — `FAILED` / `fix-in-flight` appear only in the Changelog
    PROSE, never a live Status cell; `fix-in-flight` is NON-canonical
    (HARD violation if it ever appears in a live cell).
  - **F3** — only the Pending table has a `Status` column; Done/Cancelled
    encode status by table → status-enum + stale + Q-LED-NOTE checks
    apply ONLY to Pending rows; Done→completion-date, Cancelled→cancel-date.
- **AGENT.md collision-avoidance (K4)**: this brief's `### Pending
  operator-verification ledger` subsection nests UNDER `## Queue
  pre-flight reconciliation sweep`, appended after its body. The
  queue-staleness sibling edits the numbered-STEPS list inside that
  same `##` body — DIFFERENT line range. If both land same session,
  apply this brief's `###` block strictly after the sibling's body
  edit. adr-registry edits `.claude/agents/architect.md` (no AGENT.md
  contention).
- **Self-test is INLINE `--self-test`** (architect D-LED-7), NOT a
  `scripts/tests/` file — no test dir exists today; keeps LoC budget.
