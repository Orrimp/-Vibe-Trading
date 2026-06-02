//! THROWAWAY universe-structure diagnostic (analyst, 2026-06-02).
//!
//! Reads the SAME 10-symbol Binance OHLCV via the SAME `ReplayFeed::merge_symbols`
//! path the backtest harness uses (so numbers trace to the banked parquets under
//! the revision pin 3a8b96c4…), and computes three universe-structure metrics for
//! a given calendar year:
//!
//!   1. Average pairwise correlation of 1h log-returns (45 unique pairs).
//!   2. Cross-sectional return dispersion over time (std across the 10 names at
//!      each timestamp): time-mean + percentiles.
//!   3. 1-factor decomposition: regress each name's returns on the equal-weight
//!      index return; per-name R^2 = common-beta share; average R^2 across names.
//!      A high average R^2 means the universe is ~1 factor (almost all common
//!      market beta), which would be a structural ceiling on cross-sectional alpha.
//!
//! Run (default 10-symbol large-cap baseline under data/binance):
//!       cargo run -p data --example universe_diag -- 2023
//!       cargo run -p data --example universe_diag -- 2024
//!
//! Run (broader-universe spike, 2026-06-02): override the root + symbol list to
//! re-read M1–M4 on an arbitrary banked symbol set (e.g. the 35 mid-caps fetched
//! into data/binance-broaduni for the universe-vs-method disambiguation):
//!       cargo run -p data --example universe_diag -- 2024 \
//!         --root data/binance-broaduni \
//!         --symbols NEARUSDT,ZECUSDT,WLDUSDT,XLMUSDT,SUIUSDT,...
//!
//! Both knobs are optional and additive: with no `--root` the root is
//! `data/binance`; with no `--symbols` the 10-symbol large-cap `BASELINE_10`
//! const below is used. The positional first arg is always the calendar year,
//! so the original `-- 2023` / `-- 2024` invocations are unchanged.
//!
//! NOT committed as a bin; not anchored; pure read-only over banked data.

// Pairwise/triangular correlation loops index multiple parallel arrays by position;
// explicit indexing is clearer here than iterator adaptors (disposable probe).
#![allow(clippy::needless_range_loop)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use data::ReplayFeed;
use rust_decimal::prelude::ToPrimitive;
use trading_core::{Bar, Symbol, Timeframe};

/// The original 10-symbol large-cap baseline (used when `--symbols` is omitted).
const BASELINE_10: [&str; 10] = [
    "ADAUSDT", "AVAXUSDT", "BNBUSDT", "BTCUSDT", "DOGEUSDT", "DOTUSDT", "ETHUSDT", "LINKUSDT",
    "SOLUSDT", "XRPUSDT",
];

/// Parse `<YEAR> [--root <dir>] [--symbols A,B,C]` from argv.
///
/// Returns `(year, root, symbols)`. Symbols default to `BASELINE_10`; root
/// defaults to `data/binance`. Order of the two flags does not matter.
fn parse_args() -> (i64, PathBuf, Vec<String>) {
    let argv: Vec<String> = std::env::args().collect();
    let mut year: i64 = 2023;
    let mut root = PathBuf::from("data/binance");
    let mut symbols: Vec<String> = BASELINE_10.iter().map(|s| s.to_string()).collect();

    let mut i = 1;
    let mut year_seen = false;
    while i < argv.len() {
        match argv[i].as_str() {
            "--root" => {
                root = PathBuf::from(argv.get(i + 1).expect("--root needs a value"));
                i += 2;
            }
            "--symbols" => {
                symbols = argv
                    .get(i + 1)
                    .expect("--symbols needs a comma-separated value")
                    .split(',')
                    .map(|s| s.trim().to_uppercase())
                    .filter(|s| !s.is_empty())
                    .collect();
                i += 2;
            }
            other => {
                // First bare positional is the year (preserves `-- 2024`).
                if !year_seen {
                    year = other.parse().expect("year must be an integer");
                    year_seen = true;
                }
                i += 1;
            }
        }
    }
    assert!(symbols.len() >= 2, "need at least 2 symbols to correlate");
    (year, root, symbols)
}

fn year_bounds_ms(year: i64) -> (i64, i64) {
    // [Y-01-01T00:00:00Z, (Y+1)-01-01T00:00:00Z) via time crate parity with realdata.rs.
    use time::{Date, Month, OffsetDateTime, Time};
    let start = OffsetDateTime::new_utc(
        Date::from_calendar_date(year as i32, Month::January, 1).unwrap(),
        Time::MIDNIGHT,
    )
    .unix_timestamp()
        * 1_000;
    let end = OffsetDateTime::new_utc(
        Date::from_calendar_date((year + 1) as i32, Month::January, 1).unwrap(),
        Time::MIDNIGHT,
    )
    .unix_timestamp()
        * 1_000;
    (start, end)
}

fn main() {
    let (year, root, universe) = parse_args();
    let (start_ms, end_ms) = year_bounds_ms(year);

    let feed = ReplayFeed::new(&root, true);
    let symbol_paths: Vec<(Symbol, PathBuf)> = universe
        .iter()
        .map(|s| (Symbol::new(s.as_str()), root.clone()))
        .collect();

    let mut bars: Vec<Bar> = feed
        .merge_symbols(&symbol_paths, Timeframe::OneHour)
        .expect("merge_symbols");
    bars.retain(|b| {
        let ts = b.open_ts.0.unix_timestamp() * 1_000;
        ts >= start_ms && ts < end_ms
    });

    // Group close prices by symbol, keyed on open_ts (ms) so we can align an
    // intersection grid across all 10 names (defensive against any missing bars).
    let mut by_sym: BTreeMap<String, BTreeMap<i64, f64>> = BTreeMap::new();
    for b in &bars {
        let ts = b.open_ts.0.unix_timestamp() * 1_000;
        let close: f64 = b.close.get().to_f64().unwrap_or(0.0);
        by_sym
            .entry(b.symbol.0.to_string())
            .or_default()
            .insert(ts, close);
    }

    // Intersection of timestamps present for ALL symbols in the universe.
    let mut common_ts: Vec<i64> = by_sym
        .get(&universe[0])
        .expect("first symbol present")
        .keys()
        .copied()
        .collect();
    for s in &universe[1..] {
        let m = by_sym.get(s).expect("symbol present");
        common_ts.retain(|t| m.contains_key(t));
    }
    common_ts.sort_unstable();

    // Build aligned log-return matrix: rows = time (T-1), cols = symbols.
    let n_t = common_ts.len();
    let n_sym = universe.len();
    let mut rets: Vec<Vec<f64>> = vec![Vec::with_capacity(n_t.saturating_sub(1)); n_sym];
    for (j, s) in universe.iter().enumerate() {
        let m = by_sym.get(s).unwrap();
        for w in common_ts.windows(2) {
            let p0 = m[&w[0]];
            let p1 = m[&w[1]];
            rets[j].push((p1 / p0).ln());
        }
    }
    let n_ret = rets[0].len();

    // ── Metric 1: average pairwise Pearson correlation (45 unique pairs) ──
    let means: Vec<f64> = rets
        .iter()
        .map(|r| r.iter().sum::<f64>() / r.len() as f64)
        .collect();
    let stds: Vec<f64> = rets
        .iter()
        .zip(&means)
        .map(|(r, m)| (r.iter().map(|x| (x - m).powi(2)).sum::<f64>() / r.len() as f64).sqrt())
        .collect();

    let corr = |a: usize, b: usize| -> f64 {
        let (ra, rb) = (&rets[a], &rets[b]);
        let cov: f64 = ra
            .iter()
            .zip(rb)
            .map(|(x, y)| (x - means[a]) * (y - means[b]))
            .sum::<f64>()
            / ra.len() as f64;
        cov / (stds[a] * stds[b])
    };

    let mut pair_corrs: Vec<f64> = Vec::new();
    let mut min_pair = (f64::MAX, "", "");
    let mut max_pair = (f64::MIN, "", "");
    for i in 0..n_sym {
        for j in (i + 1)..n_sym {
            let c = corr(i, j);
            pair_corrs.push(c);
            if c < min_pair.0 {
                min_pair = (c, universe[i].as_str(), universe[j].as_str());
            }
            if c > max_pair.0 {
                max_pair = (c, universe[i].as_str(), universe[j].as_str());
            }
        }
    }
    let avg_corr = pair_corrs.iter().sum::<f64>() / pair_corrs.len() as f64;

    // ── Metric 2: cross-sectional dispersion over time ──
    // At each timestamp t, std across the 10 names' returns. Report time-mean + pctiles.
    let mut xs_disp: Vec<f64> = Vec::with_capacity(n_ret);
    for t in 0..n_ret {
        let row: Vec<f64> = (0..n_sym).map(|j| rets[j][t]).collect();
        let m = row.iter().sum::<f64>() / n_sym as f64;
        let v = row.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n_sym as f64;
        xs_disp.push(v.sqrt());
    }
    let mut disp_sorted = xs_disp.clone();
    disp_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let pct = |p: f64| disp_sorted[((disp_sorted.len() as f64 - 1.0) * p).round() as usize];
    let disp_mean = xs_disp.iter().sum::<f64>() / xs_disp.len() as f64;

    // ── Metric 3: 1-factor decomposition vs equal-weight index ──
    // Index return at t = mean of the 10 names' returns (equal-weight, the BH proxy).
    let idx: Vec<f64> = (0..n_ret)
        .map(|t| (0..n_sym).map(|j| rets[j][t]).sum::<f64>() / n_sym as f64)
        .collect();
    let idx_mean = idx.iter().sum::<f64>() / idx.len() as f64;
    let idx_var = idx.iter().map(|x| (x - idx_mean).powi(2)).sum::<f64>() / idx.len() as f64;

    // Per-name R^2 = squared correlation with the index (univariate regression R^2).
    let mut r2s: Vec<(f64, &str, f64)> = Vec::new(); // (r2, sym, beta)
    for j in 0..n_sym {
        let cov: f64 = rets[j]
            .iter()
            .zip(&idx)
            .map(|(x, y)| (x - means[j]) * (y - idx_mean))
            .sum::<f64>()
            / n_ret as f64;
        let r = cov / (stds[j] * idx_var.sqrt());
        let beta = cov / idx_var;
        r2s.push((r * r, universe[j].as_str(), beta));
    }
    let avg_r2 = r2s.iter().map(|x| x.0).sum::<f64>() / r2s.len() as f64;

    // ── Metric 4: cross-sectional rank persistence (the METHOD question) ──
    // For each rebalance step, rank the 10 names by trailing-L-bar cumulative
    // return; measure whether the rank at t predicts the FORWARD 1-rebalance-period
    // return (Spearman-style rank IC averaged over time). If IC ≈ 0, x-sec ranking
    // has nothing persistent to exploit (the method is the limiter), independent of
    // signal/parameters. We sample at the momentum tier-1 rebalance cadence (every
    // L bars, L below) to mirror what the harness actually trades.
    let rank_ic = |lookback: usize| -> (f64, usize) {
        // Spearman rank IC between (trailing-lookback cum return rank) and
        // (forward-lookback cum return), averaged over non-overlapping windows.
        let mut ics: Vec<f64> = Vec::new();
        let mut t = lookback;
        while t + lookback < n_ret {
            // trailing cum log-return per name over [t-lookback, t)
            let trail: Vec<f64> = (0..n_sym)
                .map(|j| rets[j][(t - lookback)..t].iter().sum::<f64>())
                .collect();
            // forward cum log-return per name over [t, t+lookback)
            let fwd: Vec<f64> = (0..n_sym)
                .map(|j| rets[j][t..(t + lookback)].iter().sum::<f64>())
                .collect();
            ics.push(spearman(&trail, &fwd));
            t += lookback; // non-overlapping
        }
        let mean = ics.iter().sum::<f64>() / ics.len().max(1) as f64;
        (mean, ics.len())
    };

    // ── Report ──
    let n_pairs = n_sym * (n_sym - 1) / 2;
    println!("=== UNIVERSE STRUCTURE DIAGNOSTIC — {year}-FY (1h log-returns) ===");
    println!("root: {}", root.display());
    println!("symbols ({n_sym}): {}", universe.join(", "));
    println!(
        "aligned bars (intersection across {n_sym} names): {} → {} returns",
        n_t, n_ret
    );
    println!();
    println!("--- M1: pairwise return correlation ({n_pairs} unique pairs) ---");
    println!("  AVG pairwise corr : {avg_corr:.4}");
    println!(
        "  min pair          : {:.4}  ({} / {})",
        min_pair.0, min_pair.1, min_pair.2
    );
    println!(
        "  max pair          : {:.4}  ({} / {})",
        max_pair.0, max_pair.1, max_pair.2
    );
    println!();
    println!("--- M2: cross-sectional return dispersion (std across {n_sym} names per bar) ---");
    println!("  time-mean dispersion : {:.5}  ({:.3}%/bar)", disp_mean, disp_mean * 100.0);
    println!("  p10 / p50 / p90      : {:.5} / {:.5} / {:.5}", pct(0.10), pct(0.50), pct(0.90));
    println!("  ratio: avg single-name return std / avg dispersion = {:.3}", {
        let avg_single = stds.iter().sum::<f64>() / stds.len() as f64;
        avg_single / disp_mean
    });
    println!();
    println!("--- M3: 1-factor (equal-weight index) decomposition ---");
    println!("  AVG R^2 vs EW index : {avg_r2:.4}   ({:.1}% common beta)", avg_r2 * 100.0);
    println!("  per-name R^2 (common-beta share) and beta:");
    let mut sorted_r2 = r2s.clone();
    sorted_r2.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    for (r2, sym, beta) in &sorted_r2 {
        println!(
            "    {sym:9} R^2={r2:.4}  idiosyncratic={:.4}  beta={beta:.3}",
            1.0 - r2
        );
    }
    println!();
    println!("--- M4: cross-sectional rank persistence (rank IC, trailing vs forward) ---");
    for lb in [3usize, 9, 24, 60, 168, 720] {
        let (ic, n) = rank_ic(lb);
        println!(
            "  lookback={lb:>4} bars: mean rank IC = {ic:+.4}   (n={n} non-overlapping windows)"
        );
    }
    println!("  (rank IC ≈ 0 ⇒ relative-strength rank has NO forward persistence ⇒");
    println!("   x-sec ranking is the binding constraint regardless of signal/params)");
    println!();
    println!("--- INTERPRETATION GUIDE ---");
    println!("  avg_corr →1 and avg_R^2 →1  ⇒ ~1 factor ⇒ structural ceiling on x-sec alpha");
    println!("  avg dispersion is the raw material x-sec ranking can exploit (small ⇒ little to harvest)");
    println!("  rank IC is whether the ranking PERSISTS (the method question, signal-agnostic)");
}

/// Spearman rank correlation between two equal-length vectors.
fn spearman(a: &[f64], b: &[f64]) -> f64 {
    let ra = ranks(a);
    let rb = ranks(b);
    let n = ra.len() as f64;
    let ma = ra.iter().sum::<f64>() / n;
    let mb = rb.iter().sum::<f64>() / n;
    let cov: f64 = ra
        .iter()
        .zip(&rb)
        .map(|(x, y)| (x - ma) * (y - mb))
        .sum::<f64>();
    let va: f64 = ra.iter().map(|x| (x - ma).powi(2)).sum::<f64>();
    let vb: f64 = rb.iter().map(|x| (x - mb).powi(2)).sum::<f64>();
    if va == 0.0 || vb == 0.0 {
        0.0
    } else {
        cov / (va.sqrt() * vb.sqrt())
    }
}

/// Fractional ranks (1-based, ties → average rank).
fn ranks(v: &[f64]) -> Vec<f64> {
    let n = v.len();
    let mut idx: Vec<usize> = (0..n).collect();
    idx.sort_by(|&i, &j| v[i].partial_cmp(&v[j]).unwrap());
    let mut r = vec![0.0; n];
    let mut i = 0;
    while i < n {
        let mut k = i;
        while k + 1 < n && v[idx[k + 1]] == v[idx[i]] {
            k += 1;
        }
        // average rank for ties in [i, k]
        let avg = ((i + k) as f64) / 2.0 + 1.0; // 1-based
        for &ii in &idx[i..=k] {
            r[ii] = avg;
        }
        i = k + 1;
    }
    r
}
