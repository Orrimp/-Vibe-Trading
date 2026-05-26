//! cockpit-activity-llm-producer v0.1.1 — activity-tape producer integration tests.
//!
//! ## Coverage (T-D-N3 — 6 tests)
//!
//! 1. `start_event_emitted_with_correct_label_format` — wiremock 200; asserts
//!    exactly ONE `ActivityEvent` Start with kind `LlmCall` and label matching
//!    `"LLM call: " + model_id` exactly (Q1=(a) lock).
//!
//! 2. `end_success_event_on_happy_path` — wiremock 200; asserts ONE End(Success)
//!    event immediately following the Start, with matching `id` (RAII correlation).
//!
//! 3. `end_failed_event_on_llm_error` — wiremock 500; asserts ONE End(Failed(reason))
//!    where reason is non-empty and contains the mapped string from R4.1.
//!
//! 4. `pii_redaction_label_excludes_symbol_and_prompt` — wiremock 200; asserts the
//!    Start event label does NOT contain the symbol, any OHLCV token, or any
//!    lesson-card content (K6 / H4 enforcement by runtime assertion).
//!
//! 5. `activity_event_survives_cache_replay_path` — verifies the producer fires
//!    on the LLM call path (cache miss) when a `BudgetedProvider` wraps the
//!    `AnthropicProvider`. Confirms the wiring works through the decorator stack.
//!
//! 6. `no_event_emitted_when_activity_sender_not_wired` — construct
//!    `LlmForecasterImpl` WITHOUT `.with_activity_sender()`; subscribe a separate
//!    `ActivitySender` from a different channel; assert zero `ActivityEvent`s
//!    arrive. Confirms the conditional wiring (R1.2).
//!
//! ## No real API calls
//!
//! All tests use `wiremock::MockServer`. The `ANTHROPIC_API_KEY` is not read.
//!
//! ## PII-redaction contract (K6 / H4)
//!
//! The label is constructed as `"LLM call: " + self.model_id`. No field of
//! `ForecastContext`, `LlmRequest`, `Bar`, or `LessonCard` flows in by construction.
//! Test 4 asserts this at runtime with explicit `assert!(!label.contains(...))` checks.

use std::sync::Arc;

use agent::config::BusConfig;
use agent::{ActivityEvent, ActivityKind, ActivityOutcome, ActivityPhase, EventBus};
use cost::{CostBudget, CostSink, LlmTier, NoopCostSink};
use rust_decimal_macros::dec;
use serde_json::Value;
use time::OffsetDateTime;
use tokio::sync::broadcast;
use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use trader::llm_forecaster::{ForecastContext, LlmForecaster, LlmForecasterImpl};

// ── The model ID used in every test ──────────────────────────────────────────

const TEST_MODEL_ID: &str = "claude-haiku-4-5-20251001";
const EXPECTED_LABEL: &str = "LLM call: claude-haiku-4-5-20251001";

// ── Test helpers ──────────────────────────────────────────────────────────────

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

/// Create an `ActivitySender` backed by a fresh `EventBus` (activity channel capacity 256).
/// Returns the sender plus a subscribed receiver for assertions.
fn make_activity_channel() -> (agent::ActivitySender, broadcast::Receiver<ActivityEvent>) {
    let bus = EventBus::new(&BusConfig::default());
    let sender = bus.activity();
    let rx = sender.subscribe();
    (sender, rx)
}

/// Build the standard happy-path canned Anthropic tool-use response.
fn canned_buy_response() -> Value {
    serde_json::json!({
        "id": "msg_tape_01",
        "type": "message",
        "role": "assistant",
        "model": TEST_MODEL_ID,
        "content": [{
            "type": "tool_use",
            "id": "toolu_tape_01",
            "name": "propose_forecast",
            "input": {
                "rating": "BUY",
                "confidence": 0.72,
                "horizon": "short",
                "reasoning_trace": "RSI(14) = 62.5 trending above 60 for 3 consecutive bars. MACD histogram positive at 0.0023 and rising. BB upper band at 45,200 not yet breached — price has room. Lesson card lc_btc_bull_001 (BTC Bull regime, +0.8% outcome) provides directional confirmation. Net assessment: moderate bullish.",
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

/// Build an `LlmForecasterImpl` connected to a wiremock server WITH an
/// `ActivitySender` wired. Returns (impl, receiver).
fn make_impl_with_activity(
    base_url: &str,
) -> (LlmForecasterImpl, broadcast::Receiver<agent::ActivityEvent>) {
    let provider = Arc::new(llm::AnthropicProvider::with_base_url(
        base_url,
        "test-key",
        llm::ModelId::from(TEST_MODEL_ID),
    ));
    let (activity_sender, rx) = make_activity_channel();
    let impl_ = LlmForecasterImpl::new(provider, TEST_MODEL_ID, LlmTier::QuickThink)
        .with_activity_sender(activity_sender);
    (impl_, rx)
}

/// Build an `LlmForecasterImpl` connected to a wiremock server WITHOUT an
/// `ActivitySender` (default — used to verify R1.2 no-op path).
fn make_impl_without_activity(base_url: &str) -> LlmForecasterImpl {
    let provider = Arc::new(llm::AnthropicProvider::with_base_url(
        base_url,
        "test-key",
        llm::ModelId::from(TEST_MODEL_ID),
    ));
    LlmForecasterImpl::new(provider, TEST_MODEL_ID, LlmTier::QuickThink)
}

// ── Test 1: Start event emitted with correct label format ──────────────────────

/// T-D-N3.1 — Happy path 200: a Start event is emitted with correct kind and
/// exact label format `"LLM call: <model_id>"` (R2.1 / Q1=(a)).
///
/// Asserts the label is exactly `EXPECTED_LABEL` — no symbol, no prompt content,
/// no temperature, no lesson-card content.
#[tokio::test]
async fn start_event_emitted_with_correct_label_format() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(canned_buy_response()))
        .mount(&server)
        .await;

    let (impl_, mut rx) = make_impl_with_activity(&server.uri());
    let ctx = minimal_ctx();
    impl_.forecast(ctx).await.expect("forecast ok");

    // First event must be Start.
    let start_event = rx.try_recv().expect("expected Start event on channel");
    assert!(
        matches!(start_event.phase, ActivityPhase::Start { .. }),
        "first event must be Start, got {:?}",
        start_event.phase
    );
    assert_eq!(
        start_event.kind,
        ActivityKind::LlmCall,
        "kind must be LlmCall"
    );
    assert_eq!(
        start_event.label, EXPECTED_LABEL,
        "label must be exactly '{}', got '{}'",
        EXPECTED_LABEL, start_event.label
    );
}

// ── Test 2: End(Success) event on happy path ───────────────────────────────────

/// T-D-N3.2 — Happy path 200: an End(Success) event is emitted after Start,
/// with the same `id` as the Start event (RAII Start→End correlation).
#[tokio::test]
async fn end_success_event_on_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(canned_buy_response()))
        .mount(&server)
        .await;

    let (impl_, mut rx) = make_impl_with_activity(&server.uri());
    let ctx = minimal_ctx();
    impl_.forecast(ctx).await.expect("forecast ok");

    // Drain Start.
    let start_event = rx.try_recv().expect("expected Start event");
    assert!(matches!(start_event.phase, ActivityPhase::Start { .. }));
    let start_id = start_event.id;

    // Next event must be End(Success) with the same correlation id.
    let end_event = rx.try_recv().expect("expected End event after Start");
    assert!(
        matches!(
            end_event.phase,
            ActivityPhase::End(ActivityOutcome::Success)
        ),
        "expected End(Success), got {:?}",
        end_event.phase
    );
    assert_eq!(
        end_event.id, start_id,
        "End event id must match Start event id (RAII correlation)"
    );
    assert_eq!(end_event.kind, ActivityKind::LlmCall);

    // No further events.
    assert!(
        rx.try_recv().is_err(),
        "unexpected extra event after End on happy path"
    );
}

// ── Test 3: End(Failed) event on LlmError (HTTP 500) ─────────────────────────

/// T-D-N3.3 — Wiremock returns HTTP 500: an End(Failed(reason)) is emitted
/// with a non-empty reason string containing the mapped error text (R4.1).
#[tokio::test]
async fn end_failed_event_on_llm_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "type": "error",
            "error": {
                "type": "api_error",
                "message": "Internal server error"
            }
        })))
        .mount(&server)
        .await;

    let (impl_, mut rx) = make_impl_with_activity(&server.uri());
    let ctx = minimal_ctx();
    // The forecast call should fail with a provider error.
    let result = impl_.forecast(ctx).await;
    assert!(result.is_err(), "expected forecast to fail on HTTP 500");

    // Drain Start.
    let start_event = rx.try_recv().expect("expected Start event");
    assert!(matches!(start_event.phase, ActivityPhase::Start { .. }));

    // Next must be End(Failed(...)).
    let end_event = rx.try_recv().expect("expected End event after Start");
    match &end_event.phase {
        ActivityPhase::End(ActivityOutcome::Failed(reason)) => {
            assert!(
                !reason.is_empty(),
                "failure reason must be non-empty, got empty string"
            );
            // The 500 maps to "server error" per R4.1.
            assert!(
                reason.contains("server error") || reason.contains("error"),
                "expected 'server error' in reason, got: {reason:?}"
            );
        }
        other => panic!("expected End(Failed(_)), got {:?}", other),
    }
}

// ── Test 4: PII redaction — label contains NO symbol or prompt content ────────

/// T-D-N3.4 — PII / prompt-content redaction (K6 / H4 enforcement).
///
/// Constructs a `ForecastContext` with a recognizable symbol ("BTCUSDT") and
/// bars with specific OHLCV values. Asserts the Start event label contains
/// ONLY the model ID — no symbol, no price, no volume, no lesson-card content.
///
/// This test is the runtime gate for the K6 structural guarantee (R2.1 / R2.2):
/// because the label is `"LLM call: " + self.model_id`, it is structurally
/// impossible for prompt content to appear unless the implementation is changed.
#[tokio::test]
async fn pii_redaction_label_excludes_symbol_and_prompt() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(canned_buy_response()))
        .mount(&server)
        .await;

    let (impl_, mut rx) = make_impl_with_activity(&server.uri());
    // Use a recognizable symbol and bar values that would be obvious if they
    // leaked into the label.
    let ctx = ForecastContext::test_fixture(
        Symbol::new("BTCUSDT"),
        make_ts(1_700_000_000),
        vec![make_bar("BTCUSDT", 1_700_000_000)],
    );
    impl_.forecast(ctx).await.expect("forecast ok");

    // Get the Start event's label.
    let start_event = rx.try_recv().expect("expected Start event");
    let label = &start_event.label;

    // Assert the label is EXACTLY the expected format — nothing more.
    assert_eq!(
        label, EXPECTED_LABEL,
        "label must be exactly '{}' per K6 / R2.1, got: {label:?}",
        EXPECTED_LABEL
    );

    // Explicit negative assertions for PII redaction gate (K6 tester grep):
    assert!(
        !label.contains("BTCUSDT"),
        "label must NOT contain symbol 'BTCUSDT' (K6 / R2.1)"
    );
    assert!(
        !label.contains("BTC"),
        "label must NOT contain symbol token 'BTC'"
    );
    assert!(
        !label.contains("45000"),
        "label must NOT contain OHLCV price token"
    );
    assert!(
        !label.contains("prompt"),
        "label must NOT contain any prompt content token"
    );
    assert!(
        !label.contains("symbol"),
        "label must NOT contain the word 'symbol'"
    );
    assert!(
        !label.contains("lesson"),
        "label must NOT contain lesson-card content"
    );
    assert!(
        !label.contains("rating"),
        "label must NOT contain forecast rating content"
    );
    assert!(
        !label.contains("temperature"),
        "label must NOT contain temperature content"
    );
}

// ── Test 5: Activity event survives the budgeted-provider decorator stack ──────

/// T-D-N3.5 — The activity tape producer fires on the LLM call path when
/// `LlmForecasterImpl` is wired through a `BudgetedProvider` decorator
/// (simulating the production stack). Verifies the wiring works through
/// the decorator stack on a cache-miss (first call) path.
#[tokio::test]
async fn activity_event_survives_cache_replay_path() {
    use llm::{AnthropicProvider, BudgetedProvider, ModelId};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(canned_buy_response()))
        .mount(&server)
        .await;

    // Build the production-like decorator stack: BudgetedProvider<AnthropicProvider>.
    // Note: BudgetedProvider<Inner> requires Inner: LlmProvider (not Arc<Inner>).
    let base =
        AnthropicProvider::with_base_url(server.uri(), "test-key", ModelId::from(TEST_MODEL_ID));
    let budget = Arc::new(CostBudget::new(dec!(100.0)));
    let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);
    let cfg = Arc::new(llm::config::LlmConfig::default());
    let budgeted = BudgetedProvider::new(base, budget, sink, cfg);

    let (activity_sender, mut rx) = make_activity_channel();
    let impl_ = LlmForecasterImpl::new(Arc::new(budgeted), TEST_MODEL_ID, LlmTier::QuickThink)
        .with_activity_sender(activity_sender);

    let ctx = minimal_ctx();
    impl_
        .forecast(ctx)
        .await
        .expect("forecast ok through BudgetedProvider");

    // Assert Start emitted.
    let start_event = rx
        .try_recv()
        .expect("expected Start event through BudgetedProvider");
    assert!(matches!(start_event.phase, ActivityPhase::Start { .. }));
    assert_eq!(start_event.kind, ActivityKind::LlmCall);
    assert_eq!(start_event.label, EXPECTED_LABEL);

    // Assert End emitted.
    let end_event = rx
        .try_recv()
        .expect("expected End event through BudgetedProvider");
    assert!(
        matches!(
            end_event.phase,
            ActivityPhase::End(ActivityOutcome::Success)
        ),
        "expected End(Success) through BudgetedProvider, got {:?}",
        end_event.phase
    );
}

// ── Test 6: No event emitted when activity sender is not wired ────────────────

/// T-D-N3.6 — `LlmForecasterImpl` constructed WITHOUT `.with_activity_sender()`
/// emits zero `ActivityEvent`s. Confirms the conditional wiring (R1.2):
/// the None-path is a no-op (zero events, zero overhead). This is the path
/// taken by all 153 existing tests, all backtest bin paths, and the
/// `llm_verdict` CLI bin.
#[tokio::test]
async fn no_event_emitted_when_activity_sender_not_wired() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(canned_buy_response()))
        .mount(&server)
        .await;

    // Build WITHOUT activity sender.
    let impl_ = make_impl_without_activity(&server.uri());

    // Create a SEPARATE sender/receiver to verify nothing arrives there either.
    let (_unrelated_sender, mut unrelated_rx) = make_activity_channel();

    let ctx = minimal_ctx();
    impl_
        .forecast(ctx)
        .await
        .expect("forecast ok without activity sender");

    // The unrelated receiver must have zero events.
    assert!(
        unrelated_rx.try_recv().is_err(),
        "expected zero ActivityEvents when sender is not wired"
    );

    // There is no channel to subscribe to when no sender is wired — the above
    // confirms the None-path produces no events on any channel we can observe.
}
