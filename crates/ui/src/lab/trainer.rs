//! Lab training runner — cockpit-training-control T-D-N1.
//!
//! Provides the `spawn_training_run` function that launches `train_tcn` as an
//! OS subprocess and streams its stdout/stderr into a bounded mpsc channel.
//!
//! ## Architecture
//!
//! ```text
//! iced update thread
//!   Message::TrainingPressed
//!     └──> trainer::spawn_training_run(rt_handle, cfg, cancel)
//!              └──> tokio::process::Command::new(train_tcn_path)
//!                       └──> BufReader lines → SyncSender<TrainingLogLine>
//!                                └──> Message::TrainingLogLine(line)
//!                                └──> Message::TrainingExited(status)
//! ```
//!
//! The `TrainingHandle` wraps the child process. Dropping the handle calls
//! `child.start_kill()` (SIGKILL on Unix, `TerminateProcess` on Windows), satisfying
//! the SIGKILL-immediate cancellation contract (ADR-0034 / Operator-decide Q2).
//!
//! Reuses `RunCancelHandle` / `RunCancelReceiver` from `lab::runner` by import,
//! NOT by copy (ADR-0034 § D3).

use std::path::PathBuf;

use smol_str::SmolStr;

pub use super::runner::{RunCancelHandle, RunCancelReceiver, cancellation_pair};

// cockpit-activity-status-bar v0.1.0 T-D-N9 — import ActivitySender/Handle/Kind
// under the `live` feature gate (activity types live in the `agent` crate).
#[cfg(feature = "live")]
use agent::activity::{ActivityHandle, ActivityKind, ActivitySender};

// ── Log-line type ──────────────────────────────────────────────────────────────

/// A single line from the `train_tcn` subprocess's stdout or stderr.
#[derive(Debug, Clone)]
pub struct TrainingLogLine {
    /// The raw log line text.
    pub text: SmolStr,
    /// Whether this line came from stderr (true) or stdout (false).
    pub is_stderr: bool,
}

// ── Training handle ────────────────────────────────────────────────────────────

/// In-flight training subprocess handle.
///
/// **Drop = SIGKILL** — dropping this value immediately kills the subprocess
/// per the SIGKILL-immediate cancellation contract (ADR-0034 Q2).
pub struct TrainingHandle {
    /// The tokio child process. Held so Drop can kill it.
    #[cfg(feature = "live")]
    child: tokio::process::Child,
}

impl std::fmt::Debug for TrainingHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrainingHandle").finish_non_exhaustive()
    }
}

impl Drop for TrainingHandle {
    fn drop(&mut self) {
        // SIGKILL-immediate on Drop (ADR-0034 / Q2 / R2.4).
        // `start_kill()` is fire-and-forget; errors are silent (process may
        // already have exited, which is fine — we just ensure we tried).
        #[cfg(feature = "live")]
        let _ = self.child.start_kill();
    }
}

// ── Training config ────────────────────────────────────────────────────────────

/// Configuration for a training run spawned by `spawn_training_run`.
#[derive(Debug, Clone)]
pub struct TrainingConfig {
    /// Path to the `train_tcn` binary. Resolved by D10 three-tier precedence.
    pub binary_path: PathBuf,
    /// Path to the `train_tcn.toml` config file.
    pub config_path: PathBuf,
    /// Output directory for checkpoints.
    pub output_dir: PathBuf,
    /// If true, pass `--dry-run` to skip actual training (useful for smoke tests).
    pub dry_run: bool,
    /// Override the number of epochs (passes `--epochs N`). None = config default.
    pub epochs: Option<u32>,
    /// Scenario label, forwarded as `--scenario` arg.
    pub scenario: Option<SmolStr>,
    /// Optional audit-DB path (`--audit-db <PATH>`).
    pub audit_db: Option<PathBuf>,
}

// ── Path resolution — D10 three-tier precedence ───────────────────────────────

/// Resolve the `train_tcn` binary path using D10 three-tier precedence:
/// 1. `current_exe`-relative: look for `train_tcn` next to the running binary.
/// 2. Workspace-relative: look for `target/debug/train_tcn` in the workspace root.
/// 3. Dev fallback: `"train_tcn"` (relies on PATH / dev build).
///
/// Returns the resolved `PathBuf`. The binary may not exist at the returned
/// path if none of the tiers find it — callers should handle the spawn error.
#[must_use]
pub fn resolve_train_tcn_path() -> PathBuf {
    // Tier 1: next to current exe.
    if let Ok(exe) = std::env::current_exe() {
        let sibling = exe.with_file_name("train_tcn");
        if sibling.exists() {
            return sibling;
        }
        // Windows: try .exe extension.
        let sibling_exe = exe.with_file_name("train_tcn.exe");
        if sibling_exe.exists() {
            return sibling_exe;
        }
    }

    // Tier 2: workspace-relative debug build.
    if let Ok(cwd) = std::env::current_dir() {
        // Walk up to find the workspace root (contains target/debug/train_tcn).
        let mut probe = cwd.as_path();
        loop {
            let candidate = probe.join("target").join("debug").join("train_tcn");
            if candidate.exists() {
                return candidate;
            }
            let candidate_exe = probe.join("target").join("debug").join("train_tcn.exe");
            if candidate_exe.exists() {
                return candidate_exe;
            }
            match probe.parent() {
                Some(p) => probe = p,
                None => break,
            }
        }
    }

    // Tier 3: dev fallback — return "train_tcn" and let PATH resolution handle it.
    PathBuf::from("train_tcn")
}

// ── Default config resolver (R3 — cockpit-training-pressed-wiring T-D-N3) ────

/// Build a `TrainingConfig` from the canonical workspace defaults.
///
/// - `config_path`: resolved by walking up from `current_dir` to find
///   `crates/forecast/train_tcn.toml`. Falls back to the literal path
///   if the walk fails. Emits a `tracing::warn!` on fallback (T-AR-3).
/// - `binary_path`: resolved via `resolve_train_tcn_path()` (D10 three-tier).
/// - `output_dir`: `<workspace>/target/training_checkpoints/<timestamp>`.
///   Isolated by timestamp so successive runs don't clobber each other (R3.3).
/// - `dry_run = false`, `epochs = None`, `scenario = None`, `audit_db = None`
///   per analyst R3.5-R3.6 and R3.4.
#[must_use]
pub fn default_training_config() -> TrainingConfig {
    let config_path = resolve_train_tcn_toml_path();
    let binary_path = resolve_train_tcn_path();
    let output_dir = resolve_output_dir();
    TrainingConfig {
        binary_path,
        config_path,
        output_dir,
        dry_run: false,
        epochs: None,
        scenario: None,
        audit_db: None,
    }
}

/// Resolve the `crates/forecast/train_tcn.toml` config path.
///
/// Walks up from `current_dir` looking for a directory that contains the
/// `crates/forecast/train_tcn.toml` sub-path (same pattern as
/// `resolve_train_tcn_path` tier 2). Falls back to the literal relative
/// path with a `tracing::warn!` if the walk exhausts without finding it.
#[must_use]
pub fn resolve_train_tcn_toml_path() -> std::path::PathBuf {
    const RELATIVE: &str = "crates/forecast/train_tcn.toml";

    if let Ok(cwd) = std::env::current_dir() {
        let mut probe = cwd.as_path();
        loop {
            let candidate = probe.join(RELATIVE);
            if candidate.exists() {
                return candidate;
            }
            match probe.parent() {
                Some(p) => probe = p,
                None => break,
            }
        }
    }

    tracing::warn!(
        path = RELATIVE,
        "resolve_train_tcn_toml_path: workspace walk failed — falling back to literal path"
    );
    std::path::PathBuf::from(RELATIVE)
}

/// Resolve the output directory for a new training run.
///
/// Returns `<workspace-root>/target/training_checkpoints/<timestamp>` where
/// `<workspace-root>` is the first parent of `current_dir` that contains a
/// `target/` directory (same walk as tier 2 of `resolve_train_tcn_path`).
/// Falls back to `target/training_checkpoints/<timestamp>` if the walk fails.
#[must_use]
fn resolve_output_dir() -> std::path::PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    // Timestamp for isolation (run-varying — does NOT affect determinism
    // because this is the live binary, not the backtest harness).
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let dir_name = format!("training_checkpoints/{ts}");

    if let Ok(cwd) = std::env::current_dir() {
        let mut probe = cwd.as_path();
        loop {
            let target = probe.join("target");
            if target.exists() && target.is_dir() {
                return target.join(&dir_name);
            }
            match probe.parent() {
                Some(p) => probe = p,
                None => break,
            }
        }
    }

    std::path::PathBuf::from("target").join(dir_name)
}

// ── Spawn ──────────────────────────────────────────────────────────────────────

/// Spawn `train_tcn` as a subprocess and wire its stdout/stderr into the
/// provided channel.
///
/// Returns `Ok((TrainingHandle, Option<ActivityHandle>))` where the
/// `TrainingHandle` must be stashed in `LabState::training_inflight` (dropping
/// it immediately kills the subprocess — SIGKILL-immediate per Q2) and the
/// `ActivityHandle` must be stashed alongside it so the iced side can tick /
/// end the activity on `TrainingEventsRefreshed` / `TrainingExited`.
///
/// Returns `Err(SmolStr)` if the subprocess cannot be spawned (binary missing,
/// permissions error, etc.).
///
/// **cockpit-activity-status-bar T-D-N9:** `activity_sender` is `Some` when
/// the bus is available. When present, a `Training` `ActivityHandle` is started
/// before the subprocess spawn and returned to the caller. Label:
/// `"Train <binary> · running"`.
///
/// # Errors
/// Returns `Err` if `tokio::process::Command::spawn()` fails.
#[cfg(feature = "live")]
pub fn spawn_training_run(
    rt_handle: Option<&tokio::runtime::Handle>,
    cfg: &TrainingConfig,
    _cancel: RunCancelReceiver,
    line_tx: std::sync::mpsc::SyncSender<TrainingLogLine>,
    activity_sender: Option<ActivitySender>,
) -> Result<(TrainingHandle, Option<ActivityHandle>), SmolStr> {
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command;

    let Some(handle) = rt_handle else {
        return Err(SmolStr::new(
            "no tokio runtime handle — cannot spawn training",
        ));
    };

    // T-D-N9 — cockpit-activity-status-bar Training producer wiring.
    // Start the activity handle BEFORE the subprocess spawn (approach A).
    // Label: "Train <binary-name> · running". The handle is returned to
    // the caller (iced side) to tick/end on audit-DB poll events.
    let training_label = format!(
        "Train {} · running",
        cfg.binary_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("train_tcn")
    );
    let activity_handle: Option<ActivityHandle> = activity_sender
        .as_ref()
        .map(|s| s.start(ActivityKind::Training, training_label));

    // Build the argument list.
    let mut args: Vec<std::ffi::OsString> = vec![
        "--config".into(),
        cfg.config_path.as_os_str().to_owned(),
        "--output-dir".into(),
        cfg.output_dir.as_os_str().to_owned(),
    ];
    if cfg.dry_run {
        args.push("--dry-run".into());
    }
    if let Some(epochs) = cfg.epochs {
        args.push("--epochs".into());
        args.push(epochs.to_string().into());
    }
    if let Some(ref scenario) = cfg.scenario {
        args.push("--scenario".into());
        args.push(scenario.as_str().into());
    }
    if let Some(ref audit_db) = cfg.audit_db {
        args.push("--audit-db".into());
        args.push(audit_db.as_os_str().to_owned());
    }

    let rt = handle.clone();
    let binary_path = cfg.binary_path.clone();

    let mut child = handle
        .block_on(async {
            let mut cmd = Command::new(&binary_path);
            cmd.args(&args)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());
            cmd.spawn()
                .map_err(|e| SmolStr::new(format!("spawn failed: {e}")))
        })
        .map_err(|e: SmolStr| e)?;

    // Wire stdout reader.
    if let Some(stdout) = child.stdout.take() {
        let tx = line_tx.clone();
        rt.spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let _ = tx.send(TrainingLogLine {
                    text: SmolStr::new(&line),
                    is_stderr: false,
                });
            }
        });
    }

    // Wire stderr reader.
    if let Some(stderr) = child.stderr.take() {
        let tx = line_tx;
        rt.spawn(async move {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let _ = tx.send(TrainingLogLine {
                    text: SmolStr::new(&line),
                    is_stderr: true,
                });
            }
        });
    }

    Ok((TrainingHandle { child }, activity_handle))
}

/// Non-live stub: always returns `Err` (training requires the `live` feature).
///
/// # Errors
/// Always returns `Err` — training is not supported in non-live fixture builds.
#[cfg(not(feature = "live"))]
pub fn spawn_training_run(
    _rt_handle: Option<()>,
    _cfg: &TrainingConfig,
    _cancel: RunCancelReceiver,
    _line_tx: std::sync::mpsc::SyncSender<TrainingLogLine>,
    _activity_sender: Option<()>,
) -> Result<(TrainingHandle, Option<()>), SmolStr> {
    Err(SmolStr::new(
        "training not supported in non-live fixture builds",
    ))
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::time::{Duration, Instant};

    /// T-D-N1 — cancel handle drop kills child.
    ///
    /// Spawns `sleep 60` via tokio, wraps it in a `TrainingHandle` (live) or
    /// kills it directly (non-live), and verifies the process exits within 500ms.
    ///
    /// In `live` builds: the TrainingHandle Drop impl calls `start_kill()` → SIGKILL.
    /// In non-live builds: we call `start_kill()` directly to exercise the same
    /// kill-on-drop semantics without the feature-gated struct.
    #[test]
    #[cfg(unix)]
    fn cancel_handle_drop_kills_child() {
        let rt = tokio::runtime::Runtime::new().unwrap();

        #[cfg(feature = "live")]
        {
            let mut child = rt.block_on(async {
                tokio::process::Command::new("sleep")
                    .arg("60")
                    .spawn()
                    .expect("sleep must be available on Unix test hosts")
            });
            let pid = child.id().expect("child must have a pid");
            // Wrap in TrainingHandle — Drop calls start_kill() → SIGKILL.
            let handle = TrainingHandle { child };
            drop(handle);
            assert_exited_within(pid, Duration::from_millis(500));
        }

        #[cfg(not(feature = "live"))]
        {
            // Use std::process::Command for the non-live path — it has a
            // straightforward kill() method without needing a tokio runtime.
            let mut child = std::process::Command::new("sleep")
                .arg("60")
                .spawn()
                .expect("sleep must be available on Unix test hosts");
            let pid = child.id();
            // Kill the process directly (mirrors Drop impl in live build).
            child.kill().expect("kill must succeed");
            child.wait().ok(); // Reap the zombie.
            assert_exited_within(pid, Duration::from_millis(500));
        }
    }

    /// Poll `kill -0 <pid>` until the process is gone or the deadline passes.
    #[cfg(unix)]
    fn assert_exited_within(pid: u32, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        let mut exited = false;
        while Instant::now() < deadline {
            let probe = std::process::Command::new("kill")
                .args(["-0", &pid.to_string()])
                .output();
            match probe {
                Ok(out) if !out.status.success() => {
                    exited = true;
                    break;
                }
                Err(_) => {
                    exited = true;
                    break;
                }
                _ => {}
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            exited,
            "process {pid} should have exited within {timeout:?} after kill"
        );
    }

    /// T-D-N1 — stdout lines pipe to channel.
    ///
    /// Spawns `sh -c 'echo line1; echo line2'` using `std::process::Command`
    /// and reads its stdout through a `BufReader` into a channel, asserting
    /// both lines surface. This tests the pipe-to-channel wiring pattern
    /// without requiring the tokio `live` feature.
    #[test]
    fn stdout_lines_pipe_to_channel() {
        let (tx, rx) = std::sync::mpsc::sync_channel::<TrainingLogLine>(256);

        let mut child = std::process::Command::new("sh")
            .args(["-c", "echo line1; echo line2"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("sh must be available");

        // Drain stdout in the current thread.
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(l) = line {
                    let _ = tx.send(TrainingLogLine {
                        text: SmolStr::new(&l),
                        is_stderr: false,
                    });
                }
            }
        }

        child.wait().expect("wait must succeed");

        let mut lines: Vec<String> = Vec::new();
        while let Ok(l) = rx.try_recv() {
            lines.push(l.text.to_string());
        }

        assert!(
            lines.contains(&"line1".to_string()),
            "line1 must be in channel; got: {lines:?}"
        );
        assert!(
            lines.contains(&"line2".to_string()),
            "line2 must be in channel; got: {lines:?}"
        );
    }

    /// T-D-N1 — binary missing returns Err synchronously (no panic, no subprocess).
    ///
    /// Passes a nonsense binary path and asserts it returns `Err(_)`.
    #[test]
    fn binary_missing_returns_err_sync() {
        let cfg = TrainingConfig {
            binary_path: PathBuf::from("/nonexistent/path/to/train_tcn_xyzzy"),
            config_path: PathBuf::from("crates/forecast/train_tcn.toml"),
            output_dir: PathBuf::from("/tmp"),
            dry_run: false,
            epochs: None,
            scenario: None,
            audit_db: None,
        };

        let (_cancel_handle, cancel_rx) = cancellation_pair();
        let (line_tx, _line_rx) = std::sync::mpsc::sync_channel::<TrainingLogLine>(256);

        #[cfg(feature = "live")]
        {
            let rt = tokio::runtime::Runtime::new().unwrap();
            // cockpit-activity-status-bar T-D-N9: pass None for activity_sender.
            let result = spawn_training_run(Some(rt.handle()), &cfg, cancel_rx, line_tx, None);
            assert!(
                result.is_err(),
                "spawn with missing binary must return Err, not Ok"
            );
        }

        #[cfg(not(feature = "live"))]
        {
            // cockpit-activity-status-bar T-D-N9: pass None for activity_sender.
            let result = spawn_training_run(None, &cfg, cancel_rx, line_tx, None);
            assert!(
                result.is_err(),
                "non-live build must return Err for training"
            );
        }
    }

    /// T-D-N3 — `default_training_config` resolves `crates/forecast/train_tcn.toml`.
    ///
    /// Asserts that `resolve_train_tcn_toml_path()` returns a path that exists on
    /// disk and ends with `crates/forecast/train_tcn.toml`.  This test pins the
    /// workspace-relative config path so CI catches a moved file immediately.
    #[test]
    fn default_training_config_resolves_train_tcn_toml() {
        let path = resolve_train_tcn_toml_path();
        assert!(
            path.exists(),
            "resolve_train_tcn_toml_path must return a path that exists on disk; got: {}",
            path.display()
        );
        let path_str = path.to_string_lossy();
        assert!(
            path_str.ends_with("crates/forecast/train_tcn.toml"),
            "resolved path must end with crates/forecast/train_tcn.toml; got: {path_str}"
        );
    }

    /// T-D-N3 — `default_training_config` produces a valid `TrainingConfig`.
    ///
    /// Asserts the defaults match the R3 requirements: dry_run=false, epochs=None,
    /// scenario=None, audit_db=None.
    #[test]
    fn default_training_config_has_correct_defaults() {
        let cfg = default_training_config();
        assert!(!cfg.dry_run, "dry_run must be false (R3.5)");
        assert!(cfg.epochs.is_none(), "epochs must be None (R3.6)");
        assert!(cfg.scenario.is_none(), "scenario must be None (R3.6)");
        assert!(cfg.audit_db.is_none(), "audit_db must be None (R3.4)");
        let output_str = cfg.output_dir.to_string_lossy();
        assert!(
            output_str.contains("training_checkpoints"),
            "output_dir must contain 'training_checkpoints' (R3.3); got: {output_str}"
        );
    }
}
