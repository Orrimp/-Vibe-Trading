//! `KillSwitch` — halt-file watcher + heartbeat monitor (T28).
//!
//! Manages the agent's halt state:
//! - **Sticky halt:** once tripped, only recovers via manual removal of `.halt` file + restart.
//! - **Triggers:** `.halt` file detected, heartbeat timeout, ledger imbalance, clock skew.
//! - **On halt:** broadcasts `AgentMode::Halted` on the event bus.
//!
//! R7.3: once halted, the agent does NOT auto-resume. The operator must remove
//! the `.halt` file and restart.
//!
//! T809 — operator success reports Q8: on trip the kill switch
//! also (a) dual-writes the audit memo + `strategy_events`
//! `KillSwitchTripped` row via [`audit::journal::kill_switch_tripped`]
//! and (b) spawns the operator-success-report binary out-of-process
//! via the [`IncidentSpawner`] seam (real `Command` in production;
//! mocked in tests).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use thiserror::Error;
use tokio::sync::broadcast;
use tracing::{info, warn};

// ── AgentMode event ───────────────────────────────────────────────────────────

/// Agent operating mode broadcast to the cockpit and other subsystems.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentMode {
    /// Normal operation.
    Running,
    /// Halted — all positions flat, orders cancelled.
    Halted { reason: String },
}

// ── KillSwitch state ──────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum KillSwitchError {
    #[error("notify watcher error: {0}")]
    Notify(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Reasons the kill switch can be tripped.
#[derive(Debug, Clone)]
pub enum HaltReason {
    HaltFile,
    HeartbeatTimeout,
    LedgerImbalance,
    ClockSkew,
    ManualOperator,
    Test,
}

impl std::fmt::Display for HaltReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HaltFile => write!(f, "halt_file"),
            Self::HeartbeatTimeout => write!(f, "heartbeat_timeout"),
            Self::LedgerImbalance => write!(f, "ledger_imbalance"),
            Self::ClockSkew => write!(f, "clock_skew"),
            Self::ManualOperator => write!(f, "manual_operator"),
            Self::Test => write!(f, "test"),
        }
    }
}

/// Arguments handed to an [`IncidentSpawner`] when the kill switch trips
/// (T809 — operator success reports R12.1c).
///
/// The spawner builds and launches the reports-binary command from these
/// fields.  Production wires [`CommandIncidentSpawner`] (a real
/// `std::process::Command` spawn); tests wire [`MockIncidentSpawner`]
/// (records calls, never launches).
#[derive(Debug, Clone)]
pub struct IncidentSpawnArgs {
    /// Halt timestamp formatted as RFC-3339 — used as the `since:<ts>`
    /// argument to `--period` AND substituted into the output filename
    /// `incident-<ts>.md`.
    pub halt_ts_rfc3339: String,
    /// Halt reason (string form of `HaltReason`).  Carried for tracing
    /// only — the report binary itself does not consume this.
    pub reason: String,
}

/// Trait for the side-effect that spawns the operator-success-report
/// binary when the kill switch trips (T809).
///
/// Production: [`CommandIncidentSpawner`] uses
/// `std::process::Command::new("target/release/report")` (debug-build
/// fallback to `target/debug/report`).  Tests: [`MockIncidentSpawner`]
/// records the invocation arguments without actually launching a child.
pub trait IncidentSpawner: Send + Sync {
    /// Spawn the reports binary.  Failure is warn-logged by the caller —
    /// never fatal.  The kill switch must still trip.
    fn spawn(&self, args: &IncidentSpawnArgs);
}

/// Production [`IncidentSpawner`] — launches the reports binary via
/// `std::process::Command`.  Falls back from `target/release/report` to
/// `target/debug/report` if the release build is absent (R-7 mitigation
/// — never invoke `cargo run` from the trip handler).
pub struct CommandIncidentSpawner;

impl IncidentSpawner for CommandIncidentSpawner {
    fn spawn(&self, args: &IncidentSpawnArgs) {
        let release_bin = std::path::Path::new("target/release/report");
        let debug_bin = std::path::Path::new("target/debug/report");
        let bin = if release_bin.exists() {
            release_bin
        } else if debug_bin.exists() {
            debug_bin
        } else {
            warn!(
                ts = %args.halt_ts_rfc3339,
                "report binary not found at target/release/report or \
                 target/debug/report; skipping incident spawn"
            );
            return;
        };
        let period_arg = format!("since:{}", args.halt_ts_rfc3339);
        let output_arg = format!(
            "spec/operator-success-reports/reports/incident-{}.md",
            args.halt_ts_rfc3339
        );
        let res = std::process::Command::new(bin)
            .args(["--period", &period_arg, "--output", &output_arg])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
        match res {
            Ok(child) => {
                info!(
                    bin = %bin.display(),
                    pid = child.id(),
                    ts = %args.halt_ts_rfc3339,
                    "spawned incident report (fire-and-forget)"
                );
            }
            Err(e) => {
                warn!(
                    bin = %bin.display(),
                    error = %e,
                    "incident report spawn failed (non-fatal)"
                );
            }
        }
    }
}

/// Test-only [`IncidentSpawner`] — records the calls without launching
/// a child process.  Use [`MockIncidentSpawner::calls`] to read back
/// the invocation arguments in an integration test.
#[derive(Default, Clone)]
pub struct MockIncidentSpawner {
    calls: Arc<Mutex<Vec<IncidentSpawnArgs>>>,
}

impl MockIncidentSpawner {
    /// Construct a fresh recorder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of recorded calls (cloned).  Use in test assertions.
    #[must_use]
    pub fn calls(&self) -> Vec<IncidentSpawnArgs> {
        self.calls.lock().map(|g| g.clone()).unwrap_or_default()
    }
}

impl IncidentSpawner for MockIncidentSpawner {
    fn spawn(&self, args: &IncidentSpawnArgs) {
        if let Ok(mut g) = self.calls.lock() {
            g.push(args.clone());
        }
    }
}

/// The kill switch.
///
/// Thread-safe (wraps an `AtomicBool`). Call `.trip(reason)` from any async
/// task; call `.is_tripped()` to check before processing.
///
/// T809 — when constructed with [`KillSwitch::with_audit`], a trip:
///   1. broadcasts `AgentMode::Halted` (v0 behavior, unchanged);
///   2. spawns a tokio task that calls
///      [`audit::journal::kill_switch_tripped`] (memo + `strategy_events`
///      dual-write); failure is warn-logged, never fatal;
///   3. invokes the [`IncidentSpawner`] to spawn the reports binary
///      out-of-process (R12.1c).
///
/// When constructed with the v0 [`KillSwitch::new`] constructor, the
/// audit + spawn side-effects are no-ops — preserving existing call
/// sites untouched.
#[derive(Clone)]
pub struct KillSwitch {
    tripped: Arc<AtomicBool>,
    halt_file: PathBuf,
    mode_tx: broadcast::Sender<AgentMode>,
    /// Optional ledger handle — when present, [`KillSwitch::trip`]
    /// dual-writes the audit memo + `strategy_events` row.
    ledger: Option<Arc<audit::Ledger>>,
    /// Optional incident-report spawner — when present, [`KillSwitch::trip`]
    /// invokes it after the audit write.
    spawner: Option<Arc<dyn IncidentSpawner>>,
}

impl KillSwitch {
    /// Create a new kill switch (v0 — no audit, no incident spawn).
    ///
    /// `halt_file` is the path watched by the file watcher.
    /// `mode_capacity` is the broadcast channel capacity.
    #[must_use]
    pub fn new(halt_file: impl Into<PathBuf>, mode_capacity: usize) -> Self {
        let (mode_tx, _) = broadcast::channel(mode_capacity);
        let tripped = Arc::new(AtomicBool::new(false));
        Self {
            tripped,
            halt_file: halt_file.into(),
            mode_tx,
            ledger: None,
            spawner: None,
        }
    }

    /// Create a kill switch wired to the audit ledger and an incident
    /// spawner (T809 — operator success reports Q8 + R12.1c).
    ///
    /// On trip:
    /// 1. broadcasts `AgentMode::Halted` (unchanged);
    /// 2. dual-writes the audit memo + `strategy_events`
    ///    `KillSwitchTripped` row;
    /// 3. invokes `spawner.spawn(...)` to launch the reports binary.
    ///
    /// In production the agent passes `Arc::new(CommandIncidentSpawner)`.
    /// Tests pass a [`MockIncidentSpawner`] to capture the invocation.
    #[must_use]
    pub fn with_audit(
        halt_file: impl Into<PathBuf>,
        mode_capacity: usize,
        ledger: Arc<audit::Ledger>,
        spawner: Arc<dyn IncidentSpawner>,
    ) -> Self {
        let (mode_tx, _) = broadcast::channel(mode_capacity);
        let tripped = Arc::new(AtomicBool::new(false));
        Self {
            tripped,
            halt_file: halt_file.into(),
            mode_tx,
            ledger: Some(ledger),
            spawner: Some(spawner),
        }
    }

    /// Subscribe to `AgentMode` events.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<AgentMode> {
        self.mode_tx.subscribe()
    }

    /// Trip the kill switch with a reason.
    ///
    /// Idempotent after the first trip.  Side-effects in order:
    /// 1. flip the `tripped` atomic (CAS — only the first caller wins);
    /// 2. broadcast `AgentMode::Halted`;
    /// 3. dual-write to the audit ledger (if `with_audit` was used);
    /// 4. spawn the incident reports binary (if `with_audit` was used).
    pub fn trip(&self, reason: HaltReason) {
        let already = self.tripped.swap(true, Ordering::SeqCst);
        if already {
            return;
        }
        let msg = reason.to_string();
        warn!(reason = %msg, "KillSwitch tripped");
        // (1) Broadcast mode change — ignore lagged-receiver errors.
        let _ = self.mode_tx.send(AgentMode::Halted {
            reason: msg.clone(),
        });

        // (2) Dual-write to audit (Q8) — fire-and-forget, warn on error.
        if let Some(ref ledger) = self.ledger {
            let ledger = Arc::clone(ledger);
            let reason_str = msg.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    audit::journal::kill_switch_tripped(&ledger, &reason_str, "kill_switch").await
                {
                    warn!(
                        error = %e,
                        reason = %reason_str,
                        "kill_switch_tripped audit write failed (non-fatal)"
                    );
                }
            });
        }

        // (3) Spawn incident report (R12.1c) — fire-and-forget.
        if let Some(ref spawner) = self.spawner {
            let halt_ts = time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| "unknown".to_string());
            spawner.spawn(&IncidentSpawnArgs {
                halt_ts_rfc3339: halt_ts,
                reason: msg,
            });
        }
    }

    /// Returns `true` if the kill switch has been tripped.
    #[must_use]
    pub fn is_tripped(&self) -> bool {
        self.tripped.load(Ordering::SeqCst)
    }

    /// Check whether the halt file exists and trip if so.
    pub fn check_halt_file(&self) {
        if self.halt_file.exists() {
            self.trip(HaltReason::HaltFile);
        }
    }

    /// Spawn a background task that watches for the `.halt` file using `notify`.
    ///
    /// The task runs until the kill switch is tripped or the tokio runtime shuts
    /// down.  Uses polling (1-second interval) as a portable fallback; production
    /// can upgrade to inotify/kqueue via `notify`'s `RecommendedWatcher`.
    pub fn spawn_halt_file_watcher(self: Arc<Self>) {
        let halt_file = self.halt_file.clone();
        let ks = Arc::clone(&self);
        tokio::spawn(async move {
            // Check immediately on startup
            ks.check_halt_file();
            if ks.is_tripped() {
                return;
            }

            info!(path = ?halt_file, "halt-file watcher started");
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                if halt_file.exists() {
                    ks.trip(HaltReason::HaltFile);
                    return;
                }
                if ks.is_tripped() {
                    return;
                }
            }
        });
    }

    /// Spawn a heartbeat monitor.
    ///
    /// Expects `heartbeat_tx` to be pinged regularly. If no ping arrives
    /// within `timeout_ms`, the kill switch is tripped.
    pub fn spawn_heartbeat_monitor(
        self: Arc<Self>,
        mut heartbeat_rx: tokio::sync::mpsc::Receiver<()>,
        timeout_ms: u64,
    ) {
        let ks = Arc::clone(&self);
        tokio::spawn(async move {
            let timeout = tokio::time::Duration::from_millis(timeout_ms);
            loop {
                match tokio::time::timeout(timeout, heartbeat_rx.recv()).await {
                    Ok(Some(())) => {
                        // heartbeat received — continue
                    }
                    Ok(None) => {
                        // sender dropped — agent shutting down
                        return;
                    }
                    Err(_) => {
                        // timeout
                        warn!("heartbeat timeout — tripping kill switch");
                        ks.trip(HaltReason::HeartbeatTimeout);
                        return;
                    }
                }
                if ks.is_tripped() {
                    return;
                }
            }
        });
    }
}

/// Write the `.halt` file at `path` (for tests and operator tools).
pub fn write_halt_file(path: &Path) -> std::io::Result<()> {
    std::fs::write(path, b"halt\n")
}

/// Remove the `.halt` file at `path` (recovery step).
pub fn remove_halt_file(path: &Path) -> std::io::Result<()> {
    std::fs::remove_file(path)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::NamedTempFile;

    #[test]
    fn t28_new_kill_switch_not_tripped() {
        let ks = KillSwitch::new(".halt", 16);
        assert!(!ks.is_tripped());
    }

    #[test]
    fn t28_trip_is_sticky() {
        let ks = KillSwitch::new(".halt", 16);
        ks.trip(HaltReason::Test);
        assert!(ks.is_tripped());
        // Second trip is no-op
        ks.trip(HaltReason::ManualOperator);
        assert!(ks.is_tripped());
    }

    #[test]
    fn t28_halt_file_triggers_trip() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        // File exists → should trip
        let ks = KillSwitch::new(path, 16);
        ks.check_halt_file();
        assert!(ks.is_tripped());
    }

    #[test]
    fn t28_no_file_no_trip() {
        let ks = KillSwitch::new("/tmp/nonexistent_halt_file_xyz_abc.halt", 16);
        ks.check_halt_file();
        assert!(!ks.is_tripped());
    }

    #[test]
    fn t28_broadcasts_halted_mode() {
        let ks = KillSwitch::new(".halt", 16);
        let mut rx = ks.subscribe();
        ks.trip(HaltReason::ManualOperator);
        let msg = rx.try_recv().unwrap();
        assert!(matches!(msg, AgentMode::Halted { .. }));
    }

    #[tokio::test]
    async fn t28_halt_file_watcher_detects_file() {
        let dir = tempfile::tempdir().unwrap();
        let halt_path = dir.path().join(".halt");

        let ks = Arc::new(KillSwitch::new(&halt_path, 16));
        ks.clone().spawn_halt_file_watcher();

        // Not tripped yet
        assert!(!ks.is_tripped());

        // Drop the halt file
        write_halt_file(&halt_path).unwrap();

        // Wait up to 2s for watcher to detect it
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(2);
        loop {
            if ks.is_tripped() {
                break;
            }
            if tokio::time::Instant::now() >= deadline {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        assert!(
            ks.is_tripped(),
            "kill switch should be tripped after halt file was created"
        );
    }

    #[tokio::test]
    async fn t28_restart_with_halt_file_enters_halted_immediately() {
        let dir = tempfile::tempdir().unwrap();
        let halt_path = dir.path().join(".halt");

        // Pre-create the halt file (simulate restart with file present)
        write_halt_file(&halt_path).unwrap();

        let ks = Arc::new(KillSwitch::new(&halt_path, 16));
        // On check, should be immediately tripped
        ks.check_halt_file();
        assert!(
            ks.is_tripped(),
            "should be tripped immediately on restart with halt file present"
        );
    }
}
