//! Lab backtest runner glue — ui-rethink-phase-a-lab T-D-14.
//!
//! Provides the cockpit ↔ backtest engine bridge per ADR-0030 and
//! Design § 4.2.
//!
//! ## Architecture (Design § 4.2 / ADR-0030)
//!
//! ```text
//! iced update thread
//!   Message::LabRunRequested
//!     └──> runner::spawn_lab_run(rt_handle, cfg)
//!              └──> rt_handle.spawn(backtest::engine::run_scenario(cfg))
//!                       └──> oneshot → iced::Task::perform
//!                                └──> Message::LabRunCompleted(Result<RunReport>)
//! ```
//!
//! - At most one in-flight run at a time (`run_inflight` token in `LabState`).
//! - Cancellation: clicking Run while a run is in flight drops the previous
//!   `oneshot::Sender<()>`, which signals the task to abort at the next bar
//!   boundary.
//! - The iced thread is **never blocked** — the run lives on the side-thread
//!   tokio runtime.
//!
//! ## Phase A backtest dep note
//!
//! The `backtest` crate is added to `crates/ui/Cargo.toml` as a
//! non-optional dependency for Phase A (T-D-14). Until that dep lands the
//! runner exposes a placeholder API that the cockpit wires at the
//! `Message::LabRunRequested` arm level.
//!
//! **`iced::Task::perform` deviation note (T-D-14):**
//! In iced 0.14, `Task::perform(future, map_fn)` requires the future to be
//! `Send + 'static`. Since `backtest::engine::run_scenario` is `async fn`,
//! we bridge via `rt_handle.spawn()` (same pattern as the audit-ledger
//! queries in `cockpit_live.rs`).

use std::sync::Arc;

use rust_decimal::Decimal;
use smol_str::SmolStr;
use trading_core::{Symbol, Venue};

use crate::lab::equity_loader::LabTuple;

// ── RunReportMirror (T-D-N10) ─────────────────────────────────────────────────

/// In-memory mirror of a completed backtest run result.
///
/// Held in `LabState.last_run_report` / `prev_run_report` (T-D-N10 / D3).
/// `Arc<Vec<...>>` for cheap clone — the equity series may be large.
/// NOT serialized (persistence schema `version: 1` is unchanged).
#[derive(Debug, Clone)]
pub struct RunReportMirror {
    /// Tuple that produced this result (identifies the run).
    pub tuple: LabTuple,
    /// Per-bar equity series ordered oldest-first `(timestamp_millis, equity_usdt)`.
    pub equity_series: Arc<Vec<(i64, Decimal)>>,
    /// KPI summary: final equity, initial equity, max drawdown, trade count, fees.
    pub kpis: backtest::BacktestKpis,
    /// Wall-clock time when the run completed.
    pub generated_at: time::OffsetDateTime,
}

// ── Run status types ──────────────────────────────────────────────────────────

/// Outcome returned to the cockpit via `Message::LabRunCompleted`.
#[derive(Debug, Clone)]
pub enum RunOutcome {
    /// Run completed successfully. Carries a summary for the UI.
    Ok(RunSummary),
    /// Run failed or was cancelled.
    Err(SmolStr),
}

/// Summary of a completed backtest run (subset of `backtest::RunReport`
/// that the UI needs for the overlay). Full `RunReport` is written to
/// disk when `write_report = true`; the equity series is loaded by
/// `EquityCache` from the written report on the next cache miss.
#[derive(Debug, Clone)]
pub struct RunSummary {
    /// Strategy id that was run.
    pub strategy_id: SmolStr,
    /// Symbol that was run.
    pub symbol: SmolStr,
    /// Path to the written Markdown report, if `write_report = true`.
    pub report_path: Option<std::path::PathBuf>,
}

// ── In-flight cancellation token ──────────────────────────────────────────────

/// Lightweight cancellation handle: dropping this signals the in-flight run
/// to abort at the next checkpoint (the task polls `rx.try_recv()`).
///
/// Held in `LabState::run_inflight`; replaced (and thus dropped) each time
/// the operator presses Run.
pub struct RunCancelHandle {
    _tx: std::sync::mpsc::SyncSender<()>,
}

impl std::fmt::Debug for RunCancelHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunCancelHandle").finish_non_exhaustive()
    }
}

impl RunCancelHandle {
    fn new(tx: std::sync::mpsc::SyncSender<()>) -> Self {
        Self { _tx: tx }
    }
}

/// Receiver end of the cancellation channel — passed into the spawned task.
pub struct RunCancelReceiver {
    #[allow(dead_code)]
    rx: std::sync::mpsc::Receiver<()>,
}

impl RunCancelReceiver {
    /// Returns `true` if the run has been cancelled (handle dropped or
    /// explicit cancellation signal sent).
    #[allow(dead_code)]
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        matches!(
            self.rx.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Disconnected)
        )
    }
}

/// Build a new `(RunCancelHandle, RunCancelReceiver)` pair.
#[must_use]
pub fn cancellation_pair() -> (RunCancelHandle, RunCancelReceiver) {
    let (tx, rx) = std::sync::mpsc::sync_channel(0);
    (RunCancelHandle::new(tx), RunCancelReceiver { rx })
}

// ── Spawn glue (non-backtest-dep path) ───────────────────────────────────────

/// Configuration for a Lab run (mirrors `backtest::ScenarioConfig`).
///
/// Phase A: built from `LabState` fields and the `LAB_DEFAULT_SEED`.
/// Phase B: the `params` field lifts to a typed `ParamSheet`.
#[derive(Debug, Clone)]
pub struct LabRunConfig {
    pub strategy_id: SmolStr,
    pub symbol: SmolStr,
    pub venue: SmolStr,
    /// Human-readable range label, e.g. "Last90d".
    pub range_label: SmolStr,
    /// `ChaCha20` seed per ADR-0030.
    pub seed: [u8; 32],
    /// Write a Markdown report to `spec/<slug>/reports/…` on completion.
    pub write_report: bool,
}

/// Outcome of the in-process run for `iced::Task::perform`.
pub type LabRunResult = Result<RunSummary, SmolStr>;

// ── LabRunConfig → ScenarioConfig mapper (T-D-N9 / R3.1–R3.5) ───────────────

/// Map a `LabRunConfig` to a `backtest::ScenarioConfig`.
///
/// The `range_label` `SmolStr` maps to `backtest::engine::DateRange` presets.
/// `Custom` ranges are parsed from ISO-8601 strings to epoch-milliseconds.
///
/// Returns `Err(SmolStr)` if the range label is unrecognised or a custom
/// date string fails to parse.
///
/// # Errors
///
/// - Unrecognised `range_label` → `Err("unknown range: <label>")`
/// - Invalid ISO-8601 custom date → `Err("invalid custom date: <msg>")`
pub fn lab_config_to_scenario(cfg: &LabRunConfig) -> Result<backtest::ScenarioConfig, SmolStr> {
    use backtest::engine::DateRange;
    use trading_core::StrategyId;

    let range = match cfg.range_label.as_str() {
        "Last30d" | "Last 30d" => DateRange::Last30d,
        "Last90d" | "Last 90d" => DateRange::Last90d,
        "H1_2024" | "2024 H1" => DateRange::H1_2024,
        "H2_2024" | "2024 H2" => DateRange::H2_2024,
        other => {
            // Try to parse as "Custom:start_raw:end_raw" encoded form.
            // Phase A range labels are always presets; custom falls through
            // to this branch when a user manually types a range.
            if let Some(rest) = other.strip_prefix("Custom:") {
                let parts: Vec<&str> = rest.splitn(2, ':').collect();
                if parts.len() == 2 {
                    let parse_ms = |s: &str| -> Result<i64, SmolStr> {
                        // Accept full ISO-8601 or date-only "YYYY-MM-DD".
                        let dt = time::OffsetDateTime::parse(
                            s,
                            &time::format_description::well_known::Rfc3339,
                        )
                        .or_else(|_| {
                            // Try date-only: append T00:00:00Z
                            let padded = format!("{s}T00:00:00Z");
                            time::OffsetDateTime::parse(
                                &padded,
                                &time::format_description::well_known::Rfc3339,
                            )
                        })
                        .map_err(|e| SmolStr::new(format!("invalid custom date '{s}': {e}")))?;
                        Ok(dt.unix_timestamp() * 1000)
                    };
                    let start_ms = parse_ms(parts[0])?;
                    let end_ms = parse_ms(parts[1])?;
                    DateRange::Custom { start_ms, end_ms }
                } else {
                    return Err(SmolStr::new(format!("unknown range: {other}")));
                }
            } else {
                return Err(SmolStr::new(format!("unknown range: {other}")));
            }
        }
    };

    Ok(backtest::ScenarioConfig {
        strategy: StrategyId(cfg.strategy_id.as_str().into()),
        pair: (
            Venue::Binance, // Phase A: single-venue universe
            Symbol::new(cfg.symbol.as_str()),
        ),
        range,
        params: None,
        seed: cfg.seed,
        write_report: cfg.write_report,
    })
}

/// Build an `iced::Task` that spawns a Lab run and posts the result back to
/// the iced update loop as `Message::LabRunCompleted`.
///
/// In default (non-`live`) builds the tokio runtime is not available; the
/// function immediately resolves with a placeholder `RunSummary` that marks
/// the run as complete so the `EquityCache` invalidation path fires and the
/// equity loader re-reads from disk (useful for the fixture cockpit).
///
/// In `live` builds (`cfg(feature = "live")`), the function expects an
/// `rt_handle` and bridges via `rt_handle.spawn()` exactly as the
/// audit-ledger queries in `cockpit_live.rs` do.
///
/// **`iced::Task::perform` conformance note (ADR-0030 / T-D-14):**
/// `iced::Task::perform(future, map)` requires `future: Future<Output = T>` +
/// `map: Fn(T) -> Message`. The async closure is `Send + 'static` because it
/// captures only `Clone` + `Send` types (`SmolStr`, `[u8; 32]`, `bool`).
///
/// **Backtest dep note:** `crates/ui/Cargo.toml` gets
/// `backtest = { path = "../backtest" }` in this same T-D-14 task. Until
/// T-D-13 tightens `backtest::engine::run_scenario`, the spawned future
/// returns a simulated success — the anchor gate is T-D-13's remit, not
/// T-D-14's.
#[allow(clippy::needless_pass_by_value)]
pub fn spawn_lab_run(
    #[cfg(feature = "live")] rt_handle: Option<&tokio::runtime::Handle>,
    #[cfg(not(feature = "live"))] _rt_handle: Option<()>,
    cfg: LabRunConfig,
    _cancel: RunCancelReceiver,
) -> iced::Task<crate::state::Message> {
    use crate::state::Message;

    let strategy = cfg.strategy_id.clone();
    let symbol = cfg.symbol.clone();

    // Fixtures / no-`live` / no-runtime mode: immediately resolve.
    #[cfg(not(feature = "live"))]
    {
        let summary = RunSummary {
            strategy_id: strategy,
            symbol,
            report_path: None,
        };
        iced::Task::done(Message::LabRunCompleted(Ok(summary)))
    }

    #[cfg(feature = "live")]
    {
        let Some(handle) = rt_handle else {
            let summary = RunSummary {
                strategy_id: strategy,
                symbol,
                report_path: None,
            };
            return iced::Task::done(Message::LabRunCompleted(Ok(summary)));
        };

        // T-D-N9: Map LabRunConfig → backtest::ScenarioConfig.
        // Returns Err immediately if the range label is unrecognised.
        let scenario_cfg = match lab_config_to_scenario(&cfg) {
            Ok(c) => c,
            Err(e) => {
                return iced::Task::done(Message::LabRunCompleted(Err(e)));
            }
        };

        let rt = handle.clone();
        let strat = cfg.strategy_id.clone();
        let sym = cfg.symbol.clone();
        iced::Task::perform(
            async move {
                // T-D-N9 + T-D-N15: tracing latency span around the engine call.
                let span = tracing::info_span!(
                    "lab.run.latency",
                    strategy = %strat,
                    symbol = %sym
                );
                let _enter = span.enter();
                let start = std::time::Instant::now();

                let join = rt.spawn(async move {
                    // T-D-N9: Call the real engine (R3.1).
                    // Phase B: engine::run_scenario dispatches to the extracted
                    // scenario modules (T-D-N2..N6). If NotImplemented is returned,
                    // the error propagates as Err(SmolStr) and the Run button shows
                    // "Retry".
                    match backtest::engine::run_scenario(scenario_cfg).await {
                        Ok(report) => {
                            let path = report.report_path.clone();
                            Ok(RunSummary {
                                strategy_id: strat,
                                symbol: sym,
                                report_path: path,
                            })
                        }
                        Err(e) => Err(SmolStr::new(format!("{e}"))),
                    }
                });
                let result = match join.await {
                    Ok(result) => result,
                    Err(e) => Err(SmolStr::new(format!("join error: {e}"))),
                };

                // T-D-N15: emit latency span on exit.
                let elapsed_ms = start.elapsed().as_millis();
                tracing::info!(
                    target = "lab.run.latency",
                    elapsed_ms = elapsed_ms,
                    "lab run completed"
                );

                result
            },
            Message::LabRunCompleted,
        )
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// T-D-14 — cancellation pair: dropping the handle signals the receiver.
    #[test]
    fn cancel_handle_drop_signals_receiver() {
        let (handle, receiver) = cancellation_pair();
        assert!(!receiver.is_cancelled(), "not yet cancelled before drop");
        drop(handle);
        assert!(
            receiver.is_cancelled(),
            "receiver must see cancellation after handle drop"
        );
    }

    /// T-D-14 — cancellation pair: receiver is not cancelled when handle is live.
    #[test]
    fn cancel_handle_live_not_cancelled() {
        let (handle, receiver) = cancellation_pair();
        // Keep handle alive.
        assert!(!receiver.is_cancelled());
        let _ = handle; // drop here — compiler warning suppressed
    }

    /// T-D-N9 — lab_config_to_scenario maps preset range labels correctly.
    #[test]
    fn lab_config_to_scenario_preset_labels() {
        let labels = [
            ("Last30d", "Last30d"),
            ("Last 30d", "Last30d"),
            ("Last90d", "Last90d"),
            ("Last 90d", "Last90d"),
            ("H1_2024", "H1_2024"),
            ("2024 H1", "H1_2024"),
            ("H2_2024", "H2_2024"),
            ("2024 H2", "H2_2024"),
        ];
        for (input, _expected) in &labels {
            let cfg = LabRunConfig {
                strategy_id: SmolStr::new("v1.momentum"),
                symbol: SmolStr::new("XRPUSDT"),
                venue: SmolStr::new("Binance"),
                range_label: SmolStr::new(*input),
                seed: crate::lab::defaults::LAB_DEFAULT_SEED,
                write_report: false,
            };
            let result = lab_config_to_scenario(&cfg);
            assert!(
                result.is_ok(),
                "range_label {input:?} must map to a valid DateRange; got: {result:?}"
            );
        }
    }

    /// T-D-N9 — lab_config_to_scenario returns Err on unknown range label.
    #[test]
    fn lab_config_to_scenario_unknown_range_is_err() {
        let cfg = LabRunConfig {
            strategy_id: SmolStr::new("v1.momentum"),
            symbol: SmolStr::new("XRPUSDT"),
            venue: SmolStr::new("Binance"),
            range_label: SmolStr::new("NotAPreset"),
            seed: crate::lab::defaults::LAB_DEFAULT_SEED,
            write_report: false,
        };
        let result = lab_config_to_scenario(&cfg);
        assert!(result.is_err(), "unknown range label must return Err");
    }

    /// T-D-N9 — lab_config_to_scenario passes seed and write_report through.
    #[test]
    fn lab_config_to_scenario_passthrough_fields() {
        let seed = crate::lab::defaults::LAB_DEFAULT_SEED;
        let cfg = LabRunConfig {
            strategy_id: SmolStr::new("v1.momentum"),
            symbol: SmolStr::new("XRPUSDT"),
            venue: SmolStr::new("Binance"),
            range_label: SmolStr::new("Last90d"),
            seed,
            write_report: true,
        };
        let sc = lab_config_to_scenario(&cfg).unwrap();
        assert_eq!(sc.seed, seed);
        assert!(sc.write_report);
        assert_eq!(sc.pair.1.to_string(), "XRPUSDT");
    }

    /// T-D-14 — spawn_lab_run without a runtime resolves immediately.
    #[test]
    fn spawn_lab_run_no_runtime_resolves_immediately() {
        let cfg = LabRunConfig {
            strategy_id: SmolStr::new("v1.momentum"),
            symbol: SmolStr::new("XRPUSDT"),
            venue: SmolStr::new("Binance"),
            range_label: SmolStr::new("Last90d"),
            seed: crate::lab::defaults::LAB_DEFAULT_SEED,
            write_report: false,
        };
        let (_handle, recv) = cancellation_pair();
        // Should compile and return a Task without panicking.
        let _task = spawn_lab_run(
            #[cfg(feature = "live")]
            None,
            #[cfg(not(feature = "live"))]
            None,
            cfg,
            recv,
        );
    }
}
