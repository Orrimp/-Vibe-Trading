//! T519 — Criterion benches for ComposedStrategy `on_bar` performance.
//!
//! Three cases per R10.2:
//! 1. Single-rule: `rsi(14) < 30`.
//! 2. 3-rule AND: `macd_hist(12,26,9) > 0 AND close > ema(200)` (btc_macd_trend shape).
//! 3. 5-rule mixed: `(rsi(14) < 30 OR macd_cross(12,26,9)) AND close < bollinger_lower(20,2)
//!    AND volume > 1.5 * avg(volume, 20) AND NOT (close < min(low, 20))`.
//!
//! All three must stay under the v0 p99 budget: `< 5ms` for a single `on_bar` call
//! on a modern laptop. In practice they should be < 10µs, well within the budget.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use strategy::{ComposedStrategy, ComposedStrategyConfig, Strategy};
use time::OffsetDateTime;
use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

// ── Fixture generation ────────────────────────────────────────────────────────

/// Simple LCG PRNG — deterministic, no external deps.
struct Lcg(u64);
impl Lcg {
    fn next_f64(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as f64 / (u64::MAX >> 33) as f64
    }
}

fn make_bars(count: usize) -> Vec<Bar> {
    let mut rng = Lcg(0xC0_FFEE);
    let epoch = OffsetDateTime::new_utc(
        time::Date::from_calendar_date(2023, time::Month::January, 1).unwrap(),
        time::Time::MIDNIGHT,
    );
    let mut bars = Vec::with_capacity(count);
    let mut close: f64 = 16_500.0;
    for i in 0..count {
        let z = (rng.next_f64() - 0.5) * 2.0;
        let ret = z * 0.005;
        close = (close * (1.0 + ret)).clamp(1_000.0, 500_000.0);
        let open = close * (1.0 + (rng.next_f64() - 0.5) * 0.001);
        let high = close.max(open) * (1.0 + rng.next_f64() * 0.002);
        let low = close.min(open) * (1.0 - rng.next_f64() * 0.002);

        let open_ts = Timestamp::new(epoch + time::Duration::minutes(i as i64));
        let close_ts = Timestamp::new(
            epoch + time::Duration::minutes(i as i64 + 1) - time::Duration::seconds(1),
        );
        let mk_price = |v: f64| {
            Price::new(Decimal::try_from(v.max(0.01)).unwrap_or(dec!(1)))
                .unwrap_or_else(|_| Price::new(dec!(1)).unwrap())
        };
        bars.push(Bar {
            symbol: Symbol::new("BTCUSDT"),
            tf: Timeframe::OneMinute,
            open: mk_price(open),
            high: mk_price(high),
            low: mk_price(low),
            close: mk_price(close),
            volume: Quantity::new(dec!(10)).unwrap(),
            trade_count: 100,
            local_recv_ts: close_ts,
            open_ts,
            close_ts,
            venue: Venue::Binance,
        });
    }
    bars
}

fn make_strategy(signal: &str, id: &str) -> ComposedStrategy {
    let toml = format!(
        r#"id = "{id}"
kind = "composed"
symbol = "BTCUSDT"
stage = "research"
signal = "{signal}"
size = "fixed_fraction(0.1)"
"#
    );
    let cfg = ComposedStrategyConfig::from_str(&toml, id).expect("valid config");
    ComposedStrategy::from_config(cfg, SmolStr::new("bench"))
}

// ── Benchmarks ────────────────────────────────────────────────────────────────

fn bench_single_rule(c: &mut Criterion) {
    let bars = make_bars(1_000);
    let mut strategy = make_strategy("rsi(14) < 30", "bench_single");

    // Warm up the indicator state so we're benchmarking steady-state.
    for bar in &bars[..500] {
        black_box(strategy.on_bar(bar));
    }

    c.bench_function("on_bar/1-rule: rsi(14) < 30", |b| {
        let bar = &bars[500];
        b.iter(|| black_box(strategy.on_bar(bar)));
    });
}

fn bench_three_rule(c: &mut Criterion) {
    let bars = make_bars(1_000);
    let mut strategy = make_strategy("macd_hist(12,26,9) > 0 AND close > ema(200)", "bench_three");

    for bar in &bars[..500] {
        black_box(strategy.on_bar(bar));
    }

    c.bench_function(
        "on_bar/3-rule: macd_hist(12,26,9) > 0 AND close > ema(200)",
        |b| {
            let bar = &bars[500];
            b.iter(|| black_box(strategy.on_bar(bar)));
        },
    );
}

fn bench_five_rule(c: &mut Criterion) {
    let bars = make_bars(1_000);
    let signal = "(rsi(14) < 30 OR macd_cross(12,26,9)) AND close < bollinger_lower(20,2) AND volume > 1.5 * avg(volume, 20) AND NOT (close < min(low, 20))";
    let mut strategy = make_strategy(signal, "bench_five");

    for bar in &bars[..500] {
        black_box(strategy.on_bar(bar));
    }

    c.bench_function(
        "on_bar/5-rule: mixed (rsi + macd_cross + bollinger + volume + min)",
        |b| {
            let bar = &bars[500];
            b.iter(|| black_box(strategy.on_bar(bar)));
        },
    );
}

criterion_group!(
    composed_benches,
    bench_single_rule,
    bench_three_rule,
    bench_five_rule
);
criterion_main!(composed_benches);
