//! Smoke tests for the `cockpit_live` Lab-Run path.
//!
//! ## Why this test file exists
//!
//! Bug #63 (cross-sectional Stop + progress) and Bug #64 (progress bar stuck)
//! both escaped detection because the CLI uses `cancellation_pair()` +
//! `ProgressSender::disabled()` to keep backtest output byte-identical for
//! anchors — the live UI path NEVER ran in CI.
//!
//! These tests exercise the `cockpit_live` Lab-Run pipeline WITHOUT booting
//! iced: they drive `backtest::engine::run_scenario` directly through real
//! progress and cancel plumbing and assert on the `Cockpit` state post-run —
//! closing the P4 D− coverage gap identified in
//! `docs/dev-notes/archive/2026-Q2/testing-strategy-review-2026-05-25.md`.
//!
//! ## Option A (selected): what is production code here, and what is a copy
//!
//! Each test builds the cockpit model and lab state directly, runs the engine
//! on a tokio runtime with the real `(ProgressSender, Receiver<Progress>)` and
//! `(RunCancelHandle, RunCancelReceiver)` pairs, relays progress through the
//! production `ui::lab::progress::stream_impl`, and drives the lifecycle
//! messages through the production `ui::state::update`. No iced application is
//! booted and no `iced::Task` future is driven.
//!
//! Three pieces are TEST-LOCAL COPIES of binary-side or private code, so an
//! edit to the original is invisible here (the story 2-63 K3 #6 residual):
//!
//! - `apply_wrapper` copies the `RunReportMirror` rotation of the binary's
//!   `AppState::update` (`crates/ui/src/bin/cockpit_live.rs`, the block under
//!   `// T-D1.3 (lab-end-to-end-v2 T-AR-1) — rotate RunReportMirror:`). The same
//!   copy is the `fn apply_wrapper` of `lab_run_integration.rs` and of
//!   `lab_run_real_engine.rs`.
//! - `run_with_progress` mirrors the `RunSummary` assembly of `spawn_lab_run`'s
//!   non-Yahoo arm.
//! - `run_yahoo_smoke` (the body of `smoke_yahoo_cache_hit`) re-composes the
//!   steps of the private `preload_yahoo_bars` (its doc lists them).
//!
//! ## Test variants
//!
//! 1. `smoke_synthetic_short_range`  — v0.sma × BTCUSDT × Last30d (fast happy path)
//! 2. `smoke_synthetic_longer_range` — v0.sma × BTCUSDT × Last90d (multiple progress cycles)
//! 3. `smoke_cancel_mid_run`         — cancel after the first progress event, on
//!    a run of `CANCEL_RUN_BARS` synthetic bars, whose length it pins through
//!    `total_bars` (load-sensitive; its doc says why)
//! 4. `smoke_yahoo_cache_hit`        — v0.sma × BTCUSDT × H1_2024 on the COMMITTED
//!    sample corpus `data/yahoo-sample` (`yahoo` feature; it never skips): the
//!    body `run_yahoo_smoke` at the committed root
//! 5. `sample_corpus_absent_is_a_failure_not_a_skip` — the same body FAILS on an
//!    empty root, at M1 (`yahoo` feature): measures the body up to its helper call
//! 6. `sample_corpus_without_bars_is_a_failure_not_a_skip` — the same body FAILS
//!    on a root holding only the corpus manifest, at the load (`yahoo` feature):
//!    measures the body through the load
//! 7. `smoke_empty_selection_lifecycle_flags` — pair=None: the lifecycle flags
//!    through `update`, and no rotation
//!
//! ## Non-vacuity contract (bug-log #66 / story 2-63 AC3)
//!
//! - **No skip path.** The Yahoo body, `run_yahoo_smoke`, reads the corpus at
//!   the root its caller passes. `smoke_yahoo_cache_hit` passes
//!   `data/yahoo-sample`, resolved from `CARGO_MANIFEST_DIR` (never the process
//!   cwd), and `assert_sample_corpus` PANICS when its `REVISION.toml` is
//!   absent. Until story 2-63 Wave E2 this test probed the cwd-relative,
//!   gitignored `data/yahoo/`, printed a skip line and returned, so it passed
//!   on every CI run without running: the exact bug-log #66 shape.
//! - **"Fails, not skips", measured stage by stage, up to and including the
//!   load.** Each stage of the Yahoo body up to and including its
//!   `load_cached` call has its own measurement:
//!   - up to M1, the helper call: `sample_corpus_absent_is_a_failure_not_a_skip`
//!     runs the body on an empty temp dir and requires the helper's "broken
//!     checkout" panic. T-T4 E2-P13 puts the #66 skip shape in the helper
//!     `assert_sample_corpus`; E2-P14 puts it in the body, before its helper
//!     call. Each turns that control red (MEASURED, falsifier pass 2).
//!   - through the load: `sample_corpus_without_bars_is_a_failure_not_a_skip`
//!     runs the body on a temp dir holding only a copy of the committed
//!     `REVISION.toml`, so M1 passes and `load_cached` fails, and requires the
//!     load's `.expect` panic. E2-P19 puts the #66 skip shape on that load error
//!     and turns it red (MEASURED, falsifier pass 3, 2026-11-15: "test did not
//!     panic as expected"; pass 2's OWN-2 MEASURED that the empty-root control
//!     alone stays green under it). Its fixture guard is live too (MEASURED,
//!     falsifier pass 3, 2026-11-15, its OWN-A; variant 6's doc).
//!   - after the load: UNMEASURED for "fails, not skips". A skip on an engine
//!     or report error after the load (say, `let Ok(report) = .. else { return;
//!     };`) would leave all seven tests green (REASONED): the engine returns
//!     `Ok` on the committed corpus, and both controls panic before the engine
//!     runs. No control can make the engine fail on the committed corpus
//!     without a production change. What IS measured there is value, on a
//!     working corpus: `smoke_yahoo_cache_hit` runs the asserts from M3 on, and
//!     E2-P1 and E2-P2 turn them red, so they run and fire (MEASURED, falsifier
//!     pass 2, 2026-11-14).
//!
//!   `smoke_yahoo_cache_hit` stays green under E2-P13, E2-P14 and E2-P19,
//!   because the corpus is present: that is how #66 hid.
//! - **Strict drains.** Every progress drain goes through
//!   `drain_progress_to_done`: a timeout panics, a stream end before
//!   `LabRunProgressDone` panics (M7), and nothing may follow `Done`. The
//!   drains it replaced accepted `Done` and `None` alike.
//! - **Exact counts, derived.** The Yahoo test pins the bar count, the progress
//!   total and the final progress bar to the calendar (H1 2024 =
//!   31+29+31+30+31+30 = 182 days), not to "non-empty". "The engine ran over
//!   every bar" is carried by M5 and M6 (T-T4 E2-P2; Wave E2's falsifier probe
//!   E2-F3 measured M5), not by a `report.bars` count: the engine echoes its
//!   input bars into the report, so that count could not fail apart from M4.
//!   The cancel test pins its run length from its constant, on the first
//!   progress event past bar 0: `total_bars == CANCEL_RUN_BARS`. Two probes
//!   turn that pin red: T-T4 E2-P18 (the override lost, left 2160, right
//!   500000) and E2-F3 (a total one too high, left 500001, right 500000; its
//!   first red is M5), both MEASURED (falsifier pass 3, 2026-11-15, under the
//!   pin's message before pass 4's rewording; E2-P18 again by the pass-4
//!   fixer, 2026-11-15, under today's message). Later events' totals are M5's
//!   to pin, on the Yahoo run (the cancel test's doc says why).
//!
//! ## Features, and where this runs
//!
//! `live` gates the file; `yahoo` gates the corpus tests (variants 4, 5 and 6).
//! Both are DEFAULT features of `ui`, so all seven tests compile and run under
//! `cargo test -p ui --test cockpit_live_lab_run_smoke`, and under both CI ui
//! steps (`.github/workflows/ci.yml`; `--features` adds to the defaults, it
//! does not replace them):
//!
//! - macOS: `cargo test -p ui --no-fail-fast`
//! - Windows: `cargo test -p ui --features fixtures --no-fail-fast`
//!
//! ## T-T4 falsification probe table (story 2-63 AC4)
//!
//! Each probe is ONE edit, located by symbol + the quoted line text (line
//! numbers rot). Procedure:
//!
//! 1. snapshot the file (`cp -p <file> <snap>`) and apply the edit;
//! 2. run `cargo test -p ui --test cockpit_live_lab_run_smoke` — the named
//!    test(s) go RED;
//! 3. restore from the snapshot, THEN `touch <file>`. A restore that keeps the
//!    snapshot's older mtime (`cp -p`) leaves cargo's fingerprint "fresh": the
//!    next run compiles nothing and re-executes the MUTANT binary;
//! 4. re-run. The GREEN counts only if that run's output shows `Compiling` for
//!    the edited crate and the file's SHA-256 equals the pre-probe one.
//!
//! E2-P1, E2-P2 and E2-P18 edit the `data` and `backtest` crates, so every
//! dependent crate rebuilds before the test binary relinks: expect a long run.
//! E2-P13, E2-P14 and E2-P19 edit THIS file (test code, the #66 shape itself);
//! the restore and the SHA-256 check apply to it the same way.
//!
//! The Yahoo assertions live in `run_yahoo_smoke`, so a red there is reported
//! under the test that called it: `smoke_yahoo_cache_hit` for the committed
//! corpus, `sample_corpus_absent_is_a_failure_not_a_skip` for the empty root,
//! `sample_corpus_without_bars_is_a_failure_not_a_skip` for the manifest-only
//! root.
//!
//! | # | symbol (file) | quoted line today | mutation | expected RED |
//! |---|---|---|---|---|
//! | E2-P1 | `data::yahoo::YahooBarSource::load_cached`, step 5 clip (`crates/data/src/yahoo.rs`) | `ts_ms >= start_ms && ts_ms < end_ms` | `>=` → `>` | `smoke_yahoo_cache_hit`, in `run_yahoo_smoke`, at M4 "the committed sample covers every day of H1 2024" (left 181, right 182). `load_cached` still returns Ok: 181 is above its 95 % floor of 173. Not unique: `data`'s `yahoo_revision_verify::happy_path` (`loaded1.loaded_count, 31,`) is reasoned red too |
//! | E2-P2 | `backtest::scenarios::sma_composed_run::run`, bar loop (`crates/backtest/src/scenarios/sma_composed_run.rs`) | `#[allow(clippy::verbose_bit_mask)] // bitmask is more readable than trailing_zeros here` and, below it, `let poll_now = bar_idx == bar_count.saturating_sub(1)` (that second line alone also occurs in `run_with_strategy`) | → `let poll_now = false` (the final-bar clause dropped) | `smoke_yahoo_cache_hit`, in `run_yahoo_smoke`, at M6 "the last progress event is the final bar (Bug #64 final-bar emit)" (left `Some(128)`, right `Some(181)`). Both synthetic tests stay green, and so does `smoke_cancel_mid_run` (it cancels long before its final bar) |
//! | E2-P3 | `ui::lab::progress::stream_impl` (`crates/ui/src/lab/progress.rs`) | `yield Message::LabRunProgressDone;` | delete it | `smoke_synthetic_short_range`, `smoke_synthetic_longer_range`, `smoke_cancel_mid_run` (its post-cancel drain) and `smoke_yahoo_cache_hit`, each at M7 "progress stream ended without LabRunProgressDone". `sample_corpus_absent_is_a_failure_not_a_skip` stays green: its body panics at M1, before any stream exists. So does `sample_corpus_without_bars_is_a_failure_not_a_skip`: its body panics at the load, also before any stream exists (MEASURED, falsifier pass 3, 2026-11-15, which also found `smoke_cancel_mid_run`'s post-cancel drain empty: "after 0 progress events") |
//! | E2-P13 | `assert_sample_corpus` (this file, test code) | the condition `root.join("REVISION.toml").is_file()`, on the line under `assert!(` (the exact two-line text, with its indentation and trailing comma, occurs once in this file) | insert the bug-log #66 skip shape, `if !<that condition> { return; }`, before the `assert!(` | `sample_corpus_absent_is_a_failure_not_a_skip`: "panic did not contain expected string" (libtest's `should_panic` mismatch). The helper returns, so the body goes on and panics at its `load_cached` `.expect`, "the committed sample corpus must load BTC-USD 1d over H1 2024", which lacks "absent means a broken checkout" (MEASURED, falsifier pass 2, 2026-11-14: "panic did not contain expected string", the panic message being that `.expect` text with `RevisionMissing`; before Wave E2's follow-up, when the control called the helper alone, it was MEASURED as "test did not panic as expected"). `smoke_yahoo_cache_hit` stays GREEN, because the corpus is present: that is exactly how #66 hid (MEASURED, pass 2). `sample_corpus_without_bars_is_a_failure_not_a_skip` stays GREEN: its root holds `REVISION.toml`, so the skip shape does not fire (MEASURED, falsifier pass 3, 2026-11-15) |
//! | E2-P14 | `run_yahoo_smoke` (this file, test code), its M1 step | the body's one call of the helper: `assert_sample_corpus` on `&root` (that exact line, with its 4-space indentation, occurs once in this file) | insert the bug-log #66 skip shape in the BODY, `if !root.join("REVISION.toml").is_file() { return; }`, before that call | `sample_corpus_absent_is_a_failure_not_a_skip`: "test did not panic as expected" (the body returns on the empty root; MEASURED, falsifier pass 2, 2026-11-14). `smoke_yahoo_cache_hit` stays GREEN (the corpus is present; MEASURED, pass 2). `sample_corpus_without_bars_is_a_failure_not_a_skip` stays GREEN: its root holds `REVISION.toml`, so the skip shape does not fire (MEASURED, falsifier pass 3, 2026-11-15). Before the story 2-63 Wave E2 follow-up the control called only the helper, so this probe stayed GREEN there (REASONED) |
//! | E2-P18 | `backtest::engine::run_scenario`, the v0.sma dispatch arm (`crates/backtest/src/engine.rs`) | `// SMA arm never uses in-memory TOML override (SMA has a typed seam).` (once in that file) and, five lines below it, the `cfg.bars_override.clone(),` argument to `sma_composed_run::run` (that argument line alone occurs 10 times in the file, so the edit is anchored on the comment) | the argument → `if cfg.data_source == ScenarioDataSource::Synthetic { None } else { cfg.bars_override.clone() }` (the Synthetic source ignores the override) | `smoke_cancel_mid_run`, at "the engine ran exactly the override's CANCEL_RUN_BARS bars, so the cancel has its measured ~0.49 s window (a lost override gives Last90d's 2 160 bars and ~2 ms)" (left 2160, right 500000; 2 160 = 90 × 24, `Last90d` in `date_range_to_scenario_params`, derived from source). MEASURED (falsifier pass 3, 2026-11-15) at the same assert under its message before pass 4's rewording, left 2160, right 500000; MEASURED (pass-4 fixer, 2026-11-15) under this message, left 2160, right 500000, the other 6 tests GREEN. Pass 2 MEASURED the same edit as its OWN-1 with all six tests then present GREEN, the cancel test included. The others stay GREEN (MEASURED, falsifier pass 3 and pass-4 fixer, both 2026-11-15: the other 6, the two #66 controls included): the synthetic tests pass no override, `smoke_yahoo_cache_hit` runs the `YahooCache` source, which keeps it, and the two #66 controls panic before the engine runs. Not this row's only red: falsifier probe E2-F3 reddens the same assert with left 500001 (the cancel test's doc) |
//! | E2-P19 | `run_yahoo_smoke` (this file, test code), its step-4 load | the three-line `let loaded = YahooBarSource::new(root)` statement that ends in the `.expect` "the committed sample corpus must load BTC-USD 1d over H1 2024" (that exact three-line text, with its indentation, occurs once in this file) | → `let Ok(loaded) = YahooBarSource::new(root).load_cached(..) else { return; };`, the bug-log #66 skip shape on a load error, after the helper call | `sample_corpus_without_bars_is_a_failure_not_a_skip`: "test did not panic as expected" (the body returns at the cache miss). `sample_corpus_absent_is_a_failure_not_a_skip` stays GREEN (its body panics at M1, before the load) and so does `smoke_yahoo_cache_hit` (the corpus loads). MEASURED (falsifier pass 3, 2026-11-15): "test did not panic as expected" on this control, the other 6 tests GREEN. Pass 2 MEASURED the same edit as its OWN-2, with both of those GREEN and no test red |

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use futures::stream::BoxStream;
use tokio::time::timeout;
use trading_core::{StrategyId, Symbol, Venue};

use backtest::cancel::cancellation_pair;
use backtest::progress::{Progress, progress_pair};
use ui::lab::equity_loader::LabTuple;
use ui::lab::runner::{LabRunConfig, RunReportMirror, RunSummary, lab_config_to_scenario};
use ui::lab::state::{DateRange, LabDataSource, Preset};
use ui::state::{Cockpit, Message};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build a `LabRunConfig` with Synthetic data source (default path).
fn synthetic_cfg(strategy: &str, symbol: &str, range: DateRange) -> LabRunConfig {
    LabRunConfig {
        strategy_id: smol_str::SmolStr::new(strategy),
        symbol: smol_str::SmolStr::new(symbol),
        venue: smol_str::SmolStr::new("Binance"),
        range,
        seed: ui::lab::defaults::LAB_DEFAULT_SEED,
        write_report: false,
        data_source: LabDataSource::Synthetic,
        sma_fast_len: None,
        sma_slow_len: None,
    }
}

/// Drain a Lab progress stream to its END, strictly (bug-log #66 / story 2-63
/// AC3). Returns every `LabRunProgress` that preceded `LabRunProgressDone`.
///
/// It panics when:
/// - nothing resolves for 120 s (a timeout is never read as "drained");
/// - the stream ENDS (`None`) before `LabRunProgressDone` arrives: M7, the R7.4
///   belt-and-braces clear never reached the UI (T-T4 E2-P3);
/// - anything other than `LabRunProgress` precedes `Done`;
/// - the stream does not end within 2 s of `Done`, or yields anything after it.
///
/// The drains it replaced broke out of their loop on `Done | None` alike, so a
/// relay that never sent `Done` passed them.
async fn drain_progress_to_done(stream: &mut BoxStream<'static, Message>) -> Vec<Progress> {
    let mut events = Vec::new();
    loop {
        match timeout(Duration::from_secs(120), stream.next())
            .await
            .expect("progress stream must not hang for more than 120 s")
        {
            Some(Message::LabRunProgress(p)) => events.push(p),
            Some(Message::LabRunProgressDone) => break,
            None => panic!(
                "M7: progress stream ended without LabRunProgressDone (after {} progress events)",
                events.len()
            ),
            Some(other) => panic!("unexpected message from the progress stream: {other:?}"),
        }
    }
    let after = timeout(Duration::from_secs(2), stream.next())
        .await
        .expect("the progress stream must end within 2 s of LabRunProgressDone");
    assert!(
        after.is_none(),
        "nothing may follow LabRunProgressDone (got {after:?})"
    );
    events
}

/// Run the backtest engine, collect progress events, and return `(summary,
/// progress_events)`.
///
/// Mirrors the body of `spawn_lab_run` for the `live` + non-Yahoo path.
/// The `ProgressSender` is the real one; the receiver is consumed here via
/// `stream_impl` so we can assert on progress messages.
async fn run_with_progress(
    cfg: LabRunConfig,
) -> (Result<RunSummary, smol_str::SmolStr>, Vec<Progress>) {
    let scenario_cfg = lab_config_to_scenario(&cfg).expect("lab_config_to_scenario must succeed");

    // Real cancel + progress pair — the live path that was NEVER exercised in CI.
    let (_handle, cancel_rx) = cancellation_pair();
    let (progress_tx, progress_rx) = progress_pair();

    // Wrap receiver for stream_impl (mirrors cockpit_live's Arc<Mutex<Option<_>>>).
    let mut progress_stream = ui::lab::progress::stream_impl(Some(progress_rx));

    // Spawn the engine on a real tokio task so the progress sender can emit
    // concurrently while we drain progress_stream.
    let engine_task = tokio::spawn(async move {
        match backtest::engine::run_scenario(scenario_cfg, cancel_rx, progress_tx).await {
            Ok(report) => {
                let equity_series: Vec<(i64, rust_decimal::Decimal)> = report
                    .equity_series
                    .iter()
                    .map(|(ts, money)| (ts.unix_millis(), money.amount()))
                    .collect();
                let active_sym = Symbol::new(cfg.symbol.as_str());
                let position_curve: Vec<(i64, rust_decimal::Decimal)> = report
                    .position_curve_raw
                    .iter()
                    .filter(|(_, _, s)| s == &active_sym)
                    .map(|&(ts, qty, _)| (ts, qty))
                    .collect();
                Ok(RunSummary {
                    strategy_id: cfg.strategy_id.clone(),
                    symbol: cfg.symbol.clone(),
                    report_path: report.report_path.clone(),
                    equity_series,
                    fills: report.fills.clone(),
                    kpis: report.kpis.clone(),
                    bars: report.bars.clone(),
                    position_curve,
                })
            }
            Err(e) => Err(smol_str::SmolStr::new(format!("{e}"))),
        }
    });

    // Drain to `LabRunProgressDone`, then the stream end (strict: see the helper).
    let progress_events = drain_progress_to_done(&mut progress_stream).await;

    let summary = engine_task.await.expect("engine task must not panic");

    (summary, progress_events)
}

/// A TEST-LOCAL COPY of the binary-side `RunReportMirror` rotation, so that
/// `last_run_report` is populated after `LabRunCompleted`.
///
/// The original is the block under
/// `// T-D1.3 (lab-end-to-end-v2 T-AR-1) — rotate RunReportMirror:`
/// in `AppState::update` (`crates/ui/src/bin/cockpit_live.rs`), a bin target an
/// integration test cannot link. An edit there is invisible here (story 2-63 K3 #6 residual);
/// `lab_run_integration.rs` and `lab_run_real_engine.rs` carry the same copy.
fn apply_wrapper(cockpit: &mut Cockpit, summary: &RunSummary) {
    let pre_tuple = {
        let ls = &cockpit.lab_state;
        match (ls.strategy.as_ref(), ls.pair.as_ref()) {
            (Some(strategy), Some((venue, symbol))) => Some(LabTuple::new(
                strategy,
                *venue,
                symbol,
                ls.range.clone(),
                ls.data_source,
            )),
            _ => None,
        }
    };
    if let Some(tuple) = pre_tuple {
        let mirror = RunReportMirror {
            tuple,
            equity_series: Arc::new(summary.equity_series.clone()),
            kpis: summary.kpis.clone(),
            generated_at: ::time::OffsetDateTime::now_utc(),
            bars: summary.bars.clone(),
            position_curve: Arc::new(summary.position_curve.clone()),
        };
        let prev = cockpit.lab_state.last_run_report.take();
        cockpit.lab_state.prev_run_report = prev;
        cockpit.lab_state.last_run_report = Some(mirror);
    }
}

// ── Test variant 1: Synthetic short range (Last30d) ───────────────────────────

/// Variant 1 — fast happy path: v0.sma × BTCUSDT × Last30d (Synthetic).
///
/// Asserts:
/// - At least one `LabRunProgress(p)` with `p.current_bar > 0` arrives.
/// - The progress stream ends with `LabRunProgressDone` (strict drain, M7).
/// - The engine returns Ok, and `LabRunCompleted(Ok(_))` clears the flags.
/// - After the `apply_wrapper` copy, `cockpit.lab_state.last_run_report.is_some()`.
/// - `cockpit.lab_state.run_progress` is `None` (cleared after completion).
///
/// Covers: Bug #63 / Bug #64 live progress path — verifies that real
/// `ProgressSender` + `progress_pair()` actually delivers events to the UI.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn smoke_synthetic_short_range() {
    let cfg = synthetic_cfg("v0.sma", "BTCUSDT", DateRange::Preset(Preset::Last30d));

    let mut cockpit = Cockpit::new();
    cockpit.lab_state.strategy = Some(StrategyId::new("v0.sma"));
    cockpit.lab_state.pair = Some((Venue::Binance, Symbol::new("BTCUSDT")));
    cockpit.lab_state.range = DateRange::Preset(Preset::Last30d);

    // Step 1: dispatch LabRunRequested (sets lab_run_inflight = true).
    ui::state::update(&mut cockpit, Message::LabRunRequested);
    assert!(
        cockpit.lab_run_inflight,
        "lab_run_inflight must be true after LabRunRequested"
    );

    // Step 2: run engine with real progress plumbing.
    let (result, progress_events) = run_with_progress(cfg).await;

    // Step 3: assert at least one progress event with current_bar > 0.
    let meaningful_progress = progress_events.iter().any(|p| p.current_bar > 0);
    assert!(
        meaningful_progress,
        "must receive at least one LabRunProgress with current_bar > 0; \
         got {} events: {progress_events:?}",
        progress_events.len()
    );

    // Step 4: engine must succeed.
    let summary = result.expect("engine must return Ok for v0.sma × BTCUSDT × Last30d");

    // Step 5: dispatch LabRunCompleted and apply the binary-side rotation copy.
    ui::state::update(&mut cockpit, Message::LabRunCompleted(Ok(summary.clone())));
    apply_wrapper(&mut cockpit, &summary);

    // Step 6: assert post-conditions (closing conditions from the feature spec).
    assert!(
        !cockpit.lab_run_inflight,
        "lab_run_inflight must be false after LabRunCompleted"
    );
    assert!(
        cockpit.lab_state.last_run_report.is_some(),
        "last_run_report must be Some after wrapper rotation"
    );
    assert!(
        cockpit.lab_state.run_progress.is_none(),
        "run_progress must be None after LabRunCompleted (state::update clears it)"
    );
}

// ── Test variant 2: Synthetic longer range (Last90d) ─────────────────────────

/// Variant 2 — longer range: v0.sma × BTCUSDT × Last90d (Synthetic).
///
/// Last90d generates ~2160 hourly bars, so the engine emits multiple progress
/// events (every 128 bars steady-state). This verifies that progress emission
/// works across multiple cycles, not just the first one.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn smoke_synthetic_longer_range() {
    let cfg = synthetic_cfg("v0.sma", "BTCUSDT", DateRange::Preset(Preset::Last90d));

    let mut cockpit = Cockpit::new();
    cockpit.lab_state.strategy = Some(StrategyId::new("v0.sma"));
    cockpit.lab_state.pair = Some((Venue::Binance, Symbol::new("BTCUSDT")));
    cockpit.lab_state.range = DateRange::Preset(Preset::Last90d);

    ui::state::update(&mut cockpit, Message::LabRunRequested);

    let (result, progress_events) = run_with_progress(cfg).await;

    // With Last90d (~2160 bars), expect multiple progress cycles.
    // 2160 / 128 ≈ 16 steady-state events + early events every 32 bars.
    assert!(
        progress_events.len() >= 2,
        "Last90d must emit at least 2 progress events (covers multiple cycles); \
         got {}: {progress_events:?}",
        progress_events.len()
    );

    // Verify progress is monotonically non-decreasing.
    for window in progress_events.windows(2) {
        assert!(
            window[1].current_bar >= window[0].current_bar,
            "progress current_bar must be non-decreasing: {} then {}",
            window[0].current_bar,
            window[1].current_bar
        );
    }

    let summary = result.expect("engine must return Ok for v0.sma × BTCUSDT × Last90d");
    ui::state::update(&mut cockpit, Message::LabRunCompleted(Ok(summary.clone())));
    apply_wrapper(&mut cockpit, &summary);

    assert!(
        cockpit.lab_state.last_run_report.is_some(),
        "last_run_report must be Some after wrapper rotation (Last90d)"
    );
    let report = cockpit.lab_state.last_run_report.as_ref().unwrap();
    assert!(
        !report.equity_series.is_empty(),
        "equity_series must be non-empty for Last90d run"
    );
}

// ── Test variant 3: Cancel mid-run ───────────────────────────────────────────

/// The length of the cancel test's run, in synthetic bars.
///
/// MEASURED once on the dev box, 2026-11-14 (Apple M4 Pro, 14 cores, 24 GiB,
/// macOS 26.7.2, the debug test profile, load average about 4.6): an
/// UNcancelled run of these 500 000 bars took 576 ms from spawn to return, and
/// its final-bar progress event reported `elapsed_ms: 489` against `0` at bar
/// 32. So the cancel has about 0.49 s to land, where `Last90d`'s 2 160 bars
/// gave it about 2 ms (the same cost per bar, about 1 µs). Generating the bars
/// took 1.59 s, which the passing case pays too.
///
/// Why not the 2 s margin story 2-63 Wave E2's follow-up asked for (MEASURED on
/// the same box, the cost is linear in the bar count): it needs about 2.1
/// million bars, so about 6.7 s of generation on every passing run, and the
/// engine's dispatch clones `bars_override` once, so the bars are held twice
/// (peak RSS was 254 MB at 500 000 bars, uncancelled). A larger margin at no
/// such cost needs a production knob, which a test-only wave does not add.
const CANCEL_RUN_BARS: usize = 500_000;

/// The cancel test's bars: the engine's own synthetic generator
/// (`synthetic_bars_minute`, the one the Synthetic arm calls itself when
/// `bars_override` is `None`), with a fixed seed, at [`CANCEL_RUN_BARS`].
fn cancel_run_bars() -> Vec<trading_core::Bar> {
    use backtest::scenarios::sma_composed_run::{default_start_price, synthetic_bars_minute};
    let symbol = Symbol::new("BTCUSDT");
    synthetic_bars_minute(
        &symbol,
        CANCEL_RUN_BARS,
        0x2_63E2,
        default_start_price(&symbol),
        2023,
    )
}

/// Variant 3 — cancel mid-run: drop the `RunCancelHandle` after the first
/// progress event arrives, then assert `LabRunCompleted(Err(_))` with a
/// cancellation marker.
///
/// This is the core Bug #63 regression gate: the Stop button drops
/// `lab_state.run_cancel`, which drops the `RunCancelHandle`. The engine
/// observes `is_cancelled() == true` at the next poll boundary and returns
/// `Err(RunError::Cancelled)`.
///
/// After the cancelled engine returns (dropping its `ProgressSender`), the
/// progress relay must still end with `LabRunProgressDone`: the R7.4 clear on
/// the cancel path (strict drain, M7).
///
/// ## A real-time race, with a measured margin (load-sensitive)
///
/// The engine never waits for this test. The cancel must land between the
/// first progress event with `current_bar > 0` (bar 32) and the engine's LAST
/// cancel poll (its final bar); if the engine gets there first, it returns
/// `Ok` and the `Cancelled` assert goes RED (red, never a false pass). On the
/// 2 160 bars of `Last90d` that window was about 2 ms of engine CPU, so one
/// late wake of the test thread on a loaded box lost the race. Story 2-63 Wave
/// E2's follow-up widens it through the engine's existing `bars_override` knob
/// (no production change): the run is [`CANCEL_RUN_BARS`] bars, whose
/// UNcancelled length is measured on the constant. The engine itself stops at
/// the next poll after the cancel (at most 128 bars on), so the passing case
/// costs little more than generating the bars (about 1.6 s on the dev box).
/// The residual: it is still a real-time race, now with about 0.5 s of margin
/// on the dev box instead of about 2 ms (not the 2 s first asked for; the
/// constant's doc says why), so a box that stalls this test's thread for
/// longer than that still turns it red. Run it alone if a loaded CI box flakes
/// it.
///
/// ## The run length is pinned (story 2-63 Wave E2, passes 3-4)
///
/// The margin exists only while the engine runs the override, so the first
/// progress event's `total_bars` must equal [`CANCEL_RUN_BARS`]. Without that
/// pin, an engine that ignored `bars_override` on the Synthetic v0.sma arm
/// ran this test on `Last90d`'s own bars, back at the ~2 ms window, and it
/// still passed in the same 1.70 s, because the test still generates the
/// 500 000 bars it no longer uses (MEASURED, falsifier pass 2, 2026-11-14, its
/// OWN-1: the test passed). The 2 160 is derived from source, not measured:
/// `Last90d` maps to `(2023, 2_160)` (90 × 24 hourly bars) in
/// `date_range_to_scenario_params` (`crates/backtest/src/engine.rs`), and with
/// no override `sma_composed_run::run` generates that many bars.
///
/// The pin's message names E2-P18's cause, the run length. Two probes turn it
/// red:
///
/// - T-T4 E2-P18, the override lost: left 2160, right 500000. MEASURED
///   (falsifier pass 3, 2026-11-15) under the message before pass 4's
///   rewording, and MEASURED (pass-4 fixer, 2026-11-15) under this message;
/// - E2-F3 (Wave E2's falsifier probe, `total_bars: bar_count + 1` in
///   `sma_composed_run::run`), a total one too high: left 500001, right
///   500000 (MEASURED, falsifier pass 3, 2026-11-15, under the message before
///   pass 4's rewording). Under E2-F3 the engine does run the override's
///   bars and only the reported total is one too high, which the left value
///   shows, so the message's run-length reading does not fit that cause
///   (REASONED from the E2-F3 edit). Its first red is
///   `smoke_yahoo_cache_hit`'s M5.
///
/// The total is pinned on the first event only. It is one value per run
/// (REASONED from source: the emit sends `total_bars: bar_count`), and every
/// event's total is pinned by `smoke_yahoo_cache_hit`'s M5, which caught
/// falsifier pass 3's OWN-B (a wrong total on every event after bar 32) at
/// bar 64 (MEASURED, falsifier pass 3, 2026-11-15).
/// A leftover-event copy of this pin, which the pass-3 fixer added, never saw
/// an event (MEASURED, falsifier pass 3, 2026-11-15: 0 events in its E2-P3
/// run; OWN-B left this test GREEN in 21 of 21 runs) and is gone since pass 4.
///
/// The final-bar half: no progress event left after the cancel may be the
/// final bar's. It checks only the events that arrive after the cancel. On a
/// quiet run there are none, and it then pins nothing: MEASURED (falsifier
/// pass 3, 2026-11-15), its E2-P3 run found this test's post-cancel drain
/// empty ("after 0 progress events"), and OWN-B survived 21 of 21 runs. When
/// one arrives, it fires: MEASURED (falsifier pass 3, 2026-11-15, its OWN-D,
/// an engine that sends the final bar's progress after it sees the cancel): "the cancel
/// landed before the final bar, so no leftover event is the final bar's (got
/// Progress { current_bar: 499999, total_bars: 500000, elapsed_ms: 0 })".
/// That was measured on the pass-3 loop, whose deleted total assert passed
/// under OWN-D because its event carried `total_bars: 500000`; on today's
/// loop, with the final-bar assert byte-identical, the same red is REASONED.
/// Its non-emptiness is deliberately not asserted: whether an event arrives
/// is timing, not this test's subject (bug-log #189). REASONED from
/// `sma_composed_run::run`'s bar loop: each poll checks the cancel BEFORE it
/// sends progress, and no cancel check follows the loop, so a run that returns
/// `Cancelled` stopped at a poll that sent nothing. A run whose final poll
/// misses the cancel sends the final-bar progress and returns `Ok`, which the
/// `Cancelled` assert (before the drain) already rejects. So on today's engine
/// the final-bar half cannot fail apart from that assert; it would fail only
/// if the engine sent progress after seeing the cancel, as OWN-D did.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn smoke_cancel_mid_run() {
    use backtest::engine::{RunError, ScenarioDataSource};

    let scenario_cfg = {
        let cfg = synthetic_cfg("v0.sma", "BTCUSDT", DateRange::Preset(Preset::Last90d));
        let mut sc = lab_config_to_scenario(&cfg).expect("lab_config_to_scenario");
        sc.data_source = ScenarioDataSource::default();
        // The run length: Synthetic bars the engine takes verbatim.
        sc.bars_override = Some(cancel_run_bars());
        sc
    };

    // Build real cancel + progress pairs.
    let (handle, cancel_rx) = cancellation_pair();
    let (progress_tx, progress_rx) = progress_pair();

    // Move the handle into an Arc so we can drop it from this task after
    // observing the first progress event.
    let handle_arc = Arc::new(std::sync::Mutex::new(Some(handle)));
    let handle_arc_clone = Arc::clone(&handle_arc);

    let mut progress_stream = ui::lab::progress::stream_impl(Some(progress_rx));

    // Spawn the engine.
    let engine_task = tokio::spawn(async move {
        backtest::engine::run_scenario(scenario_cfg, cancel_rx, progress_tx).await
    });

    // Wait for the first progress event, then drop the handle to signal cancel.
    let first_progress = timeout(Duration::from_secs(60), async {
        loop {
            match progress_stream.next().await {
                Some(Message::LabRunProgress(p)) if p.current_bar > 0 => return Some(p),
                Some(Message::LabRunProgress(_)) => continue, // current_bar == 0 (sentinel)
                Some(Message::LabRunProgressDone) | None => return None,
                Some(other) => panic!("unexpected: {other:?}"),
            }
        }
    })
    .await
    .expect("first progress event must arrive within 60s");

    let first_progress = first_progress.expect("must receive a progress event before cancellation");
    assert_eq!(
        first_progress.total_bars, CANCEL_RUN_BARS,
        "the engine ran exactly the override's CANCEL_RUN_BARS bars, so the cancel has its measured ~0.49 s window (a lost override gives Last90d's 2 160 bars and ~2 ms)"
    );

    // Drop the handle — mirrors cockpit_live::update dropping lab_state.run_cancel.
    {
        let mut guard = handle_arc_clone.lock().unwrap();
        *guard = None; // drop the handle
    }
    drop(handle_arc);

    // Engine should now return Cancelled.
    let engine_result = timeout(Duration::from_secs(60), engine_task)
        .await
        .expect("engine must finish within 60s after cancel signal")
        .expect("engine task must not panic");

    assert!(
        matches!(engine_result, Err(RunError::Cancelled)),
        "engine must return Cancelled after handle drop; got: {engine_result:?}"
    );

    // R7.4 on the cancel path: the returned engine dropped its ProgressSender,
    // so the relay drains whatever is left, sends Done, then ends. The check
    // below sees only what arrives after the cancel, which is often nothing
    // (the test doc says what it then pins).
    let leftover_progress = drain_progress_to_done(&mut progress_stream).await;
    for p in &leftover_progress {
        assert!(
            p.current_bar < CANCEL_RUN_BARS - 1,
            "the cancel landed before the final bar, so no leftover event is the final bar's (got {p:?})"
        );
    }

    // State update: cockpit should reflect cancellation.
    let mut cockpit = Cockpit::new();
    cockpit.lab_state.strategy = Some(StrategyId::new("v0.sma"));
    cockpit.lab_state.pair = Some((Venue::Binance, Symbol::new("BTCUSDT")));
    cockpit.lab_state.range = DateRange::Preset(Preset::Last90d);

    ui::state::update(&mut cockpit, Message::LabRunRequested);

    // The cancel error propagates as LabRunCompleted(Err(_)). This is the
    // shape spawn_lab_run's engine join sends for RunError::Cancelled
    // (bug-log #215). That is MEASURED by lab_runner_cancel_e2e test 6 (probe
    // T215-P5), not here: this test builds the string with the production
    // builder and checks only the state arm.
    let err_msg =
        ui::lab::runner::preload_notice::cancelled_message(&format!("{}", RunError::Cancelled));
    ui::state::update(&mut cockpit, Message::LabRunCompleted(Err(err_msg)));
    assert!(
        cockpit.lab_state.last_run_cancelled && cockpit.lab_state.last_run_error.is_none(),
        "a cancelled run must land as last_run_cancelled with no red error (bug-log #215); \
         got cancelled={} error={:?}",
        cockpit.lab_state.last_run_cancelled,
        cockpit.lab_state.last_run_error
    );

    // Post-cancel: inflight cleared, last_run_report NOT rotated (R2.3).
    assert!(
        !cockpit.lab_run_inflight,
        "lab_run_inflight must be false after LabRunCompleted(Err)"
    );
    assert!(
        cockpit.lab_state.last_run_report.is_none(),
        "last_run_report must NOT be rotated on cancellation (R2.3)"
    );
    assert!(
        cockpit.lab_state.run_progress.is_none(),
        "run_progress must be cleared after LabRunCompleted"
    );
}

// ── Test variants 4-6: Yahoo, on the committed sample corpus ─────────────────

/// Days in H1 2024, January through June (2024 is a leap year): the number of
/// daily bars in `[2024-01-01, 2024-07-01)`. Derived from the calendar, not
/// read off `load_cached` or the engine.
#[cfg(feature = "yahoo")]
const H1_2024_DAYS: usize = 31 + 29 + 31 + 30 + 31 + 30;

/// `data/yahoo-sample`, resolved from this crate rather than the process cwd
/// (bug-log #66: the cwd-relative `data/yahoo/` root made the Yahoo test here
/// skip on every CI run). The same root `data`'s
/// `reproducibility_sample_corpus.rs` uses; that file owns the corpus's
/// integrity (its manifest and the pinned aggregate SHA).
#[cfg(feature = "yahoo")]
fn yahoo_sample_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/yahoo-sample")
}

/// The corpus precondition: PANIC when the committed sample's `REVISION.toml`
/// is missing under `root`.
///
/// The sample is tracked in git (`.gitignore` un-ignores `data/yahoo-sample`),
/// so an absent corpus is a broken checkout, and the Yahoo test must go RED,
/// never return quietly. `sample_corpus_absent_is_a_failure_not_a_skip`
/// measures that, and T-T4 E2-P13 is the probe on this helper's body (E2-P14
/// is the one on its caller's).
#[cfg(feature = "yahoo")]
fn assert_sample_corpus(root: &std::path::Path) {
    assert!(
        root.join("REVISION.toml").is_file(),
        "the committed Yahoo sample corpus must ship at {} — absent means a broken checkout, not a skip (bug-log #66)",
        root.display()
    );
}

/// The body of variants 4, 5 and 6: v0.sma × BTCUSDT × H1_2024 on the Yahoo
/// corpus at `root`, through the engine's `bars_override` path.
///
/// It never skips: a missing corpus fails [`assert_sample_corpus`] (M1), and
/// any `load_cached` error (cache miss, manifest mismatch, coverage below
/// 95 %) fails the `.expect`. No network is touched. Where "fails, not skips"
/// is measured, stage by stage, up to and including the load:
/// `sample_corpus_absent_is_a_failure_not_a_skip` runs it at an empty root and
/// measures it up to M1 (T-T4 E2-P13 in the helper, E2-P14 in the body);
/// `sample_corpus_without_bars_is_a_failure_not_a_skip` runs it at a root
/// holding only the committed manifest and measures it through the load
/// (E2-P19). After the load it is UNMEASURED: a skip on an engine or report
/// error there would pass every test (REASONED), because no control can make
/// the engine fail on the committed corpus without a production change.
/// `smoke_yahoo_cache_hit` value-measures the asserts after the load at the
/// committed root (E2-P1, E2-P2): they run and fire on a working corpus.
///
/// ## A test-local composition of `preload_yahoo_bars` (K3 #6-style residual)
///
/// Production loads Lab Yahoo bars in the private `async fn preload_yahoo_bars(`
/// (`crates/ui/src/lab/runner.rs`), which an integration test cannot call.
/// This test RE-COMPOSES its steps from the pub pieces, in its order:
///
/// 1. the ticker map: `binance_to_yahoo_ticker(&sym)`;
/// 2. the window: `let (start_ms, end_ms) = range_to_ms_pair(scenario_range);`;
/// 3. the cadence: `let interval = Interval::derive_from_range(start_ms, end_ms);`;
/// 4. the root and the load: `let src = YahooBarSource::new(cache_root);`, then
///    `src.load_cached(..)`.
///
/// So a change to preload's COMPOSITION (a step dropped, reordered or given
/// other arguments) stays invisible here. Two differences are deliberate: the
/// root is the committed `data/yahoo-sample` (production's is the cwd-relative
/// `let cache_root = std::path::PathBuf::from("data/yahoo");`, already routed
/// in backlog.md § Routed findings, "The production Yahoo cache root is
/// cwd-relative."), and there is no online-fetch fallback (an error fails).
///
/// ## Assertions (every expected value DERIVED from the calendar)
///
/// - M1: the corpus is present (`assert_sample_corpus`).
/// - M2: the window is `[2024-01-01, 2024-07-01)` UTC, built from `time::Date`.
/// - The cadence is `Days1` and the ticker is `BTC-USD` (the corpus directory).
/// - M3: `loaded.expected_count == H1_2024_DAYS` (182).
/// - M4: `loaded.bars.len() == H1_2024_DAYS`: the sample covers every day.
/// - Progress, drained strictly (M7): non-empty; M5 every `total_bars` is 182;
///   `current_bar` strictly increases; M6 the last `current_bar` is 181, the
///   final bar (Bug #64's final-bar emit). The 32/128-bar emit cadence in
///   between is NOT pinned.
/// - The report has a non-empty equity series; the state post-conditions hold
///   through `update` and the `apply_wrapper` copy. Its bar count is not
///   asserted: the engine echoes its input into `report.bars`, so that count
///   holds by construction once M4 does. M5 and M6 carry "the engine ran over
///   every bar" (E2-P2, and Wave E2's falsifier probe E2-F3).
///
/// Why M6 is deterministic (REASONED from the engine's cadence): 182 bars give
/// 6 emits (bars 0, 32, 64, 96, 128 and 181), at most the capacity of 8 of
/// `progress_pair`'s lossy channel (a `try_send` that drops when full), so no
/// emit can be dropped however late the drain runs. A denser cadence or a
/// smaller capacity would let the final emit drop under load and make M6
/// load-flaky. The emit count is deliberately not asserted: the cadence is
/// not this test's subject (bug-log #189).
#[cfg(feature = "yahoo")]
async fn run_yahoo_smoke(root: std::path::PathBuf) {
    use backtest::engine::ScenarioDataSource;
    use data::yahoo::{Interval, YahooBarSource, binance_to_yahoo_ticker};

    // M1 — the corpus at the root the caller passed (never the cwd).
    assert_sample_corpus(&root);

    let cfg = synthetic_cfg("v0.sma", "BTCUSDT", DateRange::Preset(Preset::H1_2024));
    let mut scenario_cfg = lab_config_to_scenario(&cfg).expect("lab_config_to_scenario");

    // Step 2 of preload_yahoo_bars — the window, from the production fn.
    let (start_ms, end_ms) = ui::lab::runner::range_to_ms_pair(&scenario_cfg.range);
    let utc_midnight_ms = |month: ::time::Month| {
        ::time::Date::from_calendar_date(2024, month, 1)
            .expect("a valid 2024 calendar date")
            .midnight()
            .assume_utc()
            .unix_timestamp()
            * 1_000
    };
    assert_eq!(
        (start_ms, end_ms),
        (
            utc_midnight_ms(::time::Month::January),
            utc_midnight_ms(::time::Month::July)
        ),
        "M2: the H1_2024 window is [2024-01-01, 2024-07-01) UTC"
    );

    // Step 3 — the cadence.
    let interval = Interval::derive_from_range(start_ms, end_ms);
    assert_eq!(
        interval,
        Interval::Days1,
        "a 182-day window derives the daily cadence"
    );

    // Step 1 — the ticker map (the corpus holds BTC-USD only).
    let ticker = binance_to_yahoo_ticker(&Symbol::new(cfg.symbol.as_str()))
        .expect("BTCUSDT maps to a Yahoo ticker");
    assert_eq!(
        ticker.as_str(),
        "BTC-USD",
        "BTCUSDT maps to the corpus ticker"
    );

    // Step 4 — the load. An error FAILS; it never skips.
    let loaded = YahooBarSource::new(root)
        .load_cached(ticker.as_str(), interval, start_ms, end_ms)
        .expect("the committed sample corpus must load BTC-USD 1d over H1 2024");
    assert_eq!(
        loaded.expected_count, H1_2024_DAYS,
        "M3: the expected bar count is the number of days in H1 2024"
    );
    assert_eq!(
        loaded.bars.len(),
        H1_2024_DAYS,
        "M4: the committed sample covers every day of H1 2024"
    );

    // Wire the bars into the scenario via bars_override.
    scenario_cfg.data_source = ScenarioDataSource::YahooCache;
    scenario_cfg.bars_override = Some(loaded.bars);

    let (_handle, cancel_rx) = cancellation_pair();
    let (progress_tx, progress_rx) = progress_pair();

    let mut progress_stream = ui::lab::progress::stream_impl(Some(progress_rx));

    let engine_task = tokio::spawn(async move {
        backtest::engine::run_scenario(scenario_cfg, cancel_rx, progress_tx).await
    });

    let progress_events = drain_progress_to_done(&mut progress_stream).await;

    let engine_result = engine_task.await.expect("engine task must not panic");
    let report = engine_result.expect("Yahoo H1_2024 run must succeed");

    assert!(
        !progress_events.is_empty(),
        "Yahoo cache hit must emit at least one LabRunProgress event; \
         got 0. This is the Bug #63 regression: progress was silently \
         dropped on the Yahoo path."
    );
    for p in &progress_events {
        assert_eq!(
            p.total_bars, H1_2024_DAYS,
            "M5: every progress event reports the full H1 2024 bar count (got {p:?})"
        );
    }
    for window in progress_events.windows(2) {
        assert!(
            window[1].current_bar > window[0].current_bar,
            "progress current_bar must strictly increase: {} then {}",
            window[0].current_bar,
            window[1].current_bar
        );
    }
    assert_eq!(
        progress_events.last().map(|p| p.current_bar),
        Some(H1_2024_DAYS - 1),
        "M6: the last progress event is the final bar (Bug #64 final-bar emit)"
    );

    assert!(
        !report.equity_series.is_empty(),
        "Yahoo H1_2024 equity series must be non-empty"
    );

    // State post-conditions.
    let mut cockpit = Cockpit::new();
    cockpit.lab_state.strategy = Some(StrategyId::new("v0.sma"));
    cockpit.lab_state.pair = Some((Venue::Binance, Symbol::new("BTCUSDT")));
    cockpit.lab_state.range = DateRange::Preset(Preset::H1_2024);

    let equity: Vec<(i64, rust_decimal::Decimal)> = report
        .equity_series
        .iter()
        .map(|(ts, money)| (ts.unix_millis(), money.amount()))
        .collect();
    let summary = RunSummary {
        strategy_id: smol_str::SmolStr::new("v0.sma"),
        symbol: smol_str::SmolStr::new("BTCUSDT"),
        report_path: report.report_path.clone(),
        equity_series: equity,
        fills: report.fills.clone(),
        kpis: report.kpis.clone(),
        bars: report.bars.clone(),
        position_curve: Vec::new(),
    };
    ui::state::update(&mut cockpit, Message::LabRunRequested);
    ui::state::update(&mut cockpit, Message::LabRunCompleted(Ok(summary.clone())));
    apply_wrapper(&mut cockpit, &summary);

    assert!(
        cockpit.lab_state.last_run_report.is_some(),
        "last_run_report must be Some after Yahoo run wrapper rotation"
    );
    assert!(
        cockpit.lab_state.run_progress.is_none(),
        "run_progress must be None after Yahoo LabRunCompleted"
    );
}

/// Variant 4 — Yahoo cache hit: [`run_yahoo_smoke`] on the COMMITTED sample
/// corpus, resolved from `CARGO_MANIFEST_DIR` by [`yahoo_sample_root`].
#[cfg(feature = "yahoo")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn smoke_yahoo_cache_hit() {
    run_yahoo_smoke(yahoo_sample_root()).await;
}

/// Variant 5 — the Yahoo body FAILS without the corpus; it never skips
/// (bug-log #66).
///
/// The property #66 is about: with the corpus absent, the Yahoo test must go
/// RED, not return quietly. So the body, [`run_yahoo_smoke`], run on an empty
/// temp dir must panic with the helper's "broken checkout" message. The empty
/// root stops the body at M1, its helper call, so this test measures the body
/// up to there and no further: a skip written after that call, on a load
/// error, left it green (MEASURED, falsifier pass 2, its OWN-2). That later
/// stage is variant 6's, `sample_corpus_without_bars_is_a_failure_not_a_skip`
/// (E2-P19). Before story 2-63 Wave E2 this was unmeasurable, because the skip
/// WAS the code; until its follow-up pass this control called only the helper,
/// so a skip written into the body went unseen.
///
/// The pair: `smoke_yahoo_cache_hit` runs the same body at the committed root
/// and passes. T-T4 E2-P14 (the #66 skip shape in the body) turns this test
/// red with "test did not panic as expected"; E2-P13 (the shape in the helper)
/// turns it red with "panic did not contain expected string", because the body
/// then panics at its `load_cached` `.expect` instead. Under both,
/// `smoke_yahoo_cache_hit` stays green because the corpus is present. Both
/// reds are MEASURED (falsifier pass 2, 2026-11-14): E2-P14's as "test did not
/// panic as expected", E2-P13's as "panic did not contain expected string",
/// the panic it saw being the load's `.expect` text with `RevisionMissing`
/// against the expected substring "absent means a broken checkout".
#[cfg(feature = "yahoo")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[should_panic(expected = "absent means a broken checkout")]
async fn sample_corpus_absent_is_a_failure_not_a_skip() {
    let empty = tempfile::tempdir().expect("a temp dir to stand in for a broken checkout");
    run_yahoo_smoke(empty.path().to_path_buf()).await;
}

/// Every `.parquet` file under `dir`, at any depth.
#[cfg(feature = "yahoo")]
fn parquet_files_under(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(next) = pending.pop() {
        let entries = std::fs::read_dir(&next)
            .unwrap_or_else(|e| panic!("list the fixture dir {}: {e}", next.display()));
        for entry in entries {
            let path = entry.expect("read a fixture dir entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|ext| ext == "parquet") {
                found.push(path);
            }
        }
    }
    found
}

/// Variant 6 — the Yahoo body FAILS when the corpus manifest ships but its
/// bars do not; it never skips (bug-log #66), at the load step.
///
/// The second #66 control (story 2-63 Wave E2, pass 3). Variant 5's empty root
/// stops the body at M1, its helper call, so it measures the body only up to
/// there. Here the root holds ONLY a byte copy of the committed
/// `data/yahoo-sample/REVISION.toml`: M1 passes (the manifest is a file), and
/// `load_cached` returns `CacheMiss` for the first month the window needs
/// (`BTC-USD/1d/2024/01.parquet`). The body must then panic at its load
/// `.expect`, "the committed sample corpus must load BTC-USD 1d over H1 2024".
/// So this test measures the body through the load. The test first checks
/// that its fixture is the one described: the copied manifest exists and no
/// `.parquet` file does.
///
/// The pair, on the load-error dimension: `smoke_yahoo_cache_hit` loads the
/// committed corpus and passes. T-T4 E2-P19 (the #66 skip shape on the load
/// error, `let Ok(loaded) = .. else { return; }`) turns this test red with
/// "test did not panic as expected", and leaves variant 5 and
/// `smoke_yahoo_cache_hit` GREEN (MEASURED, falsifier pass 3, 2026-11-15; pass
/// 2 MEASURED that both stay green under that edit, as its OWN-2, which no test
/// caught then). E2-P13 and E2-P14 leave this test GREEN (its manifest is
/// present, so neither skip shape fires), and so does E2-P3 (its body panics
/// at the load, before any progress stream exists): all three MEASURED
/// (falsifier pass 3, 2026-11-15).
///
/// The fixture guard is live: falsifier pass 3's OWN-A copied one month of
/// the corpus's bars into the fixture, nested at `BTC-USD/1d/2024/01.parquet`
/// (REASONED: the load would still have failed, on February), and the guard
/// turned this test red with "fixture: the root must hold no .parquet file
/// (found [..])" (MEASURED, falsifier pass 3, 2026-11-15), so
/// `parquet_files_under` finds a nested file.
#[cfg(feature = "yahoo")]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[should_panic(expected = "the committed sample corpus must load BTC-USD 1d over H1 2024")]
async fn sample_corpus_without_bars_is_a_failure_not_a_skip() {
    let manifest_only = tempfile::tempdir().expect("a temp dir to hold only the manifest");
    let manifest = manifest_only.path().join("REVISION.toml");
    std::fs::copy(yahoo_sample_root().join("REVISION.toml"), &manifest)
        .expect("copy the committed data/yahoo-sample/REVISION.toml into the fixture");
    assert!(
        manifest.is_file(),
        "fixture: the copied REVISION.toml must exist at {}",
        manifest.display()
    );
    let parquet = parquet_files_under(manifest_only.path());
    assert!(
        parquet.is_empty(),
        "fixture: the root must hold no .parquet file (found {parquet:?})"
    );
    run_yahoo_smoke(manifest_only.path().to_path_buf()).await;
}

// ── Test variant 7: Empty selection — the lifecycle flags ────────────────────

/// Variant 7 — empty selection: cold-start `strategy = None`, `pair = None`.
///
/// What it pins, through the production `ui::state::update`: `LabRunRequested`
/// sets `lab_run_inflight` even with nothing selected (a pure flag), and
/// `LabRunCompleted` clears it. With the selection empty, the `apply_wrapper`
/// COPY builds no tuple, so `last_run_report` stays `None`.
///
/// What it does NOT pin: the binary's gate. In `AppState::update`
/// (`crates/ui/src/bin/cockpit_live.rs`) the run config is built only from a
/// `(strategy, pair)` selection (`let lab_run_cfg = if lab_run_requested {`),
/// and the engine is spawned only under
/// `} else if let Some(run_cfg) = lab_run_cfg {`.
/// That is a bin target, so no engine call is observed or
/// excluded here. The old name, `smoke_empty_selection_no_engine_call`,
/// claimed otherwise, and a local `lab_run_cfg_is_none` boolean, re-stating
/// the two preconditions below, stood in for the gate. Wave E2 removed both.
#[test]
fn smoke_empty_selection_lifecycle_flags() {
    let mut cockpit = Cockpit::new();
    // pair is None — cold-start default.
    assert!(
        cockpit.lab_state.pair.is_none(),
        "cold-start cockpit must have pair=None"
    );
    assert!(
        cockpit.lab_state.strategy.is_none(),
        "cold-start cockpit must have strategy=None"
    );

    // state::update sets the inflight flag regardless of the selection; only
    // the binary-side gate (not linked here) decides whether the engine runs.
    ui::state::update(&mut cockpit, Message::LabRunRequested);
    assert!(
        cockpit.lab_run_inflight,
        "state::update sets lab_run_inflight regardless (binary gate prevents engine call)"
    );

    // No LabRunCompleted has been dispatched, so nothing has rotated yet.
    assert!(
        cockpit.lab_state.last_run_report.is_none(),
        "last_run_report must remain None when gate prevents engine call"
    );

    // Dispatch LabRunCompleted(Ok(_)) by hand: `last_run_report` stays None
    // because the apply_wrapper copy builds no tuple when strategy+pair are unset.
    let dummy_summary = RunSummary {
        strategy_id: smol_str::SmolStr::new(""),
        symbol: smol_str::SmolStr::new(""),
        report_path: None,
        equity_series: vec![],
        fills: vec![],
        kpis: backtest::BacktestKpis::default(),
        bars: Arc::new(vec![]),
        position_curve: vec![],
    };
    ui::state::update(
        &mut cockpit,
        Message::LabRunCompleted(Ok(dummy_summary.clone())),
    );
    apply_wrapper(&mut cockpit, &dummy_summary);
    // apply_wrapper's pre_tuple is None because strategy+pair are unset → no rotation.
    assert!(
        cockpit.lab_state.last_run_report.is_none(),
        "last_run_report must remain None when selection was empty"
    );
    assert!(
        !cockpit.lab_run_inflight,
        "lab_run_inflight must be false after LabRunCompleted"
    );
}
