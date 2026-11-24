//! Phase E — Report-cache scanner for the Compare matrix (R3.1-R3.5).
//!
//! Exposes:
//! - `scan_spec_tree(spec_root: &Path) -> BTreeMap<(SmolStr, Symbol, DateRange), CachedCell>`
//!   — cold-boot layout walk + frontmatter parse.
//! - `lookup_cell(strategy_id, symbol, range, cache) -> Option<&CachedCell>`
//!   — O(log n) lookup into an already-built cache.
//! - `parse_frontmatter(content: &str) -> Option<BTreeMap<SmolStr, SmolStr>>`
//!   — private hand-parser for flat YAML frontmatter (§1.1 of decomp.md).
//!
//! ## Parser shape (§1.1)
//!
//! Contract (K3 resolution — no `serde_yaml` dep, hand-parse):
//! - Reads content between the leading `---` and the next `---` line.
//! - For each line outside the `strategy:` block, splits on the first
//!   `: ` and stores the key→value pair.
//! - For lines inside the `strategy:` block (detected via 2-space indent),
//!   stores under a `strategy.<key>` namespace.
//! - Returns `None` on any parse error (fail-soft per K2).
//!
//! ## The walk (bug-log #206)
//!
//! Every root is walked through the ONE report-root layout,
//! `crate::reports::layout` (ADR-0055 § D8):
//! `<root>[/<container>]/<slug>/reports/backtest-*.md` at container depth
//! ≤ `layout::MAX_CONTAINER_DEPTH`, report dirs in (container depth, path)
//! order and each dir's files by path. A cell is keyed by its frontmatter's
//! `strategy.id`; no directory name is read as a strategy or a slug.
//!
//! ## Scenario → universe mapping (R3.2)
//!
//! Scenario names encode the universe and period via a naming convention
//! verified against the 32 live backtest reports at 2026-05-20 commit:
//! - `top10-*` → 10-symbol universe
//! - `btc-*`   → BTCUSDT only
//! - `pairs-*` → (BTCUSDT, ETHUSDT) — v1.5a.pairs universe

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use smol_str::SmolStr;
use trading_core::Symbol;

use crate::compare::state::CachedCell;
use crate::lab::state::DateRange;

// ── Top-10 momentum universe (v1.momentum + v2.5.tcn) ───────────────────────
//
// Matches the `config/strategies/top10_momentum_h1.toml` universe list.
// XRP-first ordering per Phase A Q7 default (also matches `lab::universe`).
const TOP10_UNIVERSE: &[&str] = &[
    "XRPUSDT", "BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT", "ADAUSDT", "DOTUSDT", "DOGEUSDT",
    "LINKUSDT", "AVAXUSDT",
];

// ── Pairs-MR universe (v1.5a.pairs) ─────────────────────────────────────────
//
// `config/strategies/pairs_mr_h1.toml` — confirmed by architect T-T1-2.
const PAIRS_UNIVERSE: &[&str] = &["BTCUSDT", "ETHUSDT"];

/// Derive the universe symbol list from a scenario name (R3.2).
///
/// Returns `None` when the scenario name doesn't match a known prefix
/// (e.g. unknown test scenarios) — the cell is silently skipped in
/// `scan_spec_tree`.
fn scenario_universe(scenario: &str) -> Option<Vec<Symbol>> {
    if scenario.starts_with("top10") {
        Some(TOP10_UNIVERSE.iter().map(|s| Symbol::new(*s)).collect())
    } else if scenario.starts_with("btc") {
        Some(vec![Symbol::new("BTCUSDT")])
    } else if scenario.starts_with("pairs") {
        Some(PAIRS_UNIVERSE.iter().map(|s| Symbol::new(*s)).collect())
    } else {
        None
    }
}

/// Returns `true` when the scenario covers a multi-symbol universe
/// (drives the K7 disclaimer).
fn scenario_is_multi_symbol(scenario: &str) -> bool {
    scenario.starts_with("top10") || scenario.starts_with("pairs")
}

/// Hand-parse the YAML frontmatter at the head of a backtest report.
///
/// Returns `None` on any parse error (fail-soft per K2; a `tracing::warn!`
/// records the offending call site).
///
/// Contract (§1.1 of decomp.md):
/// - Finds content between the first `---` line and the next `---` line.
/// - For lines outside the `strategy:` block: splits on the first `": "`
///   and stores `key → value`.
/// - For lines inside the `strategy:` block (2-space indent): stores
///   under `strategy.<key>`.
/// - Returns `BTreeMap<SmolStr, SmolStr>` (deterministic ordering for tests).
///
/// `pub(crate)` so the Lab's report lookup (`lab::equity_loader`) reads a
/// report's `strategy.id` through this same tolerant reader (bug-log #212).
pub(crate) fn parse_frontmatter(content: &str) -> Option<BTreeMap<SmolStr, SmolStr>> {
    let mut lines = content.lines();

    // Expect the first non-empty line to be `---`.
    let first = lines.next()?.trim();
    if first != "---" {
        return None;
    }

    let mut map = BTreeMap::new();
    let mut in_strategy_block = false;

    for line in lines {
        // End of frontmatter.
        if line.trim() == "---" {
            break;
        }

        // Detect the `strategy:` block start (key only, no value on same line).
        if line == "strategy:" {
            in_strategy_block = true;
            continue;
        }

        // Lines inside the strategy block start with 2 spaces — in the
        // hand-written fixtures. The ENGINE-written reports emit the block
        // UNINDENTED (the `report::sma` template's `\`-continuations elide
        // the leading whitespace; the rendered bytes are hash-locked by the
        // determinism anchors, so the writer cannot be fixed — 2026-07-26
        // review, discovered when the revived AC5 round-trip test found
        // `scan_one_root` skipping every engine-written report for lack of
        // `strategy.id`). Tolerant-reader rule: an unindented `key: value`
        // whose key is a KNOWN strategy sub-key stays in the block.
        if in_strategy_block {
            if let Some(inner) = line.strip_prefix("  ") {
                if let Some(sep) = inner.find(": ") {
                    let k = SmolStr::new(format!("strategy.{}", &inner[..sep]));
                    let v = SmolStr::new(inner[sep + 2..].trim());
                    map.insert(k, v);
                }
            } else if let Some(sep) = line.find(": ")
                && matches!(
                    &line[..sep],
                    "id" | "kind" | "content_hash" | "source" | "signal"
                )
            {
                let k = SmolStr::new(format!("strategy.{}", &line[..sep]));
                let v = SmolStr::new(line[sep + 2..].trim());
                map.insert(k, v);
            } else {
                // Non-indented, non-sub-key line → we've exited the block.
                in_strategy_block = false;
                // Fall through to process this line as a top-level key.
                if let Some(sep) = line.find(": ") {
                    let k = SmolStr::new(&line[..sep]);
                    let v = SmolStr::new(line[sep + 2..].trim());
                    map.insert(k, v);
                }
            }
            continue;
        }

        // Top-level key: value line.
        if let Some(sep) = line.find(": ") {
            let k = SmolStr::new(&line[..sep]);
            let v = SmolStr::new(line[sep + 2..].trim());
            map.insert(k, v);
        }
    }

    if map.is_empty() { None } else { Some(map) }
}

/// Return the report body that follows the YAML frontmatter — the substring
/// after the SECOND `---`-only delimiter line (or the whole content when there
/// is no frontmatter). Line-based so a Markdown table separator (`|---|---|`)
/// inside the body is never mistaken for the frontmatter terminator
/// (lab-run-save-compare T5 fix).
fn body_after_frontmatter(content: &str) -> &str {
    let mut dash_lines = 0u32;
    let mut offset = 0usize;
    for line in content.lines() {
        offset += line.len() + 1; // +1 for the consumed '\n'
        if line.trim() == "---" {
            dash_lines += 1;
            if dash_lines == 2 {
                return content.get(offset..).unwrap_or("");
            }
        }
    }
    // No closing frontmatter delimiter — treat the whole content as body.
    content
}

/// Extract a `CachedCell` KPI snapshot from the Markdown body of a backtest
/// report.
///
/// The KPI fields (Sharpe, total return, max drawdown, trade count, equity
/// curve) live in the Markdown body's `## Summary` table — NOT in the
/// frontmatter. This function scans the body for those fields.
///
/// Returns `None` if the body doesn't contain the expected table rows (fail-
/// soft per K2).
fn extract_kpis_from_body(
    content: &str,
    frontmatter: &BTreeMap<SmolStr, SmolStr>,
    source_path: &str,
    is_multi_symbol: bool,
    equity_series_ts: Vec<(i64, rust_decimal::Decimal)>,
) -> Option<CachedCell> {
    // Skip the frontmatter block via a LINE-based scan to the closing `---`
    // delimiter line.
    //
    // lab-run-save-compare T5 fix: the previous `content.splitn(4, "---")`
    // truncated the body at the FIRST `---` substring — which a Markdown table
    // separator row (`|----------|----------|`) always contains — so no data
    // row was ever in the returned "body" and every KPI parsed as 0. (The bug
    // was dormant: `scan_spec_tree` had no production caller until this
    // feature wired the Compare cold-boot path.) A delimiter-LINE scan stops
    // only at a line that is exactly `---`, leaving the full `## Summary`
    // table intact.
    let body = body_after_frontmatter(content);

    // Parse key metrics from the body table.
    // The body contains a Markdown table with rows like:
    //   | Sharpe ratio     | **0.94**  |
    //   | Total return     | **12.3 %**|
    //   | Max drawdown     | **-5.6 %**|
    //   | Trade count      | **42**    |
    //
    // Fail-soft contract (K2): if the body has NO recognizable KPI table row
    // at all, return `None` so the file is skipped rather than producing a
    // junk all-zero cell (e.g. a non-report `.md` that slipped the name
    // filter). `Total return` / `Sharpe ratio` are the load-bearing rows every
    // backtest report carries; absence of BOTH means "this is not a report
    // body we can read".
    let sharpe_raw = extract_table_value(body, "Sharpe ratio")
        .and_then(|v| clean_bold_value(&v).parse::<f64>().ok());
    let total_return_raw = extract_table_value(body, "Total return").and_then(|v| {
        // Strip trailing " %" or "%" before parsing.
        let cleaned = clean_bold_value(&v).replace('%', "").trim().to_string();
        cleaned.parse::<f64>().ok()
    });
    if sharpe_raw.is_none() && total_return_raw.is_none() {
        return None;
    }
    let sharpe = sharpe_raw.unwrap_or(0.0);
    let total_return_pct = total_return_raw.unwrap_or(0.0);

    let max_drawdown_pct = extract_table_value(body, "Max drawdown")
        .and_then(|v| {
            let cleaned = clean_bold_value(&v).replace('%', "").trim().to_string();
            cleaned.parse::<f64>().ok()
        })
        .unwrap_or(0.0);

    // Real reports label this `Trades`; older fixtures use `Trade count`. Try
    // both (prefix match on `extract_table_value`, so `Trade count` also
    // catches `Trades` — but be explicit for readability).
    let trade_count = extract_table_value(body, "Trade count")
        .or_else(|| extract_table_value(body, "Trades"))
        .and_then(|v| clean_bold_value(&v).parse::<u32>().ok())
        .unwrap_or(0);

    // Extract equity curve tail (at most 30 data points) from the body.
    // The body contains a `## Equity curve` or similar section with
    // comma-separated values or a table. We do a best-effort parse.
    let equity_curve_tail = extract_equity_curve_tail(body, 30);

    let generated_at = frontmatter
        .get("generated")
        .cloned()
        .unwrap_or_else(|| SmolStr::new(""));

    Some(CachedCell {
        sharpe,
        total_return_pct,
        max_drawdown_pct,
        trade_count,
        equity_curve_tail,
        // lab-compare-equity-overlay T1: the full timestamped per-bar series
        // from the report's companion equity CSV (empty for start-end-only
        // cells — older committed reports have no companion). Resolved by the
        // caller via `equity_loader::load_companion_equity_csv` so the Compare
        // overlay reads the EXACT series the Lab cold path does.
        equity_series_ts,
        source_report_path: SmolStr::new(source_path),
        generated_at,
        is_multi_symbol,
    })
}

/// Find the first occurrence of a Markdown table row with the given label
/// and return the value cell content.
///
/// Matches rows of the form: `| Label text | value |` (case-insensitive
/// prefix match on the label).
fn extract_table_value(body: &str, label: &str) -> Option<String> {
    let label_lower = label.to_lowercase();
    for line in body.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = trimmed.split('|').collect();
        // cells[0] is empty (before leading |), cells[1] is label, cells[2] is value.
        if cells.len() >= 3 {
            let cell_label = cells[1].trim().to_lowercase();
            if cell_label.starts_with(&label_lower) {
                return Some(cells[2].trim().to_string());
            }
        }
    }
    None
}

/// Strip surrounding `**` bold markers from a Markdown value cell.
fn clean_bold_value(v: &str) -> String {
    v.trim()
        .trim_start_matches("**")
        .trim_end_matches("**")
        .trim()
        .to_string()
}

/// Extract at most `max_points` equity-curve values from the body.
///
/// Looks for lines that contain only comma-separated floats or a
/// `| equity |` table — best-effort; returns empty on no match.
fn extract_equity_curve_tail(body: &str, max_points: usize) -> Vec<f64> {
    // Look for a CSV section after `## Equity curve` or similar heading.
    let mut collecting = false;
    let mut values = Vec::new();

    for line in body.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("## ") && trimmed.to_lowercase().contains("equity") {
            collecting = true;
            continue;
        }
        if collecting {
            // Stop at the next heading.
            if trimmed.starts_with('#') {
                break;
            }
            // Try to parse comma-separated floats.
            if trimmed.contains(',') || trimmed.parse::<f64>().is_ok() {
                for part in trimmed.split(',') {
                    if let Ok(v) = part.trim().parse::<f64>() {
                        values.push(v);
                    }
                }
            }
        }
    }

    // Return the trailing `max_points` entries.
    let len = values.len();
    if len > max_points {
        values[len - max_points..].to_vec()
    } else {
        values
    }
}

/// Scan a SINGLE report root for its `backtest-*.md` reports (walked as the
/// module doc's "The walk" says, bug-log #206) and build a
/// `BTreeMap<(strategy_id, symbol, range), CachedCell>` cache.
///
/// Only the most-recent report per `(strategy_id, symbol, range)` tuple
/// is kept (R3.3: most-recent wins; older reports reachable from Trail).
///
/// `spec_root` should be the absolute path to one report root (the repo's
/// `evidence/` directory — the byte-immutable reports corpus, `spec/` housed
/// it until the 2026-07-25 BMAD-migration Phase 3 `git mv` — or a
/// `lab-runs/` tree). On parse failure the file is skipped with a
/// `tracing::warn!` (K2 fail-soft).
///
/// For the production two-root union (`lab-runs/` + `evidence/`) use
/// [`scan_report_roots`] (lab-run-save-compare T5 / Q4).
#[must_use]
pub fn scan_spec_tree(spec_root: &Path) -> BTreeMap<(SmolStr, Symbol, DateRange), CachedCell> {
    let roots = [spec_root.to_path_buf()];
    scan_report_roots(&roots).0
}

/// Scan a FIXED-ORDER union of report roots and build the Compare cache
/// (lab-run-save-compare T5 / Q4 / ADR-0055 § D5).
///
/// Production passes `[default_lab_runs_root(), default_evidence_root()]` —
/// **lab-runs FIRST, then evidence/**. Precedence rules (pinned, ADR-0055 § D5):
///
/// 1. Search order is the slice order: `lab-runs/` first.
/// 2. On an **identical filename across roots** (same `backtest-<stamp>-<scenario>.md`
///    in both `lab-runs/` and `evidence/`), the FIRST root's copy wins — the later
///    root's same-named file is skipped before it is even parsed.
/// 3. Within the union, the existing **most-recent-`generated:`-wins**
///    per-tuple tiebreaker decides which report represents a tuple in Compare.
/// 4. **Inside one root** (bug-log #206) the walk's ADR-0055 D8 order settles
///    the same two first-wins rules: an identical filename keeps its first
///    copy and an equal `generated:` keeps the first report, container depth 0
///    before container depth 1, then by path. A strictly newer `generated:`
///    still wins at either container depth.
///
/// # The tally (story 6-11 AC3)
///
/// Returns what it FOUND alongside what it admitted. `#66` A.4 was invisible precisely
/// because this function could only report its output: forty reports whose
/// `strategy.id` had been filed top-level were skipped by a bare `continue`, and an
/// empty Compare matrix is indistinguishable from one whose scanner rejected
/// everything. The tally makes "0 admitted of 40 discovered, all for a missing
/// `strategy.id`" a thing the program can say.
#[must_use]
pub fn scan_report_roots(
    roots: &[std::path::PathBuf],
) -> (
    BTreeMap<(SmolStr, Symbol, DateRange), CachedCell>,
    crate::session_log::ScanTally,
) {
    let mut cache: BTreeMap<(SmolStr, Symbol, DateRange), CachedCell> = BTreeMap::new();
    let mut tally = crate::session_log::ScanTally::default();
    // Filenames already claimed by an earlier root (ADR-0055 § D5 #2:
    // lab-runs wins) or by an earlier report dir of the same root (D8 order,
    // bug-log #206), so a later same-named file is skipped.
    let mut seen_filenames: BTreeSet<SmolStr> = BTreeSet::new();
    for root in roots {
        scan_one_root(root, &mut cache, &mut seen_filenames, &mut tally);
    }
    (cache, tally)
}

/// Scan one report root into `cache`, honoring the cross-root filename
/// collision rule via `seen_filenames` (earlier roots win).
///
/// bug-log #206 — walks the ONE report-root layout (`crate::reports::layout`,
/// ADR-0055 § D8): `<root>[/<container>]/<slug>/reports/backtest-*.md` at
/// container depth ≤ `layout::MAX_CONTAINER_DEPTH`, `reports` never a
/// container or a slug, report dirs in (container depth, path) order and each
/// dir's files by path. Within one root that order settles both first-wins
/// rules below (the identical-filename skip and the equal-`generated:` skip)
/// for the container-depth-0 report, then the first by path. An absent or
/// unreadable directory contributes nothing; the layout logs it (K2).
fn scan_one_root(
    root: &Path,
    cache: &mut BTreeMap<(SmolStr, Symbol, DateRange), CachedCell>,
    seen_filenames: &mut BTreeSet<SmolStr>,
    tally: &mut crate::session_log::ScanTally,
) {
    use crate::reports::layout;
    use crate::session_log::Rejected;
    use std::fs;

    for rd in layout::report_dirs(root) {
        for report_path in layout::backtest_files(&rd.dir) {
            let Some(fname) = report_path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };

            // Collision rule: if an earlier root (lab-runs wins, ADR-0055 § D5)
            // or an earlier report dir of this root (D8 order) already claimed
            // this exact filename, skip it.
            let fname_key = SmolStr::new(fname);
            if !seen_filenames.insert(fname_key) {
                tally.reject(Rejected::DuplicateFilename);
                continue;
            }

            let Ok(content) = fs::read_to_string(&report_path) else {
                tracing::warn!("compare::cache: failed to read {}", report_path.display());
                tally.reject(Rejected::Unreadable);
                continue;
            };

            let Some(fm) = parse_frontmatter(&content) else {
                tracing::warn!(
                    "compare::cache: malformed frontmatter in {}",
                    report_path.display()
                );
                tally.reject(Rejected::MalformedFrontmatter);
                continue;
            };

            let Some(scenario) = fm.get("scenario") else {
                tally.reject(Rejected::MissingScenario);
                continue;
            };

            let Some(strategy_id_raw) = fm.get("strategy.id") else {
                // `#66` A.4's bucket: the frontmatter parsed, the key was filed
                // top-level by an unindented writer template, and every report in the
                // corpus landed here silently.
                tally.reject(Rejected::MissingStrategyId);
                continue;
            };

            let strategy_id = SmolStr::new(strategy_id_raw.as_str());

            let Some(universe) = scenario_universe(scenario) else {
                tally.reject(Rejected::UnknownScenario);
                continue;
            };

            let is_multi = scenario_is_multi_symbol(scenario);

            // Use the repo-relative path as the cell identifier.
            // Build a repo-relative path by stripping the root's parent prefix.
            let source_path = report_path
                .strip_prefix(root.parent().unwrap_or(root))
                .map_or_else(
                    |_| report_path.to_string_lossy().to_string(),
                    |p| p.to_string_lossy().to_string(),
                );

            // lab-compare-equity-overlay T1: hydrate the cell's timestamped
            // per-bar series from the companion equity CSV beside this report
            // (`backtest-<stamp>-<scenario>-equity.csv`), via the SAME loader
            // the Lab cold path uses. Graceful fallback: a missing/unparseable
            // companion yields an empty series — the overlay simply has nothing
            // to draw for this cell (no fake curve, no panic). Older committed
            // `spec/` reports without a companion CSV stay overlay-blank.
            let equity_series_ts =
                crate::lab::equity_loader::load_companion_equity_csv(&report_path)
                    .unwrap_or_default();

            let Some(cell) =
                extract_kpis_from_body(&content, &fm, &source_path, is_multi, equity_series_ts)
            else {
                tracing::warn!("compare::cache: no KPI table in {}", report_path.display());
                tally.reject(Rejected::NoKpiTable);
                continue;
            };
            // Admitted: one per REPORT, matching the denominator, which counts report
            // files. The per-symbol fan-out below multiplies cells, not candidates.
            tally.admit();

            // Use default range (Last90d) since reports don't encode a date-range
            // directly comparable to `DateRange`. The cache lookup at view-render
            // time uses the compare_screen_state.range; for now every report maps
            // to `DateRange::default()`. See R3.5 note: at v0.1.0 the cache is
            // read-only; per-range resolution is a v0.2.0 follow-up.
            let range = DateRange::default();

            // For each symbol in the scenario's universe, insert a cell (Q6=a:
            // render all cells with the same aggregate KPI for multi-symbol).
            for symbol in &universe {
                let key = (strategy_id.clone(), symbol.clone(), range.clone());

                // R3.3: keep only the most-recent report per tuple; an equal
                // `generated:` keeps the first one in D8 order.
                if cache.get(&key).is_some_and(|existing| {
                    cell.generated_at.as_str() <= existing.generated_at.as_str()
                }) {
                    continue;
                }
                cache.insert(key, cell.clone());
            }
        }
    }
}

/// Look up a single cell from a pre-built cache (O(log n)).
///
/// Returns `None` when the `(strategy_id, symbol, range)` tuple has no
/// cached report — the matrix renders the "Run" affordance (Q4=b).
#[must_use]
pub fn lookup_cell<'a>(
    strategy_id: &SmolStr,
    symbol: &Symbol,
    range: &DateRange,
    cache: &'a BTreeMap<(SmolStr, Symbol, DateRange), CachedCell>,
) -> Option<&'a CachedCell> {
    cache.get(&(strategy_id.clone(), symbol.clone(), range.clone()))
}

// ── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::needless_raw_string_hashes
)]
mod tests {
    use super::*;

    const FLAT_FRONTMATTER: &str = r#"---
scenario: btc-2023-1m-sma-cross
seed: 0xC0FFEE
generated: 2026-04-29T19:51:48Z
wall_clock_s: 1.2
data_source: synthetic
strategy:
  id: btc_sma_cross
  kind: sma_crossover
  source: config/strategies/btc_sma.toml
---
## Summary
"#;

    const MALFORMED_FRONTMATTER: &str = "not yaml at all";

    /// The REAL shape the engine writes (verbatim head of an anchored
    /// `evidence/v1/v0-paper-sma` report): the `strategy:` sub-keys are
    /// UNINDENTED because the writer template's `\`-continuations elide
    /// leading whitespace. The parser must still file them as `strategy.*`
    /// (2026-07-26 review — `scan_one_root` skipped every engine-written
    /// report before this tolerant-reader fix).
    const ENGINE_WRITTEN_FRONTMATTER: &str = r#"---
scenario: btcusdt-2024-h1-sma-cross-binance
seed: 0xC0FFEE
generated: 2026-07-26T10:00:00Z
wall_clock_s: 0.2
data_source: binance
baseline_report: n/a
ledger_imbalance_total: 0
llm_spend_usd: 0.00
strategy:
id: sma_crossover
kind: compiled-in
content_hash: n/a
source: compiled-in
signal: sma_crossover(fast=20, slow=50)
---

# Backtest Report — btcusdt-2024-h1-sma-cross-binance

## Summary
"#;

    #[test]
    fn unindented_strategy_block_parses_as_strategy_keys() {
        let fm = parse_frontmatter(ENGINE_WRITTEN_FRONTMATTER).expect("should parse");
        assert_eq!(
            fm.get("strategy.id").map(SmolStr::as_str),
            Some("sma_crossover"),
            "engine-written (unindented) strategy sub-keys must file as strategy.*"
        );
        assert_eq!(
            fm.get("strategy.kind").map(SmolStr::as_str),
            Some("compiled-in")
        );
        assert_eq!(fm.get("data_source").map(SmolStr::as_str), Some("binance"));
        assert!(
            !fm.contains_key("id"),
            "sub-keys must not leak into the top-level namespace"
        );
    }

    #[test]
    fn parses_flat_kv() {
        let fm = parse_frontmatter(FLAT_FRONTMATTER).expect("should parse");
        assert_eq!(
            fm.get("scenario").map(SmolStr::as_str),
            Some("btc-2023-1m-sma-cross")
        );
        assert_eq!(
            fm.get("generated").map(SmolStr::as_str),
            Some("2026-04-29T19:51:48Z")
        );
        assert_eq!(fm.get("seed").map(SmolStr::as_str), Some("0xC0FFEE"));
    }

    #[test]
    fn parses_strategy_block() {
        let fm = parse_frontmatter(FLAT_FRONTMATTER).expect("should parse");
        assert_eq!(
            fm.get("strategy.id").map(SmolStr::as_str),
            Some("btc_sma_cross")
        );
        assert_eq!(
            fm.get("strategy.kind").map(SmolStr::as_str),
            Some("sma_crossover")
        );
        assert_eq!(
            fm.get("strategy.source").map(SmolStr::as_str),
            Some("config/strategies/btc_sma.toml")
        );
    }

    #[test]
    fn returns_none_on_malformed() {
        let result = parse_frontmatter(MALFORMED_FRONTMATTER);
        assert!(result.is_none(), "malformed content must return None");
    }

    #[test]
    fn scenario_top10_maps_to_universe_of_10() {
        let uni = scenario_universe("top10-2023-1h-momentum").expect("must map");
        assert_eq!(uni.len(), 10, "top10 universe must have 10 symbols");
    }

    #[test]
    fn scenario_btc_maps_to_btc_only() {
        let uni = scenario_universe("btc-2023-1m-sma-cross").expect("must map");
        assert_eq!(uni.len(), 1);
        assert_eq!(uni[0], Symbol::new("BTCUSDT"));
    }

    // ── lab-run-save-compare T5 — two-root union scan ──────────────────────────

    use std::io::Write;

    /// A complete BTC SMA report with a `strategy.id` block + a `## Summary`
    /// KPI table `extract_kpis_from_body` parses. `{sharpe}` is the only
    /// substituted field so two roots can carry distinguishable cells.
    fn btc_report(generated: &str, sharpe: &str) -> String {
        format!(
            r#"---
scenario: btc-2023-1m-sma-cross
seed: 0xC0FFEE
generated: {generated}
wall_clock_s: 0.0
data_source: synthetic
strategy:
  id: btc_sma_cross
  kind: sma_crossover
  source: config/strategies/btc_sma.toml
---

# Backtest Report — btc-2023-1m-sma-cross

## Summary

| Metric        | Value      |
|---------------|------------|
| Sharpe ratio  | **{sharpe}** |
| Total return  | **12.3 %** |
| Max drawdown  | **-5.6 %** |
| Trade count   | **42**     |
"#
        )
    }

    fn write_report(dir: &std::path::Path, fname: &str, content: &str) {
        std::fs::create_dir_all(dir).unwrap();
        let mut f = std::fs::File::create(dir.join(fname)).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    /// T5 / AC5 — `scan_report_roots` over a `lab-runs/` tempdir with TWO
    /// distinct reports (two strategies) builds two `CachedCell`s with KPIs
    /// parsed. This is the minimal "compare two persisted runs" cache build.
    #[test]
    fn scan_report_roots_builds_two_cells_from_lab_runs() {
        let tmp = tempfile::tempdir().unwrap();
        let lab_runs = tmp.path().join("lab-runs");

        // Run A — BTC SMA.
        write_report(
            &lab_runs.join("v0-paper-sma").join("reports"),
            "backtest-20260601-120000-btc-2023-1m-sma-cross.md",
            &btc_report("2026-06-01T12:00:00Z", "0.94"),
        );
        // Run B — a top10 momentum report (multi-symbol universe).
        write_report(
            &lab_runs.join("v1-cross-sectional-momentum").join("reports"),
            "backtest-20260601-130000-top10-2023-1h-momentum.md",
            &format!(
                r#"---
scenario: top10-2023-1h-momentum
seed: 0xC0FFEE
generated: 2026-06-01T13:00:00Z
wall_clock_s: 0.0
data_source: synthetic
strategy:
  id: top10_momentum_h1
  kind: cross_sectional_momentum
  source: config/strategies/top10_momentum_h1.toml
---

## Summary

| Metric        | Value      |
|---------------|------------|
| Sharpe ratio  | **{}** |
| Total return  | **8.1 %**  |
| Max drawdown  | **-12.0 %**|
| Trade count   | **120**    |
"#,
                "1.20"
            ),
        );

        let roots = [lab_runs, std::path::PathBuf::from("/nonexistent/spec")];
        let (cache, _tally) = scan_report_roots(&roots);

        // BTC SMA cell.
        let btc = cache
            .get(&(
                SmolStr::new("btc_sma_cross"),
                Symbol::new("BTCUSDT"),
                DateRange::default(),
            ))
            .expect("BTC SMA cell present");
        assert!((btc.sharpe - 0.94).abs() < 1e-9, "BTC Sharpe parsed");
        assert_eq!(btc.trade_count, 42);

        // top10 momentum cell (multi-symbol — appears for every universe symbol).
        let mom = cache
            .get(&(
                SmolStr::new("top10_momentum_h1"),
                Symbol::new("XRPUSDT"),
                DateRange::default(),
            ))
            .expect("momentum cell present");
        assert!((mom.sharpe - 1.20).abs() < 1e-9, "momentum Sharpe parsed");
        assert!(mom.is_multi_symbol, "top10 is a multi-symbol universe");
    }

    /// T5 / ADR-0055 § D5 #2 — on an IDENTICAL filename across `lab-runs/` and
    /// `evidence/` (here a root named `spec`, the corpus's name before
    /// 2026-07-25), the `lab-runs/` copy wins (it is searched first). Same
    /// filename, different Sharpe in each root → the union must surface the
    /// lab-runs value. bug-log #206: the `evidence/` copy is strictly NEWER, so
    /// only the filename rule keeps the `lab-runs/` copy (an equal `generated:`
    /// would keep it by the first-wins rule alone), and the tally counts the
    /// skipped copy.
    #[test]
    fn scan_report_roots_identical_filename_lab_runs_wins() {
        let tmp = tempfile::tempdir().unwrap();
        let fname = "backtest-20260101-000000-btc-2023-1m-sma-cross.md";

        let lab_runs = tmp.path().join("lab-runs");
        write_report(
            &lab_runs.join("v0-paper-sma").join("reports"),
            fname,
            &btc_report("2026-01-01T00:00:00Z", "0.99"), // lab-runs Sharpe
        );

        let spec = tmp.path().join("spec");
        write_report(
            &spec.join("v0-paper-sma").join("reports"),
            fname,
            &btc_report("2026-01-02T00:00:00Z", "0.11"), // spec Sharpe (same name, newer)
        );

        // Production order: lab-runs FIRST.
        let roots = [lab_runs, spec];
        let (cache, got) = scan_report_roots(&roots);
        assert_eq!(
            got,
            tally(2, 1, &[(Rejected::DuplicateFilename, 1)]),
            "identical filename: the later root's copy is discovered and skipped as a \
             duplicate (#206)"
        );
        let btc = cache
            .get(&(
                SmolStr::new("btc_sma_cross"),
                Symbol::new("BTCUSDT"),
                DateRange::default(),
            ))
            .expect("BTC cell present");
        assert!(
            (btc.sharpe - 0.99).abs() < 1e-9,
            "identical filename: lab-runs copy (0.99) must win, not spec (0.11); got {}",
            btc.sharpe
        );
    }

    // ── lab-compare-equity-overlay T1 — CachedCell timestamped series ──────────

    /// Write a companion equity CSV (`<stem>-equity.csv`) beside an `.md` report
    /// via the PRODUCTION `reports::csv_artifacts::write_equity_csv` schema, so
    /// the scanner reads the exact format the Lab persistence path writes.
    fn write_companion_csv(dir: &std::path::Path, md_fname: &str, points: &[(i64, f64)]) {
        use reports::csv_artifacts::{EquitySample, write_equity_csv};
        use rust_decimal::Decimal;
        use std::str::FromStr;
        use trading_core::Timestamp;

        let stem = md_fname.strip_suffix(".md").unwrap();
        let csv_path = dir.join(format!("{stem}-equity.csv"));
        let samples: Vec<EquitySample> = points
            .iter()
            .map(|(ts_ms, eq)| EquitySample {
                ts: Timestamp::new(
                    time::OffsetDateTime::UNIX_EPOCH + time::Duration::milliseconds(*ts_ms),
                ),
                equity_total: Decimal::from_str(&format!("{eq:.2}")).unwrap(),
                realized_pnl: Decimal::ZERO,
                unrealized_pnl: Decimal::ZERO,
                cash_balance: Decimal::from_str(&format!("{eq:.2}")).unwrap(),
            })
            .collect();
        write_equity_csv(&csv_path, &samples).unwrap();
    }

    /// T1 / R1 — a report WITH a companion equity CSV hydrates the cell's
    /// `equity_series_ts` with the full timestamped per-bar series (`PerBar`
    /// fidelity), preserving timestamps + Decimal money. This is the series that
    /// feeds the two-run overlay (`equity_curve_tail` alone has no x-axis).
    #[test]
    fn cell_hydrates_timestamped_series_from_companion_csv() {
        let tmp = tempfile::tempdir().unwrap();
        let lab_runs = tmp.path().join("lab-runs");
        let reports_dir = lab_runs.join("v0-paper-sma").join("reports");
        let md_fname = "backtest-20260601-120000-btc-2023-1m-sma-cross.md";

        write_report(
            &reports_dir,
            md_fname,
            &btc_report("2026-06-01T12:00:00Z", "0.94"),
        );
        // Companion CSV with a 4-point timestamped series.
        write_companion_csv(
            &reports_dir,
            md_fname,
            &[
                (1_700_000_000_000, 100_000.0),
                (1_700_000_060_000, 100_800.0),
                (1_700_000_120_000, 99_500.0),
                (1_700_000_180_000, 103_100.0),
            ],
        );

        let roots = [lab_runs];
        let (cache, _tally) = scan_report_roots(&roots);
        let btc = cache
            .get(&(
                SmolStr::new("btc_sma_cross"),
                Symbol::new("BTCUSDT"),
                DateRange::default(),
            ))
            .expect("BTC cell present");

        assert_eq!(
            btc.equity_series_ts.len(),
            4,
            "companion CSV must hydrate the full per-bar timestamped series"
        );
        // Timestamps preserved, oldest-first.
        assert_eq!(btc.equity_series_ts[0].0, 1_700_000_000_000);
        assert_eq!(btc.equity_series_ts[3].0, 1_700_000_180_000);
        // Money is Decimal, exact.
        assert_eq!(
            btc.equity_series_ts[0].1,
            "100000.00".parse::<rust_decimal::Decimal>().unwrap()
        );
        assert_eq!(
            btc.equity_series_ts[3].1,
            "103100.00".parse::<rust_decimal::Decimal>().unwrap()
        );
    }

    /// T1 / R1 graceful fallback — a report with NO companion CSV (an older
    /// committed `spec/` report) yields an EMPTY `equity_series_ts`. The cell
    /// still populates its KPIs; the overlay simply has nothing to draw for it
    /// (no fake curve, no panic).
    #[test]
    fn cell_without_companion_csv_has_empty_timestamped_series() {
        let tmp = tempfile::tempdir().unwrap();
        let spec = tmp.path().join("spec");
        write_report(
            &spec.join("v0-paper-sma").join("reports"),
            "backtest-20260101-000000-btc-2023-1m-sma-cross.md",
            &btc_report("2026-01-01T00:00:00Z", "0.55"),
        );
        let cache = scan_spec_tree(&spec);
        let btc = cache
            .get(&(
                SmolStr::new("btc_sma_cross"),
                Symbol::new("BTCUSDT"),
                DateRange::default(),
            ))
            .expect("BTC cell present");
        assert!(
            (btc.sharpe - 0.55).abs() < 1e-9,
            "KPIs still populate without a companion (sharpe={:.4})",
            btc.sharpe
        );
        assert!(
            btc.equity_series_ts.is_empty(),
            "no companion CSV → empty timestamped series (graceful fallback)"
        );
    }

    /// T5 — `scan_spec_tree` (single-root wrapper) still works after the union
    /// refactor; a spec-rooted scan resolves a committed report unchanged.
    #[test]
    fn scan_spec_tree_single_root_still_works() {
        let tmp = tempfile::tempdir().unwrap();
        let spec = tmp.path().join("spec");
        write_report(
            &spec.join("v0-paper-sma").join("reports"),
            "backtest-20260101-000000-btc-2023-1m-sma-cross.md",
            &btc_report("2026-01-01T00:00:00Z", "0.55"),
        );
        let cache = scan_spec_tree(&spec);
        let btc = cache
            .get(&(
                SmolStr::new("btc_sma_cross"),
                Symbol::new("BTCUSDT"),
                DateRange::default(),
            ))
            .expect("single-root spec scan resolves the report");
        assert!((btc.sharpe - 0.55).abs() < 1e-9);
    }

    // ── bug-log #206 — the scan walks the ONE report-root layout (ADR-0055 D8) ──

    use std::path::PathBuf;

    use crate::reports::layout::test_tree::{self, TREE};
    use crate::session_log::{Rejected, ScanTally};
    use crate::test_support::evidence_oracle;

    /// The slug of every #206 fixture dir.
    const SLUG_206: &str = "v0-paper-sma";
    /// Container names that sort BEFORE (`a0`) and AFTER (`zz`) [`SLUG_206`],
    /// so each order test below holds whichever way the full paths sort.
    const CONTAINERS_206: [&str; 2] = ["a0", "zz"];
    /// The one `generated:` stamp of the #206 fixtures (ties are the point).
    const GENERATED_206: &str = "2026-01-01T00:00:00Z";
    /// The report name of the #206 fixtures.
    const FNAME_206: &str = "backtest-20260101-000000-btc-2023-1m-sma-cross.md";
    /// The same name one second later: it sorts AFTER [`FNAME_206`].
    const FNAME_206_LATER: &str = "backtest-20260101-000001-btc-2023-1m-sma-cross.md";
    /// The Sharpe of the report the D8 order must keep, and of the one it must not.
    const SHARPE_KEPT: &str = "0.11";
    const SHARPE_SHADOWED: &str = "0.99";

    /// The cell key `btc_report` scans into: its `strategy.id`, the one symbol
    /// of its `btc-*` scenario and the default range.
    fn btc_key() -> (SmolStr, Symbol, DateRange) {
        (
            SmolStr::new("btc_sma_cross"),
            Symbol::new("BTCUSDT"),
            DateRange::default(),
        )
    }

    /// The Sharpe a fixture wrote, parsed from the string the fixture used.
    fn sharpe_of(fixture: &str) -> f64 {
        fixture.parse().unwrap()
    }

    /// A tally with `discovered` candidates, `admitted` of them admitted and the
    /// rest in `rejected` (written out per test from its fixture).
    fn tally(discovered: usize, admitted: usize, rejected: &[(Rejected, usize)]) -> ScanTally {
        ScanTally {
            discovered,
            admitted,
            rejected: rejected.iter().copied().collect(),
        }
    }

    /// CS1 — a report at container depth 1 (`<root>/<container>/<slug>/reports/`,
    /// the committed layout) is discovered and admitted, and its
    /// `source_report_path` keeps the container. RED before the fix: the scan
    /// walked `<root>/<x>/reports/` only.
    #[test]
    fn scan_admits_a_container_depth_1_report() {
        let tmp = tempfile::tempdir().unwrap();
        let evidence = tmp.path().join("evidence");
        let sharpe = "0.77";
        write_report(
            &evidence.join("v1").join(SLUG_206).join("reports"),
            FNAME_206,
            &btc_report(GENERATED_206, sharpe),
        );
        let (cache, got) = scan_report_roots(&[evidence]);
        assert_eq!(
            got,
            tally(1, 1, &[]),
            "one report at container depth 1 must be discovered and admitted (#206)"
        );
        let cell = cache
            .get(&btc_key())
            .expect("the container-depth-1 report must scan into its cell (#206)");
        assert!(
            (cell.sharpe - sharpe_of(sharpe)).abs() < 1e-9,
            "the cell must carry the fixture's Sharpe {sharpe}; got {}",
            cell.sharpe
        );
        let want = Path::new("evidence")
            .join("v1")
            .join(SLUG_206)
            .join("reports")
            .join(FNAME_206);
        assert_eq!(
            Path::new(cell.source_report_path.as_str()),
            want.as_path(),
            "source_report_path is relative to the root's parent and keeps the container"
        );
    }

    /// CS2 — CS1's pair: the same bytes one container deeper (container depth
    /// 2, out of contract) are not read. The file exists, so the negative is
    /// not "file missing".
    #[test]
    fn scan_skips_a_container_depth_2_report() {
        let tmp = tempfile::tempdir().unwrap();
        let evidence = tmp.path().join("evidence");
        let dir = evidence.join("v9").join("x").join(SLUG_206).join("reports");
        write_report(&dir, FNAME_206, &btc_report(GENERATED_206, "0.77"));
        assert!(
            dir.join(FNAME_206).is_file(),
            "the container-depth-2 fixture must exist"
        );
        let (cache, got) = scan_report_roots(&[evidence]);
        assert_eq!(
            got,
            ScanTally::default(),
            "a container-depth-2 report must not be considered (ADR-0055 D8)"
        );
        assert!(
            cache.is_empty(),
            "a container-depth-2 report must yield no cell"
        );
    }

    /// CS3 — over the shared layout TREE (bug-log #205) the scan considers
    /// exactly the rows the TREE lists, and nothing it does not. Expected,
    /// derived from the TREE: discovered = the listed rows; no row carries
    /// frontmatter, so the first copy of each distinct name is
    /// `MalformedFrontmatter` and every later copy is `DuplicateFilename`;
    /// admitted = 0. A TREE directory named `backtest-dir.md` is not a
    /// candidate (the layout lists files only).
    #[test]
    fn scan_tally_over_the_layout_tree_counts_exactly_its_listed_rows() {
        let tmp = tempfile::tempdir().unwrap();
        test_tree::build(tmp.path());
        let listed: Vec<&str> = TREE
            .iter()
            .filter(|r| r.listed.is_some())
            .map(|r| r.path)
            .collect();
        // Container depth from the row's own path: `<slug>/reports/<file>` = 0;
        // `None` for a row less than three components deep.
        let depth = |p: &str| p.split('/').count().checked_sub(3);
        assert!(
            listed.iter().any(|p| depth(p) == Some(0)),
            "the TREE must list a container-depth-0 row"
        );
        assert!(
            listed.iter().any(|p| depth(p) == Some(1)),
            "the TREE must list a container-depth-1 row"
        );
        let unlisted_reports: Vec<&str> = TREE
            .iter()
            .filter(|r| {
                r.listed.is_none()
                    && !r.is_dir
                    && r.path
                        .rsplit('/')
                        .next()
                        .is_some_and(evidence_oracle::is_backtest_name)
            })
            .map(|r| r.path)
            .collect();
        assert!(
            unlisted_reports.iter().any(|p| p.starts_with("reports/")),
            "the TREE must hold a backtest-*.md under a `reports` container"
        );
        assert!(
            unlisted_reports
                .iter()
                .any(|p| depth(p) == Some(2) && p.split('/').nth(3) == Some("reports")),
            "the TREE must hold a backtest-*.md at container depth 2"
        );
        for p in &listed {
            let content = std::fs::read_to_string(tmp.path().join(p)).unwrap();
            assert_ne!(
                content.lines().next(),
                Some("---"),
                "a TREE row must carry no frontmatter: {p}"
            );
        }
        let distinct: BTreeSet<&str> = listed.iter().filter_map(|p| p.rsplit('/').next()).collect();
        assert!(
            listed.len() > distinct.len(),
            "the TREE must list one name twice, else the duplicate bucket pins nothing"
        );
        let want = tally(
            listed.len(),
            0,
            &[
                (Rejected::MalformedFrontmatter, distinct.len()),
                (Rejected::DuplicateFilename, listed.len() - distinct.len()),
            ],
        );
        let (cache, got) = scan_report_roots(&[tmp.path().to_path_buf()]);
        assert_eq!(
            got, want,
            "the scan must consider exactly the TREE's listed rows (#206)"
        );
        assert!(cache.is_empty(), "no TREE row is a readable report");
    }

    /// The report dirs of the `CS4a`/`CS4b` fixtures under `root`: container depth
    /// 0, then container depth 1 under `container`.
    fn depth_0_and_1_dirs(root: &Path, container: &str) -> (PathBuf, PathBuf) {
        (
            root.join(SLUG_206).join("reports"),
            root.join(container).join(SLUG_206).join("reports"),
        )
    }

    /// The two-sided precondition of `CS4a`/`CS4b`: the container-depth-1 dir sorts
    /// before the container-depth-0 dir by path under `a0` and after it under
    /// `zz`, so neither full-path order nor its reverse can pass both halves.
    fn assert_containers_straddle_the_slug(root: &Path) {
        let before: Vec<bool> = CONTAINERS_206
            .iter()
            .map(|c| {
                let (d0, d1) = depth_0_and_1_dirs(root, c);
                d1 < d0
            })
            .collect();
        assert_eq!(
            before,
            [true, false],
            "`a0` must sort before the slug and `zz` after it"
        );
    }

    /// `CS4a` — inside one root, an identical filename at container depth 0 and
    /// at container depth 1: the container-depth-0 copy is admitted and the
    /// other is a `DuplicateFilename` (ADR-0055 D8 order), whether the container
    /// sorts before or after the slug.
    #[test]
    fn duplicate_filename_inside_a_root_goes_to_container_depth_0() {
        for container in CONTAINERS_206 {
            let tmp = tempfile::tempdir().unwrap();
            let root = tmp.path().join("evidence");
            assert_containers_straddle_the_slug(&root);
            let (d0, d1) = depth_0_and_1_dirs(&root, container);
            write_report(&d0, FNAME_206, &btc_report(GENERATED_206, SHARPE_KEPT));
            write_report(&d1, FNAME_206, &btc_report(GENERATED_206, SHARPE_SHADOWED));
            let (cache, got) = scan_report_roots(&[root]);
            assert_eq!(
                got,
                tally(2, 1, &[(Rejected::DuplicateFilename, 1)]),
                "container {container}: two copies of one name, the second a duplicate (#206)"
            );
            let cell = cache.get(&btc_key()).expect("the BTC cell must be present");
            assert!(
                (cell.sharpe - sharpe_of(SHARPE_KEPT)).abs() < 1e-9,
                "container {container}: the container-depth-0 copy ({SHARPE_KEPT}) must win, not \
                 the container-depth-1 copy ({SHARPE_SHADOWED}); got {}",
                cell.sharpe
            );
        }
    }

    /// `CS4b` — inside one root, two names with an equal `generated:` for one
    /// tuple, the container-depth-1 one sorting FIRST by name: the
    /// container-depth-0 report is kept (D8 order, then `<=` keeps the first),
    /// whether the container sorts before or after the slug.
    #[test]
    fn equal_generated_inside_a_root_goes_to_container_depth_0() {
        assert!(
            FNAME_206 < FNAME_206_LATER,
            "the depth-1 name must sort first"
        );
        for container in CONTAINERS_206 {
            let tmp = tempfile::tempdir().unwrap();
            let root = tmp.path().join("evidence");
            assert_containers_straddle_the_slug(&root);
            let (d0, d1) = depth_0_and_1_dirs(&root, container);
            write_report(
                &d0,
                FNAME_206_LATER,
                &btc_report(GENERATED_206, SHARPE_KEPT),
            );
            write_report(&d1, FNAME_206, &btc_report(GENERATED_206, SHARPE_SHADOWED));
            let (cache, got) = scan_report_roots(&[root]);
            assert_eq!(
                got,
                tally(2, 2, &[]),
                "container {container}: two distinct names, both admitted (#206)"
            );
            let cell = cache.get(&btc_key()).expect("the BTC cell must be present");
            assert!(
                (cell.sharpe - sharpe_of(SHARPE_KEPT)).abs() < 1e-9,
                "container {container}: on an equal `generated:` the container-depth-0 report \
                 ({SHARPE_KEPT}) must be kept, not the container-depth-1 one \
                 ({SHARPE_SHADOWED}); got {}",
                cell.sharpe
            );
        }
    }

    /// `CS4c` — inside one dir, two names with an equal `generated:` for one
    /// tuple: the first by path is kept. The later name is written first, so
    /// creation order cannot stand in for path order.
    #[test]
    fn equal_generated_inside_one_dir_goes_to_the_first_by_path() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("evidence");
        let dir = root.join(SLUG_206).join("reports");
        assert!(
            dir.join(FNAME_206) < dir.join(FNAME_206_LATER),
            "the kept name must sort first by path"
        );
        write_report(
            &dir,
            FNAME_206_LATER,
            &btc_report(GENERATED_206, SHARPE_SHADOWED),
        );
        write_report(&dir, FNAME_206, &btc_report(GENERATED_206, SHARPE_KEPT));
        let (cache, got) = scan_report_roots(&[root]);
        assert_eq!(got, tally(2, 2, &[]), "two distinct names, both admitted");
        let cell = cache.get(&btc_key()).expect("the BTC cell must be present");
        assert!(
            (cell.sharpe - sharpe_of(SHARPE_KEPT)).abs() < 1e-9,
            "on an equal `generated:` inside one dir the first by path ({SHARPE_KEPT}) must be \
             kept, not {SHARPE_SHADOWED}; got {}",
            cell.sharpe
        );
    }

    /// The workspace root (`crates/ui` → `crates` → root).
    fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
    }

    /// CS5 — LIVE: the scan of the real `evidence/` root considers at least as
    /// many reports as git lists in contract (`evidence_oracle`, tracked or
    /// untracked on disk), at container depth 0 AND 1. Counts only, in the
    /// subset form (#189); a tracked report deleted from disk stays in the
    /// listing and turns it red (#66). Before the fix it considered only the
    /// container-depth-0 reports. Prints the tally (`--nocapture`).
    #[test]
    fn compare_scan_discovers_every_git_listed_report() {
        let ws = workspace_root();
        let listed: Vec<Vec<String>> =
            evidence_oracle::git_listed_files(&mut evidence_oracle::live_git(&ws))
                .unwrap_or_else(|e| panic!("the #206 evidence oracle must run: {e}"))
                .into_iter()
                .filter(|c| evidence_oracle::is_contract_report(c))
                .collect();
        let d0 = listed.iter().filter(|c| c.len() == 4).count();
        let d1 = listed.iter().filter(|c| c.len() == 5).count();
        assert!(d0 >= 1, "the oracle must hold a container-depth-0 report");
        assert!(d1 >= 1, "the oracle must hold a container-depth-1 report");
        let evidence = ws.join("evidence");
        assert!(evidence.is_dir(), "the workspace must hold evidence/");
        let (_cache, got) = scan_report_roots(&[evidence]);
        eprintln!(
            "#206 compare scan over evidence/: discovered {}, admitted {}, rejected {:?}; git \
             lists {} in-contract reports (container depth 0: {d0}, container depth 1: {d1})",
            got.discovered,
            got.admitted,
            got.rejected,
            listed.len()
        );
        assert!(
            got.discovered >= listed.len(),
            "the Compare scan considered {} of {} git-listed in-contract reports (container \
             depth 0: {d0}, container depth 1: {d1}) (#206)",
            got.discovered,
            listed.len()
        );
    }

    /// CS6 — the scan takes `layout::backtest_files`'s name rule: a report
    /// named `….MD` is no candidate. Two-sided: the same bytes named `….md`
    /// in another root are discovered and admitted, so only the extension's
    /// case differs. Before the fix the scan matched `.md` ignoring case.
    #[test]
    fn scan_skips_an_uppercase_md_report() {
        let upper_name = FNAME_206.replace(".md", ".MD");
        let content = btc_report(GENERATED_206, "0.77");
        let tmp = tempfile::tempdir().unwrap();
        let (upper, lower) = (tmp.path().join("upper"), tmp.path().join("lower"));
        let upper_dir = upper.join(SLUG_206).join("reports");
        write_report(&upper_dir, &upper_name, &content);
        write_report(&lower.join(SLUG_206).join("reports"), FNAME_206, &content);
        assert!(
            upper_dir.join(&upper_name).is_file(),
            "the `.MD` fixture must exist"
        );
        let (cache, got) = scan_report_roots(&[upper]);
        assert_eq!(
            got,
            ScanTally::default(),
            "a `.MD` report must not be considered (layout::backtest_files, #206)"
        );
        assert!(cache.is_empty(), "a `.MD` report must yield no cell");
        let (cache, got) = scan_report_roots(&[lower]);
        assert_eq!(
            got,
            tally(1, 1, &[]),
            "control: the same bytes named `.md` are discovered and admitted"
        );
        assert!(cache.contains_key(&btc_key()), "control: the `.md` cell");
    }
}
