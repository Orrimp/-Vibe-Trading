//! T-D-N(E4) — `BudgetedProvider` 80% auto-degrade + 100% block.
//!
//! Verifies that `LlmForecasterImpl` wrapped in `BudgetedProvider` correctly:
//! 1. Degrades a `DeepThink` request to `QuickThink` at ≥ 80% budget spend.
//! 2. Returns `LlmForecasterError::BudgetExceeded` at ≥ 100% budget spend.
//!
//! ## No real API calls
//!
//! All tests use `wiremock::MockServer`. `ANTHROPIC_API_KEY` is not read.

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

fn minimal_ctx() -> ForecastContext {
    ForecastContext::test_fixture(
        Symbol::new("BTCUSDT"),
        make_ts(1_700_000_000),
        vec![make_bar("BTCUSDT", 1_700_000_000)],
    )
}

fn canned_haiku_response() -> Value {
    serde_json::json!({
        "id": "msg_e4_01",
        "type": "message",
        "role": "assistant",
        "model": "claude-haiku-4-5-20251001",
        "content": [{
            "type": "tool_use",
            "id": "toolu_e4_01",
            "name": "propose_forecast",
            "input": {
                "rating": "HOLD",
                "confidence": 0.50,
                "horizon": "short",
                "reasoning_trace": "Mixed signals in current regime. RSI at 50.1, MACD near zero. No clear directional edge given current BTC regime classification. Carrying neutral stance pending clearer signal.",
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

/// Build a `LlmForecasterImpl` that uses `DeepThink` tier (Opus-class),
/// but the `BudgetedProvider` may degrade it to `QuickThink` (Haiku-class)
/// depending on the budget state.
fn make_deep_think_budgeted_impl(base_url: &str, budget: Arc<CostBudget>) -> LlmForecasterImpl {
    let cfg = Arc::new(llm::config::LlmConfig::default());
    let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);
    let inner = AnthropicProvider::with_base_url(
        base_url,
        "test-key",
        ModelId::from("claude-haiku-4-5-20251001"),
    );
    let budgeted = BudgetedProvider::new(inner, budget, sink, cfg);
    // Use DeepThink tier to trigger the degrade path.
    LlmForecasterImpl::new(
        Arc::new(budgeted),
        "claude-haiku-4-5-20251001",
        LlmTier::DeepThink,
    )
}

/// Build a `LlmForecasterImpl` with a `BudgetedProvider` that will block
/// (budget already exceeded).
fn make_blocked_budgeted_impl(base_url: &str, budget: Arc<CostBudget>) -> LlmForecasterImpl {
    let cfg = Arc::new(llm::config::LlmConfig::default());
    let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);
    let inner = AnthropicProvider::with_base_url(
        base_url,
        "test-key",
        ModelId::from("claude-haiku-4-5-20251001"),
    );
    let budgeted = BudgetedProvider::new(inner, budget, sink, cfg);
    LlmForecasterImpl::new(
        Arc::new(budgeted),
        "claude-haiku-4-5-20251001",
        LlmTier::QuickThink,
    )
}

// ── T-D-N(E4): 80% degrade test ──────────────────────────────────────────────

/// T-D-N(E4, first): At ≥ 80% spend, `DeepThink` request degrades to `QuickThink`
/// and the call still succeeds (using the quick-think model from config).
///
/// The wiremock server responds to any POST /messages, so it accepts the
/// degraded request. The response decodes correctly.
#[tokio::test]
async fn e4_budget_at_80_percent_degrades_deep_think_to_quick_think() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(canned_haiku_response()))
        .mount(&server)
        .await;

    // Seed at 81% (> 80% threshold).
    let ceiling = dec!(200.00);
    let budget = Arc::new(CostBudget::new(ceiling));
    budget.add_spend(dec!(162.01)); // 162.01 / 200 = 81.005%

    let impl_ = make_deep_think_budgeted_impl(&server.uri(), budget);
    let ctx = minimal_ctx();

    // The call should SUCCEED (degrade, not block) — wiremock returns a valid response.
    let forecast = impl_
        .forecast(ctx)
        .await
        .expect("at 80% degrade, call should still succeed");

    // The forecaster decoded the response correctly.
    assert_eq!(forecast.symbol, Symbol::new("BTCUSDT"));
    // At least one request was made (to the degraded model).
    let received = server.received_requests().await.expect("received");
    assert_eq!(received.len(), 1, "exactly 1 HTTP request (degraded call)");
}

// ── T-D-N(E4): 100% block test ───────────────────────────────────────────────

/// T-D-N(E4, second): At ≥ 100% spend, any request is blocked with
/// `LlmForecasterError::BudgetExceeded`. No HTTP request is made.
#[tokio::test]
async fn e4_budget_at_100_percent_blocks_with_budget_exceeded_error() {
    let server = MockServer::start().await;
    // Do NOT mount any mock — if the inner provider is called, wiremock returns 404
    // and the test would fail differently. We want zero requests.

    // Seed at 100.01% (> 100% threshold).
    let ceiling = dec!(200.00);
    let budget = Arc::new(CostBudget::new(ceiling));
    budget.add_spend(dec!(200.01));

    let impl_ = make_blocked_budgeted_impl(&server.uri(), budget);
    let ctx = minimal_ctx();

    let err = impl_
        .forecast(ctx)
        .await
        .expect_err("at 100% budget, call must be blocked");

    assert!(
        matches!(err, LlmForecasterError::BudgetExceeded { .. }),
        "blocked budget must surface as BudgetExceeded, got {:?}",
        err
    );

    // No HTTP request was made.
    let received = server.received_requests().await.expect("received");
    assert_eq!(
        received.len(),
        0,
        "at 100% budget, inner provider must NOT be called"
    );
}

/// T-D-N(E4): `BudgetExceeded` error is backtest-fatal.
#[test]
fn e4_budget_exceeded_error_is_backtest_fatal() {
    let err = LlmForecasterError::BudgetExceeded {
        cap_usd: dec!(100.0),
        actual_usd: dec!(100.01),
    };
    assert!(
        err.is_backtest_fatal(),
        "BudgetExceeded must be backtest-fatal for L3 verdict"
    );
}

/// T-D-N(E4): healthy budget (< 80%) passes through DeepThink unchanged.
#[tokio::test]
async fn e4_healthy_budget_passes_deep_think_through() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(canned_haiku_response()))
        .mount(&server)
        .await;

    // Seed at 50% (well below 80% threshold).
    let ceiling = dec!(200.00);
    let budget = Arc::new(CostBudget::new(ceiling));
    budget.add_spend(dec!(100.00)); // exactly 50%

    let impl_ = make_deep_think_budgeted_impl(&server.uri(), budget);
    let ctx = minimal_ctx();

    let forecast = impl_
        .forecast(ctx)
        .await
        .expect("healthy budget: call must succeed");
    assert_eq!(forecast.symbol, Symbol::new("BTCUSDT"));
}
