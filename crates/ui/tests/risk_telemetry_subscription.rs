//! T1707 — RiskTelemetry bus channel acceptance (Phase 3 Q3).
//!
//! Spin up a real `agent::EventBus`, hand `stream_risk_telemetry` an
//! `Arc<EventBus>`, publish a `RiskTelemetry` snapshot, and assert the
//! recipe yields `Message::RiskStateRefreshed(RiskState)` carrying the
//! published fields. Also drives the recipe's output through `update`
//! and asserts `cockpit.risk_state` flips from `Loading` to `Ready`.

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use agent::config::BusConfig;
use agent::EventBus;
use futures::StreamExt;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tokio::time::timeout;
use trading_core::{RiskTelemetry, Symbol, Venue};

use ui::live::stream_risk_telemetry;
use ui::state::{update, Cockpit, Message, PanelState};

fn synthetic_telemetry() -> RiskTelemetry {
    let mut exposure = HashMap::new();
    let mut caps = HashMap::new();
    exposure.insert((Venue::Binance, Symbol::new("BTCUSDT")), dec!(50));
    caps.insert((Venue::Binance, Symbol::new("BTCUSDT")), dec!(100));
    RiskTelemetry {
        per_symbol_exposure: exposure,
        per_symbol_caps: caps,
        daily_loss_used_pct: dec!(20),
        daily_loss_cap_pct: dec!(100),
        heartbeat_age_ms: 250,
        heartbeat_timeout_ms: 30_000,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn risk_telemetry_subscription_yields_risk_state_refreshed() {
    let bus = Arc::new(EventBus::new(&BusConfig::default()));

    // Eager subscribe before publish (per stream_market_health pattern —
    // broadcast events sent before the receiver attaches are dropped).
    let mut risk_stream = Box::pin(stream_risk_telemetry(&bus));

    // Publish on a background task so the stream's `recv().await` can
    // observe the event (broadcast::Sender::send() is sync but the
    // channel needs a polling consumer).
    let bus_w = Arc::clone(&bus);
    tokio::spawn(async move {
        // Yield to let the recipe attach its receiver before publish.
        tokio::time::sleep(Duration::from_millis(20)).await;
        bus_w.publish_risk_telemetry(synthetic_telemetry());
    });

    let msg = timeout(Duration::from_secs(2), risk_stream.next())
        .await
        .expect("stream timed out waiting for RiskTelemetry")
        .expect("stream ended unexpectedly");

    let Message::RiskStateRefreshed(state) = msg else {
        panic!("expected RiskStateRefreshed, got {msg:?}");
    };
    assert_eq!(
        state.daily_loss_used_pct,
        Decimal::from(20),
        "telemetry → state copies daily_loss_used_pct verbatim"
    );
    assert_eq!(state.heartbeat_age_ms, 250);
    assert_eq!(state.heartbeat_timeout_ms, 30_000);
    assert_eq!(state.per_symbol_exposure.len(), 1);

    // Drive into `update` and assert the panel flips Loading → Ready.
    let mut cockpit = Cockpit::default();
    assert!(matches!(cockpit.risk_state, PanelState::Loading));
    update(&mut cockpit, Message::RiskStateRefreshed(state));
    assert!(matches!(cockpit.risk_state, PanelState::Ready(_)));
}
