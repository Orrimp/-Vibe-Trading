//! T515 + T714 — canonical recipe TOML files parse, typecheck, and load successfully.
//!
//! Each of the v0.5/v1 composed recipes + the v1.5a pairs recipe under
//! `config/strategies/` must:
//!   - parse and typecheck without error,
//!   - have `stage = research`,
//!   - produce a stable deterministic hash across two loads.

use std::path::PathBuf;
use strategy::composed::{ComposedStrategyConfig, Stage};

fn recipes_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR is crates/strategy — go up two levels to workspace root.
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/strategies")
}

fn load(name: &str) -> ComposedStrategyConfig {
    let path = recipes_dir().join(format!("{name}.toml"));
    ComposedStrategyConfig::from_file(&path)
        .unwrap_or_else(|e| panic!("failed to load {name}.toml: {e}"))
}

#[test]
fn t515_btc_macd_trend_loads() {
    let cfg = load("btc_macd_trend");
    assert_eq!(cfg.id.as_str(), "btc_macd_trend");
    assert_eq!(cfg.stage, Stage::Research);
}

#[test]
fn t515_btc_rsi_reversion_loads() {
    let cfg = load("btc_rsi_reversion");
    assert_eq!(cfg.id.as_str(), "btc_rsi_reversion");
    assert_eq!(cfg.stage, Stage::Research);
}

#[test]
fn t515_btc_bbands_mean_revert_loads() {
    let cfg = load("btc_bbands_mean_revert");
    assert_eq!(cfg.id.as_str(), "btc_bbands_mean_revert");
    assert_eq!(cfg.stage, Stage::Research);
}

#[test]
fn t515_hashes_are_deterministic() {
    let cfg1 = load("btc_macd_trend");
    let cfg2 = load("btc_macd_trend");
    assert_eq!(
        cfg1.hash, cfg2.hash,
        "content hash must be deterministic across two loads"
    );
}

#[test]
fn t515_all_three_hashes_distinct() {
    let h1 = load("btc_macd_trend").hash;
    let h2 = load("btc_rsi_reversion").hash;
    let h3 = load("btc_bbands_mean_revert").hash;
    assert_ne!(h1, h2, "macd and rsi hashes must differ");
    assert_ne!(h2, h3, "rsi and bbands hashes must differ");
    assert_ne!(h1, h3, "macd and bbands hashes must differ");
}

// ── T714 — v1.5a canonical pairs TOML ────────────────────────────────────────

fn load_pairs(name: &str) -> strategy::pairs::config::MeanReversionPairsConfig {
    let path = recipes_dir().join(format!("{name}.toml"));
    strategy::pairs::config::MeanReversionPairsConfig::from_file(&path)
        .unwrap_or_else(|e| panic!("failed to load {name}.toml: {e}"))
}

#[test]
fn t714_pairs_mr_h1_loads() {
    let cfg = load_pairs("pairs_mr_h1");
    assert_eq!(cfg.id.as_str(), "pairs_mr_h1");
    assert_eq!(cfg.stage.as_str(), "research");
    assert_eq!(cfg.pairs.len(), 3, "expected 3 pairs");
}

#[test]
fn t714_pairs_mr_h1_correct_params() {
    use rust_decimal_macros::dec;
    let cfg = load_pairs("pairs_mr_h1");
    assert_eq!(cfg.lookback_minutes, 60);
    assert_eq!(cfg.cooldown_minutes, 60);
    assert_eq!(cfg.z_entry, dec!(2.0));
    assert_eq!(cfg.z_exit, dec!(0.5));
    assert_eq!(cfg.z_stop, dec!(4.0));
    assert_eq!(cfg.max_staleness_minutes, 5);
    assert_eq!(cfg.exposure_cap_per_pair, dec!(0.25));
}

#[test]
fn t714_pairs_mr_h1_expected_pairs() {
    use trading_core::Symbol;
    let cfg = load_pairs("pairs_mr_h1");
    let pair_keys: Vec<_> = cfg
        .pairs
        .iter()
        .map(|p| (p.key.a.clone(), p.key.b.clone()))
        .collect();

    // Verify expected pairs are present.
    assert!(
        pair_keys.contains(&(Symbol::new("BTCUSDT"), Symbol::new("ETHUSDT"))),
        "BTC/ETH pair missing"
    );
    assert!(
        pair_keys.contains(&(Symbol::new("ETHUSDT"), Symbol::new("SOLUSDT"))),
        "ETH/SOL pair missing"
    );
    assert!(
        pair_keys.contains(&(Symbol::new("BNBUSDT"), Symbol::new("BTCUSDT"))),
        "BNB/BTC pair missing"
    );
}

#[test]
fn t714_pairs_mr_h1_hash_deterministic() {
    use smol_str::SmolStr;
    use strategy::pairs::mean_reversion::MeanReversionPairsStrategy;

    let cfg1 = load_pairs("pairs_mr_h1");
    let cfg2 = load_pairs("pairs_mr_h1");
    let s1 = MeanReversionPairsStrategy::from_config(cfg1, SmolStr::new("pairs_mr_h1.toml"));
    let s2 = MeanReversionPairsStrategy::from_config(cfg2, SmolStr::new("pairs_mr_h1.toml"));
    assert_eq!(
        s1.hash, s2.hash,
        "content hash must be deterministic across two loads"
    );
}
