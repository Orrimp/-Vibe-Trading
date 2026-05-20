//! K5 / R2.5 — variant coverage test.
//!
//! For each non-delegating in-scope writer, wires a 64-capacity
//! `Ledger::open_with_tick_bus(":memory:", 64)`, calls `.subscribe()`,
//! drives the writer once with synthetic arguments, and asserts the next
//! tick on the subscriber carries the expected variant + non-default
//! `context.run_id` (when `with_run_id` was set).
//!
//! Delegating writers (`feed_reconnect`, `rebalance_rejected`,
//! `mean_reversion_stop`, `pair_short_observation`) are covered via the
//! `StrategyEvent { kind = … }` shape through `strategy_event`.

use audit::{bootstrap, journal, tick::AuditEvent};
use rust_decimal_macros::dec;
use trading_core::{
    Decision, FeeTier, Fill, FillId, Liquidity, Money, OrderId, Price, Quantity, Side, Signal,
    SignalEvidence, SignalKind, StrategyId, Symbol, Timestamp, Venue,
};
use uuid::Uuid;

async fn open_bus_ledger() -> (
    audit::Ledger,
    tokio::sync::broadcast::Sender<audit::tick::AuditTick<AuditEvent>>,
) {
    let (ledger, sender) = audit::Ledger::open_with_tick_bus(":memory:", 64)
        .await
        .expect("open with tick bus");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap");
    (ledger, sender)
}

fn make_fill() -> Fill {
    Fill {
        id: FillId::new(),
        order_id: OrderId::new(),
        symbol: Symbol::new("BTCUSDT"),
        side: Side::Buy,
        qty: Quantity::new(dec!(0.1)).expect("qty"),
        price: Price::new(dec!(50000)).expect("price"),
        fee: Money::from_decimal(dec!(2.0)),
        fee_tier: FeeTier::Taker,
        venue_ts: Timestamp::now(),
        local_ts: Timestamp::now(),
        liquidity: Liquidity::Taker,
        transaction_id: None,
    }
}

fn make_signal() -> Signal {
    Signal {
        strategy_id: StrategyId(smol_str::SmolStr::new("test")),
        symbol: Symbol::new("BTCUSDT"),
        ts: Timestamp::now(),
        kind: SignalKind::Buy,
        evidence: SignalEvidence::empty(),
        pair_data: None,
    }
}

// ── Row 1 — post_fill → AuditEvent::Fill ─────────────────────────────────────

#[tokio::test]
async fn post_fill_emits_fill_variant() {
    let run_id = Uuid::new_v4();
    let (base_ledger, sender) = open_bus_ledger().await;
    let ledger = base_ledger.with_run_id(run_id);
    let mut rx = audit::tick::AuditTickStream::new(sender.subscribe(), "test");

    let fill = make_fill();
    journal::post_fill(&ledger, &fill, Venue::Binance, None)
        .await
        .expect("post_fill");

    let tick = rx.next().await.expect("tick");
    assert!(
        matches!(tick.event, AuditEvent::Fill { .. }),
        "expected Fill variant, got: {:?}",
        tick.event
    );
    assert_eq!(tick.context.run_id, run_id, "run_id mismatch");
}

// ── Row 2 — post_strategy_signal → AuditEvent::StrategySignal ────────────────

#[tokio::test]
async fn post_strategy_signal_emits_strategy_signal_variant() {
    let run_id = Uuid::new_v4();
    let (base_ledger, sender) = open_bus_ledger().await;
    let ledger = base_ledger.with_run_id(run_id);
    let mut rx = audit::tick::AuditTickStream::new(sender.subscribe(), "test");

    let signal = make_signal();
    journal::post_strategy_signal(
        &ledger,
        &signal,
        trading_core::Quantity::new(dec!(0.1)).expect("qty"),
        None,
        Venue::Binance,
        false,
        None,
        None, // forecast_correlation_id (Phase D R1.3)
    )
    .await
    .expect("post_strategy_signal");

    let tick = rx.next().await.expect("tick");
    assert!(
        matches!(tick.event, AuditEvent::StrategySignal { .. }),
        "expected StrategySignal variant"
    );
    assert_eq!(tick.context.run_id, run_id);
}

// ── Row 3 — kill_switch_tripped → AuditEvent::KillSwitchTripped ──────────────

#[tokio::test]
async fn kill_switch_tripped_emits_kill_switch_variant() {
    let run_id = Uuid::new_v4();
    let (base_ledger, sender) = open_bus_ledger().await;
    let ledger = base_ledger.with_run_id(run_id);
    let mut rx = audit::tick::AuditTickStream::new(sender.subscribe(), "test");

    journal::kill_switch_tripped(&ledger, "daily_loss_cap", "test_operator")
        .await
        .expect("kill_switch_tripped");

    let tick = rx.next().await.expect("tick");
    assert!(
        matches!(tick.event, AuditEvent::KillSwitchTripped { .. }),
        "expected KillSwitchTripped variant"
    );
    assert_eq!(tick.context.run_id, run_id);
}

// ── Row 4 — strategy_event → AuditEvent::StrategyEvent ───────────────────────

#[tokio::test]
async fn strategy_event_emits_strategy_event_variant() {
    let run_id = Uuid::new_v4();
    let (base_ledger, sender) = open_bus_ledger().await;
    let ledger = base_ledger.with_run_id(run_id);
    let mut rx = audit::tick::AuditTickStream::new(sender.subscribe(), "test");

    let write = journal::StrategyEventWrite {
        kind: "Load",
        strategy_id: Some("test_strategy"),
        old_hash: None,
        new_hash: None,
        source_path: "config/strategies/test.toml",
        operator: "system",
        error_code: None,
        error_summary: None,
        ts: None,
        venue: None,
    };
    journal::strategy_event(&ledger, &write)
        .await
        .expect("strategy_event");

    let tick = rx.next().await.expect("tick");
    assert!(
        matches!(tick.event, AuditEvent::StrategyEvent { .. }),
        "expected StrategyEvent variant"
    );
    assert_eq!(tick.context.run_id, run_id);
}

// ── Row 9 — open_uptime_interval → AuditEvent::UptimeIntervalOpened ─────────

#[tokio::test]
async fn open_uptime_interval_emits_uptime_opened_variant() {
    let run_id = Uuid::new_v4();
    let (base_ledger, sender) = open_bus_ledger().await;
    let ledger = base_ledger.with_run_id(run_id);
    let mut rx = audit::tick::AuditTickStream::new(sender.subscribe(), "test");

    journal::open_uptime_interval(&ledger, "boot-001", None)
        .await
        .expect("open_uptime_interval");

    let tick = rx.next().await.expect("tick");
    assert!(
        matches!(tick.event, AuditEvent::UptimeIntervalOpened { .. }),
        "expected UptimeIntervalOpened variant"
    );
    assert_eq!(tick.context.run_id, run_id);
}

// ── Row 10 — close_uptime_interval → AuditEvent::UptimeIntervalClosed ────────

#[tokio::test]
async fn close_uptime_interval_emits_uptime_closed_variant() {
    let run_id = Uuid::new_v4();
    let (base_ledger, sender) = open_bus_ledger().await;
    let ledger = base_ledger.with_run_id(run_id);
    let mut rx = audit::tick::AuditTickStream::new(sender.subscribe(), "test");

    // Open first so close has a row to read.
    journal::open_uptime_interval(&ledger, "boot-002", None)
        .await
        .expect("open");
    // Drain the open tick.
    let _ = rx.next().await;

    journal::close_uptime_interval(&ledger, "boot-002", None)
        .await
        .expect("close");

    let tick = rx.next().await.expect("tick");
    assert!(
        matches!(tick.event, AuditEvent::UptimeIntervalClosed { .. }),
        "expected UptimeIntervalClosed variant, got: {:?}",
        tick.event
    );
    assert_eq!(tick.context.run_id, run_id);
}

// ── Hold-signal fast-return: no tick emitted ─────────────────────────────────

#[tokio::test]
async fn post_strategy_signal_hold_emits_no_tick() {
    let (ledger, sender) = open_bus_ledger().await;
    let mut rx = sender.subscribe();

    let hold_signal = Signal {
        strategy_id: StrategyId(smol_str::SmolStr::new("test")),
        symbol: Symbol::new("BTCUSDT"),
        ts: Timestamp::now(),
        kind: SignalKind::Hold,
        evidence: SignalEvidence::empty(),
        pair_data: None,
    };
    journal::post_strategy_signal(
        &ledger,
        &hold_signal,
        trading_core::Quantity::new(dec!(0.1)).expect("qty"),
        None,
        Venue::Binance,
        false,
        None,
        None, // forecast_correlation_id (Phase D R1.3)
    )
    .await
    .expect("post_strategy_signal hold");

    // No tick should be in the channel.
    assert!(rx.try_recv().is_err(), "Hold signal should not emit a tick");
}
