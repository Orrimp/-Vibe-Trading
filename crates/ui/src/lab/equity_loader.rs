//! Cached backtest report equity-curve loader — ui-rethink-phase-a-lab T-D-10.
//!
//! Scans `<root>[/<container>]/<strategy-slug>/reports/backtest-*.md` (the
//! `crate::reports::layout` contract, ADR-0055 D8 / bug-log #205) under each
//! report root (`lab-runs/`, `evidence/`) for reports that match a
//! `(strategy, pair, range)` tuple and parses the per-bar equity series.
//!
//! ## Design notes (Design § 4.3 / R7)
//!
//! - Reads are **synchronous** (< 50 KB files, < 16 ms on a cold cache for a
//!   90-day report on the operator's 3360×1890 machine).
//! - Per-tuple in-memory cache (`EquityCache`) avoids re-parsing on every
//!   paint.
//! - Closest-superset fallback: when no exact-match report exists, the loader
//!   finds the smallest superset-range report and returns it with a
//!   `narrowed_from` annotation (R5.4 / R7.2).
//! - Low-fidelity fallback: if the report has no `## Equity curve` table, a
//!   two-point segment (start → end) is synthesised from the Summary block
//!   (R7.3). `Fidelity::StartEndOnly` is set in that case; its ordinal
//!   timestamps make it draw only on the chart's bar-less path (bug-log #207).
//!
//! **No I/O on the iced thread during a paint.** `EquityCache::get_or_load`
//! blocks only on the *first* access; all subsequent accesses return the
//! cached `Arc<LabEquitySeries>`.
//!
//! **Zero string literals in this module** — all user-visible copy lives in
//! `crate::strings`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use rust_decimal::Decimal;
use smol_str::SmolStr;
use thiserror::Error;
use trading_core::{StrategyId, Symbol, Venue};

use crate::lab::state::{DateRange, LabDataSource, Preset};
use crate::reports::layout;

// ── Public data shapes ────────────────────────────────────────────────────────

/// Fidelity level of the equity series read from a report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fidelity {
    /// Per-bar data points from the report's `## Equity curve` table.
    PerBar,
    /// Only start + end equity available (older v0/v1 reports without the
    /// per-bar section or a companion CSV). The two samples carry the ORDINAL
    /// timestamps `0` (start) and `1` (end), not times: a report does not
    /// record its span (see `make_start_end_series`). So the series draws
    /// only on the chart's bar-less path, as a straight full-width
    /// start→end segment; over price bars both samples miss the bar window
    /// and the overlay is absent — no curve, no equity axis, no gutter
    /// (bug-log #207). No other consumer reads this variant: there is no
    /// legend or chip decoration for it.
    StartEndOnly,
}

/// An equity series loaded from a cached backtest report.
#[derive(Debug, Clone)]
pub struct LabEquitySeries {
    /// Ordered oldest-first equity samples `(timestamp_millis, equity_usdt)`.
    pub samples: Vec<(i64, Decimal)>,
    /// Source report name for the "narrowed from" badge (R5.4).
    pub source_report: SmolStr,
    /// Fidelity of the loaded data.
    pub fidelity: Fidelity,
    /// When the loader fell back to a superset report, this holds the
    /// human-readable report name for the narrowed-from badge. `None` when
    /// an exact-match was found.
    pub narrowed_from: Option<SmolStr>,
}

impl LabEquitySeries {
    /// Build a `PerBar` series from raw timestamped samples (oldest-first).
    ///
    /// lab-compare-equity-overlay T2 — the Compare overlay constructs one of
    /// these from a `CachedCell::equity_series_ts` (already hydrated from the
    /// companion CSV at scan time) to feed the two-run `chart::view` overlay
    /// WITHOUT re-reading disk on every paint. `source_report` labels the curve;
    /// `narrowed_from` is `None` (the cell IS the resolved report). Returns
    /// `None` when `samples` is empty — an empty series has no curve to draw.
    #[must_use]
    pub fn from_samples(samples: Vec<(i64, Decimal)>, source_report: SmolStr) -> Option<Self> {
        if samples.is_empty() {
            return None;
        }
        Some(Self {
            samples,
            source_report,
            fidelity: Fidelity::PerBar,
            narrowed_from: None,
        })
    }
}

/// Cache key: exact `(strategy_slug, symbol, range, source)` tuple.
///
/// `source` was added by the simple-strategies-realdata review (D1 — key
/// Compare/EquityCache by data source): without it, Binance and
/// Synthetic/Yahoo runs of the same `(strategy, symbol, range)` shadowed each
/// other in the cache and in report resolution (newest report won). The
/// loader resolves reports source-aware via the report frontmatter's
/// `data_source:` field (see [`load_equity`]).
///
/// NOT serialized — the Lab persistence schema (`version: 1`) stores
/// `strategy`/`pair`/`range`/`data_source` as separate `LabState` fields,
/// never this struct, so the added field is not a schema change.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LabTuple {
    pub strategy: SmolStr,
    pub symbol: SmolStr,
    pub range: DateRange,
    /// Data source the run used (`Synthetic` / `YahooCache` / `BinanceCache`).
    pub source: LabDataSource,
}

impl LabTuple {
    /// Construct from the typed lab-state fields.
    #[must_use]
    pub fn new(
        strategy: &StrategyId,
        venue: Venue,
        symbol: &Symbol,
        range: DateRange,
        source: LabDataSource,
    ) -> Self {
        let _ = venue; // Phase A universe is single-venue; key on symbol only.
        Self {
            strategy: SmolStr::new(&strategy.0),
            symbol: SmolStr::new(symbol.0.as_str()),
            range,
            source,
        }
    }
}

/// Errors from the equity loader.
#[derive(Debug, Error)]
pub enum EquityLoadError {
    /// No `backtest-*.md` file for the strategy's slug under the root, at any
    /// container depth ≤ `reports::layout::MAX_CONTAINER_DEPTH`. Despite the
    /// name it covers BOTH empties; the message says which: no `reports/` dir
    /// for the slug at all, or such dirs exist but hold no `backtest-*.md`.
    #[error("no backtest report to load: {0}")]
    ReportDirNotFound(String),
    #[error("no cached report found for ({strategy}, {symbol}, {range:?}, {data_source:?})")]
    NoReport {
        strategy: SmolStr,
        symbol: SmolStr,
        range: DateRange,
        // Named `data_source` (not `source`) because thiserror reserves a
        // field named `source` for the std error-source chain.
        data_source: LabDataSource,
    },
    #[error("report parse error in {path}: {msg}")]
    ParseError { path: String, msg: String },
    #[error("equity series is empty for {path}")]
    EmptySeries { path: String },
}

// ── EquityCache ───────────────────────────────────────────────────────────────

/// Per-session in-memory cache for loaded equity series (Design § 4.3).
///
/// All reads are synchronous. The cache grows monotonically; individual
/// tuples are invalidated via [`EquityCache::invalidate`] (called on
/// `Message::LabRunCompleted(Ok(...))`).
#[derive(Debug, Default, Clone)]
pub struct EquityCache {
    by_tuple: HashMap<LabTuple, Arc<LabEquitySeries>>,
}

impl EquityCache {
    /// Create a new empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return a cached `Arc<LabEquitySeries>` if present, otherwise load it
    /// synchronously from a SINGLE disk root, cache it, and return it.
    ///
    /// `report_root` should point to one report root (e.g. the repository's
    /// `evidence/` directory, or a `lab-runs/` tempdir). The loader searches
    /// `<root>[/<container>]/<strategy-slug>/reports/backtest-*.md`, container
    /// depth ≤ `reports::layout::MAX_CONTAINER_DEPTH` (bug-log #205).
    ///
    /// This single-root entry point is the H3 invariant's read seam
    /// (ADR-0055 § D6 — `crates/ui/tests/lab_run_engine.rs` calls it with the
    /// engine's `report_path.parent().parent().parent()` write-root). The
    /// production Lab path uses [`EquityCache::get_or_load_roots`] for the
    /// two-root (`lab-runs/` + `evidence/`) union (Q4).
    ///
    /// # Errors
    ///
    /// Returns `EquityLoadError` when no matching report is found or the
    /// report body cannot be parsed (see `load_equity`).
    pub fn get_or_load(
        &mut self,
        tuple: &LabTuple,
        report_root: &std::path::Path,
    ) -> Result<Arc<LabEquitySeries>, EquityLoadError> {
        if let Some(cached) = self.by_tuple.get(tuple) {
            return Ok(Arc::clone(cached));
        }
        let loaded = load_equity(tuple, report_root)?;
        let arc = Arc::new(loaded);
        self.by_tuple.insert(tuple.clone(), Arc::clone(&arc));
        Ok(arc)
    }

    /// Two-root union loader (lab-run-save-compare T2 / Q4 / ADR-0055 § D5).
    ///
    /// Like [`EquityCache::get_or_load`] but searches a **fixed-order slice of
    /// roots** (production passes `[default_lab_runs_root(),
    /// default_evidence_root()]`, lab-runs FIRST). The first root that yields a
    /// matching report wins — so a fresh Lab run under `lab-runs/` shadows a
    /// committed `evidence/` report for the same tuple (the collision rule:
    /// lab-runs wins). A tuple resolves to exactly one series, so the cache
    /// stays keyed on `tuple` alone (root-independent); the cached `Arc` is
    /// returned on subsequent hits regardless of which root won.
    ///
    /// # Errors
    ///
    /// Returns the LAST root's `EquityLoadError` when no root yields a report
    /// (the evidence-root error is the most informative for the operator — it
    /// is the committed tree). Empty `roots` yields a `NoReport` error.
    pub fn get_or_load_roots(
        &mut self,
        tuple: &LabTuple,
        roots: &[PathBuf],
    ) -> Result<Arc<LabEquitySeries>, EquityLoadError> {
        if let Some(cached) = self.by_tuple.get(tuple) {
            return Ok(Arc::clone(cached));
        }
        let mut last_err: Option<EquityLoadError> = None;
        for root in roots {
            match load_equity(tuple, root) {
                Ok(loaded) => {
                    let arc = Arc::new(loaded);
                    self.by_tuple.insert(tuple.clone(), Arc::clone(&arc));
                    return Ok(arc);
                }
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap_or_else(|| EquityLoadError::NoReport {
            strategy: tuple.strategy.clone(),
            symbol: tuple.symbol.clone(),
            range: tuple.range.clone(),
            data_source: tuple.source,
        }))
    }

    /// Invalidate a cached series so the next `get_or_load` re-reads from
    /// disk. Called after a fresh backtest run completes (M2.5 / T-D-14).
    pub fn invalidate(&mut self, tuple: &LabTuple) {
        self.by_tuple.remove(tuple);
    }
}

// ── Strategy slug resolution ──────────────────────────────────────────────────

/// Map a `StrategyId` to the evidence-directory slug that holds its reports.
/// Phase A supports the known strategy ids; unknown ids fall back to the
/// verbatim id string (which may produce a `ReportDirNotFound` error if the
/// directory doesn't exist).
#[must_use]
fn strategy_slug(id: &str) -> SmolStr {
    match id {
        "v1.momentum" | "top10_momentum_h1" => SmolStr::new("v1-cross-sectional-momentum"),
        "v0.sma" | "sma_crossover" => SmolStr::new("v0-paper-sma"),
        "v0.5.macd" | "btc_macd_trend" => SmolStr::new("v05-composed-strategies"),
        "v1.5a.mr" | "pairs_mr_h1" => SmolStr::new("v15a-mean-reversion-pairs"),
        other => SmolStr::new(other),
    }
}

// ── Report scanning and parsing ───────────────────────────────────────────────

/// Discover all `backtest-*.md` paths of `slug` under `report_root`: the files
/// of every `reports/` dir the layout lists for the slug —
/// `<root>/<slug>/reports/` (container depth 0) AND
/// `<root>/<container>/<slug>/reports/` (container depth 1). Production roots:
/// `lab-runs/`, and `evidence/`, whose committed reports sit at container
/// depth 1 since `1395520629dc` (bug-log #205: a one-level walk found none).
///
/// **Order = the D8 tie rule (ADR-0055 amendment).** The dirs come in
/// (container depth, path) order and each dir's files by path; they are
/// concatenated in that order and NEVER re-sorted by full path. So the
/// strict-`>` "first best" in [`load_equity`] keeps a container-depth-0 report
/// (the only place the walk looked before #205) on an equal-score tie with a
/// container-depth-1 one, whether the container name sorts before or after the
/// slug name. Pinned by E4b
/// (`equal_score_tie_across_container_depths_goes_to_container_depth_0`).
fn discover_reports(report_root: &std::path::Path, slug: &str) -> Vec<PathBuf> {
    layout::report_dirs_for_slug(report_root, slug)
        .into_iter()
        .flat_map(|d| layout::backtest_files(&d))
        .collect()
}

/// The `ReportDirNotFound` detail: names the slug, the root and the container
/// depth bound, and says which empty it is (`dirs` = how many `reports/` dirs
/// the layout lists for the slug).
fn no_report_detail(report_root: &std::path::Path, slug: &str, dirs: usize) -> String {
    let bound = layout::MAX_CONTAINER_DEPTH;
    if dirs == 0 {
        format!(
            "no `{}/` dir for slug `{slug}` under {} at container depth ≤ {bound} \
             (`<root>[/<container>]/<slug>/reports/`)",
            layout::REPORTS_DIR,
            report_root.display()
        )
    } else {
        format!(
            "{dirs} `{}/` dir(s) for slug `{slug}` under {} (container depth ≤ {bound}) hold \
             no `backtest-*.md`",
            layout::REPORTS_DIR,
            report_root.display()
        )
    }
}

/// Parsed front-matter fields relevant to cache-key matching.
#[derive(Debug)]
struct ReportMeta {
    scenario: String,
    /// The `data_source:` frontmatter value ("synthetic" / "yahoo…" /
    /// "binance"), written by every engine report since commit 93845af.
    /// `None` for legacy (pre-June) reports without the field — treated as
    /// source-unknown by [`load_equity`] (review D1: they only match when no
    /// report tagged with the requested source exists; no on-disk migration).
    data_source: Option<String>,
    /// The pair symbol extracted from the report (from `## Universe` section
    /// or the `scenario:` field). `None` for multi-symbol reports.
    symbol: Option<String>,
    /// Approximate year range extracted from scenario name (e.g. "2024" →
    /// matches `Preset::H1_2024` / `Preset::H2_2024`).
    year_hint: Option<i32>,
    /// Whether the report body contains a `## Equity curve` section.
    has_equity_section: bool,
    /// Initial capital from the Summary table.
    initial_capital: Option<Decimal>,
    /// Final equity from the Summary table.
    final_equity: Option<Decimal>,
}

/// Parse report front-matter + body summary block for matching/loading.
fn parse_report_meta(content: &str) -> Option<ReportMeta> {
    // Extract front-matter (between first and second `---`).
    let mut fm_lines: Vec<&str> = Vec::new();
    let mut in_fm = false;
    let mut dash_count = 0u32;
    let mut body = "";
    let mut body_start = 0usize;

    for line in content.lines() {
        body_start += line.len() + 1; // +1 for '\n'
        if line.trim() == "---" {
            dash_count += 1;
            if dash_count == 1 {
                in_fm = true;
                continue;
            }
            if dash_count == 2 {
                body = &content[body_start..];
                break;
            }
        }
        if in_fm {
            fm_lines.push(line);
        }
    }
    if dash_count < 2 {
        body = content;
    }

    let scenario = fm_lines
        .iter()
        .find_map(|l| l.strip_prefix("scenario: "))
        .map(|s| s.trim().to_string())?;

    // Review D1 — source-aware resolution: the engine writes `data_source:`
    // into every report frontmatter (93845af); legacy reports lack it → None.
    let data_source = fm_lines
        .iter()
        .find_map(|l| l.strip_prefix("data_source: "))
        .map(|s| s.trim().to_string());

    // Extract year hint from scenario name.
    let year_hint: Option<i32> = ["2023", "2024", "2025"]
        .iter()
        .find(|&&y| scenario.contains(y))
        .and_then(|y| y.parse::<i32>().ok());

    // Symbol: look for single-symbol scenarios (not "multi").
    let symbol = parse_symbol_from_body(body).or_else(|| {
        // Fall back to scenario name heuristic.
        let sym_candidates = [
            "XRPUSDT", "ETHUSDT", "BTCUSDT", "ADAUSDT", "AVAXUSDT", "BNBUSDT", "DOGEUSDT",
            "DOTUSDT", "LINKUSDT", "SOLUSDT",
        ];
        sym_candidates
            .iter()
            .find(|&&s| scenario.to_ascii_uppercase().contains(s))
            .map(ToString::to_string)
    });

    let has_equity_section = body.contains("## Equity curve") || body.contains("## Equity Curve");

    // Parse Summary table for initial / final capital.
    let initial_capital = parse_summary_value(body, "Initial capital")
        .or_else(|| parse_summary_value(body, "Initial Capital"));
    let final_equity = parse_summary_value(body, "Final equity")
        .or_else(|| parse_summary_value(body, "Final Equity"));

    Some(ReportMeta {
        scenario,
        data_source,
        symbol,
        year_hint,
        has_equity_section,
        initial_capital,
        final_equity,
    })
}

/// True when a report's frontmatter `data_source:` value denotes the
/// requested [`LabDataSource`] (review D1).
///
/// Prefix-matched because the Yahoo CLI emitters write extended forms like
/// `yahoo-cache:<TICKER>/<INTERVAL>/2024` (see
/// `report::yahoo::YahooReportContext::data_source()`), while the engine path
/// writes the plain `synthetic` / `yahoo` / `binance` tokens.
fn source_tag_matches(report_data_source: &str, source: LabDataSource) -> bool {
    match source {
        LabDataSource::Synthetic => report_data_source.starts_with("synthetic"),
        LabDataSource::YahooCache => report_data_source.starts_with("yahoo"),
        LabDataSource::BinanceCache => report_data_source.starts_with("binance"),
    }
}

/// Extract a single symbol from the `## Universe` section.
fn parse_symbol_from_body(body: &str) -> Option<String> {
    let mut in_universe = false;
    let mut symbols: Vec<&str> = Vec::new();
    for line in body.lines() {
        if line.trim_start().starts_with("## Universe") {
            in_universe = true;
            continue;
        }
        if in_universe {
            if line.starts_with("##") {
                break;
            }
            let sym = line.trim_start_matches(['-', ' ', '\t']);
            let sym = sym.trim();
            if !sym.is_empty() && !sym.contains(' ') {
                symbols.push(sym);
            }
        }
    }
    if symbols.len() == 1 {
        Some(symbols[0].to_string())
    } else {
        None
    }
}

/// Extract a `Decimal` value from a markdown table row like
/// `| Final equity  | $46401.41 USDT |`.
fn parse_summary_value(body: &str, field: &str) -> Option<Decimal> {
    for line in body.lines() {
        if line.contains(field) {
            // Find the value after `|` separator.
            let parts: Vec<&str> = line.splitn(3, '|').collect();
            if parts.len() >= 3 {
                let raw = parts[2]
                    .trim()
                    .trim_start_matches('$')
                    .split_whitespace()
                    .next()
                    .unwrap_or("");
                if let Ok(d) = raw.replace(',', "").parse::<Decimal>() {
                    return Some(d);
                }
            }
        }
    }
    None
}

/// Check whether a report matches the requested `(symbol, range)` tuple.
fn report_matches(meta: &ReportMeta, sym: &str, range: &DateRange) -> MatchQuality {
    // Symbol check.
    let sym_match = match &meta.symbol {
        Some(s) => s.eq_ignore_ascii_case(sym),
        None => true, // Multi-symbol reports: pass if symbol is in universe.
    };

    if !sym_match {
        return MatchQuality::NoMatch;
    }

    let range_match = range_score(meta, range);
    if range_match == 0 {
        return MatchQuality::NoMatch;
    }
    MatchQuality::Match {
        range_score: range_match,
    }
}

#[derive(Debug, PartialEq, Eq)]
enum MatchQuality {
    NoMatch,
    Match { range_score: u32 },
}

/// Score how well a report's time range covers the requested range.
/// Higher = better. `0` = no match.
fn range_score(meta: &ReportMeta, range: &DateRange) -> u32 {
    match range {
        DateRange::Preset(p) => match p {
            Preset::Last30d => {
                // Any report covers this as a superset — prefer 2024 reports.
                match meta.year_hint {
                    Some(2024) => 10,
                    Some(_) => 5,
                    None => 3,
                }
            }
            Preset::Last90d => match meta.year_hint {
                Some(2024) => 10,
                Some(_) => 5,
                None => 3,
            },
            Preset::H1_2024 => {
                if meta.scenario.contains("2024")
                    || meta.scenario.contains("h1")
                    || meta.scenario.contains("2024-h1")
                {
                    20
                } else if meta.year_hint == Some(2024) {
                    10
                } else {
                    0
                }
            }
            Preset::H2_2024 => {
                if meta.scenario.contains("2024")
                    && (meta.scenario.contains("h2") || meta.scenario.contains("H2"))
                {
                    20
                } else if meta.year_hint == Some(2024) {
                    8
                } else {
                    0
                }
            }
        },
        DateRange::Custom { .. } => {
            // Custom ranges: any report is a superset fallback.
            1
        }
    }
}

/// lab-run-save-compare Wave-2: read the companion equity CSV beside `md_path`
/// (`backtest-<stamp>-<scenario>-equity.csv`) for full per-bar fidelity, via
/// the existing [`reports::csv_artifacts::read_equity_csv`]. Returns `None` if
/// the companion is absent or unparseable — the caller then falls back to the
/// sparkline / start-end path (older committed reports have no companion).
///
/// `pub(crate)` so the Compare cold-boot cache scanner
/// (`compare::cache::scan_one_root`, lab-compare-equity-overlay T1) hydrates a
/// `CachedCell`'s timestamped equity series through the IDENTICAL companion-CSV
/// resolution the Lab cold path uses — one source of truth for "find the
/// per-bar series beside this report".
#[must_use]
pub(crate) fn load_companion_equity_csv(md_path: &std::path::Path) -> Option<Vec<(i64, Decimal)>> {
    let name = md_path.file_name()?.to_str()?;
    let stem = name.strip_suffix(".md")?;
    let csv = md_path.with_file_name(format!("{stem}-equity.csv"));
    let samples = reports::csv_artifacts::read_equity_csv(&csv).ok()?;
    if samples.is_empty() {
        return None;
    }
    Some(
        samples
            .into_iter()
            .map(|s| (s.ts.unix_millis(), s.equity_total))
            .collect(),
    )
}

/// Parse the `## Equity curve` section from a report body.
/// Returns `(timestamp_millis, equity_decimal)` pairs.
fn parse_equity_section(body: &str) -> Vec<(i64, Decimal)> {
    let mut in_section = false;
    let mut points: Vec<(i64, Decimal)> = Vec::new();

    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## Equity curve") || trimmed.starts_with("## Equity Curve") {
            in_section = true;
            continue;
        }
        if in_section {
            if trimmed.starts_with("##") {
                break;
            }
            // Skip header rows and separator rows.
            if trimmed.starts_with('|')
                && !trimmed.contains("---")
                && !trimmed.to_lowercase().contains("timestamp")
            {
                let cols: Vec<&str> = trimmed.splitn(4, '|').collect();
                // | ts_millis | equity |
                if cols.len() >= 3 {
                    let ts_raw = cols[1].trim();
                    let eq_raw = cols[2]
                        .trim()
                        .trim_start_matches('$')
                        .split_whitespace()
                        .next()
                        .unwrap_or("");
                    if let (Ok(ts), Ok(eq)) = (
                        ts_raw.parse::<i64>(),
                        eq_raw.replace(',', "").parse::<Decimal>(),
                    ) {
                        points.push((ts, eq));
                    }
                }
            }
        }
    }
    points
}

// ── Main load function ────────────────────────────────────────────────────────

/// Load an equity series for `tuple` from a report-root directory tree
/// (production roots: `evidence/` or `lab-runs/`).
/// Synchronous; call from the iced thread on first cache miss.
///
/// # Errors
///
/// Returns `EquityLoadError::ReportDirNotFound` when the slug has no
/// `backtest-*.md` under the root at all (its message names the slug, the root,
/// the container depth bound and which empty it is);
/// `EquityLoadError::NoReport` when no matching report is found;
/// `EquityLoadError::ParseError` when the best candidate cannot be parsed.
pub fn load_equity(
    tuple: &LabTuple,
    report_root: &std::path::Path,
) -> Result<LabEquitySeries, EquityLoadError> {
    let slug = strategy_slug(&tuple.strategy);
    let reports = discover_reports(report_root, &slug);

    if reports.is_empty() {
        // Error path only: one more targeted walk says WHICH empty it is.
        let dirs = layout::report_dirs_for_slug(report_root, &slug).len();
        return Err(EquityLoadError::ReportDirNotFound(no_report_detail(
            report_root,
            &slug,
            dirs,
        )));
    }

    // Find the best matching report — source-aware since the review D1 fix.
    //
    // Two tiers:
    //   1. Reports whose frontmatter `data_source:` matches `tuple.source`
    //      (the honest tier — a Binance tuple resolves a binance report).
    //   2. Legacy reports WITHOUT the field (pre-93845af) — source-unknown;
    //      they are eligible ONLY when tier 1 is empty (no on-disk migration).
    // Reports tagged with a DIFFERENT source never match — that is the
    // shadowing bug this fix removes (a Synthetic run no longer hijacks a
    // Binance tuple's resolution and vice versa).
    let mut best_tagged: Option<(u32, PathBuf)> = None;
    let mut best_untagged: Option<(u32, PathBuf)> = None;
    for path in &reports {
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        let Some(meta) = parse_report_meta(&content) else {
            continue;
        };
        let quality = report_matches(&meta, &tuple.symbol, &tuple.range);
        if let MatchQuality::Match { range_score } = quality {
            let slot = match meta.data_source.as_deref() {
                Some(ds) if source_tag_matches(ds, tuple.source) => &mut best_tagged,
                Some(_) => continue, // tagged with a different source — excluded
                None => &mut best_untagged, // legacy-unknown (tier 2)
            };
            let is_better = slot.as_ref().is_none_or(|(s, _)| range_score > *s);
            if is_better {
                *slot = Some((range_score, path.clone()));
            }
        }
    }

    let Some((_, best_path)) = best_tagged.or(best_untagged) else {
        return Err(EquityLoadError::NoReport {
            strategy: tuple.strategy.clone(),
            symbol: tuple.symbol.clone(),
            range: tuple.range.clone(),
            data_source: tuple.source,
        });
    };

    let content = std::fs::read_to_string(&best_path).map_err(|e| EquityLoadError::ParseError {
        path: best_path.display().to_string(),
        msg: e.to_string(),
    })?;

    let meta = parse_report_meta(&content).ok_or_else(|| EquityLoadError::ParseError {
        path: best_path.display().to_string(),
        msg: "failed to parse report frontmatter".to_string(),
    })?;

    let report_name = best_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Determine narrowed_from: the report is a superset if the scenario name
    // doesn't exactly match the requested range.
    let narrowed_from: Option<SmolStr> = {
        let exact = is_exact_range_match(&meta, &tuple.range);
        if exact {
            None
        } else {
            Some(SmolStr::new(&report_name))
        }
    };

    // Extract equity body.
    let body_start = find_body_start(&content);
    let body = &content[body_start..];

    // lab-run-save-compare Wave-2 (ADR-0055 § D-companion): prefer the companion
    // equity CSV (full per-bar series) if present beside the `.md`. Lab runs
    // persist it, so a saved run hydrates a REAL curve instead of the sparkline
    // start-end 2-point fallback (the `.md`'s `## Equity curve` is only a
    // visual sparkline, not machine-parseable). Older committed `evidence/`
    // reports without a companion stay at their existing fidelity.
    let (samples, fidelity) = if let Some(csv_pts) = load_companion_equity_csv(&best_path) {
        (csv_pts, Fidelity::PerBar)
    } else if meta.has_equity_section {
        let pts = parse_equity_section(body);
        if pts.is_empty() {
            // Fall through to start-end fallback.
            (
                make_start_end_series(&meta, &report_name)?,
                Fidelity::StartEndOnly,
            )
        } else {
            (pts, Fidelity::PerBar)
        }
    } else {
        (
            make_start_end_series(&meta, &report_name)?,
            Fidelity::StartEndOnly,
        )
    };

    if samples.is_empty() {
        return Err(EquityLoadError::EmptySeries {
            path: best_path.display().to_string(),
        });
    }

    Ok(LabEquitySeries {
        samples,
        source_report: SmolStr::new(&report_name),
        fidelity,
        narrowed_from,
    })
}

/// Build a two-point equity series from start/end capital values.
fn make_start_end_series(
    meta: &ReportMeta,
    path: &str,
) -> Result<Vec<(i64, Decimal)>, EquityLoadError> {
    let initial = meta
        .initial_capital
        .ok_or_else(|| EquityLoadError::ParseError {
            path: path.to_string(),
            msg: "missing initial capital in Summary".to_string(),
        })?;
    let final_eq = meta
        .final_equity
        .ok_or_else(|| EquityLoadError::ParseError {
            path: path.to_string(),
            msg: "missing final equity in Summary".to_string(),
        })?;

    // Ordinal timestamps, not times: 0 = start, 1 = end. They are not
    // anchored in time on purpose: a report records neither its first nor
    // its last bar time (only a start year, a bar count and its write time
    // `generated:`), so an anchor would be inferred, and over price bars an
    // inferred span that half-overlaps the window would draw equity the
    // report does not have. With bars on screen both samples therefore miss
    // the bar window and the chart draws no overlay and reserves no gutter
    // (bug-log #207); without bars they span the chart's full width.
    Ok(vec![(0, initial), (1, final_eq)])
}

/// True when the report's range coverage exactly matches the request.
fn is_exact_range_match(meta: &ReportMeta, range: &DateRange) -> bool {
    match range {
        DateRange::Preset(p) => match p {
            Preset::H1_2024 => {
                meta.scenario.contains("2024-h1") || meta.scenario.contains("2024_h1")
            }
            Preset::H2_2024 => {
                meta.scenario.contains("2024-h2") || meta.scenario.contains("2024_h2")
            }
            Preset::Last30d | Preset::Last90d => false, // synthetic ranges never exact
        },
        DateRange::Custom { .. } => false,
    }
}

/// Find the byte offset where the report body starts (after `---` front-matter).
fn find_body_start(content: &str) -> usize {
    let mut dash_count = 0u32;
    let mut pos = 0usize;
    for line in content.lines() {
        pos += line.len() + 1;
        if line.trim() == "---" {
            dash_count += 1;
            if dash_count == 2 {
                return pos;
            }
        }
    }
    0
}

// ── Public helpers ────────────────────────────────────────────────────────────

/// Convenience: resolve the `evidence/` root relative to the workspace
/// `Cargo.toml` (the byte-immutable reports corpus; `spec/` housed it until
/// the 2026-07-25 BMAD-migration Phase 3 `git mv`, layout preserved).
/// Used by production code; tests pass an explicit temp-dir path.
#[must_use]
pub fn default_evidence_root() -> PathBuf {
    // Walk up from the manifest directory to find the workspace root that
    // contains an `evidence/` directory.
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").map_or_else(|_| PathBuf::from("."), PathBuf::from);
    // `crates/ui` → `crates` → workspace root.
    if let Some(root) = manifest_dir.parent().and_then(|p| p.parent()) {
        let candidate = root.join("evidence");
        if candidate.is_dir() {
            return candidate;
        }
    }
    PathBuf::from("evidence")
}

/// Resolve the git-ignored `lab-runs/` root at the workspace root
/// (lab-run-save-compare T2 / Q1 / ADR-0055 § D1).
///
/// Sibling of [`default_evidence_root`]: walks up from `CARGO_MANIFEST_DIR`
/// (`crates/ui` → `crates` → workspace root) and returns `<workspace>/lab-runs`.
/// This is the home the engine's `run_scenario` writes persisted Lab reports
/// to (the developer's write-root) — read-root == write-root is the H3
/// invariant. Unlike `default_evidence_root` this does NOT require the
/// directory to exist: a fresh checkout has no `lab-runs/` until the first
/// Lab run persists, and the loaders fail soft on a missing root (return no
/// series).
#[must_use]
pub fn default_lab_runs_root() -> PathBuf {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").map_or_else(|_| PathBuf::from("."), PathBuf::from);
    // `crates/ui` → `crates` → workspace root.
    if let Some(root) = manifest_dir.parent().and_then(|p| p.parent()) {
        return root.join("lab-runs");
    }
    PathBuf::from("lab-runs")
}

/// The production Lab/Compare read-root union (Q4 / ADR-0055 § D5):
/// `[lab-runs/, evidence/]` — **lab-runs FIRST**. Persisted Lab runs shadow
/// committed `evidence/` reports on a tuple collision (lab-runs wins).
/// Exposed so the Lab screen and the Compare cold-boot wire pass the
/// identical ordered roots.
#[must_use]
pub fn default_report_roots() -> Vec<PathBuf> {
    vec![default_lab_runs_root(), default_evidence_root()]
}

// ── route_equity_overlay (T-D-N11 / R5.1–R5.4) ──────────────────────────────

/// Route the chart equity-overlay data source.
///
/// ## Priority (R5.1 / Design § D4)
///
/// 1. If `lab_state.last_run_report` is `Some` **and** its `tuple` matches
///    `current_tuple`, return a `LabEquitySeries` built from the in-memory
///    `equity_series` (no I/O). The `narrowed_from` field is `None` (exact
///    match by construction — R5.3 suppresses the narrowed-from badge).
/// 2. Otherwise fall through to the cold disk path — the two-root union
///    (`EquityCache::get_or_load_roots`, lab-runs FIRST then evidence/ —
///    lab-run-save-compare T4 / R4 / Q4). After a Lab run persists its report
///    under `lab-runs/`, the curve repaints from disk on the next boot /
///    tuple-select even with the in-memory mirror cleared (AC4). If the cache
///    miss returns an error, returns `None` so the chart renders with no
///    equity overlay (graceful degradation).
///
/// `roots` is searched in order; pass `default_report_roots()` in production.
/// `cache` is mutated only on cache misses (existing Phase A behaviour
/// unchanged).
#[must_use]
pub fn route_equity_overlay(
    lab_state: &crate::lab::state::LabState,
    cache: &mut EquityCache,
    current_tuple: &LabTuple,
    roots: &[PathBuf],
) -> Option<LabEquitySeries> {
    // Hot path: in-memory mirror from the most recent completed run.
    if let Some(ref mirror) = lab_state.last_run_report
        && &mirror.tuple == current_tuple
    {
        // Build LabEquitySeries from the Arc<Vec<(i64, Decimal)>>.
        // `clone()` on Arc is cheap (ref-count increment only).
        let samples: Vec<(i64, rust_decimal::Decimal)> = mirror.equity_series.as_ref().clone();
        if !samples.is_empty() {
            return Some(LabEquitySeries {
                samples,
                source_report: SmolStr::new("in-memory (last run)"),
                fidelity: Fidelity::PerBar,
                narrowed_from: None, // exact match — R5.3 suppresses badge
            });
        }
    }

    // Cold path: the two-root union (lab-runs first, then evidence/).
    cache
        .get_or_load_roots(current_tuple, roots)
        .ok()
        .map(|arc| arc.as_ref().clone())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::needless_raw_string_hashes,
    // Fixture builders only: a `format!` into a growing fixture string and a
    // `usize as i64` bar index. Both are test-local and cannot wrap.
    clippy::format_push_string,
    clippy::cast_possible_wrap
)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Build a minimal fixture backtest report for testing.
    fn write_fixture_report(dir: &std::path::Path, filename: &str, content: &str) {
        let mut f = std::fs::File::create(dir.join(filename)).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    const FIXTURE_REPORT_WITH_EQUITY: &str = r#"---
scenario: top10-2024-h1-momentum
seed: 0xC0FFEE
generated: 2026-04-29T19:52:43Z
wall_clock_s: 2.0
data_source: synthetic
---

# Backtest Report — top10-2024-h1-momentum

## Summary

| Metric               | Value                         |
|----------------------|-------------------------------|
| Scenario             | top10-2024-h1-momentum        |
| Initial capital      | $100000.00 USDT               |
| Final equity         | $46401.41 USDT                |
| Max drawdown         | 87.48%                        |

## Universe

- XRPUSDT

## Equity curve

| Timestamp (ms) | Equity (USDT) |
|---------------|---------------|
| 1704067200000 | 100000.00     |
| 1704153600000 | 101200.50     |
| 1704240000000 | 99800.25      |
| 1704326400000 | 46401.41      |

## Notes

- v1 cross-sectional momentum
"#;

    /// A 2-point `lab-runs/`-style report for the `top10-2024-h1-momentum`
    /// tuple — used to prove the two-root union picks the lab-runs copy over
    /// the 4-point `spec/` copy (the collision rule, T2).
    const LAB_RUNS_REPORT_TWO_POINT: &str = r#"---
scenario: top10-2024-h1-momentum
seed: 0xC0FFEE
generated: 2026-06-01T12:00:00Z
wall_clock_s: 0.0
data_source: synthetic
---

# Backtest Report — top10-2024-h1-momentum

## Summary

| Metric               | Value                         |
|----------------------|-------------------------------|
| Scenario             | top10-2024-h1-momentum        |
| Initial capital      | $100000.00 USDT               |
| Final equity         | $123456.00 USDT               |
| Max drawdown         | 12.00%                        |

## Universe

- XRPUSDT

## Equity curve

| Timestamp (ms) | Equity (USDT) |
|---------------|---------------|
| 1704067200000 | 100000.00     |
| 1719791999000 | 123456.00     |

## Notes

- lab-runs collision fixture
"#;

    const FIXTURE_REPORT_NO_EQUITY: &str = r#"---
scenario: btc-2023-1m-sma-cross
seed: 0xC0FFEE
generated: 2026-04-29T10:00:00Z
wall_clock_s: 0.2
data_source: synthetic
---

# Backtest Report — btc-2023-1m-sma-cross

## Summary

| Metric          | Value             |
|-----------------|-------------------|
| Scenario        | btc-2023-1m-sma-cross |
| Initial capital | $100000.00 USDT   |
| Final equity    | $108540.00 USDT   |

## Universe

- BTCUSDT
"#;

    fn setup_fixture_dir(tmp: &tempfile::TempDir) -> std::path::PathBuf {
        let spec = tmp.path().join("spec");
        let slug_dir = spec.join("v1-cross-sectional-momentum").join("reports");
        std::fs::create_dir_all(&slug_dir).unwrap();
        write_fixture_report(
            &slug_dir,
            "backtest-20260429-195243-top10-2024-h1-momentum.md",
            FIXTURE_REPORT_WITH_EQUITY,
        );
        spec
    }

    /// T-D-10 — loader returns a series with the correct length + start/end
    /// equity from the per-bar fixture.
    #[test]
    fn load_equity_per_bar_fixture() {
        let tmp = tempfile::tempdir().unwrap();
        let spec = setup_fixture_dir(&tmp);

        let tuple = LabTuple {
            strategy: SmolStr::new("v1.momentum"),
            symbol: SmolStr::new("XRPUSDT"),
            range: DateRange::Preset(Preset::H1_2024),
            source: LabDataSource::Synthetic,
        };

        let series = load_equity(&tuple, &spec).unwrap();
        assert_eq!(
            series.fidelity,
            Fidelity::PerBar,
            "expected per-bar fidelity"
        );
        assert_eq!(series.samples.len(), 4, "expected 4 equity points");
        // First sample: 100 000.00
        let (_, first_eq) = series.samples[0];
        assert_eq!(first_eq, Decimal::from(100_000));
        // Last sample: 46 401.41
        let (_, last_eq) = series.samples[3];
        assert_eq!(last_eq, "46401.41".parse::<Decimal>().unwrap());
    }

    /// T-D-10 — loader falls back to start-end-only when no equity section.
    #[test]
    fn load_equity_start_end_fallback() {
        let tmp = tempfile::tempdir().unwrap();
        let spec = tmp.path().join("spec");
        let slug_dir = spec.join("v0-paper-sma").join("reports");
        std::fs::create_dir_all(&slug_dir).unwrap();
        write_fixture_report(
            &slug_dir,
            "backtest-20260429-sma.md",
            FIXTURE_REPORT_NO_EQUITY,
        );

        let tuple = LabTuple {
            strategy: SmolStr::new("v0.sma"),
            symbol: SmolStr::new("BTCUSDT"),
            range: DateRange::Preset(Preset::Last90d),
            source: LabDataSource::Synthetic,
        };

        let series = load_equity(&tuple, &spec).unwrap();
        assert_eq!(
            series.fidelity,
            Fidelity::StartEndOnly,
            "expected start-end-only fidelity"
        );
        assert_eq!(series.samples.len(), 2, "start-end gives exactly 2 points");
        assert_eq!(series.samples[0].1, Decimal::from(100_000));
        assert_eq!(series.samples[1].1, "108540.00".parse::<Decimal>().unwrap());
    }

    /// T-D-10 — cache hit avoids re-parsing.
    #[test]
    fn cache_hit_returns_same_arc() {
        let tmp = tempfile::tempdir().unwrap();
        let spec = setup_fixture_dir(&tmp);

        let tuple = LabTuple {
            strategy: SmolStr::new("v1.momentum"),
            symbol: SmolStr::new("XRPUSDT"),
            range: DateRange::Preset(Preset::H1_2024),
            source: LabDataSource::Synthetic,
        };

        let mut cache = EquityCache::new();
        let first = cache.get_or_load(&tuple, &spec).unwrap();
        let second = cache.get_or_load(&tuple, &spec).unwrap();
        assert!(
            Arc::ptr_eq(&first, &second),
            "expected same Arc on cache hit"
        );
    }

    /// T-D-10 — cache invalidate causes re-read.
    #[test]
    fn cache_invalidate_re_reads() {
        let tmp = tempfile::tempdir().unwrap();
        let spec = setup_fixture_dir(&tmp);

        let tuple = LabTuple {
            strategy: SmolStr::new("v1.momentum"),
            symbol: SmolStr::new("XRPUSDT"),
            range: DateRange::Preset(Preset::H1_2024),
            source: LabDataSource::Synthetic,
        };

        let mut cache = EquityCache::new();
        let first = cache.get_or_load(&tuple, &spec).unwrap();
        cache.invalidate(&tuple);
        let after = cache.get_or_load(&tuple, &spec).unwrap();
        // Contents are the same (same file); Arc pointers differ after invalidation.
        assert!(
            !Arc::ptr_eq(&first, &after),
            "expected new Arc after invalidation"
        );
    }

    /// T-D-10 — `NoReport` error when strategy/pair combo doesn't exist.
    /// The fixture dir exists and has a report but for a different pair.
    #[test]
    fn load_equity_no_report_error() {
        let tmp = tempfile::tempdir().unwrap();
        let spec = tmp.path().join("spec");
        let slug_dir = spec.join("v1-cross-sectional-momentum").join("reports");
        std::fs::create_dir_all(&slug_dir).unwrap();
        // Write a report for XRPUSDT only — ETHUSDT+H2_2024 should be NoReport.
        write_fixture_report(
            &slug_dir,
            "backtest-20260429-195243-top10-2024-h1-momentum.md",
            FIXTURE_REPORT_WITH_EQUITY,
        );

        // XRPUSDT H1_2024 exists above; SOLUSDT H2_2024 has no match.
        let tuple = LabTuple {
            strategy: SmolStr::new("v1.momentum"),
            symbol: SmolStr::new("SOLUSDT"),
            range: DateRange::Preset(Preset::H2_2024),
            source: LabDataSource::Synthetic,
        };

        let result = load_equity(&tuple, &spec);
        assert!(
            matches!(result, Err(EquityLoadError::NoReport { .. })),
            "expected NoReport error, got: {result:?}"
        );
    }

    /// Build a v0-paper-sma report fixture with a parameterized
    /// `data_source:` frontmatter line (`None` → legacy report without the
    /// field) and a parameterized equity-point count so two same-tuple
    /// reports are distinguishable by `samples.len()`.
    fn sma_2024_h1_report(data_source: Option<&str>, equity_points: usize) -> String {
        sma_report(
            "btc-2024-h1-sma-cross",
            data_source,
            equity_points,
            "2026-06-01T12:00:00Z",
        )
    }

    /// [`sma_2024_h1_report`] with the `scenario:` and `generated:` values as
    /// parameters (bug-log #205: the tie-rule fixtures vary both).
    fn sma_report(
        scenario: &str,
        data_source: Option<&str>,
        equity_points: usize,
        generated: &str,
    ) -> String {
        let ds_line = data_source.map_or(String::new(), |ds| format!("data_source: {ds}\n"));
        let mut equity_rows = String::new();
        for i in 0..equity_points {
            equity_rows.push_str(&format!(
                "| {} | {}.00     |\n",
                1_704_067_200_000_i64 + (i as i64) * 86_400_000,
                100_000 + i * 1_000
            ));
        }
        format!(
            "---\n\
             scenario: {scenario}\n\
             seed: 0xC0FFEE\n\
             generated: {generated}\n\
             wall_clock_s: 0.1\n\
             {ds_line}\
             ---\n\
             \n\
             # Backtest Report — {scenario}\n\
             \n\
             ## Summary\n\
             \n\
             | Metric               | Value                         |\n\
             |----------------------|-------------------------------|\n\
             | Scenario             | {scenario:<30}|\n\
             | Initial capital      | $100000.00 USDT               |\n\
             | Final equity         | $103000.00 USDT               |\n\
             \n\
             ## Universe\n\
             \n\
             - BTCUSDT\n\
             \n\
             ## Equity curve\n\
             \n\
             | Timestamp (ms) | Equity (USDT) |\n\
             |---------------|---------------|\n\
             {equity_rows}\
             \n\
             ## Notes\n\
             \n\
             - source-keying fixture\n"
        )
    }

    fn sma_tuple(source: LabDataSource) -> LabTuple {
        LabTuple {
            strategy: SmolStr::new("v0.sma"),
            symbol: SmolStr::new("BTCUSDT"),
            range: DateRange::Preset(Preset::H1_2024),
            source,
        }
    }

    /// Review D1 — THE shadow-proof: same `(strategy, symbol, range)` with a
    /// `data_source: binance` report AND a `data_source: synthetic` report in
    /// the SAME root no longer shadow each other. The Binance tuple resolves
    /// the binance report (4 points), the Synthetic tuple the synthetic one
    /// (2 points), and a source with NO tagged report (Yahoo) gets `NoReport`
    /// — a different-source report never satisfies it.
    #[test]
    fn same_tuple_binance_and_synthetic_resolve_own_reports() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("lab-runs");
        let slug_dir = root.join("v0-paper-sma").join("reports");
        std::fs::create_dir_all(&slug_dir).unwrap();
        write_fixture_report(
            &slug_dir,
            "backtest-20260601-120000-btc-2024-h1-sma-cross.md",
            &sma_2024_h1_report(Some("binance"), 4),
        );
        write_fixture_report(
            &slug_dir,
            "backtest-20260601-130000-btc-2024-h1-sma-cross.md",
            &sma_2024_h1_report(Some("synthetic"), 2),
        );

        let binance = load_equity(&sma_tuple(LabDataSource::BinanceCache), &root)
            .expect("Binance tuple must resolve the binance-tagged report");
        assert_eq!(
            binance.samples.len(),
            4,
            "Binance tuple resolved the wrong report (expected the 4-point binance one)"
        );

        let synthetic = load_equity(&sma_tuple(LabDataSource::Synthetic), &root)
            .expect("Synthetic tuple must resolve the synthetic-tagged report");
        assert_eq!(
            synthetic.samples.len(),
            2,
            "Synthetic tuple resolved the wrong report (expected the 2-point synthetic one)"
        );

        let yahoo = load_equity(&sma_tuple(LabDataSource::YahooCache), &root);
        assert!(
            matches!(yahoo, Err(EquityLoadError::NoReport { .. })),
            "a source with no tagged report must get NoReport, never a \
             different source's report; got: {yahoo:?}"
        );
    }

    /// Review D1 — legacy-unknown rule: a report WITHOUT the `data_source:`
    /// field matches ONLY when no report tagged with the requested source
    /// exists (tier-2 fallback, no on-disk migration).
    #[test]
    fn legacy_untagged_report_only_matches_when_no_tagged_exists() {
        // Root A: ONLY a legacy untagged report → any source resolves it.
        let tmp_a = tempfile::tempdir().unwrap();
        let root_a = tmp_a.path().join("lab-runs");
        let slug_a = root_a.join("v0-paper-sma").join("reports");
        std::fs::create_dir_all(&slug_a).unwrap();
        write_fixture_report(
            &slug_a,
            "backtest-20260101-000000-btc-2024-h1-sma-cross.md",
            &sma_2024_h1_report(None, 3),
        );
        let via_binance = load_equity(&sma_tuple(LabDataSource::BinanceCache), &root_a).expect(
            "legacy untagged report must satisfy a Binance tuple when nothing tagged exists",
        );
        assert_eq!(via_binance.samples.len(), 3);

        // Root B: legacy untagged + a binance-tagged sibling → the tagged one
        // wins for a Binance tuple even though both match the range.
        let tmp_b = tempfile::tempdir().unwrap();
        let root_b = tmp_b.path().join("lab-runs");
        let slug_b = root_b.join("v0-paper-sma").join("reports");
        std::fs::create_dir_all(&slug_b).unwrap();
        write_fixture_report(
            &slug_b,
            "backtest-20260101-000000-btc-2024-h1-sma-cross.md",
            &sma_2024_h1_report(None, 3),
        );
        write_fixture_report(
            &slug_b,
            "backtest-20260601-120000-btc-2024-h1-sma-cross.md",
            &sma_2024_h1_report(Some("binance"), 5),
        );
        let tagged_wins = load_equity(&sma_tuple(LabDataSource::BinanceCache), &root_b)
            .expect("Binance tuple resolves");
        assert_eq!(
            tagged_wins.samples.len(),
            5,
            "the source-tagged report must win over the legacy untagged one"
        );
    }

    /// T-D-10 — `strategy_slug` maps known ids correctly.
    #[test]
    fn strategy_slug_mapping() {
        assert_eq!(strategy_slug("v1.momentum"), "v1-cross-sectional-momentum");
        assert_eq!(
            strategy_slug("top10_momentum_h1"),
            "v1-cross-sectional-momentum"
        );
        assert_eq!(strategy_slug("v0.sma"), "v0-paper-sma");
        assert_eq!(strategy_slug("v0.5.macd"), "v05-composed-strategies");
    }

    /// lab-run-save-compare T2 — `default_lab_runs_root()` resolves a sibling
    /// `lab-runs/` of `evidence/` at the workspace root, and
    /// `default_report_roots()` orders lab-runs FIRST (Q4 / ADR-0055 § D5).
    #[test]
    fn default_roots_order_lab_runs_first() {
        let evidence = default_evidence_root();
        let lab_runs = default_lab_runs_root();
        // Both resolve under the same workspace parent.
        assert_eq!(
            lab_runs.file_name().and_then(|n| n.to_str()),
            Some("lab-runs"),
            "lab-runs root must end in /lab-runs"
        );
        if let (Some(evidence_parent), Some(lr_parent)) = (evidence.parent(), lab_runs.parent()) {
            assert_eq!(
                evidence_parent, lr_parent,
                "evidence/ and lab-runs/ must be siblings at the workspace root"
            );
        }
        let roots = default_report_roots();
        assert_eq!(roots.len(), 2, "union is exactly two roots");
        assert_eq!(roots[0], lab_runs, "lab-runs MUST be searched first");
        assert_eq!(roots[1], evidence, "evidence/ is searched second");
    }

    /// lab-run-save-compare T2 — `get_or_load_roots` resolves a report from a
    /// `lab-runs/` tempdir (the write-root the engine persists to). Proves the
    /// cold loader reaches the Lab-runs home, the AC4 repaint-from-disk seam.
    #[test]
    fn get_or_load_roots_resolves_from_lab_runs() {
        let tmp = tempfile::tempdir().unwrap();
        let lab_runs = tmp.path().join("lab-runs");
        let slug_dir = lab_runs.join("v1-cross-sectional-momentum").join("reports");
        std::fs::create_dir_all(&slug_dir).unwrap();
        write_fixture_report(
            &slug_dir,
            "backtest-20260601-120000-top10-2024-h1-momentum.md",
            FIXTURE_REPORT_WITH_EQUITY,
        );

        let tuple = LabTuple {
            strategy: SmolStr::new("v1.momentum"),
            symbol: SmolStr::new("XRPUSDT"),
            range: DateRange::Preset(Preset::H1_2024),
            source: LabDataSource::Synthetic,
        };
        // lab-runs first; a nonexistent spec/ second — the union still resolves.
        let roots = [lab_runs, PathBuf::from("/nonexistent/spec")];
        let mut cache = EquityCache::new();
        let series = cache
            .get_or_load_roots(&tuple, &roots)
            .expect("lab-runs report must resolve via the union");
        assert_eq!(series.samples.len(), 4);
    }

    // ── route_equity_overlay tests (T-D-N11) ────────────────────────────────

    /// Build a minimal `LabState` with `last_run_report` set to the given
    /// mirror, leaving all other fields at default.
    fn lab_state_with_mirror(
        mirror: crate::lab::runner::RunReportMirror,
    ) -> crate::lab::state::LabState {
        let mut s = crate::lab::state::LabState::default();
        s.last_run_report = Some(mirror);
        s
    }

    /// Build a minimal `RunReportMirror` for the given tuple and equity samples.
    fn make_mirror(
        tuple: LabTuple,
        samples: Vec<(i64, rust_decimal::Decimal)>,
    ) -> crate::lab::runner::RunReportMirror {
        use backtest::engine::BacktestKpis;
        use rust_decimal::Decimal;
        use std::sync::Arc;
        use trading_core::{Money, Usdt};
        crate::lab::runner::RunReportMirror {
            tuple,
            equity_series: Arc::new(samples),
            kpis: BacktestKpis {
                final_equity: Money::<Usdt>::from_decimal(Decimal::from(100_000)),
                initial_equity: Money::<Usdt>::from_decimal(Decimal::from(100_000)),
                max_drawdown: Decimal::ZERO,
                trade_count: 0,
                total_fees: Money::<Usdt>::from_decimal(Decimal::ZERO),
                buys: 0,
                sells: 0,
                total_return_pct: Decimal::ZERO,
            },
            generated_at: time::OffsetDateTime::UNIX_EPOCH,
            bars: Arc::new(Vec::new()),
            position_curve: Arc::new(Vec::new()),
        }
    }

    /// T-D-N11 hot path: when `last_run_report` tuple matches, returns the
    /// in-memory series without touching the disk cache.
    #[test]
    fn route_overlay_hot_path_in_memory() {
        let tuple = LabTuple {
            strategy: SmolStr::new("v1.momentum"),
            symbol: SmolStr::new("XRPUSDT"),
            range: DateRange::Preset(Preset::H1_2024),
            source: LabDataSource::Synthetic,
        };
        let samples = vec![
            (0i64, Decimal::from(100_000)),
            (1i64, Decimal::from(102_000)),
        ];
        let mirror = make_mirror(tuple.clone(), samples.clone());
        let lab_state = lab_state_with_mirror(mirror);
        let mut cache = EquityCache::new();
        // Roots point nowhere — any disk read would fail.
        let roots = [
            PathBuf::from("/nonexistent/lab-runs"),
            PathBuf::from("/nonexistent/spec"),
        ];

        let result = route_equity_overlay(&lab_state, &mut cache, &tuple, &roots);
        let series = result.expect("expected Some from hot path");
        assert_eq!(series.samples.len(), 2, "should have both equity points");
        assert_eq!(series.source_report, SmolStr::new("in-memory (last run)"));
        assert_eq!(series.fidelity, Fidelity::PerBar);
        assert!(
            series.narrowed_from.is_none(),
            "exact match: no narrowed_from badge"
        );
    }

    /// T-D-N11 hot path: when the mirror's tuple does NOT match `current_tuple`,
    /// falls through to the disk cache (returns None when spec root is absent).
    #[test]
    fn route_overlay_hot_path_tuple_mismatch_falls_through() {
        let current_tuple = LabTuple {
            strategy: SmolStr::new("v1.momentum"),
            symbol: SmolStr::new("XRPUSDT"),
            range: DateRange::Preset(Preset::H1_2024),
            source: LabDataSource::Synthetic,
        };
        let mirror_tuple = LabTuple {
            strategy: SmolStr::new("v1.momentum"),
            symbol: SmolStr::new("BTCUSDT"), // different pair
            range: DateRange::Preset(Preset::H1_2024),
            source: LabDataSource::Synthetic,
        };
        let mirror = make_mirror(mirror_tuple, vec![(0, Decimal::from(100_000))]);
        let lab_state = lab_state_with_mirror(mirror);
        let mut cache = EquityCache::new();
        // Roots don't exist → cache miss → None.
        let roots = [
            PathBuf::from("/nonexistent/lab-runs"),
            PathBuf::from("/nonexistent/spec"),
        ];

        let result = route_equity_overlay(&lab_state, &mut cache, &current_tuple, &roots);
        assert!(
            result.is_none(),
            "tuple mismatch must fall through to cache (returns None here)"
        );
    }

    /// T-D-N11 cold path: when no `last_run_report`, uses the two-root union
    /// disk path. Passing `[spec]` (single root) still resolves the report —
    /// existing spec-rooted behaviour is preserved under the slice API (T2).
    #[test]
    fn route_overlay_cold_path_uses_cache() {
        let tmp = tempfile::tempdir().unwrap();
        let spec = setup_fixture_dir(&tmp);
        let tuple = LabTuple {
            strategy: SmolStr::new("v1.momentum"),
            symbol: SmolStr::new("XRPUSDT"),
            range: DateRange::Preset(Preset::H1_2024),
            source: LabDataSource::Synthetic,
        };
        let lab_state = crate::lab::state::LabState::default(); // no last_run_report
        let mut cache = EquityCache::new();

        let roots = [spec];
        let result = route_equity_overlay(&lab_state, &mut cache, &tuple, &roots);
        let series = result.expect("expected Some from disk cache");
        assert_eq!(series.samples.len(), 4, "fixture has 4 equity points");
        assert_eq!(series.fidelity, Fidelity::PerBar);
    }

    /// lab-run-save-compare T2 — the two-root union resolves a report placed
    /// in EITHER a `lab-runs/` root OR a `spec/` root, AND **lab-runs wins on a
    /// tuple collision** (Q4 / ADR-0055 § D5: lab-runs searched FIRST). Builds
    /// the SAME tuple in both roots with DISTINGUISHABLE equity tails, then
    /// asserts the union picks the lab-runs copy.
    #[test]
    fn route_overlay_two_root_union_lab_runs_wins_collision() {
        let tmp = tempfile::tempdir().unwrap();

        // spec/ copy — 4-point fixture (final equity 46401.41).
        let spec = tmp.path().join("spec");
        let spec_slug = spec.join("v1-cross-sectional-momentum").join("reports");
        std::fs::create_dir_all(&spec_slug).unwrap();
        write_fixture_report(
            &spec_slug,
            "backtest-20260101-000000-top10-2024-h1-momentum.md",
            FIXTURE_REPORT_WITH_EQUITY,
        );

        // lab-runs/ copy — SAME tuple, a DIFFERENT (2-point) equity series so
        // the winning root is unambiguous from the sample count.
        let lab_runs = tmp.path().join("lab-runs");
        let lr_slug = lab_runs.join("v1-cross-sectional-momentum").join("reports");
        std::fs::create_dir_all(&lr_slug).unwrap();
        write_fixture_report(
            &lr_slug,
            "backtest-20260601-120000-top10-2024-h1-momentum.md",
            LAB_RUNS_REPORT_TWO_POINT,
        );

        let tuple = LabTuple {
            strategy: SmolStr::new("v1.momentum"),
            symbol: SmolStr::new("XRPUSDT"),
            range: DateRange::Preset(Preset::H1_2024),
            source: LabDataSource::Synthetic,
        };
        let lab_state = crate::lab::state::LabState::default();
        let mut cache = EquityCache::new();

        // Production ordering: lab-runs FIRST, then spec.
        let roots = [lab_runs, spec];
        let series = route_equity_overlay(&lab_state, &mut cache, &tuple, &roots)
            .expect("two-root union must resolve a series");
        assert_eq!(
            series.samples.len(),
            2,
            "lab-runs copy (2-point) must win the collision, not the spec copy (4-point)"
        );
    }

    /// T-D-N11: empty `equity_series` in mirror is not returned (falls through to cache).
    #[test]
    fn route_overlay_empty_in_memory_series_falls_through() {
        let tuple = LabTuple {
            strategy: SmolStr::new("v1.momentum"),
            symbol: SmolStr::new("XRPUSDT"),
            range: DateRange::Preset(Preset::H1_2024),
            source: LabDataSource::Synthetic,
        };
        // Mirror matches tuple but has empty samples.
        let mirror = make_mirror(tuple.clone(), vec![]);
        let lab_state = lab_state_with_mirror(mirror);
        let mut cache = EquityCache::new();
        let roots = [
            PathBuf::from("/nonexistent/lab-runs"),
            PathBuf::from("/nonexistent/spec"),
        ];

        let result = route_equity_overlay(&lab_state, &mut cache, &tuple, &roots);
        // Empty samples → falls through to disk; disk absent → None.
        assert!(result.is_none(), "empty mirror series must fall through");
    }

    // ── bug-log #205: container depth (ADR-0055 D8) ──────────────────────────

    /// A v1 momentum tuple over XRPUSDT, H1 2024 (the
    /// `FIXTURE_REPORT_WITH_EQUITY` scenario).
    fn momentum_tuple(source: LabDataSource) -> LabTuple {
        LabTuple {
            strategy: SmolStr::new("v1.momentum"),
            symbol: SmolStr::new("XRPUSDT"),
            range: DateRange::Preset(Preset::H1_2024),
            source,
        }
    }

    /// Write `content` at `<root>/<rel>/<filename>`, creating dirs.
    fn write_at(root: &std::path::Path, rel: &str, filename: &str, content: &str) {
        let dir = root.join(rel);
        std::fs::create_dir_all(&dir).unwrap();
        write_fixture_report(&dir, filename, content);
    }

    /// The container depth of a fixture's `reports/` dir, counted from its
    /// root-relative path (`<slug>/reports` = 0, `<c>/<slug>/reports` = 1) —
    /// never asked of the subject.
    fn container_depth_of(rel: &str) -> usize {
        rel.split('/').count() - 2
    }

    /// E1 — a report at container depth 1 (`evidence/v1/<slug>/reports/`, the
    /// committed layout since `1395520629dc`) resolves. `load_equity_per_bar_fixture`
    /// is the container-depth-0 side of the pair.
    #[test]
    fn load_equity_resolves_a_container_depth_1_evidence_report() {
        let tmp = tempfile::tempdir().unwrap();
        let evidence = tmp.path().join("evidence");
        write_at(
            &evidence,
            "v1/v1-cross-sectional-momentum/reports",
            "backtest-20260429-195243-top10-2024-h1-momentum.md",
            FIXTURE_REPORT_WITH_EQUITY,
        );
        let series = load_equity(&momentum_tuple(LabDataSource::Synthetic), &evidence)
            .expect("a committed-layout evidence/v1/<slug>/reports report must resolve (#205)");
        assert_eq!(series.fidelity, Fidelity::PerBar);
        assert_eq!(
            series.samples.len(),
            4,
            "the 4-point fixture must resolve from container depth 1 (#205)"
        );
    }

    /// E1b — the other side of E1: container depth 2 is out of contract.
    #[test]
    fn load_equity_does_not_reach_a_container_depth_2_report() {
        let tmp = tempfile::tempdir().unwrap();
        let evidence = tmp.path().join("evidence");
        write_at(
            &evidence,
            "v1/x/v1-cross-sectional-momentum/reports",
            "backtest-20260429-195243-top10-2024-h1-momentum.md",
            FIXTURE_REPORT_WITH_EQUITY,
        );
        assert!(
            evidence
                .join("v1/x/v1-cross-sectional-momentum/reports/backtest-20260429-195243-top10-2024-h1-momentum.md")
                .is_file(),
            "the container-depth-2 fixture must exist, so the negative is not 'file missing'"
        );
        let result = load_equity(&momentum_tuple(LabDataSource::Synthetic), &evidence);
        assert!(
            matches!(result, Err(EquityLoadError::ReportDirNotFound(_))),
            "a report at container depth 2 must NOT resolve (ADR-0055 D8); got {result:?}"
        );
    }

    /// Lab-runs (container depth 0, 2 points) + committed evidence (container
    /// depth 1, 4 points) for the SAME tuple. Returns `(lab_runs, evidence)`.
    fn depth_precedence_scene(tmp: &tempfile::TempDir) -> (PathBuf, PathBuf) {
        let lab_runs = tmp.path().join("lab-runs");
        let evidence = tmp.path().join("evidence");
        write_at(
            &lab_runs,
            "v1-cross-sectional-momentum/reports",
            "backtest-20260601-120000-top10-2024-h1-momentum.md",
            LAB_RUNS_REPORT_TWO_POINT,
        );
        write_at(
            &evidence,
            "v1/v1-cross-sectional-momentum/reports",
            "backtest-20260101-000000-top10-2024-h1-momentum.md",
            FIXTURE_REPORT_WITH_EQUITY,
        );
        (lab_runs, evidence)
    }

    /// E2 — D5 precedence holds with a container-depth-1 evidence report:
    /// lab-runs, searched first, shadows it.
    #[test]
    fn lab_runs_shadows_a_container_depth_1_evidence_report() {
        let tmp = tempfile::tempdir().unwrap();
        let (lab_runs, evidence) = depth_precedence_scene(&tmp);
        let mut cache = EquityCache::new();
        let series = cache
            .get_or_load_roots(
                &momentum_tuple(LabDataSource::Synthetic),
                &[lab_runs, evidence],
            )
            .expect("the union must resolve");
        assert_eq!(
            series.samples.len(),
            2,
            "lab-runs (2-point) must shadow the container-depth-1 evidence report (4-point) — \
             ADR-0055 D5"
        );
    }

    /// E2b — E2's pair: the container-depth-1 evidence report wins when its
    /// root is searched first. Without it, E2 passes under #205 (evidence
    /// unread).
    #[test]
    fn container_depth_1_evidence_report_wins_when_searched_first() {
        let tmp = tempfile::tempdir().unwrap();
        let (lab_runs, evidence) = depth_precedence_scene(&tmp);
        let mut cache = EquityCache::new();
        let series = cache
            .get_or_load_roots(
                &momentum_tuple(LabDataSource::Synthetic),
                &[evidence, lab_runs],
            )
            .expect("the union must resolve");
        assert_eq!(
            series.samples.len(),
            4,
            "with evidence/ searched first its container-depth-1 report (4-point) must win (#205)"
        );
    }

    /// E4 — the within-root tie rule at ONE container depth (ADR-0055 D8):
    /// equal-score reports of one slug under TWO container-depth-1 dirs resolve
    /// to the first dir in path order — not to the newest stamp, not to the
    /// smallest file name, and not to the newest `generated:` (the min-path
    /// fixture carries the OLDEST `generated:`). The winning dir holds ONE
    /// report, so E4 does not pin the order inside one dir: the layout's file
    /// order is LY4's, and the Lab's within-dir pick is bug-log #209's. The
    /// expected report is derived from the fixture table.
    #[test]
    fn equal_score_tie_at_one_container_depth_goes_to_the_first_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let evidence = tmp.path().join("evidence");
        // (reports dir, file name, generated:, equity points): v1 holds the
        // MIDDLE stamp and the OLDEST `generated:`; v2 the oldest stamp, the
        // newest stamp and the newest `generated:`.
        let table: [(&str, &str, &str, usize); 3] = [
            (
                "v1/v0-paper-sma/reports",
                "backtest-20260301-000000-btc-2024-h1-sma-cross.md",
                "2026-01-01T00:00:00Z",
                4,
            ),
            (
                "v2/v0-paper-sma/reports",
                "backtest-20260101-000000-btc-2024-h1-sma-cross.md",
                "2026-05-01T00:00:00Z",
                3,
            ),
            (
                "v2/v0-paper-sma/reports",
                "backtest-20260601-000000-btc-2024-h1-sma-cross.md",
                "2026-09-01T00:00:00Z",
                5,
            ),
        ];
        for (rel, name, generated, n) in table {
            write_at(
                &evidence,
                rel,
                name,
                &sma_report("btc-2024-h1-sma-cross", Some("synthetic"), n, generated),
            );
        }
        let expected = table
            .iter()
            .min_by_key(|(rel, name, _, _)| (container_depth_of(rel), evidence.join(rel), *name))
            .map(|r| r.3)
            .unwrap();
        let newest_stamp = table.iter().max_by_key(|r| r.1).unwrap().3;
        let first_name = table.iter().min_by_key(|r| r.1).unwrap().3;
        let newest_generated = table.iter().max_by_key(|r| r.2).unwrap().3;
        assert!(
            expected != newest_stamp && expected != first_name && expected != newest_generated,
            "the fixture must make every alternative rule pick another report"
        );
        let series = load_equity(&sma_tuple(LabDataSource::Synthetic), &evidence)
            .expect("the slug resolves under both containers");
        assert_eq!(
            series.samples.len(),
            expected,
            "an equal-score tie must go to the first dir in path order ({expected} points), not the newest stamp ({newest_stamp}), the smallest file name \
             ({first_name}) or the newest generated: ({newest_generated})"
        );
    }

    /// E4b — the D8 tie rule ACROSS container depths, two-sided. In each of
    /// two roots the slug sits at container depth 0 AND 1 with EQUAL scores.
    /// In root `a0` the container name sorts BEFORE the slug name, in root `zz`
    /// AFTER it, and the stamps and `generated:` values are arranged so that
    /// full-path order, the newest stamp, the oldest stamp and the newest
    /// `generated:` each pick the container-depth-1 report in at least one
    /// root (asserted on the table). Of those four rules and "(container depth,
    /// path), first best", only the last picks container depth 0 in both —
    /// where HEAD looked before #205. Any rule that tries container depth 0
    /// first passes E4b; E4 pins the dir order within one container depth.
    #[test]
    fn equal_score_tie_across_container_depths_goes_to_container_depth_0() {
        // (reports dir, file name, generated:, equity points)
        type Fx = (&'static str, &'static str, &'static str, usize);
        let roots: [(&str, [Fx; 2]); 2] = [
            (
                "root-a0",
                [
                    (
                        "v0-paper-sma/reports",
                        "backtest-20260101-000000-btc-2024-h1-sma-cross.md",
                        "2026-01-01T00:00:00Z",
                        3,
                    ),
                    (
                        "a0/v0-paper-sma/reports",
                        "backtest-20260601-000000-btc-2024-h1-sma-cross.md",
                        "2026-06-01T00:00:00Z",
                        5,
                    ),
                ],
            ),
            (
                "root-zz",
                [
                    (
                        "v0-paper-sma/reports",
                        "backtest-20260601-000000-btc-2024-h1-sma-cross.md",
                        "2026-01-01T00:00:00Z",
                        3,
                    ),
                    (
                        "zz/v0-paper-sma/reports",
                        "backtest-20260101-000000-btc-2024-h1-sma-cross.md",
                        "2026-06-01T00:00:00Z",
                        5,
                    ),
                ],
            ),
        ];
        let tmp = tempfile::tempdir().unwrap();
        // Per root: the points each candidate rule picks.
        let mut d0_points = Vec::new();
        let mut full_path = Vec::new();
        let mut newest_stamp = Vec::new();
        let mut oldest_stamp = Vec::new();
        let mut newest_generated = Vec::new();
        for (root_name, fx) in &roots {
            let root = tmp.path().join(root_name);
            for (rel, name, generated, n) in fx {
                write_at(
                    &root,
                    rel,
                    name,
                    &sma_report("btc-2024-h1-sma-cross", Some("synthetic"), *n, generated),
                );
            }
            let d0 = fx.iter().find(|r| container_depth_of(r.0) == 0).unwrap();
            assert!(
                fx.iter().any(|r| container_depth_of(r.0) == 1),
                "each root holds the slug at container depth 0 AND 1"
            );
            d0_points.push(d0.3);
            full_path.push(
                fx.iter()
                    .min_by_key(|r| root.join(r.0).join(r.1))
                    .unwrap()
                    .3,
            );
            newest_stamp.push(fx.iter().max_by_key(|r| r.1).unwrap().3);
            oldest_stamp.push(fx.iter().min_by_key(|r| r.1).unwrap().3);
            newest_generated.push(fx.iter().max_by_key(|r| r.2).unwrap().3);
        }
        for (rule, picks) in [
            ("full-path order", &full_path),
            ("newest stamp", &newest_stamp),
            ("oldest stamp", &oldest_stamp),
            ("newest generated:", &newest_generated),
        ] {
            assert_ne!(
                picks, &d0_points,
                "the fixture must make `{rule}` miss container depth 0 in at least one root"
            );
        }
        for ((root_name, _), want) in roots.iter().zip(&d0_points) {
            let series = load_equity(
                &sma_tuple(LabDataSource::Synthetic),
                &tmp.path().join(root_name),
            )
            .expect("the slug resolves in both roots");
            assert_eq!(
                series.samples.len(),
                *want,
                "{root_name}: an equal-score tie across container depths must go to the \
                 container-depth-0 report ({want} points) — ADR-0055 D8 tie rule (#205)"
            );
        }
    }

    /// E4c — the within-root UNION is complete (falsifier O3): the strictly
    /// better report sits in the SECOND dir of the (container depth, path)
    /// order, so a reader that stops after the first dir picks the worse one.
    /// Range `Last90d`: a `2024` scenario covers it better than a `2023` one.
    #[test]
    fn a_strictly_better_report_in_the_second_report_dir_wins() {
        let tmp = tempfile::tempdir().unwrap();
        let evidence = tmp.path().join("evidence");
        // (reports dir, file name, scenario, equity points); the 2024 row wins.
        let table: [(&str, &str, &str, usize); 2] = [
            (
                "v1/v0-paper-sma/reports",
                "backtest-20260601-000000-btc-2023-sma-cross.md",
                "btc-2023-sma-cross",
                4,
            ),
            (
                "v2/v0-paper-sma/reports",
                "backtest-20260101-000000-btc-2024-sma-cross.md",
                "btc-2024-sma-cross",
                3,
            ),
        ];
        for (rel, name, scenario, n) in table {
            write_at(
                &evidence,
                rel,
                name,
                &sma_report(scenario, Some("synthetic"), n, "2026-06-01T12:00:00Z"),
            );
        }
        let better = table.iter().find(|r| r.2.contains("2024")).unwrap();
        let first_dir = table
            .iter()
            .min_by_key(|r| (container_depth_of(r.0), evidence.join(r.0)))
            .unwrap();
        assert_ne!(
            better.0, first_dir.0,
            "the better report must sit in the SECOND dir, else E4c pins nothing"
        );
        assert_ne!(
            better.3, first_dir.3,
            "the two reports must differ in point count, else E4c cannot tell which won"
        );
        let tuple = LabTuple {
            strategy: SmolStr::new("v0.sma"),
            symbol: SmolStr::new("BTCUSDT"),
            range: DateRange::Preset(Preset::Last90d),
            source: LabDataSource::Synthetic,
        };
        let series = load_equity(&tuple, &evidence).expect("the slug resolves");
        assert_eq!(
            series.samples.len(),
            better.3,
            "the better-scoring report in the second report dir must win; every dir of the \
             slug is read (#205)"
        );
    }

    /// E5 — `ReportDirNotFound` names the slug, the root and the container
    /// depth bound, and tells its two empties apart: no `reports/` dir for the
    /// slug at all, vs a container-depth-1 dir that holds no `backtest-*.md`.
    #[test]
    fn report_dir_not_found_names_slug_root_bound_and_which_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let slug = "v1-cross-sectional-momentum";
        let bound = format!("container depth ≤ {}", layout::MAX_CONTAINER_DEPTH);
        let absent = tmp.path().join("empty-root");
        std::fs::create_dir_all(&absent).unwrap();
        let with_dir = tmp.path().join("dir-root");
        write_at(
            &with_dir,
            &format!("v1/{slug}/reports"),
            "test-week2-smoke.md",
            FIXTURE_REPORT_WITH_EQUITY,
        );
        for (root, which, other) in [
            (&absent, "no `reports/` dir", "hold no `backtest-*.md`"),
            (&with_dir, "hold no `backtest-*.md`", "no `reports/` dir"),
        ] {
            let Err(EquityLoadError::ReportDirNotFound(msg)) =
                load_equity(&momentum_tuple(LabDataSource::Synthetic), root)
            else {
                panic!("{} must give ReportDirNotFound", root.display());
            };
            let root_text = root.display().to_string();
            let parts: [&str; 4] = [slug, &root_text, &bound, which];
            for part in parts {
                assert!(
                    msg.contains(part),
                    "ReportDirNotFound for {} must say `{part}`; got: {msg}",
                    root.display()
                );
            }
            assert!(
                !msg.contains(other),
                "ReportDirNotFound for {} must NOT say the other empty's `{other}`; got: {msg}",
                root.display()
            );
        }
    }

    /// E3 — the REAL committed v1 momentum report resolves through the
    /// production `evidence/` root (it sits at `evidence/v1/<slug>/reports/`).
    /// Was gated on the flat path and accepted `NoReport`, so it had skipped
    /// since `1395520629dc` (bug-log #205). Every expected value is derived
    /// from the git listing (the shared evidence oracle: tracked, or untracked
    /// on disk) and the committed file, not from the loader.
    #[test]
    fn integration_load_real_v1_report() {
        use crate::test_support::evidence_oracle;
        let evidence = default_evidence_root();
        let ws = evidence
            .parent()
            .expect("evidence/ sits under the workspace root")
            .to_path_buf();
        let files = evidence_oracle::git_listed_files(&mut evidence_oracle::live_git(&ws))
            .unwrap_or_else(|e| panic!("the #205 evidence oracle must run: {e}"));
        let hits: Vec<&Vec<String>> = files
            .iter()
            .filter(|c| {
                evidence_oracle::is_contract_report(c)
                    && c[c.len() - 3] == "v1-cross-sectional-momentum"
                    && c[c.len() - 1].contains("-2024-h1-")
            })
            .collect();
        assert_eq!(
            hits.len(),
            1,
            "exactly one git-listed (tracked or on-disk) v1-cross-sectional-momentum 2024-h1 \
             report must exist: a second one (e.g. a backtest-CLI twin at container depth 0) \
             changes which report the Lab resolves (ADR-0055 D8 tie rule), so re-derive E3; \
             got {hits:?}"
        );
        let report = hits[0].iter().fold(ws.clone(), |p, c| p.join(c));
        let body = std::fs::read_to_string(&report).expect("the committed report reads");
        let name = report.file_name().unwrap().to_str().unwrap().to_string();
        let stem = name.strip_suffix(".md").unwrap();
        let has_section = body.contains("## Equity curve") || body.contains("## Equity Curve");
        let has_csv = report
            .with_file_name(format!("{stem}-equity.csv"))
            .is_file();
        assert!(
            !has_section && !has_csv,
            "the committed momentum report is start/end-only (no ## Equity curve, no \
             -equity.csv); if that changed, re-derive E3's fidelity"
        );

        let series = load_equity(&momentum_tuple(LabDataSource::Synthetic), &evidence)
            .unwrap_or_else(|e| {
                panic!("the committed v1 momentum report must resolve (#205): {e}")
            });
        assert_eq!(series.source_report, SmolStr::new(&name));
        assert_eq!(series.fidelity, Fidelity::StartEndOnly);
        assert_eq!(
            series.samples.len(),
            2,
            "start/end-only gives exactly 2 points"
        );
        assert!(
            series.samples[0].1 > Decimal::ZERO,
            "start equity must be positive"
        );
    }

    /// E3b — E3's pair: the SAME committed slug with a `BinanceCache` tuple
    /// gives `NoReport` — the dir IS found and the synthetic-tagged reports
    /// are filtered by source. Under #205 it gave `ReportDirNotFound`.
    #[test]
    fn integration_real_v1_report_binance_tuple_is_no_report() {
        let evidence = default_evidence_root();
        let result = load_equity(&momentum_tuple(LabDataSource::BinanceCache), &evidence);
        assert!(
            matches!(result, Err(EquityLoadError::NoReport { .. })),
            "the committed v1 momentum dir must be FOUND and its synthetic reports filtered \
             (NoReport), not ReportDirNotFound (#205); got {result:?}"
        );
    }
}
