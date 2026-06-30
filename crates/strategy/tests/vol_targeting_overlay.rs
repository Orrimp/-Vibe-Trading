//! Integration tests for `VolTargetingOverlay` (T-D-N21, R11.6).
//!
//! Verifies:
//!  1. Overlay wraps inner `MomentumStrategy` (strategy ID is overlay ID, not inner ID).
//!  2. Scale clamp invariants: scale stays in `[scale_clamp_min, scale_clamp_max]`.
//!  3. Zero-sigma defensive guard: very small sigma_hat is floored → scale == clamp_max.
//!  4. `compute_scale(target_vol)` returns 1.0.
//!  5. `on_bar` without a GARCH model → bars_no_model counter increments.
//!  6. `GarchParams::init_sigma` is positive and deterministic.
//!  7. `GarchParams::forecast_step(0,0)` floors to `sqrt(omega)`.
//!
//! No `--features forecast` required — these tests do not load a checkpoint.
//!
//! # Cross-references
//!
//! - `crates/strategy/src/vol_targeting_overlay.rs` — the implementation.
//! - ADR-0038 § D5 — composition lock.

use std::collections::BTreeMap;

use rust_decimal::Decimal;
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

/// Minimal GARCH params — unconditional_var drives init_sigma().
fn stub_model() -> GarchParams {
    GarchParams {
        omega: 1e-6,
        alpha: 0.10,
        beta: 0.85,
        unconditional_var: 1e-6 / (1.0 - 0.10 - 0.85), // = 2e-5
    }
}

/// Build a minimal bar with the given symbol and close price.
fn make_bar(sym: &str, close: Decimal, offset_minutes: i64) -> Bar {
    let ts = Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::minutes(offset_minutes));
    Bar {
        symbol: Symbol::new(sym),
        tf: Timeframe::OneMinute,
        open: Price::new(close).unwrap(),
        high: Price::new(close).unwrap(),
        low: Price::new(close).unwrap(),
        close: Price::new(close).unwrap(),
        volume: Quantity::new(dec!(100)).unwrap(),
        trade_count: 1,
        local_recv_ts: ts,
        open_ts: ts,
        close_ts: ts,
        venue: Venue::Binance,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// R11.6 — overlay wraps inner: strategy ID is the overlay ID, not the inner ID.
#[test]
fn overlay_id_differs_from_inner_id() {
    let models: BTreeMap<String, GarchParams> = BTreeMap::new();
    let overlay = VolTargetingOverlay::new(stub_momentum(), models, VolTargetingConfig::default());
    let oid = overlay.id();
    assert_ne!(
        oid.0.as_str(),
        "top10_momentum_h1",
        "overlay ID must differ from inner ID"
    );
    assert_eq!(
        oid.0.as_str(),
        "vol_targeting_overlay_momentum",
        "overlay ID must be vol_targeting_overlay_momentum"
    );
}

/// R11.6 scale clamp invariants — `compute_scale` with tiny sigma hits clamp_max.
#[test]
fn scale_clamp_invariant_tiny_sigma() {
    let cfg = VolTargetingConfig::default();
    let overlay = VolTargetingOverlay::new(stub_momentum(), BTreeMap::new(), cfg.clone());
    // Tiny sigma → should hit clamp_max.
    let scale = overlay.compute_scale(1e-30);
    assert_eq!(
        scale, cfg.scale_clamp_max,
        "tiny sigma: scale must be clamped to {}, got {scale}",
        cfg.scale_clamp_max
    );
}

/// R11.6 scale clamp invariants — huge sigma hits clamp_min.
#[test]
fn scale_clamp_invariant_huge_sigma() {
    let cfg = VolTargetingConfig::default();
    let overlay = VolTargetingOverlay::new(stub_momentum(), BTreeMap::new(), cfg.clone());
    // Huge sigma → should hit clamp_min.
    let scale = overlay.compute_scale(1_000.0);
    assert_eq!(
        scale, cfg.scale_clamp_min,
        "huge sigma: scale must be clamped to {}, got {scale}",
        cfg.scale_clamp_min
    );
}

/// R11.6 zero-sigma defensive guard — sigma_hat == 0.0 floors via min_sigma_floor
/// and returns `scale_clamp_max`.
#[test]
fn zero_sigma_defensive_guard() {
    let cfg = VolTargetingConfig {
        target_vol: 0.02,
        scale_clamp_min: 0.5,
        scale_clamp_max: 2.0,
        min_sigma_floor: 1e-8,
        ..VolTargetingConfig::default()
    };
    let overlay = VolTargetingOverlay::new(stub_momentum(), BTreeMap::new(), cfg.clone());
    // Exactly 0.0 → floored to min_sigma_floor → scale = 0.02/1e-8 = 2e6 >> clamp_max.
    let scale = overlay.compute_scale(0.0);
    assert_eq!(
        scale, cfg.scale_clamp_max,
        "sigma=0.0 must floor to clamp_max={}, got {scale}",
        cfg.scale_clamp_max
    );
}

/// R11.6 scale at target_vol returns exactly 1.0.
#[test]
fn scale_at_target_vol_is_one() {
    let cfg = VolTargetingConfig::default();
    let overlay = VolTargetingOverlay::new(stub_momentum(), BTreeMap::new(), cfg.clone());
    let scale = overlay.compute_scale(cfg.target_vol);
    assert!(
        (scale - 1.0).abs() < 1e-9,
        "scale at target_vol must be 1.0, got {scale}"
    );
}

/// R11.6 overlay wraps inner: on_bar without a GARCH model increments bars_no_model.
#[test]
fn on_bar_no_model_increments_counter() {
    let models: BTreeMap<String, GarchParams> = BTreeMap::new();
    let mut overlay =
        VolTargetingOverlay::new(stub_momentum(), models, VolTargetingConfig::default());

    // Feed 10 symbols × 10 rounds.
    let symbols = [
        "BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT", "XRPUSDT", "ADAUSDT", "DOGEUSDT", "AVAXUSDT",
        "DOTUSDT", "LINKUSDT",
    ];
    let prices = [
        dec!(50000),
        dec!(2000),
        dec!(300),
        dec!(100),
        dec!(1),
        dec!(0.5),
        dec!(0.08),
        dec!(20),
        dec!(5),
        dec!(15),
    ];
    for round in 0..10_i64 {
        for (i, sym) in symbols.iter().enumerate() {
            let bar = make_bar(sym, prices[i], round * 10 + i as i64);
            let _ = overlay.on_bar(&bar);
        }
    }

    assert!(
        overlay.stats.bars_no_model > 0,
        "bars_no_model should be > 0 when no GARCH model is registered"
    );
    assert_eq!(
        overlay.stats.bars_total, 100,
        "10 symbols × 10 rounds = 100 bars"
    );
}

/// R11.6 GarchParams::init_sigma is positive and deterministic.
#[test]
fn garch_params_init_sigma_positive() {
    let m = stub_model();
    let s1 = m.init_sigma();
    let s2 = m.init_sigma();
    assert!(s1 > 0.0, "init_sigma must be positive, got {s1}");
    assert_eq!(s1, s2, "init_sigma must be deterministic");
}

/// R11.6 GarchParams::forecast_step(0,0) floors to sqrt(omega).
#[test]
fn garch_params_forecast_step_floored() {
    let m = stub_model();
    let sigma = m.forecast_step(0.0, 0.0);
    assert!(
        sigma > 0.0,
        "forecast_step(0,0) must be positive via omega floor, got {sigma}"
    );
    let expected = m.omega.sqrt();
    assert!(
        (sigma - expected).abs() < 1e-12,
        "expected sqrt(omega)={expected}, got {sigma}"
    );
}

/// R6 — scale_cache populates after on_bar is called.
///
/// After at least one `on_bar` call for a symbol with a GARCH model rigged to
/// produce sigma << target_vol (so scale hits clamp_max = 2.0), `quantity_scale`
/// must return a value != 1.0.
#[test]
fn scale_cache_populates_after_on_bar() {
    // Rig a model where sigma_hat is tiny → compute_scale hits clamp_max = 2.0.
    let mut models = BTreeMap::new();
    models.insert(
        "BTCUSDT".to_string(),
        GarchParams {
            omega: 1e-10,
            alpha: 0.05,
            beta: 0.90,
            unconditional_var: 1e-10 / (1.0 - 0.05 - 0.90),
        },
    );
    let mut overlay =
        VolTargetingOverlay::new(stub_momentum(), models, VolTargetingConfig::default());
    let btc = Symbol::new("BTCUSDT");

    // Before any on_bar: default 1.0.
    assert_eq!(
        overlay.quantity_scale(&btc),
        1.0,
        "quantity_scale must return 1.0 before any on_bar"
    );

    // Feed one bar to populate the cache.
    let bar = make_bar("BTCUSDT", dec!(50_000), 0);
    let _ = overlay.on_bar(&bar);

    // After on_bar: scale should be clamp_max = 2.0 (sigma tiny → scale big).
    let scale = overlay.quantity_scale(&btc);
    assert!(
        (scale - 1.0).abs() >= 0.01,
        "scale_cache should reflect computed factor != 1.0 after on_bar; got {scale}"
    );
    assert!(
        (scale - 2.0).abs() < 0.01,
        "expected scale ~= 2.0 (clamp_max); got {scale}"
    );
}

/// R6 — quantity_scale returns 1.0 for a symbol not in the GARCH checkpoint.
///
/// When `on_bar` is called for a symbol that has no GARCH model, the no-model
/// branch fires (`bars_no_model` ticks) and `scale_cache` records scale = 1.0
/// (because sigma_hat = target_vol → compute_scale = 1.0). Calling
/// `quantity_scale` for a completely different symbol that was never seen
/// must also return 1.0 (the default-on-miss path).
#[test]
fn quantity_scale_default_for_unseen_symbol() {
    // Register a model only for BTCUSDT.
    let mut models = BTreeMap::new();
    models.insert("BTCUSDT".to_string(), stub_model());
    let mut overlay =
        VolTargetingOverlay::new(stub_momentum(), models, VolTargetingConfig::default());

    // Feed a bar for BTCUSDT.
    let bar = make_bar("BTCUSDT", dec!(50_000), 0);
    let _ = overlay.on_bar(&bar);

    // ETHUSDT was never given an on_bar call → cache miss → must return 1.0.
    let eth = Symbol::new("ETHUSDT");
    assert_eq!(
        overlay.quantity_scale(&eth),
        1.0,
        "unseen symbol must return default 1.0 from quantity_scale"
    );
}
