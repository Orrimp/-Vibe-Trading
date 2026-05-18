//! T911 — Live-bus regression test (live-cockpit-unified).
//!
//! Risk #2 in the architect's design:
//!
//! > Cockpit panel drift when bus is fully wired (existing
//! > `crates/ui/tests/live_subscription.rs` 53 tests assume specific
//! > channel shapes; new producers may emit at higher rates than tests
//! > expect).
//!
//! Mitigation per the design: "regression-run `cargo test -p ui
//! --features live` is in V4 + per-task acceptance.  Add a single new
//! test `crates/ui/tests/live_subscription_full_bus.rs` that drives a
//! fully populated bus and asserts no panel exceeds its `Loading`
//! window past the first event."
//!
//! Acceptance per the task spec: drives 100 fills, 50 positions, 20
//! bars, 200 ticks, 5 PnL snapshots, 1 mode transition; asserts every
//! cockpit panel transitions out of `Loading` after the first relevant
//! event, without any panic or unexpected `Closed` state.
//!
//! Plus the V2b round-trip check: a kill-switch trip via the agent
//! `KillSwitch::trip` path goes through the `T905` mode-broadcast
//! forwarder that `agent::runtime::run` spawns, the cockpit's mode
//! stream observes it, and the cockpit's `kill` panel transitions to
//! `KillState::Halted`.  Uses `Arc<KillSwitch>` + `MockIncidentSpawner`
//! plus an in-memory ledger; the T905 forwarder is spawned on the
//! test's tokio runtime.

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;

use agent::EventBus;
use agent::config::BusConfig;
use futures::StreamExt;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use trading_core::{
    Bar, FeeTier, Fill, FillId, FundingObs, Liquidity, Money, OrderId, PnlSnapshot, Position,
    Price, Quantity, Side, Symbol, Tick, Timeframe, Timestamp, Venue,
};

use ui::live::{
    stream_bars, stream_fills, stream_mode, stream_pnl, stream_positions, stream_ticks,
};
use ui::state::{AgentMode, Cockpit, KillState, Latency, PanelState, update};

// ── Synthetic event factories ────────────────────────────────────────────────

fn ts(offset_secs: i64) -> Timestamp {
    Timestamp::new(
        time::OffsetDateTime::from_unix_timestamp(1_700_000_000 + offset_secs).expect("valid ts"),
    )
}

fn synthetic_fill(n: i64) -> Fill {
    Fill {
        id: FillId::new(),
        order_id: OrderId::new(),
        symbol: Symbol::new("BTCUSDT"),
        side: if n % 2 == 0 { Side::Buy } else { Side::Sell },
        qty: Quantity::new(dec!(0.1)).unwrap(),
        price: Price::new(dec!(40_000) + rust_decimal::Decimal::from(n)).unwrap(),
        fee: Money::from_decimal(dec!(1.6)),
        fee_tier: FeeTier::Taker,
        venue_ts: ts(n),
        local_ts: ts(n),
        liquidity: Liquidity::Taker,
        transaction_id: None,
    }
}

fn synthetic_position(n: i64) -> Position {
    Position {
        symbol: Symbol::new(format!("SYM{n}")),
        base_qty: rust_decimal::Decimal::from(n),
        cost_basis: Money::from_decimal(rust_decimal::Decimal::from(10_000 + n)),
        last_mark: Price::new(dec!(40_000) + rust_decimal::Decimal::from(n)).unwrap(),
        realized_pnl: Money::from_decimal(dec!(5)),
        unrealized_pnl: Money::from_decimal(dec!(7.5)),
    }
}

fn synthetic_bar(n: i64) -> Bar {
    let close = dec!(50_000) + rust_decimal::Decimal::from(n);
    Bar {
        symbol: Symbol::new("BTCUSDT"),
        tf: Timeframe::OneMinute,
        open_ts: ts(n * 60),
        close_ts: ts(n * 60 + 60),
        open: Price::new(close).unwrap(),
        high: Price::new(close).unwrap(),
        low: Price::new(close).unwrap(),
        close: Price::new(close).unwrap(),
        volume: Quantity::new(dec!(1)).unwrap(),
        trade_count: 1,
        local_recv_ts: ts(n * 60 + 60),
        venue: Venue::Binance,
    }
}

fn synthetic_tick(n: i64) -> Tick {
    Tick {
        symbol: Symbol::new("BTCUSDT"),
        venue_ts: ts(n),
        local_recv_ts: ts(n),
        price: Price::new(dec!(40_000) + rust_decimal::Decimal::from(n)).unwrap(),
        qty: Quantity::new(dec!(1)).unwrap(),
        side: Side::Buy,
        trade_id: u64::try_from(n).unwrap_or(0),
        venue: Venue::Binance,
    }
}

fn synthetic_pnl(n: i64) -> PnlSnapshot {
    PnlSnapshot {
        cash: Money::from_decimal(dec!(90_000) + rust_decimal::Decimal::from(n)),
        unrealized: Money::from_decimal(dec!(250)),
        realized: Money::from_decimal(dec!(-120.50)),
        total_equity: Money::from_decimal(dec!(90_129.50) + rust_decimal::Decimal::from(n)),
        daily_return: Money::from_decimal(dec!(129.50)),
        as_of: ts(n),
    }
}

// Silence unused-warning for FundingObs — we never use it but the import keeps
// trading_core's surface aligned with this test's expectations.
#[allow(dead_code)]
fn _unused_funding_obs_anchor() -> Option<FundingObs> {
    None
}

/// T911 acceptance — drive a fully-populated bus through the entire
/// six-channel taxonomy and assert every cockpit panel transitions out
/// of `Loading` after the first relevant event.  Volumes per the
/// architect's task spec: 100 fills, 50 positions, 20 bars, 200 ticks,
/// 5 pnl, 1 mode transition.
#[tokio::test(flavor = "current_thread")]
async fn t911_full_bus_drives_every_panel_out_of_loading() {
    let bus = Arc::new(EventBus::new(&BusConfig::default()));

    // Subscribe FIRST so no events are dropped.
    let mut fills = Box::pin(stream_fills(&bus));
    let mut positions = Box::pin(stream_positions(&bus));
    let mut bars = Box::pin(stream_bars(&bus));
    let mut ticks = Box::pin(stream_ticks(&bus));
    let mut pnl = Box::pin(stream_pnl(&bus));
    let mut mode = Box::pin(stream_mode(&bus));

    tokio::task::yield_now().await;

    // Publish in interleaved order to mirror real-world bus traffic.
    for i in 0..100i64 {
        bus.publish_fill(synthetic_fill(i));
    }
    for i in 0..50i64 {
        bus.publish_position(synthetic_position(i));
    }
    for i in 0..20i64 {
        bus.publish_bar(synthetic_bar(i));
    }
    for i in 0..200i64 {
        bus.publish_tick(synthetic_tick(i));
    }
    for i in 0..5i64 {
        bus.publish_pnl(synthetic_pnl(i));
    }
    bus.publish_mode(agent::AgentMode::Halted {
        reason: "test halt".into(),
    });

    let mut cockpit = Cockpit::new();

    // Drain every stream's first message into the cockpit.  `Loading`
    // counts every panel before any event lands; one event per channel
    // is enough to flip the panel out of `Loading`.
    let drain = async {
        let m = fills.next().await.expect("fills closed");
        update(&mut cockpit, m);
        let m = positions.next().await.expect("positions closed");
        update(&mut cockpit, m);
        let m = bars.next().await.expect("bars closed");
        update(&mut cockpit, m);
        // bars stream emits BarReceived followed by BarClose for each bar
        let m = bars.next().await.expect("bars closed");
        update(&mut cockpit, m);
        let m = ticks.next().await.expect("ticks closed");
        update(&mut cockpit, m);
        let m = pnl.next().await.expect("pnl closed");
        update(&mut cockpit, m);
        let m = mode.next().await.expect("mode closed");
        update(&mut cockpit, m);
    };

    timeout(Duration::from_secs(2), drain)
        .await
        .expect("not all channels delivered an event inside 2 s");

    // ── Assertions: every panel transitions out of Loading ───────────
    //
    // Tape: at least one fill renders.
    match &cockpit.tape {
        PanelState::Loading => panic!("tape still Loading after fills delivered"),
        PanelState::Error(e) => panic!("tape errored: {e}"),
        _ => {}
    }
    // Positions panel: at least one row.
    match &cockpit.positions {
        PanelState::Loading => panic!("positions still Loading after positions delivered"),
        PanelState::Error(e) => panic!("positions errored: {e}"),
        _ => {}
    }
    // P&L panel: snapshot present.
    match &cockpit.pnl {
        PanelState::Loading => panic!("pnl still Loading after pnl delivered"),
        PanelState::Error(e) => panic!("pnl errored: {e}"),
        _ => {}
    }
    // Latency badge transitions Known after first tick.
    assert!(
        matches!(cockpit.latency, Latency::Known { .. }),
        "latency badge stayed Unknown after ticks delivered"
    );
    // Bar timestamp populates after first BarReceived.
    assert!(
        cockpit.last_bar_ts.is_some(),
        "last_bar_ts stayed None after bars delivered"
    );
    // Mode flips Halted after the trip event.
    assert_eq!(cockpit.mode, AgentMode::Halted);
    match &cockpit.kill {
        KillState::Halted { reason } => {
            assert_eq!(reason.as_str(), "test halt");
        }
        other => panic!("kill state not Halted: {other:?}"),
    }
}

/// T911 V2b round-trip — Cockpit-button trip path:
///
/// `KillSwitch::trip(HaltReason::ManualOperator)` →
/// `kill_switch.subscribe()` channel →
/// `agent::runtime::spawn_mode_forwarder` (T905) →
/// `bus.publish_mode(...)` →
/// `ui::live::stream_mode` →
/// `Message::AgentHaltedExternally(reason)` →
/// `Cockpit::kill = KillState::Halted`.
///
/// Uses `Arc<KillSwitch>` constructed via `KillSwitch::with_audit`
/// against an in-memory ledger + `MockIncidentSpawner` so the T809
/// dual-write hooks fire exactly as they do in production.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn t911_kill_button_round_trip_via_mode_forwarder() {
    use agent::{HaltReason, IncidentSpawner, KillSwitch, MockIncidentSpawner};
    use audit::{Ledger, bootstrap};

    let ledger = Arc::new(Ledger::in_memory().await.expect("in-memory"));
    bootstrap::chart_of_accounts(&ledger).await.expect("chart");

    let spawner: Arc<dyn IncidentSpawner> = Arc::new(MockIncidentSpawner::new());
    // Use a unique path under the OS temp dir — no `tempfile` dev-dep
    // needed.  The halt-file watcher is NOT spawned here (we never call
    // `spawn_halt_file_watcher`), so the path's existence is
    // immaterial; it's just stored on the `KillSwitch` value.
    let halt_file =
        std::env::temp_dir().join(format!("t911-halt-{}", uuid::Uuid::new_v4().simple()));
    let kill_switch = Arc::new(KillSwitch::with_audit(
        &halt_file,
        32,
        Arc::clone(&ledger),
        spawner,
    ));

    let bus = Arc::new(EventBus::new(&BusConfig::default()));

    // Subscribe to the cockpit-side stream BEFORE the forwarder starts
    // so we capture every event.
    let mut mode_stream = Box::pin(stream_mode(&bus));

    // Spawn the T905 forwarder via the public API.  This is exactly what
    // `agent::runtime::run` does on its JoinSet.
    let mut set = tokio::task::JoinSet::new();
    let cancel = CancellationToken::new();
    agent::runtime::spawn_mode_forwarder(
        Arc::clone(&kill_switch),
        Arc::clone(&bus),
        &mut set,
        &cancel,
    );

    // Yield twice so the forwarder's subscribe lands before the trip.
    tokio::task::yield_now().await;
    tokio::time::sleep(Duration::from_millis(20)).await;

    // ── The cockpit-button trip ──────────────────────────────────────
    kill_switch.trip(HaltReason::ManualOperator);

    // The cockpit observes the mode change inside 1 s.
    let msg = timeout(Duration::from_secs(1), mode_stream.next())
        .await
        .expect("mode message did not arrive inside 1 s")
        .expect("mode stream closed");

    let mut cockpit = Cockpit::new();
    update(&mut cockpit, msg);

    // Halted banner present + kill panel reflects the manual reason.
    assert_eq!(cockpit.mode, AgentMode::Halted);
    match &cockpit.kill {
        KillState::Halted { reason } => {
            // The exact reason text is the `HaltReason::ManualOperator`
            // Display impl ("manual_operator").  We assert non-empty so
            // a future text tweak doesn't break this test for a
            // cosmetic reason.
            assert!(
                !reason.is_empty(),
                "kill panel reason text is empty after trip"
            );
        }
        other => panic!("kill state not Halted after trip: {other:?}"),
    }

    // Sticky-trip — a second trip is a no-op on the kill switch, so no
    // duplicate event reaches the bus.  Confirm by polling the stream
    // briefly and asserting timeout (no message arrives).
    kill_switch.trip(HaltReason::ManualOperator);
    let none = timeout(Duration::from_millis(150), mode_stream.next()).await;
    assert!(
        none.is_err(),
        "sticky-trip violated: forwarder leaked a second mode event into the bus"
    );

    // Drain the forwarder cleanly.
    cancel.cancel();
    let drain = async { while set.join_next().await.is_some() {} };
    timeout(Duration::from_secs(2), drain)
        .await
        .expect("forwarder did not drain inside 2 s");

    // Silence unused borrows.
    let _ = SmolStr::new("");
    let _ = synthetic_fill(0);
}
