//! cockpit-baseline-panel v0.1.0 — passive-BH curve loader + embedded
//! §7.1 metrics (T2 / T3).
//!
//! Pure-`ui` data layer, **no new crate edge** (`ui` already depends on
//! `trading_core` + `std::fs`). Mirrors `models/registry_read.rs`'s K2
//! never-panic contract and `viewer::load_equity_companion`'s synchronous
//! file-read shape.
//!
//! Two halves, per Decision D1 = (c) (file-load the curve, embed the
//! scalars):
//!
//! 1. [`load_baseline_curve`] — reads the committed 3-column CSV
//!    (`bar_index,timestamp_utc,equity_usd`) → [`EquitySeries`]. The
//!    timestamp column is **minute-precision Zulu** (`2024-01-01T00:00Z`),
//!    which `Rfc3339` rejects (no seconds) — so the loader parses it with
//!    an explicit `time` `format_description`. Equity is `Decimal` →
//!    `Money<Usdt>`, never `f64`. Missing file / parse miss / `from_points`
//!    error → `PanelState::Error(BASELINE_DATA_UNAVAILABLE)`; a
//!    header-only file → `PanelState::Empty`. Never panics.
//! 2. [`baseline_metrics`] — the realized §7.1 scalars for a year,
//!    embedded as a typed `const`-built [`BacktestMetrics`]. Never errors.
//!
//! **RE-SYNC contract (D1 cost):** the embedded scalars mirror
//! `docs/runbooks/artifacts/passive-baseline-2026-06-08/passive-baseline-characterization.md`
//! §7.1 (the **realized single-path** row, NOT bootstrap p50). They go
//! stale only if the characterization is re-run with different numbers;
//! the `baseline_metrics_match_characterization` unit test below trips on
//! a silent edit and is the re-sync trigger. See the per-year
//! doc-comments on [`baseline_metrics`].

#![allow(clippy::needless_pass_by_value)]

use std::path::{Path, PathBuf};

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::PrimitiveDateTime;
use time::format_description::BorrowedFormatItem;
use time::macros::format_description;
use trading_core::{BacktestMetrics, EquitySeries, Money, Timestamp, Usdt};

use crate::state::{BaselineYear, PanelState};
use crate::strings::BASELINE_DATA_UNAVAILABLE;

/// Minute-precision Zulu timestamp shape carried by the BH CSV
/// (`2024-01-01T00:00Z`). The standard `Rfc3339` parser **rejects** this
/// form because it has no seconds field; this explicit description accepts
/// it. (Some rows carry an hour-precision stamp like `2024-12-31T23:00Z`
/// — same shape, still minute-precision with `:00` minutes.)
///
/// The trailing `Z` is consumed as a **literal** character (not an offset
/// directive — `time` has no `Z`-shorthand offset token), so the value is
/// parsed as a [`PrimitiveDateTime`] and `.assume_utc()`'d. The runbook
/// CSV's `_utc` column name + the `Z` suffix both assert UTC, so the
/// assumption is exact, not a guess.
const ZULU_MINUTE: &[BorrowedFormatItem<'static>] =
    format_description!("[year]-[month]-[day]T[hour]:[minute]Z");

/// Load one year's realized BH equity curve from its committed CSV (R1).
///
/// Synchronous, **never panics** (mirrors `registry_read.rs` K2 +
/// `viewer::load_equity_companion`). Returns:
///
/// - [`PanelState::Ready`] with the [`EquitySeries`] on success (the band
///   is computed for free by `from_points`).
/// - [`PanelState::Empty`] when the file parses to **zero data rows**
///   (header only) — not expected (data is committed), but honest.
/// - [`PanelState::Error`]`(BASELINE_DATA_UNAVAILABLE)` when the file is
///   missing/unreadable, a row fails to parse (bad timestamp, non-decimal
///   equity), or `from_points` rejects the points.
///
/// `bar_index` is **informational — ignored**. File row order is
/// oldest-first (which `from_points` requires); the loader preserves it
/// and does not re-sort.
#[must_use]
pub fn load_baseline_curve(path: &Path) -> PanelState<EquitySeries> {
    let Ok(text) = std::fs::read_to_string(path) else {
        tracing::debug!(
            path = %path.display(),
            "load_baseline_curve: CSV not found or unreadable — Error state"
        );
        return PanelState::Error(BASELINE_DATA_UNAVAILABLE.into());
    };

    match parse_baseline_csv(&text) {
        Ok(points) if points.is_empty() => PanelState::Empty,
        Ok(points) => match EquitySeries::from_points(points) {
            Ok(series) => PanelState::Ready(series),
            Err(e) => {
                tracing::debug!(
                    path = %path.display(),
                    error = %e,
                    "load_baseline_curve: from_points rejected the curve — Error state"
                );
                PanelState::Error(BASELINE_DATA_UNAVAILABLE.into())
            }
        },
        Err(line_no) => {
            tracing::debug!(
                path = %path.display(),
                line = line_no,
                "load_baseline_curve: CSV row failed to parse — Error state"
            );
            PanelState::Error(BASELINE_DATA_UNAVAILABLE.into())
        }
    }
}

/// Parse the 3-column BH CSV body into `(Timestamp, Money<Usdt>)` points
/// in file order.
///
/// `Ok(Vec<…>)` on success (the vec is empty when the file is header-only
/// or blank). `Err(line_no)` on the first malformed data row, carrying the
/// 1-based line number for the breadcrumb. Splitting this out keeps
/// [`load_baseline_curve`] readable and lets the unit tests exercise the
/// timestamp + decimal parse directly.
fn parse_baseline_csv(text: &str) -> Result<Vec<(Timestamp, Money<Usdt>)>, usize> {
    let mut points: Vec<(Timestamp, Money<Usdt>)> = Vec::new();

    for (idx, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        // Skip blank lines and the header row (`bar_index,...`).
        if line.is_empty() || idx == 0 {
            continue;
        }

        let mut cols = line.split(',');
        // Column 0 = bar_index (informational, ignored).
        let _bar_index = cols.next();
        let Some(ts_raw) = cols.next() else {
            return Err(idx + 1);
        };
        let Some(equity_raw) = cols.next() else {
            return Err(idx + 1);
        };

        let Ok(pdt) = PrimitiveDateTime::parse(ts_raw.trim(), ZULU_MINUTE) else {
            return Err(idx + 1);
        };
        let Ok(equity_dec) = equity_raw.trim().parse::<Decimal>() else {
            return Err(idx + 1);
        };

        points.push((
            Timestamp::new(pdt.assume_utc()),
            Money::<Usdt>::from_decimal(equity_dec),
        ));
    }

    Ok(points)
}

/// The realized §7.1 metrics for a year, embedded as a `const`-built
/// [`BacktestMetrics`] (D1 = c). **Never errors** — this is a pure value
/// map, so a missing CSV still leaves the KPI strip populated (an honest
/// degrade: the numbers are known; only the drawn line is absent).
///
/// The six scalars map onto the six fixed `kpi_strip` cards. `win_rate`
/// and `trades` are not meaningful for buy-once-hold:
/// `win_rate_present = false` (renders `—`), `trades = 0`. Sortino /
/// Calmar have no KPI slot (A2) and surface as caption text only
/// (`BASELINE_RISK_DETAIL`) — they are NOT in this struct.
///
/// RE-SYNC: values mirror
/// `passive-baseline-characterization.md` §7.1 (the **realized
/// single-path** row, NOT bootstrap p50 — the panel draws the realized
/// curve, so the strip must match the line):
///
/// | Field              | 2023     | 2024    |
/// |--------------------|----------|---------|
/// | `total_return_pct` | `196.22` | `91.04` |
/// | `cagr_pct`         | `196.22` | `91.04` |
/// | `sharpe`           | `1.8417` | `0.8925`|
/// | `max_drawdown_pct` | `34.57`  | `48.95` |
///
/// **CAGR derivation (not a copied value).** §7.1 publishes `TotalReturn%`
/// but no separate `CAGR%`. For a single full-year hold the annualized
/// growth rate equals the total return (the period **is** one year), so
/// `cagr_pct = total_return_pct` is correct, not a fabrication. The §7.1
/// footnote derives Calmar = CAGR / maxDD; with CAGR = 196.22% and
/// maxDD = 34.57%, 1.9622 / 0.3457 ≈ 5.677 — the published Calmar — which
/// independently confirms CAGR ≈ total-return for this horizon.
#[must_use]
pub fn baseline_metrics(year: BaselineYear) -> BacktestMetrics {
    match year {
        BaselineYear::Y2023 => BacktestMetrics {
            total_return_pct: dec!(196.22),
            cagr_pct: dec!(196.22),
            cagr_present: true,
            sharpe: dec!(1.8417),
            sharpe_present: true,
            max_drawdown_pct: dec!(34.57),
            // Buy-once-hold: win rate is not meaningful — render `—`.
            win_rate_pct: Decimal::ZERO,
            win_rate_present: false,
            trades: 0,
        },
        BaselineYear::Y2024 => BacktestMetrics {
            total_return_pct: dec!(91.04),
            cagr_pct: dec!(91.04),
            cagr_present: true,
            sharpe: dec!(0.8925),
            sharpe_present: true,
            max_drawdown_pct: dec!(48.95),
            win_rate_pct: Decimal::ZERO,
            win_rate_present: false,
            trades: 0,
        },
    }
}

/// Resolve the committed CSV path for a year, relative to the workspace
/// root (`CARGO_MANIFEST_DIR` = `crates/ui`, so `../..` reaches the repo
/// root). Mirrors how `registry_read` resolves
/// `crates/forecast/checkpoints/...` from a manifest-relative base.
///
/// Isolated in one fn so the Error-state test (T8) can construct a sibling
/// "bogus path" without re-implementing the join. **Never** hardcodes an
/// absolute path.
#[must_use]
pub fn baseline_csv_path(year: BaselineYear) -> PathBuf {
    let file = match year {
        BaselineYear::Y2023 => "bh-equity-curve-2023.csv",
        BaselineYear::Y2024 => "bh-equity-curve-2024.csv",
    };
    workspace_root()
        .join("docs/runbooks/artifacts/passive-baseline-2026-06-08")
        .join(file)
}

/// Workspace root, derived from this crate's manifest dir
/// (`<root>/crates/ui` → `<root>`). Single source of the base path so the
/// Error-state test can point at a bogus child of it.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    // ── timestamp parse (T8: pin the `…T00:00Z` minute-precision shape) ──

    /// The minute-precision Zulu form the BH CSV uses parses cleanly with
    /// the explicit description (as a `PrimitiveDateTime` + `assume_utc`) —
    /// and is exactly what `Rfc3339` rejects.
    #[test]
    fn parses_minute_precision_zulu_timestamp() {
        use time::OffsetDateTime;
        use time::format_description::well_known::Rfc3339;

        let pdt = PrimitiveDateTime::parse("2024-01-01T00:00Z", ZULU_MINUTE)
            .expect("minute-precision Zulu must parse");
        let odt = pdt.assume_utc();
        assert_eq!(odt.year(), 2024);
        assert_eq!(u8::from(odt.month()), 1);
        assert_eq!(odt.day(), 1);
        assert_eq!(odt.hour(), 0);
        assert_eq!(odt.minute(), 0);
        assert_eq!(odt.offset(), time::UtcOffset::UTC);
        // An hour-precision row (`23:00`) is the same shape.
        let pdt2 = PrimitiveDateTime::parse("2024-12-31T23:00Z", ZULU_MINUTE)
            .expect("hour-precision row must parse");
        assert_eq!(pdt2.hour(), 23);

        // Falsification: Rfc3339 cannot read this shape (no seconds).
        assert!(
            OffsetDateTime::parse("2024-01-01T00:00Z", &Rfc3339).is_err(),
            "Rfc3339 is expected to reject minute-precision Zulu — that is why \
             the loader uses an explicit format_description"
        );
    }

    // ── parse_baseline_csv unit coverage ─────────────────────────────────────

    #[test]
    fn parses_well_formed_body_in_file_order() {
        let csv = "bar_index,timestamp_utc,equity_usd\n\
                   0,2024-01-01T00:00Z,100000.00\n\
                   25,2024-01-02T00:00Z,105017.03\n";
        let points = parse_baseline_csv(csv).expect("well-formed CSV parses");
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].1.amount(), dec!(100000.00));
        assert_eq!(points[1].1.amount(), dec!(105017.03));
        // File order preserved (oldest-first).
        assert!(points[0].0.unix_millis() < points[1].0.unix_millis());
    }

    #[test]
    fn header_only_file_parses_to_zero_points() {
        let csv = "bar_index,timestamp_utc,equity_usd\n";
        let points = parse_baseline_csv(csv).expect("header-only parses");
        assert!(points.is_empty());
    }

    #[test]
    fn bad_timestamp_row_returns_err_line() {
        let csv = "bar_index,timestamp_utc,equity_usd\n\
                   0,not-a-timestamp,100000.00\n";
        assert_eq!(parse_baseline_csv(csv), Err(2));
    }

    #[test]
    fn non_decimal_equity_row_returns_err_line() {
        let csv = "bar_index,timestamp_utc,equity_usd\n\
                   0,2024-01-01T00:00Z,not-a-number\n";
        assert_eq!(parse_baseline_csv(csv), Err(2));
    }

    // ── load_baseline_curve behaviour ────────────────────────────────────────

    /// Missing file → `Error(BASELINE_DATA_UNAVAILABLE)`, never panics.
    /// This is the fixtures-only checkout path (R7 / AC3).
    #[test]
    fn missing_file_yields_error_state_no_panic() {
        let bogus = workspace_root().join("definitely/not/a/real/baseline.csv");
        match load_baseline_curve(&bogus) {
            PanelState::Error(msg) => assert_eq!(msg.as_str(), BASELINE_DATA_UNAVAILABLE),
            other => panic!("expected Error, got {}", other.variant_name()),
        }
    }

    /// Header-only path on disk → `Empty` (zero data rows, R4).
    #[test]
    fn header_only_path_yields_empty_state() {
        let dir = std::env::temp_dir().join(format!("baseline_loader_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("header_only.csv");
        std::fs::write(&path, "bar_index,timestamp_utc,equity_usd\n").expect("write");
        assert!(matches!(load_baseline_curve(&path), PanelState::Empty));
        let _ = std::fs::remove_file(&path);
    }

    /// The committed CSVs load to `Ready` with first point `$100,000.00`
    /// (R1 / AC1). Gated on the file existing so a minimal checkout that
    /// omits the runbook artifacts does not fail this unit test — the
    /// missing-file path is covered separately above.
    #[test]
    fn committed_csvs_load_to_ready_first_point_100k() {
        for year in [BaselineYear::Y2023, BaselineYear::Y2024] {
            let path = baseline_csv_path(year);
            if !path.exists() {
                // Minimal checkout — skip; missing-file path is tested above.
                continue;
            }
            match load_baseline_curve(&path) {
                PanelState::Ready(series) => {
                    assert!(!series.points.is_empty(), "curve must have points");
                    assert_eq!(
                        series.points[0].equity.amount(),
                        dec!(100000.00),
                        "BH starts at $100,000.00"
                    );
                    // Drawdown band is computed for free (non-negative).
                    assert!(series.max_drawdown_pct >= Decimal::ZERO);
                }
                other => panic!(
                    "expected Ready for {}, got {}",
                    path.display(),
                    other.variant_name()
                ),
            }
        }
    }

    // ── D1 re-sync trip: embedded scalars == documented §7.1 ─────────────────

    /// RE-SYNC trip (D1 cost). Asserts the six embedded scalars equal the
    /// documented §7.1 *realized* values for each year. A silent edit to
    /// `baseline_metrics` trips this test — it is the re-sync trigger if
    /// the characterization is ever re-run.
    #[test]
    fn baseline_metrics_match_characterization() {
        // §7.1 realized row — 2023.
        let m23 = baseline_metrics(BaselineYear::Y2023);
        assert_eq!(m23.total_return_pct, dec!(196.22), "2023 TotalReturn%");
        assert_eq!(m23.cagr_pct, dec!(196.22), "2023 CAGR (= total, 1-yr)");
        assert!(m23.cagr_present);
        assert_eq!(m23.sharpe, dec!(1.8417), "2023 Sharpe");
        assert!(m23.sharpe_present);
        assert_eq!(m23.max_drawdown_pct, dec!(34.57), "2023 MaxDD%");
        assert!(!m23.win_rate_present, "win rate not meaningful for BH");
        assert_eq!(m23.trades, 0, "buy-once-hold");

        // §7.1 realized row — 2024.
        let m24 = baseline_metrics(BaselineYear::Y2024);
        assert_eq!(m24.total_return_pct, dec!(91.04), "2024 TotalReturn%");
        assert_eq!(m24.cagr_pct, dec!(91.04), "2024 CAGR (= total, 1-yr)");
        assert!(m24.cagr_present);
        assert_eq!(m24.sharpe, dec!(0.8925), "2024 Sharpe");
        assert!(m24.sharpe_present);
        assert_eq!(m24.max_drawdown_pct, dec!(48.95), "2024 MaxDD%");
        assert!(!m24.win_rate_present);
        assert_eq!(m24.trades, 0);
    }

    /// `baseline_csv_path` is workspace-relative (never an absolute hard-
    /// code) and names the right per-year file.
    #[test]
    fn csv_path_is_workspace_relative_and_year_specific() {
        let p23 = baseline_csv_path(BaselineYear::Y2023);
        let p24 = baseline_csv_path(BaselineYear::Y2024);
        assert!(p23.ends_with("bh-equity-curve-2023.csv"));
        assert!(p24.ends_with("bh-equity-curve-2024.csv"));
        assert!(
            p23.to_string_lossy()
                .contains("passive-baseline-2026-06-08")
        );
    }
}
