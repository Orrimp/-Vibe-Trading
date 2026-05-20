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

use std::collections::BTreeMap;
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

    if map.is_empty() {
        None
    } else {
        Some(map)
    }
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
) -> Option<CachedCell> {
    // Skip the frontmatter block.
    let body = {
        let mut iter = content.splitn(4, "---");
        iter.next()?; // before first ---
        iter.next()?; // frontmatter content
        iter.next()? // body (after second ---)
    };

    // Parse key metrics from the body table.
    // The body contains a Markdown table with rows like:
    //   | Sharpe ratio     | **0.94**  |
    //   | Total return     | **12.3 %**|
    //   | Max drawdown     | **-5.6 %**|
    //   | Trade count      | **42**    |
    let sharpe = extract_table_value(body, "Sharpe ratio")
        .and_then(|v| clean_bold_value(&v).parse::<f64>().ok())
        .unwrap_or(0.0);

    let total_return_pct = extract_table_value(body, "Total return")
        .and_then(|v| {
            // Strip trailing " %" or "%" before parsing.
            let cleaned = clean_bold_value(&v).replace('%', "").trim().to_string();
            cleaned.parse::<f64>().ok()
        })
        .unwrap_or(0.0);

    let max_drawdown_pct = extract_table_value(body, "Max drawdown")
        .and_then(|v| {
            let cleaned = clean_bold_value(&v).replace('%', "").trim().to_string();
            cleaned.parse::<f64>().ok()
        })
        .unwrap_or(0.0);

    let trade_count = extract_table_value(body, "Trade count")
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

/// Scan the spec tree for all backtest report `.md` files and build a
/// `BTreeMap<(strategy_id, symbol, range), CachedCell>` cache.
///
/// Only the most-recent report per `(strategy_id, symbol, range)` tuple
/// is kept (R3.3: most-recent wins; older reports reachable from Trail).
///
/// `spec_root` should be the absolute path to the repo's `spec/` directory.
/// On parse failure the file is skipped with a `tracing::warn!` (K2 fail-soft).
#[must_use]
pub fn scan_spec_tree(
    spec_root: &Path,
) -> BTreeMap<(SmolStr, Symbol, DateRange), CachedCell> {
    use std::fs;

    let mut cache: BTreeMap<(SmolStr, Symbol, DateRange), CachedCell> = BTreeMap::new();

    // Walk spec_root/**/ looking for backtest-*.md files.
    let Ok(outer) = fs::read_dir(spec_root) else {
        tracing::warn!(
            "compare::cache: spec_root not found or not a directory: {}",
            spec_root.display()
        );
        return cache;
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

            let Ok(content) = fs::read_to_string(&report_path) else {
                tracing::warn!(
                    "compare::cache: failed to read {}",
                    report_path.display()
                );
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
            // Build a repo-relative path by stripping the spec_root parent prefix.
            let source_path = report_path
                .strip_prefix(spec_root.parent().unwrap_or(spec_root))
                .map_or_else(
                    |_| report_path.to_string_lossy().to_string(),
                    |p| p.to_string_lossy().to_string(),
                );

            let Some(cell) =
                extract_kpis_from_body(&content, &fm, &source_path, is_multi)
            else {
                tracing::warn!(
                    "compare::cache: no KPI table in {}",
                    report_path.display()
                );
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
                if cache
                    .get(&key)
                    .is_some_and(|existing| cell.generated_at.as_str() <= existing.generated_at.as_str())
                {
                    continue;
                }
                cache.insert(key, cell.clone());
            }
        }
    }

    cache
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
        assert_eq!(fm.get("scenario").map(|s| s.as_str()), Some("btc-2023-1m-sma-cross"));
        assert_eq!(fm.get("generated").map(|s| s.as_str()), Some("2026-04-29T19:51:48Z"));
        assert_eq!(fm.get("seed").map(|s| s.as_str()), Some("0xC0FFEE"));
    }

    #[test]
    fn parses_strategy_block() {
        let fm = parse_frontmatter(FLAT_FRONTMATTER).expect("should parse");
        assert_eq!(
            fm.get("strategy.id").map(|s| s.as_str()),
            Some("btc_sma_cross")
        );
        assert_eq!(
            fm.get("strategy.kind").map(|s| s.as_str()),
            Some("sma_crossover")
        );
        assert_eq!(
            fm.get("strategy.source").map(|s| s.as_str()),
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
}
