---
title: Test Report
feature: operator-success-reports
run_id: 2026-05-01-0826-UTC
commit: 716f9e1b53d41b4d145520a79248d28549603878
agent: tester
verdict: PASS
---

# Test Report — operator-success-reports — 2026-05-01 08:26 UTC

## 1. Scope

- **Feature / change under test:** Wave 1 of operator-success-reports (T801–T806):
  `StrategyEventKind` v1+ variant additions; `004_journal_transactions_strategy_id.sql`
  migration + `post_fill` signature change; `audit::query::pnl_by_strategy`;
  `ledger_snapshot_sha` + `ledger_inception_ts` helpers; `feed_reconnect` writer +
  Binance reconnect call; `005_uptime_intervals.sql` migration + `agent_uptime`
  table + agent boot/heartbeat/shutdown wiring.
- **Spec refs:** `spec/features/operator-success-reports.md`,
  `spec/tasks/operator-success-reports.md`.
- **Commit SHA:** `716f9e1b53d41b4d145520a79248d28549603878`
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`
- **OS / arch:** `Darwin 25.4.0 arm64 (M-series)`

## 2. Static Analysis

| Check                                                      | Result | Notes                                  |
|------------------------------------------------------------|--------|----------------------------------------|
| `cargo fmt --all -- --check`                               | PASS   | exit 0, no diff                        |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS   | exit 0, no warnings                    |
| `cargo build --workspace --all-targets`                    | PASS   | 11.72s, no warnings                    |
| `cargo audit`                                              | n/a    | not in agent's mandate this round      |
| `cargo deny`                                               | n/a    | not in agent's mandate this round      |

## 3. Unit & Integration Tests

`cargo test --workspace --all-targets` — wall-clock dominated by backtest
determinism suite (~46 s).

| Crate          | Passed | Failed | Ignored | Notes                                                                                  |
|----------------|-------:|-------:|--------:|----------------------------------------------------------------------------------------|
| `trading_core` |     63 |      0 |       0 | 42 lib + 20 types + 1 trybuild. Includes 2 new T801 tests.                            |
| `audit`        |     45 |      0 |       0 | 8 ledger_integration (3 new T802) + 4 pnl_by_strategy (T803 new) + 5 strategy_events + 9 v15a + 6 funding_rate_history + 6 uptime (T806 new) + 3 snapshot_sha (T804 new) + 2 inception_ts (T804 new) + 2 feed_reconnect (T805 new). |
| `data`         |     15 |      0 |       3 | 8 lib + 3 funding_poller + 1 replay_60_bars + 3 binance_ws (live, ignored)            |
| `agent`        |     22 |      0 |       0 | unit + 4 v1_hot_swap + 3 v1_rebalance_reject + 2 v15a_pair_load_swap + 3 v15a_overlap |
| `ui`           |     59 |      0 |       0 | 25 lib + 32 panel_snapshots + 2 consistency                                            |
| `backtest`     |     28 |      0 |       0 | 3 lib + 18 determinism + 2 multi_pair_determinism + 5 multi_symbol_determinism        |
| `strategy`     |     96 |      0 |       0 | 76 lib + 11 bad_strategy_fixtures + 9 canonical_recipes (no bad_v1 explicitly counted here; included in workspace total) |
| `cost`         |      2 |      0 |       0 |                                                                                        |
| `risk`         |     10 |      0 |       0 |                                                                                        |
| `features`     |     55 |      0 |       0 |                                                                                        |
| `models`       |      0 |      0 |       0 |                                                                                        |
| `llm`          |      0 |      0 |       0 |                                                                                        |
| `exec`         |      0 |      0 |       0 |                                                                                        |
| **Total**      | **430**|      0 |       3 | 3 ignored = live Binance WS integration tests (require network).                       |

Doc-tests: `cargo test --workspace --doc` clean — all crates `0 passed; 0 failed; 0 ignored`
(`agent` has 1 ignored doctest — long-standing).

### New tests (Wave 1)

- **T801 (trading_core, 2 new):**
  - `strategy_events::tests::t801_strategy_event_kind_v1plus_variants`
  - `strategy_events::tests::t801_strategy_event_kind_v1plus_serde_roundtrip`
- **T802 (audit, 3 new):**
  - `ledger_integration::t802_post_fill_populates_strategy_id_when_some`
  - `ledger_integration::t802_post_fill_leaves_strategy_id_null_when_none`
  - `ledger_integration::t802_migration_004_creates_index`
- **T803 (audit, 4 new):**
  - `pnl_by_strategy::t803_12_fills_4_strategies_sorted_with_correct_stats`
  - `pnl_by_strategy::t803_unattributed_bucket_when_strategy_id_null`
  - `pnl_by_strategy::t803_empty_when_window_excludes_all_rows`
  - `pnl_by_strategy::t803_tie_break_by_strategy_id_asc`
- **T804 (audit, 5 new):**
  - `snapshot_sha::t804_snapshot_sha_byte_stable_across_two_reads`
  - `snapshot_sha::t804_snapshot_sha_flipping_one_byte_changes_digest`
  - `snapshot_sha::t804_snapshot_sha_known_vector_empty_file`
  - `inception_ts::t804_inception_ts_returns_earliest_of_three`
  - `inception_ts::t804_inception_ts_errors_on_empty_ledger`
- **T805 (audit, 2 new):**
  - `feed_reconnect_test::t805_feed_reconnect_writes_and_reads`
  - `feed_reconnect_test::t805_feed_reconnect_microsecond_timestamp_preserved`
- **T806 (audit, 6 new):**
  - `uptime_intervals_test::t806_full_open_heartbeat_close_cycle`
  - `uptime_intervals_test::t806_running_agent_has_stopped_at_none`
  - `uptime_intervals_test::t806_two_intervals_returned_in_chronological_order`
  - `uptime_intervals_test::t806_filter_by_since_excludes_earlier_rows`
  - `uptime_intervals_test::t806_default_ts_uses_microsecond_format`
  - `uptime_intervals_test::t806_uptime_interval_carries_no_money`

**Total new audit tests in Wave 1:** 20 (3 T802 + 4 T803 + 5 T804 + 2 T805 + 6 T806).
**Total new trading_core tests in Wave 1:** 2 (T801).

The dev's claim of "17 new audit tests + 2 new trading_core tests" understates
audit by 3 (the actual count is 20). All claimed new tests run green.

### Failing Tests

_none_

## 4. Property / Fuzz Tests

`trading_core` carries 3 proptest cases under `tests::order_tests::prop_*` —
all green (no shrunk failures). No new property tests in this wave.

| Suite                                | Cases | Shrunk failures | Seed     |
|--------------------------------------|------:|----------------:|----------|
| `trading_core::tests::order_tests::prop_*` | 3 | 0 | proptest default |

## 5. Backtest Results

_n/a — Wave 1 is types/schema/audit-query/wiring only; no strategy logic
changed. The V6 anchor gate (Section "Anchor verification") covers the
"backtest body bytes did not shift" guarantee for T802 — see Section 6._

## 6. Anchor Verification (V6 / mandatory gate)

Touched crates include `crates/audit/`, so `verify-anchors` is mandatory.

`bash scripts/verify_anchors.sh`:

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
---
ANCHORS PASS  (9 / 9)
```

**Verdict:** 9 / 9 PASS. The T802 schema change + `post_fill` signature change
did not leak into any rendered backtest body. V6 invariant intact.

> **Note on stale digests in T817 task body.** The task list at line 627 of
> `spec/tasks/operator-success-reports.md` cites `top10-2023-1h-momentum =
> a20431e3…` and `top10-2024-h1-momentum = 38b576335c9a…`. These do not match
> the current canonical digests in `spec/anchors.toml` (`3b60ef07…` and
> `1f33534fc7c6…` respectively, both v1 anchors with the v1.5a `top10-2023`
> body refreshed by T717 — see `spec/anchors.toml` line 43, 48). The locked
> source-of-truth is `spec/anchors.toml`, and the gate is intact. The stale
> hex digests in the task body are a documentation nit (architect / spec-update
> follow-up) — not a Wave 1 implementation defect.

## 7. Tick verification (honest-tick rule)

For each ticked task in `spec/tasks/operator-success-reports.md`, I verified
(a) the file:line citation, (b) ran the cited test command, (c) confirmed the
cited output line appears in the result.

### T801 — `StrategyEventKind` v1+ variants — **VERIFIED**

- File:line citation: `crates/core/src/strategy_events.rs:111` (`KillSwitchTripped`),
  `:113` (`FeedReconnect`), Display arms `:126-127`. **Confirmed verbatim** —
  see `crates/core/src/strategy_events.rs` lines 110–114 for variants and
  126–127 for Display arms.
- Tests at `crates/core/src/strategy_events.rs:277-303` (range slightly different
  from dev's `:277-296` — actual range extends to 303 to cover the second
  variant's roundtrip). **Functions present and correct.**
- Test cmd: `cargo test -p trading_core --lib`.
- Output: `test strategy_events::tests::t801_strategy_event_kind_v1plus_variants ... ok`,
  `test strategy_events::tests::t801_strategy_event_kind_v1plus_serde_roundtrip ... ok`,
  `42 passed; 0 failed`. Confirmed.

### T802 — `004_journal_transactions_strategy_id.sql` + `post_fill` signature — **VERIFIED**

- Migration at `crates/audit/migrations/004_journal_transactions_strategy_id.sql`
  (ALTER TABLE + `journal_transactions_sid_idx` index). Confirmed.
- `post_fill` signature change at `crates/audit/src/journal.rs:35-39` (gains
  `strategy_id: Option<&str>`); INSERT writes the column at lines 63-73.
  Confirmed (dev cited `:35-38` and `:55-65`; line numbers are within ±5 of
  actual; content matches verbatim).
- Tests at `crates/audit/tests/ledger_integration.rs` (3 new t802 tests).
- Test cmd: `cargo test -p audit --test ledger_integration`.
- Output: `test t802_migration_004_creates_index ... ok`,
  `test t802_post_fill_leaves_strategy_id_null_when_none ... ok`,
  `test t802_post_fill_populates_strategy_id_when_some ... ok`, `8 passed; 0 failed`.
- Anchor regression: 9 / 9 PASS (Section 6). Confirmed.

### T803 — `audit::query::pnl_by_strategy` — **VERIFIED**

- `StrategyPnl` struct at `crates/audit/src/query.rs:527-541` (dev cited
  `:524-538` — within ±3 lines, content matches).
- `pnl_by_strategy` at `crates/audit/src/query.rs:561-680`. Sorted by realized
  DESC with strategy_id ASC tie-break (lines 657–679). NULL → "(unattributed)"
  bucket at line 618. Closed-trade per-txn accumulator at 606–634. Confirmed.
- Tests at `crates/audit/tests/pnl_by_strategy.rs` (4 new T803 tests).
- Test cmd: `cargo test -p audit --test pnl_by_strategy`.
- Output: `test t803_12_fills_4_strategies_sorted_with_correct_stats ... ok`
  + 3 others, `4 passed; 0 failed`. Confirmed.

### T804 — `ledger_snapshot_sha` + `ledger_inception_ts` — **VERIFIED**

- `ledger_snapshot_sha` at `crates/audit/src/query.rs:876-896` (chunked 64 KiB
  read via `sha2::Sha256`).
- `ledger_inception_ts` at `crates/audit/src/query.rs:909-924`.
- Note: dev cited `:813-833` and `:846-862`, but those line numbers actually
  point to T806's `UptimeInterval`/`uptime_intervals_since`. The T804 functions
  exist and are correct (at `:876` and `:909`); the **dev's citation line
  numbers are stale by ~63 lines** — likely the dev wrote citations against
  an earlier ordering of the file before the T806 helpers were added at lines
  801–862. **Functions verified present and tests pass; the line-number drift
  is documentation noise, not a defect.**
- `sha2` runtime dep + `tempfile` dev-dep at `crates/audit/Cargo.toml:20,24`. Confirmed.
- Tests at `crates/audit/tests/snapshot_sha.rs:1-72` and `inception_ts.rs`
  (3 + 2 = 5 new T804 tests).
- Test cmd: `cargo test -p audit --test snapshot_sha --test inception_ts`.
- Output: `5 passed; 0 failed` across both files. Confirmed.

### T805 — `feed_reconnect` writer + Binance reconnect call — **VERIFIED**

- Writer at `crates/audit/src/journal.rs:531-551` (`feed_reconnect(ledger,
  symbol, ts: Option<&str>)` — `kind = "FeedReconnect"`, `error_summary =
  symbol`). Dev cited `:498-528` — actual line range is `:516-551` (+18). Drift
  again caused by uptime helpers landing earlier in the file. **Function
  present and correct.**
- Parser arm at `crates/audit/src/query.rs` (verified by tests' round-trip).
- `BinanceFeed::with_ledger` at `crates/data/src/binance.rs:119`; reconnect
  emit at `:295-306` (subscribe_bars) and `:405-416` (subscribe_trades). Dev
  cited `:285-296` and `:411-422` — within ±10 lines, content matches.
  `is_reconnect` flag at `:280` and `:393` suppresses the first connect.
- Tests at `crates/audit/tests/feed_reconnect_test.rs` (2 new T805 tests).
- Test cmd: `cargo test -p audit --test feed_reconnect_test`.
- Output: `test t805_feed_reconnect_writes_and_reads ... ok`,
  `test t805_feed_reconnect_microsecond_timestamp_preserved ... ok`, `2 passed;
  0 failed`. Confirmed.

### T806 — `005_uptime_intervals.sql` + `agent_uptime` — **VERIFIED**

- Migration at `crates/audit/migrations/005_uptime_intervals.sql` (table
  `agent_uptime` + `agent_uptime_started_idx` index). Confirmed.
- Three writers at `crates/audit/src/journal.rs:624` (`open_uptime_interval`),
  `:652` (`heartbeat_uptime`), `:677` (`close_uptime_interval`); `uptime_ts_string`
  helper at `:601`. Dev citation lines exactly match. Confirmed.
- Reader `uptime_intervals_since` at `crates/audit/src/query.rs:823-862` (dev
  cited `:802` — close, function spans `:823+`; structure confirmed).
- Agent main wiring: boot at `crates/agent/src/main.rs:106-110` (dev cited
  `:107`; UUID generated at `:106`); 30s heartbeat at `:113-134` (dev citation
  matches); close at `:284-288` (dev citation matches). Cancellation token
  cancels heartbeat before close — verified by reading the surrounding code.
- `uuid` runtime dep at `crates/agent/Cargo.toml`. Confirmed.
- Tests at `crates/audit/tests/uptime_intervals_test.rs` (6 new T806 tests).
- Test cmd: `cargo test -p audit --test uptime_intervals_test`.
- Output: `test t806_full_open_heartbeat_close_cycle ... ok` + 5 others,
  `6 passed; 0 failed`. Confirmed.

### Tick-verification summary

| Task  | Verdict   | Notes                                                                              |
|-------|-----------|------------------------------------------------------------------------------------|
| T801  | VERIFIED  | All citations match within minor line-number drift; tests green.                   |
| T802  | VERIFIED  | Migration file present; signature correct; anchor gate intact (V6 OK).             |
| T803  | VERIFIED  | Sum-equals-scalar invariant + (unattributed) bucket + tie-break tests green.       |
| T804  | VERIFIED  | Functions present at `:876` / `:909` (dev's `:813-833` / `:846-862` was stale).    |
| T805  | VERIFIED  | Line numbers drift by ~+15 due to T806 ordering; content correct; tests green.     |
| T806  | VERIFIED  | Migration + 3 writers + reader + agent wiring all confirmed; 6 tests green.        |

All 6 ticks carry citation evidence with file:line + test command + output line
satisfying the honest-tick rule. **No tick needs to be un-ticked.** The
line-number drift in T804 / T805 dev notes is documentation noise — the actual
functions exist, are correct, and the tests pass. (Recommend the developer
re-grep their citations next round; future tick verification should be more
robust to file reordering.)

`T_FINAL_REPORTS` is correctly still `[ ]` unticked (verified at
`spec/tasks/operator-success-reports.md` line 644).

## 8. Architectural notes (NOT defects — flagged for architect)

1. **New runtime edge `data → audit` is undocumented in `spec/architecture.md`.**
   The Wave 1 implementation promotes `audit` from a dev-dep to a runtime dep
   on `data` (`crates/data/Cargo.toml:9`) so the Binance reconnect handler can
   call `audit::journal::feed_reconnect`. The mermaid `Data flow` block at
   `spec/architecture.md:90-101` shows `feed → data → features → …` with no
   edge from `data` into `audit`. This is the dev's flagged delta and it is
   **real**: the architecture diagram needs an additive edge `data → audit`
   (audit-side, no reverse dependency back into `data`). **Routing:** this is
   architect-owned — I have NOT modified `spec/architecture.md` myself per
   the tester contract. Recommend the orchestrator schedules an architect
   round to record this delta before Wave 2 begins.

2. **Stale anchor digests in T817 task body.** Lines 626–631 of
   `spec/tasks/operator-success-reports.md` reference
   `top10-2023-1h-momentum = a20431e3…` and `top10-2024-h1-momentum =
   38b576335c9a…`. The canonical anchors live in `spec/anchors.toml` and read
   `3b60ef07…` and `1f33534fc7c6…`. The `verify_anchors.sh` gate uses
   `spec/anchors.toml` as source-of-truth and PASSES, so this is a
   documentation drift, not a defect. Architect / spec-update follow-up:
   reconcile T817 task body to match `spec/anchors.toml`.

## 9. Environment / Infrastructure Issues

3 ignored tests in `data` (`tests/binance_ws_integration.rs`) require a live
Binance WebSocket connection; they're explicitly gated behind `--ignored` per
their assertion string. Not flaky. Not relevant to Wave 1 verification.

## 10. Verdict

**`PASS`**

Build clean (11.72s, no warnings). `cargo fmt --check` clean. `cargo clippy
--workspace --all-targets --all-features -- -D warnings` clean. All workspace
tests pass (430 passed, 0 failed, 3 ignored — live network tests). Doc-tests
clean. Anchor gate `9 / 9 PASS` — V6 invariant intact across the T802 schema
+ `post_fill` signature change. All 6 dev-ticked rows (T801–T806) carry
citation evidence and verify against the implementation; line-number drift
in T804 / T805 dev notes is documentation noise (not a verification failure).
`T_FINAL_REPORTS` correctly still unticked.

The only architectural note is the new `data → audit` runtime edge, which the
developer correctly flagged and which is owned by the architect (not the
developer / not the tester). Wave 2 (T807–T817) is unblocked from a Wave 1
perspective; the architect should record the dependency edge before T807
lands so the architecture stays in sync with the build graph.

## 11. Routing

`VERDICT → PASS` — Wave 1 (T801–T806) ready. The orchestrator should:

1. Schedule an architect round to record the `data → audit` runtime dep
   in `spec/architecture.md` (additive edge in the data-flow diagram +
   any narrative paragraphs that enumerate cross-crate deps).
2. Optionally have spec-update reconcile the stale T817 anchor digests
   in `spec/tasks/operator-success-reports.md` against `spec/anchors.toml`.
3. Then unblock Wave 2 (T807–T817) — developer + (no UI) per the task list's
   "Handoff contract — no UI involvement" section.

`VERDICT → PASS`
