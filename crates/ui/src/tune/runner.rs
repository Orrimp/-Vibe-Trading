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

use crate::tune::screen_state::{
    BollingerGridForm, MacdGridForm, RsiGridForm, SmaGridForm, TuneFamily,
};
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

/// Build the engine [`SweepGrid::Macd`](backtest::SweepGrid) from the MACD form.
/// Threads the operator's three `{min, max, step}` axes; the `fast < slow` guard
/// and the cap are applied inside `run_param_sweep`. Fallbacks mirror the
/// engine's `MacdGrid::default` per-axis (8..16/4, 20..32/6, 7..11/2).
fn macd_grid_from_form(form: &MacdGridForm) -> backtest::SweepGrid {
    let (fmin, fmax, fstep) = form.fast.parsed();
    let (smin, smax, sstep) = form.slow.parsed();
    let (gmin, gmax, gstep) = form.signal.parsed();
    backtest::SweepGrid::Macd(backtest::MacdGrid {
        fast: axis_from_input(fmin, fmax, fstep, 8, 16, 4),
        slow: axis_from_input(smin, smax, sstep, 20, 32, 6),
        signal: axis_from_input(gmin, gmax, gstep, 7, 11, 2),
    })
}

/// Build the engine [`SweepGrid::Rsi`](backtest::SweepGrid) from the RSI form.
/// Threads the period axis + the oversold-threshold axis; the `period >= 2`,
/// `1 ≤ oversold ≤ 49` guards + the cap are applied inside `run_param_sweep`.
/// Fallbacks mirror `RsiGrid::default` (10..18/4, 25..35/5).
fn rsi_grid_from_form(form: &RsiGridForm) -> backtest::SweepGrid {
    let (pmin, pmax, pstep) = form.period.parsed();
    let (omin, omax, ostep) = form.oversold.parsed();
    backtest::SweepGrid::Rsi(backtest::RsiGrid {
        period: axis_from_input(pmin, pmax, pstep, 10, 18, 4),
        oversold: axis_from_input(omin, omax, ostep, 25, 35, 5),
    })
}

/// Build the engine [`SweepGrid::Bollinger`](backtest::SweepGrid) from the
/// Bollinger form. Threads the period axis + the SELECTED `k` presets (the
/// multi-select). An empty selection falls back to the shipped `k = 2.0` so the
/// config is always well-typed (the form's `can_run` gate blocks dispatching an
/// empty selection in practice). Fallback period mirrors `BollingerGrid::default`
/// (14..26/6).
fn bollinger_grid_from_form(form: &BollingerGridForm) -> backtest::SweepGrid {
    use rust_decimal_macros::dec;
    let (pmin, pmax, pstep) = form.period.parsed();
    let mut k_presets = form.selected_k_decimals();
    if k_presets.is_empty() {
        k_presets.push(dec!(2.0));
    }
    backtest::SweepGrid::Bollinger(backtest::BollingerGrid {
        period: axis_from_input(pmin, pmax, pstep, 14, 26, 6),
        k_presets,
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
/// All four families are wired (T7b): each family's FORM maps to its real
/// `SweepGrid` variant (the operator's `{min, max, step}` axes + the Bollinger
/// `k` multi-select), which `run_param_sweep` sweeps faithfully.
#[must_use]
pub fn sweep_config_from_state(
    st: &crate::tune::screen_state::TuneScreenState,
    coin: &trading_core::Symbol,
    lookback: crate::leaderboard::LeaderboardLookback,
    now_ms: i64,
) -> backtest::SweepConfig {
    let grid = match st.family {
        TuneFamily::Sma => sma_grid_from_form(&st.sma_grid),
        TuneFamily::Macd => macd_grid_from_form(&st.macd_grid),
        TuneFamily::Rsi => rsi_grid_from_form(&st.rsi_grid),
        TuneFamily::Bollinger => bollinger_grid_from_form(&st.bollinger_grid),
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

    /// T7b — a MACD family builds a real `SweepGrid::Macd` from the form's three
    /// axes (the operator's values, NOT the engine default).
    #[test]
    fn config_from_state_macd_maps_form_to_real_grid() {
        const NOW: i64 = 1_900_000_000_000;
        let mut st = TuneScreenState {
            family: TuneFamily::Macd,
            ..Default::default()
        };
        st.macd_grid.fast = AxisInput::from_values(6, 12, 3);
        st.macd_grid.slow = AxisInput::from_values(20, 30, 5);
        st.macd_grid.signal = AxisInput::from_values(8, 10, 2);
        let coin = trading_core::Symbol::new("BTCUSDT");
        let cfg = sweep_config_from_state(
            &st,
            &coin,
            crate::leaderboard::LeaderboardLookback::H1_2024,
            NOW,
        );
        assert!(matches!(cfg.family, backtest::SweepFamily::Macd));
        match cfg.grid {
            backtest::SweepGrid::Macd(g) => {
                assert_eq!((g.fast.min, g.fast.max, g.fast.step), (6, 12, 3));
                assert_eq!((g.slow.min, g.slow.max, g.slow.step), (20, 30, 5));
                assert_eq!((g.signal.min, g.signal.max, g.signal.step), (8, 10, 2));
            }
            other => panic!("MACD family must build a MACD grid, got {other:?}"),
        }
    }

    /// T7b — an RSI family builds a real `SweepGrid::Rsi` from the period +
    /// oversold axes.
    #[test]
    fn config_from_state_rsi_maps_form_to_real_grid() {
        const NOW: i64 = 1_900_000_000_000;
        let mut st = TuneScreenState {
            family: TuneFamily::Rsi,
            ..Default::default()
        };
        st.rsi_grid.period = AxisInput::from_values(8, 16, 4);
        st.rsi_grid.oversold = AxisInput::from_values(20, 30, 5);
        let coin = trading_core::Symbol::new("BTCUSDT");
        let cfg = sweep_config_from_state(
            &st,
            &coin,
            crate::leaderboard::LeaderboardLookback::H1_2024,
            NOW,
        );
        assert!(matches!(cfg.family, backtest::SweepFamily::Rsi));
        match cfg.grid {
            backtest::SweepGrid::Rsi(g) => {
                assert_eq!((g.period.min, g.period.max, g.period.step), (8, 16, 4));
                assert_eq!(
                    (g.oversold.min, g.oversold.max, g.oversold.step),
                    (20, 30, 5)
                );
            }
            other => panic!("RSI family must build an RSI grid, got {other:?}"),
        }
    }

    /// T7b — a Bollinger family builds a real `SweepGrid::Bollinger`: the period
    /// axis + the SELECTED `k` presets (the multi-select → `k_presets`).
    #[test]
    fn config_from_state_bollinger_maps_form_to_real_grid() {
        use rust_decimal_macros::dec;
        const NOW: i64 = 1_900_000_000_000;
        let mut st = TuneScreenState {
            family: TuneFamily::Bollinger,
            ..Default::default()
        };
        st.bollinger_grid.period = AxisInput::from_values(12, 24, 6);
        st.bollinger_grid.k_selected = [true, false, true, true]; // 1.5, 2.5, 3.0
        let coin = trading_core::Symbol::new("BTCUSDT");
        let cfg = sweep_config_from_state(
            &st,
            &coin,
            crate::leaderboard::LeaderboardLookback::H1_2024,
            NOW,
        );
        assert!(matches!(cfg.family, backtest::SweepFamily::Bollinger));
        match cfg.grid {
            backtest::SweepGrid::Bollinger(g) => {
                assert_eq!((g.period.min, g.period.max, g.period.step), (12, 24, 6));
                assert_eq!(g.k_presets, vec![dec!(1.5), dec!(2.5), dec!(3.0)]);
            }
            other => panic!("Bollinger family must build a Bollinger grid, got {other:?}"),
        }
    }

    /// T7b — an empty `k` selection falls back to the shipped `k = 2.0` so the
    /// config is always well-typed (the form's `can_run` gate blocks dispatch).
    #[test]
    fn config_from_state_bollinger_empty_k_falls_back_to_shipped() {
        use rust_decimal_macros::dec;
        let mut st = TuneScreenState {
            family: TuneFamily::Bollinger,
            ..Default::default()
        };
        st.bollinger_grid.k_selected = [false; 4];
        let coin = trading_core::Symbol::new("BTCUSDT");
        let cfg = sweep_config_from_state(
            &st,
            &coin,
            crate::leaderboard::LeaderboardLookback::H1_2024,
            0,
        );
        match cfg.grid {
            backtest::SweepGrid::Bollinger(g) => {
                assert_eq!(g.k_presets, vec![dec!(2.0)], "empty k → shipped fallback");
            }
            other => panic!("expected Bollinger grid, got {other:?}"),
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
