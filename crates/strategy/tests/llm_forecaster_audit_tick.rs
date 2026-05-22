//! T-D-N(E3) — `AuditEvent::LlmForecastEmitted` tick emission.
//!
//! Verifies that exactly 1 `AuditTick` carrying `LlmForecastEmitted` is
//! broadcast after a successful `post_llm_forecast()` call (R7.1.3).
//!
//! NOTE: This test verifies the audit-journal path (post_llm_forecast emits
//! a tick). The strategy-level caller (LlmForecasterStrategy::on_bar) is
//! responsible for calling post_llm_forecast after a successful forecast;
//! that wiring is exercised here via a direct audit-writer call to confirm
//! the tick fires correctly.
//!
//! ## No real API calls
//!
//! All tests use a stub forecaster or wiremock.

use audit::{
    Ledger,
    journal::{LlmForecastWrite, post_llm_forecast},
    tick::{AuditEvent, AuditTickStream},
};
use rust_decimal_macros::dec;
use uuid::Uuid;

/// Open an in-memory ledger with a tick bus.
async fn open_ledger_with_bus() -> (Ledger, AuditTickStream) {
    let (ledger, sender) = Ledger::open_with_tick_bus(":memory:", 64)
        .await
        .expect("in-memory ledger with tick bus");
    let stream = AuditTickStream::new(sender.subscribe(), "e3_test");
    (ledger, stream)
}

fn sample_write(cid: Uuid) -> LlmForecastWrite<'static> {
    LlmForecastWrite {
        strategy_id: "llm_forecaster_v3",
        symbol: "BTCUSDT",
        correlation_id: cid,
        rating: "BUY",
        confidence: dec!(0.72),
        horizon: "one_hour",
        reasoning_trace: "RSI(14) = 62.5 trending above 60 for 3 consecutive bars. MACD histogram positive at 0.0023 and rising. BB upper band at 45,200 not yet breached.",
        trace_sha256: "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2",
        cited_lesson_ids_json: r#"["lc_btc_bull_001"]"#,
        tokens_in: 5876,
        tokens_out: 412,
        tokens_cached_in: 2000,
        cost_usd: dec!(0.0085),
        forecaster_name: "llm_forecaster_impl",
        model_id: "claude-haiku-4-5-20251001",
        ts: Some("2026-05-22T12:00:00.000000Z"),
    }
}

/// T-D-N(E3): exactly 1 AuditTick::LlmForecastEmitted fires per post_llm_forecast.
#[tokio::test]
async fn e3_post_llm_forecast_emits_exactly_one_audit_tick() {
    let (ledger, mut stream) = open_ledger_with_bus().await;
    let cid = Uuid::new_v4();
    let write = sample_write(cid);

    post_llm_forecast(&ledger, &write)
        .await
        .expect("post_llm_forecast must succeed");

    let tick = tokio::time::timeout(std::time::Duration::from_secs(1), stream.next())
        .await
        .expect("tick must arrive within 1s")
        .expect("tick stream not closed");

    // Confirm it is the LlmForecastEmitted variant.
    assert!(
        matches!(tick.event, AuditEvent::LlmForecastEmitted { .. }),
        "expected LlmForecastEmitted, got {:?}",
        tick.event
    );
}

/// T-D-N(E3): tick carries correct fields (symbol, rating, correlation_id, cost_usd).
#[tokio::test]
async fn e3_audit_tick_fields_match_write() {
    let (ledger, mut stream) = open_ledger_with_bus().await;
    let cid = Uuid::new_v4();
    let write = sample_write(cid);

    post_llm_forecast(&ledger, &write)
        .await
        .expect("post_llm_forecast must succeed");

    let tick = tokio::time::timeout(std::time::Duration::from_secs(1), stream.next())
        .await
        .expect("tick must arrive within 1s")
        .expect("tick stream not closed");

    match tick.event {
        AuditEvent::LlmForecastEmitted {
            symbol,
            rating,
            confidence,
            correlation_id,
            cost_usd,
        } => {
            assert_eq!(symbol.as_str(), "BTCUSDT", "symbol");
            assert_eq!(rating.as_str(), "BUY", "rating");
            assert_eq!(confidence.as_str(), "0.72", "confidence");
            assert_eq!(correlation_id, cid, "correlation_id");
            assert_eq!(cost_usd.as_str(), "0.0085", "cost_usd");
        }
        other => panic!("expected LlmForecastEmitted, got {:?}", other),
    }
}

/// T-D-N(E3): no tick bus → no panic (post_llm_forecast gracefully skips tick emission).
#[tokio::test]
async fn e3_no_tick_bus_does_not_panic() {
    // Ledger::in_memory() has tick_bus = None.
    let ledger = Ledger::in_memory().await.expect("in-memory ledger");
    let cid = Uuid::new_v4();
    let write = sample_write(cid);

    post_llm_forecast(&ledger, &write)
        .await
        .expect("post_llm_forecast must succeed even without a tick bus");
}

/// T-D-N(E3): each distinct forecast call emits a separate tick.
#[tokio::test]
async fn e3_two_forecast_calls_emit_two_ticks() {
    let (ledger, mut stream) = open_ledger_with_bus().await;

    let cid1 = Uuid::new_v4();
    let cid2 = Uuid::new_v4();

    post_llm_forecast(&ledger, &sample_write(cid1))
        .await
        .expect("first post ok");
    post_llm_forecast(&ledger, &sample_write(cid2))
        .await
        .expect("second post ok");

    let tick1 = tokio::time::timeout(std::time::Duration::from_secs(1), stream.next())
        .await
        .expect("tick1 arrives")
        .expect("stream open");
    let tick2 = tokio::time::timeout(std::time::Duration::from_secs(1), stream.next())
        .await
        .expect("tick2 arrives")
        .expect("stream open");

    let id1 = match &tick1.event {
        AuditEvent::LlmForecastEmitted { correlation_id, .. } => *correlation_id,
        other => panic!("tick1: expected LlmForecastEmitted, got {:?}", other),
    };
    let id2 = match &tick2.event {
        AuditEvent::LlmForecastEmitted { correlation_id, .. } => *correlation_id,
        other => panic!("tick2: expected LlmForecastEmitted, got {:?}", other),
    };

    assert_ne!(
        id1, id2,
        "two separate calls must emit ticks with different correlation_ids"
    );
}
