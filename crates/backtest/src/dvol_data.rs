//! DVOL implied-vol data source for the `v0.dvol_regime` bake-off arm.
//!
//! Mirrors `basis_data.rs` (`BasisDataSource`) but for the Deribit DVOL
//! parquets under `data/deribit-dvol/`. Schema: per-symbol/per-year parquets
//! with columns `day_open_ts_ms` Int64, `day_close_ts_ms` Int64,
//! `dvol_open/high/low/close` Float64 — the signal consumes `dvol_close` ONLY.
//!
//! # Design decisions
//!
//! - `dvol_close` is Float64 on disk (annualized-vol points, e.g. 52.4), parsed
//!   to `rust_decimal::Decimal` at the seam — identical to how `basis_close`
//!   Float64 → `Decimal` in `basis_data.rs:load`. DVOL is dimensionless and
//!   never enters a money/P&L computation (ADR-0003 money-math rule untouched).
//! - REVISION.toml aggregate SHA is verified against the expected constant
//!   `EXPECTED_DVOL_REVISION_SHA` (D-BR.3 pattern, basis_data.rs:45).
//!   The loader **refuses to run on unverified data**.
//! - `dvol_as_of` is a pure function (M-DEV-2): given a sorted DVOL series
//!   (by `day_close_ts_ms`) and bar open-timestamps, it finds the most-recent
//!   DVOL daily close at-or-before each bar. Returns `None` for bars before
//!   the first available DVOL close (warm-up). Routes through
//!   `trading_core::pit::PitSeries` (ADR-0058 / M-DEV-3) — look-ahead impossible.
//! - The as-of KEY is `day_close_ts_ms` (the instant the daily close is FULLY
//!   observed = `day_open_ts_ms + 86_400_000 - 1`). An hourly bar opening at `t`
//!   sees ONLY the most-recent DVOL close with `day_close_ts_ms ≤ t`.
//!
//! # Feature gate
//!
//! Compiled only when `--features realdata` (which pulls in polars).

use std::path::PathBuf;

use rust_decimal::Decimal;
use time::OffsetDateTime;
use trading_core::{PitSeries, Symbol, TimestampMs};

// ── Expected revision SHA ──────────────────────────────────────────────────────

/// Locked aggregate SHA-256 for `data/deribit-dvol/REVISION.toml`.
///
/// Pinned to the real corpus aggregate after running
/// `cargo run -p data --bin fetch_deribit_dvol -- --emit-revision-manifest`.
/// Any mismatch → `DvolDataError::RevisionMismatch`.
///
/// To bypass during development (no real data yet), tests use synthetic
/// `dvol_as_of` directly without going through `DvolDataSource::load`.
///
/// **2026-07-10 (ADR-0084 T6 back-fill):** re-pinned after additively
/// fetching 2021/2022/2025/2026 BTC+ETH DVOL (was 2023/2024 only, 4 files;
/// now 12 files). The 4 pre-existing 2023/2024 parquets are byte-identical
/// (verified via `shasum -a 256 -c` against a pre-fetch snapshot) — only
/// this manifest-aggregate constant moves, because the aggregate SHA is
/// computed over the WHOLE `[files]` map (`compute_aggregate_sha`), not a
/// per-span subset; adding new file ROWS necessarily changes it even though
/// no existing row's value changed. This constant is NOT one of the 9
/// `evidence/anchors.toml` regression anchors (a fully separate pin) — updating
/// it does not touch `verify_anchors.sh`'s 119/119 gate.
pub const EXPECTED_DVOL_REVISION_SHA: &str =
    "b21dc8691c257731d9043fc3e19b858c326ab4dd3d975f10de0eccf90cf480ff";

// ── Error type ─────────────────────────────────────────────────────────────────

#[derive(thiserror::Error, Debug)]
pub enum DvolDataError {
    #[error("data/deribit-dvol/REVISION.toml not found at {path}")]
    RevisionMissing { path: String },

    #[error("REVISION.toml parse error: {0}")]
    RevisionParse(String),

    /// On-disk aggregate SHA does not match the locked expected SHA.
    ///
    /// The DVOL backtest must reject runs on unverified data.
    #[error(
        "DVOL data revision mismatch: expected={expected}, recomputed={recomputed} \
         (file {file}: manifest={manifest_sha}, on-disk={actual_sha})"
    )]
    RevisionMismatch {
        file: String,
        manifest_sha: String,
        actual_sha: String,
        expected: String,
        recomputed: String,
    },

    #[error("DVOL parquet read error for {path}: {source}")]
    Parquet {
        path: String,
        source: polars::prelude::PolarsError,
    },

    #[error("dvol_close parse error in {path} row {row}: {value}: {source}")]
    DecimalParse {
        path: String,
        row: usize,
        value: String,
        source: rust_decimal::Error,
    },

    /// A row carried a NULL timestamp column (review 3-15 HIGH).
    ///
    /// This used to be swallowed: `close_ts_col.get(i).unwrap_or(0)` mapped a
    /// NULL to epoch-0, which the span filter below then dropped with no
    /// diagnostic anywhere — a silently shrinking corpus, the same shape as the
    /// R3 coverage-gate bug (bug-log #70). `basis_data.rs` fixed the identical
    /// defect in review 1-20 (`BasisDataError::NullOpenTime`); this is the
    /// mirror for both DVOL timestamp columns.
    #[error(
        "DVOL parquet {path} row {row}: {column} is NULL — refusing to guess a timestamp \
         (a NULL here silently becomes epoch-0 and the row vanishes past the span filter)"
    )]
    NullTimestamp {
        path: String,
        row: usize,
        column: &'static str,
    },

    /// A row carried a NULL `dvol_close` (review 3-15 HIGH).
    ///
    /// **Worse here than in `basis_data.rs`**: basis's value column is a Utf8
    /// string, so a NULL yields `""` and blows up in `Decimal::from_str`. DVOL's
    /// value column is Float64, so `unwrap_or(0.0)` produced a *plausible*
    /// number — a DVOL of 0.0 is below every median, so the day is scored CALM
    /// and, worse, the 0.0 enters the trailing-median ring and drags the whole
    /// 30-day cut down. Silent, directional corruption of the signal.
    #[error(
        "DVOL parquet {path} row {row}: dvol_close is NULL — refusing to coalesce to 0.0 \
         (0.0 is below every median: the day would score CALM and the 30-day median ring \
         would be dragged down)"
    )]
    NullDvolClose { path: String, row: usize },

    /// `day_close_ts_ms != day_open_ts_ms + 86_400_000 - 1` (review 3-15 MEDIUM).
    ///
    /// The loader keys the as-of join on `day_close_ts_ms` and never looked at
    /// `day_open_ts_ms`, even though both are on disk — the same
    /// present-and-ignored pattern that made ADR-0086's Basis publication-lag
    /// row wrong. A regenerated parquet written under a different keying
    /// convention (e.g. keyed at the day OPEN, or at `open + 86_400_000`) would
    /// be accepted silently and shift every decision by up to a day.
    #[error(
        "DVOL parquet {path} row {row}: keying invariant violated — \
         day_close_ts_ms={day_close_ts_ms} but day_open_ts_ms={day_open_ts_ms} implies \
         {expected} (= day_open_ts_ms + 86_400_000 - 1). The as-of join keys on \
         day_close_ts_ms and assumes it is the FULLY-observed instant of that day; a \
         corpus written under a different convention would silently shift every \
         regime decision."
    )]
    KeyingInvariant {
        path: String,
        row: usize,
        day_open_ts_ms: i64,
        day_close_ts_ms: i64,
        expected: i64,
    },

    /// A symbol's row count for the span is below the coverage floor
    /// (review 3-15 HIGH — the third sibling; basis got one 2026-08-11,
    /// funding 2026-08-12).
    ///
    /// The failure this catches: the loader did manifest → per-file SHA →
    /// aggregate SHA → parse → span filter → sort and returned whatever
    /// survived. A corpus that lost 95% of its rows loaded clean, and the only
    /// downstream check was `is_empty()`, which a ONE-row corpus passes. A
    /// thinned DVOL series does not fail loudly — it produces a stale
    /// forward-filled median (the ring only advances when the value *changes*),
    /// so the regime decision quietly freezes while every integrity signal
    /// reports healthy. That is precisely bug-log #78's second trigger.
    #[error(
        "DVOL coverage below floor for {symbol} in [{span_start}..{span_end}): got {actual} \
         daily rows, expected ~{expected} ({pct:.2}% present, floor {floor_pct:.2}%). The \
         30-day median ring only advances when the as-of close CHANGES, so a thinned \
         corpus silently freezes the regime cut instead of failing — refusing to run."
    )]
    InsufficientCoverage {
        symbol: String,
        expected: usize,
        actual: usize,
        /// Percentage present — human-readable only; the decision uses integer
        /// arithmetic (see `load()`).
        #[allow(clippy::float_arithmetic)]
        pct: f64,
        #[allow(clippy::float_arithmetic)]
        floor_pct: f64,
        span_start: String,
        span_end: String,
    },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

// ── Coverage floor + cadence constants ────────────────────────────────────────

/// Minimum fraction of the span's DAILY rows each symbol must supply, in
/// per-mille (review 3-15 HIGH).
///
/// Mirrors `basis_data::MIN_SYMBOL_COVERAGE_PERMILLE` and the OHLCV loader's R3
/// tolerance (`realdata.rs`, 995‰ = 99.5%), which review 1-18 hardened after
/// bug-log #70. Integer per-mille so the comparison never touches a float.
pub const MIN_SYMBOL_COVERAGE_PERMILLE: usize = 995;

/// One UTC day in milliseconds — the DVOL corpus cadence (one row per day),
/// which is what makes the expected row count trivially derivable.
pub const ONE_DAY_MS: i64 = 86_400_000;

// ── DvolRow ───────────────────────────────────────────────────────────────────

/// A single DVOL daily row loaded from the parquet.
///
/// Tuple key: `(symbol, day_close_ts_ms, dvol_close)`.
/// `day_close_ts_ms = day_open_ts_ms + 86_400_000 - 1` — the instant the
/// daily close is FULLY observed (the as-of join key).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DvolRow {
    pub symbol: Symbol,
    /// `day_open_ts_ms + 86_400_000 - 1` — the as-of join key.
    pub day_close_ts_ms: i64,
    /// DVOL daily close (annualized-vol points, e.g. `52.4`).
    /// Parsed as exact `Decimal` — no f64 round-trip (ADR-0003).
    pub dvol_close: Decimal,
}

// ── LoadedDvol ────────────────────────────────────────────────────────────────

/// Result of a successful DVOL load.
#[derive(Debug)]
pub struct LoadedDvol {
    /// All DVOL rows across all symbols, sorted `(day_close_ts_ms ASC, symbol ASC)`.
    pub rows: Vec<DvolRow>,
    /// Recomputed aggregate SHA-256 (64 hex chars).
    pub revision_sha: String,
}

// ── DvolDataSource ─────────────────────────────────────────────────────────────

/// Deribit DVOL parquet loader, mirroring `BasisDataSource`.
pub struct DvolDataSource {
    /// Root of the DVOL parquet directory, e.g. `data/deribit-dvol`.
    dvol_root: PathBuf,
    /// Universe of symbols to load. Only `Symbol::new("BTC")` and
    /// `Symbol::new("ETH")` are supported (DVOL exists only for BTC+ETH).
    universe: Vec<Symbol>,
    /// Aggregate SHA the corpus must recompute to. Always
    /// [`EXPECTED_DVOL_REVISION_SHA`] in production — [`Self::new`] is the only
    /// constructor outside `cfg(test)`, so the revision lock is unconditional.
    expected_revision_sha: String,
}

impl DvolDataSource {
    /// Create a new DVOL data source.
    ///
    /// `dvol_root` must contain both the symbol subdirectories and
    /// `REVISION.toml` (same layout as `data/deribit-dvol/`).
    #[must_use]
    pub fn new(dvol_root: PathBuf, universe: Vec<Symbol>) -> Self {
        Self {
            dvol_root,
            universe,
            expected_revision_sha: EXPECTED_DVOL_REVISION_SHA.to_string(),
        }
    }

    /// Point the revision lock at a different aggregate SHA — **tests only**.
    ///
    /// Compiled ONLY under `cfg(test)`, so it does not exist in any shipped
    /// build. Copied verbatim in intent from `basis_data.rs`: without it, step 3
    /// rejects every temp-dir fixture before the parquet is ever opened, which
    /// is why the only DVOL test that exercised the column names and the
    /// symbol-from-relpath derivation had to be `#[ignore]`d against the real
    /// corpus — and therefore gated nothing (review 3-15 MEDIUM, skip-visibility).
    #[cfg(test)]
    fn with_expected_revision_sha(mut self, sha: &str) -> Self {
        self.expected_revision_sha = sha.to_string();
        self
    }

    /// Load + REVISION-verify + parse DVOL rows for the given span.
    ///
    /// Steps:
    /// 1. Check `REVISION.toml` exists → `RevisionMissing`.
    /// 2. For every parquet file in the span, verify on-disk SHA against
    ///    manifest → `RevisionMismatch`.
    /// 3. Recompute aggregate SHA and verify it equals
    ///    `EXPECTED_DVOL_REVISION_SHA`.
    /// 4. Read parquet files; parse `dvol_close` as `Decimal`. A NULL timestamp
    ///    or a NULL `dvol_close` is an ERROR, never a coalesced 0 / 0.0
    ///    (review 3-15 HIGH). The keying invariant
    ///    `day_close_ts_ms == day_open_ts_ms + 86_400_000 - 1` is asserted per
    ///    row (review 3-15 MEDIUM).
    /// 5. Filter rows to `[span.start_ms, span.end_ms)` by `day_close_ts_ms`.
    /// 6. Enforce the PER-SYMBOL daily coverage floor
    ///    ([`MIN_SYMBOL_COVERAGE_PERMILLE`]) → `InsufficientCoverage`
    ///    (review 3-15 HIGH).
    /// 7. Sort `(day_close_ts_ms ASC, symbol ASC)`.
    ///
    /// # Errors
    ///
    /// Returns `DvolDataError` on manifest missing / SHA mismatch, parquet read
    /// errors, a NULL timestamp or value cell, a violated keying invariant, a
    /// Decimal parse failure, or a symbol whose daily row count for the span is
    /// below the coverage floor.
    #[allow(clippy::too_many_lines)]
    pub fn load(
        &self,
        span: &crate::realdata::TimeSpan,
        _scenario_name: &str,
    ) -> Result<LoadedDvol, DvolDataError> {
        use polars::prelude::{LazyFrame, ScanArgsParquet};

        // Step 1: manifest exists?
        let manifest_path = self.dvol_root.join("REVISION.toml");
        if !manifest_path.exists() {
            return Err(DvolDataError::RevisionMissing {
                path: manifest_path.to_string_lossy().into_owned(),
            });
        }

        // Read manifest (no on-disk SHA check yet).
        let (files_map, _claimed_aggregate) = data::revision::read_manifest_raw(&self.dvol_root)
            .map_err(|e| DvolDataError::RevisionParse(e.to_string()))?;

        // Step 2: verify each parquet file in the span.
        let scenario_files = self.files_for_span(span);
        for relpath in &scenario_files {
            let manifest_sha =
                files_map
                    .get(relpath)
                    .ok_or_else(|| DvolDataError::RevisionMismatch {
                        file: relpath.clone(),
                        manifest_sha: "(not in manifest)".to_string(),
                        actual_sha: "n/a".to_string(),
                        expected: self.expected_revision_sha.clone(),
                        recomputed: "(not computed)".to_string(),
                    })?;
            let abs_path = self.dvol_root.join(relpath);
            let actual_sha = data::revision::file_sha256(&abs_path)
                .map_err(|e| DvolDataError::RevisionParse(format!("sha256 read error: {e}")))?;
            if &actual_sha != manifest_sha {
                return Err(DvolDataError::RevisionMismatch {
                    file: relpath.clone(),
                    manifest_sha: manifest_sha.clone(),
                    actual_sha,
                    expected: self.expected_revision_sha.clone(),
                    recomputed: "(not computed)".to_string(),
                });
            }
        }

        // Step 3: recompute aggregate SHA and verify against the locked constant.
        let recomputed = data::revision::compute_aggregate_sha(&files_map);
        if recomputed != self.expected_revision_sha {
            return Err(DvolDataError::RevisionMismatch {
                file: "(aggregate)".to_string(),
                manifest_sha: "(n/a)".to_string(),
                actual_sha: "(n/a)".to_string(),
                expected: self.expected_revision_sha.clone(),
                recomputed: recomputed.clone(),
            });
        }

        // Step 4: read + parse parquet files.
        let mut rows: Vec<DvolRow> = Vec::new();
        for relpath in &scenario_files {
            let abs_path = self.dvol_root.join(relpath);
            let path_str = abs_path.to_string_lossy().into_owned();

            let df = LazyFrame::scan_parquet(&abs_path, ScanArgsParquet::default())
                .and_then(polars::prelude::LazyFrame::collect)
                .map_err(|e| DvolDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?;

            // The symbol is encoded in the file path: <SYM>/<YEAR>.parquet
            let sym_str = relpath.split('/').next().unwrap_or("");

            // Review 3-15 MEDIUM: `day_open_ts_ms` used to be present on disk and
            // IGNORED — the same present-and-ignored pattern that made ADR-0086's
            // Basis publication-lag row wrong. It is now READ and used to assert
            // the keying invariant the as-of join depends on.
            let open_ts_col = df
                .column("day_open_ts_ms")
                .map_err(|e| DvolDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?
                .i64()
                .map_err(|e| DvolDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?;

            let close_ts_col = df
                .column("day_close_ts_ms")
                .map_err(|e| DvolDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?
                .i64()
                .map_err(|e| DvolDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?;

            let dvol_close_col = df
                .column("dvol_close")
                .map_err(|e| DvolDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?
                .f64()
                .map_err(|e| DvolDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?;

            let n_rows = df.height();
            for i in 0..n_rows {
                // Review 3-15 HIGH: these three cells used to be
                // `close_ts_col.get(i).unwrap_or(0)` and
                // `dvol_close_col.get(i).unwrap_or(0.0)`. A NULL timestamp became
                // epoch-0 and the row silently vanished past the span filter; a
                // NULL value became a *plausible* DVOL of 0.0 — below every
                // median — so the day scored CALM and the median ring was dragged
                // down. Both are now loud.
                let day_open_ts_ms =
                    open_ts_col
                        .get(i)
                        .ok_or_else(|| DvolDataError::NullTimestamp {
                            path: path_str.clone(),
                            row: i,
                            column: "day_open_ts_ms",
                        })?;
                let day_close_ts_ms =
                    close_ts_col
                        .get(i)
                        .ok_or_else(|| DvolDataError::NullTimestamp {
                            path: path_str.clone(),
                            row: i,
                            column: "day_close_ts_ms",
                        })?;
                let dvol_f64 =
                    dvol_close_col
                        .get(i)
                        .ok_or_else(|| DvolDataError::NullDvolClose {
                            path: path_str.clone(),
                            row: i,
                        })?;

                // Keying invariant (review 3-15 MEDIUM): the as-of key must be the
                // FULLY-observed instant of the same day the row describes. Checked
                // BEFORE the span filter so a mis-keyed corpus cannot hide by
                // falling outside the window.
                let expected_close = day_open_ts_ms + ONE_DAY_MS - 1;
                if day_close_ts_ms != expected_close {
                    return Err(DvolDataError::KeyingInvariant {
                        path: path_str.clone(),
                        row: i,
                        day_open_ts_ms,
                        day_close_ts_ms,
                        expected: expected_close,
                    });
                }

                // Step 5: filter to span [start_ms, end_ms) by day_close_ts_ms.
                if day_close_ts_ms < span.start_ms || day_close_ts_ms >= span.end_ms {
                    continue;
                }

                // Convert f64 DVOL close to Decimal.
                // DVOL is dimensionless (annualized-vol points like 52.4);
                // use to_string → from_str to avoid lossy f64 Decimal conversion.
                let dvol_str = format!("{dvol_f64:.4}");
                let dvol_close =
                    dvol_str
                        .parse::<Decimal>()
                        .map_err(|e| DvolDataError::DecimalParse {
                            path: path_str.clone(),
                            row: i,
                            value: dvol_str.clone(),
                            source: e,
                        })?;

                rows.push(DvolRow {
                    symbol: Symbol::new(sym_str),
                    day_close_ts_ms,
                    dvol_close,
                });
            }
        }

        // Step 6: PER-SYMBOL daily coverage gate (review 3-15 HIGH).
        //
        // The third sibling: `basis_data.rs` got one on 2026-08-11 and
        // `funding_data.rs` on 2026-08-12; DVOL had none, so a corpus that lost
        // 95% of its rows loaded clean and the only downstream check was
        // `is_empty()` — which a ONE-row corpus passes. The DVOL cadence is
        // DAILY, so the expected count is simply the span's length in days.
        //
        // Integer arithmetic only in the comparison — the float is for the message.
        let expected_per_symbol =
            usize::try_from((span.end_ms - span.start_ms) / ONE_DAY_MS).unwrap_or(0);
        if expected_per_symbol > 0 {
            let threshold = (expected_per_symbol * MIN_SYMBOL_COVERAGE_PERMILLE).div_ceil(1000);
            for sym in &self.universe {
                let actual = rows.iter().filter(|r| &r.symbol == sym).count();
                if actual < threshold {
                    #[allow(clippy::cast_precision_loss, clippy::float_arithmetic)]
                    let pct = actual as f64 / expected_per_symbol as f64 * 100.0;
                    #[allow(clippy::cast_precision_loss, clippy::float_arithmetic)]
                    let floor_pct = MIN_SYMBOL_COVERAGE_PERMILLE as f64 / 10.0;
                    return Err(DvolDataError::InsufficientCoverage {
                        symbol: sym.0.to_string(),
                        expected: expected_per_symbol,
                        actual,
                        pct,
                        floor_pct,
                        // Formatted from the numeric bounds, NOT from the
                        // `&'static str` labels — the bake-off path passes static
                        // placeholder labels (review 3-15 LOW removed the
                        // per-call `Box::leak`).
                        span_start: span.start_ms.to_string(),
                        span_end: span.end_ms.to_string(),
                    });
                }
            }
        }

        // Step 7: sort (day_close_ts_ms ASC, symbol ASC).
        rows.sort_unstable_by(|a, b| {
            a.day_close_ts_ms
                .cmp(&b.day_close_ts_ms)
                .then_with(|| a.symbol.0.as_str().cmp(b.symbol.0.as_str()))
        });

        Ok(LoadedDvol {
            rows,
            revision_sha: recomputed,
        })
    }

    /// Return relative parquet paths for the scenario span.
    ///
    /// Layout: `<SYM>/<YEAR>.parquet` — one file per symbol per year.
    #[must_use]
    pub fn files_for_span(&self, span: &crate::realdata::TimeSpan) -> Vec<String> {
        let start_dt = OffsetDateTime::from_unix_timestamp(span.start_ms / 1_000)
            .unwrap_or(OffsetDateTime::UNIX_EPOCH);
        let end_dt = OffsetDateTime::from_unix_timestamp(span.end_ms / 1_000)
            .unwrap_or(OffsetDateTime::UNIX_EPOCH);

        let start_year = start_dt.year();
        let end_year = end_dt.year();

        let mut result = Vec::new();
        for sym in &self.universe {
            for year in start_year..=end_year {
                result.push(format!("{}/{year}.parquet", sym.0.as_str()));
            }
        }
        result
    }
}

// ── M-DEV-2: as-of join ───────────────────────────────────────────────────────

/// Find the DVOL daily close at each bar open timestamp using an as-of join.
///
/// For each bar open timestamp `bar_open_ts_ms[i]`, finds the `dvol_close`
/// from the most-recent DVOL day whose `day_close_ts_ms` is **at-or-before**
/// `bar_open_ts_ms[i]`.
///
/// The as-of KEY is `day_close_ts_ms` (NOT `day_open_ts_ms`). The daily close
/// is FULLY observed only when the day ends (`day_close_ts_ms = midnight + 86_400_000 - 1 ms`).
/// An hourly bar opening at 2023-05-02T00:00Z therefore sees the 2023-05-01
/// DVOL close (`close_ts` = 2023-05-01T23:59:59.999Z ≤ `bar_open_ts`), NOT the
/// 05-02 close (which closes 24h later). This matches the `basis_as_of`
/// discipline with the cadence lifted from 1h-basis to 1-day-DVOL.
///
/// Returns `None` for bars before the first available DVOL close (warm-up).
///
/// # Arguments
///
/// - `dvol`: Sorted `(day_close_ts_ms, dvol_close)` pairs in ascending
///   `day_close_ts_ms` order. MUST be sorted for correct results.
/// - `bar_open_ts_ms`: Bar open timestamps (each processed independently
///   via binary search on `PitSeries`).
///
/// # Returns
///
/// `Vec<Option<Decimal>>` of length `bar_open_ts_ms.len()`.
/// `None` = no DVOL day has closed yet (warm-up).
///
/// # Invariant (no look-ahead)
///
/// Only the DVOL close fully observed at-or-before the bar's `open_ts` is used.
/// Future-shifting the DVOL series (e.g. by +1 day) WILL produce a different
/// result — verified by the unit test `no_look_ahead_falsifier`.
///
/// Routes through `trading_core::pit::PitSeries` (ADR-0058 / M-DEV-3).
#[must_use]
pub fn dvol_as_of(dvol: &[(i64, Decimal)], bar_open_ts_ms: &[i64]) -> Vec<Option<Decimal>> {
    if dvol.is_empty() {
        return vec![None; bar_open_ts_ms.len()];
    }

    // Build a PitSeries once. The loader pre-sorts dvol by day_close_ts_ms
    // before calling this function, so from_sorted would succeed; we use
    // from_unsorted here so this library function is infallible — the sort
    // is a stable no-op on an already-sorted slice.
    //
    // Explicit publication lag (ADR-0086 D2/D3, P3 M-DEV-5): DVOL's
    // publication_lag_ms = 0 per the feature's lag table
    // (spec/v3/advisor-pit-discipline/feature.md § D2) — the join key
    // `day_close_ts_ms` already places the record at the FULLY-observed
    // instant, so no additional lag applies. `from_unsorted_with_lag(_, 0)`
    // is byte-identical to `from_unsorted(_)` (proven by
    // `dvol_byte_identical_legacy_vs_with_lag_zero`).
    let series = PitSeries::from_unsorted_with_lag(
        dvol.iter().map(|&(t, r)| (TimestampMs(t), r)).collect(),
        0,
    );

    bar_open_ts_ms
        .iter()
        .map(|&q| series.as_of_value(TimestampMs(q)))
        .collect()
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::float_arithmetic,
    clippy::pedantic,
    clippy::identity_op
)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    const ONE_DAY_MS: i64 = 86_400_000;

    // ── dvol_as_of unit tests (M-DEV-2) ──────────────────────────────────────

    /// Bar before the first DVOL close → None (warm-up).
    #[test]
    fn warm_up_before_first_dvol_is_none() {
        // The DVOL series uses day_close_ts_ms as the join key.
        // day_close_ts_ms = day_open_ts_ms + ONE_DAY_MS - 1.
        let day1_close_ts: i64 = ONE_DAY_MS - 1; // day 1 close
        let dvol = vec![
            (day1_close_ts, dec!(50.0)),
            (2 * ONE_DAY_MS - 1, dec!(55.0)),
        ];
        // Bar at ts=0 is before day1_close_ts → None (warm-up).
        let result = dvol_as_of(&dvol, &[0]);
        assert_eq!(
            result,
            vec![None],
            "bar before first DVOL close must be None"
        );
    }

    /// Bar at or after the first DVOL close timestamp → Some.
    #[test]
    fn bar_after_first_dvol_close_is_some() {
        let day1_close_ts: i64 = ONE_DAY_MS - 1;
        let dvol = vec![(day1_close_ts, dec!(50.0))];
        // A bar opening exactly at day1_close_ts sees that close.
        let result = dvol_as_of(&dvol, &[day1_close_ts]);
        assert_eq!(result, vec![Some(dec!(50.0))]);

        // A bar opening 1 ms after (e.g. UTC midnight of day 2) also sees day1_close.
        let result2 = dvol_as_of(&dvol, &[ONE_DAY_MS]);
        assert_eq!(result2, vec![Some(dec!(50.0))]);
    }

    /// A bar opening on day 2 sees the day-1 DVOL close, NOT day-2 close.
    ///
    /// This is the load-bearing no-look-ahead property:
    /// `bar_open = 2023-05-02T00:00Z` → sees the 2023-05-01 DVOL close
    /// (close_ts = 2023-05-01T23:59:59.999Z ≤ bar_open).
    #[test]
    fn bar_on_day2_sees_day1_close() {
        // day 1: close_ts = ONE_DAY_MS - 1 (e.g. 86_399_999 ms)
        // day 2: close_ts = 2 * ONE_DAY_MS - 1 (e.g. 172_799_999 ms)
        let day1_close_ts: i64 = ONE_DAY_MS - 1;
        let day2_close_ts: i64 = 2 * ONE_DAY_MS - 1;
        let dvol = vec![(day1_close_ts, dec!(50.0)), (day2_close_ts, dec!(60.0))];
        // Bar opening at day2's midnight (ONE_DAY_MS) → sees day1_close (50.0).
        let bar_ts = ONE_DAY_MS; // 2023-01-02T00:00Z
        let result = dvol_as_of(&dvol, &[bar_ts]);
        assert_eq!(result, vec![Some(dec!(50.0))]);

        // Bar opening at day2_close_ts (the very end of day 2) → sees day2_close (60.0).
        let result2 = dvol_as_of(&dvol, &[day2_close_ts]);
        assert_eq!(result2, vec![Some(dec!(60.0))]);
    }

    /// Forward-fill across multiple intraday bars.
    ///
    /// On the 1h grid, 24 consecutive hourly bars in a day all see the prior
    /// day's DVOL close (forward-filled until the current day's close lands).
    #[test]
    fn forward_fill_across_intraday_bars() {
        let one_hour_ms: i64 = 3_600_000;
        let day1_close_ts: i64 = ONE_DAY_MS - 1;
        let dvol = vec![(day1_close_ts, dec!(52.0))];

        // 25 hourly bars from day2's midnight through end-of-day2 (past day1_close).
        let bars: Vec<i64> = (0..25).map(|h| ONE_DAY_MS + h * one_hour_ms).collect();
        let result = dvol_as_of(&dvol, &bars);

        // All 25 bars see the day1 close (forward-filled).
        for (i, val) in result.iter().enumerate() {
            assert_eq!(
                *val,
                Some(dec!(52.0)),
                "bar {i} should see day1_close = 52.0"
            );
        }
    }

    /// Empty DVOL series → all None.
    #[test]
    fn empty_dvol_series_all_none() {
        let result = dvol_as_of(&[], &[100, 200, 300]);
        assert_eq!(result, vec![None, None, None]);
    }

    // ── Byte-identity test (ADR-0086 D3 / P3 M-TEST-3) ────────────────────────

    /// Proves the P3 retrofit (`PitSeries::from_unsorted` →
    /// `from_unsorted_with_lag(_, 0)`, `dvol_data.rs::dvol_as_of`) moves NO
    /// value on a representative series + bar-open grid: the LEGACY raw
    /// `partition_point(|&(t,_)| t <= q)` predicate computed directly over
    /// the DVOL rows must equal the RETROFITTED `dvol_as_of` (which now
    /// routes through `from_unsorted_with_lag(_, 0)`) element-for-element.
    ///
    /// This is the anchor question, answered by construction: because DVOL
    /// runs `write_report = false` on the bake-off path, no anchored report
    /// body can move; this test is the load-bearing proof that the
    /// as-of VALUES themselves are unchanged.
    #[test]
    fn dvol_byte_identical_legacy_vs_with_lag_zero() {
        let day1_close_ts: i64 = ONE_DAY_MS - 1;
        let day2_close_ts: i64 = 2 * ONE_DAY_MS - 1;
        let day3_close_ts: i64 = 3 * ONE_DAY_MS - 1;
        let dvol: Vec<(i64, Decimal)> = vec![
            (day1_close_ts, dec!(50.0)),
            (day2_close_ts, dec!(70.0)),
            (day3_close_ts, dec!(40.0)),
        ];

        // A representative bar-open grid spanning warm-up, exact boundaries,
        // between-record forward-fill, and past-last-record.
        let one_hour_ms: i64 = 3_600_000;
        let mut grid: Vec<i64> = vec![0, day1_close_ts - 1, day1_close_ts, day2_close_ts - 1];
        grid.extend((0..30).map(|h| day1_close_ts + h * one_hour_ms));
        grid.push(day2_close_ts);
        grid.push(day3_close_ts);
        grid.push(day3_close_ts + ONE_DAY_MS);

        // LEGACY: the exact raw predicate `dvol_as_of` used before the P3
        // retrofit (partition_point(|&(t,_)| t <= q), idx-1, None at idx==0).
        let legacy_as_of = |query: i64| -> Option<Decimal> {
            let idx = dvol.partition_point(|&(t, _)| t <= query); // PIT-OK: legacy-predicate byte-identity oracle for the M-TEST-3 retrofit proof.
            if idx == 0 {
                None
            } else {
                Some(dvol[idx - 1].1)
            }
        };
        let legacy_results: Vec<Option<Decimal>> = grid.iter().map(|&q| legacy_as_of(q)).collect();

        // RETROFITTED: the current dvol_as_of, which now routes through
        // PitSeries::from_unsorted_with_lag(_, 0).
        let retrofitted_results = dvol_as_of(&dvol, &grid);

        assert_eq!(
            legacy_results, retrofitted_results,
            "P3 retrofit must be byte-identical to the legacy raw partition_point predicate"
        );
    }

    /// No-look-ahead falsifier: future-shifting the DVOL series changes the result.
    ///
    /// Cloned verbatim from `basis_data.rs::no_look_ahead_falsifier`.
    /// Proves the join is strictly past-only — future DVOL never leaks.
    #[test]
    fn no_look_ahead_falsifier() {
        let day1_close_ts: i64 = ONE_DAY_MS - 1;
        let day2_close_ts: i64 = 2 * ONE_DAY_MS - 1;
        let day3_close_ts: i64 = 3 * ONE_DAY_MS - 1;
        let dvol = vec![
            (day1_close_ts, dec!(50.0)),
            (day2_close_ts, dec!(70.0)),
            (day3_close_ts, dec!(40.0)),
        ];

        // Shift each DVOL close timestamp +1 day into the future.
        let future_shifted: Vec<(i64, Decimal)> =
            dvol.iter().map(|&(t, r)| (t + ONE_DAY_MS, r)).collect();

        // Query a bar at day2's midnight (the open of day 2).
        let bar_ts = ONE_DAY_MS; // 2023-01-02T00:00Z

        let causal_result = dvol_as_of(&dvol, &[bar_ts]);
        let shifted_result = dvol_as_of(&future_shifted, &[bar_ts]);

        // Causal: bar at day2 open sees day1_close_ts (< day2 open) → 50.0.
        assert_eq!(causal_result, vec![Some(dec!(50.0))]);
        // Shifted: day1 close is now at 2*ONE_DAY_MS-1, day2 open = ONE_DAY_MS → None.
        assert_eq!(shifted_result, vec![None]);
        // They MUST differ — proves no look-ahead.
        assert_ne!(
            causal_result, shifted_result,
            "no-look-ahead falsifier: causal ≠ shifted (future DVOL must not leak)"
        );
    }

    /// Decimal precision is preserved — no f64 round-trip loses information.
    #[test]
    fn decimal_precision_preserved() {
        // DVOL values like 52.1234 should round-trip through the format!("{:.4}") path.
        let raw: f64 = 52.1234;
        let s = format!("{raw:.4}");
        let parsed: Decimal = s.parse().expect("parse");
        assert_eq!(parsed, dec!(52.1234));
    }

    // ── Review 3-15: END-TO-END loader acceptance (NOT skip-gated) ────────────
    //
    // Before this block every test that drove the production `load()` was
    // `#[ignore]`d against the real (gitignored) corpus, so NOTHING in CI pinned
    // the column names, the NULL handling, the keying invariant or the coverage
    // floor. These write a real (tiny) parquet to a temp dir and drive
    // `DvolDataSource::load` end to end, mirroring `basis_data.rs`'s 1-20 block.

    /// Write a DVOL parquet at `<root>/<sym>/<year>.parquet`.
    ///
    /// Columns are named EXACTLY as the production schema — that is part of what
    /// these tests pin. `rows` are `(day_open_ts_ms, dvol_close)`; the
    /// `day_close_ts_ms` key is derived so fixtures obey the keying invariant
    /// unless a test deliberately breaks it.
    fn write_dvol_parquet(
        root: &std::path::Path,
        sym: &str,
        year: i32,
        rows: &[(i64, f64)],
    ) -> String {
        write_dvol_parquet_raw(
            root,
            sym,
            year,
            &rows
                .iter()
                .map(|&(open, close)| (Some(open), Some(open + ONE_DAY_MS - 1), Some(close)))
                .collect::<Vec<_>>(),
        )
    }

    /// Nullable/free-form variant — lets a test inject NULL cells or a broken key.
    fn write_dvol_parquet_raw(
        root: &std::path::Path,
        sym: &str,
        year: i32,
        rows: &[(Option<i64>, Option<i64>, Option<f64>)],
    ) -> String {
        use polars::prelude::*;

        let dir = root.join(sym);
        std::fs::create_dir_all(&dir).expect("create symbol dir");
        let path = dir.join(format!("{year}.parquet"));

        let opens: Vec<Option<i64>> = rows.iter().map(|r| r.0).collect();
        let closes_ts: Vec<Option<i64>> = rows.iter().map(|r| r.1).collect();
        let values: Vec<Option<f64>> = rows.iter().map(|r| r.2).collect();
        let mut df = df![
            "day_open_ts_ms" => opens,
            "day_close_ts_ms" => closes_ts,
            "dvol_close" => values,
        ]
        .expect("build fixture DataFrame");

        let mut file = std::fs::File::create(&path).expect("create parquet");
        ParquetWriter::new(&mut file)
            .finish(&mut df)
            .expect("write parquet");

        format!("{sym}/{year}.parquet")
    }

    /// Write `REVISION.toml` for the given relpaths and return the aggregate SHA.
    fn write_manifest(root: &std::path::Path, relpaths: &[String]) -> String {
        use std::collections::BTreeMap;

        let mut files_map: BTreeMap<String, String> = BTreeMap::new();
        for rel in relpaths {
            let sha = data::revision::file_sha256(&root.join(rel)).expect("sha256");
            files_map.insert(rel.clone(), sha);
        }
        let aggregate = data::revision::compute_aggregate_sha(&files_map);

        let mut toml = format!("[revision]\nsha256 = \"{aggregate}\"\n\n[files]\n");
        for (rel, sha) in &files_map {
            toml.push_str(&format!("\"{rel}\" = \"{sha}\"\n"));
        }
        std::fs::write(root.join("REVISION.toml"), toml).expect("write REVISION.toml");
        aggregate
    }

    /// 2023-01-01T00:00:00Z.
    const Y2023_START_MS: i64 = 1_672_531_200_000;

    /// A 10-day span in 2023, so `files_for_span` asks for exactly ONE file per
    /// symbol and the coverage floor expects 10 daily rows per symbol.
    fn ten_day_span_2023() -> crate::realdata::TimeSpan {
        crate::realdata::TimeSpan {
            start_ms: Y2023_START_MS,
            end_ms: Y2023_START_MS + 10 * ONE_DAY_MS,
            start_label: "2023-01-01T00:00:00Z",
            end_label: "2023-01-11T00:00:00Z",
        }
    }

    /// `n` in-span daily rows starting at 2023-01-01.
    fn day_rows(n: usize) -> Vec<(i64, f64)> {
        (0..n)
            .map(|d| {
                #[allow(clippy::cast_precision_loss)]
                let v = 50.0 + d as f64;
                (Y2023_START_MS + (d as i64) * ONE_DAY_MS, v)
            })
            .collect()
    }

    /// END-TO-END: `DvolDataSource::load` over a real parquet, not `#[ignore]`d.
    ///
    /// Pins, in the production call path: the three COLUMN NAMES, the
    /// symbol-from-relpath derivation (`<SYM>/<YEAR>.parquet`), the
    /// `[start_ms, end_ms)` span filter on `day_close_ts_ms`, the exact Decimal
    /// value, and the `(day_close_ts_ms ASC, symbol ASC)` sort.
    #[test]
    fn load_end_to_end_pins_columns_symbol_and_span_filter() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        let span = ten_day_span_2023();

        let mut btc = day_rows(10);
        // Two OUT-OF-SPAN rows in the same file: one whose day closes before the
        // span starts, one whose day closes at/after the span ends.
        btc.push((Y2023_START_MS - ONE_DAY_MS, 999.5));
        btc.push((span.end_ms, 888.5));
        let eth = day_rows(10);

        let rel_btc = write_dvol_parquet(root, "BTC", 2023, &btc);
        let rel_eth = write_dvol_parquet(root, "ETH", 2023, &eth);
        let aggregate = write_manifest(root, &[rel_btc, rel_eth]);

        let src = DvolDataSource::new(
            root.to_path_buf(),
            vec![Symbol::new("BTC"), Symbol::new("ETH")],
        )
        .with_expected_revision_sha(&aggregate);

        let loaded = src.load(&span, "e2e-fixture").expect(
            "load must succeed on a well-formed fixture — if this fails on a column \
             lookup, the production schema names changed",
        );

        assert_eq!(
            loaded.rows.len(),
            20,
            "10 in-span daily rows per symbol × 2 symbols; the two out-of-span BTC \
             rows must be filtered by [start_ms, end_ms) on day_close_ts_ms"
        );
        assert!(
            !loaded
                .rows
                .iter()
                .any(|r| r.dvol_close == dec!(999.5) || r.dvol_close == dec!(888.5)),
            "out-of-span sentinel values must not survive the span filter"
        );
        // The value survives EXACTLY (4dp round-trip through the Decimal parse).
        assert_eq!(
            loaded
                .rows
                .iter()
                .find(|r| r.symbol == Symbol::new("BTC"))
                .map(|r| r.dvol_close),
            Some(dec!(50.0)),
            "the first BTC daily close must survive as an exact Decimal"
        );
        // Symbol-from-relpath.
        for sym in ["BTC", "ETH"] {
            assert_eq!(
                loaded
                    .rows
                    .iter()
                    .filter(|r| r.symbol == Symbol::new(sym))
                    .count(),
                10,
                "each symbol's rows must be attributed to the symbol in its relpath prefix"
            );
        }
        // Sort order.
        let mut sorted = loaded.rows.clone();
        sorted.sort_by(|a, b| {
            a.day_close_ts_ms
                .cmp(&b.day_close_ts_ms)
                .then_with(|| a.symbol.0.as_str().cmp(b.symbol.0.as_str()))
        });
        assert_eq!(
            loaded.rows, sorted,
            "rows must be (ts ASC, symbol ASC) sorted"
        );
        assert_eq!(loaded.revision_sha, aggregate);
    }

    /// The column names are LOAD-BEARING: renaming one must fail the load, not
    /// silently yield an empty/garbage series.
    #[test]
    fn load_rejects_a_renamed_dvol_column() {
        use polars::prelude::*;
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        let span = ten_day_span_2023();

        let dir = root.join("BTC");
        std::fs::create_dir_all(&dir).expect("create dir");
        let rows = day_rows(10);
        let opens: Vec<i64> = rows.iter().map(|&(t, _)| t).collect();
        let closes_ts: Vec<i64> = opens.iter().map(|&t| t + ONE_DAY_MS - 1).collect();
        let values: Vec<f64> = rows.iter().map(|&(_, v)| v).collect();
        let mut df = df![
            "day_open_ts_ms" => opens,
            "day_close_ts_ms" => closes_ts,
            // Plausible schema drift: `dvol_close` renamed to `dvol`.
            "dvol" => values,
        ]
        .expect("build DataFrame");
        let mut file = std::fs::File::create(dir.join("2023.parquet")).expect("create");
        ParquetWriter::new(&mut file)
            .finish(&mut df)
            .expect("write parquet");

        let aggregate = write_manifest(root, &["BTC/2023.parquet".to_string()]);
        let src = DvolDataSource::new(root.to_path_buf(), vec![Symbol::new("BTC")])
            .with_expected_revision_sha(&aggregate);

        let err = src
            .load(&span, "renamed-column")
            .expect_err("a parquet without a `dvol_close` column must fail the load");
        assert!(
            matches!(err, DvolDataError::Parquet { .. }),
            "expected a Parquet column-lookup error, got: {err}"
        );
    }

    /// Review 3-15 HIGH: a NULL `dvol_close` must ERROR, never coalesce to 0.0.
    ///
    /// This is the worst of the three NULL cells: 0.0 is a *plausible* DVOL that
    /// is below every median, so the day scores CALM **and** the 0.0 enters the
    /// 30-day median ring and drags the cut down for the next 30 distinct closes.
    #[test]
    fn load_rejects_a_null_dvol_close() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        let span = ten_day_span_2023();

        let mut raw: Vec<(Option<i64>, Option<i64>, Option<f64>)> = day_rows(10)
            .into_iter()
            .map(|(open, v)| (Some(open), Some(open + ONE_DAY_MS - 1), Some(v)))
            .collect();
        raw[4].2 = None; // NULL value cell, mid-span

        let rel = write_dvol_parquet_raw(root, "BTC", 2023, &raw);
        let aggregate = write_manifest(root, &[rel]);
        let src = DvolDataSource::new(root.to_path_buf(), vec![Symbol::new("BTC")])
            .with_expected_revision_sha(&aggregate);

        let err = src
            .load(&span, "null-value")
            .expect_err("a NULL dvol_close must fail the load, not become 0.0");
        let msg = err.to_string();
        assert!(
            matches!(err, DvolDataError::NullDvolClose { row: 4, .. }),
            "expected NullDvolClose at row 4, got: {msg}"
        );
        assert!(
            msg.contains("0.0"),
            "the diagnostic must say WHY coalescing is unsafe. Got: {msg}"
        );
    }

    /// Review 3-15 HIGH: a NULL timestamp must ERROR, never become epoch-0.
    ///
    /// Under `unwrap_or(0)` the row became epoch-0 and then vanished past the
    /// span filter — a silently shrinking corpus with no diagnostic anywhere.
    #[test]
    fn load_rejects_a_null_timestamp() {
        use tempfile::TempDir;

        for (col_idx, col_name) in [(0_usize, "day_open_ts_ms"), (1, "day_close_ts_ms")] {
            let tmp = TempDir::new().expect("tempdir");
            let root = tmp.path();
            let span = ten_day_span_2023();

            let mut raw: Vec<(Option<i64>, Option<i64>, Option<f64>)> = day_rows(10)
                .into_iter()
                .map(|(open, v)| (Some(open), Some(open + ONE_DAY_MS - 1), Some(v)))
                .collect();
            if col_idx == 0 {
                raw[2].0 = None;
            } else {
                raw[2].1 = None;
            }

            let rel = write_dvol_parquet_raw(root, "BTC", 2023, &raw);
            let aggregate = write_manifest(root, &[rel]);
            let src = DvolDataSource::new(root.to_path_buf(), vec![Symbol::new("BTC")])
                .with_expected_revision_sha(&aggregate);

            let err = src
                .load(&span, "null-ts")
                .expect_err("a NULL timestamp must fail the load, not become epoch-0");
            let msg = err.to_string();
            assert!(
                matches!(err, DvolDataError::NullTimestamp { row: 2, column, .. } if column == col_name),
                "expected NullTimestamp({col_name}) at row 2, got: {msg}"
            );
        }
    }

    /// Review 3-15 MEDIUM: `day_close_ts_ms` must equal
    /// `day_open_ts_ms + 86_400_000 - 1`.
    ///
    /// `day_open_ts_ms` was present on disk and ignored. A regenerated corpus
    /// written under a different keying convention (here: keyed at the day OPEN)
    /// would have loaded silently and shifted every regime decision by a day.
    #[test]
    fn load_rejects_a_foreign_keying_convention() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        let span = ten_day_span_2023();

        // Every row keyed at the day OPEN instead of the fully-observed close.
        let raw: Vec<(Option<i64>, Option<i64>, Option<f64>)> = day_rows(10)
            .into_iter()
            .map(|(open, v)| (Some(open), Some(open), Some(v)))
            .collect();

        let rel = write_dvol_parquet_raw(root, "BTC", 2023, &raw);
        let aggregate = write_manifest(root, &[rel]);
        let src = DvolDataSource::new(root.to_path_buf(), vec![Symbol::new("BTC")])
            .with_expected_revision_sha(&aggregate);

        let err = src
            .load(&span, "foreign-keying")
            .expect_err("a corpus keyed at the day OPEN must be rejected");
        let msg = err.to_string();
        assert!(
            matches!(err, DvolDataError::KeyingInvariant { row: 0, .. }),
            "expected KeyingInvariant at row 0, got: {msg}"
        );

        // And the correct convention must still load — the guard is a floor,
        // not a tax (this is the shape the shipped corpus uses).
        let tmp_ok = TempDir::new().expect("tempdir");
        let root_ok = tmp_ok.path();
        let rel_ok = write_dvol_parquet(root_ok, "BTC", 2023, &day_rows(10));
        let agg_ok = write_manifest(root_ok, &[rel_ok]);
        assert!(
            DvolDataSource::new(root_ok.to_path_buf(), vec![Symbol::new("BTC")])
                .with_expected_revision_sha(&agg_ok)
                .load(&span, "correct-keying")
                .is_ok(),
            "the shipped `day_open + 86_400_000 - 1` convention must still load"
        );
    }

    /// Review 3-15 HIGH: the PER-SYMBOL daily coverage floor.
    ///
    /// One symbol at 2 of 10 days is the failure this exists for: the arm's
    /// median ring only advances when the as-of close CHANGES, so a thinned
    /// corpus freezes the regime cut and renders a plausible verdict instead of
    /// failing. Nothing downstream but `is_empty()` looked, and a 1-row corpus
    /// passes that.
    #[test]
    fn load_rejects_a_symbol_with_missing_coverage() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        let span = ten_day_span_2023();

        let btc = day_rows(10);
        let eth: Vec<(i64, f64)> = day_rows(10).into_iter().take(2).collect();

        let rel_btc = write_dvol_parquet(root, "BTC", 2023, &btc);
        let rel_eth = write_dvol_parquet(root, "ETH", 2023, &eth);
        let aggregate = write_manifest(root, &[rel_btc, rel_eth]);

        let src = DvolDataSource::new(
            root.to_path_buf(),
            vec![Symbol::new("BTC"), Symbol::new("ETH")],
        )
        .with_expected_revision_sha(&aggregate);

        let err = src
            .load(&span, "deficient-symbol")
            .expect_err("a symbol below the coverage floor must fail the load");
        let msg = err.to_string();
        assert!(
            matches!(err, DvolDataError::InsufficientCoverage { .. }),
            "expected InsufficientCoverage, got: {msg}"
        );
        assert!(
            msg.contains("ETH") && !msg.contains("BTC"),
            "the failure must NAME the deficient symbol and not blame the complete \
             one. Got: {msg}"
        );
    }

    /// A completely ABSENT symbol (zero rows in span) is the worst case — it
    /// must fail with the same loud, symbol-naming error.
    ///
    /// **This is bug-log #78's second trigger in miniature**: the corpus is
    /// present, its SHA verifies, and the requested window simply has no rows.
    /// Before the floor this returned `Ok(rows_for_the_other_symbol)` and the
    /// arm degenerated in silence.
    #[test]
    fn load_rejects_a_span_the_corpus_does_not_cover() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();

        // Corpus holds 2023-01-01..2023-01-10; ask for 2023-06-01..2023-06-11.
        let rel = write_dvol_parquet(root, "BTC", 2023, &day_rows(10));
        let aggregate = write_manifest(root, &[rel]);
        let stale_span = crate::realdata::TimeSpan {
            start_ms: Y2023_START_MS + 151 * ONE_DAY_MS,
            end_ms: Y2023_START_MS + 161 * ONE_DAY_MS,
            start_label: "2023-06-01T00:00:00Z",
            end_label: "2023-06-11T00:00:00Z",
        };

        let err = DvolDataSource::new(root.to_path_buf(), vec![Symbol::new("BTC")])
            .with_expected_revision_sha(&aggregate)
            .load(&stale_span, "stale-window")
            .expect_err(
                "a window the corpus does not cover must FAIL, not return an empty \
                 series that degenerates into a mislabelled arm (bug-log #78)",
            );
        assert!(
            matches!(
                err,
                DvolDataError::InsufficientCoverage {
                    actual: 0,
                    expected: 10,
                    ..
                }
            ),
            "expected InsufficientCoverage(0 of 10), got: {err}"
        );
    }

    /// The floor must not be a tax: a corpus with a single missing day inside a
    /// LONG span still loads (995‰), while the deficient case above fails.
    #[test]
    fn coverage_floor_tolerates_a_single_gap_in_a_long_span() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        // 300-day span, kept inside 2023 (a span ENDING exactly on a year
        // boundary makes `files_for_span` demand the next year's parquet too).
        // 995‰ → threshold = ceil(300 * 995 / 1000) = 299.
        let span = crate::realdata::TimeSpan {
            start_ms: Y2023_START_MS,
            end_ms: Y2023_START_MS + 300 * ONE_DAY_MS,
            start_label: "2023-01-01T00:00:00Z",
            end_label: "2023-10-28T00:00:00Z",
        };
        let mut rows = day_rows(300);
        rows.remove(100); // one missing day
        let rel = write_dvol_parquet(root, "BTC", 2023, &rows);
        let aggregate = write_manifest(root, &[rel]);

        let loaded = DvolDataSource::new(root.to_path_buf(), vec![Symbol::new("BTC")])
            .with_expected_revision_sha(&aggregate)
            .load(&span, "one-gap")
            .expect("299 of 300 days is above the 995‰ floor — must still load");
        assert_eq!(loaded.rows.len(), 299);
    }

    // ── Real-corpus smoke test (on-machine only) ──────────────────────────────

    /// Smoke test: load from the real DVOL corpus and verify rows exist.
    ///
    /// On-machine only: requires `data/deribit-dvol/BTC/2024.parquet`.
    /// Set `#[ignore]` so CI doesn't fail without the corpus.
    ///
    /// Uses `CARGO_MANIFEST_DIR` to resolve the workspace root from the crate
    /// root (`crates/backtest/`), since unit tests run with `cwd = crate dir`.
    #[test]
    #[ignore = "on-machine: requires data/deribit-dvol corpus + SHA match"]
    fn real_corpus_load_smoke() {
        use std::path::PathBuf;
        // CARGO_MANIFEST_DIR = ".../crates/backtest"; go up two levels to workspace root.
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("canonicalize workspace root");
        let dvol_root = workspace_root.join("data/deribit-dvol");
        eprintln!("dvol_root = {dvol_root:?}");

        let src = DvolDataSource::new(dvol_root, vec![Symbol::new("BTC")]);
        let span = crate::realdata::TimeSpan {
            start_ms: 1_704_067_200_000,
            end_ms: 1_719_792_000_000,
            start_label: "2024-01-01T00:00:00Z",
            end_label: "2024-07-01T00:00:00Z",
        };
        let result = src.load(&span, "smoke-test");
        match &result {
            Ok(loaded) => eprintln!(
                "OK: {} rows, sha={}",
                loaded.rows.len(),
                loaded.revision_sha
            ),
            Err(e) => eprintln!("ERR: {e:?}"),
        }
        let loaded = result.expect("real corpus load must succeed");
        assert!(!loaded.rows.is_empty(), "must have at least one DVOL row");
        assert_eq!(
            loaded.revision_sha, EXPECTED_DVOL_REVISION_SHA,
            "revision SHA mismatch"
        );
    }

    /// **Review 3-15 HIGH — the coverage-floor MEASUREMENT against the shipped
    /// corpus.** A floor that rejects the corpus it is supposed to protect is a
    /// regression, so this prints per-symbol counts for every span the product
    /// actually loads and asserts the load SUCCEEDS.
    ///
    /// On-machine only (the parquets are gitignored). Run:
    ///
    /// ```sh
    /// cargo test -p backtest --features realdata --lib \
    ///   dvol_data::tests::real_corpus_coverage_floor_accepts_shipped_corpus \
    ///   -- --include-ignored --nocapture
    /// ```
    #[test]
    #[ignore = "on-machine: requires data/deribit-dvol corpus + SHA match"]
    fn real_corpus_coverage_floor_accepts_shipped_corpus() {
        use std::path::PathBuf;
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("canonicalize workspace root");
        let dvol_root = workspace_root.join("data/deribit-dvol");

        // H1-2024 (the story's evaluation window) and the PRE-EXTENDED span the
        // bake-off now loads (review 3-15 MEDIUM: warm-up burn-in must not be
        // charged to the evaluation window).
        let h1_start = 1_704_067_200_000_i64; // 2024-01-01T00:00:00Z
        let h1_end = 1_719_792_000_000_i64; // 2024-07-01T00:00:00Z
        let warmup_ms = crate::bakeoff::DVOL_WARMUP_DAYS * ONE_DAY_MS;

        let spans: Vec<(&str, crate::realdata::TimeSpan)> = vec![
            (
                "H1-2024 (raw evaluation window)",
                crate::realdata::TimeSpan {
                    start_ms: h1_start,
                    end_ms: h1_end,
                    start_label: "2024-01-01T00:00:00Z",
                    end_label: "2024-07-01T00:00:00Z",
                },
            ),
            (
                "H1-2024 pre-extended by DVOL_WARMUP_DAYS (crosses 2023/2024)",
                crate::realdata::TimeSpan {
                    start_ms: h1_start - warmup_ms,
                    end_ms: h1_end,
                    start_label: "(warm-up-extended)",
                    end_label: "2024-07-01T00:00:00Z",
                },
            ),
            (
                "H2-2024",
                crate::realdata::TimeSpan {
                    start_ms: 1_719_792_000_000 - warmup_ms,
                    end_ms: 1_735_689_600_000,
                    start_label: "(warm-up-extended)",
                    end_label: "2025-01-01T00:00:00Z",
                },
            ),
        ];

        let mut failures: Vec<String> = Vec::new();
        for (label, span) in &spans {
            let expected = usize::try_from((span.end_ms - span.start_ms) / ONE_DAY_MS).unwrap_or(0);
            let threshold = (expected * MIN_SYMBOL_COVERAGE_PERMILLE).div_ceil(1000);
            for sym in ["BTC", "ETH"] {
                let src = DvolDataSource::new(dvol_root.clone(), vec![Symbol::new(sym)]);
                match src.load(span, "coverage-measurement") {
                    Ok(loaded) => {
                        #[allow(clippy::cast_precision_loss)]
                        let pct = loaded.rows.len() as f64 / expected as f64 * 100.0;
                        eprintln!(
                            "[coverage] {label:<58} {sym}: {:>4} / {expected:>4} daily rows \
                             ({pct:6.2}%, floor {threshold} rows) OK",
                            loaded.rows.len()
                        );
                    }
                    Err(e) => {
                        eprintln!("[coverage] {label:<58} {sym}: FAILED — {e}");
                        failures.push(format!("{label} / {sym}: {e}"));
                    }
                }
            }
        }

        assert!(
            failures.is_empty(),
            "the coverage floor must NOT reject the shipped corpus on any span the \
             product loads. Failures:\n  {}",
            failures.join("\n  ")
        );
    }

    // ── REVISION-mismatch rejection test ─────────────────────────────────────

    /// When the REVISION.toml claims an aggregate SHA different from the
    /// locked constant, `load()` must reject with `RevisionMismatch`.
    #[test]
    fn revision_mismatch_is_rejected() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();

        // Write a fake parquet file and a REVISION.toml with a wrong aggregate.
        let sym_dir = root.join("BTC");
        fs::create_dir_all(&sym_dir).expect("create dir");
        let fake_parquet_path = sym_dir.join("2023.parquet");
        fs::write(&fake_parquet_path, b"fake data").expect("write fake parquet");

        // Compute actual SHA of the fake file for the manifest.
        let fake_sha = data::revision::file_sha256(&fake_parquet_path).expect("sha256");

        // Build a REVISION.toml with a SHA that won't match EXPECTED_DVOL_REVISION_SHA.
        let wrong_aggregate_sha =
            "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
        let toml_content = format!(
            "[revision]\nsha256 = \"{wrong_aggregate_sha}\"\n\n[files]\n\"BTC/2023.parquet\" = \"{fake_sha}\"\n"
        );
        fs::write(root.join("REVISION.toml"), &toml_content).expect("write revision toml");

        let span = crate::realdata::TimeSpan::full_year(2023);
        let src = DvolDataSource::new(root.to_path_buf(), vec![Symbol::new("BTC")]);

        let result = src.load(&span, "test-scenario");
        assert!(
            result.is_err(),
            "load() must fail when the aggregate SHA does not match the locked constant"
        );
        let err = result.unwrap_err();
        let err_str = err.to_string();
        assert!(
            err_str.contains("revision mismatch")
                || matches!(err, DvolDataError::RevisionMismatch { .. }),
            "error must be RevisionMismatch, got: {err_str}"
        );
    }
}
