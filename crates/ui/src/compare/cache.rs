//! Phase E — Report-cache scanner for the Compare matrix (R3.1-R3.5).
//!
//! Exposes:
//! - `scan_spec_tree(spec_root: &Path) -> BTreeMap<(SmolStr, Symbol, DateRange), CachedCell>`
//!   — cold-boot glob + frontmatter parse.
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
fn parse_frontmatter(content: &str) -> Option<BTreeMap<SmolStr, SmolStr>> {
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

        // Lines inside the strategy block start with 2 spaces.
        if in_strategy_block {
            if let Some(inner) = line.strip_prefix("  ") {
                if let Some(sep) = inner.find(": ") {
                    let k = SmolStr::new(format!("strategy.{}", &inner[..sep]));
                    let v = SmolStr::new(inner[sep + 2..].trim());
                    map.insert(k, v);
                }
            } else {
                // Non-indented line → we've exited the strategy block.
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

/// Scan a SINGLE report root for all backtest report `.md` files and build a
/// `BTreeMap<(strategy_id, symbol, range), CachedCell>` cache.
///
/// Only the most-recent report per `(strategy_id, symbol, range)` tuple
/// is kept (R3.3: most-recent wins; older reports reachable from Trail).
///
/// `spec_root` should be the absolute path to one report root (the repo's
/// `spec/` directory, or a `lab-runs/` tree). On parse failure the file is
/// skipped with a `tracing::warn!` (K2 fail-soft).
///
/// For the production two-root union (`lab-runs/` + `spec/`) use
/// [`scan_report_roots`] (lab-run-save-compare T5 / Q4).
#[must_use]
pub fn scan_spec_tree(spec_root: &Path) -> BTreeMap<(SmolStr, Symbol, DateRange), CachedCell> {
    let roots = [spec_root.to_path_buf()];
    scan_report_roots(&roots)
}

/// Scan a FIXED-ORDER union of report roots and build the Compare cache
/// (lab-run-save-compare T5 / Q4 / ADR-0055 § D5).
///
/// Production passes `[default_lab_runs_root(), default_spec_root()]` —
/// **lab-runs FIRST, then spec/**. Precedence rules (pinned, ADR-0055 § D5):
///
/// 1. Search order is the slice order: `lab-runs/` first.
/// 2. On an **identical filename across roots** (same `backtest-<stamp>-<scenario>.md`
///    in both `lab-runs/` and `spec/`), the FIRST root's copy wins — the later
///    root's same-named file is skipped before it is even parsed.
/// 3. Within the union, the existing **most-recent-`generated:`-wins**
///    per-tuple tiebreaker decides which report represents a tuple in Compare.
#[must_use]
pub fn scan_report_roots(
    roots: &[std::path::PathBuf],
) -> BTreeMap<(SmolStr, Symbol, DateRange), CachedCell> {
    let mut cache: BTreeMap<(SmolStr, Symbol, DateRange), CachedCell> = BTreeMap::new();
    // Filenames already claimed by a higher-priority (earlier) root. The
    // collision rule (ADR-0055 § D5 #2): identical filename ⇒ lab-runs wins,
    // so a later root's same-named file is skipped.
    let mut seen_filenames: BTreeSet<SmolStr> = BTreeSet::new();
    for root in roots {
        scan_one_root(root, &mut cache, &mut seen_filenames);
    }
    cache
}

/// Scan one report root into `cache`, honoring the cross-root filename
/// collision rule via `seen_filenames` (earlier roots win).
fn scan_one_root(
    spec_root: &Path,
    cache: &mut BTreeMap<(SmolStr, Symbol, DateRange), CachedCell>,
    seen_filenames: &mut BTreeSet<SmolStr>,
) {
    use std::fs;

    // Walk spec_root/**/ looking for backtest-*.md files.
    let Ok(outer) = fs::read_dir(spec_root) else {
        // A missing root is normal (a fresh checkout has no `lab-runs/`);
        // fail soft. `trace!` (not `warn!`) so the absent-lab-runs case is
        // not noisy on every cold boot.
        tracing::trace!(
            "compare::cache: report root not found or not a directory: {}",
            spec_root.display()
        );
        return;
    };

    for entry in outer.flatten() {
        let strategy_dir = entry.path();
        if !strategy_dir.is_dir() {
            continue;
        }
        let reports_dir = strategy_dir.join("reports");
        if !reports_dir.is_dir() {
            continue;
        }

        let Ok(reports) = fs::read_dir(&reports_dir) else {
            continue;
        };

        for report_entry in reports.flatten() {
            let report_path = report_entry.path();
            let Some(fname) = report_path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !fname.starts_with("backtest-")
                || !std::path::Path::new(fname)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
            {
                continue;
            }

            // Collision rule: if an earlier (higher-priority) root already
            // claimed this exact filename, skip it (lab-runs wins).
            let fname_key = SmolStr::new(fname);
            if !seen_filenames.insert(fname_key) {
                continue;
            }

            let Ok(content) = fs::read_to_string(&report_path) else {
                tracing::warn!("compare::cache: failed to read {}", report_path.display());
                continue;
            };

            let Some(fm) = parse_frontmatter(&content) else {
                tracing::warn!(
                    "compare::cache: malformed frontmatter in {}",
                    report_path.display()
                );
                continue;
            };

            let Some(scenario) = fm.get("scenario") else {
                continue;
            };

            let Some(strategy_id_raw) = fm.get("strategy.id") else {
                continue;
            };

            let strategy_id = SmolStr::new(strategy_id_raw.as_str());

            let Some(universe) = scenario_universe(scenario) else {
                // Unknown scenario prefix — skip silently.
                continue;
            };

            let is_multi = scenario_is_multi_symbol(scenario);

            // Use the repo-relative path as the cell identifier.
            // Build a repo-relative path by stripping the root's parent prefix.
            let source_path = report_path
                .strip_prefix(spec_root.parent().unwrap_or(spec_root))
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
                continue;
            };

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

                // R3.3: keep only the most-recent report per tuple.
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
        let cache = scan_report_roots(&roots);

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
    /// `spec/`, the `lab-runs/` copy wins (it is searched first). Same filename,
    /// different Sharpe in each root → the union must surface the lab-runs value.
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
            &btc_report("2026-01-01T00:00:00Z", "0.11"), // spec Sharpe (same name)
        );

        // Production order: lab-runs FIRST.
        let roots = [lab_runs, spec];
        let cache = scan_report_roots(&roots);
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
        let cache = scan_report_roots(&roots);
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
}
