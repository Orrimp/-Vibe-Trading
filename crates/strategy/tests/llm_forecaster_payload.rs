//! Integration tests for the LLM forecaster Wave A payload types.
//!
//! ## T-D-N(A5) acceptance criteria
//!
//! All of the following must pass:
//! - `Rating::to_signal_kind()` round-trip for all 5 variants.
//! - `LlmForecast` serde round-trip (JSON).
//! - `ForecastContext::request_hash()` determinism:
//!   - identical inputs → identical SHA-256.
//!   - different symbol → different SHA-256.
//!   - different model_id → different SHA-256.
//!   - different bars → different SHA-256.
//! - `Confidence::new()` clamps to [0, 1].
//! - `LlmForecasterConfig::default()` has `enabled = false`.
//! - `StubForecaster` produces a `LlmForecast` with the configured rating.
//! - `canonicalize::hex_encode` produces lowercase 64-char hex.
//! - `canonicalize::sha256("")` matches well-known SHA-256("").
//!
//! ## Cross-references
//!
//! - `spec/v3-llm-forecaster/decomp.md § T-AR-7 Wave A` — test literals.
//! - `crates/strategy/src/llm_forecaster/types.rs` — implementation.
//! - `crates/strategy/src/llm_forecaster/canonicalize.rs` — helpers.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::str::FromStr;
use std::sync::Arc;
use time::OffsetDateTime;
use trading_core::{Bar, Price, Quantity, SignalKind, Symbol, Timeframe, Timestamp, Venue};

use strategy::llm_forecaster::{
    Confidence, ForecastContext, Horizon, LessonCardRef, LlmForecast, LlmForecasterConfig,
    LlmForecasterStrategy, Rating, StubForecaster, UnknownRating, canonicalize,
};

// ── Test helpers ──────────────────────────────────────────────────────────────

fn sym(s: &str) -> Symbol {
    Symbol::new(s)
}

fn ts(epoch_s: i64) -> Timestamp {
    Timestamp::new(OffsetDateTime::from_unix_timestamp(epoch_s).expect("valid ts"))
}

fn bar(symbol: &str, open_ts_s: i64, close_price: Decimal) -> Bar {
    let s = sym(symbol);
    let t = ts(open_ts_s);
    Bar {
        symbol: s,
        tf: Timeframe::OneHour,
        open_ts: t,
        close_ts: ts(open_ts_s + 3600),
        open: Price::new(close_price).expect("positive price"),
        high: Price::new(close_price).expect("positive price"),
        low: Price::new(close_price).expect("positive price"),
        close: Price::new(close_price).expect("positive price"),
        volume: Quantity::new(dec!(1000)).expect("positive qty"),
        trade_count: 100,
        local_recv_ts: t,
        venue: Venue::Binance,
    }
}

fn minimal_context(symbol: &str, epoch_s: i64) -> ForecastContext {
    ForecastContext::test_fixture(
        sym(symbol),
        ts(epoch_s),
        vec![bar(symbol, epoch_s, dec!(45000))],
    )
}

// ── T-D-N(A5): Rating round-trip tests ───────────────────────────────────────

/// All 5 Rating variants map to the correct SignalKind.
#[test]
fn rating_to_signal_kind_all_variants() {
    assert_eq!(Rating::StrongBuy.to_signal_kind(), SignalKind::Buy);
    assert_eq!(Rating::Buy.to_signal_kind(), SignalKind::Buy);
    assert_eq!(Rating::Hold.to_signal_kind(), SignalKind::Hold);
    assert_eq!(Rating::Sell.to_signal_kind(), SignalKind::Sell);
    assert_eq!(Rating::StrongSell.to_signal_kind(), SignalKind::Sell);
}

/// Rating serde round-trip through JSON for all 5 variants.
#[test]
fn rating_serde_round_trip_json() {
    for rating in [
        Rating::StrongBuy,
        Rating::Buy,
        Rating::Hold,
        Rating::Sell,
        Rating::StrongSell,
    ] {
        let json = serde_json::to_string(&rating).expect("serialize");
        let back: Rating = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, rating, "serde round-trip failed for {rating:?}");
    }
}

/// Rating::try_parse handles all canonical SCREAMING_SNAKE_CASE strings.
#[test]
fn rating_try_parse_canonical() {
    assert_eq!(Rating::try_parse("STRONG_BUY"), Some(Rating::StrongBuy));
    assert_eq!(Rating::try_parse("BUY"), Some(Rating::Buy));
    assert_eq!(Rating::try_parse("HOLD"), Some(Rating::Hold));
    assert_eq!(Rating::try_parse("SELL"), Some(Rating::Sell));
    assert_eq!(Rating::try_parse("STRONG_SELL"), Some(Rating::StrongSell));
    assert_eq!(Rating::try_parse("UNKNOWN_RATING"), None);
}

/// Rating::from_str (FromStr trait) returns Err on unknown values.
#[test]
fn rating_from_str_trait_errors_on_unknown() {
    assert_eq!(Rating::from_str("HOLD"), Ok(Rating::Hold));
    let err = Rating::from_str("NOT_A_RATING").expect_err("should fail");
    // UnknownRating wraps the bad string.
    assert_eq!(err, UnknownRating("NOT_A_RATING".to_string()));
}

/// Rating::as_str matches try_parse round-trip.
#[test]
fn rating_as_str_round_trips() {
    for rating in [
        Rating::StrongBuy,
        Rating::Buy,
        Rating::Hold,
        Rating::Sell,
        Rating::StrongSell,
    ] {
        let s = rating.as_str();
        let back = Rating::try_parse(s).expect("as_str should produce parseable value");
        assert_eq!(back, rating);
    }
}

/// Rating::histogram_index is in [0, 4] and each variant maps uniquely.
#[test]
fn rating_histogram_index_unique_and_in_range() {
    let all = [
        Rating::StrongSell,
        Rating::Sell,
        Rating::Hold,
        Rating::Buy,
        Rating::StrongBuy,
    ];
    let indices: Vec<usize> = all.iter().map(|r| r.histogram_index()).collect();
    // Check range.
    assert!(indices.iter().all(|&i| i <= 4));
    // Check uniqueness.
    let mut sorted = indices.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 5, "histogram indices must be unique");
    // HOLD is index 2 per ADR-0039 § D1.b.
    assert_eq!(Rating::Hold.histogram_index(), 2);
}

// ── T-D-N(A5): LlmForecast serde round-trip ──────────────────────────────────

/// LlmForecast serialises and deserialises through JSON, preserving all fields.
#[test]
fn llm_forecast_serde_round_trip_full() {
    let forecast = LlmForecast::new(
        sym("BTCUSDT"),
        ts(1_700_000_000),
        Rating::StrongBuy,
        Confidence::new(dec!(0.92)),
        Horizon::OneHour,
        "Very strong bullish signal: RSI divergence + volume surge.".to_string(),
        vec![
            LessonCardRef {
                card_id: "card_abc".to_string(),
            },
            LessonCardRef {
                card_id: "card_def".to_string(),
            },
        ],
        None,
        "test-forecaster".to_string(),
        uuid::Uuid::nil(),
    );

    let json = serde_json::to_string(&forecast).expect("serialize LlmForecast");
    let back: LlmForecast = serde_json::from_str(&json).expect("deserialize LlmForecast");

    assert_eq!(back.symbol.0.as_str(), "BTCUSDT");
    assert_eq!(back.rating, Rating::StrongBuy);
    assert_eq!(back.confidence, forecast.confidence);
    assert_eq!(back.horizon, Horizon::OneHour);
    assert_eq!(back.reasoning_trace, forecast.reasoning_trace);
    assert_eq!(back.reasoning_trace_sha256, forecast.reasoning_trace_sha256);
    assert_eq!(back.cited_lessons.len(), 2);
    assert_eq!(back.forecaster_name, "test-forecaster");
    assert!(back.cost_ref.is_none());
}

/// `reasoning_trace_sha256` is stable (same input → same SHA).
#[test]
fn reasoning_trace_sha256_stable_across_constructions() {
    let trace = "Stable reasoning trace for hash test.".to_string();
    let f1 = LlmForecast::new(
        sym("ETHUSDT"),
        ts(1_700_000_000),
        Rating::Hold,
        Confidence::new(dec!(0.5)),
        Horizon::OneHour,
        trace.clone(),
        Vec::new(),
        None,
        "stub".to_string(),
        uuid::Uuid::nil(),
    );
    let f2 = LlmForecast::new(
        sym("ETHUSDT"),
        ts(1_700_000_000),
        Rating::Hold,
        Confidence::new(dec!(0.5)),
        Horizon::OneHour,
        trace,
        Vec::new(),
        None,
        "stub".to_string(),
        uuid::Uuid::nil(),
    );
    assert_eq!(f1.reasoning_trace_sha256, f2.reasoning_trace_sha256);
}

/// `reasoning_trace_sha256_hex` is lowercase 64-char hex.
#[test]
fn reasoning_trace_sha256_hex_is_lowercase_64_chars() {
    let f = LlmForecast::new(
        sym("BTCUSDT"),
        ts(1_700_000_000),
        Rating::Buy,
        Confidence::new(dec!(0.7)),
        Horizon::OneHour,
        "test trace".to_string(),
        Vec::new(),
        None,
        "stub".to_string(),
        uuid::Uuid::nil(),
    );
    let hex = f.reasoning_trace_sha256_hex();
    assert_eq!(hex.len(), 64);
    assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    assert!(hex.chars().all(|c| !c.is_ascii_uppercase()));
}

// ── T-D-N(A3 + A4): ForecastContext::request_hash determinism ────────────────

/// Identical ForecastContext inputs produce identical SHA-256.
#[test]
fn forecast_context_request_hash_identical_inputs_equal_sha() {
    let ctx1 = minimal_context("BTCUSDT", 1_700_000_000);
    let ctx2 = minimal_context("BTCUSDT", 1_700_000_000);
    assert_eq!(
        ctx1.request_hash(),
        ctx2.request_hash(),
        "identical contexts must produce identical SHA-256"
    );
}

/// Different symbol → different SHA.
#[test]
fn forecast_context_request_hash_different_symbol_different_sha() {
    let ctx_btc = minimal_context("BTCUSDT", 1_700_000_000);
    let ctx_eth = ForecastContext::test_fixture(
        sym("ETHUSDT"),
        ts(1_700_000_000),
        vec![bar("ETHUSDT", 1_700_000_000, dec!(2500))],
    );
    assert_ne!(
        ctx_btc.request_hash(),
        ctx_eth.request_hash(),
        "different symbols must produce different SHA-256"
    );
}

/// Different model_id → different SHA.
#[test]
fn forecast_context_request_hash_different_model_different_sha() {
    let mut ctx1 = minimal_context("BTCUSDT", 1_700_000_000);
    ctx1.model_id = "claude-haiku-4-5-20251001".to_string();

    let mut ctx2 = minimal_context("BTCUSDT", 1_700_000_000);
    ctx2.model_id = "claude-sonnet-4-6".to_string();

    assert_ne!(
        ctx1.request_hash(),
        ctx2.request_hash(),
        "different model_ids must produce different SHA-256"
    );
}

/// Different bars → different SHA.
#[test]
fn forecast_context_request_hash_different_bars_different_sha() {
    let ctx1 = ForecastContext::test_fixture(
        sym("BTCUSDT"),
        ts(1_700_000_000),
        vec![bar("BTCUSDT", 1_700_000_000, dec!(45000))],
    );
    let ctx2 = ForecastContext::test_fixture(
        sym("BTCUSDT"),
        ts(1_700_000_000),
        vec![bar("BTCUSDT", 1_700_000_000, dec!(48000))],
    );
    assert_ne!(
        ctx1.request_hash(),
        ctx2.request_hash(),
        "different bar prices must produce different SHA-256"
    );
}

/// Different timestamp → different SHA.
#[test]
fn forecast_context_request_hash_different_timestamp_different_sha() {
    let ctx1 = minimal_context("BTCUSDT", 1_700_000_000);
    let ctx2 = minimal_context("BTCUSDT", 1_700_003_600);
    assert_ne!(
        ctx1.request_hash(),
        ctx2.request_hash(),
        "different timestamps must produce different SHA-256"
    );
}

/// request_hash output is a 32-byte (non-zero) array.
#[test]
fn forecast_context_request_hash_is_32_bytes_non_zero() {
    let ctx = minimal_context("BTCUSDT", 1_700_000_000);
    let hash = ctx.request_hash();
    assert_eq!(hash.len(), 32);
    // Extremely unlikely to be all zeros (only if SHA-256 of input == 0, which it never is).
    assert_ne!(hash, [0u8; 32]);
}

// ── Confidence ────────────────────────────────────────────────────────────────

/// Confidence::new clamps values to [0, 1].
#[test]
fn confidence_clamps_to_unit_interval() {
    assert_eq!(Confidence::new(dec!(-1)).value(), dec!(0));
    assert_eq!(Confidence::new(dec!(2)).value(), dec!(1));
    assert_eq!(Confidence::new(dec!(0.5)).value(), dec!(0.5));
    assert_eq!(Confidence::new(dec!(0)).value(), dec!(0));
    assert_eq!(Confidence::new(dec!(1)).value(), dec!(1));
}

// ── LlmForecasterConfig ───────────────────────────────────────────────────────

/// Default config has `enabled = false` (R9.3 default-disabled).
#[test]
fn config_default_is_disabled() {
    let cfg = LlmForecasterConfig::default();
    assert!(
        !cfg.enabled,
        "default config must have enabled = false (R9.3)"
    );
}

/// Default config model_id matches DEFAULT_MODEL_ID.
#[test]
fn config_default_model_id_matches_constant() {
    let cfg = LlmForecasterConfig::default();
    assert_eq!(cfg.model_id, strategy::llm_forecaster::DEFAULT_MODEL_ID);
}

// ── StubForecaster ────────────────────────────────────────────────────────────

/// StubForecaster produces a LlmForecast with the configured rating.
#[test]
fn stub_forecaster_produces_configured_rating() {
    let stub = Arc::new(StubForecaster::with_rating(Rating::StrongBuy));
    let ctx = minimal_context("BTCUSDT", 1_700_000_000);

    let result = pollster::block_on(
        <StubForecaster as strategy::llm_forecaster::LlmForecaster>::forecast(&*stub, ctx),
    );
    let forecast = result.expect("stub must not error");
    assert_eq!(forecast.rating, Rating::StrongBuy);
    assert_eq!(forecast.horizon, Horizon::OneHour);
    assert!(!forecast.reasoning_trace.is_empty());
    assert_eq!(forecast.forecaster_name, "stub");
}

/// StubForecaster default returns Hold.
#[test]
fn stub_forecaster_default_returns_hold() {
    let stub = Arc::new(StubForecaster::default());
    let ctx = minimal_context("ETHUSDT", 1_700_000_000);

    let result = pollster::block_on(
        <StubForecaster as strategy::llm_forecaster::LlmForecaster>::forecast(&*stub, ctx),
    );
    let forecast = result.expect("stub must not error");
    assert_eq!(forecast.rating, Rating::Hold);
}

// ── LlmForecasterStrategy integration ────────────────────────────────────────

/// Strategy with enabled=false returns no signals on any bar.
#[test]
fn strategy_disabled_returns_no_signals() {
    use strategy::Strategy;
    let cfg = LlmForecasterConfig::default(); // enabled = false
    let stub = Arc::new(StubForecaster::default());
    let mut strat = LlmForecasterStrategy::new(cfg, stub, None);

    for i in 0..30 {
        let b = bar("BTCUSDT", i * 3600, dec!(45000));
        let sigs = strat.on_bar(&b);
        assert!(
            sigs.is_empty(),
            "disabled strategy must return no signals at bar {i}"
        );
    }
}

/// Strategy fires once on first bar (no prior forecast), then carries forward.
#[test]
fn strategy_enabled_fires_on_first_bar_then_carries_forward() {
    use strategy::Strategy;
    let cfg = LlmForecasterConfig {
        enabled: true,
        fire_every_n_bars: 5,
        ..LlmForecasterConfig::default()
    };
    let stub = Arc::new(StubForecaster::with_rating(Rating::Buy));
    let mut strat = LlmForecasterStrategy::new(cfg, stub, None);

    // Bar 0: fires → Buy
    let sigs0 = strat.on_bar(&bar("BTCUSDT", 0, dec!(45000)));
    assert_eq!(sigs0.len(), 1);
    assert_eq!(sigs0[0].kind, SignalKind::Buy, "bar 0 should fire Buy");

    // Bars 1-4: carry-forward → Buy
    for i in 1..5 {
        let sigs = strat.on_bar(&bar("BTCUSDT", i * 3600, dec!(45000)));
        assert_eq!(
            sigs[0].kind,
            SignalKind::Buy,
            "bar {i} carry-forward should be Buy"
        );
    }

    // Bar 5: re-fires → Buy again (stub is fixed)
    let sigs5 = strat.on_bar(&bar("BTCUSDT", 5 * 3600, dec!(45000)));
    assert_eq!(sigs5[0].kind, SignalKind::Buy, "bar 5 should re-fire");
}

// ── canonicalize module ───────────────────────────────────────────────────────

/// hex_encode produces lowercase 64-char string.
#[test]
fn canonicalize_hex_encode_lowercase_64_chars() {
    let bytes = [0xABu8; 32];
    let hex = canonicalize::hex_encode(&bytes);
    assert_eq!(hex.len(), 64);
    assert!(hex.chars().all(|c| !c.is_ascii_uppercase()));
}

/// sha256("") matches well-known value.
#[test]
fn canonicalize_sha256_empty_well_known() {
    let h = canonicalize::sha256(b"");
    let hex = canonicalize::hex_encode(&h);
    assert_eq!(
        hex,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

/// canonicalize version constants are coherent.
#[test]
fn canonicalize_versions_coherent() {
    assert!(canonicalize::versions_coherent());
}
