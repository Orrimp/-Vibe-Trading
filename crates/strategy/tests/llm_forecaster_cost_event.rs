//! T-D-N(E2) — `CostEvent::Llm` row emission via `BudgetedProvider`.
//!
//! Verifies that exactly 1 `CostEvent::Llm` row is posted to the cost sink
//! on a successful `LlmForecasterImpl::forecast()` call when the provider
//! is wrapped with `BudgetedProvider`.
//!
//! ## No real API calls
//!
//! All tests use `wiremock::MockServer`. `ANTHROPIC_API_KEY` is not read.

use std::sync::{Arc, Mutex};

use cost::{CostBudget, CostEvent, CostSink, LlmTier};
use llm::{AnthropicProvider, BudgetedProvider, ModelId};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde_json::Value;
use time::OffsetDateTime;
use trading_core::{Bar, CostError, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use strategy::llm_forecaster::{ForecastContext, LlmForecaster, LlmForecasterImpl};

// ── CaptureCostSink ───────────────────────────────────────────────────────────

/// Test-only sink that captures every `CostEvent` into a `Vec` for assertions.
struct CaptureCostSink {
    events: Mutex<Vec<CostEvent>>,
}

impl CaptureCostSink {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            events: Mutex::new(Vec::new()),
        })
    }

    fn events(&self) -> Vec<CostEvent> {
        self.events.lock().expect("lock").clone()
    }
}

impl CostSink for CaptureCostSink {
    fn record(&self, event: CostEvent) -> Result<(), CostError> {
        self.events.lock().expect("lock").push(event);
        Ok(())
    }
}

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

fn canned_buy_response() -> Value {
    serde_json::json!({
        "id": "msg_e2_01",
        "type": "message",
        "role": "assistant",
        "model": "claude-haiku-4-5-20251001",
        "content": [{
            "type": "tool_use",
            "id": "toolu_e2_01",
            "name": "propose_forecast",
            "input": {
                "rating": "BUY",
                "confidence": 0.72,
                "horizon": "short",
                "reasoning_trace": "RSI(14) = 62.5 trending above 60 for 3 bars. MACD histogram positive. BB upper band not yet breached. Strong bullish momentum. Net assessment: moderate bullish signal confirmed.",
                "cited_lesson_ids": ["lc_btc_bull_001"]
            }
        }],
        "stop_reason": "tool_use",
        "usage": {
            "input_tokens": 5876,
            "output_tokens": 412,
            "cache_read_input_tokens": 2000
        }
    })
}

/// Build a `LlmForecasterImpl` wrapped in `BudgetedProvider` with a
/// `CaptureCostSink`.
fn make_budgeted_impl(
    base_url: &str,
    budget: Arc<CostBudget>,
    sink: Arc<CaptureCostSink>,
) -> LlmForecasterImpl {
    let cfg = Arc::new(llm::config::LlmConfig::default());
    let inner = AnthropicProvider::with_base_url(
        base_url,
        "test-key",
        ModelId::from("claude-haiku-4-5-20251001"),
    );
    let budgeted = BudgetedProvider::new(inner, budget, sink as Arc<dyn CostSink>, cfg);
    LlmForecasterImpl::new(
        Arc::new(budgeted),
        "claude-haiku-4-5-20251001",
        LlmTier::QuickThink,
    )
}

// ── T-D-N(E2) tests ───────────────────────────────────────────────────────────

/// T-D-N(E2): exactly 1 `CostEvent::Llm` row posted on a successful forecast call.
#[tokio::test]
async fn e2_successful_forecast_emits_exactly_one_cost_event() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(canned_buy_response()))
        .mount(&server)
        .await;

    let budget = Arc::new(CostBudget::new(dec!(200.00)));
    let sink = CaptureCostSink::new();
    let impl_ = make_budgeted_impl(&server.uri(), budget, Arc::clone(&sink));

    let ctx = minimal_ctx();
    let forecast = impl_.forecast(ctx).await.expect("forecast ok");
    assert_eq!(forecast.symbol, Symbol::new("BTCUSDT"));

    let events = sink.events();
    assert_eq!(
        events.len(),
        1,
        "exactly 1 CostEvent::Llm must be emitted per forecast call; got {events:?}"
    );

    // Verify the event is a CostEvent::Llm with expected token counts.
    match &events[0] {
        CostEvent::Llm {
            tokens_in,
            tokens_out,
            tokens_cached_in,
            ..
        } => {
            assert_eq!(*tokens_in, 5876, "tokens_in from response");
            assert_eq!(*tokens_out, 412, "tokens_out from response");
            assert_eq!(*tokens_cached_in, 2000, "tokens_cached_in from response");
        }
        other => panic!("expected CostEvent::Llm, got {:?}", other),
    }
}

/// T-D-N(E2): failed forecast call does NOT post a cost event (R9.3 — no billing on failure).
#[tokio::test]
async fn e2_failed_forecast_does_not_emit_cost_event() {
    let server = MockServer::start().await;
    // Return a 500 — the call fails.
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(500).set_body_string("server error"))
        .mount(&server)
        .await;

    let budget = Arc::new(CostBudget::new(dec!(200.00)));
    let sink = CaptureCostSink::new();
    let impl_ = make_budgeted_impl(&server.uri(), budget, Arc::clone(&sink));

    let ctx = minimal_ctx();
    let _ = impl_
        .forecast(ctx)
        .await
        .expect_err("500 should produce an error");

    let events = sink.events();
    assert_eq!(
        events.len(),
        0,
        "failed call must NOT emit a CostEvent (R9.3); got {events:?}"
    );
}

/// T-D-N(E2): cost event `usd` field is greater than zero after a real token spend.
///
/// Note: `CostBudget.spent()` tracks in whole-cent granularity. For small
/// calls (sub-cent cost), `budget.spent()` may remain 0 due to truncation.
/// We assert on `CostEvent.usd` which uses full Decimal precision.
#[tokio::test]
async fn e2_cost_event_usd_is_positive_after_token_spend() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(canned_buy_response()))
        .mount(&server)
        .await;

    let budget = Arc::new(CostBudget::new(dec!(200.00)));
    let sink = CaptureCostSink::new();
    let impl_ = make_budgeted_impl(&server.uri(), budget.clone(), Arc::clone(&sink));

    let ctx = minimal_ctx();
    impl_.forecast(ctx).await.expect("forecast ok");

    let events = sink.events();
    assert_eq!(events.len(), 1);
    let usd = events[0].usd();
    assert!(
        usd > Decimal::ZERO,
        "CostEvent.usd must be > 0 after a call with non-zero tokens; got {usd}"
    );
    // Token counts in the event must match the wiremock response.
    match &events[0] {
        CostEvent::Llm {
            tokens_in,
            tokens_out,
            ..
        } => {
            assert!(*tokens_in > 0, "tokens_in must be positive");
            assert!(*tokens_out > 0, "tokens_out must be positive");
        }
        other => panic!("expected Llm event, got {:?}", other),
    }
}
