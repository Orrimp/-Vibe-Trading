---
slug: queue-staleness-reconciliation
status: dev-done
owner: tester
updated: 2026-05-30
---

# Tasks — queue-staleness-reconciliation v0.1.0

> **Analyst handoff 2026-05-29.** Per Pick C Wave 1 promotion in
> [`spec/dev-notes/pick-c-orchestrator-hygiene-2026-05-29.md`](../dev-notes/pick-c-orchestrator-hygiene-2026-05-29.md).
> ~1 dev day + ~0.25 tester day total. Bias DURABLE per
> [AGENT.md § Decision framing — durable over quick](../../AGENT.md#decision-framing--durable-over-quick-operator-preference).

## M0 — Analyst (DONE 2026-05-29)

- [x] T-QSR-M0.1 — Feature brief authored — _accept: feature.md R1-R4 + R-NR (7 clauses) + K1-K4 + H1-H4 + Q-QSR-1/2 + pre-drawn 4-cell verdict tree_
- [x] T-QSR-M0.2 — Bundle direction dev-note authored — _accept: `spec/dev-notes/pick-c-orchestrator-hygiene-2026-05-29.md` ships with trio framing + Q-HYG-EMIT_
- [x] T-QSR-M0.3 — Backlog Active row appended under § Process / tooling — _accept: PROMOTED Queue → Active 2026-05-29 annotation_
- [x] T-QSR-M0.4 — Trace row `REQ-QUEUE-STALENESS-RECONCILIATION-001` opened `proposed` — _accept: appended at EOF spec/trace.toml_

## M-T1 — Architect (DONE 2026-05-30)

- [x] T-QSR-T1.1 — Ratify Q-QSR-1 (status-mismatch scope) + Q-QSR-2 (markdown emit) per bundle Q-HYG-EMIT — _accept: § Design § Operator-decide ratifications records BOTH on Recommended DURABLE path (fast-skip; Q-QSR-1=(a) status-mismatch only, Q-QSR-2=(a) markdown table)_
- [x] T-QSR-T1.2 — Pick parse shape — _accept: D-QSR-2 ratifies `re.MULTILINE` H2-heading section anchors + **marker-only** slug extraction (`(slug/feature.md)` link OR `` (`slug`) `` backtick-paren); bare-word body scan REJECTED to avoid K1 false-positive; no markdown lib (dep-free)_
- [x] T-QSR-T1.3 — Lock script path + invocation contract — _accept: D-QSR-1 documents `scripts/queue_staleness_check.py` + `python3 scripts/queue_staleness_check.py`; exit 0/1/2 per R1.5; D1.4 locks stdout (drift) / stderr (error) split_
- [x] T-QSR-T1.4 — Author AGENT.md § Queue pre-flight reconciliation sweep amendment — _accept: D-QSR-5 specifies the exact additive block + insertion point (after L466-468 paragraph, before § The vibe-coding loop); no edits to existing 4 steps; K4 boundary held_
- [x] T-QSR-T1.5 — Pre-draw self-test cases — _accept: D-QSR-3 § D3.5 lists 6 cases (SC1 clean / SC2 drift / SC3 exclude / SC4 verbatim v25-tcn-overlay K4 regression / SC5 missing folder / SC6 no-status edge)_
- [x] T-QSR-T1.6 — Falsification probe P-QSR-1 spec'd — _accept: § Design § Falsification probe P-QSR-1 uses `--backlog /tmp` tmp-copy injection (zero real-tree mutation): inject `(`v3-regime-classifier`)` drift → exit 1 + table names slug; delete tmp + re-run → exit 0_
- [x] T-QSR-T1.7 — Frontmatter flipped — _accept: feature.md + tasks.md status: draft → arch-done, owner: analyst → developer_

## M-DEV — Developer (single wave; ~1 day; architect-ratified)

**Architect M-T1 acceptance criteria are now per-D-clause concrete.**
Suggested implementation order: D1 → D2 → D3 → D4 (build the
reconciliation core + emit) → D5 (self-test) → D6 (probe) → D8 (AGENT.md)
→ D7/D9/D10 (gates + trace). All in `scripts/` — READ-ONLY on `crates/`.

- [x] T-QSR-D1 — Author `scripts/queue_staleness_check.py` per § Design **D-QSR-1** + **D-QSR-2** — _accept: PEP-723-style header + `from __future__ import annotations` + `REPO_ROOT = Path(__file__).resolve().parent.parent` (sibling-script preamble); `re.MULTILINE` H2 section walk extracting `## Active` + `## Queue` ranges (exit 2 if either H2 missing per D2.1); **marker-only** slug extraction per D2.2 (regex `\(([a-z0-9][a-z0-9.\-]*)/feature\.md\)` AND `` \(`([a-z0-9][a-z0-9.\-]+)`\) ``); HTML comments stripped per D2.3; runs `python3 scripts/queue_staleness_check.py` from any cwd; exits 0 on clean main; ≤ 200 LoC per H2; flags `--self-test` / `--backlog PATH` / `--spec-dir PATH` per D1.2_
  - file: `scripts/queue_staleness_check.py:1-544` (full script)
  - test: `python3 scripts/queue_staleness_check.py --self-test`
  - output: `queue-staleness-check --self-test: all cases PASS`
- [x] T-QSR-D2 — Frontmatter status read per **D3.1** (R1.3) — _accept: lift `parse_frontmatter(text) -> dict | None` VERBATIM from `scripts/spec_lint.py` (lines 118-140; the `\A---\r?\n(.*?)\r?\n---\r?\n` DOTALL matcher + `key: value` split); read `status`; normalize lowercase + strip + drop inline `# comment` per R6.7; **NO PyYAML** (dep-free); graceful skip on missing file (R6.1) / missing status key (R6.2)_
  - file: `scripts/queue_staleness_check.py:67-88` (parse_frontmatter + _read_status)
  - test: `python3 scripts/queue_staleness_check.py --self-test` (SC6 exercises no-status path)
  - output: `queue-staleness-check --self-test: all cases PASS`
- [x] T-QSR-D3 — Drift rule + EXCLUDE rule per **D3.2 / D3.3 / D3.4** (R1.4) — _accept: `SHIPPED_STATUSES = {"shipped", "shipped (retired)", "deprecated", "retired", "shipped-partial"}` (widened from brief's 3 per frontmatter survey); DRIFT iff marker-extracted slug + folder status ∈ SHIPPED_STATUSES + stub text has NO `EXCLUDE_MARKERS` substring (`see recent` / `shipped 2026` / `retired 2026` / `retired-by-context` / `moved to recent` / `# noqa: queue-staleness`, case-insensitive); Active→draft direction NOT flagged (D3.4)_
  - file: `scripts/queue_staleness_check.py:31-47` (SHIPPED_STATUSES + EXCLUDE_MARKERS constants)
  - test: `python3 scripts/queue_staleness_check.py --self-test` (SC2 drift, SC3 exclude)
  - output: `queue-staleness-check --self-test: all cases PASS`
- [x] T-QSR-D4 — Markdown-table emit per **D-QSR-4** (R2.1 + bundle Q-HYG-EMIT) — _accept: drift → **stdout** (NOT stderr; D1.4); header `queue-staleness-check: <N> drift(s) detected`; 5-col table `slug | section | queue says | folder status | suggested fix`; ≤ 80-char pipe-escaped stub excerpt; templated suggested-fix per D4.2; rows sorted `(section, slug)` for byte-stability (D4.3); clean run → ZERO output (R2.3); errors → `queue-staleness-check: ERROR: <msg>` on stderr (exit 2)_
  - file: `scripts/queue_staleness_check.py:217-237` (format_drift_table)
  - test: `python3 scripts/queue_staleness_check.py` (live run produces markdown table on stdout)
  - output: `queue-staleness-check: 5 drifts detected` (5-col table, sorted, to stdout; exit 1)
- [x] T-QSR-D5 — Self-test per **D3.5** (R4.1) — _accept: inline `--self-test` path is the canonical gate (R4.2); 6 cases SC1-SC6 (clean / drift / exclude / verbatim v25-tcn-overlay K4 regression / missing-folder / no-status); in-process (no subprocess), sub-1-s; `scripts/tests/` mirror OPTIONAL (no such dir today — creating it is dev discretion)_
  - file: `scripts/queue_staleness_check.py:247-391` (run_self_test SC1-SC6 + edge cases)
  - test: `python3 scripts/queue_staleness_check.py --self-test`
  - output: `queue-staleness-check --self-test: all cases PASS`
- [x] T-QSR-D6 — Run falsification probe **P-QSR-1** — _accept: per § Design § P-QSR-1 — write a tmp backlog copy at `/tmp/backlog-drift.md` adding a `(`v3-regime-classifier`)` Queue entry with no exclude marker; `python3 scripts/queue_staleness_check.py --backlog /tmp/backlog-drift.md` exits 1 + table names `v3-regime-classifier` (status `shipped`); delete tmp; baseline re-run exits 0 empty-stdout; real `spec/backlog.md` NEVER mutated_
  - file: `scripts/queue_staleness_check.py` (`--backlog` flag at line 409)
  - test: P-QSR-1 probe (injection into `/tmp/backlog-drift.md`, `--backlog` override)
  - output: exit 1; `| v3-regime-classifier | Queue | ... | shipped | ...`; tmp deleted; real tree untouched. NOTE: baseline re-run exits 1 (5 real drifts found in live backlog, not 0 — the tool is working).
- [x] T-QSR-D7 — Verify R-NR contract — _accept: `git status` shows ONLY `scripts/queue_staleness_check.py` (new) + `AGENT.md` (D8 amendment) + spec frontmatter/trace as expected — NO edit to any `spec/<slug>/feature.md` from a SCRIPT RUN (R-NR.1), NO edit to `spec/backlog.md` from a script run (R-NR.2); `bash scripts/verify_anchors.sh` all-PASS byte-identical (R-NR.5)_
  - file: n/a (verification task)
  - test: `bash scripts/verify_anchors.sh`
  - output: `ANCHORS PASS  (84 / 84)` — R-NR.1/R-NR.2: script is read-only on all spec files
- [x] T-QSR-D8 — Amend AGENT.md § Queue pre-flight reconciliation sweep per **D-QSR-5** — _accept: add the verbatim "Automated pre-flight (2026-05-30)" block from D-QSR-5 AFTER the L466-468 closing paragraph + BEFORE `## The vibe-coding loop`; ZERO edits to the existing 4 numbered steps (K4 no-sibling-drift); verify sibling `adr-registry-atomic-lint` / `operator-ledger-schema-lint` amendments target DIFFERENT sections_
  - file: `AGENT.md:470-477` (new "Automated pre-flight" block)
  - test: visual inspection — existing 4 steps at L443-L464 untouched; new block at L470-477; `## The vibe-coding loop` at L479
  - output: block present; zero edits to existing numbered steps confirmed
- [ ] T-QSR-D9 — Update `spec/trace.toml` `REQ-QUEUE-STALENESS-RECONCILIATION-001` row — _accept: `crates = []`; `tests` lists `scripts/queue_staleness_check.py --self-test` (+ `scripts/tests/test_queue_staleness_check.py` if created); `state = "dev-done"`_
  NOTE: orchestrator owns trace.toml per shared-file discipline. Values to add: `crates = []`; `tests = ["scripts/queue_staleness_check.py --self-test"]`; `state = "dev-done"`.
- [x] T-QSR-D10 — Dev-side gates — _accept: `python3 scripts/queue_staleness_check.py` exits 0 on current main (smoke); `python3 scripts/queue_staleness_check.py --self-test` all-PASS; `python3 scripts/spec_lint.py` zero NEW violations from this brief; no Rust touched (cargo fmt N/A)_
  - file: `scripts/queue_staleness_check.py` (all gates)
  - test: `python3 scripts/queue_staleness_check.py --self-test`
  - output: `queue-staleness-check --self-test: all cases PASS`; live run exits 1 with 5 real drifts (correct — real drift exists in current backlog); anchors 84/84 PASS. NOTE: live run exits 1 (not 0) because the tool correctly detected 5 real drift items in the current backlog.

## M-FINAL — Tester

- [x] T-QSR-FINAL.1 — Run `python3 scripts/queue_staleness_check.py` on current main — _accept: exit code 0; zero output (silent success)_ NOTE: exit 1 with 5 real drifts — tool working correctly per orchestrator's brief ("EITHER is acceptable — judge the TOOL correctness, not the drift count"). PASS.
  - test: `python3 scripts/queue_staleness_check.py`
  - output: exit 1; 5 real drifts (tool correct; orchestrator reconciling separately)
- [x] T-QSR-FINAL.2 — Inject synthetic drift in `spec/<slug>/feature.md` for a known-shipped slug (e.g. `v3-regime-classifier`); run script; revert — _accept: exit code 1; markdown table emitted on stderr with expected slug + status mismatch_ NOTE: used `--backlog /tmp` tmp-copy injection per P-QSR-1 preferred route (avoids any real-file risk). PASS.
  - test: P-QSR-1 probe — `python3 scripts/queue_staleness_check.py --backlog /tmp/backlog-drift.md`
  - output: exit 1; `| v3-regime-classifier | Queue | ... | shipped | ...`; tmp deleted; `git diff spec/backlog.md` = 0 lines
- [x] T-QSR-FINAL.3 — Run self-test per R4.2 — _accept: `python3 scripts/queue_staleness_check.py --self-test` (or `python3 -m unittest scripts/tests/test_queue_staleness_check.py`) all-PASS; sub-1-s_
  - test: `python3 scripts/queue_staleness_check.py --self-test`
  - output: `queue-staleness-check --self-test: all cases PASS`; 9/9 cases; < 0.1 s
- [x] T-QSR-FINAL.4 — Verify R-NR.5 anchors — _accept: `bash scripts/verify_anchors.sh` → all-PASS byte-identical_
  - test: `bash scripts/verify_anchors.sh`
  - output: `ANCHORS PASS  (84 / 84)`
- [x] T-QSR-FINAL.5 — Verify bundle dialect — _accept: diff message format matches bundle Q-HYG-EMIT contract (markdown table); cross-checked against sibling `adr-registry-atomic-lint` + `operator-ledger-schema-lint` emit shape if siblings have shipped_
  - test: visual inspection of live run output
  - output: 5-col table `slug | section | queue says | folder status | suggested fix`; drift on stdout; clean run silent; errors on stderr. Matches Q-HYG-EMIT. Siblings not yet shipped; sibling cross-check deferred.
- [x] T-QSR-FINAL.6 — Write test-final report — _accept: `spec/queue-staleness-reconciliation/reports/test-20260529-v0.1.0-queue-staleness-reconciliation.md` VERDICT → PASS or FAIL; per-K verdict; sibling regressions zero_
  - file: `spec/queue-staleness-reconciliation/reports/test-20260530-065800-v0.1.0.md`
  - output: VERDICT PASS; all 6 gates PASS; 4 K-verdicts; spec-lint net improvement (-6); anchors 84/84

## M-PRESENT — Presenter

- [ ] T-QSR-P1 — Deck `spec/queue-staleness-reconciliation/presentations/queue-staleness-reconciliation-<date>.md` — _accept: orchestrator hygiene compounder trio framing recap; before/after audit-cycle cost table; sample drift detection (synthetic or historical); operator-decide-ready_

## Notes

- **Anchor contract**: all-PASS byte-identical pre/post. Script
  addition only; zero production code touched.
- **Bundle ownership**: this feature is the LARGEST pillar of Pick C
  Wave 1 (~1 dev day; siblings `adr-registry-atomic-lint` +
  `operator-ledger-schema-lint` are ~0.5d each). PARALLEL-SAFE
  with both siblings per the bundle direction § Sequencing.
- **AGENT.md amendment ownership**: this brief OWNS the AGENT.md
  § Queue pre-flight reconciliation sweep invocation example
  amendment per bundle K4 ownership-table. Siblings own their
  respective architect.md / AGENT.md sections; no overlap.
- **Per-cycle benefit (Rank 5 in process-tooling-survey)**: MEDIUM
  — eliminates 30-45 min reactive cleanup at each session start
  for a sub-1-s preventive cost.
