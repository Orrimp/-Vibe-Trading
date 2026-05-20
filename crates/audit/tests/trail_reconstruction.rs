#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T-D-N25 — `audit::query::trail_for_fill_id` integration test.
//!
//! Acceptance gate per tasks.md T-D-N25:
//! - Write a `Signal + Fill + ForecastEvent` row triplet to a fresh
//!   in-memory ledger.
//! - Call `trail_for_fill_id(ledger, fill_audit_id)`.
//! - Assert all three stages are populated; `debate == None`.
//! - Fill-only scenario (no `signal_id` link) returns `fill` populated,
//!   `signal == None`, `forecast == None`.
//! - Non-existent audit_id returns default (all None).

use audit::Ledger;
use audit::bootstrap;
use audit::journal::{post_fill, post_fill_with_signal, post_forecast_event, post_strategy_signal};
use audit::query::trail_for_fill_id;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{
    Direction, FeeTier, FillId, ForecastOverlay, Liquidity, OrderId, Price, Quantity, Side, Signal,
    SignalEvidence, SignalKind, StrategyId, Symbol, Timestamp, Venue,
};
use uuid::Uuid;

fn ts_epoch() -> Timestamp {
    Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(1_000_000))
}

async fn seeded_ledger() -> Ledger {
    let l = Ledger::in_memory().await.expect("in-memory ledger");
    bootstrap::chart_of_accounts(&l)
        .await
        .expect("bootstrap chart of accounts");
    l
}

fn make_fill(symbol: &str) -> trading_core::Fill {
    let ts = ts_epoch();
    trading_core::Fill {
        id: FillId::new(),
        order_id: OrderId::new(),
        symbol: Symbol::new(symbol),
        side: Side::Buy,
        price: Price::new(dec!(50_000)).unwrap(),
        qty: Quantity::new(dec!(0.01)).unwrap(),
        fee: trading_core::Money::from_decimal(dec!(0.1)),
        fee_tier: FeeTier::Taker,
        venue_ts: ts,
        local_ts: ts,
        liquidity: Liquidity::Taker,
        transaction_id: None,
    }
}

fn make_signal(symbol: &str, strategy: &str) -> Signal {
    Signal {
        strategy_id: StrategyId::new(strategy),
        symbol: Symbol::new(symbol),
        ts: ts_epoch(),
        kind: SignalKind::Buy,
        evidence: SignalEvidence::empty(),
        pair_data: None,
    }
}

fn make_overlay() -> ForecastOverlay {
    ForecastOverlay {
        correlation_id: Uuid::new_v4(),
        confidence: dec!(0.75),
        direction: Direction::Up,
        horizon_bars: 1,
        model_revision: "d1c3696d".to_string(),
        sampled_at: OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(1_000_000),
    }
}

/// T-D-N25 (a) — Full triplet: Signal + Fill + Forecast.
/// `trail_for_fill_id` returns all three stages populated; `debate == None`.
#[tokio::test]
async fn trail_full_triplet_returns_all_three_stages() {
    let ledger = seeded_ledger().await;

    let overlay = make_overlay();

    // 1. Write strategy signal linked to the forecast correlation_id.
    let signal = make_signal("BTCUSDT", "tcn_overlay_momentum");
    let intended_qty = Quantity::new(dec!(0.01)).unwrap();
    let signal_row_id_smolstr = post_strategy_signal(
        &ledger,
        &signal,
        intended_qty,
        None, // market order
        Venue::Binance,
        false,
        None,
        Some(overlay.correlation_id), // Phase D link
    )
    .await
    .expect("post_strategy_signal");
    let signal_row_id = signal_row_id_smolstr.as_str();
    assert!(
        !signal_row_id.is_empty(),
        "signal row id must be non-empty for Buy signal"
    );

    // 2. Write the fill linked to the signal row.
    let fill = make_fill("BTCUSDT");
    let fill_tx_id = post_fill_with_signal(
        &ledger,
        &fill,
        Venue::Binance,
        Some("tcn_overlay_momentum"),
        Some(signal_row_id),
    )
    .await
    .expect("post_fill_with_signal");
    let fill_audit_id = fill_tx_id.as_str().to_string();

    // 3. Write the forecast event.
    post_forecast_event(&ledger, &overlay, "tcn_overlay_momentum", "BTCUSDT", false)
        .await
        .expect("post_forecast_event");

    // 4. Reconstruct the trail.
    let trail = trail_for_fill_id(&ledger, &fill_audit_id)
        .await
        .expect("trail_for_fill_id");

    // Fill stage populated.
    let fill_row = trail.fill.expect("fill stage must be populated");
    assert_eq!(fill_row.id, fill_audit_id);
    assert!(
        fill_row.signal_id.is_some(),
        "signal_id must be linked on the fill row"
    );
    assert_eq!(
        fill_row.signal_id.as_deref(),
        Some(signal_row_id),
        "signal_id must match the written signal row id"
    );

    // Signal stage populated.
    let signal_row = trail.signal.expect("signal stage must be populated");
    assert_eq!(signal_row.id, signal_row_id);
    assert_eq!(signal_row.side.to_lowercase(), "buy", "side must be buy");
    assert!(
        signal_row.forecast_correlation_id.is_some(),
        "forecast_correlation_id must be linked"
    );
    assert_eq!(
        signal_row.forecast_correlation_id.as_deref(),
        Some(overlay.correlation_id.to_string().as_str()),
        "forecast_correlation_id must match overlay correlation_id"
    );

    // Forecast stage populated.
    let forecast_row = trail.forecast.expect("forecast stage must be populated");
    assert_eq!(forecast_row.direction, "up", "direction must match");
    assert_eq!(
        forecast_row.model_revision, "d1c3696d",
        "model_revision must match"
    );
    assert!(!forecast_row.cache_hit, "cache_hit must be false");

    // Debate always None at v0.1.0 (R1.5).
    assert!(trail.debate.is_none(), "debate must be None at v0.1.0");
}

/// T-D-N25 (b) — Fill only (no signal_id link → pre-mig-011 scenario).
/// `trail_for_fill_id` returns `fill` populated, `signal == None`.
#[tokio::test]
async fn trail_fill_only_returns_fill_and_nones() {
    let ledger = seeded_ledger().await;

    // Write a fill with no signal link (thin post_fill wrapper).
    let fill = make_fill("ETHUSDT");
    let fill_tx_id = post_fill(&ledger, &fill, Venue::Binance, Some("sma_crossover"))
        .await
        .expect("post_fill");
    let fill_audit_id = fill_tx_id.as_str().to_string();

    let trail = trail_for_fill_id(&ledger, &fill_audit_id)
        .await
        .expect("trail_for_fill_id");

    let fill_row = trail.fill.expect("fill stage must be populated");
    assert_eq!(fill_row.id, fill_audit_id);
    assert!(
        fill_row.signal_id.is_none(),
        "pre-mig-011 fill must have no signal_id link"
    );

    assert!(trail.signal.is_none(), "signal stage must be None");
    assert!(trail.forecast.is_none(), "forecast stage must be None");
    assert!(trail.debate.is_none(), "debate must be None");
}

/// T-D-N25 (c) — Non-existent audit_id returns default (all None).
#[tokio::test]
async fn trail_missing_fill_returns_default() {
    let ledger = seeded_ledger().await;

    let trail = trail_for_fill_id(&ledger, "does-not-exist-uuid")
        .await
        .expect("trail_for_fill_id must not error on missing id");

    assert!(trail.fill.is_none(), "fill must be None for missing id");
    assert!(trail.signal.is_none());
    assert!(trail.forecast.is_none());
}
