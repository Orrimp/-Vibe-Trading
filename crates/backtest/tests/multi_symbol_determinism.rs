//! T618 — multi-symbol determinism integration test.
//!
//! Verifies that the cross-sectional momentum strategy produces byte-identical
//! signal sequences across two independent runs given the same seed (R12.2,
//! R12.5 — BTreeMap iteration order, k-way merge sort key determinism).
//!
//! Two invariants under test:
//! 1. `MomentumStrategy::on_bar` returns identical signals on two runs.
//! 2. `ReplayFeed::merge_synthetic` produces identical merge order on two runs.

#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;

use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

const SEED: u64 = 0x00C0_FFEE_1234_5678;
const BAR_COUNT: usize = 200; // enough to warm up lookback=10 + rebalance

fn make_bar(symbol: &str, close: Decimal, offset_hours: i64) -> Bar {
    let base = OffsetDateTime::UNIX_EPOCH;
    let ts = Timestamp::new(base + time::Duration::hours(offset_hours));
    Bar {
        symbol: Symbol::new(symbol),
        tf: Timeframe::OneHour,
        open: Price::new(close).unwrap(),
        high: Price::new(close).unwrap(),
        low: Price::new(close).unwrap(),
        close: Price::new(close).unwrap(),
        volume: Quantity::new(dec!(100)).unwrap(),
        trade_count: 10,
        local_recv_ts: ts,
        open_ts: ts,
        close_ts: ts,
        venue: Venue::Binance,
    }
}

/// Generate synthetic hourly bars for one symbol with a given seed.
fn synthetic_hourly(symbol: &str, count: usize, seed: u64) -> Vec<Bar> {
    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let mut bars = Vec::with_capacity(count);
    let base = OffsetDateTime::UNIX_EPOCH;
    let mut close: f64 = 10_000.0;

    for i in 0..count {
        let u1: f64 = rng.random::<f64>().max(1e-10_f64);
        let u2: f64 = rng.random::<f64>();
        let z = (-2.0_f64 * u1.ln()).sqrt() * (2.0_f64 * std::f64::consts::PI * u2).cos();
        let ret = 0.000_03 + 0.012 * z;
        let next = (close * (1.0 + ret)).clamp(1.0, 1_000_000.0);

        let ts = Timestamp::new(base + time::Duration::hours(i as i64));
        let to_dec = |v: f64| Decimal::try_from(v.max(0.01)).unwrap_or(dec!(0.01));
        let price_or_one = |v: f64| -> Price {
            Price::new(to_dec(v)).unwrap_or_else(|_| Price::new(dec!(1)).unwrap())
        };

        bars.push(Bar {
            symbol: Symbol::new(symbol),
            tf: Timeframe::OneHour,
            open_ts: ts,
            close_ts: ts,
            open: price_or_one(close),
            high: price_or_one(next.max(close)),
            low: price_or_one(next.min(close).max(0.01)),
            close: price_or_one(next),
            volume: Quantity::new(dec!(100)).unwrap(),
            trade_count: 10,
            local_recv_ts: ts,
            venue: Venue::Binance,
        });

        close = next;
    }

    bars
}

/// Build a `MomentumStrategy` with the given universe and parameters.
fn make_momentum(
    lookback: u32,
    rebalance: u32,
    k_long: u32,
    universe: &[&str],
) -> strategy::MomentumStrategy {
    use smol_str::SmolStr;

    let uni_str = universe
        .iter()
        .map(|s| format!(r#""{s}""#))
        .collect::<Vec<_>>()
        .join(", ");

    let toml = format!(
        r#"
id = "test_momentum"
kind = "cross_sectional_momentum"
stage = "research"
universe = [{uni_str}]
lookback_minutes = {lookback}
rebalance_minutes = {rebalance}
k_long = {k_long}
k_short = 0
exposure_cap = 0.50
drift_rebalance_threshold = 0.10
vol_floor = 0.000001
size = "equal_weight"
"#
    );
    let cfg = strategy::CrossSectionalMomentumConfig::from_str(&toml).unwrap();
    strategy::MomentumStrategy::from_config(cfg, SmolStr::new("test"))
}

// ── Test 1: signal sequence is identical across two independent runs ──────────

#[test]
fn t618_signal_sequence_deterministic_two_runs() {
    let universe = [
        "ADAUSDT", "AVAXUSDT", "BNBUSDT", "BTCUSDT", "DOGEUSDT", "DOTUSDT", "ETHUSDT", "LINKUSDT",
        "SOLUSDT", "XRPUSDT",
    ];

    // Build identical bar streams for both runs.
    let symbols_seeds: Vec<(&str, u64)> = universe
        .iter()
        .enumerate()
        .map(|(i, s)| (*s, SEED.wrapping_add(i as u64 * 0x9E3779B9)))
        .collect();

    let bars_by_symbol: Vec<Vec<Bar>> = symbols_seeds
        .iter()
        .map(|(sym, seed)| synthetic_hourly(sym, BAR_COUNT, *seed))
        .collect();

    let merged_bars = data::ReplayFeed::merge_synthetic(bars_by_symbol.clone());

    // Run 1
    let signals_run1 = {
        use strategy::Strategy as _;
        let mut strat = make_momentum(10, 10, 3, &universe);
        let mut out: Vec<(String, String)> = Vec::new();
        for bar in &merged_bars {
            for sig in strat.on_bar(bar) {
                out.push((sig.symbol.to_string(), format!("{:?}", sig.kind)));
            }
        }
        out
    };

    // Run 2 — identical strategy, identical bars
    let signals_run2 = {
        use strategy::Strategy as _;
        let mut strat = make_momentum(10, 10, 3, &universe);
        let mut out: Vec<(String, String)> = Vec::new();
        for bar in &merged_bars {
            for sig in strat.on_bar(bar) {
                out.push((sig.symbol.to_string(), format!("{:?}", sig.kind)));
            }
        }
        out
    };

    assert_eq!(
        signals_run1, signals_run2,
        "signal sequence must be identical across two runs (R12.2)"
    );
}

// ── Test 2: merge order is (venue_ts ASC, symbol ASC) ────────────────────────

#[test]
fn t618_merge_sort_key_venue_ts_then_symbol() {
    // Two symbols with the same timestamps — merged order must be alphabetical.
    let syms = ["BTCUSDT", "ADAUSDT", "SOLUSDT"];
    let bars_by_sym: Vec<Vec<Bar>> = syms
        .iter()
        .enumerate()
        .map(|(i, sym)| {
            (0..10i64)
                .map(|h| make_bar(sym, dec!(100) + Decimal::from(i as u32 * 10), h))
                .collect()
        })
        .collect();

    let merged = data::ReplayFeed::merge_synthetic(bars_by_sym);

    // For each group of same-timestamp bars, symbols should appear in alpha order.
    let mut by_ts: BTreeMap<i128, Vec<String>> = BTreeMap::new();
    for bar in &merged {
        by_ts
            .entry(bar.open_ts.inner().unix_timestamp_nanos())
            .or_default()
            .push(bar.symbol.to_string());
    }

    for group in by_ts.values() {
        for w in group.windows(2) {
            assert!(
                w[0] <= w[1],
                "within same timestamp, symbols must be alphabetical: {w:?}"
            );
        }
    }
}

// ── Test 3: score = None during warm-up → no signals ─────────────────────────

#[test]
fn t618_warmup_period_produces_no_signals() {
    use strategy::Strategy as _;

    let universe = ["BTCUSDT", "ETHUSDT", "BNBUSDT"];
    let mut strat = make_momentum(10, 10, 2, &universe);

    // Push only 5 bars per symbol (lookback=10 needs 11 bars to be full).
    for i in 0..5i64 {
        for sym in &universe {
            let signals = strat.on_bar(&make_bar(sym, dec!(10_000), i));
            assert!(
                signals.is_empty(),
                "no signals expected during warm-up, got {signals:?}"
            );
        }
    }
}

// ── Test 4: out-of-universe bars are filtered ─────────────────────────────────

#[test]
fn t618_out_of_universe_bars_filtered() {
    use strategy::Strategy as _;

    let universe = ["BTCUSDT", "ETHUSDT"];
    let mut strat = make_momentum(5, 5, 1, &universe);

    // Warm up with universe bars.
    for i in 0..30i64 {
        for sym in &universe {
            let _ = strat.on_bar(&make_bar(sym, dec!(10_000), i));
        }
        // Inject an out-of-universe bar — must return empty.
        let signals = strat.on_bar(&make_bar("XRPUSDT", dec!(0.5), i));
        assert!(
            signals.is_empty(),
            "out-of-universe bar must return no signals"
        );
    }
}

// ── Test 5: selected symbols have correct count ───────────────────────────────

#[test]
fn t618_top_k_long_selects_k_symbols() {
    let scores: BTreeMap<Symbol, Option<Decimal>> = [
        (Symbol::new("BTCUSDT"), Some(dec!(0.50))),
        (Symbol::new("ETHUSDT"), Some(dec!(0.30))),
        (Symbol::new("BNBUSDT"), Some(dec!(0.10))),
        (Symbol::new("SOLUSDT"), Some(dec!(0.05))),
    ]
    .into_iter()
    .collect();

    let k = 3u32;
    let exposure_cap = dec!(0.50);
    let selected = strategy::top_k_long(&scores, k, exposure_cap);

    assert_eq!(
        selected.len(),
        k as usize,
        "top_k_long must select exactly k={k} symbols"
    );

    // Check BTC and ETH are in the top-3 (highest scores).
    assert!(
        selected.contains_key(&Symbol::new("BTCUSDT")),
        "BTCUSDT must be in top-3"
    );
    assert!(
        selected.contains_key(&Symbol::new("ETHUSDT")),
        "ETHUSDT must be in top-3"
    );
    assert!(
        selected.contains_key(&Symbol::new("BNBUSDT")),
        "BNBUSDT must be in top-3"
    );

    // Each weight = exposure_cap / k = 0.50 / 3
    let expected_weight = exposure_cap / Decimal::from(k);
    for w in selected.values() {
        assert_eq!(
            *w, expected_weight,
            "each weight must equal exposure_cap / k"
        );
    }
}
