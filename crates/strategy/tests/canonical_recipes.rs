//! T515 — canonical recipe TOML files parse, typecheck, and load successfully.
//!
//! Each of the three canonical recipes under `config/strategies/` must:
//!   - parse and typecheck without error,
//!   - have `stage = research`,
//!   - produce a stable deterministic hash across two loads.

use std::path::PathBuf;
use strategy::composed::{ComposedStrategyConfig, Stage};

fn recipes_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR is crates/strategy — go up two levels to workspace root.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../config/strategies")
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
    assert_eq!(cfg1.hash, cfg2.hash, "content hash must be deterministic across two loads");
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
