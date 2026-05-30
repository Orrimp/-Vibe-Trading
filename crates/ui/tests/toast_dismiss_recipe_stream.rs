//! T-D-C3 — `ToastDismissRecipe` stream boundary tests (Surface 1).
//!
//! Drives `ui::live::toast_dismiss_stream_impl` under
//! `#[tokio::test(flavor = "current_thread", start_paused = true)]` +
//! `tokio::time::advance(500ms) × N` and asserts N `Message::ToastTick`
//! messages arrive with monotone `Instant` values.
//!
//! ## Why `toast_dismiss_stream_impl` and not the full `Recipe::stream()`?
//!
//! `Recipe::stream()` requires an `EventStream` (iced's runtime event bus).
//! Constructing one without a running iced application is non-trivial.
//! `toast_dismiss_stream_impl` is the extracted inner helper that contains all
//! the interval logic; `Recipe::stream()` delegates to it directly.
//! Testing `stream_impl` directly is equivalent and avoids the iced runtime
//! dependency for the unit-level tests.
//!
//! ## Time-control protocol
//!
//! `tokio::time::interval(500ms)` fires its FIRST tick immediately (at t=0).
//! The stream body skips this first tick via the leading `interval.tick().await`
//! before entering the loop.  Under paused time:
//!
//! 1. Spawn the stream poller task.
//! 2. `yield_now()` — task runs, consumes the immediate t=0 tick (skip), enters
//!    loop, awaits the t=500ms tick (blocks).
//! 3. For each tick wanted: `advance(500ms)` + `yield_now()` → task fires, forwards tick.
//! 4. Collect ticks from the mpsc receiver.
//!
//! This sequence gives exactly one tick per 500ms advance, matching the always-on
//! 500ms auto-dismiss cadence in production.
//!
//! ## T-T4 falsification probe (D-V0.2.0-3 row 7)
//!
//! **Probe**: in `crates/ui/src/live.rs`, inside `toast_dismiss_stream_impl`,
//! comment out the line `yield Message::ToastTick(Instant::now());`
//! (currently the only yield in the loop body).
//!
//! **Expected failure**: the spawned poller task never sends anything to the
//! channel.  After 3 × advance(500ms) + yield_now(), `ticks` is empty →
//! `assert_eq!(ticks.len(), 3)` fails with `left: 0, right: 3`.
//!
//! **Restore**: reinstate the `yield` line verbatim; all tests PASS.
//!
//! ## Coverage
//!
//! | Test ID  | What it pins                                                           |
//! |----------|------------------------------------------------------------------------|
//! | C3-T1    | Stream yields ToastTick every 500 ms under paused time (3 ticks)       |
//! | C3-T2    | Tick Instants are monotonically non-decreasing                         |
//! | C3-T3    | Stream remains open after N ticks (never terminates spontaneously)     |
//!
//! `#[cfg(feature = "live")]` gates the whole file since `toast_dismiss_stream_impl`
//! is only compiled under the `live` feature.

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::{Duration, Instant};

use futures::StreamExt;
use ui::live::toast_dismiss_stream_impl;
use ui::state::Message;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Extract the `Instant` payload from a `Message::ToastTick`.
/// Panics if the message is not `ToastTick`.
fn unwrap_toast_tick(msg: Message) -> Instant {
    match msg {
        Message::ToastTick(t) => t,
        other => panic!("expected Message::ToastTick, got {other:?}"),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// C3-T1 — stream yields `Message::ToastTick` every 500 ms under paused time.
///
/// ## Protocol
///
/// 1. `start_paused = true` — take control of all tokio timers.
/// 2. Construct `toast_dismiss_stream_impl` and spawn a poller task that
///    forwards each `Instant` payload into a `tokio::sync::mpsc::channel`.
/// 3. `yield_now()` — task runs; consumes the immediate t=0 interval tick
///    (the skip tick); enters loop; awaits the t=500ms tick (blocks).
/// 4. For each of N=3 iterations: `advance(500ms)` + `yield_now()` to give
///    the poller task time to run, then drain the receiver.
/// 5. Assert exactly N=3 `ToastTick` messages were collected.
///
/// ## T-T4 falsification probe (D-V0.2.0-3 row 7)
///
/// Comment out `yield Message::ToastTick(Instant::now());` in
/// `toast_dismiss_stream_impl` — poller task never forwards a message →
/// `ticks.len() == 0` → `assert_eq!(ticks.len(), 3)` fails.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn stream_yields_toast_tick_every_500ms() {
    let rt = tokio::runtime::Handle::current();
    let (forward_tx, mut forward_rx) = tokio::sync::mpsc::channel::<Instant>(16);

    // Spawn a task that drives the stream and forwards ToastTick instants.
    let mut stream = toast_dismiss_stream_impl(&rt);
    tokio::spawn(async move {
        while let Some(msg) = stream.next().await {
            let t = unwrap_toast_tick(msg);
            if forward_tx.send(t).await.is_err() {
                break;
            }
        }
    });

    // Yield to let the task start and consume the immediate t=0 interval tick
    // (the "skip" tick in the stream body).  After this yield, the task is
    // blocked waiting for the t=500ms tick.
    tokio::task::yield_now().await;

    // Collect 3 ticks by advancing time 500 ms for each.
    let mut ticks: Vec<Instant> = Vec::with_capacity(3);
    for _ in 0..3_usize {
        // Fire the next interval tick.
        tokio::time::advance(Duration::from_millis(500)).await;
        // Yield to let the spawned poller task process the interval tick.
        tokio::task::yield_now().await;
        // Drain all available ticks (should be exactly 1 per advance).
        while let Ok(t) = forward_rx.try_recv() {
            ticks.push(t);
        }
    }

    // PRIMARY assertion: T-T4 falsification probe target.
    // Suppress `yield` in stream_impl → ticks.len() == 0 → FAILS here.
    assert_eq!(
        ticks.len(),
        3,
        "expected exactly 3 ToastTick messages (one per 500 ms advance); \
         got {}: {:?}. \
         T-T4 probe: comment out `yield Message::ToastTick(Instant::now())` \
         in toast_dismiss_stream_impl to reproduce the left==0 failure.",
        ticks.len(),
        ticks
    );
}

/// C3-T2 — consecutive `ToastTick` `Instant` values are monotonically
/// non-decreasing.
///
/// `std::time::Instant::now()` is sourced from the wall clock.  Under paused
/// tokio time, the real wall clock still advances (tokio only pauses its
/// virtual timer, not `std::time::Instant`), so successive calls yield
/// non-decreasing values.  This test pins the monotonicity invariant.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn toast_tick_instants_are_monotone() {
    let rt = tokio::runtime::Handle::current();
    let (forward_tx, mut forward_rx) = tokio::sync::mpsc::channel::<Instant>(16);

    let mut stream = toast_dismiss_stream_impl(&rt);
    tokio::spawn(async move {
        while let Some(msg) = stream.next().await {
            let t = unwrap_toast_tick(msg);
            if forward_tx.send(t).await.is_err() {
                break;
            }
        }
    });

    // Consume the immediate t=0 tick (skip).
    tokio::task::yield_now().await;

    // Collect 2 ticks.
    let mut ticks: Vec<Instant> = Vec::with_capacity(2);
    for _ in 0..2_usize {
        tokio::time::advance(Duration::from_millis(500)).await;
        tokio::task::yield_now().await;
        while let Ok(t) = forward_rx.try_recv() {
            ticks.push(t);
        }
    }

    assert_eq!(
        ticks.len(),
        2,
        "expected 2 ticks for monotonicity check; got {}",
        ticks.len()
    );

    let t1 = ticks[0];
    let t2 = ticks[1];

    // Monotonicity: t2 must be >= t1 (non-decreasing real wall-clock).
    assert!(
        t2 >= t1,
        "ToastTick Instant values must be non-decreasing: t1={t1:?} t2={t2:?}"
    );
}

/// C3-T3 — stream remains open after N=3 ticks (never terminates spontaneously).
///
/// `ToastDismissRecipe` is an always-on process-lifetime recipe — it must
/// never terminate spontaneously.  This test collects 3 ticks then asserts the
/// stream is still open (poller task still running, channel not closed).
///
/// Pins against the silent-termination failure mode where the auto-dismiss
/// sweep stops running because the stream ended after a finite count.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn toast_dismiss_stream_remains_open() {
    let rt = tokio::runtime::Handle::current();
    let (forward_tx, mut forward_rx) = tokio::sync::mpsc::channel::<Instant>(16);

    let mut stream = toast_dismiss_stream_impl(&rt);
    tokio::spawn(async move {
        while let Some(msg) = stream.next().await {
            let t = unwrap_toast_tick(msg);
            if forward_tx.send(t).await.is_err() {
                break;
            }
        }
    });

    // Consume the immediate t=0 tick (skip).
    tokio::task::yield_now().await;

    // Collect 3 ticks.
    let mut ticks: Vec<Instant> = Vec::with_capacity(3);
    for _ in 0..3_usize {
        tokio::time::advance(Duration::from_millis(500)).await;
        tokio::task::yield_now().await;
        while let Ok(t) = forward_rx.try_recv() {
            ticks.push(t);
        }
    }

    assert_eq!(
        ticks.len(),
        3,
        "expected 3 ticks before open-stream check; got {}",
        ticks.len()
    );

    // Advance one more tick period and check the channel is still alive.
    // If the stream terminated, the poller task exited and dropped `forward_tx`,
    // closing the channel — `try_recv()` returns `Disconnected`.
    tokio::time::advance(Duration::from_millis(500)).await;
    tokio::task::yield_now().await;

    match forward_rx.try_recv() {
        Ok(_) => {
            // Tick 4 arrived — stream is definitely still alive.
        }
        Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
            // No tick yet — channel is open (not disconnected). Acceptable.
            // (Poller task may not have run yet in this window.)
        }
        Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
            panic!(
                "ToastDismiss stream terminated after 3 ticks — channel disconnected. \
                 This is the silent-termination bug: the auto-dismiss ticker stopped \
                 firing after a finite count."
            );
        }
    }
}
