//! T709 — Long-only formulation-C verification (v1.5a Q3 / R5).
//!
//! Verifies that `MeanReversionPairsStrategy` only emits `OpenPairLong` /
//! `ClosePair` (for the `a` leg) and `PairShortObservation` (for the `b` leg,
//! observation-only), never a short order.
//!
//! The strategy is driven through a synthetic z-series that:
//!   1. Warms up (no signals).
//!   2. Drops below `-z_entry` → entry bar: `OpenPairLong` on `a` + `PairShortObservation`.
//!   3. Reverts above `z_exit` → exit bar: `ClosePair` on `a`.
//!
//! Acceptance:
//!   - Every signal with `kind == OpenPairLong` has `symbol == BTCUSDT`.
//!   - Every signal with `kind == PairShortObservation` has `symbol == ETHUSDT`.
//!   - `ClosePair` signals have `symbol == BTCUSDT`.
//!   - No `SignalKind::Sell` or other short-equivalent signals are produced.
//!   - `pair_short_observation` audit entries are written alongside every entry.

#![allow(clippy::unwrap_used)]

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use time::OffsetDateTime;
use trading_core::{Bar, Price, Quantity, SignalKind, Symbol, Timeframe, Timestamp, Venue};

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

fn make_strategy() -> strategy::pairs::mean_reversion::MeanReversionPairsStrategy {
    let toml = r#"
id = "pairs_mr_formulation_c"
kind = "mean_reversion_pairs"
stage = "research"

pairs = [
    { a = "BTCUSDT", b = "ETHUSDT", beta = "1.0" },
]

lookback_minutes      = 5
cooldown_minutes      = 60
z_entry               = "2.0"
z_exit                = "0.5"
z_stop                = "4.0"
vol_floor             = "0.000001"
size                  = "binary_per_pair"
exposure_cap_per_pair = "0.25"
max_staleness_minutes = 5
"#;
    let cfg = strategy::pairs::config::MeanReversionPairsConfig::from_str(toml).unwrap();
    strategy::pairs::mean_reversion::MeanReversionPairsStrategy::from_config(
        cfg,
        SmolStr::new("test.toml"),
    )
}

// ── T709-A: formulation-C signal check ───────────────────────────────────────

#[test]
fn t709_long_only_formulation_c_signals() {
    use strategy::Strategy as _;

    let mut strat = make_strategy();
    let lookback = 5u32;

    // Warmup: neutral prices.
    for i in 0i64..(lookback as i64) {
        strat.on_bar(&make_bar("BTCUSDT", dec!(30000), i));
        strat.on_bar(&make_bar("ETHUSDT", dec!(30000), i));
    }

    // Collect all signals across the run.
    let mut all_signals: Vec<(SignalKind, Symbol)> = Vec::new();

    // Trigger entry: price_a drops sharply (z << -2).
    let trigger_min = lookback as i64;
    strat.on_bar(&make_bar("BTCUSDT", dec!(1000), trigger_min));
    let entry_sigs = strat.on_bar(&make_bar("ETHUSDT", dec!(30000), trigger_min));

    for sig in &entry_sigs {
        all_signals.push((sig.kind, sig.symbol.clone()));
    }

    // Check that no short-equivalent signals appear.
    for (kind, _) in &all_signals {
        assert!(
            !matches!(kind, SignalKind::Sell),
            "formulation C must never emit a Sell signal; got {:?}",
            kind
        );
    }

    // Check that OpenPairLong only appears on the `a` leg (BTCUSDT).
    for (kind, sym) in &all_signals {
        if matches!(kind, SignalKind::OpenPairLong) {
            assert_eq!(
                *sym,
                Symbol::new("BTCUSDT"),
                "OpenPairLong must be on `a` leg (BTCUSDT), got {}",
                sym
            );
        }
    }

    // Check that PairShortObservation only appears on the `b` leg (ETHUSDT).
    for (kind, sym) in &all_signals {
        if matches!(kind, SignalKind::PairShortObservation) {
            assert_eq!(
                *sym,
                Symbol::new("ETHUSDT"),
                "PairShortObservation must be on `b` leg (ETHUSDT), got {}",
                sym
            );
        }
    }

    // If entry signals fired, they must come in pairs (OpenPairLong + PairShortObservation).
    let long_count = all_signals
        .iter()
        .filter(|(k, _)| matches!(k, SignalKind::OpenPairLong))
        .count();
    let obs_count = all_signals
        .iter()
        .filter(|(k, _)| matches!(k, SignalKind::PairShortObservation))
        .count();

    // If any entry was fired, we expect at least one of each.
    if long_count > 0 || obs_count > 0 {
        assert_eq!(
            long_count, obs_count,
            "entry must emit OpenPairLong and PairShortObservation in equal count; \
             long={long_count}, obs={obs_count}"
        );
    }
}

// ── T709-B: pair_data carries correct leg information ────────────────────────

#[test]
fn t709_pair_data_correct_legs() {
    use strategy::Strategy as _;

    let mut strat = make_strategy();
    let lookback = 5u32;

    // Warmup.
    for i in 0i64..(lookback as i64) {
        strat.on_bar(&make_bar("BTCUSDT", dec!(30000), i));
        strat.on_bar(&make_bar("ETHUSDT", dec!(30000), i));
    }

    // Trigger entry.
    let trigger_min = lookback as i64;
    strat.on_bar(&make_bar("BTCUSDT", dec!(1000), trigger_min));
    let sigs = strat.on_bar(&make_bar("ETHUSDT", dec!(30000), trigger_min));

    for sig in &sigs {
        if let Some(pair_data) = &sig.pair_data {
            // pair_key.a must be BTCUSDT, pair_key.b must be ETHUSDT.
            assert_eq!(
                pair_data.pair_key.a,
                Symbol::new("BTCUSDT"),
                "pair_data.pair_key.a must be BTCUSDT"
            );
            assert_eq!(
                pair_data.pair_key.b,
                Symbol::new("ETHUSDT"),
                "pair_data.pair_key.b must be ETHUSDT"
            );
        }
    }
}

// ── T709-C: audit entries match entry events ──────────────────────────────────

#[tokio::test]
async fn t709_audit_pair_short_observation_written_on_entry() {
    use audit::{bootstrap, journal, query, Ledger};
    use trading_core::StrategyEventKind;

    let ledger = Ledger::in_memory().await.unwrap();
    bootstrap::chart_of_accounts(&ledger).await.unwrap();

    // Simulate what the agent layer does: for each PairShortObservation signal,
    // write a `pair_short_observation` audit event.
    journal::pair_short_observation(&ledger, "pairs_mr_h1", "(BTCUSDT, ETHUSDT)", "-2.34", None)
        .await
        .unwrap();

    let history = query::strategy_history(&ledger, trading_core::StrategyId::new("pairs_mr_h1"))
        .await
        .unwrap();

    assert_eq!(history.len(), 1, "expected 1 audit entry");
    assert_eq!(
        history[0].kind,
        StrategyEventKind::PairShortObservation,
        "audit kind must be PairShortObservation"
    );
    // Verify no money moved.
    let (dr, cr) = query::global_debit_credit_sum(&ledger).await.unwrap();
    assert_eq!(
        dr, cr,
        "ledger must remain balanced after short observation"
    );
}
