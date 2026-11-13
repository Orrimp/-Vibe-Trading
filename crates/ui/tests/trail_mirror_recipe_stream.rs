//! Subscription-pipe tests for `TrailMirrorRecipe`, the relay that hands the
//! trail-mirror broadcast (`TrailMirrorHandle::tick_tx`) to iced. Closes the
//! `channel-recipe-state-widget seam` testing gap identified in
//! `docs/dev-notes/archive/2026-Q2/testing-framework-audit-2026-05-25.md`
//! (R1 / batch item 4).
//!
//! `trail_mirror_stream_impl` (`crates/ui/src/live.rs`) is the stream
//! `<TrailMirrorRecipe as Recipe>::stream` returns: a loop over the broadcast
//! receiver with three arms. These tests pin each arm by what comes OUT of the
//! stream:
//! - `Ok(tick)` relays one `Message::TrailMirrorTick` per tick, with the audit
//!   id unchanged, in send order;
//! - `Lagged(n)` is survived: the retained ticks, and every later one, are
//!   still relayed;
//! - `Closed` ends the stream, but only once every sender is gone, and only
//!   after the ticks buffered before the close are relayed.
//!
//! ## The real `Recipe::stream` (T-TM-1d)
//!
//! The five `stream_impl` tests hand the seam a receiver they built
//! themselves, so they cannot see the Recipe's own line: the subscribe it runs
//! BEFORE the stream is first polled (`let rx = self.handle.tick_tx.subscribe();`
//! in `<TrailMirrorRecipe as Recipe>::stream`). T-TM-1d,
//! `trail_mirror_recipe_stream_end_to_end`, drives the real Recipe. The struct
//! is private (`struct TrailMirrorRecipe {` in `live.rs`), but its pub
//! constructor `ui::live::trail_mirror_subscription` returns it inside an iced
//! `Subscription`, and iced's `iced::advanced::subscription::into_recipes`
//! unpacks that. No iced application is needed: the recipe ignores its
//! `EventStream`, so an empty one is faithful. The eager subscribe is OBSERVED
//! as the sender's receiver count, read right after `stream()` returned and
//! before the first poll.
//!
//! The off-runtime drain of the same Recipe on iced's executor is story 2-63
//! Wave B's `trail_mirror_recipe_drains_off_runtime`
//! (`crates/ui/tests/recipe_off_runtime_poll.rs`, probes B-P8, B-P12, B-P37).
//! T-TM-1d is its on-runtime twin, with a strict end.
//!
//! ## Non-vacuity contract (bug-log #66 / story 2-63 AC3)
//!
//! Every test proves it RAN: the exact relayed audit ids in order, and a
//! stream end observed as `Ok(None)` inside a timeout that FAILS the test. No
//! timeout is ever accepted as "drained": [`next_or_fail`] panics on elapse.
//! "Nothing more is buffered and the stream is live" is OBSERVED with
//! [`assert_pending`], never read off a counter the test increments itself.
//! Checks that hold by construction (the receiver count before `stream()`) are
//! not asserted. The claims come in two-sided pairs:
//! - over channel capacity: capacity 16 relays every tick it is sent, while
//!   capacity 2 drops the oldest two. The lag premise is read off the output,
//!   not assumed;
//! - over the senders: dropping every sender ends the stream (with ticks
//!   buffered and without), while a live sender keeps it open, at once and
//!   after an hour of quiet on a paused clock.
//!
//! ## T-T4 falsification probe table (story 2-63 AC4, Wave E1)
//!
//! Each probe is ONE edit to production code, located by symbol + the quoted
//! line text (line numbers rot). Procedure:
//!
//! 1. snapshot the file (`cp -p <file> <snap>`) and apply the edit;
//! 2. run `cargo test -p ui --test trail_mirror_recipe_stream`. The named
//!    test(s) go RED;
//! 3. restore from the snapshot, THEN `touch <file>`. A restore that keeps the
//!    snapshot's older mtime (`cp -p`) leaves cargo's fingerprint "fresh": the
//!    next run compiles nothing and re-executes the MUTANT binary;
//! 4. re-run. The GREEN counts only if that run's output shows
//!    `Compiling ui` and the file's SHA-256 equals the pre-probe one.
//!
//! Probe ids are the Wave E1 series, shared across the seven v0.2
//! recipe-stream files E1 sweeps; this file's rows are E1-P3, E1-P4, E1-P5,
//! E1-P6, E1-P28 and E1-P35. All six edit `crates/ui/src/live.rs`. E1-P4,
//! E1-P28 and E1-P35 share the `Ok(tick)` line, so apply one at a time. E1-P35
//! is the activity file's measured survivor AT-OWN-1 (E1a falsifier), applied
//! to this relay. E1-P6 leaves the private
//! `handle` field unread, so it warns `dead_code` while applied: never run
//! clippy with a probe applied.
//!
//! | # | symbol (file) | quoted line today | mutation | expected RED |
//! |---|---|---|---|---|
//! | E1-P3 | `ui::live::trail_mirror_stream_impl`, the `Closed` arm (`crates/ui/src/live.rs`) | `debug!(channel = "trail_mirror", "broadcast closed — stopping recipe");` | the `break;` below it → `std::future::pending::<()>().await;` | `stream_impl_yields_event_then_done`, at "trail: dropping the sender ENDS the stream after the relayed tick" (`Err(Elapsed)`); `stream_impl_on_already_closed_channel_ends_without_yielding`, at "trail: an already-closed channel ENDS the stream without yielding" (`Err(Elapsed)`); `stream_impl_multiple_events_in_order`, whose drain times out after the three ticks ("timed out after 2s waiting for trail: all three ticks are relayed in send order, then the stream ENDS"); `trail_mirror_recipe_stream_end_to_end`, whose final drain times out ("timed out after 2s waiting for trail recipe: dropping the last sender ENDS the stream with nothing more relayed") |
//! | E1-P4 | `ui::live::trail_mirror_stream_impl`, the `Ok(tick)` arm | `Ok(tick) => yield Message::TrailMirrorTick(TrailMirrorUiTick::from(tick)),` | → `Ok(tick) => { yield Message::TrailMirrorTick(TrailMirrorUiTick::from(tick)); break; }` (the relay ends after its first tick) | `stream_impl_multiple_events_in_order`, at "trail: all three ticks are relayed in send order, then the stream ENDS" (left `["audit-a"]`); `trail_mirror_stream_stays_open_while_sender_alive`, at its second `assert_pending`, "trail: after the relayed tick the stream is still live with nothing buffered" (the stream already ENDED); `trail_mirror_stream_continues_after_lag`, at "trail lag: after Lagged(2) the retained ticks audit-3, audit-4 then the marker are relayed" (left `[Some("audit-3"), None]`) |
//! | E1-P5 | `ui::live::trail_mirror_stream_impl`, the `Lagged(n)` arm | `// Continue: lag is not fatal; the cockpit re-fetches on` | append `break;` after the comment's next line, `// the next visible chevron click (R1.5 drop-on-lag policy).` | `trail_mirror_stream_continues_after_lag`, at "trail lag: after Lagged(2) the retained ticks audit-3, audit-4 then the marker are relayed" (left `[None, None]`: the stream ENDED at the lag) |
//! | E1-P6 | `<TrailMirrorRecipe as Recipe>::stream` (story-designated, wave row E1) | `let rx = self.handle.tick_tx.subscribe();` | → `let rx = tokio::sync::broadcast::channel::<reflection::trail_mirror::TrailMirrorTick>(16).1;` (subscribe to a fresh channel, not the handle's sender) | `trail_mirror_recipe_stream_end_to_end`, at "trail recipe: `Recipe::stream` subscribed EAGERLY to the handle's own sender" (the count stays at `before`: left 1, right 2) |
//! | E1-P28 | `ui::live::trail_mirror_stream_impl`, the receive | `match rx.recv().await {` (the line above `Ok(tick) => yield Message::TrailMirrorTick(TrailMirrorUiTick::from(tick)),`) | → `match rx.try_recv().map_err(\|_\| tokio::sync::broadcast::error::RecvError::Closed) {` (an empty channel, a lag and a close all end the loop) | `trail_mirror_stream_stays_open_while_sender_alive`, at "trail: with a sender alive the stream is live, not ended" (the stream already ENDED); `trail_mirror_stream_continues_after_lag`, at "trail lag: after Lagged(2) the retained ticks audit-3, audit-4 then the marker are relayed" (the lag is mapped to `Closed`, so left `[None, None]`) |
//! | E1-P35 | `ui::live::trail_mirror_stream_impl`, the receive | `match rx.recv().await {` (the line above `Ok(tick) => yield Message::TrailMirrorTick(TrailMirrorUiTick::from(tick)),`) | → `match tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv()).await.unwrap_or(Err(tokio::sync::broadcast::error::RecvError::Closed)) {` (the relay ENDS after 100 ms of quiet with a sender alive) | `trail_mirror_stream_stays_open_while_sender_alive`, at "trail: after an hour of quiet with a sender alive the stream is still live (paused clock)" (the stream already ENDED) |
//!
//! The tests each probe leaves GREEN are the other side of its pair: under
//! E1-P3 nothing else closes; under E1-P4 the close tests that send at most
//! one tick still end (the relay ends right after that tick, which is when the
//! close would end it anyway); under E1-P5 nothing else lags; under E1-P6 no
//! other test reaches `Recipe::stream`; under E1-P28 the close tests still end
//! (`try_recv` relays the buffered ticks, then reads `Closed`); under E1-P35
//! every read the other tests make is immediate. Outside this target
//! (MEASURED by the E1a falsifier): Wave B's
//! `trail_mirror_recipe_drains_off_runtime` is red under E1-P6 too, but at
//! `expect_parked` ("trail mirror: the stream ENDED on its first poll"), one
//! assertion before its receiver count: the mutant's fresh channel drops its
//! sender at once. Conversely, Wave B's B-P12 (a lazy subscribe) now turns
//! T-TM-1d red here, at "trail recipe: `Recipe::stream` subscribed EAGERLY to
//! the handle's own sender" (reasoned; Wave B's sibling column predates E1a).
//! No lib unit test drives the trail relay (`live::tests` has no trail fn), so
//! none goes red.
//!
//! ## Coverage
//!
//! | origin | test | what it pins |
//! |---|---|---|
//! | v0.2 T-TM-1a, strict | `stream_impl_yields_event_then_done` | one tick relayed with its audit id unchanged; dropping the sender then ends the stream: `Ok(None)` within 300 ms |
//! | v0.2 T-TM-1b, renamed and strict | `stream_impl_on_already_closed_channel_ends_without_yielding` | the sender dropped before the stream is built: `Ok(None)` within 300 ms, nothing relayed |
//! | E1, negative control | `trail_mirror_stream_stays_open_while_sender_alive` | with a live sender and nothing sent the stream is live, not ended, at once and after an hour of quiet (paused clock); a tick sent then is relayed, and the stream is live again |
//! | v0.2 T-TM-1c, strict | `stream_impl_multiple_events_in_order` | capacity 16, three ticks buffered before a close: exactly `["audit-a", "audit-b", "audit-c"]`, then the end |
//! | E1 | `trail_mirror_stream_continues_after_lag` | capacity 2, four ticks: the relay survives `Lagged(2)` and yields `audit-3`, `audit-4`, then a later `marker`, then is live |
//! | v0.2 T-TM-1d, rewritten through the real Recipe | `trail_mirror_recipe_stream_end_to_end` | `into_recipes(trail_mirror_subscription(..))` holds one recipe; its `stream()` subscribes to the handle's own sender before the first poll; a tick sent before that poll is relayed; dropping the last sender ends it with nothing more |
//!
//! Phase D+ / testing-framework-audit batch item 4.

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use futures::stream::BoxStream;
use futures::{FutureExt, StreamExt};
use iced::advanced::subscription::{EventStream, Recipe, into_recipes};
use tokio::sync::{broadcast, mpsc};
use tokio::time::timeout;

use reflection::trail_mirror::{TrailMirrorHandle, TrailMirrorRequest, TrailMirrorTick};
use ui::live::trail_mirror_stream_impl;
use ui::state::{Message, TrailMirrorUiTick};

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Construct a `(broadcast::Sender, broadcast::Receiver)` pair for
/// `TrailMirrorTick`, with a capacity no test here outruns (at most three
/// ticks are buffered).
fn trail_tick_pair() -> (
    broadcast::Sender<TrailMirrorTick>,
    broadcast::Receiver<TrailMirrorTick>,
) {
    broadcast::channel(16)
}

/// Build a minimal `TrailMirrorHandle` without spawning a real `TrailMirror`
/// task. The `req_tx` half is kept alive in the caller to avoid the channel
/// closing immediately.
fn make_handle(
    tick_tx: broadcast::Sender<TrailMirrorTick>,
) -> (TrailMirrorHandle, mpsc::Receiver<TrailMirrorRequest>) {
    let (req_tx, req_rx) = mpsc::channel(8);
    let handle = TrailMirrorHandle { req_tx, tick_tx };
    (handle, req_rx)
}

/// The iced runtime event stream a `Recipe::stream` receives; the trail
/// recipe ignores it, so an empty one is faithful.
fn empty_event_stream() -> EventStream {
    Box::pin(futures::stream::empty::<iced::advanced::subscription::Event>())
}

/// The audit id a relayed message carries. Any other message, a `TrailReady`
/// included, fails the test, naming it.
fn trail_updated_id(msg: &Message, what: &str) -> String {
    match msg {
        Message::TrailMirrorTick(TrailMirrorUiTick::TrailUpdated(id)) => id.to_string(),
        other => panic!("{what}: expected TrailMirrorTick(TrailUpdated), got {other:?}"),
    }
}

/// The next relayed tick's audit id under a 2 s [`next_or_fail`]: `Some(id)`
/// for a relayed tick, `None` if the stream ENDED.
async fn next_id(stream: &mut BoxStream<'static, Message>, what: &str) -> Option<String> {
    next_or_fail(stream, 2, what)
        .await
        .map(|msg| trail_updated_id(&msg, what))
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

// ── Unit tests (stream_impl) ─────────────────────────────────────────────────

/// T-TM-1a, strict: one `TrailUpdated` tick is relayed as a matching
/// `Message::TrailMirrorTick` with its audit id unchanged. Dropping the only
/// sender then ENDS the stream: `Ok(None)` within 300 ms, never a timeout.
///
/// Its pair over the senders is `trail_mirror_stream_stays_open_while_sender_alive`.
#[tokio::test]
async fn stream_impl_yields_event_then_done() {
    let (tx, rx) = trail_tick_pair();
    let mut stream = trail_mirror_stream_impl(rx);

    let sent_id = "audit-001";
    tx.send(TrailMirrorTick::TrailUpdated(sent_id.to_owned()))
        .expect("send must succeed: the relay's receiver is alive");
    let what = "trail: the audit id is forwarded unchanged";
    assert_eq!(
        next_id(&mut stream, what).await.as_deref(),
        Some(sent_id),
        "{what}"
    );

    drop(tx);
    let end = timeout(Duration::from_millis(300), stream.next()).await;
    assert!(
        matches!(end, Ok(None)),
        "trail: dropping the sender ENDS the stream after the relayed tick \
         (Ok(None) within 300 ms); got {end:?}"
    );
}

/// T-TM-1b, strict: the only sender is dropped BEFORE the stream is built, so
/// the relay's first `rx.recv()` reads `Closed` and the stream ENDS without
/// yielding: `Ok(None)` within 300 ms. This is the graceful-shutdown path (the
/// `TrailMirror` task exits before iced first polls the recipe).
///
/// Its pair over the senders is `trail_mirror_stream_stays_open_while_sender_alive`:
/// the same fixture with the sender kept alive is live, not ended.
#[tokio::test]
async fn stream_impl_on_already_closed_channel_ends_without_yielding() {
    let (tx, rx) = trail_tick_pair();
    drop(tx);
    let mut stream = trail_mirror_stream_impl(rx);

    let end = timeout(Duration::from_millis(300), stream.next()).await;
    assert!(
        matches!(end, Ok(None)),
        "trail: an already-closed channel ENDS the stream without yielding \
         (Ok(None) within 300 ms); got {end:?}"
    );
}

/// The negative control of the close tests, over the senders: with the sender
/// kept alive and nothing sent, the stream is LIVE, not ended. A tick sent then
/// is relayed, and the stream is live again afterwards.
///
/// A relay that reads an empty channel as closed passes every close test and
/// fails the first observation. One that ends itself after a quiet spell (a
/// receive under a timeout read as a close, E1-P35) fails the observation after
/// an hour of quiet on the paused clock. One that stops after its first tick
/// fails the last.
#[tokio::test(start_paused = true)]
async fn trail_mirror_stream_stays_open_while_sender_alive() {
    let (tx, rx) = trail_tick_pair();
    let mut stream = trail_mirror_stream_impl(rx);

    assert_pending(
        &mut stream,
        "trail: with a sender alive the stream is live, not ended",
    );

    // The relay is now parked on its receiver. An hour of quiet passes;
    // `advance` fires any timer the relay armed, with no real wait.
    tokio::time::advance(std::time::Duration::from_secs(3600)).await;
    tokio::task::yield_now().await;
    assert_pending(
        &mut stream,
        "trail: after an hour of quiet with a sender alive the stream is still live (paused clock)",
    );

    tx.send(TrailMirrorTick::TrailUpdated("audit-live".to_owned()))
        .expect("send must succeed: the relay's receiver is alive");
    let what = "trail: a tick sent to the live stream is relayed";
    assert_eq!(
        next_id(&mut stream, what).await.as_deref(),
        Some("audit-live"),
        "{what}"
    );
    assert_pending(
        &mut stream,
        "trail: after the relayed tick the stream is still live with nothing buffered",
    );
    // The sender outlives both polls, so the relay's channel was open at each.
    drop(tx);
}

/// T-TM-1c, strict: capacity 16, three `TrailUpdated` ticks buffered, then the
/// only sender dropped, all before the stream is built. tokio hands a closed
/// receiver its buffered values first (`Receiver::recv_ref` returns `Closed`
/// only once the channel is empty for that receiver), so the relay yields all
/// three in send order and then ENDS: exactly `["audit-a", "audit-b",
/// "audit-c"]`. A `TrailReady`, or any other message, fails the test.
///
/// This is the no-lag side of a pair over channel capacity: the capacity-2
/// test below loses the oldest ticks, this one relays every tick it is sent.
#[tokio::test]
async fn stream_impl_multiple_events_in_order() {
    const SENT: [&str; 3] = ["audit-a", "audit-b", "audit-c"];
    let (tx, rx) = trail_tick_pair();
    for id in SENT {
        tx.send(TrailMirrorTick::TrailUpdated(id.to_owned()))
            .expect("send must succeed: the relay's receiver is alive");
    }
    drop(tx);
    let mut stream = trail_mirror_stream_impl(rx);

    let what = "trail: all three ticks are relayed in send order, then the stream ENDS";
    let ids: Vec<String> = drain_to_end(&mut stream, what)
        .await
        .iter()
        .map(|msg| trail_updated_id(msg, what))
        .collect();
    assert_eq!(ids, SENT, "{what}");
}

/// Capacity 2, four ticks sent before the first poll: the ring keeps only the
/// newest two, so the relay's first `recv` returns `Lagged(2)` and tokio moves
/// its cursor to the oldest retained value (`Receiver::recv_ref`). The relay
/// must survive that (the R1.5 drop-on-lag policy): it yields `audit-3` and
/// `audit-4`, then a later `marker`, then is live.
///
/// The lag premise is READ OFF the output: `audit-1` and `audit-2` are absent,
/// whereas the capacity-16 test above relays every tick it is sent. The
/// retained ids are derived from the capacity, not from the stream.
#[tokio::test]
async fn trail_mirror_stream_continues_after_lag() {
    const CAPACITY: usize = 2;
    const SENT: [&str; 4] = ["audit-1", "audit-2", "audit-3", "audit-4"];
    let (tx, rx) = broadcast::channel::<TrailMirrorTick>(CAPACITY);
    let mut stream = trail_mirror_stream_impl(rx);
    for id in SENT {
        tx.send(TrailMirrorTick::TrailUpdated(id.to_owned()))
            .expect("send must succeed: the relay's receiver is alive");
    }

    let what = "trail lag: after Lagged(2) the retained ticks audit-3, audit-4 then the marker are relayed";
    // tokio keeps the newest CAPACITY values; the receiver lags by the rest.
    let expected: Vec<Option<String>> = SENT[SENT.len() - CAPACITY..]
        .iter()
        .map(|id| Some((*id).to_owned()))
        .collect();
    let mut retained = Vec::new();
    for _ in 0..CAPACITY {
        retained.push(next_id(&mut stream, what).await);
    }
    assert_eq!(retained, expected, "{what}");

    tx.send(TrailMirrorTick::TrailUpdated("marker".to_owned()))
        .expect("send must succeed: the relay's receiver is alive");
    assert_eq!(
        next_id(&mut stream, what).await.as_deref(),
        Some("marker"),
        "{what}"
    );
    assert_pending(
        &mut stream,
        "trail lag: after the marker nothing else is buffered and the stream is still live",
    );
    drop(tx);
}

// ── Integration test (the real Recipe) ───────────────────────────────────────

/// T-TM-1d: `<TrailMirrorRecipe as Recipe>::stream`, reached through the
/// production constructor `ui::live::trail_mirror_subscription` and iced's
/// `into_recipes` (the struct is private). The constructor batches exactly one
/// recipe. Its `stream()` subscribes EAGERLY to the handle's own sender: one
/// more receiver right after `stream()` returned, before the first poll. A tick
/// sent before that poll is relayed with its audit id unchanged. Dropping the
/// last sender then ends the stream with nothing more relayed.
#[tokio::test]
async fn trail_mirror_recipe_stream_end_to_end() {
    let (tick_tx, _dummy_rx) = broadcast::channel::<TrailMirrorTick>(16);
    let (handle, _req_rx) = make_handle(tick_tx.clone());

    let mut recipes = into_recipes(ui::live::trail_mirror_subscription(handle));
    assert_eq!(
        recipes.len(),
        1,
        "trail recipe: trail_mirror_subscription batches exactly one recipe"
    );
    let recipe = recipes
        .pop()
        .expect("the one recipe the length check counted");

    // Not asserted: it is 1 by construction (`_dummy_rx`). It is only the
    // baseline the eager subscribe adds to.
    let before = tick_tx.receiver_count();
    let mut stream = Recipe::stream(recipe, empty_event_stream());
    assert_eq!(
        tick_tx.receiver_count(),
        before + 1,
        "trail recipe: `Recipe::stream` subscribed EAGERLY to the handle's own sender"
    );

    let sent_id = "audit-e2e-001";
    tick_tx
        .send(TrailMirrorTick::TrailUpdated(sent_id.to_owned()))
        .expect("send must succeed: the channel has live receivers");
    let what = "trail recipe: the tick sent before the first poll is relayed";
    assert_eq!(
        next_id(&mut stream, what).await.as_deref(),
        Some(sent_id),
        "{what}"
    );

    // The handle's sender clone was consumed with the recipe, so this is the
    // channel's last sender.
    drop(tick_tx);
    let what = "trail recipe: dropping the last sender ENDS the stream with nothing more relayed";
    let tail = drain_to_end(&mut stream, what).await;
    assert!(tail.is_empty(), "{what}; got {tail:?}");
}
