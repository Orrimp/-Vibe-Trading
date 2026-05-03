---
title: Test Report — per-symbol-position-accounts (final)
slug: per-symbol-position-accounts
report_type: test-final
feature: per-symbol-position-accounts
run_id: 2026-05-03-0803-UTC
commit: uncommitted (working tree)
agent: tester
verdict: PASS
---

# Test Report — per-symbol-position-accounts — 2026-05-03 08:03 UTC

## 1. Scope

- **Feature / change under test:** per-symbol position accounts. Migration
  `006_per_symbol_position_accounts.sql` seeds 10 `assets:position:<SYMBOL>`
  rows for the universe at `config/agent.toml [funding].universe`;
  `audit::journal::post_fill` writes per-pair instead of the legacy
  hardcoded `"assets:position:BTC"`; `audit::query::open_positions_at`
  gains a Q4 defensive cross-check (warn-only); `bootstrap::seed_universe_accounts`
  marked `#[deprecated]`; T1004 fixture extended with mixed legacy/new
  rows; new V1+V2+V5+V8 tests in `crates/audit/tests/per_symbol_post_fill.rs`;
  new V3+V7 tests in `crates/reports/tests/open_positions_mixed_ledger.rs`.
- **Spec refs:**
  - `spec/features/per-symbol-position-accounts.md`
  - `spec/tasks/per-symbol-position-accounts.md`
  - `spec/anchors.toml` (11 entries)
- **Commit SHA:** uncommitted (no `.git` in cwd; working tree)
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`
- **OS / arch:** Darwin 25.4.0 (arm64)

## 2. Static Analysis

| Check                                              | Result | Notes                       |
|----------------------------------------------------|--------|-----------------------------|
| `cargo fmt --all -- --check`                       | PASS   | clean (zero diff)           |
| `cargo build --workspace --all-targets`            | PASS   | 23.92s, 0 warnings          |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS | clean (1.01s; cached) |
| `cargo build -p agent --features in_process_cron`  | PASS   | clean (11.51s) — Inv-T810   |
| `cargo audit`                                      | n/a    | not in this gate's scope    |
| `cargo deny`                                       | n/a    | not in this gate's scope    |

## 3. Unit & Integration Tests

| Suite                                       | Passed | Failed | Ignored | Notes |
|---------------------------------------------|-------:|-------:|--------:|-------|
| `cargo test --workspace --all-targets`      | ~641 across 85 binaries | 0 | 0 | every per-binary run reported `test result: ok.` |
| `cargo test --workspace --doc`              |    0   |    0   |    0    | no doc tests; all `0 passed; 0 failed` |
| `cargo test -p audit --test per_symbol_post_fill` (T1105) | 4 | 0 | 0 | V1, V2, V5, V8 |
| `cargo test -p audit --test t1102_per_symbol_post_fill` (T1102) | 2 | 0 | 0 | per-pair writer + legacy-row reader |
| `cargo test -p audit --test migration_006_smoke` (T1101) | 2 | 0 | 0 | seeds + idempotent |
| `cargo test -p audit --test ledger_integration` | 8 | 0 | 0 | account-list + balance invariants |
| `cargo test -p audit --test feed_reconnect_test` (Inv-T805) | 2 | 0 | 0 | feed-reconnect writer |
| `cargo test -p audit --test uptime_intervals_test` (Inv-T806) | 6 | 0 | 0 | agent uptime |
| `cargo test -p audit --test kill_switch_dual_write_test` (Inv-T809) | 4 | 0 | 0 | trip dual-write |
| `cargo test -p reports --test open_positions_mixed_ledger` (T1106) | 2 | 0 | 0 | V3, V7 |
| `cargo test -p reports --test fixture_with_open_positions_smoke` (T1104) | 3 | 0 | 0 | original T1004 smoke still green |
| `cargo test -p reports --test report_scenarios --release` | 4 | 0 | 0 | T816 anchor-locked render |
| `cargo test -p ui --test prometheus_toggle_test` (T912) | 3 | 0 | 0 | Prometheus toggle |
| `cargo test -p ui --test bus_drops_on_shutdown` (T903d) | 1 | 0 | 0 | bus strong-count collapses |
| `cargo test -p ui --test kill_switch_trip_writes_both` (T911 / T809 stitch) | 3 | 0 | 0 | kill-button → audit dual-write |
| `cargo test -p ui --test unified_uptime_test` (T910) | 1 | 0 | 0 | graceful shutdown |
| `cargo test -p ui --test metrics_endpoint` (T27) | 1 | 0 | 0 | Prometheus names |

### Failing Tests

_none_

## 4. Property / Fuzz Tests

_n/a — this feature is plumbing-only; no proptest / cargo-fuzz suites apply._

## 5. Backtest Results

_n/a — chart-of-accounts plumbing; no new strategy or render path.
The 9 backtest body anchors remain byte-identical; covered under
section 6 (Anchor gate)._

## 6. Anchor Gate (V4)

`bash scripts/verify_anchors.sh` verbatim stdout:

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

`spec/anchors.toml` unchanged at 11 entries (9 backtest + 2 v1+).

## 7. Tick verification (T1101 – T1107)

| Task | Status | Citation verified |
|---|---|---|
| **T1101** — migration `006_per_symbol_position_accounts.sql` + smoke | VERIFIED | `crates/audit/migrations/006_per_symbol_position_accounts.sql:15-24` (10 `INSERT OR IGNORE` lines for the universe) + `crates/audit/tests/migration_006_smoke.rs` (`t1101_migration_006_seeds_per_symbol_accounts`, `t1101_migration_006_is_idempotent` — 2/2 PASS) |
| **T1102** — `post_fill` writer per-pair + Q4 cross-check | VERIFIED | Writer: `crates/audit/src/journal.rs:63` hoists `position_account_id`, used at lines 94 (Buy debit) + 147 (Sell credit). Signature unchanged: `pub async fn post_fill(ledger, fill, strategy_id) -> Result<()>` at line 39. Reader: `crates/audit/src/query.rs:1043-1053` LEFT JOINs position-side `account_id`; lines 1084-1100 emit `tracing::warn!` when `account_id` matches neither the legacy `assets:position:BTC` whitelist nor `format!("assets:position:{symbol}")`. Description-parse stays primary at line 1082. Tests: `t1102_per_symbol_post_fill.rs` 2/2 PASS. |
| **T1103** — `seed_universe_accounts` `#[deprecated]` | VERIFIED | `crates/audit/src/bootstrap.rs:64-71` carries `#[deprecated(since = "1.6.0", note = "shape mismatch …")]`. `grep -rn "seed_universe_accounts" --include='*.rs' .` → only the function definition itself at `bootstrap.rs:73` (zero callers). Workspace clippy clean (the deprecation warning stays silent). |
| **T1104** — `build_ledger_with_open_positions_7d` mixed extension | VERIFIED | `crates/reports/tests/fixtures/build_ledger_with_open_positions_7d.rs:384-433` adds `build_ledger_mixed_legacy_and_per_symbol_7d`. Pre-006 BTCUSDT + ETHUSDT legacy buys via `insert_legacy_buy(...)` (lines 447-502) write directly to `assets:position:BTC` with descriptions `"buy 1.0 BTCUSDT @ 60000"` / `"buy 5.0 ETHUSDT @ 2500"` (lowercase fixed — matches `Side::Display` and `open_positions_at`'s `description LIKE 'buy %'` filter at `query.rs:1050`). Post-006 SOLUSDT buy at `qty=10 @ price=100, strategy_id=Some("test_strategy")` via `journal::post_fill`. Existing T1004 smoke still 3/3 PASS. |
| **T1105** — V1 + V2 + V5 + V8 tests | VERIFIED | `crates/audit/tests/per_symbol_post_fill.rs` `t1105_v1_post_fill_writes_per_symbol_account` (line 144), `t1105_v2_legacy_row_readable_after_migration` (line 223), `t1105_v5_balance_invariant_pre_and_post_migration` (line 352), `t1105_v8_universe_coverage` (line ~430). `cargo test -p audit --test per_symbol_post_fill` → 4/4 PASS. |
| **T1106** — V3 + V7 tests | VERIFIED | `crates/reports/tests/open_positions_mixed_ledger.rs` `t1106_v3_mixed_ledger_correct_open_positions` + `t1106_v7_two_reads_byte_identical`. Mounts T1104 fixture via `#[path]`. `cargo test -p reports --test open_positions_mixed_ledger` → 2/2 PASS. |
| **T1107** — anchor sweep + workspace test sweep | VERIFIED | `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)` (re-verified twice in this run, pre- and post-tests; identical output both times). `cargo test --workspace --all-targets` → 0 failures across 85 binaries. |

All citations resolve to the live source/test code.

## 8. Verification matrix V1–V8

| V | Description | Status | Evidence (file:line / cmd / output) |
|---|---|---|---|
| V1 | `post_fill` writes per-symbol account-id | VERIFIED | `crates/audit/tests/per_symbol_post_fill.rs::t1105_v1_post_fill_writes_per_symbol_account` (line 144). `cargo test -p audit --test per_symbol_post_fill -- t1105_v1_post_fill_writes_per_symbol_account` → `test result: ok. 1 passed` (rolled into `4 passed; 0 failed` in the per-binary line). Asserts the position-side rows group by exactly 3 per-pair account-ids and zero rows on legacy `assets:position:BTC`. |
| V2 | Pre-migration legacy rows still readable | VERIFIED | `t1105_v2_legacy_row_readable_after_migration` (line 223). Hand-crafts an ETHUSDT Sell whose entries reference `assets:position:BTC`; asserts (a) account-id preserved verbatim post-006, (b) `verify_balance(...) == Ok(())`, (c) `pnl_by_symbol(...)` buckets `(ETHUSDT, +100 USDT)` via description-parse. |
| V3 | `open_positions_at` correct on mixed ledger | VERIFIED | `crates/reports/tests/open_positions_mixed_ledger.rs::t1106_v3_mixed_ledger_correct_open_positions`. Runs T1104 mixed fixture (2 legacy BTC+ETH rows on `assets:position:BTC` + 1 post-006 SOLUSDT row on `assets:position:SOLUSDT`). Asserts `Vec<OpenPosition>` length 3 sorted alphabetically with correct `(qty, avg_cost_basis, opened_at, strategy_id)`. |
| V4 | Anchor regression 11/11 | VERIFIED | `bash scripts/verify_anchors.sh` → `ANCHORS PASS (11 / 11)`. Section 6 above carries the verbatim stdout. |
| V5 | Reconciliation invariant Σ Dr == Σ Cr | VERIFIED | `t1105_v5_balance_invariant_pre_and_post_migration` (line 352). Iterates every txn in the mixed fixture, asserts `journal::verify_balance(...) == Ok(())`; then asserts global `Σ debit_amount == Σ credit_amount` on `journal_entries`. |
| V6 | operator-success-reports + live-cockpit-unified invariants | VERIFIED | `cargo test --workspace --all-targets` → 0 failures across 85 binaries. T802 / T805 / T806 / T809 / T810 / T901 / T903a–d / T905 / T906–T908 / T910 / T911 / T912 all green. See section 9 below. |
| V7 | Determinism: two reads byte-identical | VERIFIED | `t1106_v7_two_reads_byte_identical` in `open_positions_mixed_ledger.rs`. Two consecutive `query::open_positions_at(&ledger, period_end)` calls; `assert_eq!(first, second)` + `assert_eq!(first.len(), 3)`. |
| V8 | Universe coverage | VERIFIED | `t1105_v8_universe_coverage`. Parses `config/agent.toml [funding].universe` directly via `toml::from_str`; for each of the 10 symbols asserts `SELECT 1 FROM accounts WHERE id = 'assets:position:<SYM>'` returns one row. |

## 9. Operator-success-reports + live-cockpit-unified invariants

| Invariant | Status | Evidence |
|---|---|---|
| **Inv-T802** — `post_fill(strategy_id)` signature unchanged | VERIFIED | `crates/audit/src/journal.rs:39-43` `pub async fn post_fill(ledger: &Ledger, fill: &Fill, strategy_id: Option<&str>) -> Result<(), LedgerError>`. T802 ledger_integration suite (8/8 PASS) confirms behavioural compat. |
| **Inv-T805** — `feed_reconnect` writer | VERIFIED | `feed_reconnect_test.rs` 2/2 PASS — `t805_feed_reconnect_microsecond_timestamp_preserved`, `t805_feed_reconnect_writes_and_reads`. |
| **Inv-T806** — `agent_uptime` open/heartbeat/close | VERIFIED | `uptime_intervals_test.rs` 6/6 PASS, including `t806_full_open_heartbeat_close_cycle` and `t806_uptime_interval_carries_no_money`. |
| **Inv-T809** — `KillSwitch::trip` dual-write | VERIFIED | `kill_switch_dual_write_test.rs` 4/4 PASS + `kill_switch_trip_writes_both.rs` 3/3 PASS (audit-side + ui-side). Includes `t809_dual_write_atomic_in_one_transaction`. |
| **Inv-T810** — `--features in_process_cron` builds clean | VERIFIED | `cargo build -p agent --features in_process_cron` → `Finished dev profile [unoptimized + debuginfo] target(s) in 11.51s`, exit 0, zero warnings. |
| **Inv-T901** — Prometheus toggle | VERIFIED | `prometheus_toggle_test.rs` 3/3 PASS — `t912_disabled_skips_bind_via_public_api`, `t912_enabled_attempts_parse`, `t912_runtime_with_prometheus_disabled_does_not_bind_9100`. |
| **Inv-T902** — `runtime::run` clean cancel | VERIFIED | `runtime::tests::t902_runtime_run_returns_clean_on_cancel` PASS in agent unit tests. |
| **Inv-T903a** — `paper::on_fill` announce-second after `audit::post_fill` | VERIFIED | `exec` `paper::tests::t903a_paper_publishes_fill_and_position`, `t903a_multiple_fills_publish_once_each`, `t903a_backtest_path_is_inert` PASS. agent `bus::tests::t903a_glue_event_bus_impls_fill_publisher`, `t903a_glue_paper_engine_publisher_routes_to_bus` PASS. |
| **Inv-T903b** — bar/tick taps | VERIFIED | `runtime::tests::t903b_taps_publish_bars_and_ticks` PASS. |
| **Inv-T903c** — reconciler `PnlSnapshot` | VERIFIED | `reconciler::tests::t903c_after_bar_close_publishes_pnl` PASS. |
| **Inv-T903d** — bus drops on shutdown | VERIFIED | `bus_drops_on_shutdown.rs::t903d_bus_strong_count_collapses_on_cancel` PASS. |
| **Inv-T905** — kill-switch / mode forwarder | VERIFIED | `runtime::tests::t905_kill_switch_trip_emits_to_bus_mode` PASS. |
| **Inv-T906–T908** — UI panels | VERIFIED | `panel_snapshots.rs` 32/32 PASS (positions / pnl / strategies / kill / tape / latency). |
| **Inv-T910** — uptime smoke | VERIFIED | `unified_uptime_test.rs::t910_v3_graceful_shutdown_within_two_seconds_with_close_uptime_row` PASS. |
| **Inv-T911** — kill-button trips kill-switch | VERIFIED | `kill_switch_trip_writes_both.rs` 3/3 PASS. |
| **Inv-T912** — Prometheus toggle test | VERIFIED | `prometheus_toggle_test.rs` 3/3 PASS (see Inv-T901). |
| **2 v1+ anchors stay byte-identical** | VERIFIED | `verify_anchors.sh`: `report-sample-7d ab06dbcb…` + `report-sample-90d 2ef403f1…` PASS. |
| **9 backtest anchors byte-identical** | VERIFIED | `verify_anchors.sh`: all 9 v0/v0.5/v1/v1.5a anchors PASS. |

## 10. Environment / Infrastructure Issues

_none._ Build cache warm; tests deterministic across re-runs; anchors held
byte-identical pre- and post-test.

## 11. Verdict

**`PASS`**

All eight V-items VERIFIED. All 11 anchors PASS. All 5
operator-success-reports invariants and 11 live-cockpit-unified invariants
hold. T1101–T1107 citations resolve. Dev-side citation drift on Buy/buy
capitalization in T1104 (spec said `"Buy"`, runtime emits `"buy"`) was
caught and corrected by the developer mid-task; the lowercase fixture
matches `Side::Display` at `crates/core/src/symbol.rs:69` and
`open_positions_at`'s `description LIKE 'buy %'` filter at
`query.rs:1050`. Q4 cross-check correctly emits warn-only and keeps
description-parse primary; zero callers of the deprecated
`seed_universe_accounts`; migration `006` is idempotent and additive
(no money moves; no row UPDATEs).

## 12. Routing

`VERDICT → PASS` — feature ready to ship; presenter spawns next.
T_FINAL_PER_SYMBOL ticked in `spec/tasks/per-symbol-position-accounts.md`;
feature + task frontmatter bumped from `in-progress` → `shipped`.

## Changelog

- 2026-05-03 (tester): final test report. All gates green:
  fmt clean, build clean (23.92s), clippy clean, test workspace
  ~641 PASS / 0 FAIL across 85 binaries, doc tests clean,
  in_process_cron build clean, 11/11 anchors PASS (re-verified
  pre- and post-tests). V1–V8 all VERIFIED. T_FINAL_PER_SYMBOL
  ticked; status flipped in-progress → shipped.
