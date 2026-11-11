//! `TrainingLogRecipe` — iced Subscription that streams `TrainingLogLine` events
//! from the std-mpsc `Receiver<TrainingLogLine>` produced by `trainer::spawn_training_run`
//! into the iced update loop as `Message::TrainingLogLine(line)`.
//!
//! ## Design (H2 resolution — cockpit-training-pressed-wiring T-AR-1)
//!
//! Mirrors `lab::progress::LabProgressRecipe` symbol-for-symbol with one delta:
//! `LabProgressRecipe` holds a `tokio::sync::mpsc::Receiver<Progress>` (natively
//! async), whereas this recipe holds a `std::sync::mpsc::Receiver<TrainingLogLine>`
//! (blocking). The bridge is a dedicated OS thread that owns the blocking `recv()`
//! and forwards each line into a `futures` unbounded channel, so the stream body
//! needs NO async runtime. It used to be `tokio::task::spawn_blocking`, which
//! panics with "there is no reactor running" under the executor iced actually uses
//! — `iced_futures` is built with `thread-pool` only, no `tokio` — so the cockpit's
//! training-log subscription died on its first poll while every test, all running
//! under `#[tokio::test]`, supplied the missing reactor (bug-log `#196`).
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

    use futures::StreamExt;
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
    /// - When `rx_opt` is `Some(rx)`: a dedicated OS thread (`training-log-bridge`)
    ///   owns the blocking `std::sync::mpsc::Receiver` and forwards every
    ///   `TrainingLogLine` into a `futures` unbounded channel; the stream yields
    ///   `Message::TrainingLogLine(line.text)` for each. The stream ends when the
    ///   sender side closes (the subprocess exited) and the bridge thread returns.
    ///   If the stream is dropped first, the bridge's next forward fails and it
    ///   returns too; until then it is parked in `recv()` (bounded by the
    ///   subprocess's lifetime).
    /// - When `rx_opt` is `None` (i.e. `stream()` was called a second time
    ///   after the receiver was already taken): the stream yields nothing.
    ///
    /// The body needs no async runtime, so it is correct under ANY executor —
    /// including iced's futures `ThreadPool`, which has no tokio reactor
    /// (bug-log `#196`; pinned by `recipe_stream_drains_without_a_tokio_runtime`).
    #[must_use]
    pub fn stream_impl(
        rx_opt: Option<std::sync::mpsc::Receiver<TrainingLogLine>>,
    ) -> BoxStream<'static, Message> {
        let Some(rx) = rx_opt else {
            // Double-stream() call after `.take()` already consumed the receiver.
            return Box::pin(futures::stream::empty());
        };
        let (tx, lines) = futures::channel::mpsc::unbounded::<TrainingLogLine>();
        let bridge = std::thread::Builder::new()
            .name("training-log-bridge".into())
            .spawn(move || {
                while let Ok(line) = rx.recv() {
                    if tx.unbounded_send(line).is_err() {
                        break; // the stream was dropped
                    }
                }
            });
        if let Err(e) = bridge {
            tracing::warn!(error = %e, "training-log bridge thread could not be spawned; no log lines will be shown");
            return Box::pin(futures::stream::empty());
        }
        Box::pin(lines.map(|line| Message::TrainingLogLine(SmolStr::new(line.text.as_str()))))
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

    /// `stream_impl` yields lines then terminates when sender drops.
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

    /// `stream_impl` with None yields nothing.
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

    /// bug-log `#196`: the PRODUCTION topology. iced polls subscription streams on its
    /// futures `ThreadPool` (`iced_futures` "thread-pool", no "tokio"), so the real
    /// recipe is streamed and polled with `futures::executor::block_on` on a thread
    /// that has no tokio context — asserted, not assumed. Before the fix this panicked
    /// at the `spawn_blocking` ("there is no reactor running"); every other test here
    /// supplies a reactor and could not see it.
    #[test]
    fn recipe_stream_drains_without_a_tokio_runtime() {
        use super::live_recipe::TrainingLogRecipe;
        use iced::advanced::subscription::Recipe;
        use std::sync::{Arc, Mutex};

        // The recipe carries a handle (production passes the agent runtime's), but
        // nothing may need it while the stream is polled.
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        let (tx, rx) = std::sync::mpsc::channel::<TrainingLogLine>();
        for text in ["alpha", "beta"] {
            tx.send(TrainingLogLine {
                text: SmolStr::new(text),
                is_stderr: false,
            })
            .unwrap();
        }
        drop(tx);
        let recipe = Box::new(TrainingLogRecipe {
            rt_handle: rt.handle().clone(),
            rx: Arc::new(Mutex::new(Some(rx))),
            salt: 1,
        });

        let (out_tx, out_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let no_reactor = tokio::runtime::Handle::try_current().is_err();
            let stream = recipe.stream(Box::pin(futures::stream::empty()));
            let got: Vec<Message> = futures::executor::block_on(stream.collect());
            let _ = out_tx.send((no_reactor, got));
        });
        // A panic on the polling thread drops `out_tx`, so this fails instead of hanging.
        let (no_reactor, got) = out_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("the polling thread reported back (it did not panic or hang)");

        assert!(
            no_reactor,
            "premise: the polling thread must have NO tokio context"
        );
        assert_eq!(got.len(), 2, "exactly the two lines, then the stream ends");
        assert!(matches!(&got[0], Message::TrainingLogLine(s) if s == "alpha"));
        assert!(matches!(&got[1], Message::TrainingLogLine(s) if s == "beta"));
    }
}
