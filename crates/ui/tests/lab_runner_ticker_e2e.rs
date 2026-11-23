//! E2E test — the Yahoo preload ticker fires ≥ 3 times during a 1 s preload,
//! with the PRODUCTION arm polled off-runtime.
//!
//! Bug #64 D.1.1 attempt-3 / ADR-0050 § D3; rewritten by story 2-63 (v0.3)
//! Wave F.
//!
//! ## What this test proves
//!
//! D-R1.3: with the D-R1.1 guard (`let _guard = rt.enter()` before
//! `tokio::time::interval(250ms)`), the ticker fires ≥ 3 times during a 1 s
//! mock preload, with non-decreasing `elapsed_ms`.
//!
//! The subject is `ui::lab::runner::run_yahoo_preload_arm`, the function
//! `spawn_lab_run` calls for every Yahoo source. The test calls it through
//! `tests/yahoo_arm_support/mod.rs`, which polls it with
//! `futures::executor::block_on` on a bare thread with NO tokio context (an
//! in-future `HARNESS:` assert checks that) — how iced's `futures::ThreadPool`
//! polls the Lab Task. The runtime the arm gets a `Handle` to is never
//! entered.
//!
//! Until Wave F this file copied the arm's ticker loop inline and ran the copy
//! under a multi-thread `#[tokio::test]`, whose ambient runtime context made
//! the guard unobservable: deleting it from `crates/ui/src/lab/runner.rs` left
//! the copy green. The premise (a tokio timer built off-runtime panics) is
//! pinned separately by
//! `lab_runner_cold_cache_fetch_e2e.rs::tokio_time_timeout_without_rt_enter_panics`.
//!
//! **What a missing guard looks like** (verified in the pinned tokio 1.52.3
//! source, ADR-0050 § Changelog, story 2-63 Wave F; measured by T-T4 F-P3
//! below): not a silent hang but a PANIC on the polling thread, "there is no
//! reactor running, must be called from the context of a Tokio 1.x runtime".
//! `support::Run::wait_done`
//! surfaces it as `arm thread panicked: <payload>`.
//!
//! ## T-T4 falsification probe table (story 2-63 AC4)
//!
//! Each probe is ONE exact-once replace in `crates/ui/src/lab/runner.rs`
//! (`grep -c` of the OLD text is 1), located by symbol + quoted line. Run
//! `cargo test -p ui --test lab_runner_ticker_e2e`, restore from a `cp -p`
//! snapshot, `cmp` it, `touch` it, re-run GREEN.
//!
//! | # | symbol | quoted line today | mutation | expected RED |
//! |---|---|---|---|---|
//! | F-P3 (AC4, designated here) | `run_yahoo_preload_arm` | `let _guard = rt.enter(); // enter tokio reactor context` | replace with `// F-P3: guard deleted` | `ticker_fires_at_least_3_times_in_1s_window` at its `wait_done`: "arm thread panicked: there is no reactor running, must be called from the context of a Tokio 1.x runtime" |
//! | F-P2 (AC4) | `run_yahoo_preload_arm` | `_ = ticker.tick() => {` | `_ = std::future::pending::<()>() => {` | the same test at "the ticker must fire ≥ 3 times during a 1000 ms preload; got 0" |
//! | F-P12 | `run_yahoo_preload_arm` | `tokio::time::interval(std::time::Duration::from_millis(250))` | `250` → `500` | the same assert (1-2 ticks: the 1000 ms tick can fall due just before the preload's 1000 ms sleep ends) |

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "yahoo_arm_support/mod.rs"]
mod support;

use std::time::Duration;

use support::{Behavior, Spec, start_arm};

// ── Test — the ticker fires ≥ 3 times in a 1 s preload window ────────────────

/// T-BUG64-D3 / D-R1.3 / ADR-0050 § D3.
///
/// **Pass condition**: the production arm, polled with no tokio context,
/// returns `Ok(())` and emits ≥ 3 `(0, 1, elapsed > 0)` ticker events with
/// non-decreasing `elapsed_ms` during a 1000 ms mock preload. The sentinel
/// `(0, 1, 0)` is not counted.
#[test]
fn ticker_fires_at_least_3_times_in_1s_window() {
    let mut run = start_arm(Spec::new(1_000, Behavior::Bars(10)));

    // `wait_done` reports a panic on the arm thread as
    // "arm thread panicked: <payload>" (the F-P3 red).
    let done = run
        .wait_done(Duration::from_secs(3))
        .expect("the arm must return within 3 s of a 1000 ms preload");
    assert_eq!(done.result, Ok(()), "a 10-bar preload must succeed");

    let items = run.items_until_marker(Duration::from_secs(1));
    for (p, _) in &items {
        assert!(
            p.current_bar == 0 && p.total_bars == 1,
            "every preload progress item must be (0, 1, ·); got {p:?}"
        );
    }
    let ticks: Vec<u64> = items
        .iter()
        .map(|(p, _)| p.elapsed_ms)
        .filter(|&ms| ms > 0)
        .collect();
    assert!(
        ticks.len() >= 3,
        "the ticker must fire ≥ 3 times during a 1000 ms preload; got {} ({ticks:?})",
        ticks.len()
    );
    assert!(
        ticks.windows(2).all(|w| w[1] >= w[0]),
        "ticker elapsed_ms must be non-decreasing; got {ticks:?}"
    );
}
