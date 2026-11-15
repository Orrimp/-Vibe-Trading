//! `ActivityAuditAggregator`: select-arm survival and window-count tests
//! (T-D-B2; tightened by story 2-63 Wave E2).
//!
//! Validates three things about the `tokio::select!` two-arm loop in
//! `run_aggregator_loop`. The `rx.recv()` arm is NOT starved when the
//! `interval.tick()` arm fires repeatedly. The loop EXITS CLEANLY (no hang, no
//! panic) when the tick bus closes. A window's tick count reaches the activity
//! channel exactly, and the activity it starts is ended exactly once: by the
//! close (tests 3-4) or, when the next window is idle, by the idle arm while
//! the loop keeps running (test 5, whose next tick then starts a fresh activity
//! that the close ends).
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
//! - The two survival tests (1 + 2) observe recv-arm liveness via the loop's
//!   **exit**. When the tick bus is closed (`tx` dropped), the recv arm sees
//!   `RecvError::Closed` and `break`s. The exit is EXPECTED twice over: the
//!   500 ms `timeout` must not elapse (a hang FAILs), and the task's
//!   `JoinResult` must be `Ok` (a panic on `Closed` FAILs). Before Wave E2 they
//!   asserted `result.is_ok()` on the timeout layer only, which a task that
//!   PANICKED on `Closed` also satisfies: `Ok(Err(JoinError))` (probe E2-P4).
//!   They pin exit only, so the interval arm (P-B2) cannot change their verdict.
//! - `recv_arm_increments_counter` (1 tick) and
//!   `recv_arm_counts_every_tick_after_idle_windows` (3 ticks after five idle
//!   windows) expect the same clean exit, then pin the activity channel
//!   EXACTLY: `[Start(AuditLedgerWrite, "Audit: N writes"), End(Success)]`
//!   under one `ActivityId`, with N derived from the ticks the test sent. The
//!   two are a pair on the window count (N = 1 against N = 3), so a count that
//!   is right for one tick and wrong for several (E2-P5) passes one and fails
//!   the other.
//! - Those two also pin END-ON-CLOSE. A drain taken after the last window
//!   boundary, just before `bus.close()`, must hold the `Start` ALONE. The
//!   full sequence alone cannot tell an `End` emitted by the close from one
//!   emitted when the window started (E2-A1 passes the full-sequence check).
//!   Both tests close before the next interval boundary, so the idle arm cannot
//!   be the `End`'s source either: the clock is paused and nothing advances it
//!   while the loop task is runnable.
//! - `idle_window_ends_the_activity_before_the_close` (test 5) is the idle-arm
//!   half. It runs test 3's protocol plus ONE more boundary with no tick sent,
//!   and pins that boundary's drain as exactly `[End(Success)]` under the
//!   `Start`'s `ActivityId`. Then it asserts the loop is still running, and a
//!   resume half (one more tick and boundary, then the close) must give one
//!   fresh activity ended by the close, checked by test 3's helper. Why it
//!   exists: because tests 3-4 close before the next boundary, what the loop
//!   does there is outside what they observe, so the "no `Tick`" half of their
//!   exact lists is observed only inside the `Start`'s own window. MEASURED
//!   (falsifier pass 2, 2026-11-14, its OWN-3, now row E2-P16): re-counting the
//!   first window after its `Start` left tests 1-4 GREEN, and in that run only
//!   the real-time `activity_audit_aggregator::aggregator_idle_drops_handle`
//!   went red ("exactly 1 End{Success}", left 0, right 1). That sibling red is
//!   timing-dependent: under the same edit falsifier pass 3 (2026-11-15)
//!   MEASURED it red in 3 of 21 runs (GREEN in the main run, red in 3 of 20
//!   repeats). REASONED cause: `TICK_THROTTLE` and `AGGREGATION_WINDOW` are
//!   both 100 ms, and the interval's first tick is immediate, so which
//!   boundary lets the re-counted window through is jitter. Test 5's drain 2 is
//!   the deterministic red: it reddens there under E2-P16 and under E2-P17 (the
//!   idle arm keeps its handle), MEASURED (falsifier pass 3, 2026-11-15).
//! - Why test 5 asserts the loop is still running after drain 2: with nothing
//!   sent, that drain's `End` comes from the idle arm OR from the loop's exit,
//!   whose handle drop sends the same `End`. MEASURED (falsifier pass 3,
//!   2026-11-15, its OWN-C, now row E2-P20): an idle arm that `break`s the loop
//!   left all five tests GREEN, test 5 then included; of the two targets run
//!   (this file and `activity_audit_aggregator`), only the real-time
//!   `aggregator_idle_drops_handle` ("aggregator must still be running after
//!   idle-end") and `aggregator_handle_resumes_after_idle` ("burst2 must
//!   produce at least 1 event") went red; the invariants, boot and storm
//!   targets were not run under it. The `is_finished` assert closes that
//!   gap on the paused clock: under E2-P20 test 5 goes red there and tests 1-4
//!   stay GREEN (MEASURED, pass-4 fixer, 2026-11-15). The resume half then
//!   shows the loop still turns ticks into a new activity.
//! - Tests 3-4's End-on-close and test 5's drains hold only if the loop has
//!   handled every boundary it was advanced past BEFORE a tick is sent, so
//!   tests 3-5 yield after each `advance`. Otherwise the due boundary and the
//!   tick are ready in the same `tokio::select!` poll, and `select!` polls its
//!   branches in a random order. That makes the tick's window, and so the
//!   `End`'s source, a coin flip. Measured during Wave E2:
//!   test 3 kept its old protocol (no yield after its first `advance`) and its
//!   End-on-close check failed 23 of 40 runs (the idle arm's `End` at the next
//!   boundary), while its full-sequence check passed every time. One
//!   `yield_now` after that `advance` fixes it (next section).
//! - Tests 1 and 2 take the same yield after their first `advance` (since the
//!   Wave E2 follow-up), so the windows their step comments describe are the
//!   ones tests 3 and 4 pin. They still assert the exit only, so for them those
//!   comments are reasoned, not measured.
//! - Every tick send is checked: it must reach exactly one receiver, the
//!   aggregator. A send that reached nobody would be a tick the loop never saw.
//!
//! ## What the yield rests on: tokio's scheduling order, not a guarantee
//!
//! Reasoned from the source of tokio 1.52.3 (the version `Cargo.lock` pins). No
//! documented tokio guarantee says this:
//!
//! 1. `advance` only moves the paused clock, then yields once. The timers it
//!    made due fire at the runtime's next park, and that wakes the loop task.
//! 2. After every park, the current_thread `block_on` loop polls the TEST's
//!    future before any queued task. So when `advance` returns, the due boundary
//!    is still unhandled.
//! 3. `yield_now` defers the test's wake to the next park. The loop task, already
//!    queued, therefore runs first. It handles every due boundary in that one poll:
//!    tokio's default `Burst` interval fires missed boundaries back to back. So one
//!    yield is enough, after 100 ms or 500 ms alike.
//!
//! So tests 3-5's determinism rests on that scheduling order. Wall-clock load
//! cannot reorder it: the clock is paused and the runtime is current_thread, so
//! nothing depends on how fast the machine is. A tokio bump can reorder it. That
//! would surface as a flake at the End-on-close assert (tests 3-4) or at test
//! 5's drains, not as a false pass.
//!
//! On any tokio upgrade, re-run each of the five tests 200 times against the
//! test binary cargo builds. Each run is a fresh process, so `select!` gets a
//! fresh random seed. The loop below runs in bash from any shell (zsh included),
//! from the repository root. It needs `jq`, and it exits non-zero unless all
//! 1 000 runs pass. A run counts only if it reports `1 passed`, so a renamed test
//! (0 tests run) fails the loop rather than passing it. On a Mac whose default
//! SDK cannot link, export `SDKROOT` first; cargo inherits it.
//!
//! ```text
//! bash <<'LOOP'
//! set -u
//! BIN=$(cargo test -p agent --test activity_audit_aggregator_select_arm_survival --no-run --message-format=json \
//!   | jq -r 'select(.reason == "compiler-artifact" and .target.name == "activity_audit_aggregator_select_arm_survival") | .executable // empty')
//! [ -x "$BIN" ] || { echo "no test binary (got '$BIN')"; exit 2; }
//! bad=0
//! for t in recv_arm_increments_after_interval_fires recv_arm_survives_n_interval_boundaries \
//!          recv_arm_increments_counter recv_arm_counts_every_tick_after_idle_windows \
//!          idle_window_ends_the_activity_before_the_close; do
//!   pass=0
//!   for i in $(seq 200); do
//!     "$BIN" --exact "$t" 2>&1 | grep -q '^test result: ok. 1 passed' && pass=$((pass + 1))
//!   done
//!   echo "$t: $pass of 200 passed"
//!   [ "$pass" -eq 200 ] || bad=1
//! done
//! exit "$bad"
//! LOOP
//! ```
//!
//! ## T-T4 falsification probe table (story 2-63 AC4)
//!
//! Each probe is ONE edit to `pub async fn run_aggregator_loop` in
//! `crates/agent/src/activity_audit_aggregator.rs`, located by symbol + the
//! quoted line text (line numbers rot). Each quoted line occurs exactly once in
//! that file; apply the edit to that occurrence. Procedure:
//!
//! 1. snapshot the file (`cp -p <file> <snap>`) and apply the edit;
//! 2. run `cargo test -p agent --test activity_audit_aggregator_select_arm_survival`.
//!    The named test(s) go RED;
//! 3. restore from the snapshot, THEN `touch <file>`. A restore that keeps the
//!    snapshot's older mtime (`cp -p`) leaves cargo's fingerprint "fresh": the
//!    next run compiles nothing and re-executes the MUTANT binary;
//! 4. re-run. The GREEN counts only if that run's output shows
//!    `Compiling agent` and the file's SHA-256 equals the pre-probe one.
//!
//! The five tests are `recv_arm_increments_after_interval_fires` (test 1),
//! `recv_arm_survives_n_interval_boundaries` (test 2),
//! `recv_arm_increments_counter` (test 3),
//! `recv_arm_counts_every_tick_after_idle_windows` (test 4) and
//! `idle_window_ends_the_activity_before_the_close` (test 5). "Exit expect 1"
//! is the timeout's `expect` ("aggregator did not exit within 500 ms after
//! bus.close()"). "Exit expect 2" is the `JoinResult`'s ("aggregator loop
//! panicked instead of exiting cleanly on Closed"). Test 5's "drain 1" and
//! "drain 2" are its asserts on the `Start` window and the idle window; its
//! "`is_finished` assert" follows drain 2; its "resume pair" is drain 3 (the
//! resume window) and drain 4 (after the exit), checked together by
//! `assert_one_activity_ended_by_close`. Drains 1-2 and the `is_finished`
//! assert run before its close, so a probe that breaks them reddens test 5
//! there, ahead of its exit expects. On tests 1-4, a MEASURED label cites the
//! falsifier's pass 2 (2026-11-14) unless it names another pass; the P-B2,
//! E2-P5 and E2-A1 cells on tests 1-4 were measured by its pass 1, before
//! tests 1-2 took their first-`advance` yields. On test 5, the drain 1-2 cells
//! are MEASURED by the falsifier's pass 3 (2026-11-15): pass 4 left everything
//! in test 5 up to and including drain 2's two asserts byte-identical, so they
//! hold. The cells pass 4 moved (E2-P20, E2-P4, and E2-P5 over the resume
//! half) say who measured them, or REASONED.
//!
//! | # | symbol | quoted line today | mutation | expected RED |
//! |---|---|---|---|---|
//! | P-B1 | `run_aggregator_loop`, recv arm | `recv_result = agg.rx.recv() => {` | replace the whole arm, from this line to its closing brace, with `_ = futures::future::pending::<()>() => {}` | tests 1-4, at exit expect 1: the loop never sees `Closed` (MEASURED, falsifier passes 2 and 3). Test 5 at drain 1 (`left: []`): no tick is ever counted, so no `Start` (MEASURED, falsifier pass 3, 2026-11-15) |
//! | P-B2 | `run_aggregator_loop`, interval arm (negative control) | `_ = agg.interval.tick() => {` | empty the arm's body: `_ = agg.interval.tick() => {}` | tests 3 and 4, at their full-sequence assert (`left: []`, no `Start`); test 5 at drain 1 (`left: []`; MEASURED, falsifier pass 3, 2026-11-15). Tests 1 and 2 stay GREEN: the survival tests do not depend on the interval arm (MEASURED, falsifier pass 3, 2026-11-15) |
//! | E2-P0 | `run_aggregator_loop`, `Ok(_tick)` arm (the wave plan's designated probe) | `agg.counter.fetch_add(1, Ordering::Relaxed);` | delete the statement | tests 3 and 4, at their full-sequence assert (`left: []`, no `Start`); test 5 at drain 1 (`left: []`; MEASURED, falsifier pass 3, 2026-11-15). Tests 1 and 2 stay GREEN. Not unique: ui `cockpit_audit_aggregator_boot` test 1 goes red too (MEASURED, falsifier pass 2: "timeout: no activity event within 1 s of first AuditTick"), and the agent storm tests also see no `Start` (reasoned) |
//! | E2-P4 | `run_aggregator_loop`, `RecvError::Closed` arm | `// Drop the handle to emit End{Success} and exit.` | insert `if agg.counter.load(Ordering::Relaxed) < u32::MAX { panic!("E2-P4 probe: aggregator panics on shutdown"); }` between this line and the `break;` below it | all five tests, at exit expect 2 (tests 1-4 MEASURED, falsifier passes 2 and 3). Test 5 too (MEASURED, pass-4 fixer, 2026-11-15): its drains 1-2 and its `is_finished` assert pass first, the panic comes at its close, and its resume pair's check, which follows the exit expects, never runs. Before Wave E2, tests 1-3 were GREEN under it (tests 4-5 are new): `is_ok()` on the timeout layer (tests 1, 2) and `let _ =` (test 3) accept a panicked task |
//! | E2-P5 | `run_aggregator_loop`, interval arm's window snapshot | `let n = agg.counter.swap(0, Ordering::Relaxed);` | append `let n = if n > 1 { n - 1 } else { n };` | test 4 only, at its full-sequence assert: `label: "Audit: 2 writes"` where 3 ticks were sent (MEASURED, falsifier pass 3, 2026-11-15). Tests 3 and 5 (1 tick per window) stay GREEN, which is why the pair exists: MEASURED for test 3 and for test 5 as pass 3 had it (falsifier pass 3); test 5's resume half, also 1 tick, is REASONED GREEN |
//! | E2-A1 | `run_aggregator_loop`, `(n, None)` arm | `agg.handle = Some(h);` | replace with `drop(h);` | tests 3 and 4, at their End-on-close assert ("before bus.close() the channel holds the Start ALONE"); their full-sequence assert stays GREEN under it, which is why the pre-close drain exists. Test 5 at drain 1, which holds `[Start, End(Success)]`: the `End` arrives in the `Start`'s own window (MEASURED, falsifier pass 3, 2026-11-15) |
//! | E2-P15 | `run_aggregator_loop`, `(n, None)` arm, the `Start` label | `let label = Aggregator::format_label(n);` | replace `n` with `n + 1` | tests 3 and 4, at their full-sequence assert, `label: "Audit: 2 writes"` for 1 tick and `"Audit: 4 writes"` for 3 (MEASURED, falsifier pass 2); test 5 at drain 1, `label: "Audit: 2 writes"` (MEASURED, falsifier pass 3, 2026-11-15). Tests 1 and 2 stay GREEN (MEASURED, falsifier passes 2 and 3, 2026-11-14 and 2026-11-15). The boot file keeps its own row for ui `cockpit_audit_aggregator_boot` |
//! | E2-P16 | `run_aggregator_loop`, `(n, None)` arm, after the handle is stored | `agg.handle = Some(h);` | append `agg.counter.fetch_add(n, Ordering::Relaxed);` after it: the first window is counted again at the next boundary | test 5 only, at drain 2 (`left: []`; MEASURED, falsifier pass 3, 2026-11-15). Reasoned: the next boundary sees n = 1 with a handle, so the `(n, Some(h))` arm calls `ActivityHandle::tick`, whose throttle reads `std::time::Instant` (`crates/agent/src/activity.rs`), which the paused clock does not move and the real clock has barely moved since the `Start`; so the tick is throttled, the count is put back, and no `End` comes. (Only a real-time stall of 100 ms or more between the `Start` and that boundary would let a `Tick { current: 1 }` through: drain 2 is red either way.) This is the deterministic red. Tests 1-4 stay GREEN (MEASURED, falsifier passes 2 and 3, 2026-11-14 and 2026-11-15; in pass 2 as its OWN-3). The real-time sibling agent `activity_audit_aggregator::aggregator_idle_drops_handle` ("exactly 1 End{Success}", left 0, right 1) is red in 3 of 21 runs under this edit (MEASURED, falsifier pass 3, 2026-11-15: GREEN in the main run, red in 3 of 20 repeats; pass 2 saw it red in its one run): timing-dependent. REASONED cause: `TICK_THROTTLE` and `AGGREGATION_WINDOW` are both 100 ms, and the interval's first tick is immediate, so which boundary lets the re-counted window through is jitter. ui `cockpit_audit_aggregator_boot` tests 1-2 and agent `activity_audit_aggregator_invariants` stay GREEN (MEASURED, falsifier passes 2 and 3, 2026-11-14 and 2026-11-15) |
//! | E2-P17 | `run_aggregator_loop`, `(0, Some(_))` arm, the idle end | `agg.handle = None; // Drop impl emits End { Success }` | replace with `// E2-P17 probe: the idle arm keeps its handle` | test 5 only, at drain 2 (`left: []`): the idle window emits nothing, and the `End` comes only from the close (MEASURED, falsifier pass 3, 2026-11-15). Tests 1-4 stay GREEN: none of them reaches an idle boundary while a handle is held (MEASURED, falsifier pass 3, 2026-11-15). Siblings, MEASURED (falsifier pass 3, 2026-11-15): agent `activity_audit_aggregator`'s `aggregator_idle_drops_handle` ("exactly 1 End{Success}", left 0, right 1) and `aggregator_handle_resumes_after_idle` ("burst2 must use a different ActivityId (idle-end fired between bursts)") red; its `aggregator_emits_one_tick_per_window`, `activity_audit_aggregator_invariants`, ui `cockpit_audit_aggregator_boot` tests 1-2 and ui `activity_tape_audit_ledger_event_storm` GREEN |
//! | E2-P20 | `run_aggregator_loop`, `(0, Some(_))` arm, the idle end (the same line as E2-P17) | `agg.handle = None; // Drop impl emits End { Success }` | replace with `break; // E2-P20 probe: the idle arm exits the loop, so the post-loop drop emits the End` | test 5 only, at its `is_finished` assert ("after the idle window the aggregator is still running: the idle arm ends the activity, not the loop"). Drains 1-2 pass first, and tests 1-4 stay GREEN (that red text, the passing drains and the GREENs MEASURED, pass-4 fixer, 2026-11-15). REASONED: the `End` drain 2 sees comes from the exit's handle drop, and tests 1-4 never reach an idle boundary while a handle is held. Before pass 4 it survived every test here, test 5 included (MEASURED, falsifier pass 3, 2026-11-15, as its OWN-C). Siblings: the real-time agent `activity_audit_aggregator::aggregator_idle_drops_handle` ("aggregator must still be running after idle-end") and `aggregator_handle_resumes_after_idle` ("burst2 must produce at least 1 event") red, `aggregator_emits_one_tick_per_window` GREEN (MEASURED, falsifier pass 3 as OWN-C, and again by the pass-4 fixer, 2026-11-15) |
//!
//! `biased;` with the interval arm listed first is NOT a probe of this file.
//! Reasoned: tokio's default `Burst` interval fires its missed boundaries back
//! to back and then returns `Pending`, so the recv arm is still polled.
//!
//! ## Run
//!
//! ```text
//! cargo test -p agent --test activity_audit_aggregator_select_arm_survival
//! ```

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use tokio::sync::broadcast;
use tokio::sync::broadcast::error::TryRecvError;

use agent::activity::{ActivityEvent, ActivityKind, ActivityOutcome, ActivityPhase};
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
    ///
    /// Checked: the send must reach exactly one receiver, the aggregator's. The
    /// constructor drops the channel's first receiver, so any other count means
    /// the test is not measuring the loop it spawned.
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
        let receivers = self
            .tx
            .send(tick)
            .expect("an audit tick must reach the aggregator's receiver, not an empty bus");
        assert_eq!(
            receivers, 1,
            "exactly one receiver (the aggregator) must be subscribed to the mock tick bus"
        );
    }

    /// Close the bus so the aggregator loop exits cleanly (recv arm sees `Closed`).
    fn close(self) {
        drop(self.tx);
    }
}

// ── Activity-channel observation ──────────────────────────────────────────────

/// One activity event, reduced to the fields these tests pin, in a form
/// `assert_eq!` can compare. The wire types (`trading_core::activity`) do not
/// derive `PartialEq`. The `ActivityId` is compared separately: it comes from
/// a process-wide counter, so a test cannot know its value in advance.
#[derive(Debug, PartialEq, Eq)]
enum Seen {
    Start {
        kind: ActivityKind,
        label: String,
        total_units: Option<u64>,
    },
    Tick {
        current: u64,
    },
    End(Ended),
}

/// `ActivityOutcome`, comparable.
#[derive(Debug, PartialEq, Eq)]
enum Ended {
    Success,
    Failed(String),
    Cancelled,
}

fn seen(ev: &ActivityEvent) -> Seen {
    match &ev.phase {
        ActivityPhase::Start { total_units } => Seen::Start {
            kind: ev.kind,
            label: ev.label.clone(),
            total_units: *total_units,
        },
        ActivityPhase::Tick { current, .. } => Seen::Tick { current: *current },
        ActivityPhase::End(ActivityOutcome::Success) => Seen::End(Ended::Success),
        ActivityPhase::End(ActivityOutcome::Failed(reason)) => {
            Seen::End(Ended::Failed(reason.clone()))
        }
        ActivityPhase::End(ActivityOutcome::Cancelled) => Seen::End(Ended::Cancelled),
    }
}

/// The `Start` an aggregator window that counted `ticks` must emit. The label
/// is derived from the ticks the test sent, never read off the subject.
fn expected_start(ticks: u32) -> Seen {
    Seen::Start {
        kind: ActivityKind::AuditLedgerWrite,
        label: format!("Audit: {ticks} writes"),
        total_units: None,
    }
}

/// Every event published so far, in order. The channel is still open (the
/// test holds `event_bus`), so the drain ends at `Empty`. `Lagged` (events
/// lost) or `Closed` FAIL instead of ending the drain early.
fn drain_activity(rx: &mut broadcast::Receiver<ActivityEvent>) -> Vec<ActivityEvent> {
    let mut events = Vec::new();
    loop {
        match rx.try_recv() {
            Ok(ev) => events.push(ev),
            Err(TryRecvError::Empty) => return events,
            Err(other) => panic!(
                "activity channel drain hit {other:?}: an event was lost or the bus closed under the test"
            ),
        }
    }
}

/// Pins ONE complete audit activity, started by a window that counted
/// `ticks_sent` ticks and ended by the close.
///
/// `before_close` is drained after the last window boundary, just before
/// `bus.close()`; `after_exit` is drained once the loop has exited. The checks
/// run in this order, so a probe's red text names the one it hit:
///
/// 1. full sequence: `before_close` then `after_exit` is exactly
///    `[Start(AuditLedgerWrite, "Audit: N writes"), End(Success)]`;
/// 2. the `End` carries the `Start`'s `ActivityId`;
/// 3. End-on-close: `before_close` is exactly `[Start(..)]`, so the `End`
///    came from the close, not from the window that started the activity.
fn assert_one_activity_ended_by_close(
    before_close: &[ActivityEvent],
    after_exit: &[ActivityEvent],
    ticks_sent: u32,
    what: &str,
) {
    let label = format!("Audit: {ticks_sent} writes");
    let all: Vec<&ActivityEvent> = before_close.iter().chain(after_exit).collect();
    let observed: Vec<Seen> = all.iter().copied().map(seen).collect();
    assert_eq!(
        observed,
        vec![expected_start(ticks_sent), Seen::End(Ended::Success)],
        "{what} ⇒ exactly [Start({label:?}), End(Success)]"
    );
    assert_eq!(
        all[0].id, all[1].id,
        "{what}: the End must close the Start's activity (one ActivityId)"
    );
    let observed_before: Vec<Seen> = before_close.iter().map(seen).collect();
    assert_eq!(
        observed_before,
        vec![expected_start(ticks_sent)],
        "{what}: before bus.close() the channel holds the Start ALONE, so the End is the close's"
    );
}

// ── Test 1: recv_arm_increments_after_interval_fires ─────────────────────────

/// T-D-B2 test 1 — advance 100 ms (interval fires once) then send a tick;
/// the aggregator must exit cleanly when the bus is closed.
///
/// ## Protocol
///
/// 1. `start_paused = true` — take control of all tokio timers.
/// 2. Spawn `run_aggregator_loop(agg)` as a background task.
/// 3. `advance(100 ms)`, then yield — the loop handles the first interval
///    boundary (counter=0 → idle, no activity event) before the tick exists.
/// 4. `tx.send(tick)` — tick enters the broadcast channel.
/// 5. Yield — let task process the recv arm (counter := 1).
/// 6. `advance(100 ms)` — fires the second interval; counter=1 → emits `Start`.
/// 7. Yield.
/// 8. `bus.close()` — recv arm sees `Closed` → loop exits.
/// 9. Expect the exit twice (PRIMARY assertion — decoupled from interval arm
///    behaviour): within 500 ms, and without a panic.
///
/// The yield in step 3 is test 3's (story 2-63 Wave E2 follow-up), so the
/// windows and counter values in steps 3-6 are the ones test 3 pins. This
/// test asserts the EXIT only: nothing here checks a window, a counter or an
/// activity event, so the step comments are reasoned, not measured.
///
/// **P-B1 falsification**: replace recv arm with `pending` →
/// `Closed` never arrives → loop hangs → timeout fires → this test FAILs.
///
/// **E2-P4 falsification**: panic on `Closed` → the task's `JoinResult` is
/// `Err(JoinError)` → the second `expect` FAILs.
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
    // Let the loop handle that boundary BEFORE the tick exists (test 3's yield;
    // see the module's observation strategy).
    tokio::task::yield_now().await;

    // Step 2: send one tick.
    bus.send_tick();

    // Yield to let the spawned task process the recv arm (counter := 1).
    tokio::task::yield_now().await;

    // Step 3: advance past the second interval boundary (counter=1 → Start).
    tokio::time::advance(Duration::from_millis(100)).await;
    tokio::task::yield_now().await;

    // Step 4: close the bus → recv arm sees Closed → loop exits.
    bus.close();

    // PRIMARY assertion: the loop exits within 500 ms, and cleanly.
    // P-B1 mutation causes the first expect to FAIL with Elapsed.
    // E2-P4 mutation (panic on Closed) causes the second expect to FAIL.
    // P-B2 mutation (interval no-op) does NOT affect either — PASS.
    tokio::time::timeout(Duration::from_millis(500), loop_handle)
        .await
        .expect(
            "aggregator did not exit within 500 ms after bus.close() — \
             recv arm must process RecvError::Closed to break the loop. \
             P-B1 falsification: replace recv arm with pending → this FAILs.",
        )
        .expect("aggregator loop panicked instead of exiting cleanly on Closed");
}

// ── Test 2: recv_arm_survives_n_interval_boundaries ──────────────────────────

/// T-D-B2 test 2 — advance 500 ms (5 interval fires) then send ticks;
/// the recv arm must still be alive and process the subsequent `Closed` signal.
///
/// ## Protocol
///
/// 1. `start_paused = true`.
/// 2. Spawn `run_aggregator_loop(agg)`.
/// 3. `advance(500 ms)`, then yield — the loop handles all 5 interval
///    boundaries (counter=0 → idle) before the ticks exist.
/// 4. Send 3 ticks — recv arm must still be listening after 5 idle boundaries.
/// 5. Yield — let task process the recv arm (counter := 3).
/// 6. `advance(100 ms)` — 6th interval fires; counter=3 → emits `Start`.
/// 7. Yield.
/// 8. `bus.close()` → recv arm sees `Closed` → loop exits.
/// 9. Expect the exit twice (PRIMARY assertion): within 500 ms, and without a
///    panic.
///
/// The yield in step 3 is the one test 1 and test 3 take after their first
/// `advance` (story 2-63 Wave E2 follow-up). Test 4 runs this protocol with
/// two yields at step 3 and two at step 5 and pins the window it describes;
/// one yield at each is enough, reasoned in the module's
/// 'What the yield rests on' section.
/// This test asserts the EXIT only: nothing here checks a window, a counter or
/// an activity event, so the step comments are reasoned, not measured.
///
/// **P-B1 falsification**: replace recv arm with `pending` →
/// loop never exits → timeout → FAIL.
///
/// **E2-P4 falsification**: panic on `Closed` → the second `expect` FAILs.
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
    // Let the loop handle those boundaries BEFORE the ticks exist (test 3's
    // yield; see the module's observation strategy).
    tokio::task::yield_now().await;

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

    // PRIMARY assertion: the loop exits within 500 ms, and cleanly.
    // P-B1 mutation (recv arm = pending) causes the first expect to FAIL.
    // E2-P4 mutation (panic on Closed) causes the second expect to FAIL.
    // P-B2 mutation (interval no-op) does NOT affect either — PASS.
    tokio::time::timeout(Duration::from_millis(500), loop_handle)
        .await
        .expect(
            "aggregator did not exit within 500 ms after bus.close() — \
             recv arm must survive N interval boundaries and process RecvError::Closed. \
             P-B1 falsification: replace recv arm with pending → this FAILs.",
        )
        .expect("aggregator loop panicked instead of exiting cleanly on Closed");
}

// ── Test 3: recv_arm_increments_counter ──────────────────────────────────────

/// T-D-B2 test 3 — one tick between two interval boundaries reaches the
/// activity channel as exactly `[Start("Audit: 1 writes"), End(Success)]`,
/// and the `End` is emitted by the close.
///
/// This is the 1-tick half of the window-count pair with
/// `recv_arm_counts_every_tick_after_idle_windows` (3 ticks). Before Wave E2 it
/// discarded the loop's exit (`let _ =` on the timeout) and asserted only
/// `start_count >= 1`. Wave E2 also added the `yield_now` after its first
/// `advance` (see the module's observation strategy). Without that yield, the
/// tick's window was decided by `select!`'s random branch order.
///
/// ## Falsification (rows in the module's T-T4 table)
///
/// - E2-P0 (delete the recv arm's `fetch_add`): the counter stays 0, no
///   `Start` is emitted, and the full-sequence assert FAILs (`left: []`).
/// - P-B2 (interval arm no-op): the count is never drained into a `Start`; the
///   full-sequence assert FAILs. P-B2 is the negative control for the
///   SURVIVAL tests (1 + 2) only, not for this test.
/// - P-B1 and E2-P4: the exit expects FAIL (hang; panic).
/// - E2-A1 (handle dropped at start): the End-on-close assert FAILs.
/// - E2-P5 (count off by one for n > 1) leaves this test GREEN: n = 1.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn recv_arm_increments_counter() {
    const TICKS: u32 = 1;

    let bus = MockAuditTickBus::new();
    let event_bus = EventBus::new(&BusConfig::default());
    let mut activity_rx = event_bus.activity().subscribe();

    let rx = bus.subscribe();
    let agg = Aggregator::new(rx, event_bus.activity());

    let loop_handle = tokio::spawn(run_aggregator_loop(agg));

    // Advance past the first interval boundary (counter=0 → idle, no Start yet).
    tokio::time::advance(Duration::from_millis(100)).await;
    // Let the loop handle that boundary BEFORE the tick exists. Without this
    // yield, the boundary and the tick are ready in one `select!` poll, its
    // random branch order decides the tick's window, and the idle arm can emit
    // the End at the next boundary, before the close (measured: 23 of 40 runs).
    tokio::task::yield_now().await;

    // Send one tick — recv arm fires, counter := 1.
    for _ in 0..TICKS {
        bus.send_tick();
    }
    tokio::task::yield_now().await;

    // Advance past the second interval boundary — counter=1 → emits Start.
    tokio::time::advance(Duration::from_millis(100)).await;
    tokio::task::yield_now().await;

    // What the window emitted, BEFORE the close (the End-on-close check).
    let before_close = drain_activity(&mut activity_rx);

    // Close, then EXPECT a clean exit: the drained events are complete only
    // once the loop has exited.
    bus.close();
    tokio::time::timeout(Duration::from_millis(500), loop_handle)
        .await
        .expect(
            "aggregator did not exit within 500 ms after bus.close() — \
             the activity it started is ended only by the loop's exit",
        )
        .expect("aggregator loop panicked instead of exiting cleanly on Closed");

    let after_exit = drain_activity(&mut activity_rx);
    assert_one_activity_ended_by_close(
        &before_close,
        &after_exit,
        TICKS,
        "one tick between two interval boundaries, closed before the next",
    );
}

// ── Test 4: recv_arm_counts_every_tick_after_idle_windows ────────────────────

/// T-D-B2 test 4 (story 2-63 Wave E2) — after five idle windows, three ticks
/// in one window reach the activity channel as exactly
/// `[Start("Audit: 3 writes"), End(Success)]`, and the `End` is emitted by
/// the close.
///
/// Test 2's protocol, which pinned only the exit, plus the event assert and
/// explicit yields. The yields after `advance(500 ms)` let the idle burst
/// drain before the sends, and the yields after the sends let all three be
/// counted before the next boundary, so the three cannot split across windows.
///
/// The 3-tick half of the window-count pair with `recv_arm_increments_counter`
/// (1 tick). It is the only test in this crate that pins a window's count
/// through the loop for n > 1.
///
/// ## Falsification (rows in the module's T-T4 table)
///
/// - E2-P5 (report `n - 1` for n > 1): the label reads `"Audit: 2 writes"` and
///   the full-sequence assert FAILs. Unique to this test: test 3 has n = 1.
/// - E2-P0 and P-B2: no `Start`; the full-sequence assert FAILs (`left: []`).
/// - P-B1 and E2-P4: the exit expects FAIL (hang; panic).
/// - E2-A1 (handle dropped at start): the End-on-close assert FAILs.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn recv_arm_counts_every_tick_after_idle_windows() {
    const TICKS: u32 = 3;

    let bus = MockAuditTickBus::new();
    let event_bus = EventBus::new(&BusConfig::default());
    let mut activity_rx = event_bus.activity().subscribe();

    let rx = bus.subscribe();
    let agg = Aggregator::new(rx, event_bus.activity());

    let loop_handle = tokio::spawn(run_aggregator_loop(agg));

    // Five idle windows: every boundary sees counter=0 and no handle.
    tokio::time::advance(Duration::from_millis(500)).await;
    tokio::task::yield_now().await;
    tokio::task::yield_now().await;

    // Three ticks inside one window — the recv arm must count every one.
    for _ in 0..TICKS {
        bus.send_tick();
    }
    tokio::task::yield_now().await;
    tokio::task::yield_now().await;

    // The next boundary drains counter=3 into one Start.
    tokio::time::advance(Duration::from_millis(100)).await;
    tokio::task::yield_now().await;

    // What the window emitted, BEFORE the close (the End-on-close check).
    let before_close = drain_activity(&mut activity_rx);

    bus.close();
    tokio::time::timeout(Duration::from_millis(500), loop_handle)
        .await
        .expect(
            "aggregator did not exit within 500 ms after bus.close() — \
             the activity it started is ended only by the loop's exit",
        )
        .expect("aggregator loop panicked instead of exiting cleanly on Closed");

    let after_exit = drain_activity(&mut activity_rx);
    assert_one_activity_ended_by_close(
        &before_close,
        &after_exit,
        TICKS,
        "three ticks after five idle windows",
    );
}

// ── Test 5: idle_window_ends_the_activity_before_the_close ───────────────────

/// T-D-B2 test 5 (story 2-63 Wave E2, passes 3-4) — the idle-arm half: one
/// tick, then ONE more interval boundary with no tick sent. That idle boundary
/// must end the activity, as exactly `[End(Success)]` under the `Start`'s
/// `ActivityId`, while the loop keeps running; one more tick then starts a
/// fresh activity, which the close ends.
///
/// Test 3's protocol plus one idle boundary, then test 3's protocol again
/// (the resume half, pass 4). Tests 3 and 4 close right after the window that
/// emits the `Start`, so the next boundary is outside what they observe (see
/// the module's observation strategy, which also gives the pass-2 measurement
/// that made this test necessary).
///
/// ## Drains and asserts, in order
///
/// 1. drain 1, after the boundary that counts the tick: exactly
///    `[Start(AuditLedgerWrite, "Audit: 1 writes")]`;
/// 2. drain 2, after the next boundary, with no tick sent: exactly
///    `[End(Success)]`, with the `Start`'s `ActivityId`. With nothing sent,
///    that `End` comes from the `(0, Some(_))` idle arm OR from the loop's
///    exit: a loop that returned at that boundary drops its handle on the way
///    out and sends the same `End`. Drain 2 alone cannot tell them apart
///    (MEASURED, falsifier pass 3, 2026-11-15, its OWN-C: the idle arm turned
///    into a `break`, and this test, then ending at an empty drain after the
///    exit, stayed GREEN);
/// 3. the `is_finished` assert: the loop is still running, so the `End` was
///    the idle arm's (REASONED: the idle arm and the exit's handle drop are
///    the loop's only two `End` sources);
/// 4. the resume half: one more tick and one boundary, then drain 3 (the
///    resume window, before the close), the close, both exit expects, and
///    drain 4 (after the exit). `assert_one_activity_ended_by_close` (test 3's
///    helper) pins drains 3-4 as ONE fresh activity, `[Start("Audit: 1
///    writes"), End(Success)]` under one `ActivityId`, with the `Start` alone
///    before the close. So the loop still turns ticks into a new activity after
///    the idle `End`, and nothing else follows that `End`. The two activities'
///    ids are not compared: every `Start` takes a fresh id from a process-wide
///    counter (REASONED: `ActivitySender::start` calls `ActivityId::next()`),
///    so that check could not fail.
///
/// Everything up to and including drain 2's two asserts is byte-identical to
/// pass 3, so pass 3's measurements of drains 1-2 hold for this version.
///
/// ## Falsification (rows in the module's T-T4 table)
///
/// - E2-P16 (the first window is counted again after its handle is stored)
///   and E2-P17 (the idle arm keeps its handle): drain 2 FAILs, `left: []`.
///   Both leave tests 1-4 GREEN, so this test is their only red here
///   (MEASURED, falsifier pass 3, 2026-11-15).
/// - E2-P20 (the idle arm exits the loop): drains 1-2 pass, and the
///   `is_finished` assert FAILs. Tests 1-4 stay GREEN (MEASURED, pass-4 fixer,
///   2026-11-15).
/// - P-B1, P-B2 and E2-P0: drain 1 FAILs, `left: []` (no `Start`). Under P-B1
///   that comes before the exit expects, because drain 1 is taken before the
///   close (MEASURED, falsifier pass 3, 2026-11-15).
/// - E2-A1 (handle dropped at start): drain 1 FAILs, holding
///   `[Start(..), End(Success)]` (MEASURED, falsifier pass 3, 2026-11-15).
/// - E2-P15 (the `Start` label counts n + 1): drain 1 FAILs at the label,
///   `"Audit: 2 writes"` (MEASURED, falsifier pass 3, 2026-11-15).
/// - E2-P4 (panic on `Closed`): exit expect 2 FAILs; drains 1-2 and the
///   `is_finished` assert pass first, and the resume pair's check, after the
///   exit expects, never runs (MEASURED, pass-4 fixer, 2026-11-15).
/// - E2-P5 (count off by one for n > 1) leaves this test GREEN: n = 1 in both
///   halves (MEASURED GREEN by falsifier pass 3 on the pass-3 test, which ended
///   at its close; the resume half also counts 1 tick, REASONED).
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn idle_window_ends_the_activity_before_the_close() {
    const TICKS: u32 = 1;

    let bus = MockAuditTickBus::new();
    let event_bus = EventBus::new(&BusConfig::default());
    let mut activity_rx = event_bus.activity().subscribe();

    let rx = bus.subscribe();
    let agg = Aggregator::new(rx, event_bus.activity());

    let loop_handle = tokio::spawn(run_aggregator_loop(agg));

    // First boundary: counter=0, no handle → idle, nothing emitted. Yield so
    // the loop handles it BEFORE the tick exists (test 3's yield).
    tokio::time::advance(Duration::from_millis(100)).await;
    tokio::task::yield_now().await;

    // One tick — recv arm fires, counter := 1.
    for _ in 0..TICKS {
        bus.send_tick();
    }
    tokio::task::yield_now().await;

    // Second boundary: counter=1, no handle → Start.
    tokio::time::advance(Duration::from_millis(100)).await;
    tokio::task::yield_now().await;

    // Drain 1: the Start window.
    let start_window = drain_activity(&mut activity_rx);
    let observed_start: Vec<Seen> = start_window.iter().map(seen).collect();
    assert_eq!(
        observed_start,
        vec![expected_start(TICKS)],
        "drain 1: the window that counted {TICKS} tick emits exactly its Start"
    );

    // Third boundary, no tick sent: counter=0 with a handle → the idle arm
    // drops the handle, whose Drop emits End(Success).
    tokio::time::advance(Duration::from_millis(100)).await;
    tokio::task::yield_now().await;

    // Drain 2: the idle window.
    let idle_window = drain_activity(&mut activity_rx);
    let observed_idle: Vec<Seen> = idle_window.iter().map(seen).collect();
    assert_eq!(
        observed_idle,
        vec![Seen::End(Ended::Success)],
        "drain 2: the next window, with no tick sent, ends the activity (the idle arm) before the close"
    );
    assert_eq!(
        idle_window[0].id, start_window[0].id,
        "drain 2: the idle window's End closes the Start's activity (one ActivityId)"
    );

    // With nothing sent, drain 2's End could also come from a loop that
    // RETURNED at that boundary: its exit drops the handle, which sends the
    // same End (T-T4 E2-P20). The bus is still open, so a loop that still
    // runs has not finished.
    assert!(
        !loop_handle.is_finished(),
        "after the idle window the aggregator is still running: the idle arm ends the activity, not the loop"
    );

    // Resume: the same ticks again, inside the next window — recv arm fires,
    // counter := 1.
    for _ in 0..TICKS {
        bus.send_tick();
    }
    tokio::task::yield_now().await;

    // Fourth boundary: counter=1, no handle → a fresh Start.
    tokio::time::advance(Duration::from_millis(100)).await;
    tokio::task::yield_now().await;

    // Drain 3: the resume window, BEFORE the close.
    let resume_window = drain_activity(&mut activity_rx);

    bus.close();
    tokio::time::timeout(Duration::from_millis(500), loop_handle)
        .await
        .expect(
            "aggregator did not exit within 500 ms after bus.close() — \
             recv arm must process RecvError::Closed to break the loop",
        )
        .expect("aggregator loop panicked instead of exiting cleanly on Closed");

    // Drain 4: after the exit. The resume window and the close make ONE
    // activity, ended by the close; nothing else follows the idle End.
    let after_exit = drain_activity(&mut activity_rx);
    assert_one_activity_ended_by_close(
        &resume_window,
        &after_exit,
        TICKS,
        "after the idle End, one more tick between two interval boundaries, closed before the next",
    );
}
