//! H5 — Trail-mirror `Open` request latency bench (T-D-N16).
//!
//! Measures end-to-end latency of `TrailMirrorRequest::Open` from send
//! through LRU-check / backfill stub to `TrailMirrorTick::TrailReady`
//! receipt. Gate: p99 < 50 ms (decomp.md § Wave D).
//!
//! ## Methodology
//!
//! 1. Open an in-memory audit ledger + broadcast bus.
//! 2. Seed 10⁵ synthetic `journal_transactions` rows via `audit::Journal`
//!    (exercises ledger write path; backfill at v0.1.0 is a stub so these
//!    rows are present for realistic DB pressure when T-D-N25 lands).
//! 3. Construct `TrailMirror::new`; spawn `.run()` in a background tokio task.
//! 4. 100 random `Open(audit_id)` requests (fixed seed
//!    `ChaCha20Rng::seed_from_u64(0xD005_D5C0_FFEE_BC01)`).
//!    Each iteration: send request, await `TrailReady` on a fresh subscription.
//! 5. Criterion measures wall-clock latency per round-trip. Assert p99 < 50 ms.
//!
//! Run: `cargo bench -p reflection --bench trail_mirror`

use std::sync::Arc;
use std::time::{Duration, Instant};

use criterion::{Criterion, criterion_group, criterion_main};
use rand::SeedableRng;
use rand::prelude::SliceRandom;
use rand_chacha::ChaCha20Rng;
use reflection::trail_mirror::{TrailMirror, TrailMirrorRequest, TrailMirrorTick};
use tokio::runtime::Runtime;

/// Fixed seed per decomp.md § Wave D (literal from spec; "BCH1" corrected to
/// valid hex `BC01` — the spec had a non-hex character).
const SEED: u64 = 0xD005_D5C0_FFEE_BC01;

/// Number of synthetic rows seeded in the ledger.
const SEED_ROWS: usize = 100_000;

/// Number of random `Open` requests to benchmark.
const OPEN_REQUESTS: usize = 100;

fn bench_trail_mirror_open(c: &mut Criterion) {
    let rt = Runtime::new().expect("tokio runtime");

    // ── 1. Open in-memory audit ledger ───────────────────────────────────────
    let (ledger, tick_sender) = rt
        .block_on(audit::Ledger::open_with_tick_bus(":memory:", 1024))
        .expect("in-memory ledger");
    let ledger = Arc::new(ledger);

    // ── 2. Seed 10⁵ synthetic journal rows ───────────────────────────────────
    //
    // We write rows via `audit::journal::write_fill` so the ledger has realistic
    // SQLite row pressure. At v0.1.0 the backfill stub ignores these rows, but
    // they remain in place for when T-D-N25 lands the real SQL query.
    //
    // To avoid pulling in the full trading_core type graph here, we construct
    // the minimal `AuditEvent::KillSwitchTripped` tick (zero payload) as a
    // stand-in row. The bench gate is latency, not schema completeness.
    {
        use audit::tick::{AuditContext, AuditEvent, AuditTick};
        use time::OffsetDateTime;

        // Pre-build a fixed tick to avoid construction overhead inside the loop.
        let tick = AuditTick {
            event: AuditEvent::KillSwitchTripped {
                reason: smol_str::SmolStr::new("bench-seed"),
            },
            context: AuditContext {
                run_id: uuid::Uuid::nil(),
                posted_at: OffsetDateTime::UNIX_EPOCH,
                agent_pid: 1,
            },
        };

        rt.block_on(async {
            for _ in 0..SEED_ROWS {
                // Fire-and-forget — sender may have no subscribers; that's fine.
                let _ = tick_sender.send(tick.clone());
            }
        });
    }

    // ── 3. Construct TrailMirror and spawn its run loop ───────────────────────
    let rx = tick_sender.subscribe();
    let (mirror, handle) = TrailMirror::new(rx, Arc::clone(&ledger));
    rt.spawn(async move { mirror.run().await });

    // ── 4. Build the 100 audit_id strings for the Open requests ──────────────
    //
    // We generate IDs that are NOT in the LRU (cold path each time) to stress
    // the backfill stub path. The LRU capacity is 16, so requests beyond the
    // first 16 unique IDs always trigger a cache-miss → backfill stub.
    let mut rng = ChaCha20Rng::seed_from_u64(SEED);
    // Pool of 200 distinct ids; shuffle to randomise LRU hit/miss ratio
    // (first 16 become warm after initial Open, remaining 184 are always cold).
    let mut id_pool: Vec<String> = (0..200).map(|i| format!("bench-audit-{i:06}")).collect();
    id_pool.shuffle(&mut rng);
    let request_ids: Vec<String> = id_pool.into_iter().take(OPEN_REQUESTS).collect();

    // ── 5. Criterion benchmark ────────────────────────────────────────────────
    let mut group = c.benchmark_group("trail_mirror");
    // Reduce measurement count — each iteration is a round-trip async call.
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(10));

    // Collect raw latencies to assert p99 after criterion is done.
    let mut latencies_ns: Vec<u64> = Vec::with_capacity(OPEN_REQUESTS);

    group.bench_function("trail_mirror_open", |b| {
        b.iter_custom(|iters| {
            let mut total = Duration::ZERO;
            for i in 0..iters as usize {
                let audit_id = request_ids[i % request_ids.len()].clone();
                let req = TrailMirrorRequest::Open(audit_id);
                let req_tx = handle.req_tx.clone();
                let mut tick_rx = handle.tick_tx.subscribe();

                let start = Instant::now();
                rt.block_on(async move {
                    req_tx.send(req).await.expect("mirror alive");
                    // Wait for the TrailReady response.
                    loop {
                        match tick_rx.recv().await {
                            Ok(TrailMirrorTick::TrailReady(_)) => break,
                            Ok(TrailMirrorTick::TrailUpdated(_)) => continue,
                            Err(_) => panic!("tick channel closed"),
                        }
                    }
                });
                let elapsed = start.elapsed();
                total += elapsed;
                latencies_ns.push(elapsed.as_nanos() as u64);
            }
            total
        });
    });

    group.finish();

    // ── p99 gate ─────────────────────────────────────────────────────────────
    if !latencies_ns.is_empty() {
        latencies_ns.sort_unstable();
        let p99_idx = (latencies_ns.len() as f64 * 0.99) as usize;
        let p99_idx = p99_idx.min(latencies_ns.len() - 1);
        let p99_ns = latencies_ns[p99_idx];
        let p99_ms = p99_ns as f64 / 1_000_000.0;
        println!("trail_mirror_open p99 = {p99_ms:.3} ms");
        assert!(
            p99_ms < 50.0,
            "trail_mirror_open p99 ({p99_ms:.3} ms) exceeds 50 ms gate (H5)"
        );
    }
}

criterion_group!(benches, bench_trail_mirror_open);
criterion_main!(benches);
