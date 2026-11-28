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

// cockpit-activity-status-bar v0.1.0 T-D-N7 — import ActivityKind for the
// Yahoo preload producer wiring. Its sole consumer is `run_yahoo_preload_arm`
// (story 2-63 Wave F), which is gated on `live` only, so the import is too;
// a `binance`-without-`yahoo` build (`binance` implies `live`) needs it.
// `ActivitySender` is used via the `agent::activity::ActivitySender` path.
#[cfg(feature = "live")] // sole consumer: run_yahoo_preload_arm (live-only)
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
    /// The window to run, typed (bug-log `#230`): the Lab's own `DateRange`, so a `Custom`
    /// range's two bounds travel as two fields and a bound with a time reaches the parse whole.
    /// It replaces `range_label: SmolStr`, which joined them with `:` and was split back on the
    /// first `:`. [`lab_config_to_scenario`] maps it; [`LabRunConfig::range_label`] prints it.
    pub range: crate::lab::state::DateRange,
    /// `ChaCha20` seed per ADR-0030.
    pub seed: [u8; 32],
    /// Write a Markdown report to `evidence/<slug>/reports/…` on completion.
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

impl LabRunConfig {
    /// The window as logs, activity labels and no-data notices print it: the text the
    /// `range_label` field held before bug-log `#230`. Display only; nothing parses it.
    #[must_use]
    pub fn range_label(&self) -> SmolStr {
        use crate::lab::state::{DateRange, Preset};
        use crate::strings as s;
        match &self.range {
            DateRange::Preset(Preset::Last30d) => SmolStr::new_static(s::LAB_RUN_RANGE_LAST30D),
            DateRange::Preset(Preset::Last90d) => SmolStr::new_static(s::LAB_RUN_RANGE_LAST90D),
            DateRange::Preset(Preset::H1_2024) => SmolStr::new_static(s::LAB_RUN_RANGE_H1_2024),
            DateRange::Preset(Preset::H2_2024) => SmolStr::new_static(s::LAB_RUN_RANGE_H2_2024),
            DateRange::Custom { start_raw, end_raw } => {
                SmolStr::new(s::fmt_lab_run_range_custom(start_raw, end_raw))
            }
        }
    }
}

/// The run a Lab `Run` press dispatches (bug-log `#230`; was inline in `cockpit_live`'s
/// `update`). `None` until a strategy and a pair are selected. The range is
/// [`LabState::run_range`](crate::lab::state::LabState::run_range): what the range fields show.
#[must_use]
pub fn lab_run_config(lab: &crate::lab::state::LabState) -> Option<LabRunConfig> {
    let (strategy, (venue, symbol)) = (lab.strategy.as_ref()?, lab.pair.as_ref()?);
    Some(LabRunConfig {
        strategy_id: SmolStr::new(&strategy.0),
        symbol: SmolStr::new(symbol.0.as_str()),
        venue: SmolStr::new(format!("{venue:?}")),
        range: lab.run_range(),
        seed: crate::lab::defaults::LAB_DEFAULT_SEED,
        write_report: true,
        // lab-yahoo-realdata T-C3.6: mirror the current UI data_source selection into the run
        // config so the runner can dispatch accordingly.
        data_source: lab.data_source,
        // lab-polish-round-2 R2: pass operator-tuned SMA windows through. None → the engine
        // falls back to the (20, 50) default.
        sma_fast_len: lab.sma_fast_len,
        sma_slow_len: lab.sma_slow_len,
    })
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

/// Shared preload abstraction over a Lab bar source (Yahoo OR Binance).
///
/// simple-strategies-realdata T-B3 / A3 — the generalization that lets BOTH
/// `LabYahooBarSource` and `LabBinanceBarSource` route through the single
/// `spawn_preload_on_rt` enforcement point (ADR-0050 § D4). It carries the
/// one method the spawn helper needs — `preload` returning a `PreloadFuture`.
///
/// `LabYahooBarSource` and `LabBinanceBarSource` are kept as distinct
/// **marker super-traits** of this base so the two injection seams stay
/// type-distinct and self-documenting, while the spawn helper is generic over
/// the shared `LabBarSource`. Both seams are pub arm functions — the
/// production arms themselves — that tests call directly with a mock:
/// [`run_yahoo_preload_arm`] and [`run_binance_preload_arm`] (review patch 5;
/// story 2-63 Wave F). Yahoo also keeps the `yahoo_source_override`
/// PARAMETER on `spawn_lab_run`, which routes into the same single Yahoo
/// call site; Binance has no such parameter.
///
/// Object-safe via `BoxFuture` return (no async-trait crate needed).
/// Bounded by `Send + Sync + 'static` so the impl can be moved into
/// `iced::Task::perform`'s `Send + 'static` async closure.
///
/// Feature gate: only compiled under `live` so tests with `--features live`
/// can construct fakes without requiring `yahoo` / `binance` flags.
#[cfg(feature = "live")]
pub trait LabBarSource: Send + Sync + 'static {
    /// Preload bars for the given config + scenario range.
    ///
    /// Returns `(bars, revision_sha)` on success, `Err(SmolStr)` on failure.
    fn preload<'a>(
        &'a self,
        cfg: &'a LabRunConfig,
        range: &'a backtest::engine::DateRange,
    ) -> PreloadFuture<'a>;
}

/// Marker trait for the Yahoo bar preload seam in `spawn_lab_run`.
///
/// Allows tests to inject a `MockLabYahooBarSource` without touching HTTP/parquet.
/// Production path uses `DefaultLabYahooBarSource` which delegates to
/// `preload_yahoo_bars` (the existing parquet+http impl).
///
/// simple-strategies-realdata T-B3 — this is now a **pure marker super-trait
/// of [`LabBarSource`]**: the `preload` method lives on `LabBarSource`, and
/// `LabYahooBarSource` only TAGS a type as the Yahoo seam (so the
/// `yahoo_source_override` injection parameter is type-distinct from the
/// Binance seam, [`run_binance_preload_arm`]). Because
/// `dyn LabYahooBarSource: LabBarSource`, the one Yahoo call
/// `spawn_preload_on_rt(rt, source, ..)` inside [`run_yahoo_preload_arm`] —
/// and the `lab_runner_preload_callthrough_e2e` regression guard — coerce
/// `Box<dyn LabYahooBarSource>` into the generalized spawn helper's
/// `Box<S: LabBarSource + ?Sized>` for every Yahoo source.
///
/// Feature gate: only compiled under `live` so tests with `--features live`
/// can construct mocks without requiring the `yahoo` feature flag.
#[cfg(feature = "live")]
pub trait LabYahooBarSource: LabBarSource {}

/// Marker trait for the Binance bar preload seam in `spawn_lab_run`
/// (simple-strategies-realdata T-B3 / A3, sibling to [`LabYahooBarSource`]).
///
/// A pure marker super-trait of [`LabBarSource`]: the `preload` method lives
/// on `LabBarSource`; this trait only TAGS a type as the Binance seam,
/// type-distinct from the Yahoo one. There is NO `binance_source_override`
/// parameter on `spawn_lab_run` (review patch 5 corrected the docs that
/// claimed one) — injection happens by calling the pub production arm
/// [`run_binance_preload_arm`] with any `Box<dyn LabBinanceBarSource>`.
/// Production wires `DefaultLabBinanceBarSource` into that arm; the harness
/// (`crates/ui/tests/spawn_lab_run_binance_harness.rs`) injects a fake
/// `LabBinanceBarSource` (AC8) without touching the real corpus.
///
/// Gated on `live` only (NOT `binance`) so a `--features live` test can build
/// a fake Binance source without the `binance` feature — exactly the pattern
/// `LabYahooBarSource` uses for Yahoo.
#[cfg(feature = "live")]
pub trait LabBinanceBarSource: LabBarSource {}

/// Production `LabBinanceBarSource` implementation — delegates to
/// `preload_binance_bars` (simple-strategies-realdata T-B3).
///
/// Wired by `spawn_lab_run`'s `#[cfg(feature = "binance")]` block into
/// [`run_binance_preload_arm`] (the Binance seam — review patch 5). Only
/// compiled when both `live` and `binance` features are enabled because
/// `preload_binance_bars` itself requires `all(live, binance)`.
///
/// # ADR-0050 § D4 (rt.spawn invariant — inherited)
///
/// `preload_binance_bars` is a pure parquet read (no HTTP, no
/// `spawn_blocking`), so its reactor requirement is weaker than Yahoo's. It is
/// nonetheless routed through the SAME `spawn_preload_on_rt` enforcement point
/// as Yahoo (no second inline `rt.spawn`) — keeping the single-enforcement-
/// point invariant and the `lab_runner_preload_callthrough_e2e` guard
/// meaningful for the unified helper.
#[cfg(all(feature = "live", feature = "binance"))]
pub struct DefaultLabBinanceBarSource;

#[cfg(all(feature = "live", feature = "binance"))]
impl LabBarSource for DefaultLabBinanceBarSource {
    fn preload<'a>(
        &'a self,
        cfg: &'a LabRunConfig,
        range: &'a backtest::engine::DateRange,
    ) -> PreloadFuture<'a> {
        Box::pin(preload_binance_bars(cfg, range))
    }
}

#[cfg(all(feature = "live", feature = "binance"))]
impl LabBinanceBarSource for DefaultLabBinanceBarSource {}

/// Production `LabYahooBarSource` implementation — delegates to `preload_yahoo_bars`.
///
/// Wired by `spawn_lab_run` when no mock is injected (default path).
/// Only compiled when both `live` and `yahoo` features are enabled because
/// `preload_yahoo_bars` itself requires `#[cfg(feature = "yahoo")]`.
///
/// # ADR-0050 § D4 (rt.spawn fix — recurrence #3)
///
/// The `rt` field has been removed. The production path now spawns the entire
/// `preload_yahoo_bars` call onto the tokio runtime via [`spawn_preload_on_rt`]
/// in [`run_yahoo_preload_arm`] (story 2-63 Wave F). Running the
/// future on a tokio worker thread (not iced's `futures::ThreadPool`) guarantees
/// reactor context is present for reqwest's DNS `spawn_blocking` and all
/// `tokio::time::*` calls. See `bug-64-arch-revalidation-rt-spawn-2026-05-29.md`.
#[cfg(all(feature = "live", feature = "yahoo"))]
pub struct DefaultLabYahooBarSource;

#[cfg(all(feature = "live", feature = "yahoo"))]
impl LabBarSource for DefaultLabYahooBarSource {
    fn preload<'a>(
        &'a self,
        cfg: &'a LabRunConfig,
        range: &'a backtest::engine::DateRange,
    ) -> PreloadFuture<'a> {
        Box::pin(preload_yahoo_bars(cfg, range))
    }
}

#[cfg(all(feature = "live", feature = "yahoo"))]
impl LabYahooBarSource for DefaultLabYahooBarSource {}

// ── Preload spawn helper (ADR-0050 § D4 / T-BUG64-CT1) ───────────────────────

/// Spawn a `LabYahooBarSource::preload` call onto a tokio worker thread.
///
/// **ADR-0050 § D4 — durable invariant (Bug #64 recurrence #3):**
/// Any future that calls `tokio::task::spawn_blocking` (reqwest/hyper DNS,
/// `fetch_and_cache`, etc.) MUST run on a tokio worker thread. Calling such
/// a future from `futures::executor::block_on` (iced's `futures::ThreadPool`)
/// without `rt.spawn()` wrapping panics: "there is no reactor running".
///
/// This function IS the single `rt.spawn()` enforcement point. ALL preload
/// paths route their preload call through here:
/// - the Yahoo path, [`run_yahoo_preload_arm`], for every Yahoo source (an
///   override, or `DefaultLabYahooBarSource` under `#[cfg(feature =
///   "yahoo")]`; story 2-63 Wave F),
/// - and (simple-strategies-realdata T-B3) the Binance path — production AND
///   mock alike via [`run_binance_preload_arm`] (`DefaultLabBinanceBarSource`
///   under `#[cfg(feature = "binance")]`; there is no
///   `binance_source_override` parameter — review patch 5).
///
/// This invariant is structural: adding a second inline `rt.spawn` at any
/// production site is the regression pattern that caused Bug #64 recurrences
/// #1–#3. Keep every call site going through this function (T-BUG64-UN1).
///
/// # Generalization (simple-strategies-realdata T-B3 / A3)
///
/// The helper is generic over `S: LabBarSource + ?Sized` so BOTH
/// `Box<dyn LabYahooBarSource>` and `Box<dyn LabBinanceBarSource>` coerce in
/// (each `dyn` trait is a `LabBarSource` super-trait object). The `rt.spawn`
/// body is unchanged — the single enforcement point is preserved, not
/// duplicated. The Binance future does NOT call `spawn_blocking` (pure
/// parquet read, no HTTP), so its reactor requirement is weaker than Yahoo's;
/// routing it through the same point keeps the code symmetric AND keeps the
/// regression guard below meaningful for the unified helper.
///
/// # Regression guard (T-BUG64-CT1)
///
/// `crates/ui/tests/lab_runner_preload_callthrough_e2e.rs` calls this function
/// directly under `futures::executor::block_on` with a `SpawnBlockingFakeSource`
/// that calls `tokio::task::spawn_blocking` in its `preload()` impl. Replacing
/// `rt.spawn(...)` with a direct `.await` in this function causes that test to
/// panic with "there is no reactor running". The test is the regression gate;
/// the generic signature change keeps that test compiling (its
/// `Box<dyn LabYahooBarSource>` coerces to `Box<S>` via the super-trait).
///
/// # Returns
///
/// A `JoinHandle` to be polled/awaited by the caller. The caller is responsible
/// for handling `JoinError` (panicked or aborted task) and the inner
/// `Result<(Vec<Bar>, SmolStr), SmolStr>`.
#[cfg(feature = "live")]
#[must_use = "JoinHandle must be awaited or aborted; dropping detaches the task"]
pub fn spawn_preload_on_rt<S: LabBarSource + ?Sized>(
    rt: &tokio::runtime::Handle,
    source: Box<S>,
    cfg: LabRunConfig,
    range: backtest::engine::DateRange,
) -> tokio::task::JoinHandle<Result<(Vec<trading_core::Bar>, SmolStr), SmolStr>> {
    // INVARIANT (ADR-0050 § D4): do NOT change this to a direct `.await`.
    // The source.preload() future may call tokio::task::spawn_blocking
    // (reqwest DNS, fetch_and_cache, etc.). That requires a tokio reactor on
    // the polling thread. rt.spawn() guarantees the future runs on a tokio
    // worker thread regardless of what executor called spawn_preload_on_rt.
    rt.spawn(async move { source.preload(&cfg, &range).await })
}

// ── Yahoo bar pre-loading helpers (lab-yahoo-realdata T-C3.6 / T-AR1) ────────

/// Map a `backtest::engine::DateRange` to `(start_ms, end_ms)` UTC epoch-millis.
///
/// `H1_2024` / `H2_2024` use fixed calendar boundaries so they are deterministic.
/// `Last30d` / `Last90d` use wall-clock `now()` — intentional; these are the
/// operator's "show me recent data" presets for Yahoo real bars (H1 hypothesis).
/// `Custom` passes the caller's values through unchanged.
///
/// Note: `time::OffsetDateTime::now_utc()` is only reachable when
/// `data_source == YahooCache` (rolling presets are date-relative by design
/// for real-data); the synthetic path never calls this function.
///
/// Exposed as `pub` for integration tests (`lab_yahoo_range_clamp.rs`).
/// Internal use only — not part of the stable API surface.
#[cfg(feature = "yahoo")]
#[must_use]
pub fn range_to_ms_pair(range: &backtest::engine::DateRange) -> (i64, i64) {
    use backtest::engine::DateRange;
    const MS_PER_DAY: i64 = 86_400_000;
    let now_ms = time::OffsetDateTime::now_utc().unix_timestamp() * 1_000;
    let (start_ms, end_ms) = match range {
        DateRange::Last30d => (now_ms - 30 * MS_PER_DAY, now_ms),
        DateRange::Last90d => (now_ms - 90 * MS_PER_DAY, now_ms),
        DateRange::H1_2024 => (1_704_067_200_000, 1_719_792_000_000), // 2024-01-01 .. 2024-07-01 UTC
        DateRange::H2_2024 => (1_719_792_000_000, 1_735_689_600_000), // 2024-07-01 .. 2025-01-01 UTC
        DateRange::Custom { start_ms, end_ms } => (*start_ms, *end_ms),
    };
    // lab-yahoo-empty-range-ux v0.1.0 — D-ER-2 (Q2=(a) clamp / M-DEV.6):
    // Clamp end_ms to now when future-dated. Applies ONLY when end_ms > now_ms
    // (K3: H1_2024/H2_2024/past Custom ranges are provably < now_ms and pass
    // through byte-identical; Last30d/Last90d already set end_ms = now_ms so
    // the clamp is a no-op for them too). start_ms is NEVER clamped.
    let end_ms = end_ms.min(now_ms);
    (start_ms, end_ms)
}

/// Pre-load Yahoo bars upstream of engine dispatch (T-AR1 / Q1 = (b)).
///
/// Called by `spawn_lab_run` when `cfg.data_source == YahooCache`.
/// Converts the UI Binance-style symbol to Yahoo-native at the dispatch
/// boundary (Q6 = (a) / D7), derives the adaptive cadence (Q4 = (c) / D6),
/// and returns `(bars, revision_sha)`. The revision SHA is verified at load
/// and logged HERE; `spawn_lab_run` then drops it (`Ok((bars, _sha))`) — it
/// is NOT carried into reports (review patch 13; the engine-path SMA report
/// writer passes `rev_sha: None`). Binance-symmetric.
///
/// # Errors
///
/// Returns `Err(SmolStr)` with an operator-friendly message on:
/// - Unknown ticker (`UnmappedTicker`)
/// - Cache miss (`CacheMiss`) — includes a `cargo run` hint
/// - Coverage below 95% (`MissingData`)
/// - Revision manifest missing or tampered (`RevisionMissing` / `RevisionMismatch`)
///
/// # ADR-0050 § D4 (rt.spawn fix — recurrence #3)
///
/// This function no longer accepts an `rt: &Handle` parameter. The CALLER
/// (`spawn_lab_run`) is responsible for spawning this future onto the tokio
/// runtime via `rt.spawn(async move { preload_yahoo_bars(...).await })`.
/// Running on a tokio worker thread guarantees reactor context for reqwest's
/// DNS `spawn_blocking` and for `tokio::time::*` calls inside
/// `fetch_with_backoff`. The prior `rt.enter()` guards inside
/// `fetch_with_backoff` are removed — they are redundant (and were
/// insufficient) once the task runs on-runtime.
/// See `bug-64-arch-revalidation-rt-spawn-2026-05-29.md § 1-2`.
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
            // Note: no `rt` parameter — we run on-runtime (tokio worker thread),
            // so reactor context is guaranteed. See ADR-0050 § D4.
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
                // lab-yahoo-empty-range-ux v0.1.0 — D-ER-1 (M-DEV.4):
                // NoDataForRange is the typed K1-correct signal from fetch_and_cache
                // (HTTP-200 + 0 quotes). Build the sentinel-tagged notice instead of
                // the generic "Check network" message — this routes to
                // last_run_notice (muted) rather than last_run_error (red ⚠).
                Err(YahooError::NoDataForRange {
                    ticker: t,
                    start_label,
                    end_label,
                }) => {
                    tracing::info!(
                        target: "lab.yahoo",
                        ticker = %t,
                        start = %start_label,
                        end = %end_label,
                        "Yahoo returned no data for range (expected — future-dated or delisted)"
                    );
                    return Err(preload_notice::no_data_message(
                        &t,
                        &start_label,
                        &end_label,
                    ));
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

// ── Binance bar pre-loading (simple-strategies-realdata T-B3 / A3) ───────────

/// Pinned root of the Binance hourly parquet corpus (ADR-0032).
///
/// Layout: `data/binance/<SYM>USDT/<YEAR>/<MM>.parquet`, `interval = "1h"`,
/// revision `3a8b96c4…`. Bulk parquets gitignored + manually re-fetchable
/// (only `data/binance/REVISION.toml` is tracked); NO auto-fetch.
///
/// Gate note (review patch 10): all four Binance loader items are gated
/// `all(live, binance)` — their only callers live inside `spawn_lab_run`'s
/// `#[cfg(feature = "live")]` block, so a `--no-default-features --features
/// binance` build (no `live`) would otherwise compile them dead and fail
/// `-D warnings` on `dead_code`.
#[cfg(all(feature = "live", feature = "binance"))]
const BINANCE_CORPUS_ROOT: &str = "data/binance";

/// Pinned aggregate revision SHA of the Binance hourly corpus (ADR-0032,
/// pinned 2026-05-18 — aggregate over all 240 hourly parquets, 10 symbols ×
/// 24 months, 2023-01-01 .. 2024-12-31). Mirrors the CLI pin literal in
/// `crates/backtest/src/main.rs` (`expected_revision_sha` of the realdata
/// scenarios). The Lab loader asserts the on-disk aggregate equals THIS pin
/// (feature-AC3 pin-assert clause, review patch 3) — manifest
/// self-consistency alone would accept a consistently re-fetched-divergent
/// corpus.
#[cfg(all(feature = "live", feature = "binance"))]
const BINANCE_PINNED_REVISION_SHA: &str =
    "3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7";

/// Pinned-corpus span start: 2023-01-01T00:00:00Z (ADR-0032).
#[cfg(all(feature = "live", feature = "binance"))]
const BINANCE_CORPUS_SPAN_START_MS: i64 = 1_672_531_200_000;

/// Pinned-corpus span end (exclusive): 2025-01-01T00:00:00Z (ADR-0032).
#[cfg(all(feature = "live", feature = "binance"))]
const BINANCE_CORPUS_SPAN_END_MS: i64 = 1_735_689_600_000;

/// Map a `backtest::engine::DateRange` to `(start_ms, end_ms)` UTC epoch-millis
/// for the Binance corpus (simple-strategies-realdata A3).
///
/// Mirrors `range_to_ms_pair` (the Yahoo mapper) but lives behind its own
/// gate so the two features are independent. The fixed-calendar presets are
/// deterministic; `Last30d` / `Last90d` are wall-clock-relative; `Custom`
/// passes through. The corpus spans 2023-01 .. 2024-12, so a `Custom` 2023
/// range or `H1_2024` / `H2_2024` all resolve to on-disk months; windows with
/// NO intersection with that span (e.g. the rolling presets once the wall
/// clock passes 2025-03) are caught by the out-of-span early check in
/// `preload_binance_bars` (review patch 2).
#[cfg(all(feature = "live", feature = "binance"))]
#[must_use]
fn binance_range_to_ms_pair(range: &backtest::engine::DateRange) -> (i64, i64) {
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

/// Pre-load pinned Binance **hourly** bars upstream of engine dispatch
/// (simple-strategies-realdata T-B3 / R2 / R6 / Q-tf / Q-miss).
///
/// Called by `spawn_lab_run` when `cfg.data_source == BinanceCache` (the
/// production path via `DefaultLabBinanceBarSource`). It:
///
/// 1. Rejects windows with NO intersection with the pinned corpus span
///    (2023-01 .. 2024-12) up front with an honest pick-another-range notice
///    — re-fetching cannot extend a PINNED corpus (review patch 2).
/// 2. Verifies the on-disk manifest via
///    `data::revision::read_and_verify_revision_manifest("data/binance")` AND
///    asserts the recomputed aggregate equals the pin
///    [`BINANCE_PINNED_REVISION_SHA`] (review patch 3, mirroring the CLI) — a
///    tampered / re-fetched-divergent corpus fails loudly (R6 / AC3), never
///    producing a silently-wrong report. The verified SHA is returned as the
///    second tuple element to satisfy the `LabBarSource::preload` contract
///    and is logged here; `spawn_lab_run` then DROPS it — it is NOT carried
///    into reports or any further plumbing (review patch 13; the engine-path
///    SMA report writer passes `rev_sha: None`).
/// 3. Reads single-symbol bars at `Timeframe::OneHour` via
///    `data::ReplayFeed::merge_symbols(&[(sym, root)], OneHour)` (the exact
///    timeframe-parametric read `RealDataBarSource` uses; the CLI's 1m
///    single-symbol auto-detect path is deliberately NOT reused — Q-tf).
/// 4. Clips to the selected range `[start_ms, end_ms)`.
///
/// # Timeframe (Q-tf)
///
/// Bars are HOURLY. The four simple strategies are cadence-agnostic (they
/// consume a `Bar` stream); SMA 20/50 windows are bar-counts, so on the hourly
/// series they mean 20h/50h — a legitimate hourly strategy. The engine has NO
/// timeframe field; cadence is fixed here at load (A2).
///
/// # Errors (Q-miss — NEVER a silent synthetic fallback)
///
/// Returns `Err(SmolStr)` with an operator-friendly message on:
/// - Revision manifest missing / mismatched (`LAB_BINANCE_REVISION_ERROR`).
/// - Cache miss / no parquet for the symbol / zero bars in range
///   (`LAB_BINANCE_CACHE_MISS_NOTICE` with a re-fetch hint pointing at the
///   offline fetch tool — Binance is pinned + manually re-fetchable, ADR-0032;
///   there is NO in-Lab auto-fetch, unlike Yahoo).
///
/// It NEVER synthesizes bars on miss — the design-side half of the AC4
/// no-op-source guard (a silent synthetic fallback would let the operator
/// believe they are testing real BTC while seeing a random walk).
///
/// # ADR-0050 § D4
///
/// No `rt` parameter and no `spawn_blocking` — this is a pure parquet read.
/// The CALLER (`spawn_lab_run`) routes this future through
/// `spawn_preload_on_rt` (the single rt.spawn enforcement point) for symmetry
/// with the Yahoo path.
///
/// `clippy::unused_async`: the `async` signature is intentional — it is the
/// `LabBarSource::preload` boxed-future contract `DefaultLabBinanceBarSource`
/// pins, symmetric with `preload_yahoo_bars`. The body is currently
/// synchronous (a pure parquet read), but keeping it `async` (a) lets it route
/// through the same `spawn_preload_on_rt` enforcement point and (b) leaves room
/// for a future on-demand re-fetch path (mirroring Yahoo) without a
/// signature-breaking change. The await-free body is correct, not a smell.
#[cfg(all(feature = "live", feature = "binance"))]
#[allow(clippy::unused_async)]
async fn preload_binance_bars(
    cfg: &LabRunConfig,
    scenario_range: &backtest::engine::DateRange,
) -> Result<(Vec<trading_core::Bar>, SmolStr), SmolStr> {
    use trading_core::{Symbol, Timeframe};

    let root = std::path::PathBuf::from(BINANCE_CORPUS_ROOT);
    let (start_ms, end_ms) = binance_range_to_ms_pair(scenario_range);

    // Step 1 (review patch 2): out-of-corpus-span early check. When the
    // requested window's intersection with the pinned span (2023-01-01 ..
    // 2025-01-01) is EMPTY — e.g. Last30d/Last90d once the wall clock passes
    // 2025-03 — a re-fetch hint would misdirect the operator (the corpus is
    // PINNED; re-fetching cannot extend it). Emit the honest pick-another-
    // range notice instead, on the amber notice channel.
    let intersection_empty =
        end_ms.min(BINANCE_CORPUS_SPAN_END_MS) <= start_ms.max(BINANCE_CORPUS_SPAN_START_MS);
    if intersection_empty {
        tracing::info!(
            target: "lab.binance",
            symbol = %cfg.symbol,
            range = %cfg.range_label(),
            start_ms,
            end_ms,
            "requested window has no overlap with the pinned 2023-2024 corpus — \
             emitting out-of-span notice (NOT a re-fetch hint)"
        );
        let window = format_ms_window(start_ms, end_ms);
        return Err(SmolStr::new(format!(
            "{}{}",
            preload_notice::NO_DATA_TAG,
            crate::strings::LAB_BINANCE_OUT_OF_SPAN_NOTICE
                .replace("{symbol}", cfg.symbol.as_str())
                .replace("{window}", &window)
        )));
    }

    // Step 2 (review patch 3): verify the manifest AND assert the pin.
    // `read_and_verify_revision_manifest` proves the on-disk corpus is
    // self-consistent with its own manifest; comparing the recomputed
    // aggregate against `BINANCE_PINNED_REVISION_SHA` proves it is THE pinned
    // corpus (mirrors the CLI's `expected_revision_sha` assert in
    // `crates/backtest/src/main.rs`). Loud Err on missing/mismatch — the
    // corpus IS the determinism contract (ADR-0032 / R6).
    let revision_sha = data::revision::read_and_verify_revision_manifest(&root).map_err(|e| {
        tracing::warn!(
            target: "lab.binance",
            error = %e,
            "Binance revision check failed"
        );
        SmolStr::new(crate::strings::LAB_BINANCE_REVISION_ERROR.replace("{detail}", &e.to_string()))
    })?;
    if revision_sha != BINANCE_PINNED_REVISION_SHA {
        tracing::warn!(
            target: "lab.binance",
            on_disk = %revision_sha,
            pinned = BINANCE_PINNED_REVISION_SHA,
            "Binance corpus aggregate SHA does not match the pinned revision"
        );
        return Err(SmolStr::new(
            crate::strings::LAB_BINANCE_REVISION_ERROR.replace(
                "{detail}",
                &format!(
                    "data revision mismatch: pinned {BINANCE_PINNED_REVISION_SHA} \
                     but on-disk computed {revision_sha}"
                ),
            ),
        ));
    }

    // Step 3: read single-symbol HOURLY bars (Q-tf). merge_symbols over a
    // single-element slice mirrors RealDataBarSource exactly and returns a
    // plain Vec<Bar> (no async stream plumbing). FeedError::Io on a missing
    // symbol dir maps to the operator-friendly cache-miss notice (Q-miss).
    let sym = Symbol::new(cfg.symbol.as_str());
    let feed = data::ReplayFeed::new(&root, true);
    let symbol_paths = [(sym.clone(), root.clone())];

    let bars = match feed.merge_symbols(&symbol_paths, Timeframe::OneHour) {
        Ok(mut bars) => {
            // Step 4: clip to the selected range [start_ms, end_ms).
            bars.retain(|b| {
                let ts_ms = b.open_ts.unix_millis();
                ts_ms >= start_ms && ts_ms < end_ms
            });
            bars
        }
        Err(e) => {
            tracing::info!(
                target: "lab.binance",
                symbol = %cfg.symbol,
                error = %e,
                "Binance corpus read failed — emitting cache-miss notice (NO synthetic fallback)"
            );
            return Err(binance_cache_miss_notice(
                cfg.symbol.as_str(),
                scenario_range,
            ));
        }
    };

    // A real symbol dir that yielded ZERO bars in the requested window is a
    // coverage shortfall — same operator action (re-fetch / widen range), so
    // surface the cache-miss notice rather than feeding an empty bars_override
    // to the engine (which would silently produce a zero-equity run).
    if bars.is_empty() {
        tracing::info!(
            target: "lab.binance",
            symbol = %cfg.symbol,
            range = %cfg.range_label(),
            "Binance corpus has 0 bars in range — emitting cache-miss notice"
        );
        return Err(binance_cache_miss_notice(
            cfg.symbol.as_str(),
            scenario_range,
        ));
    }

    tracing::info!(
        target: "lab.binance",
        symbol = %cfg.symbol,
        range = %cfg.range_label(),
        bars = bars.len(),
        revision_sha = %revision_sha,
        "Binance hourly bars ready for Lab run"
    );

    Ok((bars, SmolStr::new(revision_sha)))
}

/// Render a `[start_ms, end_ms)` epoch-millis window as
/// `YYYY-MM-DD..YYYY-MM-DD` (review patch 6 — the old label printed raw
/// days-since-epoch integers, negative for pre-1970 Custom bounds).
///
/// Pre-1970 (negative) millis are valid `time` inputs and render as real
/// dates (e.g. `1969-12-31`); only values outside `time`'s representable
/// range (± year 9999) fall back to a labeled raw-millis form — never a
/// panic.
#[cfg(all(feature = "live", feature = "binance"))]
fn format_ms_window(start_ms: i64, end_ms: i64) -> String {
    let fmt_one = |ms: i64| -> String {
        time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000)
            .map_or_else(|_| format!("{ms}ms(raw)"), |dt| dt.date().to_string())
    };
    format!("{}..{}", fmt_one(start_ms), fmt_one(end_ms))
}

/// Build the operator-friendly Binance cache-miss notice with a
/// `YYYY-MM-DD..YYYY-MM-DD` `{window}` label derived from the resolved range
/// (simple-strategies-realdata Q-miss; review patch 6).
///
/// The notice is [`preload_notice::NO_DATA_TAG`]-tagged (review patch 11) so
/// it routes to the amber `last_run_notice` channel like Yahoo's K1 no-data
/// notice — a missing/short pinned corpus is an expected, actionable state,
/// not a red-⚠ engine failure. (The revision-check failure stays an untagged
/// hard error — a tampered corpus IS alarming.)
#[cfg(all(feature = "live", feature = "binance"))]
fn binance_cache_miss_notice(symbol: &str, range: &backtest::engine::DateRange) -> SmolStr {
    let (start_ms, end_ms) = binance_range_to_ms_pair(range);
    let window = format_ms_window(start_ms, end_ms);
    let body = crate::strings::LAB_BINANCE_CACHE_MISS_NOTICE
        .replace("{symbol}", symbol)
        .replace("{window}", &window);
    SmolStr::new(format!("{}{}", preload_notice::NO_DATA_TAG, body))
}

/// Exponential-backoff retry wrapper around `YahooBarSource::fetch_and_cache`.
/// Mirrors the CLI binary's `fetch_with_backoff` so the in-flight auto-fetch
/// path is equivalent. 5 retries, 1s → 60s cap.
///
/// # ADR-0050 § D4 (rt.spawn fix — recurrence #3)
///
/// The `rt: &Handle` parameter and per-line `rt.enter()` guards that existed
/// in the prior hotfix (`bug-64-d11-attempt-3`) have been REMOVED. They were
/// both insufficient and redundant:
///
/// - **Insufficient**: `rt.enter()` sets a thread-local that is dropped at
///   the first `.await`. reqwest's DNS resolver (`GaiResolver`) calls
///   `tokio::task::spawn_blocking` lazily INSIDE the awaited future, long
///   after the guard is dropped. That panicked on recurrence #3.
/// - **Redundant**: This function is now always called from inside a future
///   spawned via `rt.spawn()` (see `spawn_lab_run`). Spawned tasks run on
///   tokio worker threads which carry reactor + time-driver context. Every
///   `tokio::time::*` call and every `spawn_blocking` (including reqwest DNS)
///   finds a runtime context unconditionally.
///
/// See `bug-64-arch-revalidation-rt-spawn-2026-05-29.md § 1-3`.
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
    //
    // No rt.enter() guard needed — this function runs on a tokio worker
    // thread (spawned by rt.spawn() in spawn_lab_run). Reactor context
    // is guaranteed. See ADR-0050 § D4.
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
                // lab-yahoo-empty-range-ux v0.1.0 — D-ER-1 (M-DEV.4 / K1):
                // NoDataForRange is a terminal, non-transient outcome — retrying
                // burns the 5×60s budget on a window that will never have data.
                // Return immediately without consuming any retry slot.
                //
                // This arm is intentionally kept explicit (not merged into the
                // catch-all below) to document the non-retry decision at the
                // exact point it matters; the bodies are identical by design.
                #[allow(clippy::match_same_arms)]
                Err(e @ YahooError::NoDataForRange { .. }) => return Err(e),
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

/// One bound of a Lab `Custom` range, in epoch ms: full RFC 3339, or a date-only `YYYY-MM-DD`
/// read as midnight UTC. The only reader of a Custom bound (bug-log `#230`): the run mapper and
/// the report lookup (through [`custom_range_ms`]), the Custom editor's colour
/// (`widgets::date_range`) and commit rule (`LabState::classify_custom_edit`), and the cadence
/// badge.
///
/// # Errors
///
/// `Err("invalid custom date '<raw>': …")`, naming the bound whole, as typed.
pub(crate) fn custom_bound_ms(raw: &str) -> Result<i64, SmolStr> {
    // Accept full RFC 3339 or date-only "YYYY-MM-DD".
    let dt = time::OffsetDateTime::parse(raw, &time::format_description::well_known::Rfc3339)
        .or_else(|_| {
            // Try date-only: append T00:00:00Z
            let padded = format!("{raw}T00:00:00Z");
            time::OffsetDateTime::parse(&padded, &time::format_description::well_known::Rfc3339)
        })
        .map_err(|e| SmolStr::new(format!("invalid custom date '{raw}': {e}")))?;
    Ok(dt.unix_timestamp() * 1000)
}

/// A Lab `Custom` range's epoch-millisecond bounds, each read by [`custom_bound_ms`]. The one
/// parse the run mapper ([`lab_config_to_scenario`]) and the Lab's report lookup
/// (`lab::equity_loader`, which matches a report's `custom<start_ms>to<end_ms>` token against
/// it, bug-log #212) share.
///
/// # Errors
///
/// `Err("invalid custom date '<raw>': …")` when either bound parses as neither, naming that
/// bound whole (the start's error first).
pub(crate) fn custom_range_ms(start_raw: &str, end_raw: &str) -> Result<(i64, i64), SmolStr> {
    Ok((custom_bound_ms(start_raw)?, custom_bound_ms(end_raw)?))
}

/// Map a `LabRunConfig` to a `backtest::ScenarioConfig`.
///
/// The typed `range` (bug-log `#230`) maps preset for preset onto
/// `backtest::engine::DateRange`; a `Custom` range's two bounds are parsed to epoch-milliseconds
/// by [`custom_range_ms`], each bound whole, and the pair must increase.
///
/// # Errors
///
/// - A `Custom` bound that parses as neither RFC 3339 nor `YYYY-MM-DD` →
///   `Err("invalid custom date '<raw>': …")`, naming the bound as typed.
/// - A `Custom` pair whose start is not before its end →
///   `Err(strings::fmt_lab_custom_range_not_increasing(..))`, naming both bounds as typed. Every
///   source's run maps here before any preload, so this is the one order rule.
pub fn lab_config_to_scenario(cfg: &LabRunConfig) -> Result<backtest::ScenarioConfig, SmolStr> {
    use crate::lab::state::{DateRange as LabRange, Preset};
    use backtest::engine::DateRange;
    use trading_core::StrategyId;

    let range = match &cfg.range {
        LabRange::Preset(Preset::Last30d) => DateRange::Last30d,
        LabRange::Preset(Preset::Last90d) => DateRange::Last90d,
        LabRange::Preset(Preset::H1_2024) => DateRange::H1_2024,
        LabRange::Preset(Preset::H2_2024) => DateRange::H2_2024,
        // bug-log #230: two fields, so `2024-01-01T00:00:00Z` reaches the parse whole.
        LabRange::Custom { start_raw, end_raw } => {
            let (start_ms, end_ms) = custom_range_ms(start_raw, end_raw)?;
            // bug-log #230 review: the one order rule, for every source and before any preload
            // (a Binance or Yahoo preload would otherwise answer a reversed pair with a
            // misleading no-data notice; the engine's `InvalidRange` is reached only on
            // Synthetic). Each bound still commits on its own parse; Run refuses the pair here.
            if start_ms >= end_ms {
                return Err(SmolStr::new(
                    crate::strings::fmt_lab_custom_range_not_increasing(start_raw, end_raw),
                ));
            }
            DateRange::Custom { start_ms, end_ms }
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
        // lab-run-save-compare ADR-0055 § D3 — Lab caller supplies the
        // lab-runs root; None falls back to the engine's workspace-root default.
        reports_dir: None,
        // ADR-0068 D1: Lab UI does not expose short_enabled yet; always long-only.
        // The ui-designer will wire this in T-U* tasks.
        short_enabled: false,
        // leaderboard-timeframe-capital knobs: Lab path always uses 100_000 default.
        initial_capital: None,
        // Lab UI path always loads strategy from disk — no in-memory TOML override.
        composed_toml_override: None,
        // Lab UI path does not supply DVOL data — DVOL arm not available via Lab.
        dvol_override: None,
        macro_regime_series: None,
    })
}

/// Shared helper — classify a zero-bar preload success as a no-data notice.
///
/// lab-yahoo-empty-range-ux v0.1.0 — D-ER-1 step 2 / M-DEV.5 (Caution #2).
///
/// Called from BOTH the mock-path arm AND the production Yahoo arm of
/// `spawn_lab_run`'s `preload_result` match. A single shared helper ensures
/// both paths apply the `bars.is_empty()` → no-data routing identically.
///
/// # Decision
///
/// - `Ok((bars, sha))` where `bars.is_empty()` → the ticker name is not
///   available at this call site; use a generic "no data returned" notice.
///   The `preload_yahoo_bars` path will have already returned a `NoDataForRange`-
///   tagged message before reaching here (real Yahoo path). For the mock path
///   (test injection), the mock returns `Ok((vec![], sha))` directly, so we
///   must handle it here.
/// - `Ok((bars, sha))` where `!bars.is_empty()` → `Ok(Some((bars, sha)))`.
/// - `Err(e)` → `Err(e)` pass-through.
///
/// Returns `Err` (either the original error OR a tagged no-data notice) when
/// the result should short-circuit the run; `Ok(Some(...))` when bars are ready;
/// `Ok(None)` never (reserved for future use).
#[cfg(feature = "live")]
fn classify_preload_result(
    preload_result: Result<(Vec<trading_core::Bar>, SmolStr), SmolStr>,
    cfg: &LabRunConfig,
) -> Result<(Vec<trading_core::Bar>, SmolStr), SmolStr> {
    match preload_result {
        Ok((bars, _sha)) if bars.is_empty() => {
            // Zero bars on a successful preload — either the mock returned empty
            // or the real preload path returned a no-data condition that was
            // caught upstream and converted. If somehow an empty success slipped
            // through, build a notice now to avoid feeding an empty bars_override
            // to the engine (which would silently produce a zero-equity run).
            //
            // Review patch 11: the copy + tracing target are SOURCE-aware — a
            // Binance run must not ship Yahoo-branded copy under a `lab.yahoo`
            // target (tracing targets must be literals, hence two macro arms).
            let window = cfg.range_label();
            let body = if cfg.data_source == crate::lab::state::LabDataSource::BinanceCache {
                tracing::info!(
                    target: "lab.binance",
                    symbol = %cfg.symbol,
                    range = %cfg.range_label(),
                    "preload returned 0 bars — emitting no-data notice"
                );
                crate::strings::LAB_BINANCE_OUT_OF_SPAN_NOTICE
                    .replace("{symbol}", cfg.symbol.as_str())
                    .replace("{window}", &window)
            } else {
                tracing::info!(
                    target: "lab.yahoo",
                    symbol = %cfg.symbol,
                    range = %cfg.range_label(),
                    "preload returned 0 bars — emitting no-data notice"
                );
                crate::strings::LAB_YAHOO_NO_DATA_NOTICE
                    .replace("{ticker}", cfg.symbol.as_str())
                    .replace("{window}", &window)
            };
            Err(SmolStr::new(format!(
                "{}{}",
                preload_notice::NO_DATA_TAG,
                body
            )))
        }
        other => other,
    }
}

/// Run the Binance preload arm of `spawn_lab_run` against an injectable
/// [`LabBinanceBarSource`] — THE Binance test seam (simple-strategies-
/// realdata review patch 5).
///
/// This IS the production glue, not a copy: `spawn_lab_run`'s
/// `#[cfg(feature = "binance")]` block calls this exact function with
/// `Box::new(DefaultLabBinanceBarSource)`, and the harness
/// (`crates/ui/tests/spawn_lab_run_binance_harness.rs`) calls it with a mock
/// `LabBinanceBarSource`. There is NO `binance_source_override` parameter on
/// `spawn_lab_run` — Binance injection happens by exercising this seam
/// directly. The Yahoo seam is [`run_yahoo_preload_arm`] (story 2-63 Wave F);
/// Yahoo additionally keeps a `yahoo_source_override` parameter, through which
/// tests drive `spawn_lab_run`'s Task with
/// `iced_test::runtime::task::into_stream`.
///
/// Behaviour (mirrors the Yahoo arm):
/// 1. Emit the `Progress { 0, 1, 0 }` sentinel BEFORE the preload await so
///    the progress label ticks rather than sitting static.
/// 2. Route the preload through [`spawn_preload_on_rt`] — the single
///    `rt.spawn` enforcement point (ADR-0050 § D4; no second inline
///    `rt.spawn`).
/// 3. Classify via [`classify_preload_result`] (zero-bars → tagged
///    source-appropriate no-data notice; review patch 11).
/// 4. On success, set `scenario_cfg.data_source = BinanceCache` and
///    `scenario_cfg.bars_override = Some(bars)`. The loader-verified revision
///    SHA is dropped here — verified + logged at load, NOT carried into
///    reports (review patch 13).
///
/// Gated on `live` only (NOT `binance`) so a `--features live` harness can
/// inject a fake source without the real-corpus feature — the same pattern
/// as the Yahoo mock path.
///
/// # Errors
///
/// Returns the classified preload error (tagged amber notice or hard error)
/// when the run must short-circuit; `scenario_cfg` is left untouched in that
/// case.
#[cfg(feature = "live")]
pub async fn run_binance_preload_arm(
    rt: &tokio::runtime::Handle,
    source: Box<dyn LabBinanceBarSource>,
    cfg: &LabRunConfig,
    scenario_cfg: &mut backtest::ScenarioConfig,
    progress_tx: &backtest::progress::ProgressSender,
) -> Result<(), SmolStr> {
    // Sentinel emission BEFORE preload (mirrors the Yahoo path).
    progress_tx.try_send(backtest::progress::Progress {
        current_bar: 0,
        total_bars: 1,
        elapsed_ms: 0,
    });

    let cfg_for_spawn = cfg.clone();
    let range_for_spawn = scenario_cfg.range.clone();
    let preload_result = match spawn_preload_on_rt(rt, source, cfg_for_spawn, range_for_spawn).await
    {
        Ok(inner) => inner,
        Err(join_err) => Err(SmolStr::new(format!(
            "binance preload join error: {join_err}"
        ))),
    };
    // classify_preload_result guards the empty-bars case (defence-in-depth;
    // preload_binance_bars already returns Err on zero bars, so this is
    // belt-and-braces symmetry with the Yahoo path).
    match classify_preload_result(preload_result, cfg) {
        Ok((bars, _sha)) => {
            scenario_cfg.data_source = backtest::engine::ScenarioDataSource::BinanceCache;
            scenario_cfg.bars_override = Some(bars);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// Run the Yahoo preload arm of `spawn_lab_run` against an injectable
/// [`LabYahooBarSource`] — THE Yahoo test seam (story 2-63 Wave F; mirrors
/// [`run_binance_preload_arm`]).
///
/// This IS the production glue, not a copy: `spawn_lab_run` calls this exact
/// function once, with `yahoo_source_override` or, when that is `None`,
/// `Box::new(DefaultLabYahooBarSource)`. The harnesses
/// (`spawn_lab_run_yahoo_harness.rs`, `lab_runner_ticker_e2e.rs`,
/// `lab_runner_cancel_e2e.rs`) call it with a mock source from a thread with
/// no tokio context, which is how iced's `futures::ThreadPool` polls it.
///
/// Behaviour:
/// 1. Emit the `Progress { 0, 1, 0 }` sentinel before any await.
/// 2. Build a 250 ms ticker under `rt.enter()`. Without the guard,
///    `tokio::time::interval` panics on the polling thread (ADR-0050 § D1).
/// 3. Start the `YahooPreload` activity when an `ActivitySender` is wired.
/// 4. Spawn the preload via [`spawn_preload_on_rt`] (ADR-0050 § D4).
/// 5. Race fetch, cancel and ticker in that `biased` order. Cancel calls
///    `abort()` on the preload task; dropping the `JoinHandle` only detaches it.
/// 6. Classify via `classify_preload_result` (private); on success set
///    `scenario_cfg.data_source` to `YahooCache` and `scenario_cfg.bars_override`.
///
/// # Errors
///
/// Returns the classified preload error (tagged no-data notice or hard error),
/// a `"preload task join error: …"`, or, on an operator Stop,
/// `"operator cancelled during preload"` tagged with
/// [`preload_notice::CANCELLED_TAG`] (bug-log `#215`; `classify` gives
/// `RunMessageKind::Cancelled`). `scenario_cfg` is left untouched in every
/// error case.
#[cfg(feature = "live")]
pub async fn run_yahoo_preload_arm(
    rt: &tokio::runtime::Handle,
    source: Box<dyn LabYahooBarSource>,
    cfg: &LabRunConfig,
    scenario_cfg: &mut backtest::ScenarioConfig,
    progress_tx: &backtest::progress::ProgressSender,
    cancel: &RunCancelReceiver,
    activity: Option<&agent::activity::ActivitySender>,
) -> Result<(), SmolStr> {
    // Bug #64 D.1.1 — the preload sentinel. It is emitted FIRST, before any
    // await, so the progress label moves at once rather than sitting static
    // (`yahoo_arm_emits_sentinel_before_preload_resolves` pins it). Spawn the
    // preload ONCE and race its JoinHandle below; consume the interval's
    // immediate first tick after the sentinel, so the first ticker event is
    // ~250 ms after it. Once the fetch wins, the loop exits and the ticker is
    // dropped, so no ticker event follows the arm's return
    // (`yahoo_arm_ticker_stops_after_preload_resolves`).
    //
    // `total_bars = 1` is the sentinel placeholder; the real value arrives
    // from the engine's bar-loop first emit.
    progress_tx.try_send(backtest::progress::Progress {
        current_bar: 0,
        total_bars: 1,
        elapsed_ms: 0,
    });

    let preload_start = std::time::Instant::now();
    // D-R1.1 (Bug #64 attempt-3 / ADR-0050 § D1): this future is polled on
    // iced's futures::ThreadPool, which has NO tokio reactor context.
    // `tokio::time::interval` needs one at construction time: built without
    // the guard it PANICS here, "there is no reactor running" (tokio 1.52
    // `Sleep::new_timeout`; ADR-0050 § Changelog, story 2-63 Wave F). The gate
    // is `lab_runner_ticker_e2e.rs::ticker_fires_at_least_3_times_in_1s_window`.
    // Same pattern as `ui::live::server_time_stream_impl`.
    //
    // Tick every 250 ms — visible animation on cold cache.
    let mut ticker = {
        let _guard = rt.enter(); // enter tokio reactor context
        tokio::time::interval(std::time::Duration::from_millis(250))
        // _guard dropped here; the constructed Sleep futures carry their
        // reactor binding and continue to fire.
    };
    // Consume the immediate (t=0) tick so the first sleep is ~250 ms from
    // here — well after the sentinel has fired.
    ticker.tick().await;

    // T-D-N7 — cockpit-activity-status-bar Yahoo preload producer wiring
    // (approach A: inline handle). Label: "Yahoo <SYMBOL> · <RANGE>". The
    // `ActivityHandle` is Send but not Sync; it is held across this
    // function's awaits inside `spawn_lab_run`'s Send-required
    // `iced::Task::perform` future and never leaves this arm.
    let yahoo_label = format!("Yahoo {} · {}", cfg.symbol, cfg.range_label());
    let yahoo_activity_handle = activity.map(|s| s.start(ActivityKind::YahooPreload, yahoo_label));

    // ADR-0050 § D4 (rt.spawn fix — recurrence #3): route through
    // spawn_preload_on_rt, the single guarded enforcement point
    // (T-BUG64-UN1). Every Yahoo source, the default and an override, takes
    // this one path (lab_runner_preload_callthrough_e2e.rs).
    let cfg_for_spawn = cfg.clone();
    let range_for_spawn = scenario_cfg.range.clone();
    let mut fetch_join = spawn_preload_on_rt(rt, source, cfg_for_spawn, range_for_spawn);

    // Race the JoinHandle (not the raw future) against cancel and the ticker,
    // `biased` in that order: fetch > cancel > ticker (Bug #64 attempt-3 /
    // ADR-0050).
    let preload_result = loop {
        // D-R1.4 (ADR-0050 § D1 defense-in-depth): yield to the executor
        // before each select! iteration, a canonical wake point for iced's
        // subscription reconciliation. Cost: ~0 µs.
        tokio::task::yield_now().await;

        tokio::select! {
            biased;
            joined = &mut fetch_join => {
                // JoinError means the spawned task panicked or was aborted —
                // map it to Err(SmolStr) so the UI shows a failure banner
                // rather than hanging.
                match joined {
                    Ok(inner) => break inner,
                    Err(e) => break Err(SmolStr::new(format!(
                        "preload task join error: {e}"
                    ))),
                }
            }
            // D-R2.2 (Bug #64 / ADR-0050 § D2): operator Stop during the
            // preload. The token resolves when the RunCancelHandle is dropped.
            // ADR-0050 § D4 + T-BUG64-RS3: the arm MUST abort the JoinHandle —
            // dropping it only DETACHES the task and the HTTP request keeps
            // running. A Stop is the operator's own action, not a failure
            // (bug-log #215): the activity ends Cancelled (the status bar's
            // tape drops the row at once), and the error is CANCELLED_TAG-
            // tagged, so the Lab draws RunState::Cancelled, not a red error.
            () = cancel.cancelled() => {
                fetch_join.abort();
                if let Some(h) = yahoo_activity_handle {
                    h.cancel();
                }
                return Err(preload_notice::cancelled_message("operator cancelled during preload"));
            }
            _ = ticker.tick() => {
                // Clamp to u64::MAX on overflow (impossible in practice —
                // preload is bounded to ~5 min by the per-attempt timeout in
                // fetch_with_backoff).
                let elapsed_ms = u64::try_from(
                    preload_start.elapsed().as_millis()
                ).unwrap_or(u64::MAX);
                // Non-blocking and lossy by design: a full receiver (buffer
                // = 8) skips this tick rather than blocking the preload loop.
                progress_tx.try_send(
                    backtest::progress::Progress {
                        current_bar: 0,
                        total_bars: 1,
                        elapsed_ms,
                    },
                );
            }
        }
    };
    // The ticker goes here: no ticker event after this point.
    drop(ticker);

    // lab-yahoo-empty-range-ux v0.1.0 — D-ER-1 / M-DEV.5: classify every
    // Yahoo preload result (zero bars → tagged no-data notice).
    match classify_preload_result(preload_result, cfg) {
        Ok((bars, _sha)) => {
            scenario_cfg.data_source = backtest::engine::ScenarioDataSource::YahooCache;
            scenario_cfg.bars_override = Some(bars);
            // Dropping the handle emits End { Success } (R1.3).
            drop(yahoo_activity_handle);
            Ok(())
        }
        Err(e) => {
            // End { Failed } before returning the error (R1.3 / F3), with the
            // NO_DATA_TAG sentinel stripped for log cleanliness (Caution #4).
            if let Some(handle) = yahoo_activity_handle {
                handle.fail(preload_notice::classify(e.as_str()).msg().as_str());
            }
            Err(e)
        }
    }
}

/// Build an `iced::Task` that spawns a Lab run and posts the result back to
/// the iced update loop as `Message::LabRunCompleted`.
///
/// In default (non-`live`) builds the tokio runtime is not available; the
/// function immediately resolves with a placeholder `RunSummary` that marks
/// the run as complete so `state::update` clears the `EquityCache` and the
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
/// The `LabRun` `ActivityHandle` lifecycle (T-D-N8) is managed by the caller
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
    // Some(source) => test injection; its first callers are story 2-63 Wave
    // F's Task-driven tests (spawn_lab_run_yahoo_harness.rs's `task_driven`
    // module and lab_yahoo_empty_range_classification.rs). Both paths go
    // through the one `run_yahoo_preload_arm` call below.
    // Only compiled under `live`; non-live builds receive a dummy Option<()>.
    #[cfg(feature = "live")] yahoo_source_override: Option<Box<dyn LabYahooBarSource>>,
    #[cfg(not(feature = "live"))] _yahoo_source_override: Option<()>,
) -> iced::Task<crate::state::Message> {
    use crate::state::Message;

    let strategy = cfg.strategy_id.clone();
    let symbol = cfg.symbol.clone();

    // ── #91 FIXED 2026-08-22 ─────────────────────────────────────────────
    // This arm used to return `Message::LabRunCompleted(Ok(RunSummary{ empty }))`
    // — a SUCCESS carrying zero fills, zero equity, default KPIs and no report
    // path: the exact wire shape of a real run that produced nothing. Both
    // siblings answer the same case with `Err`:
    //     spawn_bakeoff       -> Err(LEADERBOARD_RUN_NEEDS_LIVE)   (leaderboard/runner.rs:246)
    //     spawn_training_run  -> Err("training not supported ...")  (lab/trainer.rs:363)
    // Two of three bailed; one returned a plausible value. That is the same
    // discriminator that separated harmless `backtest/candle` (all off-arms
    // bail) from bug-log #81 (`backtest/realdata` returns a bare `None`).
    //
    // The stub's original rationale — "useful for the fixture cockpit" — expired
    // on 2026-05-25, the day AFTER it was written, when `live` became a default
    // feature and `--features fixtures` began ADDING to defaults rather than
    // replacing them. No shipping target has taken this branch since.
    //
    // It is reachable and verifiable again only because bug-log #92 fixed the
    // `--no-default-features` build; before that this code could not even be
    // compiled, which is why the fix waited.
    #[cfg(not(feature = "live"))]
    {
        let _ = (strategy, symbol, cancel, progress_tx);
        return iced::Task::done(Message::LabRunCompleted(Err(smol_str::SmolStr::new(
            crate::strings::LAB_RUN_NEEDS_LIVE,
        ))));
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
        // Returns Err immediately if a Custom bound does not parse (bug-log #230).
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

        // simple-strategies-realdata T-B5 — friendly rebuild guard mirroring
        // the Yahoo one above. Selecting Binance without the `binance` feature
        // returns a clear "rebuild with --features binance" message, never a
        // panic and never a silent synthetic fallback.
        #[cfg(not(feature = "binance"))]
        {
            if cfg.data_source == crate::lab::state::LabDataSource::BinanceCache {
                return iced::Task::done(Message::LabRunCompleted(Err(SmolStr::new(
                    "Binance data source requires the `binance` feature; \
                     rebuild with `--features binance`",
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
        // Consumed by the single `run_yahoo_preload_arm` call below, which is
        // compiled under `live` (story 2-63 Wave F).
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
                // lab-recipe-test-harness T-D1 / ADR-0048, story 2-63 Wave F:
                // ONE Yahoo call site. The override (tests) and the default
                // (production) differ only in the boxed source, so a test that
                // drives this Task with an override runs the exact call the
                // shipped path takes (spawn_lab_run_yahoo_harness.rs). The `let`s
                // between the two `if`s are deliberate (clippy collapsible_if).
                if cfg_for_preload.data_source == crate::lab::state::LabDataSource::YahooCache {
                    #[cfg(feature = "yahoo")]
                    let yahoo_source: Option<Box<dyn LabYahooBarSource>> = Some(
                        yahoo_source_override.unwrap_or_else(|| Box::new(DefaultLabYahooBarSource)),
                    );
                    // Unreachable without `yahoo` (YahooCache returns early
                    // above); keeps the override consumed in a live-only build.
                    #[cfg(not(feature = "yahoo"))]
                    let yahoo_source: Option<Box<dyn LabYahooBarSource>> = yahoo_source_override;
                    if let Some(source) = yahoo_source {
                        run_yahoo_preload_arm(
                            &rt,
                            source,
                            &cfg_for_preload,
                            &mut scenario_cfg,
                            &progress_tx,
                            &cancel,
                            activity_sender_for_closure.as_ref(),
                        )
                        .await?;
                    }
                }

                // simple-strategies-realdata T-B4 — Binance preload block.
                // Independent of the Yahoo if/else above (Binance and Yahoo are
                // mutually exclusive data sources). The whole arm lives in
                // `run_binance_preload_arm` — THE Binance test seam (review
                // patch 5): the harness calls the same function with a mock
                // `LabBinanceBarSource`, so this call site and the harness
                // exercise identical sentinel → `spawn_preload_on_rt` (ADR-0050
                // § D4, no second inline rt.spawn) → classify →
                // `bars_override = Some(bars)` glue. On miss / coverage
                // shortfall / out-of-span windows the loader returns a typed
                // tagged notice — NEVER a silent synthetic fallback (the AC4
                // no-op-source guard, design-side half). There is NO
                // `binance_source_override` parameter on `spawn_lab_run`;
                // injection happens at the `run_binance_preload_arm` seam.
                #[cfg(feature = "binance")]
                {
                    if cfg_for_preload.data_source == crate::lab::state::LabDataSource::BinanceCache
                    {
                        run_binance_preload_arm(
                            &rt,
                            Box::new(DefaultLabBinanceBarSource),
                            &cfg_for_preload,
                            &mut scenario_cfg,
                            &progress_tx,
                        )
                        .await?;
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
                        // bug-log #215: an operator Stop that reaches the engine
                        // (a Synthetic run, a cached Yahoo run, or a Stop during
                        // the Binance preload, which has no cancel arm) is a
                        // Stop, not an error. Keyed on `RunError::Cancelled`,
                        // not on the token, so a genuine error stays an error.
                        // Qualification: for the momentum, pairs and tcn arms
                        // the engine itself derives that variant from an anyhow
                        // message containing "Cancelled" (engine.rs, the
                        // `e.to_string().contains("Cancelled")` maps, ~1136 /
                        // 1186 / 1242); their only such text today is their own
                        // cancel error.
                        Err(backtest::engine::RunError::Cancelled) => {
                            Err(preload_notice::cancelled_message(
                                &backtest::engine::RunError::Cancelled.to_string(),
                            ))
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

// ── preload_notice — sentinel-tagged run-message classifier (D-ER-1 / D-ER-3; #215) ──

/// Sentinel-tagged run-message builders and classifier.
///
/// lab-yahoo-empty-range-ux v0.1.0 — D-ER-1 step 2, H2; bug-log `#215`.
///
/// The `LabRunResult = Result<RunSummary, SmolStr>` type is kept byte-identical
/// (94 usages; widening to a typed enum would ripple across ~7 test files).
/// Instead, the error string's leading tag says what kind of outcome it is —
/// a no-data NOTICE (`NO_DATA_TAG`), an operator STOP (`CANCELLED_TAG`), or
/// neither (a hard error) — decoded by `classify`. Both sentinels open with a
/// non-renderable control character that cannot appear in any operator-facing
/// copy (ADR-0040 § Changelog 2026-05-30 item (3); `CANCELLED_TAG` since
/// bug-log `#215`).
pub mod preload_notice {
    use smol_str::SmolStr;

    /// Non-renderable sentinel prefix (U+0001 START OF HEADING).
    /// Presence of this prefix signals a no-data NOTICE, not a hard error.
    /// Must be stripped before rendering to the operator.
    pub const NO_DATA_TAG: &str = "\u{1}NODATA\u{1}";

    /// Non-renderable sentinel prefix (U+0001) marking an operator STOP: the
    /// run ended because its `RunCancelHandle` was dropped (bug-log `#215`).
    /// Neither a notice nor an error; must be stripped before display.
    pub const CANCELLED_TAG: &str = "\u{1}CANCELLED\u{1}";

    /// Typed classification of a `LabRunResult` error string.
    pub enum RunMessageKind {
        /// A no-data NOTICE — expected, non-alarming outcome (e.g. future-dated
        /// range). Rendered muted; the `SmolStr` is the tag-stripped operator copy.
        Notice(SmolStr),
        /// A hard error — network failure, 429, parse error, etc.
        /// Rendered red ⚠. The `SmolStr` is the verbatim error message.
        Error(SmolStr),
        /// An operator Stop (bug-log `#215`): not an error and not a notice.
        /// The Lab draws `RunState::Cancelled`; the `SmolStr` is the
        /// tag-stripped body, for logs.
        Cancelled(SmolStr),
    }

    impl RunMessageKind {
        /// Extract the inner message string regardless of variant.
        #[must_use]
        pub fn msg(&self) -> &SmolStr {
            match self {
                Self::Notice(s) | Self::Error(s) | Self::Cancelled(s) => s,
            }
        }
    }

    /// Classify a raw `LabRunResult` error string.
    ///
    /// Returns `Cancelled(stripped)` if `raw` starts with `CANCELLED_TAG`,
    /// `Notice(stripped)` if it starts with `NO_DATA_TAG` (the tag removed from
    /// the returned string in both cases); otherwise `Error(raw)`.
    /// An empty string → `Error("")`.
    #[must_use]
    pub fn classify(raw: &str) -> RunMessageKind {
        if let Some(stripped) = raw.strip_prefix(CANCELLED_TAG) {
            RunMessageKind::Cancelled(SmolStr::new(stripped))
        } else if let Some(stripped) = raw.strip_prefix(NO_DATA_TAG) {
            RunMessageKind::Notice(SmolStr::new(stripped))
        } else {
            RunMessageKind::Error(SmolStr::new(raw))
        }
    }

    /// Tag `body` as an operator Stop (bug-log `#215`), so `classify` routes it
    /// to `RunMessageKind::Cancelled` instead of `Error`.
    ///
    /// Used by `run_yahoo_preload_arm`'s cancel branch and by `spawn_lab_run`
    /// for an engine `RunError::Cancelled`.
    #[must_use]
    pub fn cancelled_message(body: &str) -> SmolStr {
        SmolStr::new(format!("{CANCELLED_TAG}{body}"))
    }

    /// Build a sentinel-tagged no-data notice for the operator.
    ///
    /// Formats `strings::LAB_YAHOO_NO_DATA_NOTICE` with the given
    /// ticker + window pair, then prepends `NO_DATA_TAG` so `classify`
    /// can route it to `last_run_notice` (muted) instead of
    /// `last_run_error` (red ⚠).
    #[must_use]
    pub fn no_data_message(ticker: &str, start_label: &str, end_label: &str) -> SmolStr {
        let window = format!("{start_label}..{end_label}");
        let body = crate::strings::LAB_YAHOO_NO_DATA_NOTICE
            .replace("{ticker}", ticker)
            .replace("{window}", &window);
        SmolStr::new(format!("{NO_DATA_TAG}{body}"))
    }

    #[cfg(test)]
    #[allow(clippy::unwrap_used, clippy::doc_markdown)]
    mod classify_tests {
        use super::*;

        /// D-ER-4 classify unit — tagged string → Notice(stripped).
        #[test]
        fn tagged_string_classifies_as_notice_stripped() {
            let raw = format!("{NO_DATA_TAG}No Yahoo data for BTC-USD in 2026-04-29..2026-05-29");
            match classify(&raw) {
                RunMessageKind::Notice(s) => {
                    assert!(!s.starts_with(NO_DATA_TAG), "tag must be stripped");
                    assert!(s.contains("BTC-USD"), "operator copy preserved");
                }
                RunMessageKind::Error(_) => panic!("expected Notice, got Error"),
                RunMessageKind::Cancelled(_) => panic!("expected Notice, got Cancelled"),
            }
        }

        /// D-ER-4 classify unit — untagged string → Error(verbatim).
        #[test]
        fn untagged_string_classifies_as_error_verbatim() {
            let raw = "network error: connection refused";
            match classify(raw) {
                RunMessageKind::Error(s) => assert_eq!(s.as_str(), raw),
                RunMessageKind::Notice(_) => panic!("expected Error, got Notice"),
                RunMessageKind::Cancelled(_) => panic!("expected Error, got Cancelled"),
            }
        }

        /// D-ER-4 classify unit — empty string → Error.
        #[test]
        fn empty_string_classifies_as_error() {
            match classify("") {
                RunMessageKind::Error(s) => assert!(s.is_empty()),
                RunMessageKind::Notice(_) => panic!("expected Error, got Notice"),
                RunMessageKind::Cancelled(_) => panic!("expected Error, got Cancelled"),
            }
        }

        /// bug-log `#215` — a `CANCELLED_TAG`-tagged string → `Cancelled`, with
        /// the tag stripped and the body kept. The body and the tag are spelled
        /// out here, not read off `cancelled_message` (bug-log `#191`).
        #[test]
        fn cancel_tagged_string_classifies_as_cancelled_stripped() {
            let raw = "\u{1}CANCELLED\u{1}operator cancelled during preload";
            match classify(raw) {
                RunMessageKind::Cancelled(s) => {
                    assert_eq!(
                        s.as_str(),
                        "operator cancelled during preload",
                        "the tag must be stripped and the body kept"
                    );
                }
                RunMessageKind::Error(s) => panic!("expected Cancelled, got Error({s:?})"),
                RunMessageKind::Notice(s) => panic!("expected Cancelled, got Notice({s:?})"),
            }
        }

        /// bug-log `#215` — the two-sided pair with
        /// `untagged_string_classifies_as_error_verbatim`: the SAME body is
        /// `Cancelled` with the tag `cancelled_message` prepends and `Error`
        /// without it, and a Stop is never a notice. Probe T215-P4 (the
        /// `CANCELLED_TAG` branch never matches) turns both `#215` tests here
        /// red (measured).
        #[test]
        fn cancelled_message_is_tagged_and_not_a_notice() {
            let body = "backtest run cancelled by operator";
            let msg = cancelled_message(body);
            assert_eq!(
                msg.as_str(),
                "\u{1}CANCELLED\u{1}backtest run cancelled by operator",
                "cancelled_message must prepend the CANCELLED sentinel to the body"
            );
            assert!(
                !msg.starts_with(NO_DATA_TAG),
                "a Stop must not carry the no-data tag"
            );
            assert!(
                matches!(classify(&msg), RunMessageKind::Cancelled(s) if s == body),
                "the tagged body must classify as Cancelled(body)"
            );
            assert!(
                matches!(classify(body), RunMessageKind::Error(s) if s == body),
                "the same body untagged must classify as Error(body)"
            );
        }

        /// D-ER-4 no_data_message — produces tagged, readable string.
        #[test]
        fn no_data_message_is_tagged_and_readable() {
            let msg = no_data_message("SOL-USD", "2026-04-29", "2026-05-29");
            assert!(
                msg.starts_with(NO_DATA_TAG),
                "no_data_message must start with NO_DATA_TAG sentinel"
            );
            let stripped = msg.strip_prefix(NO_DATA_TAG).unwrap();
            assert!(stripped.contains("SOL-USD"), "ticker must appear in body");
            assert!(
                stripped.contains("2026-04-29..2026-05-29"),
                "window must appear in body"
            );
            assert!(
                !stripped.contains("CacheMiss"),
                "must not contain internal variant name"
            );
            assert!(
                !stripped.contains("MissingData"),
                "must not contain internal variant name"
            );
            assert!(
                !stripped.contains("Check network"),
                "must not contain misleading hint"
            );
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
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

    // ── bug-log #230: the typed range carrier and the one bound parse ─────────

    use crate::lab::state::{CustomRangeDraft, DateRange as LabRange, LabState, Preset};

    /// A Lab `Custom` range of the two bounds, as typed.
    fn custom(start: &str, end: &str) -> LabRange {
        LabRange::Custom {
            start_raw: SmolStr::new(start),
            end_raw: SmolStr::new(end),
        }
    }

    /// A run config over `range`, every other field fixed.
    fn cfg_over(range: LabRange) -> LabRunConfig {
        LabRunConfig {
            strategy_id: SmolStr::new("v1.momentum"),
            symbol: SmolStr::new("XRPUSDT"),
            venue: SmolStr::new("Binance"),
            range,
            seed: crate::lab::defaults::LAB_DEFAULT_SEED,
            write_report: false,
            data_source: crate::lab::state::LabDataSource::default(),
            sma_fast_len: None,
            sma_slow_len: None,
        }
    }

    /// The engine's `Custom` bounds a config maps to, or the mapper's error.
    fn engine_custom_ms(cfg: &LabRunConfig) -> Result<(i64, i64), SmolStr> {
        match lab_config_to_scenario(cfg)?.range {
            backtest::engine::DateRange::Custom { start_ms, end_ms } => Ok((start_ms, end_ms)),
            other => panic!("a Custom range must map to the engine's Custom, got {other:?}"),
        }
    }

    /// R1 (bug-log #230) — a start bound with a time reaches the engine whole: at HEAD it was cut
    /// at its first `:` and refused as `'2024-01-01T12'`.
    #[test]
    fn a_custom_start_bound_with_a_time_reaches_the_engine_whole() {
        let got = engine_custom_ms(&cfg_over(custom("2024-01-01T12:30:00Z", "2024-02-01")));
        assert_eq!(got, Ok((1_704_112_200_000, 1_706_745_600_000)));
    }

    /// R2 — R1's twin in the dimension "the start bound holds a `:`": a date-only pair reaches
    /// the engine at midnight UTC (green at HEAD too).
    #[test]
    fn a_date_only_custom_range_reaches_the_engine_at_midnight_utc() {
        let got = engine_custom_ms(&cfg_over(custom("2024-01-01", "2024-02-01")));
        assert_eq!(got, Ok((1_704_067_200_000, 1_706_745_600_000)));
    }

    /// R3 — a bound that does not parse is refused by its whole text, start and end alike.
    #[test]
    fn a_custom_bound_that_does_not_parse_is_refused_by_its_whole_text() {
        let start = engine_custom_ms(&cfg_over(custom("2024-01-01T00:00:00Q", "2024-02-01")));
        let end = engine_custom_ms(&cfg_over(custom("2024-01-01", "2024-02-01T00:00:00Q")));
        let start_err = start.expect_err("an unparseable start bound is refused");
        let end_err = end.expect_err("an unparseable end bound is refused");
        assert!(
            start_err.contains("'2024-01-01T00:00:00Q'"),
            "the start's refusal names it whole: {start_err}"
        );
        assert!(
            end_err.contains("'2024-02-01T00:00:00Q'"),
            "the end's refusal names it whole: {end_err}"
        );
    }

    /// R4 — every preset maps to its own engine range (replaces the label-alias test; an
    /// unknown range can no longer be represented).
    #[test]
    fn every_preset_maps_to_its_engine_range() {
        use backtest::engine::DateRange as E;
        let map = |p| lab_config_to_scenario(&cfg_over(LabRange::Preset(p))).map(|sc| sc.range);
        assert!(matches!(map(Preset::Last30d), Ok(E::Last30d)));
        assert!(matches!(map(Preset::Last90d), Ok(E::Last90d)));
        assert!(matches!(map(Preset::H1_2024), Ok(E::H1_2024)));
        assert!(matches!(map(Preset::H2_2024), Ok(E::H2_2024)));
    }

    /// R5 — `range_label()` prints the text the `range_label` field held before #230, and
    /// operator text that holds a `{…}` placeholder prints as typed.
    #[test]
    fn range_label_prints_the_text_the_field_held_before_230() {
        let label = |r| cfg_over(r).range_label();
        assert_eq!(label(LabRange::Preset(Preset::Last30d)), "Last30d");
        assert_eq!(label(LabRange::Preset(Preset::Last90d)), "Last90d");
        assert_eq!(label(LabRange::Preset(Preset::H1_2024)), "H1_2024");
        assert_eq!(label(LabRange::Preset(Preset::H2_2024)), "H2_2024");
        assert_eq!(
            label(custom("2024-01-01T00:00:00Z", "2024-02-01")),
            "Custom:2024-01-01T00:00:00Z:2024-02-01"
        );
        assert_eq!(label(custom("{end}x", "E")), "Custom:{end}x:E");
    }

    /// R6 — `custom_bound_ms` reads RFC 3339 and `YYYY-MM-DD` and nothing else, in pairs that
    /// differ in one dimension. The last five Err rows are the deleted `is_valid_date` cases.
    #[test]
    fn custom_bound_ms_reads_rfc3339_and_date_only_and_nothing_else() {
        assert_eq!(custom_bound_ms("2024-02-29"), Ok(1_709_164_800_000));
        assert!(custom_bound_ms("2024-02-30").is_err());
        assert_eq!(
            custom_bound_ms("2024-01-01T00:00:00Z"),
            Ok(1_704_067_200_000)
        );
        assert!(custom_bound_ms("2024-01-01T00:00:00").is_err());
        assert_eq!(
            custom_bound_ms("2024-01-01T01:00:00+01:00"),
            Ok(1_704_067_200_000)
        );
        for raw in [
            "",
            "2024-1-1",
            "20240101",
            "not-a-date",
            "2024-01-01T00",
            "2024-13-01",
            "2024-00-01",
            "2024-01-00",
            "2024-01-32",
            "abcd-01-01",
        ] {
            assert!(custom_bound_ms(raw).is_err(), "{raw:?} must not parse");
        }
    }

    /// R9 (bug-log #230 review) — a Custom pair that does not increase is refused by name, for
    /// every source; its twin one day longer maps. The reversed and the equal pair differ from
    /// the valid one only in the end bound (#161: the equal pair is the boundary).
    #[test]
    fn a_custom_range_that_does_not_increase_is_refused_by_name_for_every_source() {
        use crate::lab::state::LabDataSource;
        for source in [
            LabDataSource::Synthetic,
            LabDataSource::YahooCache,
            LabDataSource::BinanceCache,
        ] {
            let over = |start: &str, end: &str| {
                let mut cfg = cfg_over(custom(start, end));
                cfg.data_source = source;
                cfg
            };
            let refusal = |start: &str, end: &str| -> Result<(i64, i64), SmolStr> {
                Err(SmolStr::new(refusal_text(start, end)))
            };
            assert_eq!(
                engine_custom_ms(&over("2024-03-01", "2024-02-01")),
                refusal("2024-03-01", "2024-02-01"),
                "{source:?}: a reversed pair"
            );
            assert_eq!(
                engine_custom_ms(&over("2024-02-01", "2024-02-01")),
                refusal("2024-02-01", "2024-02-01"),
                "{source:?}: an equal pair"
            );
            assert_eq!(
                engine_custom_ms(&over("2024-02-01", "2024-02-02")),
                Ok((1_706_745_600_000, 1_706_832_000_000)),
                "{source:?}: the next day maps"
            );
        }
        let named = refusal_text("2024-03-01", "2024-02-01");
        assert!(
            named.contains("'2024-03-01'") && named.contains("'2024-02-01'"),
            "the refusal names both bounds as typed: {named}"
        );
    }

    fn refusal_text(start: &str, end: &str) -> String {
        crate::strings::fmt_lab_custom_range_not_increasing(start, end)
    }

    /// The Lab of R7/R8: v0.sma × Binance BTCUSDT over `range`.
    // `LabState`'s compare set is private to `lab::state`, so struct-update syntax cannot
    // build it from here.
    #[allow(clippy::field_reassign_with_default)]
    fn lab_over(range: LabRange) -> LabState {
        let mut lab = LabState::default();
        lab.strategy = Some(trading_core::StrategyId::new("v0.sma"));
        lab.pair = Some((Venue::Binance, Symbol::new("BTCUSDT")));
        lab.range = range;
        lab
    }

    /// R7 — a range committed from the editor with a time runs: the keystroke goes through
    /// `state::update`, the run config through the production seam `lab_run_config`.
    #[test]
    fn a_custom_range_committed_from_the_editor_with_a_time_runs() {
        let mut cockpit = crate::state::Cockpit {
            lab_state: lab_over(custom("", "")),
            ..Default::default()
        };
        crate::state::update(
            &mut cockpit,
            crate::state::Message::LabCustomRangeEdited {
                start_raw: SmolStr::new("2024-01-01T12:30:00Z"),
                end_raw: SmolStr::new("2024-02-01"),
            },
        );
        let cfg = lab_run_config(&cockpit.lab_state).expect("strategy and pair are selected");
        assert_eq!(
            engine_custom_ms(&cfg),
            Ok((1_704_112_200_000, 1_706_745_600_000))
        );
    }

    /// R8a — a Run from an uncommitted edit runs what the fields show, and is refused naming the
    /// bound as typed.
    #[test]
    fn a_run_from_an_uncommitted_edit_is_refused_naming_the_typed_bound() {
        let mut lab = lab_over(custom("2024-01-15", "2024-02-01"));
        lab.custom_draft = Some(CustomRangeDraft {
            base: lab.range.clone(),
            start_raw: SmolStr::new("2024-01-1"),
            end_raw: SmolStr::new("2024-02-01"),
        });
        assert!(
            lab.custom_draft.is_some(),
            "precondition: an edit is pending"
        );
        let cfg = lab_run_config(&lab).expect("strategy and pair are selected");
        let err = engine_custom_ms(&cfg).expect_err("the draft never parses");
        assert!(err.contains("'2024-01-1'"), "names the typed bound: {err}");
    }

    /// R8b — R8a's twin without the draft: the committed pair runs.
    #[test]
    fn a_run_with_no_uncommitted_edit_maps_the_committed_pair() {
        let lab = lab_over(custom("2024-01-15", "2024-02-01"));
        assert!(lab.custom_draft.is_none());
        let cfg = lab_run_config(&lab).expect("strategy and pair are selected");
        assert_eq!(
            engine_custom_ms(&cfg),
            Ok((1_705_276_800_000, 1_706_745_600_000))
        );
    }

    /// T-D-N9 — `lab_config_to_scenario` passes seed and `write_report` through.
    #[test]
    fn lab_config_to_scenario_passthrough_fields() {
        let seed = crate::lab::defaults::LAB_DEFAULT_SEED;
        let cfg = LabRunConfig {
            strategy_id: SmolStr::new("v1.momentum"),
            symbol: SmolStr::new("XRPUSDT"),
            venue: SmolStr::new("Binance"),
            range: crate::lab::state::DateRange::Preset(crate::lab::state::Preset::Last90d),
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

    /// T-D-14 — `spawn_lab_run` without a runtime resolves immediately.
    #[test]
    fn spawn_lab_run_no_runtime_resolves_immediately() {
        let cfg = LabRunConfig {
            strategy_id: SmolStr::new("v1.momentum"),
            symbol: SmolStr::new("XRPUSDT"),
            venue: SmolStr::new("Binance"),
            range: crate::lab::state::DateRange::Preset(crate::lab::state::Preset::Last90d),
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
