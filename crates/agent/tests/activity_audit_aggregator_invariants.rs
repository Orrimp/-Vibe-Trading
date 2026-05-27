//! cockpit-activity-audit-ledger-producer v0.1.0 — Wave D K5 invariant tests (T-D-N9).
//!
//! Tests the aggregator's panic-isolation invariant (K5 in feature.md):
//!
//! > K5 — Aggregator task panics silently. The tokio task lives for the
//! > cockpit process lifetime; a panic during the broadcast `recv()` or the
//! > `interval.tick().await` would kill the aggregator silently unless we
//! > wrap with `tokio::task::JoinHandle` polling.
//!
//! The aggregator's design does NOT have an explicit panic injection point
//! (the `AuditEvent` broadcast carries typed events, not poison-pills). The
//! K5 mitigation at v0.1.0 is:
//! 1. The `spawn_aggregator` function returns a `JoinHandle<()>` — the caller
//!    can observe task completion via `JoinHandle::await`.
//! 2. The `Lagged` arm logs `tracing::warn` — observable via the tracing
//!    subscriber without panicking.
//! 3. The `Closed` arm breaks cleanly.
//!
//! The `aggregator_panic_isolated` test is marked `#[ignore]` per the
//! architect's K5 fallback guidance (tasks.md T-D-N2 footnote) because there
//! is no safe poison-pill injection point in the typed `AuditEvent` enum.
//! The test compiles and documents the invariant; manual verification that the
//! aggregator handles `Lagged` without panicking is covered by the storm test
//! (T-D-N8) and the unit test `no_failed_events_on_happy_path`.
//!
//! Run: `cargo test -p agent --test activity_audit_aggregator_invariants`

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use tokio::sync::broadcast;

use agent::activity_audit_aggregator::spawn_aggregator;
use agent::bus::EventBus;
use agent::config::BusConfig;
use audit::tick::{AuditContext, AuditEvent, AuditTick};

// ── Helper ─────────────────────────────────────────────────────────────────────

/// Simple `KillSwitchTripped` tick — the lightest `AuditEvent` variant to construct.
fn make_simple_tick() -> AuditTick<AuditEvent> {
    use smol_str::SmolStr;
    use time::OffsetDateTime;
    use uuid::Uuid;

    AuditTick {
        event: AuditEvent::KillSwitchTripped {
            reason: SmolStr::new("test"),
        },
        context: AuditContext {
            run_id: Uuid::nil(),
            posted_at: OffsetDateTime::UNIX_EPOCH,
            agent_pid: 0,
        },
    }
}

// ── Test: aggregator handles Lagged without panicking ────────────────────────

/// K5 invariant (1/2) — The aggregator handles `RecvError::Lagged` without
/// panicking. When the tick bus is saturated, the aggregator logs a warning
/// and continues running.
///
/// This is the observable K5 mitigation: the aggregator task doesn't crash
/// on Lagged, it increments the counter and emits a tracing warn.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn aggregator_handles_lagged_without_panicking() {
    // Capacity 8 — tiny ring to force Lagged quickly.
    let (tick_tx, _) = broadcast::channel::<AuditTick<AuditEvent>>(8);
    let bus = EventBus::new(&BusConfig::default());

    // Spawn aggregator.
    let agg_handle = spawn_aggregator(Some(&tick_tx), &bus);

    // Send 100 events — the 8-slot ring will lag.
    for _ in 0..100 {
        let _ = tick_tx.send(make_simple_tick());
    }

    // Wait past one window boundary.
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Aggregator must still be running (not panicked).
    assert!(
        !agg_handle.is_finished(),
        "aggregator must still be running after Lagged events"
    );

    // Clean up.
    drop(tick_tx);
    let _ = tokio::time::timeout(Duration::from_millis(500), agg_handle).await;
}

// ── Test: aggregator exits cleanly on Closed ─────────────────────────────────

/// K5 invariant (2/2) — The aggregator exits cleanly (JoinHandle resolves
/// without panic) when the tick bus sender is dropped (`Closed`).
#[tokio::test(flavor = "current_thread")]
async fn aggregator_exits_cleanly_on_closed() {
    let (tick_tx, _) = broadcast::channel::<AuditTick<AuditEvent>>(64);
    let bus = EventBus::new(&BusConfig::default());

    let agg_handle = spawn_aggregator(Some(&tick_tx), &bus);

    // Send a few ticks.
    for _ in 0..3 {
        let _ = tick_tx.send(make_simple_tick());
    }

    // Drop sender → Closed.
    drop(tick_tx);

    // Aggregator must finish cleanly within 1 s.
    let join_result = tokio::time::timeout(Duration::from_secs(1), agg_handle)
        .await
        .expect("aggregator did not exit within 1 s after Closed");

    assert!(
        join_result.is_ok(),
        "aggregator task must exit without panic on Closed"
    );
}

// ── K5 poison-pill test (compile-only; #[ignore]) ────────────────────────────

/// K5 invariant (3/3) — Panic injection is NOT possible via the typed
/// `AuditEvent` enum (no poison-pill variant). This test documents the
/// invariant and is marked `#[ignore]` per the architect's K5 fallback:
///
/// > "If the worker design is robust enough that this is unreachable,
/// > document it inline and skip with a `#[ignore]` + comment — but the
/// > test must compile."
///
/// The aggregator's Lagged arm (`warn + fetch_add`) cannot panic on valid u64
/// arithmetic. The Closed arm breaks cleanly. The interval arm can only panic
/// if the `ActivityHandle::tick` panics — which it cannot (it's a no-op on
/// throttle or a broadcast send that silently drops on no-receivers).
/// Therefore, the aggregator is panic-free by construction at v0.1.0.
///
/// **v0.1.1 forward-list**: if a future variant introduces a `#[non_exhaustive]`
/// match in the aggregator, revisit this test.
#[test]
#[ignore = "K5 poison-pill injection not possible via typed AuditEvent — aggregator is panic-free by construction at v0.1.0 (ADR-0044 § D2 / tasks.md T-D-N2 K5 fallback)"]
fn aggregator_panic_isolated() {
    // This test intentionally does nothing — it exists to document the K5
    // invariant and confirm the test compiles. The `#[ignore]` is per the
    // architect's K5 fallback guidance in tasks.md T-D-N2.
    //
    // If you are reading this in v0.2.0+:
    // 1. Check if `AuditEvent` has any `#[non_exhaustive]` arms in the
    //    aggregator's recv handler.
    // 2. If yes, implement a poison-pill via a custom `AuditEvent::Unknown`
    //    or a mock broadcast that sends panic-triggering payloads.
    // 3. Remove this `#[ignore]` and verify the aggregator's JoinHandle
    //    detects the panic and logs it.
    //
    // ADR-0044 § "What costs this incurs" / K5: "For now, a tracing::warn!
    // on the recv-loop's Err(Lagged) arm + a #[ignore]-marked
    // aggregator_panic_isolated test document the gap."
}
