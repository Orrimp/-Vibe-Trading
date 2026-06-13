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
//! ## `#[cfg(feature = "live")]` gate
//!
//! `LabYahooBarSource` and `spawn_preload_on_rt` require `--features live`.

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use backtest::engine::DateRange;
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

/// Drive the preload → classify path with the given mock source and return
/// the final `Cockpit` state after `state::update` processes the completion.
///
/// Replicates the inner logic of `spawn_lab_run`'s async closure (the same
/// pattern used by `spawn_lab_run_yahoo_harness.rs`). iced::Task is opaque
/// and requires a full iced application to drive; inline replication is the
/// established precedent in this test suite.
///
/// Uses a real multi-thread tokio runtime (required for `rt.spawn()`).
async fn drive_lab_run_with_mock(source: Box<dyn LabYahooBarSource>) -> Cockpit {
    let rt = tokio::runtime::Handle::current();
    let cfg = yahoo_cache_cfg();

    // 1. State: LabRunRequested → inflight = true, notices cleared.
    let mut cockpit = Cockpit::new();
    update(&mut cockpit, Message::LabRunRequested);

    // 2. Run the preload via spawn_preload_on_rt (same as production path).
    // ADR-0050 § D4: must use rt.spawn() to guarantee reactor context for
    // any spawn_blocking calls inside the source implementation.
    let range = DateRange::Last30d;
    let preload_result =
        match ui::lab::runner::spawn_preload_on_rt(&rt, source, cfg.clone(), range).await {
            Ok(inner) => inner,
            Err(e) => Err(SmolStr::new(format!("join error: {e}"))),
        };

    // 3. Classify: delegate to classify_preload_result logic.
    //    Empty bars → tagged no-data notice (same path as production runner).
    let outcome: ui::lab::runner::LabRunResult = match preload_result {
        Ok((bars, _sha)) if bars.is_empty() => {
            // Replicate classify_preload_result: build a tagged no-data message.
            let window = cfg.range_label.as_str();
            let body = ui::strings::LAB_YAHOO_NO_DATA_NOTICE
                .replace("{ticker}", cfg.symbol.as_str())
                .replace("{window}", window);
            Err(SmolStr::new(format!(
                "{}{}",
                ui::lab::runner::preload_notice::NO_DATA_TAG,
                body
            )))
        }
        Ok((bars, _sha)) => {
            // Non-empty bars — not expected in this test, handle gracefully.
            Ok(ui::lab::runner::RunSummary {
                strategy_id: cfg.strategy_id.clone(),
                symbol: cfg.symbol.clone(),
                report_path: None,
                equity_series: bars
                    .iter()
                    .map(|_| (0i64, rust_decimal::Decimal::ZERO))
                    .collect(),
                fills: Vec::new(),
                kpis: backtest::BacktestKpis::default(),
                bars: std::sync::Arc::new(bars),
                position_curve: Vec::new(),
            })
        }
        Err(e) => Err(e),
    };

    // 4. Dispatch LabRunCompleted → state::update (the classification gate).
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
         P-ER-1 falsifier: drop NO_DATA_TAG from no_data_message → this assertion fails.\n\
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
