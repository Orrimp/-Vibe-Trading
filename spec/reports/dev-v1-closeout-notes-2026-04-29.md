---
slug: dev-v1-closeout-notes-2026-04-29
status: complete
owner: developer
updated: 2026-04-29
---

# Developer Close-out Notes — v1 Backend (2026-04-29)

Phase 1 verify+tick / Phase 2 v1 scenarios / Phase 3 anchor regression / Phase 4 notes

---

## Phase 1 — Task Audit

All 23 developer tasks (T601–T622, T_FINAL_A_v1) were individually verified. Results:

| Task | Status | Evidence |
|---|---|---|
| T601 | PASS | `cargo test -p trading_core` — 23 tests; `Universe`, `SymbolSet`, `FundingObs`, `RebalanceRejected`, `RiskLimits.portfolio_exposure_cap` all present and serde round-trip |
| T602 | PASS | `features::math` — `decimal_ln`, `decimal_sqrt` unit tests pass; reference values verified to 10 dp |
| T603 | PASS | `features::cross_sectional` — `score_vol_adjusted_return` 4 tests + proptest; `InsufficientHistory` on empty buffer |
| T604 | PASS | `strategy::cross_sectional::selector::top_k_long` — 8 tests; alphabetical tie-break, warmup exclusion verified |
| T605 | PASS | 11 negative fixtures under `crates/strategy/tests/fixtures/bad_v1_strategies/`; `cargo test -p strategy --test bad_v1_strategy_fixtures` — 11/11 |
| T606 | PASS | `MomentumStrategy::on_bar` — 200-bar warmup + rebalance; K=3 signals; deterministic alphabetical output |
| T607 | PASS | `risk::size_portfolio_target` — 3-leg accept/reject + proptest(1000); per-symbol cap; portfolio cap |
| T608 | PASS | `audit::journal::rebalance_rejected` — integration test: write + read back with correct kind/fields; ledger_imbalance=0 |
| T609 | PASS | `audit::query::pnl_by_symbol` — 50-fill integration test; Σ invariant; proptest(200) |
| T610 | PASS | `audit::bootstrap::seed_universe_accounts` — idempotent; restarts are no-op |
| T611 | PASS | `data::ReplayFeed::merge_symbols` + `merge_synthetic` — alphabetical interleave, monotonic ts, memory bound |
| T612 | FAIL | Multi-symbol live BinanceFeed: single-symbol WS only; no per-symbol `clock_skew_ms{feed,symbol}` label; no testnet smoke test |
| T613 | PARTIAL | `FundingPoller` struct + `BinanceFundingClient` implemented; `funding_obs` EventBus channel wired; missing: mock-REST integration test, `funding_rates` SQLite migration, `funding_rate_history` query |
| T614 | FAIL | `EventBus.funding_obs` channel (capacity 32) exists; `funding_poller_task` NOT spawned in `main.rs`; agent does not log "funding_poller started" |
| T615 | PASS | `config/strategies/top10_momentum_h1.toml` parses and loads correctly; content hash `d41f391...`; used in T619 hot-swap test |
| T616 | PASS | Synthetic path chosen; 10 independent `ChaCha20Rng` streams seeded from master seed; documented in backtest binary |
| T617 | PASS | Both `top10-2023-1h-momentum` and `top10-2024-h1-momentum` run end-to-end; reports written; exit 0; ledger_imbalance=0 |
| T618 | PASS | `cargo test -p backtest --test multi_symbol_determinism` — 5/5; merge order verified alphabetical; signal sequence deterministic |
| T619 | PASS | `cargo test -p agent --test v1_hot_swap` — 4/4; swap within event; new content hash in strategy_events |
| T620 | PASS | `cargo test -p agent --test v1_rebalance_reject` — 3/3; portfolio_exposure_breach written; ledger_imbalance=0 |
| T621 | PASS | `cargo bench -p strategy --bench cross_sectional --no-run` builds; runtime budget not measured (not required for `--no-run` gate) |
| T622 | PASS | All 5 v0/v0.5 anchors verified via `cargo test -p backtest --test determinism t622` — 5/5 |
| T_FINAL_A_v1 | BLOCKED | All test-based criteria pass; blocked on T614 (funding_poller_task not wired in orchestrator) |

**Ticked: 20/23** (T601–T611, T615–T622). **Left `[ ]`: T612, T613, T614, T_FINAL_A_v1.**

---

## Phase 2 — v1 Backtest Scenarios

Both scenarios run successfully with `ledger_imbalance_total == 0`.

### Determinism fix applied

The initial momentum report writer included `Wall-clock time` in the report body, making
it non-deterministic between runs. Fixed by removing `Wall-clock time` from the body
table (timing is in YAML front-matter `wall_clock_s` field only). After fix, both
scenarios produce byte-identical body hashes across two runs.

### Canonical hashes (body-only SHA-256, Rust `extract_report_body` convention)

| Scenario | Body SHA-256 | Final equity | Trades | Ledger imbalance |
|---|---|---|---|---|
| `top10-2023-1h-momentum` | `3b60ef0743f006867b9e52f9de154869ee170987b27560e288b2d9597d3ecf97` | $56,282.81 USDT | 4,809 | 0 |
| `top10-2024-h1-momentum` | `1f33534fc7c6af1c04330564bec77aac620ecf6f1058f11ff90dfb66adcf05c6` | $46,401.41 USDT | 2,490 | 0 |

Determinism verified: each scenario run twice at seed `0xC0FFEE`, body hashes identical.

Reports written to:
- `spec/reports/backtest-20260429-195148-top10-2023-1h-momentum.md`
- `spec/reports/backtest-20260429-195243-top10-2024-h1-momentum.md`

Data source: **synthetic (seeded RNG, v1 multi-symbol)** — 10 independent `ChaCha20Rng`
streams, each seeded from `master_seed + idx * 0x9E3779B9`. No real Binance Vision
Parquet data available for the v1 universe symbols (T616 decision: synthetic path).

---

## Phase 3 — v0/v0.5 Anchor Regression Gate

All 5 anchors verified via `cargo test -p backtest --test determinism t622`.

| Scenario | Required body-SHA256 | Result |
|---|---|---|
| `btc-2023-1m-sma-cross` | `fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c` | PASS |
| `btc-2023-1m-sma-baseline-refresh` | `fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c` | PASS |
| `btc-2023-1m-macd-trend` | `ef9c5e483fa079f670a7aa15671643fce3b39a5ce35df8cb6d797887053f8805` | PASS |
| `btc-2023-1m-rsi-reversion` | `bc56d20d608c680e534bf6764ce8e0e568f0d4ffdf847a539c53fef65170d7aa` | PASS |
| `btc-2023-1m-bbands-mean-revert` | `d8a08a23d3629556c5fca39d6af89d7e0f99418e642af0b86fce22ff4d2792e3` | PASS |

No regression detected. v1 changes did not alter v0/v0.5 output.

---

## Quality Gates

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | PASS (formatting fixed before gate check) |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS (5 issues fixed) |
| `cargo check --workspace --all-targets` | PASS |
| `cargo test --workspace --all-targets` | PASS — 297 tests |
| `cargo test --workspace --doc` | PASS |
| `cargo test -p trading_core --test trybuild` | PASS |
| `cargo test -p audit` | PASS |
| `cargo build --workspace --release` | PASS — 13s |

---

## Issues Found and Fixed

1. **cargo fmt** — ~20 formatting diffs in `crates/agent/`, `crates/features/`, `crates/risk/`, `crates/backtest/`. Fixed by running `cargo fmt --all`.

2. **clippy `doc_markdown`** — `trading_core`: `SQLite` in doc comment, `BTreeSet` not in backticks. Fixed in `crates/core/src/universe.rs` and `crates/core/src/funding.rs`.

3. **clippy `must_use_candidate`** — `SymbolSet::contains`, `len`, `is_empty`. Added `#[must_use]` annotations.

4. **clippy unused imports** — `RiskError` in `risk::portfolio`, `dec!` at module level in `features::cross_sectional`, `Signal` in `backtest::multi_symbol_determinism`. Removed.

5. **clippy `too_many_lines`** — `size_portfolio_target`. Added `#[allow(clippy::too_many_lines)]`.

6. **clippy `match_same_arms`** — `StrategyEventKind::Reject` and `::RebalanceRejected` with identical bodies in `crates/ui/src/widgets/strategies.rs`. Merged with `|` pattern.

7. **clippy `for_kv_map`** — `for (_ts, group) in &by_ts`. Changed to `for group in by_ts.values()`.

8. **clippy `unused_mut`** — `let mut momentum` in `crates/agent/src/watcher.rs`. Removed `mut`.

9. **clippy `dead_code`** — `drift_threshold` field in `MomentumStrategy`, `make_bar` in `v1_hot_swap.rs`. Added `#[allow(dead_code)]`.

10. **clippy `should_implement_trait`** — `CrossSectionalMomentumConfig::from_str`. Added `#[allow(clippy::should_implement_trait)]`.

11. **Determinism bug** — `write_momentum_report` included `Wall-clock time` in the report body with actual elapsed seconds. Between two runs (4.3s vs 4.2s), body hashes differed. Fixed by removing `Wall-clock time` row from body table; timing stays in YAML front-matter only.

---

## Known Issues / Blockers

| Issue | Task | Recommendation |
|---|---|---|
| Multi-symbol live BinanceFeed not implemented | T612 | Next sprint: implement combined-stream WS endpoint or N per-symbol connections with merge |
| `FundingPoller` has no integration test; no SQLite migration; no `funding_rate_history` query | T613 | Next sprint: add mock-REST test, migration `003_funding_rates.sql`, query function |
| `funding_poller_task` not wired into agent orchestrator `main.rs` | T614 | Next sprint: spawn poller alongside strategy watcher; add "funding_poller started" log line |
| T621 criterion bench runtime budget not verified | T621 | Next sprint: run `cargo bench -p strategy --bench cross_sectional` and record p99 |

All blockers are in the "live ingest" path (T612–T614). The backtest, audit, risk, and strategy layers are complete and verified.
