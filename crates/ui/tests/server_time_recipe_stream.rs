//! Subscription-pipe tests for the server-time stream, the status-bar clock's
//! 1 Hz ticker: `ui::live::server_time_stream_impl` (`crates/ui/src/live.rs`),
//! the whole body of the bin's `ServerTimeRecipe::stream`. Closes the Wave 1
//! carve-out from `subscription-pipe-server-time-template v0.1.0`.
//!
//! ## What this file drives, and what it cannot reach
//!
//! `ServerTimeRecipe` is private to the `cockpit_live` bin
//! (`struct ServerTimeRecipe {` in `crates/ui/src/bin/cockpit_live.rs`), and an
//! integration test cannot link a bin target, so no test here builds the
//! Recipe. Its `stream()` is one line,
//! `ui::live::server_time_stream_impl(&self.rt_handle)`, and these tests drive
//! that seam. The delegation itself and the Recipe's `hash` are a residual
//! here, not a claim. Story 2-63 Wave C1 moves the recipe into `ui::live`;
//! Wave E1b then adds `server_time_recipe_stream_yields_tick`, which streams
//! `ui::live::ServerTimeRecipe` through `Recipe::stream` (probe E1-P21).
//!
//! Wave E1a deleted T-ST-1d (`server_time_stream_impl_recipe_path_end_to_end`).
//! Its doc said it exercised the full `Recipe::stream()` path, but it called
//! the seam exactly as T-ST-1a does and discarded the payload unchecked, so it
//! pinned nothing T-ST-1a does not.
//!
//! The K8 enter guard (`let _guard = rt_handle.enter();` in
//! `server_time_stream_impl`) is NOT pinned here either. Every test here runs
//! under `#[tokio::test]`, whose thread already has a tokio context, so a
//! deleted guard is invisible: story 2-63 Wave B measured this file GREEN
//! under B-P1. The guard is pinned off-runtime by Wave B's
//! `server_time_stream_ticks_off_runtime` (`crates/ui/tests/recipe_off_runtime_poll.rs`).
//!
//! ## Coverage
//!
//! | Test ID | test | What it pins |
//! |---|---|---|
//! | T-ST-1a | `server_time_stream_impl_yields_tick` | The first message is a `ServerTimeTick`, within 1.5 s of real time. Accepts any interval under 1.5 s and an unskipped immediate tick, by design |
//! | T-ST-1b | `server_time_stream_impl_emits_at_1_hz_cadence` (Wave B) | The exact 1 s cadence on a paused clock: every gap, for an hour of paused time (3,600 ticks), each stamp fresh (two-sided), and the stream still live afterwards. The only test here that pins the period and the skip of the immediate tick |
//! | T-ST-1c | `server_time_stream_impl_stream_remains_open` | The clock never freezes and the stream never ends: 7,200 paused ticks (twice T-ST-1b's horizon), each within 1.5 s of paused time |
//! | (E1b) | `server_time_recipe_stream_yields_tick` | After Wave C1: the real `Recipe::stream` yields a tick. Not in this file yet |
//!
//! ## Non-vacuity contract (bug-log #66 / story 2-63 AC3)
//!
//! Every tick is awaited under a timeout that FAILS the test: no timeout is
//! ever accepted as "the stream is still open". A stream END (`None`) fails
//! too, with its own message. The paused-clock tests (T-ST-1b, T-ST-1c) cost
//! real time only for their polls: whenever the runtime is idle tokio jumps
//! the paused clock to the next timer deadline, so a tick that never comes
//! shows up as the timeout's deadline firing, not as a slow test.
//!
//! ## T-T4 falsification probe table (story 2-63 AC4, Wave E1)
//!
//! Each probe is ONE edit to production code, located by symbol + the quoted
//! line text (line numbers rot). Procedure:
//!
//! 1. snapshot the file (`cp -p <file> <snap>`) and apply the edit;
//! 2. run `cargo test -p ui --test server_time_recipe_stream`. The named
//!    test(s) go RED;
//! 3. restore from the snapshot, THEN `touch <file>`. A restore that keeps the
//!    snapshot's older mtime (`cp -p`) leaves cargo's fingerprint "fresh": the
//!    next run compiles nothing and re-executes the MUTANT binary;
//! 4. re-run. The GREEN counts only if that run's output shows
//!    `Compiling ui` and the file's SHA-256 equals the pre-probe one.
//!
//! Probe ids are the Wave E1 series, shared across the seven v0.2
//! recipe-stream files E1 sweeps; this file's rows are E1-P11 and E1-P24.
//! Both edit `crates/ui/src/live.rs`. E1-P21 (Wave E1b, after Wave C1) is not
//! a row yet: the line it edits does not exist in `ui::live` until C1 moves
//! the recipe there.
//!
//! | # | symbol (file) | quoted line today | mutation | expected RED |
//! |---|---|---|---|---|
//! | E1-P11 | `ui::live::server_time_stream_impl` (`crates/ui/src/live.rs`) | `loop {` (the line above `interval.tick().await;` and `yield Message::ServerTimeTick(Timestamp::now());`) | → `for _ in 0..3600 {`, and `std::future::pending::<()>().await;` after the loop's closing brace (the clock FREEZES, without ending, right after T-ST-1b's 3,600th tick) | `server_time_stream_impl_stream_remains_open`, at "server time: tick 3601 of 7200 did not arrive within 1.5 s of paused time (the clock froze)". MEASURED by the E1a falsifier: exactly this red, T-ST-1b and T-ST-1a GREEN, and Wave B's file 18/18 GREEN |
//! | E1-P24 | `ui::live::server_time_stream_impl` | `tokio::time::interval(Duration::from_secs(1))` | → `tokio::time::interval(Duration::from_millis(100))` (Wave B's B-P14, re-run as a regression guard: E1's edits to this file did not weaken T-ST-1b or its helper) | `server_time_stream_impl_emits_at_1_hz_cadence`, at "on a paused clock every gap (`stream()` to tick 1, then tick to tick) must be exactly the documented period, 1s" (`left: [100ms, 100ms, 100ms, 100ms]`). Outside this target, MEASURED by the E1a falsifier: Wave B's `recipe_off_runtime_poll::server_time_stream_ticks_off_runtime` is red too, at "ServerTimeTick #1: yielded ~102ms after `stream()` began, expected no earlier than 1s" (B-P14's own Wave B primary) |
//!
//! What each probe leaves GREEN, and why (reasoned; the falsifier measures):
//! - E1-P11: T-ST-1b takes exactly its 3,600 ticks and then polls once with
//!   no time passing. A stream parked on `pending()` answers that poll
//!   `Pending`, just as a live one waiting for its next tick does, so T-ST-1b
//!   cannot see the freeze. T-ST-1a takes one tick. Outside this target, Wave
//!   B's `server_time_stream_ticks_off_runtime` takes two ticks and then finds
//!   the stream live. So the red is attributable to T-ST-1c without a
//!   `--skip` filter. This is why T-ST-1c reads 7,200 ticks: at T-ST-1b's own
//!   horizon it would pin nothing T-ST-1b does not.
//! - E1-P24: T-ST-1a and T-ST-1c accept any interval under 1.5 s, by design
//!   (the period is T-ST-1b's), so both stay green.
//!
//! Wave B's own probes on T-ST-1b (B-P13, B-P14, B-P27, B-P28, B-P30, B-P34,
//! B-P38, B-P39) are tabled with their round-6 measurements in
//! `crates/ui/tests/recipe_off_runtime_poll.rs`'s T-T4 table. Those
//! measurements are of this file BEFORE Wave E1a (SHA-256 `d132975d...`).
//! E1a did not edit T-ST-1b's code, only two sentences of its doc that
//! described the old T-ST-1c. Under E1a's T-ST-1c, three of them are expected
//! red there as well (reasoned, not measured): B-P28 (2 s) and B-P39 (1.6 s)
//! at tick 1's 1.5 s timeout, B-P38 (an end after four ticks) at "server time:
//! the stream ENDED before tick 5".
//!
//! subscription-pipe-server-time-template v0.1.0 Wave A; story 2-63 Waves B and E1a.

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use futures::StreamExt;
use tokio::time::timeout;

use ui::live::server_time_stream_impl;
use ui::state::Message;

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Extract the `Timestamp` value from a `Message::ServerTimeTick`.
/// Panics if the message is not `ServerTimeTick`.
fn unwrap_tick(msg: Message) -> trading_core::Timestamp {
    match msg {
        Message::ServerTimeTick(ts) => ts,
        other => panic!("expected ServerTimeTick, got {other:?}"),
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// T-ST-1a — happy path: `server_time_stream_impl(rt_handle)` yields the first
/// `Message::ServerTimeTick` within ~1.5 s.
///
/// The stream body skips the immediate (zero-delay) tick from
/// `tokio::time::interval`, so the first tick arrives ~1 s after construction.
/// We allow 1.5 s for scheduler jitter. That window accepts any interval under
/// 1.5 s and an unskipped immediate tick, by design: T-ST-1b pins the period
/// and the skip, and T-ST-1c that the clock never freezes.
///
/// This test does NOT pin the K8 enter guard. It runs under `#[tokio::test]`,
/// so its thread already has a tokio context, and deleting
/// `let _guard = rt_handle.enter();` from `server_time_stream_impl` leaves it
/// green (story 2-63 Wave B measured this file GREEN under B-P1). The guard is
/// pinned off-runtime by `server_time_stream_ticks_off_runtime` in
/// `crates/ui/tests/recipe_off_runtime_poll.rs` (Wave B, B-P1).
#[tokio::test]
async fn server_time_stream_impl_yields_tick() {
    let rt = tokio::runtime::Handle::current();
    let mut stream = server_time_stream_impl(&rt);

    // The stream body skips the immediate tick; allow 1.5 s for the first real tick.
    let first = timeout(Duration::from_millis(1500), stream.next())
        .await
        .expect("first ServerTimeTick arrived within 1.5 s")
        .expect("stream produced a message");

    assert!(
        matches!(first, Message::ServerTimeTick(_)),
        "expected ServerTimeTick, got {first:?}"
    );
}

/// T-ST-1b — the 1 Hz cadence, pinned EXACTLY on a paused clock, every
/// payload stamped when it was yielded, and the stream still live after an
/// hour of (paused) ticks.
///
/// ## Why a paused clock
///
/// `start_paused = true` freezes tokio's clock. Whenever the runtime is idle
/// it jumps the clock straight to the next timer deadline (tokio's
/// auto-advance), and nothing moves it in between. So the
/// `tokio::time::Instant` read right after a tick arrives IS that tick's
/// deadline, with no scheduler jitter in it, and each gap can be compared for
/// EQUALITY with the documented period ("once per second"). Four gaps are
/// checked: `stream()` to tick 1 (the interval's immediate first tick is
/// skipped, so tick 1 is one whole period in), then tick 1 to 2, 2 to 3 and
/// 3 to 4. Each must be exactly 1 s. The clock was not advanced before
/// `stream()` built its interval, so even the first gap carries no rounding
/// from tokio's 1 ms timer wheel.
///
/// This is the exact-period half of story 2-63's split. The off-runtime test
/// (`recipe_off_runtime_poll.rs`, `server_time_stream_ticks_off_runtime`)
/// runs on the real clock and pins only the window `[1 s, 1.5 s)`. This test
/// is also the only one in THIS file that pins the period: the others wait up
/// to 1.5 s for each tick (T-ST-1a of real time; T-ST-1c, since story 2-63
/// Wave E1a, of paused time), so they accept any interval under 1.5 s (round
/// 5 measured the three real-time siblings of that revision GREEN under a
/// 1.4 s interval, B-P30, and under an unskipped immediate tick, B-P34). That
/// is by design (story 2-63, round-5 decision 1): the period is pinned here.
///
/// ## Stamps: freshness, not monotonicity
///
/// Each payload is wall-clock `Timestamp::now()`, which keeps running while
/// tokio's clock is paused. Right after each tick arrives the wall clock is
/// read again, and the stamp may be neither LATER than that sample (a future
/// stamp, B-P27) nor half a period or more OLDER (a stale stamp, B-P13).
/// Between the two reads there is one return from `poll_next`: microseconds
/// of real time, against a 500 ms bound. The bound assumes the wall clock does
/// not step backward inside that window. A stamp stale by less than half a
/// period passes, a stated limit (the off-runtime test has the same one).
///
/// This replaces the old monotonicity check (`t2 >= t1`), which could not
/// fail on any production edit: round 5 measured it GREEN under both B-P13
/// and B-P27, because on a paused clock the stamps are microseconds apart in
/// real time, so only a wall-clock step backward could have failed it.
///
/// ## Never terminates, for an hour
///
/// The first four gaps are compared as one array. The test then takes ticks
/// up to `TICKS`, an hour of PAUSED time (it costs real time only for the
/// polls), each gap still exactly 1 s and each stamp fresh. Then one more
/// poll, with no time passing, must find the stream PENDING: neither ended,
/// nor a tick that needed no time. A loop that ends after a few ticks (B-P38:
/// `for _ in 0..4`) fails at "the stream ENDED before ServerTimeTick #5".
/// That last poll cannot tell a stream PARKED on a never-ready future from a
/// live one waiting for its next tick, so a ticker that freezes right after
/// tick `TICKS` passes here. T-ST-1c reads twice as many paused ticks and
/// catches it (story 2-63 Wave E1a, E1-P11).
///
/// ## Interval mutants, reasoned (story 2-63 Wave B probe ids)
///
/// - 100 ms (B-P14): every gap is 100 ms, so the gap assertion fails. Round 3
///   measured the previous version of this test GREEN under it, because it
///   asserted only `t2 >= t1`.
/// - 1.4 s (B-P30): every gap is 1.4 s, so the gap assertion fails. 1.4 s
///   lies inside the off-runtime test's window, so this test is the one that
///   catches it.
/// - 2 s (B-P28): every gap is 2 s, so the gap assertion fails. The per-tick
///   bound is paused time too, and it is 60 s, so it cannot expire first.
/// - the skip of the immediate tick deleted (B-P34): the first gap is 0, so
///   the gap assertion fails.
///
/// Round 5 measured all four RED at the gap assertion. Freshness is checked
/// per tick, before the gaps are compared at tick 4, and none of these four
/// stamps stale or in the future, so they still fail there.
///
/// The per-tick bound is 60 s of PAUSED time. It is a hang detector only: it
/// costs no real time, and its expiry fails the test.
#[tokio::test(start_paused = true)]
async fn server_time_stream_impl_emits_at_1_hz_cadence() {
    /// `server_time_stream_impl`'s documented period: "once per second".
    const PERIOD: Duration = Duration::from_secs(1);
    /// The first gaps, compared as one array so a failure shows them all.
    const GAPS: usize = 4;
    /// Ticks taken in all: one hour of PAUSED time.
    const TICKS: usize = 3_600;
    const TICK_WAIT: Duration = Duration::from_secs(60);

    /// Freshness, two-sided: `wall`, read right after tick `k` arrived, may
    /// be neither earlier than the stamp nor half a period or more later.
    fn assert_stamped_on_arrival(
        k: usize,
        stamp: trading_core::Timestamp,
        wall: trading_core::Timestamp,
    ) {
        let age = wall.inner() - stamp.inner();
        assert!(
            !age.is_negative(),
            "ServerTimeTick #{k}: the payload is stamped AFTER it was yielded (a future time): \
             the stamp {stamp:?} is {:?} past the wall clock read right after the tick arrived \
             ({wall:?})",
            age.unsigned_abs()
        );
        let half = PERIOD / 2;
        assert!(
            age.unsigned_abs() < half,
            "ServerTimeTick #{k}: the payload must be stamped when it was yielded: the wall clock \
             read right after the tick arrived ({wall:?}) is {:?} past the stamp ({stamp:?}), \
             half a period ({half:?}) or more",
            age.unsigned_abs()
        );
    }

    let rt = tokio::runtime::Handle::current();
    let built_at = tokio::time::Instant::now();
    let mut stream = server_time_stream_impl(&rt);

    let mut previous = built_at;
    let mut first_gaps = Vec::with_capacity(GAPS);
    for k in 1..=TICKS {
        let msg = timeout(TICK_WAIT, stream.next())
            .await
            .unwrap_or_else(|_| {
                panic!("ServerTimeTick #{k}: none within {TICK_WAIT:?} of paused time")
            })
            .unwrap_or_else(|| panic!("the stream ENDED before ServerTimeTick #{k}"));
        let arrived = tokio::time::Instant::now();
        let wall = trading_core::Timestamp::now();
        assert_stamped_on_arrival(k, unwrap_tick(msg), wall);

        let gap = arrived - previous;
        previous = arrived;
        if k <= GAPS {
            first_gaps.push(gap);
            if k == GAPS {
                assert_eq!(
                    first_gaps, [PERIOD; GAPS],
                    "on a paused clock every gap (`stream()` to tick 1, then tick to tick) must be \
                     exactly the documented period, {PERIOD:?}: a shorter gap is a faster \
                     interval, a longer one a slower interval, and a first gap of 0 means the \
                     immediate tick was not skipped"
                );
            }
        } else {
            assert_eq!(
                gap, PERIOD,
                "the gap before ServerTimeTick #{k} must be exactly {PERIOD:?} too: the interval \
                 drifted"
            );
        }
    }

    // Still live: one more poll, with no time passing, must be Pending.
    match futures::FutureExt::now_or_never(stream.next()) {
        None => {}
        Some(None) => panic!(
            "the stream ENDED after {TICKS} ticks: an always-on stream must never end (the \
             status-bar clock would freeze)"
        ),
        Some(Some(msg)) => panic!("a tick after #{TICKS} with no time passing: {msg:?}"),
    }
}

/// T-ST-1c — the clock never freezes and the stream never ends: 7,200 ticks
/// on a paused clock, each within 1.5 s of PAUSED time.
///
/// `ServerTimeRecipe` is a process-lifetime always-on recipe. If its stream
/// ends, or parks without ending, the status-bar clock freezes. With
/// `start_paused = true` tokio jumps the clock to the next timer deadline
/// whenever the runtime is idle, so 7,200 ticks cost real time only for the
/// polls. Each tick is awaited under a timeout of 1.5 s of paused time. On a
/// live 1 Hz ticker the interval's deadline (1 s on) always comes before the
/// timeout's (1.5 s on). On a frozen one the timeout's is the only deadline
/// left, so the clock jumps to it and the test fails. A `None` fails with its
/// own message (the stream ENDED), and every message must be a
/// `ServerTimeTick`.
///
/// ## Why 7,200
///
/// It is twice T-ST-1b's horizon (`const TICKS: usize = 3_600;` in
/// `server_time_stream_impl_emits_at_1_hz_cadence`). T-ST-1b ends on one poll
/// with no time passing, which finds a stream parked on a never-ready future
/// `Pending`, just as it finds a live one waiting for its next tick. So a
/// ticker that freezes right after T-ST-1b's last tick passes T-ST-1b, and
/// this test catches it, at tick 3,601 (E1-P11). A freeze or an end after
/// 7,200 ticks or later passes here too: a stated limit of any finite horizon.
///
/// ## What it does not pin
///
/// The period. Any interval under 1.5 s passes (a 100 ms one, E1-P24, by
/// design), and so does an unskipped immediate tick. T-ST-1b pins both, so no
/// skip check is repeated here.
///
/// The previous revision read three real-time ticks and then accepted a
/// 200 ms timeout as "still open", so a ticker that parked after its third
/// tick passed it.
#[tokio::test(start_paused = true)]
async fn server_time_stream_impl_stream_remains_open() {
    /// Twice T-ST-1b's `TICKS` (3,600): see the doc above.
    const OPEN_TICKS: usize = 7_200;
    /// The per-tick bound, in PAUSED time: 1.5 periods. The failure messages
    /// below say "1.5 s"; change them together.
    const TICK_WAIT: Duration = Duration::from_millis(1_500);

    let rt = tokio::runtime::Handle::current();
    let mut stream = server_time_stream_impl(&rt);

    for k in 1..=OPEN_TICKS {
        let msg = timeout(TICK_WAIT, stream.next())
            .await
            .unwrap_or_else(|_| {
                panic!(
                    "server time: tick {k} of {OPEN_TICKS} \
                     did not arrive within 1.5 s of paused time (the clock froze)"
                )
            })
            .unwrap_or_else(|| panic!("server time: the stream ENDED before tick {k}"));
        assert!(
            matches!(msg, Message::ServerTimeTick(_)),
            "server time: tick {k} of {OPEN_TICKS} is not a ServerTimeTick: {msg:?}"
        );
    }
}
