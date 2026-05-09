//! `ReflectionWriterTask::run` — consumer side of the bounded mpsc.
//!
//! Per request: `post_mortem_analyst::generate_card` → `store.upsert`.
//! Idempotent skips (R2.4) are logged at `tracing::debug` level.

use std::sync::Arc;

use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::post_mortem_analyst::generate_card;
use crate::store::{ReflectionStore, ReflectionStoreError};
use crate::types::LessonCardWriteRequest;

/// Consumer task that drains the mpsc and persists lesson cards.
pub struct ReflectionWriterTask {
    rx: mpsc::Receiver<LessonCardWriteRequest>,
    store: Arc<dyn ReflectionStore>,
}

impl ReflectionWriterTask {
    /// Create a writer task tied to the given receiver + store.
    #[must_use]
    pub fn new(
        rx: mpsc::Receiver<LessonCardWriteRequest>,
        store: Arc<dyn ReflectionStore>,
    ) -> Self {
        Self { rx, store }
    }

    /// Drain `rx`; for each request, generate a card via
    /// `post_mortem_analyst::generate_card` and upsert it.
    ///
    /// Loops until the sender side is dropped.  A persistent store
    /// error is logged at `tracing::warn` and the loop continues —
    /// dropping a single card is preferable to blocking the
    /// hot-path producer.
    pub async fn run(mut self) {
        while let Some(req) = self.rx.recv().await {
            let card = match generate_card(&req.closed_trade, req.opening_capital, &req.btc_closes)
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    warn!(error = %e, "reflection writer: generate_card failed");
                    continue;
                }
            };
            match self.store.upsert(&card).await {
                Ok(true) => debug!(card_id = %card.card_id, "reflection writer: card inserted"),
                Ok(false) => {
                    debug!(card_id = %card.card_id, "reflection writer: idempotent skip")
                }
                Err(e) => warn!(error = %e, "reflection writer: upsert failed"),
            }
        }
        debug!("reflection writer: receiver closed, exiting");
    }
}

/// Helper alias so callers don't need to import the error type.
pub type WriterRunResult = Result<(), ReflectionStoreError>;
