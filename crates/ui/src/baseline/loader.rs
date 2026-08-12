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
//! the `baseline_metrics_match_characterization` unit test below is the
//! re-sync trigger. **Since the story-2-18 review that test PARSES the
//! characterization document** — it used to assert the constants against
//! literals typed into this same file, which cannot detect a re-run: both
//! sides of the comparison were authored here, so the "trigger" would have
//! stayed green forever while the doc moved underneath it.

#![allow(clippy::needless_pass_by_value)]

use std::path::{Path, PathBuf};

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::PrimitiveDateTime;
use time::format_description::BorrowedFormatItem;
use time::macros::format_description;
use trading_core::{BacktestMetrics, EquitySeries, Money, Timestamp, Usdt};

use crate::state::{BaselineYear, PanelState};
use crate::strings::{
    BASELINE_DATA_CORRUPT, BASELINE_DATA_UNAVAILABLE, BASELINE_RISK_DETAIL_FMT,
    BASELINE_SAMPLING_NOTE_FMT, BASELINE_SHARPE_NOTE_FMT,
};

/// The exact header the committed BH CSV must carry, in order.
///
/// Validated rather than skipped-by-index (story-2-18 review M-schema). The
/// old loader discarded line 0 unconditionally, so:
///
/// * a **headerless** file silently dropped its bar-0 `$100,000.00` anchor and
///   re-based every derived figure (peak, drawdown, total return) against the
///   second row;
/// * a column inserted before `equity_usd` fed the *timestamp* or an index
///   into the equity slot — or, worse, a plausible-looking dollar column into
///   the drawdown math — and still produced a clean `Ready` panel.
///
/// Neither is detectable downstream: both yield a well-formed curve of the
/// right shape carrying wrong numbers.
const EXPECTED_CSV_HEADER: [&str; 3] = ["bar_index", "timestamp_utc", "equity_usd"];

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

/// Why a BH CSV could not be turned into points. Carries enough to write an
/// honest breadcrumb — and, crucially, to distinguish "absent" from "damaged"
/// on the operator's screen.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CsvDefect {
    /// The file holds no header line at all (0-byte or all-blank) — a
    /// truncated artifact, not an empty one.
    MissingHeader,
    /// The header line is present but is not
    /// `bar_index,timestamp_utc,equity_usd`; carries the offending line.
    BadHeader(String),
    /// A data row failed to parse; carries the 1-based line number.
    BadRow(usize),
}

/// Load one year's realized BH equity curve from its committed CSV (R1).
///
/// Synchronous, **never panics** (mirrors `registry_read.rs` K2 +
/// `viewer::load_equity_companion`). Returns:
///
/// - [`PanelState::Ready`] with the [`EquitySeries`] on success (the band
///   is computed for free by `from_points`).
/// - [`PanelState::Empty`] when the file carries a **valid header and zero
///   data rows** — not expected (data is committed), but honest.
/// - [`PanelState::Error`]`(BASELINE_DATA_UNAVAILABLE)` when the file is
///   missing or unreadable — "go fetch the artifacts".
/// - [`PanelState::Error`]`(BASELINE_DATA_CORRUPT)` when the file **is**
///   present but malformed: truncated/0-byte, wrong or missing header, a bad
///   row, or values `from_points` rejects — "this artifact is damaged".
///
/// The last two used to be the same message (story-2-18 review M-states),
/// which told an operator holding a corrupt CSV that it "isn't bundled in this
/// build" — the wrong diagnosis, pointing at the wrong fix.
///
/// `bar_index` is **informational — ignored** (but its column must be
/// present and named, see [`EXPECTED_CSV_HEADER`]). File row order is
/// oldest-first (which `from_points` requires); the loader preserves it
/// and does not re-sort.
#[must_use]
pub fn load_baseline_curve(path: &Path) -> PanelState<EquitySeries> {
    let Ok(text) = std::fs::read_to_string(path) else {
        tracing::debug!(
            path = %path.display(),
            "load_baseline_curve: CSV not found or unreadable — Error(unavailable)"
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
                    "load_baseline_curve: from_points rejected the curve — Error(corrupt)"
                );
                PanelState::Error(BASELINE_DATA_CORRUPT.into())
            }
        },
        Err(defect) => {
            tracing::debug!(
                path = %path.display(),
                defect = ?defect,
                "load_baseline_curve: CSV failed schema/row validation — Error(corrupt)"
            );
            PanelState::Error(BASELINE_DATA_CORRUPT.into())
        }
    }
}

/// Parse the 3-column BH CSV body into `(Timestamp, Money<Usdt>)` points
/// in file order, after **validating the header** against
/// [`EXPECTED_CSV_HEADER`].
///
/// `Ok(Vec<…>)` on success (the vec is empty only when the file is
/// header-only). `Err(CsvDefect)` on the first structural problem.
fn parse_baseline_csv(text: &str) -> Result<Vec<(Timestamp, Money<Usdt>)>, CsvDefect> {
    let mut points: Vec<(Timestamp, Money<Usdt>)> = Vec::new();
    let mut header_seen = false;

    for (idx, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        // The FIRST non-blank line must be the header. Validating it (rather
        // than skipping line 0 by index) is what makes a headerless file and a
        // reordered/renamed column detectable at all.
        if !header_seen {
            header_seen = true;
            let cols: Vec<&str> = line.split(',').map(str::trim).collect();
            if cols.len() != EXPECTED_CSV_HEADER.len()
                || !cols
                    .iter()
                    .zip(EXPECTED_CSV_HEADER.iter())
                    .all(|(got, want)| got.eq_ignore_ascii_case(want))
            {
                return Err(CsvDefect::BadHeader(line.to_string()));
            }
            continue;
        }

        let cols: Vec<&str> = line.split(',').collect();
        // Exactly three columns — a row that grew or lost one is a schema
        // break, not something to read positionally and hope.
        if cols.len() != EXPECTED_CSV_HEADER.len() {
            return Err(CsvDefect::BadRow(idx + 1));
        }
        // Column 0 = bar_index (informational, ignored).
        let (ts_raw, equity_raw) = (cols[1], cols[2]);

        let Ok(pdt) = PrimitiveDateTime::parse(ts_raw.trim(), ZULU_MINUTE) else {
            return Err(CsvDefect::BadRow(idx + 1));
        };
        let Ok(equity_dec) = equity_raw.trim().parse::<Decimal>() else {
            return Err(CsvDefect::BadRow(idx + 1));
        };

        points.push((
            Timestamp::new(pdt.assume_utc()),
            Money::<Usdt>::from_decimal(equity_dec),
        ));
    }

    if header_seen {
        Ok(points)
    } else {
        // Zero non-blank lines: a 0-byte or truncated-to-nothing artifact.
        // Reporting this as `Empty` (the pre-review behaviour) called a
        // damaged file "no data" and looked benign.
        Err(CsvDefect::MissingHeader)
    }
}

/// The realized §7.1 metrics for a year, embedded as a `const`-built
/// [`BacktestMetrics`] (D1 = c). **Never errors** — this is a pure value
/// map, so a missing CSV still leaves the KPI strip populated (an honest
/// degrade: the numbers are known; only the drawn line is absent).
///
/// The six scalars map onto the six fixed `kpi_strip` cards. `win_rate`
/// and `trades` are not meaningful for buy-once-hold:
/// `win_rate_present = false` (renders `—`), `trades = 0`. Sortino /
/// Calmar have no KPI slot (A2) and surface as caption text only (via
/// [`baseline_risk_detail`], backed by [`baseline_risk_facts`]) — they are
/// NOT in this struct.
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

/// The §7.1 / §7.3 characterization scalars that have **no KPI card**, kept
/// beside [`baseline_metrics`] so every embedded number has one home and one
/// re-sync test.
///
/// `bootstrap_p50_sharpe` comes from §7.3's reconciliation table, not §7.1:
/// it is the MEDIAN over 200 block-resampled paths and is the figure the
/// PRD/README headline as the passive bar (+1.74 / +1.10). The KPI card shows
/// the realized single-path Sharpe instead — correctly, since the panel draws
/// the realized path — which is precisely why the screen has to name which is
/// which (review H3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaselineRiskFacts {
    pub sortino: Decimal,
    pub calmar: Decimal,
    pub bootstrap_p50_sharpe: Decimal,
}

/// §7.1 Sortino / Calmar + §7.3 bootstrap-p50 Sharpe for a year.
#[must_use]
pub fn baseline_risk_facts(year: BaselineYear) -> BaselineRiskFacts {
    match year {
        BaselineYear::Y2023 => BaselineRiskFacts {
            sortino: dec!(2.5126),
            calmar: dec!(5.677),
            bootstrap_p50_sharpe: dec!(1.7353),
        },
        BaselineYear::Y2024 => BaselineRiskFacts {
            sortino: dec!(1.2047),
            calmar: dec!(1.853),
            bootstrap_p50_sharpe: dec!(1.1047),
        },
    }
}

/// The Sortino / Calmar caption line for the **active year** (review M-static).
///
/// Two decimals, matching how the figures read in §7.1 at the precision an
/// operator can act on.
#[must_use]
pub fn baseline_risk_detail(year: BaselineYear) -> String {
    let f = baseline_risk_facts(year);
    BASELINE_RISK_DETAIL_FMT
        .replace("{sortino}", &f.sortino.round_dp(2).to_string())
        .replace("{calmar}", &f.calmar.round_dp(2).to_string())
        .replace("{year}", year.label())
}

/// The Sharpe-provenance caption line for the active year (review H3) —
/// names the card's figure as **realized single-path** and cites the
/// **bootstrap p50** the project publishes as the passive bar.
#[must_use]
pub fn baseline_sharpe_note(year: BaselineYear) -> String {
    let realized = baseline_metrics(year).sharpe;
    let p50 = baseline_risk_facts(year).bootstrap_p50_sharpe;
    BASELINE_SHARPE_NOTE_FMT
        .replace("{realized}", &realized.round_dp(2).to_string())
        .replace("{p50}", &p50.round_dp(2).to_string())
        .replace("{year}", year.label())
}

/// Max drawdown of a loaded curve, in **percentage points** to two decimals
/// (`0.418176…` → `41.82`).
///
/// The `× 100` is the conversion the units-rename in `trading_core` exists to
/// keep visible: `EquitySeries::max_drawdown_frac` is a FRACTION,
/// `BacktestMetrics::max_drawdown_pct` is PERCENT.
#[must_use]
pub fn curve_max_drawdown_pct(series: &EquitySeries) -> Decimal {
    (series.max_drawdown_frac * Decimal::ONE_HUNDRED).round_dp(2)
}

/// The sampling-provenance caption for the active year (review H2), or `None`
/// when the curve is not loaded (nothing to compare against).
///
/// The curve figure is computed from the **loaded series**, never embedded:
/// regenerate the CSV and the on-screen number follows it, so the card/line
/// disagreement can never again be silent.
#[must_use]
pub fn baseline_sampling_note(year: BaselineYear, series: &EquitySeries) -> String {
    let card = baseline_metrics(year).max_drawdown_pct;
    BASELINE_SAMPLING_NOTE_FMT
        .replace("{rows}", &series.points.len().to_string())
        .replace(
            "{curve}",
            &format!("{}%", curve_max_drawdown_pct(series).normalize()),
        )
        .replace("{card}", &format!("{}%", card.normalize()))
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
        assert_eq!(parse_baseline_csv(csv), Err(CsvDefect::BadRow(2)));
    }

    #[test]
    fn non_decimal_equity_row_returns_err_line() {
        let csv = "bar_index,timestamp_utc,equity_usd\n\
                   0,2024-01-01T00:00Z,not-a-number\n";
        assert_eq!(parse_baseline_csv(csv), Err(CsvDefect::BadRow(2)));
    }

    // ── schema validation (story-2-18 review M-schema) ───────────────────────

    /// A **headerless** file must be rejected, not silently parsed with its
    /// first data row discarded.
    ///
    /// The pre-fix loader dropped line 0 by index, so this input lost the
    /// bar-0 `$100,000.00` anchor and re-based every derived figure against
    /// $110,000 — a `Ready` panel showing a wrong total return, a wrong peak
    /// and a wrong drawdown, with nothing anywhere to notice it.
    #[test]
    fn headerless_file_is_rejected_not_silently_rebased() {
        let csv = "0,2024-01-01T00:00Z,100000.00\n\
                   24,2024-01-02T00:00Z,110000.00\n";
        match parse_baseline_csv(csv) {
            Err(CsvDefect::BadHeader(line)) => {
                assert!(line.starts_with('0'), "carries the offending line: {line}");
            }
            other => panic!("headerless file must be a BadHeader defect, got {other:?}"),
        }
    }

    /// A column inserted BEFORE `equity_usd` used to feed the wrong field into
    /// the drawdown math and still produce a clean `Ready` state. The header
    /// check catches it at the door.
    #[test]
    fn reordered_or_extra_column_is_rejected() {
        let inserted = "bar_index,timestamp_utc,close_usd,equity_usd\n\
                        0,2024-01-01T00:00Z,42000.00,100000.00\n";
        assert!(matches!(
            parse_baseline_csv(inserted),
            Err(CsvDefect::BadHeader(_))
        ));

        let swapped = "bar_index,equity_usd,timestamp_utc\n\
                       0,100000.00,2024-01-01T00:00Z\n";
        assert!(matches!(
            parse_baseline_csv(swapped),
            Err(CsvDefect::BadHeader(_))
        ));
    }

    /// A 0-byte / whitespace-only file is a TRUNCATED artifact, not an empty
    /// one — it must not read as the benign "no data yet" state.
    #[test]
    fn zero_byte_file_is_a_missing_header_defect() {
        assert_eq!(parse_baseline_csv(""), Err(CsvDefect::MissingHeader));
        assert_eq!(parse_baseline_csv("\n  \n"), Err(CsvDefect::MissingHeader));
    }

    /// A data row that lost (or gained) a column is a schema break.
    #[test]
    fn short_data_row_is_rejected() {
        let csv = "bar_index,timestamp_utc,equity_usd\n\
                   0,2024-01-01T00:00Z\n";
        assert_eq!(parse_baseline_csv(csv), Err(CsvDefect::BadRow(2)));
    }

    /// **The state honesty gate.** Absent, corrupt and empty must reach the
    /// operator as three DIFFERENT statements — the pre-fix loader collapsed
    /// corrupt into "isn't bundled in this build", sending a reader with a
    /// damaged file off to re-download one they already have.
    #[test]
    fn absent_corrupt_and_empty_are_distinguishable_states() {
        let dir = std::env::temp_dir().join(format!("baseline_states_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");

        // Absent.
        let missing = dir.join("nope.csv");
        let _ = std::fs::remove_file(&missing);
        match load_baseline_curve(&missing) {
            PanelState::Error(msg) => assert_eq!(msg.as_str(), BASELINE_DATA_UNAVAILABLE),
            other => panic!("absent → unavailable, got {}", other.variant_name()),
        }

        // Present but truncated to nothing.
        let truncated = dir.join("truncated.csv");
        std::fs::write(&truncated, "").expect("write");
        match load_baseline_curve(&truncated) {
            PanelState::Error(msg) => assert_eq!(
                msg.as_str(),
                BASELINE_DATA_CORRUPT,
                "a 0-byte file is damaged, not absent and not empty"
            ),
            other => panic!("truncated → corrupt, got {}", other.variant_name()),
        }

        // Present, valid header, zero rows → genuinely Empty.
        let empty = dir.join("empty.csv");
        std::fs::write(&empty, "bar_index,timestamp_utc,equity_usd\n").expect("write");
        assert!(matches!(load_baseline_curve(&empty), PanelState::Empty));

        // The two error copies must not be the same string.
        assert_ne!(BASELINE_DATA_UNAVAILABLE, BASELINE_DATA_CORRUPT);

        let _ = std::fs::remove_file(&truncated);
        let _ = std::fs::remove_file(&empty);
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
    /// (R1 / AC1).
    ///
    /// **No skip-if-absent** (story-2-18 review M-skips). Both CSVs are
    /// committed under `docs/runbooks/artifacts/`, so "the file is not there"
    /// is a defect this gate exists to catch — and the artifacts directory has
    /// already moved once (`spec/runbooks/…` → `docs/runbooks/…`), which a
    /// silent `continue` would have absorbed without a word.
    #[test]
    fn committed_csvs_load_to_ready_first_point_100k() {
        for year in BaselineYear::ALL {
            let path = baseline_csv_path(year);
            assert!(
                path.exists(),
                "committed BH curve for {} is missing at {} — the artifacts \
                 directory moved or the file was pruned; this gate must fail, \
                 not skip",
                year.label(),
                path.display()
            );
            match load_baseline_curve(&path) {
                PanelState::Ready(series) => {
                    assert!(!series.points.is_empty(), "curve must have points");
                    assert_eq!(
                        series.points[0].equity.amount(),
                        dec!(100000.00),
                        "BH starts at $100,000.00"
                    );
                    // Drawdown band is computed for free (non-negative).
                    assert!(series.max_drawdown_frac >= Decimal::ZERO);
                }
                other => panic!(
                    "expected Ready for {}, got {}",
                    path.display(),
                    other.variant_name()
                ),
            }
        }
    }

    /// **The year toggle actually swaps the DATA** (story-2-18 review
    /// M-toggle). Nothing asserted that the curve loaded for a year covers
    /// that year, so swapping the two CSVs' contents — or wiring the screen to
    /// one file — left every test green while the screen showed 2023's line
    /// under 2024's cards.
    #[test]
    fn each_years_curve_spans_its_own_year() {
        for year in BaselineYear::ALL {
            let expected: i32 = year.label().parse().expect("year label is numeric");
            let PanelState::Ready(series) = load_baseline_curve(&baseline_csv_path(year)) else {
                panic!("committed curve for {} must load Ready", year.label());
            };
            for p in &series.points {
                assert_eq!(
                    p.ts.inner().year(),
                    expected,
                    "{} curve carries a point stamped {} — the CSVs are \
                     crossed or the loader read the wrong file",
                    year.label(),
                    p.ts.inner()
                );
            }
            // …and it is a FULL year of daily samples, not a stub.
            assert!(
                series.points.len() > 360,
                "{} curve has only {} points",
                year.label(),
                series.points.len()
            );
        }
    }

    // ── D1 re-sync trip: embedded scalars == the DOCUMENT's §7.1 ─────────────

    /// Path of the characterization the embedded scalars mirror.
    fn characterization_path() -> PathBuf {
        workspace_root()
            .join("docs/runbooks/artifacts/passive-baseline-2026-06-08")
            .join("passive-baseline-characterization.md")
    }

    /// One parsed §7.1 row: `Sharpe | Sortino | Calmar | MaxDD% |
    /// TotalReturn%` for a year.
    #[derive(Debug)]
    struct Section71Row {
        sharpe: Decimal,
        sortino: Decimal,
        calmar: Decimal,
        max_dd_pct: Decimal,
        total_return_pct: Decimal,
    }

    /// Strip the markdown table's decoration from one cell: `+1.8417` →
    /// `1.8417`, `34.57%` → `34.57`, `**+1.8417**` → `1.8417`.
    fn cell_to_decimal(raw: &str) -> Decimal {
        let cleaned: String = raw
            .trim()
            .trim_matches('*')
            .trim()
            .trim_end_matches('%')
            .trim_start_matches('+')
            .replace(',', "");
        cleaned
            .parse::<Decimal>()
            .unwrap_or_else(|e| panic!("§7.1 cell {raw:?} is not a number: {e}"))
    }

    /// Parse the §7.1 metrics row for `year` out of the characterization
    /// markdown. Panics with a pointed message when the table shape moved —
    /// which is itself the signal that the document was re-run/reformatted and
    /// the embedded constants need a human.
    fn parse_section_7_1(md: &str, year: BaselineYear) -> Section71Row {
        let table = md
            .split("### 7.1")
            .nth(1)
            .expect("characterization must contain a '### 7.1' section");
        let prefix = format!("| {} |", year.label());
        let line = table
            .lines()
            .find(|l| l.trim_start().starts_with(&prefix))
            .unwrap_or_else(|| {
                panic!(
                    "§7.1 has no row for {} — the table shape changed; \
                     re-sync `baseline_metrics` by hand",
                    year.label()
                )
            });
        // | Year | Sharpe | Sortino | Calmar | MaxDD% | TotalReturn% | …
        let cells: Vec<&str> = line.trim().trim_matches('|').split('|').collect();
        assert!(
            cells.len() >= 6,
            "§7.1 row for {} has {} cells, expected ≥6: {line}",
            year.label(),
            cells.len()
        );
        Section71Row {
            sharpe: cell_to_decimal(cells[1]),
            sortino: cell_to_decimal(cells[2]),
            calmar: cell_to_decimal(cells[3]),
            max_dd_pct: cell_to_decimal(cells[4]),
            total_return_pct: cell_to_decimal(cells[5]),
        }
    }

    /// Parse the §7.3 reconciliation table's **bootstrap p50 Sharpe** for a
    /// year (`| 2023 | **+1.8417** | +1.7353 | …`).
    fn parse_bootstrap_p50(md: &str, year: BaselineYear) -> Decimal {
        let table = md
            .split("### 7.3")
            .nth(1)
            .expect("characterization must contain a '### 7.3' section");
        let prefix = format!("| {} |", year.label());
        let line = table
            .lines()
            .find(|l| l.trim_start().starts_with(&prefix))
            .unwrap_or_else(|| panic!("§7.3 has no row for {}", year.label()));
        let cells: Vec<&str> = line.trim().trim_matches('|').split('|').collect();
        assert!(cells.len() >= 3, "§7.3 row too short: {line}");
        cell_to_decimal(cells[2])
    }

    /// **RE-SYNC trip (D1 cost) — now against the DOCUMENT.**
    ///
    /// This test's whole purpose is to fire when the characterization is
    /// re-run with different numbers. Until the story-2-18 review it compared
    /// the embedded constants against literals typed **into this same file**,
    /// so both sides were authored here: re-run the characterization, publish
    /// new numbers, and the "re-sync trigger" stayed green forever. It now
    /// reads `passive-baseline-characterization.md` and parses §7.1 (+ §7.3
    /// for the bootstrap p50), which is the authority the constants claim to
    /// mirror — bug-log #77's rule applied to a document instead of a
    /// snapshot: the expected value must come from somewhere the
    /// implementation cannot reach.
    ///
    /// Markdown parsing is brittle by nature; the failure modes are handled by
    /// making every structural surprise a LOUD panic naming what moved, which
    /// is the correct outcome — a reformatted §7.1 is exactly when a human
    /// should re-read these constants.
    #[test]
    fn baseline_metrics_match_characterization() {
        let path = characterization_path();
        assert!(
            path.exists(),
            "the characterization the embedded scalars mirror is missing at {} \
             — the re-sync trigger cannot fire without it",
            path.display()
        );
        let md = std::fs::read_to_string(&path).expect("characterization is readable");

        for year in BaselineYear::ALL {
            let row = parse_section_7_1(&md, year);
            let m = baseline_metrics(year);
            let facts = baseline_risk_facts(year);
            let y = year.label();

            assert_eq!(m.sharpe, row.sharpe, "{y} Sharpe vs §7.1");
            assert_eq!(m.max_drawdown_pct, row.max_dd_pct, "{y} MaxDD% vs §7.1");
            assert_eq!(
                m.total_return_pct, row.total_return_pct,
                "{y} TotalReturn% vs §7.1"
            );
            // CAGR is DERIVED, not published: for a single full-year hold the
            // annualized rate equals the total return (the period IS one year).
            assert_eq!(m.cagr_pct, row.total_return_pct, "{y} CAGR = TotalReturn%");
            assert_eq!(facts.sortino, row.sortino, "{y} Sortino vs §7.1");
            assert_eq!(facts.calmar, row.calmar, "{y} Calmar vs §7.1");
            assert_eq!(
                facts.bootstrap_p50_sharpe,
                parse_bootstrap_p50(&md, year),
                "{y} bootstrap p50 Sharpe vs §7.3"
            );

            assert!(m.cagr_present);
            assert!(m.sharpe_present);
            assert!(!m.win_rate_present, "win rate not meaningful for BH");
            assert_eq!(m.trades, 0, "buy-once-hold");
        }
    }

    /// The parser is not vacuous: it really reads the document, and a changed
    /// document really changes what it returns. (Without this, a parser that
    /// silently returned the constants would satisfy the test above.)
    #[test]
    fn section_7_1_parser_reads_the_document_it_is_given() {
        let md = "### 7.1 Full Metrics Table\n\
                  | Year | Sharpe | Sortino | Calmar | MaxDD% | TotalReturn% |\n\
                  |---|---|---|---|---|---|\n\
                  | 2023 | +9.9999 | +8.8888 | +7.777 | 12.34% | +56.78% |\n";
        let row = parse_section_7_1(md, BaselineYear::Y2023);
        assert_eq!(row.sharpe, dec!(9.9999));
        assert_eq!(row.sortino, dec!(8.8888));
        assert_eq!(row.calmar, dec!(7.777));
        assert_eq!(row.max_dd_pct, dec!(12.34));
        assert_eq!(row.total_return_pct, dec!(56.78));

        let recon = "### 7.3 Bootstrap Reconciliation\n\
                     | Year | Realized Sharpe | Bootstrap p50 Sharpe | Gap |\n\
                     | 2024 | **+0.8925** | +4.2000 | -0.2 |\n";
        assert_eq!(parse_bootstrap_p50(recon, BaselineYear::Y2024), dec!(4.2));
    }

    // ── review H2 — the card and the drawn line must not diverge silently ────

    /// **The card/curve binding.** The KPI card carries the §7.1 **hourly**
    /// max drawdown (34.57 / 48.95); the drawn curve is the **daily-sampled**
    /// CSV, which misses intraday extremes and yields 33.31 / 41.82 — a gap of
    /// up to 7.13 points between a number and the line directly beneath it,
    /// with nothing on screen saying why.
    ///
    /// The chosen fix is DISCLOSURE, not re-derivation: recomputing the card
    /// from the daily curve would put the cockpit at odds with §7.1 and with
    /// every artifact that cites it. This test is what stops the two drifting
    /// again — it pins both quantities and asserts the on-screen note names
    /// them BOTH, so regenerating the CSV without revisiting the constants
    /// fails here.
    ///
    /// Expected curve values derived independently of this crate (an
    /// arbitrary-precision peak/trough walk over the committed CSVs), not read
    /// off the implementation.
    #[test]
    fn card_and_curve_max_drawdown_are_bound_and_disclosed() {
        // (year, hourly card %, daily curve %, gap in points)
        for (year, card_pct, curve_pct) in [
            (BaselineYear::Y2023, dec!(34.57), dec!(33.31)),
            (BaselineYear::Y2024, dec!(48.95), dec!(41.82)),
        ] {
            let PanelState::Ready(series) = load_baseline_curve(&baseline_csv_path(year)) else {
                panic!("committed curve for {} must load Ready", year.label());
            };

            assert_eq!(
                baseline_metrics(year).max_drawdown_pct,
                card_pct,
                "{} KPI card holds the §7.1 hourly max drawdown",
                year.label()
            );
            assert_eq!(
                curve_max_drawdown_pct(&series),
                curve_pct,
                "{} drawn curve's max drawdown moved — if the CSV was \
                 regenerated, the card's §7.1 value and this screen's \
                 disclosure both need re-checking",
                year.label()
            );

            // The disclosure must carry BOTH numbers, in percent.
            let note = baseline_sampling_note(year, &series);
            assert!(
                note.contains(&format!("{}%", card_pct.normalize())),
                "sampling note must name the card figure {card_pct}: {note}"
            );
            assert!(
                note.contains(&format!("{}%", curve_pct.normalize())),
                "sampling note must name the drawn-curve figure {curve_pct}: {note}"
            );
            assert!(
                note.contains(&series.points.len().to_string()),
                "sampling note must say how many rows the drawn curve has: {note}"
            );
            // …and it must say WHY they differ.
            let lower = note.to_lowercase();
            assert!(
                lower.contains("hourly") && lower.contains("daily"),
                "sampling note must name both samplings: {note}"
            );

            // The gap is real and material — this is not a rounding artifact.
            let gap = card_pct - curve_pct;
            assert!(
                gap > Decimal::ONE,
                "{} gap {gap} should exceed 1 point (the reason the note \
                 exists); if it collapsed, the samplings converged and the \
                 copy should be revisited",
                year.label()
            );
        }
    }

    // ── review H3 — the Sharpe on screen is NAMED, and reconciled ────────────

    /// The realized single-path Sharpe and the published bootstrap p50 differ
    /// by ~6 % (2023, realized above) and ~19 % (2024, realized below). The
    /// card shows the realized figure — correctly, since the panel draws the
    /// realized path — so the screen must SAY so and cite the other.
    ///
    /// Literals below come from §7.1 / §7.3 (and the PRD/README headline
    /// +1.74 / +1.10), not from the implementation.
    #[test]
    fn sharpe_note_names_realized_and_cites_bootstrap_p50() {
        let n23 = baseline_sharpe_note(BaselineYear::Y2023);
        assert!(n23.contains("1.84"), "2023 realized Sharpe 1.8417: {n23}");
        assert!(n23.contains("1.74"), "2023 bootstrap p50 1.7353: {n23}");
        assert!(n23.contains("2023"), "names its year: {n23}");

        let n24 = baseline_sharpe_note(BaselineYear::Y2024);
        assert!(n24.contains("0.89"), "2024 realized Sharpe 0.8925: {n24}");
        assert!(n24.contains("1.10"), "2024 bootstrap p50 1.1047: {n24}");

        // The vocabulary the operator needs — none of these words appeared
        // anywhere in the UI before this patch.
        for word in ["realized", "single-path", "bootstrap", "median", "p50"] {
            let lower = n24.to_lowercase();
            assert!(lower.contains(word), "note must contain {word:?}: {n24}");
        }
    }

    /// The Sortino / Calmar line follows the ACTIVE year (review M-static): it
    /// used to be one static string printing both years at once, so half of it
    /// always described the year the operator had toggled away from.
    ///
    /// §7.1: 2023 Sortino +2.5126 / Calmar +5.677; 2024 +1.2047 / +1.853.
    #[test]
    fn risk_detail_is_year_specific() {
        let d23 = baseline_risk_detail(BaselineYear::Y2023);
        assert_eq!(d23, "Sortino 2.51 / Calmar 5.68 (2023)");
        let d24 = baseline_risk_detail(BaselineYear::Y2024);
        assert_eq!(d24, "Sortino 1.20 / Calmar 1.85 (2024)");
        assert!(
            !d23.contains("2024") && !d24.contains("2023"),
            "each year's line must describe only that year"
        );
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
