//! cockpit-activity-audit-ledger-producer v0.1.0 — Wave B integration test (T-D-N5).
//!
//! Asserts that:
//! 1. The aggregator starts successfully on cockpit boot (can be spawned).
//! 2. The aggregator emits its first activity event within 1 s of the first
//!    `AuditTick`: a `Start` labelled `"Audit: 1 writes"` for the one tick sent.
//! 3. Closing the tick bus makes the aggregator EXIT CLEANLY (no hang, no
//!    panic), and the channel then holds exactly the `End(Success)` of the
//!    activity it started: no `Tick`, no second activity.
//! 4. With no tick bus (`spawn_aggregator(None, ..)`), the no-op task finishes
//!    cleanly and emits nothing.
//!
//! This test constructs the minimum plumbing needed to verify the aggregator's
//! boot contract WITHOUT launching the full iced application (which requires macOS
//! GUI and a running tokio runtime that lives for the process lifetime).
//!
//! Run: `cargo test -p ui --test cockpit_audit_aggregator_boot`
//!
//! ## Non-vacuity contract (bug-log #66 / story 2-63 AC3)
//!
//! A task's exit is read through BOTH layers of its result: the `timeout`
//! (`Err(Elapsed)` = a hang) and the `JoinHandle` (`Err(JoinError)` = a panic).
//! Before story 2-63 Wave E2 the first test discarded that result (`let _ =`)
//! and the second checked only the timeout layer with `is_ok()`, which a
//! panicked task satisfies, so a panic on shutdown passed both. "Nothing was
//! emitted" is `Err(TryRecvError::Empty)` exactly: a lagged or closed channel
//! no longer counts as empty.
//!
//! What test 1 pins exactly, in real time: the event LIST. One tick gives one
//! window, so one `Start` (`"Audit: 1 writes"`) and then one `End(Success)`
//! under its id, with nothing else (story 2-63 Wave E2 follow-up; until then
//! test 1 accepted any list that contained such an `End`). Its doc gives the
//! source facts this rests on.
//!
//! What test 1 does NOT pin: WHICH `End` source fired. In `run_aggregator_loop`
//! the aggregator ends its activity either at the next idle window,
//! `agg.handle = None; // Drop impl emits End { Success }`, or on exit,
//! `drop(agg.handle);`, and both yield `Success` (reasoned). This test runs in
//! real time, so either can come first. The deterministic End-on-close pin is
//! the paused-clock pair in
//! `crates/agent/tests/activity_audit_aggregator_select_arm_survival.rs`, and
//! the deterministic idle-arm `End` is pinned by that file's test 5,
//! `idle_window_ends_the_activity_before_the_close` (story 2-63 Wave E2, pass 3).
//!
//! ## T-T4 falsification probe table (story 2-63 AC4)
//!
//! Each probe is ONE edit to production code, located by symbol + the quoted
//! line text (line numbers rot). Procedure: snapshot the file (`cp -p`), apply
//! the edit, run `cargo test -p ui --test cockpit_audit_aggregator_boot` (the
//! named test goes RED), restore from the snapshot THEN `touch` it, and re-run:
//! the GREEN counts only if that run shows `Compiling agent` and the file's
//! SHA-256 equals the pre-probe one.
//!
//! | # | symbol (file) | quoted line today | mutation | expected RED |
//! |---|---|---|---|---|
//! | E2-P4 | `agent::activity_audit_aggregator::run_aggregator_loop`, its `RecvError::Closed` arm (`crates/agent/src/activity_audit_aggregator.rs`) | `// Drop the handle to emit End{Success} and exit.`, and the `break;` under it | insert `if agg.counter.load(Ordering::Relaxed) < u32::MAX { panic!("E2-P4 probe: aggregator panics on shutdown"); }` before that `break;` | `aggregator_starts_and_emits_first_event_within_1s`, at "aggregator task must not panic on shutdown". GREEN before Wave E2: its `let _ =` teardown discarded the `JoinError` |
//! | E2-P6 | `agent::activity_audit_aggregator::spawn_aggregator`, its `None` arm | `tokio::spawn(async {})` | → `tokio::spawn(async { panic!("E2-P6 probe: no-op aggregator panics") })` | `aggregator_noop_when_tick_sender_is_none`, at "no-op aggregator task must not panic". GREEN before Wave E2: `join_result.is_ok()` held for `Ok(Err(JoinError))`, and this test is the only caller of `spawn_aggregator(None, ..)` in the suite |
//! | E2-P15 | `agent::activity_audit_aggregator::run_aggregator_loop`, its `(n, None)` arm (the first non-empty window) | `let label = Aggregator::format_label(n);` | → `let label = Aggregator::format_label(n + 1);` (the `Start` reports one write too many) | `aggregator_starts_and_emits_first_event_within_1s`, at "the Start's label counts the one tick sent" (left `"Audit: 2 writes"`, right `"Audit: 1 writes"`). GREEN before the Wave E2 follow-up: no ui test pinned the label. MEASURED (falsifier pass 2, 2026-11-14), this test's red and its siblings: agent `activity_audit_aggregator_select_arm_survival`'s `recv_arm_increments_counter` (`one tick between two interval boundaries, closed before the next ⇒ exactly [Start("Audit: 1 writes"), End(Success)]`, left label `"Audit: 2 writes"`) and `recv_arm_counts_every_tick_after_idle_windows` (`three ticks after five idle windows ⇒ exactly [Start("Audit: 3 writes"), End(Success)]`, left label `"Audit: 4 writes"`), at the full-sequence assert of its `assert_one_activity_ended_by_close`; its tests 1-2 stayed green (they pin exit only), and so did this file's `aggregator_noop_when_tick_sender_is_none`. `activity_tape_audit_ledger_event_storm` stayed green (MEASURED, the same run): it floors coverage from below, and the probe adds one. That file's test 5, `idle_window_ends_the_activity_before_the_close` (added in pass 3), is red too, at its first drain's label, `"Audit: 2 writes"` (MEASURED, falsifier pass 3, 2026-11-15) |
//!
//! Sibling, rowed elsewhere: story 2-63's designated probe E2-P0 (the recv
//! arm's `fetch_add` deleted, rowed in the agent `select_arm_survival` file)
//! turns `aggregator_starts_and_emits_first_event_within_1s` red as well, at
//! "timeout: no activity event within 1 s of first AuditTick" (MEASURED,
//! falsifier pass 2, 2026-11-14: "timeout: no activity event within 1 s of
//! first AuditTick: Elapsed(())"). It is not unique to this file.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use tokio::sync::broadcast;
use tokio::sync::broadcast::error::TryRecvError;
use tokio::time::timeout;

use agent::activity::{ActivityOutcome, ActivityPhase};
use agent::activity_audit_aggregator::spawn_aggregator;
use agent::bus::EventBus;
use agent::config::BusConfig;
use audit::tick::{AuditContext, AuditEvent, AuditTick};

/// The ticks test 1 sends: ONE. The expected `Start` label is derived from it.
/// It must stay 1: with more ticks, real-time scheduling could split them
/// across windows (a `Start`, then a `Tick`), and the exact event list test 1
/// pins would no longer hold.
const TICKS_SENT: u32 = 1;

// ── Helper: synthetic AuditTick ───────────────────────────────────────────────

fn make_audit_tick() -> AuditTick<AuditEvent> {
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;
    use trading_core::{
        FeeTier, Fill, FillId, Liquidity, Money, OrderId, Price, Quantity, Side, Symbol, Timestamp,
    };
    use uuid::Uuid;

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
/// 2. A single `AuditTick` sent on the tick bus produces, within 1 s, a `Start`
///    whose label counts that one write: `"Audit: 1 writes"`, the 1 derived
///    from [`TICKS_SENT`] (T-T4 E2-P15).
/// 3. Dropping the tick sender (the ledger shutting down) makes the aggregator
///    task exit within 2 s WITHOUT panicking (T-T4 E2-P4).
/// 4. After the `Start`, the channel holds EXACTLY one more event: the
///    `End(Success)` of the Start's activity. No `Tick`, no second activity.
///
/// Why 4 is exact in real time (REASONED from `run_aggregator_loop`, checked in
/// source for story 2-63 Wave E2's follow-up): a `Tick` comes only from its
/// `(n, Some(h))` arm, which needs a tick counted AFTER the `Start`, and this
/// test sends one tick in all. A `Start` comes only from the `(n, None)` arm,
/// so one tick gives one window and one activity. Its `End` is `Success`
/// whichever source sends it, the idle window or the exit's `drop`, and it is
/// sent once: the handle is dropped once. What stays unpinned is WHICH source
/// sent it (module doc). The "no `Tick`" half rests on that premise and is not
/// pinned here against a loop that counts the first window again after its
/// `Start`: the falsifier's pass-2 OWN-3 did exactly that and this test stayed
/// GREEN (MEASURED, 2026-11-14). The agent select_arm file's test 5 catches
/// that loop on the paused clock (T-T4 E2-P16 there) through the missing idle
/// `End`: its tick is throttled, because `ActivityHandle` reads
/// `std::time::Instant`; a `Tick` itself is not reliably observable there: only
/// a real-time stall of 100 ms or more lets one through, and drain 2 is red
/// either way (REASONED).
/// Test 5's red there, at its drain 2 with `left: []`, is MEASURED (falsifier
/// pass 3, 2026-11-15).
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

    // ── 5. Send TICKS_SENT (one) AuditTick to the bus ────────────────────────
    for _ in 0..TICKS_SENT {
        let send_result = tick_tx.send(make_audit_tick());
        assert!(
            send_result.is_ok(),
            "tick send must succeed with aggregator subscribed"
        );
    }

    // ── 6. Assert: first activity event arrives within 1 s ───────────────────
    // The aggregator emits on the first non-empty 100 ms window. We wait up
    // to 1 s (10× the window) to allow for tokio scheduler variation.
    let first_event = timeout(Duration::from_secs(1), activity_rx.recv())
        .await
        .expect("timeout: no activity event within 1 s of first AuditTick")
        .expect("activity channel closed unexpectedly");

    // The first event must be a Start (since the aggregator starts a fresh handle
    // on the first non-empty window). Nothing came before it: the receiver
    // subscribed before the spawn, and the aggregator is the bus's only producer.
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

    // Assert: the Start counts the ticks sent (derived, never read off the
    // aggregator). T-T4 E2-P15 reports one write too many and fails here.
    assert_eq!(
        first_event.label,
        format!("Audit: {TICKS_SENT} writes"),
        "the Start's label counts the one tick sent"
    );

    // ── 7. Shut down: the task must EXIT, and exit CLEANLY ───────────────────
    // Closing the tick bus is the aggregator's shutdown signal: its recv arm
    // sees `RecvError::Closed`, breaks out of the loop and drops its handle.
    // Both result layers are read: the timeout (a hang) and the join (a panic).
    drop(tick_tx);
    timeout(Duration::from_secs(2), agg_handle)
        .await
        .expect("aggregator must exit within 2 s of the tick bus closing")
        .expect("aggregator task must not panic on shutdown");

    // ── 8. Exactly one more event: the Start's End(Success) ──────────────────
    // The task has exited, so whichever End source fired (the idle window or
    // the exit's drop) has already sent its event.
    let mut later_events = Vec::new();
    loop {
        match activity_rx.try_recv() {
            Ok(event) => later_events.push(event),
            Err(TryRecvError::Empty) => break,
            Err(e) => panic!("the activity channel must not lag or close here: {e:?}"),
        }
    }
    assert!(
        matches!(
            later_events.as_slice(),
            [end] if end.id == first_event.id
                && matches!(end.phase, ActivityPhase::End(ActivityOutcome::Success))
        ),
        "after the Start, exactly one event: the Start's activity ended with End(Success) \
         (no Tick, no second activity; got {later_events:?})"
    );
}

// ── Test: aggregator no-op when tick bus is None ─────────────────────────────

/// T-D-N5 (auxiliary) — `spawn_aggregator(None, &bus)` spawns a no-op task.
///
/// When `tick_bus_capacity = 0` (config gate), `open_with_tick_bus` is not
/// called and `_tick_bus_sender` is `None`. The no-op task must finish within
/// 200 ms WITHOUT panicking (T-T4 E2-P6), and leave the activity channel EMPTY.
///
/// It is the `None` half of a pair on the tick-sender dimension: with
/// `Some(sender)` and one tick, the test above OBSERVES a Start; here the
/// channel holds exactly nothing.
#[tokio::test(flavor = "current_thread")]
async fn aggregator_noop_when_tick_sender_is_none() {
    let bus = EventBus::new(&BusConfig::default());
    let mut activity_rx = bus.activity().subscribe();

    // Spawn with None — should return immediately.
    let agg_handle = spawn_aggregator(None, &bus);

    // The no-op task finishes near-instantly. Both result layers are read:
    // the timeout (a hang) and the join (a panic).
    timeout(Duration::from_millis(200), agg_handle)
        .await
        .expect("no-op aggregator must finish within 200 ms")
        .expect("no-op aggregator task must not panic");

    // No activity events, and the channel neither lagged nor closed.
    let recv_result = activity_rx.try_recv();
    assert!(
        matches!(recv_result, Err(TryRecvError::Empty)),
        "no-op aggregator must leave the activity channel empty (got {recv_result:?})"
    );
}
