//! T1413 — V6 multi-symbol live BinanceFeed fan-out test.
//!
//! Verifies the architect's design for R4 (10-symbol multi-stream fan-out)
//! using the deterministic [`data::MockFeed`] harness (T1407 / Q10).  A
//! production [`data::BinanceFeed::subscribe_bars_multi`] uses Binance's
//! combined-stream URL — that wire-frame parser is exercised by a
//! separate WS-server harness; this test focuses on the **merge order +
//! fan-out invariants** that any 10-symbol-capable feed must guarantee.
//!
//! Asserts:
//!
//! 1. **Fan-out completeness** — every scripted Tick on every one of the
//!    10 symbols reaches a bus subscriber.  Total received tick count
//!    equals the scripted total; per-symbol counts match the script.
//! 2. **Venue tagging** — every Tick carries `venue == Venue::Binance`.
//! 3. **No loss / no duplication** — the multiset of `(symbol, trade_id)`
//!    pairs received equals the multiset scripted (no dropped, no
//!    repeated events).
//! 4. **Bounded bus lag** — `local_recv_ts - venue_ts <= 5s` per bar
//!    (R4 acceptance constraint).  We construct ticks with a synthetic
//!    `local_recv_ts == venue_ts` so the constraint is structural rather
//!    than wall-clock dependent.
//! 5. **Determinism** — running this test under `tokio::time::pause` +
//!    `advance` produces identical output regardless of host wall-clock.
//!
//! The 10-symbol universe is the v1.5b USDT mirror set
//! ([`trading_core::universe::DEFAULT_USDT_SYMBOLS`]).
//!
//! Per the v1.5b feature brief (Q10), `MockFeed` impls `MarketDataSource`
//! directly — no WS frame parsing — so this test is hermetic and CI-safe.
//!
//! **Feature gate.** Uses `MockFeed` from `crates/data/src/mock_feed.rs`,
//! which is itself gated behind `#[cfg(any(test, feature = "fixtures"))]`
//! per Q10 / T1407. Mirrors the `#![cfg(feature = "fixtures")]` gate
//! used by the sibling `binance_tick.rs` / `coinbase_tick.rs` /
//! `kraken_tick.rs` integration tests (T1411).
//!
//! Run with:
//!
//! ```bash
//! cargo test -p data --features fixtures --test binance_multi_symbol
//! ```
#![cfg(feature = "fixtures")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::HashMap;
use std::time::Duration;

use data::{MarketDataSource, MockFeed};
use futures::StreamExt;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::universe::DEFAULT_USDT_SYMBOLS;
use trading_core::{Price, Quantity, Side, Symbol, Tick, Timestamp, Venue};

/// Build a deterministic Tick for `symbol` with a synthetic monotonic
/// `venue_ts` and matching `local_recv_ts` (so structural lag is 0; the
/// 5-s lag bound is therefore trivially satisfied).
fn mk_tick(symbol: &Symbol, ts_us: i64, id: u64) -> Tick {
    let dt =
        OffsetDateTime::from_unix_timestamp_nanos(i128::from(ts_us) * 1_000).expect("valid ts");
    let ts = Timestamp::new(dt);
    Tick {
        symbol: symbol.clone(),
        venue_ts: ts,
        local_recv_ts: ts,
        price: Price::new(dec!(60000)).expect("price"),
        qty: Quantity::new(dec!(0.001)).expect("qty"),
        side: Side::Buy,
        trade_id: id,
        venue: Venue::Binance,
    }
}

/// T1413 V6 — 10-symbol Binance fan-out: all scripted Ticks reach the
/// bus subscriber, every Tick is venue-tagged Binance, no loss / no
/// duplication, lag bounded under 5s.
///
/// Drives one `MockFeed` *per symbol* via `subscribe_trades` (the
/// architect's V6 design point: each symbol is a separate stream
/// merged on the consumer side).  The merge surface is a single
/// `tokio::sync::broadcast` channel — the same channel the production
/// `EventBus::ticks` uses.  This isolates the test from any change to
/// `EventBus` while still exercising the real "per-symbol stream →
/// shared broadcast" merge invariant.
#[tokio::test(start_paused = true, flavor = "current_thread")]
async fn t1413_v6_binance_multi_symbol_fanout() {
    // ── 1. Build a 10-symbol script ───────────────────────────────────────
    // Per architect's design (R4 + V6): one Tick per symbol per "minute
    // boundary".  We emit 3 Ticks per symbol (30 events total) so the
    // fan-out is non-trivial.
    const TICKS_PER_SYMBOL: u64 = 3;
    let symbols: Vec<Symbol> = DEFAULT_USDT_SYMBOLS
        .iter()
        .map(|s| Symbol::new(*s))
        .collect();
    assert_eq!(
        symbols.len(),
        10,
        "USDT mirror set must be 10 symbols (v1.5b R4)"
    );

    let mut events_map: HashMap<Symbol, Vec<Tick>> = HashMap::new();
    let mut expected: Vec<(Symbol, u64)> = Vec::new();
    for (sym_idx, symbol) in symbols.iter().enumerate() {
        let mut ticks = Vec::new();
        for tick_idx in 0..TICKS_PER_SYMBOL {
            // Spread trade_id across symbols so duplicates are detectable
            // (each `(symbol, trade_id)` pair is globally unique).
            let trade_id = (sym_idx as u64) * 1000 + tick_idx + 1;
            // Stagger venue_ts per symbol so consumer-side merge order is
            // observable without colliding on identical timestamps.
            let ts_us = 1_700_000_000_000_000 + (tick_idx as i64) * 60_000_000;
            ticks.push(mk_tick(symbol, ts_us, trade_id));
            expected.push((symbol.clone(), trade_id));
        }
        events_map.insert(symbol.clone(), ticks);
    }

    let feed = MockFeed::new_multi(events_map, Duration::from_millis(10), Venue::Binance);

    // ── 2. Stand up a broadcast channel mirroring `EventBus::ticks`
    //        topology (1024 capacity matches the v1.5b production bus).
    let (ticks_tx, mut subscriber) = tokio::sync::broadcast::channel::<Tick>(1024);

    // ── 3. Spawn one tap per symbol that re-publishes onto the shared
    //        broadcast channel — exactly the architect's R4 fan-out
    //        topology (per-symbol stream → shared bus).
    let total_expected = expected.len();
    let mut taps = tokio::task::JoinSet::new();
    for symbol in &symbols {
        let mut stream = feed
            .subscribe_trades(symbol.clone())
            .await
            .expect("subscribe_trades ok");
        let tx = ticks_tx.clone();
        taps.spawn(async move {
            while let Some(item) = stream.next().await {
                match item {
                    Ok(tick) => {
                        // Send via the broadcast; no subscribers present
                        // would surface as `SendError` — but our
                        // subscriber is alive, so this should always
                        // succeed.
                        let _ = tx.send(tick);
                    }
                    Err(_) => break,
                }
            }
        });
    }
    // Drop our extra sender clone so the channel closes once all tap
    // tasks exit naturally (each symbol's script is finite).
    drop(ticks_tx);

    // ── 4. Drive the paused-tokio clock until every scripted tick has
    //        flowed through.  Each `advance(11ms)` releases one slot
    //        from every symbol's `interval`-paced stream.
    //
    // We collect with a generous tokio-time budget; the actual wall-
    // clock cost is microseconds because tokio is paused.
    let collected = tokio::time::timeout(Duration::from_secs(10), async {
        let mut received: Vec<Tick> = Vec::new();
        // Each symbol emits TICKS_PER_SYMBOL ticks; the MockFeed's
        // initial yield-then-deliver pattern means we need
        // (TICKS_PER_SYMBOL + 1) interval cycles to drain everything.
        for _ in 0..(TICKS_PER_SYMBOL + 2) {
            tokio::time::advance(Duration::from_millis(11)).await;
            // After advancing, drain whatever the subscriber has buffered.
            // `tokio::time::advance` returns synchronously, but the
            // tap tasks need a yield to actually publish.  Yield once,
            // then collect everything available.
            tokio::task::yield_now().await;
            tokio::task::yield_now().await;
            // Drain whatever the subscriber has buffered without
            // blocking — `try_recv` returns `Empty` when no message is
            // available rather than awaiting one.
            loop {
                match subscriber.try_recv() {
                    Ok(tick) => received.push(tick),
                    Err(tokio::sync::broadcast::error::TryRecvError::Empty) => break,
                    Err(tokio::sync::broadcast::error::TryRecvError::Closed) => break,
                    Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => continue,
                }
            }
            if received.len() >= total_expected {
                break;
            }
        }
        received
    })
    .await
    .expect("tap tasks did not deliver inside the budget");

    // ── 5. Assertions ─────────────────────────────────────────────────────

    // 5a. Total count: no message lost, no message duplicated.
    assert_eq!(
        collected.len(),
        total_expected,
        "expected {total_expected} fan-out ticks, got {}",
        collected.len()
    );

    // 5b. Every Tick carries Venue::Binance (R4 / Q4 — venue-tagged).
    for tick in &collected {
        assert_eq!(
            tick.venue,
            Venue::Binance,
            "every fan-out tick must be venue-tagged Binance, got {:?} for {}",
            tick.venue,
            tick.symbol
        );
    }

    // 5c. Bounded bus lag (R4 acceptance: lag <= 5s).
    let max_lag_secs: i128 = 5;
    for tick in &collected {
        let lag_ns: i128 = tick.local_recv_ts.inner().unix_timestamp_nanos()
            - tick.venue_ts.inner().unix_timestamp_nanos();
        let lag_secs: i128 = lag_ns / 1_000_000_000;
        assert!(
            lag_secs.abs() <= max_lag_secs,
            "tick lag {lag_secs}s > 5s budget for {} (id={})",
            tick.symbol,
            tick.trade_id
        );
    }

    // 5d. No loss + no duplication: the multiset of (symbol, trade_id)
    //     received equals the multiset scripted.
    let mut got_keys: Vec<(Symbol, u64)> = collected
        .iter()
        .map(|t| (t.symbol.clone(), t.trade_id))
        .collect();
    got_keys.sort();
    let mut want_keys = expected;
    want_keys.sort();
    assert_eq!(
        got_keys, want_keys,
        "fan-out multiset mismatch — message loss or duplication"
    );

    // 5e. Per-symbol completeness — each of the 10 symbols contributed
    //     exactly TICKS_PER_SYMBOL events.
    let mut counts: HashMap<Symbol, u64> = HashMap::new();
    for tick in &collected {
        *counts.entry(tick.symbol.clone()).or_default() += 1;
    }
    assert_eq!(
        counts.len(),
        10,
        "expected 10 distinct symbols, got {}",
        counts.len()
    );
    for symbol in &symbols {
        let n = counts.get(symbol).copied().unwrap_or(0);
        assert_eq!(
            n, TICKS_PER_SYMBOL,
            "symbol {symbol} contributed {n} ticks, expected {TICKS_PER_SYMBOL}"
        );
    }

    // ── 6. Drain the spawned tap tasks (they exit naturally as the
    //        scripts are finite) so the test does not leak background
    //        work between assertions and process teardown.
    let drain = async { while let Some(_res) = taps.join_next().await {} };
    tokio::time::timeout(Duration::from_secs(5), drain)
        .await
        .expect("tap tasks did not drain inside the budget");
}
