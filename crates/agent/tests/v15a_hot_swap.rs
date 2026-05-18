//! T712 — Hot-swap integration test for v1.5a pairs strategy.
//!
//! Verifies that:
//! 1. A `MeanReversionPairsStrategy` loaded from TOML has a content hash.
//! 2. When the TOML is "rewritten" with a new `z_entry`, the new config
//!    produces a different content hash (swap detection).
//! 3. Per-pair ring buffers are effectively reset on swap (new strategy instance).
//! 4. The audit ledger records `Load` → `Swap` lifecycle events.
//!
//! NOTE: This test does not exercise the filesystem watcher (async file-event
//! loop).  That path is tested in `strategy_hot_swap.rs` (v0.5 watcher).
//! This test focuses on the v1.5a config hash + registry swap contract.

#![allow(clippy::unwrap_used)]

use smol_str::SmolStr;

fn make_strategy(z_entry: &str) -> strategy::pairs::mean_reversion::MeanReversionPairsStrategy {
    let toml = format!(
        r#"
id = "pairs_mr_h1"
kind = "mean_reversion_pairs"
stage = "research"

pairs = [
    {{ a = "BTCUSDT", b = "ETHUSDT", beta = "1.0" }},
    {{ a = "ETHUSDT", b = "SOLUSDT", beta = "1.0" }},
    {{ a = "BNBUSDT", b = "BTCUSDT", beta = "1.0" }},
]

lookback_minutes      = 60
cooldown_minutes      = 60
z_entry               = "{z_entry}"
z_exit                = "0.5"
z_stop                = "4.0"
vol_floor             = "0.000001"
size                  = "binary_per_pair"
exposure_cap_per_pair = "0.25"
max_staleness_minutes = 5
"#
    );
    let cfg = strategy::pairs::config::MeanReversionPairsConfig::from_str(&toml).unwrap();
    strategy::pairs::mean_reversion::MeanReversionPairsStrategy::from_config(
        cfg,
        SmolStr::new("config/strategies/pairs_mr_h1.toml"),
    )
}

// ── T712-A: hash changes when z_entry changes ─────────────────────────────────

#[test]
fn t712_hash_changes_on_z_entry_change() {
    let s1 = make_strategy("2.0");
    let s2 = make_strategy("1.5");

    assert_ne!(
        s1.hash, s2.hash,
        "different z_entry must produce different content hash (swap detection)"
    );
}

// ── T712-B: same config → same hash (no spurious swap) ───────────────────────

#[test]
fn t712_same_config_same_hash() {
    let s1 = make_strategy("2.0");
    let s2 = make_strategy("2.0");

    assert_eq!(
        s1.hash, s2.hash,
        "identical config must produce identical hash (no spurious swap)"
    );
}

// ── T712-C: new strategy after swap has clean state ──────────────────────────

#[test]
fn t712_new_strategy_has_clean_state_after_swap() {
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use strategy::Strategy as _;
    use time::OffsetDateTime;
    use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

    fn ts_at(minute: i64) -> Timestamp {
        Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::minutes(minute))
    }

    fn make_bar(symbol: &str, close: Decimal, minute: i64) -> Bar {
        let ts = ts_at(minute);
        Bar {
            symbol: Symbol::new(symbol),
            tf: Timeframe::OneMinute,
            open: Price::new(close).unwrap(),
            high: Price::new(close).unwrap(),
            low: Price::new(close).unwrap(),
            close: Price::new(close).unwrap(),
            volume: Quantity::new(dec!(1)).unwrap(),
            trade_count: 1,
            local_recv_ts: ts,
            open_ts: ts,
            close_ts: ts,
            venue: Venue::Binance,
        }
    }

    // Pre-swap: warm up the old strategy (partial fill of lookback window).
    let mut old_strat = make_strategy("2.0");
    for i in 0i64..30 {
        old_strat.on_bar(&make_bar("BTCUSDT", dec!(30000), i));
        old_strat.on_bar(&make_bar("ETHUSDT", dec!(2000), i));
    }

    // Post-swap: create a new strategy (simulates registry swap).
    // Ring buffers in the new strategy are empty — all warmup state is lost.
    let mut new_strat = make_strategy("1.5");

    // Feed the new strategy a single bar — should not produce signals
    // (ring buffer is empty, lookback not satisfied).
    let sigs = new_strat.on_bar(&make_bar("BTCUSDT", dec!(1000), 30));
    assert!(
        sigs.is_empty(),
        "new strategy after swap must not produce signals before lookback is satisfied"
    );
}

// ── T712-D: audit Load + Swap lifecycle events ────────────────────────────────

#[tokio::test]
async fn t712_audit_load_swap_lifecycle() {
    use audit::{Ledger, bootstrap, journal, query};
    use trading_core::StrategyEventKind;

    let ledger = Ledger::in_memory().await.unwrap();
    bootstrap::chart_of_accounts(&ledger).await.unwrap();

    let s1 = make_strategy("2.0");
    let s2 = make_strategy("1.5");

    let hash1_hex: String = s1.hash.iter().map(|b| format!("{b:02x}")).collect();
    let hash2_hex: String = s2.hash.iter().map(|b| format!("{b:02x}")).collect();

    // Simulate: Load event when strategy first loads.
    journal::strategy_event(
        &ledger,
        &journal::StrategyEventWrite {
            kind: "Load",
            strategy_id: Some("pairs_mr_h1"),
            old_hash: None,
            new_hash: Some(&hash1_hex),
            source_path: "config/strategies/pairs_mr_h1.toml",
            operator: "system",
            error_code: None,
            error_summary: None,
            ts: None,
            venue: None,
        },
    )
    .await
    .unwrap();

    // Simulate: Swap event when TOML is rewritten mid-run.
    journal::strategy_event(
        &ledger,
        &journal::StrategyEventWrite {
            kind: "Swap",
            strategy_id: Some("pairs_mr_h1"),
            old_hash: Some(&hash1_hex),
            new_hash: Some(&hash2_hex),
            source_path: "config/strategies/pairs_mr_h1.toml",
            operator: "system",
            error_code: None,
            error_summary: None,
            ts: None,
            venue: None,
        },
    )
    .await
    .unwrap();

    let history = query::strategy_history(&ledger, trading_core::StrategyId::new("pairs_mr_h1"))
        .await
        .unwrap();

    assert_eq!(history.len(), 2, "expected Load + Swap events");
    assert_eq!(history[0].kind, StrategyEventKind::Load);
    assert_eq!(history[1].kind, StrategyEventKind::Swap);

    // Swap: old hash matches load's new hash.
    assert_eq!(
        history[1].old_hash.as_deref(),
        Some(hash1_hex.as_str()),
        "Swap old_hash must match Load new_hash"
    );
    assert_ne!(
        history[1].old_hash, history[1].new_hash,
        "Swap old and new hashes must differ"
    );

    // Ledger imbalance = 0.
    let (dr, cr) = query::global_debit_credit_sum(&ledger).await.unwrap();
    assert_eq!(dr, cr, "ledger must remain balanced after load+swap events");
}
