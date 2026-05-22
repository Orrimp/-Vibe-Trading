//! Integration test for `post_llm_forecast` (T-D-N(E1) + T-D-N(E3)).
//!
//! Verifies that:
//! 1. `JournalEntry { kind: "llm_forecast", payload }` round-trips through
//!    `llm_forecast_entries` (mig 012).
//! 2. All fields persist verbatim (rating, confidence, trace, trace_sha256, cost, tokens).
//! 3. Duplicate `correlation_id` → idempotent INSERT OR IGNORE (no error, 1 row).
//! 4. `AuditEvent::LlmForecastEmitted` tick fires after commit.

use audit::{
    Ledger,
    journal::{LlmForecastWrite, post_llm_forecast},
    tick::{AuditEvent, AuditTickStream},
};
use rust_decimal_macros::dec;
use uuid::Uuid;

/// Open an in-memory ledger for tests.
async fn open_ledger() -> Ledger {
    Ledger::in_memory().await.expect("in-memory ledger opens")
}

/// Open an in-memory ledger with a tick bus so we can observe tick emissions.
async fn open_ledger_with_bus() -> (Ledger, AuditTickStream) {
    let (ledger, sender) = Ledger::open_with_tick_bus(":memory:", 64)
        .await
        .expect("in-memory ledger with tick bus");
    let stream = AuditTickStream::new(sender.subscribe(), "test");
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

/// T-D-N(E1) — basic round-trip: row persists and all fields read back correctly.
#[tokio::test]
async fn post_llm_forecast_round_trip() {
    let ledger = open_ledger().await;
    let cid = Uuid::new_v4();
    let write = sample_write(cid);

    post_llm_forecast(&ledger, &write)
        .await
        .expect("post_llm_forecast must succeed");

    // Read back the row.
    let row: (
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        i64,
        i64,
        i64,
        String,
    ) = sqlx::query_as(
        "SELECT strategy_id, symbol, correlation_id, rating, confidence, \
         trace_sha256, cost_usd, tokens_in, tokens_out, tokens_cached_in, \
         model_id \
         FROM llm_forecast_entries WHERE correlation_id = ?",
    )
    .bind(cid.to_string())
    .fetch_one(ledger.pool())
    .await
    .expect("row must exist after post_llm_forecast");

    assert_eq!(row.0, "llm_forecaster_v3", "strategy_id");
    assert_eq!(row.1, "BTCUSDT", "symbol");
    assert_eq!(row.2, cid.to_string(), "correlation_id");
    assert_eq!(row.3, "BUY", "rating");
    assert_eq!(row.4, "0.72", "confidence");
    assert_eq!(row.5, write.trace_sha256, "trace_sha256");
    assert_eq!(row.6, "0.0085", "cost_usd");
    assert_eq!(row.7, 5876_i64, "tokens_in");
    assert_eq!(row.8, 412_i64, "tokens_out");
    assert_eq!(row.9, 2000_i64, "tokens_cached_in");
    assert_eq!(row.10, "claude-haiku-4-5-20251001", "model_id");
}

/// T-D-N(E1) — idempotency: duplicate correlation_id → INSERT OR IGNORE, 1 row.
#[tokio::test]
async fn post_llm_forecast_idempotent_on_duplicate_correlation_id() {
    let ledger = open_ledger().await;
    let cid = Uuid::new_v4();
    let write = sample_write(cid);

    post_llm_forecast(&ledger, &write)
        .await
        .expect("first insert ok");
    post_llm_forecast(&ledger, &write)
        .await
        .expect("second insert must not error (INSERT OR IGNORE)");

    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM llm_forecast_entries WHERE correlation_id = ?")
            .bind(cid.to_string())
            .fetch_one(ledger.pool())
            .await
            .expect("count query");

    assert_eq!(
        count.0, 1,
        "INSERT OR IGNORE: exactly 1 row for duplicate correlation_id"
    );
}

/// T-D-N(E1) / T-D-N(E3) — AuditTick::LlmForecastEmitted fires post-commit.
#[tokio::test]
async fn post_llm_forecast_emits_audit_tick() {
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
            assert_eq!(symbol.as_str(), "BTCUSDT");
            assert_eq!(rating.as_str(), "BUY");
            assert_eq!(confidence.as_str(), "0.72");
            assert_eq!(correlation_id, cid);
            assert_eq!(cost_usd.as_str(), "0.0085");
        }
        other => panic!("expected LlmForecastEmitted, got {:?}", other),
    }
}

/// T-D-N(E1) — all 5 rating variants persist correctly.
#[tokio::test]
async fn post_llm_forecast_all_rating_variants() {
    let ledger = open_ledger().await;
    for rating in ["STRONG_BUY", "BUY", "HOLD", "SELL", "STRONG_SELL"] {
        let cid = Uuid::new_v4();
        let write = LlmForecastWrite {
            rating,
            ..sample_write(cid)
        };
        post_llm_forecast(&ledger, &write)
            .await
            .unwrap_or_else(|e| panic!("failed for rating {rating}: {e}"));

        let row: (String,) =
            sqlx::query_as("SELECT rating FROM llm_forecast_entries WHERE correlation_id = ?")
                .bind(cid.to_string())
                .fetch_one(ledger.pool())
                .await
                .expect("row must exist");
        assert_eq!(row.0, rating, "rating round-trip for {rating}");
    }
}
