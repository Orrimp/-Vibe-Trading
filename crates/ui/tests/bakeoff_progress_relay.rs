//! advisor-bakeoff-progress — the candidate-progress agent→ui "last-mile" RELAY
//! test (the headline ask's verification floor).
//!
//! ## Why this file exists — the bug class the fixture render test CANNOT catch
//!
//! The bake-off progress render test (`bakeoff_progress_render.rs`) constructs
//! the leaderboard state DIRECTLY (with a fixture `BakeoffProgress` set) and
//! renders it. It proves the populated progress bar *draws* — but it BYPASSES
//! the channel, so it cannot catch the actual bug class this project keeps
//! shipping: the cockpit recipe that consumes the engine→ui channel is never
//! wired, so the `BakeoffProgress` receiver is never drained and the bar stays
//! stuck on the indeterminate spinner ("channel built but no recipe consumes
//! it" — this exact gap has bitten 5×).
//!
//! This test drives the EXTRACTED relay function the recipe wraps
//! (`bakeoff_progress_stream_impl` — the `async_stream` body over an `rx`, the
//! `lab_progress_recipe_stream.rs` / `forward_narration_relay.rs` precedent) and
//! asserts the full channel→`Message`→`update`→populated-state path end to end:
//! a `BakeoffProgress { done, total, current_id }` fed into the channel yields
//! `Message::BakeoffProgress`, AND applying it via `update` lands the leaderboard
//! progress state at the fed value (the determinate bar's data). The bar's own
//! "X of N" label and fill are view math, pinned at the pixel layer by
//! `bakeoff_progress_render.rs`, not here.
//!
//! ## Non-vacuity contract (bug-log #66 / story 2-63 AC3)
//!
//! Every relay test proves it RAN: exact counts and order, the stream end
//! observed as `Ok(None)` inside a timeout that FAILS the test (never an
//! accepting `Err(_) => break`), every send CHECKED ([`send_checked`] — the
//! production `try_send` is lossy by design), and state preconditions seeded so
//! they can actually fail (a stale `progress` is planted BEFORE `begin_run`).
//! Checks that hold by construction are not asserted.
//!
//! ## T-T4 falsification probe table (story 2-63 AC4)
//!
//! One edit each to production code, located by symbol + quoted line text.
//! Procedure:
//!
//! 1. snapshot the file (`cp -p <file> <snap>`) and apply the edit;
//! 2. run `cargo test -p ui --test bakeoff_progress_relay` — the named test(s)
//!    go RED;
//! 3. restore from the snapshot, THEN `touch <file>`. A restore that keeps the
//!    snapshot's older mtime (`cp -p`) leaves cargo's fingerprint "fresh": the
//!    next run compiles nothing and re-executes the MUTANT binary;
//! 4. re-run. The GREEN counts only if that run's output shows
//!    `Compiling ui` and the file's SHA-256 equals the pre-probe one.
//!
//! Probe numbers are shared across the three Wave A files.
//!
//! | # | symbol (file) | quoted line today | mutation | expected RED |
//! |---|---|---|---|---|
//! | P7 | `ui::live::bakeoff_progress_stream_impl` (`crates/ui/src/live.rs`) | `yield Message::BakeoffProgress(progress);` | insert `if progress.done == 0 { continue; }` before it | `bakeoff_progress_relay_forwards_every_event_in_order_then_ends`, `bakeoff_progress_relay_latest_event_wins_on_state` (its first relayed event is not `done = 0`) |
//! | P8 | `ui::live::bakeoff_progress_stream_impl` | `debug!(channel = "bakeoff_progress", "progress channel closed — stopping recipe");` | append `futures::future::pending::<()>().await;` | `bakeoff_progress_relay_latest_event_wins_on_state` (its timeout now panics), the forwards-every-event test, `bakeoff_progress_relay_terminates_on_sender_drop`, `bakeoff_progress_relay_yields_message_and_populates_state`, `bakeoff_progress_recipe_stream_end_to_end` |
//! | P20 | `ui::live::bakeoff_progress_stream_impl` | `while let Some(progress) = rx.recv().await {` (the line above `yield Message::BakeoffProgress(progress);`) | → `while let Some(progress) = None::<backtest::progress::BakeoffProgress> {` | `bakeoff_progress_relay_terminates_on_sender_drop` (0 messages, wanted 1), `bakeoff_progress_relay_yields_message_and_populates_state`, the forwards-every-event and latest-wins tests, `bakeoff_progress_recipe_stream_end_to_end` (its `assert_pending` finds the stream ENDED); only `bakeoff_progress_relay_none_yields_nothing` stays green |
//! | P9 | `ui::leaderboard::LeaderboardScreenState::begin_run` (`crates/ui/src/leaderboard/state.rs`) | `self.progress = None;` (under the comment `a stale "7 of 7" from the last run must never show on the new one`) | delete the line | `bakeoff_progress_relay_yields_message_and_populates_state` (stale progress seeded before `begin_run` survives) |
//! | P23 | `ui::state::update`, arm `Message::BakeoffProgress` (`crates/ui/src/state.rs`) | `model.leaderboard_screen_state.set_progress(progress);` | → `if progress.done > 0 { model.leaderboard_screen_state.set_progress(progress); }` | `bakeoff_progress_relay_latest_event_wins_on_state` (after the `done = 0` event the state does not show it); every relay-only test stays green |
//! | P21 | `<BakeoffProgressRecipe as Recipe>::stream` | `bakeoff_progress_stream_impl(rx_opt)` | → `bakeoff_progress_stream_impl(None)` | `bakeoff_progress_recipe_stream_end_to_end`, at its `assert_pending` right after `stream()` — before any send |
//! | P22 | `<BakeoffProgressRecipe as Recipe>::stream` | `bakeoff_progress_stream_impl(rx_opt)` | insert `if rx_opt.is_none() { return Box::pin(futures::stream::pending()); }` before it | `bakeoff_progress_recipe_stream_end_to_end`, at the SECOND recipe on the taken cell (its 1 s `Ok(None)` times out) |
//!
//! Not pinned here: the recipe's per-run `salt` (`self.salt.hash(state);`) —
//! the tests build `salt: 1` but never hash it. Salt identity is story 2-63 §5
//! (`subscription_batch_inclusion.rs`, Wave C).
//!
//! Gated on `live` (the only build where the recipe wiring lives, mirroring
//! `forward_narration_relay.rs`), NOT on `target_os` (no pixels here — pure
//! channel/state logic, deterministic on every OS).

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use futures::stream::BoxStream;
use futures::{FutureExt, StreamExt};
use smol_str::SmolStr;
use tokio::time::timeout;

use backtest::progress::{BakeoffProgress, bakeoff_progress_pair};
use ui::state::{Cockpit, Message, update};

/// Await the next item, FAILING the test if nothing resolves in `secs`.
///
/// `Some(msg)` = a relayed message, `None` = the stream ENDED. A timeout is
/// never read as either (the accepting `Err(_) => break` is the bug-log #66
/// vacuity this file used to carry).
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

/// A CHECKED send: `BakeoffProgressSender::try_send` swallows a full/closed
/// channel by design (lossy UI progress), which would let a test "send" an
/// event that never entered the channel. Go through the inner sender and fail
/// on any drop.
fn send_checked(tx: &backtest::progress::BakeoffProgressSender, p: BakeoffProgress) {
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

/// The relayed bake-off progress, or a panic naming what arrived instead.
fn bakeoff_progress(msg: &Message) -> &BakeoffProgress {
    match msg {
        Message::BakeoffProgress(p) => p,
        other => panic!("expected Message::BakeoffProgress, got {other:?}"),
    }
}

/// **The last-mile proof (T-T4 P9).** Feeding a `BakeoffProgress` into the
/// bake-off progress channel makes the relay yield `Message::BakeoffProgress`
/// with every field carried, AND applying that message via `update` lands the
/// leaderboard's progress state at the fed value, over a STALE progress that
/// `begin_run` had to clear first. Then the stream ends on sender drop.
#[tokio::test]
async fn bakeoff_progress_relay_yields_message_and_populates_state() {
    let (tx, rx) = bakeoff_progress_pair();
    let mut stream = ui::live::bakeoff_progress_stream_impl(Some(rx));

    // `done = 3, total = 7` → the 4th candidate ("v0.5.bbands") is now running.
    send_checked(&tx, progress(3, 7, "v0.5.bbands"));

    let msg = next_or_fail(&mut stream, 2, "the relayed progress event")
        .await
        .expect("the relay yielded a Message");

    // 1) The relay maps the engine BakeoffProgress → Message::BakeoffProgress
    //    with the fields carried faithfully.
    let p = bakeoff_progress(&msg);
    assert_eq!(p.done, 3, "done carried");
    assert_eq!(p.total, 7, "total carried");
    assert_eq!(p.current_id.as_str(), "v0.5.bbands", "current_id carried");

    // 2) Applying the Message populates the leaderboard progress state. A run is
    //    in flight, so begin_run() first (mirrors the real BakeoffRunRequested
    //    pure-state half), THEN the progress event lands.
    //
    //    A STALE progress ("7 of 7" from a previous run) is planted BEFORE
    //    begin_run, so the "begin_run clears progress" precondition below can
    //    actually fail (from a fresh `Cockpit` it was `None` by Default —
    //    vacuous; T-T4 P9).
    let mut cockpit = Cockpit::new();
    cockpit.leaderboard_screen_state.progress = Some(progress(7, 7, "buy_and_hold"));
    cockpit.leaderboard_screen_state.begin_run();
    assert!(
        cockpit.leaderboard_screen_state.progress.is_none(),
        "precondition: begin_run clears the stale progress (no stale bar)"
    );

    update(&mut cockpit, msg);

    let st = &cockpit.leaderboard_screen_state;
    let p = st
        .progress
        .as_ref()
        .expect("the progress event populated the leaderboard state");
    assert_eq!(p.done, 3, "state.progress.done == fed done");
    assert_eq!(p.total, 7, "state.progress.total == fed total");
    assert_eq!(
        p.current_id.as_str(),
        "v0.5.bbands",
        "state.progress.current_id == fed id"
    );

    drop(tx);
    let end = next_or_fail(&mut stream, 2, "the relay end after drop").await;
    assert!(end.is_none(), "the relay ends on sender drop (got {end:?})");
}

/// **The state FOLLOWS the relay, event by event (T-T4 P23).** Three events
/// `done = 0, 1, 2` (sent CHECKED), sender dropped. Each relayed event is
/// applied via `update`, and the state is checked after EVERY step: it holds
/// exactly the event just applied — including the `done = 0` "first candidate
/// starting" event, so an `update` arm that skips it is red here while every
/// relay-only test stays green. The run then ends, observed as `Ok(None)`.
#[tokio::test]
async fn bakeoff_progress_relay_latest_event_wins_on_state() {
    let (tx, rx) = bakeoff_progress_pair();
    let mut stream = ui::live::bakeoff_progress_stream_impl(Some(rx));

    let mut cockpit = Cockpit::new();
    update(&mut cockpit, Message::BakeoffRunRequested);

    let sent = [(0u16, "v0.sma"), (1, "v0.5.macd"), (2, "v0.5.rsi")];
    for (done, id) in sent {
        send_checked(&tx, progress(done, 7, id));
    }
    drop(tx); // close so the stream terminates after draining

    // A timeout PANICS (story 2-63: this loop used to `Err(_) => break`, so a
    // stream that never ends — T-T4 P8 — "drained" and passed).
    let mut relayed = 0usize;
    while let Some(msg) = next_or_fail(&mut stream, 2, "the next progress event or the end").await {
        let (want_done, want_id) = *sent
            .get(relayed)
            .unwrap_or_else(|| panic!("more events relayed than were sent: {msg:?}"));
        let p = bakeoff_progress(&msg);
        assert_eq!(
            (p.done, p.current_id.as_str()),
            (want_done, want_id),
            "relay step {relayed}: events arrive in send order"
        );
        update(&mut cockpit, msg);
        let st = cockpit
            .leaderboard_screen_state
            .progress
            .as_ref()
            .unwrap_or_else(|| {
                panic!("after step {relayed} (done = {want_done}) the state shows no progress")
            });
        assert_eq!(
            (st.done, st.total, st.current_id.as_str()),
            (want_done, 7, want_id),
            "after step {relayed} the state holds exactly the event just applied"
        );
        relayed += 1;
    }

    assert_eq!(
        relayed,
        sent.len(),
        "every one of the {} sent events was relayed",
        sent.len()
    );
}

/// **Every event, in order, then the end (T-T4 P7).** Three events
/// `done = 0, 1, 2` (well under capacity 8, sent CHECKED), sender dropped:
/// the relay yields exactly `[0, 1, 2]` with the ids in send order, then the
/// stream ends (`Ok(None)`). A relay that filters or drops any event (e.g. the
/// "first candidate starting" `done == 0` event) is red here, while every
/// single-event test stays green.
#[tokio::test]
async fn bakeoff_progress_relay_forwards_every_event_in_order_then_ends() {
    let (tx, rx) = bakeoff_progress_pair();
    let mut stream = ui::live::bakeoff_progress_stream_impl(Some(rx));

    let sent = [(0u16, "v0.sma"), (1, "v0.5.macd"), (2, "v0.5.rsi")];
    for (done, id) in sent {
        send_checked(&tx, progress(done, 7, id));
    }
    drop(tx);

    let mut got: Vec<(u16, u16, String)> = Vec::new();
    while let Some(msg) = next_or_fail(&mut stream, 2, "the next progress event or the end").await {
        let p = bakeoff_progress(&msg);
        got.push((p.done, p.total, p.current_id.to_string()));
        assert!(
            got.len() <= sent.len(),
            "more events relayed than were sent: {got:?}"
        );
    }

    let dones: Vec<u16> = got.iter().map(|(d, _, _)| *d).collect();
    assert_eq!(dones, [0, 1, 2], "every event forwarded, in send order");
    let ids: Vec<&str> = got.iter().map(|(_, _, id)| id.as_str()).collect();
    assert_eq!(
        ids,
        ["v0.sma", "v0.5.macd", "v0.5.rsi"],
        "current_id carried in send order"
    );
    assert!(
        got.iter().all(|(_, total, _)| *total == 7),
        "total carried on every event"
    );
}

/// The relay forwards what was buffered and THEN terminates when the sender
/// drops (channel closed) — no hang, no panic. Rewritten for story 2-63 (the
/// `forward_plan_relay_terminates_on_sender_drop` precedent): the old version
/// sent nothing, so a relay that never enters its loop (T-T4 P20) also
/// "terminated" and passed. Now exactly one message must precede the end.
#[tokio::test]
async fn bakeoff_progress_relay_terminates_on_sender_drop() {
    let (tx, rx) = bakeoff_progress_pair();
    let mut stream = ui::live::bakeoff_progress_stream_impl(Some(rx));

    send_checked(&tx, progress(6, 7, "v0.buyhold"));
    drop(tx);

    let first = next_or_fail(&mut stream, 2, "the buffered progress event")
        .await
        .expect("the event sent before the drop is relayed, not lost");
    let p = bakeoff_progress(&first);
    assert_eq!(
        (p.done, p.total, p.current_id.as_str()),
        (6, 7, "v0.buyhold"),
        "the buffered event"
    );

    let end = next_or_fail(&mut stream, 2, "the stream end after the drop").await;
    assert!(
        end.is_none(),
        "the relay ends when the channel closes (got {end:?})"
    );
}

/// `None` receiver (the double-`take()` case) yields nothing — the silent-empty
/// guard the `lab_progress_recipe_stream.rs` smoking-gun test pins (a bar that
/// never advances because the stream silently yields nothing).
#[tokio::test]
async fn bakeoff_progress_relay_none_yields_nothing() {
    let mut stream = ui::live::bakeoff_progress_stream_impl(None);
    let next = next_or_fail(&mut stream, 1, "the None receiver's immediate end").await;
    assert!(
        next.is_none(),
        "None receiver ⇒ empty stream (got {next:?})"
    );
}

/// **Recipe-path integration (T-T4 P21, P22).** The FULL
/// `<BakeoffProgressRecipe as Recipe>::stream` (take via
/// `Arc<Mutex<Option<_>>>`, then delegate to `bakeoff_progress_stream_impl`) —
/// the code path `cockpit_live::subscription()` uses. Three claims, the same as
/// its siblings in `forward_narration_relay.rs` / `sweep_progress_relay.rs`:
///
/// 1. **take + own:** `stream()` empties the cell AND returns a stream live on
///    the receiver it took (P21 red at `assert_pending`, before any send) that
///    relays `done = 5` with its fields unchanged;
/// 2. **`stream()` on a taken cell ENDS:** a second recipe on the same Arc
///    yields an empty stream that finishes within 1 s (P22 red only here);
/// 3. **end:** the first stream ends on sender drop.
///
/// What a SECOND `stream()` call means in production (a frame that re-streams
/// without bumping the salt) is claim 2's subject; whether the salt actually
/// gives each run a fresh identity is NOT pinned here (see the module doc).
#[tokio::test]
async fn bakeoff_progress_recipe_stream_end_to_end() {
    use std::sync::{Arc, Mutex};

    use iced::advanced::subscription::{EventStream, Recipe};
    use ui::live::BakeoffProgressRecipe;

    fn empty_event_stream() -> EventStream {
        Box::pin(futures::stream::empty::<iced::advanced::subscription::Event>())
    }

    let (tx, rx) = bakeoff_progress_pair();
    let rx_arc = Arc::new(Mutex::new(Some(rx)));

    let recipe = BakeoffProgressRecipe {
        rt_handle: tokio::runtime::Handle::current(),
        rx: Arc::clone(&rx_arc),
        salt: 1,
    };
    let mut stream = Box::new(recipe).stream(empty_event_stream());

    // 1) take + own.
    assert!(
        rx_arc.lock().unwrap().is_none(),
        "after Recipe::stream() calls take(), the Arc<Mutex<Option<_>>> holds None"
    );
    assert_pending(
        &mut stream,
        "Recipe::stream relays the receiver it took (a live stream, not an empty one)",
    );
    assert!(
        !tx.0.as_ref().expect("enabled sender").is_closed(),
        "the receiver Recipe::stream took is alive inside the returned stream"
    );
    send_checked(&tx, progress(5, 7, "v0.8.vote.majority"));
    let first = next_or_fail(&mut stream, 2, "the progress event via Recipe::stream")
        .await
        .expect("the event sent after stream() is relayed by the recipe's stream");
    let p = bakeoff_progress(&first);
    assert_eq!(
        (p.done, p.total, p.current_id.as_str()),
        (5, 7, "v0.8.vote.majority"),
        "BakeoffProgressRecipe::stream() forwards BakeoffProgress fields unchanged"
    );

    // 2) stream() on the now-taken cell ends instead of parking.
    let second = BakeoffProgressRecipe {
        rt_handle: tokio::runtime::Handle::current(),
        rx: Arc::clone(&rx_arc),
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
