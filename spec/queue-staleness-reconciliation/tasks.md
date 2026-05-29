---
slug: queue-staleness-reconciliation
status: draft
owner: analyst
updated: 2026-05-29
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

## M-T1 — Architect (PENDING)

- [ ] T-QSR-T1.1 — Ratify Q-QSR-1 (status-mismatch scope) + Q-QSR-2 (markdown emit) per bundle Q-HYG-EMIT — _accept: § Design § Operator-decide ratifications records both on Recommended DURABLE path; fast-skip path expected_
- [ ] T-QSR-T1.2 — Pick parse shape — _accept: regex-based section parse vs lightweight markdown lib; ratify in D-QSR-1; analyst recommends `re.MULTILINE` with section anchors for zero-dep stdlib-only posture_
- [ ] T-QSR-T1.3 — Lock script path + invocation contract — _accept: D-QSR-2 documents `scripts/queue_staleness_check.py` with `python3 scripts/queue_staleness_check.py` invocation; exit codes 0/1/≥2 per R1.5_
- [ ] T-QSR-T1.4 — Author AGENT.md § Queue pre-flight reconciliation sweep amendment — _accept: 1-line invocation example added under existing § contract (codified 2026-05-29); brief OWNS this amendment per bundle K4 ownership-table; no AGENT.md sibling-section drift_
- [ ] T-QSR-T1.5 — Pre-draw self-test cases — _accept: D-QSR-3 lists ≥ 3 self-test cases (clean / drift / exclude-rule) including the verbatim 2026-05-21 v25-tcn-overlay historical drift case per K4 mitigation_
- [ ] T-QSR-T1.6 — Falsification probe P-QSR-1 spec'd — _accept: § Design § Falsification probe P-QSR-1 includes (a) inject a synthetic drift into mock backlog, assert script exits 1 with the diff message; (b) revert + assert exit 0_
- [ ] T-QSR-T1.7 — Frontmatter flipped — _accept: feature.md + tasks.md status: draft → arch-done, owner: analyst → developer_

## M-DEV — Developer (single wave; ~1 day; architect-ratified)

**Architect M-T1 will fill the per-D-clause acceptance criteria;
developer follows.** Pre-positioned task slots:

- [ ] T-QSR-D1 — Author `scripts/queue_staleness_check.py` per § Design D-QSR-1 (section parse) + D-QSR-2 (invocation contract) — _accept: script runs `python3 scripts/queue_staleness_check.py` from repo root; exits 0 on clean; ≤ 200 LoC per H2_
- [ ] T-QSR-D2 — Implement frontmatter status read per R1.3 — _accept: each Queue slug's `spec/<slug>/feature.md` frontmatter `status:` field read via PyYAML-stdlib (`yaml.safe_load`) OR plain regex on `^status:` line per architect M-T1 pick; falls back gracefully on missing files_
- [ ] T-QSR-D3 — Implement drift rule per R1.4 — _accept: exclude-rule for already-annotated post-ship Queue text confirmed; only flag `shipped | shipped (retired) | deprecated` frontmatter against active Queue stubs_
- [ ] T-QSR-D4 — Implement markdown-table emit per R2.1 + bundle Q-HYG-EMIT — _accept: stderr emit on drift; markdown table with slug + queue excerpt + folder status + suggested fix columns_
- [ ] T-QSR-D5 — Author self-test per R4.1 — _accept: `scripts/tests/test_queue_staleness_check.py` OR inline `--self-test` flag per architect M-T1; ≥ 3 cases pass; sub-1-s wall-clock_
- [ ] T-QSR-D6 — Run falsification probe P-QSR-1 — _accept: synthetic drift inject → exit 1 + diff message confirmed; revert → exit 0_
- [ ] T-QSR-D7 — Verify R-NR contract — _accept: `git diff spec/<slug>/feature.md` shows zero edits to any feature.md (READ-ONLY); `git diff spec/backlog.md` zero edits from script run; anchors all-PASS_
- [ ] T-QSR-D8 — Amend AGENT.md § Queue pre-flight reconciliation sweep with invocation example per T-QSR-T1.4 — _accept: 1-line addition under existing § contract; verified by manual diff that bundle-sibling K4-ownership amendments do not conflict_
- [ ] T-QSR-D9 — Update `spec/trace.toml` `REQ-QUEUE-STALENESS-RECONCILIATION-001` row — _accept: `crates = []` (no Rust crates touched); `tests` lists self-test path; `state = "dev-done"`_
- [ ] T-QSR-D10 — Dev-side gates — _accept: cargo fmt clean (no Rust); spec-lint zero new violations from this brief; manual smoke `python3 scripts/queue_staleness_check.py` exits 0 on current main_

## M-FINAL — Tester

- [ ] T-QSR-FINAL.1 — Run `python3 scripts/queue_staleness_check.py` on current main — _accept: exit code 0; zero output (silent success)_
- [ ] T-QSR-FINAL.2 — Inject synthetic drift in `spec/<slug>/feature.md` for a known-shipped slug (e.g. `v3-regime-classifier`); run script; revert — _accept: exit code 1; markdown table emitted on stderr with expected slug + status mismatch_
- [ ] T-QSR-FINAL.3 — Run self-test per R4.2 — _accept: `python3 scripts/queue_staleness_check.py --self-test` (or `python3 -m unittest scripts/tests/test_queue_staleness_check.py`) all-PASS; sub-1-s_
- [ ] T-QSR-FINAL.4 — Verify R-NR.5 anchors — _accept: `bash scripts/verify_anchors.sh` → all-PASS byte-identical_
- [ ] T-QSR-FINAL.5 — Verify bundle dialect — _accept: diff message format matches bundle Q-HYG-EMIT contract (markdown table); cross-checked against sibling `adr-registry-atomic-lint` + `operator-ledger-schema-lint` emit shape if siblings have shipped_
- [ ] T-QSR-FINAL.6 — Write test-final report — _accept: `spec/queue-staleness-reconciliation/reports/test-20260529-v0.1.0-queue-staleness-reconciliation.md` VERDICT → PASS or FAIL; per-K verdict; sibling regressions zero_

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
