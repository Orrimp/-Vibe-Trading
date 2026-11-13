//! Surface 1 — boundary tests for the `TrainingLogRecipe` stream path.
//!
//! lab-recipe-test-harness v0.2.0 Wave A (T-D-A1 / ADR-0048 D3); swept and
//! extended by story 2-63 (lab-recipe-test-harness v0.3) Wave E1.
//!
//! ## What this file tests
//!
//! The three regression categories from Bug #64, applied to `TrainingLogRecipe`.
//! `TrainingLogRecipe` is the **exact Bug #64 shape**: a blocking
//! `std::sync::mpsc::Receiver`, taken once out of an `Arc<Mutex<Option<_>>>` by
//! `<TrainingLogRecipe as Recipe>::stream`, re-identified per run by a salt, and
//! bridged to async by a dedicated std thread (`training-log-bridge`, the
//! `std::thread::Builder::new()` in `ui::lab::training_log::stream_impl`) that
//! forwards each line into a `futures` unbounded channel. Before bug-log `#196`
//! (`dcce75e2`) the bridge was `tokio::task::spawn_blocking`.
//!
//! **A — Sentinel emission timing**: a line buffered before the stream is built
//! arrives first, within 50 ms, while the sender is still alive (the stream is
//! live as soon as `Some(rx)` is handed over).
//!
//! **B — The take-once ownership hand-off**: the real `Recipe::stream` takes the
//! receiver out of the shared cell before it returns, its stream relays every
//! line, and a second recipe (the next run's salt) on the taken cell ENDS
//! without yielding. The seam's two branches are pinned beside it:
//! `stream_impl(Some(rx))` relays every line, `stream_impl(None)` ends at once.
//!
//! **C — Graceful shutdown (sender-drop EOF)**: when the last `SyncSender` is
//! dropped, the stream relays what was buffered and then ENDS. The end is the
//! proof the bridge thread returned: its closure owns the unbounded sender the
//! stream reads from, so the stream cannot end while the closure still runs.
//!
//! ## The seam and the real `Recipe::stream`
//!
//! Three tests drive `stream_impl`, the stream `<TrainingLogRecipe as
//! Recipe>::stream` returns once it has taken the receiver. They hand the seam
//! a receiver (or `None`) they built themselves, so they cannot see the
//! Recipe's own line: the take it runs BEFORE it returns (the `let rx_opt = {`
//! block, `.take()`). `training_log_recipe_stream_takes_receiver_once` drives
//! the real Recipe: its fields are `pub` (re-exported as
//! `ui::lab::training_log::TrainingLogRecipe`) and it ignores its
//! `EventStream`, so an empty one is faithful and no iced application is
//! needed. That test observes the take as the shared cell holding `None` right
//! after `stream()` returned, before the first poll.
//!
//! The off-runtime drain of the same Recipe on iced's executor is pinned
//! elsewhere: the lib's `lab::training_log::tests::recipe_stream_drains_without_a_tokio_runtime`
//! and story 2-63 Wave B's `training_log_recipe_drains_off_runtime`
//! (`crates/ui/tests/recipe_off_runtime_poll.rs`, B-P20..B-P22). Every test
//! here polls on a tokio worker (`#[tokio::test]`), so it is the on-runtime
//! twin, with strict ends.
//!
//! Not covered here: the salt's effect on the recipe id (`self.salt.hash(state);`;
//! no test here hashes the recipe, story 2-63 C2's
//! `salted_recipe_ids_change_with_salt` is planned for it); the
//! `let _guard = self.rt_handle.enter();` around the take (a std `Mutex` take
//! needs no tokio context, so removing the guard is unobservable; story 2-63
//! §9, K3 exclusion 1; a LEAKED guard, Wave B's B-P21, is red here too, see
//! below); the `is_stderr` flag
//! (`Message::TrainingLogLine` carries only the text, so it is never relayed);
//! the bridge's spawn-failure arm (`tracing::warn!(error = %e, "training-log
//! bridge thread could not be spawned; ...")`, which no test can induce); the
//! `update` arms for the message (`training_log_state_gating.rs`).
//!
//! ## The channel
//!
//! Every test sends over a std `sync_channel::<TrainingLogLine>(16)`.
//! Production's is `sync_channel::<TrainingLogLine>(256)` (`cockpit_live.rs`,
//! `trainer::spawn_training_run`); no test here buffers more than three lines,
//! so the depth is unobservable. Every send is checked (`SyncSender::send`
//! returns the failure): what a test counts as sent did enter the channel.
//!
//! ## Non-vacuity contract (bug-log #66 / story 2-63 AC3)
//!
//! Every test proves it RAN: the exact relayed lines in order, and a stream end
//! observed as `Ok(None)` inside a timeout that FAILS the test. No timeout is
//! ever accepted as "drained" or "yields nothing": [`next_or_fail`] panics on
//! elapse, [`drain_to_end`] is bounded, and [`assert_ends_within`] rejects
//! `Err(Elapsed)`. "The stream is live and nothing is buffered" is OBSERVED
//! with [`assert_pending`], never read off a counter the test increments
//! itself. Checks that hold by construction (the cell holding `Some(rx)` one
//! line after the test put it there; the test's own hand-off of its own
//! receiver) are not asserted. The claims come in two-sided pairs:
//! - over the sender's liveness: while a sender is alive the stream is live
//!   with nothing more buffered; once the last one drops, it ENDS;
//! - over the receiver's presence: `Some(rx)` relays every line, while `None`
//!   ends at once with nothing;
//! - over the cell's take state: the recipe that takes the receiver relays,
//!   while a second recipe on the taken cell is empty.
//!
//! ## T-T4 falsification probe table (story 2-63 AC4, Wave E1)
//!
//! Each probe is ONE edit to production code, located by symbol + the quoted
//! line text (line numbers rot). Procedure:
//!
//! 1. snapshot the file (`cp -p <file> <snap>`) and apply the edit;
//! 2. run `cargo test -p ui --test training_log_recipe_harness`. The named
//!    test(s) go RED;
//! 3. restore from the snapshot, THEN `touch <file>`. A restore that keeps the
//!    snapshot's older mtime (`cp -p`) leaves cargo's fingerprint "fresh": the
//!    next run compiles nothing and re-executes the MUTANT binary;
//! 4. re-run. The GREEN counts only if that run's output shows
//!    `Compiling ui` and the file's SHA-256 equals the pre-probe one.
//!
//! Probe ids are the Wave E1 series, shared across the seven v0.2
//! recipe-stream files E1 sweeps; this file's rows are E1-P14, E1-P15 and
//! E1-P16. All three edit `crates/ui/src/lab/training_log.rs`. E1-P15's OLD
//! (the four lines `self.rx` ... `.take()`) lies inside E1-P14's (the whole
//! `let rx_opt = {` block), so apply them one at a time, each to a byte-exact
//! HEAD copy. E1-P16 emits unused-import warnings (`SmolStr`, `StreamExt`)
//! while applied; never run clippy with a probe applied.
//!
//! | # | symbol (file) | quoted line today | mutation | expected RED |
//! |---|---|---|---|---|
//! | E1-P14 | `<TrainingLogRecipe as Recipe>::stream` (`crates/ui/src/lab/training_log.rs`; story-designated, wave row E1: "training_log.rs:83-88 take-removal") | the `let rx_opt = {` block (`let _guard = self.rt_handle.enter();`, `self.rx`, `.lock()`, `.unwrap_or_else(std::sync::PoisonError::into_inner)`, `.take()`, `};`) | replace the block with `let rx_opt: Option<std::sync::mpsc::Receiver<TrainingLogLine>> = None;` (the recipe never takes; its stream is `stream_impl(None)`) | `training_log_recipe_stream_takes_receiver_once`, at "training log recipe: `Recipe::stream` took the receiver (cell None right after stream() returns)" (the cell still holds `Some`) |
//! | E1-P15 | `<TrainingLogRecipe as Recipe>::stream` | `self.rx` / `.lock()` / `.unwrap_or_else(std::sync::PoisonError::into_inner)` / `.take()` (inside the `let rx_opt = {` block) | take, then re-arm the cell with a DEAD receiver: `let taken = cell.take(); *cell = Some(std::sync::mpsc::channel::<TrainingLogLine>().1); taken` | `training_log_recipe_stream_takes_receiver_once`, ONLY at "training log recipe: `Recipe::stream` took the receiver (cell None right after stream() returns)" (the cell holds the dead receiver). Past that line the test would stay green (reasoned): the taken receiver still relays, and the salt-2 recipe takes the dead receiver, whose sender is already dropped, so its bridge's `recv()` errs, the closure returns and that stream ENDS |
//! | E1-P16 | `ui::lab::training_log::stream_impl` (the old P1, re-cited onto the post-`#196` body) | `Box::pin(lines.map(` ... `))`, the last line of `stream_impl` | → `drop(lines);` then `Box::pin(futures::stream::empty())` (the relay yields nothing and ends at once) | all four tests: `sentinel_log_line_emitted_before_subprocess_spawn`, at `.expect("stream must not be empty when receiver has a pending message")`; `stream_impl_some_relays_every_line_none_relays_nothing`, at "training log: stream_impl(Some(rx)) relays every line in order, then the stream ENDS" (left `[]`); `log_stream_survives_recipe_drop`, at "stream must yield exactly the 2 pre-queued lines before EOF" (left `0`); `training_log_recipe_stream_takes_receiver_once`, at "training log recipe: the three lines are relayed in order, then the stream ENDS" (left `[]`) |
//!
//! The tests each probe leaves GREEN (reasoned): under E1-P14 and E1-P15, the
//! three `stream_impl` tests, which never call `Recipe::stream` and so cannot
//! see the take; under E1-P16, none (the probe is this file's positive
//! control). Within a test, the `None` half of
//! `stream_impl_some_relays_every_line_none_relays_nothing` holds under all
//! three probes; that test goes red under E1-P16 at its `Some` half.
//!
//! Before Wave E1, the v0.2 tests let two of the three through. E1-P14 and
//! E1-P15: all three green (reasoned): no test referenced `TrainingLogRecipe`
//! (the file imported only `use ui::lab::training_log::stream_impl;`), and the
//! old P2 probe's `salt_bump_survives_arc_mutex_take` took from a test-owned
//! cell through its own `take_rx` closure, so its "Option exhausted by first
//! take" asserts held by construction under any production edit. E1-P16: red
//! in all three (reasoned). The old P1 row cited a `yield` at
//! `training_log.rs:124` that no longer exists (`stream_impl` has no `yield`
//! since `dcce75e2`) and a test, `stream_yields_lines_in_order`, that never
//! existed in this file.
//!
//! Outside this target (reasoned, not measured here):
//! - the lib's `lab::training_log::tests::recipe_stream_drains_without_a_tokio_runtime`
//!   is red under E1-P14 (the recipe hands over `None`: `got.len()` is 0, not
//!   2) and E1-P16, and green under E1-P15 (it never inspects the cell, and
//!   the real receiver still relays); the lib's `stream_yields_lines_and_terminates`
//!   is red under E1-P16 only; run `cargo test -p ui --lib lab::training_log`;
//! - Wave B's `training_log_recipe_drains_off_runtime` is expected red under
//!   E1-P14 and E1-P15 (the orchestrator's Wave B gate (d): it reads
//!   `assert_taken(&built, "training log")`). Under E1-P15 the red is at that
//!   assert (the cell holds the dead receiver). Under E1-P14 it comes one line
//!   earlier, at `expect_parked` ("the stream ENDED on its first poll"),
//!   because `stream_impl(None)` ends at once; it is expected red under E1-P16
//!   at the same `expect_parked`;
//! - Wave B's B-P20 row names this file as a GREEN sibling; that stays true
//!   after E1: every test here polls on a tokio worker, which supplies the
//!   reactor B-P20's tokio timeout needs. Its B-P21 and B-P22 rows predate E1a
//!   and are now STALE for this file: both are MEASURED red here (orchestrator
//!   hand probes, 2026-11-13), in `training_log_recipe_stream_takes_receiver_once`
//!   only, 3 passed / 1 failed. The red is NOT at a designed assertion: the
//!   leaked `EnterGuard` trips tokio's own drop-order check after the body,
//!   "`EnterGuard` values dropped out of order" (tokio-1.52.3
//!   `runtime/context/current.rs:82`), because this test calls the real
//!   `Recipe::stream` on the thread whose `block_on` holds the outer guard.
//!
//! ## Coverage
//!
//! | origin | test | what it pins |
//! |---|---|---|
//! | v0.2 T-D-A1 A, bounded end | `sentinel_log_line_emitted_before_subprocess_spawn` | a line buffered before the stream is built is relayed first, within 50 ms; with the sender alive the stream is live with nothing more buffered; dropping the sender ENDS it with nothing more relayed (a bounded drain, not the old unbounded `while`) |
//! | v0.2 T-D-A1 B, renamed and slimmed (was `salt_bump_survives_arc_mutex_take`) | `stream_impl_some_relays_every_line_none_relays_nothing` | `stream_impl(Some(rx))`: exactly `line-1`, `line-2`, `line-3`, then the end (bounded drain); `stream_impl(None)`: `Ok(None)` within 200 ms. The old test-owned `has_rx`/`take_rx` closures and their asserts are gone |
//! | v0.2 T-D-A1 C | `log_stream_survives_recipe_drop` | the last sender dropped before the stream is built: exactly the two pre-queued lines, in order, then the end within 2 s |
//! | E1, the real P2 | `training_log_recipe_stream_takes_receiver_once` | the real `Recipe::stream` (salt 1): the cell holds `None` right after `stream()` returned, before the first poll; the three lines are relayed in order, then the end; a salt-2 recipe on the taken cell: `Ok(None)` within 200 ms |
//!
//! ## `#[cfg(feature = "live")]` gate
//!
//! `stream_impl` and `TrainingLogRecipe` are `#[cfg(feature = "live")]`. `live`
//! is a default feature of `ui` (`default = ["live", ...]`), so
//! `cargo test -p ui --test training_log_recipe_harness` runs all four tests.
//! Under `--no-default-features` this file compiles to ZERO tests: a build
//! that skips them, never a pass.

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures::stream::BoxStream;
use futures::{FutureExt, StreamExt};
use iced::advanced::subscription::{EventStream, Recipe};
use smol_str::SmolStr;
use tokio::time::timeout;
use ui::lab::trainer::TrainingLogLine;
use ui::lab::training_log::{TrainingLogRecipe, stream_impl};
use ui::state::Message;

// ── Helpers ─────────────────────────────────────────────────────────────────

/// The depth of every test channel here (production's is 256, see the module
/// doc); no test buffers more than three lines.
const CHANNEL_DEPTH: usize = 16;

/// The three lines the take-once and the `Some`/`None` tests send, in order:
/// `(text, is_stderr)`. The third is on stderr; the flag is not relayed
/// (`Message::TrainingLogLine` carries only the text).
const LINES: [(&str, bool); 3] = [("line-1", false), ("line-2", false), ("line-3", true)];

/// The texts of [`LINES`], in send order: what a faithful relay yields. Derived
/// from the test's own input, never from the stream under test.
fn sent_texts() -> Vec<&'static str> {
    LINES.iter().map(|&(text, _)| text).collect()
}

/// Send every line of [`LINES`], each send CHECKED (`SyncSender::send` returns
/// the failure; the channel is empty and deep enough, so none blocks).
fn send_lines(tx: &std::sync::mpsc::SyncSender<TrainingLogLine>, what: &str) {
    for (text, is_stderr) in LINES {
        tx.send(TrainingLogLine {
            text: SmolStr::new(text),
            is_stderr,
        })
        .unwrap_or_else(|e| panic!("{what}: the checked send of {text:?} failed: {e}"));
    }
}

/// The text of every relayed message, in order. Any message other than
/// `TrainingLogLine` fails the test, naming it.
fn line_texts(msgs: &[Message], what: &str) -> Vec<String> {
    msgs.iter()
        .map(|msg| match msg {
            Message::TrainingLogLine(text) => text.to_string(),
            other => panic!("{what}: expected TrainingLogLine, got {other:?}"),
        })
        .collect()
}

/// The iced runtime event stream a `Recipe::stream` receives; the training-log
/// recipe ignores it, so an empty one is faithful.
fn empty_event_stream() -> EventStream {
    Box::pin(futures::stream::empty::<iced::advanced::subscription::Event>())
}

/// Await the next item, FAILING the test if nothing resolves in `secs`.
///
/// The returned `Option` is the stream's own verdict: `Some(msg)` = a relayed
/// message, `None` = the stream ENDED. A timeout is never read as either
/// (the accepting `Err(_) => break` shape is exactly the bug-log #66 vacuity).
async fn next_or_fail(
    stream: &mut BoxStream<'static, Message>,
    secs: u64,
    what: &str,
) -> Option<Message> {
    timeout(Duration::from_secs(secs), stream.next())
        .await
        .unwrap_or_else(|_| panic!("timed out after {secs}s waiting for {what}"))
}

/// Assert the stream ENDS: `Ok(None)` within `ms`. A timeout (`Err(Elapsed)`,
/// a stream that parks) fails, and so does any item.
async fn assert_ends_within(stream: &mut BoxStream<'static, Message>, ms: u64, what: &str) {
    let end = timeout(Duration::from_millis(ms), stream.next()).await;
    assert!(
        matches!(end, Ok(None)),
        "{what} (Ok(None) within {ms} ms); got {end:?}"
    );
}

/// Assert the stream is LIVE with NOTHING buffered: a single poll returns
/// `Pending` (the relay is parked on its receiver). An observation, not a
/// count: it fails if the stream already ENDED (an empty or stopped relay) and
/// if it yields an item that was never sent.
fn assert_pending(stream: &mut BoxStream<'static, Message>, what: &str) {
    match stream.next().now_or_never() {
        None => {}
        Some(None) => panic!("{what}: the stream already ENDED (expected it live on its receiver)"),
        Some(Some(msg)) => panic!("{what}: an unexpected extra message was buffered: {msg:?}"),
    }
}

/// Drain a stream whose senders are already dropped: every item until the
/// stream END (`Ok(None)`), each step under a failing timeout. Bounded so a
/// runaway stream fails instead of spinning.
async fn drain_to_end(stream: &mut BoxStream<'static, Message>, what: &str) -> Vec<Message> {
    let mut out = Vec::new();
    loop {
        match next_or_fail(stream, 2, what).await {
            Some(msg) => out.push(msg),
            None => return out,
        }
        assert!(out.len() <= 64, "{what}: stream did not end after 64 items");
    }
}

// ── MockTrainingLogChannel ────────────────────────────────────────────────────

/// Test double for the `std::sync::mpsc` channel that the real
/// `spawn_training_run` produces.
///
/// Holds the `SyncSender<TrainingLogLine>` so test code can drive lines into
/// the stream, and the `Receiver` in an `Arc<Mutex<Option<_>>>` shaped like
/// production's cell. `take_rx()` is the TEST's own hand-off of that receiver
/// to `stream_impl`; it is not production's take, which only
/// `training_log_recipe_stream_takes_receiver_once` exercises (through the
/// real `Recipe::stream`).
///
/// Design: per-Recipe-specific (D-V0.2.0-1 rationale — no shared trait;
/// mirrors `MockLabYahooBarSource` in `spawn_lab_run_yahoo_harness.rs`).
struct MockTrainingLogChannel {
    /// The sender half. Tests drive this; the channel closes when ALL sender
    /// clones are dropped.
    pub tx: std::sync::mpsc::SyncSender<TrainingLogLine>,
    /// The receiver, in a cell shaped like the one production stores on
    /// `AppState`.
    pub rx_opt: Arc<Mutex<Option<std::sync::mpsc::Receiver<TrainingLogLine>>>>,
}

impl MockTrainingLogChannel {
    /// Depth [`CHANNEL_DEPTH`] (16), as the lib's
    /// `stream_yields_lines_and_terminates` unit test uses.
    fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::sync_channel::<TrainingLogLine>(CHANNEL_DEPTH);
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

    /// Take the `Receiver` out of the mock's cell, to hand it to
    /// `stream_impl`. Returns `None` if it was already taken.
    fn take_rx(&self) -> Option<std::sync::mpsc::Receiver<TrainingLogLine>> {
        self.rx_opt
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
    }
}

// ── Test 1 — Sentinel log line emitted before subprocess spawn ─────────────────

/// T-D-A1 / ADR-0048 D3 category A — sentinel emission timing.
///
/// Asserts: the first `Message::TrainingLogLine` arrives within 50 ms of the
/// stream being built over a receiver that already holds the line (i.e. the
/// `stream_impl` path is live immediately on construction; no warm-up delay).
/// Two-sided over the sender's liveness: with the sender alive, the stream is
/// then live with nothing more buffered ([`assert_pending`]); once the mock
/// (the only sender) drops, a bounded drain finds the END and nothing more.
///
/// Analogue of v0.1.0 Test 1 (`sentinel_fires_before_preload_await`) applied
/// to `TrainingLogRecipe`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sentinel_log_line_emitted_before_subprocess_spawn() {
    let mock = MockTrainingLogChannel::new();

    // Pre-load a sentinel line BEFORE taking the receiver — simulates the
    // in-memory buffer being populated before the Recipe stream is started.
    mock.send("[info] epoch 1/30 complete", false)
        .expect("training log: the sentinel send fits (an empty channel of 16, receiver alive)");

    let rx = mock
        .take_rx()
        .expect("receiver must be present before first stream call");

    let start = Instant::now();

    // Start the stream.
    let mut stream = stream_impl(Some(rx));

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

    assert_pending(
        &mut stream,
        "training log: with the sender alive the stream is live with nothing more buffered",
    );

    // Cleanly terminate: drop the mock (the only sender) so the stream ends.
    drop(mock);
    let what = "training log: dropping the last sender ENDS the stream with nothing more relayed";
    let rest = drain_to_end(&mut stream, what).await;
    assert!(rest.is_empty(), "{what}; got {rest:?}");
}

// ── Test 2 — The seam's two branches: Some relays, None ends ─────────────────

/// T-D-A1 / ADR-0048 D3 category B, the seam half: `stream_impl`'s two
/// branches, pinned by what comes OUT of the stream. Two-sided over the
/// receiver's presence:
/// - `Some(rx)` with three lines buffered and the sender then dropped: exactly
///   `line-1`, `line-2`, `line-3`, in send order, then the END (a bounded
///   drain; a short relay, an extra item and a stream that parks each fail);
/// - `None`, what a `stream()` on an already taken cell hands over: the stream
///   ENDS at once without yielding (`Ok(None)` within 200 ms; a timeout fails).
///
/// The take itself is production's, in `Recipe::stream`; it is pinned by
/// `training_log_recipe_stream_takes_receiver_once`, not here.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stream_impl_some_relays_every_line_none_relays_nothing() {
    let (tx, rx) = std::sync::mpsc::sync_channel::<TrainingLogLine>(CHANNEL_DEPTH);
    send_lines(&tx, "training log");

    let mut some = stream_impl(Some(rx));
    // Close the channel by dropping the only sender → EOF to the stream.
    drop(tx);
    let what =
        "training log: stream_impl(Some(rx)) relays every line in order, then the stream ENDS";
    let got = drain_to_end(&mut some, what).await;
    assert_eq!(line_texts(&got, what), sent_texts(), "{what}");

    let mut none = stream_impl(None);
    assert_ends_within(
        &mut none,
        200,
        "training log: stream_impl(None) ENDS at once without yielding",
    )
    .await;
}

// ── Test 3 — Log stream survives recipe drop (graceful shutdown) ──────────────

/// T-D-A1 / ADR-0048 D3 category C — sender-drop EOF.
///
/// Asserts: when the `SyncSender` is dropped (subprocess exits), the stream
/// terminates cleanly, with no `unwrap` panic. The END proves that the
/// `training-log-bridge` closure DROPPED the unbounded sender the stream reads
/// from. It does not prove that the closure RETURNED: a bridge that drops the
/// sender and then blocks forever (an orphaned OS thread per training run)
/// passes this test and every other one. That was measured as LT-OWN-2 in story
/// 2-63's E1a falsification. Joining the bridge needs a `JoinHandle` seam in
/// production, which this test-only wave does not add.
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
    // Use a 2-second budget — the bridge thread can take a moment to start on CI.
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
                     Regression C: the training-log-bridge thread never returned / stream never ends. \
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

// ── Test 4 — The real Recipe::stream takes the receiver once ──────────────────

/// T-D-A1 / ADR-0048 D3 category B, the Recipe half (story 2-63 Wave E1, the
/// real P2): the real `<TrainingLogRecipe as Recipe>::stream` (salt 1), built
/// as `cockpit_live.rs` builds it, on a cell holding a receiver with three
/// lines buffered.
///
/// - The take is EAGER: right after `stream()` returned, before the stream is
///   first polled, the shared cell holds `None`.
/// - Dropping the sender: the three lines are relayed in order, then the
///   stream ENDS (a bounded drain).
/// - A second recipe (salt 2, as the next `TrainingPressed` builds it) on the
///   same, already taken cell ENDS without yielding: `Ok(None)` within 200 ms.
///
/// Two-sided over the take state: the recipe that took the receiver relays
/// every line, while the recipe that found the cell empty relays nothing. The
/// cell is not checked for `Some` before `stream()`: the test puts `Some(rx)`
/// there one line earlier, so that would hold by construction.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn training_log_recipe_stream_takes_receiver_once() {
    let (tx, rx) = std::sync::mpsc::sync_channel::<TrainingLogLine>(CHANNEL_DEPTH);
    let cell = Arc::new(Mutex::new(Some(rx)));
    send_lines(&tx, "training log recipe");

    let mut stream = Box::new(TrainingLogRecipe {
        rt_handle: tokio::runtime::Handle::current(),
        rx: Arc::clone(&cell),
        salt: 1,
    })
    .stream(empty_event_stream());

    assert!(
        cell.lock().unwrap().is_none(),
        "training log recipe: `Recipe::stream` took the receiver (cell None right after stream() returns)"
    );

    drop(tx);
    let what = "training log recipe: the three lines are relayed in order, then the stream ENDS";
    let got = drain_to_end(&mut stream, what).await;
    assert_eq!(line_texts(&got, what), sent_texts(), "{what}");

    let mut second = Box::new(TrainingLogRecipe {
        rt_handle: tokio::runtime::Handle::current(),
        rx: cell,
        salt: 2,
    })
    .stream(empty_event_stream());
    assert_ends_within(
        &mut second,
        200,
        "training log recipe: a second recipe (salt 2) on the taken cell ENDS without yielding",
    )
    .await;
}
