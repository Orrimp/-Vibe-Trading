//! THROWAWAY on-chain STABLECOIN-SUPPLY predictive-power diagnostic (analyst,
//! 2026-06-08).
//!
//! The on-chain research spike, PIVOTED to stablecoin-supply. Exchange net-flows
//! were ruled out at the data-feasibility / PIT gate: the canonical free source
//! (CryptoQuant) requires a PAID API key AND its own docs disclaim point-in-time
//! accuracy ("does not support Point-In-Time accuracy due to periodic updates to
//! wallet address clustering. Historical data may change as new exchange wallets
//! are discovered"). That is the address-relabeling look-ahead the fork note
//! pre-registered as the net-flow killer. The pre-named cleaner-PIT fallback is
//! STABLECOIN SUPPLY: mint/burn at the issuer contract is immutable on-chain, and
//! DefiLlama records a forward-dated daily snapshot per chain (a new chain's
//! series begins at its launch date — verified: Base chart starts 2023-08-15, its
//! mainnet-launch week, with zero pre-launch backfill).
//!
//! Hypothesis: rising stablecoin supply = "dry powder" entering the system =
//! latent buying capacity → leads forward price. Two framings the data supports:
//!   (1) PER-CHAIN time-series: Δ stablecoin-supply ON a chain → forward return of
//!       that chain's native token. Buildable universe = chains with meaningful
//!       full-2023-2024 supply: Ethereum(ETH), BSC(BNB), Solana(SOL),
//!       Avalanche(AVAX). (ADA/DOT/XRP/DOGE supply is negligible or post-2024.)
//!   (2) AGGREGATE: Δ total-stablecoin-supply → forward BROAD-MARKET return
//!       (proxied by BTC, which has no native stablecoin chain).
//!
//! This is a CROSS-CHANNEL probe: a new on-chain signal is only interesting if it
//! is orthogonal to BOTH channels already shown dead — price-momentum AND funding.
//! So B3 correlates the stablecoin signal against both.
//!
//! ## Data source (free, no auth, daily, full history; NO fabrication)
//!
//! DefiLlama stablecoins API (`https://stablecoins.llama.fi`), two endpoints:
//! `/stablecoincharts/{chain}` (daily per-chain `totalCirculatingUSD.peggedUSD`)
//! and `/stablecoincharts/all` (daily aggregate across all chains). Computed by
//! DefiLlama as mints-minus-burns at issuer contracts (immutable on-chain).
//! Fetched live by this probe; the fetched series is banked to
//! `data/defillama-stablecoins/<chain>.parquet` plus a REVISION.toml pin on first
//! run (mirrors data/binance-basis discipline: parquets gitignored, pin tracked).
//!
//! ## No-look-ahead (strict) — THE GATE
//!
//! Stablecoin supply for UTC-day D is the snapshot at the END of day D (DefiLlama
//! timestamps each point at 00:00 UTC of the day; the totalSupply read reflects
//! state through that day). The most-recent supply FULLY OBSERVABLE at the open of
//! trading day D is therefore day D-1's snapshot. The probe uses `supply[D-1]` as
//! the as-of supply at the open of day D; the trailing supply-CHANGE signal over a
//! window ending at D uses snapshots strictly before D. Forward returns use FUTURE
//! days only. The `--leak-check` flag recomputes the IC with a deliberately leaked
//! (contemporaneous) supply window; causal MUST differ from leaked.
//!
//! Run (banked OHLCV under data/binance, funding under data/binance-funding):
//!       cargo run -p data --example stablecoin_diag -- 2023
//!       cargo run -p data --example stablecoin_diag -- 2024
//!       cargo run -p data --example stablecoin_diag -- 2024 --leak-check
//!
//! NOT committed as a bin; not anchored; read-only over banked data + one HTTP
//! pull of a free public endpoint.

// Pairwise/positional loops over parallel arrays; explicit indexing is clearer
// here than iterator adaptors (disposable probe, mirrors basis_diag.rs).
#![allow(clippy::needless_range_loop)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use data::ReplayFeed;
use polars::prelude::*;
use rust_decimal::prelude::ToPrimitive;
use trading_core::{Bar, Symbol, Timeframe};

/// Universe of (OHLCV symbol, DefiLlama chain) pairs with meaningful, full
/// 2023-2024 native stablecoin supply. ETH/BNB/SOL/AVAX only — the honest
/// buildable per-chain universe (verified supply magnitudes 2023-01-02:
/// 85B / 9.3B / 1.8B / 1.6B). BTC/LINK have no native stablecoin chain;
/// ADA/DOT/XRP/DOGE supply is negligible or starts after 2024.
const CHAIN_UNIVERSE: [(&str, &str); 4] = [
    ("ETHUSDT", "Ethereum"),
    ("BNBUSDT", "BSC"),
    ("SOLUSDT", "Solana"),
    ("AVAXUSDT", "Avalanche"),
];

const DEFILLAMA_BASE: &str = "https://stablecoins.llama.fi";
const SECONDS_PER_DAY: i64 = 86_400;

struct Args {
    year: i64,
    ohlcv_root: PathBuf,
    funding_root: PathBuf,
    bank_root: PathBuf,
    leak_check: bool,
    refetch: bool,
}

fn parse_args() -> Args {
    let argv: Vec<String> = std::env::args().collect();
    let mut year: i64 = 2023;
    let mut ohlcv_root = PathBuf::from("data/binance");
    let mut funding_root = PathBuf::from("data/binance-funding");
    let mut bank_root = PathBuf::from("data/defillama-stablecoins");
    let mut leak_check = false;
    let mut refetch = false;
    let mut i = 1;
    let mut year_seen = false;
    while i < argv.len() {
        match argv[i].as_str() {
            "--ohlcv-root" => {
                ohlcv_root = PathBuf::from(argv.get(i + 1).expect("--ohlcv-root needs a value"));
                i += 2;
            }
            "--funding-root" => {
                funding_root =
                    PathBuf::from(argv.get(i + 1).expect("--funding-root needs a value"));
                i += 2;
            }
            "--bank-root" => {
                bank_root = PathBuf::from(argv.get(i + 1).expect("--bank-root needs a value"));
                i += 2;
            }
            "--leak-check" => {
                leak_check = true;
                i += 1;
            }
            "--refetch" => {
                refetch = true;
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
    Args {
        year,
        ohlcv_root,
        funding_root,
        bank_root,
        leak_check,
        refetch,
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

/// UTC-day bucket key (days since epoch) for a millisecond timestamp.
fn day_key_ms(ts_ms: i64) -> i64 {
    ts_ms.div_euclid(SECONDS_PER_DAY * 1_000)
}

// ── DefiLlama stablecoin supply (fetch + bank) ─────────────────────────────────

/// Daily `{day_key -> totalCirculatingUSD}` for one chain. Banks a parquet
/// snapshot under `<bank_root>/<chain>.parquet` (day_key i64, supply_usd f64) on
/// first run; reuses it thereafter (so re-runs are deterministic + offline). The
/// fetched JSON is the only network I/O.
fn load_or_fetch_chain_supply(
    bank_root: &Path,
    chain: &str,
    refetch: bool,
) -> BTreeMap<i64, f64> {
    let path = bank_root.join(format!("{chain}.parquet"));
    if path.exists() && !refetch {
        return read_supply_parquet(&path);
    }
    let series = fetch_chain_supply(chain);
    std::fs::create_dir_all(bank_root).expect("create bank dir");
    write_supply_parquet(&path, &series);
    series
}

/// HTTP GET the DefiLlama daily stablecoin chart for a chain (or "all"), parse
/// to `{day_key -> totalCirculatingUSD.peggedUSD}`.
fn fetch_chain_supply(chain: &str) -> BTreeMap<i64, f64> {
    let url = if chain == "all" {
        format!("{DEFILLAMA_BASE}/stablecoincharts/all")
    } else {
        format!("{DEFILLAMA_BASE}/stablecoincharts/{chain}")
    };
    // reqwest's `blocking` feature is not enabled workspace-wide; drive the async
    // client on a tiny current-thread runtime (tokio full features ARE available
    // to this crate). Spike-local — no workspace Cargo.toml change.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("rt");
    let body = rt.block_on(async {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .user_agent("stablecoin-spike-diag")
            .build()
            .expect("client")
            .get(&url)
            .send()
            .await
            .unwrap_or_else(|e| panic!("GET {url}: {e}"))
            .text()
            .await
            .unwrap_or_else(|e| panic!("body {url}: {e}"))
    });
    let v: serde_json::Value =
        serde_json::from_str(&body).unwrap_or_else(|e| panic!("json {url}: {e}"));
    let arr = v.as_array().expect("chart is an array");
    let mut out: BTreeMap<i64, f64> = BTreeMap::new();
    for p in arr {
        let date = p
            .get("date")
            .and_then(|d| d.as_str().map(|s| s.parse::<i64>().ok()).unwrap_or(d.as_i64()))
            .expect("date");
        let supply = p
            .get("totalCirculatingUSD")
            .and_then(|t| t.get("peggedUSD"))
            .and_then(|x| x.as_f64())
            .unwrap_or(0.0);
        out.insert(date.div_euclid(SECONDS_PER_DAY), supply);
    }
    out
}

fn write_supply_parquet(path: &Path, series: &BTreeMap<i64, f64>) {
    let days: Vec<i64> = series.keys().copied().collect();
    let supply: Vec<f64> = series.values().copied().collect();
    let mut df = DataFrame::new(vec![
        Series::new("day_key".into(), days).into(),
        Series::new("supply_usd".into(), supply).into(),
    ])
    .expect("df");
    let mut f = std::fs::File::create(path).expect("create parquet");
    ParquetWriter::new(&mut f).finish(&mut df).expect("write parquet");
}

fn read_supply_parquet(path: &Path) -> BTreeMap<i64, f64> {
    let df = LazyFrame::scan_parquet(path, ScanArgsParquet::default())
        .and_then(LazyFrame::collect)
        .unwrap_or_else(|e| panic!("scan {}: {e}", path.display()));
    let dk = df.column("day_key").unwrap().i64().unwrap();
    let su = df.column("supply_usd").unwrap().f64().unwrap();
    let mut out = BTreeMap::new();
    for i in 0..df.height() {
        out.insert(dk.get(i).unwrap_or(0), su.get(i).unwrap_or(0.0));
    }
    out
}

// ── funding (as-of), mirrors basis_diag.rs ─────────────────────────────────────

fn load_funding(root: &Path, sym: &str, year: i64) -> Vec<(i64, f64)> {
    let mut out: Vec<(i64, f64)> = Vec::new();
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

/// As-of funding at a bar open: the rate of the last settlement at-or-before
/// `bar_ts`. Mirrors `funding_data.rs::funding_as_of`.
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

    // ── Load OHLCV (1h) via the harness reader, fold to DAILY closes per symbol ──
    // Daily close = the close of the last 1h bar in each UTC day. Daily funding
    // as-of = funding settled at-or-before the day's 00:00 UTC open.
    let feed = ReplayFeed::new(&args.ohlcv_root, true);
    let all_syms: Vec<&str> = CHAIN_UNIVERSE.iter().map(|(s, _)| *s).collect();
    // include BTC for the aggregate-vs-market leg.
    let mut ohlcv_syms = all_syms.clone();
    ohlcv_syms.push("BTCUSDT");

    let symbol_paths: Vec<(Symbol, PathBuf)> = ohlcv_syms
        .iter()
        .map(|s| (Symbol::new(*s), args.ohlcv_root.clone()))
        .collect();
    let mut bars: Vec<Bar> = feed
        .merge_symbols(&symbol_paths, Timeframe::OneHour)
        .expect("merge_symbols");
    bars.retain(|b| {
        let ts = b.open_ts.0.unix_timestamp() * 1_000;
        ts >= start_ms && ts < end_ms
    });

    // daily close per symbol: day_key -> last close in that day.
    let mut dayclose: BTreeMap<String, BTreeMap<i64, f64>> = BTreeMap::new();
    for b in &bars {
        let ts = b.open_ts.0.unix_timestamp() * 1_000;
        let dk = day_key_ms(ts);
        let c: f64 = b.close.get().to_f64().unwrap_or(0.0);
        // overwrite → keeps the LAST (latest-in-day) close.
        dayclose
            .entry(b.symbol.0.to_string())
            .or_default()
            .insert(dk, c);
    }

    // funding per chain-symbol (daily as-of at 00:00 UTC of each day).
    let mut funding_by_sym: BTreeMap<String, Vec<(i64, f64)>> = BTreeMap::new();
    for (sym, _) in CHAIN_UNIVERSE {
        funding_by_sym.insert(
            sym.to_string(),
            load_funding(&args.funding_root, sym, args.year),
        );
    }

    // ── Load stablecoin supply per chain + aggregate ──
    let mut supply_by_chain: BTreeMap<String, BTreeMap<i64, f64>> = BTreeMap::new();
    for (_, chain) in CHAIN_UNIVERSE {
        supply_by_chain.insert(
            chain.to_string(),
            load_or_fetch_chain_supply(&args.bank_root, chain, args.refetch),
        );
    }
    let agg_supply = load_or_fetch_chain_supply(&args.bank_root, "all", args.refetch);

    // ── Common daily grid: days present in OHLCV AND supply for ALL chain names ──
    let first_chain_sym = CHAIN_UNIVERSE[0].0;
    let mut common_days: Vec<i64> = dayclose
        .get(first_chain_sym)
        .expect("first chain OHLCV present")
        .keys()
        .copied()
        .collect();
    for (sym, chain) in CHAIN_UNIVERSE {
        let dc = dayclose.get(sym).expect("OHLCV present");
        let sup = supply_by_chain.get(chain).expect("supply present");
        common_days.retain(|d| dc.contains_key(d) && sup.contains_key(d));
    }
    common_days.sort_unstable();
    let n_d = common_days.len();
    assert!(n_d > 300, "too few aligned days: {n_d} (is supply fetched?)");

    let n_sym = CHAIN_UNIVERSE.len();
    // price[j][k], supply[j][k], funding[j][k] on the common daily grid.
    let mut price: Vec<Vec<f64>> = vec![Vec::with_capacity(n_d); n_sym];
    let mut supply: Vec<Vec<f64>> = vec![Vec::with_capacity(n_d); n_sym];
    let mut fund: Vec<Vec<f64>> = vec![Vec::with_capacity(n_d); n_sym];
    for (j, (sym, chain)) in CHAIN_UNIVERSE.iter().enumerate() {
        let dc = dayclose.get(*sym).unwrap();
        let sup = supply_by_chain.get(*chain).unwrap();
        let fv = funding_by_sym.get(*sym).unwrap();
        for &d in &common_days {
            price[j].push(dc[&d]);
            supply[j].push(sup[&d]);
            // funding as-of the day's 00:00 UTC open.
            let day_open_ms = d * SECONDS_PER_DAY * 1_000;
            fund[j].push(funding_as_of(fv, day_open_ms));
        }
    }

    // daily log-returns ret[j][k] = ln(price[k+1]/price[k]), length n_d-1.
    let n_ret = n_d - 1;
    let mut ret: Vec<Vec<f64>> = vec![Vec::with_capacity(n_ret); n_sym];
    for j in 0..n_sym {
        for k in 0..n_ret {
            ret[j].push((price[j][k + 1] / price[j][k]).ln());
        }
    }

    // BTC daily returns + aggregate supply on the common grid (for the agg leg).
    let btc_dc = dayclose.get("BTCUSDT").expect("BTC OHLCV present");
    let mut btc_days: Vec<i64> = common_days
        .iter()
        .copied()
        .filter(|d| btc_dc.contains_key(d) && agg_supply.contains_key(d))
        .collect();
    btc_days.sort_unstable();
    let btc_price: Vec<f64> = btc_days.iter().map(|d| btc_dc[d]).collect();
    let btc_agg: Vec<f64> = btc_days.iter().map(|d| agg_supply[d]).collect();
    let btc_ret: Vec<f64> = (0..btc_price.len().saturating_sub(1))
        .map(|k| (btc_price[k + 1] / btc_price[k]).ln())
        .collect();

    // ── Report header ──
    println!(
        "=== STABLECOIN-SUPPLY DIAGNOSTIC — {}-FY (daily grid) ===",
        args.year
    );
    println!("ohlcv:   {}", args.ohlcv_root.display());
    println!("supply:  DefiLlama /stablecoincharts/{{chain}} (banked {})", args.bank_root.display());
    println!("funding: {}", args.funding_root.display());
    println!(
        "chain universe ({n_sym}): {}",
        CHAIN_UNIVERSE
            .iter()
            .map(|(s, c)| format!("{s}↔{c}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("aligned days (OHLCV ∩ supply, all {n_sym} chains): {n_d} → {n_ret} daily returns");
    println!("BTC∩agg days: {} → {} returns", btc_days.len(), btc_ret.len());
    println!();

    // B0: supply level sanity (per chain, $B).
    println!("--- B0: stablecoin supply level sanity (USD billions) ---");
    for (j, (sym, chain)) in CHAIN_UNIVERSE.iter().enumerate() {
        let v = &supply[j];
        let first = v.first().copied().unwrap_or(0.0);
        let last = v.last().copied().unwrap_or(0.0);
        let chg = if first != 0.0 {
            (last / first - 1.0) * 100.0
        } else {
            0.0
        };
        println!(
            "  {sym:9} ({chain:10}) start={:8.3}B  end={:8.3}B  Δ={:+7.1}%",
            first / 1e9,
            last / 1e9,
            chg
        );
    }
    println!();

    // ── B1: per-chain TIME-SERIES IC of trailing supply-CHANGE vs forward return ──
    // signal[D] = pct change in supply over [D-L, D) (past-only, supply[D-1] is the
    // last fully-observed snapshot at day-D open). forward = cum log-ret [D, D+L).
    // Reported per-chain AND pooled. (No cross-section: 4 names is too thin for a
    // rank-IC; the basis used 10. This is the honest framing.)
    println!("--- B1: per-chain TS IC (trailing Δsupply over [D-L,D) vs fwd return [D,D+L)) ---");
    let chain_ts_ic = |lookback: usize, leak: bool| -> (Vec<f64>, f64, usize) {
        // per-chain IC + pooled IC.
        let mut pooled_x: Vec<f64> = Vec::new();
        let mut pooled_y: Vec<f64> = Vec::new();
        let mut per: Vec<f64> = Vec::new();
        for j in 0..n_sym {
            let mut xs: Vec<f64> = Vec::new();
            let mut ys: Vec<f64> = Vec::new();
            let mut d = lookback;
            while d + lookback <= n_ret {
                // supply pct-change over the window.
                let (a, b) = if leak {
                    // LEAK: contemporaneous window [D, D+L) (look-ahead).
                    (supply[j][d], supply[j][d + lookback])
                } else {
                    // causal: [D-L, D) past-only.
                    (supply[j][d - lookback], supply[j][d])
                };
                let sig = if a != 0.0 { b / a - 1.0 } else { 0.0 };
                let fwd = ret[j][d..(d + lookback)].iter().sum::<f64>();
                xs.push(sig);
                ys.push(fwd);
                d += lookback;
            }
            per.push(spearman(&xs, &ys));
            pooled_x.extend(&xs);
            pooled_y.extend(&ys);
        }
        let pooled = spearman(&pooled_x, &pooled_y);
        (per, pooled, pooled_x.len())
    };
    for lb in [1usize, 3, 7, 14, 30] {
        let (per, pooled, n) = chain_ts_ic(lb, false);
        let per_s: Vec<String> = CHAIN_UNIVERSE
            .iter()
            .zip(per.iter())
            .map(|((s, _), ic)| format!("{}={ic:+.3}", &s[..3]))
            .collect();
        println!(
            "  L={lb:>3}d: pooled rank-IC={pooled:+.4}  [{}]  (n={n} pooled)",
            per_s.join(" ")
        );
    }
    println!();

    // ── B1-LEAK: no-look-ahead falsifier (optional) ──
    if args.leak_check {
        println!("--- B1-LEAK: no-look-ahead falsifier (causal trailing vs leaked contemporaneous) ---");
        for lb in [1usize, 3, 7, 14, 30] {
            let (_, causal, _) = chain_ts_ic(lb, false);
            let (_, leaked, _) = chain_ts_ic(lb, true);
            let differ = (causal - leaked).abs() > 1e-9;
            println!("  L={lb:>3}d: causal={causal:+.4}  leaked(contemporaneous)={leaked:+.4}  differ={differ}");
        }
        println!("  (causal MUST differ from leaked at every horizon ⇒ B1 uses past-only supply)");
        println!();
    }

    // ── B2: AGGREGATE dry-powder → forward BROAD-MARKET (BTC) return ──
    // signal[D] = pct change in TOTAL stablecoin supply over [D-L, D); fwd = BTC
    // cum log-ret [D, D+L). Pearson (single series, not a rank).
    println!("--- B2: aggregate Δtotal-supply over [D-L,D) vs forward BTC return [D,D+L) ---");
    let n_btc = btc_ret.len();
    let agg_ic = |lookback: usize| -> (f64, usize) {
        let mut xs: Vec<f64> = Vec::new();
        let mut ys: Vec<f64> = Vec::new();
        let mut d = lookback;
        while d + lookback <= n_btc {
            let a = btc_agg[d - lookback];
            let b = btc_agg[d];
            let sig = if a != 0.0 { b / a - 1.0 } else { 0.0 };
            let fwd = btc_ret[d..(d + lookback)].iter().sum::<f64>();
            xs.push(sig);
            ys.push(fwd);
            d += lookback;
        }
        (pearson(&xs, &ys), xs.len())
    };
    for lb in [1usize, 3, 7, 14, 30] {
        let (ic, n) = agg_ic(lb);
        println!("  L={lb:>3}d: corr(Δagg-supply, fwd BTC ret) = {ic:+.4}  (n={n})");
    }
    println!();

    // ── B3: orthogonality — supply-change signal vs price-momentum AND funding ──
    // A new on-chain signal must be orthogonal to BOTH dead channels. Pool over
    // chains+windows: corr(Δsupply, trailing-momentum) and corr(Δsupply, funding).
    println!("--- B3: orthogonality (Δsupply vs price-momentum, and vs funding) ---");
    let orthogonality = |lookback: usize| -> (f64, f64, usize) {
        let mut s_sig: Vec<f64> = Vec::new();
        let mut m_sig: Vec<f64> = Vec::new();
        let mut f_sig: Vec<f64> = Vec::new();
        for j in 0..n_sym {
            let mut d = lookback;
            while d + lookback <= n_ret {
                let a = supply[j][d - lookback];
                let b = supply[j][d];
                let ss = if a != 0.0 { b / a - 1.0 } else { 0.0 };
                let ms = ret[j][(d - lookback)..d].iter().sum::<f64>();
                let fs = fund[j][d];
                s_sig.push(ss);
                m_sig.push(ms);
                f_sig.push(fs);
                d += lookback;
            }
        }
        let sm = pearson(&s_sig, &m_sig);
        let sf = pearson_masked(&s_sig, &f_sig);
        (sm, sf, s_sig.len())
    };
    for lb in [1usize, 3, 7, 14, 30] {
        let (sm, sf, n) = orthogonality(lb);
        println!("  L={lb:>3}d: corr(Δsupply, momentum) = {sm:+.4}   corr(Δsupply, funding) = {sf:+.4}  (n={n})");
    }
    println!();

    println!("--- INTERPRETATION GUIDE ---");
    println!("  B1/B2 ≈ 0 (|IC|<~0.05, no stable sign, both years) ⇒ supply carries ~no forward info");
    println!("  B1/B2 persistently |IC|≥0.05 same-sign 2023 AND 2024 ⇒ LIVE candidate");
    println!("  B3 |corr| small vs BOTH momentum AND funding ⇒ orthogonal (a genuinely new channel)");
    println!("  LIVE iff (B1 or B2) persistently sign-stable |IC|≥0.05 AND orthogonal to BOTH dead channels");
}

// ── stats helpers (identical to basis_diag.rs) ─────────────────────────────────

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

fn spearman(a: &[f64], b: &[f64]) -> f64 {
    let ra = ranks(a);
    let rb = ranks(b);
    pearson(&ra, &rb)
}

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
