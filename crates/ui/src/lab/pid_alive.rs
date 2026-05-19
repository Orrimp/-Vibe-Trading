//! Cross-platform PID liveness check (T-D-N14, ADR-0034 § D7).
//!
//! Used by the cockpit boot-time orphan-detect path to check whether a
//! training run's PID is still alive before annotating the status strip.
//!
//! ## Platform-specific implementation
//!
//! - **Unix** (macOS / Linux): `libc::kill(pid, 0)`. Returns `true` if the
//!   process exists and is addressable. PID 0 or negative PIDs are always
//!   `false`. Note: `kill(pid, 0)` may return `EPERM` for processes owned by
//!   another user — we treat `EPERM` as alive (the process exists).
//! - **Windows**: `OpenProcess(SYNCHRONIZE, FALSE, pid)` + non-null handle
//!   check. If the process is not found, returns `false`.
//! - **Other**: always returns `false` (conservative).
//!
//! ## PID-reuse caveat
//!
//! PID reuse is a known false-positive surface on all platforms. The 24-hour
//! orphan window (controlled by `query::orphan_training_runs`'s
//! `fresh_window_secs` parameter) bounds the probability to an acceptable
//! level for an observability-only annotation. See ADR-0034 § D7.

/// Returns `true` if the given PID represents a running process.
///
/// A `pid` of 0 or negative always returns `false`.
///
/// # Platform notes
///
/// On Unix, a `true` result only means the process exists — it may be a
/// zombie waiting to be reaped. For our purposes (orphan detection) this
/// is acceptable: a zombie `train_tcn` process is still "alive" in the sense
/// that it hasn't been completely cleaned up.
#[must_use]
pub fn pid_alive(pid: i64) -> bool {
    if pid <= 0 {
        return false;
    }
    pid_alive_platform(pid)
}

#[cfg(unix)]
fn pid_alive_platform(pid: i64) -> bool {
    // SAFETY: `kill(pid, 0)` is the POSIX-standard way to probe process
    // existence without sending a signal. We cast i64 → libc::pid_t (i32)
    // after range-checking; pids that don't fit in i32 are impossible on
    // any real OS (Linux max PID is 4_194_304, macOS max is 99_998).
    if pid > i64::from(i32::MAX) {
        return false;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let pid_t = pid as libc::pid_t;
    // SAFETY: `kill` is async-signal-safe and does not modify program state.
    let result = unsafe { libc::kill(pid_t, 0) };
    if result == 0 {
        return true;
    }
    // EPERM means the process exists but we don't have permission to signal it.
    // Treat as alive.
    let errno = unsafe { *libc::__error() };
    errno == libc::EPERM
}

#[cfg(windows)]
fn pid_alive_platform(pid: i64) -> bool {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE};

    // SAFETY: Win32 API call with well-defined semantics.
    unsafe {
        match OpenProcess(PROCESS_SYNCHRONIZE, false, pid as u32) {
            Ok(handle) => {
                let _ = CloseHandle(handle);
                true
            }
            Err(_) => false,
        }
    }
}

#[cfg(not(any(unix, windows)))]
fn pid_alive_platform(_pid: i64) -> bool {
    // Conservative fallback: assume not alive on unknown platforms.
    false
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// PID 0 and negative PIDs must always return false.
    #[test]
    fn pid_alive_zero_and_negative_are_false() {
        assert!(!pid_alive(0), "pid=0 must return false");
        assert!(!pid_alive(-1), "pid=-1 must return false");
        assert!(!pid_alive(-1000), "negative pid must return false");
    }

    /// The current process's PID must be alive.
    #[test]
    fn pid_alive_returns_true_for_self() {
        let my_pid = std::process::id() as i64;
        assert!(
            pid_alive(my_pid),
            "current process (pid={my_pid}) must be alive"
        );
    }

    /// A non-existent PID must return false.
    ///
    /// We use PID `i32::MAX` (2_147_483_647) — no OS will ever assign this.
    #[test]
    fn pid_alive_returns_false_for_nonexistent() {
        let impossible_pid = i64::from(i32::MAX);
        assert!(
            !pid_alive(impossible_pid),
            "impossible pid={impossible_pid} must return false"
        );
    }
}
