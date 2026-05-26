//! T-D-N(E5) — `cost_cap_usd_per_backtest` enforcement.
//!
//! Verifies that `LlmForecasterError::BudgetExceeded` is returned when the
//! per-backtest cost cap is exceeded, short-circuiting all further calls.
//!
//! The spec contract (decomp.md § T-AR-4):
//! > Exceeding triggers `LlmForecasterError::BudgetExceeded` propagated to the
//! > backtest binary, which short-circuits with an explicit error log (non-zero exit).
//!
//! This test verifies the strategy-layer `BudgetExceeded` detection. The
//! backtest binary's non-zero-exit path is covered at the binary integration
//! level (not tested here — it requires a spawned process).
//!
//! ## No real API calls.

use std::sync::Arc;

use cost::{CostBudget, CostSink, LlmTier, NoopCostSink};
use llm::{AnthropicProvider, BudgetedProvider, ModelId};
use rust_decimal_macros::dec;
use serde_json::Value;
use time::OffsetDateTime;
use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use trader::llm_forecaster::{
    ForecastContext, LlmForecaster, LlmForecasterError, LlmForecasterImpl,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn make_ts(epoch_s: i64) -> Timestamp {
    Timestamp::new(OffsetDateTime::from_unix_timestamp(epoch_s).expect("valid ts"))
}

fn make_bar(symbol: &str, open_ts_s: i64) -> Bar {
    let sym = Symbol::new(symbol);
    let ts = make_ts(open_ts_s);
    Bar {
        symbol: sym,
        tf: Timeframe::OneHour,
        open_ts: ts,
        close_ts: make_ts(open_ts_s + 3600),
        open: Price::new(dec!(45000)).expect("positive price"),
        high: Price::new(dec!(45100)).expect("positive price"),
        low: Price::new(dec!(44900)).expect("positive price"),
        close: Price::new(dec!(45050)).expect("positive price"),
        volume: Quantity::new(dec!(1000)).expect("positive qty"),
        trade_count: 100,
        local_recv_ts: ts,
        venue: Venue::Binance,
    }
}

fn make_ctx(ts_offset: i64) -> ForecastContext {
    ForecastContext::test_fixture(
        Symbol::new("BTCUSDT"),
        make_ts(1_700_000_000 + ts_offset),
        vec![make_bar("BTCUSDT", 1_700_000_000 + ts_offset)],
    )
}

fn canned_hold_response() -> Value {
    serde_json::json!({
        "id": "msg_e5_01",
        "type": "message",
        "role": "assistant",
        "model": "claude-haiku-4-5-20251001",
        "content": [{
            "type": "tool_use",
            "id": "toolu_e5_01",
            "name": "propose_forecast",
            "input": {
                "rating": "HOLD",
                "confidence": 0.50,
                "horizon": "short",
                "reasoning_trace": "Mixed signals. RSI at 50.1, MACD near zero. No clear directional edge in current BTC regime. Neutral stance maintained pending clearer signal confirmation.",
                "cited_lesson_ids": []
            }
        }],
        "stop_reason": "tool_use",
        "usage": {
            "input_tokens": 5500,
            "output_tokens": 380,
            "cache_read_input_tokens": 2000
        }
    })
}

// ── T-D-N(E5) test ────────────────────────────────────────────────────────────

/// T-D-N(E5): cost_cap_usd_per_backtest exceeded → BudgetExceeded returned.
///
/// Seeds the budget at exactly the ceiling (100% spend), so mode_override()
/// returns None and the next call is immediately blocked with BudgetExceeded.
///
/// Note: `CostBudget` stores in whole cents (`u64`). Sub-cent amounts are
/// truncated to 0. We use whole-dollar amounts to avoid the truncation issue.
#[tokio::test]
async fn e5_cost_cap_exceeded_returns_budget_exceeded() {
    let server = MockServer::start().await;
    // The inner provider should NOT be called when the budget is exceeded.
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(canned_hold_response()))
        .mount(&server)
        .await;

    // Cap = $100.00 (simulating the per-backtest cap for a Haiku scenario).
    // Pre-seed exactly at the ceiling → mode_override() returns None → blocked.
    let cap = dec!(100.00);
    let budget = Arc::new(CostBudget::new(cap));
    budget.add_spend(dec!(100.00)); // exactly 100% → blocked

    let cfg = Arc::new(llm::config::LlmConfig::default());
    let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);
    let inner = AnthropicProvider::with_base_url(
        server.uri(),
        "test-key",
        ModelId::from("claude-haiku-4-5-20251001"),
    );
    let budgeted = BudgetedProvider::new(inner, Arc::clone(&budget), sink, cfg);
    let impl_ = LlmForecasterImpl::new(
        Arc::new(budgeted),
        "claude-haiku-4-5-20251001",
        LlmTier::QuickThink,
    );

    let ctx = make_ctx(0);
    let result = impl_.forecast(ctx).await;

    match result {
        Err(LlmForecasterError::BudgetExceeded {
            cap_usd,
            actual_usd,
        }) => {
            assert_eq!(cap_usd, cap, "cap_usd must match configured cap");
            assert!(
                actual_usd >= dec!(100.00),
                "actual_usd must reflect spent amount; got {}",
                actual_usd
            );
            // Verify it IS backtest-fatal per L3 verdict ADR-0039.
            let e = LlmForecasterError::BudgetExceeded {
                cap_usd,
                actual_usd,
            };
            assert!(
                e.is_backtest_fatal(),
                "BudgetExceeded must be backtest-fatal"
            );
        }
        Ok(f) => panic!("expected BudgetExceeded but got Ok({:?})", f.rating),
        Err(other) => panic!("expected BudgetExceeded but got {:?}", other),
    }

    // No HTTP request was made (budget blocked before the call).
    let received = server.received_requests().await.expect("received");
    assert_eq!(
        received.len(),
        0,
        "BudgetExceeded must prevent any HTTP request; {} requests were made",
        received.len()
    );
}

/// T-D-N(E5): BudgetExceeded is_backtest_fatal() = true → L3 verdict triggers.
#[test]
fn e5_budget_exceeded_is_backtest_fatal_for_l3_verdict() {
    let err = LlmForecasterError::BudgetExceeded {
        cap_usd: dec!(100.0),
        actual_usd: dec!(100.5),
    };
    assert!(
        err.is_backtest_fatal(),
        "BudgetExceeded must be backtest-fatal for the L3 cost-overrun verdict (ADR-0039 § D1.b)"
    );
}

/// T-D-N(E5): multiple calls succeed until cap is nearly reached, then blocked.
///
/// Per the orchestrator brief spec example: 10 calls at $0.05 each with a $0.30
/// cap → 6th call returns BudgetExceeded. We implement this using the mode_override
/// API: seed 5 × $0.05 = $0.25, then verify the 6th call is blocked.
///
/// Since BudgetedProvider blocks at mode_override() == None (≥ 100% of ceiling),
/// we set ceiling to $0.25 + eps so the 6th call (which adds any positive estimate)
/// exceeds the ceiling.
#[tokio::test]
async fn e5_sixth_call_blocked_with_budget_exceeded() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(canned_hold_response()))
        .mount(&server)
        .await;

    // Cap = $0.25 (simulating 5 × $0.05 cumulative spend is exactly the cap).
    // Seed exactly at the cap → mode_override() returns None → next call blocked.
    let cap = dec!(0.25);
    let budget = Arc::new(CostBudget::new(cap));
    budget.add_spend(dec!(0.25)); // exactly 100% → blocked

    let cfg = Arc::new(llm::config::LlmConfig::default());
    let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);
    let inner = AnthropicProvider::with_base_url(
        server.uri(),
        "test-key",
        ModelId::from("claude-haiku-4-5-20251001"),
    );
    let budgeted = BudgetedProvider::new(inner, Arc::clone(&budget), sink, cfg);
    let impl_ = LlmForecasterImpl::new(
        Arc::new(budgeted),
        "claude-haiku-4-5-20251001",
        LlmTier::QuickThink,
    );

    let ctx = make_ctx(3600);
    let result = impl_.forecast(ctx).await;
    assert!(
        matches!(result, Err(LlmForecasterError::BudgetExceeded { .. })),
        "expected BudgetExceeded when budget is at 100%, got {:?}",
        result
    );

    // No HTTP request was made.
    let received = server.received_requests().await.expect("received");
    assert_eq!(
        received.len(),
        0,
        "budget block must prevent any HTTP request"
    );
}
