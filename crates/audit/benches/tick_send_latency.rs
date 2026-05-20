//! H1 — `tokio::sync::broadcast::Sender::send` latency benchmark.
//!
//! Measures the per-send latency with 0, 1, 4, and 16 subscribers.
//! Claim (H1): p99 ≤ 1µs @ 16 subscribers. Numbers produced, not gated.
//!
//! Run: `cargo bench -p audit --bench tick_send_latency`

use audit::tick::{AuditContext, AuditEvent, AuditTick};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use smol_str::SmolStr;
use time::OffsetDateTime;
use tokio::sync::broadcast;
use uuid::Uuid;

fn make_tick() -> AuditTick<AuditEvent> {
    AuditTick {
        event: AuditEvent::KillSwitchTripped {
            reason: SmolStr::new("bench"),
        },
        context: AuditContext {
            run_id: Uuid::nil(),
            posted_at: OffsetDateTime::UNIX_EPOCH,
            agent_pid: 1,
        },
    }
}

fn bench_send(c: &mut Criterion) {
    let _rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("rt");

    let mut group = c.benchmark_group("tick_send_latency");

    for n_subscribers in [0usize, 1, 4, 16] {
        group.bench_with_input(
            BenchmarkId::new("subscribers", n_subscribers),
            &n_subscribers,
            |b, &n| {
                let (sender, _) = broadcast::channel::<AuditTick<AuditEvent>>(1024);
                // Subscribe n receivers (never read — lagged drop is expected).
                let _rxs: Vec<_> = (0..n).map(|_| sender.subscribe()).collect();

                b.iter(|| {
                    let tick = make_tick();
                    let _ = sender.send(tick);
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_send);
criterion_main!(benches);
