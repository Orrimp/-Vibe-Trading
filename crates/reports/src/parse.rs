//! Markdown summary-table parser — Phase 4 (R3 / Q3a).
//!
//! Reads the existing `## Summary` table out of any backtest report
//! body. No new artefact, no write-path change. The 11 anchored
//! reports' bodies are read byte-identically; the parser never edits.
//!
//! **Failure mode is graceful** — missing fields tolerated per R3.5
//! (`cagr_pct` and `win_rate_pct` are absent from the live samples;
//! the parser flips their `_present` flags to `false` and the KPI
//! strip renders `—` dashes for those cards). Returns
//! `Err(ParseError)` only on truly malformed bytes (no `## Summary`
//! heading found).

use std::fs;
use std::path::Path;
use std::str::FromStr;

use rust_decimal::Decimal;
use thiserror::Error;
use trading_core::BacktestMetrics;

/// Parser errors — only returned for truly malformed input. Missing
/// fields are tolerated and surface via the metric struct's
/// `*_present` flags.
#[derive(Debug, Error)]
pub enum ParseError {
    /// File could not be read.
    #[error("read report: {0}")]
    Io(#[from] std::io::Error),
    /// No `## Summary` heading found in the body. Indicates a future
    /// report-format change or a non-backtest report path.
    #[error("no `## Summary` heading found in report body")]
    NoSummaryHeading,
}

/// Parse the metric struct out of a backtest report on disk.
///
/// # Errors
///
/// - [`ParseError::Io`] when the file cannot be read.
/// - [`ParseError::NoSummaryHeading`] when the body lacks a
///   `## Summary` heading.
pub fn parse_from_report(path: &Path) -> Result<BacktestMetrics, ParseError> {
    let raw = fs::read_to_string(path)?;
    parse_from_str(&raw)
}

/// Inner parser — split out from [`parse_from_report`] so unit tests
/// can feed in fixture strings without touching the filesystem.
fn parse_from_str(raw: &str) -> Result<BacktestMetrics, ParseError> {
    let body = strip_front_matter(raw);

    // Locate the `## Summary` heading. Walk lines until we find it,
    // then iterate until the next `##` heading.
    let mut found_summary = false;
    let mut metrics = BacktestMetrics::all_absent();

    for line in body.lines() {
        let trimmed = line.trim_start();
        if !found_summary {
            if trimmed.starts_with("## Summary") {
                found_summary = true;
            }
            continue;
        }
        // Stop at the next `## ` heading.
        if trimmed.starts_with("## ") || trimmed.starts_with("##\t") {
            break;
        }
        // Match `| Metric | Value |` rows. Skip the header / divider
        // rows ("Metric", "----") + non-pipe rows.
        let Some((label, value)) = split_table_row(line) else {
            continue;
        };
        match label {
            "Total return" => {
                if let Some(d) = parse_pct(value) {
                    metrics.total_return_pct = d;
                }
            }
            "CAGR" => {
                if let Some(d) = parse_pct(value) {
                    metrics.cagr_pct = d;
                    metrics.cagr_present = true;
                }
            }
            "Sharpe ratio (ann.)" | "Sharpe" => {
                if let Some(d) = parse_decimal_loose(value) {
                    metrics.sharpe = d;
                    metrics.sharpe_present = true;
                }
            }
            "Max drawdown" => {
                if let Some(d) = parse_pct(value) {
                    metrics.max_drawdown_pct = d.abs();
                }
            }
            "Win rate" => {
                if let Some(d) = parse_pct(value) {
                    metrics.win_rate_pct = d;
                    metrics.win_rate_present = true;
                }
            }
            "Trades" => {
                if let Some(n) = parse_count(value) {
                    metrics.trades = n;
                }
            }
            _ => {}
        }
    }

    if !found_summary {
        return Err(ParseError::NoSummaryHeading);
    }

    Ok(metrics)
}

/// Strip the YAML front-matter (between the leading two `---` lines)
/// from a report body, returning the body slice. If the file has no
/// front-matter, the whole input is returned.
fn strip_front_matter(raw: &str) -> &str {
    let trimmed = raw.trim_start_matches('\u{feff}');
    if !trimmed.starts_with("---\n") && !trimmed.starts_with("---\r\n") {
        return trimmed;
    }
    let after_first = match trimmed.find('\n') {
        Some(n) => &trimmed[n + 1..],
        None => return trimmed,
    };
    if let Some(idx) = find_closing_fence(after_first) {
        // Skip past the closing `---\n`.
        let rest = &after_first[idx..];
        if let Some(nl) = rest.find('\n') {
            return &rest[nl + 1..];
        }
    }
    trimmed
}

/// Find the byte offset of the next `---` line (a `\n---\n` or
/// `\n---\r\n` boundary, or `---` at start).
fn find_closing_fence(s: &str) -> Option<usize> {
    if s.starts_with("---\n") || s.starts_with("---\r\n") {
        return Some(0);
    }
    let mut search_from = 0usize;
    while let Some(rel) = s[search_from..].find("\n---") {
        let abs = search_from + rel;
        let after = &s[abs + 4..]; // bytes after "\n---"
        if after.starts_with('\n') || after.starts_with("\r\n") || after.is_empty() {
            return Some(abs + 1); // offset of "---..." (skip leading \n)
        }
        search_from = abs + 4;
    }
    None
}

/// Split a markdown table row `| label | value |` into `(label, value)`,
/// trimmed. Returns `None` for header / divider rows ("Metric" /
/// "----") and any line that doesn't look like a 2-column row.
fn split_table_row(line: &str) -> Option<(&str, &str)> {
    let line = line.trim();
    if !line.starts_with('|') {
        return None;
    }
    let inner = line.trim_matches('|');
    let parts: Vec<&str> = inner.split('|').map(str::trim).collect();
    if parts.len() < 2 {
        return None;
    }
    let label = parts[0];
    let value = parts[1];
    // Skip header row + divider row.
    if label.eq_ignore_ascii_case("metric") {
        return None;
    }
    if label.chars().all(|c| c == '-' || c == ':' || c == ' ') {
        return None;
    }
    Some((label, value))
}

/// Parse a percentage value: `"-57.80%"` → `dec!(-57.80)`. Tolerates
/// unicode minus (`−`), trailing `%`, and incidental whitespace.
fn parse_pct(s: &str) -> Option<Decimal> {
    let cleaned = s
        .trim()
        .trim_end_matches('%')
        .trim()
        .replace('\u{2212}', "-") // unicode minus
        .replace([',', '_'], "");
    Decimal::from_str(&cleaned).ok()
}

/// Parse a decimal value with tolerance for `$`, `USDT`, unicode
/// minus, thousands-separators, etc.
fn parse_decimal_loose(s: &str) -> Option<Decimal> {
    let cleaned = s
        .trim()
        .trim_start_matches('$')
        .trim_end_matches(" USDT")
        .trim_end_matches("USDT")
        .trim()
        .replace('\u{2212}', "-")
        .replace([',', '_'], "");
    Decimal::from_str(&cleaned).ok()
}

/// Parse a `u64` count with tolerance for thousands-separators.
fn parse_count(s: &str) -> Option<u64> {
    let cleaned = s.trim().replace([',', '_'], "");
    u64::from_str(&cleaned).ok()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use rust_decimal_macros::dec;

    const RSI_REVERSION_REPORT: &str = "---
scenario: btc-2023-1m-rsi-reversion
seed: 0xC0FFEE
generated: 2026-04-20T15:20:17Z
---

# Backtest Report — btc-2023-1m-rsi-reversion

## Summary

| Metric               | Value                      |
|----------------------|----------------------------|
| Scenario             | btc-2023-1m-rsi-reversion  |
| Total return         | -57.80%                    |
| Sharpe ratio (ann.)  | -55.4257                   |
| Max drawdown         | 57.81%                     |
| Trades               | 14118                      |

## Reconciliation

PASS.
";

    #[test]
    fn parses_rsi_reversion_sample_report() {
        let m = parse_from_str(RSI_REVERSION_REPORT).expect("parse ok");
        assert_eq!(m.total_return_pct, dec!(-57.80));
        assert_eq!(m.sharpe, dec!(-55.4257));
        assert!(m.sharpe_present);
        assert_eq!(m.max_drawdown_pct, dec!(57.81));
        assert_eq!(m.trades, 14118);
        assert!(!m.cagr_present);
        assert!(!m.win_rate_present);
    }

    #[test]
    fn parses_negative_return_sample_correctly() {
        let body = "## Summary\n\n| Metric | Value |\n|---|---|\n| Total return | -42.10% |\n";
        let m = parse_from_str(body).expect("parse ok");
        assert_eq!(m.total_return_pct, dec!(-42.10));
    }

    #[test]
    fn parses_zero_trades_sample_returns_ok() {
        let body = "## Summary\n\n| Metric | Value |\n|---|---|\n| Trades | 0 |\n";
        let m = parse_from_str(body).expect("parse ok");
        assert_eq!(m.trades, 0);
    }

    #[test]
    fn missing_field_returns_marked_absent() {
        let body = "## Summary\n\n| Metric | Value |\n|---|---|\n| Total return | 0% |\n";
        let m = parse_from_str(body).expect("parse ok");
        assert!(!m.cagr_present);
        assert!(!m.sharpe_present);
        assert!(!m.win_rate_present);
        assert_eq!(m.cagr_pct, Decimal::ZERO);
        assert_eq!(m.win_rate_pct, Decimal::ZERO);
        assert_eq!(m.sharpe, Decimal::ZERO);
    }

    #[test]
    fn no_summary_heading_returns_err() {
        let body = "# Some other report\n\nNo summary table.\n";
        let res = parse_from_str(body);
        assert!(matches!(res, Err(ParseError::NoSummaryHeading)));
    }

    #[test]
    fn parses_cagr_and_win_rate_when_present() {
        let body = "## Summary\n\n| Metric | Value |\n|---|---|\n| CAGR | 12.30% |\n| Win rate | 55.50% |\n";
        let m = parse_from_str(body).expect("parse ok");
        assert!(m.cagr_present);
        assert_eq!(m.cagr_pct, dec!(12.30));
        assert!(m.win_rate_present);
        assert_eq!(m.win_rate_pct, dec!(55.50));
    }

    /// Iterate every committed `spec/<feature>/reports/backtest-*.md`
    /// and assert each parse returns `Ok(_)` (no field aborts the
    /// parser on any of the anchored reports + any extras).
    #[test]
    fn all_anchored_reports_parse_ok() {
        // From this crate's manifest dir up to the workspace root.
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let spec_root = workspace_root.join("spec");
        let mut backtests: Vec<PathBuf> = Vec::new();
        collect_backtest_reports(&spec_root, &mut backtests);
        for path in &backtests {
            let res = parse_from_report(path);
            assert!(res.is_ok(), "parse failed for {}: {res:?}", path.display());
        }
        assert!(
            backtests.len() >= 9,
            "expected ≥ 9 anchored backtest-*.md across spec/<feature>/reports/, found {}",
            backtests.len()
        );
    }

    /// Recursively walk `root` collecting every `*/reports/backtest-*.md`
    /// path. Skips well-known cross-cutting subtrees (`design`,
    /// `archive`) so the walk doesn't descend into the design system or
    /// the historical tarball.
    fn collect_backtest_reports(root: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(root) else { return; };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if path.is_dir() {
                if name == "design" || name == "archive" {
                    continue;
                }
                collect_backtest_reports(&path, out);
            } else if name.starts_with("backtest-") && name.ends_with(".md") {
                out.push(path);
            }
        }
    }
}
