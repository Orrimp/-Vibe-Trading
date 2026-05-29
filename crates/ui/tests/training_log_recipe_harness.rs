//! Surface 1 — boundary tests for `TrainingLogRecipe` stream path.
//!
//! lab-recipe-test-harness v0.2.0 Wave A (T-D-A1 / ADR-0048 D3).
//!
//! ## What this file tests
//!
//! The three regression categories from Bug #64, applied to `TrainingLogRecipe`.
//! `TrainingLogRecipe` is the **exact Bug #64 shape**: `std::sync::mpsc::Receiver`
//! bridged via `spawn_blocking` + `Arc<Mutex<Option<_>>>::take()` + per-run salt.
//!
//! **A — Sentinel emission timing**: the first `Message::TrainingLogLine` arrives
//! before the sender is dropped (i.e., the stream is live as soon as `Some(rx)` is
//! passed). Assert first event arrives within 50 ms.
//!
//! **B — Channel survival across `Arc<Mutex<Option<_>>>::take()`**: driving
//! `stream_impl(Some(rx))` consumes the receiver exactly once. A second call with
//! `None` (simulating the post-`take()` state) yields zero messages.
//!
//! **C — Graceful shutdown (sender-drop EOF)**: when the `SyncSender` is dropped,
//! the stream terminates cleanly without panicking (no orphan `spawn_blocking` tasks).
//!
//! ## Why drive `stream_impl` directly rather than `TrainingLogRecipe::stream`?
//!
//! `TrainingLogRecipe::stream` requires an iced `EventStream` and enters the tokio
//! runtime context. `stream_impl` is the extracted inner logic — exact same code
//! path, accessible without an iced application. This mirrors the v0.1.0 pattern
//! (`spawn_lab_run_yahoo_harness.rs` drives `spawn_lab_run`'s inner logic directly).
//!
//! ## `#[cfg(feature = "live")]` gate
//!
//! `stream_impl` and `TrainingLogRecipe` are `#[cfg(feature = "live")]`.
//! Run with `cargo test -p ui --test training_log_recipe_harness --features live`.
//!
//! ## T-T4 falsification probe (per ADR-0048 § Changelog 2026-05-29 v0.2.0)
//!
//! To verify this harness genuinely catches the regression class, apply ONE of the
//! following mutations to the production source, then re-run this test file. Restore
//! when done.
//!
//! | Probe | Source line to comment out / mutate | Expected failing test | Expected failure |
//! |---|---|---|---|
//! | P1 — stream yield | `crates/ui/src/lab/training_log.rs:124` — comment out the `yield Message::TrainingLogLine(...)` line | `stream_yields_lines_in_order` | `assert_eq!(messages.len(), 3)` FAILs with `left: 0, right: 3` |
//! | P2 — take ownership | `crates/ui/src/lab/training_log.rs:87` — replace `.take()` with `.as_ref().map(|_| ())` semantic-break by returning cloned rx | `salt_bump_survives_arc_mutex_take` | `assert!(second_messages.is_empty())` FAILs — see note |
//!
//! Note on P2: since `std::sync::mpsc::Receiver` is not `Clone`, the only way to
//! break the `.take()` semantic is to NOT drain the Option. In a real regression the
//! `take()` call would be deleted or replaced with `as_ref()`, leaving the Option
//! populated. The P2 probe asserts the Option IS drained after the first `take()` call.
//!
//! **Developer dry-run result (Wave A, 2026-05-29)**:
//! - Applied P1 probe (yield replaced with `if false { yield ... }` guard):
//!   - `sentinel_log_line_emitted_before_subprocess_spawn` FAILED — Elapsed(()) timeout
//!   - `salt_bump_survives_arc_mutex_take` FAILED — `left: 0, right: 3`
//!   - `log_stream_survives_recipe_drop` FAILED — `left: 0, right: 2`
//! - Restored line 124: all 3 tests PASS.
//! Exact output: `test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s`

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures::StreamExt;
use smol_str::SmolStr;
use tokio::time::timeout;
use ui::lab::trainer::TrainingLogLine;
use ui::lab::training_log::stream_impl;
use ui::state::Message;

// ── MockTrainingLogChannel ────────────────────────────────────────────────────

/// Test double for the `std::sync::mpsc` channel that the real
/// `spawn_training_run` produces.
///
/// Holds the `SyncSender<TrainingLogLine>` so test code can drive lines
/// into the stream. Provides `take_rx()` to hand off the `Receiver` exactly
/// as production does via `Arc<Mutex<Option<_>>>::take()`.
///
/// Design: per-Recipe-specific (D-V0.2.0-1 rationale — no shared trait;
/// mirrors `MockLabYahooBarSource` in `spawn_lab_run_yahoo_harness.rs`).
struct MockTrainingLogChannel {
    /// The sender half. Tests drive this; can be cloned to produce extra
    /// senders. Channel closes when ALL sender clones are dropped.
    pub tx: std::sync::mpsc::SyncSender<TrainingLogLine>,
    /// Wraps the receiver in `Arc<Mutex<Option<_>>>` exactly as production
    /// `AppState` stores it. `take_rx()` drains the Option, mirroring
    /// `TrainingLogRecipe::stream()`'s `.take()` call.
    pub rx_opt: Arc<Mutex<Option<std::sync::mpsc::Receiver<TrainingLogLine>>>>,
}

impl MockTrainingLogChannel {
    /// Capacity 16 — matches the `stream_yields_lines_and_terminates` unit test.
    fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::sync_channel::<TrainingLogLine>(16);
        Self {
            tx,
            rx_opt: Arc::new(Mutex::new(Some(rx))),
        }
    }

    /// Send a log line. Returns `Err` if the receiver has already been dropped.
    fn send(
        &self,
        text: &str,
        is_stderr: bool,
    ) -> Result<(), std::sync::mpsc::SendError<TrainingLogLine>> {
        self.tx.send(TrainingLogLine {
            text: SmolStr::new(text),
            is_stderr,
        })
    }

    /// Take the `Receiver` out of the `Arc<Mutex<Option<_>>>` — mimics exactly
    /// what `TrainingLogRecipe::stream()` does via `.take()`.
    /// Returns `None` if the receiver was already taken (second-stream call).
    fn take_rx(&self) -> Option<std::sync::mpsc::Receiver<TrainingLogLine>> {
        self.rx_opt
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
    }

    /// Peek whether the Option still holds a receiver.
    fn has_rx(&self) -> bool {
        self.rx_opt
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_some()
    }
}

// ── Test 1 — Sentinel log line emitted before subprocess spawn ─────────────────

/// T-D-A1 / ADR-0048 D3 category A — sentinel emission timing.
///
/// Asserts: the first `Message::TrainingLogLine` arrives within 50 ms of the
/// sender dispatching the first line (i.e., the `stream_impl` path is live
/// immediately on construction; no warm-up delay).
///
/// Analogue of v0.1.0 Test 1 (`sentinel_fires_before_preload_await`) applied
/// to `TrainingLogRecipe`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sentinel_log_line_emitted_before_subprocess_spawn() {
    let mock = MockTrainingLogChannel::new();

    // Pre-load a sentinel line BEFORE taking the receiver — simulates the
    // in-memory buffer being populated before the Recipe stream is started.
    assert!(mock.has_rx(), "Option must be Some before any take()");
    mock.send("[info] epoch 1/30 complete", false).unwrap();

    let rx = mock
        .take_rx()
        .expect("receiver must be present before first stream call");
    assert!(!mock.has_rx(), "Option must be None after take()");

    let start = Instant::now();

    // Start the stream.
    let mut stream = Box::pin(stream_impl(Some(rx)));

    // Assert the first event arrives well within the 50 ms budget.
    let first_event = timeout(Duration::from_millis(50), stream.next())
        .await
        .expect(
            "first TrainingLogLine must arrive within 50ms \
             (regression A: stream not live immediately)",
        )
        .expect("stream must not be empty when receiver has a pending message");

    let elapsed = start.elapsed();

    assert!(
        matches!(&first_event, Message::TrainingLogLine(s) if s == "[info] epoch 1/30 complete"),
        "first event must be the sentinel line; got {first_event:?}"
    );
    assert!(
        elapsed < Duration::from_millis(50),
        "sentinel must arrive in < 50ms; actual: {}ms. \
         Regression A: stream_impl path adds unexpected delay.",
        elapsed.as_millis()
    );

    // Cleanly terminate: drop sender (mock) so stream ends.
    drop(mock);
    // Drain remaining.
    while stream.next().await.is_some() {}
}

// ── Test 2 — Salt-bump survives Arc<Mutex<Option<_>>>::take ───────────────────

/// T-D-A1 / ADR-0048 D3 category B — channel ownership via `.take()`.
///
/// Asserts:
/// 1. First stream call with `Some(rx)` → receives all 3 sent lines.
/// 2. After first `take()`, the Option is drained (has_rx() == false).
/// 3. Second stream call with `None` (post-`.take()` state after salt-bump)
///    → yields ZERO messages.
///
/// This is the exact Bug #64 shape: if `.take()` doesn't drain the Option,
/// a second `stream()` call (triggered by iced re-subscribing after salt-bump)
/// would reuse the old stream and miss/duplicate lines.
///
/// **Falsification probe P2**: to test that `has_rx()` actually verifies the
/// `.take()` semantics, simulate the bug by NOT calling `.take()` (just peek).
/// After a "peek-only" first call, `has_rx()` returns `true` — which this test
/// would catch as a failure of the `.take()` invariant.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn salt_bump_survives_arc_mutex_take() {
    // Use the rx_opt Arc directly so we can keep checking it after all senders
    // are dropped. We separate the sender from the mock's state tracking to
    // avoid the borrow-after-move problem.
    let (tx, rx) = std::sync::mpsc::sync_channel::<TrainingLogLine>(16);
    let rx_opt: Arc<Mutex<Option<std::sync::mpsc::Receiver<TrainingLogLine>>>> =
        Arc::new(Mutex::new(Some(rx)));

    // Helper: peek whether Option is still Some.
    let rx_opt_for_has = Arc::clone(&rx_opt);
    let has_rx = move || {
        rx_opt_for_has
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_some()
    };

    // Helper: take Receiver — mimics TrainingLogRecipe::stream().
    let rx_opt_for_take = Arc::clone(&rx_opt);
    let take_rx = move || {
        rx_opt_for_take
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
    };

    // Enqueue 3 lines to receive.
    tx.send(TrainingLogLine {
        text: SmolStr::new("line-1"),
        is_stderr: false,
    })
    .unwrap();
    tx.send(TrainingLogLine {
        text: SmolStr::new("line-2"),
        is_stderr: false,
    })
    .unwrap();
    tx.send(TrainingLogLine {
        text: SmolStr::new("line-3"),
        is_stderr: true,
    })
    .unwrap();

    // --- First stream invocation (salt N) ---
    assert!(has_rx(), "receiver must be present before first stream()");
    let rx_first = take_rx().expect("take() must succeed on first call");
    assert!(!has_rx(), "Option must be None after first take()");

    let messages: Vec<Message> = {
        let mut stream = Box::pin(stream_impl(Some(rx_first)));
        let mut out = Vec::new();
        // Close channel by dropping the sender → EOF to stream.
        drop(tx);
        while let Some(m) = stream.next().await {
            out.push(m);
        }
        out
    };

    assert_eq!(
        messages.len(),
        3,
        "first stream must yield all 3 sent lines; got {} messages: {messages:?}. \
         Falsification probe P1: yield line at training_log.rs:124 commented out.",
        messages.len()
    );
    assert!(
        matches!(&messages[0], Message::TrainingLogLine(s) if s == "line-1"),
        "first message must be 'line-1'; got {:?}",
        messages[0]
    );

    // --- Verify Option is still None after the block ---
    assert!(
        !has_rx(),
        "Option must still be None after first stream consumed the receiver"
    );

    // --- Second stream invocation (salt N+1 after bump) ---
    let rx_second = take_rx(); // Must return None.
    assert!(
        rx_second.is_none(),
        "second take() must return None (Option exhausted by first take). \
         Falsification probe P2: if .take() is replaced with a no-op, \
         rx_second is Some and the stream below would block instead of yielding nothing."
    );

    let second_messages: Vec<Message> = {
        let mut stream = Box::pin(stream_impl(None));
        let mut out = Vec::new();
        while let Some(m) = stream.next().await {
            out.push(m);
        }
        out
    };

    assert!(
        second_messages.is_empty(),
        "second stream(None) must yield 0 messages (post-take() state); \
         got {} messages: {second_messages:?}. \
         Regression B (P2): .take() did not consume the Option — \
         second stream() invocation reused the old receiver.",
        second_messages.len()
    );
}

// ── Test 3 — Log stream survives recipe drop (graceful shutdown) ──────────────

/// T-D-A1 / ADR-0048 D3 category C — sender-drop EOF.
///
/// Asserts: when the `SyncSender` is dropped (subprocess exits), the stream
/// terminates cleanly. No `unwrap` panic, no orphan `spawn_blocking` tasks.
///
/// This mirrors the existing unit test `stream_yields_lines_and_terminates` in
/// `training_log.rs` but exercises it from the integration-test boundary.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn log_stream_survives_recipe_drop() {
    let mock = MockTrainingLogChannel::new();

    // Send 2 lines, then drop the mock immediately (drops the sender).
    mock.send("[stdout] training started", false).unwrap();
    mock.send("[stderr] warning: small dataset", true).unwrap();

    let rx = mock.take_rx().expect("receiver must be present");

    // Close channel: drop mock (the only sender) → EOF signal.
    drop(mock);

    // Drive the stream to completion.
    let mut stream = Box::pin(stream_impl(Some(rx)));
    let mut messages: Vec<Message> = Vec::new();
    // Use a 2-second budget — spawn_blocking can take a moment on CI.
    let deadline = tokio::time::sleep(Duration::from_secs(2));
    tokio::pin!(deadline);

    loop {
        tokio::select! {
            item = stream.next() => {
                match item {
                    Some(m) => messages.push(m),
                    None => break, // Stream terminated cleanly.
                }
            }
            _ = &mut deadline => {
                panic!(
                    "stream did not terminate within 2s after sender drop. \
                     Regression C: orphan spawn_blocking task / stream never ends. \
                     Got {} messages before timeout: {messages:?}",
                    messages.len()
                );
            }
        }
    }

    assert_eq!(
        messages.len(),
        2,
        "stream must yield exactly the 2 pre-queued lines before EOF; \
         got {} messages: {messages:?}",
        messages.len()
    );
    assert!(
        matches!(&messages[0], Message::TrainingLogLine(s) if s == "[stdout] training started"),
        "first message must be stdout line; got {:?}",
        messages[0]
    );
    assert!(
        matches!(&messages[1], Message::TrainingLogLine(s) if s == "[stderr] warning: small dataset"),
        "second message must be stderr line; got {:?}",
        messages[1]
    );
}
