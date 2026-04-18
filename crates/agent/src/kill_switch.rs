//! `KillSwitch` — halt-file watcher + heartbeat monitor (T28).
//!
//! Manages the agent's halt state:
//! - **Sticky halt:** once tripped, only recovers via manual removal of `.halt` file + restart.
//! - **Triggers:** `.halt` file detected, heartbeat timeout, ledger imbalance, clock skew.
//! - **On halt:** broadcasts `AgentMode::Halted` on the event bus.
//!
//! R7.3: once halted, the agent does NOT auto-resume. The operator must remove
//! the `.halt` file and restart.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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

/// The kill switch.
///
/// Thread-safe (wraps an `AtomicBool`). Call `.trip(reason)` from any async
/// task; call `.is_tripped()` to check before processing.
#[derive(Clone)]
pub struct KillSwitch {
    tripped: Arc<AtomicBool>,
    halt_file: PathBuf,
    mode_tx: broadcast::Sender<AgentMode>,
}

impl KillSwitch {
    /// Create a new kill switch.
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
        }
    }

    /// Subscribe to `AgentMode` events.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<AgentMode> {
        self.mode_tx.subscribe()
    }

    /// Trip the kill switch with a reason.
    ///
    /// Idempotent after the first trip.
    pub fn trip(&self, reason: HaltReason) {
        let already = self.tripped.swap(true, Ordering::SeqCst);
        if !already {
            let msg = reason.to_string();
            warn!(reason = %msg, "KillSwitch tripped");
            // Broadcast mode change — ignore lagged-receiver errors.
            let _ = self.mode_tx.send(AgentMode::Halted { reason: msg });
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
