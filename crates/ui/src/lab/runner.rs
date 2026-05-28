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
use trading_core::{Bar, Symbol, Venue};

// cockpit-activity-status-bar v0.1.0 T-D-N7 — import ActivitySender/Kind
// only when the `live` feature (and therefore the `agent` crate) is enabled.
// cockpit-activity-status-bar v0.1.0 T-D-N7 — import ActivityKind for the
// Yahoo preload producer wiring inside `spawn_lab_run`. `ActivitySender` is
// used via the `agent::ActivitySender` path in the parameter type.
#[cfg(feature = "live")]
use agent::activity::ActivityKind;

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
    /// The bars used for this run (from `RunReport.bars`), shared cheaply via Arc.
    ///
    /// Lab screen prefers these bars over the live `chart_buffer` so fill
    /// triangle markers anchor correctly even when `chart_buffer` is empty
    /// (e.g. Yahoo or synthetic-2023 runs).
    pub bars: Arc<Vec<Bar>>,
    /// lab-polish-round-2 R1 — position-curve for the Lab position-curve widget.
    ///
    /// Already filtered to the active symbol (from `RunSummary.position_curve`).
    /// `(close_ts_millis, signed_qty)` ordered oldest-first.
    /// Empty when no position data is available.
    pub position_curve: Arc<Vec<(i64, Decimal)>>,
}

// ── Run status types ──────────────────────────────────────────────────────────

/// Outcome returned to the cockpit via `Message::LabRunCompleted`.
///
/// The size disparity between `Ok(RunSummary)` and `Err(SmolStr)` is
/// intentional — `RunSummary` carries the full run payload (equity
/// series, fills, bars) and is moved into the cockpit's `LabState`.
/// `Box`ing `RunSummary` would add an allocation on the success path
/// where most runs land; the lint trade-off favours direct embedding.
#[allow(clippy::large_enum_variant)]
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
///
/// v2 / Q3=(a) — carries the in-memory equity series + fills + KPIs
/// so the binary-side wrapper can build a `RunReportMirror` and
/// dispatch `ChartMarkersLoaded` without re-reading the written
/// Markdown report from disk.
#[derive(Debug, Clone)]
pub struct RunSummary {
    /// Strategy id that was run.
    pub strategy_id: SmolStr,
    /// Symbol that was run.
    pub symbol: SmolStr,
    /// Path to the written Markdown report, if `write_report = true`.
    pub report_path: Option<std::path::PathBuf>,
    /// Per-bar equity curve `(timestamp_millis, equity_usdt)`.
    /// Built from `RunReport.equity_series` in `spawn_lab_run`'s
    /// post-completion block (R2.4).
    pub equity_series: Vec<(i64, Decimal)>,
    /// Executed fills in chronological order. May be empty for
    /// scenarios that don't yet populate `RunReport.fills` (today
    /// momentum / pairs / TCN all return `Vec::new()` per the Phase B
    /// TODO at engine.rs:307; R5.2 extends them in Wave D-2).
    pub fills: Vec<trading_core::FillView>,
    /// Aggregate KPI summary from `RunReport.kpis`.
    pub kpis: backtest::BacktestKpis,
    /// Bars used by this run (from `RunReport.bars`).
    ///
    /// Empty for cross-sectional strategies (momentum/pairs/TCN).
    /// Non-empty for single-symbol SMA/Composed arms.
    /// Used by `RunReportMirror` so the Lab chart can anchor fill markers.
    pub bars: Arc<Vec<Bar>>,
    /// lab-polish-round-2 R1 — position-curve already filtered to the active
    /// symbol. `(close_ts_millis, signed_qty)` oldest-first.
    /// Empty when the scenario produced no position data.
    pub position_curve: Vec<(i64, Decimal)>,
}

// ── In-flight cancellation token (re-exported from backtest::cancel) ─────────

/// Re-export `RunCancelHandle` from `backtest::cancel` so existing call
/// sites in `cockpit_live.rs` and tests need no import changes.
pub use backtest::cancel::RunCancelHandle;

/// Re-export `RunCancelReceiver` from `backtest::cancel`.
pub use backtest::cancel::RunCancelReceiver;

/// Build a new `(RunCancelHandle, RunCancelReceiver)` pair.
///
/// Delegates to `backtest::cancel::cancellation_pair()`.  Existing call
/// sites in `cockpit_live.rs` need no changes.
#[must_use]
pub fn cancellation_pair() -> (RunCancelHandle, RunCancelReceiver) {
    backtest::cancel::cancellation_pair()
}

// ── Spawn glue (non-backtest-dep path) ───────────────────────────────────────

/// Configuration for a Lab run (mirrors `backtest::ScenarioConfig`).
///
/// Phase A: built from `LabState` fields and the `LAB_DEFAULT_SEED`.
/// Phase B: the `params` field lifts to a typed `ParamSheet`.
/// v0.1.0 (lab-yahoo-realdata T-C3.6): `data_source` added.
#[derive(Debug, Clone)]
pub struct LabRunConfig {
    pub strategy_id: SmolStr,
    /// UI-side Binance-style symbol (e.g. `BTCUSDT`). Conversion to Yahoo-
    /// native format happens at the dispatch boundary in `preload_yahoo_bars`
    /// via `data::yahoo::binance_to_yahoo_ticker` (Q6 = (a) / D7).
    pub symbol: SmolStr,
    pub venue: SmolStr,
    /// Human-readable range label, e.g. "Last90d".
    pub range_label: SmolStr,
    /// `ChaCha20` seed per ADR-0030.
    pub seed: [u8; 32],
    /// Write a Markdown report to `spec/<slug>/reports/…` on completion.
    pub write_report: bool,
    /// Data source for this run. `Synthetic` (default) preserves byte-identical
    /// pre-v0.1.0 behaviour (H5 / R-NR.8). `YahooCache` loads real bars from
    /// the local parquet cache before engine dispatch (T-AR1 / Q1 = (b)).
    pub data_source: crate::lab::state::LabDataSource,
    /// lab-polish-round-2 R2 — operator-tuned SMA fast window (None → 20 default).
    pub sma_fast_len: Option<usize>,
    /// lab-polish-round-2 R2 — operator-tuned SMA slow window (None → 50 default).
    pub sma_slow_len: Option<usize>,
}

/// Outcome of the in-process run for `iced::Task::perform`.
pub type LabRunResult = Result<RunSummary, SmolStr>;

// ── LabYahooBarSource trait (lab-recipe-test-harness T-D1 / ADR-0048) ─────────

/// Boxed future returned by `LabYahooBarSource::preload`.
///
/// Type alias avoids the `clippy::type_complexity` lint on the trait method
/// return type while keeping the trait object-safe.
#[cfg(feature = "live")]
pub type PreloadFuture<'a> = std::pin::Pin<
    Box<
        dyn std::future::Future<Output = Result<(Vec<trading_core::Bar>, SmolStr), SmolStr>>
            + Send
            + 'a,
    >,
>;

/// Abstraction over the Yahoo bar preload step in `spawn_lab_run`.
///
/// Allows tests to inject a `MockLabYahooBarSource` without touching HTTP/parquet.
/// Production path uses `DefaultLabYahooBarSource` which delegates to
/// `preload_yahoo_bars` (the existing parquet+http impl).
///
/// Object-safe via `BoxFuture` return (no async-trait crate needed).
/// Bounded by `Send + Sync + 'static` so the impl can be moved into
/// `iced::Task::perform`'s `Send + 'static` async closure.
///
/// Feature gate: only compiled under `live` so tests with `--features live`
/// can construct mocks without requiring the `yahoo` feature flag.
#[cfg(feature = "live")]
pub trait LabYahooBarSource: Send + Sync + 'static {
    /// Preload Yahoo bars for the given config + scenario range.
    ///
    /// Returns `(bars, revision_sha)` on success, `Err(SmolStr)` on failure.
    /// The production impl calls `preload_yahoo_bars`; the mock sleeps then
    /// returns deterministic bars.
    fn preload<'a>(
        &'a self,
        cfg: &'a LabRunConfig,
        range: &'a backtest::engine::DateRange,
    ) -> PreloadFuture<'a>;
}

/// Production `LabYahooBarSource` implementation — delegates to `preload_yahoo_bars`.
///
/// Wired by `spawn_lab_run` when no mock is injected (default path).
/// Only compiled when both `live` and `yahoo` features are enabled because
/// `preload_yahoo_bars` itself requires `#[cfg(feature = "yahoo")]`.
#[cfg(all(feature = "live", feature = "yahoo"))]
pub struct DefaultLabYahooBarSource;

#[cfg(all(feature = "live", feature = "yahoo"))]
impl LabYahooBarSource for DefaultLabYahooBarSource {
    fn preload<'a>(
        &'a self,
        cfg: &'a LabRunConfig,
        range: &'a backtest::engine::DateRange,
    ) -> PreloadFuture<'a> {
        Box::pin(preload_yahoo_bars(cfg, range))
    }
}

// ── Yahoo bar pre-loading helpers (lab-yahoo-realdata T-C3.6 / T-AR1) ────────

/// Map a `backtest::engine::DateRange` to `(start_ms, end_ms)` UTC epoch-millis.
#[cfg(feature = "yahoo")]
///
/// `H1_2024` / `H2_2024` use fixed calendar boundaries so they are deterministic.
/// `Last30d` / `Last90d` use wall-clock `now()` — intentional; these are the
/// operator's "show me recent data" presets for Yahoo real bars (H1 hypothesis).
/// `Custom` passes the caller's values through unchanged.
///
/// Note: `time::OffsetDateTime::now_utc()` is only reachable when
/// `data_source == YahooCache` (rolling presets are date-relative by design
/// for real-data); the synthetic path never calls this function.
fn range_to_ms_pair(range: &backtest::engine::DateRange) -> (i64, i64) {
    use backtest::engine::DateRange;
    const MS_PER_DAY: i64 = 86_400_000;
    let now_ms = time::OffsetDateTime::now_utc().unix_timestamp() * 1_000;
    match range {
        DateRange::Last30d => (now_ms - 30 * MS_PER_DAY, now_ms),
        DateRange::Last90d => (now_ms - 90 * MS_PER_DAY, now_ms),
        DateRange::H1_2024 => (1_704_067_200_000, 1_719_792_000_000), // 2024-01-01 .. 2024-07-01 UTC
        DateRange::H2_2024 => (1_719_792_000_000, 1_735_689_600_000), // 2024-07-01 .. 2025-01-01 UTC
        DateRange::Custom { start_ms, end_ms } => (*start_ms, *end_ms),
    }
}

/// Pre-load Yahoo bars upstream of engine dispatch (T-AR1 / Q1 = (b)).
///
/// Called by `spawn_lab_run` when `cfg.data_source == YahooCache`.
/// Converts the UI Binance-style symbol to Yahoo-native at the dispatch
/// boundary (Q6 = (a) / D7), derives the adaptive cadence (Q4 = (c) / D6),
/// and returns `(bars, revision_sha)` for logging / report forensics.
///
/// # Errors
///
/// Returns `Err(SmolStr)` with an operator-friendly message on:
/// - Unknown ticker (`UnmappedTicker`)
/// - Cache miss (`CacheMiss`) — includes a `cargo run` hint
/// - Coverage below 95% (`MissingData`)
/// - Revision manifest missing or tampered (`RevisionMissing` / `RevisionMismatch`)
#[cfg(feature = "yahoo")]
async fn preload_yahoo_bars(
    cfg: &LabRunConfig,
    scenario_range: &backtest::engine::DateRange,
) -> Result<(Vec<trading_core::Bar>, SmolStr), SmolStr> {
    use data::yahoo::{Interval, YahooBarSource, YahooError, binance_to_yahoo_ticker};
    use trading_core::Symbol;

    // Convert UI ticker (BTCUSDT) → Yahoo ticker (BTC-USD) at the boundary.
    let sym = Symbol::new(cfg.symbol.as_str());
    let yahoo_ticker =
        binance_to_yahoo_ticker(&sym).map_err(|e| SmolStr::new(format!("ticker mapping: {e}")))?;

    // Derive adaptive cadence from the selected date range.
    let (start_ms, end_ms) = range_to_ms_pair(scenario_range);
    let interval = Interval::derive_from_range(start_ms, end_ms);

    // Construct the cache source. Ensure the parent exists so the auto-fetch
    // fallback can write into it (`fetch_and_cache` calls
    // `create_dir_all(<TICKER>/<INTERVAL>/<YEAR>/)` under the hood, but the
    // root must exist first).
    let cache_root = std::path::PathBuf::from("data/yahoo");
    let _ = std::fs::create_dir_all(&cache_root);
    let src = YahooBarSource::new(cache_root);

    // First: try the cache.
    let cache_attempt = src.load_cached(yahoo_ticker.as_str(), interval, start_ms, end_ms);

    // On CacheMiss / RevisionMissing, fall back to fetching online (operator
    // decision 2026-05-25 — auto-fetch on demand). Other errors surface as-is.
    let loaded = match cache_attempt {
        Ok(loaded) => loaded,
        Err(err @ (YahooError::CacheMiss { .. } | YahooError::RevisionMissing { .. })) => {
            tracing::info!(
                target: "lab.yahoo",
                ticker = %yahoo_ticker,
                interval = ?interval,
                reason = %err,
                "cache miss — auto-fetching from Yahoo Finance"
            );
            // Online fetch with exponential backoff (mirrors the CLI's
            // `fetch_with_backoff` shape). Errors here propagate up.
            match fetch_with_backoff(&src, yahoo_ticker.as_str(), interval, start_ms, end_ms).await
            {
                Ok(()) => {
                    tracing::info!(
                        target: "lab.yahoo",
                        ticker = %yahoo_ticker,
                        "Yahoo fetch completed; reloading cache"
                    );
                    src.load_cached(yahoo_ticker.as_str(), interval, start_ms, end_ms)
                        .map_err(|e| SmolStr::new(format!("yahoo cache load (post-fetch): {e}")))?
                }
                Err(e) => {
                    return Err(SmolStr::new(format!(
                        "Yahoo auto-fetch failed for {yahoo_ticker}: {e}. \
                         Check network connectivity or run the fetch CLI manually."
                    )));
                }
            }
        }
        Err(e) => return Err(SmolStr::new(format!("yahoo cache load: {e}"))),
    };

    tracing::info!(
        target: "lab.yahoo",
        ticker = %yahoo_ticker,
        interval = ?interval,
        bars = loaded.loaded_count,
        revision_sha = %loaded.revision_sha,
        "Yahoo bars ready for Lab run"
    );

    Ok((loaded.bars, SmolStr::new(loaded.revision_sha)))
}

/// Exponential-backoff retry wrapper around `YahooBarSource::fetch_and_cache`.
/// Mirrors the CLI binary's `fetch_with_backoff` so the in-flight auto-fetch
/// path is equivalent. 5 retries, 1s → 60s cap.
#[cfg(feature = "yahoo")]
async fn fetch_with_backoff(
    src: &data::yahoo::YahooBarSource,
    ticker: &str,
    interval: data::yahoo::Interval,
    start_ms: i64,
    end_ms: i64,
) -> Result<(), data::yahoo::YahooError> {
    use data::yahoo::YahooError;
    use std::time::Duration;

    let max_retries: u32 = 5;
    let mut backoff = Duration::from_secs(1);
    let cap = Duration::from_secs(60);

    // Bug #63 — per-attempt timeout so a hung Yahoo endpoint can't freeze
    // the cockpit indefinitely. 60 s is well above normal fetch time
    // (<5 s typical) and well below operator patience threshold.
    let per_attempt_timeout = Duration::from_secs(60);

    for attempt in 0..=max_retries {
        let fetch_future = src.fetch_and_cache(ticker, interval, start_ms, end_ms);
        match tokio::time::timeout(per_attempt_timeout, fetch_future).await {
            Err(_) => {
                tracing::warn!(
                    target: "lab.yahoo",
                    ticker = %ticker,
                    attempt,
                    timeout_s = per_attempt_timeout.as_secs(),
                    "fetch timed out — retrying with backoff"
                );
                if attempt < max_retries {
                    tokio::time::sleep(backoff).await;
                    backoff = (backoff * 2).min(cap);
                    continue;
                }
                return Err(YahooError::Http(format!(
                    "fetch timeout ({}s) after {} attempts for {ticker}",
                    per_attempt_timeout.as_secs(),
                    max_retries + 1
                )));
            }
            Ok(result) => match result {
                Ok(loaded) => {
                    tracing::info!(
                        target: "lab.yahoo",
                        ticker = %ticker,
                        bars = loaded.loaded_count,
                        expected = loaded.expected_count,
                        revision_sha = %&loaded.revision_sha[..8],
                        "Yahoo fetch OK"
                    );
                    return Ok(());
                }
                Err(YahooError::RateLimited { retry_after_secs }) if attempt < max_retries => {
                    let delay = backoff.max(Duration::from_secs(retry_after_secs));
                    tracing::warn!(
                        target: "lab.yahoo",
                        ticker = %ticker,
                        attempt,
                        delay_s = delay.as_secs(),
                        "rate-limited by Yahoo, backing off"
                    );
                    tokio::time::sleep(delay).await;
                    backoff = (backoff * 2).min(cap);
                }
                Err(e) => return Err(e),
            },
        }
    }

    Err(YahooError::Http(format!(
        "max retries ({max_retries}) exhausted for {ticker}"
    )))
}

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
            Venue::Binance, // Phase A: single-venue universe (Yahoo bars override on data_source)
            Symbol::new(cfg.symbol.as_str()),
        ),
        range,
        params: None,
        seed: cfg.seed,
        write_report: cfg.write_report,
        // lab-yahoo-realdata T-C3.6: data_source and bars_override are set by
        // the caller (spawn_lab_run) after this helper returns; for non-Yahoo
        // paths these remain at their defaults (Synthetic, None).
        data_source: backtest::engine::ScenarioDataSource::default(),
        bars_override: None,
        // lab-polish-round-2 R2 — pass operator-tuned SMA windows through.
        sma_fast_len: cfg.sma_fast_len,
        sma_slow_len: cfg.sma_slow_len,
        // v5-latency-slippage-sim R1 — default noop (anchor-safe).
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
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
///
/// **cockpit-activity-status-bar v0.1.0 T-D-N7:**
/// `activity_sender` is `Some` in live builds (provided by the caller).
/// When `Some`, the function emits a `YahooPreload` `ActivityHandle` around
/// the preload block (T-D-N7). `ActivitySender` is `Clone + Send` so it
/// crosses the `iced::Task::perform` async closure safely.
/// The LabRun `ActivityHandle` lifecycle (T-D-N8) is managed by the caller
/// on the iced side — see `AppState::lab_activity_handle`.
#[allow(clippy::needless_pass_by_value, clippy::too_many_lines)]
pub fn spawn_lab_run(
    #[cfg(feature = "live")] rt_handle: Option<&tokio::runtime::Handle>,
    #[cfg(not(feature = "live"))] _rt_handle: Option<()>,
    cfg: LabRunConfig,
    cancel: RunCancelReceiver,
    progress_tx: backtest::progress::ProgressSender,
    #[cfg(feature = "live")] activity_sender: Option<agent::activity::ActivitySender>,
    #[cfg(not(feature = "live"))] _activity_sender: Option<()>,
    // lab-recipe-test-harness T-D1 / ADR-0048: injectable Yahoo bar source.
    // None => production path (DefaultLabYahooBarSource via preload_yahoo_bars).
    // Some(source) => test injection (e.g. MockLabYahooBarSource).
    // Only compiled under `live`; non-live builds receive a dummy Option<()>.
    #[cfg(feature = "live")] yahoo_source_override: Option<Box<dyn LabYahooBarSource>>,
    #[cfg(not(feature = "live"))] _yahoo_source_override: Option<()>,
) -> iced::Task<crate::state::Message> {
    use crate::state::Message;

    let strategy = cfg.strategy_id.clone();
    let symbol = cfg.symbol.clone();

    // Fixtures / no-`live` / no-runtime mode: immediately resolve.
    #[cfg(not(feature = "live"))]
    {
        // In fixture mode there's no tokio runtime to drive the progress channel.
        // Drop the cancel receiver + progress sender immediately (no-op).
        let _ = cancel;
        let _ = progress_tx;
        let summary = RunSummary {
            strategy_id: strategy,
            symbol,
            report_path: None,
            equity_series: Vec::new(),
            fills: Vec::new(),
            kpis: backtest::BacktestKpis::default(),
            bars: Arc::new(Vec::new()),
            position_curve: Vec::new(),
        };
        iced::Task::done(Message::LabRunCompleted(Ok(summary)))
    }

    #[cfg(feature = "live")]
    {
        let Some(handle) = rt_handle else {
            let _ = cancel;
            let _ = progress_tx;
            let summary = RunSummary {
                strategy_id: strategy,
                symbol,
                report_path: None,
                equity_series: Vec::new(),
                fills: Vec::new(),
                kpis: backtest::BacktestKpis::default(),
                bars: Arc::new(Vec::new()),
                position_curve: Vec::new(),
            };
            return iced::Task::done(Message::LabRunCompleted(Ok(summary)));
        };

        // T-D-N9: Map LabRunConfig → backtest::ScenarioConfig.
        // Returns Err immediately if the range label is unrecognised.
        #[allow(unused_mut)]
        let mut scenario_cfg = match lab_config_to_scenario(&cfg) {
            Ok(c) => c,
            Err(e) => {
                return iced::Task::done(Message::LabRunCompleted(Err(e)));
            }
        };

        // lab-yahoo-realdata T-C3.6 / T-AR1: when data source is YahooCache,
        // pre-load bars BEFORE the engine dispatch. As of 2026-05-25 the
        // preload is async (auto-fetches online on cache miss), so the call
        // is moved INTO the spawned task below.
        // The `yahoo` feature gate ensures this compiles out in builds that
        // don't want the parquet dependency (R-NR.7 / H6).
        #[cfg(not(feature = "yahoo"))]
        {
            if cfg.data_source == crate::lab::state::LabDataSource::YahooCache {
                return iced::Task::done(Message::LabRunCompleted(Err(SmolStr::new(
                    "YahooCache data source requires the `yahoo` feature; \
                     rebuild with `--features yahoo`",
                ))));
            }
        }

        let rt = handle.clone();
        let strat = cfg.strategy_id.clone();
        let sym = cfg.symbol.clone();
        let cfg_for_preload = cfg.clone();
        // cockpit-activity-status-bar T-D-N7: clone ActivitySender into the
        // async closure. ActivitySender wraps broadcast::Sender which is
        // Clone + Send — safe to move across the iced::Task::perform boundary.
        let activity_sender_for_closure = activity_sender;
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

                // YahooCache pre-load with auto-fetch fallback (operator
                // decision 2026-05-25). On cache miss, fetches online via
                // `fetch_and_cache` then retries the load. First Run on a
                // ticker takes 30-60 s for the network round-trip; subsequent
                // Runs hit the cache.
                // lab-recipe-test-harness T-D1 / ADR-0048: choose the Yahoo source.
                // When `yahoo_source_override` is `Some`, use it (test injection).
                // When `None`, fall through to the production `#[cfg(feature = "yahoo")]`
                // block which uses `DefaultLabYahooBarSource` (same as before).
                //
                // The `Option<Box<dyn LabYahooBarSource>>` is moved into the async
                // closure above so it is available here regardless of yahoo feature.
                let yahoo_source_moved: Option<Box<dyn LabYahooBarSource>> = yahoo_source_override;

                // When a test source override is provided, use it for any
                // data_source == YahooCache run (even when `yahoo` feature is off).
                if let Some(ref source) = yahoo_source_moved {
                    if cfg_for_preload.data_source == crate::lab::state::LabDataSource::YahooCache {
                        // lab-recipe-test-harness T-D1: sentinel emission BEFORE preload.
                        // This is the contract tested by `sentinel_fires_before_preload_await`.
                        progress_tx.try_send(backtest::progress::Progress {
                            current_bar: 0,
                            total_bars: 1,
                            elapsed_ms: 0,
                        });

                        let preload_result =
                            source.preload(&cfg_for_preload, &scenario_cfg.range).await;
                        match preload_result {
                            Ok((bars, _sha)) => {
                                scenario_cfg.data_source =
                                    backtest::engine::ScenarioDataSource::YahooCache;
                                scenario_cfg.bars_override = Some(bars);
                            }
                            Err(e) => {
                                return Err(e);
                            }
                        }
                    }
                } else {
                    #[cfg(feature = "yahoo")]
                    {
                        if cfg_for_preload.data_source
                            == crate::lab::state::LabDataSource::YahooCache
                        {
                            // Bug #64 — emit a sentinel Progress event BEFORE the
                            // network/disk preload await so the operator sees an
                            // explicit 0%-with-label state instead of the 30%
                            // indeterminate fallback during the wait. (Cold cache
                            // misses can spin for 30-60 s on the Yahoo round-trip.)
                            // `total_bars = 1` is a placeholder; the real value
                            // arrives once the engine's bar loop emits its first
                            // event. The widget renders this as 0% with label
                            // "0 / 1 bars · 0.0s" — clearly identifiable as a
                            // pre-engine state.
                            progress_tx.try_send(backtest::progress::Progress {
                                current_bar: 0,
                                total_bars: 1,
                                elapsed_ms: 0,
                            });

                            // T-D-N7 — cockpit-activity-status-bar Yahoo preload
                            // producer wiring (approach A: inline handle, no Send
                            // needed — preload runs in the iced::Task::perform
                            // closure, NOT inside rt.spawn).
                            //
                            // Build label: "Yahoo <SYMBOL> · <RANGE>" (≤ 64 chars).
                            // ActivitySender is Clone + Send; ActivityHandle is !Send
                            // but lives entirely within this async closure (single task).
                            let yahoo_label = format!(
                                "Yahoo {} · {}",
                                cfg_for_preload.symbol, cfg_for_preload.range_label
                            );
                            let yahoo_activity_handle = activity_sender_for_closure
                                .as_ref()
                                .map(|s| s.start(ActivityKind::YahooPreload, yahoo_label));

                            let preload_result =
                                preload_yahoo_bars(&cfg_for_preload, &scenario_cfg.range).await;
                            match preload_result {
                                Ok((bars, _sha)) => {
                                    scenario_cfg.data_source =
                                        backtest::engine::ScenarioDataSource::YahooCache;
                                    scenario_cfg.bars_override = Some(bars);
                                    // yahoo_activity_handle dropped here →
                                    // emits End { Success } automatically (R1.3).
                                    drop(yahoo_activity_handle);
                                }
                                Err(e) => {
                                    // Emit End { Failed } before returning the error (R1.3 / F3).
                                    if let Some(handle) = yahoo_activity_handle {
                                        handle.fail(e.as_str());
                                    }
                                    return Err(e);
                                }
                            }
                        }
                    }
                }

                let join = rt.spawn(async move {
                    // T-D-N9: Call the real engine (R3.1).
                    // Phase B: engine::run_scenario dispatches to the extracted
                    // scenario modules (T-D-N2..N6). If NotImplemented is returned,
                    // the error propagates as Err(SmolStr) and the Run button shows
                    // "Retry".
                    match backtest::engine::run_scenario(scenario_cfg, cancel, progress_tx).await {
                        Ok(report) => {
                            let path = report.report_path.clone();
                            // R2.4 — promote the in-memory equity / fills / kpis
                            // from RunReport into RunSummary so the binary-side
                            // wrapper avoids a disk round-trip.
                            let equity_series: Vec<(i64, rust_decimal::Decimal)> = report
                                .equity_series
                                .iter()
                                .map(|(ts, money)| (ts.unix_millis(), money.amount()))
                                .collect();
                            // lab-polish-round-2 R1 — filter position_curve_raw to
                            // the active symbol so the UI gets a ready-to-render
                            // `Vec<(i64, Decimal)>` without any further filtering.
                            let active_sym = trading_core::Symbol::new(sym.as_str());
                            let position_curve: Vec<(i64, rust_decimal::Decimal)> = report
                                .position_curve_raw
                                .iter()
                                .filter(|(_, _, s)| s == &active_sym)
                                .map(|&(ts, qty, _)| (ts, qty))
                                .collect();
                            Ok(RunSummary {
                                strategy_id: strat,
                                symbol: sym,
                                report_path: path,
                                equity_series,
                                fills: report.fills.clone(),
                                kpis: report.kpis.clone(),
                                bars: report.bars.clone(),
                                position_curve,
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
                data_source: crate::lab::state::LabDataSource::default(),
                sma_fast_len: None,
                sma_slow_len: None,
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
            data_source: crate::lab::state::LabDataSource::default(),
            sma_fast_len: None,
            sma_slow_len: None,
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
            data_source: crate::lab::state::LabDataSource::default(),
            sma_fast_len: None,
            sma_slow_len: None,
        };
        let sc = lab_config_to_scenario(&cfg).unwrap();
        assert_eq!(sc.seed, seed);
        assert!(sc.write_report);
        assert_eq!(sc.pair.1.to_string(), "XRPUSDT");
    }

    /// T-C3.5 — `LabSelectDataSource` updates `lab_state.data_source`.
    #[test]
    fn lab_select_data_source_updates_state() {
        let mut cockpit = crate::state::Cockpit::default();
        // Default is Synthetic.
        assert_eq!(
            cockpit.lab_state.data_source,
            crate::lab::state::LabDataSource::Synthetic
        );
        // Toggle to YahooCache.
        crate::state::update(
            &mut cockpit,
            crate::state::Message::LabSelectDataSource(
                crate::lab::state::LabDataSource::YahooCache,
            ),
        );
        assert_eq!(
            cockpit.lab_state.data_source,
            crate::lab::state::LabDataSource::YahooCache
        );
        // Toggle back to Synthetic.
        crate::state::update(
            &mut cockpit,
            crate::state::Message::LabSelectDataSource(crate::lab::state::LabDataSource::Synthetic),
        );
        assert_eq!(
            cockpit.lab_state.data_source,
            crate::lab::state::LabDataSource::Synthetic
        );
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
            data_source: crate::lab::state::LabDataSource::default(),
            sma_fast_len: None,
            sma_slow_len: None,
        };
        let (_handle, recv) = cancellation_pair();
        let progress_tx = backtest::progress::ProgressSender::disabled();
        // Should compile and return a Task without panicking.
        let _task = spawn_lab_run(
            #[cfg(feature = "live")]
            None,
            #[cfg(not(feature = "live"))]
            None,
            cfg,
            recv,
            progress_tx,
            // cockpit-activity-status-bar T-D-N7: no EventBus in unit tests.
            #[cfg(feature = "live")]
            None,
            #[cfg(not(feature = "live"))]
            None,
            // lab-recipe-test-harness T-D1: no source override in this unit test.
            #[cfg(feature = "live")]
            None,
            #[cfg(not(feature = "live"))]
            None,
        );
    }
}
