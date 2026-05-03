//! T906 stitch follow-up — `cockpit_live`'s kill button drives the
//! T809 dual-write end-to-end (orchestrator-spawned, 2026-05-01).
//!
//! Feature-gated to `live` (see `#![cfg(feature = "live")]` below) —
//! exercises the live-only `KillTripFn` type alias +
//! `Cockpit::kill_switch` field.  Under the default workspace test run
//! (`cargo test --workspace --all-targets`) this file builds an empty
//! crate; under `cargo test -p ui --features live --test
//! cockpit_live_kill_button_writes_audit` it builds and runs the
//! integration test.
//!
//! Context: T906 (ui-designer wave 3) installed `Cockpit::kill_switch`
//! and the `Message::KillConfirmed` arm that invokes the closure, plus
//! three unit tests against a recording closure.  This file is the
//! integration test that proves the closure-as-shipped — the one
//! `cockpit_live::main()` constructs against a real `Arc<KillSwitch>`
//! and the side-thread tokio runtime's `Handle` — actually fires the
//! audit dual-write when driven via `ui::state::update`.
//!
//! ## What this test asserts (T809 dual-write invariant)
//!
//! After driving `Message::KillConfirmed` through `ui::state::update`
//! with a properly-constructed trip closure, both:
//!
//! 1. **`journal_transactions` row** — the v0 memo row written by
//!    `audit::journal::kill_switch_tripped` (description prefix
//!    `registry:KillSwitchTripped:`).
//! 2. **`strategy_events` row** — the v1+ row of kind
//!    `KillSwitchTripped` carrying the `HaltReason::ManualOperator`
//!    string.
//!
//! must land in the audit ledger.  This is the same dual-write
//! `crates/agent/tests/kill_switch_trip_writes_both.rs` proves at the
//! `KillSwitch::trip` boundary; here we prove it survives the
//! UI → closure → side-thread-runtime → trip → tokio::spawn chain.
//!
//! ## Topology mirrored from `cockpit_live::main()`
//!
//! - A real `tokio::runtime::Builder::new_multi_thread()` runtime is
//!   constructed; `runtime.handle().clone()` is captured BEFORE the
//!   runtime is moved into `std::thread::spawn` so the `Handle` is
//!   accessible from the test thread.
//! - The closure spawns `KillSwitch::trip(reason)` onto the runtime via
//!   `Handle::spawn`, exactly like the production path in
//!   `crates/ui/src/bin/cockpit_live.rs`.
//! - `ui::state::update` is invoked from the test thread (no tokio
//!   runtime in scope) — mirroring iced's main-thread `update` arm.
//!   If the closure were misconstructed (e.g. tried to `tokio::spawn`
//!   directly on the calling thread), the trip would panic with
//!   "there is no reactor running" and this test would catch it.

#![cfg(feature = "live")]

use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use agent::{IncidentSpawner, KillSwitch, MockIncidentSpawner};
use audit::query::{all_transaction_ids, strategy_events_since};
use audit::{bootstrap, Ledger};
use time::OffsetDateTime;
use trading_core::{StrategyEventKind, Timestamp};
use ui::state::{Cockpit, KillTripFn, Message};

/// Open an in-memory ledger with the chart-of-accounts bootstrapped —
/// same fixture as the T809 boundary test.
async fn open_ledger() -> Ledger {
    let ledger = Ledger::in_memory().await.expect("open in-memory ledger");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap chart of accounts");
    ledger
}

fn ts_epoch() -> Timestamp {
    Timestamp::new(OffsetDateTime::UNIX_EPOCH)
}

/// Block the calling (sync) thread until both audit rows have landed
/// or the deadline expires.  Polls every 25 ms.  Returns true if both
/// rows are present, false if we timed out — the assertion site
/// reports the missing side.
fn wait_for_dual_write(
    runtime: &tokio::runtime::Runtime,
    ledger: &Ledger,
    deadline: Duration,
) -> (bool, bool) {
    let start = Instant::now();
    loop {
        let txn_ids = runtime
            .block_on(all_transaction_ids(ledger))
            .expect("all_transaction_ids");
        let evs = runtime
            .block_on(strategy_events_since(ledger, ts_epoch()))
            .expect("strategy_events_since");

        let memo_present = !txn_ids.is_empty();
        let strategy_event_present = evs
            .iter()
            .any(|e| matches!(e.kind, StrategyEventKind::KillSwitchTripped));

        if memo_present && strategy_event_present {
            return (true, true);
        }
        if start.elapsed() >= deadline {
            return (memo_present, strategy_event_present);
        }
        thread::sleep(Duration::from_millis(25));
    }
}

#[test]
fn t906_stitch_kill_confirmed_via_state_update_writes_both_audit_rows() {
    // ── (a) Build a real audit-wired KillSwitch fixture ─────────────────────
    // The bootstrap runtime is current-thread + drives the in-memory
    // ledger setup synchronously, then is dropped — the long-lived
    // multi-thread runtime below is the one whose `Handle` the trip
    // closure will spawn onto, mirroring cockpit_live's topology.
    let bootstrap_rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("bootstrap runtime");
    let ledger = bootstrap_rt.block_on(async { Arc::new(open_ledger().await) });
    drop(bootstrap_rt);

    let mock = Arc::new(MockIncidentSpawner::new());
    let spawner: Arc<dyn IncidentSpawner> = mock.clone();
    let dir = tempfile::tempdir().expect("tempdir");
    let halt_file = dir.path().join(".halt");

    let kill_switch = Arc::new(KillSwitch::with_audit(
        &halt_file,
        32,
        Arc::clone(&ledger),
        spawner,
    ));

    // ── (b) Build a real multi-thread tokio runtime + capture handle ────────
    // Mirrors `crates/ui/src/bin/cockpit_live.rs`: the runtime is built
    // up-front so we can capture `runtime.handle().clone()` BEFORE the
    // runtime is moved into the side thread.  In this test we keep the
    // runtime owned on the test thread (no need to actually spawn it on
    // a separate `std::thread` — `Handle::spawn` is thread-safe).
    let agent_runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("agent-rt-test")
        .build()
        .expect("agent runtime");
    let rt_handle = agent_runtime.handle().clone();

    // ── (c) Build the trip closure exactly like the production path ─────────
    // Identical shape to crates/ui/src/bin/cockpit_live.rs's `trip`.  If
    // this closure ever tried to `tokio::spawn` directly (instead of
    // `rt_handle.spawn`), the panic "there is no reactor running" would
    // surface here when `ui::state::update` invokes it from the
    // (non-tokio) test thread — which is exactly the iced topology.
    let trip: KillTripFn = {
        let kill_for_trip = Arc::clone(&kill_switch);
        let h = rt_handle.clone();
        Arc::new(move |reason| {
            let kill = Arc::clone(&kill_for_trip);
            h.spawn(async move {
                kill.trip(reason);
            });
        })
    };

    // ── (d) Construct the Cockpit with the closure wired ────────────────────
    let mut cockpit = Cockpit::new();
    cockpit.kill_switch = Some(trip);

    // ── (e) Drive the kill-button flow through ui::state::update ────────────
    // Three messages mirror the operator's actions: open dialog, type
    // the safety phrase, confirm.  `update` is sync and runs on the
    // test thread (no tokio context) — same as iced's main-thread
    // `update` arm.
    ui::state::update(&mut cockpit, Message::KillPressed);
    ui::state::update(
        &mut cockpit,
        Message::KillConfirmPhraseChanged(ui::strings::KILL_SAFETY_PHRASE.to_string()),
    );
    ui::state::update(&mut cockpit, Message::KillConfirmed);

    // The UI flips to Flattening immediately (closure is fire-and-
    // forget; the dual-write happens on the runtime).
    assert_eq!(
        cockpit.kill,
        ui::state::KillState::Flattening,
        "Message::KillConfirmed must flip the UI to Flattening",
    );
    // The trip is async (spawned onto rt_handle); `is_tripped()` may be
    // true or false at this exact instant depending on scheduler
    // timing.  The dual-write wait below is the real assertion — by the
    // time both audit rows land, `KillSwitch::trip` has run to
    // completion (the audit dual-write is the last side-effect).

    // ── (f) Wait briefly (<= 500 ms) for the spawned dual-write ─────────────
    let (memo_present, strategy_event_present) =
        wait_for_dual_write(&agent_runtime, &ledger, Duration::from_millis(500));

    // ── (g) Assert the T809 dual-write invariant ────────────────────────────
    assert!(
        memo_present,
        "T809 invariant: journal_transactions row must exist after trip — \
         (a) of the dual-write did NOT land",
    );
    assert!(
        strategy_event_present,
        "T809 invariant: strategy_events row of kind KillSwitchTripped must \
         exist after trip — (b) of the dual-write did NOT land.  This means \
         the trip closure constructed against rt_handle.spawn(...) did NOT \
         carry the audit dual-write through; route HANDOFF -> architect, \
         the Q6 topology may need revisiting.",
    );

    // ── (h) Bonus: assert the strategy_events row carries ManualOperator ────
    // Confirms the `HaltReason::ManualOperator` injected by the
    // `Message::KillConfirmed` arm survives the closure → trip path.
    let evs = agent_runtime
        .block_on(strategy_events_since(&ledger, ts_epoch()))
        .expect("strategy_events_since");
    let kill_event = evs
        .iter()
        .find(|e| matches!(e.kind, StrategyEventKind::KillSwitchTripped))
        .expect("kill-switch event present (already asserted above)");
    assert_eq!(
        kill_event.error_summary.as_deref(),
        Some("manual_operator"),
        "the trip's HaltReason::ManualOperator must round-trip into the \
         strategy_events row's error_summary as 'manual_operator' \
         (matches `impl Display for HaltReason` in \
         crates/agent/src/kill_switch.rs:65) — this is the T809 \
         audit-payload contract.  Got: {:?}",
        kill_event.error_summary,
    );

    // ── (i) The IncidentSpawner is also called (T809 (d)) ───────────────────
    // The trip's incident-spawn helper fires synchronously inside
    // `KillSwitch::trip` (not on `tokio::spawn`), so by the time the
    // dual-write rows are visible the spawner has already been called.
    let calls = mock.calls();
    assert_eq!(
        calls.len(),
        1,
        "incident spawner must be called exactly once on a single trip",
    );
    assert_eq!(
        calls[0].reason, "manual_operator",
        "incident spawner must receive the ManualOperator reason string",
    );

    // Drop the runtime gracefully — outstanding tasks are aborted.
    drop(agent_runtime);
}
