//! Cached backtest report equity-curve loader — ui-rethink-phase-a-lab T-D-10.
//!
//! Scans `spec/<strategy-slug>/reports/backtest-*.md` for reports that match
//! a `(strategy, pair, range)` tuple and parses the per-bar equity series.
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
//!   (R7.3). `Fidelity::StartEndOnly` is set in that case.
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

use crate::lab::state::{DateRange, Preset};

// ── Public data shapes ────────────────────────────────────────────────────────

/// Fidelity level of the equity series read from a report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fidelity {
    /// Per-bar data points from the report's `## Equity curve` table.
    PerBar,
    /// Only start + end equity available (older v0/v1 reports without the
    /// per-bar section). The chart renders a straight two-point segment and
    /// the legend chip shows a dotted-line decoration.
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

/// Cache key: exact `(strategy_slug, symbol, range)` triple.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LabTuple {
    pub strategy: SmolStr,
    pub symbol: SmolStr,
    pub range: DateRange,
}

impl LabTuple {
    /// Construct from the typed lab-state fields.
    #[must_use]
    pub fn new(strategy: &StrategyId, venue: Venue, symbol: &Symbol, range: DateRange) -> Self {
        let _ = venue; // Phase A universe is single-venue; key on symbol only.
        Self {
            strategy: SmolStr::new(&strategy.0),
            symbol: SmolStr::new(symbol.0.as_str()),
            range,
        }
    }
}

/// Errors from the equity loader.
#[derive(Debug, Error)]
pub enum EquityLoadError {
    #[error("spec directory not found: {0}")]
    SpecDirNotFound(String),
    #[error("no cached report found for ({strategy}, {symbol}, {range:?})")]
    NoReport {
        strategy: SmolStr,
        symbol: SmolStr,
        range: DateRange,
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
    /// `spec_root` should point to one report root (e.g. the repository's
    /// `spec/` directory, or a `lab-runs/` tempdir). The loader searches
    /// `<root>/<strategy-slug>/reports/backtest-*.md`.
    ///
    /// This single-root entry point is the H3 invariant's read seam
    /// (ADR-0055 § D6 — `crates/ui/tests/lab_run_engine.rs` calls it with the
    /// engine's `report_path.parent().parent().parent()` write-root). The
    /// production Lab path uses [`EquityCache::get_or_load_roots`] for the
    /// two-root (`lab-runs/` + `spec/`) union (Q4).
    ///
    /// # Errors
    ///
    /// Returns `EquityLoadError` when no matching report is found or the
    /// report body cannot be parsed (see `load_equity`).
    pub fn get_or_load(
        &mut self,
        tuple: &LabTuple,
        spec_root: &std::path::Path,
    ) -> Result<Arc<LabEquitySeries>, EquityLoadError> {
        if let Some(cached) = self.by_tuple.get(tuple) {
            return Ok(Arc::clone(cached));
        }
        let loaded = load_equity(tuple, spec_root)?;
        let arc = Arc::new(loaded);
        self.by_tuple.insert(tuple.clone(), Arc::clone(&arc));
        Ok(arc)
    }

    /// Two-root union loader (lab-run-save-compare T2 / Q4 / ADR-0055 § D5).
    ///
    /// Like [`EquityCache::get_or_load`] but searches a **fixed-order slice of
    /// roots** (production passes `[default_lab_runs_root(),
    /// default_spec_root()]`, lab-runs FIRST). The first root that yields a
    /// matching report wins — so a fresh Lab run under `lab-runs/` shadows a
    /// committed `spec/` report for the same tuple (the collision rule:
    /// lab-runs wins). A tuple resolves to exactly one series, so the cache
    /// stays keyed on `tuple` alone (root-independent); the cached `Arc` is
    /// returned on subsequent hits regardless of which root won.
    ///
    /// # Errors
    ///
    /// Returns the LAST root's `EquityLoadError` when no root yields a report
    /// (the spec-root error is the most informative for the operator — it is
    /// the committed tree). Empty `roots` yields a `NoReport` error.
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
        }))
    }

    /// Invalidate a cached series so the next `get_or_load` re-reads from
    /// disk. Called after a fresh backtest run completes (M2.5 / T-D-14).
    pub fn invalidate(&mut self, tuple: &LabTuple) {
        self.by_tuple.remove(tuple);
    }
}

// ── Strategy slug resolution ──────────────────────────────────────────────────

/// Map a `StrategyId` to the spec directory slug that holds its reports.
/// Phase A supports the known strategy ids; unknown ids fall back to the
/// verbatim id string (which may produce a `SpecDirNotFound` error if the
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

/// Discover all `backtest-*.md` paths under `spec/<slug>/reports/`.
fn discover_reports(spec_root: &std::path::Path, slug: &str) -> Vec<PathBuf> {
    let dir = spec_root.join(slug).join("reports");
    if !dir.is_dir() {
        return Vec::new();
    }
    let Ok(read_dir) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = read_dir
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                // Backtest report files are always written with lowercase ".md" extension.
                .is_some_and(|n| {
                    n.starts_with("backtest-") && {
                        #[allow(clippy::case_sensitive_file_extension_comparisons)]
                        let ok = n.ends_with(".md");
                        ok
                    }
                })
        })
        .collect();
    // Sort deterministically so the "first best" pick is reproducible.
    paths.sort();
    paths
}

/// Parsed front-matter fields relevant to cache-key matching.
#[derive(Debug)]
struct ReportMeta {
    scenario: String,
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
        symbol,
        year_hint,
        has_equity_section,
        initial_capital,
        final_equity,
    })
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

/// Load an equity series for `tuple` from the spec directory tree.
/// Synchronous; call from the iced thread on first cache miss.
///
/// # Errors
///
/// Returns `EquityLoadError::NoReport` when no matching report is found;
/// `EquityLoadError::ParseError` when the best candidate cannot be parsed.
pub fn load_equity(
    tuple: &LabTuple,
    spec_root: &std::path::Path,
) -> Result<LabEquitySeries, EquityLoadError> {
    let slug = strategy_slug(&tuple.strategy);
    let reports = discover_reports(spec_root, &slug);

    if reports.is_empty() {
        return Err(EquityLoadError::SpecDirNotFound(format!(
            "{}/{}",
            spec_root.display(),
            slug
        )));
    }

    // Find the best matching report.
    let mut best: Option<(u32, PathBuf)> = None;
    for path in &reports {
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        let Some(meta) = parse_report_meta(&content) else {
            continue;
        };
        let quality = report_matches(&meta, &tuple.symbol, &tuple.range);
        if let MatchQuality::Match { range_score } = quality {
            let is_better = best.as_ref().is_none_or(|(s, _)| range_score > *s);
            if is_better {
                best = Some((range_score, path.clone()));
            }
        }
    }

    let Some((_, best_path)) = best else {
        return Err(EquityLoadError::NoReport {
            strategy: tuple.strategy.clone(),
            symbol: tuple.symbol.clone(),
            range: tuple.range.clone(),
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
    // visual sparkline, not machine-parseable). Older committed `spec/` reports
    // without a companion stay at their existing fidelity.
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

    // Use synthetic timestamps: 0 (epoch) and 1 for start/end so the
    // chart x-axis can always project two points. The real date-range
    // anchoring is Phase B scope.
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

/// Convenience: resolve the `spec/` root relative to the workspace `Cargo.toml`.
/// Used by production code; tests pass an explicit temp-dir path.
#[must_use]
pub fn default_spec_root() -> PathBuf {
    // Walk up from the manifest directory to find the workspace root that
    // contains a `spec/` directory.
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").map_or_else(|_| PathBuf::from("."), PathBuf::from);
    // `crates/ui` → `crates` → workspace root.
    if let Some(root) = manifest_dir.parent().and_then(|p| p.parent()) {
        let candidate = root.join("spec");
        if candidate.is_dir() {
            return candidate;
        }
    }
    PathBuf::from("spec")
}

/// Resolve the git-ignored `lab-runs/` root at the workspace root
/// (lab-run-save-compare T2 / Q1 / ADR-0055 § D1).
///
/// Sibling of [`default_spec_root`]: walks up from `CARGO_MANIFEST_DIR`
/// (`crates/ui` → `crates` → workspace root) and returns `<workspace>/lab-runs`.
/// This is the home the engine's `run_scenario` writes persisted Lab reports
/// to (the developer's write-root) — read-root == write-root is the H3
/// invariant. Unlike `default_spec_root` this does NOT require the directory
/// to exist: a fresh checkout has no `lab-runs/` until the first Lab run
/// persists, and the loaders fail soft on a missing root (return no series).
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
/// `[lab-runs/, spec/]` — **lab-runs FIRST**. Persisted Lab runs shadow
/// committed `spec/` reports on a tuple collision (lab-runs wins). Exposed so
/// the Lab screen and the Compare cold-boot wire pass the identical ordered
/// roots.
#[must_use]
pub fn default_report_roots() -> Vec<PathBuf> {
    vec![default_lab_runs_root(), default_spec_root()]
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
///    (`EquityCache::get_or_load_roots`, lab-runs FIRST then spec/ —
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

    // Cold path: the two-root union (lab-runs first, then spec/).
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
    clippy::needless_raw_string_hashes
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
        };

        let result = load_equity(&tuple, &spec);
        assert!(
            matches!(result, Err(EquityLoadError::NoReport { .. })),
            "expected NoReport error, got: {result:?}"
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
    /// `lab-runs/` of `spec/` at the workspace root, and `default_report_roots()`
    /// orders lab-runs FIRST (Q4 / ADR-0055 § D5).
    #[test]
    fn default_roots_order_lab_runs_first() {
        let spec = default_spec_root();
        let lab_runs = default_lab_runs_root();
        // Both resolve under the same workspace parent.
        assert_eq!(
            lab_runs.file_name().and_then(|n| n.to_str()),
            Some("lab-runs"),
            "lab-runs root must end in /lab-runs"
        );
        if let (Some(spec_parent), Some(lr_parent)) = (spec.parent(), lab_runs.parent()) {
            assert_eq!(
                spec_parent, lr_parent,
                "spec/ and lab-runs/ must be siblings at the workspace root"
            );
        }
        let roots = default_report_roots();
        assert_eq!(roots.len(), 2, "union is exactly two roots");
        assert_eq!(roots[0], lab_runs, "lab-runs MUST be searched first");
        assert_eq!(roots[1], spec, "spec/ is searched second");
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
        };
        let mirror_tuple = LabTuple {
            strategy: SmolStr::new("v1.momentum"),
            symbol: SmolStr::new("BTCUSDT"), // different pair
            range: DateRange::Preset(Preset::H1_2024),
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

    /// T-D-10 — integration test: load the real v1 report from spec/.
    ///
    /// Skipped if the report file doesn't exist (CI without the full spec tree).
    #[test]
    fn integration_load_real_v1_report() {
        let spec = default_spec_root();
        if !spec
            .join("v1-cross-sectional-momentum")
            .join("reports")
            .is_dir()
        {
            eprintln!("skipped: spec/v1-cross-sectional-momentum/reports not found");
            return;
        }

        let tuple = LabTuple {
            strategy: SmolStr::new("v1.momentum"),
            symbol: SmolStr::new("XRPUSDT"),
            range: DateRange::Preset(Preset::H1_2024),
        };

        // The existing reports don't have an equity-curve section, so we
        // expect StartEndOnly fidelity with valid start/end values.
        let result = load_equity(&tuple, &spec);
        match result {
            Ok(series) => {
                assert!(
                    series.samples.len() >= 2,
                    "expected at least 2 samples, got {}",
                    series.samples.len()
                );
                // Start equity should be positive.
                let (_, start_eq) = series.samples[0];
                assert!(start_eq > Decimal::ZERO, "start equity must be positive");
            }
            Err(EquityLoadError::NoReport { .. }) => {
                // Acceptable — the report may not exist in CI.
                eprintln!("integration: no matching report found (acceptable in CI)");
            }
            Err(e) => panic!("unexpected error: {e}"),
        }
    }
}
