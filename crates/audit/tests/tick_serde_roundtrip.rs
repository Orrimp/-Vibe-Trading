//! R1.1 / R1.3 — serde round-trip for `AuditTick<AuditEvent>`.
//!
//! For each `AuditEvent` variant: `serde_json::to_string` → `from_str` →
//! asserts bit-identical `Debug` output. Guards `#[non_exhaustive]` against
//! accidental field reorders under future derives.

use audit::tick::{AuditContext, AuditEvent, AuditTick};
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use time::OffsetDateTime;
use trading_core::{
    Direction, FeeTier, Fill, FillId, ForecastOverlay, Liquidity, Money, OrderId, Price, Quantity,
    Side, Signal, SignalEvidence, SignalKind, StrategyId, Symbol, Timestamp, Venue,
};
use uuid::Uuid;

fn make_context() -> AuditContext {
    AuditContext {
        run_id: Uuid::nil(),
        posted_at: OffsetDateTime::UNIX_EPOCH,
        agent_pid: 42,
    }
}

fn make_tick(event: AuditEvent) -> AuditTick<AuditEvent> {
    AuditTick {
        event,
        context: make_context(),
    }
}

fn roundtrip(tick: AuditTick<AuditEvent>) {
    let json = serde_json::to_string(&tick).expect("serialize");
    let back: AuditTick<AuditEvent> = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(
        format!("{:?}", tick.event),
        format!("{:?}", back.event),
        "round-trip Debug mismatch for variant"
    );
    assert_eq!(
        tick.context.run_id, back.context.run_id,
        "context.run_id mismatch"
    );
    assert_eq!(
        tick.context.agent_pid, back.context.agent_pid,
        "context.agent_pid mismatch"
    );
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
        strategy_id: StrategyId(SmolStr::new("test")),
        symbol: Symbol::new("BTCUSDT"),
        ts: Timestamp::now(),
        kind: SignalKind::Buy,
        evidence: SignalEvidence::empty(),
        pair_data: None,
    }
}

fn make_overlay() -> ForecastOverlay {
    ForecastOverlay {
        correlation_id: Uuid::nil(),
        confidence: dec!(0.75),
        direction: Direction::Up,
        horizon_bars: 1,
        model_revision: "test-rev".to_string(),
        sampled_at: OffsetDateTime::UNIX_EPOCH,
    }
}

#[test]
fn fill_roundtrip() {
    roundtrip(make_tick(AuditEvent::Fill {
        fill: Box::new(make_fill()),
        fees: dec!(2.0),
    }));
}

#[test]
fn strategy_signal_roundtrip() {
    roundtrip(make_tick(AuditEvent::StrategySignal {
        strategy_id: StrategyId(SmolStr::new("test")),
        signal: Box::new(make_signal()),
    }));
}

#[test]
fn strategy_event_roundtrip() {
    roundtrip(make_tick(AuditEvent::StrategyEvent {
        kind: SmolStr::new("Load"),
        payload_json: "{}".to_string(),
    }));
}

#[test]
fn forecast_emitted_roundtrip() {
    roundtrip(make_tick(AuditEvent::ForecastEmitted {
        overlay: make_overlay(),
        cache_hit: true,
    }));
}

#[test]
fn kill_switch_tripped_roundtrip() {
    roundtrip(make_tick(AuditEvent::KillSwitchTripped {
        reason: SmolStr::new("daily_loss_cap"),
    }));
}

#[test]
fn feed_reconnect_roundtrip() {
    roundtrip(make_tick(AuditEvent::FeedReconnect {
        venue: Venue::Binance,
        symbol: Symbol::new("BTCUSDT"),
        gap_ms: 1234,
    }));
}

#[test]
fn uptime_interval_opened_roundtrip() {
    roundtrip(make_tick(AuditEvent::UptimeIntervalOpened {
        run_id: Uuid::nil(),
    }));
}

#[test]
fn uptime_interval_closed_roundtrip() {
    roundtrip(make_tick(AuditEvent::UptimeIntervalClosed {
        run_id: Uuid::nil(),
        duration_s: 3600,
    }));
}
