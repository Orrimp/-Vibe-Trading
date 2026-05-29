//! Cancellation primitives for the backtest engine (lab-end-to-end-v2 T-AR-5 / Wave D-3).
//!
//! `RunCancelHandle` is held by the UI and dropped to signal cancel.
//! `RunCancelReceiver` is passed into the scenario bar loop and polled at
//! the 32/128-bar boundary (K4 mitigation).
//!
//! ## ADR-0050 § D2 — CancellationToken as canonical primitive
//!
//! Bug #64 D.1.1 attempt-3 (2026-05-29): this module replaces the previous
//! `std::sync::mpsc::sync_channel(0)` disconnect-idiom with
//! `tokio_util::sync::CancellationToken`. Rationale:
//!
//! - One source of truth: a single `CancellationToken` shared between handle
//!   and receiver, no parallel std-channel + tokio-Notify signals.
//! - `cancelled() -> impl Future` is directly usable in `tokio::select!` as
//!   a third arm during the preload loop (D-R2.2 fix for R2 structural
//!   omission — Stop button broken during cold-cache window).
//! - Production-proven in axum/hyper/sqlx graceful-shutdown paths; boring
//!   choice per CLAUDE.md § Coding rules.
//!
//! Public API is backward-compatible: `is_cancelled()` continues to work
//! for all existing callers in the engine's bar loops. New callers
//! (`runner.rs` select! preload arm) use `cancelled()`.

use tokio_util::sync::CancellationToken;

// ── Types ─────────────────────────────────────────────────────────────────────

/// Lightweight cancellation handle.
///
/// Held in the UI by `LabState::run_cancel: Option<RunCancelHandle>`.
/// Dropping this handle signals the in-flight run to abort at the next poll
/// boundary. The signal is delivered via `CancellationToken::cancel()` which
/// flips the token atomically — any `cancelled().await` in progress wakes
/// immediately.
pub struct RunCancelHandle {
    token: CancellationToken,
}

impl std::fmt::Debug for RunCancelHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunCancelHandle").finish_non_exhaustive()
    }
}

impl Drop for RunCancelHandle {
    /// Dropping the handle cancels the token — mirrors the old
    /// `std::sync::mpsc::SyncSender` disconnect semantics.
    fn drop(&mut self) {
        self.token.cancel();
    }
}

/// Receiver end of the cancellation channel.
///
/// Passed into `run_scenario` and polled at the bar-loop poll boundary.
/// `is_cancelled()` returns `true` when the handle has been dropped.
/// `cancelled()` returns a `Future` that resolves when the handle is dropped —
/// usable directly in `tokio::select!`.
pub struct RunCancelReceiver {
    token: CancellationToken,
}

impl std::fmt::Debug for RunCancelReceiver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunCancelReceiver").finish_non_exhaustive()
    }
}

impl RunCancelReceiver {
    /// Returns `true` if the run has been cancelled (handle dropped).
    ///
    /// Checks `CancellationToken::is_cancelled()` — an atomic load, equivalent
    /// to the previous `try_recv() == Err(Disconnected)` pattern.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }

    /// Returns a `Future` that resolves when the handle is dropped (cancelled).
    ///
    /// This future is safe to use in `tokio::select!` as a third arm alongside
    /// the preload and ticker arms (D-R2.2 / ADR-0050 § D2). Cancels atomically;
    /// no blocking or allocation.
    ///
    /// Usage:
    /// ```rust,ignore
    /// tokio::select! {
    ///     biased;
    ///     result = &mut preload_future => { break result; }
    ///     _ = cancel.cancelled() => { return Err(SmolStr::new("cancelled")); }
    ///     _ = ticker.tick() => { /* emit elapsed */ }
    /// }
    /// ```
    pub async fn cancelled(&self) {
        self.token.cancelled().await;
    }
}

/// Build a `(RunCancelHandle, RunCancelReceiver)` pair.
///
/// The handle is held by the caller; the receiver is passed into the scenario.
/// Dropping the handle signals cancellation via `CancellationToken::cancel()`.
#[must_use]
pub fn cancellation_pair() -> (RunCancelHandle, RunCancelReceiver) {
    let token = CancellationToken::new();
    (
        RunCancelHandle {
            token: token.clone(),
        },
        RunCancelReceiver { token },
    )
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
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

    /// ADR-0050 § D2 — `cancelled()` future resolves after handle drop.
    #[tokio::test]
    async fn cancelled_future_resolves_after_drop() {
        let (handle, receiver) = cancellation_pair();
        // Spawn a task that waits for cancellation.
        let wait = tokio::spawn(async move { receiver.cancelled().await });
        // Drop the handle; the future should wake.
        drop(handle);
        // Must complete within 100 ms.
        tokio::time::timeout(std::time::Duration::from_millis(100), wait)
            .await
            .expect("timeout — cancelled() future did not resolve")
            .expect("task panicked");
    }

    /// Verify `is_cancelled()` before and after explicit cancel.
    #[test]
    fn is_cancelled_reflects_token_state() {
        let (handle, receiver) = cancellation_pair();
        assert!(!receiver.is_cancelled());
        drop(handle);
        assert!(receiver.is_cancelled());
        // Second call is idempotent.
        assert!(receiver.is_cancelled());
    }
}
