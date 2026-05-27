//! cockpit-activity-audit-ledger-producer v0.1.0 — Wave B integration test (T-D-N5).
//!
//! Asserts that:
//! 1. The aggregator starts successfully on cockpit boot (can be spawned).
//! 2. The aggregator emits its first activity event within 1 s of the first `AuditTick`.
//!
//! This test constructs the minimum plumbing needed to verify the aggregator's
//! boot contract WITHOUT launching the full iced application (which requires macOS
//! GUI and a running tokio runtime that lives for the process lifetime).
//!
//! Run: `cargo test -p ui --test cockpit_audit_aggregator_boot`

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use tokio::sync::broadcast;
use tokio::time::timeout;

use agent::activity::ActivityPhase;
use agent::activity_audit_aggregator::spawn_aggregator;
use agent::config::BusConfig;
use agent::bus::EventBus;
use audit::tick::{AuditContext, AuditEvent, AuditTick};

// ── Helper: synthetic AuditTick ───────────────────────────────────────────────

fn make_audit_tick() -> AuditTick<AuditEvent> {
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;
    use uuid::Uuid;
    use trading_core::{
        FeeTier, Fill, FillId, Liquidity, Money, OrderId, Price, Quantity, Side, Symbol, Timestamp,
    };

    AuditTick {
        event: AuditEvent::Fill {
            fill: Box::new(Fill {
                id: FillId::new(),
                order_id: OrderId::new(),
                symbol: Symbol::new("BTCUSDT"),
                side: Side::Buy,
                qty: Quantity::new(dec!(0.1)).unwrap(),
                price: Price::new(dec!(40_000)).unwrap(),
                fee: Money::from_decimal(dec!(1.6)),
                fee_tier: FeeTier::Taker,
                venue_ts: Timestamp::new(OffsetDateTime::UNIX_EPOCH),
                local_ts: Timestamp::new(OffsetDateTime::UNIX_EPOCH),
                liquidity: Liquidity::Taker,
                transaction_id: None,
            }),
            fees: dec!(1.6),
        },
        context: AuditContext {
            run_id: Uuid::nil(),
            posted_at: OffsetDateTime::UNIX_EPOCH,
            agent_pid: 0,
        },
    }
}

// ── Test: aggregator boots and emits first event within 1 s ──────────────────

/// T-D-N5 — aggregator starts on cockpit boot and emits its first activity event
/// within 1 second of the first `AuditTick`.
///
/// Acceptance criteria:
/// 1. `spawn_aggregator` returns without panicking.
/// 2. A single `AuditTick` sent on the tick bus produces at least one activity
///    event (Start or Tick) within 1 s.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn aggregator_starts_and_emits_first_event_within_1s() {
    // ── 1. Build the tick bus (simulates `audit::Ledger::open_with_tick_bus`) ──
    let (tick_tx, _) = broadcast::channel::<AuditTick<AuditEvent>>(256);

    // ── 2. Build the EventBus (simulates cockpit_live boot) ──────────────────
    let bus = EventBus::new(&BusConfig::default());

    // ── 3. Subscribe to the activity channel BEFORE spawning the aggregator ──
    // This mimics the iced ActivityRecipe subscribing before the aggregator
    // starts emitting. The iced subscription is staged before the aggregator
    // spawns (K6 ordering — in cockpit_live.rs the aggregator is inside
    // `rt.block_on` which runs after `iced::application.run()` stages subs).
    let mut activity_rx = bus.activity().subscribe();

    // ── 4. Spawn the aggregator (the K6-ordered call) ────────────────────────
    let agg_handle = spawn_aggregator(Some(&tick_tx), &bus);

    // Assert: aggregator spawned without panic.
    assert!(!agg_handle.is_finished(), "aggregator task must be running");

    // ── 5. Send one AuditTick to the bus ─────────────────────────────────────
    let send_result = tick_tx.send(make_audit_tick());
    assert!(send_result.is_ok(), "tick send must succeed with aggregator subscribed");

    // ── 6. Assert: first activity event arrives within 1 s ───────────────────
    // The aggregator emits on the first non-empty 100 ms window. We wait up
    // to 1 s (10× the window) to allow for tokio scheduler variation.
    let first_event = timeout(Duration::from_secs(1), activity_rx.recv())
        .await
        .expect("timeout: no activity event within 1 s of first AuditTick")
        .expect("activity channel closed unexpectedly");

    // The first event must be a Start (since the aggregator starts a fresh handle
    // on the first non-empty window).
    assert!(
        matches!(first_event.phase, ActivityPhase::Start { .. }),
        "first activity event must be a Start, got {:?}",
        first_event.phase
    );

    // Assert: the event is from the AuditLedgerWrite kind.
    assert_eq!(
        first_event.kind,
        agent::ActivityKind::AuditLedgerWrite,
        "first event kind must be AuditLedgerWrite"
    );

    // ── 7. Clean up ──────────────────────────────────────────────────────────
    drop(tick_tx);
    // Allow the aggregator to observe Closed and exit.
    let _ = timeout(Duration::from_millis(500), agg_handle).await;
}

// ── Test: aggregator no-op when tick bus is None ─────────────────────────────

/// T-D-N5 (auxiliary) — `spawn_aggregator(None, &bus)` spawns a no-op task.
///
/// When `tick_bus_capacity = 0` (config gate), `open_with_tick_bus` is not
/// called and `_tick_bus_sender` is `None`. The no-op path must compile and
/// run without panicking.
#[tokio::test(flavor = "current_thread")]
async fn aggregator_noop_when_tick_sender_is_none() {
    let bus = EventBus::new(&BusConfig::default());
    let mut activity_rx = bus.activity().subscribe();

    // Spawn with None — should return immediately.
    let agg_handle = spawn_aggregator(None, &bus);

    // The no-op task finishes instantly (or near-instantly).
    let join_result = timeout(Duration::from_millis(200), agg_handle).await;
    assert!(
        join_result.is_ok(),
        "no-op aggregator must finish quickly (timeout: 200 ms)"
    );

    // No activity events should have been emitted.
    let recv_result = activity_rx.try_recv();
    assert!(
        recv_result.is_err(),
        "no-op aggregator must not emit any activity events"
    );
}
