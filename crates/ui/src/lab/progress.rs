//! `LabProgressRecipe` — iced Subscription that streams backtest progress events
//! from the tokio `mpsc::Receiver<Progress>` produced by `backtest::progress::progress_pair()`
//! into the iced update loop as `Message::LabRunProgress(Progress)`.
//!
//! ## K8 mitigation (same as `ServerTimeRecipe`)
//!
//! The `stream()` method enters the tokio runtime context with
//! `rt_handle.enter()`, creates the necessary state, then drops the guard
//! **before** `Box::pin(...)`. This prevents the `EnterGuard`'s `!Send`
//! constraint from leaking into the `BoxStream<'static, Message>` return type.
//!
//! ## Salt-bump per run
//!
//! `LabProgressRecipe::hash()` mixes in a `salt: u64` that is bumped on
//! every `LabRunRequested`. This ensures iced treats each new run as a
//! distinct subscription and calls `stream()` again (otherwise iced
//! de-duplicates by hash and reuses the old — now closed — stream).
//!
//! ## Channel ownership
//!
//! The receiver is stored in `Arc<Mutex<Option<Receiver<Progress>>>>` on
//! `AppState`. The `LabProgressRecipe` takes ownership of the `Receiver`
//! in `stream()` via `.take()`. This is the same pattern used by
//! `ServerTimeRecipe` in `cockpit_live.rs`.
//!
//! lab-end-to-end-v2 T-AR-6 / R7.3-R7.4 / Wave D-4.

#[cfg(feature = "live")]
pub use live_recipe::LabProgressRecipe;

#[cfg(feature = "live")]
pub use live_recipe::stream_impl;

#[cfg(feature = "live")]
mod live_recipe {
    use std::sync::{Arc, Mutex};

    use backtest::progress::Progress;
    use futures::stream::BoxStream;
    use iced::advanced::subscription::{EventStream, Hasher, Recipe};

    use crate::state::Message;

    /// Iced `Recipe` that reads progress events from the in-flight backtest
    /// and emits them as `Message::LabRunProgress(Progress)`.
    ///
    /// Constructed by `cockpit_live.rs::subscription()` when `lab_progress_rx`
    /// is `Some`.  Each `LabRunRequested` bumps `salt` so iced sees a new
    /// identity and constructs a new `stream()`.
    pub struct LabProgressRecipe {
        pub rt_handle: tokio::runtime::Handle,
        pub rx: Arc<Mutex<Option<tokio::sync::mpsc::Receiver<Progress>>>>,
        /// Per-run salt for iced de-duplication (incremented on `LabRunRequested`).
        pub salt: u64,
    }

    impl Recipe for LabProgressRecipe {
        type Output = Message;

        fn hash(&self, state: &mut Hasher) {
            use std::any::TypeId;
            use std::hash::Hash;
            TypeId::of::<Self>().hash(state);
            self.salt.hash(state);
        }

        fn stream(
            self: Box<Self>,
            _input: EventStream,
        ) -> futures::stream::BoxStream<'static, Self::Output> {
            // K8 — enter the tokio runtime context to safely take the
            // `tokio::sync::mpsc::Receiver`, then drop the guard before
            // `Box::pin` so the returned `BoxStream` remains `Send + 'static`.
            let rx_opt = {
                let _guard = self.rt_handle.enter();
                self.rx
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .take()
            };

            stream_impl(rx_opt)
        }
    }

    /// Inner stream logic, extracted so integration tests can drive it directly
    /// without needing a running iced application or an `EventStream`.
    ///
    /// - When `rx_opt` is `Some(rx)`: drains progress events as
    ///   `Message::LabRunProgress`, then emits `Message::LabRunProgressDone`
    ///   when the sender side closes.
    /// - When `rx_opt` is `None` (i.e. `stream()` was called a second time
    ///   after the receiver was already taken): the stream yields nothing.
    ///   This is the smoking-gun case: a silent empty stream means the UI
    ///   never receives progress messages.
    #[must_use]
    pub fn stream_impl(
        rx_opt: Option<tokio::sync::mpsc::Receiver<Progress>>,
    ) -> BoxStream<'static, Message> {
        Box::pin(async_stream::stream! {
            if let Some(mut rx) = rx_opt {
                while let Some(progress) = rx.recv().await {
                    yield Message::LabRunProgress(progress);
                }
                // R7.4 — channel closed: engine completed or was cancelled.
                // Belt-and-suspenders clear before LabRunCompleted arrives.
                yield Message::LabRunProgressDone;
            }
            // If rx_opt was None: stream yields nothing (double-stream() call
            // after .take() already consumed the receiver). This is the
            // silent-failure case that lab_progress_recipe_stream.rs tests.
        })
    }
}
