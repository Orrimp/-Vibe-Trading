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

pub mod benchmark;
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

// ── The report body's risk line, DERIVED (bug-log #178) ───────────────────────

/// Render the report body's risk sentence from the limits the lane actually ran with.
///
/// Until 2026-11-08 four writers printed this sentence as a LITERAL — three as
/// `per-symbol cap=40% (ENFORCED in Order::new); NO portfolio gross cap on this lane` and
/// `regime_dispatcher` as `per-symbol cap=40%, portfolio cap=50%`. Both were copies of a
/// configuration, and copies drift:
///
/// - `#95`'s gate wiring, committed the same day, made "NO portfolio gross cap" false for
///   four of the five lanes rendered by `report/tcn_overlay.rs`;
/// - `regime_dispatcher`'s "portfolio cap=50%" was false in the other direction for as long
///   as the cap was declared-but-unenforced, and is true now only by coincidence;
/// - the `40%` is currently true of every lane's ENTRY limits. (`sma_composed_run.rs:470`
///   sets `Decimal::MAX`, but that is `cover_limits` — a deliberate per-leg override for the
///   short-cover leg under `#80`, not a lane limit. Checked before claiming otherwise.)
///
/// So the sentence is a function of the values. A sentence asserting a configuration is a
/// copy of that configuration; this is the one shape bug-log `#175` and `#178` share.
#[must_use]
pub fn risk_line(
    per_symbol: rust_decimal::Decimal,
    portfolio: Option<rust_decimal::Decimal>,
) -> String {
    use rust_decimal::Decimal;
    let hundred = Decimal::from(100u32);
    let per_symbol_str = if per_symbol >= Decimal::ONE {
        // 100 % of equity or more is not a cap any order can hit; say so rather than
        // printing an arithmetic artifact like `17014118346046923173168730371588410572%`.
        "unbounded".to_string()
    } else {
        format!("{}%", (per_symbol * hundred).normalize())
    };
    let portfolio_str = match portfolio {
        None => "none declared on this lane (bug-log #95)".to_string(),
        Some(c) => format!(
            "{}% — ENFORCED as a per-order entry gate on RESULTING exposure (bug-log #95, #71); see the Gross-cap refusals row",
            (c * hundred).normalize()
        ),
    };
    format!(
        "per-symbol cap={per_symbol_str} (ENFORCED in Order::new); portfolio gross cap: {portfolio_str}"
    )
}

#[cfg(test)]
mod risk_line_tests {
    use super::risk_line;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    /// Both sides of the portfolio cap, because the whole defect was one side printed
    /// unconditionally.
    #[test]
    fn a_declared_portfolio_cap_and_an_absent_one_render_differently_and_correctly() {
        let declared = risk_line(dec!(0.40), Some(dec!(0.50)));
        let absent = risk_line(dec!(0.40), None);
        assert!(declared.contains("portfolio gross cap: 50%"), "{declared}");
        assert!(
            declared.contains("ENFORCED as a per-order entry gate"),
            "{declared}"
        );
        assert!(absent.contains("none declared"), "{absent}");
        assert!(
            !absent.contains("entry gate"),
            "an absent cap must not claim enforcement: {absent}"
        );
        assert_ne!(declared, absent);
    }

    /// Both percentages track the VALUES they are given — without this the line would be
    /// the old literal with an extra step.
    #[test]
    fn both_percentages_track_the_caps_they_are_given() {
        let l = risk_line(dec!(0.25), Some(dec!(0.35)));
        assert!(l.contains("per-symbol cap=25%"), "{l}");
        assert!(l.contains("portfolio gross cap: 35%"), "{l}");
        assert!(!l.contains("40%"), "the old literal must not survive: {l}");
        assert!(!l.contains("50%"), "the old literal must not survive: {l}");
    }

    /// `sma_composed_run.rs:470` runs with `Decimal::MAX`. Printing that times 100 would be
    /// a 39-digit number presented as a percentage.
    #[test]
    fn an_effectively_absent_per_symbol_cap_says_unbounded_not_an_astronomical_number() {
        let l = risk_line(Decimal::MAX, Some(dec!(0.50)));
        assert!(l.contains("per-symbol cap=unbounded"), "{l}");
        assert!(!l.contains("170141183460469"), "{l}");
        // And a real cap must NOT read as unbounded, or the branch is untested.
        assert!(!risk_line(dec!(0.40), None).contains("unbounded"));
    }
}
