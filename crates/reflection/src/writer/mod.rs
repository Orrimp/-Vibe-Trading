//! `ReflectionWriter` (producer) + `LessonCardWriteRequest`.
//!
//! Bounded tokio mpsc, capacity Q8 = 1024 default.  Producer-side
//! `try_send`; on `TrySendError::Full` we increment
//! `reflection_card_dropped_total{reason="back_pressure"}` and return
//! `Err(TryEnqueueError::BackPressure)`.  R7.1 hot-path invariant —
//! never block the executor's submit-fill thread.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use thiserror::Error;
use tokio::sync::mpsc;

use crate::store::ReflectionStore;
use crate::types::LessonCardWriteRequest;

pub mod task;

pub use task::ReflectionWriterTask;

/// Producer side of the bounded mpsc.  Cheap to clone — `Sender` is
/// internally reference-counted; the `dropped` `AtomicU64` is shared.
#[derive(Clone)]
pub struct ReflectionWriter {
    tx: mpsc::Sender<LessonCardWriteRequest>,
    dropped: Arc<AtomicU64>,
}

/// Errors from `try_enqueue`.
#[derive(Debug, Error)]
pub enum TryEnqueueError {
    /// Channel full — message dropped, counter incremented.
    #[error("channel full — dropped under back-pressure")]
    BackPressure,
    /// Receiver closed (writer task exited).
    #[error("receiver closed")]
    Closed,
}

impl ReflectionWriter {
    /// Create the writer + the consumer task pair.
    ///
    /// The returned `ReflectionWriter` is held by the executor's
    /// fill-handler tap; the `ReflectionWriterTask` is spawned in
    /// `agent::main` (gated by `cfg.reflection.enable_writer`).
    #[must_use]
    pub fn new(store: Arc<dyn ReflectionStore>, capacity: usize) -> (Self, ReflectionWriterTask) {
        let (tx, rx) = mpsc::channel(capacity);
        let dropped = Arc::new(AtomicU64::new(0));
        let writer = Self {
            tx,
            dropped: dropped.clone(),
        };
        let task = ReflectionWriterTask::new(rx, store);
        (writer, task)
    }

    /// Test-only constructor — no consumer wired.
    #[doc(hidden)]
    #[must_use]
    pub fn for_test(capacity: usize) -> (Self, mpsc::Receiver<LessonCardWriteRequest>) {
        let (tx, rx) = mpsc::channel(capacity);
        let dropped = Arc::new(AtomicU64::new(0));
        (Self { tx, dropped }, rx)
    }

    /// Enqueue a write request.  Returns `Err(BackPressure)` on a
    /// full channel; the dropped counter is incremented + the
    /// Prometheus counter `reflection_card_dropped_total{reason="back_pressure"}`
    /// is bumped.
    ///
    /// # Errors
    ///
    /// - [`TryEnqueueError::BackPressure`] on a full channel.
    /// - [`TryEnqueueError::Closed`] when the receiver is gone.
    pub fn try_enqueue(&self, req: LessonCardWriteRequest) -> Result<(), TryEnqueueError> {
        match self.tx.try_send(req) {
            Ok(()) => Ok(()),
            Err(mpsc::error::TrySendError::Full(_)) => {
                self.dropped.fetch_add(1, Ordering::Relaxed);
                metrics::counter!(
                    "reflection_card_dropped_total",
                    "reason" => "back_pressure"
                )
                .increment(1);
                Err(TryEnqueueError::BackPressure)
            }
            Err(mpsc::error::TrySendError::Closed(_)) => Err(TryEnqueueError::Closed),
        }
    }

    /// Snapshot of the dropped counter (test helper).
    #[must_use]
    pub fn dropped_count(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}
