//! T1412 V5 — 1-second bar aggregation test (multi-venue feature v1.5b).
//!
//! Per [spec/features/v1-5b-multi-venue.md → V5] and
//! [spec/tasks/v1-5b-multi-venue.md → T1412 acceptance]:
//!
//! > V5 — 1-second bars from synthetic trades. A synthetic trade-stream
//! > fixture feeds the 1s aggregator; emitted bars match expected OHLCV
//! > by hand. Determinism: two runs against the same fixture emit
//! > byte-identical bars. Maps to R5.
//!
//! This file feeds 60 synthetic Ticks at 100ms strides (10 ticks per
//! second × 6 seconds) into `aggregate_one_second_iter` and asserts:
//! - 6 Bars emitted (one per UTC second).
//! - Each Bar's `open` / `high` / `low` / `close` / `volume` matches
//!   expected OHLCV by hand.
//! - `tf == Timeframe::OneSecond` on every Bar.
//! - `venue == Venue::Binance` (input ticks are stamped Binance).
//! - `open_ts` is exactly `floor(first_tick_ts.unix_micros() /
//!   1_000_000) * 1_000_000`.
//! - Determinism: running the test twice produces byte-identical Bar
//!   streams.
//!
//! This integration test is the architect-prescribed V5 cross-crate
//! verification (the in-crate unit test
//! `bar_aggregator::tests::t1406_v5_synthetic_stream_aggregates_to_n_bars`
//! covers the same surface from inside the crate).
#![allow(clippy::expect_used, clippy::unwrap_used)]

use data::aggregate_one_second_iter;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{Price, Quantity, Side, Symbol, Tick, Timeframe, Timestamp, Venue};

fn mk_tick(symbol: &Symbol, ts_micros: i64, price: Decimal, qty: Decimal, id: u64) -> Tick {
    let dt = OffsetDateTime::from_unix_timestamp_nanos(i128::from(ts_micros) * 1_000)
        .expect("valid timestamp");
    Tick {
        symbol: symbol.clone(),
        venue_ts: Timestamp::new(dt),
        local_recv_ts: Timestamp::new(OffsetDateTime::UNIX_EPOCH),
        price: Price::new(price).unwrap(),
        qty: Quantity::new(qty).unwrap(),
        side: Side::Buy,
        trade_id: id,
        venue: Venue::Binance,
    }
}

/// Build the canonical V5 fixture: 60 ticks across 6 seconds, 10 ticks
/// per second at 100ms strides. Tick prices are deterministic
/// (`60_000 + i`) so the per-bucket OHLC is computable by hand.
fn build_v5_fixture(symbol: &Symbol) -> Vec<Tick> {
    let mut ticks = Vec::with_capacity(60);
    for i in 0..60_i64 {
        // 100ms = 100_000 microseconds → 6 buckets of 10 ticks each.
        let ts_us = i * 100_000;
        let price = Decimal::from(60_000) + Decimal::from(i);
        ticks.push(mk_tick(symbol, ts_us, price, dec!(0.001), i as u64));
    }
    ticks
}

/// T1412 V5 — synthetic stream of 60 ticks at 100ms intervals → 6 bars.
///
/// Hand-computed expected OHLCV per bucket:
///
/// | bucket | tick range | prices         | open  | high  | low   | close | volume |
/// |--------|------------|----------------|-------|-------|-------|-------|--------|
/// | s=0    | i=0..9     | 60000..60009   | 60000 | 60009 | 60000 | 60009 | 0.010  |
/// | s=1    | i=10..19   | 60010..60019   | 60010 | 60019 | 60010 | 60019 | 0.010  |
/// | s=2    | i=20..29   | 60020..60029   | 60020 | 60029 | 60020 | 60029 | 0.010  |
/// | s=3    | i=30..39   | 60030..60039   | 60030 | 60039 | 60030 | 60039 | 0.010  |
/// | s=4    | i=40..49   | 60040..60049   | 60040 | 60049 | 60040 | 60049 | 0.010  |
/// | s=5    | i=50..59   | 60050..60059   | 60050 | 60059 | 60050 | 60059 | 0.010  |
#[test]
fn t1412_v5_synthetic_stream_aggregates_to_n_bars() {
    let symbol = Symbol::new("BTCUSDT");
    let ticks = build_v5_fixture(&symbol);
    assert_eq!(ticks.len(), 60, "fixture: 60 ticks across 6s");

    let bars = aggregate_one_second_iter(ticks, Venue::Binance);
    assert_eq!(bars.len(), 6, "expected 6 bars from 60 ticks across 6s");

    // Per-bar hand-computed OHLCV table (see docstring).
    for (idx, bar) in bars.iter().enumerate() {
        let base = 60_000_i64 + (idx as i64) * 10;
        // tf + venue + symbol gates.
        assert_eq!(bar.tf, Timeframe::OneSecond, "tf == OneSecond on every bar");
        assert_eq!(
            bar.venue,
            Venue::Binance,
            "venue == Venue::Binance (propagated from caller)"
        );
        assert_eq!(bar.symbol, symbol, "symbol propagated from input tick");
        // OHLC.
        assert_eq!(bar.open.get(), Decimal::from(base), "bar[{idx}].open");
        assert_eq!(bar.high.get(), Decimal::from(base + 9), "bar[{idx}].high");
        assert_eq!(bar.low.get(), Decimal::from(base), "bar[{idx}].low");
        assert_eq!(bar.close.get(), Decimal::from(base + 9), "bar[{idx}].close");
        // Volume = 10 × 0.001 = 0.010.
        assert_eq!(bar.volume.get(), dec!(0.01), "bar[{idx}].volume");
        assert_eq!(bar.trade_count, 10, "bar[{idx}].trade_count == 10");
        // open_ts at exact UTC second.
        let open_us = bar.open_ts.inner().unix_timestamp_nanos() / 1_000;
        let expected_open_us = (idx as i128) * 1_000_000;
        assert_eq!(
            open_us, expected_open_us,
            "bar[{idx}].open_ts at UTC-second boundary",
        );
    }

    // Bucket-boundary stride: each bar's open_ts is 1s after the previous.
    for w in bars.windows(2) {
        let delta_us = w[1].open_ts.inner().unix_timestamp_nanos() / 1_000
            - w[0].open_ts.inner().unix_timestamp_nanos() / 1_000;
        assert_eq!(delta_us, 1_000_000, "1s stride between adjacent bars");
    }

    // open_ts of the first bar matches the floor formula:
    // floor(first_tick_ts_micros / 1_000_000) * 1_000_000 = 0.
    let first_tick_us = 0_i64;
    let expected_first_open_us = i128::from((first_tick_us / 1_000_000) * 1_000_000);
    assert_eq!(
        bars[0].open_ts.inner().unix_timestamp_nanos() / 1_000,
        expected_first_open_us,
        "first bar's open_ts matches floor(first_tick_ts/1s)*1s",
    );
}

/// T1412 determinism gate (R5.3) — running the aggregator twice on the
/// same fixture produces byte-identical Bar streams.
#[test]
fn t1412_v5_aggregation_is_byte_identical_across_runs() {
    let symbol = Symbol::new("BTCUSDT");
    let bars_a = aggregate_one_second_iter(build_v5_fixture(&symbol), Venue::Binance);
    let bars_b = aggregate_one_second_iter(build_v5_fixture(&symbol), Venue::Binance);
    assert_eq!(bars_a.len(), bars_b.len(), "same length both runs");
    for (a, b) in bars_a.iter().zip(bars_b.iter()) {
        // Compare every byte-comparable field exhaustively.
        assert_eq!(a.symbol, b.symbol);
        assert_eq!(a.tf, b.tf);
        assert_eq!(a.open_ts, b.open_ts);
        assert_eq!(a.close_ts, b.close_ts);
        assert_eq!(a.open.get(), b.open.get());
        assert_eq!(a.high.get(), b.high.get());
        assert_eq!(a.low.get(), b.low.get());
        assert_eq!(a.close.get(), b.close.get());
        assert_eq!(a.volume.get(), b.volume.get());
        assert_eq!(a.trade_count, b.trade_count);
        assert_eq!(a.local_recv_ts, b.local_recv_ts);
        assert_eq!(a.venue, b.venue);
    }
}
