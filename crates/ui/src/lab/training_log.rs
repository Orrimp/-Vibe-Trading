//! `TrainingLogRecipe` — iced Subscription that streams `TrainingLogLine` events
//! from the std-mpsc `Receiver<TrainingLogLine>` produced by `trainer::spawn_training_run`
//! into the iced update loop as `Message::TrainingLogLine(line)`.
//!
//! ## Design (H2 resolution — cockpit-training-pressed-wiring T-AR-1)
//!
//! Mirrors `lab::progress::LabProgressRecipe` symbol-for-symbol with one delta:
//! `LabProgressRecipe` holds a `tokio::sync::mpsc::Receiver<Progress>` (natively
//! async), whereas this recipe holds a `std::sync::mpsc::Receiver<TrainingLogLine>`
//! (blocking). The bridge is `tokio::task::spawn_blocking`: each blocking `recv()`
//! call is lifted into a blocking-thread task so it doesn't block the async executor.
//!
//! ## K8 mitigation (same as `LabProgressRecipe` and `ServerTimeRecipe`)
//!
//! The `stream()` method enters the tokio runtime context with `rt_handle.enter()`,
//! takes the receiver from the `Arc<Mutex<Option<_>>>`, then drops the guard BEFORE
//! `Box::pin(...)`. This prevents the `EnterGuard`'s `!Send` constraint from leaking
//! into the `BoxStream<'static, Message>` return type.
//!
//! ## Salt-bump per run
//!
//! `TrainingLogRecipe::hash()` mixes in a `salt: u64` that is bumped on every
//! `TrainingPressed`. This ensures iced treats each new run as a distinct
//! subscription and calls `stream()` again (otherwise iced de-duplicates by hash
//! and reuses the old — now closed — stream).
//!
//! ## Channel ownership
//!
//! The receiver is stored in `Arc<Mutex<Option<Receiver<TrainingLogLine>>>>` on
//! `AppState`. The `TrainingLogRecipe` takes ownership of the `Receiver`
//! in `stream()` via `.take()`. Same pattern as `LabProgressRecipe`.
//!
//! cockpit-training-pressed-wiring v0.1.0 T-D-N3.

#[cfg(feature = "live")]
pub use live_recipe::TrainingLogRecipe;

#[cfg(feature = "live")]
pub use live_recipe::stream_impl;

#[cfg(feature = "live")]
mod live_recipe {
    use std::sync::{Arc, Mutex};

    use futures::stream::BoxStream;
    use iced::advanced::subscription::{EventStream, Hasher, Recipe};
    use smol_str::SmolStr;

    use crate::lab::trainer::TrainingLogLine;
    use crate::state::Message;

    /// Iced `Recipe` that reads training log lines from the in-flight training
    /// subprocess and emits them as `Message::TrainingLogLine(line)`.
    ///
    /// Constructed by `cockpit_live.rs::subscription()` when `training_log_rx`
    /// is `Some`. Each `TrainingPressed` bumps `salt` so iced sees a new
    /// identity and constructs a new `stream()`.
    pub struct TrainingLogRecipe {
        pub rt_handle: tokio::runtime::Handle,
        pub rx: Arc<Mutex<Option<std::sync::mpsc::Receiver<TrainingLogLine>>>>,
        /// Per-run salt for iced de-duplication (incremented on `TrainingPressed`).
        pub salt: u64,
    }

    impl Recipe for TrainingLogRecipe {
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
            // `std::sync::mpsc::Receiver`, then drop the guard before
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
    /// - When `rx_opt` is `Some(rx)`: bridges the blocking `std::sync::mpsc::Receiver`
    ///   to the async stream via `tokio::task::spawn_blocking`. Each received
    ///   `TrainingLogLine` becomes `Message::TrainingLogLine(line.text)`.
    ///   When the sender side closes (channel disconnected), the stream ends.
    /// - When `rx_opt` is `None` (i.e. `stream()` was called a second time
    ///   after the receiver was already taken): the stream yields nothing.
    pub fn stream_impl(
        rx_opt: Option<std::sync::mpsc::Receiver<TrainingLogLine>>,
    ) -> BoxStream<'static, Message> {
        Box::pin(async_stream::stream! {
            if let Some(rx) = rx_opt {
                // Wrap the blocking receiver in an Arc<Mutex<_>> so we can
                // move it into the spawn_blocking closure on each iteration.
                let rx = Arc::new(Mutex::new(rx));
                loop {
                    let rx_clone = Arc::clone(&rx);
                    // Bridge std-mpsc blocking recv() into the async executor.
                    let result = tokio::task::spawn_blocking(move || {
                        rx_clone
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .recv()
                    })
                    .await;

                    match result {
                        Ok(Ok(line)) => {
                            yield Message::TrainingLogLine(SmolStr::new(line.text.as_str()));
                        }
                        // Channel disconnected (sender dropped) — subprocess exited.
                        Ok(Err(_recv_error)) => break,
                        // spawn_blocking task panicked — stop the stream.
                        Err(_join_error) => break,
                    }
                }
            }
            // If rx_opt was None: stream yields nothing (double-stream() call
            // after .take() already consumed the receiver).
        })
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[cfg(feature = "live")]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::live_recipe::stream_impl;
    use crate::lab::trainer::TrainingLogLine;
    use crate::state::Message;
    use futures::StreamExt;
    use smol_str::SmolStr;

    /// stream_impl yields lines then terminates when sender drops.
    #[test]
    fn stream_yields_lines_and_terminates() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let (tx, rx) = std::sync::mpsc::sync_channel::<TrainingLogLine>(16);

        tx.send(TrainingLogLine {
            text: SmolStr::new("hello"),
            is_stderr: false,
        })
        .unwrap();
        tx.send(TrainingLogLine {
            text: SmolStr::new("world"),
            is_stderr: true,
        })
        .unwrap();
        drop(tx); // Close the channel.

        let messages: Vec<Message> = rt.block_on(async {
            let s = stream_impl(Some(rx));
            futures::pin_mut!(s);
            let mut out = Vec::new();
            while let Some(m) = s.next().await {
                out.push(m);
            }
            out
        });

        assert_eq!(messages.len(), 2);
        assert!(matches!(&messages[0], Message::TrainingLogLine(s) if s == "hello"));
        assert!(matches!(&messages[1], Message::TrainingLogLine(s) if s == "world"));
    }

    /// stream_impl with None yields nothing.
    #[test]
    fn stream_with_none_yields_nothing() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let messages: Vec<Message> = rt.block_on(async {
            let s = stream_impl(None);
            futures::pin_mut!(s);
            let mut out = Vec::new();
            while let Some(m) = s.next().await {
                out.push(m);
            }
            out
        });
        assert!(messages.is_empty());
    }
}
