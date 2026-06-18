//! THROWAWAY perp-spot-basis predictive-power diagnostic (analyst, 2026-06-05).
//!
//! The perp-spot-basis research spike. Question: is the perpetual **basis** (the
//! premium index = `(markPrice − indexPrice) / indexPrice`) a LIVE, orthogonal
//! signal, or funding's dead twin? Mirrors `universe_diag.rs`: it reads the SAME
//! banked OHLCV via the SAME `ReplayFeed::merge_symbols` path the harness uses,
//! joins the banked **basis** parquets (`data/binance-basis/`, fetched by
//! `fetch_binance_premium`) and the banked **funding** parquets
//! (`data/binance-funding/`) onto the common hourly grid, and computes:
//!
//!   1. BASIS IC (the core question): (a) cross-sectional rank-IC of the
//!      trailing-basis rank vs the forward-return rank across the 10 names at
//!      several horizons; (b) per-asset time-series IC of own trailing-basis vs
//!      own forward-return.
//!   2. ORTHOGONALITY: correlate the trailing-basis signal against (a) the OHLCV
//!      trailing-return / momentum signal, and (b) the funding/carry signal
//!      (as-of funding) — the redundancy check.
//!
//! ## No-look-ahead (strict)
//!
//! The premium-index kline for bar `t` (open_time=t) has its `close` known only
//! at `t + 1h`. So the most-recent FULLY-OBSERVED basis available at the **open**
//! of bar `t` (decision time) is the close of bar `t-1`. The probe therefore uses
//! `basis_close[t-1]` as the as-of basis at the open of bar `t`; the trailing
//! basis "signal" over a window ending at `t` uses bars strictly before `t`.
//! Funding is joined as-of (last settlement at-or-before the bar open) exactly
//! like `funding_data.rs::funding_as_of`. Forward returns use FUTURE bars only
//! (the thing we are trying to predict); signals use PAST bars only.
//!
//! Run (both years; basis + funding banked under data/binance-basis +
//! data/binance-funding):
//!       cargo run -p data --example basis_diag -- 2023
//!       cargo run -p data --example basis_diag -- 2024
//!
//! Optional flags (default to the 10-symbol large-cap baseline + the banked roots):
//!       --ohlcv-root   data/binance        (OHLCV parquets)
//!       --basis-root   data/binance-basis   (premium-index parquets)
//!       --funding-root data/binance-funding (funding parquets)
//!       --symbols      <comma-separated>    (default BASELINE_10)
//!
//! NOT committed as a bin; not anchored; pure read-only over banked data.

// Pairwise/positional loops over parallel arrays; explicit indexing is clearer
// here than iterator adaptors (disposable probe, mirrors universe_diag.rs).
#![allow(clippy::needless_range_loop)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use data::ReplayFeed;
use polars::prelude::{LazyFrame, ScanArgsParquet};
use rust_decimal::prelude::ToPrimitive;
use trading_core::{Bar, Symbol, Timeframe};

/// The original 10-symbol large-cap baseline (used when `--symbols` is omitted).
const BASELINE_10: [&str; 10] = [
    "ADAUSDT", "AVAXUSDT", "BNBUSDT", "BTCUSDT", "DOGEUSDT", "DOTUSDT", "ETHUSDT", "LINKUSDT",
    "SOLUSDT", "XRPUSDT",
];

struct Args {
    year: i64,
    ohlcv_root: PathBuf,
    basis_root: PathBuf,
    funding_root: PathBuf,
    symbols: Vec<String>,
    /// No-look-ahead falsifier: also recompute B1 using a basis signal shifted
    /// +L bars into the FUTURE (deliberate leak). The causal B1 (past-only) MUST
    /// differ from the leaked B1; if they matched, the legit join would be using
    /// future basis. Prints a side-by-side B1-causal vs B1-leaked table.
    leak_check: bool,
}

fn parse_args() -> Args {
    let argv: Vec<String> = std::env::args().collect();
    let mut year: i64 = 2023;
    let mut ohlcv_root = PathBuf::from("data/binance");
    let mut basis_root = PathBuf::from("data/binance-basis");
    let mut leak_check = false;
    let mut funding_root = PathBuf::from("data/binance-funding");
    let mut symbols: Vec<String> = BASELINE_10.iter().map(|s| s.to_string()).collect();

    let mut i = 1;
    let mut year_seen = false;
    while i < argv.len() {
        match argv[i].as_str() {
            "--ohlcv-root" => {
                ohlcv_root = PathBuf::from(argv.get(i + 1).expect("--ohlcv-root needs a value"));
                i += 2;
            }
            "--basis-root" => {
                basis_root = PathBuf::from(argv.get(i + 1).expect("--basis-root needs a value"));
                i += 2;
            }
            "--funding-root" => {
                funding_root =
                    PathBuf::from(argv.get(i + 1).expect("--funding-root needs a value"));
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
            "--leak-check" => {
                leak_check = true;
                i += 1;
            }
            other => {
                if !year_seen {
                    year = other.parse().expect("year must be an integer");
                    year_seen = true;
                }
                i += 1;
            }
        }
    }
    assert!(symbols.len() >= 2, "need at least 2 symbols");
    Args {
        year,
        ohlcv_root,
        basis_root,
        funding_root,
        symbols,
        leak_check,
    }
}

fn year_bounds_ms(year: i64) -> (i64, i64) {
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

/// Read all banked basis parquets for one symbol+year into a `ts_ms -> basis_close`
/// map. Layout `<root>/<SYM>/<YEAR>/<MM>.parquet`, schema produced by
/// `fetch_binance_premium`: `open_time` Int64, `basis_close` Utf8 (signed decimal).
fn load_basis_close(root: &Path, sym: &str, year: i64) -> BTreeMap<i64, f64> {
    let mut out: BTreeMap<i64, f64> = BTreeMap::new();
    for month in 1..=12u32 {
        let path = root
            .join(sym)
            .join(year.to_string())
            .join(format!("{month:02}.parquet"));
        if !path.exists() {
            continue;
        }
        let df = LazyFrame::scan_parquet(&path, ScanArgsParquet::default())
            .and_then(LazyFrame::collect)
            .unwrap_or_else(|e| panic!("scan basis {}: {e}", path.display()));
        let ots = df.column("open_time").unwrap().i64().unwrap();
        let bc = df.column("basis_close").unwrap().str().unwrap();
        for i in 0..df.height() {
            let ts = ots.get(i).unwrap_or(0);
            let v: f64 = bc.get(i).unwrap_or("0").parse().unwrap_or(0.0);
            out.insert(ts, v);
        }
    }
    out
}

/// Read all banked funding parquets for one symbol+year into a SORTED
/// `(funding_time_ms, rate)` vec. Layout/schema from `fetch_binance_funding`:
/// `funding_time` Int64, `funding_rate` Utf8.
fn load_funding(root: &Path, sym: &str, year: i64) -> Vec<(i64, f64)> {
    let mut out: Vec<(i64, f64)> = Vec::new();
    // Funding settled late in the prior year can be the as-of value for an early
    // January bar; include the prior December for a correct warm-up join.
    let spans: [(i64, u32); 2] = [(year - 1, 12), (year, 0)];
    for (yr, only_month) in spans {
        let months: Vec<u32> = if only_month == 0 {
            (1..=12).collect()
        } else {
            vec![only_month]
        };
        for month in months {
            let path = root
                .join(sym)
                .join(yr.to_string())
                .join(format!("{month:02}.parquet"));
            if !path.exists() {
                continue;
            }
            let df = LazyFrame::scan_parquet(&path, ScanArgsParquet::default())
                .and_then(LazyFrame::collect)
                .unwrap_or_else(|e| panic!("scan funding {}: {e}", path.display()));
            let ft = df.column("funding_time").unwrap().i64().unwrap();
            let fr = df.column("funding_rate").unwrap().str().unwrap();
            for i in 0..df.height() {
                let t = ft.get(i).unwrap_or(0);
                let r: f64 = fr.get(i).unwrap_or("0").parse().unwrap_or(0.0);
                out.push((t, r));
            }
        }
    }
    out.sort_unstable_by_key(|&(t, _)| t);
    out
}

// PIT: research-grade f64 mirror of trading_core::pit::PitSeries; NaN = warm-up (None in the Decimal API).
/// As-of funding at a bar open: the rate of the last settlement at-or-before
/// `bar_ts`. `None` (→ NaN here) before the first settlement. Mirrors
/// `funding_data.rs::funding_as_of`.
fn funding_as_of(funding: &[(i64, f64)], bar_ts: i64) -> f64 {
    let idx = funding.partition_point(|&(t, _)| t <= bar_ts);
    if idx == 0 {
        f64::NAN
    } else {
        funding[idx - 1].1
    }
}

fn main() {
    let args = parse_args();
    let (start_ms, end_ms) = year_bounds_ms(args.year);
    let universe = &args.symbols;
    let n_sym = universe.len();

    // ── Load OHLCV via the harness reader (same path as realdata.rs) ──
    let feed = ReplayFeed::new(&args.ohlcv_root, true);
    let symbol_paths: Vec<(Symbol, PathBuf)> = universe
        .iter()
        .map(|s| (Symbol::new(s.as_str()), args.ohlcv_root.clone()))
        .collect();
    let mut bars: Vec<Bar> = feed
        .merge_symbols(&symbol_paths, Timeframe::OneHour)
        .expect("merge_symbols");
    bars.retain(|b| {
        let ts = b.open_ts.0.unix_timestamp() * 1_000;
        ts >= start_ms && ts < end_ms
    });

    // close price by symbol, keyed on open_ts(ms).
    let mut close_by_sym: BTreeMap<String, BTreeMap<i64, f64>> = BTreeMap::new();
    for b in &bars {
        let ts = b.open_ts.0.unix_timestamp() * 1_000;
        let c: f64 = b.close.get().to_f64().unwrap_or(0.0);
        close_by_sym
            .entry(b.symbol.0.to_string())
            .or_default()
            .insert(ts, c);
    }

    // ── Load basis + funding per symbol ──
    let mut basis_by_sym: BTreeMap<String, BTreeMap<i64, f64>> = BTreeMap::new();
    let mut funding_by_sym: BTreeMap<String, Vec<(i64, f64)>> = BTreeMap::new();
    for s in universe {
        basis_by_sym.insert(s.clone(), load_basis_close(&args.basis_root, s, args.year));
        funding_by_sym.insert(s.clone(), load_funding(&args.funding_root, s, args.year));
    }

    // ── Common timestamp grid: bars present in OHLCV AND basis for ALL names ──
    let mut common_ts: Vec<i64> = close_by_sym
        .get(&universe[0])
        .expect("first symbol OHLCV present")
        .keys()
        .copied()
        .collect();
    for s in universe {
        let cm = close_by_sym.get(s).expect("OHLCV present");
        let bm = basis_by_sym.get(s).expect("basis present");
        common_ts.retain(|t| cm.contains_key(t) && bm.contains_key(t));
    }
    common_ts.sort_unstable();
    let n_t = common_ts.len();
    assert!(n_t > 800, "too few aligned bars: {n_t} (is basis banked?)");

    // ── Aligned matrices on the common grid ──
    // price[j][k], basis[j][k], funding[j][k] for k in 0..n_t (per bar open).
    let mut price: Vec<Vec<f64>> = vec![Vec::with_capacity(n_t); n_sym];
    let mut basis: Vec<Vec<f64>> = vec![Vec::with_capacity(n_t); n_sym];
    let mut fund: Vec<Vec<f64>> = vec![Vec::with_capacity(n_t); n_sym];
    for (j, s) in universe.iter().enumerate() {
        let cm = close_by_sym.get(s).unwrap();
        let bm = basis_by_sym.get(s).unwrap();
        let fv = funding_by_sym.get(s).unwrap();
        for &t in &common_ts {
            price[j].push(cm[&t]);
            basis[j].push(bm[&t]);
            fund[j].push(funding_as_of(fv, t));
        }
    }

    // log-returns: ret[j][k] = ln(price[k+1]/price[k]), length n_t-1.
    let n_ret = n_t - 1;
    let mut ret: Vec<Vec<f64>> = vec![Vec::with_capacity(n_ret); n_sym];
    for j in 0..n_sym {
        for k in 0..n_ret {
            ret[j].push((price[j][k + 1] / price[j][k]).ln());
        }
    }

    // basis-funding coverage report (how much funding is NaN warm-up).
    let funding_nan: usize = fund
        .iter()
        .flat_map(|v| v.iter())
        .filter(|x| x.is_nan())
        .count();

    // ── Report header ──
    println!(
        "=== PERP-SPOT-BASIS DIAGNOSTIC — {}-FY (1h grid) ===",
        args.year
    );
    println!("ohlcv: {}", args.ohlcv_root.display());
    println!("basis: {}", args.basis_root.display());
    println!("funding: {}", args.funding_root.display());
    println!("symbols ({n_sym}): {}", universe.join(", "));
    println!(
        "aligned bars (OHLCV ∩ basis, all {n_sym} names): {n_t} → {n_ret} returns  (funding NaN cells: {funding_nan})"
    );
    println!();

    // Quick basis level sanity (per-name mean/min/max of the basis_close, %).
    println!("--- B0: basis level sanity (basis_close, in %) ---");
    for (j, s) in universe.iter().enumerate() {
        let v = &basis[j];
        let mean = v.iter().sum::<f64>() / v.len() as f64;
        let min = v.iter().cloned().fold(f64::MAX, f64::min);
        let max = v.iter().cloned().fold(f64::MIN, f64::max);
        println!(
            "  {s:9} mean={:+.4}%  min={:+.4}%  max={:+.4}%",
            mean * 100.0,
            min * 100.0,
            max * 100.0
        );
    }
    println!();

    // ── B1: cross-sectional rank-IC of trailing-basis vs forward-return ──
    // At each non-overlapping step t, rank the names by the trailing-mean basis
    // observed STRICTLY BEFORE t (mean of basis over [t-L, t), i.e. fully-observed
    // bars), and correlate (Spearman) vs the forward-L cumulative return [t, t+L).
    println!("--- B1: cross-sectional rank-IC (trailing-basis rank vs forward-return) ---");
    let xsec_basis_ic = |lookback: usize| -> (f64, usize) {
        let mut ics: Vec<f64> = Vec::new();
        let mut t = lookback;
        while t + lookback <= n_ret {
            // trailing-mean basis per name over [t-L, t) — past bars only.
            let sig: Vec<f64> = (0..n_sym)
                .map(|j| {
                    let w = &basis[j][(t - lookback)..t];
                    w.iter().sum::<f64>() / w.len() as f64
                })
                .collect();
            // forward cum log-return over [t, t+L).
            let fwd: Vec<f64> = (0..n_sym)
                .map(|j| ret[j][t..(t + lookback)].iter().sum::<f64>())
                .collect();
            ics.push(spearman(&sig, &fwd));
            t += lookback;
        }
        let mean = ics.iter().sum::<f64>() / ics.len().max(1) as f64;
        (mean, ics.len())
    };
    for lb in [3usize, 9, 24, 60, 168, 720] {
        let (ic, n) = xsec_basis_ic(lb);
        println!("  lookback={lb:>4} bars: mean x-sec rank IC = {ic:+.4}   (n={n} windows)");
    }
    println!();

    // ── No-look-ahead falsifier (optional, --leak-check) ──
    // Recompute B1 using a CONTEMPORANEOUS basis signal — the mean basis over the
    // SAME forward window [t, t+L) that the forward return spans (a deliberate
    // look-ahead leak). The causal B1 (trailing [t-L, t)) MUST differ from this
    // leaked version; a match would mean the legit join is using future basis.
    if args.leak_check {
        println!(
            "--- B1-LEAK: no-look-ahead falsifier (causal trailing vs leaked contemporaneous) ---"
        );
        let xsec_basis_ic_leaked = |lookback: usize| -> f64 {
            let mut ics: Vec<f64> = Vec::new();
            let mut t = lookback;
            while t + lookback <= n_ret {
                // LEAK: basis over the forward window [t, t+L) (look-ahead).
                let sig: Vec<f64> = (0..n_sym)
                    .map(|j| {
                        let w = &basis[j][t..(t + lookback)];
                        w.iter().sum::<f64>() / w.len() as f64
                    })
                    .collect();
                let fwd: Vec<f64> = (0..n_sym)
                    .map(|j| ret[j][t..(t + lookback)].iter().sum::<f64>())
                    .collect();
                ics.push(spearman(&sig, &fwd));
                t += lookback;
            }
            ics.iter().sum::<f64>() / ics.len().max(1) as f64
        };
        for lb in [3usize, 9, 24, 60, 168, 720] {
            let (causal, _) = xsec_basis_ic(lb);
            let leaked = xsec_basis_ic_leaked(lb);
            let differ = (causal - leaked).abs() > 1e-9;
            println!(
                "  lookback={lb:>4} bars: causal={causal:+.4}  leaked(contemporaneous)={leaked:+.4}  differ={differ}"
            );
        }
        println!("  (causal MUST differ from leaked at every horizon ⇒ B1 uses past-only basis)");
        println!();
    }

    // ── B2: per-asset time-series IC (own trailing-basis vs own forward-return) ──
    // Pearson IC between trailing-mean basis [t-L,t) and forward-L return [t,t+L),
    // pooled across all non-overlapping windows AND all names (time-series, NOT
    // cross-sectional). Also report the per-name spread.
    println!("--- B2: per-asset time-series IC (own trailing-basis vs own forward-return) ---");
    let ts_basis_ic = |lookback: usize| -> (f64, f64, f64, usize) {
        // returns (pooled_ic, min_name_ic, max_name_ic, n_samples_pooled)
        let mut per_name: Vec<(f64, f64)> = Vec::new(); // (ic, _) per name
        let mut pooled_x: Vec<f64> = Vec::new();
        let mut pooled_y: Vec<f64> = Vec::new();
        for j in 0..n_sym {
            let mut xs: Vec<f64> = Vec::new();
            let mut ys: Vec<f64> = Vec::new();
            let mut t = lookback;
            while t + lookback <= n_ret {
                let w = &basis[j][(t - lookback)..t];
                let sig = w.iter().sum::<f64>() / w.len() as f64;
                let fwd = ret[j][t..(t + lookback)].iter().sum::<f64>();
                xs.push(sig);
                ys.push(fwd);
                t += lookback;
            }
            let ic = pearson(&xs, &ys);
            per_name.push((ic, 0.0));
            pooled_x.extend(&xs);
            pooled_y.extend(&ys);
        }
        let pooled_ic = pearson(&pooled_x, &pooled_y);
        let min_ic = per_name.iter().map(|x| x.0).fold(f64::MAX, f64::min);
        let max_ic = per_name.iter().map(|x| x.0).fold(f64::MIN, f64::max);
        (pooled_ic, min_ic, max_ic, pooled_x.len())
    };
    for lb in [3usize, 9, 24, 60, 168, 720] {
        let (ic, lo, hi, n) = ts_basis_ic(lb);
        println!(
            "  lookback={lb:>4} bars: pooled TS IC = {ic:+.4}   per-name[min={lo:+.4}, max={hi:+.4}]  (n={n} pooled)"
        );
    }
    println!();

    // ── B3: orthogonality of the trailing-basis signal vs OHLCV-momentum + funding ──
    // For each horizon L, build per-(name,time) aligned triples:
    //   basis_sig  = trailing-mean basis over [t-L, t)          (past basis)
    //   mom_sig    = trailing-L cum return over [t-L, t)        (OHLCV momentum)
    //   fund_sig   = as-of funding at bar t (already past-only)
    // and correlate basis_sig vs mom_sig, and basis_sig vs fund_sig, pooled over
    // all names + all non-overlapping windows. corr≈±1 vs funding ⇒ redundant twin.
    println!("--- B3: orthogonality (trailing-basis vs OHLCV-momentum, and vs funding) ---");
    let orthogonality = |lookback: usize| -> (f64, f64, usize) {
        let mut b_sig: Vec<f64> = Vec::new();
        let mut m_sig: Vec<f64> = Vec::new();
        let mut f_sig: Vec<f64> = Vec::new();
        for j in 0..n_sym {
            let mut t = lookback;
            while t + lookback <= n_ret {
                let w = &basis[j][(t - lookback)..t];
                let bs = w.iter().sum::<f64>() / w.len() as f64;
                let ms = ret[j][(t - lookback)..t].iter().sum::<f64>();
                let fs = fund[j][t]; // as-of funding at decision bar t
                b_sig.push(bs);
                m_sig.push(ms);
                f_sig.push(fs);
                t += lookback;
            }
        }
        // funding may carry NaN warm-up; mask pairwise for the basis-vs-funding corr.
        let bf = pearson_masked(&b_sig, &f_sig);
        let bm = pearson(&b_sig, &m_sig);
        (bm, bf, b_sig.len())
    };
    for lb in [3usize, 9, 24, 60, 168, 720] {
        let (bm, bf, n) = orthogonality(lb);
        println!(
            "  lookback={lb:>4} bars: corr(basis, OHLCV-mom) = {bm:+.4}   corr(basis, funding) = {bf:+.4}  (n={n})"
        );
    }
    println!();

    // ── B4: contemporaneous basis↔funding redundancy (the twin check, level-on-level) ──
    // Pool the as-of funding and the as-of basis (basis_close[t-1], the last
    // fully-observed basis at bar t's open) across all names+bars and correlate.
    println!("--- B4: as-of basis ↔ as-of funding redundancy (level, all names+bars) ---");
    {
        let mut bvec: Vec<f64> = Vec::new();
        let mut fvec: Vec<f64> = Vec::new();
        for j in 0..n_sym {
            // basis as-of at bar t open = basis_close of bar t-1 (fully observed).
            for k in 1..n_t {
                bvec.push(basis[j][k - 1]);
                fvec.push(fund[j][k]);
            }
        }
        let c = pearson_masked(&bvec, &fvec);
        println!(
            "  corr(as-of basis_close[t-1], as-of funding[t]) = {c:+.4}  (n={})",
            bvec.len()
        );
        println!("  (≈ +1 ⇒ basis is funding's redundant twin; |corr| small ⇒ distinct quantity)");
    }
    println!();

    println!("--- INTERPRETATION GUIDE ---");
    println!(
        "  B1/B2 ≈ 0 (|IC|<~0.03, no stable sign, both years) ⇒ basis carries ~no forward info"
    );
    println!(
        "  B3/B4 corr(basis,funding) ≈ +1 ⇒ basis ≈ funding (redundant; carry already FRAGILE)"
    );
    println!("  LIVE iff (B1 or B2) persistently |IC|≥0.03 AND meaningfully orthogonal to funding");
}

// ── stats helpers ──────────────────────────────────────────────────────────────

/// Pearson correlation of two equal-length vectors (no NaN handling).
fn pearson(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len();
    if n == 0 {
        return f64::NAN;
    }
    let ma = a.iter().sum::<f64>() / n as f64;
    let mb = b.iter().sum::<f64>() / n as f64;
    let mut cov = 0.0;
    let mut va = 0.0;
    let mut vb = 0.0;
    for i in 0..n {
        let da = a[i] - ma;
        let db = b[i] - mb;
        cov += da * db;
        va += da * da;
        vb += db * db;
    }
    if va == 0.0 || vb == 0.0 {
        0.0
    } else {
        cov / (va.sqrt() * vb.sqrt())
    }
}

/// Pearson correlation, skipping pairs where either side is NaN (funding warm-up).
fn pearson_masked(a: &[f64], b: &[f64]) -> f64 {
    let mut xa: Vec<f64> = Vec::with_capacity(a.len());
    let mut xb: Vec<f64> = Vec::with_capacity(b.len());
    for i in 0..a.len() {
        if a[i].is_nan() || b[i].is_nan() {
            continue;
        }
        xa.push(a[i]);
        xb.push(b[i]);
    }
    pearson(&xa, &xb)
}

/// Spearman rank correlation between two equal-length vectors.
fn spearman(a: &[f64], b: &[f64]) -> f64 {
    let ra = ranks(a);
    let rb = ranks(b);
    pearson(&ra, &rb)
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
        let avg = ((i + k) as f64) / 2.0 + 1.0;
        for &ii in &idx[i..=k] {
            r[ii] = avg;
        }
        i = k + 1;
    }
    r
}
