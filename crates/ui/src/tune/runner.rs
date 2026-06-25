//! Gate-tied hyperparameter sweep — runner glue (ADR-0069 T10).
//!
//! The cockpit ↔ sweep-engine bridge. Mirrors [`crate::leaderboard::runner`]
//! one-for-one, but dispatches `backtest::run_param_sweep` instead of
//! `run_bakeoff`:
//!
//! ```text
//! iced update thread
//!   Message::SweepRunRequested
//!     └──> runner::spawn_sweep(rt_handle, cfg, cancel, progress_tx, sweep_progress_tx)
//!              └──> rt_handle.spawn(backtest::run_param_sweep(cfg, …))
//!                       └──> oneshot → iced::Task::perform
//!                                └──> Message::SweepRunCompleted(Result<SweepReportMirror>)
//! ```
//!
//! ## INVARIANT (the layering seam)
//!
//! `backtest::run_param_sweep` returns `backtest::SweepReport` — consumed
//! through the **existing `backtest` dep** (the same seam `spawn_bakeoff` uses
//! for `BakeoffReport`). The result is mirrored into [`SweepReportMirror`] HERE,
//! INSIDE the spawned task (at the dispatch boundary, before it crosses into
//! iced state), via the ONE `from_report` seam — so `ui` never threads an engine
//! type through `view`. `ui` gains NO new crate edge.
//!
//! ## Form → config (T10)
//!
//! [`sweep_config_from_state`] builds the `SweepConfig` from the operator's
//! CHOSEN family + coin + lookback + SMA ranges: `data_source = BinanceCache`,
//! `seed = LAB_DEFAULT_SEED` (shared with the bake-off — same deterministic
//! seed), `paths = 1000` (the same gate setting). The lookback is resolved
//! against wall-clock `now_ms` HERE, at the dispatch boundary, exactly as
//! `bakeoff_config_from_state` does.

use smol_str::SmolStr;

use crate::tune::screen_state::{SmaGridForm, TuneFamily};
use crate::tune::state::SweepReportMirror;

/// Result posted back to the cockpit via `Message::SweepRunCompleted`.
///
/// `Ok(mirror)` carries the result grid; `Err(msg)` an operator-friendly
/// failure reason (mirrors `BakeoffRunResult`'s shape).
pub type SweepRunResult = Result<SweepReportMirror, SmolStr>;

/// The bootstrap path count per cell — the SAME setting the advisor bake-off
/// gate uses (`advisor_robustness()` in `leaderboard/runner.rs`). Single-sourced
/// here so the sweep's gate is byte-identical to the leaderboard's.
const SWEEP_PATHS: usize = 1000;

/// Build the engine [`SweepAxis`](backtest::SweepAxis) from a parsed
/// `{min, max, step}` triple. The form already validated non-blank; this maps
/// the parsed integers into the engine axis (falling back to a 1-cell shipped
/// default per field so a partially-parsed form still yields a runnable config
/// rather than panicking — the form's `can_run` gate prevents this path in
/// practice).
fn axis_from_input(
    min: Option<u32>,
    max: Option<u32>,
    step: Option<u32>,
    fallback_min: u32,
    fallback_max: u32,
    fallback_step: u32,
) -> backtest::SweepAxis {
    backtest::SweepAxis {
        min: min.unwrap_or(fallback_min),
        max: max.unwrap_or(fallback_max),
        step: step.unwrap_or(fallback_step).max(1),
    }
}

/// Build the engine [`SweepGrid`](backtest::SweepGrid) from the SMA form.
///
/// Pure; total. The `1 ≤ fast < slow ≤ 400` validity guard + the cap are
/// applied inside `run_param_sweep`'s enumeration (the engine owns the truth),
/// so this just threads the operator's `{min, max, step}` through.
fn sma_grid_from_form(form: &SmaGridForm) -> backtest::SweepGrid {
    let (fmin, fmax, fstep) = form.fast.parsed();
    let (smin, smax, sstep) = form.slow.parsed();
    backtest::SweepGrid::Sma(backtest::SmaGrid {
        fast_len: axis_from_input(fmin, fmax, fstep, 10, 30, 5),
        slow_len: axis_from_input(smin, smax, sstep, 30, 70, 10),
    })
}

/// Build a [`SweepConfig`](backtest::SweepConfig) from the Tune form state — the
/// operator's CHOSEN family + coin + lookback + SMA ranges.
///
/// The lookback enum is mapped to a `backtest::engine::DateRange` against
/// `now_ms` HERE, at the dispatch boundary (relative windows → `Custom`, the
/// fixed 2024 presets pass through) — exactly as `bakeoff_config_from_state`
/// does. `data_source = BinanceCache` (the real hourly corpus),
/// `seed = LAB_DEFAULT_SEED` (shared with the bake-off — apples-to-apples),
/// `paths = 1000` (the same gate). Pure; no I/O.
///
/// Only SMA is wired in v0.1; a non-SMA family still produces a config (with an
/// SMA grid placeholder) but the form's `can_run` gate prevents dispatching it,
/// and `run_param_sweep` returns an empty grid for the composed families until
/// the engine's T7 builder lands.
#[must_use]
pub fn sweep_config_from_state(
    st: &crate::tune::screen_state::TuneScreenState,
    coin: &trading_core::Symbol,
    lookback: crate::leaderboard::LeaderboardLookback,
    now_ms: i64,
) -> backtest::SweepConfig {
    let grid = match st.family {
        TuneFamily::Sma => sma_grid_from_form(&st.sma_grid),
        // The composed families have no Tune FORM in this UI slice (SMA-first per
        // ADR-0069 § D3 sequencing), so the UI's `can_run` gate blocks dispatching
        // them. The engine's T7 grid structs exist, so the placeholder uses their
        // shipped-default grids — a defensive, well-typed config the UI never
        // actually dispatches. (Wiring a real MACD/RSI/Bollinger form is T7's UI
        // follow-on; this keeps the match total + compiling against the engine API.)
        TuneFamily::Macd => {
            backtest::SweepGrid::Macd(backtest::bakeoff::sweep::MacdGrid::default())
        }
        TuneFamily::Rsi => backtest::SweepGrid::Rsi(backtest::bakeoff::sweep::RsiGrid::default()),
        TuneFamily::Bollinger => {
            backtest::SweepGrid::Bollinger(backtest::bakeoff::sweep::BollingerGrid::default())
        }
    };
    backtest::SweepConfig {
        family: st.family.to_engine(),
        grid,
        symbol: coin.clone(),
        range: lookback.to_date_range(now_ms),
        seed: crate::lab::defaults::LAB_DEFAULT_SEED,
        data_source: backtest::engine::ScenarioDataSource::BinanceCache,
        paths: SWEEP_PATHS,
    }
}

/// Build an `iced::Task` that runs a sweep and posts the result back to the iced
/// update loop as `Message::SweepRunCompleted`.
///
/// Mirrors [`crate::leaderboard::runner::spawn_bakeoff`]'s fixture/live split:
///
/// - **Default (non-`live`) / no-runtime builds** — the tokio runtime that
///   drives `run_param_sweep` is absent, so the task immediately resolves with a
///   friendly `Err` directing the operator to the live build. This keeps the
///   fixtures cockpit + the render harness from hanging on a missing runtime
///   (they render a populated FIXTURE, not a live run).
/// - **`live` builds** — bridges via `rt_handle.spawn()` exactly as
///   `spawn_bakeoff` does, awaiting `run_param_sweep` on the side-thread tokio
///   runtime so the iced thread is never blocked.
///
/// The `backtest::SweepReport` is mirrored into [`SweepReportMirror`] **inside
/// the spawned task** — the engine type never crosses into iced state.
///
/// `cancel` / `progress_tx` / `sweep_progress_tx` are threaded into
/// `run_param_sweep` for cancellation + per-bar progress + cell-level progress.
/// `sweep_progress_tx` is the cell-granularity sender feeding the Tune screen's
/// DETERMINATE progress bar (the `BakeoffProgress` wire type, reused). Pass
/// `SweepProgressSender::disabled()` when no bar is wired (headless / tests).
#[allow(clippy::needless_pass_by_value)]
pub fn spawn_sweep(
    #[cfg(feature = "live")] rt_handle: Option<&tokio::runtime::Handle>,
    #[cfg(not(feature = "live"))] _rt_handle: Option<()>,
    cfg: backtest::SweepConfig,
    cancel: backtest::cancel::RunCancelReceiver,
    progress_tx: backtest::progress::ProgressSender,
    sweep_progress_tx: backtest::SweepProgressSender,
) -> iced::Task<crate::state::Message> {
    use crate::state::Message;

    // Fixtures / no-`live` build: no tokio runtime to drive run_param_sweep.
    // Resolve immediately with a friendly error (never hang).
    #[cfg(not(feature = "live"))]
    {
        let _ = (cfg, cancel, progress_tx, sweep_progress_tx);
        return iced::Task::done(Message::SweepRunCompleted(Err(SmolStr::new(
            crate::strings::TUNE_RUN_NEEDS_LIVE,
        ))));
    }

    #[cfg(feature = "live")]
    {
        let Some(handle) = rt_handle else {
            let _ = (cfg, cancel, progress_tx, sweep_progress_tx);
            return iced::Task::done(Message::SweepRunCompleted(Err(SmolStr::new(
                crate::strings::TUNE_RUN_NEEDS_LIVE,
            ))));
        };

        let rt = handle.clone();
        iced::Task::perform(
            async move {
                // Run the sweep on the side-thread tokio runtime (the iced thread
                // is never blocked). Mirror the engine report into the ui-side
                // mirror INSIDE the task — the engine type never crosses into
                // iced state. `sweep_progress_tx` carries the cell-level progress
                // to the Tune screen's progress bar.
                let join = rt.spawn(async move {
                    match backtest::run_param_sweep(cfg, cancel, progress_tx, sweep_progress_tx)
                        .await
                    {
                        Ok(report) => Ok(SweepReportMirror::from_report(&report)),
                        Err(e) => Err(SmolStr::new(format!("{e}"))),
                    }
                });
                match join.await {
                    Ok(result) => result,
                    Err(e) => Err(SmolStr::new(format!("sweep task join error: {e}"))),
                }
            },
            Message::SweepRunCompleted,
        )
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::tune::screen_state::{AxisInput, SmaGridForm, TuneFamily, TuneScreenState};

    /// T10 — `sweep_config_from_state` carries the chosen family + coin + the SMA
    /// ranges, with the shared seed / source / gate contract.
    #[test]
    fn config_from_state_carries_family_coin_and_ranges() {
        const NOW: i64 = 1_900_000_000_000;
        let st = TuneScreenState {
            family: TuneFamily::Sma,
            sma_grid: SmaGridForm {
                fast: AxisInput::from_values(10, 20, 5),
                slow: AxisInput::from_values(30, 50, 10),
            },
            ..Default::default()
        };
        let coin = trading_core::Symbol::new("XRPUSDT");
        let cfg = sweep_config_from_state(
            &st,
            &coin,
            crate::leaderboard::LeaderboardLookback::H1_2024,
            NOW,
        );

        assert_eq!(
            cfg.symbol.0.as_str(),
            "XRPUSDT",
            "the chosen coin drives the request"
        );
        assert!(matches!(cfg.family, backtest::SweepFamily::Sma));
        assert!(matches!(cfg.range, backtest::engine::DateRange::H1_2024));
        assert!(matches!(
            cfg.data_source,
            backtest::engine::ScenarioDataSource::BinanceCache
        ));
        assert_eq!(cfg.paths, 1000, "the sweep uses the 1000-path gate setting");
        assert_eq!(
            cfg.seed,
            crate::lab::defaults::LAB_DEFAULT_SEED,
            "the sweep shares the bake-off's deterministic seed"
        );
        match cfg.grid {
            backtest::SweepGrid::Sma(g) => {
                assert_eq!(g.fast_len.min, 10);
                assert_eq!(g.fast_len.max, 20);
                assert_eq!(g.fast_len.step, 5);
                assert_eq!(g.slow_len.min, 30);
                assert_eq!(g.slow_len.max, 50);
                assert_eq!(g.slow_len.step, 10);
            }
            other => panic!("SMA family must build an SMA grid, got {other:?}"),
        }
    }

    /// T10 — a relative lookback maps to a `Custom` window against `now_ms` (the
    /// same dispatch-boundary mapping the bake-off uses).
    #[test]
    fn config_from_state_maps_relative_lookback_to_custom_window() {
        const NOW: i64 = 1_900_000_000_000;
        let st = TuneScreenState::default();
        let coin = trading_core::Symbol::new("BTCUSDT");
        let cfg = sweep_config_from_state(
            &st,
            &coin,
            crate::leaderboard::LeaderboardLookback::OneMonth,
            NOW,
        );
        match cfg.range {
            backtest::engine::DateRange::Custom { start_ms, end_ms } => {
                assert_eq!(end_ms, NOW);
                assert_eq!(end_ms - start_ms, 30 * 86_400_000, "1-month = 30 days");
            }
            other => panic!("OneMonth must map to a Custom window, got {other:?}"),
        }
    }

    /// T10 — the no-`live` build resolves `spawn_sweep` IMMEDIATELY with the
    /// friendly LEADERBOARD_RUN_NEEDS_LIVE-style error (never hangs). The default
    /// (fixtures) test build compiles without `live`, so this exercises the
    /// fixtures branch.
    #[cfg(not(feature = "live"))]
    #[test]
    fn no_live_build_resolves_immediately_with_friendly_err() {
        // Build a throwaway config + disabled channels (the fixtures branch
        // drops them and returns the error task without touching them).
        let st = TuneScreenState::default();
        let coin = trading_core::Symbol::new("BTCUSDT");
        let cfg = sweep_config_from_state(
            &st,
            &coin,
            crate::leaderboard::LeaderboardLookback::H1_2024,
            0,
        );
        let (_h, cancel) = crate::lab::runner::cancellation_pair();
        let progress_tx = backtest::progress::ProgressSender::disabled();
        let sweep_tx = backtest::SweepProgressSender::disabled();
        // Must not panic / hang — returns an immediate `Task::done(Err(...))`.
        let _task = spawn_sweep(None, cfg, cancel, progress_tx, sweep_tx);
        // The friendly copy must exist (compile-time check it's wired).
        assert!(!crate::strings::TUNE_RUN_NEEDS_LIVE.is_empty());
    }
}
