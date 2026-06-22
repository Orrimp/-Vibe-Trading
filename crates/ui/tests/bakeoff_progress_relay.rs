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
//! progress state at "X of N" (the determinate bar's data). This is the proof the
//! fixtures cannot give — that the receiver is actually consumed (no dead `_rx`).
//!
//! Gated on `live` (the only build where the recipe wiring lives, mirroring
//! `forward_narration_relay.rs`), NOT on `target_os` (no pixels here — pure
//! channel/state logic, deterministic on every OS).

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use futures::StreamExt;
use smol_str::SmolStr;
use tokio::time::timeout;

use backtest::progress::{BakeoffProgress, bakeoff_progress_pair};
use ui::state::{Cockpit, Message, update};

/// **The last-mile proof.** Feeding a `BakeoffProgress` into the bake-off
/// progress channel makes the relay yield `Message::BakeoffProgress`, AND
/// applying that message via `update` lands the leaderboard's progress state
/// at the fed value (the determinate bar's "X of N" data). This is the exact
/// path that would otherwise be dead — the receiver never consumed.
#[tokio::test]
async fn bakeoff_progress_relay_yields_message_and_populates_state() {
    let (tx, rx) = bakeoff_progress_pair();
    let mut stream = ui::live::bakeoff_progress_stream_impl(Some(rx));

    // `done = 3, total = 7` → the 4th candidate ("v0.5.bbands") is now running.
    let sent = BakeoffProgress {
        done: 3,
        total: 7,
        current_id: SmolStr::new("v0.5.bbands"),
    };
    tx.try_send(sent);

    let msg = timeout(Duration::from_secs(2), stream.next())
        .await
        .expect("a Message arrived within 2s")
        .expect("the relay yielded a Message");

    // 1) The relay maps the engine BakeoffProgress → Message::BakeoffProgress
    //    with the fields carried faithfully.
    let progress = match &msg {
        Message::BakeoffProgress(p) => p.clone(),
        other => panic!("expected BakeoffProgress, got {other:?}"),
    };
    assert_eq!(progress.done, 3, "done carried");
    assert_eq!(progress.total, 7, "total carried");
    assert_eq!(
        progress.current_id.as_str(),
        "v0.5.bbands",
        "current_id carried"
    );

    // 2) Applying the Message populates the leaderboard progress state. A run is
    //    in flight, so begin_run() first (mirrors the real BakeoffRunRequested
    //    pure-state half), THEN the progress event lands.
    let mut cockpit = Cockpit::new();
    cockpit.leaderboard_screen_state.begin_run();
    assert!(
        cockpit.leaderboard_screen_state.progress.is_none(),
        "precondition: begin_run clears progress (no stale bar)"
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
    // The determinate bar reads `done + 1` of `total` = "4 of 7" and fills
    // `done / total` ≈ 0.4286 — the X-of-N the operator asked for.
    let one_based = u32::from(p.done) + 1;
    assert_eq!(one_based, 4, "the running candidate is the 4th of 7");
    let fill = f32::from(p.done) / f32::from(p.total);
    assert!(
        (fill - 0.428_571_4).abs() < 1e-4,
        "fill is done/total ≈ 0.4286 (got {fill})"
    );
}

/// Multiple progress events arrive in order; the LATEST one wins on the state
/// (the determinate bar advances). Proves the relay forwards every event and
/// `update` keeps only the most recent (the bar never goes backwards across a
/// monotone sequence).
#[tokio::test]
async fn bakeoff_progress_relay_latest_event_wins_on_state() {
    let (tx, rx) = bakeoff_progress_pair();
    let mut stream = ui::live::bakeoff_progress_stream_impl(Some(rx));

    let mut cockpit = Cockpit::new();
    cockpit.leaderboard_screen_state.begin_run();

    // Three monotone events (channel capacity = 8).
    for (done, id) in [(0u16, "v0.sma"), (1, "v0.5.macd"), (2, "v0.5.rsi")] {
        tx.try_send(BakeoffProgress {
            done,
            total: 7,
            current_id: SmolStr::new(id),
        });
    }
    drop(tx); // close so the stream terminates after draining

    let mut last_done = None;
    loop {
        match timeout(Duration::from_secs(2), stream.next()).await {
            Ok(Some(msg @ Message::BakeoffProgress(_))) => {
                if let Message::BakeoffProgress(ref p) = msg {
                    last_done = Some(p.done);
                }
                update(&mut cockpit, msg);
            }
            Ok(Some(other)) => panic!("unexpected message: {other:?}"),
            Ok(None) => break, // channel closed, all drained
            Err(_) => break,
        }
    }

    assert_eq!(last_done, Some(2), "the last event drained was done=2");
    let p = cockpit
        .leaderboard_screen_state
        .progress
        .as_ref()
        .expect("state holds the latest progress");
    assert_eq!(p.done, 2, "the LATEST event wins on the state");
    assert_eq!(p.current_id.as_str(), "v0.5.rsi", "latest id wins");
}

/// The relay terminates cleanly when the sender drops (channel closed) — no
/// hang, no panic. Mirrors the `LabProgressRecipe` close behaviour.
#[tokio::test]
async fn bakeoff_progress_relay_terminates_on_sender_drop() {
    let (tx, rx) = bakeoff_progress_pair();
    let mut stream = ui::live::bakeoff_progress_stream_impl(Some(rx));
    drop(tx);
    let next = timeout(Duration::from_secs(2), stream.next())
        .await
        .expect("the stream resolved within 2s");
    assert!(next.is_none(), "the relay ends when the channel closes");
}

/// `None` receiver (the double-`take()` case) yields nothing — the silent-empty
/// guard the `lab_progress_recipe_stream.rs` smoking-gun test pins (a bar that
/// never advances because the stream silently yields nothing).
#[tokio::test]
async fn bakeoff_progress_relay_none_yields_nothing() {
    let mut stream = ui::live::bakeoff_progress_stream_impl(None);
    let next = timeout(Duration::from_secs(1), stream.next())
        .await
        .expect("resolved within 1s");
    assert!(next.is_none(), "None receiver ⇒ empty stream");
}

/// **Recipe-path integration.** Exercises the FULL `BakeoffProgressRecipe::stream()`
/// (take via `Arc<Mutex<Option<_>>>` then delegate to `bakeoff_progress_stream_impl`)
/// — the exact code path `cockpit_live::subscription()` uses. If `stream()` were
/// ever called twice with the same Arc (a salt-bump bug), the second `take()`
/// would return `None` and this test would fail to get any progress.
#[tokio::test]
async fn bakeoff_progress_recipe_stream_end_to_end() {
    use std::sync::{Arc, Mutex};

    use iced::advanced::subscription::{EventStream, Recipe};
    use ui::live::BakeoffProgressRecipe;

    let rt = tokio::runtime::Handle::current();
    let (tx, rx) = bakeoff_progress_pair();
    let rx_arc = Arc::new(Mutex::new(Some(rx)));

    let recipe = BakeoffProgressRecipe {
        rt_handle: rt,
        rx: Arc::clone(&rx_arc),
        salt: 1,
    };

    let event_stream: EventStream =
        Box::pin(futures::stream::empty::<iced::advanced::subscription::Event>());
    let mut stream = Box::new(recipe).stream(event_stream);

    tx.try_send(BakeoffProgress {
        done: 5,
        total: 7,
        current_id: SmolStr::new("v0.8.vote.majority"),
    });

    let first = timeout(Duration::from_secs(2), stream.next())
        .await
        .expect("BakeoffProgress arrived within 2s")
        .expect("stream produced a message");

    match first {
        Message::BakeoffProgress(p) => {
            assert_eq!(
                p.done, 5,
                "BakeoffProgressRecipe::stream() must forward BakeoffProgress fields unchanged"
            );
            assert_eq!(p.current_id.as_str(), "v0.8.vote.majority");
        }
        other => panic!("expected BakeoffProgress, got {other:?}"),
    }

    // The Arc now holds None (the receiver was taken once) — the salt-bump
    // contract that prevents a silent double-take.
    assert!(
        rx_arc.lock().unwrap().is_none(),
        "after Recipe::stream() calls take(), the Arc<Mutex<Option<_>>> holds None"
    );
}
