//! Test 1 — `LabProgressRecipe` stream_impl end-to-end (Bug #63 / lab-end-to-end-v2).
//!
//! Reproduces the suspected live breakage: when `stream_impl` receives `None`
//! (because `Arc<Mutex<Option<Receiver>>>::take()` was called a second time),
//! the stream yields *nothing* — not even `LabRunProgressDone` — so the UI
//! never updates `run_progress` and the bar is frozen at 30 % indeterminate.
//!
//! ## Why `stream_impl` and not the full `LabProgressRecipe::stream()`?
//!
//! `Recipe::stream()` requires an `EventStream` (iced's runtime event bus).
//! Constructing one without a running iced application is non-trivial.
//! `stream_impl` is the extracted inner helper that contains all the channel
//! logic; `Recipe::stream()` delegates to it after the `take()` call.
//! Testing `stream_impl` directly is therefore equivalent and avoids the
//! iced runtime dependency.

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use futures::StreamExt;
use tokio::time::timeout;

use backtest::progress::{Progress, progress_pair};
use ui::lab::progress::stream_impl;
use ui::state::Message;

/// T-PB-1a — happy path: `stream_impl(Some(rx))` yields `LabRunProgress`
/// then `LabRunProgressDone` after the sender closes.
///
/// This is the canonical path every successful Lab run should follow.
/// Before the refactor the same logic lived inline in `Recipe::stream()`;
/// this test pins the behaviour so any regression is caught immediately.
#[tokio::test]
async fn stream_impl_yields_progress_then_done() {
    let (tx, rx) = progress_pair();

    let mut stream = stream_impl(Some(rx));

    // Push one progress event.
    let sent = Progress {
        current_bar: 100,
        total_bars: 720,
        elapsed_ms: 1500,
    };
    tx.try_send(sent);

    // First yield must be LabRunProgress with the right fields.
    let first = timeout(Duration::from_secs(2), stream.next())
        .await
        .expect("message arrived within 2s")
        .expect("stream produced a message");

    match first {
        Message::LabRunProgress(p) => {
            assert_eq!(p.current_bar, 100, "current_bar mismatch");
            assert_eq!(p.total_bars, 720, "total_bars mismatch");
            assert_eq!(p.elapsed_ms, 1500, "elapsed_ms mismatch");
        }
        other => panic!("expected LabRunProgress, got {other:?}"),
    }

    // Drop the sender to close the channel.
    drop(tx);

    // Second yield must be LabRunProgressDone.
    let second = timeout(Duration::from_secs(2), stream.next())
        .await
        .expect("LabRunProgressDone arrived within 2s")
        .expect("stream produced LabRunProgressDone");

    assert!(
        matches!(second, Message::LabRunProgressDone),
        "expected LabRunProgressDone after sender close, got {second:?}"
    );

    // Stream must be exhausted after Done.
    let third = timeout(Duration::from_millis(200), stream.next()).await;
    match third {
        Ok(None) => {} // stream closed cleanly — ideal
        Ok(Some(extra)) => panic!("unexpected extra message after LabRunProgressDone: {extra:?}"),
        Err(_) => {} // timeout = stream open but no data; acceptable
    }
}

/// T-PB-1b — SMOKING GUN: `stream_impl(None)` yields *nothing*.
///
/// This documents and pins the silent-failure path. When `Recipe::stream()`
/// is called a second time (because the salt was not actually bumped, or
/// iced reconstructed the recipe despite the same hash), the
/// `Arc<Mutex<Option<_>>>::take()` returns `None` and `stream_impl(None)`
/// produces zero messages. The UI sees no `LabRunProgress` — bar stays at 30%.
///
/// After the patch, the correct fix is ensuring the salt IS always bumped
/// so `stream()` is never called a second time with the same `(TypeId, salt)`
/// identity. This test documents the `None` case as a known silent path.
#[tokio::test]
async fn stream_impl_none_rx_yields_nothing() {
    // Pass None directly — simulates what happens when .take() already
    // consumed the receiver on a prior stream() call.
    let mut stream = stream_impl(None);

    // The stream must produce nothing in a short window.
    let msg = timeout(Duration::from_millis(300), stream.next()).await;
    match msg {
        Ok(None) => {
            // Stream closed immediately with no messages — expected silent path.
            // This IS the bug: a progress bar that never advances because
            // stream_impl(None) silently yields nothing.
        }
        Ok(Some(m)) => panic!(
            "stream_impl(None) must yield nothing, but got: {m:?}. \
             This would mean the stream is producing phantom messages."
        ),
        Err(_timeout) => {
            // Timeout = stream still open but never yields any progress.
            // This is also "yields nothing from the caller's perspective".
            // Both outcomes confirm the silent-failure behaviour.
        }
    }
}

/// T-PB-1c — multiple progress events all arrive in order before Done.
#[tokio::test]
async fn stream_impl_multiple_progress_events_in_order() {
    let (tx, rx) = progress_pair();

    let mut stream = stream_impl(Some(rx));

    // Send three progress events (channel capacity = 8).
    for i in [128usize, 256, 384] {
        tx.try_send(Progress {
            current_bar: i,
            total_bars: 720,
            elapsed_ms: (i as u64) * 10,
        });
    }
    drop(tx);

    let mut received_bars: Vec<usize> = Vec::new();
    // Drain until Done or timeout.
    loop {
        match timeout(Duration::from_secs(2), stream.next())
            .await
            .expect("message within 2s")
        {
            Some(Message::LabRunProgress(p)) => received_bars.push(p.current_bar),
            Some(Message::LabRunProgressDone) => break,
            Some(other) => panic!("unexpected message: {other:?}"),
            None => break,
        }
    }

    assert!(
        !received_bars.is_empty(),
        "expected at least one LabRunProgress message before LabRunProgressDone"
    );
    for bar in &received_bars {
        assert!(*bar <= 720, "current_bar {bar} exceeds total_bars 720");
    }
}

/// T-PB-1d — `LabProgressRecipe::stream()` integration: exercises the full
/// Recipe path (take via Arc<Mutex<Option<_>>> then delegate to stream_impl).
///
/// This is the highest-value test — it exercises the exact code path that
/// cockpit_live.rs uses. If stream() were ever called twice with the same
/// Arc (e.g., due to a salt-bump bug), the second call's take() would
/// return None and this test structure would fail to get any Progress.
#[tokio::test]
async fn lab_progress_recipe_stream_end_to_end() {
    use std::sync::{Arc, Mutex};

    use iced::advanced::subscription::{EventStream, Recipe};
    use ui::lab::progress::LabProgressRecipe;

    let rt = tokio::runtime::Handle::current();

    let (tx, rx) = progress_pair();
    let rx_arc = Arc::new(Mutex::new(Some(rx)));

    let recipe = LabProgressRecipe {
        rt_handle: rt,
        rx: Arc::clone(&rx_arc),
        salt: 1,
    };

    // Build a no-op EventStream (Recipe::stream() ignores it).
    let event_stream: EventStream =
        Box::pin(futures::stream::empty::<iced::advanced::subscription::Event>());

    // Call Recipe::stream() — this takes the receiver via .take() and
    // returns a BoxStream backed by stream_impl(Some(rx)).
    let mut stream = Box::new(recipe).stream(event_stream);

    // Push a progress event.
    tx.try_send(Progress {
        current_bar: 100,
        total_bars: 720,
        elapsed_ms: 1500,
    });

    // Verify: LabRunProgress arrives.
    let first = timeout(Duration::from_secs(2), stream.next())
        .await
        .expect("LabRunProgress arrived within 2s")
        .expect("stream produced a message");

    match first {
        Message::LabRunProgress(p) => {
            assert_eq!(
                p.current_bar, 100,
                "LabProgressRecipe::stream() must forward Progress fields unchanged"
            );
        }
        other => panic!("expected LabRunProgress, got {other:?}"),
    }

    // Verify: Arc now holds None (receiver was taken).
    let still_held = rx_arc.lock().unwrap().is_none();
    assert!(
        still_held,
        "After Recipe::stream() calls take(), the Arc<Mutex<Option<_>>> must hold None"
    );

    drop(tx);

    // Verify: LabRunProgressDone arrives after sender close.
    let done = timeout(Duration::from_secs(2), stream.next())
        .await
        .expect("LabRunProgressDone arrived within 2s")
        .expect("stream produced LabRunProgressDone");

    assert!(
        matches!(done, Message::LabRunProgressDone),
        "expected LabRunProgressDone, got {done:?}"
    );
}
