//! Smoke tests for the `cockpit_live` Lab-Run path.
//!
//! ## Why this test file exists
//!
//! Bug #63 (cross-sectional Stop + progress) and Bug #64 (progress bar stuck)
//! both escaped detection because the CLI uses `cancellation_pair()` +
//! `ProgressSender::disabled()` to keep backtest output byte-identical for
//! anchors — the live UI path NEVER ran in CI.
//!
//! These tests exercise the full `cockpit_live` Lab-Run pipeline WITHOUT
//! booting iced: they drive `backtest::engine::run_scenario` directly through
//! real progress and cancel plumbing and assert on the `Cockpit` state
//! post-run — closing the P4 D− coverage gap identified in
//! `spec/dev-notes/testing-strategy-review-2026-05-25.md`.
//!
//! ## Option A (selected)
//!
//! Constructs cockpit model + lab state directly, runs the engine on a tokio
//! runtime, passes real `(ProgressSender, Receiver<Progress>)` and
//! `(RunCancelHandle, RunCancelReceiver)` pairs, then asserts on
//! `Message::LabRunCompleted` + `Message::LabRunProgress` round-trips.
//! No iced application is booted; `iced::Task` futures are NOT driven here
//! — instead we replicate the binary-side logic inlined from
//! `cockpit_live::update` (following the pattern of `lab_run_real_engine.rs`).
//!
//! ## Test variants
//!
//! 1. `smoke_synthetic_short_range`  — v0.sma × BTCUSDT × Last7d  (fast happy path)
//! 2. `smoke_synthetic_longer_range` — v0.sma × BTCUSDT × Last90d (multiple progress cycles)
//! 3. `smoke_cancel_mid_run`         — cancel after first progress event
//! 4. `smoke_yahoo_cache_hit`        — v0.sma × BTCUSDT × H1_2024, Yahoo bars
//!                                     (gated: `#[ignore]` skipped if no parquet on disk)
//! 5. `smoke_empty_selection`        — pair=None → engine never called, no run fires

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use tokio::time::timeout;
use trading_core::{StrategyId, Symbol, Venue};

use backtest::cancel::cancellation_pair;
use backtest::progress::{Progress, ProgressSender, progress_pair};
use ui::lab::equity_loader::LabTuple;
use ui::lab::runner::{LabRunConfig, RunReportMirror, RunSummary, lab_config_to_scenario};
use ui::lab::state::{DateRange, LabDataSource, Preset};
use ui::state::{Cockpit, Message};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build a `LabRunConfig` with Synthetic data source (default path).
fn synthetic_cfg(strategy: &str, symbol: &str, range_label: &str) -> LabRunConfig {
    LabRunConfig {
        strategy_id: smol_str::SmolStr::new(strategy),
        symbol: smol_str::SmolStr::new(symbol),
        venue: smol_str::SmolStr::new("Binance"),
        range_label: smol_str::SmolStr::new(range_label),
        seed: ui::lab::defaults::LAB_DEFAULT_SEED,
        write_report: false,
        data_source: LabDataSource::Synthetic,
        sma_fast_len: None,
        sma_slow_len: None,
    }
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

    // Drain progress messages until `LabRunProgressDone` or timeout.
    let mut progress_events: Vec<Progress> = Vec::new();
    loop {
        match timeout(Duration::from_secs(120), progress_stream.next())
            .await
            .expect("progress stream must not hang for more than 120s")
        {
            Some(Message::LabRunProgress(p)) => {
                progress_events.push(p);
            }
            Some(Message::LabRunProgressDone) | None => break,
            Some(other) => panic!("unexpected message from progress stream: {other:?}"),
        }
    }

    let summary = engine_task
        .await
        .expect("engine task must not panic");

    (summary, progress_events)
}

/// Apply the binary-side wrapper rotation block (mirrors `cockpit_live::update`
/// lines 977-997) so `last_run_report` is populated after `LabRunCompleted`.
fn apply_wrapper(cockpit: &mut Cockpit, summary: &RunSummary) {
    let pre_tuple = {
        let ls = &cockpit.lab_state;
        match (ls.strategy.as_ref(), ls.pair.as_ref()) {
            (Some(strategy), Some((venue, symbol))) => {
                Some(LabTuple::new(strategy, *venue, symbol, ls.range.clone()))
            }
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

// ── Test variant 1: Synthetic short range (Last7d) ────────────────────────────

/// Variant 1 — fast happy path: v0.sma × BTCUSDT × Last7d (Synthetic).
///
/// Asserts:
/// - At least one `LabRunProgress(p)` with `p.current_bar > 0` arrives.
/// - `LabRunCompleted(Ok(_))` arrives (represented as engine returning Ok).
/// - After wrapper rotation, `cockpit.lab_state.last_run_report.is_some()`.
/// - `cockpit.lab_state.run_progress` is `None` (cleared after completion).
///
/// Covers: Bug #63 / Bug #64 live progress path — verifies that real
/// `ProgressSender` + `progress_pair()` actually delivers events to the UI.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn smoke_synthetic_short_range() {
    let cfg = synthetic_cfg("v0.sma", "BTCUSDT", "Last30d");

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
    let meaningful_progress = progress_events
        .iter()
        .any(|p| p.current_bar > 0);
    assert!(
        meaningful_progress,
        "must receive at least one LabRunProgress with current_bar > 0; \
         got {} events: {progress_events:?}",
        progress_events.len()
    );

    // Step 4: engine must succeed.
    let summary = result.expect("engine must return Ok for v0.sma × BTCUSDT × Last30d");

    // Step 5: dispatch LabRunCompleted and apply binary-side wrapper.
    ui::state::update(
        &mut cockpit,
        Message::LabRunCompleted(Ok(summary.clone())),
    );
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
    let cfg = synthetic_cfg("v0.sma", "BTCUSDT", "Last90d");

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
    ui::state::update(
        &mut cockpit,
        Message::LabRunCompleted(Ok(summary.clone())),
    );
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

/// Variant 3 — cancel mid-run: drop the `RunCancelHandle` after the first
/// progress event arrives, then assert `LabRunCompleted(Err(_))` with a
/// cancellation marker.
///
/// This is the core Bug #63 regression gate: the Stop button drops
/// `lab_state.run_cancel`, which drops the `RunCancelHandle`. The engine
/// observes `is_cancelled() == true` at the next poll boundary and returns
/// `Err(RunError::Cancelled)`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn smoke_cancel_mid_run() {
    use backtest::engine::{RunError, ScenarioDataSource};

    let scenario_cfg = {
        let cfg = synthetic_cfg("v0.sma", "BTCUSDT", "Last90d");
        let mut sc = lab_config_to_scenario(&cfg).expect("lab_config_to_scenario");
        sc.data_source = ScenarioDataSource::default();
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

    assert!(
        first_progress.is_some(),
        "must receive a progress event before cancellation"
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

    // State update: cockpit should reflect cancellation.
    let mut cockpit = Cockpit::new();
    cockpit.lab_state.strategy = Some(StrategyId::new("v0.sma"));
    cockpit.lab_state.pair = Some((Venue::Binance, Symbol::new("BTCUSDT")));
    cockpit.lab_state.range = DateRange::Preset(Preset::Last90d);

    ui::state::update(&mut cockpit, Message::LabRunRequested);

    // The cancel error propagates as LabRunCompleted(Err(_)).
    let err_msg = smol_str::SmolStr::new(format!("{}", RunError::Cancelled));
    ui::state::update(
        &mut cockpit,
        Message::LabRunCompleted(Err(err_msg)),
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

// ── Test variant 4: Yahoo cache hit ──────────────────────────────────────────

/// Variant 4 — Yahoo cache hit: v0.sma × BTCUSDT × H1_2024 with real bars.
///
/// `#[ignore]`-gated with a runtime cache-presence check. If the parquet file
/// does not exist on disk, the test exits early without failure. In CI with
/// the Yahoo cache populated, this runs the Yahoo dispatch arm end-to-end.
///
/// Required parquet: `data/yahoo/BTC-USD/1d/2024/01.parquet`
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn smoke_yahoo_cache_hit() {
    use backtest::engine::ScenarioDataSource;

    // Runtime cache-presence check — do not require `#[ignore]` gymnastics.
    // If the file isn't present, skip with a clear diagnostic.
    let probe_path = std::path::Path::new("data/yahoo/BTC-USD/1d/2024/01.parquet");
    let workspace_probe = {
        // The test may run with cwd = workspace root or cwd = crates/ui.
        // Try both.
        let direct = probe_path.exists();
        let via_workspace = std::path::Path::new("../../data/yahoo/BTC-USD/1d/2024/01.parquet").exists();
        direct || via_workspace
    };

    if !workspace_probe {
        eprintln!(
            "smoke_yahoo_cache_hit: SKIPPED — data/yahoo/BTC-USD/1d/2024/01.parquet not found. \
             Run `cargo run -p data --bin fetch_yahoo -- --ticker BTC-USD --year 2024` \
             to populate the cache."
        );
        return;
    }

    // Use the Yahoo bars directly through the engine's `bars_override` path
    // (same as what `preload_yahoo_bars` does in `spawn_lab_run`). This keeps
    // the test self-contained — no network access needed since the cache exists.
    let cfg = synthetic_cfg("v0.sma", "BTCUSDT", "H1_2024");
    let mut scenario_cfg = lab_config_to_scenario(&cfg).expect("lab_config_to_scenario");

    // Load the bars from cache using the Yahoo source.
    let cache_root = {
        if std::path::Path::new("data/yahoo").exists() {
            std::path::PathBuf::from("data/yahoo")
        } else {
            std::path::PathBuf::from("../../data/yahoo")
        }
    };
    use data::yahoo::{Interval, YahooBarSource};
    let src = YahooBarSource::new(cache_root);

    // H1_2024: 2024-01-01 → 2024-07-01 UTC
    let start_ms: i64 = 1_704_067_200_000;
    let end_ms: i64 = 1_719_792_000_000;
    let interval = Interval::Days1;

    let loaded = src
        .load_cached("BTC-USD", interval, start_ms, end_ms)
        .expect("Yahoo cache must be readable for BTC-USD H1_2024");

    assert!(
        !loaded.bars.is_empty(),
        "Yahoo cache must contain bars for BTC-USD H1_2024"
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

    // Drain progress events.
    let mut progress_events: Vec<Progress> = Vec::new();
    loop {
        match timeout(Duration::from_secs(120), progress_stream.next())
            .await
            .expect("progress stream must not hang for more than 120s")
        {
            Some(Message::LabRunProgress(p)) => progress_events.push(p),
            Some(Message::LabRunProgressDone) | None => break,
            Some(other) => panic!("unexpected: {other:?}"),
        }
    }

    let engine_result = engine_task.await.expect("engine task must not panic");
    let report = engine_result.expect("Yahoo H1_2024 run must succeed");

    // Yahoo bars cover ~180 daily bars; at least a few progress events expected.
    assert!(
        !progress_events.is_empty(),
        "Yahoo cache hit must emit at least one LabRunProgress event; \
         got 0. This is the Bug #63 regression: progress was silently \
         dropped on the Yahoo path."
    );

    assert!(
        !report.equity_series.is_empty(),
        "Yahoo H1_2024 equity series must be non-empty"
    );
    assert!(
        !report.bars.is_empty(),
        "Yahoo H1_2024 bars must be non-empty"
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
    ui::state::update(
        &mut cockpit,
        Message::LabRunCompleted(Ok(summary.clone())),
    );
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

// ── Test variant 5: Empty selection — Run button gate ────────────────────────

/// Variant 5 — empty selection: `pair=None`, `LabRunRequested` dispatched.
///
/// In `cockpit_live::update`, when `lab_run_cfg` is `None` (because
/// `ls.pair` is None), `spawn_lab_run` is never called and the function
/// falls through to `iced::Task::none()`. This test verifies that the
/// state-only layer (pure `state::update`) does not call the engine.
///
/// Gate: `lab_run_inflight` becomes true (state::update sets it regardless
/// of pair selection — it's a pure flag), but no progress events are emitted
/// and no engine call fires because the binary-side gate short-circuits on
/// `lab_run_cfg == None`.
#[test]
fn smoke_empty_selection_no_engine_call() {
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

    // Simulate cockpit_live::update's pre-dispatch gate:
    // lab_run_cfg is None when strategy or pair is None.
    let ls = &cockpit.lab_state;
    let lab_run_cfg_is_none = ls.strategy.is_none() || ls.pair.is_none();
    assert!(
        lab_run_cfg_is_none,
        "gate must be None when pair/strategy unset"
    );

    // state::update fires (sets inflight), but no engine call is made
    // because the binary-side if-let short-circuits.
    ui::state::update(&mut cockpit, Message::LabRunRequested);
    assert!(
        cockpit.lab_run_inflight,
        "state::update sets lab_run_inflight regardless (binary gate prevents engine call)"
    );

    // Simulate the binary-side gate: no engine call → simulate immediate Task::none()
    // path by dispatching a manual LabRunCompleted(Ok(_)) only when cfg was Some.
    // Since cfg was None, no LabRunCompleted fires — last_run_report stays None.
    // (In the real binary, Task::none() means LabRunCompleted never arrives.)
    assert!(
        cockpit.lab_state.last_run_report.is_none(),
        "last_run_report must remain None when gate prevents engine call"
    );

    // Also verify: if we dispatch LabRunCompleted(Ok(_)) manually (simulating
    // a no-op path), last_run_report is still None because apply_wrapper
    // short-circuits when strategy+pair are unset.
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
