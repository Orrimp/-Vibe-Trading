//! Cancellation primitives for the backtest engine (lab-end-to-end-v2 T-AR-5 / Wave D-3).
//!
//! `RunCancelHandle` is held by the UI and dropped to signal cancel.
//! `RunCancelReceiver` is passed into the scenario bar loop and polled at
//! the 32/128-bar boundary (K4 mitigation).
//!
//! ## Why std `mpsc::sync_channel(0)`?
//!
//! Cancellation is a one-shot disconnect signal — the sender side is dropped
//! and `try_recv` returns `Disconnected`. A zero-capacity channel is the
//! correct primitive: no buffered messages, disconnect is the signal.

// ── Types ─────────────────────────────────────────────────────────────────────

/// Lightweight cancellation handle.
///
/// Held in the UI by `LabState::run_cancel: Option<RunCancelHandle>`.
/// Dropping this handle signals the in-flight run to abort at the next poll
/// boundary.
pub struct RunCancelHandle {
    _tx: std::sync::mpsc::SyncSender<()>,
}

impl std::fmt::Debug for RunCancelHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunCancelHandle").finish_non_exhaustive()
    }
}

impl RunCancelHandle {
    fn new(tx: std::sync::mpsc::SyncSender<()>) -> Self {
        Self { _tx: tx }
    }
}

/// Receiver end of the cancellation channel.
///
/// Passed into `run_scenario` and polled at the bar-loop poll boundary.
/// `is_cancelled()` returns `true` when the handle has been dropped.
pub struct RunCancelReceiver {
    rx: std::sync::mpsc::Receiver<()>,
}

impl std::fmt::Debug for RunCancelReceiver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunCancelReceiver").finish_non_exhaustive()
    }
}

impl RunCancelReceiver {
    /// Returns `true` if the run has been cancelled (handle dropped).
    ///
    /// Checks for `TryRecvError::Disconnected` which fires when the sender
    /// side (`RunCancelHandle._tx`) is dropped.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        matches!(
            self.rx.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Disconnected)
        )
    }
}

/// Build a `(RunCancelHandle, RunCancelReceiver)` pair.
///
/// The handle is held by the caller; the receiver is passed into the scenario.
/// Dropping the handle signals cancellation at the receiver's next poll.
#[must_use]
pub fn cancellation_pair() -> (RunCancelHandle, RunCancelReceiver) {
    let (tx, rx) = std::sync::mpsc::sync_channel(0);
    (RunCancelHandle::new(tx), RunCancelReceiver { rx })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn cancel_handle_drop_signals_receiver() {
        let (handle, receiver) = cancellation_pair();
        assert!(!receiver.is_cancelled(), "not cancelled before drop");
        drop(handle);
        assert!(receiver.is_cancelled(), "cancelled after handle drop");
    }

    #[test]
    fn cancel_handle_live_not_cancelled() {
        let (handle, receiver) = cancellation_pair();
        assert!(!receiver.is_cancelled(), "not cancelled while live");
        let _ = handle;
    }
}
