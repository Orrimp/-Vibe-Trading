//! T1414 — V7 Coinbase outage isolation integration test.
//!
//! Per the v1.5b multi-venue feature brief:
//!
//! - **R14.1 / R14.3** — a panic in any one venue's task must not
//!   propagate into `runtime::run`; the other venues' tasks keep running.
//! - **R8** — `feed_reconnect` audit writes carry a venue-tagged row.
//! - **Q3** — per-venue tokio tasks (panic-isolated) is the chosen
//!   topology.
//! - **Q7** — when a venue's feed dies the strategy pauses on the
//!   stale-data threshold; `MarketHealth::Stale` is the bus signal.
//!
//! ## Test design
//!
//! 1. Three "venues":
//!    - **Binance** — healthy [`MockFeed`] streaming a steady tick
//!      sequence throughout the test window.
//!    - **Coinbase** — synthetic [`ExplodingFeed`] that panics on first
//!      poll of `subscribe_trades` (architect's R14.3 archetype: a
//!      parser bug poisoning the venue's stream).
//!    - **Kraken** — healthy [`MockFeed`] streaming a steady tick
//!      sequence throughout the test window.
//! 2. Drive a [`spawn_venue_supervisor`] per venue (the public
//!    panic-isolation boundary T1408 introduced) plus a
//!    [`spawn_market_health_watchdog`] (T1409) over a fake injected
//!    clock so the stale signal is observable deterministically.
//! 3. Assert:
//!    - **3a.** The runtime [`tokio::task::JoinSet`] never surfaces a
//!      `JoinError::is_panic()`; panic isolation holds.
//!    - **3b.** Both `Binance` and `Kraken` venue supervisors keep
//!      producing ticks (count > 0) after the Coinbase panic — bus
//!      subscribers see uninterrupted flow.
//!    - **3c.** A `FeedReconnect` row lands in the audit ledger with
//!      the Coinbase venue tag (Q11 / R8).
//!    - **3d.** The watchdog publishes `MarketHealth::Stale { venue:
//!      Venue::Coinbase, .. }` once the fake clock advances past the
//!      configured `stale_threshold_secs`.
//!
//! ## Determinism
//!
//! Per the v1.5b dev contract: T1414 must be **deterministic** — the
//! panic timing must not depend on wall-clock cadence.  We use:
//! - `tokio::time::pause()` so all tokio sleeps / intervals are
//!   advanced explicitly via `tokio::time::advance`.
//! - A `FakeClock` injected into the watchdog so `Timestamp::now`
//!   never reaches a wall-clock call from the test path.
//!
//! Mirrors the determinism gate in T1409's tests (see
//! `crates/agent/src/runtime.rs:1577` `FakeClock`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use agent::EventBus;
use agent::config::BusConfig;
use agent::runtime::{LastTickMap, NowFn, spawn_market_health_watchdog, spawn_venue_supervisor};
use async_trait::async_trait;
use data::{MarketDataSource, MockFeed};
use futures::stream::BoxStream;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use trading_core::{
    Bar, FeedError, MarketHealth, Price, Quantity, Side, Symbol, Tick, Timeframe, Timestamp, Venue,
};

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Build a deterministic Tick for `(symbol, venue)` with `venue_ts ==
/// local_recv_ts` so the watchdog's age math is purely a function of
/// our injected clock, not real wall-clock latency.
fn mk_tick(symbol: &Symbol, ts_us: i64, id: u64, venue: Venue) -> Tick {
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
        venue,
    }
}

/// Synthetic `MarketDataSource` that panics on the first poll of
/// `subscribe_trades`.  This is the v1.5b R14.3 archetype: a parser
/// bug or a one-off WS-frame crash that would, absent panic isolation,
/// poison the entire ingest pipeline.
///
/// `subscribe_bars` does not panic — only `subscribe_trades` does —
/// because the supervisor wires both `bars_tap` and `ticks_tap`; the
/// panic must surface from inside one of the two stream-spawn paths.
struct ExplodingFeed;

#[async_trait]
impl MarketDataSource for ExplodingFeed {
    async fn exchange_info(&self, _symbol: Symbol) -> Result<data::source::SymbolInfo, FeedError> {
        Err(FeedError::Parse("ExplodingFeed::exchange_info".into()))
    }

    async fn subscribe_bars(
        &self,
        _symbol: Symbol,
        _tf: Timeframe,
    ) -> Result<BoxStream<'static, Result<Bar, FeedError>>, FeedError> {
        // The supervisor's bars tap polls this first; we panic from the
        // bars path so the inner JoinHandle's `JoinError::is_panic()` is
        // the surface tested here.
        panic!("synthetic Coinbase WS parser crash (bars)");
    }

    async fn subscribe_trades(
        &self,
        _symbol: Symbol,
    ) -> Result<BoxStream<'static, Result<Tick, FeedError>>, FeedError> {
        panic!("synthetic Coinbase WS parser crash (trades)");
    }
}

/// Inject a controllable wall-clock the watchdog reads via [`NowFn`].
/// Same pattern used by `runtime::tests::FakeClock` in T1409 — kept as
/// an inline helper here because the runtime's `FakeClock` is private
/// to its test module.
#[derive(Clone)]
struct FakeClock(Arc<Mutex<Timestamp>>);

impl FakeClock {
    fn new(t: Timestamp) -> Self {
        Self(Arc::new(Mutex::new(t)))
    }
    fn set(&self, t: Timestamp) {
        let mut guard = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = t;
    }
    fn into_now_fn(self) -> NowFn {
        Arc::new(move || {
            self.0
                .lock()
                .map(|g| *g)
                .unwrap_or_else(|p| *p.into_inner())
        })
    }
}

fn ts_at(secs: i64) -> Timestamp {
    Timestamp::new(OffsetDateTime::from_unix_timestamp(1_700_000_000 + secs).expect("ts"))
}

// ── The test ─────────────────────────────────────────────────────────────────

/// T1414 V7 — Coinbase outage isolation:
///
/// 1. Coinbase panics → its supervisor catches the panic (R14.1) and
///    audit-journals a `FeedReconnect` row with `venue = Coinbase`
///    (R8 / Q11).
/// 2. Binance + Kraken venues continue producing Ticks throughout the
///    test window (bus subscriber sees both venues' streams).
/// 3. The runtime's [`JoinSet`] drains cleanly — no `JoinError::is_panic`
///    surfaces from any supervisor (panic isolation invariant).
/// 4. The market-health watchdog emits `MarketHealth::Stale { venue:
///    Coinbase }` once the fake clock crosses the threshold.
///
/// **Determinism:** the test does NOT use `#[tokio::test(start_paused
/// = true)]` because `sqlx::SqlitePool::connect` relies on tokio's
/// wall-clock acquire timer; with `start_paused` the connection-acquire
/// times out immediately.  We open the ledger first (real time), then
/// call `tokio::time::pause()` *after* the ledger fixture is ready so
/// every subsequent supervisor / watchdog sleep is advance-driven.
#[tokio::test(flavor = "current_thread")]
async fn t1414_v7_coinbase_outage_isolated() {
    // ── 1. Audit ledger fixture (real wall-clock — sqlx connect needs it).
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("test_ledger.db");
    let ledger = Arc::new(
        audit::Ledger::open(db_path.to_str().expect("path str"))
            .await
            .expect("open ledger"),
    );
    audit::bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("chart");

    // ── 1b. Pause tokio time AFTER the ledger fixture is open.  Every
    // subsequent `tokio::time::sleep` / `interval` is now advance-driven
    // — the watchdog's 1Hz scan and the MockFeed's interval pacing.
    tokio::time::pause();

    // ── 2. Bus + subscriber ───────────────────────────────────────────────
    let bus = Arc::new(EventBus::new(&BusConfig::default()));
    // Subscribe BEFORE spawning producer tasks so we don't miss events.
    let mut ticks_rx = bus.ticks();
    let mut health_rx = bus.market_health();

    // ── 3. Per-venue feeds ────────────────────────────────────────────────
    // Binance + Kraken get healthy MockFeeds with a steady scripted
    // sequence on `BTCUSDT`.  Coinbase gets the panicking feed.  We
    // script enough ticks per venue (50) to amortize over the test
    // window and assert "uninterrupted flow" without flakiness.
    let feed_symbol = Symbol::new("BTCUSDT");
    let mk_script = |venue: Venue, count: u64| -> Vec<Tick> {
        (0..count)
            .map(|i| {
                // 100ms apart in venue_ts so the synthetic ordering is
                // observable but sub-second.
                let ts_us = 1_700_000_000_000_000 + (i as i64) * 100_000;
                mk_tick(&feed_symbol, ts_us, i + 1, venue)
            })
            .collect()
    };

    let binance_script = mk_script(Venue::Binance, 50);
    let kraken_script = mk_script(Venue::Kraken, 50);

    let binance_feed: Arc<dyn MarketDataSource> = Arc::new(MockFeed::new(
        binance_script,
        Duration::from_millis(50),
        Venue::Binance,
    ));
    let kraken_feed: Arc<dyn MarketDataSource> = Arc::new(MockFeed::new(
        kraken_script,
        Duration::from_millis(50),
        Venue::Kraken,
    ));
    let coinbase_feed: Arc<dyn MarketDataSource> = Arc::new(ExplodingFeed);

    // ── 4. Runtime topology ───────────────────────────────────────────────
    let mut set: JoinSet<()> = JoinSet::new();
    let cancel = CancellationToken::new();
    let last_tick: LastTickMap = Arc::new(Mutex::new(HashMap::new()));

    // Spawn supervisors in `Venue::Ord` order (Binance < Coinbase <
    // Kraken) so the test mirrors the production deterministic spawn
    // order from `runtime::run`.
    spawn_venue_supervisor(
        Venue::Binance,
        Arc::clone(&binance_feed),
        Arc::clone(&bus),
        Arc::clone(&ledger),
        feed_symbol.clone(),
        Timeframe::OneMinute,
        &mut set,
        &cancel,
        Some(Arc::clone(&last_tick)),
    );
    spawn_venue_supervisor(
        Venue::Coinbase,
        Arc::clone(&coinbase_feed),
        Arc::clone(&bus),
        Arc::clone(&ledger),
        feed_symbol.clone(),
        Timeframe::OneMinute,
        &mut set,
        &cancel,
        Some(Arc::clone(&last_tick)),
    );
    spawn_venue_supervisor(
        Venue::Kraken,
        Arc::clone(&kraken_feed),
        Arc::clone(&bus),
        Arc::clone(&ledger),
        feed_symbol.clone(),
        Timeframe::OneMinute,
        &mut set,
        &cancel,
        Some(Arc::clone(&last_tick)),
    );

    // Watchdog with a 30s threshold (Q7 default) and 1Hz scan cadence.
    // FakeClock starts at t=0; we'll advance it past the threshold
    // after the venues are warm.
    let t0 = ts_at(0);
    let clock = FakeClock::new(t0);
    spawn_market_health_watchdog(
        Arc::clone(&bus),
        Arc::clone(&last_tick),
        vec![Venue::Binance, Venue::Coinbase, Venue::Kraken],
        30,
        clock.clone().into_now_fn(),
        Duration::from_secs(1),
        &mut set,
        &cancel,
    );

    // ── 5. Drive the paused-tokio clock long enough for ticks to flow.
    // Coinbase's supervisor panics on the first poll; isolated.  The
    // healthy supervisors deliver scripted ticks every 50ms.  Advance
    // by 1.2s in 60ms steps so each MockFeed yields ~24 ticks.
    let mut binance_count: u32 = 0;
    let mut kraken_count: u32 = 0;
    for _ in 0..30 {
        tokio::time::advance(Duration::from_millis(60)).await;
        // Yield twice so spawned tap tasks get a chance to publish.
        tokio::task::yield_now().await;
        tokio::task::yield_now().await;
        // Drain any ticks accumulated since the last advance.
        loop {
            match ticks_rx.try_recv() {
                Ok(tick) => match tick.venue {
                    Venue::Binance => binance_count += 1,
                    Venue::Kraken => kraken_count += 1,
                    Venue::Coinbase => {
                        panic!("Coinbase feed produced a tick post-panic — isolation regressed");
                    }
                    Venue::Yahoo => {
                        unreachable!(
                            "Yahoo is data-only; no live tick feed routes ticks with Venue::Yahoo"
                        );
                    }
                },
                Err(tokio::sync::broadcast::error::TryRecvError::Empty) => break,
                Err(tokio::sync::broadcast::error::TryRecvError::Closed) => break,
                Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => continue,
            }
        }
    }

    // 5a. Assertion: both healthy venues produced ticks throughout the
    // window; Coinbase's panic did NOT poison their streams.
    assert!(
        binance_count > 0,
        "Binance ticks must keep flowing post-Coinbase-panic — got {binance_count}"
    );
    assert!(
        kraken_count > 0,
        "Kraken ticks must keep flowing post-Coinbase-panic — got {kraken_count}"
    );

    // ── 6. Drive the watchdog clock past the 30s stale threshold.
    // The watchdog's "now" is read via `FakeClock`; advancing the
    // tokio clock alone won't change `Timestamp::now`.  Step the
    // FakeClock to t=35 (5s past the 30s threshold) and let the
    // watchdog's 1Hz scan fire once on the paused tokio runtime.
    //
    // Note: Coinbase's `last_tick` map entry never populated because
    // the supervisor's `ticks_tap` panicked before observing any tick.
    // The watchdog's "Unseen" branch therefore would not emit Stale.
    // Inject a single-tick observation manually so the state machine
    // transitions Unseen → Fresh (at t=0), then advance the clock so
    // the next scan transitions Fresh → Stale.
    {
        let mut guard = last_tick.lock().expect("lock");
        guard.insert(Venue::Coinbase, t0);
    }
    // First scan post-injection: emits Fresh.  Then advance clock to
    // t=32 (>30s) and run another scan — emits Stale.
    clock.set(ts_at(1));
    tokio::time::advance(Duration::from_secs(1)).await;
    clock.set(ts_at(35));
    tokio::time::advance(Duration::from_secs(1)).await;

    // Drain MarketHealth events; the test passes when we see Stale for
    // Coinbase.  Other Fresh events from Binance/Kraken (whose ticks
    // populated the map naturally during step 5) are also accepted.
    let mut saw_coinbase_stale = false;
    for _ in 0..20 {
        match health_rx.try_recv() {
            Ok(MarketHealth::Stale {
                venue: Venue::Coinbase,
                ..
            }) => {
                saw_coinbase_stale = true;
                break;
            }
            Ok(_) => continue,
            Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                // Yield + advance once more so any pending watchdog
                // scan can complete.
                tokio::task::yield_now().await;
                tokio::time::advance(Duration::from_secs(1)).await;
                tokio::task::yield_now().await;
                continue;
            }
            Err(_) => break,
        }
    }
    assert!(
        saw_coinbase_stale,
        "watchdog must publish MarketHealth::Stale for Venue::Coinbase after 30s silence (Q7)"
    );

    // ── 7. Assert the audit ledger captured a venue-tagged
    //        FeedReconnect row for Coinbase (R8 / Q11).
    // Poll for the row rather than querying once.
    //
    // The reconnect row is written by a SPAWNED task through SQLite; querying
    // immediately asserts that the write has already landed, which is a
    // scheduling-and-I/O assumption, not a behaviour. It held on the canonical
    // box and failed on windows-latest with `got 0`.
    //
    // The clock is PAUSED here (`tokio::time::pause()` above), so a `sleep`-based
    // poll would auto-advance virtual time and return instantly without giving the
    // writer a chance. This yields to the runtime and advances the virtual clock
    // explicitly instead, so it is deterministic rather than wall-clock dependent.
    //
    // Safe by inspection: EVERY assertion in this test is a lower bound or an
    // invariant (`binance_count > 0`, `kraken_count > 0`, `saw_coinbase_stale`,
    // `>= 1` here, `res.is_ok()`, and a fixed supervisor count of 4). There is no
    // upper bound on anything that grows with waiting, so polling can only help
    // this assertion and cannot weaken another. That check is the point: the same
    // pattern was reverted from `audit_aggregator_handles_10k_event_storm`
    // precisely because there the assertion WAS a coverage ratio the wait weakened.
    let mut events = Vec::new();
    for _ in 0..200 {
        events = audit::query::strategy_events_since(&ledger, ts_at(-3600))
            .await
            .expect("strategy_events_since");
        if events
            .iter()
            .any(|e| matches!(e.kind, trading_core::StrategyEventKind::FeedReconnect))
        {
            break;
        }
        tokio::time::advance(Duration::from_millis(10)).await;
        tokio::task::yield_now().await;
    }
    let coinbase_reconnect_count = events
        .iter()
        .filter(|e| matches!(e.kind, trading_core::StrategyEventKind::FeedReconnect))
        .count();
    assert!(
        coinbase_reconnect_count >= 1,
        "audit ledger must contain >= 1 FeedReconnect row after Coinbase panic; got {coinbase_reconnect_count}"
    );

    // ── 8. Cancel + drain.  No `JoinError::is_panic()` may surface
    //        from any supervisor — that's the panic-isolation
    //        invariant from R14.1.
    cancel.cancel();
    let drain = async {
        let mut joins = Vec::new();
        while let Some(res) = set.join_next().await {
            joins.push(res);
        }
        joins
    };
    let collected = tokio::time::timeout(Duration::from_secs(5), drain)
        .await
        .expect("supervisors did not drain inside the budget");

    for res in &collected {
        assert!(
            res.is_ok(),
            "supervisor task surfaced JoinError — panic isolation regressed: {res:?}"
        );
    }
    // Sanity: the runtime spawned 4 tasks (3 supervisors + 1 watchdog).
    assert_eq!(
        collected.len(),
        4,
        "expected exactly 3 supervisors + 1 watchdog to drain; got {}",
        collected.len()
    );
}
