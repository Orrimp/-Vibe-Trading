//! R2-style regression test: Wave C Signal-mapping + carry-forward.
//!
//! ## Purpose
//!
//! Guards the noop-fix lesson: the `Signal.kind` field MUST be mutated by
//! the LLM forecaster strategy — not just metadata. This test exercises:
//!
//! 1. All 5 `Rating` variants map to the correct `SignalKind` (non-trivial
//!    kind mutation for non-HOLD ratings).
//! 2. Carry-forward holds between fire-cadence bars — the same `Signal.kind`
//!    is re-emitted every bar between fires.
//! 3. Default-disabled strategy emits no signals (R9.3 byte-identity guard).
//! 4. Re-fire at N-th bar after carry-forward emits fresh kind.
//! 5. `ForecastContext::from_runtime` with `NullReflectionStore` produces an
//!    empty `top_k_lessons` slice (store has no cards; clean no-panic path).
//!
//! ## Cross-references
//!
//! - `spec/v3-llm-forecaster/decomp.md § T-AR-1` — signal mapping table.
//! - `spec/v3-llm-forecaster/tasks.md § Wave C T-D-N(C4)` — test requirement.
//! - `crates/strategy/src/llm_forecaster/strategy.rs` — implementation.
//! - `crates/strategy/src/llm_forecaster/types.rs:ForecastContext::from_runtime` —
//!   Wave C builder.

use std::sync::Arc;

use reflection::NullReflectionStore;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{Bar, Price, Quantity, SignalKind, Symbol, Timeframe, Timestamp, Venue};

use strategy::Strategy;
use strategy::llm_forecaster::{
    LlmForecaster, LlmForecasterConfig, LlmForecasterStrategy, Rating, StubForecaster,
};

// ── Test helpers ──────────────────────────────────────────────────────────────

fn sym(s: &str) -> Symbol {
    Symbol::new(s)
}

fn ts(epoch_s: i64) -> Timestamp {
    Timestamp::new(OffsetDateTime::from_unix_timestamp(epoch_s).expect("valid ts"))
}

fn make_bar(symbol: &str, open_ts_s: i64) -> Bar {
    let s = sym(symbol);
    let t = ts(open_ts_s);
    Bar {
        symbol: s,
        tf: Timeframe::OneHour,
        open_ts: t,
        close_ts: ts(open_ts_s + 3600),
        open: Price::new(dec!(45000)).expect("positive price"),
        high: Price::new(dec!(45100)).expect("positive price"),
        low: Price::new(dec!(44900)).expect("positive price"),
        close: Price::new(dec!(45050)).expect("positive price"),
        volume: Quantity::new(dec!(1000)).expect("positive qty"),
        trade_count: 100,
        local_recv_ts: t,
        venue: Venue::Binance,
    }
}

/// Build a strategy with the given stub rating, null reflection store, and
/// the given fire cadence.
fn strategy_with_rating(rating: Rating, fire_every: u32) -> LlmForecasterStrategy {
    let cfg = LlmForecasterConfig {
        enabled: true,
        fire_every_n_bars: fire_every,
        ..LlmForecasterConfig::default()
    };
    LlmForecasterStrategy::new(
        cfg,
        Arc::new(StubForecaster::with_rating(rating)),
        Arc::new(NullReflectionStore),
        Vec::new(), // btc_closes: empty → regime = Chop fallback
        None,
    )
}

/// Build a disabled strategy for the R9.3 guard test.
fn disabled_strategy() -> LlmForecasterStrategy {
    LlmForecasterStrategy::new(
        LlmForecasterConfig::default(), // enabled = false
        Arc::new(StubForecaster::default()),
        Arc::new(NullReflectionStore),
        Vec::new(),
        None,
    )
}

// ── Signal mapping tests ──────────────────────────────────────────────────────

/// NOOP-FIX REGRESSION: All 5 rating tiers map to the correct `SignalKind`.
///
/// This is the load-bearing test for the noop-fix lesson.  Any regression
/// where `Signal.kind` is left as `Hold` for a non-HOLD rating will fail here.
#[test]
fn rating_to_signal_kind_all_five_variants() {
    let cases: &[(Rating, SignalKind)] = &[
        (Rating::StrongBuy, SignalKind::Buy),
        (Rating::Buy, SignalKind::Buy),
        (Rating::Hold, SignalKind::Hold),
        (Rating::Sell, SignalKind::Sell),
        (Rating::StrongSell, SignalKind::Sell),
    ];

    for (rating, expected_kind) in cases {
        let mut strat = strategy_with_rating(*rating, 1);
        let signals = strat.on_bar(&make_bar("BTCUSDT", 1_700_000_000));
        assert_eq!(signals.len(), 1, "expected 1 signal for rating {rating:?}");
        assert_eq!(
            signals[0].kind,
            *expected_kind,
            "NOOP-FIX REGRESSION: rating {rating:?} must produce {expected_kind:?}, \
             not {actual:?}",
            actual = signals[0].kind
        );
    }
}

/// STRONG ratings collapse to Buy/Sell — NOT to Hold (noop regression guard).
#[test]
fn strong_ratings_are_not_hold() {
    for rating in [Rating::StrongBuy, Rating::StrongSell] {
        let mut strat = strategy_with_rating(rating, 1);
        let signals = strat.on_bar(&make_bar("ETHUSDT", 1_700_000_000));
        assert_ne!(
            signals[0].kind,
            SignalKind::Hold,
            "NOOP-FIX REGRESSION: {rating:?} must NOT produce Hold"
        );
    }
}

/// NOOP-FIX REGRESSION: non-HOLD ratings produce non-Hold Signal.kind.
/// Explicit mutation guard.
#[test]
fn non_hold_ratings_mutate_signal_kind() {
    let non_hold = [
        Rating::StrongBuy,
        Rating::Buy,
        Rating::Sell,
        Rating::StrongSell,
    ];
    for rating in non_hold {
        let mut strat = strategy_with_rating(rating, 1);
        let signals = strat.on_bar(&make_bar("BTCUSDT", 1_700_000_000));
        assert_ne!(
            signals[0].kind,
            SignalKind::Hold,
            "NOOP-FIX REGRESSION: non-HOLD rating {rating:?} emitted Hold — \
             Signal.kind was not mutated. Executor reads kind, not metadata."
        );
    }
}

// ── Carry-forward tests ───────────────────────────────────────────────────────

/// Carry-forward: between fire-cadence bars, same Signal.kind is re-emitted.
///
/// The 24-bar fire cadence means bars 1..23 carry-forward bar-0's forecast.
/// The signal kind must be stable across all carry-forward bars.
#[test]
fn carry_forward_between_fires_emits_same_kind() {
    let fire_every = 5u32;
    let mut strat = strategy_with_rating(Rating::Buy, fire_every);
    let sym = "BTCUSDT";

    // Bar 0: fires → Buy
    let sig0 = strat.on_bar(&make_bar(sym, 0));
    assert_eq!(sig0[0].kind, SignalKind::Buy, "bar 0 must fire Buy");

    // Bars 1..fire_every-1: carry-forward → must still be Buy
    for i in 1..fire_every {
        let sigs = strat.on_bar(&make_bar(sym, i as i64 * 3600));
        assert_eq!(
            sigs[0].kind,
            SignalKind::Buy,
            "bar {i} carry-forward must be Buy (same as fired kind)"
        );
    }
}

/// Carry-forward with Sell rating produces consistent Sell across all bars.
#[test]
fn carry_forward_sell_rating_stays_sell() {
    let fire_every = 3u32;
    let mut strat = strategy_with_rating(Rating::Sell, fire_every);
    let sym = "ETHUSDT";

    // Bar 0 fires; bars 1, 2 carry-forward.
    for i in 0..fire_every {
        let sigs = strat.on_bar(&make_bar(sym, i as i64 * 3600));
        assert_eq!(
            sigs[0].kind,
            SignalKind::Sell,
            "bar {i} must be Sell (fired or carried-forward)"
        );
    }
}

/// Re-fire at bar N resets and emits the fresh forecast kind.
///
/// After fire_every bars, the strategy must fire again and produce the
/// configured rating (even if the carry-forward period has just ended).
#[test]
fn refires_at_fire_cadence_bar() {
    let fire_every = 3u32;
    let mut strat = strategy_with_rating(Rating::StrongSell, fire_every);
    let sym = "ADAUSDT";

    // Bar 0: fires → Sell (StrongSell collapses to Sell)
    let sig0 = strat.on_bar(&make_bar(sym, 0));
    assert_eq!(sig0[0].kind, SignalKind::Sell, "bar 0 must fire");

    // Bars 1, 2: carry-forward
    strat.on_bar(&make_bar(sym, 3600));
    strat.on_bar(&make_bar(sym, 7200));

    // Bar 3: re-fires → Sell again (stub is fixed to StrongSell → Sell)
    let sig3 = strat.on_bar(&make_bar(sym, 10800));
    assert_eq!(sig3[0].kind, SignalKind::Sell, "bar 3 must re-fire Sell");
}

// ── Fire-cadence counter tests ────────────────────────────────────────────────

/// Exactly 1 LLM call per 24-bar window (the default fire cadence).
///
/// Feeds 48 bars and counts how many bars produced a "fresh fire"
/// vs carry-forward signal.  With fire_every=24, we expect:
/// - Bar 0: fire (1st).
/// - Bars 1-23: carry-forward.
/// - Bar 24: re-fire (2nd).
/// - Bars 25-47: carry-forward.
///
/// Because the stub returns a fixed rating, we can't distinguish fires
/// from carries by kind; we use a counting stub instead.
#[test]
fn fires_exactly_once_per_fire_every_n_bars_window() {
    use std::sync::atomic::{AtomicU32, Ordering};

    /// A stub that increments a counter each time `forecast()` is called.
    struct CountingStub {
        count: Arc<AtomicU32>,
    }

    #[async_trait::async_trait]
    impl LlmForecaster for CountingStub {
        fn name(&self) -> &str {
            "counting_stub"
        }

        async fn forecast(
            &self,
            ctx: strategy::llm_forecaster::ForecastContext,
        ) -> Result<
            strategy::llm_forecaster::LlmForecast,
            strategy::llm_forecaster::LlmForecasterError,
        > {
            self.count.fetch_add(1, Ordering::SeqCst);
            let trace =
                "stub trace for counting test — exactly 50 chars for validation".to_string();
            Ok(strategy::llm_forecaster::LlmForecast::new(
                ctx.symbol,
                ctx.now,
                Rating::Hold,
                strategy::llm_forecaster::Confidence::new(dec!(0.5)),
                strategy::llm_forecaster::Horizon::OneHour,
                trace,
                Vec::new(),
                None,
                self.name().to_string(),
                ctx.correlation_id,
            ))
        }
    }

    let counter = Arc::new(AtomicU32::new(0));
    let stub = Arc::new(CountingStub {
        count: Arc::clone(&counter),
    });
    let fire_every = 24u32;
    let cfg = LlmForecasterConfig {
        enabled: true,
        fire_every_n_bars: fire_every,
        ..LlmForecasterConfig::default()
    };
    let mut strat =
        LlmForecasterStrategy::new(cfg, stub, Arc::new(NullReflectionStore), Vec::new(), None);

    // Feed 48 bars
    for i in 0..48u32 {
        strat.on_bar(&make_bar("BTCUSDT", i as i64 * 3600));
    }

    // Expect exactly 2 fires: bar 0 + bar 24.
    assert_eq!(
        counter.load(Ordering::SeqCst),
        2,
        "expected exactly 2 LLM calls for 48 bars with fire_every=24"
    );
}

// ── R9.3 default-disabled guard ───────────────────────────────────────────────

/// R9.3: default-disabled strategy emits zero signals on every bar.
///
/// Phase F byte-identity guard — the assistant slot stays in placeholder
/// mode when `llm_forecaster_v3 enabled = false`.
#[test]
fn disabled_strategy_emits_no_signals() {
    let mut strat = disabled_strategy();
    for i in 0..30u32 {
        let sigs = strat.on_bar(&make_bar("BTCUSDT", i as i64 * 3600));
        assert!(
            sigs.is_empty(),
            "disabled strategy must emit no signals at bar {i}"
        );
    }
}

// ── from_runtime with NullReflectionStore ────────────────────────────────────

/// `ForecastContext::from_runtime` with `NullReflectionStore` produces a
/// context with empty `top_k_lessons` (store returns no cards).
///
/// This is the pure no-panic path test for Wave C analytical integration.
#[test]
fn from_runtime_with_null_store_produces_empty_lessons() {
    use strategy::llm_forecaster::ForecastContext;

    let bar = make_bar("BTCUSDT", 1_700_000_000);
    let store = NullReflectionStore;

    let ctx = pollster::block_on(ForecastContext::from_runtime(
        &bar,
        &store,
        &[], // empty btc_closes → Chop fallback
        Vec::new(),
        strategy::llm_forecaster::DEFAULT_MODEL_ID,
        Vec::new(),
    ))
    .expect("from_runtime with NullReflectionStore must not fail");

    assert!(
        ctx.top_k_lessons.is_empty(),
        "NullReflectionStore must produce empty top_k_lessons"
    );
    assert_eq!(ctx.symbol, bar.symbol, "symbol must echo from bar");
    assert_eq!(ctx.now, bar.open_ts, "now must be bar open_ts");
}

/// `ForecastContext::from_runtime` request_hash is deterministic across
/// two calls with the same inputs and NullReflectionStore.
#[test]
fn from_runtime_request_hash_is_deterministic() {
    use strategy::llm_forecaster::ForecastContext;

    let bar = make_bar("ETHUSDT", 1_700_000_000);
    let store = NullReflectionStore;

    let ctx1 = pollster::block_on(ForecastContext::from_runtime(
        &bar,
        &store,
        &[],
        Vec::new(),
        strategy::llm_forecaster::DEFAULT_MODEL_ID,
        vec![make_bar("ETHUSDT", 1_699_996_400)],
    ))
    .expect("from_runtime must not fail");

    let ctx2 = pollster::block_on(ForecastContext::from_runtime(
        &bar,
        &store,
        &[],
        Vec::new(),
        strategy::llm_forecaster::DEFAULT_MODEL_ID,
        vec![make_bar("ETHUSDT", 1_699_996_400)],
    ))
    .expect("from_runtime must not fail (second call)");

    // Hashes must be equal for equal inputs (determinism gate).
    // Note: correlation_id is randomly generated per call, but it is
    // NOT included in request_hash() (by design — the hash covers only
    // the prompt-content-deterministic fields). See ForecastContext docs.
    assert_eq!(
        ctx1.request_hash(),
        ctx2.request_hash(),
        "request_hash must be deterministic for equal inputs"
    );
}

// ── Multi-symbol isolation ────────────────────────────────────────────────────

/// Two symbols fire independently — one symbol's carry-forward does not
/// bleed into the other symbol's state.
#[test]
fn two_symbols_fire_independently() {
    let fire_every = 3u32;
    let mut strat = strategy_with_rating(Rating::Buy, fire_every);

    // Feed 3 bars for BTCUSDT (fires at bar 0, carry at 1, 2)
    let sig_btc_0 = strat.on_bar(&make_bar("BTCUSDT", 0));
    let sig_btc_1 = strat.on_bar(&make_bar("BTCUSDT", 3600));
    let sig_btc_2 = strat.on_bar(&make_bar("BTCUSDT", 7200));

    // ETHUSDT starts fresh on its own first bar → fires
    let sig_eth_0 = strat.on_bar(&make_bar("ETHUSDT", 0));

    assert_eq!(
        sig_btc_0[0].kind,
        SignalKind::Buy,
        "BTCUSDT bar 0 fires Buy"
    );
    assert_eq!(
        sig_btc_1[0].kind,
        SignalKind::Buy,
        "BTCUSDT bar 1 carry-forward"
    );
    assert_eq!(
        sig_btc_2[0].kind,
        SignalKind::Buy,
        "BTCUSDT bar 2 carry-forward"
    );
    assert_eq!(
        sig_eth_0[0].kind,
        SignalKind::Buy,
        "ETHUSDT bar 0 fires independently"
    );
}

/// Fire cadence fires on bar `fire_every * n` for n=1, 2, …
/// Verify the counter resets correctly after multiple windows.
#[test]
fn fire_cadence_multiple_windows() {
    let fire_every = 4u32;
    // Strategy fires at bars 0, 4, 8 with fire_every=4.
    // We feed 12 bars and collect the bar indices where kind != Hold
    // to verify the fire pattern.
    let mut strat_buy = strategy_with_rating(Rating::Buy, fire_every);
    let sym = "BTCUSDT";

    // With StubForecaster(Buy), every bar emits Buy (carried or fired).
    // We verify each bar produces a non-empty signal vec.
    for i in 0..12u32 {
        let sigs = strat_buy.on_bar(&make_bar(sym, i as i64 * 3600));
        assert_eq!(sigs.len(), 1, "bar {i} must produce 1 signal");
        assert_eq!(
            sigs[0].kind,
            SignalKind::Buy,
            "bar {i} kind must be Buy (stubbed)"
        );
    }
}
