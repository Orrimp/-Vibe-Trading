//! `ActivityAuditAggregator` — Wave B select-arm survival tests (T-D-B2).
//!
//! Validates that the `tokio::select!` two-arm loop in `run_aggregator_loop`
//! does NOT starve the `rx.recv()` arm when the `interval.tick()` arm fires
//! repeatedly.
//!
//! ## Bug #64 regression class
//!
//! The Bug #64 select-arm starvation pattern: if either arm gets starved by a
//! future `biased;` reordering or a tick-vs-recv priority swap, audit events
//! get dropped silently and — critically — the `Closed` event from the tick
//! bus never reaches the recv arm, causing the aggregator loop to hang
//! indefinitely instead of exiting cleanly.
//!
//! ## Observation strategy
//!
//! Both survival tests (1 + 2) observe recv-arm liveness via the aggregator's
//! **exit behaviour**: when the audit tick bus is closed (`tx` dropped), the
//! recv arm sees `RecvError::Closed` and `break`s the loop, letting the task
//! finish within the timeout.  If the recv arm is replaced with `pending`
//! (P-B1), it never processes `Closed`, the loop hangs, and the timeout
//! assertion FAILs.
//!
//! A third test (`recv_arm_increments_counter`) asserts on the activity-channel
//! `Start` event — providing the D-V0.2.0-3 row-4 probe coverage (comment out
//! `fetch_add` → counter stays 0 → no `Start` event).  This test is
//! deliberately decoupled from the survival tests so P-B2 (interval arm no-op)
//! does NOT affect the survival-test verdicts.
//!
//! ## T-T4 falsification probes (per D-V0.2.0-3, Wave B rows 4 + 5)
//!
//! **P-B1 — recv arm replaced with `futures::future::pending::<()>()`
//! (EXPECTED FAIL on survival tests 1 + 2)**
//!
//! Mutation: in `crates/agent/src/activity_audit_aggregator.rs`, inside
//! `pub async fn run_aggregator_loop`, replace the recv arm body:
//!
//! ```text
//! // BEFORE (production):
//! recv_result = agg.rx.recv() => { ... }
//!
//! // AFTER (P-B1 probe):
//! _ = futures::future::pending::<()>() => {}
//! ```
//!
//! Expected FAILing tests: `recv_arm_increments_after_interval_fires` AND
//! `recv_arm_survives_n_interval_boundaries` — both FAIL with
//! "aggregator did not exit within 500 ms after bus.close()".
//! `recv_arm_increments_counter` also FAILs (recv never fires → counter stays
//! 0 → no Start event).
//!
//! **P-B2 — interval arm body replaced with no-op `{}` (EXPECTED PASS on
//! survival tests 1 + 2 — negative control)**
//!
//! Mutation: replace the interval arm body with an empty block:
//!
//! ```text
//! // BEFORE (production):
//! _ = agg.interval.tick() => { let n = agg.counter.swap(...); ... }
//!
//! // AFTER (P-B2 probe):
//! _ = agg.interval.tick() => {}
//! ```
//!
//! Expected: `recv_arm_increments_after_interval_fires` and
//! `recv_arm_survives_n_interval_boundaries` PASS — the recv arm is unaffected;
//! it still processes `Closed` and exits.  `recv_arm_increments_counter` FAILS
//! (interval arm no-op → no `Start` event emitted) — this is intentional and
//! documents that P-B2 is the negative control for the SURVIVAL tests only.
//!
//! This proves P-B1 is not a tautology: the two survival tests are sensitive to
//! recv-arm starvation specifically, not to the interval arm firing.
//!
//! ## Run
//!
//! ```
//! cargo test -p agent --test activity_audit_aggregator_select_arm_survival
//! ```

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use tokio::sync::broadcast;

use agent::activity::ActivityPhase;
use agent::activity_audit_aggregator::{Aggregator, run_aggregator_loop};
use agent::bus::EventBus;
use agent::config::BusConfig;
use audit::tick::{AuditContext, AuditEvent, AuditTick};

// ── MockAuditTickBus ──────────────────────────────────────────────────────────

/// Per-Recipe mock for `ActivityAuditAggregator` (D-V0.2.0-1 / D-V0.2.0-2).
///
/// Wraps a real `tokio::sync::broadcast::channel::<AuditTick<AuditEvent>>(16)`.
/// The mock holds the `Sender`; tests drive `tx.send(tick)` interleaved with
/// `tokio::time::advance(...)` to fire the `interval.tick()` arm.
struct MockAuditTickBus {
    tx: broadcast::Sender<AuditTick<AuditEvent>>,
}

impl MockAuditTickBus {
    fn new() -> Self {
        let (tx, _) = broadcast::channel(16);
        Self { tx }
    }

    /// Subscribe to the bus (gives to the `Aggregator::new` constructor).
    fn subscribe(&self) -> broadcast::Receiver<AuditTick<AuditEvent>> {
        self.tx.subscribe()
    }

    /// Send a single `KillSwitchTripped` tick (the lightest `AuditEvent` variant).
    fn send_tick(&self) {
        use smol_str::SmolStr;
        // Use crate-rooted path to avoid shadowing by any local `time` alias.
        use ::time::OffsetDateTime;
        use uuid::Uuid;

        let tick = AuditTick {
            event: AuditEvent::KillSwitchTripped {
                reason: SmolStr::new("test"),
            },
            context: AuditContext {
                run_id: Uuid::nil(),
                posted_at: OffsetDateTime::UNIX_EPOCH,
                agent_pid: 0,
            },
        };
        let _ = self.tx.send(tick);
    }

    /// Close the bus so the aggregator loop exits cleanly (recv arm sees `Closed`).
    fn close(self) {
        drop(self.tx);
    }
}

// ── Test 1: recv_arm_increments_after_interval_fires ─────────────────────────

/// T-D-B2 test 1 — advance 100 ms (interval fires once) then send a tick;
/// the aggregator must exit cleanly when the bus is closed.
///
/// ## Protocol
///
/// 1. `start_paused = true` — take control of all tokio timers.
/// 2. Spawn `run_aggregator_loop(agg)` as a background task.
/// 3. `advance(100 ms)` — fires the first interval tick (counter=0 → idle,
///    no activity event).
/// 4. `tx.send(tick)` — tick enters the broadcast channel.
/// 5. Yield — let task process the recv arm (counter := 1).
/// 6. `advance(100 ms)` — fires the second interval; counter=1 → emits `Start`.
/// 7. Yield.
/// 8. `bus.close()` — recv arm sees `Closed` → loop exits.
/// 9. Assert loop exits within 500 ms (PRIMARY assertion — decoupled from
///    interval arm behaviour).
///
/// **P-B1 falsification**: replace recv arm with `pending` →
/// `Closed` never arrives → loop hangs → timeout fires → this test FAILs.
///
/// **P-B2 negative control**: interval arm body is a no-op → no `Start`
/// event, but recv arm still processes `Closed` → loop exits → PASS.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn recv_arm_increments_after_interval_fires() {
    let bus = MockAuditTickBus::new();
    let event_bus = EventBus::new(&BusConfig::default());

    let rx = bus.subscribe();
    let agg = Aggregator::new(rx, event_bus.activity());

    // Spawn the loop.
    let loop_handle = tokio::spawn(run_aggregator_loop(agg));

    // Step 1: advance past the first interval boundary (counter=0 → idle).
    tokio::time::advance(Duration::from_millis(100)).await;

    // Step 2: send one tick.
    bus.send_tick();

    // Yield to let the spawned task process the recv arm (counter := 1).
    tokio::task::yield_now().await;

    // Step 3: advance past the second interval boundary (counter=1 → Start).
    tokio::time::advance(Duration::from_millis(100)).await;
    tokio::task::yield_now().await;

    // Step 4: close the bus → recv arm sees Closed → loop exits.
    bus.close();

    // PRIMARY assertion: loop exits within 500 ms.
    // P-B1 mutation causes this to FAIL with Elapsed.
    // P-B2 mutation (interval no-op) does NOT affect this assertion — PASS.
    let result = tokio::time::timeout(Duration::from_millis(500), loop_handle).await;
    assert!(
        result.is_ok(),
        "aggregator did not exit within 500 ms after bus.close() — \
         recv arm must process RecvError::Closed to break the loop. \
         P-B1 falsification: replace recv arm with pending → this FAILs."
    );
}

// ── Test 2: recv_arm_survives_n_interval_boundaries ──────────────────────────

/// T-D-B2 test 2 — advance 500 ms (5 interval fires) then send ticks;
/// the recv arm must still be alive and process the subsequent `Closed` signal.
///
/// ## Protocol
///
/// 1. `start_paused = true`.
/// 2. Spawn `run_aggregator_loop(agg)`.
/// 3. `advance(500 ms)` — 5 interval boundaries, all with counter=0 → idle.
/// 4. Send 3 ticks — recv arm must still be listening after 5 idle boundaries.
/// 5. Yield — let task process the recv arm (counter := 3).
/// 6. `advance(100 ms)` — 6th interval fires; counter=3 → emits `Start`.
/// 7. Yield.
/// 8. `bus.close()` → recv arm sees `Closed` → loop exits.
/// 9. Assert loop exits within 500 ms (PRIMARY assertion).
///
/// **P-B1 falsification**: replace recv arm with `pending` →
/// loop never exits → timeout → FAIL.
///
/// **P-B2 negative control**: interval arm body is no-op → no `Start`
/// emitted, but recv arm still processes `Closed` → PASS.
/// This proves P-B1 is not a tautology: the survival tests are sensitive to
/// recv-arm starvation, not to the interval arm body.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn recv_arm_survives_n_interval_boundaries() {
    let bus = MockAuditTickBus::new();
    let event_bus = EventBus::new(&BusConfig::default());

    let rx = bus.subscribe();
    let agg = Aggregator::new(rx, event_bus.activity());

    // Spawn the loop.
    let loop_handle = tokio::spawn(run_aggregator_loop(agg));

    // Step 1: advance 500 ms — 5 interval boundaries, all counter=0 → idle.
    tokio::time::advance(Duration::from_millis(500)).await;

    // Step 2: send 3 ticks — recv arm must still be listening.
    bus.send_tick();
    bus.send_tick();
    bus.send_tick();

    // Yield to let the spawned task process the recv arm (counter := 3).
    tokio::task::yield_now().await;

    // Step 3: advance 100 ms — 6th interval fires; counter=3 → emits Start.
    tokio::time::advance(Duration::from_millis(100)).await;
    tokio::task::yield_now().await;

    // Step 4: close the bus → recv arm sees Closed → loop exits.
    bus.close();

    // PRIMARY assertion: loop exits within 500 ms.
    // P-B1 mutation (recv arm = pending) causes this to FAIL.
    // P-B2 mutation (interval no-op) does NOT affect this — PASS.
    let result = tokio::time::timeout(Duration::from_millis(500), loop_handle).await;
    assert!(
        result.is_ok(),
        "aggregator did not exit within 500 ms after bus.close() — \
         recv arm must survive N interval boundaries and process RecvError::Closed. \
         P-B1 falsification: replace recv arm with pending → this FAILs."
    );
}

// ── Test 3: recv_arm_increments_counter ──────────────────────────────────────

/// T-D-B2 test 3 — assert the recv arm actually increments the counter,
/// verified via the activity-channel `Start` event.
///
/// This test covers D-V0.2.0-3 row-4: commenting out `fetch_add(1)` in the
/// recv arm causes the counter to stay 0 → interval arm sees counter=0 → no
/// `Start` event → assertion FAILs.
///
/// **Note on P-B2**: under the P-B2 probe (interval arm no-op), this test
/// FAILs because no `Start` event is emitted (counter fills but isn't drained).
/// This is documented and expected — P-B2 is the negative control for the
/// SURVIVAL tests (tests 1 + 2) only, not for this counter-increment test.
///
/// ## Falsification (D-V0.2.0-3 row 4)
///
/// Comment out `agg.counter.fetch_add(1, Ordering::Relaxed)` in the
/// `Ok(_tick)` arm of `run_aggregator_loop`. The counter stays 0,
/// the interval arm sees counter=0, no `Start` event is emitted,
/// and `assert!(start_count >= 1, ...)` FAILs.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn recv_arm_increments_counter() {
    let bus = MockAuditTickBus::new();
    let event_bus = EventBus::new(&BusConfig::default());
    let mut activity_rx = event_bus.activity().subscribe();

    let rx = bus.subscribe();
    let agg = Aggregator::new(rx, event_bus.activity());

    let loop_handle = tokio::spawn(run_aggregator_loop(agg));

    // Advance past the first interval boundary (counter=0 → idle, no Start yet).
    tokio::time::advance(Duration::from_millis(100)).await;

    // Send one tick — recv arm fires, counter := 1.
    bus.send_tick();
    tokio::task::yield_now().await;

    // Advance past the second interval boundary — counter=1 → emits Start.
    tokio::time::advance(Duration::from_millis(100)).await;
    tokio::task::yield_now().await;

    // Close and wait for clean exit.
    bus.close();
    let _ = tokio::time::timeout(Duration::from_millis(500), loop_handle).await;

    // Assert ≥ 1 Start event (proves recv arm incremented the counter).
    let mut start_count = 0usize;
    while let Ok(ev) = activity_rx.try_recv() {
        if matches!(ev.phase, ActivityPhase::Start { .. }) {
            start_count += 1;
        }
    }
    assert!(
        start_count >= 1,
        "expected ≥ 1 Start event — proves recv arm incremented counter \
         (fetch_add) and interval arm drained it (counter.swap). Got {start_count}. \
         Row-4 probe: comment out fetch_add → counter stays 0 → this FAILs."
    );
}
