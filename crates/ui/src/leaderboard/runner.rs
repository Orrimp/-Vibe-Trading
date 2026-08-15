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

/// The advisor bake-off field: 11 single rule engines (4 original, 5 ADR-0071
/// signal-library arms, 1 ADR-0072 `v0.dvol_regime`, 1 ADR-0073 `v0.macro_riskon`)
/// plus the 8 F8/ADR-0067 vote ensembles → 19 arms.
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
/// `advisor_field()` entries **that can actually run in this build**, plus the
/// buy-and-hold benchmark that `run_bakeoff` always appends.
///
/// Single-sourced from `advisor_field()` filtered through
/// `backtest::bakeoff::arm_runs_in_this_build` — the same predicate the bake-off
/// dispatch loop uses — so the number on screen can never claim an arm the loop
/// drops.
///
/// Lineage: 13 (4 singles + 8 ensembles + buy-and-hold) → 18 (+5 ADR-0071) →
/// 19 (+1 ADR-0072 DVOL) → 20 declared (+1 ADR-0073 macro). **Today it returns
/// 19**, because the macro arm cannot run in the shipped build (bug-log #81:
/// `macro_regime` is `#![cfg(feature = "yahoo")]` and nothing enables
/// `backtest/yahoo`, so the loop drops it to ABSENCE on every run). With
/// `backtest/yahoo` on it returns 20 again — the count follows the field that
/// runs, not the field that was declared.
///
/// **Symbol-blind — prefer [`advisor_field_arm_count_for`].** Review 3-15 LOW:
/// this used to be unconditional, so a SOLUSDT operator was told 20 strategies
/// ran when 19 did (the ADR-0072 D8 DVOL arm is dropped for non-BTC/ETH coins).
#[must_use]
pub fn advisor_field_arm_count() -> usize {
    advisor_field()
        .iter()
        .filter(|id| backtest::bakeoff::arm_runs_in_this_build(id.0.as_str()))
        .count()
        + 1
}

/// The number of arms the bake-off actually puts head-to-head **for this coin**.
///
/// ADR-0072 D8: `v0.dvol_regime` is dropped from the field for any coin outside
/// {BTCUSDT, ETHUSDT} (DVOL exists only for BTC and ETH), so the honest count is
/// one lower there. Routed through `backtest::bakeoff::dvol_supported` — the same
/// predicate the bake-off loop uses — so the screen can never disagree with the
/// field that runs (review 3-15 MEDIUM: that allowlist used to be copied in three
/// places).
///
/// Note this is the count the field declares **for arms that can run at all in
/// this build** (see [`advisor_field_arm_count`] — the ADR-0073 macro arm is
/// excluded outright while `backtest/yahoo` is off, bug-log #81). Both remaining
/// corpus-backed arms can *additionally* be dropped at run time when their
/// corpus is missing or does not cover the requested window (bug-log #78/#81);
/// that is reported by the arm's absence from the leaderboard rows, which is the
/// honest rendering of "did not run".
#[must_use]
pub fn advisor_field_arm_count_for(symbol_str: &str) -> usize {
    let full = advisor_field_arm_count();
    if backtest::bakeoff::dvol_supported(symbol_str) {
        full
    } else {
        full - 1
    }
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

    /// Review 3-15 LOW: the arm count must be resolved FOR THE COIN.
    ///
    /// It used to be symbol-blind, so a SOLUSDT operator read "20 strategies
    /// head-to-head" while 19 ran — the ADR-0072 D8 DVOL arm is BTC/ETH-only.
    #[test]
    fn arm_count_drops_the_dvol_arm_for_unsupported_coins() {
        let full = advisor_field_arm_count();
        for supported in ["BTCUSDT", "ETHUSDT"] {
            assert_eq!(
                advisor_field_arm_count_for(supported),
                full,
                "{supported} runs the full field including v0.dvol_regime"
            );
        }
        for unsupported in ["SOLUSDT", "DOGEUSDT", "XRPUSDT"] {
            assert_eq!(
                advisor_field_arm_count_for(unsupported),
                full - 1,
                "{unsupported} has no DVOL corpus, so the v0.dvol_regime arm is \
                 ABSENT from the field and the operator must be told {} — not {full}",
                full - 1
            );
        }
        // And the predicate is the SAME one the bake-off dispatch loop uses —
        // not a fourth copy of the allowlist.
        assert!(backtest::bakeoff::dvol_supported("BTCUSDT"));
        assert!(!backtest::bakeoff::dvol_supported("SOLUSDT"));
    }

    /// bug-log #81 / review 3-16 CRITICAL: the operator-facing arm count must
    /// count arms that CAN RUN, not arms that were declared.
    ///
    /// `v0.macro_riskon` is declared in `advisor_field()` but its regime loader
    /// is `#![cfg(feature = "yahoo")]` on the `backtest` crate, which nothing in
    /// the workspace enables — so `run_bakeoff` drops it to ABSENCE on every
    /// run of the shipped build. The count must follow the drop, or the screen
    /// tells a retail operator that a strategy was tried when it never ran.
    ///
    /// This asserts the RELATION (declared − not-runnable == counted), so it
    /// stays true under either feature setting and cannot rot into a stale
    /// literal.
    #[test]
    fn arm_count_excludes_arms_that_cannot_run_in_this_build() {
        let declared = advisor_field();
        let not_runnable = declared
            .iter()
            .filter(|id| !backtest::bakeoff::arm_runs_in_this_build(id.0.as_str()))
            .count();

        assert_eq!(
            advisor_field_arm_count(),
            declared.len() - not_runnable + 1,
            "the arm count must drop every declared arm that cannot run, + 1 benchmark"
        );

        // The macro arm is the one that can be structurally impossible, and the
        // predicate must agree with the feature that gates its loader.
        assert!(
            declared.iter().any(|id| id.0.as_str() == "v0.macro_riskon"),
            "the macro arm is still DECLARED (ADR-0073) — it is the RUN that is gated"
        );
        assert_eq!(
            backtest::bakeoff::arm_runs_in_this_build("v0.macro_riskon"),
            backtest::bakeoff::macro_arm_compiled(),
            "the macro arm runs iff its loader was compiled (backtest/yahoo)"
        );
        // Every other declared arm is always dispatchable.
        for id in declared
            .iter()
            .filter(|i| i.0.as_str() != "v0.macro_riskon")
        {
            assert!(
                backtest::bakeoff::arm_runs_in_this_build(id.0.as_str()),
                "{} must be dispatchable in every build",
                id.0.as_str()
            );
        }

        // And in the SHIPPED build (no backtest/yahoo) the honest number is 19,
        // not the 20 the field declares.
        if !backtest::bakeoff::macro_arm_compiled() {
            assert_eq!(
                advisor_field_arm_count(),
                19,
                "shipped build: 18 runnable arms + buy-and-hold (the macro arm is ABSENT)"
            );
        }
    }

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
        // Field lineage: 12 (4+8) -> 17 (+5 ADR-0071 signal-library arms) ->
        // 19 (+1 ADR-0072 v0.dvol_regime, +1 ADR-0073 v0.macro_riskon).
        assert_eq!(
            cfg.request.field.len(),
            19,
            "11 single rule engines (4 original + 5 ADR-0071 + DVOL + macro_riskon) + 8 vote ensembles"
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
        // Field lineage: 12 (4+8) -> 17 (+5 ADR-0071) -> 19 (+DVOL ADR-0072, +macro ADR-0073).
        assert_eq!(
            cfg.request.field.len(),
            19,
            "11 single rule engines (4 original + 5 ADR-0071 + DVOL + macro_riskon) + 8 vote ensembles"
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
