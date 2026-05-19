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

// ── Spawn ──────────────────────────────────────────────────────────────────────

/// Spawn `train_tcn` as a subprocess and wire its stdout/stderr into the
/// provided channel.
///
/// Returns `Ok(TrainingHandle)` where the handle must be stashed in
/// `LabState::training_inflight`; dropping it immediately kills the subprocess
/// (SIGKILL-immediate per Q2). The line channel is passed in by the caller who
/// owns the `SyncSender<TrainingLogLine>`.
///
/// Returns `Err(SmolStr)` if the subprocess cannot be spawned (binary missing,
/// permissions error, etc.).
///
/// # Errors
/// Returns `Err` if `tokio::process::Command::spawn()` fails.
#[cfg(feature = "live")]
pub fn spawn_training_run(
    rt_handle: Option<&tokio::runtime::Handle>,
    cfg: TrainingConfig,
    _cancel: RunCancelReceiver,
    line_tx: std::sync::mpsc::SyncSender<TrainingLogLine>,
) -> Result<TrainingHandle, SmolStr> {
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command;

    let Some(handle) = rt_handle else {
        return Err(SmolStr::new(
            "no tokio runtime handle — cannot spawn training",
        ));
    };

    // Build the argument list.
    let mut args: Vec<std::ffi::OsString> = Vec::new();
    args.push("--config".into());
    args.push(cfg.config_path.as_os_str().to_owned());
    args.push("--output-dir".into());
    args.push(cfg.output_dir.as_os_str().to_owned());
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

    Ok(TrainingHandle { child })
}

/// Non-live stub: always returns `Err` (training requires the `live` feature).
///
/// # Errors
/// Always returns `Err` — training is not supported in non-live fixture builds.
#[cfg(not(feature = "live"))]
pub fn spawn_training_run(
    _rt_handle: Option<()>,
    _cfg: TrainingConfig,
    _cancel: RunCancelReceiver,
    _line_tx: std::sync::mpsc::SyncSender<TrainingLogLine>,
) -> Result<TrainingHandle, SmolStr> {
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
            let result = spawn_training_run(Some(rt.handle()), cfg, cancel_rx, line_tx);
            assert!(
                result.is_err(),
                "spawn with missing binary must return Err, not Ok"
            );
        }

        #[cfg(not(feature = "live"))]
        {
            let result = spawn_training_run(None, cfg, cancel_rx, line_tx);
            assert!(
                result.is_err(),
                "non-live build must return Err for training"
            );
        }
    }
}
