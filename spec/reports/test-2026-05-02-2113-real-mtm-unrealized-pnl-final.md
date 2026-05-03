---
title: Test Report
feature: real-mtm-unrealized-pnl
run_id: 2026-05-02-2113-UTC
commit: uncommitted
agent: tester
verdict: FAIL
---

# Test Report — real-mtm-unrealized-pnl — 2026-05-02 21:13 UTC

## 1. Scope

- **Feature / change under test:** Real mark-to-market unrealized P&L —
  T1001 `OpenPosition` struct, T1002 `audit::query::open_positions_at`
  reader, T1003 orchestrator integration in `crates/reports/src/lib.rs`,
  T1004 fixture `build_ledger_with_open_positions_7d.rs`, T1005 V1+V4+V7
  reader tests, T1006 V2 + V6 orchestrator tests, T1007 V8 perf smoke,
  T1008 anchor regression sweep.
- **Spec refs:** `spec/features/real-mtm-unrealized-pnl.md`,
  `spec/tasks/real-mtm-unrealized-pnl.md`.
- **Commit SHA:** uncommitted (working tree).
- **Rust toolchain:** rustc 1.94.1 (e408947bf 2026-03-25).
- **OS / arch:** Darwin 25.4.0 arm64.

## 2. Static Analysis

| Check               | Result | Notes                                            |
|---------------------|--------|--------------------------------------------------|
| `cargo fmt --check` | PASS   | clean — no diff.                                 |
| `cargo build --workspace --all-targets` | PASS | 0 warnings, 11.54s cold build. |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS | clean. |
| `cargo audit`       | n/a    | not part of this gate; not run.                  |
| `cargo deny`        | n/a    | not part of this gate; not run.                  |

## 3. Unit & Integration Tests

| Crate           | Passed | Failed | Ignored | Duration |
|-----------------|-------:|-------:|--------:|---------:|
| `agent` (lib + integration) | 51 | 0 | 0 | ~3s |
| `audit` (integration) | 70 | 0 | 0 | ~0.4s |
| `backtest` (incl. anchor determinism) | 28 | 0 | 0 | ~50s |
| `exec`          | 9      | 0      | 0       | ~0s      |
| `reports` (lib) | 98     | 0      | 0       | 0.06s    |
| `reports` (integration, sequential) | 53 | 1 (FLAKY) | 0 | ~18s |
| `strategy`      | 107    | 0      | 0       | ~0s      |
| `trading_core` (lib + integration + trybuild) | 66 | 0 | 0 | ~2s |
| `ui`            | 59     | 0      | 0       | ~0.3s    |
| **Total**       | ~541   | 1 FLAKY | 0      | ~75s     |

Doc-tests: `cargo test --workspace --doc` → 0 passed, 0 failed, 1 ignored
(agent::bus doc snippet) — clean.

### Failing Tests

**`crates/reports/tests/mark_unavailable_warns.rs::t1006_v6_mark_miss_warns_and_zeroes`** —
INTERMITTENT failure. The test uses
`tracing::subscriber::with_default(subscriber, || rt.block_on(...))` to
capture WARN events emitted by the orchestrator when a `MarkSource`
omits an open-position symbol. When run in parallel with
`t1006_v6_footnote_present_when_miss` (same binary, default cargo-test
parallelism), the capture sometimes observes 0 WARN events instead of
the expected 1 — failing the `assert_eq!(captured.len(), 1, ...)` at
`crates/reports/tests/mark_unavailable_warns.rs:231`.

```
thread 't1006_v6_mark_miss_warns_and_zeroes' (19115439) panicked at
  crates/reports/tests/mark_unavailable_warns.rs:231:5:
assertion `left == right` failed: T1006 V6 contract: expected exactly
ONE `mark unavailable for open position` warn (one per missed
open-position mark; the fixture has 1 ETHUSDT open position and the
marks CSV omits ETHUSDT); got 0 events: []
  left: 0
 right: 1
```

Reproducibility: 4 runs of `cargo test -p reports --test
mark_unavailable_warns` on this box → 2 PASS, 2 FAIL. Isolation
(`-- --test-threads=1` or `-- t1006_v6_mark_miss_warns_and_zeroes`) →
always PASS.

Root cause analysis: `tracing::subscriber::with_default` installs a
**thread-local** dispatcher for the duration of its closure. When
`t1006_v6_footnote_present_when_miss` (a `#[tokio::test]` with no
subscriber) and `t1006_v6_mark_miss_warns_and_zeroes` (custom `Layer`
+ `WarnVisitor` capture) run on different threads in the same process,
the orchestrator's `tracing::warn!` call site lazy-initialises its
dispatcher cache on whichever thread first hit the call site. If that
first thread had no `with_default` scope, subsequent calls on the
capture thread bypass the thread-local and route to the global
NoSubscriber → 0 events captured. The footnote is still rendered
because `mark_misses > 0` is computed from a `u32` counter independent
of `tracing` configuration (verified — `t1006_v6_footnote_present_when_miss`
always passes).

**Production behaviour is NOT affected.** The orchestrator emits the
warn exactly as specified (`crates/reports/src/lib.rs:160-164`), the
counter increments, and the footnote renders. The flake is in T1006's
test capture infrastructure only.

## 4. Property / Fuzz Tests

_n/a — no property tests in this feature's surface; the existing
`proptest`s in `trading_core::tests::order_tests::prop_*` continue to
pass (3/3)._

## 5. Backtest Results

_n/a — this feature is plumbing on the audit-query reader + the
`reports::generate(...)` orchestrator. No strategy / exec / backtest
code path is touched. The 9 v0/v0.5/v1/v1.5a backtest anchors verify
unchanged via `verify_anchors.sh` (see § 6 anchor gate)._

## 6. Anchor Gate

`bash scripts/verify_anchors.sh` →

```
PASS  btc-2023-1m-sma-cross                 fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
PASS  btc-2023-1m-sma-baseline-refresh      fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
PASS  btc-2023-1m-macd-trend                ef9c5e483fa079f670a7aa15671643fce3b39a5ce35df8cb6d797887053f8805
PASS  btc-2023-1m-rsi-reversion             bc56d20d608c680e534bf6764ce8e0e568f0d4ffdf847a539c53fef65170d7aa
PASS  btc-2023-1m-bbands-mean-revert        d8a08a23d3629556c5fca39d6af89d7e0f99418e642af0b86fce22ff4d2792e3
PASS  top10-2023-1h-momentum                3b60ef0743f006867b9e52f9de154869ee170987b27560e288b2d9597d3ecf97
PASS  top10-2024-h1-momentum                1f33534fc7c6af1c04330564bec77aac620ecf6f1058f11ff90dfb66adcf05c6
PASS  pairs-2023-zscore-mr                  90591a0ecc5d56c8ff93834b127a3780a31f51634f38f12c3c412391116abbd0
PASS  pairs-2024-h1-zscore-mr               14f50a598ba8343fc9be198a78716d036407d585c641c0b054eae6c062f1507f
PASS  report-sample-7d                      ab06dbcbe9a2d81be0f1ad0eecaab1d513c4bcbe5469b4eec4e9b58989482b4c
PASS  report-sample-90d                     2ef403f1845b8eb3b87fe381f89279c488bc54840b1d0306d95e6122bbdffd0f
---
ANCHORS PASS  (11 / 11)
```

11/11 — Q4's load-bearing claim ("byte-identical bodies on
empty-positions anchored fixtures") confirmed empirically.

`cargo test -p reports --test report_scenarios --release` → 4/4 PASS
(both v1+ anchored scenarios re-rendered to byte-identical bodies
matching `ab06dbcb…` / `2ef403f1…`).

## 7. Tick verification (T1001 – T1008)

| Tick | Citation file:line | Cmd | Output match | Status |
|------|--------------------|-----|--------------|--------|
| **T1001** | `crates/core/src/position.rs:88` (`pub struct OpenPosition`); `crates/core/src/lib.rs:39` (re-export) | `cargo test -p trading_core --lib position::tests` | 3 passed (`t1001_open_position_partialeq_round_trip` + `_distinguishes_strategy_id` + `_distinguishes_qty_and_cost_basis`) | **VERIFIED** |
| **T1002** | `crates/audit/src/query.rs:1008` (`pub async fn open_positions_at`); `crates/audit/tests/open_positions.rs` (8 tests) | `cargo test -p audit --test open_positions` | 8 passed (`t1002_*`) | **VERIFIED** |
| **T1003** | `crates/reports/src/lib.rs:148` (open-positions loop); `crates/reports/src/render/reconciliation.rs:21` (footnote const); `:33` (`pub fn render(report, mark_unavailable: bool)`); `crates/reports/tests/t1003_orchestrator_smoke.rs` (3 tests) | `cargo test -p reports --test t1003_orchestrator_smoke` | 3 passed | **VERIFIED**. Anchor gate quote in dev tick reproduced — `ANCHORS PASS (11/11)`. Collateral fix to `crates/reports/tests/fixtures/build_ledger_1y.rs` long-only invariant verified — `t815_perf_smoke_90d_under_10s_and_under_256mib` green (`finished in 4.58s` — well under 10s budget). |
| **T1004** | `crates/reports/tests/fixtures/build_ledger_with_open_positions_7d.rs:88` (`pub async fn build_ledger_with_open_positions_7d`); `:239` (`pub fn frozen_marks_csv`); 3 smoke tests at `crates/reports/tests/fixture_with_open_positions_smoke.rs` | `cargo test -p reports --test fixture_with_open_positions_smoke` | 3 passed | **VERIFIED** |
| **T1005** | `crates/audit/tests/open_positions_at.rs:83`, `:175`, `:212`, `:241` (4 tests) | `cargo test -p audit --test open_positions_at` | 4 passed (`t1005_v1_*`, `t1005_v4_*`, `t1005_v7_*`, `t1005_q8_*`) | **VERIFIED** |
| **T1006** | `crates/reports/tests/unrealized_orchestrator.rs:92` (V2); `crates/reports/tests/mark_unavailable_warns.rs:154` (V6 capture) and `:271` (V6 footnote) | `cargo test -p reports --test unrealized_orchestrator --test mark_unavailable_warns` | V2 passes (1/1); V6 footnote passes (1/1); V6 capture FLAKY (1 PASS / 1 FAIL across 4 runs) | **PARTIAL — V2 + V6 footnote VERIFIED; V6 warn-capture UN-VERIFIED (flake under default parallel test execution)** |
| **T1007** | `crates/reports/tests/perf_smoke_open_positions.rs:189` | `cargo test -p reports --test perf_smoke_open_positions --release -- --nocapture` | 1 passed; output: `T1007 V8 wall-clock: 0.216ms (budget < 100ms) — PASS` | **VERIFIED** (this run measured 0.216ms; dev tick cited 0.287ms — both well under 100ms budget). |
| **T1008** | verification-only; no source touched | `bash scripts/verify_anchors.sh` + `cargo test -p reports --test report_scenarios --release` | `ANCHORS PASS (11 / 11)`; `report_scenarios` 4/4 PASS | **VERIFIED** |

## 8. V1–V8 Verification Matrix

| V-id | Description | Status | Evidence |
|------|-------------|--------|----------|
| **V1** | open-positions reader correctness | **VERIFIED** | `cargo test -p audit --test open_positions_at -- t1005_v1_reader_emits_two_open_positions` → ok at `crates/audit/tests/open_positions_at.rs:83`. |
| **V2** | orchestrator computes unrealized = +200 USDT | **VERIFIED** | `cargo test -p reports --test unrealized_orchestrator` → `t1006_v2_unrealized_equals_200_usdt` ok at `crates/reports/tests/unrealized_orchestrator.rs:92`. Asserts R11.1 Ledger-side cell = +200 (architect-aligned scope-out from CSV column per Design § R3). |
| **V3** | empty-positions backwards compat | **VERIFIED** | `cargo test -p reports --test report_scenarios --release` → 4/4 PASS; both anchored fixtures re-render to byte-identical bodies. |
| **V4** | reconciliation invariant Σ debits == Σ credits | **VERIFIED** | `cargo test -p audit --test open_positions_at -- t1005_v4_balance_invariant_per_txn` → ok at `crates/audit/tests/open_positions_at.rs:175`. Iterates every `journal_transactions.id` in the T1004 fixture and asserts `verify_balance == Ok(())`. |
| **V5** | anchor regression 11/11 PASS | **VERIFIED** | `bash scripts/verify_anchors.sh` → `ANCHORS PASS (11 / 11)`. |
| **V6** | mark-miss → ZERO + footnote present + warn log | **PARTIAL — un-verified on warn capture** | (a) Footnote rendering: `t1006_v6_footnote_present_when_miss` consistently passes at `crates/reports/tests/mark_unavailable_warns.rs:271`. (b) `generate(...)` returns `Ok(_)` (Q6 contract): consistently asserted in both V6 tests and in `t1003_orchestrator_handles_mark_miss`. (c) Warn-event capture via custom `Layer + WarnVisitor`: FLAKY — 0 events observed when run in parallel with the footnote test in the same binary; 1 event observed when run sequentially. The orchestrator's `tracing::warn!` site IS executed (proven by the `mark_misses > 0` counter triggering the footnote); the test's capture infrastructure cannot reliably observe it under cargo's default parallel execution. |
| **V7** | determinism: two reads byte-identical | **VERIFIED** | `cargo test -p audit --test open_positions_at -- t1005_v7_two_reads_byte_identical` → ok at `crates/audit/tests/open_positions_at.rs:212`. |
| **V8** | perf budget < 100ms | **VERIFIED** | `cargo test -p reports --test perf_smoke_open_positions --release -- --nocapture` → `0.216ms (budget < 100ms) — PASS` at `crates/reports/tests/perf_smoke_open_positions.rs:189`. |

## 9. Operator-success-reports invariants (T802 / T805 / T806 / T809 / T810)

| Inv | Description | Evidence |
|-----|-------------|----------|
| **T802** | `post_fill(strategy_id)` signature preserved | `grep "pub async fn post_fill"` → single match `crates/audit/src/journal.rs:35` with sig `(ledger: &Ledger, fill: &Fill, strategy_id: Option<&str>) -> Result<(), LedgerError>`. **PASS** |
| **T805** | `feed_reconnect` writer | `cargo test -p audit --test feed_reconnect_test` → 2/2 PASS. |
| **T806** | `agent_uptime` open / heartbeat / close | `cargo test -p audit --test uptime_intervals_test` → 6/6 PASS. |
| **T809** | `KillSwitch::trip` dual-write | `cargo test -p audit --test kill_switch_dual_write_test` → 4/4 PASS; `cargo test -p agent --test kill_switch_trip_writes_both` → 3/3 PASS. |
| **T810** | `--features in_process_cron` build | `cargo build -p agent --features in_process_cron` → clean (8.82s). |

## 10. Live-cockpit-unified invariants (T901 – T912 + cockpit_live)

- `cargo test -p ui` → 59/59 PASS (state machine, panel snapshots, kill flow, latency thresholds, num formatting, consistency lints).
- `cargo test -p agent --test kill_switch_trip_writes_both` → 3/3 PASS (cockpit_live trip stitch).
- `cargo test -p agent --test prometheus_toggle_test` → 3/3 PASS (T912 toggle).
- `cargo test -p agent --test unified_uptime_test` → 1/1 PASS (T910 graceful-shutdown).
- All other agent tests (33 lib + 11 integration) pass — cockpit-live event flow intact.

## 11. Smoke run

`cargo run -p reports --bin report -- --period 7d --ledger /tmp/audit.db --output /tmp/wave2-real-mtm.md`
→ `wrote /tmp/wave2-real-mtm.md (run_id=ccddc74afcca4f86)`. Exit 0.

## 12. Environment / Infrastructure Issues

The V6 warn-capture flake (§3, §8) is a tracing-dispatcher-cache race
that surfaces only under cargo's default parallel test execution
within the `mark_unavailable_warns` binary. It is reproducible across
multiple runs on this box. Production behaviour (the orchestrator's
warn emission, mark_misses counter, footnote rendering, Ok return)
is NOT affected.

## 13. Verdict

**`FAIL`**

The feature's production behaviour is correct on every dimension this
gate exercises: V1, V2 (R11.1 cell = +200), V3 (anchors 11/11), V4,
V5, V7, V8 (0.216ms), all 5 operator-success-reports invariants,
all 12 live-cockpit-unified invariants, smoke run exits 0. The
orchestrator's mark-miss handling per Q6 (warn + zero + footnote +
no propagation) is functionally correct — the footnote IS rendered,
the run returns Ok, and `t1003_orchestrator_handles_mark_miss` +
`t1006_v6_footnote_present_when_miss` consistently pass.

However, the dev's T1006 ticked citation includes the test
`t1006_v6_mark_miss_warns_and_zeroes` which is FLAKY under default
cargo test execution (intermittent 0-event capture vs the asserted
1-event capture). Per AGENT.md process discipline (honest tick rule)
and `.claude/agents/tester.md` (overclaim → re-verify and un-tick),
a flaky test cannot anchor a `T_FINAL_*` PASS. The flake is a
test-side `tracing::Dispatch` cache race when the same test binary
runs the no-subscriber `t1006_v6_footnote_present_when_miss`
concurrently with the `with_default(...)`-scoped capture test.

Suggested remediations (developer call):
1. Add `#[serial_test::serial]` (already a workspace dev-dep
   candidate) to both T1006 V6 tests, or
2. Combine the two V6 tests into one `#[test]` body so the
   `with_default` scope wraps both the warn assertion and the
   footnote assertion, or
3. Replace `tracing::subscriber::with_default(...)` with
   `tracing::dispatcher::set_default(...)` returning the guard +
   ensuring a fresh dispatch cache on the test thread, or
4. Split the V6 capture test into its own integration-test
   binary file so cargo's per-binary process isolation guarantees
   a clean dispatcher-cache state.

T_FINAL_REAL_MTM **NOT TICKED**. The feature stays
`status: in-progress` until the V6 warn-capture test is stable.

## 14. Routing

`HANDOFF → developer (re-verify and un-tick T1006; stabilize the
V6 warn-capture test against parallel execution)` — quoted failed
citation:

> **T1006 honest-tick (dev, 2026-05-01):** "test cmd —
> `cargo test -p reports --test unrealized_orchestrator --test
> mark_unavailable_warns`. test output — running 2 tests / test
> t1006_v6_mark_miss_warns_and_zeroes ... ok / test
> t1006_v6_footnote_present_when_miss ... ok"

Re-running the same command produces an intermittent FAIL on
`t1006_v6_mark_miss_warns_and_zeroes` (4-run sample: 2 PASS / 2
FAIL). The dev's "ok" line is therefore not reproducible.

Once the test is stable, the tester will re-run all gates, tick
T_FINAL_REAL_MTM, and bump the feature + task frontmatter to
`status: shipped`.
