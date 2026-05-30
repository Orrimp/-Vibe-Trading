---
slug: operator-ledger-schema-lint
status: arch-done
owner: developer
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

- [ ] T-LED-D1 — Author `scripts/operator_ledger_check.py` per § Design D-LED-1 — _accept: PEP-723 stdlib header (`requires-python = ">=3.11"`, matches `spec_brief.py`); resolves ledger path via `Path(__file__).resolve().parent.parent` (cwd-independent); `argparse` with `--today` / `--ledger` / `--self-test`; exits 0 on clean ledger; ≤ 200 LoC (+~50 self-test) per H2_
- [ ] T-LED-D2 — Implement heading-anchored 3-table parser per D-LED-2 — _accept: table discovery by `## ` heading match against `SCHEMA` (NOT by position) so an empty table immediately followed by the next heading parses clean (F1); escaped-pipe split `(?<!\\)\|`; `normalize_cell` strips bold/italic/backticks/links for enum+date match while RAW cell preserved for diff `observed`; parses all 3 tables; undersized row → HARD `schema-row-truncated` (no silent mis-parse); regression-test against verbatim Bug #64 D.1.1 line-36 Done row_
- [ ] T-LED-D3 — Implement schema check per R1.4 (a)+(b)+(c) + D-LED-3 — _accept: header-row prefix matches `SCHEMA` (extra trailing cols OK); per-row `len(cells) >= len(required)`; Pending `Status` normalizes to `CANONICAL_STATUS = {pending, FAILED, done, cancelled}` (Done/Cancelled have NO Status col per F3 — skip); `Date surfaced` parses ISO; non-canonical (incl. `fix-in-flight`) → HARD `schema-status-enum`_
- [ ] T-LED-D4 — Implement stale-FAILED per R1.4 (e) + D-LED-4 — _accept: `STALE_FAILED_DAYS = 7` named constant; today = `--today` if given else `date.today()`; age from `Date surfaced`; Pending FAILED `age >= 7` → HARD `stale-failed` (exit 1) with escalation + `escalates <date>`; `0 <= age < 7` → SOFT `failed-within-window` on STDOUT (exit unaffected, R2.5); `age < 0` → HARD `schema-future-date`_
- [ ] T-LED-D5 — Implement Q-LED-NOTE check per R1.4 (f) + D-LED-5 — _accept: Pending FAILED rows require regex `spec/dev-notes/[A-Za-z0-9._\-/]+\.md` in RAW Notes cell; missing → HARD `missing-devnote-citation` (exit 1); INDEPENDENT of staleness (1-day-old uncited FAILED still hard-fails)_
- [ ] T-LED-D6 — Implement done-row completion-date check per R1.4 (d) + D-LED-5 — _accept: Done rows `Completed` (idx 3) is non-empty ISO date → else HARD `missing-completion-date`; Cancelled rows `Cancelled` checked symmetrically as `missing-cancel-date`; Cancelled NOT required to have a *completion* col (exclusion rule)_
- [ ] T-LED-D7 — Implement markdown-table emit per R2 + bundle Q-HYG-EMIT + D-LED-6 — _accept: HARD issues → STDERR single markdown table `| issue | row | observed | expected | action |`; SOFT carry-over → STDOUT one-line-per-row block; clean → ZERO output; script failure (exit ≥ 2) → stderr `error:` + ledger path + line where available_
- [ ] T-LED-D8 — Author embedded `--self-test` per R4.1 + D-LED-7 — _accept: INLINE `--self-test` flag (NOT a separate `scripts/tests/` file — architect choice per H2 LoC budget); 8 cases (clean / schema-enum / stale@8d / not-stale@1d soft / missing-completion-date / missing-devnote-citation / cancelled-exclusion / Bug #64 D.1.1 verbatim); each uses `tempfile` fixture + explicit `--today`; `--self-test` prints `self-test: N passed` exit 0 / `self-test FAILED: <case>` exit 1; sub-1-s (R4.2)_
- [ ] T-LED-D9 — Run falsification probe P-LED-1 per § Design — _accept: temp-fixture (NOT live ledger edit) synthetic FAILED row `2026-05-20` no citation; `--today 2026-05-29` → exit 1 with BOTH `stale-failed` (9d) + `missing-devnote-citation`; negative control (citation + 1-day-old) → exit 0 + soft line; temp fixture deleted_
- [ ] T-LED-D10 — Verify R-NR contract — _accept: `git diff spec/dev-notes/operator-side-pending-ledger.md` shows ONLY the R3.3 frontmatter `validated_by:` line + the one Changelog row, zero TABLE-ROW body modifications; `bash scripts/verify_anchors.sh` all-PASS byte-identical_
- [ ] T-LED-D11 — Apply ledger frontmatter R3.3 amendment per § Design `### R3.3` — _accept: add `validated_by: scripts/operator_ledger_check.py  # operator-ledger-schema-lint v0.1.0` after the `updated:` frontmatter line + the spec'd Changelog row; bump `updated:`; all table rows byte-identical_
- [ ] T-LED-D12 — Amend AGENT.md per § Design `### R3.4` — _accept: add the drafted `### Pending operator-verification ledger (2026-05-29 contract)` subsection nested under `## Queue pre-flight reconciliation sweep`, placed AFTER its existing body (after line ~468) and BEFORE `## The vibe-coding loop`; if queue-staleness sibling already edited that `##` body, append strictly after — NO overlap with the sibling's numbered-steps edit or adr-registry's architect.md edit_
- [ ] T-LED-D13 — Update `spec/trace.toml` `REQ-OPERATOR-LEDGER-SCHEMA-LINT-001` row — _accept: `crates = []`; `tests = ["scripts/operator_ledger_check.py"]` (inline `--self-test`, no separate test file); `state = "dev-done"`_
- [ ] T-LED-D14 — Dev-side gates — _accept: `cargo fmt` clean (no Rust touch); spec-lint zero new violations; smoke `python3 scripts/operator_ledger_check.py` on current main exits **0 with ZERO output** (live ledger = 0 pending / 7 done / 0 cancelled per F1 — clean); `python3 scripts/operator_ledger_check.py --self-test` exit 0_

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
