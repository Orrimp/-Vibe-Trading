//! Basis-data source for the perp-basis-signal-robustness backtest harness.
//!
//! Mirrors `funding_data.rs` (`FundingDataSource`) but for the basis parquets
//! under `data/binance-basis/`. Schema: `open_time` Int64 (Unix ms),
//! `close_time` Int64 (Unix ms), `basis_open/high/low/close` Utf8 signed
//! decimal strings — `basis_close` is `(markPrice − indexPrice)/indexPrice`.
//!
//! # Design decisions
//!
//! - `basis_close` is parsed as `rust_decimal::Decimal` (never f64) to
//!   preserve precision end-to-end (ADR-0003). The value is signed — negative
//!   means the perp trades below spot (shorts pay longs).
//! - REVISION.toml aggregate SHA is verified against the expected constant
//!   `EXPECTED_BASIS_REVISION_SHA` (D-BR.3, R-BR.3).
//! - The loader output is a leaner `Vec<BasisRow>` (no mark/index fields —
//!   only the derived basis_close is needed for the signal).
//! - `basis_as_of` is a pure function (M-DEV-2): given a sorted basis series
//!   and a slice of bar open-timestamps (ms), it finds the most-recent
//!   basis_close at-or-before each bar. Returns `None` for bars before the
//!   first available basis (warm-up). On the native 1h grid a query at `t`
//!   returns **`basis_close[t]`** — the value realised at the CLOSE of the bar
//!   that OPENS at `t` — because the key is the bar's OPEN time and the join
//!   predicate is `≤`. See the "Join key and causality" block on
//!   [`basis_as_of`] for why that is still causal in the anchored lane and
//!   where it would stop being so.
//!
//! # Feature gate
//!
//! Compiled only when `--features realdata` (which pulls in polars).
//!
//! # Channel reuse note (D-BR.3)
//!
//! The basis arm reuses the `funding_by_symbol`/`funding_map` channel as a
//! generic sidecar carrier — the value is the BASIS, not funding, and is
//! consumed ONLY by `basis_reversal_score`, NEVER by the `run_path` accrual
//! (which stays gated `None` for the basis arm — D-BR.1).

use std::path::PathBuf;
use std::str::FromStr;

use rust_decimal::Decimal;
use time::OffsetDateTime;
use trading_core::{PitSeries, Symbol, TimestampMs};

// ── Expected revision SHA ──────────────────────────────────────────────────────

/// Locked aggregate SHA-256 for `data/binance-basis/REVISION.toml`.
/// Any mismatch → `BasisDataError::RevisionMismatch`.
pub const EXPECTED_BASIS_REVISION_SHA: &str =
    "aa72409aa0f856960385a823bc61be1b8274e84f658439b65e5d1b1b1a48f1cd";

// ── Error type ─────────────────────────────────────────────────────────────────

#[derive(thiserror::Error, Debug)]
pub enum BasisDataError {
    #[error("data/binance-basis/REVISION.toml not found at {path}")]
    RevisionMissing { path: String },

    #[error("REVISION.toml parse error: {0}")]
    RevisionParse(String),

    /// On-disk aggregate SHA does not match the locked expected SHA.
    ///
    /// The basis backtest must reject runs on unverified data.
    #[error(
        "basis data revision mismatch: expected={expected}, recomputed={recomputed} \
         (file {file}: manifest={manifest_sha}, on-disk={actual_sha})"
    )]
    RevisionMismatch {
        file: String,
        manifest_sha: String,
        actual_sha: String,
        expected: String,
        recomputed: String,
    },

    #[error("basis parquet read error for {path}: {source}")]
    Parquet {
        path: String,
        source: polars::prelude::PolarsError,
    },

    #[error("basis_close parse error in {path} row {row}: {value}: {source}")]
    DecimalParse {
        path: String,
        row: usize,
        value: String,
        source: rust_decimal::Error,
    },

    /// A row carried a NULL `open_time` (review 1-20 L).
    ///
    /// This used to be swallowed: `open_times_col.get(i).unwrap_or(0)` mapped a
    /// NULL to epoch-0, which the span filter then silently dropped. A row
    /// vanishing without a diagnostic is exactly the shape of the R3
    /// coverage-gate bug (bug-log #70). `basis_close` NULLs already errored
    /// loudly (`""` fails `Decimal::from_str`); the timestamp column now does
    /// too.
    #[error("basis parquet {path} row {row}: open_time is NULL — refusing to guess a timestamp")]
    NullOpenTime { path: String, row: usize },

    /// A symbol's row count for the span is below the coverage floor
    /// (review 1-20 M, modelled on `RealDataError::MissingData`).
    ///
    /// The failure this catches: ONE symbol with zero (or near-zero) rows makes
    /// `basis_reversal_score` return `None` for that symbol forever. On a
    /// long-only top-K arm the symbol simply never ranks, so the θ-surface stays
    /// plausible while silently running a smaller universe than its
    /// `held_constant` row claims — and if the deficit is wide enough the whole
    /// surface goes inert while still rendering a verdict.
    #[error(
        "basis coverage below floor for {symbol} in [{span_start}..{span_end}): got {actual} \
         rows, expected ~{expected} ({pct:.2}% present, floor {floor_pct:.2}%). A symbol \
         with missing basis scores None forever and silently drops out of the \
         cross-sectional rank — refusing to run on a corpus that would render a \
         plausible surface over a smaller universe than it reports."
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

    /// The two per-symbol collections handed to [`build_basis_at_return`]
    /// disagree in length (review 1-20 wave-2 M).
    ///
    /// They are assembled in the same loop in the sweep driver and are equal
    /// only by construction. A `zip` would truncate to the shorter one and drop
    /// the tail symbols in silence.
    #[error(
        "basis/bar-timestamp collections disagree: {basis_len} symbols of basis vs \
         {bar_ts_len} symbols of bar timestamps. A zip here would silently truncate to \
         the shorter one; the dropped symbols would never warm, and because the ranker \
         requires EVERY universe symbol to be warm the whole arm would render flat cells \
         under a plausible verdict — refusing to run."
    )]
    LengthMismatch { basis_len: usize, bar_ts_len: usize },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Minimum fraction of the span's hourly rows each symbol must supply, in
/// per-mille (review 1-20 M).
///
/// Mirrors the OHLCV loader's R3 tolerance (`realdata.rs`, 995‰ = 99.5%), which
/// review 1-18 hardened after bug-log #70. Integer per-mille so the comparison
/// never touches a float.
pub const MIN_SYMBOL_COVERAGE_PERMILLE: usize = 995;

// ── BasisRow ──────────────────────────────────────────────────────────────────

/// A single basis bar record loaded from the parquet.
///
/// Tuple: `(symbol, open_time_ms, basis_close)`.
/// `basis_close = (markPrice − indexPrice) / indexPrice` — signed:
///   - Positive → perp trades rich to spot (crowded long, the reversal setup).
///   - Negative → perp trades below spot (shorts pay longs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasisRow {
    pub symbol: Symbol,
    /// Unix millisecond timestamp of the bar open.
    pub open_time_ms: i64,
    /// The basis at bar close: `(markPrice − indexPrice) / indexPrice`.
    /// Parsed as exact `Decimal` — no f64 round-trip (ADR-0003).
    pub basis_close: Decimal,
}

// ── LoadedBasis ───────────────────────────────────────────────────────────────

/// Result of a successful basis load.
#[derive(Debug)]
pub struct LoadedBasis {
    /// All basis rows across all symbols, sorted `(open_time_ms ASC, symbol ASC)`.
    pub rows: Vec<BasisRow>,
    /// Recomputed aggregate SHA-256 (64 hex chars). Written into the basis
    /// report body's `## Data source` section.
    pub revision_sha: String,
}

// ── BasisDataSource ───────────────────────────────────────────────────────────

/// Basis parquet loader, mirroring `FundingDataSource`.
pub struct BasisDataSource {
    /// Root of the basis parquet directory, e.g. `data/binance-basis`.
    basis_root: PathBuf,
    /// Universe of symbols to load (must match the basis scenario universe).
    universe: Vec<Symbol>,
    /// Aggregate SHA the corpus must recompute to. Always
    /// [`EXPECTED_BASIS_REVISION_SHA`] in production — [`Self::new`] is the only
    /// constructor outside `cfg(test)`, so the revision lock is unconditional.
    expected_revision_sha: String,
}

impl BasisDataSource {
    /// Create a new basis data source.
    ///
    /// `basis_root` must contain both the symbol subdirectories and
    /// `REVISION.toml` (same layout as `data/binance-basis/`).
    #[must_use]
    pub fn new(basis_root: PathBuf, universe: Vec<Symbol>) -> Self {
        Self {
            basis_root,
            universe,
            expected_revision_sha: EXPECTED_BASIS_REVISION_SHA.to_string(),
        }
    }

    /// Point the revision lock at a different aggregate SHA — **tests only**.
    ///
    /// Compiled ONLY under `cfg(test)`, so it does not exist in any shipped
    /// build and cannot be reached from the sweep binary. It exists so the
    /// acceptance test can drive the REAL [`Self::load`] end-to-end against a
    /// small temp-dir corpus: without it, step 3 rejects every fixture before
    /// the parquet is ever opened, which is why the only test that exercised
    /// the column names and the symbol-from-relpath derivation had to be
    /// `#[ignore]`d against the real corpus (and therefore gated nothing).
    #[cfg(test)]
    fn with_expected_revision_sha(mut self, sha: &str) -> Self {
        self.expected_revision_sha = sha.to_string();
        self
    }

    /// Load + REVISION-verify + parse basis rows for the given span.
    ///
    /// Steps:
    /// 1. Check `REVISION.toml` exists → `RevisionMissing`.
    /// 2. For every parquet file in the span, verify on-disk SHA against
    ///    manifest → `RevisionMismatch`.
    /// 3. Recompute aggregate SHA and verify it equals
    ///    `EXPECTED_BASIS_REVISION_SHA`.
    /// 4. Read parquet files; parse `basis_close` as `Decimal`. A NULL
    ///    `open_time` is an error, never a guessed timestamp (review 1-20 L).
    /// 5. Filter rows to `[span.start_ms, span.end_ms)`.
    /// 6. Enforce the PER-SYMBOL coverage floor
    ///    ([`MIN_SYMBOL_COVERAGE_PERMILLE`]) → `InsufficientCoverage`
    ///    (review 1-20 M).
    /// 7. Sort `(open_time_ms ASC, symbol ASC)`.
    ///
    /// # Errors
    ///
    /// Returns `BasisDataError` on manifest missing / SHA mismatch, parquet read
    /// errors, a NULL `open_time`, a Decimal parse failure, or a symbol whose
    /// row count for the span is below the coverage floor.
    #[allow(clippy::too_many_lines)]
    pub fn load(
        &self,
        span: &crate::realdata::TimeSpan,
        _scenario_name: &str,
    ) -> Result<LoadedBasis, BasisDataError> {
        use polars::prelude::{LazyFrame, ScanArgsParquet};

        // Step 1: manifest exists?
        let manifest_path = self.basis_root.join("REVISION.toml");
        if !manifest_path.exists() {
            return Err(BasisDataError::RevisionMissing {
                path: manifest_path.to_string_lossy().into_owned(),
            });
        }

        // Read manifest (no on-disk SHA check yet).
        let (files_map, _claimed_aggregate) =
            data::revision::read_manifest_raw(&self.basis_root)
                .map_err(|e| BasisDataError::RevisionParse(e.to_string()))?;

        // Step 2: verify each parquet file in the span.
        let scenario_files = self.files_for_span(span);
        for relpath in &scenario_files {
            let manifest_sha =
                files_map
                    .get(relpath)
                    .ok_or_else(|| BasisDataError::RevisionMismatch {
                        file: relpath.clone(),
                        manifest_sha: "(not in manifest)".to_string(),
                        actual_sha: "n/a".to_string(),
                        expected: self.expected_revision_sha.clone(),
                        recomputed: "(not computed)".to_string(),
                    })?;
            let abs_path = self.basis_root.join(relpath);
            let actual_sha = data::revision::file_sha256(&abs_path)
                .map_err(|e| BasisDataError::RevisionParse(format!("sha256 read error: {e}")))?;
            if &actual_sha != manifest_sha {
                return Err(BasisDataError::RevisionMismatch {
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
            return Err(BasisDataError::RevisionMismatch {
                file: "(aggregate)".to_string(),
                manifest_sha: "(n/a)".to_string(),
                actual_sha: "(n/a)".to_string(),
                expected: self.expected_revision_sha.clone(),
                recomputed: recomputed.clone(),
            });
        }

        // Step 4: read + parse parquet files.
        let mut rows: Vec<BasisRow> = Vec::new();
        for relpath in &scenario_files {
            let abs_path = self.basis_root.join(relpath);
            let path_str = abs_path.to_string_lossy().into_owned();

            let df = LazyFrame::scan_parquet(&abs_path, ScanArgsParquet::default())
                .and_then(polars::prelude::LazyFrame::collect)
                .map_err(|e| BasisDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?;

            // The symbol is encoded in the file path: <SYM>/<YEAR>/<MM>.parquet
            // Parse it from the relpath prefix.
            let sym_str = relpath.split('/').next().unwrap_or("");

            let open_times_col = df
                .column("open_time")
                .map_err(|e| BasisDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?
                .i64()
                .map_err(|e| BasisDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?;

            let basis_close_col = df
                .column("basis_close")
                .map_err(|e| BasisDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?
                .str()
                .map_err(|e| BasisDataError::Parquet {
                    path: path_str.clone(),
                    source: e,
                })?;

            let n_rows = df.height();
            for i in 0..n_rows {
                // Review 1-20 L: a NULL `open_time` used to become epoch-0 via
                // `unwrap_or(0)`, which the span filter below then dropped with
                // no diagnostic anywhere — a silently shrinking corpus. Fail
                // loudly instead, matching how a NULL `basis_close` already
                // behaves (it yields "" and blows up in `Decimal::from_str`).
                let open_time_ms =
                    open_times_col
                        .get(i)
                        .ok_or_else(|| BasisDataError::NullOpenTime {
                            path: path_str.clone(),
                            row: i,
                        })?;
                let basis_str = basis_close_col.get(i).unwrap_or("");

                // Step 5: filter to span [start_ms, end_ms).
                if open_time_ms < span.start_ms || open_time_ms >= span.end_ms {
                    continue;
                }

                let basis_close =
                    Decimal::from_str(basis_str).map_err(|e| BasisDataError::DecimalParse {
                        path: path_str.clone(),
                        row: i,
                        value: basis_str.to_string(),
                        source: e,
                    })?;

                rows.push(BasisRow {
                    symbol: Symbol::new(sym_str),
                    open_time_ms,
                    basis_close,
                });
            }
        }

        // Step 6: PER-SYMBOL coverage gate (review 1-20 M).
        //
        // The OHLCV loader got an equivalent gate in review 1-18 after bug-log
        // #70; the basis sidecar had none. Without it, one symbol contributing
        // zero rows is completely silent: `basis_reversal_score` returns `None`
        // for that symbol on every bar forever, so it never enters the
        // cross-sectional rank, and the θ-surface renders a full, plausible
        // verdict over a universe smaller than its own `held_constant` row
        // claims. Checked PER SYMBOL, not in aggregate, precisely because a
        // total-row check passes happily while one member is empty.
        //
        // Integer arithmetic only in the comparison — the float is for the
        // message. The basis series is native hourly, so the expected row count
        // for a span is its length in hours.
        let expected_per_symbol =
            usize::try_from((span.end_ms - span.start_ms) / 3_600_000).unwrap_or(0);
        if expected_per_symbol > 0 {
            let threshold = (expected_per_symbol * MIN_SYMBOL_COVERAGE_PERMILLE).div_ceil(1000);
            for sym in &self.universe {
                let actual = rows.iter().filter(|r| &r.symbol == sym).count();
                if actual < threshold {
                    #[allow(clippy::cast_precision_loss, clippy::float_arithmetic)]
                    let pct = actual as f64 / expected_per_symbol as f64 * 100.0;
                    #[allow(clippy::cast_precision_loss, clippy::float_arithmetic)]
                    let floor_pct = MIN_SYMBOL_COVERAGE_PERMILLE as f64 / 10.0;
                    return Err(BasisDataError::InsufficientCoverage {
                        symbol: sym.0.to_string(),
                        expected: expected_per_symbol,
                        actual,
                        pct,
                        floor_pct,
                        span_start: span.start_label.to_string(),
                        span_end: span.end_label.to_string(),
                    });
                }
            }
        }

        // Step 7: sort (open_time_ms ASC, symbol ASC).
        rows.sort_unstable_by(|a, b| {
            a.open_time_ms
                .cmp(&b.open_time_ms)
                .then_with(|| a.symbol.0.as_str().cmp(b.symbol.0.as_str()))
        });

        Ok(LoadedBasis {
            rows,
            revision_sha: recomputed,
        })
    }

    /// Return relative parquet paths for the scenario span.
    ///
    /// Layout: `<SYM>/<YEAR>/<MM>.parquet` — identical to
    /// `FundingDataSource::files_for_span`.
    #[must_use]
    pub fn files_for_span(&self, span: &crate::realdata::TimeSpan) -> Vec<String> {
        let start_dt = OffsetDateTime::from_unix_timestamp(span.start_ms / 1_000)
            .unwrap_or(OffsetDateTime::UNIX_EPOCH);
        let end_dt = OffsetDateTime::from_unix_timestamp(span.end_ms / 1_000)
            .unwrap_or(OffsetDateTime::UNIX_EPOCH);

        let mut result = Vec::new();
        let mut year = start_dt.year();
        let mut month = start_dt.month();

        loop {
            let month_start = month_start_ms(year, month);
            if month_start >= span.end_ms {
                break;
            }

            let month_num = month as u8;
            for sym in &self.universe {
                result.push(format!("{}/{year}/{month_num:02}.parquet", sym.0.as_str()));
            }

            // Advance to next month (mirrors realdata.rs exactly).
            let next_num = month_num % 12 + 1;
            let next_year = if next_num == 1 { year + 1 } else { year };
            let next_month = time::Month::try_from(next_num).unwrap_or(time::Month::January);
            let next_ms = month_start_ms(next_year, next_month);
            if next_ms >= span.end_ms {
                break;
            }
            if next_year > end_dt.year()
                || (next_year == end_dt.year() && next_month > end_dt.month())
            {
                break;
            }
            year = next_year;
            month = next_month;
        }

        result
    }
}

// ── Date helpers (mirror funding_data.rs) ─────────────────────────────────────

fn month_start_ms(year: i32, month: time::Month) -> i64 {
    let date = time::Date::from_calendar_date(year, month, 1)
        .unwrap_or(time::Date::from_ordinal_date(2023, 1).unwrap_or(time::Date::MIN));
    OffsetDateTime::new_utc(date, time::Time::MIDNIGHT).unix_timestamp() * 1_000
}

// ── M-DEV-2: as-of join ───────────────────────────────────────────────────────

/// Find the basis at each bar open timestamp using an as-of join.
///
/// For each bar open timestamp `bar_open_ts_ms[i]`, finds the `basis_close`
/// from the row whose `open_time_ms` is the most-recent one **at-or-before**
/// `bar_open_ts_ms[i]`.
///
/// On the aligned native 1h grid a query at `t` therefore returns
/// **`basis_close[t]`** — the basis row that OPENS at `t`, carrying the value
/// realised at that row's CLOSE (`t + 1h`). It is NOT `basis_close[t-1]`; see
/// "Join key and causality" below, which is the load-bearing part of this doc.
///
/// Returns `None` for bars before the first available basis (warm-up).
///
/// # Arguments
///
/// - `basis`: Sorted `(open_time_ms, basis_close)` pairs, in ascending
///   `open_time_ms` order. MUST be sorted for correct results.
/// - `bar_open_ts_ms`: Bar open timestamps in any order (each is processed
///   independently via binary search).
///
/// # Returns
///
/// `Vec<Option<Decimal>>` of length `bar_open_ts_ms.len()`.
/// `None` = no basis bar has opened yet (warm-up).
///
/// # Invariant (monotone in the series)
///
/// No basis row with `open_time_ms > q` can ever be returned for a query `q`.
/// Future-shifting the basis series (e.g. by +1h) WILL produce a different
/// result — verified by the unit test `no_look_ahead_falsifier`.
///
/// # Join key and causality (corrected by review 1-20 M)
///
/// **What the code does.** The basis parquet keys each row by `open_time`, not
/// `close_time`, while the VALUE it carries (`basis_close`) is realised at that
/// row's CLOSE. The join predicate is `≤` on that OPEN-time key. So for a query
/// at `t` on the aligned 1h grid the largest key `≤ t` is `t` itself, and the
/// function returns `basis_close[t]` — a value realised at `t + 1h`. The
/// previous version of this block claimed the opposite (`basis_close[t-1]`,
/// "strict no-look-ahead") and justified it by analogy to
/// [`crate::funding_data`]'s `funding_as_of`. **That analogy is false and has
/// been deleted:** `funding_as_of` keys on SETTLEMENT time — the instant the
/// value becomes known — where `≤` genuinely is causal. Keying on OPEN time
/// while carrying a CLOSE-realised value is a different join, and `≤` does not
/// mean the same thing on it. The unit tests immediately below this module's
/// `basis_as_of` (`bar_at_basis_open_uses_that_bar`,
/// `step_function_one_hour_correctness`) assert the ACTUAL behaviour and always
/// did; only the prose disagreed.
///
/// **Why the anchored lane is nonetheless causal.** In the anchored basis
/// surfaces the join output never touches a replayed bar. It is folded into
/// `basis_at_return[sym][k]` (see [`build_basis_at_return`]) and co-resampled by
/// the block bootstrap, and the strategy consumes it as a SCORE at the synthetic
/// bar's open while `PaperEngine` fills every order at that same bar's CLOSE
/// (`FillPriceMode::BarClose`). The decision is therefore priced at the instant
/// the basis value itself is realised — the value is known no later than the
/// price the trade gets. Nothing scored at `t` is executed before `t`'s close.
///
/// **The latent hazard — read this before reusing the function.** The causality
/// argument above is a property of the CALLER, not of this join. Joined onto
/// real replayed bars in a path that decides at a bar's OPEN and fills at that
/// same bar's open (or at any price struck before the bar closes), this would be
/// a genuine **one-bar look-ahead**: the score would embed a value not yet
/// realised at decision time. Any new consumer must either shift the key by one
/// bar or re-derive the fill-timing argument for itself. Correcting the join
/// here would re-price all eight anchored basis surfaces plus the twelve MN
/// surfaces, so the fix is owned by the re-lock program (story 1-25), not by
/// this documentation pass (ADR-0038 § D6).
///
/// Routes through `trading_core::pit::PitSeries` (ADR-0058 / M-DEV-3).
/// The public signature is kept byte-stable (existing callers and tests
/// are unchanged). The migration is behaviour-preserving: same
/// `partition_point(t <= q)` predicate, same `None` warm-up, `Decimal`
/// moved with no `f64` round-trip — identical as-of values.
#[must_use]
pub fn basis_as_of(basis: &[(i64, Decimal)], bar_open_ts_ms: &[i64]) -> Vec<Option<Decimal>> {
    if basis.is_empty() {
        return vec![None; bar_open_ts_ms.len()];
    }

    // Build a PitSeries once. The loader pre-sorts basis by open_time_ms
    // before calling this function, so from_sorted would succeed; we use
    // from_unsorted here so this library function is infallible — the sort
    // is a stable no-op on an already-sorted slice.
    let series =
        PitSeries::from_unsorted(basis.iter().map(|&(t, r)| (TimestampMs(t), r)).collect());

    bar_open_ts_ms
        .iter()
        .map(|&q| series.as_of_value(TimestampMs(q)))
        .collect()
}

/// Build the `basis_at_return[sym_i][k]` array used by the bootstrap.
///
/// The bootstrap constructs `n_returns = T-1` log-returns from `T` bars:
/// `r[k] = ln(close[k+1] / close[k])`. The basis aligned to return step `k`
/// is the as-of basis at the **open** of source bar `k` (the bar the return
/// departs from). This is computed once on the real data and stored in a
/// `Vec<Vec<Option<Decimal>>>` indexed `[sym_i][return_step]`.
///
/// # Arguments
///
/// - `basis_by_symbol`: For each symbol, the sorted `(open_time_ms, basis_close)`
///   pairs covering the real span.
/// - `bar_open_ts_ms_by_symbol`: For each symbol, the slice of bar open
///   timestamps on the real grid (length `T`). The return-step slice has
///   length `T-1` = `bar_open_ts_ms_by_symbol[s][0..T-1]`.
///
/// # Returns
///
/// `basis_at_return[sym_i][k]` = `Option<Decimal>` for k in 0..T-1.
///
/// # Errors
///
/// Returns `LengthMismatch` when the two outer slices differ in length
/// (review 1-20 wave-2 M).
///
/// This used to be a `zip`, which **silently truncates to the shorter slice**.
/// The two inputs are built in the same loop in the sweep driver and agree only
/// BY CONSTRUCTION — nothing asserted it. Any future divergence would drop the
/// tail symbols from `basis_at_return` entirely, so those symbols get no map
/// entries, never warm, and — because `MomentumStrategy::all_warmed` requires
/// EVERY universe symbol to be warm before it will rank anything — the WHOLE
/// arm never trades. All six θ-cells then render flat equity under a
/// normal-looking FRAGILE verdict. One missing symbol silently kills the entire
/// surface, and nothing in the report says so. A truncating `zip` is not an
/// acceptable way to discover that.
pub fn build_basis_at_return(
    basis_by_symbol: &[&[(i64, Decimal)]],
    bar_open_ts_ms_by_symbol: &[&[i64]],
) -> Result<Vec<Vec<Option<Decimal>>>, BasisDataError> {
    if basis_by_symbol.len() != bar_open_ts_ms_by_symbol.len() {
        return Err(BasisDataError::LengthMismatch {
            basis_len: basis_by_symbol.len(),
            bar_ts_len: bar_open_ts_ms_by_symbol.len(),
        });
    }
    Ok(basis_by_symbol
        .iter()
        .zip(bar_open_ts_ms_by_symbol.iter())
        .map(|(basis, bar_ts)| {
            // Return series has T-1 steps; align to bars 0..T-1.
            let n_returns = bar_ts.len().saturating_sub(1);
            let return_bar_ts = &bar_ts[..n_returns];
            basis_as_of(basis, return_bar_ts)
        })
        .collect())
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

    // ── basis_as_of unit tests (M-DEV-2) ─────────────────────────────────────

    /// Verify: bar before the first basis bar → None (warm-up).
    #[test]
    fn warm_up_before_first_basis_is_none() {
        let basis = vec![
            (1_000, dec!(0.001)),
            (2_000, dec!(0.002)),
            (3_000, dec!(0.003)),
        ];
        // bar at t=500 is before the first basis bar at t=1000.
        let result = basis_as_of(&basis, &[500]);
        assert_eq!(
            result,
            vec![None],
            "bar before first basis must be None (warm-up)"
        );
    }

    /// Verify: bar exactly at a basis bar's open_time uses that bar's basis_close.
    #[test]
    fn bar_at_basis_open_uses_that_bar() {
        let basis = vec![(1_000, dec!(0.001)), (2_000, dec!(0.002))];
        // Query at exactly 1_000 (the open_time of the first bar):
        // The as-of join finds basis[0] (open_time=1000 ≤ 1000) → Some(0.001).
        let result = basis_as_of(&basis, &[1_000]);
        assert_eq!(result, vec![Some(dec!(0.001))]);
    }

    /// Verify: bar between two basis bars uses the earlier one (step-function forward-fill).
    #[test]
    fn bar_between_basis_bars_uses_earlier() {
        let basis = vec![(1_000, dec!(0.001)), (3_000, dec!(0.003))];
        // bar at 2_000 is between open_time=1_000 and open_time=3_000 → uses 1_000.
        let result = basis_as_of(&basis, &[2_000]);
        assert_eq!(result, vec![Some(dec!(0.001))]);
    }

    /// Step-function correctness: a sequence of bars on the 1h grid.
    ///
    /// Review 1-20 M: the assertions here have always been right; the prose
    /// around them was not. A query at `T` returns the row whose `open_time` is
    /// `T` — i.e. `basis_close[T]`, realised at `T+1h` — NOT `basis_close[T-1]`.
    /// The old comment claimed the latter and then argued with itself in-line
    /// about whether `≤` or `<` was correct. See the "Join key and causality"
    /// block on `basis_as_of` for what makes this causal in the anchored lane
    /// (score at open, fill at close) and where it would stop being causal.
    #[test]
    fn step_function_one_hour_correctness() {
        let one_hour_ms: i64 = 3_600_000;
        // Three hourly basis bars at t=0, 1h, 2h.
        let basis = vec![
            (0, dec!(0.001)),
            (one_hour_ms, dec!(0.002)),
            (2 * one_hour_ms, dec!(0.003)),
        ];
        // Query at each hour boundary.
        let bars: Vec<i64> = (0..5).map(|h| h * one_hour_ms).collect();
        let result = basis_as_of(&basis, &bars);

        // Bar at t=0: basis row with open_time=0 ≤ 0 → uses basis[0] = 0.001.
        assert_eq!(result[0], Some(dec!(0.001)));
        // Bar at t=1h: the largest key ≤ 1h is 1h itself → basis[1] = 0.002.
        // This is `basis_close[t]`, whose value is realised at t+1h — NOT
        // `basis_close[t-1]`. The window is (t−L, t], not "strictly before t".
        assert_eq!(result[1], Some(dec!(0.002)));
        // Bar at t=2h: largest key ≤ 2h is 2h → basis[2] = 0.003.
        assert_eq!(result[2], Some(dec!(0.003)));
        // Bar at t=3h: no basis bar beyond 2h → uses basis[2] = 0.003 (forward-fill).
        assert_eq!(result[3], Some(dec!(0.003)));
        // Bar at t=4h: same forward-fill.
        assert_eq!(result[4], Some(dec!(0.003)));
    }

    /// No-look-ahead falsifier (R-BR.5):
    /// Shifting basis series +1 bar into the FUTURE changes the result.
    /// This proves the join is causal (past-only) — future basis never leaks.
    ///
    /// Mirrors `funding_data.rs::no_look_ahead_falsifier` exactly.
    #[test]
    fn no_look_ahead_falsifier() {
        let one_hour_ms: i64 = 3_600_000;
        let basis = vec![
            (1 * one_hour_ms, dec!(0.001)),
            (2 * one_hour_ms, dec!(-0.002)),
            (3 * one_hour_ms, dec!(0.003)),
        ];

        // Shift each basis bar's open_time +1 hour into the future.
        let future_shifted: Vec<(i64, Decimal)> =
            basis.iter().map(|&(t, r)| (t + one_hour_ms, r)).collect();

        // Query a bar at the original bar boundary.
        let bar_ts = &[2 * one_hour_ms];

        let causal_result = basis_as_of(&basis, bar_ts);
        let shifted_result = basis_as_of(&future_shifted, bar_ts);

        // Causal: bar at 2h sees basis open_time=2h → basis[1] = -0.002.
        assert_eq!(causal_result, vec![Some(dec!(-0.002))]);
        // Shifted: the second basis bar is now at 3h, so at 2h only open_time=2h
        // (was 1h) is available → shifted basis[0] = 0.001.
        assert_eq!(shifted_result, vec![Some(dec!(0.001))]);
        // They MUST differ — proves no look-ahead.
        assert_ne!(
            causal_result, shifted_result,
            "no-look-ahead falsifier: causal ≠ shifted (future basis must not leak)"
        );
    }

    /// Empty basis series → all None.
    #[test]
    fn empty_basis_series_all_none() {
        let result = basis_as_of(&[], &[100, 200, 300]);
        assert_eq!(result, vec![None, None, None]);
    }

    /// build_basis_at_return aligns to T-1 return steps, not T bars.
    #[test]
    fn build_basis_at_return_aligns_to_t_minus_1() {
        let one_hour_ms: i64 = 3_600_000;
        let basis: Vec<(i64, Decimal)> = vec![(0, dec!(0.001)), (one_hour_ms, dec!(-0.002))];
        // 5 bars → 4 return steps.
        let bar_ts: Vec<i64> = (0..5).map(|h| h * one_hour_ms).collect();

        let result = build_basis_at_return(&[basis.as_slice()], &[bar_ts.as_slice()])
            .expect("equal-length inputs must build");

        // Should have length T-1 = 4 for the one symbol.
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 4, "T-1 return steps");
        // Bar 0 (t=0h): basis open_time=0 ≤ 0 → Some(0.001).
        assert_eq!(result[0][0], Some(dec!(0.001)));
        // Bar 1 (t=1h): basis open_time=1h ≤ 1h → Some(-0.002).
        assert_eq!(result[0][1], Some(dec!(-0.002)));
        // Bar 3 (t=3h): no basis beyond 1h → forward-fill Some(-0.002).
        assert_eq!(result[0][3], Some(dec!(-0.002)));
    }

    /// Review 1-20 wave-2 M: mismatched per-symbol collections must ERROR, not
    /// silently truncate to the shorter one.
    ///
    /// The old body was a bare `zip`. The two inputs are assembled in the same
    /// loop in the sweep driver and are equal only BY CONSTRUCTION — nothing
    /// asserted it. Under a truncation the tail symbols get no basis entries at
    /// all, so they never warm; `MomentumStrategy::all_warmed` requires EVERY
    /// universe symbol to be warm before the ranker selects anything, so the
    /// whole arm stops trading and all six θ-cells render flat equity under a
    /// plausible FRAGILE verdict — with nothing in the report to say a symbol
    /// went missing.
    #[test]
    fn build_basis_at_return_rejects_mismatched_symbol_counts() {
        let one_hour_ms: i64 = 3_600_000;
        let basis_a: Vec<(i64, Decimal)> = vec![(0, dec!(0.001))];
        let basis_b: Vec<(i64, Decimal)> = vec![(0, dec!(0.002))];
        let bar_ts: Vec<i64> = (0..5).map(|h| h * one_hour_ms).collect();

        // TWO symbols of basis, ONE of bar timestamps. A `zip` would return a
        // single row and drop the second symbol without a word.
        let err = build_basis_at_return(
            &[basis_a.as_slice(), basis_b.as_slice()],
            &[bar_ts.as_slice()],
        )
        .expect_err("mismatched symbol counts must be rejected");
        let msg = err.to_string();
        assert!(
            matches!(err, BasisDataError::LengthMismatch { .. }),
            "expected LengthMismatch, got: {msg}"
        );
        assert!(
            msg.contains('2') && msg.contains('1'),
            "the failure must report BOTH lengths so the operator can see which side \
             is short. Got: {msg}"
        );

        // The converse orientation must be rejected too.
        assert!(
            build_basis_at_return(
                &[basis_a.as_slice()],
                &[bar_ts.as_slice(), bar_ts.as_slice()]
            )
            .is_err(),
            "the mismatch check must be symmetric"
        );

        // And the equal-length case must still succeed — the guard is a floor,
        // not a tax (this is the shape every anchored invocation uses).
        assert!(
            build_basis_at_return(
                &[basis_a.as_slice(), basis_b.as_slice()],
                &[bar_ts.as_slice(), bar_ts.as_slice()],
            )
            .is_ok(),
            "equal-length inputs — the anchored shape — must still build"
        );
    }

    /// Decimal precision is preserved — no f64 round-trip.
    #[test]
    fn decimal_precision_preserved() {
        // Use a basis with many decimal places that would lose precision as f64.
        let basis_str = "0.00012345";
        let parsed = Decimal::from_str(basis_str).expect("parse");
        let expected = dec!(0.00012345);
        assert_eq!(parsed, expected, "decimal precision must be exact");
        // Confirm round-trip through string preserves value.
        let round_tripped = parsed.to_string();
        let reparsed = Decimal::from_str(&round_tripped).expect("reparse");
        assert_eq!(parsed, reparsed);
    }

    /// `Decimal::from_str` round-trips a signed basis string.
    ///
    /// **Scope note (review 1-20 M):** this test calls `Decimal::from_str`
    /// DIRECTLY. It is a property check on `rust_decimal`, not on this loader —
    /// it would stay green if `load()` dropped the sign, ignored the column, or
    /// never ran. It was previously described as "load-bearing"; it is not. The
    /// loader-level guarantee that a negative basis survives the real parse path
    /// is `load_end_to_end_pins_columns_symbol_and_negative_sign`, which drives
    /// `BasisDataSource::load` over an actual parquet and is not `#[ignore]`d.
    #[test]
    fn signed_negative_basis_parse() {
        let negative_str = "-0.0012";
        let parsed = Decimal::from_str(negative_str).expect("parse negative basis");
        assert!(
            parsed < Decimal::ZERO,
            "negative basis string must parse to a negative Decimal, got: {parsed}"
        );
        assert_eq!(parsed, dec!(-0.0012));

        // Larger magnitude negative.
        let large_neg = Decimal::from_str("-0.05").expect("parse");
        assert_eq!(large_neg, dec!(-0.05));

        // Positive for completeness.
        let pos = Decimal::from_str("0.0034").expect("parse");
        assert_eq!(pos, dec!(0.0034));
    }

    /// Out-of-span filter: basis bars before the span's start → None (warm-up).
    ///
    /// Verified at the `basis_as_of` level (pure function — no parquet I/O).
    #[test]
    fn out_of_span_filter_via_basis_as_of() {
        // Basis bars only from 2023-01-01 onward.
        let year_start_ms: i64 = 1_672_531_200_000;
        let one_hour_ms: i64 = 3_600_000;
        let basis = vec![
            (year_start_ms, dec!(0.0001)),
            (year_start_ms + one_hour_ms, dec!(0.0002)),
        ];

        // A bar before the year start is treated as warm-up.
        let pre_span_bar = year_start_ms - 1;
        let result = basis_as_of(&basis, &[pre_span_bar, year_start_ms]);
        assert_eq!(result[0], None, "bar before first basis must be None");
        assert_eq!(
            result[1],
            Some(dec!(0.0001)),
            "bar at first basis must be Some"
        );
    }

    // ── Review 1-20 M: END-TO-END loader acceptance (NOT skip-gated) ──────────
    //
    // Before this block, the loader's two named acceptance tests gated nothing:
    //
    //   * `real_parquet_parses_to_expected_rows` is `#[ignore]`d (it needs the
    //     real corpus), so NOTHING in CI pinned the `open_time` / `basis_close`
    //     column names or the symbol-from-relpath derivation. Rename a column in
    //     a future schema bump and every gate stays green.
    //   * `signed_negative_basis_parse` calls `Decimal::from_str` directly — it
    //     is a property test for `rust_decimal`, not for this loader, and would
    //     stay green if `load()` dropped the sign entirely.
    //
    // The tests below write a real (tiny) parquet to a temp dir and drive the
    // production `BasisDataSource::load` over it end to end.

    /// Write a basis parquet at `<root>/<sym>/<year>/<month>.parquet`.
    ///
    /// Columns are named EXACTLY as the production schema
    /// (`open_time` Int64, `basis_close` Utf8) — that is the point of the test:
    /// `load()` must find them under these names.
    fn write_basis_parquet(
        root: &std::path::Path,
        sym: &str,
        year: i32,
        month: u32,
        rows: &[(i64, &str)],
    ) -> String {
        use polars::prelude::*;

        let dir = root.join(format!("{sym}/{year}"));
        std::fs::create_dir_all(&dir).expect("create symbol dir");
        let path = dir.join(format!("{month:02}.parquet"));

        let open_times: Vec<i64> = rows.iter().map(|&(t, _)| t).collect();
        let basis: Vec<&str> = rows.iter().map(|&(_, b)| b).collect();
        let mut df = df![
            "open_time" => open_times,
            "basis_close" => basis,
        ]
        .expect("build fixture DataFrame");

        let mut file = std::fs::File::create(&path).expect("create parquet");
        ParquetWriter::new(&mut file)
            .finish(&mut df)
            .expect("write parquet");

        format!("{sym}/{year}/{month:02}.parquet")
    }

    /// Write `REVISION.toml` for the given relpaths and return the aggregate SHA
    /// the loader will recompute.
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

    /// A 24-hour span on 2023-01-01, so `files_for_span` asks for exactly ONE
    /// month file per symbol and the coverage floor expects 24 rows per symbol.
    fn one_day_span_2023() -> crate::realdata::TimeSpan {
        let start = 1_672_531_200_000_i64; // 2023-01-01T00:00:00Z
        crate::realdata::TimeSpan {
            start_ms: start,
            end_ms: start + 24 * 3_600_000,
            start_label: "2023-01-01T00:00:00Z",
            end_label: "2023-01-02T00:00:00Z",
        }
    }

    const HOUR_MS: i64 = 3_600_000;

    /// Build 24 in-span hourly rows for a symbol. `neg_at` gets a NEGATIVE
    /// basis; everything else is positive.
    fn day_rows(neg_at: usize, neg_value: &'static str) -> Vec<(i64, &'static str)> {
        let start = 1_672_531_200_000_i64;
        (0..24_usize)
            .map(|h| {
                let ts = start + (h as i64) * HOUR_MS;
                let v = if h == neg_at { neg_value } else { "0.0034" };
                (ts, v)
            })
            .collect()
    }

    /// END-TO-END: `BasisDataSource::load` over a real parquet.
    ///
    /// Pins, in the production call path and with no `#[ignore]`:
    /// - the `open_time` and `basis_close` COLUMN NAMES;
    /// - the symbol-from-relpath derivation (`<SYM>/<YEAR>/<MM>.parquet`);
    /// - that a NEGATIVE basis survives the load **with its sign**;
    /// - the `[start_ms, end_ms)` span filter; and
    /// - the `(open_time ASC, symbol ASC)` sort.
    #[test]
    fn load_end_to_end_pins_columns_symbol_and_negative_sign() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        let span = one_day_span_2023();

        // Two symbols. AAAUSDT carries a negative basis at hour 5; BBBUSDT at
        // hour 0 (so the negative value is also the FIRST row after sorting).
        let mut a_rows = day_rows(5, "-0.0012");
        let b_rows = day_rows(0, "-0.05");

        // Two OUT-OF-SPAN rows in the same file: one before the span start and
        // one at/after the span end. Both must be filtered out.
        a_rows.push((span.start_ms - HOUR_MS, "9.9999"));
        a_rows.push((span.end_ms, "8.8888"));

        let rel_a = write_basis_parquet(root, "AAAUSDT", 2023, 1, &a_rows);
        let rel_b = write_basis_parquet(root, "BBBUSDT", 2023, 1, &b_rows);
        let aggregate = write_manifest(root, &[rel_a, rel_b]);

        let src = BasisDataSource::new(
            root.to_path_buf(),
            vec![Symbol::new("AAAUSDT"), Symbol::new("BBBUSDT")],
        )
        .with_expected_revision_sha(&aggregate);

        let loaded = src.load(&span, "e2e-fixture").expect(
            "load must succeed on a well-formed fixture — if this fails on a column \
             lookup, the production schema names changed",
        );

        // Span filter: 24 in-span rows per symbol, the two out-of-span rows gone.
        assert_eq!(
            loaded.rows.len(),
            48,
            "24 in-span rows per symbol × 2 symbols; the two out-of-span AAAUSDT \
             rows must be filtered by [start_ms, end_ms)"
        );
        assert!(
            !loaded
                .rows
                .iter()
                .any(|r| r.basis_close == dec!(9.9999) || r.basis_close == dec!(8.8888)),
            "out-of-span sentinel values must not survive the span filter"
        );

        // Symbol-from-relpath: the symbol comes from the path prefix, nowhere else.
        let a_count = loaded
            .rows
            .iter()
            .filter(|r| r.symbol == Symbol::new("AAAUSDT"))
            .count();
        let b_count = loaded
            .rows
            .iter()
            .filter(|r| r.symbol == Symbol::new("BBBUSDT"))
            .count();
        assert_eq!(
            (a_count, b_count),
            (24, 24),
            "each symbol's rows must be attributed to the symbol in its relpath prefix"
        );

        // THE load-bearing one: a negative basis must survive with its sign.
        let a_neg: Vec<Decimal> = loaded
            .rows
            .iter()
            .filter(|r| r.symbol == Symbol::new("AAAUSDT") && r.basis_close < Decimal::ZERO)
            .map(|r| r.basis_close)
            .collect();
        assert_eq!(
            a_neg,
            vec![dec!(-0.0012)],
            "AAAUSDT's hour-5 basis of -0.0012 must come back NEGATIVE and exact. \
             If the sign is gone the arm inverts: negative basis is the \
             reversal-favored leg the whole signal is built to buy."
        );
        let b_neg: Vec<Decimal> = loaded
            .rows
            .iter()
            .filter(|r| r.symbol == Symbol::new("BBBUSDT") && r.basis_close < Decimal::ZERO)
            .map(|r| r.basis_close)
            .collect();
        assert_eq!(b_neg, vec![dec!(-0.05)], "BBBUSDT's hour-0 basis of -0.05");

        // Sort: (open_time ASC, symbol ASC).
        let mut sorted = loaded.rows.clone();
        sorted.sort_by(|a, b| {
            a.open_time_ms
                .cmp(&b.open_time_ms)
                .then_with(|| a.symbol.0.as_str().cmp(b.symbol.0.as_str()))
        });
        assert_eq!(
            loaded.rows, sorted,
            "rows must be (ts ASC, symbol ASC) sorted"
        );
        assert_eq!(loaded.revision_sha, aggregate);
    }

    /// The column names are LOAD-BEARING: renaming either one must fail the
    /// load, not silently yield an empty/garbage series.
    #[test]
    fn load_rejects_a_renamed_basis_column() {
        use polars::prelude::*;
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        let span = one_day_span_2023();

        // Same data, but `basis_close` renamed to `basis` — a plausible schema
        // drift. `load()` must error on the column lookup.
        let dir = root.join("AAAUSDT/2023");
        std::fs::create_dir_all(&dir).expect("create dir");
        let rows = day_rows(3, "-0.0012");
        let open_times: Vec<i64> = rows.iter().map(|&(t, _)| t).collect();
        let basis: Vec<&str> = rows.iter().map(|&(_, b)| b).collect();
        let mut df = df![
            "open_time" => open_times,
            "basis" => basis,
        ]
        .expect("build DataFrame");
        let mut file = std::fs::File::create(dir.join("01.parquet")).expect("create");
        ParquetWriter::new(&mut file)
            .finish(&mut df)
            .expect("write parquet");

        let aggregate = write_manifest(root, &["AAAUSDT/2023/01.parquet".to_string()]);
        let src = BasisDataSource::new(root.to_path_buf(), vec![Symbol::new("AAAUSDT")])
            .with_expected_revision_sha(&aggregate);

        let err = src
            .load(&span, "renamed-column")
            .expect_err("a parquet without a `basis_close` column must fail the load");
        assert!(
            matches!(err, BasisDataError::Parquet { .. }),
            "expected a Parquet column-lookup error, got: {err}"
        );
    }

    /// Review 1-20 M: the PER-SYMBOL coverage gate.
    ///
    /// One symbol with (near-)zero rows is the failure this exists for: it
    /// scores `None` forever, silently drops out of the cross-sectional rank,
    /// and the surface still renders a plausible verdict over a smaller
    /// universe than its `held_constant` row claims.
    #[test]
    fn load_rejects_a_symbol_with_missing_coverage() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        let span = one_day_span_2023();

        // AAAUSDT is complete (24/24). BBBUSDT has 2 of 24 rows.
        let a_rows = day_rows(5, "-0.0012");
        let b_rows: Vec<(i64, &str)> = day_rows(0, "-0.05").into_iter().take(2).collect();

        let rel_a = write_basis_parquet(root, "AAAUSDT", 2023, 1, &a_rows);
        let rel_b = write_basis_parquet(root, "BBBUSDT", 2023, 1, &b_rows);
        let aggregate = write_manifest(root, &[rel_a, rel_b]);

        let src = BasisDataSource::new(
            root.to_path_buf(),
            vec![Symbol::new("AAAUSDT"), Symbol::new("BBBUSDT")],
        )
        .with_expected_revision_sha(&aggregate);

        let err = src
            .load(&span, "deficient-symbol")
            .expect_err("a symbol below the coverage floor must fail the load");
        let msg = err.to_string();
        assert!(
            matches!(err, BasisDataError::InsufficientCoverage { .. }),
            "expected InsufficientCoverage, got: {msg}"
        );
        assert!(
            msg.contains("BBBUSDT"),
            "the failure must NAME the deficient symbol so the operator knows which \
             one to backfill. Got: {msg}"
        );
        assert!(
            !msg.contains("AAAUSDT"),
            "the complete symbol must not be blamed. Got: {msg}"
        );
    }

    /// A completely ABSENT symbol (zero rows) is the worst case — it must fail
    /// with the same loud, symbol-naming error, not silently shrink the universe.
    #[test]
    fn load_rejects_a_symbol_with_zero_rows() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        let span = one_day_span_2023();

        let a_rows = day_rows(5, "-0.0012");
        // BBBUSDT's file exists but holds only OUT-OF-SPAN rows → zero in-span.
        let b_rows: Vec<(i64, &str)> = vec![
            (span.start_ms - 2 * HOUR_MS, "0.001"),
            (span.start_ms - HOUR_MS, "0.002"),
        ];

        let rel_a = write_basis_parquet(root, "AAAUSDT", 2023, 1, &a_rows);
        let rel_b = write_basis_parquet(root, "BBBUSDT", 2023, 1, &b_rows);
        let aggregate = write_manifest(root, &[rel_a, rel_b]);

        let src = BasisDataSource::new(
            root.to_path_buf(),
            vec![Symbol::new("AAAUSDT"), Symbol::new("BBBUSDT")],
        )
        .with_expected_revision_sha(&aggregate);

        let err = src
            .load(&span, "absent-symbol")
            .expect_err("a symbol with ZERO in-span rows must fail the load");
        let msg = err.to_string();
        assert!(
            msg.contains("BBBUSDT") && msg.contains("got 0 rows"),
            "the failure must name the empty symbol and say it has zero rows. Got: {msg}"
        );
    }

    /// A fully-covered corpus must NOT be rejected — the gate is a floor, not a
    /// tax. (The shipped corpus is verified separately by
    /// `real_corpus_passes_the_coverage_gate`, which needs the real parquets.)
    #[test]
    fn load_accepts_full_coverage() {
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        let span = one_day_span_2023();

        let rel_a = write_basis_parquet(root, "AAAUSDT", 2023, 1, &day_rows(5, "-0.0012"));
        let rel_b = write_basis_parquet(root, "BBBUSDT", 2023, 1, &day_rows(9, "-0.05"));
        let aggregate = write_manifest(root, &[rel_a, rel_b]);

        let src = BasisDataSource::new(
            root.to_path_buf(),
            vec![Symbol::new("AAAUSDT"), Symbol::new("BBBUSDT")],
        )
        .with_expected_revision_sha(&aggregate);

        let loaded = src
            .load(&span, "full-coverage")
            .expect("a fully-covered corpus must pass the coverage gate");
        assert_eq!(loaded.rows.len(), 48);
    }

    /// Review 1-20 L: a NULL `open_time` must be LOUD, not mapped to epoch-0 and
    /// then dropped by the span filter with no diagnostic anywhere.
    #[test]
    fn load_rejects_a_null_open_time() {
        use polars::prelude::*;
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        let span = one_day_span_2023();

        let dir = root.join("AAAUSDT/2023");
        std::fs::create_dir_all(&dir).expect("create dir");

        // 24 well-formed rows, with hour 7's open_time set to NULL.
        let rows = day_rows(5, "-0.0012");
        let open_times: Vec<Option<i64>> = rows
            .iter()
            .enumerate()
            .map(|(i, &(t, _))| if i == 7 { None } else { Some(t) })
            .collect();
        let basis: Vec<&str> = rows.iter().map(|&(_, b)| b).collect();
        let mut df = df![
            "open_time" => open_times,
            "basis_close" => basis,
        ]
        .expect("build DataFrame");
        let mut file = std::fs::File::create(dir.join("01.parquet")).expect("create");
        ParquetWriter::new(&mut file)
            .finish(&mut df)
            .expect("write parquet");

        let aggregate = write_manifest(root, &["AAAUSDT/2023/01.parquet".to_string()]);
        let src = BasisDataSource::new(root.to_path_buf(), vec![Symbol::new("AAAUSDT")])
            .with_expected_revision_sha(&aggregate);

        let err = src
            .load(&span, "null-open-time")
            .expect_err("a NULL open_time must fail the load, not vanish silently");
        let msg = err.to_string();
        assert!(
            matches!(err, BasisDataError::NullOpenTime { .. }),
            "expected NullOpenTime, got: {msg}"
        );
        assert!(
            msg.contains("row 7"),
            "the failure must name the offending row. Got: {msg}"
        );
    }

    // ── REVISION-mismatch rejection test (M-DEV-1) ────────────────────────────

    /// When the REVISION.toml claims an aggregate SHA different from the
    /// locked constant, `load()` must reject with `RevisionMismatch`.
    ///
    /// We construct a temp-dir with a fake REVISION.toml claiming a wrong SHA.
    #[test]
    fn revision_mismatch_is_rejected() {
        use std::fs;
        use tempfile::TempDir;

        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();

        // Write a fake parquet file and a REVISION.toml with a wrong aggregate.
        let sym_dir = root.join("BTCUSDT/2023");
        fs::create_dir_all(&sym_dir).expect("create dir");
        let fake_parquet_path = sym_dir.join("01.parquet");
        fs::write(&fake_parquet_path, b"fake data").expect("write fake parquet");

        // Compute actual SHA of the fake file for the manifest.
        let fake_sha = data::revision::file_sha256(&fake_parquet_path).expect("sha256");

        // Build a REVISION.toml with a SHA that won't match EXPECTED_BASIS_REVISION_SHA.
        let wrong_aggregate_sha =
            "0000000000000000000000000000000000000000000000000000000000000000";
        let toml_content = format!(
            "[revision]\nsha256 = \"{wrong_aggregate_sha}\"\n\n[files]\n\"BTCUSDT/2023/01.parquet\" = \"{fake_sha}\"\n"
        );
        fs::write(root.join("REVISION.toml"), &toml_content).expect("write revision toml");

        let span = crate::realdata::TimeSpan::full_year(2023);
        let src = BasisDataSource::new(root.to_path_buf(), vec![Symbol::new("BTCUSDT")]);

        let result = src.load(&span, "test-scenario");
        assert!(
            result.is_err(),
            "load() must fail when the aggregate SHA does not match the locked constant"
        );
        let err = result.unwrap_err();
        let err_str = err.to_string();
        assert!(
            err_str.contains("revision mismatch")
                || matches!(err, BasisDataError::RevisionMismatch { .. }),
            "error must be RevisionMismatch, got: {err_str}"
        );
    }

    // ── Known parquet parse test (M-DEV-1) ────────────────────────────────────
    // This test is ignored by default because it requires the real basis
    // parquet files at data/binance-basis/. Run with:
    //   cargo test -p backtest --features "candle realdata" --lib basis_data -- --include-ignored
    /// Review 1-20 M: the shipped corpus must PASS the per-symbol coverage gate.
    ///
    /// A validation addition may reject only combinations no checked-in corpus
    /// uses, so the gate has to be measured against the real data, not just
    /// against fixtures. This drives the production `load()` over the full
    /// anchored universe for BOTH anchored years and prints the per-symbol row
    /// counts. `#[ignore]`d because it needs `data/binance-basis/` on disk (the
    /// same reason as the sibling below); it is a MEASUREMENT, and the fixture
    /// tests above are what gate CI.
    #[test]
    #[ignore = "requires real data/binance-basis/ parquet files on disk"]
    fn real_corpus_passes_the_coverage_gate() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .expect("workspace root");
        let basis_root = workspace_root.join("data/binance-basis");

        let universe: Vec<Symbol> = crate::scenarios::momentum::top10_symbols_with_prices()
            .into_iter()
            .map(|(s, _)| s)
            .collect();

        for year in [2023, 2024] {
            let span = crate::realdata::TimeSpan::full_year(year);
            let src = BasisDataSource::new(basis_root.clone(), universe.clone());
            let loaded = src.load(&span, "coverage-probe").unwrap_or_else(|e| {
                panic!(
                    "COVERAGE GATE REJECTED THE SHIPPED CORPUS for {year}: {e}\n\
                     The gate must not reject data the anchored surfaces were built on — \
                     either the floor (MIN_SYMBOL_COVERAGE_PERMILLE) is too high or the \
                     corpus really is deficient."
                )
            });

            let expected = (span.end_ms - span.start_ms) / 3_600_000;
            for sym in &universe {
                let n = loaded.rows.iter().filter(|r| &r.symbol == sym).count();
                println!("{year} {sym:>10}: {n} / {expected} rows");
            }
        }
    }

    #[test]
    #[ignore = "requires real data/binance-basis/ parquet files on disk"]
    fn real_parquet_parses_to_expected_rows() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .expect("workspace root");
        let basis_root = workspace_root.join("data/binance-basis");

        let span = crate::realdata::TimeSpan::full_year(2023);
        let src = BasisDataSource::new(basis_root, vec![Symbol::new("BTCUSDT")]);

        let loaded = src.load(&span, "test-real").expect("load must succeed");

        // BTCUSDT 2023 hourly basis: 365 days × 24 bars/day = 8,760 expected rows.
        // Allow ±24 for any edge-of-year boundary bars.
        assert!(
            loaded.rows.len() >= 8_730 && loaded.rows.len() <= 8_790,
            "expected ~8760 BTCUSDT basis rows for 2023-FY, got {}",
            loaded.rows.len()
        );

        // Every row must have a valid symbol.
        for row in &loaded.rows {
            assert_eq!(row.symbol, Symbol::new("BTCUSDT"));
            let _basis: Decimal = row.basis_close; // already a Decimal — compile check
        }

        // Revision SHA must match the locked constant.
        assert_eq!(
            loaded.revision_sha, EXPECTED_BASIS_REVISION_SHA,
            "revision SHA must match the locked constant"
        );
    }
}
