//! K6 noop-fix e2e divergence gate for `RegimeDispatcher` (CLAUDE.md non-negotiable).
//!
//! ## Purpose
//!
//! This test asserts that the `RegimeDispatcher` changes the number of signals
//! (and thus the equity curve) when the classifier emits a non-trivial regime
//! for ≥ 1 bar in the test window.  If the dispatcher is a no-op (the K6
//! regression risk — same bug as v3-vol-targeting noop-fix 2026-05-22), the
//! assertion here FAILS because signal counts are byte-identical.
//!
//! ## Design
//!
//! We drive two strategies over the same bar sequence:
//!
//! 1. **Baseline** — bare `MomentumStrategy` (unconditioned).
//! 2. **Dispatcher** — `RegimeDispatcher` wrapping the same `MomentumStrategy`
//!    with a `StubClassifier` that emits a Volatile posterior with high
//!    confidence (max_p = 0.90 > 0.70).
//!
//! After warm-up, the dispatcher is in `CashHold` routing.  The baseline
//! `MomentumStrategy` emits Buy/Sell/Hold signals normally; the dispatcher
//! emits only Hold.  We assert that the **total number of non-Hold signals**
//! emitted by the two strategies diverges — confirming the dispatcher is NOT
//! a no-op.
//!
//! The "equity" proxy used here is signal count rather than a full backtest
//! simulation; the key invariant is that any non-Hold signal from the baseline
//! that would be suppressed by the dispatcher reduces the dispatcher's non-Hold
//! count.  A sufficiently active test window (many bars driving rebalance
//! events in the momentum strategy) ensures at least one divergence ≥ 1 signal.
//!
//! ## Cross-references
//!
//! - CLAUDE.md non-negotiable: "Every strategy overlay or sizing-modifier ships
//!   with a baseline-equity-divergence end-to-end test from day 1."
//! - K6 risk register entry in `spec/v1/v3-regime-classifier/feature.md`.
//! - ADR-0049 § D3 dispatcher contract.
//! - Pattern reference: `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`.

use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::symbol::Symbol;
use trading_core::{Bar, Price, Quantity, SignalKind, Timeframe, Timestamp, Venue};

use forecast::markov_switching::{RegimeClassifier, RegimeError, RegimeProbability};
use strategy::{
    CashHoldStrategy, DispatchedRegime, MomentumStrategy, RegimeDispatcher, RegimeDispatcherConfig,
    Strategy, cross_sectional::CrossSectionalMomentumConfig,
};

// ── Stub classifier ───────────────────────────────────────────────────────────

/// A deterministic stub `RegimeClassifier` for e2e tests.
///
/// Always reports the same fixed posterior.  Used to drive the dispatcher
/// into a known routing state without requiring a real `MarkovSwitchingClassifier`
/// (which needs ≥ 50 bars + EM convergence).
struct StubClassifier {
    posterior: [f64; 4],
    fitted: bool,
}

impl StubClassifier {
    /// Volatile with high confidence (max_p = 0.90 >> 0.70 threshold).
    fn volatile_high_confidence() -> Self {
        Self {
            posterior: [0.02, 0.03, 0.90, 0.05],
            fitted: false,
        }
    }
}

impl RegimeClassifier for StubClassifier {
    fn fit(&mut self, _log_returns: &[f64]) -> Result<(), RegimeError> {
        self.fitted = true;
        Ok(())
    }

    fn forward_filter(&self, history: &[f64]) -> Result<Vec<RegimeProbability>, RegimeError> {
        if !self.fitted {
            return Err(RegimeError::NotFitted);
        }
        Ok(vec![RegimeProbability { p: self.posterior }; history.len()])
    }
}

// ── Bar builder ───────────────────────────────────────────────────────────────

/// Build a bar with a close price that trends gently upward.
///
/// Prices are designed to ensure momentum scores differ across symbols
/// (so the momentum strategy actually fires Buy/Sell signals during rebalance).
fn make_bar_with_price(symbol: &str, ts_secs: i64, close: rust_decimal::Decimal) -> Bar {
    let ts = Timestamp::new(
        OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(1_700_000_000 + ts_secs),
    );
    Bar {
        symbol: Symbol::new(symbol),
        tf: Timeframe::OneHour,
        open_ts: ts,
        close_ts: ts,
        local_recv_ts: ts,
        venue: Venue::Binance,
        open: Price::new(close).unwrap(),
        high: Price::new(close + dec!(10.0)).unwrap(),
        low: Price::new(close - dec!(5.0)).unwrap(),
        close: Price::new(close).unwrap(),
        volume: Quantity::new(dec!(1000.0)).unwrap(),
        trade_count: 100,
    }
}

// ── Momentum strategy builder ─────────────────────────────────────────────────

fn build_momentum() -> MomentumStrategy {
    // Lookback = 3 bars so the strategy warms up quickly.
    let toml = r#"
id    = "top10_momentum_h1"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT", "XRPUSDT",
            "ADAUSDT", "DOGEUSDT", "AVAXUSDT", "DOTUSDT", "LINKUSDT"]
lookback_minutes = 3
rebalance_minutes = 3
k_long = 3
k_short = 0
exposure_cap = 0.50
drift_rebalance_threshold = 0.10
vol_floor = 0.000001
size = "equal_weight"
"#;
    let cfg = CrossSectionalMomentumConfig::from_str(toml).expect("valid config");
    MomentumStrategy::from_config(cfg, smol_str::SmolStr::new("e2e_test"))
}

// ── Symbols used in the basket ────────────────────────────────────────────────

const SYMBOLS: &[&str] = &[
    "BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT", "XRPUSDT", "ADAUSDT", "DOGEUSDT", "AVAXUSDT",
    "DOTUSDT", "LINKUSDT",
];

// ── Main e2e divergence test ──────────────────────────────────────────────────

/// Build a momentum strategy with a very short lookback so it warms up in 4 bars.
fn build_momentum_short_lookback() -> MomentumStrategy {
    // lookback=3 bars → ring buffer capacity = 4.
    // rebalance=1 minute → any bar triggers rebalance after warm-up (bars are 60+ mins apart).
    let toml = r#"
id    = "top10_momentum_short"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT", "XRPUSDT",
            "ADAUSDT", "DOGEUSDT", "AVAXUSDT", "DOTUSDT", "LINKUSDT"]
lookback_minutes = 3
rebalance_minutes = 1
k_long = 3
k_short = 0
exposure_cap = 0.50
drift_rebalance_threshold = 0.10
vol_floor = 0.000001
size = "equal_weight"
"#;
    let cfg = CrossSectionalMomentumConfig::from_str(toml).expect("valid config");
    MomentumStrategy::from_config(cfg, smol_str::SmolStr::new("e2e_short"))
}

/// Build a momentum strategy with varying prices that cause top-K rotation.
///
/// Prices are designed so that every N bars a different symbol emerges as
/// the top performer, causing Buy/Sell rebalance signals on each rotation.
fn oscillating_price(sym_idx: usize, bar_idx: i64) -> rust_decimal::Decimal {
    let base = 10_000 + sym_idx as i64 * 1_000;
    // Oscillate each symbol with a different phase so ranking rotates.
    // Period: 4 bars (roughly one lookback window).
    let phase = sym_idx as i64;
    let wave = (bar_idx + phase) % 10;
    // Symbol "wins" when wave < 3 (top-3), loses when wave >= 3.
    // Use a large swing (±500) so vol-adjusted scores differ clearly.
    let delta = if wave < 3 { 500 } else { -200 };
    rust_decimal::Decimal::from((base + delta).max(1_000))
}

#[test]
fn dispatcher_equity_diverges_from_baseline_when_volatile_regime_fires() {
    // This test proves the dispatcher is NOT a no-op when the classifier emits
    // a Volatile (high-confidence) regime.
    //
    // Design:
    // 1. Build a baseline MomentumStrategy and a RegimeDispatcher wrapping the
    //    same strategy configuration.
    // 2. The dispatcher's classifier is a StubClassifier that always emits
    //    "Volatile" with max_p = 0.90, above the 0.70 threshold.
    // 3. After a short warm-up phase (enough to fit the classifier and fill the
    //    ring buffers), the dispatcher is in CashHold routing.
    // 4. In phase 2, we feed bars where the MomentumStrategy's ranking oscillates
    //    (prices designed to cause top-K rotation).  The baseline emits Buy/Sell
    //    signals.  The dispatcher emits only Hold.
    // 5. We assert: baseline_non_hold > dispatcher_non_hold (≥ 1 divergence).
    //
    // This is the K6 noop-fix gate per CLAUDE.md non-negotiable.

    const WARM_UP_BARS: i64 = 10; // 10 bars per symbol = 100 total on_bar calls.
    // Momentum lookback=3 warms up after 4 bars/symbol.
    // Classifier min_fit_bars=3 → fit fires at bar_idx=3 on BTCUSDT.
    const SIGNAL_BARS: i64 = 30; // 30 more bars per symbol.  Oscillating prices
    // ensure top-K rotation and Buy/Sell signals.
    const MIN_FIT_BARS: usize = 3; // Classifier fits after 4 closes on BTCUSDT.

    // ── Baseline ──────────────────────────────────────────────────────────────
    let mut baseline = build_momentum_short_lookback();

    // ── Dispatcher (volatile routing) ─────────────────────────────────────────
    let mut dispatcher = RegimeDispatcher::new(
        build_momentum_short_lookback(),
        CashHoldStrategy::new(),
        StubClassifier::volatile_high_confidence(),
        RegimeDispatcherConfig {
            min_fit_bars: MIN_FIT_BARS,
            refit_interval: 1_000_000, // no re-fit during test
            history_capacity: 10_000,
        },
    );

    // ── Phase 1: warm-up ──────────────────────────────────────────────────────
    for bar_idx in 0..WARM_UP_BARS {
        for (sym_idx, sym) in SYMBOLS.iter().enumerate() {
            let price = oscillating_price(sym_idx, bar_idx);
            let ts_secs = bar_idx * 3600 + sym_idx as i64;
            let bar = make_bar_with_price(sym, ts_secs, price);
            baseline.on_bar(&bar);
            dispatcher.on_bar(&bar);
        }
    }

    // After warm-up, the dispatcher must be in CashHold (K6 pre-condition).
    assert_eq!(
        dispatcher.current_regime(),
        DispatchedRegime::CashHold,
        "Dispatcher must be in CashHold after warm-up phase. \
         StubClassifier::volatile_high_confidence emits max_p=0.90 >= 0.70 threshold. \
         Check that BTCUSDT has received >= min_fit_bars+1={} closes.",
        MIN_FIT_BARS + 1
    );

    // ── Phase 2: measure divergence ───────────────────────────────────────────
    let mut baseline_non_hold_signals: u64 = 0;
    let mut dispatcher_non_hold_signals: u64 = 0;

    for bar_idx in WARM_UP_BARS..(WARM_UP_BARS + SIGNAL_BARS) {
        for (sym_idx, sym) in SYMBOLS.iter().enumerate() {
            let price = oscillating_price(sym_idx, bar_idx);
            let ts_secs = bar_idx * 3600 + sym_idx as i64;
            let bar = make_bar_with_price(sym, ts_secs, price);

            let base_sigs = baseline.on_bar(&bar);
            for sig in &base_sigs {
                if sig.kind != SignalKind::Hold {
                    baseline_non_hold_signals += 1;
                }
            }

            let disp_sigs = dispatcher.on_bar(&bar);
            for sig in &disp_sigs {
                if sig.kind != SignalKind::Hold {
                    dispatcher_non_hold_signals += 1;
                }
            }
        }
    }

    // ── K6 noop-fix gate ──────────────────────────────────────────────────────
    //
    // The baseline emits Buy/Sell signals when top-K rotates.
    // The dispatcher (in CashHold routing) emits only Hold.
    //
    // If baseline_non_hold == dispatcher_non_hold, the dispatcher is a no-op
    // and this test FAILS — the K6 regression is present.

    assert!(
        baseline_non_hold_signals > 0,
        "Baseline momentum strategy emitted NO non-Hold signals in phase 2 \
         ({SIGNAL_BARS} bars × {} symbols) — the test is degenerate. \
         The oscillating_price function must produce top-K rotation. \
         baseline_non_hold_signals={}",
        SYMBOLS.len(),
        baseline_non_hold_signals
    );

    assert_eq!(
        dispatcher_non_hold_signals, 0,
        "K6 noop-fix gate FAILED: dispatcher emitted {dispatcher_non_hold_signals} \
         non-Hold signals in phase 2, expected 0. \
         The RegimeDispatcher is a no-op — CashHold routing is not suppressing signals. \
         Check that on_bar routes to cash_hold.on_bar() when current_regime = CashHold."
    );

    let suppressed = baseline_non_hold_signals;
    assert!(
        suppressed >= 1,
        "Dispatcher must suppress ≥ 1 non-Hold signal vs baseline; \
         suppressed = {suppressed}"
    );

    // Equity divergence: baseline sent ≥ 1 position-changing signal that the
    // dispatcher suppressed.  By definition, the equity curves diverge.
    // Diagnostic output (visible with --nocapture).
    eprintln!(
        "[regime_dispatcher_e2e] PASS: baseline_non_hold={baseline_non_hold_signals}, \
         dispatcher_non_hold={dispatcher_non_hold_signals}, \
         suppressed={suppressed} signals (≥ 1 bp divergence assured)"
    );
}

// ── Hysteresis gate ───────────────────────────────────────────────────────────

#[test]
fn dispatcher_retains_default_routing_when_confidence_below_threshold() {
    // Build a dispatcher with a stub that always reports uniform (below-threshold) posteriors.
    struct UniformClassifier {
        fitted: bool,
    }
    impl RegimeClassifier for UniformClassifier {
        fn fit(&mut self, _: &[f64]) -> Result<(), RegimeError> {
            self.fitted = true;
            Ok(())
        }
        fn forward_filter(&self, history: &[f64]) -> Result<Vec<RegimeProbability>, RegimeError> {
            if !self.fitted {
                return Err(RegimeError::NotFitted);
            }
            // Uniform: max_p = 0.25, below the 0.70 threshold.
            Ok(vec![
                RegimeProbability {
                    p: [0.25, 0.25, 0.25, 0.25]
                };
                history.len()
            ])
        }
    }

    let mut dispatcher = RegimeDispatcher::new(
        build_momentum(),
        CashHoldStrategy::new(),
        UniformClassifier { fitted: false },
        RegimeDispatcherConfig {
            min_fit_bars: 5,
            refit_interval: 1_000_000,
            history_capacity: 1_000,
        },
    );

    // Default regime before fit: Momentum.
    assert_eq!(dispatcher.current_regime(), DispatchedRegime::Momentum);

    // Feed enough bars to trigger fit.
    for i in 0..15_i64 {
        let bar = make_bar_with_price(
            "BTCUSDT",
            i * 3600,
            dec!(50_000.0) + rust_decimal::Decimal::from(i),
        );
        dispatcher.on_bar(&bar);
    }

    // Routing must still be Momentum (confidence too low to switch).
    assert_eq!(
        dispatcher.current_regime(),
        DispatchedRegime::Momentum,
        "Below-threshold confidence must NOT trigger a routing switch; \
         hysteresis must retain previous routing"
    );
}
