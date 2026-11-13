//! Relay tests for `LabProgressRecipe`, the take-once recipe that hands the
//! backtest's progress receiver to iced (Bug #63 / lab-end-to-end-v2 T-AR-6).
//!
//! `ui::lab::progress::stream_impl` (`crates/ui/src/lab/progress.rs`) is the
//! stream `<LabProgressRecipe as Recipe>::stream` returns once it has taken the
//! receiver out of its shared `Arc<Mutex<Option<Receiver<Progress>>>>`. It has
//! two branches, and these tests pin each by what comes OUT of the stream:
//! - `Some(rx)`: one `Message::LabRunProgress` per progress event, fields
//!   unchanged, in send order, while a sender is alive; then exactly one
//!   `Message::LabRunProgressDone` once every sender is gone; then the end;
//! - `None` (the receiver was already taken): the stream ENDS at once with
//!   nothing relayed, not even `LabRunProgressDone`. This is the Bug #63
//!   silent path: a second `stream()` on a taken cell feeds the UI nothing, so
//!   `run_progress` never moves and the bar stays frozen.
//!
//! ## The real `Recipe::stream`
//!
//! The three `stream_impl` tests hand the seam a receiver (or `None`) they
//! built themselves, so they cannot see the Recipe's own line: the take it
//! runs BEFORE it returns (the `let rx_opt = {` block, `.take()`, in
//! `<LabProgressRecipe as Recipe>::stream`). Two tests drive the real Recipe.
//! Its fields are `pub` and it ignores its `EventStream`, so an empty one is
//! faithful and no iced application is needed:
//! - T-PB-1d, `lab_progress_recipe_stream_end_to_end`: the take is OBSERVED as
//!   the shared cell holding `None` right after `stream()` returned, before the
//!   first poll; then the taken receiver relays, then `Done`, then the end;
//! - `lab_progress_second_recipe_on_a_taken_cell_ends_without_yielding`: a
//!   second recipe (salt 2, as the next run builds it) on the same, already
//!   taken cell ENDS without yielding, while the first recipe's stream relays.
//!
//! The off-runtime drain of the same Recipe on iced's executor is story 2-63
//! Wave B's `lab_progress_recipe_drains_off_runtime`
//! (`crates/ui/tests/recipe_off_runtime_poll.rs`, probes B-P7, B-P18). T-PB-1d
//! is its on-runtime twin, with a strict end.
//!
//! Not covered here: the salt's effect on the recipe id (no test hashes
//! `LabProgressRecipe`; story 2-63 C2's `salted_recipe_ids_change_with_salt`
//! is planned for it); the `let _guard = self.rt_handle.enter();` around the
//! take (a std `Mutex` take needs no tokio context, so removing the guard is
//! unobservable; story 2-63 §9, K3 exclusion 1); the `update` arms for these
//! messages (`lab_run_cancel.rs`).
//!
//! ## The channel
//!
//! Every test sends over a raw `tokio::sync::mpsc::channel::<Progress>(8)`:
//! the type and capacity `backtest::progress::progress_pair` builds
//! (`let (tx, rx) = tokio::sync::mpsc::channel(8);`). Production's
//! `ProgressSender::try_send` is lossy and returns `()`
//! (`let _ = tx.try_send(progress);`), so a send it dropped would be invisible
//! to the test. Here every `try_send` is checked with `.expect(..)`: what a
//! test counts as sent did enter the channel.
//!
//! ## Non-vacuity contract (bug-log #66 / story 2-63 AC3)
//!
//! Every test proves it RAN: the exact relayed items in order, and a stream end
//! observed as `Ok(None)` inside a timeout that FAILS the test. No timeout is
//! ever accepted as "drained" or "yields nothing": [`next_or_fail`] panics on
//! elapse, and [`assert_ends_within`] rejects `Err(Elapsed)`. "The stream is
//! live and nothing is buffered" is OBSERVED with [`assert_pending`], never
//! read off a counter the test increments itself. Checks that hold by
//! construction (the cell holding `Some(rx)` one line after the test put it
//! there) are not asserted. The claims come in two-sided pairs:
//! - over the sender's liveness: while a sender is alive the stream is live
//!   with no `Done` buffered; once the last one drops, `Done`, then the end;
//! - over the receiver's presence: `Some(rx)` relays and ends with `Done`,
//!   while `None` ends at once with nothing;
//! - over the cell's take state: the recipe that takes the receiver relays,
//!   while a second recipe on the taken cell is empty.
//!
//! ## T-T4 falsification probe table (story 2-63 AC4, Wave E1)
//!
//! Each probe is ONE edit to production code, located by symbol + the quoted
//! line text (line numbers rot). Procedure:
//!
//! 1. snapshot the file (`cp -p <file> <snap>`) and apply the edit;
//! 2. run `cargo test -p ui --test lab_progress_recipe_stream`. The named
//!    test(s) go RED;
//! 3. restore from the snapshot, THEN `touch <file>`. A restore that keeps the
//!    snapshot's older mtime (`cp -p`) leaves cargo's fingerprint "fresh": the
//!    next run compiles nothing and re-executes the MUTANT binary;
//! 4. re-run. The GREEN counts only if that run's output shows
//!    `Compiling ui` and the file's SHA-256 equals the pre-probe one.
//!
//! Probe ids are the Wave E1 series, shared across the seven v0.2
//! recipe-stream files E1 sweeps; this file's rows are E1-P7, E1-P8, E1-P9,
//! E1-P10 and E1-P29. All five edit `crates/ui/src/lab/progress.rs`. E1-P8,
//! E1-P9 and E1-P29 share an OLD prefix (E1-P29's OLD is the first line of
//! E1-P8's, which is the start of E1-P9's), so apply them one at a time, each
//! to a byte-exact HEAD copy.
//!
//! | # | symbol (file) | quoted line today | mutation | expected RED |
//! |---|---|---|---|---|
//! | E1-P7 | `ui::lab::progress::stream_impl`, the `Some(rx)` loop (`crates/ui/src/lab/progress.rs`) | `while let Some(progress) = rx.recv().await {` | `while` → `if` (the relay yields one progress, then `Done` with the sender still alive) | `stream_impl_multiple_progress_events_in_order`, at "lab progress: every sent progress is relayed in order, then Done, then the end" (left `[Progress { current_bar: 128, .. }, Done]`); `stream_impl_yields_progress_then_done`, at its `assert_pending`, "lab progress: LabRunProgressDone must not arrive while a sender is alive" ("an unexpected extra message was buffered: LabRunProgressDone") |
//! | E1-P8 | `ui::lab::progress::stream_impl`, after the `Some(rx)` loop | `yield Message::LabRunProgressDone;` (followed by the `}` that closes `if let Some(mut rx) = rx_opt {`) | append `std::future::pending::<()>().await;` after it (the relay parks after `Done` instead of ending) | `stream_impl_yields_progress_then_done`, at "lab progress: the stream ENDS after LabRunProgressDone" (`Err(Elapsed)`); `stream_impl_multiple_progress_events_in_order`, whose drain times out after `Done` ("timed out after 2s waiting for lab progress: every sent progress is relayed in order, then Done, then the end"); `lab_progress_recipe_stream_end_to_end`, at "lab progress recipe: the stream ENDS after LabRunProgressDone" (`Err(Elapsed)`) |
//! | E1-P9 | `ui::lab::progress::stream_impl`, the `None` branch | `// If rx_opt was None: stream yields nothing (double-stream() call` (right after the `}` that closes `if let Some(mut rx) = rx_opt {`) | give the `if let` an `else { std::future::pending::<()>().await; }` (the `None` branch parks instead of ending) | `stream_impl_none_rx_yields_nothing`, at "lab progress: stream_impl(None) ENDS at once without yielding" (`Err(Elapsed)`); `lab_progress_second_recipe_on_a_taken_cell_ends_without_yielding`, at "lab progress recipe: a second recipe on the taken cell ENDS without yielding" (`Err(Elapsed)`) |
//! | E1-P10 | `<LabProgressRecipe as Recipe>::stream` | the `let rx_opt = {` block (`let _guard = self.rt_handle.enter();` ... `.take()`) and `stream_impl(rx_opt)` | take lazily: `let cell = std::sync::Arc::clone(&self.rx);` and return a `stream!` that takes from `cell` at its first poll, then relays `stream_impl(rx_opt)` (Wave B's B-P18, the same edit) | `lab_progress_recipe_stream_end_to_end`, at "lab progress recipe: `Recipe::stream` took the receiver before the first poll" (the cell still holds `Some`) |
//! | E1-P29 | `ui::lab::progress::stream_impl` (story-designated, wave row E1: "progress.rs:107") | `yield Message::LabRunProgressDone;` | delete the line | `stream_impl_yields_progress_then_done`, at "lab progress: dropping the last sender relays LabRunProgressDone" (left `None`); `stream_impl_multiple_progress_events_in_order`, at "lab progress: every sent progress is relayed in order, then Done, then the end" (the three progress items, no `Done`); `lab_progress_recipe_stream_end_to_end`, at "lab progress recipe: dropping the last sender relays LabRunProgressDone" (left `None`) |
//!
//! The tests each probe leaves GREEN are the other side of its pair (reasoned):
//! under E1-P7 the recipe tests send one progress and look at nothing between
//! it and the drop, after which `Done` comes next anyway; under E1-P8 and
//! E1-P29 the two `None`-path tests never reach the end of the `Some(rx)`
//! branch; under E1-P9 no `Some(rx)` test takes the `None` branch; under
//! E1-P10 only T-PB-1d reads the cell before the first poll (the second-recipe
//! test's first stream takes at its first poll, before the second recipe is
//! built, so the second still finds the cell empty).
//!
//! Before Wave E1, the v0.2 tests let four of the five through. E1-P7: all
//! four green (reasoned): T-PB-1c's `None => break,` with a non-empty check
//! accepted `[128, Done]`, and T-PB-1a only checked that `Done` came after the
//! drop. E1-P8: green (reasoned): T-PB-1a accepted the timeout after `Done`
//! (`Err(_) => {} // timeout = stream open but no data; acceptable`), and
//! T-PB-1c broke on `Done`. E1-P9: green (reasoned): T-PB-1b accepted the
//! timeout (`Err(_timeout) => {`). E1-P10: green (measured by Wave B, its
//! B-P18 row): T-PB-1d read the cell only after the first message, when even a
//! lazy take has run. E1-P29: red in T-PB-1a and T-PB-1d, green in T-PB-1c
//! (reasoned): its `None => break,` accepted a missing `Done`.
//!
//! Outside this target (reasoned, not measured here): Wave B's
//! `lab_progress_recipe_drains_off_runtime` is red under E1-P10 (measured
//! there as B-P18), and expected red under E1-P7 (`Done` is relayed after the
//! first bar, with a sender alive), E1-P8 (its close tail never ends) and
//! E1-P29 (its tail is `[]`, not `[Done]`); it never builds a `None` stream, so
//! E1-P9 leaves it green. Wave B's B-P18 row still names this file as its
//! green sibling ("its end-to-end test reads the cell only after the first
//! message"); that is no longer true after E1. `cockpit_live_lab_run_smoke.rs`
//! also drives `stream_impl`, but its drains stop at
//! `Some(Message::LabRunProgressDone) | None => break`; it is not reasoned
//! about here. `progress.rs` has no lib unit tests.
//!
//! ## Coverage
//!
//! | origin | test | what it pins |
//! |---|---|---|
//! | v0.2 T-PB-1a, strict | `stream_impl_yields_progress_then_done` | one progress relayed with its three fields unchanged; with the sender alive the stream is live with nothing buffered (no `Done`); dropping the sender relays `Done`, then `Ok(None)` within 200 ms |
//! | v0.2 T-PB-1b, strict | `stream_impl_none_rx_yields_nothing` | `stream_impl(None)`: `Ok(None)` within 300 ms, nothing relayed |
//! | v0.2 T-PB-1c, strict | `stream_impl_multiple_progress_events_in_order` | three progress events buffered before the drop: exactly bars 128, 256, 384 (all three fields), then `Done`, then the end |
//! | v0.2 T-PB-1d, take checked before the first poll | `lab_progress_recipe_stream_end_to_end` | the real `Recipe::stream` (salt 1): the cell holds `None` right after `stream()` returned; a progress sent then is relayed; the drop relays `Done`, then `Ok(None)` within 200 ms |
//! | E1 | `lab_progress_second_recipe_on_a_taken_cell_ends_without_yielding` | recipe salt 1 takes the receiver and relays bar 100 (the live side); recipe salt 2 on the same cell: `Ok(None)` within 300 ms |

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::stream::BoxStream;
use futures::{FutureExt, StreamExt};
use iced::advanced::subscription::{EventStream, Recipe};
use tokio::sync::mpsc;
use tokio::time::timeout;

use backtest::progress::Progress;
use ui::lab::progress::{LabProgressRecipe, stream_impl};
use ui::state::Message;

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Production's channel depth: `backtest::progress::progress_pair` builds
/// `tokio::sync::mpsc::channel(8)`. No test here buffers more than three.
const PROGRESS_DEPTH: usize = 8;

/// A raw progress channel of production's type and depth. Raw, not
/// `progress_pair`, because `ProgressSender::try_send` swallows a failed send;
/// the raw sender's `try_send` returns it, so every send here is checked.
fn progress_channel() -> (mpsc::Sender<Progress>, mpsc::Receiver<Progress>) {
    mpsc::channel(PROGRESS_DEPTH)
}

/// The iced runtime event stream a `Recipe::stream` receives; the lab-progress
/// recipe ignores it, so an empty one is faithful.
fn empty_event_stream() -> EventStream {
    Box::pin(futures::stream::empty::<iced::advanced::subscription::Event>())
}

/// A relayed message, projected so an assertion can compare it (`Message` and
/// `Progress` have no `PartialEq`).
#[derive(Debug, PartialEq, Eq)]
enum Relayed {
    Progress {
        current_bar: usize,
        total_bars: usize,
        elapsed_ms: u64,
    },
    Done,
}

/// Project a relayed message. Any message other than `LabRunProgress` or
/// `LabRunProgressDone` fails the test, naming it.
fn relayed(msg: &Message, what: &str) -> Relayed {
    match msg {
        Message::LabRunProgress(p) => Relayed::Progress {
            current_bar: p.current_bar,
            total_bars: p.total_bars,
            elapsed_ms: p.elapsed_ms,
        },
        Message::LabRunProgressDone => Relayed::Done,
        other => panic!("{what}: expected LabRunProgress or LabRunProgressDone, got {other:?}"),
    }
}

/// The next relayed item under a 2 s [`next_or_fail`], projected: `Some(item)`
/// for a relayed message, `None` if the stream ENDED.
async fn next_relayed(stream: &mut BoxStream<'static, Message>, what: &str) -> Option<Relayed> {
    next_or_fail(stream, 2, what)
        .await
        .map(|msg| relayed(&msg, what))
}

/// Await the next item, FAILING the test if nothing resolves in `secs`.
///
/// The returned `Option` is the stream's own verdict: `Some(msg)` = a relayed
/// message, `None` = the stream ENDED. A timeout is never read as either
/// (the accepting `Err(_) => {}` shape is exactly the bug-log #66 vacuity).
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

// ── stream_impl ─────────────────────────────────────────────────────────────

/// T-PB-1a, strict: `stream_impl(Some(rx))` relays one progress event with its
/// three fields unchanged. Two-sided over the sender's liveness: while the
/// sender is alive the stream is live with nothing buffered (no
/// `LabRunProgressDone` yet); dropping it relays `LabRunProgressDone`, and the
/// stream then ENDS (`Ok(None)` within 200 ms, never a timeout).
#[tokio::test]
async fn stream_impl_yields_progress_then_done() {
    let (tx, rx) = progress_channel();
    let mut stream = stream_impl(Some(rx));

    tx.try_send(Progress {
        current_bar: 100,
        total_bars: 720,
        elapsed_ms: 1500,
    })
    .expect("lab progress: the send fits (an empty channel of 8, its receiver alive)");
    let what = "lab progress: the first progress is relayed with its fields unchanged";
    assert_eq!(
        next_relayed(&mut stream, what).await,
        Some(Relayed::Progress {
            current_bar: 100,
            total_bars: 720,
            elapsed_ms: 1500,
        }),
        "{what}"
    );

    assert_pending(
        &mut stream,
        "lab progress: LabRunProgressDone must not arrive while a sender is alive",
    );

    drop(tx);
    let what = "lab progress: dropping the last sender relays LabRunProgressDone";
    assert_eq!(
        next_relayed(&mut stream, what).await,
        Some(Relayed::Done),
        "{what}"
    );
    assert_ends_within(
        &mut stream,
        200,
        "lab progress: the stream ENDS after LabRunProgressDone",
    )
    .await;
}

/// T-PB-1b, strict: `stream_impl(None)`, what a `stream()` on an already taken
/// cell hands over, ENDS at once without yielding: `Ok(None)` within 300 ms.
/// No `LabRunProgress`, no `LabRunProgressDone`, and never a stream that parks
/// (a timeout fails). This is the Bug #63 silent path, pinned as an end.
///
/// Its pair over the receiver's presence is `stream_impl_yields_progress_then_done`.
#[tokio::test]
async fn stream_impl_none_rx_yields_nothing() {
    let mut stream = stream_impl(None);
    assert_ends_within(
        &mut stream,
        300,
        "lab progress: stream_impl(None) ENDS at once without yielding",
    )
    .await;
}

/// T-PB-1c, strict: three progress events buffered before the last sender
/// drops are relayed exactly, in send order, with every field unchanged; then
/// one `LabRunProgressDone`; then the end. A missing `Done`, a short relay and
/// a stream that parks after `Done` each fail.
#[tokio::test]
async fn stream_impl_multiple_progress_events_in_order() {
    let (tx, rx) = progress_channel();
    let mut stream = stream_impl(Some(rx));

    for (current_bar, elapsed_ms) in [(128, 1280), (256, 2560), (384, 3840)] {
        tx.try_send(Progress {
            current_bar,
            total_bars: 720,
            elapsed_ms,
        })
        .expect("lab progress: the send fits (three events in a channel of 8, receiver alive)");
    }
    drop(tx);

    let what = "lab progress: every sent progress is relayed in order, then Done, then the end";
    let got: Vec<Relayed> = drain_to_end(&mut stream, what)
        .await
        .iter()
        .map(|msg| relayed(msg, what))
        .collect();
    assert_eq!(
        got,
        [
            Relayed::Progress {
                current_bar: 128,
                total_bars: 720,
                elapsed_ms: 1280,
            },
            Relayed::Progress {
                current_bar: 256,
                total_bars: 720,
                elapsed_ms: 2560,
            },
            Relayed::Progress {
                current_bar: 384,
                total_bars: 720,
                elapsed_ms: 3840,
            },
            Relayed::Done,
        ],
        "{what}"
    );
}

// ── The real Recipe::stream ─────────────────────────────────────────────────

/// T-PB-1d: the real `<LabProgressRecipe as Recipe>::stream` (salt 1), the
/// path `cockpit_live.rs` builds. The take is EAGER: right after `stream()`
/// returned, before the stream is first polled, the shared cell holds `None`.
/// A progress sent then is relayed with its fields unchanged (the stream holds
/// the receiver it took); dropping the last sender relays
/// `LabRunProgressDone`, then the stream ENDS (`Ok(None)` within 200 ms).
///
/// The cell is not checked for `Some` before `stream()`: the test puts
/// `Some(rx)` there one line earlier, so that would hold by construction.
#[tokio::test]
async fn lab_progress_recipe_stream_end_to_end() {
    let (tx, rx) = progress_channel();
    let cell = Arc::new(Mutex::new(Some(rx)));
    let recipe = LabProgressRecipe {
        rt_handle: tokio::runtime::Handle::current(),
        rx: Arc::clone(&cell),
        salt: 1,
    };
    let mut stream = Box::new(recipe).stream(empty_event_stream());

    assert!(
        cell.lock().unwrap().is_none(),
        "lab progress recipe: `Recipe::stream` took the receiver before the first poll \
         (right after stream() returned, the shared cell must hold None)"
    );

    tx.try_send(Progress {
        current_bar: 100,
        total_bars: 720,
        elapsed_ms: 1500,
    })
    .expect("lab progress recipe: the send fits (an empty channel of 8, its receiver alive)");
    let what = "lab progress recipe: the progress sent on the taken receiver is relayed unchanged";
    assert_eq!(
        next_relayed(&mut stream, what).await,
        Some(Relayed::Progress {
            current_bar: 100,
            total_bars: 720,
            elapsed_ms: 1500,
        }),
        "{what}"
    );

    drop(tx);
    let what = "lab progress recipe: dropping the last sender relays LabRunProgressDone";
    assert_eq!(
        next_relayed(&mut stream, what).await,
        Some(Relayed::Done),
        "{what}"
    );
    assert_ends_within(
        &mut stream,
        200,
        "lab progress recipe: the stream ENDS after LabRunProgressDone",
    )
    .await;
}

/// Two-sided over the cell's take state. Recipe salt 1 takes the receiver and
/// its stream relays bar 100: the live side, so the cell really was armed and
/// really was taken. Recipe salt 2 (the bumped salt the next run builds) on
/// the same, now taken cell: its stream ENDS without yielding, `Ok(None)`
/// within 300 ms. A parked `None` path fails (a timeout), and so does a cell
/// that was re-armed (the second stream would relay `LabRunProgressDone`).
///
/// The sender stays alive to the end, so the first receiver is still open
/// while the second recipe streams: its end cannot come from a closed channel.
#[tokio::test]
async fn lab_progress_second_recipe_on_a_taken_cell_ends_without_yielding() {
    let (tx, rx) = progress_channel();
    let cell = Arc::new(Mutex::new(Some(rx)));

    let first = LabProgressRecipe {
        rt_handle: tokio::runtime::Handle::current(),
        rx: Arc::clone(&cell),
        salt: 1,
    };
    let mut first_stream = Box::new(first).stream(empty_event_stream());
    tx.try_send(Progress {
        current_bar: 100,
        total_bars: 720,
        elapsed_ms: 1500,
    })
    .expect("lab progress recipe: the send fits (an empty channel of 8, its receiver alive)");
    let what = "lab progress recipe: the first recipe relays a progress on the receiver it took";
    assert_eq!(
        next_relayed(&mut first_stream, what).await,
        Some(Relayed::Progress {
            current_bar: 100,
            total_bars: 720,
            elapsed_ms: 1500,
        }),
        "{what}"
    );

    let second = LabProgressRecipe {
        rt_handle: tokio::runtime::Handle::current(),
        rx: Arc::clone(&cell),
        salt: 2,
    };
    let mut second_stream = Box::new(second).stream(empty_event_stream());
    assert_ends_within(
        &mut second_stream,
        300,
        "lab progress recipe: a second recipe on the taken cell ENDS without yielding",
    )
    .await;

    drop(tx);
}
