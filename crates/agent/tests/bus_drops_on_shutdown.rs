//! T903d — Bus-drop test (live-cockpit-unified).
//!
//! Risk #6 in the architect's design:
//!
//! > Bus producer wiring (T903a/b/c) lands but a producer holds an
//! > extra `Arc<EventBus>` clone past shutdown, preventing the
//! > broadcast senders from dropping → `RecvError::Closed` never fires
//! > on the cockpit side → cockpit panels show stale data forever
//! > after window close.
//!
//! Mitigation per the design: every spawned task receives a fresh
//! `Arc::clone(&handles.bus)` and drops it on `cancel.cancelled()`; the
//! reference count is bounded by the spawned-task count.  After
//! `agent::runtime::run` returns, only the test harness's outer
//! reference may remain.
//!
//! Acceptance per the task spec: construct `RunHandles` with a real
//! bus, invoke `runtime::run` on a tokio runtime, await ~500 ms, call
//! `cancel.cancel()`, await the future to completion (with a 2 s
//! timeout), assert `Arc::strong_count(&bus) == 1`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;

use agent::config::{BusConfig, Config};
use agent::{EventBus, IncidentSpawner, KillSwitch, MockIncidentSpawner, RunHandles};
use tokio_util::sync::CancellationToken;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn t903d_bus_strong_count_collapses_on_cancel() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("test_ledger.db");
    let halt_file = temp.path().join(".halt");

    // Research-mode config so no Binance reach-out fires.
    let mut cfg = Config::default();
    cfg.audit.ledger_db_path = db_path.to_string_lossy().into_owned();
    cfg.kill_switch.halt_file = halt_file.to_string_lossy().into_owned();
    cfg.data.historical.parquet_root = temp.path().to_string_lossy().into_owned();

    let ledger = Arc::new(
        audit::Ledger::open(&cfg.audit.ledger_db_path)
            .await
            .expect("open ledger"),
    );
    audit::bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("chart");

    let spawner: Arc<dyn IncidentSpawner> = Arc::new(MockIncidentSpawner::new());
    let kill_switch = Arc::new(KillSwitch::with_audit(
        &cfg.kill_switch.halt_file,
        32,
        Arc::clone(&ledger),
        spawner,
    ));

    let bus = Arc::new(EventBus::new(&BusConfig::default()));
    let registry = Arc::new(strategy::StrategyRegistry::new());

    let boot_id = uuid::Uuid::new_v4().to_string();
    audit::journal::open_uptime_interval(&ledger, &boot_id, None)
        .await
        .expect("open_uptime");

    // Hold ONE outer reference on `bus` for the assertion below.  Every
    // other clone the runtime makes (one per spawned task) must drop on
    // `cancel.cancel()` so the strong count returns to 1.
    let bus_outer = Arc::clone(&bus);

    let handles = RunHandles {
        config: Arc::new(cfg),
        ledger: Arc::clone(&ledger),
        bus: Arc::clone(&bus),
        kill_switch: Arc::clone(&kill_switch),
        registry,
        boot_id: boot_id.clone(),
    };

    // Drop the outer `bus` rebinding so `bus_outer` is the only "outside
    // the runtime" handle once `RunHandles.bus` is consumed by `run`.
    drop(bus);

    let cancel = CancellationToken::new();
    let cancel_for_run = cancel.clone();
    let run_handle =
        tokio::spawn(async move { agent::runtime::run(handles, cancel_for_run).await });

    // Let the runtime spin up its full task graph (heartbeat, watcher,
    // halt-file watcher, feed taps, mode forwarder).
    tokio::time::sleep(Duration::from_millis(500)).await;

    cancel.cancel();

    // `run` must return inside 2 s so a regression that leaks an Arc
    // past shutdown is a hard failure rather than a hung test.
    let res = tokio::time::timeout(Duration::from_secs(2), run_handle)
        .await
        .expect("runtime did not return inside 2 s");
    res.expect("join error").expect("runtime returned Err");

    // Close-uptime row matches the headless trading bin's flow.
    agent::runtime::shutdown_writer(Arc::clone(&ledger), &boot_id).await;

    // Give any final task drops a tokio yield.  `JoinSet::join_next`
    // in `run` already drained, but the returned future + the spawn
    // wrappers may take one yield to release.
    tokio::task::yield_now().await;

    // ── The actual T903d invariant ───────────────────────────────────
    // Every Arc clone the spawned tasks held is dropped; only the test
    // harness's outer `bus_outer` remains.  If a producer leaks an
    // extra clone, this assertion fires with the strong_count showing
    // the leak surface.
    let strong = Arc::strong_count(&bus_outer);
    assert_eq!(
        strong, 1,
        "bus Arc strong_count = {strong}; expected 1 (only the test's outer ref).  \
         A spawned task held its `Arc<EventBus>` past shutdown — see Risk #6 in \
         spec/features/live-cockpit-unified.md ## Risks + mitigations."
    );
}
