//! Canonical Yahoo-cache report-emit helper — lab-yahoo-realdata v0.1.3.
//!
//! This module is the **single point of truth** for all Yahoo-cache-sourced
//! backtest report emission (D-V0.1.3-1).  It owns two Yahoo-specific
//! concerns:
//!
//! 1. **Data-source string formation.**  Body line is
//!    `yahoo-cache:{ticker}/{interval}/{year}` — NO `rev=` substring.
//!    `run_yahoo_*` binaries MUST NOT hand-format this string; they call
//!    [`YahooReportContext::data_source`] instead.
//!
//! 2. **`revision_sha:` frontmatter injection.**  The full 64-char hex
//!    aggregate SHA from `data/yahoo/REVISION.toml` is written as a new
//!    top-level frontmatter line `revision_sha:` immediately after
//!    `data_source:`.  This is accomplished by passing `Some(sha)` to the
//!    underlying [`crate::report::sma::write`] parameter; the `None` arm
//!    preserves byte-identical output for the 33 existing Binance SMA
//!    anchors (D-V0.1.3-1 `None`-arm contract).
//!
//! # Adding a future Yahoo emitter (v0.2.0+)
//!
//! 1. Add `pub fn emit_{strategy}_report(ctx, input, ...)` here (around 10 lines of code).
//! 2. Add optional `revision_sha: Option<&str>` to the underlying
//!    `report::{strategy}::write` (Binance path passes `None`).
//! 3. The `run_yahoo_{strategy}` binary calls `emit_{strategy}_report`.
//! 4. Do NOT call `report::{strategy}::write` directly from any Yahoo binary.

use std::path::Path;

use anyhow::Result;
use rust_decimal::Decimal;

use crate::cli_types::{BacktestState, SmaScenarioInput, StrategyMeta};

// ── Context ───────────────────────────────────────────────────────────────────

/// Caller-supplied Yahoo-specific context for report emission.
///
/// Carries the three fields that form the `data_source:` body line PLUS the
/// `revision_sha:` frontmatter value.  All four fields are `&str` slices;
/// the caller owns the allocations.
pub struct YahooReportContext<'a> {
    /// Yahoo ticker, e.g. `"BTC-USD"`.
    pub ticker: &'a str,
    /// Fetch cadence, e.g. `"1d"`.
    pub interval: &'a str,
    /// Calendar year, e.g. `2024`.
    pub year: u16,
    /// Full 64-char hex aggregate SHA from `data/yahoo/REVISION.toml`.
    pub revision_sha: &'a str,
}

impl YahooReportContext<'_> {
    /// Returns the body `Data source` line WITHOUT any `rev=` substring.
    ///
    /// This is the single constructor for Yahoo data-source strings.
    /// No other code may format `yahoo-cache:…` strings.
    ///
    /// Example output: `"yahoo-cache:BTC-USD/1d/2024"`
    #[must_use]
    pub fn data_source(&self) -> String {
        format!(
            "yahoo-cache:{}/{}/{}",
            self.ticker, self.interval, self.year
        )
    }
}

// ── SMA emitter ───────────────────────────────────────────────────────────────

/// Emit a Yahoo-cache SMA backtest report.
///
/// This is the only permitted call site for SMA report emission from Yahoo
/// binaries.  It delegates to [`crate::report::sma::write`] with:
///
/// - `data_source` = `ctx.data_source()` (no `rev=`).
/// - `revision_sha = Some(ctx.revision_sha)` → injects `revision_sha: <sha>`
///   into the YAML front-matter immediately after `data_source:`.
///
/// # Errors
///
/// Propagates any I/O error from the underlying writer.
#[allow(clippy::too_many_arguments)]
pub fn emit_sma_report(
    ctx: &YahooReportContext<'_>,
    sma_input: &SmaScenarioInput,
    state: &BacktestState,
    initial_capital: Decimal,
    final_equity: Decimal,
    seed: u64,
    elapsed_secs: f64,
    report_path: &Path,
    strategy_meta: &StrategyMeta,
) -> Result<()> {
    let data_source = ctx.data_source();
    crate::report::sma::write(
        sma_input,
        state,
        initial_capital,
        final_equity,
        seed,
        &data_source,
        elapsed_secs,
        report_path,
        strategy_meta,
        Some(ctx.revision_sha),
    )
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_source_no_rev_suffix() {
        let ctx = YahooReportContext {
            ticker: "BTC-USD",
            interval: "1d",
            year: 2024,
            revision_sha: "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
        };
        let ds = ctx.data_source();
        assert_eq!(ds, "yahoo-cache:BTC-USD/1d/2024");
        assert!(
            !ds.contains("rev="),
            "data_source MUST NOT contain 'rev=': got '{ds}'"
        );
    }

    #[test]
    fn data_source_eth() {
        let ctx = YahooReportContext {
            ticker: "ETH-USD",
            interval: "1d",
            year: 2024,
            revision_sha: "0000000000000000000000000000000000000000000000000000000000000000",
        };
        assert_eq!(ctx.data_source(), "yahoo-cache:ETH-USD/1d/2024");
    }

    #[test]
    fn revision_sha_is_64_hex_chars() {
        // Smoke-check: any REVISION.toml SHA should be exactly 64 hex chars.
        let ctx = YahooReportContext {
            ticker: "BTC-USD",
            interval: "1d",
            year: 2024,
            revision_sha: "7b33166e1eb80dc0e0076dcde89ca56f36b9b0d695d21aed8effcb2e052ef5d7",
        };
        assert_eq!(ctx.revision_sha.len(), 64);
        assert!(
            ctx.revision_sha.chars().all(|c| c.is_ascii_hexdigit()),
            "revision_sha must be 64 hex chars"
        );
    }
}
