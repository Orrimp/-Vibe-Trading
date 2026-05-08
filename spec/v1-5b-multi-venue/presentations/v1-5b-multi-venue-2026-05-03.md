---
slug: v1-5b-multi-venue
mode: release
status: approved
audience: human-operator
updated: 2026-05-03
generated: 2026-05-03T19:52:55Z
approved_by: vitaliy.schreibmann@senacor.com
approved_at: 2026-05-03
---

# v1.5b — Multi-venue + 1s aggregated trades — release

## TL;DR

The agent now reads market data from Binance + Coinbase + Kraken (3 venues, 6 streams), supports USDC pairs, and aggregates 1-second bars client-side — finishing T612 (multi-symbol live BinanceFeed) and unblocking the v1.5a USDC pair-strategy carry-forward.

## What changed

- **Three venue feeds.** New `data::CoinbaseFeed` (Coinbase Advanced Trade WS) and `data::KrakenFeed` (Kraken WS v2) ingest adapters next to the existing Binance feed; Binance gains real multi-symbol fan-out via `subscribe_bars_multi` / `subscribe_trades_multi` (T612 closed).
- **Venue-tagged core types.** `core::Venue` enum (`Binance`, `Coinbase`, `Kraken`) is now a required field on every `Tick` and `Bar`. Audit migration `007_strategy_events_venue.sql` adds a typed `venue TEXT` column so `feed_reconnect` provenance is per-venue, not a single Binance counter.
- **Resilience.** `MarketHealth` watchdog with 30s stale-data threshold + per-venue `tokio::JoinSet` topology with panic-isolation supervisor — one venue's crash does not kill the others.
- **USDC + 1-second bars.** USDC mirror universe (10 symbols: BTCUSDC, ETHUSDC, …, LINKUSDC) via `[universe].usdc_enabled = true` opt-in, and a client-side 1-second bar aggregator (`data::bar_aggregator`) emitting `Timeframe::OneSecond` bars from the raw tick stream.

## Why

Today's single-venue topology is fragile (a Binance outage halts the whole agent) and crypto USDC liquidity is concentrated on Coinbase / Kraken, not Binance. v1.5b is the data-plumbing sibling deliberately split out of v1.5 by the analyst's `## Why` ([`spec/v1-5b-multi-venue/feature.md`](../features/v1-5b-multi-venue.md)) so multi-venue infra and the v1.5a strategy edge fail independently. With the venue-tagged Tick/Bar primitives in place, v1.5b is the foundation that future cross-venue arbitrage strategies (v2+) sit on, and a Binance outage no longer takes the whole feed down.

## What you can do now

The default config (Binance only, USDT only) is **unchanged behavior** — backwards-compat by construction (R10.2 + R10.3). Multi-venue is opt-in.

| Action | Command |
|--------|---------|
| Keep current behavior (Binance + USDT only) | (no change — defaults preserved) |
| Enable USDC mirror universe (10 extra symbols) | edit `config/agent.toml` → `[universe]` → `usdc_enabled = true` |
| Enable Coinbase ingest | uncomment / add `[data.sources.coinbase]` `enabled = true` stanza in `config/agent.toml` |
| Enable Kraken ingest | uncomment / add `[data.sources.kraken]` `enabled = true` stanza in `config/agent.toml` |
| Verify 3-venue Tick emission | `cargo test -p data --features fixtures --test binance_tick --test coinbase_tick --test kraken_tick -- --nocapture` |
| Re-verify byte-stable backtests | `bash scripts/verify_anchors.sh` |

## Live demo

```
$ cargo test -p data --features fixtures --test binance_tick --test coinbase_tick --test kraken_tick -- --nocapture
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.60s
     Running tests/binance_tick.rs (target/debug/deps/binance_tick-f360776b8f552096)

running 1 test
test t1411_v1_binance_tick_regression ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/coinbase_tick.rs (target/debug/deps/coinbase_tick-0b155c02bbdf3299)

running 1 test
test t1411_v2_coinbase_tick_emits_with_venue ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/kraken_tick.rs (target/debug/deps/kraken_tick-488fa28a73076510)

running 1 test
test t1411_v3_kraken_tick_emits_with_venue ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Each test asserts a Tick emitted by the corresponding venue's adapter carries the right `venue:` enum tag (Binance / Coinbase / Kraken), the symbol normalized through the venue's adapter (`BTCUSDC ↔ XBT/USDC` for Kraken), and the standard price / qty / side / timestamps invariants. Three venues, three Tick streams, all green.

Anchor gate (run separately):

```
$ bash scripts/verify_anchors.sh
PASS  btc-2023-1m-sma-cross                 fc2e3b4a...
PASS  btc-2023-1m-sma-baseline-refresh      fc2e3b4a...
PASS  btc-2023-1m-macd-trend                ef9c5e48...
PASS  btc-2023-1m-rsi-reversion             bc56d20d...
PASS  btc-2023-1m-bbands-mean-revert        d8a08a23...
PASS  top10-2023-1h-momentum                3b60ef07...
PASS  top10-2024-h1-momentum                1f33534f...
PASS  pairs-2023-zscore-mr                  90591a0e...
PASS  pairs-2024-h1-zscore-mr               14f50a59...
PASS  report-sample-7d                      ab06dbcb...
PASS  report-sample-90d                     2ef403f1...
---
ANCHORS PASS  (11 / 11)
```

## Screenshots

_n/a — backend infrastructure feature; no UI surface (cockpit consumes via existing EventBus channels). Future cockpit work could surface per-venue health badges; not in v1.5b scope._

## Verification

| V-id | Description | Status | Evidence |
|------|-------------|--------|----------|
| V1 | Binance feed regression unchanged | VERIFIED | `cargo test -p data` 41 lib tests green; single-symbol API untouched (R10.3); `crates/data/tests/binance_tick.rs:61` `t1411_v1_binance_tick_regression` PASS. |
| V2 | Coinbase feed connects + emits Tick (venue tagged) | VERIFIED | `crates/data/tests/coinbase_tick.rs:62` `t1411_v2_coinbase_tick_emits_with_venue` — `venue == Venue::Coinbase` + all Tick fields verified. |
| V3 | Kraken feed connects + emits Tick (venue tagged) | VERIFIED | `crates/data/tests/kraken_tick.rs:65` `t1411_v3_kraken_tick_emits_with_venue` — `BTCUSDC → XBT/USDC` symbol normalization end-to-end. |
| V4 | USDC pairs in audit DB | VERIFIED | `audit::tests::per_symbol_post_fill::t1105_v8_universe_coverage` green; T1410 USDC opt-in seeds 20 `assets:position:<SYMBOL>` rows when `usdc_enabled = true`. |
| V5 | 1s bars from synthetic trades + determinism | VERIFIED | `crates/data/tests/bar_aggregator_synth.rs:77` 6-bar OHLCV by hand; `:134` byte-identical across two runs. |
| V6 | Multi-symbol live BinanceFeed fan-out (T612 closeout) | VERIFIED | `crates/data/tests/binance_multi_symbol.rs:87` 10-symbol fan-out; lag ≤5s; `(venue_ts ASC, symbol ASC)` ordering preserved. |
| V7 | Coinbase outage scenario / panic isolation | VERIFIED | `crates/agent/tests/coinbase_outage_isolation.rs:180` — Binance + Kraken streams continue, venue-tagged `FeedReconnect` row written, `MarketHealth::Stale { venue: Venue::Coinbase }` published. |
| V8 | Anchor regression — 11/11 PASS | VERIFIED | `bash scripts/verify_anchors.sh` → `ANCHORS PASS (11 / 11)` (re-run this presentation). |
| V9 | T802/T805/T806/T809/T810 invariants | VERIFIED | `cargo test -p audit -p reports -p agent` green; `feed_reconnect` writes carry venue (`crates/audit/src/journal.rs:682`). T805 2/2; T806 6/6; T809 4/4; T810 in-process-cron clean. |
| V10 | T901-T912 (live cockpit) invariants | VERIFIED | `cargo test -p ui --features live` 102/102; cockpit live unchanged (bus channels stay venue-blind; consumer reads `bar.venue` on demand). |
| V11 | T1101-T1107 (per-symbol position accounts) | VERIFIED | `cargo test -p audit --test per_symbol_post_fill` 4/4; bootstrap seeding intact across both 10-symbol and 20-symbol universes. |
| V12 | T1201-T1209 + T1301-T1305 + cost telemetry | VERIFIED | UI panel + tape + modal suites all green; v1.5b uses no LLM and no paid market-data — cost ceiling preserved by construction. |

Performance budget (R5.5) — explicitly deferred: aggregator unit tests show sub-millisecond paths; formal criterion harness is a follow-up ticket (see Known follow-ups).

## Numbers that matter

- **Tests:** 96 result lines across the workspace, **all `0 failed`** — ~797 tests passed (no failures, 3 pre-existing `ignored` for live-Binance integration). UI live: 102/102. Doc tests clean.
- **Backend tasks:** 15 dev tasks shipped (T1401–T1415) + 5 venue/aggregator/MockFeed test files + 4 V-test files (`binance_tick`, `coinbase_tick`, `kraken_tick`, `bar_aggregator_synth`, `binance_multi_symbol`, `coinbase_outage_isolation`).
- **Mechanical-migration scope:** 35+ sites updated for the new required `Venue` field on every `Bar`/`Tick` literal (T1401, ~39 sites verified by `grep`).
- **Anchors:** 11/11 PASS (`ANCHORS PASS (11 / 11)` — body-SHA-256 byte-stable across the 9 backtest scenarios + 2 v1+ scenario anchors).
- **New crate dependencies:** **0** — architect-confirmed; all 3 venues reuse `tokio_tungstenite` + `serde_json` + `reqwest` already in the workspace.
- **Cost discipline:** **$0/mo** — Binance, Coinbase Advanced Trade, and Kraken WS v2 all expose free unauthenticated public market-data endpoints (Q8 confirmed). Hosting footprint stays inside the v2 $80/mo ceiling (≈30 long-lived TCP connections fit on a single VM).
- **Schema migrations:** 1 (additive) — `007_strategy_events_venue.sql` adds `venue TEXT NULLABLE` + index. No schema break.

## UI principles compliance

_n/a — backend feature, no UI surface._

## Known follow-ups

Two non-blocking items the tester surfaced. Neither blocks ship; both are tracked as architect follow-up tickets.

1. **RUSTSEC-2026-0104** in `rustls-webpki 0.103.12` (transitive: `metrics-exporter-prometheus 0.16.2 → hyper-rustls 0.27.9 → rustls 0.23.38 → rustls-webpki`). Pre-existing in the dependency graph — **not introduced by v1.5b** (architect-confirmed `git diff Cargo.toml Cargo.lock` for this feature shows zero new deps). Not reachable from any agent code path (no CRL parsing). Upstream `rustls-webpki ≥ 0.103.13` bump is the fix when available.
2. **Criterion bench harness for `bar_aggregator`** (R5.5 p99 < 500µs assertion) deferred — `crates/data/benches/` directory does not yet exist. The aggregator's runtime characteristics are exercised in 6 unit tests under `bar_aggregator::tests::t1406_*`, all sub-millisecond. Formal measurement is a follow-up ticket; does not block this ship.

## Open decisions

_no decisions pending — ready to ship_

## Approval

- [x] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback

<!-- empty until operator fills -->

## Changelog

- 2026-05-03 (presenter): initial draft. Release-mode presentation for v1.5b multi-venue (largest backend feature shipped this session). Embeds tester `VERDICT → PASS` from `spec/archive/test-2026-05-03-1946-v1-5b-multi-venue-final.md (archived; see spec/archive/README.md)`, `ANCHORS PASS (11 / 11)`, live 3-venue Tick demo (`binance_tick` / `coinbase_tick` / `kraken_tick`), V1–V12 verification matrix, and the two non-blocking architect follow-ups (RUSTSEC-2026-0104 transitive advisory, deferred criterion bench harness for R5.5). Approval block ships UN-ticked per presenter contract.
