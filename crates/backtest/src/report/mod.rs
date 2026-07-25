//! Extracted report-writer modules — Phase B (ADR-0035).
//!
//! Each sub-module contains the extracted body of one `write_*_report`
//! function from `main.rs`. Writers are called from scenario modules after
//! the backtest run completes.
//!
//! # Determinism contract
//!
//! The bytes produced by each writer must be byte-identical to those
//! produced by the original `main.rs` functions when given the same inputs.
//! The 22 body-SHA-256 anchors in `evidence/anchors.toml` guard this contract.

pub mod momentum;
pub mod pairs;
pub mod regime_dispatcher;
pub mod sma;
pub mod tcn_overlay;
pub mod yahoo;

/// Write the equity companion CSV beside a just-emitted backtest report.
///
/// Layout: `<report_dir>/artifacts/<report_file_stem>/equity-<stamp>.csv`
///
/// `<stamp>` is extracted from the report file stem
/// (`backtest-<stamp>-<scenario>`), so the companion filename is stable
/// and `starts_with("equity-")` + `.csv` (the loader's match criteria).
///
/// Column schema (matches `reports::csv_artifacts::read_equity_csv`):
/// `ts,equity_total_usdt,realized_pnl_usdt,unrealized_pnl_usdt,cash_balance_usdt`.
/// `ts` is RFC3339.  `equity_total_usdt` = real per-bar equity.
/// `realized_pnl_usdt`, `unrealized_pnl_usdt`, `cash_balance_usdt` = `0`
/// (honest zero — these columns are not tracked per-bar at the CLI seam;
/// matches the ADR-0055 § D-companion precedent for `lab-runs/`).
///
/// # Errors
///
/// Returns `std::io::Error` on directory creation or file write failure,
/// or on timestamp formatting failure.
///
/// # Anchor safety
///
/// This function writes a **new** `.csv` file only.  It never modifies any
/// `.md` report body.  `scripts/verify_anchors.sh` globs only
/// `*/reports/backtest-*-<scenario>.md` and never descends `artifacts/`,
/// so the 119 body-SHA-256 anchors are unaffected.
pub fn write_equity_companion(
    report_path: &std::path::Path,
    equity_curve: &[rust_decimal::Decimal],
    start_year: i32,
) -> std::io::Result<()> {
    use std::fmt::Write as _;
    use time::format_description::well_known::Rfc3339;

    // Derive the artifacts sub-directory name from the report file stem
    // (= `backtest-<stamp>-<scenario>`).  The loader's first-match scan
    // (`artifacts/<any-subdir>/equity-*.csv`) does not require a true run_id.
    let stem = report_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "report_path has no valid UTF-8 file stem",
            )
        })?;

    // Extract the stamp from the stem: `backtest-<stamp>-<scenario>`.
    // The stem is `backtest-YYYYmmdd-HHMMSS-<scenario>`.
    // Split off the first two dash-separated tokens after "backtest-" to
    // reconstruct "YYYYmmdd-HHMMSS".  Fall back to the full stem if the
    // prefix is absent or too short.
    let stamp_owned: Option<String> = stem.strip_prefix("backtest-").and_then(|rest| {
        // Collect exactly the first two tokens: ["YYYYmmdd", "HHMMSS"]
        let parts: Vec<&str> = rest.splitn(3, '-').take(2).collect();
        if parts.len() == 2 {
            Some(parts.join("-"))
        } else {
            None
        }
    });
    let stamp = stamp_owned
        .as_deref()
        .filter(|s| s.len() >= 8) // sanity: at minimum YYYYMMDD
        .unwrap_or(stem);

    let report_dir = report_path
        .parent()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "no parent dir"))?;

    let artifacts_dir = report_dir.join("artifacts").join(stem);
    std::fs::create_dir_all(&artifacts_dir)?;

    let csv_path = artifacts_dir.join(format!("equity-{stamp}.csv"));

    // Build per-bar timestamps using the same helper the engine uses for the
    // in-memory RunReport.equity_series, so the ts column matches exactly.
    let timestamps = crate::engine::synthetic_timestamps(start_year, equity_curve.len());

    let mut out = String::from(
        "ts,equity_total_usdt,realized_pnl_usdt,unrealized_pnl_usdt,cash_balance_usdt\n",
    );
    for (ts, eq) in timestamps.iter().zip(equity_curve.iter()) {
        let ts_str = ts
            .inner()
            .format(&Rfc3339)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        // Decimal → string (never f64).  realized/unrealized/cash are not
        // tracked per-bar at the CLI seam → honest 0 (ADR-0055 § D-companion).
        let _ = writeln!(out, "{ts_str},{eq},0,0,0");
    }
    std::fs::write(&csv_path, out)
}
