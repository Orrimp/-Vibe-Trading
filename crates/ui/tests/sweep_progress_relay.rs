//! advisor-param-tuning (ADR-0069 T10) — the sweep cell-progress agent→ui
//! "last-mile" RELAY test (story 2-63, lab-recipe-test-harness v0.3, Wave A S1).
//!
//! ## Why this file exists
//!
//! `SweepProgressRecipe` is a 1:1 copy of `BakeoffProgressRecipe` that yields
//! `Message::SweepProgress` instead of `Message::BakeoffProgress`, driving the
//! Tune screen's DETERMINATE bar. Before story 2-63 it had ZERO test
//! references: not the recipe, not its seam `sweep_progress_stream_impl`, not
//! `Message::SweepProgress`. The copy-paste sibling is exactly where a wrong
//! variant or a wrong delegate compiles and ships: the bar would silently stay
//! on the indeterminate spinner, or — worse — sweep progress would land on the
//! LEADERBOARD's bar. These tests drive the seam and the full `Recipe::stream`
//! and assert channel → `Message::SweepProgress` → `update` → tune state, with
//! a leaderboard cross-talk control. Modelled on `bakeoff_progress_relay.rs`.
//!
//! ## Non-vacuity contract (bug-log #66 / story 2-63 AC3)
//!
//! Every test proves it RAN: exact counts and order, the stream end observed
//! as `Ok(None)` inside a timeout that FAILS the test (never an accepting
//! `Err(_) => break`), sends that fail if the event did not enter the channel,
//! and preconditions that can fail — a stale tune progress and an in-flight
//! bake-off bar are SEEDED, so "cleared" and "untouched" are observed, not
//! read off `Cockpit::new()` defaults. Checks that hold by construction are not
//! asserted. `BakeoffProgress` has no `PartialEq`, so fields are compared one
//! by one.
//!
//! ## T-T4 falsification probe table (story 2-63 AC4)
//!
//! One edit each to production code, located by symbol + quoted line text.
//! Procedure:
//!
//! 1. snapshot the file (`cp -p <file> <snap>`) and apply the edit;
//! 2. run `cargo test -p ui --test sweep_progress_relay` — the named test(s) go
//!    RED;
//! 3. restore from the snapshot, THEN `touch <file>`. A restore that keeps the
//!    snapshot's older mtime (`cp -p`) leaves cargo's fingerprint "fresh": the
//!    next run compiles nothing and re-executes the MUTANT binary (measured in
//!    story 2-63's falsifier round: 0 crates compiled, the P13 mutant re-ran
//!    red, every SHA-256 matched);
//! 4. re-run. The GREEN counts only if that run's output shows
//!    `Compiling ui` and the file's SHA-256 equals the pre-probe one.
//!
//! Probe numbers are shared across the three Wave A files.
//!
//! | # | symbol (file) | quoted line today | mutation | expected RED |
//! |---|---|---|---|---|
//! | P10 | `ui::live::sweep_progress_stream_impl` (`crates/ui/src/live.rs`) | `yield Message::SweepProgress(progress);` | → `yield Message::BakeoffProgress(progress);` | `sweep_progress_relay_yields_sweep_message_and_populates_tune_state` (AC4-designated), `sweep_progress_relay_forwards_every_event_in_order_then_ends`, `sweep_progress_recipe_stream_takes_receiver_once`, all at the `sweep_progress` variant check; only `sweep_progress_relay_none_receiver_yields_nothing` stays green |
//! | P11 | `ui::live::sweep_progress_stream_impl` | `while let Some(progress) = rx.recv().await {` (the line above `yield Message::SweepProgress(progress);`) | `while` → `if` | `sweep_progress_relay_forwards_every_event_in_order_then_ends` (1 event, wanted 5) |
//! | P12 | `ui::live::sweep_progress_stream_impl` | `if let Some(mut rx) = rx_opt {` | insert `if rx_opt.is_none() { futures::future::pending::<()>().await; }` before it | `sweep_progress_relay_none_receiver_yields_nothing` (its 1 s timeout panics), `sweep_progress_recipe_stream_takes_receiver_once` (its second recipe on the taken cell) |
//! | P13 | `<SweepProgressRecipe as Recipe>::stream` | `sweep_progress_stream_impl(rx_opt)` | → `bakeoff_progress_stream_impl(rx_opt)` (the copy-paste sibling) | `sweep_progress_recipe_stream_takes_receiver_once` (a live stream that relays `BakeoffProgress`) |
//! | P26 | `<SweepProgressRecipe as Recipe>::stream` | `sweep_progress_stream_impl(rx_opt)` | → `sweep_progress_stream_impl(None)` | `sweep_progress_recipe_stream_takes_receiver_once`, at its `assert_pending` right after `stream()` — before any send |
//! | P24 | `ui::tune::TuneScreenState::begin_run` (`crates/ui/src/tune/screen_state.rs`) | `self.progress = None;` (the line after `self.running = true;`) | delete the line | `sweep_progress_relay_yields_sweep_message_and_populates_tune_state` (the seeded stale tune progress survives `SweepRunRequested`) |
//! | P25 | `ui::state::update`, arm `Message::SweepProgress` (`crates/ui/src/state.rs`) | `model.tune_screen_state.set_progress(progress);` | → `model.leaderboard_screen_state.set_progress(progress);` | `sweep_progress_relay_yields_sweep_message_and_populates_tune_state`, at the cross-talk control (the seeded bake-off bar is overwritten); every relay-only test stays green |
//!
//! Not pinned here: the recipe's per-run `salt` (built as `salt: 1`, never
//! hashed). Salt identity is story 2-63 §5 (`subscription_batch_inclusion.rs`,
//! Wave C).
//!
//! Gated on `live` (the only build where the recipe wiring lives), NOT on
//! `target_os` (no pixels — pure channel/state logic, deterministic on every OS).

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures::stream::BoxStream;
use futures::{FutureExt, StreamExt};
use iced::advanced::subscription::{EventStream, Recipe};
use smol_str::SmolStr;
use tokio::time::timeout;

use backtest::progress::{BakeoffProgress, BakeoffProgressSender, bakeoff_progress_pair};
use ui::live::SweepProgressRecipe;
use ui::state::{Cockpit, Message, PanelState, update};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Await the next item, FAILING the test if nothing resolves in `secs`.
///
/// `Some(msg)` = a relayed message, `None` = the stream ENDED. A timeout is
/// never read as either (bug-log #66: an accepted timeout is a skip-as-pass).
async fn next_or_fail(
    stream: &mut BoxStream<'static, Message>,
    secs: u64,
    what: &str,
) -> Option<Message> {
    timeout(Duration::from_secs(secs), stream.next())
        .await
        .unwrap_or_else(|_| panic!("timed out after {secs}s waiting for {what}"))
}

/// Assert the stream is LIVE with NOTHING buffered: a single poll returns
/// `Pending` (the relay is parked on its receiver). It fails if the stream
/// already ENDED (an empty relay) and if it yields an item that was never sent.
fn assert_pending(stream: &mut BoxStream<'static, Message>, what: &str) {
    match stream.next().now_or_never() {
        None => {}
        Some(None) => panic!("{what}: the stream already ENDED (expected it live on its receiver)"),
        Some(Some(msg)) => panic!("{what}: an unexpected message was buffered: {msg:?}"),
    }
}

/// A CHECKED send. `BakeoffProgressSender::try_send` (the production sweep
/// sender) swallows a full/closed channel by design (`progress.rs`: "dropping
/// it if the channel is full or closed"), so a test "send" could silently not
/// happen. Go through the inner sender and fail on any drop.
fn send_checked(tx: &BakeoffProgressSender, p: BakeoffProgress) {
    tx.0.as_ref()
        .expect("bakeoff_progress_pair builds an enabled sender")
        .try_send(p)
        .expect("the event entered the channel (capacity 8 not exceeded, receiver alive)");
}

fn progress(done: u16, total: u16, id: &str) -> BakeoffProgress {
    BakeoffProgress {
        done,
        total,
        current_id: SmolStr::new(id),
    }
}

/// The relayed sweep progress, or a panic naming what arrived instead (a
/// `Message::BakeoffProgress` here is the copy-paste-sibling bug).
fn sweep_progress(msg: &Message) -> &BakeoffProgress {
    match msg {
        Message::SweepProgress(p) => p,
        other => panic!("expected Message::SweepProgress, got {other:?}"),
    }
}

/// The leaderboard bar's `(done, total, current_id)`, `None` when it shows none.
fn leaderboard_bar(cockpit: &Cockpit) -> Option<(u16, u16, String)> {
    cockpit
        .leaderboard_screen_state
        .progress
        .as_ref()
        .map(|p| (p.done, p.total, p.current_id.to_string()))
}

fn empty_event_stream() -> EventStream {
    Box::pin(futures::stream::empty::<iced::advanced::subscription::Event>())
}

// ── S1: the seam ──────────────────────────────────────────────────────────────

/// **The last-mile proof (T-T4 P10 AC4-designated, P24, P25).** One
/// cell-progress event fed into the sweep channel yields
/// `Message::SweepProgress` (NOT the bake-off variant) with every field
/// carried; applying it after a real `SweepRunRequested` lands it on the TUNE
/// state.
///
/// Both state claims are made against SEEDED state, so each can fail:
///
/// - a stale tune progress from a prior sweep is planted before
///   `SweepRunRequested`, so "begin_run cleared it" is observed (P24);
/// - an in-flight bake-off's bar (2 of 7, driven through `update`) is on the
///   leaderboard, so the cross-talk control compares it UNCHANGED after the
///   sweep event, and runs FIRST — an `update` arm that writes the sweep event
///   to the leaderboard is red at the control itself (P25), not at a later
///   tune check.
#[tokio::test]
async fn sweep_progress_relay_yields_sweep_message_and_populates_tune_state() {
    let (tx, rx) = bakeoff_progress_pair();
    let mut stream = ui::live::sweep_progress_stream_impl(Some(rx));

    send_checked(&tx, progress(3, 12, "fast=15, slow=40"));

    let msg = next_or_fail(&mut stream, 2, "the sweep progress event")
        .await
        .expect("the relay yielded a Message");

    // 1) Fields carried one by one (BakeoffProgress has no PartialEq).
    let p = sweep_progress(&msg);
    assert_eq!(p.done, 3, "done carried");
    assert_eq!(p.total, 12, "total carried");
    assert_eq!(
        p.current_id.as_str(),
        "fast=15, slow=40",
        "current_id carried"
    );

    // 2) Seed the state both claims are made against.
    let mut cockpit = Cockpit::new();
    update(&mut cockpit, Message::BakeoffRunRequested);
    update(
        &mut cockpit,
        Message::BakeoffProgress(progress(2, 7, "v0.5.macd")),
    );
    let bakeoff_bar = Some((2, 7, "v0.5.macd".to_string()));
    assert_eq!(
        leaderboard_bar(&cockpit),
        bakeoff_bar,
        "seed: the in-flight bake-off's bar shows 2 of 7"
    );
    cockpit.tune_screen_state.progress = Some(progress(12, 12, "fast=30, slow=90"));

    // 3) Drive the real run lifecycle: SweepRunRequested → running, stale bar
    //    cleared; then the relayed event lands on the Tune state.
    assert!(
        cockpit.tune_screen_state.can_run(),
        "precondition: the default SMA form is runnable (else Requested is a no-op)"
    );
    update(&mut cockpit, Message::SweepRunRequested);
    assert!(
        cockpit.tune_screen_state.running,
        "precondition: SweepRunRequested started a run"
    );
    assert!(
        matches!(cockpit.tune_screen_state.result, PanelState::Loading),
        "precondition: the sweep result is Loading"
    );
    assert!(
        cockpit.tune_screen_state.progress.is_none(),
        "precondition: SweepRunRequested (begin_run) cleared the prior sweep's stale progress"
    );

    update(&mut cockpit, msg);

    // Cross-talk control FIRST: sweep progress must never drive (or clear) the
    // bake-off bar.
    assert_eq!(
        leaderboard_bar(&cockpit),
        bakeoff_bar,
        "sweep progress did NOT land on the leaderboard: its bar is unchanged"
    );
    let tp = cockpit
        .tune_screen_state
        .progress
        .as_ref()
        .expect("the relayed event populated the TUNE progress");
    assert_eq!(tp.done, 3, "tune progress.done == fed done");
    assert_eq!(tp.total, 12, "tune progress.total == fed total");
    assert_eq!(
        tp.current_id.as_str(),
        "fast=15, slow=40",
        "tune progress.current_id == fed id"
    );
    assert!(
        cockpit.tune_screen_state.running,
        "a progress event does not end the run"
    );

    drop(tx);
    let end = next_or_fail(&mut stream, 2, "the relay end after drop").await;
    assert!(end.is_none(), "the relay ends on sender drop (got {end:?})");
}

/// **Every event, in order, then the end (T-T4 P11).** Five events
/// `done = 0..=4` (≤ capacity 8 — `try_send` is lossy past it — sent CHECKED),
/// sender dropped: exactly `[0, 1, 2, 3, 4]` as `SweepProgress`, ids in send
/// order, then `Ok(None)`. A `while`→`if` relay yields one and stops.
#[tokio::test]
async fn sweep_progress_relay_forwards_every_event_in_order_then_ends() {
    let (tx, rx) = bakeoff_progress_pair();
    let mut stream = ui::live::sweep_progress_stream_impl(Some(rx));

    let ids = [
        "fast=10, slow=30",
        "fast=10, slow=40",
        "fast=15, slow=30",
        "fast=15, slow=40",
        "fast=20, slow=30",
    ];
    for (done, id) in (0u16..).zip(ids) {
        send_checked(&tx, progress(done, 5, id));
    }
    drop(tx);

    let mut got: Vec<(u16, u16, String)> = Vec::new();
    while let Some(msg) = next_or_fail(&mut stream, 2, "the next sweep event or the end").await {
        let p = sweep_progress(&msg);
        got.push((p.done, p.total, p.current_id.to_string()));
        assert!(
            got.len() <= ids.len(),
            "more events relayed than were sent: {got:?}"
        );
    }

    let dones: Vec<u16> = got.iter().map(|(d, _, _)| *d).collect();
    assert_eq!(
        dones,
        [0, 1, 2, 3, 4],
        "every event forwarded, in send order"
    );
    let got_ids: Vec<&str> = got.iter().map(|(_, _, id)| id.as_str()).collect();
    assert_eq!(got_ids, ids, "current_id carried in send order");
    assert!(
        got.iter().all(|(_, total, _)| *total == 5),
        "total carried on every event"
    );
}

/// **The negative half of the pair (T-T4 P12).** A `None` receiver (the
/// double-`take()` case) gives an EMPTY stream that ends at once — observed as
/// `Ok(None)` within 1 s. A stream that parks forever on `None` (the
/// "subscription alive, bar frozen" shape) times out and fails.
#[tokio::test]
async fn sweep_progress_relay_none_receiver_yields_nothing() {
    let mut stream = ui::live::sweep_progress_stream_impl(None);
    let next = next_or_fail(&mut stream, 1, "the None receiver's immediate end").await;
    assert!(
        next.is_none(),
        "None receiver ⇒ empty stream (got {next:?})"
    );
}

// ── S1: the full Recipe::stream ───────────────────────────────────────────────

/// **Recipe-path integration (T-T4 P13, P26).** The FULL
/// `<SweepProgressRecipe as Recipe>::stream` (salt 1) — enter the runtime,
/// `take()` the receiver, delegate — the code `cockpit_live::subscription()`
/// runs. Three claims:
///
/// 1. **take + own:** `stream()` empties the cell AND returns a stream live on
///    the receiver it took (P26 red at `assert_pending`, before any send) that
///    relays `done == 5` as `SweepProgress` (a recipe delegating to the
///    bake-off sibling yields `BakeoffProgress` — P13);
/// 2. **`stream()` on a taken cell ENDS:** a second recipe on the same Arc
///    yields an empty stream within 1 s (a parking `None` path — P12 — is red);
/// 3. **end:** the first stream ends on sender drop.
#[tokio::test]
async fn sweep_progress_recipe_stream_takes_receiver_once() {
    let (tx, rx) = bakeoff_progress_pair();
    let cell = Arc::new(Mutex::new(Some(rx)));

    let recipe = SweepProgressRecipe {
        rt_handle: tokio::runtime::Handle::current(),
        rx: Arc::clone(&cell),
        salt: 1,
    };
    let mut stream = Box::new(recipe).stream(empty_event_stream());

    // 1) take + own.
    assert!(
        cell.lock().unwrap().is_none(),
        "Recipe::stream take()-s the receiver: the cell holds None afterwards"
    );
    assert_pending(
        &mut stream,
        "Recipe::stream relays the receiver it took (a live stream, not an empty one)",
    );
    assert!(
        !tx.0.as_ref().expect("enabled sender").is_closed(),
        "the receiver Recipe::stream took is alive inside the returned stream"
    );
    send_checked(&tx, progress(5, 12, "fast=20, slow=50"));
    let msg = next_or_fail(&mut stream, 2, "the sweep event via Recipe::stream")
        .await
        .expect("the event sent after stream() is relayed by the recipe's stream");
    let p = sweep_progress(&msg);
    assert_eq!(p.done, 5, "done carried through the recipe");
    assert_eq!(p.total, 12, "total carried through the recipe");
    assert_eq!(
        p.current_id.as_str(),
        "fast=20, slow=50",
        "current_id carried through the recipe"
    );

    // 2) A second recipe on the same (taken) cell — what a frame that
    //    re-streams without a salt bump would build — ends instead of parking.
    let second = SweepProgressRecipe {
        rt_handle: tokio::runtime::Handle::current(),
        rx: Arc::clone(&cell),
        salt: 1,
    };
    let mut second_stream = Box::new(second).stream(empty_event_stream());
    let none = next_or_fail(&mut second_stream, 1, "the second recipe's end").await;
    assert!(
        none.is_none(),
        "a second recipe on a taken cell is empty (got {none:?})"
    );

    // 3) end.
    drop(tx);
    let end = next_or_fail(&mut stream, 2, "the recipe stream end after drop").await;
    assert!(
        end.is_none(),
        "the recipe stream ends on sender drop (got {end:?})"
    );
}
