//! T-D-C3: `toast_dismiss_stream_impl` boundary tests (Surface 1).
//!
//! `ui::live::toast_dismiss_stream_impl` (`crates/ui/src/live.rs`) is the
//! stream `<ToastDismissRecipe as Recipe>::stream` hands to iced: the always-on
//! ticker that ages toasts out. The three seam tests drive it under
//! `#[tokio::test(flavor = "current_thread", start_paused = true)]` with
//! `tokio::time::advance`, and assert that `Message::ToastTick` messages arrive
//! at one tick per 500 ms period, exactly: none before a period ends, exactly
//! one when it ends, and the ticker never stops.
//!
//! The Recipe itself is driven too: story 2-63 Wave C1 moved
//! `ToastDismissRecipe` from the `cockpit_live` bin (which an integration test
//! cannot link) into `ui::live` as a `pub` struct (`pub struct ToastDismissRecipe {`
//! in `crates/ui/src/live.rs`), and Wave E1b's
//! `toast_dismiss_recipe_stream_yields_tick` streams it through
//! `Recipe::stream` on a paused clock (rows E1-P22 and E1-P23 below). The other
//! three tests drive the seam its `stream` delegates to,
//! `toast_dismiss_stream_impl(&self.rt_handle)`.
//!
//! ## What this file does not pin
//!
//! - **The Recipe's `hash`.** No test here hashes the recipe.
//!   `subscription_batch_inclusion.rs` pins the ids of the batch production's
//!   `ui::live::subscription_batch` builds, from a TEST ctx (its identity
//!   tests; the bin's ctx literal stays checked by nothing), and its
//!   `batched_toast_dismiss_recipe_relays` streams the unit that batch carries.
//! - **The runtime guard**, `let _guard = rt_handle.enter();`. Every test here
//!   runs inside a tokio runtime, so deleting the guard is unobservable here:
//!   Wave B's B-P2 left this file green (measured, rounds 1 and 2).
//!   `toast_dismiss_stream_ticks_off_runtime` in `recipe_off_runtime_poll.rs`
//!   pins it.
//! - **The payload.** Each `ToastTick` carries `std::time::Instant::now()`,
//!   which keeps running while tokio's clock is paused. This file checks only
//!   that each message IS a `ToastTick`. Its freshness is pinned by the same
//!   Wave B test (B-P15, a stale stamp; B-P26, a future one; both left this
//!   file green, measured in round 3). The monotonicity check this file used to
//!   make (`t2 >= t1` on two payloads) could not fail on any production edit:
//!   on a paused clock the two stamps are microseconds apart in real time, so
//!   a constant shift of the stamp stays monotone. It is deleted.
//!
//! ## Time-control protocol (the three seam tests)
//!
//! This protocol is the three seam tests' (`stream_yields_toast_tick_every_500ms`,
//! `toast_period_is_exactly_500ms`, `toast_dismiss_stream_remains_open`).
//! `tokio::time::interval(500ms)` fires its FIRST tick immediately (at t = 0).
//! The stream body skips it with the `interval.tick().await` before its loop.
//! Under paused time:
//!
//! 1. Build the stream and spawn a poller task that forwards every message,
//!    unchanged, into a `tokio::sync::mpsc` channel ([`spawn_toast_poller`]).
//!    The poller holds the only sender, so the channel reads `Disconnected`
//!    exactly when the stream has ENDED.
//! 2. `yield_now()`: the poller runs, takes the immediate t = 0 tick (the
//!    skip), and parks on the tick due at 500 ms.
//! 3. Per step: `advance(d)` + `yield_now()`. Every timer due by then fires and
//!    the poller forwards what it yields, before the test thread reads the
//!    channel with `try_recv`.
//!
//! A step of 500 ms therefore yields exactly one tick. The 499 ms + 1 ms split
//! in [`toast_period_is_exactly_500ms`] rests on tokio's paused-clock timer
//! resolution: a deadline is rounded UP to the end of its millisecond
//! (`TimeSource::deadline_to_tick`, `tokio/src/runtime/time/source.rs`), and
//! the interval is created at the runtime's start instant (no time passes on a
//! paused clock before it), so the tick due at 500 ms fires when the clock
//! reaches 500 ms and not at 499 ms. Both halves pass at HEAD (nothing at
//! 499 ms, one tick at 500 ms), so the rounding is observed, not assumed; the
//! absence half's red side is E1-P12's to measure.
//!
//! The E1b recipe test, `toast_dismiss_recipe_stream_yields_tick`, follows none
//! of it: no poller task and no `advance`. It polls the Recipe's stream itself
//! and lets tokio's auto-advance move the paused clock to the next timer
//! deadline.
//!
//! ## Non-vacuity contract (bug-log #66 / story 2-63 AC3)
//!
//! In the three seam tests, each read is one `try_recv` after the poller has
//! had its turn, and all three outcomes are told apart: `Ok` (a message, whose kind is checked on the test
//! thread), `Empty` (the poller is parked on a live stream) and `Disconnected`
//! (the stream ENDED). No outcome is accepted as "the poller may not have run
//! yet": on a paused current-thread runtime the delivery is deterministic. The
//! claims are two-sided:
//! - per period: nothing before 500 ms, and exactly one tick at 500 ms;
//! - per tick: the tick arrives, and the stream is still live with nothing
//!   more buffered.
//!
//! The E1b recipe test reads no channel. Its first poll, with no time passing,
//! is a three-way match (`now_or_never`: parked and live, ENDED, or yielded
//! with no time passing), of which only "parked and live" passes; then it
//! awaits the first item under a 750 ms timeout that FAILS the test (and an
//! end fails with its own text).
//!
//! ## T-T4 falsification probe table (story 2-63 AC4, Wave E1)
//!
//! Each probe is ONE edit to production code, located by symbol + the quoted
//! line text (line numbers rot). Procedure:
//!
//! 1. snapshot the file (`cp -p <file> <snap>`) and apply the edit;
//! 2. run `cargo test -p ui --test toast_dismiss_recipe_stream --no-fail-fast`.
//!    The named test(s) go RED;
//! 3. restore from the snapshot, THEN `touch <file>`. A restore that keeps the
//!    snapshot's older mtime (`cp -p`) leaves cargo's fingerprint "fresh": the
//!    next run compiles nothing and re-executes the MUTANT binary;
//! 4. re-run. The GREEN counts only if that run's output shows
//!    `Compiling ui` and the file's SHA-256 equals the pre-probe one.
//!
//! Probe ids are the Wave E1 series, shared across the seven v0.2
//! recipe-stream files E1 sweeps. B-P41 keeps its Wave B id: it is Wave B's
//! measured survivor, routed to this file. E1-P12 and E1-P25 edit the same
//! line, as do E1-P13, B-P41 and E1-P36, and E1-P37 shares E1-P12's: apply one
//! at a time. Every row was MEASURED: the E1a rows by the E1a falsifier or
//! the orchestrator's hand probes (2026-11-13), the E1b rows (E1-P22, E1-P23;
//! they share one quoted line, applied in separate runs) by the Wave C2
//! falsifier (2026-11-14, workflow `wf_a82b74dd-7bb`). That quoted line is
//! also the quoted line of `subscription_batch_inclusion.rs`'s C2-P20: apply
//! one at a time. C2-P20 is the EQUIVALENT edit of E1-P22, not the same text
//! (its NEW spells the item type, `futures::stream::empty::<Message>()`).
//!
//! | # | symbol (file) | quoted line today | mutation | expected RED |
//! |---|---|---|---|---|
//! | E1-P12 | `ui::live::toast_dismiss_stream_impl` (`crates/ui/src/live.rs`) | `tokio::time::interval(Duration::from_millis(500))` | `500` → `400` | `toast_period_is_exactly_500ms`, at "toast period 1: no ToastTick before 500 ms" (the tick due at 400 ms is delivered inside the 499 ms step). Also `toast_dismiss_stream_remains_open`, at "toast: the stream is still live after tick 4" (the 4th advance delivers the ticks due at 1600 ms and 2000 ms). `stream_yields_toast_tick_every_500ms` stays GREEN: 1500 ms holds three 400 ms ticks. This is the survivor that refuted "already pins the period exactly" (story 2-63, Wave B decision 1). Outside this target, MEASURED by the E1a falsifier: Wave B's `recipe_off_runtime_poll::toast_dismiss_stream_ticks_off_runtime` is red too, at "ToastTick #1: stamped ~401ms after `stream()` began, expected no earlier than 500ms" |
//! | E1-P25 | `ui::live::toast_dismiss_stream_impl` | `tokio::time::interval(Duration::from_millis(500))` | `500` → `50` (Wave B's B-P16, re-run after E1) | `stream_yields_toast_tick_every_500ms`, at its `assert_eq!(` count, "expected exactly 3 ToastTick messages (one per 500 ms advance)" (B-P16 measured it red in v0.2, 3/3); `toast_period_is_exactly_500ms`, at "toast period 1: no ToastTick before 500 ms"; `toast_dismiss_stream_remains_open`, at "expected 3 ticks before open-stream check". Outside this target, MEASURED by the E1a falsifier: Wave B's `toast_dismiss_stream_ticks_off_runtime` is red too (its own B-P16 primary, "expected no earlier than 500ms") |
//! | E1-P13 | `ui::live::toast_dismiss_stream_impl`, its `loop {` (two lines above the quoted line) | `yield Message::ToastTick(Instant::now());` | the four-line `loop { … }` → `for _ in 0..3 { … }` followed by `std::future::pending::<()>().await;` (the ticker freezes after three ticks; the stream never ends) | `toast_dismiss_stream_remains_open`, at "toast: tick 4 arrives on the 4th advance (the stream does not freeze)" (`Empty`: v0.2 accepted exactly that). By design `toast_period_is_exactly_500ms` and `stream_yields_toast_tick_every_500ms` stay GREEN: both read three periods |
//! | B-P41 | `ui::live::toast_dismiss_stream_impl`, its `loop {` | `yield Message::ToastTick(Instant::now());` | `loop {` → `for _ in 0..4 {` (the ticker stops after four ticks and the stream ENDS: the toast analogue of B-P38) | `toast_dismiss_stream_remains_open`, at "toast: the stream is still live after tick 4" (the stream ENDED). Wave B measured it a survivor (`recipe_off_runtime_poll` 18/18 and this file 3/3 green, round 6). Any `for _ in 0..N` with 4 <= N <= 7,200 fails at "the stream is still live after tick N", a smaller N earlier; N > 7,200 passes, a stated limit. MEASURED red by the E1a falsifier (as ST-OWN-1), at exactly that text: the Wave B survivor is killed |
//! | E1-P36 | `ui::live::toast_dismiss_stream_impl`, inside its `loop {` | `yield Message::ToastTick(Instant::now());` | the line duplicated (two ticks per period, the immediate skip untouched) | `toast_period_is_exactly_500ms`, at "toast period 1: exactly one ToastTick at 500 ms" (`an unexpected message was buffered`: the second tick of the period). This is the 'exactly one' side, which no other probe reaches. Also red: `stream_yields_toast_tick_every_500ms` (got 6) and `toast_dismiss_stream_remains_open` ("expected 3 ticks before open-stream check; got 6"). MEASURED (orchestrator hand probe, 2026-11-13) |
//! | E1-P37 | `ui::live::toast_dismiss_stream_impl` | `tokio::time::interval(Duration::from_millis(500))` | `500` → `700` (Wave B's B-P31, re-run against this version) | `toast_period_is_exactly_500ms`, at "toast period 1: exactly one ToastTick at 500 ms" (`nothing was buffered`: no tick is due by 500 ms). This is the presence side. Also red: `stream_yields_toast_tick_every_500ms` (got 2) and `toast_dismiss_stream_remains_open` ("got 2"). MEASURED (orchestrator hand probe, 2026-11-13) |
//! | E1-P22 (E1b) | `<ToastDismissRecipe as Recipe>::stream` (`crates/ui/src/live.rs`, since Wave C1) | `toast_dismiss_stream_impl(&self.rt_handle)` (the body's one line) | → `Box::pin(futures::stream::empty())` | `toast_dismiss_recipe_stream_yields_tick`, at "toast recipe: `ToastDismissRecipe::stream` returns a live stream" (its full text: "...: it ENDED at its first poll"). MEASURED (2026-11-14, `wf_a82b74dd-7bb` falsifier): exactly this red; the other three tests call the seam directly and stayed GREEN. Outside this target, MEASURED in the same run: `subscription_batch_inclusion::batched_toast_dismiss_recipe_relays` is red too, at "toast: stream ENDED before its first item" (its C2-P20 is the equivalent edit) |
//! | E1-P23 (E1b) | `<ToastDismissRecipe as Recipe>::stream` | as E1-P22 | → `server_time_stream_impl(&self.rt_handle)` (the copy-paste cross-wire C1 makes possible) | `toast_dismiss_recipe_stream_yields_tick`, at "toast recipe: the first ToastTick arrives within 750 ms (paused clock)": the cross-wired tick is due at 1 s, so the 750 ms timeout fires first under auto-advance. MEASURED (2026-11-14, `wf_a82b74dd-7bb` falsifier): exactly this red; the other three tests stayed GREEN. Outside this target, MEASURED in the same run: `batched_toast_dismiss_recipe_relays` is red too, at "toast first item: ServerTimeTick(..)" (its 2 s bound lets the 1 s tick arrive) |
//!
//! Wave B's other interval probes were MEASURED red in v0.2's version of this
//! file at its count asserts: B-P29 (1000 ms, round 4), B-P31 (700 ms) and
//! B-P40 (800 ms) (round 6). For this version, B-P31 is MEASURED as E1-P37
//! above, red at `toast_period_is_exactly_500ms`, "toast period 1: exactly one
//! ToastTick at 500 ms" (no tick is due by 500 ms); B-P29 and B-P40 are
//! reasoned red at the same text, by the same mechanism. A deleted skip of the immediate tick
//! (the toast analogue of B-P34) is red at "toast period 1: no ToastTick
//! before 500 ms": the t = 0 tick is buffered before the first step.
//!
//! ## Coverage
//!
//! | origin | test | what it pins |
//! |---|---|---|
//! | v0.2 C3-T1 | `stream_yields_toast_tick_every_500ms` | three 500 ms advances yield exactly 3 `ToastTick`s in all (a count over 1.5 s: alone, it accepts any period in (375 ms, 500 ms]) |
//! | E1, replaces v0.2 C3-T2 (`toast_tick_instants_are_monotone`) | `toast_period_is_exactly_500ms` | each of 3 periods, two-sided: nothing after 499 ms, exactly one `ToastTick` at 500 ms |
//! | v0.2 C3-T3, strict | `toast_dismiss_stream_remains_open` | tick 4 arrives on the 4th advance, then exactly one tick per advance for an hour of paused time (7,200 ticks), and the stream is live after every one |
//! | E1b | `toast_dismiss_recipe_stream_yields_tick` | the real `<ToastDismissRecipe as Recipe>::stream` (in `ui::live` since Wave C1): live at its first poll, then a `ToastTick` within 750 ms of paused time. It accepts any period up to 750 ms (tokio's `Timeout` polls the inner future before its deadline, so a tick due exactly at 750 ms still arrives); the exact period is `toast_period_is_exactly_500ms`'s |
//!
//! `#[cfg(feature = "live")]` gates the whole file since `toast_dismiss_stream_impl`
//! is only compiled under the `live` feature.

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::{Duration, Instant};

use futures::StreamExt;
use iced::advanced::subscription::{Event, EventStream, Recipe};
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TryRecvError;
use ui::live::{ToastDismissRecipe, toast_dismiss_stream_impl};
use ui::state::Message;

/// The documented period: `toast_dismiss_stream_impl` "Emits
/// `Message::ToastTick(Instant::now())` every 500 ms" (its doc comment), the
/// always-on auto-dismiss cadence of ADR-0046.
const PERIOD: Duration = Duration::from_millis(500);

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Build the stream (its interval starts NOW, on the paused clock) and spawn
/// the poller that forwards every message, unchanged, into a channel.
///
/// The poller holds the only sender and drops it when the stream ends, so the
/// receiver reads `Disconnected` exactly when the stream has ENDED.
fn spawn_toast_poller() -> mpsc::Receiver<Message> {
    let rt = tokio::runtime::Handle::current();
    let mut stream = toast_dismiss_stream_impl(&rt);
    let (forward_tx, forward_rx) = mpsc::channel::<Message>(16);
    tokio::spawn(async move {
        while let Some(msg) = stream.next().await {
            if forward_tx.send(msg).await.is_err() {
                break;
            }
        }
    });
    forward_rx
}

/// Extract the `Instant` payload from a `Message::ToastTick`.
/// Panics if the message is not `ToastTick`.
fn unwrap_toast_tick(msg: Message) -> Instant {
    match msg {
        Message::ToastTick(t) => t,
        other => panic!("expected Message::ToastTick, got {other:?}"),
    }
}

/// Take the ONE message that must be buffered now, and check it is a
/// `ToastTick`. Fails on `Empty` (no tick: a frozen or slower ticker), on
/// `Disconnected` (the stream ENDED) and on any other message.
fn expect_tick(rx: &mut mpsc::Receiver<Message>, what: &str) {
    match rx.try_recv() {
        Ok(Message::ToastTick(_)) => {}
        Ok(other) => panic!("{what}: expected Message::ToastTick, got {other:?}"),
        Err(TryRecvError::Empty) => {
            panic!("{what}: nothing was buffered (`Empty`: the ticker is parked, not ticking)")
        }
        Err(TryRecvError::Disconnected) => {
            panic!("{what}: the stream ENDED (the poller dropped its sender)")
        }
    }
}

/// Observe the stream LIVE with NOTHING buffered: `Empty`. Fails on
/// `Disconnected` (the stream ENDED) and on any buffered message (a tick that
/// was not due, or a second one).
fn assert_idle(rx: &mut mpsc::Receiver<Message>, what: &str) {
    match rx.try_recv() {
        Err(TryRecvError::Empty) => {}
        Err(TryRecvError::Disconnected) => {
            panic!("{what}: the stream ENDED (the poller dropped its sender)")
        }
        Ok(msg) => panic!("{what}: an unexpected message was buffered: {msg:?}"),
    }
}

/// The iced runtime event stream a `Recipe::stream` receives. The ticker
/// ignores it, so an empty one is faithful.
fn empty_event_stream() -> EventStream {
    Box::pin(futures::stream::empty::<Event>())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// C3-T1: three 500 ms advances of the paused clock yield exactly 3
/// `Message::ToastTick`s.
///
/// ## Protocol
///
/// 1. `start_paused = true`: take control of all tokio timers.
/// 2. [`spawn_toast_poller`].
/// 3. `yield_now()`: the poller takes the immediate t = 0 interval tick (the
///    skip) and parks on the tick due at 500 ms.
/// 4. For each of N = 3 iterations: `advance(500ms)` + `yield_now()`, then
///    drain the receiver.
/// 5. Assert exactly N = 3 `ToastTick` messages were collected.
///
/// A total over 1.5 s accepts any period in (375 ms, 500 ms]; the exact
/// period is [`toast_period_is_exactly_500ms`]'s. Probes: the T-T4 table in
/// the module doc (E1-P25).
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn stream_yields_toast_tick_every_500ms() {
    let mut forward_rx = spawn_toast_poller();

    // Let the poller take the immediate t=0 interval tick (the "skip" tick in
    // the stream body). After this yield, it is parked on the t=500ms tick.
    tokio::task::yield_now().await;

    // Collect 3 ticks by advancing time 500 ms for each.
    let mut ticks: Vec<Instant> = Vec::with_capacity(3);
    for _ in 0..3_usize {
        // Fire the next interval tick.
        tokio::time::advance(PERIOD).await;
        // Yield to let the spawned poller task process the interval tick.
        tokio::task::yield_now().await;
        // Drain all available ticks (should be exactly 1 per advance).
        while let Ok(msg) = forward_rx.try_recv() {
            ticks.push(unwrap_toast_tick(msg));
        }
    }

    assert_eq!(
        ticks.len(),
        3,
        "expected exactly 3 ToastTick messages (one per 500 ms advance); got {}: {:?}",
        ticks.len(),
        ticks
    );
}

/// E1 (replaces v0.2's C3-T2, `toast_tick_instants_are_monotone`): the period
/// is exactly 500 ms, pinned from both sides in each of three periods.
///
/// Per period `k`: `advance(499ms)` + `yield_now()` must leave the channel
/// `Empty` ("no ToastTick before 500 ms"); then `advance(1ms)` + `yield_now()`
/// must deliver exactly one `ToastTick`, after which the channel is `Empty`
/// again ("exactly one ToastTick at 500 ms").
///
/// A faster interval (400 ms, E1-P12; 50 ms, E1-P25) delivers a tick inside
/// the 499 ms step. A slower one (700 ms, 800 ms, 1000 ms: Wave B's B-P31,
/// B-P40, B-P29) delivers none at 500 ms. E1-P12 is what makes the absence
/// half non-vacuous: it passes every count this file made before E1.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn toast_period_is_exactly_500ms() {
    /// Periods checked, each from both sides.
    const PERIODS: usize = 3;
    /// One millisecond: tokio's paused-clock timer resolution (module doc).
    const LAST_MS: Duration = Duration::from_millis(1);

    let early = PERIOD - LAST_MS;
    let mut forward_rx = spawn_toast_poller();

    // The poller takes the immediate t=0 tick (the skip) and parks.
    tokio::task::yield_now().await;

    for k in 1..=PERIODS {
        tokio::time::advance(early).await;
        tokio::task::yield_now().await;
        assert_idle(
            &mut forward_rx,
            &format!("toast period {k}: no ToastTick before 500 ms ({early:?} into the period)"),
        );

        tokio::time::advance(LAST_MS).await;
        tokio::task::yield_now().await;
        let what = format!("toast period {k}: exactly one ToastTick at 500 ms");
        expect_tick(&mut forward_rx, &what);
        assert_idle(&mut forward_rx, &what);
    }
}

/// C3-T3: the stream never stops ticking. `ToastDismissRecipe` is an always-on
/// process-lifetime recipe: a ticker that ends, or freezes, after a finite
/// count stops ageing toasts out, and every toast stays on screen.
///
/// Ticks 1-3 are taken as v0.2 took them. Tick 4 is then REQUIRED on the 4th
/// advance: v0.2 accepted `Empty` there, so a ticker frozen after three ticks
/// passed (E1-P13). After tick 4, and after every later tick up to `TICKS` (an
/// hour of PAUSED time, which costs real time only for the polls), the stream
/// must be live with nothing more buffered: a ticker that ENDS after N ticks
/// (B-P41: N = 4) fails right after tick N for any 4 <= N <= `TICKS`, and a
/// second tick on one advance fails too. A ticker that ends after more than
/// `TICKS` ticks passes: a stated limit, as in `server_time_recipe_stream`.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn toast_dismiss_stream_remains_open() {
    /// Ticks taken in all: one hour of PAUSED time at one tick per 500 ms.
    const TICKS: usize = 7_200;

    let mut forward_rx = spawn_toast_poller();

    // Consume the immediate t=0 tick (skip).
    tokio::task::yield_now().await;

    // Collect 3 ticks.
    let mut ticks: Vec<Instant> = Vec::with_capacity(3);
    for _ in 0..3_usize {
        tokio::time::advance(PERIOD).await;
        tokio::task::yield_now().await;
        while let Ok(msg) = forward_rx.try_recv() {
            ticks.push(unwrap_toast_tick(msg));
        }
    }

    assert_eq!(
        ticks.len(),
        3,
        "expected 3 ticks before open-stream check; got {}",
        ticks.len()
    );

    // Tick 4 must arrive on the 4th advance: `Empty` here is a frozen ticker,
    // `Disconnected` an ended one.
    tokio::time::advance(PERIOD).await;
    tokio::task::yield_now().await;
    expect_tick(
        &mut forward_rx,
        "toast: tick 4 arrives on the 4th advance (the stream does not freeze)",
    );
    assert_idle(
        &mut forward_rx,
        "toast: the stream is still live after tick 4",
    );

    // Every later tick, for an hour of paused time.
    for k in 5..=TICKS {
        tokio::time::advance(PERIOD).await;
        tokio::task::yield_now().await;
        expect_tick(
            &mut forward_rx,
            &format!("toast: tick {k} arrives on advance {k} (the stream does not freeze)"),
        );
        assert_idle(
            &mut forward_rx,
            &format!("toast: the stream is still live after tick {k}"),
        );
    }
}

/// Wave E1b — the real `<ToastDismissRecipe as Recipe>::stream`, moved into
/// `ui::live` by story 2-63 Wave C1, polled directly on a PAUSED clock (no
/// poller task: tokio's auto-advance moves the clock).
///
/// The first poll, with no time passing, must find the stream LIVE: it takes
/// the interval's immediate tick (the skip) and parks on the one due at
/// 500 ms. A body that returns an ended stream fails there (E1-P22). Then the
/// first item must arrive within 750 ms of paused time and be a `ToastTick`.
/// A body cross-wired to `server_time_stream_impl` (E1-P23) has its first tick
/// due at 1 s, so the timeout's 750 ms deadline is the earlier one: tokio
/// jumps the clock there and the test fails at the timeout.
///
/// The other three tests call the seam, `toast_dismiss_stream_impl`, directly;
/// this is the only test in the file that goes through the `Recipe`.
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn toast_dismiss_recipe_stream_yields_tick() {
    let recipe = Box::new(ToastDismissRecipe {
        rt_handle: tokio::runtime::Handle::current(),
    });
    let mut stream = recipe.stream(empty_event_stream());

    match futures::FutureExt::now_or_never(stream.next()) {
        None => {}
        Some(None) => panic!(
            "toast recipe: `ToastDismissRecipe::stream` returns a live stream: it ENDED at its \
             first poll"
        ),
        Some(Some(msg)) => panic!(
            "toast recipe: `ToastDismissRecipe::stream` returns a live stream: it yielded with no \
             time passing: {msg:?}"
        ),
    }
    let first = tokio::time::timeout(Duration::from_millis(750), stream.next())
        .await
        .unwrap_or_else(|_| {
            panic!("toast recipe: the first ToastTick arrives within 750 ms (paused clock)")
        })
        .unwrap_or_else(|| panic!("toast recipe: the stream ENDED before its first ToastTick"));
    assert!(
        matches!(first, Message::ToastTick(_)),
        "toast recipe: the first message is a ToastTick: {first:?}"
    );
}
