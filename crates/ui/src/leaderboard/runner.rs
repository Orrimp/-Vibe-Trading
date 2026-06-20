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
//! ## Default-coin trigger (F1 → F3 boundary)
//!
//! v0.1.0 ships a MINIMAL trigger: a single "Run bake-off" action with a
//! default coin (`BTCUSDT`) + lookback (`H1_2024`), `data_source =
//! BinanceCache`, `robustness = Skip`. The full guided coin/budget input is
//! the next feature (F3); a default-coin button is enough to demonstrate the
//! leaderboard end-to-end here.

use smol_str::SmolStr;

use crate::leaderboard::state::BakeoffReportMirror;

/// Result posted back to the cockpit via `Message::BakeoffRunCompleted`.
///
/// `Ok(mirror)` carries the ranked leaderboard; `Err(msg)` an
/// operator-friendly failure reason (mirrors `LabRunResult`'s shape).
pub type BakeoffRunResult = Result<BakeoffReportMirror, SmolStr>;

/// The default coin the v0.1.0 trigger runs the bake-off on (F3 will make this
/// operator-selectable). Binance-style symbol, resolved against the pinned
/// hourly corpus.
pub const DEFAULT_BAKEOFF_COIN: &str = "BTCUSDT";

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
            field: backtest::BakeoffConfig::default_field(),
        },
        data_source: backtest::engine::ScenarioDataSource::BinanceCache,
        robustness: backtest::RobustnessMode::Skip,
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
/// `cancel` / `progress_tx` are threaded into `run_bakeoff` for cancellation +
/// progress, matching `run_scenario`'s contract.
#[allow(clippy::needless_pass_by_value)]
pub fn spawn_bakeoff(
    #[cfg(feature = "live")] rt_handle: Option<&tokio::runtime::Handle>,
    #[cfg(not(feature = "live"))] _rt_handle: Option<()>,
    cfg: backtest::BakeoffConfig,
    cancel: backtest::cancel::RunCancelReceiver,
    progress_tx: backtest::progress::ProgressSender,
) -> iced::Task<crate::state::Message> {
    use crate::state::Message;

    // Fixtures / no-`live` build: no tokio runtime to drive run_bakeoff.
    // Resolve immediately with a friendly error (never hang).
    #[cfg(not(feature = "live"))]
    {
        let _ = (cfg, cancel, progress_tx);
        return iced::Task::done(Message::BakeoffRunCompleted(Err(SmolStr::new(
            crate::strings::LEADERBOARD_RUN_NEEDS_LIVE,
        ))));
    }

    #[cfg(feature = "live")]
    {
        let Some(handle) = rt_handle else {
            let _ = (cfg, cancel, progress_tx);
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
                // crosses into iced state.
                let join = rt.spawn(async move {
                    match backtest::run_bakeoff(cfg, cancel, progress_tx).await {
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
    /// rule engines, real Binance data, gate skipped (the v0.1.0 trigger
    /// contract). Buy-and-hold is NOT in the field — `run_bakeoff` appends it.
    #[test]
    fn default_config_targets_default_coin_h1_binance_skip() {
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
        assert!(matches!(cfg.robustness, backtest::RobustnessMode::Skip));
        assert_eq!(cfg.request.field.len(), 4, "the 4 rule engines");
        assert!(
            !cfg.request
                .field
                .iter()
                .any(|s| s.0.as_str() == "v0.buyhold"),
            "buy-and-hold must NOT be in the field — run_bakeoff appends it"
        );
    }
}
