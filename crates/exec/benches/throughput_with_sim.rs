//! Throughput regression bench: noop vs enabled config (v5-latency-slippage-sim R-NR.4).
//!
//! Measures the overhead of apply_latency + apply_slippage on the hot path over
//! 8760 simulated fill iterations (the full-year momentum scenario size). The
//! delta between noop and enabled must be < 1% to pass R-NR.4.
//!
//! Note: this bench measures the function overhead in isolation (not the full
//! scenario, which would take minutes). The 1% gate from R-NR.4 is evaluated
//! by comparing the two bench times; criterion's estimates give statistical
//! bounds to make the comparison reliable.
//!
//! Run with: `cargo bench -p exec --bench throughput_with_sim`

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use rust_decimal::Decimal;
use trading_core::Side;

const SCENARIO_SEED: [u8; 32] = [
    0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
    0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
];

const FILLS_PER_SCENARIO: u64 = 8_760; // matches full-year hourly scenario

/// Benchmark: noop path (latency=0, slippage=0) over 8760 fills.
fn bench_throughput_noop(c: &mut Criterion) {
    let ts = black_box(1_700_000_000_000_i64);
    let price = black_box(Decimal::from(50_000_u64));
    let notional = black_box(Decimal::from(5_000_000_u64));

    let mut group = c.benchmark_group("throughput_with_sim");
    group.throughput(Throughput::Elements(FILLS_PER_SCENARIO));

    group.bench_function("noop_8760_fills", |b| {
        b.iter(|| {
            for i in 0_u64..FILLS_PER_SCENARIO {
                let _ = exec::apply_latency(
                    black_box(ts),
                    black_box(0_u64),
                    black_box(0_u64),
                    &SCENARIO_SEED,
                    i.to_le_bytes(),
                );
                let _ = cost::apply_slippage(
                    black_box(price),
                    black_box(Side::Buy),
                    black_box(notional),
                    black_box(0_u32),
                );
            }
        })
    });
}

/// Benchmark: enabled path (latency=50..100ms, slippage=10bps) over 8760 fills.
fn bench_throughput_enabled(c: &mut Criterion) {
    let ts = black_box(1_700_000_000_000_i64);
    let price = black_box(Decimal::from(50_000_u64));
    let notional = black_box(Decimal::from(5_000_000_u64));

    let mut group = c.benchmark_group("throughput_with_sim");
    group.throughput(Throughput::Elements(FILLS_PER_SCENARIO));

    group.bench_function("enabled_8760_fills", |b| {
        b.iter(|| {
            for i in 0_u64..FILLS_PER_SCENARIO {
                let _ = exec::apply_latency(
                    black_box(ts),
                    black_box(50_u64),
                    black_box(100_u64),
                    &SCENARIO_SEED,
                    i.to_le_bytes(),
                );
                let _ = cost::apply_slippage(
                    black_box(price),
                    black_box(Side::Buy),
                    black_box(notional),
                    black_box(10_u32),
                );
            }
        })
    });
}

criterion_group!(benches, bench_throughput_noop, bench_throughput_enabled);
criterion_main!(benches);
