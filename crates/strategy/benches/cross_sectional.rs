//! T621 — Criterion benchmarks for the cross-sectional momentum strategy.
//!
//! Measures:
//! - `on_bar` throughput for a 10-symbol universe (warm — post-warmup).
//! - `top_k_long` selector with 10 symbols.
//! - `decimal_ln` and `decimal_sqrt` hot paths.
//! - `score_vol_adjusted_return` with lookback=60.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use time::OffsetDateTime;
use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp};

// ── Helpers ────────────────────────────────────────────────────────────────────

fn make_bar(symbol: &str, close: Decimal, offset_hours: i64) -> Bar {
    let base = OffsetDateTime::UNIX_EPOCH;
    let ts = Timestamp::new(base + time::Duration::hours(offset_hours));
    let price = Price::new(close).unwrap();
    Bar {
        symbol: Symbol::new(symbol),
        tf: Timeframe::OneHour,
        open: price,
        high: price,
        low: price,
        close: price,
        volume: Quantity::new(dec!(100)).unwrap(),
        trade_count: 10,
        local_recv_ts: ts,
        open_ts: ts,
        close_ts: ts,
    }
}

fn build_warmed_strategy(k_long: u32) -> strategy::MomentumStrategy {
    use strategy::Strategy as _;

    let toml = format!(
        r#"id = "bench_momentum"
kind = "cross_sectional_momentum"
stage = "research"
universe = ["ADAUSDT", "AVAXUSDT", "BNBUSDT", "BTCUSDT", "DOGEUSDT",
            "DOTUSDT", "ETHUSDT", "LINKUSDT", "SOLUSDT", "XRPUSDT"]
lookback_minutes = 60
rebalance_minutes = 60
k_long = {k_long}
k_short = 0
exposure_cap = 0.50
drift_rebalance_threshold = 0.10
vol_floor = 0.000001
size = "equal_weight"
"#
    );
    let cfg = strategy::CrossSectionalMomentumConfig::from_str(&toml).unwrap();
    let mut strat = strategy::MomentumStrategy::from_config(cfg, SmolStr::new("bench"));

    let symbols = [
        ("ADAUSDT", dec!(0.25)),
        ("AVAXUSDT", dec!(11.00)),
        ("BNBUSDT", dec!(240.00)),
        ("BTCUSDT", dec!(16_500.00)),
        ("DOGEUSDT", dec!(0.07)),
        ("DOTUSDT", dec!(4.50)),
        ("ETHUSDT", dec!(1_200.00)),
        ("LINKUSDT", dec!(6.00)),
        ("SOLUSDT", dec!(10.00)),
        ("XRPUSDT", dec!(0.34)),
    ];

    // Warm up: push 65 bars per symbol (lookback=60, need >60 to fill).
    for i in 0..65i64 {
        for (sym, base_price) in &symbols {
            // Slight upward trend to get non-trivial scores.
            let price = *base_price
                * (Decimal::ONE + Decimal::try_from(i as f64 * 0.001).unwrap_or(Decimal::ZERO));
            let bar = make_bar(sym, price, i);
            let _ = strat.on_bar(&bar);
        }
    }

    strat
}

// ── Benchmarks ─────────────────────────────────────────────────────────────────

fn bench_on_bar_warm(c: &mut Criterion) {
    use strategy::Strategy as _;

    let mut strat = build_warmed_strategy(3);
    let bar = make_bar("BTCUSDT", dec!(17_000), 100);

    c.bench_function("momentum_on_bar_warm_10sym", |b| {
        b.iter(|| {
            let signals = strat.on_bar(black_box(&bar));
            black_box(signals)
        })
    });
}

fn bench_top_k_long(c: &mut Criterion) {
    use std::collections::BTreeMap;

    let scores: BTreeMap<Symbol, Option<Decimal>> = [
        (Symbol::new("ADAUSDT"), Some(dec!(0.10))),
        (Symbol::new("AVAXUSDT"), Some(dec!(0.30))),
        (Symbol::new("BNBUSDT"), Some(dec!(0.05))),
        (Symbol::new("BTCUSDT"), Some(dec!(0.50))),
        (Symbol::new("DOGEUSDT"), Some(dec!(0.20))),
        (Symbol::new("DOTUSDT"), Some(dec!(0.15))),
        (Symbol::new("ETHUSDT"), Some(dec!(0.45))),
        (Symbol::new("LINKUSDT"), Some(dec!(0.08))),
        (Symbol::new("SOLUSDT"), Some(dec!(0.35))),
        (Symbol::new("XRPUSDT"), Some(dec!(0.25))),
    ]
    .into_iter()
    .collect();

    c.bench_function("top_k_long_10sym_k3", |b| {
        b.iter(|| {
            let selected =
                strategy::top_k_long(black_box(&scores), black_box(3), black_box(dec!(0.50)));
            black_box(selected)
        })
    });
}

fn bench_decimal_ln(c: &mut Criterion) {
    let x = dec!(1.0523);
    c.bench_function("decimal_ln", |b| {
        b.iter(|| {
            let r = features::decimal_ln(black_box(x));
            black_box(r)
        })
    });
}

fn bench_decimal_sqrt(c: &mut Criterion) {
    let x = dec!(16_500.00);
    c.bench_function("decimal_sqrt", |b| {
        b.iter(|| {
            let r = features::decimal_sqrt(black_box(x));
            black_box(r)
        })
    });
}

fn bench_score_vol_adjusted_return(c: &mut Criterion) {
    use features::RingBuffer;

    // Pre-fill a ring buffer with 61 values (lookback=60).
    let mut rb = RingBuffer::new(61);
    let mut price = dec!(10_000);
    for i in 0..61 {
        let delta = Decimal::try_from(0.001 * i as f64).unwrap_or(Decimal::ZERO);
        price += delta;
        rb.push(price);
    }

    c.bench_function("score_vol_adjusted_return_lb60", |b| {
        b.iter(|| {
            let r = features::score_vol_adjusted_return(
                black_box(&rb),
                black_box(60),
                black_box(dec!(0.000001)),
            );
            black_box(r)
        })
    });
}

fn bench_on_bar_out_of_universe(c: &mut Criterion) {
    use strategy::Strategy as _;

    let mut strat = build_warmed_strategy(3);
    // Bar for a symbol NOT in the universe — exercises the Q5 fast-return path.
    let bar = make_bar("ALGOUSDT", dec!(0.50), 100);

    c.bench_function("momentum_on_bar_out_of_universe", |b| {
        b.iter(|| {
            let signals = strat.on_bar(black_box(&bar));
            black_box(signals)
        })
    });
}

criterion_group!(
    benches,
    bench_on_bar_warm,
    bench_top_k_long,
    bench_decimal_ln,
    bench_decimal_sqrt,
    bench_score_vol_adjusted_return,
    bench_on_bar_out_of_universe,
);
criterion_main!(benches);
