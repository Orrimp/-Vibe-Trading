//! advisor-leaderboard-screen v0.1.0 — bake-off runner glue.
//!
//! The cockpit ↔ bake-off engine bridge. Mirrors `lab::runner::spawn_lab_run`
//! (ADR-0030 / Design § 4.2) one-for-one, but dispatches
//! `backtest::run_bakeoff` instead of `run_scenario`:
//!
//! ```text
//! iced update thread
//!   Message::BakeoffRunRequested
//!     └──> runner::spawn_bakeoff(rt_handle, cfg)
//!              └──> rt_handle.spawn(backtest::run_bakeoff(cfg))
//!                       └──> oneshot → iced::Task::perform
//!                                └──> Message::BakeoffRunCompleted(Result<BakeoffReportMirror>)
//! ```
//!
//! ## INVARIANT (the layering seam)
//!
//! `backtest::run_bakeoff` returns `backtest::BakeoffReport` — consumed
//! through the **existing `backtest` dep**, the same seam `spawn_lab_run` uses
//! for `RunReport`. The result is mirrored into [`BakeoffReportMirror`] HERE
//! (at the dispatch boundary, before it crosses into iced state), so `ui`
//! never threads an engine type through `view`. `ui` gains NO new crate edge
//! (`strategy`/`exec`/`forecast`/`llm` stay out of the dep graph).
//!
//! ## Guided-input trigger (F3)
//!
//! [`bakeoff_config_from_state`] builds the config from the operator's CHOSEN
//! coin + lookback (the F3 guided input), `data_source = BinanceCache`,
//! `robustness = Skip`. [`default_bakeoff_config`] is retained as the
//! cold-start default (`BTCUSDT` / `H1_2024`) that seeds the guided-input
//! state and as a stable reference for tests.

use smol_str::SmolStr;

use crate::leaderboard::state::BakeoffReportMirror;

/// Result posted back to the cockpit via `Message::BakeoffRunCompleted`.
///
/// `Ok(mirror)` carries the ranked leaderboard; `Err(msg)` an
/// operator-friendly failure reason (mirrors `LabRunResult`'s shape).
pub type BakeoffRunResult = Result<BakeoffReportMirror, SmolStr>;

/// The default coin the cold-start config runs the bake-off on. The F3 guided
/// input makes this operator-selectable (`LeaderboardScreenState::coin`); this
/// remains the default the state seeds with + the `default_bakeoff_config`
/// fallback. Binance-style symbol, resolved against the pinned hourly corpus.
pub const DEFAULT_BAKEOFF_COIN: &str = "BTCUSDT";

/// The advisor bake-off field: the 9 single rule engines (4 original + 5 ADR-0071
/// signal-library arms) + the 8 F8/ADR-0067 vote ensembles + 1 ADR-0073 macro arm.
/// Buy-and-hold is appended by `run_bakeoff`. The cockpit opts into the ensembles
/// and the macro arm HERE — anchored paths are unaffected (anchor-additive contract;
/// all new arms run `write_report=false`).
fn advisor_field() -> Vec<trading_core::StrategyId> {
    let mut field = backtest::BakeoffConfig::default_field();
    field.extend(backtest::BakeoffConfig::default_ensemble_field());
    // ADR-0073: cross-asset macro regime probe (requires data/yahoo-macro/ corpus).
    field.extend(backtest::BakeoffConfig::default_macro_field());
    field
}

/// The total number of arms the advisor bake-off puts head-to-head — the
/// `advisor_field()` size **plus the buy-and-hold benchmark** that `run_bakeoff`
/// always appends. Post-ADR-0073 this is 20 for BTC/ETH (10 single rule engines +
/// 8 vote ensembles + 1 ADR-0073 macro arm + buy-and-hold; the 10 = 4 original +
/// 5 ADR-0071 arms + 1 ADR-0072 DVOL arm). For other symbols the DVOL arm is
/// filtered → 19 arms. Single-sourced from `advisor_field()` so it can never drift
/// from the real field; surfaced in the leaderboard header context (OQ-2).
/// `+ 1` is the appended benchmark.
#[must_use]
pub fn advisor_field_arm_count() -> usize {
    advisor_field().len() + 1
}

/// The advisor robustness mode: the real moving-block bootstrap gate (ADR-0063
/// § D4), seeded deterministically from the Lab seed's low 8 bytes. This opt-in
/// ACTIVATES the gate on the advisor path — which has `write_report = false`, so
/// it stays anchor-safe (`verify_anchors` 119/119). `default()` elsewhere stays
/// `Skip`, so anchored CLI paths are byte-unchanged.
fn advisor_robustness() -> backtest::RobustnessMode {
    let s = crate::lab::defaults::LAB_DEFAULT_SEED;
    backtest::RobustnessMode::Bootstrap {
        paths: 1000,
        seed: u64::from_le_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]),
    }
}

/// Build the default `BakeoffConfig` for the v0.1.0 trigger.
///
/// Default coin (`BTCUSDT`) + lookback (`H1_2024`), the default strategy field
/// (SMA / MACD / RSI / `BBands` — buy-and-hold is always appended by the loop),
/// `data_source = BinanceCache` (the real hourly corpus), `robustness = Skip`
/// (fast; the ranking is correct — `Skipped` is eligible). Pure; no I/O.
///
/// `LAB_DEFAULT_SEED` is reused so the bake-off shares the Lab's deterministic
/// seed (same-seed-every-arm is the apples-to-apples invariant, enforced
/// inside `run_bakeoff`).
#[must_use]
pub fn default_bakeoff_config() -> backtest::BakeoffConfig {
    use trading_core::Symbol;

    backtest::BakeoffConfig {
        request: backtest::BakeoffRequest {
            symbol: Symbol::new(DEFAULT_BAKEOFF_COIN),
            range: backtest::engine::DateRange::H1_2024,
            seed: crate::lab::defaults::LAB_DEFAULT_SEED,
            field: advisor_field(),
            // Defaults: H1 identity pass-through + 100_000 USDT legacy capital.
            timeframe: backtest::resample::Horizon::OneHour,
            initial_capital: rust_decimal_macros::dec!(100_000),
        },
        data_source: backtest::engine::ScenarioDataSource::BinanceCache,
        robustness: advisor_robustness(),
    }
}

/// Build a `BakeoffConfig` from the F3 guided-input state — the operator's
/// CHOSEN coin + lookback (replacing the hardcoded `BTCUSDT` / `H1_2024`
/// default).
///
/// The lookback enum is mapped to a `backtest::engine::DateRange` against
/// `now_ms` (wall-clock UTC epoch-millis) HERE, at the dispatch boundary —
/// relative windows become `Custom { now - N days, now }`, the fixed 2024
/// presets pass through. Everything else matches `default_bakeoff_config`: the
/// 4 rule engines (buy-and-hold is appended by `run_bakeoff`), the Lab's
/// deterministic seed (apples-to-apples across arms), `BinanceCache` (the real
/// hourly corpus), `robustness = Skip` (fast; ranking correct). Pure; no I/O.
///
/// The budget is intentionally NOT threaded here — the bake-off ranking is
/// budget-independent (product § journey: ranking compares risk-adjusted
/// return, the same for any budget). The budget carries forward to F4 (sizing)
/// + F5 (paper-trade) and is shown in the leaderboard header for context.
#[must_use]
pub fn bakeoff_config_from_state(
    st: &crate::leaderboard::LeaderboardScreenState,
    now_ms: i64,
) -> backtest::BakeoffConfig {
    backtest::BakeoffConfig {
        request: backtest::BakeoffRequest {
            symbol: st.coin.clone(),
            range: st.lookback.to_date_range(now_ms),
            seed: crate::lab::defaults::LAB_DEFAULT_SEED,
            field: advisor_field(),
            // Thread the operator-chosen timeframe + start capital.
            timeframe: st.timeframe.to_horizon(),
            initial_capital: st.start_capital(),
        },
        data_source: backtest::engine::ScenarioDataSource::BinanceCache,
        robustness: advisor_robustness(),
    }
}

/// Build an `iced::Task` that runs a bake-off and posts the result back to the
/// iced update loop as `Message::BakeoffRunCompleted`.
///
/// Mirrors `lab::runner::spawn_lab_run`'s fixture/live split:
///
/// - **Default (non-`live`) / no-runtime builds** — the tokio runtime that
///   drives `run_bakeoff` is absent, so the task immediately resolves with a
///   friendly `Err` directing the operator to the live build. This keeps the
///   fixtures cockpit + the render harness from hanging on a missing runtime
///   (they render a populated FIXTURE, not a live run).
/// - **`live` builds** — bridges via `rt_handle.spawn()` exactly as
///   `spawn_lab_run` does, awaiting `run_bakeoff` on the side-thread tokio
///   runtime so the iced thread is never blocked.
///
/// The `backtest::BakeoffReport` is mirrored into `BakeoffReportMirror`
/// **inside the spawned task** — the engine type never crosses into iced
/// state.
///
/// `cancel` / `progress_tx` / `bakeoff_progress_tx` are threaded into
/// `run_bakeoff` for cancellation + per-bar progress + candidate-level progress.
/// `bakeoff_progress_tx` is the candidate-granularity sender feeding the
/// leaderboard's DETERMINATE progress bar (the `BakeoffProgress` channel): the
/// binary builds it from `bakeoff_progress_pair()` and holds the matching
/// `Receiver` for `BakeoffProgressRecipe`. Pass
/// `BakeoffProgressSender::disabled()` when no bar is wired (headless / tests).
#[allow(clippy::needless_pass_by_value)]
pub fn spawn_bakeoff(
    #[cfg(feature = "live")] rt_handle: Option<&tokio::runtime::Handle>,
    #[cfg(not(feature = "live"))] _rt_handle: Option<()>,
    cfg: backtest::BakeoffConfig,
    cancel: backtest::cancel::RunCancelReceiver,
    progress_tx: backtest::progress::ProgressSender,
    bakeoff_progress_tx: backtest::progress::BakeoffProgressSender,
) -> iced::Task<crate::state::Message> {
    use crate::state::Message;

    // Fixtures / no-`live` build: no tokio runtime to drive run_bakeoff.
    // Resolve immediately with a friendly error (never hang).
    #[cfg(not(feature = "live"))]
    {
        let _ = (cfg, cancel, progress_tx, bakeoff_progress_tx);
        return iced::Task::done(Message::BakeoffRunCompleted(Err(SmolStr::new(
            crate::strings::LEADERBOARD_RUN_NEEDS_LIVE,
        ))));
    }

    #[cfg(feature = "live")]
    {
        let Some(handle) = rt_handle else {
            let _ = (cfg, cancel, progress_tx, bakeoff_progress_tx);
            return iced::Task::done(Message::BakeoffRunCompleted(Err(SmolStr::new(
                crate::strings::LEADERBOARD_RUN_NEEDS_LIVE,
            ))));
        };

        let rt = handle.clone();
        iced::Task::perform(
            async move {
                // Run the bake-off on the side-thread tokio runtime (the iced
                // thread is never blocked). Mirror the engine report into the
                // ui-side mirror INSIDE the task — the engine type never
                // crosses into iced state. `bakeoff_progress_tx` carries the
                // candidate-level progress to the leaderboard's progress bar.
                let join = rt.spawn(async move {
                    match backtest::run_bakeoff(cfg, cancel, progress_tx, bakeoff_progress_tx).await
                    {
                        Ok(report) => Ok(BakeoffReportMirror::from_report(&report)),
                        Err(e) => Err(SmolStr::new(format!("{e}"))),
                    }
                });
                match join.await {
                    Ok(result) => result,
                    Err(e) => Err(SmolStr::new(format!("bake-off task join error: {e}"))),
                }
            },
            Message::BakeoffRunCompleted,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The default config targets the default coin + H1 2024 over the four
    /// rule engines + the two F8 vote ensembles, real Binance data, and the
    /// real bootstrap robustness gate (ADR-0063 — the advisor opts in).
    /// Buy-and-hold is NOT in the field — `run_bakeoff` appends it.
    #[test]
    fn default_config_targets_default_coin_h1_binance_bootstrap() {
        let cfg = default_bakeoff_config();
        assert_eq!(cfg.request.symbol.0.as_str(), DEFAULT_BAKEOFF_COIN);
        assert!(matches!(
            cfg.request.range,
            backtest::engine::DateRange::H1_2024
        ));
        assert!(matches!(
            cfg.data_source,
            backtest::engine::ScenarioDataSource::BinanceCache
        ));
        assert!(
            matches!(
                cfg.robustness,
                backtest::RobustnessMode::Bootstrap { paths: 1000, .. }
            ),
            "the advisor opts into the real bootstrap gate (ADR-0063)"
        );
        // ADR-0071: field grew from 12 (4+8) to 17 (9+8) with the 5 new signal-library arms.
        assert_eq!(
            cfg.request.field.len(),
            17,
            "9 single rule engines (4 original + 5 ADR-0071) + 8 vote ensembles"
        );
        let ids: Vec<&str> = cfg.request.field.iter().map(|s| s.0.as_str()).collect();
        // F8 original arms must be present.
        assert!(
            ids.contains(&"v0.8.vote.majority") && ids.contains(&"v0.8.vote.unanimous"),
            "both F8 vote ensembles must be in the live field; got {ids:?}"
        );
        // advisor-combination-search new arms (ADR-0067) must be present.
        assert!(
            ids.contains(&"v0.8.vote.trend_pair"),
            "trend_pair arm must be in the live field; got {ids:?}"
        );
        assert!(
            ids.contains(&"v0.8.vote.tr_mr_macd_rsi"),
            "tr_mr_macd_rsi arm must be in the live field; got {ids:?}"
        );
        assert!(
            ids.contains(&"v0.8.vote.tr_mr_sma_bb"),
            "tr_mr_sma_bb arm must be in the live field; got {ids:?}"
        );
        assert!(
            ids.contains(&"v0.8.vote.any1of4"),
            "any1of4 arm must be in the live field; got {ids:?}"
        );
        assert!(
            ids.contains(&"v0.8.vote.k2of4"),
            "k2of4 arm must be in the live field; got {ids:?}"
        );
        assert!(
            ids.contains(&"v0.8.vote.k3of4"),
            "k3of4 arm must be in the live field; got {ids:?}"
        );
        assert!(
            !ids.contains(&"v0.buyhold"),
            "buy-and-hold must NOT be in the field — run_bakeoff appends it"
        );
    }

    /// F3 — `bakeoff_config_from_state` carries the operator's chosen coin +
    /// lookback into the request (replacing the hardcoded default), keeps the
    /// same field / seed / source / gate contract.
    #[test]
    fn config_from_state_uses_chosen_coin_and_lookback() {
        use crate::leaderboard::{LeaderboardLookback, LeaderboardScreenState};
        use trading_core::Symbol;

        const NOW: i64 = 1_900_000_000_000;
        let mut st = LeaderboardScreenState {
            coin: Symbol::new("XRPUSDT"),
            lookback: LeaderboardLookback::OneMonth,
            ..Default::default()
        };
        st.budget_input = "200".to_string();

        let cfg = bakeoff_config_from_state(&st, NOW);
        assert_eq!(
            cfg.request.symbol.0.as_str(),
            "XRPUSDT",
            "the chosen coin drives the request"
        );
        match cfg.request.range {
            backtest::engine::DateRange::Custom { start_ms, end_ms } => {
                assert_eq!(end_ms, NOW);
                assert_eq!(end_ms - start_ms, 30 * 86_400_000, "1-month = 30 days");
            }
            other => panic!("OneMonth must map to a Custom window, got {other:?}"),
        }
        // Field / seed / source / gate match the default advisor contract.
        // ADR-0071: field grew from 12 (4+8) to 17 (9+8) with the 5 new arms.
        assert_eq!(
            cfg.request.field.len(),
            17,
            "9 single rule engines (4 original + 5 ADR-0071) + 8 vote ensembles"
        );
        assert!(matches!(
            cfg.data_source,
            backtest::engine::ScenarioDataSource::BinanceCache
        ));
        assert!(matches!(
            cfg.robustness,
            backtest::RobustnessMode::Bootstrap { paths: 1000, .. }
        ));
    }
}
