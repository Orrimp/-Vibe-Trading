//! Cached backtest report equity-curve loader — ui-rethink-phase-a-lab T-D-10.
//!
//! Scans `<root>[/<container>]/<strategy-slug>/reports/backtest-*.md` (the
//! `crate::reports::layout` contract, ADR-0055 D8 / bug-log #205) under each
//! report root (`lab-runs/`, `evidence/`) for reports that match a
//! `(strategy, pair, range, source)` tuple and parses the per-bar equity
//! series. The slug is the engine's own write map
//! (`backtest::engine::strategy_dir_slug`, ADR-0055 § D1).
//!
//! ## The match (ADR-0055 D9, bug-log #212)
//!
//! A report answers a tuple only if all three hold:
//! - **Strategy:** `backtest::engine::report_strategy_id` of the report's
//!   `strategy:` `id:` equals that of the tuple's strategy, both `Some`. A
//!   report with no `id:` answers no tuple.
//! - **Coin:** the report's coin is its `## Summary` `| Symbol |` row, else its
//!   `## Universe` list, else a ticker in its scenario name; a report naming
//!   no coin answers no tuple.
//! - **Range:** the scenario name's span token (`last30d`, `last90d`,
//!   `<year>-h1`, `<year>-h2`, `<year>`, `custom<start_ms>to<end_ms>`) is the
//!   tuple's range exactly, or, for `H1_2024`/`H2_2024`, a whole-year `2024`
//!   report, which answers as a narrowed superset.
//!
//! Then the source tier: a report tagged with the tuple's `data_source:` beats
//! an untagged legacy one, and a differently tagged one never matches.
//!
//! ## The pick (ADR-0055 § D5, bug-log #209)
//!
//! Over the union of both roots the Lab keeps the matching report with the
//! greatest key (tier, range score, `generated:`): a tagged report beats an
//! untagged one, an exact range beats a superset, and then the newest
//! `generated:` wins, whichever root holds it. An identical filename keeps its
//! first copy in walk order (`lab-runs/` before `evidence/`, container depth 0
//! before 1): a later same-named copy is skipped before it is read, as Compare
//! does (§ D5). An equal key goes to the root searched first (`lab-runs/`),
//! then to the first report in D8 order. The candidates are read in that
//! order and the first that reads wins, so a broken report falls through to
//! the next candidate, in its own root or in the next, but never across the
//! source tier: an untagged report is eligible only when no tagged one
//! matches (ADR-0055 D9), so a broken tagged report leaves the tuple with no
//! curve. Inside the tier a broken exact report does fall through to a
//! readable superset, which the `narrowed_from` badge discloses. A report
//! whose companion CSV is present but does not read, or holds no sample,
//! counts as broken; one with no companion CSV reads from its `.md`. A report
//! with no parseable `generated:` ranks below any dated one.
//!
//! ## Design notes (Design § 4.3 / R7)
//!
//! - Reads are **synchronous** (< 50 KB files, < 16 ms on a cold cache for a
//!   90-day report on the operator's 3360×1890 machine).
//! - Per-tuple in-memory cache (`EquityCache`) avoids re-parsing on every
//!   paint.
//! - Superset fallback: an `H1_2024`/`H2_2024` tuple with no exact-range
//!   report takes a whole-year `2024` one, with a `narrowed_from` annotation
//!   (R5.4 / R7.2). No other range has a superset: a report does not record
//!   its span beyond its scenario name.
//! - Low-fidelity fallback: if the report has no `## Equity curve` table, a
//!   two-point segment (start → end) is synthesised from the Summary block
//!   (R7.3). `Fidelity::StartEndOnly` is set in that case; its ordinal
//!   timestamps make it draw only on the chart's bar-less path (bug-log #207).
//!
//! **I/O on the iced thread: once per tuple per cache generation.** A tuple's
//! first lookup walks the roots synchronously from `lab::view`; its outcome
//! (series OR error) is cached, so later views of it touch no file (the roots
//! are resolved once, into `Cockpit::lab_report_roots`). Every
//! `Message::LabRunCompleted` clears the cache. A report written by anything
//! else while the app runs is seen by a tuple's first lookup of the current
//! generation; a tuple already looked up since the last clear does not see it
//! until the next completed run or a restart (bug-log #208).
//!
//! **Zero string literals in this module** — all user-visible copy lives in
//! `crate::strings`.

use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
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
#[derive(Debug, Clone, Error)]
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
/// Caches each tuple's outcome: a hit (the series) or a miss (the error), so
/// a tuple that resolves nothing costs one walk, not one per view. Emptied
/// whole by [`EquityCache::clear`], which `state::update` calls on every
/// `Message::LabRunCompleted` (bug-log #208).
#[derive(Debug, Default, Clone)]
pub struct EquityCache {
    by_tuple: HashMap<LabTuple, Result<Arc<LabEquitySeries>, EquityLoadError>>,
}

impl EquityCache {
    /// Create a new empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the tuple's cached outcome if present, otherwise load it
    /// synchronously from a SINGLE disk root, cache the outcome (series or
    /// error), and return it.
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
    /// report body cannot be parsed (see `load_equity`). A cached miss
    /// returns a clone of that error without touching the root.
    pub fn get_or_load(
        &mut self,
        tuple: &LabTuple,
        report_root: &std::path::Path,
    ) -> Result<Arc<LabEquitySeries>, EquityLoadError> {
        let roots = [report_root.to_path_buf()];
        // One root: the walk's error is that root's, as before.
        self.get_or_load_roots(tuple, &roots)
    }

    /// Two-root union loader (lab-run-save-compare T2 / Q4 / ADR-0055 § D5).
    ///
    /// Like [`EquityCache::get_or_load`] but searches a **fixed-order slice of
    /// roots** (production passes `[default_lab_runs_root(),
    /// default_evidence_root()]`, lab-runs FIRST), through `resolve_roots`.
    /// Every matching report of every root is a candidate
    /// (`matching_candidates`); over the union the greatest key (tier, range
    /// score, `generated:`) wins, so the newest run of a tuple is shown
    /// whichever root holds it (§ D5, bug-log #209). One filename set spans
    /// the walk of every root, so an identical filename keeps its first copy
    /// (`lab-runs/` before `evidence/`, container depth 0 before 1) and a
    /// later same-named copy is skipped before it is read, whatever its
    /// `generated:` (§ D5's identical-filename rule, as in Compare). An equal
    /// key goes to the root searched first, then to the first report in D8
    /// order. The candidates are read in that order and the first that reads
    /// wins, so a broken report falls through to the next candidate of its
    /// source tier, in the same root or in the next one. A tuple resolves to
    /// exactly one series, so the cache stays keyed on `tuple` alone
    /// (root-independent); the cached `Arc` is returned on subsequent hits
    /// regardless of which root won.
    ///
    /// # Errors
    ///
    /// Returns the LAST root's `EquityLoadError` when no root yields a report
    /// (the evidence-root error is the most informative for the operator — it
    /// is the committed tree): its walk's error, or the read error of its last
    /// candidate tried, else `NoReport` (see `resolve_roots`). Empty `roots`
    /// yields a `NoReport` error. A cached miss returns a clone of that error
    /// without touching the roots.
    pub fn get_or_load_roots(
        &mut self,
        tuple: &LabTuple,
        roots: &[PathBuf],
    ) -> Result<Arc<LabEquitySeries>, EquityLoadError> {
        if let Some(cached) = self.by_tuple.get(tuple) {
            return cached.clone();
        }
        let resolved = resolve_roots(tuple, roots).map(Arc::new);
        self.by_tuple.insert(tuple.clone(), resolved.clone());
        resolved
    }

    /// Drop every cached outcome, hits and misses (bug-log #208).
    pub fn clear(&mut self) {
        self.by_tuple.clear();
    }
}

// ── Strategy slug resolution ──────────────────────────────────────────────────

/// Map a `StrategyId` to the directory slug that holds its reports: the
/// engine's own write map, `backtest::engine::strategy_dir_slug` (ADR-0055 §
/// D1). Until bug-log #212 this was a second, shorter table, and five of the
/// eight Lab chips fell through it to a raw-id directory the engine never
/// writes. Unknown ids fall back to the verbatim id string (which may produce a
/// `ReportDirNotFound` error if the directory doesn't exist).
#[must_use]
fn strategy_slug(id: &str) -> SmolStr {
    SmolStr::new(backtest::engine::strategy_dir_slug(id))
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
/// concatenated in that order and NEVER re-sorted by full path. So
/// `resolve_roots`' stable sort keeps a container-depth-0 report (the only
/// place the walk looked before #205) on an equal (tier, range score,
/// `generated:`) tie with a container-depth-1 one, whether the container name
/// sorts before or after the slug name (bug-log #209: between distinct
/// filenames a strictly newer `generated:` wins at either depth; an identical
/// filename keeps the container-depth-0 copy, F2). Pinned by E4b
/// (`equal_generated_tie_across_container_depths_goes_to_container_depth_0`).
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

/// The coins a report covers (bug-log #212), in the order they are read.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Coins {
    /// The `## Summary` `| Symbol |` row (every single-symbol engine and
    /// committed report writes it), or, without one, a ticker in the
    /// scenario name.
    One(String),
    /// The `## Universe` list (the cross-sectional reports).
    Universe(Vec<String>),
    /// No coin found: the report answers no tuple.
    Unknown,
}

impl Coins {
    /// Whether the report covers `sym` (ASCII case-insensitive).
    fn covers(&self, sym: &str) -> bool {
        match self {
            Coins::One(c) => c.eq_ignore_ascii_case(sym),
            Coins::Universe(list) => list.iter().any(|c| c.eq_ignore_ascii_case(sym)),
            Coins::Unknown => false,
        }
    }
}

/// The span a report's scenario name states (bug-log #212): its first
/// `-`/`_`-separated token that is `last30d`, `last90d`,
/// `custom<start_ms>to<end_ms>` or a four-digit year (a following `h1`/`h2`
/// token makes the year a half). The engine writes these tokens
/// (`backtest::engine::lab_scenario_name` / `lab_universe_scenario_name`), and
/// the committed reports' names carry a year (`btc-2023-1m-…`,
/// `top10-2024-h1-…`). A pre-1970 custom range, whose `-` sign splits its
/// token, reads as `Unknown`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Span {
    Last30d,
    Last90d,
    /// `(year, half)`, `half` ∈ {1, 2}.
    Half(i32, u8),
    Year(i32),
    /// `(start_ms, end_ms)`.
    Custom(i64, i64),
    Unknown,
}

/// How a report's span covers the tuple's range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RangeFit {
    /// The span is the range.
    Exact,
    /// The span strictly contains the range (a whole year for a half):
    /// the series is shown with a `narrowed_from` annotation.
    Superset,
}

impl RangeFit {
    /// The range score: an exact span beats a superset.
    fn score(self) -> u32 {
        match self {
            RangeFit::Exact => 20,
            RangeFit::Superset => 10,
        }
    }
}

/// Parsed front-matter fields relevant to cache-key matching.
#[derive(Debug)]
struct ReportMeta {
    /// The `strategy:` block's `id:` (bug-log #212), read with Compare's
    /// tolerant frontmatter reader, which also takes the engine's unindented
    /// `id:` line. `None` when the report has none: it then answers no tuple.
    strategy_id: Option<String>,
    /// The `data_source:` frontmatter value ("synthetic" / "yahoo…" /
    /// "binance"), written by every engine report since commit 93845af.
    /// `None` for legacy (pre-June) reports without the field — treated as
    /// source-unknown by [`load_equity`] (review D1: they only match when no
    /// report tagged with the requested source matches; no on-disk migration).
    data_source: Option<String>,
    /// The coins the report covers (Summary row, else Universe, else a
    /// scenario-name ticker).
    coins: Coins,
    /// The span the `scenario:` name states.
    span: Span,
    /// The `generated:` write time (bug-log #209), RFC 3339 as every writer
    /// emits it. `None` when missing or unparseable: it ranks below any
    /// parsed one.
    generated: Option<time::OffsetDateTime>,
    /// Whether the report body contains a `## Equity curve` section.
    has_equity_section: bool,
    /// Initial capital from the Summary table.
    initial_capital: Option<Decimal>,
    /// Final equity from the Summary table.
    final_equity: Option<Decimal>,
}

/// `raw`, one item of `str::split_inclusive('\n')`, without its terminator: a
/// trailing `\n` and then a `\r` before it, exactly what `str::lines` strips
/// (bug-log #232).
fn line_text(raw: &str) -> &str {
    raw.strip_suffix('\n')
        .map_or(raw, |l| l.strip_suffix('\r').unwrap_or(l))
}

/// Parse report front-matter + body summary block for matching/loading.
fn parse_report_meta(content: &str) -> Option<ReportMeta> {
    // Extract front-matter (between first and second `---`).
    let mut fm_lines: Vec<&str> = Vec::new();
    let mut in_fm = false;
    let mut dash_count = 0u32;
    let mut body = "";
    let mut body_start = 0usize;

    // Each line counted with its own terminator (`\n`, `\r\n`, or none on the
    // last line), so `body_start` stays within `content` and on a char
    // boundary (bug-log #232).
    for raw in content.split_inclusive('\n') {
        body_start += raw.len();
        let line = line_text(raw);
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

    // bug-log #212 — the strategy, through Compare's tolerant reader.
    let strategy_id = crate::compare::cache::parse_frontmatter(content)
        .and_then(|fm| fm.get("strategy.id").map(ToString::to_string));

    // bug-log #212 — the coin: the Summary's `| Symbol |` row, else the
    // `## Universe` list, else a ticker in the scenario name.
    let coins = if let Some(sym) = summary_cell(body, "Symbol") {
        Coins::One(sym)
    } else if let Some(list) = parse_universe_from_body(body) {
        Coins::Universe(list)
    } else {
        let sym_candidates = [
            "XRPUSDT", "ETHUSDT", "BTCUSDT", "ADAUSDT", "AVAXUSDT", "BNBUSDT", "DOGEUSDT",
            "DOTUSDT", "LINKUSDT", "SOLUSDT",
        ];
        sym_candidates
            .iter()
            .find(|&&s| scenario.to_ascii_uppercase().contains(s))
            .map_or(Coins::Unknown, |s| Coins::One((*s).to_string()))
    };
    let span = scenario_span(&scenario);

    // bug-log #209 — the write time, the § D5 tiebreak.
    let generated = fm_lines
        .iter()
        .find_map(|l| l.strip_prefix("generated: "))
        .and_then(|v| {
            time::OffsetDateTime::parse(v.trim(), &time::format_description::well_known::Rfc3339)
                .ok()
        });

    let has_equity_section = body.contains("## Equity curve") || body.contains("## Equity Curve");

    // Parse Summary table for initial / final capital.
    let initial_capital = parse_summary_value(body, "Initial capital")
        .or_else(|| parse_summary_value(body, "Initial Capital"));
    let final_equity = parse_summary_value(body, "Final equity")
        .or_else(|| parse_summary_value(body, "Final Equity"));

    Some(ReportMeta {
        strategy_id,
        data_source,
        coins,
        span,
        generated,
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

/// The coins of the `## Universe` section (`- <SYMBOL>` lines), or `None`
/// when the report has no such section or it lists none.
fn parse_universe_from_body(body: &str) -> Option<Vec<String>> {
    let mut in_universe = false;
    let mut symbols: Vec<String> = Vec::new();
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
                symbols.push(sym.to_string());
            }
        }
    }
    if symbols.is_empty() {
        None
    } else {
        Some(symbols)
    }
}

/// The value of the `## Summary` table row whose first cell is exactly
/// `field` (`| Symbol | BTCUSDT |` → `BTCUSDT`), or `None` when the section
/// has no such row or its value cell is empty (bug-log #212). Only the
/// `## Summary` section is read.
fn summary_cell(body: &str, field: &str) -> Option<String> {
    let mut in_summary = false;
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") {
            in_summary = trimmed.starts_with("## Summary");
            continue;
        }
        if !in_summary || !trimmed.starts_with('|') {
            continue;
        }
        let mut cells = trimmed.trim_matches('|').split('|').map(str::trim);
        if cells.next() == Some(field) {
            return cells
                .next()
                .filter(|v| !v.is_empty())
                .map(ToString::to_string);
        }
    }
    None
}

/// The [`Span`] of a scenario name (bug-log #212). See [`Span`] for the
/// token rule.
fn scenario_span(scenario: &str) -> Span {
    let lower = scenario.to_ascii_lowercase();
    let tokens: Vec<&str> = lower.split(['-', '_']).collect();
    for (i, tok) in tokens.iter().enumerate() {
        match *tok {
            "last30d" => return Span::Last30d,
            "last90d" => return Span::Last90d,
            _ => {}
        }
        if let Some(rest) = tok.strip_prefix("custom")
            && let Some((start, end)) = rest.split_once("to")
            && let (Ok(start), Ok(end)) = (start.parse::<i64>(), end.parse::<i64>())
        {
            return Span::Custom(start, end);
        }
        if tok.len() == 4
            && tok.bytes().all(|b| b.is_ascii_digit())
            && let Ok(year) = tok.parse::<i32>()
        {
            return match tokens.get(i + 1).copied() {
                Some("h1") => Span::Half(year, 1),
                Some("h2") => Span::Half(year, 2),
                _ => Span::Year(year),
            };
        }
    }
    Span::Unknown
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

/// Whether a report answers `tuple` (ADR-0055 D9, bug-log #212): its
/// strategy, its coin and its range must all fit. The source tier is
/// [`load_equity`]'s.
fn report_matches(meta: &ReportMeta, tuple: &LabTuple) -> MatchQuality {
    // Strategy: both sides through the engine's one writer-id map. A report
    // with no `id:`, or an id the map does not know, answers nothing.
    let want = backtest::engine::report_strategy_id(&tuple.strategy);
    let have = meta
        .strategy_id
        .as_deref()
        .and_then(backtest::engine::report_strategy_id);
    if want.is_none() || want != have {
        return MatchQuality::NoMatch;
    }
    if !meta.coins.covers(&tuple.symbol) {
        return MatchQuality::NoMatch;
    }
    match range_fit(meta.span, &tuple.range) {
        Some(fit) => MatchQuality::Match { fit },
        None => MatchQuality::NoMatch,
    }
}

#[derive(Debug, PartialEq, Eq)]
enum MatchQuality {
    NoMatch,
    Match { fit: RangeFit },
}

/// How the report span `span` covers `range`, or `None` (bug-log #212). Only
/// the same token is exact; the one superset is a whole year 2024 for an
/// `H1_2024`/`H2_2024` tuple. A `Custom` tuple matches only a report whose
/// `custom<start_ms>to<end_ms>` equals the range the run mapper derives from
/// its date strings (`lab::runner::custom_range_ms`).
fn range_fit(span: Span, range: &DateRange) -> Option<RangeFit> {
    match (range, span) {
        (DateRange::Preset(Preset::Last30d), Span::Last30d)
        | (DateRange::Preset(Preset::Last90d), Span::Last90d)
        | (DateRange::Preset(Preset::H1_2024), Span::Half(2024, 1))
        | (DateRange::Preset(Preset::H2_2024), Span::Half(2024, 2)) => Some(RangeFit::Exact),
        (DateRange::Preset(Preset::H1_2024 | Preset::H2_2024), Span::Year(2024)) => {
            Some(RangeFit::Superset)
        }
        (DateRange::Custom { start_raw, end_raw }, Span::Custom(start, end)) => {
            crate::lab::runner::custom_range_ms(start_raw, end_raw)
                .ok()
                .filter(|&ms| ms == (start, end))
                .map(|_| RangeFit::Exact)
        }
        _ => None,
    }
}

/// lab-run-save-compare Wave-2: read the companion equity CSV beside `md_path`
/// (`backtest-<stamp>-<scenario>-equity.csv`) for full per-bar fidelity, via
/// the existing [`reports::csv_artifacts::read_equity_csv`]. Returns `None` if
/// the companion is absent, unparseable or empty — the caller then has no
/// per-bar series from it (older committed reports have no companion).
///
/// `pub(crate)` so the Compare cold-boot cache scanner
/// (`compare::cache::scan_one_root`, lab-compare-equity-overlay T1) hydrates a
/// `CachedCell`'s timestamped equity series through the same companion-CSV
/// lookup the Lab cold path uses — one source of truth for "find the per-bar
/// series beside this report": this is the lenient form of
/// [`read_companion_equity_csv`], which the Lab reads strictly (bug-log #209):
/// the same series when the CSV reads; a cut or empty CSV gives Compare an
/// empty series, while the Lab treats the report as broken.
#[must_use]
pub(crate) fn load_companion_equity_csv(md_path: &std::path::Path) -> Option<Vec<(i64, Decimal)>> {
    read_companion_equity_csv(md_path).ok().flatten()
}

/// The companion equity CSV beside `md_path`, read strictly for the Lab
/// (bug-log #209): `Ok(None)` when there is none (older committed reports, or
/// a `md_path` with no `.md` name), the per-bar series when it reads.
///
/// # Errors
///
/// `ParseError` naming the CSV when one is present but does not read, and
/// `EmptySeries` when it holds no sample: the engine writes the `.md` first
/// and the CSV second, so a run cut short on disk (ENOSPC while the CSV was
/// written) leaves a complete `.md` beside a truncated CSV, and the Lab must
/// not draw that run as a start/end pair.
fn read_companion_equity_csv(
    md_path: &std::path::Path,
) -> Result<Option<Vec<(i64, Decimal)>>, EquityLoadError> {
    let Some(stem) = md_path
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_suffix(".md"))
    else {
        return Ok(None);
    };
    let csv = md_path.with_file_name(format!("{stem}-equity.csv"));
    let unreadable = |msg: String| EquityLoadError::ParseError {
        path: csv.display().to_string(),
        msg,
    };
    if !csv.try_exists().map_err(|e| unreadable(e.to_string()))? {
        return Ok(None);
    }
    let samples =
        reports::csv_artifacts::read_equity_csv(&csv).map_err(|e| unreadable(e.to_string()))?;
    if samples.is_empty() {
        return Err(EquityLoadError::EmptySeries {
            path: csv.display().to_string(),
        });
    }
    Ok(Some(
        samples
            .into_iter()
            .map(|s| (s.ts.unix_millis(), s.equity_total))
            .collect(),
    ))
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

/// A report's source tier (ADR-0055 D9 item 4): a report tagged with the
/// tuple's `data_source:` beats an untagged legacy one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Tier {
    /// No `data_source:` (pre-93845af): source-unknown.
    Untagged,
    /// `data_source:` names the tuple's source.
    Tagged,
}

/// The order of two candidate reports (bug-log #209): the field order is the
/// comparison order (the derived `Ord`), so the tier decides first, then the
/// range score, then `generated:` (`None` lowest).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct CandidateKey {
    tier: Tier,
    range_score: u32,
    generated: Option<time::OffsetDateTime>,
}

/// A matching report of one root for a tuple (bug-log #209), not yet read for
/// its series; [`resolve_roots`] orders them by [`CandidateKey`].
#[derive(Debug, Clone)]
struct Candidate {
    tier: Tier,
    fit: RangeFit,
    generated: Option<time::OffsetDateTime>,
    path: PathBuf,
}

impl Candidate {
    fn key(&self) -> CandidateKey {
        CandidateKey {
            tier: self.tier,
            range_score: self.fit.score(),
            generated: self.generated,
        }
    }
}

/// Every matching report under `report_root` for `tuple`, in D8 walk order
/// (bug-log #209). A file whose name is already in `seen` (an earlier root or
/// container held a report of that name) is skipped before it is read, and
/// every name walked is added, matching or not: § D5's identical-filename
/// rule, as Compare's `seen_filenames`. The order is kept so that
/// [`resolve_roots`]' stable sort gives an equal key to the first report in
/// D8 order (the rule Compare uses, bug-log #206).
///
/// # Errors
///
/// `ReportDirNotFound` when the slug has no `backtest-*.md` under the root;
/// `NoReport` when none of them matches.
fn matching_candidates(
    tuple: &LabTuple,
    report_root: &std::path::Path,
    seen: &mut HashSet<OsString>,
) -> Result<Vec<Candidate>, EquityLoadError> {
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

    // Source-aware since the review D1 fix. Two tiers:
    //   1. Reports whose frontmatter `data_source:` matches `tuple.source`
    //      (the honest tier — a Binance tuple resolves a binance report).
    //   2. Legacy reports WITHOUT the field (pre-93845af) — source-unknown;
    //      they are eligible ONLY when no tier-1 report matches, in any root
    //      ([`resolve_roots`] reads no candidate below the top tier; no
    //      on-disk migration).
    // Reports tagged with a DIFFERENT source never match — that is the
    // shadowing bug the D1 fix removed.
    let mut out: Vec<Candidate> = Vec::new();
    for path in &reports {
        if !path.file_name().is_some_and(|n| seen.insert(n.to_owned())) {
            continue; // an earlier copy of this filename was walked first
        }
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        let Some(meta) = parse_report_meta(&content) else {
            continue;
        };
        let MatchQuality::Match { fit } = report_matches(&meta, tuple) else {
            continue;
        };
        let tier = match meta.data_source.as_deref() {
            Some(ds) if source_tag_matches(ds, tuple.source) => Tier::Tagged,
            Some(_) => continue,    // tagged with a different source — excluded
            None => Tier::Untagged, // legacy-unknown (tier 2)
        };
        out.push(Candidate {
            tier,
            fit,
            generated: meta.generated,
            path: path.clone(),
        });
    }

    if out.is_empty() {
        return Err(no_report_error(tuple));
    }
    Ok(out)
}

/// The `NoReport` error for `tuple`.
fn no_report_error(tuple: &LabTuple) -> EquityLoadError {
    EquityLoadError::NoReport {
        strategy: tuple.strategy.clone(),
        symbol: tuple.symbol.clone(),
        range: tuple.range.clone(),
        data_source: tuple.source,
    }
}

/// Resolve `tuple` over `roots`, searched in order (bug-log #209, ADR-0055 §
/// D5): every root's matching candidates are concatenated in root order
/// (each root's in D8 walk order), with one filename set over the whole walk;
/// a stable sort puts the greatest [`CandidateKey`] first, so an equal key
/// keeps root order, then D8 order; the candidates are read in that order and
/// the first that reads wins, so a broken report falls through to the next
/// candidate, in its own root or in the next. The reads stop at the source
/// tier of the first candidate (D9: an untagged report is eligible only when
/// no tagged one matches), so a broken tagged report never yields to an
/// untagged one. Inside the tier a broken exact report yields to a readable
/// superset, drawn with its `narrowed_from` badge. [`load_equity`] is this
/// over one root, [`EquityCache::get_or_load_roots`] this behind its cache.
///
/// # Errors
///
/// The LAST root's error when nothing reads: its walk's error, or the read
/// error of its last candidate tried. A root whose candidates all sit below
/// the tier boundary records none, so when it is the last root the result is
/// `NoReport` (none of its reports is eligible). Empty `roots` yields
/// `NoReport`.
fn resolve_roots(tuple: &LabTuple, roots: &[PathBuf]) -> Result<LabEquitySeries, EquityLoadError> {
    let mut errs: Vec<Option<EquityLoadError>> = vec![None; roots.len()];
    let mut cands: Vec<(usize, Candidate)> = Vec::new();
    let mut seen: HashSet<OsString> = HashSet::new();
    for (i, root) in roots.iter().enumerate() {
        match matching_candidates(tuple, root, &mut seen) {
            Ok(found) => cands.extend(found.into_iter().map(|c| (i, c))),
            Err(e) => errs[i] = Some(e),
        }
    }
    // Greatest key first; the sort is stable, so an equal key keeps root
    // order (lab-runs/ first), then D8 order inside a root.
    cands.sort_by(|a, b| b.1.key().cmp(&a.1.key()));
    // D9 item 4: no read below the top candidate's source tier.
    let top_tier = cands.first().map(|(_, c)| c.tier);
    for (i, c) in cands.iter().take_while(|(_, c)| Some(c.tier) == top_tier) {
        match materialize(c) {
            Ok(loaded) => return Ok(loaded),
            Err(e) => errs[*i] = Some(e),
        }
    }
    Err(errs
        .pop()
        .flatten()
        .unwrap_or_else(|| no_report_error(tuple)))
}

/// Read the candidate's equity series: the companion CSV, else the `##
/// Equity curve` table, else the Summary's start/end pair.
///
/// # Errors
///
/// `ParseError` when the report cannot be read or parsed, or when its
/// companion CSV is present but does not read (bug-log #209); `EmptySeries`
/// when it yields no sample.
fn materialize(cand: &Candidate) -> Result<LabEquitySeries, EquityLoadError> {
    let cand_path = &cand.path;
    let cand_fit = cand.fit;
    let content = std::fs::read_to_string(cand_path).map_err(|e| EquityLoadError::ParseError {
        path: cand_path.display().to_string(),
        msg: e.to_string(),
    })?;

    let meta = parse_report_meta(&content).ok_or_else(|| EquityLoadError::ParseError {
        path: cand_path.display().to_string(),
        msg: "failed to parse report frontmatter".to_string(),
    })?;

    let report_name = cand_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // narrowed_from: the report's span is a superset of the requested range
    // (the same fit that scored it, so the badge and the score cannot drift).
    let narrowed_from: Option<SmolStr> =
        (cand_fit == RangeFit::Superset).then(|| SmolStr::new(&report_name));

    // Extract equity body.
    let body_start = find_body_start(&content);
    let body = &content[body_start..];

    // lab-run-save-compare Wave-2 (ADR-0055 § D-companion): prefer the companion
    // equity CSV (full per-bar series) if present beside the `.md`. Lab runs
    // persist it, so a saved run hydrates a REAL curve instead of the sparkline
    // start-end 2-point fallback (the `.md`'s `## Equity curve` is only a
    // visual sparkline, not machine-parseable). Older committed `evidence/`
    // reports without a companion stay at their existing fidelity. A present
    // companion that does not read makes the report unreadable (bug-log #209).
    let companion = read_companion_equity_csv(cand_path)?;
    let (samples, fidelity) = if let Some(csv_pts) = companion {
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
            path: cand_path.display().to_string(),
        });
    }

    Ok(LabEquitySeries {
        samples,
        source_report: SmolStr::new(&report_name),
        fidelity,
        narrowed_from,
    })
}

/// Load an equity series for `tuple` from ONE report-root directory tree
/// (production roots: `evidence/` or `lab-runs/`): `resolve_roots` over that
/// root, so its matching reports are read greatest `CandidateKey` first
/// and the first that reads wins. Synchronous; call from the iced thread on
/// first cache miss.
///
/// # Errors
///
/// Returns `EquityLoadError::ReportDirNotFound` when the slug has no
/// `backtest-*.md` under the root at all (its message names the slug, the root,
/// the container depth bound and which empty it is);
/// `EquityLoadError::NoReport` when no matching report is found;
/// `EquityLoadError::ParseError` or `EquityLoadError::EmptySeries` when no
/// matching candidate of the top source tier can be read: the error of the
/// last one tried (a report that does not parse, a companion CSV that does not
/// read, or one that holds no sample).
pub fn load_equity(
    tuple: &LabTuple,
    report_root: &std::path::Path,
) -> Result<LabEquitySeries, EquityLoadError> {
    resolve_roots(tuple, &[report_root.to_path_buf()])
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

/// Find the byte offset where the report body starts (after `---` front-matter).
/// Each line is counted with its own terminator, as in [`parse_report_meta`],
/// so the offset is within `content` and on a char boundary (bug-log #232).
fn find_body_start(content: &str) -> usize {
    let mut dash_count = 0u32;
    let mut pos = 0usize;
    for raw in content.split_inclusive('\n') {
        pos += raw.len();
        let line = line_text(raw);
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
/// `[lab-runs/, evidence/]` — **lab-runs FIRST**. On a tuple collision the
/// newest `generated:` wins in either reader (in the Lab after the source
/// tier and the range score, bug-log #209), except on an identical filename,
/// where both readers keep the copy walked first (`lab-runs/`) and skip the
/// later one; the root order also settles an equal `generated:`. Exposed so
/// the Lab screen and the Compare cold-boot wire pass the identical ordered
/// roots.
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
///    (`EquityCache::get_or_load_roots`: the greatest (tier, range score,
///    `generated:`) over `lab-runs/` and `evidence/`, an identical filename
///    or an equal key to lab-runs — lab-run-save-compare T4 / R4 / Q4, bug-log
///    #209). After a Lab run persists its report
///    under `lab-runs/`, the curve repaints from disk on the next boot /
///    tuple-select even with the in-memory mirror cleared (AC4). If the cache
///    miss returns an error, returns `None` so the chart renders with no
///    equity overlay (graceful degradation).
///
/// `roots` is searched in order; pass `default_report_roots()` in production.
/// `cache` is mutated only for a tuple not seen since the last clear; its
/// outcome, hit or miss, is then cached.
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
strategy:
id: top10_momentum_h1
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
strategy:
id: top10_momentum_h1
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

    /// The `generated:` of [`FIXTURE_REPORT_WITH_EQUITY`].
    const FIXTURE_GENERATED: &str = "2026-04-29T19:52:43Z";

    /// `content` with its `generated:` frontmatter value replaced by
    /// `generated` (bug-log #209: the root-order pins need an equal
    /// `generated:` on both sides).
    fn with_generated(content: &str, generated: &str) -> String {
        let line = content
            .lines()
            .find(|l| l.starts_with("generated: "))
            .expect("the fixture has a generated: line");
        let out = content.replacen(line, &format!("generated: {generated}"), 1);
        assert!(
            out.lines().any(|l| l == format!("generated: {generated}")),
            "the fixture now carries generated: {generated}"
        );
        out
    }

    /// A start/end-only sma report for the `Last90d` tuple of
    /// `load_equity_start_end_fallback` (bug-log #212: a `btc-2023-1m-…` name
    /// answered `Last90d` only through the old any-report-matches rule).
    const FIXTURE_REPORT_NO_EQUITY: &str = r#"---
scenario: btcusdt-last90d-sma-cross-synthetic
seed: 0xC0FFEE
generated: 2026-04-29T10:00:00Z
wall_clock_s: 0.2
data_source: synthetic
strategy:
id: sma_crossover
---

# Backtest Report — btcusdt-last90d-sma-cross-synthetic

## Summary

| Metric          | Value             |
|-----------------|-------------------|
| Scenario        | btcusdt-last90d-sma-cross-synthetic |
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

    /// T-D-10 — clearing the cache causes a re-read (bug-log #208: the
    /// per-tuple `invalidate` it replaced had no production caller).
    #[test]
    fn cache_clear_re_reads() {
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
        cache.clear();
        let after = cache.get_or_load(&tuple, &spec).unwrap();
        // Contents are the same (same file); Arc pointers differ after the clear.
        assert!(
            !Arc::ptr_eq(&first, &after),
            "expected new Arc after the clear"
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
             strategy:\n\
             id: sma_crossover\n\
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
        // bug-log #212: the five Lab chips the old table sent to a raw-id dir.
        assert_eq!(strategy_slug("v0.5.rsi"), "v05-composed-strategies");
        assert_eq!(strategy_slug("v0.5.bbands"), "v05-composed-strategies");
        assert_eq!(strategy_slug("v1.5a.pairs"), "v15a-mean-reversion-pairs");
        assert_eq!(strategy_slug("v2.5.tcn"), "v2.5.tcn_overlay");
        assert_eq!(
            strategy_slug("v2.5.tcn.weights"),
            "v2.5.tcn_overlay_weights"
        );
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

    /// bug-log #208 (a) — a cached miss is not re-read until the cache is
    /// cleared. Both roots start absent, so the first lookup misses
    /// (`ReportDirNotFound`). A matching report written afterwards under
    /// `<root>/<rel>` is NOT found by the same cache: the miss is cached and
    /// returned without touching EITHER root. Control: a fresh cache finds the
    /// report, so the file is there and matches; after the clear the original
    /// cache finds it too.
    fn assert_cached_miss_is_not_re_read_until_clear(root: &str, rel: &str) {
        const POINTS: usize = 3;
        let tmp = tempfile::tempdir().unwrap();
        let lab_runs = tmp.path().join("lab-runs");
        let roots = [lab_runs.clone(), tmp.path().join("evidence")];
        let write_root = tmp.path().join(root);
        let tuple = sma_tuple(LabDataSource::Synthetic);

        let mut cache = EquityCache::new();
        let first = cache
            .get_or_load_roots(&tuple, &roots)
            .expect_err("both roots are absent: the first lookup misses");
        assert!(
            matches!(first, EquityLoadError::ReportDirNotFound(_)),
            "an absent root is ReportDirNotFound, got {first:?}"
        );

        write_at(
            &write_root,
            rel,
            "backtest-20260601-120000-btc-2024-h1-sma-cross.md",
            &sma_2024_h1_report(Some("synthetic"), POINTS),
        );

        let second = cache
            .get_or_load_roots(&tuple, &roots)
            .expect_err("the cached miss must stay a miss: the roots are not re-read");
        assert_eq!(
            second.to_string(),
            first.to_string(),
            "a cached miss returns the walk's own error"
        );

        let fresh = EquityCache::new()
            .get_or_load_roots(&tuple, &roots)
            .expect("control: a fresh cache resolves the written report");
        assert_eq!(fresh.samples.len(), POINTS);

        cache.clear();
        let after = cache
            .get_or_load_roots(&tuple, &roots)
            .expect("after the clear the written report resolves");
        assert_eq!(after.samples.len(), POINTS);
    }

    /// bug-log #208 (a), the `lab-runs/` root (container depth 0).
    #[test]
    fn cached_miss_is_not_re_read_until_clear() {
        assert_cached_miss_is_not_re_read_until_clear("lab-runs", "v0-paper-sma/reports");
    }

    /// bug-log #208 (a), the `evidence/` root at container depth 1 — the root
    /// `#205` widened the walk to. A cache that re-walked only the last root
    /// on a cached miss would find this report.
    #[test]
    fn cached_miss_is_not_re_read_until_clear_under_evidence() {
        assert_cached_miss_is_not_re_read_until_clear("evidence", "v1/v0-paper-sma/reports");
    }

    /// bug-log #208 (d) — a cached HIT is stale until the clear, then the
    /// newer `lab-runs/` report wins (its `generated:` is newer, bug-log
    /// #209). A committed
    /// report of `COMMITTED` points under `evidence/` (container depth 1) is
    /// cached as the tuple's hit; a `lab-runs/` report of `NEWER` points
    /// written afterwards is not seen by the same cache (the same `Arc` comes
    /// back). Control: a fresh cache resolves the `lab-runs/` report. After
    /// `clear()` the original cache does too.
    #[test]
    fn clear_replaces_a_stale_committed_hit_with_the_newer_lab_runs_report() {
        const COMMITTED: usize = 2;
        const NEWER: usize = 5;
        let tmp = tempfile::tempdir().unwrap();
        let lab_runs = tmp.path().join("lab-runs");
        let evidence = tmp.path().join("evidence");
        let roots = [lab_runs.clone(), evidence.clone()];
        let tuple = sma_tuple(LabDataSource::Synthetic);
        write_at(
            &evidence,
            "v1/v0-paper-sma/reports",
            "backtest-20260101-000000-btc-2024-h1-sma-cross.md",
            &sma_report(
                "btc-2024-h1-sma-cross",
                Some("synthetic"),
                COMMITTED,
                "2026-01-01T00:00:00Z",
            ),
        );

        let mut cache = EquityCache::new();
        let first = cache
            .get_or_load_roots(&tuple, &roots)
            .expect("the committed report resolves");
        assert_eq!(first.samples.len(), COMMITTED);

        write_at(
            &lab_runs,
            "v0-paper-sma/reports",
            "backtest-20260601-120000-btc-2024-h1-sma-cross.md",
            &sma_report(
                "btc-2024-h1-sma-cross",
                Some("synthetic"),
                NEWER,
                "2026-06-01T12:00:00Z",
            ),
        );

        let second = cache
            .get_or_load_roots(&tuple, &roots)
            .expect("the cached hit is returned");
        assert!(
            Arc::ptr_eq(&first, &second),
            "before the clear the cached hit is returned as is"
        );
        assert_eq!(second.samples.len(), COMMITTED);

        let fresh = EquityCache::new()
            .get_or_load_roots(&tuple, &roots)
            .expect("control: a fresh cache resolves the lab-runs report");
        assert_eq!(fresh.samples.len(), NEWER);

        cache.clear();
        let after = cache
            .get_or_load_roots(&tuple, &roots)
            .expect("after the clear the lab-runs report resolves");
        assert_eq!(after.samples.len(), NEWER);
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
    /// in EITHER a `lab-runs/` root OR a `spec/` root, AND **lab-runs wins a
    /// tuple collision with an equal key** (Q4 / ADR-0055 § D5: lab-runs
    /// searched FIRST). Builds the SAME tuple in both roots with
    /// DISTINGUISHABLE equity tails and ONE `generated:` (bug-log #209: a
    /// strictly newer `generated:` wins in either root, X1/X2), then asserts
    /// the union picks the lab-runs copy.
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
            &with_generated(LAB_RUNS_REPORT_TWO_POINT, FIXTURE_GENERATED),
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
    /// depth 1, 4 points) for the SAME tuple and ONE `generated:`, so the key
    /// is equal and only the root order decides (bug-log #209). Returns
    /// `(lab_runs, evidence)`.
    fn depth_precedence_scene(tmp: &tempfile::TempDir) -> (PathBuf, PathBuf) {
        let lab_runs = tmp.path().join("lab-runs");
        let evidence = tmp.path().join("evidence");
        write_at(
            &lab_runs,
            "v1-cross-sectional-momentum/reports",
            "backtest-20260601-120000-top10-2024-h1-momentum.md",
            &with_generated(LAB_RUNS_REPORT_TWO_POINT, FIXTURE_GENERATED),
        );
        write_at(
            &evidence,
            "v1/v1-cross-sectional-momentum/reports",
            "backtest-20260101-000000-top10-2024-h1-momentum.md",
            FIXTURE_REPORT_WITH_EQUITY,
        );
        (lab_runs, evidence)
    }

    /// E2 — D5 precedence holds with a container-depth-1 evidence report: on
    /// an equal key (tier, range score, `generated:`) lab-runs, searched
    /// first, shadows it.
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

    /// E2b — E2's pair: on the same equal key the container-depth-1 evidence
    /// report wins when its root is searched first. Without it, E2 passes
    /// under #205 (evidence unread) and under a rule that ignores the root
    /// order on a tie (bug-log #209).
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
    /// reports of one slug with an equal key (score AND `generated:`, bug-log
    /// #209) under TWO container-depth-1 dirs resolve to the first dir in path
    /// order — not to the newest stamp and not to the smallest file name. The
    /// winning dir holds ONE report, so E4 does not pin the order inside one
    /// dir: the layout's file order is LY4's, the Lab's in-dir tie N2's. The
    /// expected report is derived from the fixture table.
    #[test]
    fn equal_score_and_generated_tie_at_one_container_depth_goes_to_the_first_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let evidence = tmp.path().join("evidence");
        // (reports dir, file name, equity points): v1 holds the MIDDLE stamp;
        // v2 the oldest stamp and the newest stamp. One `generated:`.
        let generated = "2026-05-01T00:00:00Z";
        let table: [(&str, &str, usize); 3] = [
            (
                "v1/v0-paper-sma/reports",
                "backtest-20260301-000000-btc-2024-h1-sma-cross.md",
                4,
            ),
            (
                "v2/v0-paper-sma/reports",
                "backtest-20260101-000000-btc-2024-h1-sma-cross.md",
                3,
            ),
            (
                "v2/v0-paper-sma/reports",
                "backtest-20260601-000000-btc-2024-h1-sma-cross.md",
                5,
            ),
        ];
        for (rel, name, n) in table {
            write_at(
                &evidence,
                rel,
                name,
                &sma_report("btc-2024-h1-sma-cross", Some("synthetic"), n, generated),
            );
        }
        let expected = table
            .iter()
            .min_by_key(|(rel, name, _)| (container_depth_of(rel), evidence.join(rel), *name))
            .map(|r| r.2)
            .unwrap();
        let newest_stamp = table.iter().max_by_key(|r| r.1).unwrap().2;
        let first_name = table.iter().min_by_key(|r| r.1).unwrap().2;
        assert!(
            expected != newest_stamp && expected != first_name,
            "the fixture must make every alternative rule pick another report"
        );
        let series = load_equity(&sma_tuple(LabDataSource::Synthetic), &evidence)
            .expect("the slug resolves under both containers");
        assert_eq!(
            series.samples.len(),
            expected,
            "an equal-key tie must go to the first dir in path order ({expected} points), not \
             the newest stamp ({newest_stamp}) or the smallest file name ({first_name})"
        );
    }

    /// E4b — the D8 tie rule ACROSS container depths, two-sided, on an equal
    /// key (score AND `generated:`, bug-log #209). In each of two roots the
    /// slug sits at container depth 0 AND 1. In root `a0` the container name
    /// sorts BEFORE the slug name, in root `zz` AFTER it, and the stamps are
    /// arranged so that full-path order, the newest stamp and the oldest stamp
    /// each pick the container-depth-1 report in at least one root (asserted
    /// on the table). Of those rules and "(container depth, path), first
    /// best", only the last picks container depth 0 in both — where HEAD looked
    /// before #205. Compare's #206 pin is the same shape. E4b's companion
    /// [`a_strictly_newer_generated_at_container_depth_1_wins`] puts a
    /// strictly newer `generated:` at container depth 1.
    #[test]
    fn equal_generated_tie_across_container_depths_goes_to_container_depth_0() {
        let tmp = tempfile::tempdir().unwrap();
        let generated = "2026-03-01T00:00:00Z";
        let mut d0_points = Vec::new();
        let mut full_path = Vec::new();
        let mut newest_stamp = Vec::new();
        let mut oldest_stamp = Vec::new();
        for (root_name, fx) in &depth_tie_roots() {
            let root = tmp.path().join(root_name);
            for (rel, name, n) in fx {
                write_at(
                    &root,
                    rel,
                    name,
                    &sma_report("btc-2024-h1-sma-cross", Some("synthetic"), *n, generated),
                );
            }
            d0_points.push(fx.iter().find(|r| container_depth_of(r.0) == 0).unwrap().2);
            full_path.push(
                fx.iter()
                    .min_by_key(|r| root.join(r.0).join(r.1))
                    .unwrap()
                    .2,
            );
            newest_stamp.push(fx.iter().max_by_key(|r| r.1).unwrap().2);
            oldest_stamp.push(fx.iter().min_by_key(|r| r.1).unwrap().2);
        }
        for (rule, picks) in [
            ("full-path order", &full_path),
            ("newest stamp", &newest_stamp),
            ("oldest stamp", &oldest_stamp),
        ] {
            assert_ne!(
                picks, &d0_points,
                "the fixture must make `{rule}` miss container depth 0 in at least one root"
            );
        }
        for ((root_name, _), want) in depth_tie_roots().iter().zip(&d0_points) {
            let series = load_equity(
                &sma_tuple(LabDataSource::Synthetic),
                &tmp.path().join(root_name),
            )
            .expect("the slug resolves in both roots");
            assert_eq!(
                series.samples.len(),
                *want,
                "{root_name}: an equal-key tie across container depths must go to the \
                 container-depth-0 report ({want} points) — ADR-0055 D8 tie rule (#205)"
            );
        }
    }

    /// One report of E4b's table: (reports dir, file name, equity points).
    type DepthTieFx = (&'static str, &'static str, usize);

    /// The two roots of E4b and its companion: per root, the slug's report at
    /// container depth 0 and at 1.
    fn depth_tie_roots() -> [(&'static str, [DepthTieFx; 2]); 2] {
        [
            (
                "root-a0",
                [
                    (
                        "v0-paper-sma/reports",
                        "backtest-20260101-000000-btc-2024-h1-sma-cross.md",
                        3,
                    ),
                    (
                        "a0/v0-paper-sma/reports",
                        "backtest-20260601-000000-btc-2024-h1-sma-cross.md",
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
                        3,
                    ),
                    (
                        "zz/v0-paper-sma/reports",
                        "backtest-20260101-000000-btc-2024-h1-sma-cross.md",
                        5,
                    ),
                ],
            ),
        ]
    }

    /// E4b's companion (bug-log #209): in both of E4b's roots the
    /// container-depth-1 report carries a strictly newer `generated:`, so it
    /// wins at either container depth; the D8 order settles only an equal key
    /// or an identical filename (F2).
    #[test]
    fn a_strictly_newer_generated_at_container_depth_1_wins() {
        let tmp = tempfile::tempdir().unwrap();
        for (root_name, fx) in &depth_tie_roots() {
            let root = tmp.path().join(root_name);
            for (rel, name, n) in fx {
                let generated = if container_depth_of(rel) == 1 {
                    "2026-09-01T00:00:00Z"
                } else {
                    "2026-03-01T00:00:00Z"
                };
                write_at(
                    &root,
                    rel,
                    name,
                    &sma_report("btc-2024-h1-sma-cross", Some("synthetic"), *n, generated),
                );
            }
            let want = fx.iter().find(|r| container_depth_of(r.0) == 1).unwrap().2;
            let series = load_equity(&sma_tuple(LabDataSource::Synthetic), &root)
                .expect("the slug resolves");
            assert_eq!(
                series.samples.len(),
                want,
                "{root_name}: the strictly newer container-depth-1 report must win"
            );
        }
    }

    /// E4c — the within-root UNION is complete (falsifier O3): the strictly
    /// better report sits in the SECOND dir of the (container depth, path)
    /// order, so a reader that stops after the first dir picks the worse one.
    /// Range `H1_2024` (bug-log #212, re-derived from `Last90d`, which no
    /// longer matches any year): the `2024-h1` scenario is an exact match, the
    /// whole-year `2024` one only a narrowed superset (M7).
    #[test]
    fn a_strictly_better_report_in_the_second_report_dir_wins() {
        let tmp = tempfile::tempdir().unwrap();
        let evidence = tmp.path().join("evidence");
        // (reports dir, file name, scenario, equity points); the 2024-h1 row wins.
        let table: [(&str, &str, &str, usize); 2] = [
            (
                "v1/v0-paper-sma/reports",
                "backtest-20260601-000000-btc-2024-sma-cross.md",
                "btc-2024-sma-cross",
                4,
            ),
            (
                "v2/v0-paper-sma/reports",
                "backtest-20260101-000000-btc-2024-h1-sma-cross.md",
                "btc-2024-h1-sma-cross",
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
        let better = table.iter().find(|r| r.2.contains("2024-h1")).unwrap();
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
        let tuple = sma_tuple(LabDataSource::Synthetic);
        let series = load_equity(&tuple, &evidence).expect("the slug resolves");
        assert_eq!(
            series.samples.len(),
            better.3,
            "the better-scoring report in the second report dir must win; every dir of the \
             slug is read (#205)"
        );
        assert!(
            series.narrowed_from.is_none(),
            "the winner is an exact match"
        );
        // The first dir's report is a real competitor: alone, it answers the
        // same tuple as a narrowed superset (bug-log #212).
        let worse_only = tmp.path().join("worse-only");
        write_at(
            &worse_only,
            first_dir.0,
            first_dir.1,
            &sma_report(
                first_dir.2,
                Some("synthetic"),
                first_dir.3,
                "2026-06-01T12:00:00Z",
            ),
        );
        let worse = load_equity(&tuple, &worse_only).expect("the year report answers H1_2024");
        assert_eq!(worse.samples.len(), first_dir.3);
        assert_eq!(
            worse.narrowed_from.as_deref(),
            Some(first_dir.1),
            "a whole-year 2024 report answers H1_2024 as a narrowed superset"
        );
    }

    // ── bug-log #209: the newest generated: wins over the union (§ D5) ─────

    /// One `H1_2024` sma report for the tuple of [`sma_tuple`]: `(root, reports
    /// dir, file name, scenario, data_source:, generated:, equity points)`.
    type Fx209 = (
        &'static str,
        &'static str,
        &'static str,
        &'static str,
        Option<&'static str>,
        Option<&'static str>,
        usize,
    );

    /// Write each `Fx209` row under `<tmp>/<root>`; a row without
    /// `generated:` drops the line. Returns `[<tmp>/lab-runs, <tmp>/evidence]`.
    fn write_209(tmp: &tempfile::TempDir, rows: &[Fx209]) -> [PathBuf; 2] {
        for (root, rel, name, scenario, ds, generated, n) in rows {
            let text = sma_report(scenario, *ds, *n, generated.unwrap_or("DROP"));
            let text = if generated.is_none() {
                text.replacen("generated: DROP\n", "", 1)
            } else {
                text
            };
            assert_eq!(
                text.contains("generated: "),
                generated.is_some(),
                "fixture {name}: generated: line present iff declared"
            );
            write_at(&tmp.path().join(root), rel, name, &text);
        }
        [tmp.path().join("lab-runs"), tmp.path().join("evidence")]
    }

    /// The points of the row the union must pick, DERIVED from the table by
    /// § D5 as amended (bug-log #209). The walk order is (root: `lab-runs`
    /// then `evidence`; container depth; dir; file name); a file name met a
    /// second time in that order is dropped (the identical-filename rule).
    /// Of the rest the greatest (tier, range score, `generated:`) wins, an
    /// equal key going to the earliest in walk order. Tier: tagged
    /// `synthetic` > untagged; score: a `-2024-h1-` scenario 20, a whole-year
    /// `-2024-` 10.
    fn expected_209(rows: &[Fx209]) -> usize {
        let key = |r: &Fx209| {
            let tier = u8::from(r.4.is_some());
            let score = if r.3.contains("2024-h1") { 20 } else { 10 };
            (tier, score, r.5)
        };
        let walk = |r: &Fx209| {
            (
                usize::from(r.0 != "lab-runs"),
                container_depth_of(r.1),
                r.1,
                r.2,
            )
        };
        let mut walked: Vec<&Fx209> = rows.iter().collect();
        walked.sort_by_key(|r| walk(r));
        let mut kept: Vec<&Fx209> = Vec::new();
        for r in walked {
            if !kept.iter().any(|k| k.2 == r.2) {
                kept.push(r);
            }
        }
        kept.into_iter()
            .max_by(|a, b| key(a).cmp(&key(b)).then_with(|| walk(b).cmp(&walk(a))))
            .unwrap()
            .6
    }

    /// Resolve the [`sma_tuple`] over the union and assert the derived pick;
    /// `why` names the rule under test.
    fn assert_union_picks(rows: &[Fx209], why: &str) {
        let tmp = tempfile::tempdir().unwrap();
        let roots = write_209(&tmp, rows);
        let want = expected_209(rows);
        let got = EquityCache::new()
            .get_or_load_roots(&sma_tuple(LabDataSource::Synthetic), &roots)
            .expect("the union resolves");
        assert_eq!(got.samples.len(), want, "{why} (rows {rows:?})");
    }

    const H1: &str = "btc-2024-h1-sma-cross";
    const Y2024: &str = "btc-2024-sma-cross";
    const LR: &str = "v0-paper-sma/reports";
    const EV: &str = "v1/v0-paper-sma/reports";
    /// Container depth 0 under `evidence/` (bug-log #209's identical-filename
    /// pin inside one root).
    const EV0: &str = "v0-paper-sma/reports";
    const SYN: Option<&str> = Some("synthetic");

    /// N1-A — one dir, equal score: the newer `generated:` sits in the LARGER
    /// path and wins. Every fixture's `generated:` differs from its filename
    /// stamp.
    #[test]
    fn a_newer_generated_in_the_larger_path_wins() {
        let rows: [Fx209; 2] = [
            (
                "lab-runs",
                LR,
                "backtest-20260101-000000-a.md",
                H1,
                SYN,
                Some("2026-02-01T00:00:00Z"),
                3,
            ),
            (
                "lab-runs",
                LR,
                "backtest-20260102-000000-b.md",
                H1,
                SYN,
                Some("2026-03-01T00:00:00Z"),
                5,
            ),
        ];
        assert_eq!(
            expected_209(&rows),
            5,
            "the table's newer run is the larger path"
        );
        assert_union_picks(&rows, "N1-A: the newer generated: wins");
    }

    /// N1-B — the newer `generated:` sits in the SMALLER path and still wins:
    /// it tells "newest `generated:`" from "last path / newest stamp".
    #[test]
    fn a_newer_generated_in_the_smaller_path_wins() {
        let rows: [Fx209; 2] = [
            (
                "lab-runs",
                LR,
                "backtest-20260101-000000-a.md",
                H1,
                SYN,
                Some("2026-03-01T00:00:00Z"),
                3,
            ),
            (
                "lab-runs",
                LR,
                "backtest-20260102-000000-b.md",
                H1,
                SYN,
                Some("2026-02-01T00:00:00Z"),
                5,
            ),
        ];
        assert_eq!(
            expected_209(&rows),
            3,
            "the table's newer run is the smaller path"
        );
        assert_union_picks(&rows, "N1-B: the newer generated: wins over the last path");
    }

    /// N2 — one dir, equal score and equal `generated:` (two runs in one
    /// second): the first report in D8 order, the smaller path (Compare's
    /// #206 rule; the deliberate same-second residual).
    #[test]
    fn equal_score_and_generated_inside_one_dir_goes_to_the_first_path() {
        let g = Some("2026-02-01T00:00:00Z");
        let rows: [Fx209; 2] = [
            (
                "lab-runs",
                LR,
                "backtest-20260101-000000-a.md",
                H1,
                SYN,
                g,
                3,
            ),
            (
                "lab-runs",
                LR,
                "backtest-20260101-000000-b.md",
                H1,
                SYN,
                g,
                5,
            ),
        ];
        assert_eq!(expected_209(&rows), 3);
        assert_union_picks(&rows, "N2: an equal key goes to the first path");
    }

    /// N3 — an exact-range report beats a newer whole-year superset: the
    /// range score outranks `generated:`.
    #[test]
    fn an_exact_range_beats_a_newer_superset() {
        let rows: [Fx209; 2] = [
            (
                "lab-runs",
                LR,
                "backtest-20260101-000000-a.md",
                Y2024,
                SYN,
                Some("2026-09-01T00:00:00Z"),
                3,
            ),
            (
                "lab-runs",
                LR,
                "backtest-20260102-000000-b.md",
                H1,
                SYN,
                Some("2026-02-01T00:00:00Z"),
                5,
            ),
        ];
        assert_eq!(expected_209(&rows), 5);
        assert_union_picks(&rows, "N3: exact beats a newer superset");
    }

    /// N4 — a report with no `generated:` ranks below any dated one, though it
    /// sits at the smaller path.
    #[test]
    fn a_report_without_generated_ranks_oldest() {
        let rows: [Fx209; 2] = [
            (
                "lab-runs",
                LR,
                "backtest-20260101-000000-a.md",
                H1,
                SYN,
                None,
                3,
            ),
            (
                "lab-runs",
                LR,
                "backtest-20260102-000000-b.md",
                H1,
                SYN,
                Some("2020-01-01T00:00:00Z"),
                5,
            ),
        ];
        assert_eq!(expected_209(&rows), 5);
        assert_union_picks(&rows, "N4: no generated: ranks oldest");
    }

    /// X1 — across roots the newer `generated:` wins: `evidence/` holds a
    /// newer run than `lab-runs/` (searched first).
    #[test]
    fn a_newer_evidence_report_beats_an_older_lab_runs_one() {
        let rows: [Fx209; 2] = [
            (
                "lab-runs",
                LR,
                "backtest-20260601-000000-a.md",
                H1,
                SYN,
                Some("2026-02-01T00:00:00Z"),
                3,
            ),
            (
                "evidence",
                EV,
                "backtest-20260101-000000-b.md",
                H1,
                SYN,
                Some("2026-03-01T00:00:00Z"),
                5,
            ),
        ];
        assert_eq!(expected_209(&rows), 5);
        assert_union_picks(&rows, "X1: the newer evidence report wins");
    }

    /// X2 — an equal `generated:` across roots goes to `lab-runs/`, the root
    /// searched first (§ D5's identical-filename rule extends to an equal key).
    #[test]
    fn an_equal_generated_across_roots_goes_to_lab_runs() {
        let g = Some("2026-02-01T00:00:00Z");
        let rows: [Fx209; 2] = [
            (
                "lab-runs",
                LR,
                "backtest-20260601-000000-a.md",
                H1,
                SYN,
                g,
                3,
            ),
            (
                "evidence",
                EV,
                "backtest-20260101-000000-b.md",
                H1,
                SYN,
                g,
                5,
            ),
        ];
        assert_eq!(expected_209(&rows), 3);
        assert_union_picks(&rows, "X2: an equal key goes to lab-runs");
    }

    /// X3 — the tier outranks `generated:` across roots: a tagged older
    /// `evidence/` report beats an untagged newer `lab-runs/` one.
    #[test]
    fn a_tagged_older_evidence_report_beats_an_untagged_newer_lab_runs_one() {
        let rows: [Fx209; 2] = [
            (
                "lab-runs",
                LR,
                "backtest-20260601-000000-a.md",
                H1,
                None,
                Some("2026-09-01T00:00:00Z"),
                3,
            ),
            (
                "evidence",
                EV,
                "backtest-20260101-000000-b.md",
                H1,
                SYN,
                Some("2026-02-01T00:00:00Z"),
                5,
            ),
        ];
        assert_eq!(expected_209(&rows), 5);
        assert_union_picks(&rows, "X3: tagged beats a newer untagged");
    }

    /// X4 — the range score outranks `generated:` across roots: an exact
    /// `evidence/` report beats a newer `lab-runs/` superset.
    #[test]
    fn an_exact_evidence_report_beats_a_newer_lab_runs_superset() {
        let rows: [Fx209; 2] = [
            (
                "lab-runs",
                LR,
                "backtest-20260601-000000-a.md",
                Y2024,
                SYN,
                Some("2026-09-01T00:00:00Z"),
                3,
            ),
            (
                "evidence",
                EV,
                "backtest-20260101-000000-b.md",
                H1,
                SYN,
                Some("2026-02-01T00:00:00Z"),
                5,
            ),
        ];
        assert_eq!(expected_209(&rows), 5);
        assert_union_picks(&rows, "X4: exact beats a newer superset across roots");
    }

    /// F1 — § D5's identical-filename rule across roots, as Compare's
    /// `scan_report_roots_identical_filename_lab_runs_wins`: the SAME file
    /// name in `lab-runs/` (older `generated:`) and in `evidence/` (newer)
    /// resolves the `lab-runs/` copy, which is walked first; the later copy is
    /// skipped before it is read. Pair: with different names the newer
    /// `evidence/` report wins (X1's rule).
    #[test]
    fn an_identical_filename_across_roots_goes_to_lab_runs() {
        let same = "backtest-20260601-000000-btc-2024-h1-sma-cross.md";
        let older = Some("2026-02-01T00:00:00Z");
        let newer = Some("2026-03-01T00:00:00Z");
        let rows: [Fx209; 2] = [
            ("lab-runs", LR, same, H1, SYN, older, 3),
            ("evidence", EV, same, H1, SYN, newer, 5),
        ];
        assert_eq!(
            expected_209(&rows),
            3,
            "the table's first copy is lab-runs'"
        );
        assert_union_picks(&rows, "F1: an identical filename keeps the lab-runs copy");
        let pair: [Fx209; 2] = [
            ("lab-runs", LR, same, H1, SYN, older, 3),
            (
                "evidence",
                EV,
                "backtest-20260101-000000-b.md",
                H1,
                SYN,
                newer,
                5,
            ),
        ];
        assert_eq!(expected_209(&pair), 5, "distinct names: the newer report");
        assert_union_picks(
            &pair,
            "F1 pair: distinct names, the newer evidence report wins",
        );
    }

    /// F2 — the same rule inside one root, as Compare's
    /// `duplicate_filename_inside_a_root_goes_to_container_depth_0`: the SAME
    /// file name at container depth 0 (older `generated:`) and 1 (newer)
    /// resolves the container-depth-0 copy, through the union and through
    /// `load_equity` alone. Two-sided in the container's sort position, as
    /// E4b and Compare's `CONTAINERS_206`: once with a container that sorts
    /// before the slug (`a0`), once after it (`zz`), so plain full-path order
    /// would pick the depth-1 copy in one of them. Pair: with different names
    /// the newer container-depth-1 report wins, in both.
    #[test]
    fn an_identical_filename_inside_a_root_goes_to_container_depth_0() {
        let same = "backtest-20260601-000000-btc-2024-h1-sma-cross.md";
        let older = Some("2026-02-01T00:00:00Z");
        let newer = Some("2026-03-01T00:00:00Z");
        for depth1 in ["a0/v0-paper-sma/reports", "zz/v0-paper-sma/reports"] {
            assert_eq!(container_depth_of(depth1), 1);
            let rows: [Fx209; 2] = [
                ("evidence", EV0, same, H1, SYN, older, 3),
                ("evidence", depth1, same, H1, SYN, newer, 5),
            ];
            let want = expected_209(&rows);
            assert_eq!(
                want, 3,
                "{depth1}: the table's first copy is container depth 0"
            );
            assert_union_picks(
                &rows,
                "F2: an identical filename keeps the container-depth-0 copy",
            );
            let tmp = tempfile::tempdir().unwrap();
            let [_, evidence] = write_209(&tmp, &rows);
            let alone = load_equity(&sma_tuple(LabDataSource::Synthetic), &evidence)
                .expect("the root resolves");
            assert_eq!(
                alone.samples.len(),
                want,
                "{depth1}: F2 through load_equity alone"
            );
            let pair: [Fx209; 2] = [
                ("evidence", EV0, same, H1, SYN, older, 3),
                (
                    "evidence",
                    depth1,
                    "backtest-20260101-000000-b.md",
                    H1,
                    SYN,
                    newer,
                    5,
                ),
            ];
            assert_eq!(
                expected_209(&pair),
                5,
                "{depth1}: distinct names, the newer report"
            );
            assert_union_picks(
                &pair,
                "F2 pair: distinct names, the newer depth-1 report wins",
            );
        }
        assert!(
            "a0" < "v0-paper-sma" && "zz" > "v0-paper-sma",
            "the two containers straddle the slug name"
        );
    }

    /// F4 — a file name is claimed when it is walked, ahead of every check
    /// that can reject it (read, parse, match, source), as Compare's
    /// `seen_filenames` claims it before it reads: the FIRST copy of name N, in
    /// `lab-runs/`, is not a candidate for the Synthetic `H1_2024` tuple; the
    /// same name N in `evidence/` is valid and matches, but it is the later
    /// copy and is skipped, so the union misses (Compare drops the later copy
    /// too). Four variants of the first copy, one per check, each asserted to
    /// be rejected by that check: non-UTF-8 bytes (the read), no `scenario:`
    /// line (`parse_report_meta`), a `btc-2023-1m-…` range (`report_matches`)
    /// and `data_source: binance` (the source tier). Pair: with different
    /// names the `evidence/` report resolves. Every variant is run, and the
    /// test names each that fails.
    #[test]
    fn a_non_matching_first_copy_still_claims_its_file_name() {
        let same = "backtest-20260601-000000-btc-2024-h1-sma-cross.md";
        let older = "2026-02-01T00:00:00Z";
        let later = sma_report(H1, SYN, 5, "2026-03-01T00:00:00Z");
        let tuple = sma_tuple(LabDataSource::Synthetic);
        let mut not_utf8 = sma_report(H1, SYN, 3, older).into_bytes();
        not_utf8.extend_from_slice(b"- \xFF\xFE\n");
        let no_scenario =
            sma_report(H1, SYN, 3, older).replacen(&format!("scenario: {H1}\n"), "", 1);
        // The check that rejects `bytes` for `tuple`, asked in the order
        // `matching_candidates` asks them.
        let rejected_by = |bytes: &[u8]| -> &'static str {
            let Ok(text) = std::str::from_utf8(bytes) else {
                return "read";
            };
            let Some(meta) = parse_report_meta(text) else {
                return "parse";
            };
            let MatchQuality::Match { .. } = report_matches(&meta, &tuple) else {
                return "match";
            };
            match meta.data_source.as_deref() {
                Some(ds) if !source_tag_matches(ds, tuple.source) => "source",
                _ => "none",
            }
        };
        let firsts: [(&str, Vec<u8>); 4] = [
            ("read", not_utf8),
            ("parse", no_scenario.into_bytes()),
            (
                "match",
                sma_report("btc-2023-1m-sma-cross", SYN, 3, older).into_bytes(),
            ),
            (
                "source",
                sma_report(H1, Some("binance"), 3, older).into_bytes(),
            ),
        ];
        let write = |root: &std::path::Path, rel: &str, name: &str, bytes: &[u8]| {
            let dir = root.join(rel);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(name), bytes).unwrap();
        };
        let mut fails: Vec<String> = Vec::new();
        for (why, first) in &firsts {
            assert_eq!(
                rejected_by(first),
                *why,
                "precondition: the {why} variant is rejected by the {why} check"
            );
            for (name, want_hit) in [(same, false), ("backtest-20260101-000000-b.md", true)] {
                let tmp = tempfile::tempdir().unwrap();
                let (lab_runs, evidence) =
                    (tmp.path().join("lab-runs"), tmp.path().join("evidence"));
                write(&lab_runs, LR, same, first);
                write(&evidence, EV, name, later.as_bytes());
                let got = EquityCache::new().get_or_load_roots(&tuple, &[lab_runs, evidence]);
                let hit = got.as_ref().ok().map(|s| s.samples.len());
                if hit != want_hit.then_some(5) {
                    fails.push(format!(
                        "{why} variant, {}: got {got:?}",
                        if want_hit {
                            "pair (distinct names): the evidence report must resolve"
                        } else {
                            "the later same-named copy must be skipped"
                        }
                    ));
                }
            }
        }
        assert!(fails.is_empty(), "F4: {fails:#?}");
    }

    /// A matching `H1_2024` sma report that cannot be read for a series: no
    /// `## Equity curve`, no companion CSV and no `Initial capital` row, so
    /// `make_start_end_series` returns `ParseError`.
    fn broken_h1_report(generated: &str) -> String {
        format!(
            "---\nscenario: btc-2024-h1-sma-cross\nseed: 0xC0FFEE\ngenerated: {generated}\n\
             wall_clock_s: 0.1\ndata_source: synthetic\nstrategy:\nid: sma_crossover\n---\n\n\
             # Backtest Report — btc-2024-h1-sma-cross\n\n## Summary\n\n\
             | Metric       | Value           |\n|--------------|-----------------|\n\
             | Final equity | $103000.00 USDT |\n\n## Universe\n\n- BTCUSDT\n"
        )
    }

    /// F3 — a candidate that wins the key but cannot be read falls through to
    /// the next one: `evidence/` holds the NEWER matching report, broken
    /// (precondition: alone it is a `ParseError`); `lab-runs/` an older valid
    /// one, which the union resolves. Pair: with the `evidence/` report valid,
    /// it wins.
    #[test]
    fn a_broken_newer_report_falls_through_to_the_next_candidate() {
        let tuple = sma_tuple(LabDataSource::Synthetic);
        for broken in [true, false] {
            let tmp = tempfile::tempdir().unwrap();
            let (lab_runs, evidence) = (tmp.path().join("lab-runs"), tmp.path().join("evidence"));
            write_at(
                &lab_runs,
                LR,
                "backtest-20260601-000000-a.md",
                &sma_report(H1, SYN, 3, "2026-02-01T00:00:00Z"),
            );
            let newer = "2026-03-01T00:00:00Z";
            let text = if broken {
                broken_h1_report(newer)
            } else {
                sma_report(H1, SYN, 5, newer)
            };
            write_at(&evidence, EV, "backtest-20260101-000000-b.md", &text);
            let alone = load_equity(&tuple, &evidence);
            assert_eq!(
                matches!(alone, Err(EquityLoadError::ParseError { .. })),
                broken,
                "precondition: the evidence report matches and is broken iff declared; got \
                 {alone:?}"
            );
            let want = if broken { 3 } else { 5 };
            let got = EquityCache::new()
                .get_or_load_roots(&tuple, &[lab_runs, evidence])
                .expect("the union resolves");
            assert_eq!(
                got.samples.len(),
                want,
                "broken = {broken}: a newer report that cannot be read must fall through"
            );
        }
    }

    /// F5 — the fall-through inside ONE root: `lab-runs/` holds an older
    /// valid report `a` (3 points) and a NEWER broken one `b` (no equity
    /// table, no companion CSV, no `Initial capital` row; precondition: alone
    /// it is a `ParseError`). `b` wins the key but cannot be read, so `a`
    /// resolves, through the union and through `load_equity` alone. Pair: with
    /// `b` made valid (5 points) it wins. The expected values are the
    /// declared fixture point counts.
    #[test]
    fn a_broken_newest_report_falls_through_to_an_older_one_in_its_root() {
        let tuple = sma_tuple(LabDataSource::Synthetic);
        for broken in [true, false] {
            let tmp = tempfile::tempdir().unwrap();
            let (lab_runs, evidence) = (tmp.path().join("lab-runs"), tmp.path().join("evidence"));
            write_at(
                &lab_runs,
                LR,
                "backtest-20260101-000000-a.md",
                &sma_report(H1, SYN, 3, "2026-02-01T00:00:00Z"),
            );
            let newer = "2026-03-01T00:00:00Z";
            let text = if broken {
                broken_h1_report(newer)
            } else {
                sma_report(H1, SYN, 5, newer)
            };
            write_at(&lab_runs, LR, "backtest-20260102-000000-b.md", &text);
            let b_only = tmp.path().join("b-only");
            write_at(&b_only, LR, "backtest-20260102-000000-b.md", &text);
            let b_alone = load_equity(&tuple, &b_only);
            assert_eq!(
                matches!(b_alone, Err(EquityLoadError::ParseError { .. })),
                broken,
                "precondition: b matches and is broken iff declared; got {b_alone:?}"
            );
            let want = if broken { 3 } else { 5 };
            let union = EquityCache::new()
                .get_or_load_roots(&tuple, &[lab_runs.clone(), evidence])
                .expect("the union resolves");
            assert_eq!(
                union.samples.len(),
                want,
                "broken = {broken}: a newest report that cannot be read must yield to the older \
                 one in its own root"
            );
            let alone = load_equity(&tuple, &lab_runs).expect("the root resolves");
            assert_eq!(
                alone.samples.len(),
                want,
                "broken = {broken}: through load_equity alone"
            );
        }
    }

    /// The variant name of a lookup's error, `"hit"` for a series.
    fn outcome(result: &Result<LabEquitySeries, EquityLoadError>) -> &'static str {
        match result {
            Ok(_) => "hit",
            Err(EquityLoadError::ReportDirNotFound(_)) => "ReportDirNotFound",
            Err(EquityLoadError::NoReport { .. }) => "NoReport",
            Err(EquityLoadError::ParseError { .. }) => "ParseError",
            Err(EquityLoadError::EmptySeries { .. }) => "EmptySeries",
        }
    }

    /// F6 — the reads stop at the source tier (ADR-0055 D9 item 4, bug-log
    /// #209): an untagged legacy report is eligible only when no tagged report
    /// MATCHES, so a tagged matching report that cannot be read leaves the
    /// tuple with no curve; a readable untagged one does not stand in. Three
    /// placements: both in `lab-runs/`; the broken tagged report in
    /// `lab-runs/` and the untagged one in `evidence/` (the case the `#212`
    /// root-by-root union let through); and the reverse. The miss is the last
    /// root's error, as `resolve_roots` documents, DERIVED from what
    /// `evidence/` holds: nothing, `ReportDirNotFound`; the broken tagged
    /// report, its `ParseError`; only the untagged one, `NoReport` (it records
    /// no error: none of its reports is eligible). Pair: with no tagged
    /// report the untagged one resolves (3 points). Every placement is run,
    /// and the test names each that fails.
    #[test]
    fn a_broken_tagged_report_never_yields_to_an_untagged_one() {
        let tuple = sma_tuple(LabDataSource::Synthetic);
        let tagged = "backtest-20260101-000000-b.md";
        let untagged = "backtest-20260102-000000-a.md";
        let broken = broken_h1_report("2026-02-01T00:00:00Z");
        // The untagged report is the NEWER one: only the tier ranks it lower.
        let legacy = sma_report(H1, None, 3, "2026-03-01T00:00:00Z");
        let tmp = tempfile::tempdir().unwrap();
        write_at(tmp.path(), LR, tagged, &broken);
        assert_eq!(
            outcome(&load_equity(&tuple, tmp.path())),
            "ParseError",
            "precondition: the tagged report matches and cannot be read"
        );
        let mut fails: Vec<String> = Vec::new();
        for (placement, tagged_root, untagged_root) in [
            ("one root", "lab-runs", "lab-runs"),
            ("cross-root", "lab-runs", "evidence"),
            ("reverse cross-root", "evidence", "lab-runs"),
        ] {
            for with_tagged in [true, false] {
                let tmp = tempfile::tempdir().unwrap();
                let rel = |root: &str| if root == "lab-runs" { LR } else { EV };
                if with_tagged {
                    write_at(
                        &tmp.path().join(tagged_root),
                        rel(tagged_root),
                        tagged,
                        &broken,
                    );
                }
                write_at(
                    &tmp.path().join(untagged_root),
                    rel(untagged_root),
                    untagged,
                    &legacy,
                );
                let roots = [tmp.path().join("lab-runs"), tmp.path().join("evidence")];
                let want = match (with_tagged, tagged_root, untagged_root) {
                    (false, _, _) => "hit",
                    (true, "evidence", _) => "ParseError",
                    (true, _, "evidence") => "NoReport",
                    (true, _, _) => "ReportDirNotFound",
                };
                let got = EquityCache::new()
                    .get_or_load_roots(&tuple, &roots)
                    .map(|s| (*s).clone());
                let points = got.as_ref().ok().map(|s| s.samples.len());
                if outcome(&got) != want || (want == "hit" && points != Some(3)) {
                    fails.push(format!(
                        "{placement}, tagged report {}: want {want}, got {got:?}",
                        if with_tagged { "present" } else { "absent" }
                    ));
                }
                if placement == "one root" {
                    let alone = load_equity(&tuple, &roots[0]);
                    let want_alone = if with_tagged { "ParseError" } else { "hit" };
                    if outcome(&alone) != want_alone {
                        fails.push(format!(
                            "{placement} through load_equity, tagged report {}: want \
                             {want_alone}, got {alone:?}",
                            if with_tagged { "present" } else { "absent" }
                        ));
                    }
                }
            }
        }
        assert!(fails.is_empty(), "F6: {fails:#?}");
    }

    /// F7 — inside the tier the reads DO fall through the range score: an
    /// exact `H1_2024` report that cannot be read yields to a readable
    /// whole-year `2024` superset, drawn with its `narrowed_from` badge, which
    /// discloses the narrowing (bug-log #209). Pair: with the exact report
    /// readable it wins, with no badge. The superset is the NEWER report, so
    /// only the range score ranks it lower. Precondition: alone, the exact
    /// report matches and is a `ParseError` iff it is the broken one.
    #[test]
    fn a_broken_exact_report_falls_through_to_a_superset_with_its_badge() {
        let tuple = sma_tuple(LabDataSource::Synthetic);
        let exact = "backtest-20260101-000000-b.md";
        let superset = "backtest-20260102-000000-a.md";
        for broken in [true, false] {
            let tmp = tempfile::tempdir().unwrap();
            let lab_runs = tmp.path().join("lab-runs");
            let exact_text = if broken {
                broken_h1_report("2026-02-01T00:00:00Z")
            } else {
                sma_report(H1, SYN, 5, "2026-02-01T00:00:00Z")
            };
            write_at(&lab_runs, LR, exact, &exact_text);
            let exact_only = tmp.path().join("exact-only");
            write_at(&exact_only, LR, exact, &exact_text);
            let alone = load_equity(&tuple, &exact_only);
            assert_eq!(
                matches!(alone, Err(EquityLoadError::ParseError { .. })),
                broken,
                "precondition: the exact report matches and is broken iff declared; got {alone:?}"
            );
            write_at(
                &lab_runs,
                LR,
                superset,
                &sma_report(Y2024, SYN, 3, "2026-03-01T00:00:00Z"),
            );
            let got = EquityCache::new()
                .get_or_load_roots(&tuple, &[lab_runs, tmp.path().join("evidence")])
                .expect("the union resolves");
            let (want_name, want_points, want_badge) = if broken {
                (superset, 3, Some(superset))
            } else {
                (exact, 5, None)
            };
            assert_eq!(
                (
                    got.source_report.as_str(),
                    got.samples.len(),
                    got.narrowed_from.as_deref()
                ),
                (want_name, want_points, want_badge),
                "broken = {broken}: a broken exact report yields to the superset, with its badge"
            );
        }
    }

    /// F8 — a companion CSV that is present but does not read makes its run
    /// unreadable (bug-log #209): the engine writes the `.md` first and the
    /// CSV second, so a disk-full write leaves a complete `.md` beside a cut
    /// or empty CSV. In `lab-runs/`: an older complete run `a` (its `.md`
    /// start/end-only, a 4-sample CSV) and a newer run `b` (the same `.md`
    /// shape). With `b`'s CSV cut inside its last record, or empty, `a`
    /// resolves from its CSV. Pair: with `b`'s CSV intact `b` resolves from it
    /// (6 samples); with no CSV at all `b` resolves from its `.md`, a start/end
    /// pair (an absent companion is not a broken one: older committed reports
    /// have none). Compare's lenient `load_companion_equity_csv` reads the
    /// cut and the empty CSV as `None`, as before. Every variant is run, and
    /// the test names each that fails.
    #[test]
    fn a_run_whose_csv_is_cut_short_falls_through_to_an_older_one() {
        use crate::test_support::write_companion_equity_csv_for_md;
        let tuple = sma_tuple(LabDataSource::Synthetic);
        let samples = |n: i64, base: i64| -> Vec<(i64, Decimal)> {
            (0..n)
                .map(|i| {
                    (
                        1_704_067_200_000 + i * 86_400_000,
                        Decimal::from(base + i * 1_000),
                    )
                })
                .collect()
        };
        let (a, b) = (
            "backtest-20260101-000000-a.md",
            "backtest-20260102-000000-b.md",
        );
        let mut fails: Vec<String> = Vec::new();
        for csv_b in ["cut", "empty", "intact", "absent"] {
            let tmp = tempfile::tempdir().unwrap();
            let lab_runs = tmp.path().join("lab-runs");
            let dir = lab_runs.join(LR);
            write_at(
                &lab_runs,
                LR,
                a,
                &sma_report(H1, SYN, 0, "2026-02-01T00:00:00Z"),
            );
            write_at(
                &lab_runs,
                LR,
                b,
                &sma_report(H1, SYN, 0, "2026-03-01T00:00:00Z"),
            );
            write_companion_equity_csv_for_md(&dir.join(a), &samples(4, 100_000)).unwrap();
            let b_csv = dir.join("backtest-20260102-000000-b-equity.csv");
            if csv_b != "absent" {
                write_companion_equity_csv_for_md(&dir.join(b), &samples(6, 200_000)).unwrap();
                let text = std::fs::read_to_string(&b_csv).unwrap();
                let keep = match csv_b {
                    "cut" => text.trim_end().rfind('\n').unwrap() + 1 + 15,
                    "empty" => 0,
                    _ => text.len(),
                };
                std::fs::write(&b_csv, &text[..keep]).unwrap();
                assert_eq!(
                    reports::csv_artifacts::read_equity_csv(&b_csv).map_or(0, |v| v.len()),
                    if csv_b == "intact" { 6 } else { 0 },
                    "precondition: b's {csv_b} CSV reads 6 samples iff intact"
                );
                assert_eq!(
                    load_companion_equity_csv(&dir.join(b)).is_some(),
                    csv_b == "intact",
                    "Compare's lenient reader: b's {csv_b} CSV is None unless intact"
                );
            }
            let (want_name, want_points, want_fidelity) = match csv_b {
                "cut" | "empty" => (a, 4, Fidelity::PerBar),
                "intact" => (b, 6, Fidelity::PerBar),
                _ => (b, 2, Fidelity::StartEndOnly),
            };
            let roots = [lab_runs.clone(), tmp.path().join("evidence")];
            for (via, got) in [
                (
                    "union",
                    EquityCache::new()
                        .get_or_load_roots(&tuple, &roots)
                        .map(|s| (*s).clone()),
                ),
                ("load_equity", load_equity(&tuple, &lab_runs)),
            ] {
                let seen = got
                    .as_ref()
                    .map(|s| (s.source_report.to_string(), s.samples.len(), s.fidelity));
                if seen.as_ref().ok() != Some(&(want_name.to_string(), want_points, want_fidelity))
                {
                    fails.push(format!(
                        "b's CSV {csv_b}, via {via}: want ({want_name}, {want_points}, \
                         {want_fidelity:?}), got {seen:?}"
                    ));
                }
            }
        }
        assert!(fails.is_empty(), "F8: {fails:#?}");
    }

    // ── bug-log #232: the body offset never leaves the report ───────────────

    /// The body offset the code before bug-log #232 computed: one byte per
    /// line, plus the line's `str::lines` text, through the second `---`.
    fn pre_232_body_offset(content: &str) -> Option<usize> {
        let mut dashes = 0;
        let mut pos = 0;
        for line in content.lines() {
            pos += line.len() + 1;
            if line.trim() == "---" {
                dashes += 1;
                if dashes == 2 {
                    return Some(pos);
                }
            }
        }
        None
    }

    /// bug-log #232 — a report whose text ends at its frontmatter's closing
    /// `---` with no newline (a `.md` cut right there by a disk-full write)
    /// must not panic the walk (`parse_report_meta`) or the read
    /// (`find_body_start`); the old offset was one past the end
    /// (precondition). DERIVED outcome: its body is empty, so it has no
    /// Summary, while its scenario names its coin (`btcusdt-…`), so it matches
    /// the tuple and is a candidate that cannot be read: alone a `ParseError`,
    /// and beside an older readable run `a` of its root, which it outranks
    /// (newer `generated:`, smaller name), it yields to `a` (bug-log #209).
    /// Pair: the same text with the trailing newline, which the old offset
    /// handled, gives the same outcome.
    #[test]
    fn a_report_ending_at_its_closing_dashes_is_a_broken_candidate() {
        let tuple = sma_tuple(LabDataSource::Synthetic);
        let head = "---\nscenario: btcusdt-2024-h1-sma-cross\ngenerated: 2026-03-01T00:00:00Z\n\
                    data_source: synthetic\nstrategy:\nid: sma_crossover\n---";
        let (b, a) = (
            "backtest-20260101-000000-b.md",
            "backtest-20260102-000000-a.md",
        );
        for newline in ["", "\n"] {
            let text = format!("{head}{newline}");
            assert_eq!(
                pre_232_body_offset(&text),
                Some(text.len() + usize::from(newline.is_empty())),
                "precondition: the old offset is one past the end iff no newline"
            );
            let tmp = tempfile::tempdir().unwrap();
            let b_only = tmp.path().join("b-only");
            write_at(&b_only, LR, b, &text);
            let alone = load_equity(&tuple, &b_only);
            assert!(
                matches!(alone, Err(EquityLoadError::ParseError { .. })),
                "newline {newline:?}: alone, the frontmatter-only report is a matching report \
                 that cannot be read; got {alone:?}"
            );
            let lab_runs = tmp.path().join("lab-runs");
            write_at(&lab_runs, LR, b, &text);
            write_at(
                &lab_runs,
                LR,
                a,
                &sma_report(H1, SYN, 3, "2026-02-01T00:00:00Z"),
            );
            let got = load_equity(&tuple, &lab_runs).expect("the older run resolves");
            assert_eq!(
                (got.source_report.as_str(), got.samples.len()),
                (a, 3),
                "newline {newline:?}: the frontmatter-only report yields to the older run"
            );
        }
    }

    /// bug-log #232 — a report with `\r\n` line endings and a multi-byte
    /// character in its frontmatter must not panic: the old offset counted
    /// each line one byte short, and here it lands inside the character
    /// (precondition). DERIVED outcome: it reads exactly as its `\n` twin
    /// does (every other parser here reads it through `str::lines`, which
    /// strips both endings), and the twin is its declared 4-point per-bar
    /// table. Pair: the `\n` twin, whose offset the old code got right.
    #[test]
    fn a_crlf_report_reads_like_its_lf_twin() {
        let tuple = sma_tuple(LabDataSource::Synthetic);
        let lf = sma_report(H1, SYN, 4, "2026-02-01T00:00:00Z").replacen(
            "id: sma_crossover\n---\n",
            "id: sma_crossover\nnote: \u{e9}\u{e9}\u{e9}\n---\n",
            1,
        );
        assert!(lf.contains("note: "), "the note line landed");
        let crlf = lf.replace('\n', "\r\n");
        let true_offset = crlf.find("---\r\n\r\n# Backtest").unwrap() + 5;
        let old = pre_232_body_offset(&crlf).unwrap();
        assert!(
            old < true_offset && !crlf.is_char_boundary(old),
            "precondition: the old offset ({old}) lands inside a character before the body \
             ({true_offset})"
        );
        assert_eq!(
            pre_232_body_offset(&lf),
            lf.find("\n\n# Backtest").map(|i| i + 1)
        );
        let read = |text: &str| {
            let tmp = tempfile::tempdir().unwrap();
            write_at(tmp.path(), LR, "backtest-20260101-000000-a.md", text);
            load_equity(&tuple, tmp.path())
                .map(|s| (s.samples, s.fidelity, s.narrowed_from))
                .map_err(|e| e.to_string())
        };
        let twin = read(&lf);
        assert_eq!(
            twin.as_ref().map(|(s, f, _)| (s.len(), *f)),
            Ok((4, Fidelity::PerBar)),
            "the LF twin reads its declared 4-point table"
        );
        assert_eq!(read(&crlf), twin, "the CRLF report reads as its LF twin");
    }

    // ── bug-log #212: the report match (ADR-0055 D9) ─────────────────────────

    use crate::test_support::{LabReportFixture, lab_report};

    /// A `(strategy, symbol, H1_2024, Synthetic)` tuple.
    fn h1_tuple(strategy: &str, symbol: &str) -> LabTuple {
        LabTuple {
            strategy: SmolStr::new(strategy),
            symbol: SmolStr::new(symbol),
            range: DateRange::Preset(Preset::H1_2024),
            source: LabDataSource::Synthetic,
        }
    }

    /// An `H1_2024` synthetic report fixture for `strategy_id` whose curve
    /// starts at `level` (so the first equity value names the report).
    fn h1_fixture<'a>(
        strategy_id: Option<&'a str>,
        scenario: &'a str,
        symbol_row: Option<&'a str>,
        universe: &'a [&'a str],
        level: i64,
    ) -> LabReportFixture<'a> {
        LabReportFixture {
            strategy_id,
            scenario,
            symbol_row,
            universe,
            generated: "2026-06-01T12:00:00Z",
            data_source: "synthetic",
            level,
            falling: false,
        }
    }

    /// The first equity value of a resolved series, or `None` for a miss.
    fn first_level(result: &Result<LabEquitySeries, EquityLoadError>) -> Option<Decimal> {
        result
            .as_ref()
            .ok()
            .and_then(|s| s.samples.first())
            .map(|(_, v)| *v)
    }

    /// M1 — a report whose coin is named ONLY in its `## Summary` `| Symbol |`
    /// row (the shape of all 23 committed single-coin reports and of every
    /// engine-written single-symbol report: no `## Universe`, no ticker in the
    /// scenario) answers only that coin. The BTC report sits at the smaller
    /// path, so a coin-blind reader hands the ETH and SOL tuples the BTC run.
    #[test]
    fn a_coin_named_only_in_the_summary_row_matches_that_coin_only() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("lab-runs");
        // (file name, scenario, Summary coin, level)
        let table: [(&str, &str, &str, i64); 2] = [
            (
                "backtest-20260601-120000000-btc-2024-h1-sma-cross.md",
                "btc-2024-h1-sma-cross",
                "BTCUSDT",
                100_000,
            ),
            (
                "backtest-20260601-130000000-eth-2024-h1-sma-cross.md",
                "eth-2024-h1-sma-cross",
                "ETHUSDT",
                200_000,
            ),
        ];
        assert!(
            table[0].0 < table[1].0,
            "the BTC report has the smaller path"
        );
        for (name, scenario, coin, level) in table {
            let text = lab_report(&h1_fixture(
                Some("sma_crossover"),
                scenario,
                Some(coin),
                &[],
                level,
            ));
            assert!(
                !text.contains("## Universe") && !scenario.to_ascii_uppercase().contains("USDT"),
                "precondition: the coin is named only in the Summary row"
            );
            write_at(&root, "v0-paper-sma/reports", name, &text);
        }
        for coin in ["ETHUSDT", "BTCUSDT", "SOLUSDT"] {
            let want = table
                .iter()
                .find(|r| r.2 == coin)
                .map(|r| Decimal::from(r.3));
            let got = load_equity(&h1_tuple("v0.sma", coin), &root);
            assert_eq!(first_level(&got), want, "{coin}: got {got:?}");
            if want.is_none() {
                assert!(
                    matches!(got, Err(EquityLoadError::NoReport { .. })),
                    "{coin}: no report names it, so NoReport; got {got:?}"
                );
            }
        }
    }

    /// M2 — a multi-symbol report answers only a coin its `## Universe`
    /// lists.
    #[test]
    fn a_multi_symbol_report_matches_only_a_coin_in_its_universe() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("lab-runs");
        let universe = ["BTCUSDT", "ETHUSDT"];
        write_at(
            &root,
            "v1-cross-sectional-momentum/reports",
            "backtest-20260601-120000000-top10-2024-h1-momentum.md",
            &lab_report(&h1_fixture(
                Some("top10_momentum_h1"),
                "top10-2024-h1-momentum",
                None,
                &universe,
                100_000,
            )),
        );
        for coin in ["BTCUSDT", "ETHUSDT", "SOLUSDT"] {
            let got = load_equity(&h1_tuple("v1.momentum", coin), &root);
            if universe.contains(&coin) {
                assert_eq!(first_level(&got), Some(Decimal::from(100_000)), "{coin}");
            } else {
                assert!(
                    matches!(got, Err(EquityLoadError::NoReport { .. })),
                    "{coin} is outside the universe: NoReport; got {got:?}"
                );
            }
        }
    }

    /// M3 — a report that names no coin (no Summary `Symbol` row, no
    /// `## Universe`, no ticker in its scenario) answers no coin. Control: the
    /// same report with a Summary row answers its coin.
    #[test]
    fn a_report_naming_no_coin_matches_no_coin() {
        let tmp = tempfile::tempdir().unwrap();
        for (dir, symbol_row, resolves) in [("none", None, false), ("row", Some("BTCUSDT"), true)] {
            let root = tmp.path().join(dir);
            write_at(
                &root,
                "v0-paper-sma/reports",
                "backtest-20260601-120000000-btc-2024-h1-sma-cross.md",
                &lab_report(&h1_fixture(
                    Some("sma_crossover"),
                    "btc-2024-h1-sma-cross",
                    symbol_row,
                    &[],
                    100_000,
                )),
            );
            let got = load_equity(&h1_tuple("v0.sma", "BTCUSDT"), &root);
            assert_eq!(got.is_ok(), resolves, "{dir}: got {got:?}");
        }
    }

    /// M4 — a report answers only its own strategy. `v05-composed-strategies`
    /// holds a bbands report at the smaller path and a macd report at the
    /// larger one, equal in coin, range and source, so a strategy-blind reader
    /// hands the macd tuple the bbands run (and the rsi tuple too). The
    /// writer ids are DECLARED from the engine's dispatch arms.
    #[test]
    fn a_report_matches_only_its_own_strategy() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("lab-runs");
        // (file name, scenario, writer id, level)
        let table: [(&str, &str, &str, i64); 2] = [
            (
                "backtest-20260601-120000000-btcusdt-2024-h1-bbands-mean-revert-synthetic.md",
                "btcusdt-2024-h1-bbands-mean-revert-synthetic",
                "btc_bbands_mean_revert",
                100_000,
            ),
            (
                "backtest-20260601-130000000-btcusdt-2024-h1-macd-trend-synthetic.md",
                "btcusdt-2024-h1-macd-trend-synthetic",
                "btc_macd_trend",
                200_000,
            ),
        ];
        assert!(
            table[0].0 < table[1].0,
            "the bbands report has the smaller path"
        );
        for (name, scenario, id, level) in table {
            write_at(
                &root,
                "v05-composed-strategies/reports",
                name,
                &lab_report(&h1_fixture(Some(id), scenario, Some("BTCUSDT"), &[], level)),
            );
        }
        // (chip, the id its arm writes)
        for (chip, writes) in [
            ("v0.5.macd", "btc_macd_trend"),
            ("v0.5.bbands", "btc_bbands_mean_revert"),
            ("v0.5.rsi", "btc_rsi_reversion"),
        ] {
            let want = table
                .iter()
                .find(|r| r.2 == writes)
                .map(|r| Decimal::from(r.3));
            let got = load_equity(&h1_tuple(chip, "BTCUSDT"), &root);
            assert_eq!(first_level(&got), want, "{chip}: got {got:?}");
            if want.is_none() {
                assert!(
                    matches!(got, Err(EquityLoadError::NoReport { .. })),
                    "{chip}: its dir holds no report of its own, so NoReport; got {got:?}"
                );
            }
        }
    }

    /// M5 — a report with no `strategy:` `id:` answers no strategy. Control:
    /// the same report with its id answers.
    #[test]
    fn a_report_without_a_strategy_id_matches_no_strategy() {
        let tmp = tempfile::tempdir().unwrap();
        for (dir, id, resolves) in [("no-id", None, false), ("id", Some("sma_crossover"), true)] {
            let root = tmp.path().join(dir);
            write_at(
                &root,
                "v0-paper-sma/reports",
                "backtest-20260601-120000000-btcusdt-2024-h1-sma-cross-synthetic.md",
                &lab_report(&h1_fixture(
                    id,
                    "btcusdt-2024-h1-sma-cross-synthetic",
                    Some("BTCUSDT"),
                    &[],
                    100_000,
                )),
            );
            let got = load_equity(&h1_tuple("v0.sma", "BTCUSDT"), &root);
            assert_eq!(got.is_ok(), resolves, "{dir}: got {got:?}");
        }
    }

    /// M6 — a report answers a range only through its scenario's span token:
    /// the same token is an exact match, a whole-year `2024` report a narrowed
    /// superset of `H1_2024`/`H2_2024`, anything else nothing (a half of
    /// another year included). One root per (tuple range × fixture scenario). Every expectation comes from the
    /// DECLARED label table below, never from the loader; the two custom
    /// ranges' millisecond bounds are derived from the calendar.
    #[test]
    fn range_tokens_match_exactly() {
        use time::macros::datetime;
        let ms = |dt: time::OffsetDateTime| dt.unix_timestamp() * 1_000;
        let a = (
            ms(datetime!(2024-01-01 0:00 UTC)),
            ms(datetime!(2024-02-01 0:00 UTC)),
        );
        let b = (
            ms(datetime!(2024-03-01 0:00 UTC)),
            ms(datetime!(2024-04-01 0:00 UTC)),
        );
        let custom_a = format!("btcusdt-custom{}to{}-sma-cross-synthetic", a.0, a.1);
        let custom_b = format!("btcusdt-custom{}to{}-sma-cross-synthetic", b.0, b.1);
        // (label, fixture scenario). `h1y2023` and `h2y2025` are halves of
        // other years: they pin the year inside `Span::Half` (probe R13).
        let scenarios: [(&str, &str); 11] = [
            ("last30d", "btcusdt-last30d-sma-cross-synthetic"),
            ("last90d", "btcusdt-last90d-sma-cross-synthetic"),
            ("h1", "btcusdt-2024-h1-sma-cross-synthetic"),
            ("h2", "btcusdt-2024-h2-sma-cross-synthetic"),
            ("h1y2023", "btcusdt-2023-h1-sma-cross-synthetic"),
            ("h2y2025", "btcusdt-2025-h2-sma-cross-synthetic"),
            ("y2024", "btc-2024-sma-cross"),
            ("y2023", "btc-2023-1m-sma-cross"),
            ("ca", &custom_a),
            ("cb", &custom_b),
            ("none", "v1.momentum"),
        ];
        let custom = |start: &str, end: &str| DateRange::Custom {
            start_raw: SmolStr::new(start),
            end_raw: SmolStr::new(end),
        };
        // (tuple range, the label it matches exactly, the labels it takes as a
        // narrowed superset)
        let ranges: [(DateRange, &str, &[&str]); 6] = [
            (DateRange::Preset(Preset::Last30d), "last30d", &[]),
            (DateRange::Preset(Preset::Last90d), "last90d", &[]),
            (DateRange::Preset(Preset::H1_2024), "h1", &["y2024"]),
            (DateRange::Preset(Preset::H2_2024), "h2", &["y2024"]),
            (custom("2024-01-01", "2024-02-01"), "ca", &[]),
            (custom("2024-03-01", "2024-04-01"), "cb", &[]),
        ];
        let tmp = tempfile::tempdir().unwrap();
        let mut failures = Vec::new();
        for (ri, (range, exact, supersets)) in ranges.iter().enumerate() {
            for (si, (label, scenario)) in scenarios.iter().enumerate() {
                let root = tmp.path().join(format!("r{ri}-s{si}"));
                write_at(
                    &root,
                    "v0-paper-sma/reports",
                    "backtest-20260601-120000000-fixture.md",
                    &lab_report(&h1_fixture(
                        Some("sma_crossover"),
                        scenario,
                        Some("BTCUSDT"),
                        &[],
                        100_000,
                    )),
                );
                let want = if label == exact {
                    "exact"
                } else if supersets.contains(label) {
                    "superset"
                } else {
                    "none"
                };
                let tuple = LabTuple {
                    range: range.clone(),
                    ..h1_tuple("v0.sma", "BTCUSDT")
                };
                let got = match load_equity(&tuple, &root) {
                    Ok(s) if s.narrowed_from.is_none() => "exact",
                    Ok(_) => "superset",
                    Err(EquityLoadError::NoReport { .. }) => "none",
                    Err(e) => panic!("{range:?} × {scenario}: unexpected {e}"),
                };
                if got != want {
                    failures.push(format!("{range:?} × {scenario}: want {want}, got {got}"));
                }
            }
        }
        assert!(
            failures.is_empty(),
            "range mismatches:\n{}",
            failures.join("\n")
        );
    }

    /// The `id:` line of the committed real-weights TCN reports
    /// (`*-tcn-overlay-weights.md`, read from the files git lists), asserted
    /// to be one id. M8's writer-id row for `v2.5.tcn.weights`.
    fn committed_tcn_weights_id() -> String {
        use crate::test_support::evidence_oracle;
        let evidence = default_evidence_root();
        let ws = evidence.parent().unwrap().to_path_buf();
        let files = evidence_oracle::git_listed_files(&mut evidence_oracle::live_git(&ws))
            .unwrap_or_else(|e| panic!("the evidence oracle must run: {e}"));
        let mut ids: Vec<String> = files
            .iter()
            .filter(|c| {
                evidence_oracle::is_contract_report(c)
                    && c[c.len() - 1].ends_with("-tcn-overlay-weights.md")
            })
            .map(|c| {
                let body =
                    std::fs::read_to_string(c.iter().fold(ws.clone(), |p, x| p.join(x))).unwrap();
                body.lines()
                    .find_map(|l| l.strip_prefix("id: "))
                    .unwrap()
                    .trim()
                    .to_string()
            })
            .collect();
        ids.sort();
        ids.dedup();
        assert_eq!(
            ids.len(),
            1,
            "the committed real-weights TCN reports carry one id: {ids:?}"
        );
        ids.remove(0)
    }

    /// M8 — every Lab chip, and every fixtures-build strategy id, reads the
    /// directory the engine writes it to (`backtest::engine::strategy_dir_slug`)
    /// and matches the `id:` its writer emits. The writer ids are a DECLARED
    /// table, never `report_strategy_id`; the real-weights TCN row is the
    /// `id:` line of the committed `*-tcn-overlay-weights.md` reports (no ui
    /// build enables `backtest/candle`, so no test can run that writer). The
    /// chip list is the one `cockpit_live` seeds (`LAB_SEED_STRATEGIES`) plus
    /// `fixtures::fake_strategy_rows()`, asserted equal to the table's.
    #[test]
    fn every_lab_chip_reads_the_dir_the_engine_writes() {
        let weights_id = committed_tcn_weights_id();
        let weights_id = weights_id.as_str();
        let single: &[&str] = &[];
        let universe: &[&str] = &["BTCUSDT", "ETHUSDT"];
        // (chip id, the id its writer emits, Summary coin, Universe)
        let table: [(&str, &str, Option<&str>, &[&str]); 11] = [
            ("v0.sma", "sma_crossover", Some("BTCUSDT"), single),
            ("v0.5.macd", "btc_macd_trend", Some("BTCUSDT"), single),
            ("v0.5.rsi", "btc_rsi_reversion", Some("BTCUSDT"), single),
            (
                "v0.5.bbands",
                "btc_bbands_mean_revert",
                Some("BTCUSDT"),
                single,
            ),
            ("v1.momentum", "top10_momentum_h1", None, universe),
            ("v1.5a.pairs", "pairs_mr_h1", None, universe),
            (
                "v2.5.tcn",
                "tcn_overlay_momentum/tcn_overlay_momentum",
                None,
                universe,
            ),
            ("v2.5.tcn.weights", weights_id, None, universe),
            ("btc_macd_trend", "btc_macd_trend", Some("BTCUSDT"), single),
            (
                "btc_rsi_reversion",
                "btc_rsi_reversion",
                Some("BTCUSDT"),
                single,
            ),
            (
                "btc_bbands_mean_revert",
                "btc_bbands_mean_revert",
                Some("BTCUSDT"),
                single,
            ),
        ];
        let mut production: Vec<String> = crate::lab::defaults::LAB_SEED_STRATEGIES
            .iter()
            .map(|(id, _)| (*id).to_string())
            .chain(
                crate::fixtures::fake_strategy_rows()
                    .into_iter()
                    .map(|r| r.id.0.to_string()),
            )
            .collect();
        let mut declared: Vec<String> = table.iter().map(|r| r.0.to_string()).collect();
        production.sort();
        declared.sort();
        assert_eq!(
            declared, production,
            "the table covers exactly the production ids"
        );

        let tmp = tempfile::tempdir().unwrap();
        let mut failures = Vec::new();
        for (i, (chip, writes, symbol_row, universe)) in table.iter().enumerate() {
            let root = tmp.path().join(format!("chip-{i}"));
            write_at(
                &root,
                &format!("{}/reports", backtest::engine::strategy_dir_slug(chip)),
                "backtest-20260601-120000000-btcusdt-2024-h1-lab-chip-synthetic.md",
                &lab_report(&h1_fixture(
                    Some(writes),
                    "btcusdt-2024-h1-lab-chip-synthetic",
                    *symbol_row,
                    universe,
                    100_000,
                )),
            );
            let got = load_equity(&h1_tuple(chip, "BTCUSDT"), &root);
            if first_level(&got) != Some(Decimal::from(100_000)) {
                failures.push(format!("{chip} (writes `{writes}`): {got:?}"));
            }
        }
        assert!(
            failures.is_empty(),
            "chips that do not read back their engine-written report:\n{}",
            failures.join("\n")
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
