//! K2 empty-vs-error classification test — lab-yahoo-empty-range-ux v0.1.0.
//!
//! D-ER-4 / M-DEV.12 — REQUIRED GATE.
//!
//! ## What this file tests
//!
//! Two cases exercising the preload classification path:
//!
//! **Case A — no-data (empty source):**
//! A mock returning `Ok((vec![], sha))` drives the full
//! `spawn_lab_run` → `LabRunCompleted(Err(...))` → `state::update` → classify
//! path. Asserts:
//! - `last_run_notice.is_some()` (muted notice rendered)
//! - `last_run_error.is_none()` (red ⚠ NOT shown)
//! - notice string contains no `CacheMiss`, `MissingData`, or `Check network`
//! - notice string names the range window (operator confusion source from Bug #64)
//! - `lab_run_inflight == false` (R3: terminal state, no spinner hang)
//!
//! **Case B — transport error:**
//! A mock returning `Err(SmolStr::new("network error: connection refused"))`
//! (untagged) drives the same path. Asserts:
//! - `last_run_error.is_some()` (red ⚠ shown — K1: genuine errors stay red)
//! - `last_run_notice.is_none()` (muted notice NOT shown)
//!
//! **These two must NOT collapse to the same surface** — that is the K2 gate.
//!
//! ## P-ER-1 falsifier documentation
//!
//! To verify Case A's assertions would catch a regression (the test is not
//! vacuously green):
//!
//! 1. In `crates/ui/src/lab/runner.rs`, edit `preload_notice::no_data_message`
//!    to omit `NO_DATA_TAG` (return just the body without the prefix).
//! 2. Run Case A — `last_run_notice.is_some()` MUST FAIL because `classify`
//!    returns `Error(...)` instead of `Notice(...)`, routing to `last_run_error`.
//! 3. Restore the sentinel. The test returns to green.
//!
//! This proves the test discriminates the two surfaces and would catch any
//! regression that re-collapses no-data into the red error path.
//!
//! Correction (story 2-63 Wave F): the empty-preload path does NOT call
//! `no_data_message` — `classify_preload_result` formats `NO_DATA_TAG` + the
//! notice body itself — so P-ER-1 as written does not reach Case A (and the
//! pre-Wave-F drive, which built the tag in the test, could not see either
//! edit). The probe for this path is T-T4 F-P7 below.
//!
//! ## The drive is the real `spawn_lab_run` (story 2-63 Wave F, Q3)
//!
//! Until Wave F, `drive_lab_run_with_mock` re-implemented
//! `classify_preload_result` inline (it built the tagged no-data message
//! itself) and fed its own result to `state::update`, so a regression in the
//! production classifier left Case A green. It now calls the REAL
//! `ui::lab::runner::spawn_lab_run` with the mock as `yahoo_source_override`
//! and drains the returned `iced::Task` through
//! `iced_test::runtime::task::into_stream`: the `LabRunCompleted` it feeds to
//! `state::update` is the one production builds, through
//! `run_yahoo_preload_arm`.
//!
//! The tests stay `#[tokio::test(flavor = "multi_thread")]`: their subject is
//! the classification plus `state::update`, not executor fidelity (that is the
//! job of `spawn_lab_run_yahoo_harness.rs`, which polls off-runtime). The
//! ambient runtime context also means T-T4 F-P3 (the deleted `rt.enter()`
//! guard) leaves these three tests GREEN — expected and recorded.
//!
//! ## T-T4 falsification probe table (story 2-63 AC4)
//!
//! One exact-once replace in `crates/ui/src/lab/runner.rs`. The OLD is a
//! 3-line block, unique only as a block (count it as a substring; its first
//! line alone also occurs in `run_binance_preload_arm`, so `grep -c` of that
//! line is 2). Run `cargo test -p ui --test lab_yahoo_empty_range_classification`,
//! restore from a `cp -p` snapshot, `cmp`, `touch`, re-run GREEN.
//!
//! | # | symbol | quoted line today | mutation | expected RED |
//! |---|---|---|---|---|
//! | F-P7 | `run_yahoo_preload_arm` (NOT the Binance arm) | the 3-line block `match classify_preload_result(preload_result, cfg) {` / `Ok((bars, _sha)) => {` / `scenario_cfg.data_source = backtest::engine::ScenarioDataSource::YahooCache;` | first line → `match preload_result {` | `case_a_empty_source_routes_to_notice` at its first assert ("last_run_notice must be Some"), and `k2_empty_vs_error_surfaces_are_distinct`; `case_b_transport_error_routes_to_error` stays GREEN (an `Err` passes through either way — expected, recorded) |
//!
//! ## Gate
//!
//! `#![cfg(all(feature = "live", feature = "yahoo"))]` (both default):
//! `LabYahooBarSource` needs `live`, and without `yahoo` `spawn_lab_run`
//! returns early for a YahooCache config, before the override is used.

#![cfg(all(feature = "live", feature = "yahoo"))]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use futures::StreamExt as _;
use iced_test::runtime::Action;
use smol_str::SmolStr;
use ui::lab::runner::{LabBarSource, LabRunConfig, LabYahooBarSource, PreloadFuture};
use ui::lab::state::LabDataSource;
use ui::state::{Cockpit, Message, update};

// ── Mock bar sources ──────────────────────────────────────────────────────────

/// Mock that returns an empty bar list — simulates "Yahoo returned 0 quotes
/// for the window" (future-dated range or delisted ticker, HTTP-200, no error).
struct EmptySuccessMock;

// simple-strategies-realdata T-B3: `preload` body on the shared `LabBarSource`;
// `LabYahooBarSource` is a pure marker tagging this as the Yahoo seam.
impl LabBarSource for EmptySuccessMock {
    fn preload<'a>(
        &'a self,
        _cfg: &'a LabRunConfig,
        _range: &'a backtest::engine::DateRange,
    ) -> PreloadFuture<'a> {
        Box::pin(async move {
            // Simulate fast response (warm cache / future-dated returns immediately).
            tokio::time::sleep(Duration::from_millis(10)).await;
            Ok((
                Vec::new(), // ← 0 bars: the no-data case
                SmolStr::new("mock-sha-empty-0000000000000000"),
            ))
        })
    }
}

impl LabYahooBarSource for EmptySuccessMock {}

/// Mock that returns a transport error — simulates network failure / 429 / timeout.
/// The error string is untagged (no `NO_DATA_TAG` prefix) so `classify` routes
/// it to `last_run_error` (red ⚠, K1).
struct TransportErrMock;

impl LabBarSource for TransportErrMock {
    fn preload<'a>(
        &'a self,
        _cfg: &'a LabRunConfig,
        _range: &'a backtest::engine::DateRange,
    ) -> PreloadFuture<'a> {
        Box::pin(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            Err(SmolStr::new("network error: connection refused"))
        })
    }
}

impl LabYahooBarSource for TransportErrMock {}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn yahoo_cache_cfg() -> LabRunConfig {
    LabRunConfig {
        strategy_id: SmolStr::new("v0.sma"),
        symbol: SmolStr::new("BTCUSDT"),
        venue: SmolStr::new("Binance"),
        range_label: SmolStr::new("Last30d"),
        seed: ui::lab::defaults::LAB_DEFAULT_SEED,
        write_report: false,
        data_source: LabDataSource::YahooCache,
        sma_fast_len: None,
        sma_slow_len: None,
    }
}

/// Drive the REAL `spawn_lab_run` Task with the given mock source as
/// `yahoo_source_override` and return the final `Cockpit` state after
/// `state::update` processes the `LabRunCompleted` the Task produced.
///
/// The cancel handle and the progress receiver stay bound until the Task has
/// completed: dropping the handle early would cancel the run and fail the
/// test for the wrong reason.
async fn drive_lab_run_with_mock(source: Box<dyn LabYahooBarSource>) -> Cockpit {
    let rt = tokio::runtime::Handle::current();
    let cfg = yahoo_cache_cfg();

    // 1. State: LabRunRequested → inflight = true, notices cleared.
    let mut cockpit = Cockpit::new();
    update(&mut cockpit, Message::LabRunRequested);

    // 2. The production Task, with the mock injected at the one Yahoo call
    //    site (`run_yahoo_preload_arm` inside `spawn_lab_run`).
    let (cancel_handle, cancel_rx) = ui::lab::runner::cancellation_pair();
    let (progress_tx, _progress_rx) = backtest::progress::progress_pair();
    let task =
        ui::lab::runner::spawn_lab_run(Some(&rt), cfg, cancel_rx, progress_tx, None, Some(source));
    let mut stream = iced_test::runtime::task::into_stream(task)
        .expect("HARNESS: spawn_lab_run must return a Task with a stream");
    let outcome = match stream.next().await {
        Some(Action::Output(Message::LabRunCompleted(r))) => r,
        Some(Action::Output(other)) => {
            panic!("HARNESS: the Task must output LabRunCompleted; got {other:?}")
        }
        Some(other) => panic!("HARNESS: the Task must output a message; got {other:?}"),
        None => panic!("HARNESS: the Task's stream ended without an output"),
    };
    drop(cancel_handle);

    // 3. Dispatch LabRunCompleted → state::update (the classification gate).
    update(&mut cockpit, Message::LabRunCompleted(outcome));

    cockpit
}

// ── Case A — empty source → NOTICE surface ───────────────────────────────────

/// D-ER-4 T1 Case A — REQUIRED GATE.
///
/// Empty mock (`Ok((vec![], sha))`) → `last_run_notice.is_some()` AND
/// `last_run_error.is_none()`. K2: no-data MUST NOT collapse to red error.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn case_a_empty_source_routes_to_notice() {
    let cockpit = drive_lab_run_with_mock(Box::new(EmptySuccessMock)).await;

    // Primary assertion: notice surface set.
    assert!(
        cockpit.lab_state.last_run_notice.is_some(),
        "Case A (empty source): last_run_notice must be Some — \
         no-data outcome must show a muted notice, not disappear silently.\n\
         T-T4 F-P7 falsifier: bypass classify_preload_result in run_yahoo_preload_arm → this assertion fails.\n\
         Actual notice: {:?}\nActual error: {:?}",
        cockpit.lab_state.last_run_notice,
        cockpit.lab_state.last_run_error,
    );

    // Primary assertion: error surface NOT set.
    assert!(
        cockpit.lab_state.last_run_error.is_none(),
        "Case A (empty source): last_run_error must be None — \
         no-data is NOT a hard error (would mislead operator into retrying).\n\
         Actual notice: {:?}\nActual error: {:?}",
        cockpit.lab_state.last_run_notice,
        cockpit.lab_state.last_run_error,
    );

    // Content assertions: notice must name the range and exclude confusing copy.
    let notice = cockpit.lab_state.last_run_notice.as_ref().unwrap();

    assert!(
        !notice.contains("CacheMiss"),
        "notice must not reference internal variant 'CacheMiss' (R2): {notice:?}"
    );
    assert!(
        !notice.contains("MissingData"),
        "notice must not reference internal variant 'MissingData' (R2): {notice:?}"
    );
    assert!(
        !notice.contains("Check network"),
        "notice must not include 'Check network' hint (R2 — misleading for no-data): {notice:?}"
    );

    // Notice must reference the range — operator's primary confusion source.
    // Under Last30d the window label will be "Last30d" from range_label.
    assert!(
        notice.contains("BTCUSDT") || notice.contains("Last30d"),
        "notice must name the ticker or range so the operator understands the context: {notice:?}"
    );

    // R3: terminal state — no spinner hang.
    assert!(
        !cockpit.lab_run_inflight,
        "Case A: lab_run_inflight must be false (R3 — no spinner hang): {cockpit:?}"
    );
    assert!(
        cockpit.lab_state.run_progress.is_none(),
        "Case A: run_progress must be None after completion (R3): {cockpit:?}"
    );
}

// ── Case B — transport error → ERROR surface ──────────────────────────────────

/// D-ER-4 T1 Case B — REQUIRED GATE.
///
/// Transport-error mock (`Err(SmolStr)` untagged) → `last_run_error.is_some()`
/// AND `last_run_notice.is_none()`. K1: genuine errors MUST stay red ⚠.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn case_b_transport_error_routes_to_error() {
    let cockpit = drive_lab_run_with_mock(Box::new(TransportErrMock)).await;

    // Primary assertion: error surface set.
    assert!(
        cockpit.lab_state.last_run_error.is_some(),
        "Case B (transport error): last_run_error must be Some — \
         genuine errors must remain red ⚠ (K1 / R-NR.2).\n\
         Actual notice: {:?}\nActual error: {:?}",
        cockpit.lab_state.last_run_notice,
        cockpit.lab_state.last_run_error,
    );

    // Primary assertion: notice surface NOT set.
    assert!(
        cockpit.lab_state.last_run_notice.is_none(),
        "Case B (transport error): last_run_notice must be None — \
         transport errors must NOT be downgraded to muted notices (K1).\n\
         Actual notice: {:?}\nActual error: {:?}",
        cockpit.lab_state.last_run_notice,
        cockpit.lab_state.last_run_error,
    );

    // R3: terminal state.
    assert!(
        !cockpit.lab_run_inflight,
        "Case B: lab_run_inflight must be false (R3): {cockpit:?}"
    );
}

// ── Surfaces must NOT be the same (the discriminator gate) ───────────────────

/// K2 discriminator: Cases A and B route to DIFFERENT fields.
///
/// This is the essential non-collapse assertion — that the two mock types
/// produce different `(last_run_notice, last_run_error)` patterns.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn k2_empty_vs_error_surfaces_are_distinct() {
    let cockpit_a = drive_lab_run_with_mock(Box::new(EmptySuccessMock)).await;
    let cockpit_b = drive_lab_run_with_mock(Box::new(TransportErrMock)).await;

    let a_notice = cockpit_a.lab_state.last_run_notice.is_some();
    let a_error = cockpit_a.lab_state.last_run_error.is_some();
    let b_notice = cockpit_b.lab_state.last_run_notice.is_some();
    let b_error = cockpit_b.lab_state.last_run_error.is_some();

    // A: notice=true, error=false.
    // B: notice=false, error=true.
    // They must differ on both fields — if they collapsed to the same pair,
    // the operator cannot distinguish "no data" from "broken".
    assert!(
        (a_notice, a_error) != (b_notice, b_error),
        "K2 FAILED: empty source and transport error route to the same surface \
         ({a_notice}/{a_error} vs {b_notice}/{b_error}). \
         This means 'no data' and 'broken' are indistinguishable to the operator."
    );
    assert!(
        a_notice && !a_error,
        "K2 Case A must route to notice=true/error=false, got notice={a_notice}/error={a_error}"
    );
    assert!(
        !b_notice && b_error,
        "K2 Case B must route to notice=false/error=true, got notice={b_notice}/error={b_error}"
    );
}
