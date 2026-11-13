//! T-D-D2: `activity_stream_impl` boundary tests (the relay `ActivityRecipe`
//! returns).
//!
//! `activity_stream_impl` (`crates/ui/src/live.rs`) is the stream
//! `<ActivityRecipe as Recipe>::stream` hands to iced: a loop over the activity
//! broadcast receiver with three arms. These tests pin each arm by what comes
//! OUT of the stream:
//! - `Ok(event)` relays one `Message::ActivityEventReceived` per event, in
//!   send order;
//! - `Lagged(n)` is survived: the retained events, and every later one, are
//!   still relayed;
//! - `Closed` ends the stream, but only once every sender is gone, and only
//!   after the events buffered before the close are relayed.
//!
//! ## What this file does not pin: the Recipe's eager subscribe
//!
//! The Recipe's own line is the subscribe it runs BEFORE the stream is first
//! polled (`let rx = self.bus.activity().subscribe();` in
//! `<ActivityRecipe as Recipe>::stream`). Story 2-63 Wave B pins it in
//! `crates/ui/tests/recipe_off_runtime_poll.rs`:
//! `activity_recipe_subscribes_before_its_first_poll` (probe B-P19) publishes
//! between `stream()` and the first poll, and `activity_recipe_drains_off_runtime`
//! drains the full `Recipe::stream` on iced's executor. Every test here builds
//! its own receiver and hands it to `activity_stream_impl`, so a lazy subscribe
//! in the Recipe leaves this file green, by design.
//!
//! ## Non-vacuity contract (bug-log #66 / story 2-63 AC3)
//!
//! Every test proves it RAN: the exact relayed ids in order, and a stream end
//! observed as `Ok(None)` inside a timeout that FAILS the test. No timeout is
//! ever accepted as "drained": [`next_or_fail`] panics on elapse. "Nothing more
//! is buffered and the stream is live" is OBSERVED with [`assert_pending`],
//! never read off a counter the test increments itself. The claims come in
//! two-sided pairs:
//! - over channel capacity: capacity 16 relays every event it is sent, while
//!   capacity 2 drops the oldest two. The lag premise is read off the output,
//!   not assumed;
//! - over the senders: dropping every sender ends the stream, while a live
//!   sender keeps it open, at once and after an hour of quiet on a paused
//!   clock.
//!
//! ## T-T4 falsification probe table (story 2-63 AC4, Wave E1)
//!
//! Each probe is ONE edit to production code, located by symbol + the quoted
//! line text (line numbers rot). Procedure:
//!
//! 1. snapshot the file (`cp -p <file> <snap>`) and apply the edit;
//! 2. run `cargo test -p ui --test activity_recipe_stream`. The named test(s)
//!    go RED;
//! 3. restore from the snapshot, THEN `touch <file>`. A restore that keeps the
//!    snapshot's older mtime (`cp -p`) leaves cargo's fingerprint "fresh": the
//!    next run compiles nothing and re-executes the MUTANT binary;
//! 4. re-run. The GREEN counts only if that run's output shows
//!    `Compiling ui` and the file's SHA-256 equals the pre-probe one.
//!
//! Probe ids are the Wave E1 series, shared across the seven v0.2
//! recipe-stream files E1 sweeps; this file's rows are E1-P1, E1-P2, E1-P27
//! and E1-P34. All four edit `ui::live::activity_stream_impl`
//! (`crates/ui/src/live.rs`). E1-P34 is the E1a falsifier's measured survivor
//! AT-OWN-1, routed back here as a designed probe.
//!
//! | # | symbol (file) | quoted line today | mutation | expected RED |
//! |---|---|---|---|---|
//! | E1-P1 | `ui::live::activity_stream_impl`, the `Lagged(n)` arm (`crates/ui/src/live.rs`) | `// Continue: lag is not fatal. The tape may show stale` | append `break;` after the comment's next line, `// state until the next event lands (R6.3 contract).` | `stream_survives_lagged_and_delivers_retained_events`, at "activity lag: after Lagged(2) the retained events 3 and 4 are relayed" (left `[None, None]`: the stream ENDED at the lag) |
//! | E1-P2 | `ui::live::activity_stream_impl`, the `Closed` arm | `debug!(target: "ui.activity", "activity broadcast closed — stopping recipe");` | the `break;` below it → `std::future::pending::<()>().await;` | `stream_terminates_on_sender_close`, at "activity: dropping every sender ENDS the stream (Ok(None) within 500 ms)" (`Err(Elapsed)`); `stream_yields_buffered_event_then_ends_on_close`, whose drain times out after event 7 ("timed out after 2s waiting for activity: a buffered event is relayed before the close ends the stream"); `stream_stays_open_while_sender_alive`, whose last read times out ("timed out after 2s waiting for activity: the close then ends it") |
//! | E1-P27 | `ui::live::activity_stream_impl`, the receive | `Ok(event) => yield Message::ActivityEventReceived(event),` | the line above it, `match rx.recv().await {` → `match rx.try_recv().map_err(\|_\| tokio::sync::broadcast::error::RecvError::Closed) {` (an empty channel, a lag and a close all end the loop) | `stream_stays_open_while_sender_alive`, at "activity: with a sender alive and nothing sent the stream is live, not ended" (the stream already ENDED); `stream_yields_activity_events_in_send_order`, at its trailing "activity: after the 3 sent events nothing else is buffered and the stream is still live"; `stream_survives_lagged_and_delivers_retained_events`, at "activity lag: after Lagged(2) the retained events 3 and 4 are relayed" (the lag is mapped to `Closed`, so left `[None, None]`) |
//! | E1-P34 | `ui::live::activity_stream_impl`, the receive | `Ok(event) => yield Message::ActivityEventReceived(event),` | the line above it, `match rx.recv().await {` → `match tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv()).await.unwrap_or(Err(tokio::sync::broadcast::error::RecvError::Closed)) {` (the relay ENDS after 100 ms of quiet with a sender alive) | `stream_stays_open_while_sender_alive`, at its second `assert_pending`, "activity: after an hour of quiet with a sender alive the stream is still live (paused clock)" (the stream already ENDED). Every other test reads at t≈0, so this is the only red: before E1a's paused-clock observation the mutant passed 5/5 |
//!
//! The tests each probe leaves GREEN are the other side of its pair: under
//! E1-P1 nothing else lags; under E1-P2 nothing else closes; under E1-P27 the
//! two close tests still end (`try_recv` relays the buffered 7, then reads
//! `Closed`); under E1-P34 every read the other tests make is immediate.
//! Outside this target (MEASURED by the E1a falsifier): the lib test
//! `live::tests::activity_recipe_handles_lag` is red under E1-P1 and E1-P27.
//!
//! ## Coverage
//!
//! | origin | test | what it pins |
//! |---|---|---|
//! | v0.2 D2-T1, tightened | `stream_yields_activity_events_in_send_order` | capacity 16, three events sent before the first poll: ids `[1, 2, 3]` in send order, then live with nothing buffered |
//! | E1, replaces v0.2's lag test | `stream_survives_lagged_and_delivers_retained_events` | capacity 2, four events: the relay survives `Lagged(2)` and yields `[3, 4]`, then a later event 99, then is live |
//! | v0.2 D2-T3, strict | `stream_terminates_on_sender_close` | every sender dropped: `Ok(None)` within 500 ms, nothing relayed |
//! | E1, negative control | `stream_stays_open_while_sender_alive` | the same fixture with a live sender and nothing sent is live, not ended, at once and after an hour of quiet (paused clock); dropping the senders then ends it |
//! | E1 | `stream_yields_buffered_event_then_ends_on_close` | an event buffered before the close is relayed, then the end: exactly `[7]` |

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use agent::EventBus;
use agent::activity::{ActivityEvent, ActivityId, ActivityKind, ActivityPhase, ActivitySender};
use agent::config::BusConfig;
use futures::stream::BoxStream;
use futures::{FutureExt, StreamExt};
use tokio::sync::broadcast;
use tokio::time::timeout;
use ui::live::activity_stream_impl;
use ui::state::Message;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Build a minimal `ActivityEvent` with the given monotone ID.
///
/// Uses a raw broadcast channel (not `EventBus`) so we can send without
/// going through the RAII `ActivitySender::start()` path (which forces a
/// specific event shape and auto-emits End on drop).
fn make_event(id_val: u64) -> ActivityEvent {
    ActivityEvent {
        id: ActivityId(id_val),
        kind: ActivityKind::LabRun,
        label: format!("test-event-{id_val}"),
        phase: ActivityPhase::Start { total_units: None },
        ts_ms: 0,
    }
}

/// The id a relayed message carries. Any other message variant fails the
/// test, naming it.
fn event_id(msg: &Message, what: &str) -> u64 {
    match msg {
        Message::ActivityEventReceived(ev) => ev.id.0,
        other => panic!("{what}: expected ActivityEventReceived, got {other:?}"),
    }
}

/// The next relayed event's id under a 2 s [`next_or_fail`]: `Some(id)` for a
/// relayed event, `None` if the stream ENDED.
async fn next_id(stream: &mut BoxStream<'static, Message>, what: &str) -> Option<u64> {
    next_or_fail(stream, 2, what)
        .await
        .map(|msg| event_id(&msg, what))
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

/// The production fixture of the two sender tests: a real `EventBus`, its
/// `ActivitySender`, and the relay over a receiver subscribed from that
/// sender. The bus's `activity_tx` and the sender's clone of it are the
/// channel's only senders (`EventBus::new` spawns no task).
fn bus_relay() -> (EventBus, ActivitySender, BoxStream<'static, Message>) {
    let bus = EventBus::new(&BusConfig::default());
    let sender = bus.activity();
    let stream = activity_stream_impl(sender.subscribe());
    (bus, sender, stream)
}

// ── Ok(event): relay in send order ───────────────────────────────────────────

/// D2-T1: capacity 16, three events sent before the first poll. The relay
/// yields their ids in send order, then is LIVE with nothing buffered.
///
/// This is the no-lag side of a pair over channel capacity: the capacity-2
/// test below loses the oldest events, this one relays every event it is sent.
#[tokio::test]
async fn stream_yields_activity_events_in_send_order() {
    const SENT: [u64; 3] = [1, 2, 3];
    let (tx, rx) = broadcast::channel::<ActivityEvent>(16);
    let mut stream = activity_stream_impl(rx);
    for id in SENT {
        tx.send(make_event(id))
            .expect("send must succeed: the relay's receiver is alive");
    }

    let what = "activity: the 3 sent events are relayed in send order";
    let mut ids = Vec::new();
    for _ in SENT {
        ids.push(next_id(&mut stream, what).await);
    }
    assert_eq!(ids, SENT.map(Some), "{what}");
    assert_pending(
        &mut stream,
        "activity: after the 3 sent events nothing else is buffered and the stream is still live",
    );
    // The sender outlives that poll, so the relay's channel was open there.
    drop(tx);
}

// ── Lagged(n): survived ──────────────────────────────────────────────────────

/// Capacity 2, four events sent before the first poll: the ring keeps only the
/// newest two, so the relay's first `recv` returns `Lagged(2)` and tokio moves
/// its cursor to the oldest retained value (`Receiver::recv_ref`). The relay
/// must survive that: it yields 3 and 4, then a later event 99, then is live.
///
/// The lag premise is READ OFF the output: ids 1 and 2 are absent, whereas the
/// capacity-16 test above relays every event it is sent. This replaces v0.2's
/// lag test, which subscribed after its sends (`subscribe` skips earlier
/// values, so it never lagged) and accepted any outcome.
#[tokio::test]
async fn stream_survives_lagged_and_delivers_retained_events() {
    let (tx, rx) = broadcast::channel::<ActivityEvent>(2);
    let mut stream = activity_stream_impl(rx);
    for id in 1..=4 {
        tx.send(make_event(id))
            .expect("send must succeed: the relay's receiver is alive");
    }

    let what = "activity lag: after Lagged(2) the retained events 3 and 4 are relayed";
    let retained = [
        next_id(&mut stream, what).await,
        next_id(&mut stream, what).await,
    ];
    assert_eq!(retained, [Some(3), Some(4)], "{what}");

    tx.send(make_event(99))
        .expect("send must succeed: the relay's receiver is alive");
    let what = "activity lag: the stream keeps relaying after a lag (event 99)";
    assert_eq!(next_id(&mut stream, what).await, Some(99), "{what}");
    assert_pending(
        &mut stream,
        "activity lag: after event 99 nothing else is buffered and the stream is still live",
    );
    drop(tx);
}

// ── Closed: the end, and only on a close ─────────────────────────────────────

/// D2-T3, strict: dropping the bus and its `ActivitySender`, the channel's
/// only senders, ends the stream: `Ok(None)` within 500 ms, with nothing
/// relayed. The cockpit must not hang on the activity relay when the agent
/// drops the bus at shutdown.
///
/// Its pair over the senders is `stream_stays_open_while_sender_alive`: the
/// same fixture with the senders kept alive is live, not ended.
#[tokio::test]
async fn stream_terminates_on_sender_close() {
    let (bus, sender, mut stream) = bus_relay();
    drop(sender);
    drop(bus);

    let end = timeout(Duration::from_millis(500), stream.next()).await;
    assert!(
        matches!(end, Ok(None)),
        "activity: dropping every sender ENDS the stream (Ok(None) within 500 ms); got {end:?}"
    );
}

/// The negative control of the strict close test, over the senders: the same
/// fixture with the bus and its `ActivitySender` kept alive and nothing sent
/// is LIVE, not ended, at once AND after an hour of quiet on a paused clock.
/// Dropping them then ends it.
///
/// A relay that ends without a close (one that reads an empty channel as
/// closed) passes the strict close test and fails the first observation. One
/// that ends itself after a quiet spell (a receive under a timeout read as a
/// close, E1-P34) passes the first and fails the second: the clock is paused,
/// so `advance` fires any such timer deterministically, with no real wait.
#[tokio::test(start_paused = true)]
async fn stream_stays_open_while_sender_alive() {
    let (bus, sender, mut stream) = bus_relay();

    // One scheduler turn before the observation. Nothing else runs on this
    // runtime, so the poll below sees the relay alone, parked on its receiver.
    tokio::task::yield_now().await;
    assert_pending(
        &mut stream,
        "activity: with a sender alive and nothing sent the stream is live, not ended",
    );

    // The relay is now parked on its receiver. An hour of quiet passes.
    tokio::time::advance(Duration::from_secs(3600)).await;
    tokio::task::yield_now().await;
    assert_pending(
        &mut stream,
        "activity: after an hour of quiet with a sender alive the stream is still live (paused clock)",
    );

    drop(sender);
    drop(bus);
    let what = "activity: the close then ends it";
    let after_close = next_or_fail(&mut stream, 2, what).await;
    assert!(after_close.is_none(), "{what}; got {after_close:?}");
}

/// A close with an event buffered: id 7 is sent, then the only sender is
/// dropped, both before the first poll. tokio hands a closed receiver its
/// buffered values first (`Receiver::recv_ref` returns `Closed` only once the
/// channel is empty for that receiver), so the relay yields 7 and then ends:
/// exactly `[7]`.
///
/// A relay that handled `Closed` before draining would yield `[]`; one that
/// never ends fails the drain's 2 s step.
#[tokio::test]
async fn stream_yields_buffered_event_then_ends_on_close() {
    let (tx, rx) = broadcast::channel::<ActivityEvent>(16);
    let mut stream = activity_stream_impl(rx);
    tx.send(make_event(7))
        .expect("send must succeed: the relay's receiver is alive");
    drop(tx);

    let what = "activity: a buffered event is relayed before the close ends the stream";
    let ids: Vec<u64> = drain_to_end(&mut stream, what)
        .await
        .iter()
        .map(|msg| event_id(msg, what))
        .collect();
    assert_eq!(ids, [7], "{what}");
}
