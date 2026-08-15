//! F5b — forward-run engine fidelity tests.
//!
//! Guarantees that `build_registry_for` for each non-SMA bake-off id registers
//! the REAL strategy (ComposedStrategy / AlwaysLongStrategy), NOT the old
//! SmaCrossover proxy.
//!
//! ## Anti-fake gate (day-1 requirement)
//!
//! Prior to F5b every non-SMA id was silently proxied to SmaCrossover.
//! These tests would FAIL if that regression were reintroduced:
//!
//! 1. **Identity test** — the registered strategy id must match the TOML's `id`
//!    field, not `"sma_crossover"`.
//! 2. **Behavioural-divergence test** — feeding the same bar sequence to the
//!    MACD registry and the SMA registry produces at least one different signal.
//!    If they were secretly both SMA underneath, their outputs would be identical
//!    and this assertion would FAIL.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use agent::{build_registry_for, config::ForwardRunConfig};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{
    Bar, Money, Price, Quantity, StrategyId, Symbol, Timeframe, Timestamp, Usdt, Venue,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn default_cfg() -> agent::config::Config {
    agent::config::Config::default()
}

/// A `ForwardRunConfig` with the given strategy id and a fixed BTCUSDT symbol /
/// €200 budget (the product defaults).
fn fwd_cfg(strategy_id: &str) -> ForwardRunConfig {
    ForwardRunConfig {
        strategy: StrategyId::new(strategy_id),
        symbol: Symbol::new("BTCUSDT"),
        budget: Money::<Usdt>::from_decimal(dec!(200)),
        lookback: None,
        param_override: None,
        confidence: None, // P0-3: no scorecard in integration tests
    }
}

/// Build a bar at `ts_offset_hours` hours from epoch with the given close price.
fn make_bar(ts_offset_hours: i64, close: Decimal) -> Bar {
    let ts = Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::hours(ts_offset_hours));
    let price = Price::new(close).expect("close price must be positive");
    Bar {
        symbol: Symbol::new("BTCUSDT"),
        tf: Timeframe::OneHour,
        venue: Venue::Binance,
        open_ts: ts,
        close_ts: ts,
        local_recv_ts: ts,
        open: price,
        high: price,
        low: price,
        close: price,
        volume: Quantity::new(dec!(100)).expect("100 is valid qty"),
        trade_count: 1,
    }
}

// ── Identity tests ────────────────────────────────────────────────────────────

/// The registered strategy id for "v0.5.macd" must be "btc_macd_trend",
/// NOT "sma_crossover".
#[test]
fn f5b_macd_identity_is_btc_macd_trend_not_sma_crossover() {
    let cfg = default_cfg();
    let fwd = fwd_cfg("v0.5.macd");
    let registry = build_registry_for(&cfg, Some(&fwd))
        .expect("build_registry_for(v0.5.macd) must succeed — TOML must be loadable");

    // The registry has exactly one strategy registered.
    assert_eq!(
        registry.len(),
        1,
        "exactly one strategy registered for v0.5.macd"
    );

    // Drain pending events: the Load event carries the StrategyId.
    let events = registry.drain_pending_events();
    assert_eq!(events.len(), 1, "exactly one Load event");
    let loaded_id = events[0].strategy_id.0.as_str();

    // Must be the ComposedStrategy id from the TOML, NOT "sma_crossover".
    assert_ne!(
        loaded_id, "sma_crossover",
        "FAIL: v0.5.macd registered 'sma_crossover' — SMA proxy regression detected"
    );
    assert_eq!(
        loaded_id, "btc_macd_trend",
        "v0.5.macd must register 'btc_macd_trend', got '{loaded_id}'"
    );
}

/// The registered strategy id for "v0.5.rsi" must be "btc_rsi_reversion".
#[test]
fn f5b_rsi_identity_is_btc_rsi_reversion_not_sma_crossover() {
    let cfg = default_cfg();
    let fwd = fwd_cfg("v0.5.rsi");
    let registry =
        build_registry_for(&cfg, Some(&fwd)).expect("build_registry_for(v0.5.rsi) must succeed");

    let events = registry.drain_pending_events();
    assert_eq!(events.len(), 1, "exactly one Load event");
    let loaded_id = events[0].strategy_id.0.as_str();

    assert_ne!(
        loaded_id, "sma_crossover",
        "FAIL: v0.5.rsi registered 'sma_crossover' — SMA proxy regression detected"
    );
    assert_eq!(
        loaded_id, "btc_rsi_reversion",
        "v0.5.rsi must register 'btc_rsi_reversion', got '{loaded_id}'"
    );
}

/// The registered strategy id for "v0.5.bbands" must be "btc_bbands_mean_revert".
#[test]
fn f5b_bbands_identity_is_btc_bbands_mean_revert_not_sma_crossover() {
    let cfg = default_cfg();
    let fwd = fwd_cfg("v0.5.bbands");
    let registry =
        build_registry_for(&cfg, Some(&fwd)).expect("build_registry_for(v0.5.bbands) must succeed");

    let events = registry.drain_pending_events();
    assert_eq!(events.len(), 1, "exactly one Load event");
    let loaded_id = events[0].strategy_id.0.as_str();

    assert_ne!(
        loaded_id, "sma_crossover",
        "FAIL: v0.5.bbands registered 'sma_crossover' — SMA proxy regression detected"
    );
    assert_eq!(
        loaded_id, "btc_bbands_mean_revert",
        "v0.5.bbands must register 'btc_bbands_mean_revert', got '{loaded_id}'"
    );
}

/// The registered strategy id for "v0.buyhold" must be "always_long".
#[test]
fn f5b_buyhold_identity_is_always_long_not_sma_crossover() {
    let cfg = default_cfg();
    let fwd = fwd_cfg("v0.buyhold");
    let registry =
        build_registry_for(&cfg, Some(&fwd)).expect("build_registry_for(v0.buyhold) must succeed");

    let events = registry.drain_pending_events();
    assert_eq!(events.len(), 1, "exactly one Load event");
    let loaded_id = events[0].strategy_id.0.as_str();

    assert_ne!(
        loaded_id, "sma_crossover",
        "FAIL: v0.buyhold registered 'sma_crossover' — SMA proxy regression detected"
    );
    assert_eq!(
        loaded_id, "always_long",
        "v0.buyhold must register 'always_long', got '{loaded_id}'"
    );
}

// ── Behavioural-divergence tests ──────────────────────────────────────────────

/// THE STRONG ANTI-FAKE GATE: feed the same bar sequence to both the MACD
/// forward registry and an SMA forward registry, and assert their signal/decision
/// sequences DIFFER at some point.
///
/// If both registries secretly ran SMA underneath (the pre-F5b bug), they would
/// emit identical signal kinds for every bar, and this assertion would FAIL.
///
/// The bar sequence is designed so that MACD(12,26,9) and SMA(20,50) genuinely
/// disagree: we feed a long rising price series (where SMA crossover eventually
/// fires a Buy but MACD histogram may be negative or zero during the warmup
/// phase), or a flat sequence where MACD emits nothing while SMA has crossed.
///
/// Concretely: 60 bars of slowly rising prices (1 % per bar).  After 50 bars,
/// SMA(20) > SMA(50) so SMA should emit Buy.  MACD(12,26,9) has a 26-bar fast
/// EMA, so it requires fewer bars to warm up — but its histogram also differs
/// from the SMA crossover value, so the signal sequence is demonstrably different.
///
/// If the test is brittle (both happen to agree every bar by coincidence), extend
/// with a price drop after bar 50 — MACD reacts faster than SMA(50) there.
#[test]
fn f5b_macd_registry_differs_from_sma_registry_on_same_bars() {
    let cfg = default_cfg();

    // Build the MACD forward registry (btc_macd_trend ComposedStrategy).
    let macd_fwd = fwd_cfg("v0.5.macd");
    let macd_reg = build_registry_for(&cfg, Some(&macd_fwd)).expect("MACD registry must load");

    // Build the SMA forward registry (SmaCrossover fast=20 slow=50).
    let sma_fwd = fwd_cfg("v0.sma");
    let sma_reg = build_registry_for(&cfg, Some(&sma_fwd)).expect("SMA registry must load");

    // The btc_macd_trend strategy has `close > ema(200)` in its signal, so it
    // requires at least 200 bars to warm up the EMA(200). We use 260 bars.
    //
    // Price pattern: slow linear rise (100 USDT/bar) for the first 200 bars, then
    // a steeper rise for 60 bars. This ensures:
    // - ema(200) warms up.
    // - MACD(12,26,9) generates a non-trivial histogram (fast EMA tracks the
    //   acceleration).
    // - SMA(20) > SMA(50) is true for the entire rising phase (200+) but the
    //   SMA crossover's signal timing differs from MACD's (different indicators).
    let n_bars = 260i64;
    let bars: Vec<Bar> = (0..n_bars)
        .map(|i| {
            let close = dec!(50_000) + Decimal::from(i) * dec!(100);
            make_bar(i, close)
        })
        .collect();

    // Collect all signals from both registries.
    let macd_signals: Vec<trading_core::SignalKind> = bars
        .iter()
        .flat_map(|bar| macd_reg.on_bar(bar))
        .map(|s| s.kind)
        .collect();

    let sma_signals: Vec<trading_core::SignalKind> = bars
        .iter()
        .flat_map(|bar| sma_reg.on_bar(bar))
        .map(|s| s.kind)
        .collect();

    // Sanity: each registry emitted at least one signal.
    // MACD warms up after ~200 bars (ema(200)), SMA after 50.
    assert!(
        !macd_signals.is_empty(),
        "MACD registry emitted no signals across {} bars — \
         ema(200) warmup not complete or strategy broken",
        n_bars
    );
    assert!(
        !sma_signals.is_empty(),
        "SMA registry emitted no signals — warmup too long or strategy broken"
    );

    // THE DIVERGENCE GATE: at least one bar must produce a different signal kind,
    // OR the total signal counts must differ.
    //
    // If both are secretly SMA, every output would be identical.
    // MACD warmup is longer than SMA(50): different total counts is proof.
    let diverges = if macd_signals.len() != sma_signals.len() {
        true // different warmup → different signal counts → definitively different
    } else {
        macd_signals
            .iter()
            .zip(sma_signals.iter())
            .any(|(m, s)| m != s)
    };

    assert!(
        diverges,
        "FAIL: MACD registry and SMA registry produced IDENTICAL signals for all {n_bars} bars.\n\
         This means the MACD arm is still secretly running SMA — F5b anti-fake gate TRIPPED.\n\
         MACD signals ({} total): {:?}\n\
         SMA  signals ({} total): {:?}",
        macd_signals.len(),
        macd_signals,
        sma_signals.len(),
        sma_signals,
    );
}

/// Buy-and-hold (v0.buyhold) diverges from SMA: buy-hold emits Buy on bar 1
/// then Hold forever, while SMA(20,50) emits nothing for the first 49 bars
/// (warmup) then Buy/Sell based on the crossover.
///
/// These are definitively different — no coincidence is possible.
#[test]
fn f5b_buyhold_registry_differs_from_sma_registry_on_same_bars() {
    let cfg = default_cfg();

    let buyhold_reg = build_registry_for(&cfg, Some(&fwd_cfg("v0.buyhold")))
        .expect("buy-hold registry must load");
    let sma_reg =
        build_registry_for(&cfg, Some(&fwd_cfg("v0.sma"))).expect("SMA registry must load");

    // 55 bars: enough for SMA(50) to warm up.
    let bars: Vec<Bar> = (0i64..55)
        .map(|i| make_bar(i, dec!(50_000) + Decimal::from(i) * dec!(100)))
        .collect();

    let buyhold_signals: Vec<trading_core::SignalKind> = bars
        .iter()
        .flat_map(|bar| buyhold_reg.on_bar(bar))
        .map(|s| s.kind)
        .collect();

    let sma_signals: Vec<trading_core::SignalKind> = bars
        .iter()
        .flat_map(|bar| sma_reg.on_bar(bar))
        .map(|s| s.kind)
        .collect();

    // buy-hold emits 55 signals (one per bar, starting with Buy).
    // SMA emits signals only after bar 50 (warmup).
    assert_eq!(
        buyhold_signals.len(),
        55,
        "AlwaysLong must emit one signal per bar"
    );
    // The first buy-hold signal must be Buy.
    assert_eq!(
        buyhold_signals[0],
        trading_core::SignalKind::Buy,
        "AlwaysLong must emit Buy on first bar"
    );
    // All subsequent buy-hold signals must be Hold.
    for (i, kind) in buyhold_signals.iter().enumerate().skip(1) {
        assert_eq!(
            *kind,
            trading_core::SignalKind::Hold,
            "AlwaysLong must emit Hold on bar {i} (after first)"
        );
    }

    // SMA warmup: bars 0..49 must emit NO signals (both SMA windows not yet full).
    // buy-hold emits signals from bar 0, so lengths already diverge.
    let diverges = buyhold_signals.len() != sma_signals.len()
        || buyhold_signals
            .iter()
            .zip(sma_signals.iter())
            .any(|(b, s)| b != s);

    assert!(
        diverges,
        "FAIL: buy-hold and SMA registries produced identical outputs — \
         buy-hold must be AlwaysLong, not SMA proxy"
    );
}

// ── Error-path test ───────────────────────────────────────────────────────────

/// An unknown strategy id must return Err, NOT silently fall back to SMA.
/// This is the explicit F5b anti-fake requirement.
#[test]
fn f5b_unknown_strategy_id_returns_err_not_sma_fallback() {
    let cfg = default_cfg();
    let fwd = fwd_cfg("v0.unknown_does_not_exist");
    let result = build_registry_for(&cfg, Some(&fwd));
    assert!(
        result.is_err(),
        "FAIL: unknown strategy id must return Err — \
         got Ok(registry) instead, meaning it silently fell back to SMA proxy"
    );
    let err_msg = match result {
        Err(e) => e.to_string(),
        Ok(_) => unreachable!(),
    };
    assert!(
        err_msg.contains("unknown strategy id") || err_msg.contains("unknown"),
        "error message must mention 'unknown strategy id', got: {err_msg}"
    );
}

// ── R1 (ADR-0077): 14 missing forward-buildability tests ─────────────────────
//
// Guarantees that each of the 14 arms that previously hit the `bail!` arm
// in `build_registry_for` now builds successfully.  The `None` path and
// anti-fake gate are NOT changed — both tested below (byte-identical).
//
// Test families:
// 1. ADR-0071 DSL arms (5) — donchian_break, donchian_floor, vol_breakout,
//    roc_momentum, obv — each loads its TOML from `config/strategies/`.
// 2. ADR-0067 vote ensemble arms (6) — trend_pair, tr_mr_macd_rsi,
//    tr_mr_sma_bb, any1of4, k2of4, k3of4 — routed to `build_ensemble`.
// 3. ADR-0072 DVOL regime (1) — registers `DvolRegimeStrategy` (warm-up).
// 4. ADR-0073 macro risk-on (1) — registers `AlwaysLongStrategy` (graceful
//    degradation; no corpus in forward run).
//
// Contract: each test asserts `build_registry_for` returns `Ok` (NOT `Err`).
// The `unknown` bail-gate sentinel test below still asserts `Err`.

/// ADR-0071 arm: `v0.donchian_break` must build (not bail).
#[test]
fn r1_donchian_break_builds_not_bails() {
    let cfg = default_cfg();
    let fwd = fwd_cfg("v0.donchian_break");
    let result = build_registry_for(&cfg, Some(&fwd));
    assert!(
        result.is_ok(),
        "v0.donchian_break must build — got Err: {:?}",
        result.err()
    );
    let registry = result.unwrap();
    let events = registry.drain_pending_events();
    assert_eq!(events.len(), 1, "exactly one strategy registered");
    assert_ne!(
        events[0].strategy_id.0.as_str(),
        "sma_crossover",
        "v0.donchian_break must NOT register sma_crossover (proxy regression)"
    );
}

/// ADR-0071 arm: `v0.donchian_floor` must build (not bail).
#[test]
fn r1_donchian_floor_builds_not_bails() {
    let cfg = default_cfg();
    let fwd = fwd_cfg("v0.donchian_floor");
    let result = build_registry_for(&cfg, Some(&fwd));
    assert!(
        result.is_ok(),
        "v0.donchian_floor must build — got Err: {:?}",
        result.err()
    );
    let registry = result.unwrap();
    let events = registry.drain_pending_events();
    assert_eq!(events.len(), 1, "exactly one strategy registered");
    assert_ne!(
        events[0].strategy_id.0.as_str(),
        "sma_crossover",
        "v0.donchian_floor must NOT register sma_crossover (proxy regression)"
    );
}

/// ADR-0071 arm: `v0.vol_breakout` must build (not bail).
#[test]
fn r1_vol_breakout_builds_not_bails() {
    let cfg = default_cfg();
    let fwd = fwd_cfg("v0.vol_breakout");
    let result = build_registry_for(&cfg, Some(&fwd));
    assert!(
        result.is_ok(),
        "v0.vol_breakout must build — got Err: {:?}",
        result.err()
    );
    let registry = result.unwrap();
    let events = registry.drain_pending_events();
    assert_eq!(events.len(), 1, "exactly one strategy registered");
    assert_ne!(
        events[0].strategy_id.0.as_str(),
        "sma_crossover",
        "v0.vol_breakout must NOT register sma_crossover (proxy regression)"
    );
}

/// ADR-0071 arm: `v0.roc_momentum` must build (not bail).
#[test]
fn r1_roc_momentum_builds_not_bails() {
    let cfg = default_cfg();
    let fwd = fwd_cfg("v0.roc_momentum");
    let result = build_registry_for(&cfg, Some(&fwd));
    assert!(
        result.is_ok(),
        "v0.roc_momentum must build — got Err: {:?}",
        result.err()
    );
    let registry = result.unwrap();
    let events = registry.drain_pending_events();
    assert_eq!(events.len(), 1, "exactly one strategy registered");
    assert_ne!(
        events[0].strategy_id.0.as_str(),
        "sma_crossover",
        "v0.roc_momentum must NOT register sma_crossover (proxy regression)"
    );
}

/// ADR-0071 arm: `v0.obv` must build (not bail).
#[test]
fn r1_obv_builds_not_bails() {
    let cfg = default_cfg();
    let fwd = fwd_cfg("v0.obv");
    let result = build_registry_for(&cfg, Some(&fwd));
    assert!(
        result.is_ok(),
        "v0.obv must build — got Err: {:?}",
        result.err()
    );
    let registry = result.unwrap();
    let events = registry.drain_pending_events();
    assert_eq!(events.len(), 1, "exactly one strategy registered");
    assert_ne!(
        events[0].strategy_id.0.as_str(),
        "sma_crossover",
        "v0.obv must NOT register sma_crossover (proxy regression)"
    );
}

/// ADR-0067 ensemble arm: `v0.8.vote.trend_pair` must build (not bail).
#[test]
fn r1_ensemble_trend_pair_builds_not_bails() {
    let cfg = default_cfg();
    let fwd = fwd_cfg("v0.8.vote.trend_pair");
    let result = build_registry_for(&cfg, Some(&fwd));
    assert!(
        result.is_ok(),
        "v0.8.vote.trend_pair must build — got Err: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap().len(), 1, "exactly one strategy registered");
}

/// ADR-0067 ensemble arm: `v0.8.vote.tr_mr_macd_rsi` must build (not bail).
#[test]
fn r1_ensemble_tr_mr_macd_rsi_builds_not_bails() {
    let cfg = default_cfg();
    let fwd = fwd_cfg("v0.8.vote.tr_mr_macd_rsi");
    let result = build_registry_for(&cfg, Some(&fwd));
    assert!(
        result.is_ok(),
        "v0.8.vote.tr_mr_macd_rsi must build — got Err: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap().len(), 1, "exactly one strategy registered");
}

/// ADR-0067 ensemble arm: `v0.8.vote.tr_mr_sma_bb` must build (not bail).
#[test]
fn r1_ensemble_tr_mr_sma_bb_builds_not_bails() {
    let cfg = default_cfg();
    let fwd = fwd_cfg("v0.8.vote.tr_mr_sma_bb");
    let result = build_registry_for(&cfg, Some(&fwd));
    assert!(
        result.is_ok(),
        "v0.8.vote.tr_mr_sma_bb must build — got Err: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap().len(), 1, "exactly one strategy registered");
}

/// ADR-0067 ensemble arm: `v0.8.vote.any1of4` must build (not bail).
#[test]
fn r1_ensemble_any1of4_builds_not_bails() {
    let cfg = default_cfg();
    let fwd = fwd_cfg("v0.8.vote.any1of4");
    let result = build_registry_for(&cfg, Some(&fwd));
    assert!(
        result.is_ok(),
        "v0.8.vote.any1of4 must build — got Err: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap().len(), 1, "exactly one strategy registered");
}

/// ADR-0067 ensemble arm: `v0.8.vote.k2of4` must build (not bail).
#[test]
fn r1_ensemble_k2of4_builds_not_bails() {
    let cfg = default_cfg();
    let fwd = fwd_cfg("v0.8.vote.k2of4");
    let result = build_registry_for(&cfg, Some(&fwd));
    assert!(
        result.is_ok(),
        "v0.8.vote.k2of4 must build — got Err: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap().len(), 1, "exactly one strategy registered");
}

/// ADR-0067 ensemble arm: `v0.8.vote.k3of4` must build (not bail).
#[test]
fn r1_ensemble_k3of4_builds_not_bails() {
    let cfg = default_cfg();
    let fwd = fwd_cfg("v0.8.vote.k3of4");
    let result = build_registry_for(&cfg, Some(&fwd));
    assert!(
        result.is_ok(),
        "v0.8.vote.k3of4 must build — got Err: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap().len(), 1, "exactly one strategy registered");
}

/// ADR-0072 DVOL regime arm: `v0.dvol_regime` must build (not bail).
/// Registers `DvolRegimeStrategy` with empty as_of (warm-up-only / flat).
#[test]
fn r1_dvol_regime_builds_not_bails() {
    let cfg = default_cfg();
    let fwd = fwd_cfg("v0.dvol_regime");
    let result = build_registry_for(&cfg, Some(&fwd));
    assert!(
        result.is_ok(),
        "v0.dvol_regime must build — got Err: {:?}",
        result.err()
    );
    let registry = result.unwrap();
    assert_eq!(registry.len(), 1, "exactly one strategy registered");
    let events = registry.drain_pending_events();
    assert_eq!(events.len(), 1, "exactly one Load event");
    assert_eq!(
        events[0].strategy_id.0.as_str(),
        "dvol_regime",
        "v0.dvol_regime must register DvolRegimeStrategy (id='dvol_regime')"
    );
}

/// ADR-0073 macro risk-on arm: `v0.macro_riskon` must **BAIL**, not substitute.
///
/// This test previously asserted the opposite — that the arm builds and
/// registers `AlwaysLongStrategy` "for graceful degradation". **It was asserting
/// the defect** (bug-log #81, review 3-16 CRITICAL): the bake-off path that
/// RANKS this arm degrades to 100% CASH and `AlwaysLongStrategy` is 100% COIN,
/// so the substitute is not a degradation of the macro arm but the opposite
/// strategy wearing its label. The F5b anti-fake gate in `build_registry_for`
/// exists to refuse exactly that, and it was being waved through because the id
/// was *known*.
///
/// The gate must now fire, and the error must name the arm — a silent `Err` that
/// says nothing is only marginally better than the substitution.
#[test]
fn r1_macro_riskon_bails_rather_than_substituting_a_different_strategy() {
    let cfg = default_cfg();
    let fwd = fwd_cfg("v0.macro_riskon");
    let result = build_registry_for(&cfg, Some(&fwd));
    let err = match result {
        Ok(registry) => panic!(
            "v0.macro_riskon must NOT build a registry — it has no Strategy impl and no \
             regime corpus in the forward run, so any registration is a substitute. \
             Registered {} strategy/strategies instead of bailing.",
            registry.len()
        ),
        Err(e) => format!("{e:#}"),
    };
    assert!(
        err.contains("v0.macro_riskon"),
        "the refusal must name the arm; got: {err}"
    );
    assert!(
        err.contains("CASH") && err.contains("COIN"),
        "the refusal must state WHY a substitute is not a degradation (cash vs coin); got: {err}"
    );
}

/// `forward = None` returns the default SMA registry (byte-identical headless path).
#[test]
fn f5b_no_forward_returns_default_sma_registry() {
    let cfg = default_cfg();
    let registry = build_registry_for(&cfg, None).expect("no-forward path must always succeed");

    let events = registry.drain_pending_events();
    assert_eq!(
        events.len(),
        1,
        "exactly one strategy registered in default path"
    );
    assert_eq!(
        events[0].strategy_id.0.as_str(),
        "sma_crossover",
        "no-forward path must register SmaCrossover"
    );
}
