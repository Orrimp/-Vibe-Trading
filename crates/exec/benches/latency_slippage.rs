//! Micro-benchmarks for latency and slippage simulation (v5-latency-slippage-sim R7).
//!
//! # Targets (R7)
//!
//! - `apply_latency_noop` — at zero ms; target ≤ 5 ns.
//! - `apply_latency_jitter` — at 50..=100 ms range; target ≤ 50 ns.
//! - `apply_slippage_10bps` — at 10 bps; target ≤ 10 ns.
//!
//! Run with: `cargo bench -p exec --bench latency_slippage`

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use rust_decimal::Decimal;
use trading_core::Side;

const SCENARIO_SEED: [u8; 32] = [
    0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
    0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
];

/// Benchmark 1: apply_latency at zero ms (noop path).
/// Target: ≤ 5 ns (R7).
fn bench_apply_latency_noop(c: &mut Criterion) {
    let ts = black_box(1_700_000_000_000_i64);
    let order_id = black_box([1_u8, 0, 0, 0, 0, 0, 0, 0]);

    c.bench_function("apply_latency_noop", |b| {
        b.iter(|| {
            exec::apply_latency(
                black_box(ts),
                black_box(0_u64),
                black_box(0_u64),
                &SCENARIO_SEED,
                black_box(order_id),
            )
        })
    });
}

/// Benchmark 2: apply_latency with 50..=100 ms jitter (RNG path).
/// Target: ≤ 50 ns (R7).
fn bench_apply_latency_jitter(c: &mut Criterion) {
    let ts = black_box(1_700_000_000_000_i64);
    let order_id = black_box([42_u8, 0, 0, 0, 0, 0, 0, 0]);

    c.bench_function("apply_latency_jitter", |b| {
        b.iter(|| {
            exec::apply_latency(
                black_box(ts),
                black_box(50_u64),
                black_box(100_u64),
                &SCENARIO_SEED,
                black_box(order_id),
            )
        })
    });
}

/// Benchmark 3: apply_slippage at 10 bps (multiply path).
/// Target: ≤ 10 ns (R7).
fn bench_apply_slippage_10bps(c: &mut Criterion) {
    let price = black_box(Decimal::from(50_000_u64));
    let notional = black_box(Decimal::from(1_000_000_u64));

    c.bench_function("apply_slippage_10bps", |b| {
        b.iter(|| {
            cost::apply_slippage(
                black_box(price),
                black_box(Side::Buy),
                black_box(notional),
                black_box(10_u32),
            )
        })
    });
}

criterion_group!(
    benches,
    bench_apply_latency_noop,
    bench_apply_latency_jitter,
    bench_apply_slippage_10bps
);
criterion_main!(benches);
