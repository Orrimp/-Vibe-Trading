---
slug: operator-success-reports
mode: release
status: draft
audience: human-operator
updated: 2026-05-08
generated: 2026-05-08T09:32:00Z
supersedes: _none — first presenter fire on this feature_
---

# Operator success reports — release

## TL;DR

The `report` bin renders a single-page "is this working?" markdown for any time window, deterministic to the byte and reconciled to the cent — locked by two new anchors (7d + 90d).

## What changed

- New `cargo run -p reports --bin report -- --period <window> --ledger <db>` binary that writes one markdown file per run, atomic, with body-only SHA-256 stable across re-runs at fixed seed.
- Two scenarios anchored in `spec/anchors.toml` (`report-sample-7d`, `report-sample-90d`) — the 9-anchor regression gate grew to **11/11**, all PASS.
- Read-only `audit::query` API extended additively with `pnl_by_strategy(...)`; no schema migration, no new bus channels, zero LLM tokens.
- Drop-in cron entry point (`crates/agent/src/cron.rs`) and kill-switch incident-report hook (`crates/agent/src/kill_switch.rs`) now write to `spec/operator-success-reports/reports/`.

## Why

Quoting `spec/operator-success-reports/feature.md` (lines 11–43): the operator's question is always *"is this working?"* and a one-page markdown is the answer. The report is **the moat made legible** — every metric reconciles to a journal entry, so a future operator (or follow-up project lead) can demonstrate the system's institutional memory and financial-grade ledger discipline without grepping the audit DB by hand. The reconciliation invariant (R11) is load-bearing: a satoshi-level mismatch between report headline and ledger sum means the report has *failed*, regardless of whether it rendered.

## What you can do now

| Action | Command |
|---|---|
| Render a 7-day report against your ledger | `cargo run -p reports --bin report -- --period 7d --ledger data/audit/ledger.db` |
| Render a 90-day report (longer windows are 5-min downsampled) | `cargo run -p reports --bin report -- --period 90d --ledger data/audit/ledger.db` |
| Render a since-RFC3339 window (e.g. the last halt) | `cargo run -p reports --bin report -- --period since:2026-04-01T00:00:00Z --ledger data/audit/ledger.db` |
| Lock weekly cron in-process (Mondays 09:00) | enable `agent` crate's `in_process_cron` feature |

## Live demo

Two end-to-end scenarios re-rendered twice each from the test fixture, then SHA-locked against `spec/anchors.toml`:

```
$ cargo test -p reports --test report_scenarios -- --nocapture
test t816_v10_cron_friendly_3x_parallel_renders_atomic ... ok
T816 report-sample-7d body SHA-256: ab06dbcbe9a2d81be0f1ad0eecaab1d513c4bcbe5469b4eec4e9b58989482b4c
test t816_report_sample_7d_determinism_and_anchor_lock ... ok
T816 report-sample-90d body SHA-256: 2ef403f1845b8eb3b87fe381f89279c488bc54840b1d0306d95e6122bbdffd0f
test t816_report_sample_90d_determinism_and_anchor_lock ... ok
test t816_v10_cron_friendly_3x_parallel_bin_processes ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.43s
```

The two body SHAs match `report-sample-7d` and `report-sample-90d` in `spec/anchors.toml` byte-for-byte. `t816_v10_cron_friendly_3x_parallel_*` confirms three parallel renders against the same ledger snapshot land atomically — the cron-friendliness invariant (R12).

## Screenshots

_n/a — non-UI feature. The report bin emits a markdown file; the cockpit's `viewer` binary already knows how to render markdown inline ([feature.md L27–29](../feature.md))._

## Verification

| V-id | Description | Status | Evidence |
|---|---|---|---|
| V1 | fmt + clippy + audit + deny clean | VERIFIED | tester report final §3 (archived: `spec/archive/pre-lumen-tester-reports-2026-04-to-05-03.tar.gz`, file `test-2026-05-01-1828-operator-success-reports-final.md`) |
| V2 | `cargo test --workspace` 580 PASS / 0 FAIL / 3 IGNORED | VERIFIED | tester final §4 line 71 |
| V3 | Both 7d + 90d scenarios render end-to-end | VERIFIED | live demo above (`test_result: ok. 4 passed`) |
| V4 | Body-only SHA-256 byte-identical across two same-seed runs | VERIFIED | live demo SHA `ab06dbcb…` (7d) + `2ef403f1…` (90d) match anchor table |
| V5 | Reconciliation Δ = $0.00 + deliberate-mismatch exits 1 | VERIFIED | tester final §5 `t814_reconciliation_fail_*` PASS |
| V6 | 11/11 anchors PASS (9 prior + 2 new) | VERIFIED | `scripts/verify_anchors.sh` → `ANCHORS PASS (11 / 11)` (2026-05-08 09:31 UTC) |
| V7 | `audit::query` API extended additively only | VERIFIED | tester final §6; `pnl_by_strategy` is the only new query, all 13 prior queries unchanged |
| V8 | LLM tokens consumed = 0; `expense:llm:*` = 0 | VERIFIED | tester final §8 |
| V9 | 90d wall-clock < 10s, RSS < 256 MiB | VERIFIED | `tests/perf_smoke.rs` gate; tester final §9 |
| V10 | 3× parallel cron renders land atomically | VERIFIED | `t816_v10_cron_friendly_3x_parallel_*` (live demo above) |

## Numbers that matter

- Tests: **580 PASS / 0 FAIL / 3 ignored** at ship time (tester final 2026-05-01); reports crate alone: **143 tests**.
- Anchors: **11 / 11 PASS** as of 2026-05-08 09:31 UTC (re-verified during this presentation).
- New anchored scenarios: **2** (`report-sample-7d` `ab06dbcb…`, `report-sample-90d` `2ef403f1…`).
- Live demo wall-clock: **1.43s** for the 4-test scenario suite (fixtures, debug build).
- LLM cost incurred by this feature: **$0.00** (zero tokens, zero new bus channels).

## Anchor table (first 8 chars per locked body-SHA)

| Scenario | SHA-256 prefix |
|---|---|
| btc-2023-1m-sma-cross | fc2e3b4a… |
| btc-2023-1m-sma-baseline-refresh | fc2e3b4a… |
| btc-2023-1m-macd-trend | ef9c5e48… |
| btc-2023-1m-rsi-reversion | bc56d20d… |
| btc-2023-1m-bbands-mean-revert | d8a08a23… |
| top10-2023-1h-momentum | 3b60ef07… |
| top10-2024-h1-momentum | 1f33534f… |
| pairs-2023-zscore-mr | 90591a0e… |
| pairs-2024-h1-zscore-mr | 14f50a59… |
| report-sample-7d | ab06dbcb… |
| report-sample-90d | 2ef403f1… |

## Open decisions

_no decisions pending — feature shipped 2026-05-01; this deck is a presenter smoke test (queue item from `spec/backlog.md`) to dry-run the new agent pipeline against a known-good feature before a real new feature fires._

## Smoke-test findings (presenter pipeline)

Surfaced while running the `present-results` + `verify-anchors` + `capture-screenshot` skills end-to-end against this feature:

1. **Tester reports for shipped pre-Lumen features live in `spec/archive/`**, not `spec/<slug>/reports/`. The presenter skill's procedure step 3 ("Read the latest test report") needs to fall back to `tar -xzf spec/archive/pre-lumen-tester-reports-2026-04-to-05-03.tar.gz -C /tmp <name>` when no `test-*.md` exists in the per-feature reports folder. Currently the skill text in `.claude/skills/present-results/SKILL.md:26-27` doesn't mention the archive — would have produced a `HANDOFF → tester (rerun before presentation)` for any pre-Lumen feature.
2. **`capture-screenshot` skill has no "non-UI feature" branch.** Procedure step 1 only handles "file exists" vs "needs capture". For non-UI features (this one, plus future report/audit/risk-only features), the right answer is "n/a — no screenshots needed", not a manual-capture instruction. Recommend adding a 4th branch keyed on the feature's `## UI` section presence.
3. **Tester report's "Routing" section references old paths.** The archived final tester report's §14 says `spec/features/...` and `spec/tasks/...` — pre-restructure language. Confirms the decision to leave archived reports immutable was correct (they describe the layout at time of writing).
4. **Cross-references in `spec/backlog.md` Recent section** still use relative paths like `reports/test-2026-05-07-...md` instead of `phase-5-humancontrol-agentfeed/reports/...md` (lines 85–91). Bulk-rewrite missed them because they were inside markdown link parens. Cosmetic; not blocking.

## Approval

- [x] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback

_empty until operator fills_

## Changelog

- 2026-05-08 (presenter, smoke-test fire from `spec/backlog.md` Process / tooling queue): initial draft for an already-shipped feature. Pulled evidence from feature brief, archived tester report, live `cargo test -p reports --test report_scenarios` re-run, and a fresh `scripts/verify_anchors.sh` PASS. Surfaces 4 smoke-test findings about the presenter pipeline (see § "Smoke-test findings").
- 2026-05-08 (operator, verbal approval via orchestrator chat): ticked `[x] Approved — ship`. Verbal approval recorded in the chat transcript at the orchestrator session for 2026-05-08. The two associated skill-plumbing fixes (archive-fallback in `present-results` step 3; non-UI-feature branch in `capture-screenshot` step 1 + `present-results` step 6) shipped in commit 8b139c2 alongside this deck.
