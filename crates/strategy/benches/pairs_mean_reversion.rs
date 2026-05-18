//! T718 — Criterion benches for `MeanReversionPairsStrategy` `on_bar` performance.
//!
//! Three cases per V7 performance budget (< 5ms p99 per pair-bar at 3 pairs):
//!
//! 1. `sync_incomplete` — only one leg arrives; no spread computation.
//! 2. `sync_complete_no_decision` — both legs arrive but z-score does not
//!    cross any threshold (warmup bar).
//! 3. `sync_complete_decision` — both legs arrive and z-score crosses entry
//!    threshold (entry signal emitted).
//!
//! All three cases must complete well under 5ms per pair-bar on a modern laptop.
//! In practice, expect sub-microsecond for cases 1–2 and low-microsecond for 3.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use strategy::{Strategy, pairs::mean_reversion::MeanReversionPairsStrategy};
use time::OffsetDateTime;
use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

// ── Fixture helpers ────────────────────────────────────────────────────────────

fn ts_at(minute: i64) -> Timestamp {
    let epoch = OffsetDateTime::new_utc(
        time::Date::from_calendar_date(2023, time::Month::January, 1).unwrap(),
        time::Time::MIDNIGHT,
    );
    Timestamp::new(epoch + time::Duration::minutes(minute))
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

fn make_strategy(lookback: u32) -> MeanReversionPairsStrategy {
    let toml = format!(
        r#"
id = "pairs_mr_bench"
kind = "mean_reversion_pairs"
stage = "research"

pairs = [
    {{ a = "BTCUSDT", b = "ETHUSDT", beta = "1.0" }},
    {{ a = "ETHUSDT", b = "SOLUSDT", beta = "1.0" }},
    {{ a = "BNBUSDT", b = "BTCUSDT", beta = "1.0" }},
]

lookback_minutes      = {lookback}
cooldown_minutes      = 60
z_entry               = "2.0"
z_exit                = "0.5"
z_stop                = "4.0"
vol_floor             = "0.000001"
size                  = "binary_per_pair"
exposure_cap_per_pair = "0.25"
max_staleness_minutes = 5
"#
    );
    let cfg = strategy::pairs::config::MeanReversionPairsConfig::from_str(&toml)
        .expect("valid bench config");
    MeanReversionPairsStrategy::from_config(cfg, SmolStr::new("bench.toml"))
}

/// Warm up the strategy to just before the decision bar.
fn warmup_strategy(strat: &mut MeanReversionPairsStrategy, lookback: u32) {
    let symbols = ["BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT"];
    let prices = [dec!(30000), dec!(2000), dec!(240), dec!(10)];

    for minute in 0i64..(lookback as i64 - 1) {
        for (sym, price) in symbols.iter().zip(prices.iter()) {
            strat.on_bar(&make_bar(sym, *price, minute));
        }
    }
}

// ── Bench: sync-incomplete (fast-return path) ─────────────────────────────────

fn bench_sync_incomplete(c: &mut Criterion) {
    // Only the `a` leg arrives — sync is incomplete, no spread computation.
    let bar = make_bar("BTCUSDT", dec!(30000), 100);

    c.bench_function("pairs_on_bar_sync_incomplete", |b| {
        b.iter_with_setup(
            || make_strategy(60),
            |mut strat| {
                // Feed only the a-leg bar — sync never completes.
                let sigs = strat.on_bar(black_box(&bar));
                black_box(sigs);
            },
        )
    });
}

// ── Bench: sync-complete, no decision (warmup spread pushed) ──────────────────

fn bench_sync_complete_no_decision(c: &mut Criterion) {
    let lookback = 60u32;

    c.bench_function("pairs_on_bar_sync_complete_no_decision", |b| {
        b.iter_with_setup(
            || {
                let mut strat = make_strategy(lookback);
                warmup_strategy(&mut strat, lookback);
                strat
            },
            |mut strat| {
                // One more bar at the same price — no threshold crossing.
                let minute = (lookback - 1) as i64;
                strat.on_bar(black_box(&make_bar("BTCUSDT", dec!(30000), minute)));
                let sigs = strat.on_bar(black_box(&make_bar("ETHUSDT", dec!(2000), minute)));
                black_box(sigs);
            },
        )
    });
}

// ── Bench: sync-complete, decision bar (entry signal emitted) ─────────────────

fn bench_sync_complete_decision(c: &mut Criterion) {
    let lookback = 60u32;

    c.bench_function("pairs_on_bar_sync_complete_decision", |b| {
        b.iter_with_setup(
            || {
                let mut strat = make_strategy(lookback);
                // Warmup with neutral prices.
                let symbols = ["BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT"];
                let prices = [dec!(30000), dec!(30000), dec!(30000), dec!(30000)];
                for minute in 0i64..(lookback as i64) {
                    for (sym, price) in symbols.iter().zip(prices.iter()) {
                        strat.on_bar(&make_bar(sym, *price, minute));
                    }
                }
                strat
            },
            |mut strat| {
                // Entry bar: price_a drops sharply → z << -z_entry.
                let minute = lookback as i64;
                strat.on_bar(black_box(&make_bar("BTCUSDT", dec!(1000), minute)));
                let sigs = strat.on_bar(black_box(&make_bar("ETHUSDT", dec!(30000), minute)));
                black_box(sigs);
            },
        )
    });
}

// ── Bench: spread computation ─────────────────────────────────────────────────

fn bench_spread_compute(c: &mut Criterion) {
    use features::spread;

    c.bench_function("spread_compute_beta1", |b| {
        b.iter(|| {
            let s = spread(
                black_box(dec!(30000)),
                black_box(dec!(2000)),
                black_box(dec!(1)),
            );
            let _ = black_box(s);
        });
    });
}

// ── Bench: z-score computation ────────────────────────────────────────────────

fn bench_zscore_compute(c: &mut Criterion) {
    use features::{RingBuffer, rolling_zscore};

    let mut buf = RingBuffer::new(61);
    for i in 0..60 {
        buf.push(dec!(0) + rust_decimal::Decimal::from(i) * dec!(0.001));
    }

    c.bench_function("zscore_60bar_lookback", |b| {
        b.iter(|| {
            let z = rolling_zscore(black_box(&buf), black_box(60), black_box(dec!(0.000001)));
            let _ = black_box(z);
        });
    });
}

criterion_group!(
    benches,
    bench_sync_incomplete,
    bench_sync_complete_no_decision,
    bench_sync_complete_decision,
    bench_spread_compute,
    bench_zscore_compute,
);
criterion_main!(benches);
