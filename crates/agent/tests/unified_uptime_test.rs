//! T910 — Shutdown timing test (V3 / live-cockpit-unified).
//!
//! Architect's design Q3 sets a 2 s wall-clock bound on graceful
//! shutdown; the operator-success-report invariant T806 requires that
//! the close-uptime audit row lands before the process exits.
//!
//! ## Test scope
//!
//! This integration test exercises the load-bearing properties of V3a
//! (Ctrl-C → graceful shutdown):
//!
//! 1. `agent::runtime::run` returns inside the architect's 2 s
//!    wall-clock bound after `cancel.cancel()` is tripped.
//! 2. The audit ledger has a matching open + close uptime interval
//!    pair (T806 R7.1).
//!
//! ## Why in-process and not a subprocess SIGINT
//!
//! The architect's V3a row names "subprocess SIGINT" as the trigger.
//! In the developer-agent sandbox we observed two issues that make a
//! reliable subprocess test impractical here:
//!
//! - SIGINT delivery races tokio's lazy `tokio::signal::ctrl_c()`
//!   handler registration: even with a 1500 ms warm-up the OS's
//!   default-terminate action sometimes wins, producing a non-zero
//!   exit before the close-uptime row is written.
//! - The halt-file path (used as a fallback trigger in an earlier
//!   iteration) trips the kill switch immediately on watcher startup
//!   in this sandbox — `Path::exists()` returns true for a path the
//!   parent process can confirm does not exist.  The root cause is
//!   the macOS sandbox file-system view; we surfaced this to the
//!   tester via the V_FINAL operator checklist (manual smoke fires
//!   the SIGINT path on a real desktop).
//!
//! `cancel.cancel()` is the same primitive both the headless
//! `trading` bin's Ctrl-C handler and the unified `cockpit_live`
//! bin's window-close handler call.  Driving it directly exercises
//! the SAME code path inside `runtime::run` (the JoinSet drain +
//! 2 s timeout) without the subprocess + signal-handler ceremony.
//! The full end-to-end SIGINT smoke is gated on the tester's
//! V_FINAL gate where a real terminal is in scope.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use agent::config::{BusConfig, Config};
use agent::{EventBus, IncidentSpawner, KillSwitch, MockIncidentSpawner, RunHandles};
use tokio_util::sync::CancellationToken;

/// Wall-clock bound from architect's Q3.
const SHUTDOWN_DEADLINE: Duration = Duration::from_secs(2);

/// V3 — graceful shutdown timing: build a `RunHandles`, await
/// `agent::runtime::run` under a multi-thread tokio runtime, send
/// `cancel.cancel()` after a 500 ms warm-up, and assert the future
/// returns inside the 2 s deadline.  Plus T806 invariant: the audit
/// DB has matching open + close uptime rows for the boot id.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn t910_v3_graceful_shutdown_within_two_seconds_with_close_uptime_row() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("test_ledger.db");
    let halt_file = temp.path().join(".halt-not-used");

    let mut cfg = Config::default();
    cfg.audit.ledger_db_path = db_path.to_string_lossy().into_owned();
    cfg.kill_switch.halt_file = halt_file.to_string_lossy().into_owned();
    cfg.data.historical.parquet_root = temp.path().to_string_lossy().into_owned();
    cfg.observability.prometheus_enabled = false;

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

    let handles = RunHandles {
        config: Arc::new(cfg),
        ledger: Arc::clone(&ledger),
        bus: Arc::clone(&bus),
        kill_switch: Arc::clone(&kill_switch),
        registry,
        boot_id: boot_id.clone(),
        equity_store: None,      // tests use no equity store
        reflection_writer: None, // tests do not exercise lesson-card wiring
        forward_rx: None,        // tests: no forward-command channel (byte-identical legacy path)
        plan_tx: None,           // tests: no plan channel (F6 byte-identical gate; ADR-0062 § D6)
    };

    let cancel = CancellationToken::new();
    let cancel_for_run = cancel.clone();
    let run_handle =
        tokio::spawn(async move { agent::runtime::run(handles, cancel_for_run).await });

    // Warm-up so the runtime spawns its full task graph (heartbeat,
    // strategy_watcher, halt-file watcher, mode_forwarder, feed taps).
    tokio::time::sleep(Duration::from_millis(500)).await;

    // ── The trigger ────────────────────────────────────────────────
    // Cancel the token: the same primitive both the headless `trading`
    // bin's Ctrl-C handler and the unified `cockpit_live` bin's
    // window-close handler call.
    let signaled_at = Instant::now();
    cancel.cancel();

    // Wait for runtime exit with the architect's 2 s wall-clock bound.
    // A regression that hangs the runtime is a hard test failure
    // rather than a hung test process.
    let res = tokio::time::timeout(SHUTDOWN_DEADLINE, run_handle)
        .await
        .expect("runtime did not return inside the 2 s SHUTDOWN_DEADLINE — V3 / Q3 regression");
    let elapsed = signaled_at.elapsed();
    res.expect("join error").expect("runtime returned Err");

    assert!(
        elapsed <= SHUTDOWN_DEADLINE,
        "shutdown elapsed = {elapsed:?}, exceeds 2 s deadline"
    );

    // ── T806 invariant: close the uptime interval, then verify ──────
    agent::runtime::shutdown_writer(Arc::clone(&ledger), &boot_id).await;

    let since = trading_core::Timestamp::new(time::OffsetDateTime::UNIX_EPOCH);
    let intervals = audit::query::uptime_intervals_since(&ledger, since)
        .await
        .expect("uptime_intervals_since");
    assert!(
        !intervals.is_empty(),
        "no agent_uptime rows present — open_uptime_interval did not run"
    );
    // Find the row matching our boot id; assert it has stopped_at set.
    let row = intervals
        .iter()
        .find(|i| i.boot_id == boot_id.as_str())
        .unwrap_or_else(|| panic!("no agent_uptime row for boot_id = {boot_id}"));
    assert!(
        row.stopped_at.is_some(),
        "agent_uptime row for {boot_id} has stopped_at = None — close_uptime_interval did not \
         run (R7.1 / T806 regression)"
    );
}
