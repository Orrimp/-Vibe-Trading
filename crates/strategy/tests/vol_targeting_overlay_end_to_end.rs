//! R2 end-to-end regression test (v3-volatility-forecaster-noop-fix).
//!
//! Asserts that `VolTargetingOverlay::quantity_scale(symbol)` returns
//! the cached per-symbol scale factor from the most recent `on_bar`,
//! and that a `scale != 1.0` query result differs from a `scale = 1.0`
//! query result by a testable epsilon (>= 0.01).
//!
//! This is the gate that would have caught the v0.1.0 / v0.1.0-rebaseline
//! no-op: under the pre-fix code, `quantity_scale` returns the default
//! `1.0` regardless of the GARCH state, so the assertion at the bottom
//! FAILS. Under the fix, the cache populates and the assertion PASSES.
//!
//! # Forensic gate
//!
//! Run this test against current main BEFORE the fix lands (T-D-N3a).
//! Expected: assertion failed with
//! 'vol-target overlay produced scale=1 after 5 on_bar calls — expected != 1.0
//! (no-op signature)' — the trait's default `1.0` is what gets returned.
//! After the fix lands, the test passes.
//!
//! # Cross-references
//!
//! - feature.md § R2 — end-to-end equity-divergence regression.
//! - feature.md § R6 — unit + integration guards.
//! - decomp.md § T-AR-4 — this file's shape.

use std::collections::BTreeMap;

use rust_decimal_macros::dec;
use strategy::{
    GarchParams, MomentumStrategy, Strategy, VolTargetingConfig, VolTargetingOverlay,
    cross_sectional::CrossSectionalMomentumConfig,
};
use time::OffsetDateTime;
use trading_core::symbol::Symbol;
use trading_core::{Bar, Price, Quantity, Timeframe, Timestamp, Venue};

// ── Helper builders ───────────────────────────────────────────────────────────

/// Parse an inline TOML for the 10-symbol momentum config used in tests.
fn stub_momentum() -> MomentumStrategy {
    let toml = r#"
id    = "top10_momentum_h1"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT", "XRPUSDT",
            "ADAUSDT", "DOGEUSDT", "AVAXUSDT", "DOTUSDT", "LINKUSDT"]
lookback_minutes  = 60
rebalance_minutes = 60
k_long  = 3
k_short = 0
exposure_cap               = 0.50
drift_rebalance_threshold  = 0.10
vol_floor                  = 0.000001
size = "equal_weight"
"#;
    let cfg = CrossSectionalMomentumConfig::from_str(toml).expect("valid stub config");
    MomentumStrategy::from_config(cfg, smol_str::SmolStr::new("stub"))
}

/// GARCH params rigged so that omega + alpha + beta = 0.95 (stable),
/// with low unconditional_var → init_sigma is small → compute_scale
/// (target_vol / sigma_hat) hits the clamp_max = 2.0.
fn high_scale_model() -> GarchParams {
    // omega = 1e-10, alpha = 0.05, beta = 0.90 → stationary (sum = 0.95)
    // unconditional_var = omega / (1 - alpha - beta) = 1e-10 / 0.05 = 2e-9
    // init_sigma = sqrt(2e-9) ≈ 4.47e-5  << target_vol=0.02 → scale hits clamp_max=2.0
    GarchParams {
        omega: 1e-10,
        alpha: 0.05,
        beta: 0.90,
        unconditional_var: 1e-10 / (1.0 - 0.05 - 0.90),
    }
}

/// Build a minimal bar with the given symbol, unix timestamp offset (seconds), and close price.
fn make_bar(symbol: &str, ts_offset_secs: i64, close: rust_decimal::Decimal) -> Bar {
    let ts = Timestamp::new(
        OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(1_700_000_000 + ts_offset_secs),
    );
    Bar {
        symbol: Symbol::new(symbol),
        tf: Timeframe::OneHour,
        open_ts: ts,
        close_ts: ts,
        local_recv_ts: ts,
        venue: Venue::Binance,
        open: Price::new(close).unwrap(),
        high: Price::new(close).unwrap(),
        low: Price::new(close).unwrap(),
        close: Price::new(close).unwrap(),
        volume: Quantity::new(dec!(1.0)).unwrap(),
        trade_count: 1,
    }
}

// ── Forensic gate test ────────────────────────────────────────────────────────

#[test]
fn overlay_quantity_scale_reflects_computed_factor() {
    let inner = stub_momentum();
    let mut models = BTreeMap::new();
    models.insert("BTCUSDT".to_string(), high_scale_model());

    let mut overlay = VolTargetingOverlay::new(
        inner,
        models,
        VolTargetingConfig::default(), // target_vol = 0.02
    );

    let btc = Symbol::new("BTCUSDT");

    // Pre-on_bar query → default 1.0 (no cache entry yet).
    let scale_before = overlay.quantity_scale(&btc);
    assert_eq!(
        scale_before, 1.0,
        "default-on-miss must be 1.0 (no on_bar yet)"
    );

    // Drive on_bar with a sequence of bars on BTCUSDT.
    // With high_scale_model, sigma_hat stays tiny → compute_scale hits clamp_max = 2.0.
    for i in 0..5_i64 {
        let bar = make_bar("BTCUSDT", i * 3600, dec!(50_000.0));
        let _signals = overlay.on_bar(&bar);
    }

    // Post-on_bar query → cached scale (expected ~2.0, the clamp_max).
    let scale_after = overlay.quantity_scale(&btc);
    assert!(
        (scale_after - 1.0).abs() >= 0.01,
        "vol-target overlay produced scale={scale_after} after 5 on_bar calls — \
         expected != 1.0 (no-op signature). This is the R2 forensic gate; \
         under the pre-fix code this assertion FAILS because quantity_scale \
         returns the default 1.0 regardless of GARCH state."
    );
    assert!(
        (scale_after - 2.0).abs() < 0.01,
        "expected scale ~= 2.0 (clamp_max for low-sigma regime); got {scale_after}"
    );

    // Symbol not in the GARCH checkpoint → default 1.0 (no cache write).
    let eth = Symbol::new("ETHUSDT");
    let scale_eth = overlay.quantity_scale(&eth);
    assert_eq!(scale_eth, 1.0, "no-model symbol must inherit default 1.0");
}
