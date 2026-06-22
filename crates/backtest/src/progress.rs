//! Progress primitives for the backtest engine (lab-end-to-end-v2 T-AR-5 / Wave D-4).
//!
//! The progress channel piggybacks on the cancel-poll boundary per Q4=(b):
//! at `bar_idx & 0x7F == 0` (every 128 bars steady-state, every 32 bars
//! for the first 128 bars — K4 mitigation), the scenario emits a `Progress`
//! event via `ProgressSender::try_send`.
//!
//! ## Why tokio `mpsc::channel(8)` for progress?
//!
//! Progress updates are bounded, lossy-by-design: a slow UI simply drops
//! stale events. `tokio::sync::mpsc::channel(8)` gives bounded backpressure
//! with a `try_send` that discards when full.  This is orthogonal to the
//! cancellation channel (which uses std `mpsc::sync_channel(0)`).

// ── Types ─────────────────────────────────────────────────────────────────────

/// Per-bar progress event emitted at the poll boundary (Q4=(b)).
#[derive(Debug, Clone, Copy)]
pub struct Progress {
    /// Current bar index (0-based).
    pub current_bar: usize,
    /// Total bars in the scenario.
    pub total_bars: usize,
    /// Wall-clock elapsed since the run started (milliseconds).
    pub elapsed_ms: u64,
}

/// Candidate-granularity bake-off progress event.
///
/// Emitted by `run_bakeoff` immediately BEFORE each `run_scenario` call so
/// the UI can display "running X of N: <strategy-id>".
///
/// - `done` — number of candidates **fully completed** so far (0 before the
///   first candidate starts, 1 after the first finishes, …).
/// - `total` — total candidate count, including the buy-and-hold benchmark.
/// - `current_id` — the strategy id **about to start** (not yet complete).
///
/// The ui-designer consumes this directly; `backtest` is already a dep of `ui`.
/// `SmolStr` keeps the type `Clone` and heap-allocation-free for short ids.
#[derive(Debug, Clone)]
pub struct BakeoffProgress {
    /// Candidates fully completed so far (0-based: 0 means "first one starting").
    pub done: u16,
    /// Total number of candidates (field size + 1 for buy-and-hold benchmark).
    pub total: u16,
    /// Strategy id of the candidate now starting (e.g. `"v0.sma"`, `"v0.5.macd"`).
    pub current_id: smol_str::SmolStr,
}

/// Optional lossy sender for candidate-level bake-off progress.
///
/// `None` inner variant ⇒ no-op / disabled (headless / test path).  The channel
/// is separate from the per-bar `ProgressSender` — two orthogonal concerns.
#[derive(Debug, Clone)]
pub struct BakeoffProgressSender(pub Option<tokio::sync::mpsc::Sender<BakeoffProgress>>);

impl BakeoffProgressSender {
    /// Build a sender backed by the provided `tokio::sync::mpsc::Sender`.
    #[must_use]
    pub fn new(tx: tokio::sync::mpsc::Sender<BakeoffProgress>) -> Self {
        Self(Some(tx))
    }

    /// Build a no-op sender.  No channel allocation occurs.
    #[must_use]
    pub fn disabled() -> Self {
        Self(None)
    }

    /// Send a progress event, dropping it if the channel is full or closed.
    /// Never blocks.
    pub fn try_send(&self, progress: BakeoffProgress) {
        if let Some(tx) = &self.0 {
            let _ = tx.try_send(progress);
        }
    }
}

/// Build a `(BakeoffProgressSender, tokio::sync::mpsc::Receiver<BakeoffProgress>)` pair.
///
/// Capacity 8 — lossy (same convention as `progress_pair`).
#[must_use]
pub fn bakeoff_progress_pair() -> (
    BakeoffProgressSender,
    tokio::sync::mpsc::Receiver<BakeoffProgress>,
) {
    let (tx, rx) = tokio::sync::mpsc::channel(8);
    (BakeoffProgressSender::new(tx), rx)
}

/// Lossy sender — wraps `tokio::sync::mpsc::Sender<Progress>`.
///
/// `None` inner variant is the no-op / disabled path used by the CLI and
/// by determinism tests that don't need progress events.
#[derive(Debug, Clone)]
pub struct ProgressSender(Option<tokio::sync::mpsc::Sender<Progress>>);

impl ProgressSender {
    /// Build a sender backed by a `tokio::sync::mpsc::channel(capacity=8)`.
    #[must_use]
    pub fn new(tx: tokio::sync::mpsc::Sender<Progress>) -> Self {
        Self(Some(tx))
    }

    /// Build a no-op sender for tests / CLI / no-progress call sites.
    ///
    /// `try_send` is a no-op; no channel allocation occurs.
    #[must_use]
    pub fn disabled() -> Self {
        Self(None)
    }

    /// Send a progress event, dropping it if the channel is full or the
    /// receiver has been closed. Never blocks.
    pub fn try_send(&self, progress: Progress) {
        if let Some(tx) = &self.0 {
            let _ = tx.try_send(progress);
        }
    }
}

/// Build a `(ProgressSender, tokio::sync::mpsc::Receiver<Progress>)` pair.
///
/// Capacity 8 — lossy: if the UI is slow, old events are dropped by the
/// sender. The receiver is owned by the `LabProgressRecipe` subscription.
#[must_use]
pub fn progress_pair() -> (ProgressSender, tokio::sync::mpsc::Receiver<Progress>) {
    let (tx, rx) = tokio::sync::mpsc::channel(8);
    (ProgressSender::new(tx), rx)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation
)]
mod tests {
    use super::*;

    /// `ProgressSender::disabled()` does not panic on `try_send`.
    #[test]
    fn disabled_sender_try_send_is_noop() {
        let sender = ProgressSender::disabled();
        // Should not panic or block.
        sender.try_send(Progress {
            current_bar: 0,
            total_bars: 100,
            elapsed_ms: 0,
        });
    }

    /// `progress_pair()` produces a working channel.
    #[tokio::test]
    async fn progress_pair_sends_and_receives() {
        let (tx, mut rx) = progress_pair();
        tx.try_send(Progress {
            current_bar: 128,
            total_bars: 1000,
            elapsed_ms: 50,
        });
        let got = rx.recv().await.expect("should receive progress");
        assert_eq!(got.current_bar, 128);
        assert_eq!(got.total_bars, 1000);
    }

    /// Lossy: when channel is full, `try_send` drops silently.
    #[tokio::test]
    async fn progress_sender_drops_when_full() {
        let (tx, _rx) = progress_pair();
        // Fill the channel (capacity = 8) but don't receive.
        for i in 0..20u64 {
            tx.try_send(Progress {
                current_bar: i as usize,
                total_bars: 100,
                elapsed_ms: i,
            });
        }
        // No panic — extra sends are dropped silently.
    }
}
