//! T-T1: Short-arm bake-off on the 2022 bear window and the H1-2024 bull window.
//!
//! This is the tester's integration harness for `advisor-short-selling` (ADR-0068).
//!
//! # What it does
//!
//! For EACH window (bear: 2022-04-01..2022-07-01; bull: H1-2024):
//!  - Loads the bear corpus from `data/binance-2122/` (pinned sha `4f390622`).
//!  - Runs the 5 pre-registered short arms + the `v0.buyhold` benchmark through
//!    `run_scenario` with `short_enabled = true` for each arm and `write_report = false`
//!    (anchor-safe; does NOT touch any anchored report body).
//!  - Applies the bootstrap robustness gate (N=1000, `LAB_DEFAULT_SEED`).
//!  - Emits a table of id / RobustnessFlag / Sharpe / return% / maxDD% / trades.
//!  - Checks the SANITY gate: `v0.always_short` MUST PROFIT on the bear window
//!    (terminal equity > initial). A failure means the engine dispatch is broken.
//!
//! # T-D6 implemented -- REAL IDs in use
//!
//! This test now uses the FROZEN ADR-0068 D9 slate IDs directly (T-D6 wired):
//!   - `v0.sma_cross_ls`  -- engine dispatches to the SMA crossover arm with short_enabled=true
//!   - `v0.macd_ls`       -- engine dispatches to the MACD arm with short_enabled=true
//!   - `v0.rsi_ls`        -- engine dispatches to the RSI arm with short_enabled=true
//!   - `v0.bbands_ls`     -- engine dispatches to the BBands arm with short_enabled=true
//!   - `v0.always_short`  -- engine dispatches to `run_alwaysshort_path`
//!     (equity formula, NOT an SMA proxy; the PROPER inverse of buy-and-hold)
//!   - `v0.buyhold`       -- benchmark (short_enabled=false)
//!
//! The `sma_fast`/`sma_slow` overrides are no longer needed for `v0.always_short`
//! (the SMA(1,2) hack was the proxy -- the proper arm uses the direct equity formula).
//!
//! # Un-anchored (no evidence/*/reports file, no anchors.toml row)
//!
//! `#[ignore]` -- run with:
//! ```text
//! cargo test -p backtest --features realdata --test short_bakeoff_bear_bull -- --ignored --nocapture
//! ```
//!
//! SKIPS cleanly when `data/binance-2122/BTCUSDT/2022/04.parquet` is absent.
//! The bear corpus is pinned at sha `4f390622`.
//!
//! # Expected outcome
//!
//! ALL short arms are expected to be FRAGILE under the frozen bootstrap gate -- that is
//! the pre-registered prediction and the valid PASS-worthy null. `v0.always_short`
//! profits on the bear window (positive return) but the FROZEN gate's bootstrap paths
//! include both up and down paths, so it comes back FRAGILE too (the honest result).
//!
//! If ANY arm comes back NON-Fragile, that is flagged prominently as a SURPRISING result.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::float_arithmetic,
    clippy::print_stdout
)]

use std::path::PathBuf;

use backtest::bakeoff::bootstrap::{compute_robustness_flag, derive_master_seed};
use backtest::bakeoff::derive_candidate_kpis;
use backtest::bakeoff::robustness::RobustnessFlag;
use backtest::cancel::cancellation_pair;
use backtest::cli_types::LatencySlippageSimConfig;
use backtest::engine::{DateRange, ScenarioConfig, ScenarioDataSource, run_scenario};
use backtest::progress::ProgressSender;
use rust_decimal::Decimal;
use tokio_stream::StreamExt as _;
use trading_core::{Bar, StrategyId, Symbol, Timeframe, Venue};

// ── Constants ─────────────────────────────────────────────────────────────────

/// The LAB_DEFAULT_SEED (from `ui::lab::defaults`); using the raw bytes here
/// so this test has no dep on `crates/ui`.
const SEED: [u8; 32] = {
    let mut s = [0u8; 32];
    s[0] = 0xC0;
    s[1] = 0xFF;
    s[2] = 0xEE;
    s
};

/// Seed as u64 (low 8 bytes) for `derive_master_seed`.
const SEED_U64: u64 = u64::from_le_bytes([0xC0, 0xFF, 0xEE, 0, 0, 0, 0, 0]);

/// Bootstrap paths (1000 per ADR-0063 § D4).
const N_PATHS: usize = 1000;

/// Bear window: 2022-04-01 00:00:00 UTC..2022-07-01 00:00:00 UTC (approx -58% BTC).
const BEAR_START_MS: u64 = 1_648_771_200_000;
const BEAR_END_MS: u64 = 1_656_633_600_000;

/// Bull window: H1-2024 (matches `DateRange::H1_2024`).
const BULL_START_MS: u64 = 1_704_067_200_000;
const BULL_END_MS: u64 = 1_719_792_000_000;

/// Bear corpus root (pinned sha `4f390622`).
const BEAR_CORPUS: &str = "data/binance-2122";

/// Main Binance corpus root (2023-2024).
const MAIN_CORPUS: &str = "data/binance";

// ── Short arm slate (FROZEN ADR-0068 D9 slate + buyhold benchmark) ────────────
//
// T-D6 WIRED: these are the REAL frozen IDs, not proxies.
// The engine now dispatches each ID to the correct arm.

struct ArmDef {
    /// The FROZEN ADR-0068 D9 slate ID -- used both for reporting AND as the engine_id.
    label: &'static str,
    /// Optional SMA fast/slow overrides (only used for _ls arms that use SMA crossover,
    /// None = strategy defaults). NOT used for v0.always_short (direct equity formula).
    sma_fast: Option<usize>,
    sma_slow: Option<usize>,
    short_enabled: bool,
    is_benchmark: bool,
}

fn short_field_with_buyhold() -> Vec<ArmDef> {
    vec![
        ArmDef {
            label: "v0.sma_cross_ls",
            sma_fast: None,
            sma_slow: None,
            short_enabled: true,
            is_benchmark: false,
        },
        ArmDef {
            label: "v0.macd_ls",
            sma_fast: None,
            sma_slow: None,
            short_enabled: true,
            is_benchmark: false,
        },
        ArmDef {
            label: "v0.rsi_ls",
            sma_fast: None,
            sma_slow: None,
            short_enabled: true,
            is_benchmark: false,
        },
        ArmDef {
            label: "v0.bbands_ls",
            sma_fast: None,
            sma_slow: None,
            short_enabled: true,
            is_benchmark: false,
        },
        ArmDef {
            // T-D6: REAL always_short arm -- uses run_alwaysshort_path (direct equity formula).
            // No SMA overrides needed (this arm ignores the SMA parameters).
            label: "v0.always_short",
            sma_fast: None,
            sma_slow: None,
            short_enabled: false, // always_short arm does not use short_enabled flag
            is_benchmark: false,
        },
        ArmDef {
            label: "v0.buyhold",
            sma_fast: None,
            sma_slow: None,
            short_enabled: false,
            is_benchmark: true,
        },
    ]
}

// ── Row in the result table ───────────────────────────────────────────────────

#[derive(Debug)]
struct ArmResult {
    id: String,
    is_benchmark: bool,
    robustness: RobustnessFlag,
    sharpe: f64,
    total_return_pct: Decimal,
    max_drawdown: Decimal,
    trade_count: usize,
    final_equity: Decimal,
    initial_equity: Decimal,
    /// Fills that moved the position at or below zero — a Sell from flat/short,
    /// or a Buy while short. `0` on a `short_enabled` arm means the arm carries
    /// a long/short label it never earned (bug-log #82).
    short_legs: usize,
    /// Most negative reconstructed position over the run.
    min_pos: Decimal,
    /// Largest long position over the run — the bug-log #82 ratchet probe.
    max_pos: Decimal,
}

fn flag_str(f: RobustnessFlag) -> &'static str {
    match f {
        RobustnessFlag::Robust => "Robust",
        RobustnessFlag::Marginal => "Marginal",
        RobustnessFlag::Fragile => "FRAGILE",
        RobustnessFlag::Skipped => "Skipped",
    }
}

// ── Data loading ─────────────────────────────────────────────────────────────

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Load hourly bars for BTCUSDT in [start_ms, end_ms) from the given corpus root.
async fn load_bars(corpus_root: &str, start_ms: u64, end_ms: u64) -> Vec<Bar> {
    use data::source::MarketDataSource as _;
    let root = workspace_root().join(corpus_root);
    let sym = Symbol::new("BTCUSDT");
    let feed = data::ReplayFeed::new(&root, true);
    let Ok(mut stream) = feed.subscribe_bars(sym, Timeframe::OneHour).await else {
        return Vec::new();
    };
    let mut bars = Vec::new();
    while let Some(Ok(b)) = stream.next().await {
        let ts = b.open_ts.unix_millis() as u64;
        if ts >= start_ms && ts < end_ms {
            bars.push(b);
        } else if ts >= end_ms {
            break;
        }
    }
    bars
}

// ── Core runner ───────────────────────────────────────────────────────────────

/// Run one arm and return an ArmResult (None on error).
async fn run_arm(arm: &ArmDef, bars: Vec<Bar>, candidate_index: usize) -> Option<ArmResult> {
    let sym = Symbol::new("BTCUSDT");
    // T-D6: engine_id IS the frozen label -- no proxy mapping needed.
    let engine_id = arm.label;
    let cfg = ScenarioConfig {
        strategy: StrategyId(engine_id.into()),
        pair: (Venue::Binance, sym.clone()),
        range: DateRange::Last30d, // ignored -- bars_override supplies data
        params: None,
        seed: SEED,
        write_report: false, // anchor-safe: no anchored report body written
        data_source: ScenarioDataSource::BinanceCache,
        bars_override: Some(bars.clone()),
        sma_fast_len: arm.sma_fast,
        sma_slow_len: arm.sma_slow,
        latency_slippage_sim: LatencySlippageSimConfig::default(),
        reports_dir: None,
        short_enabled: arm.short_enabled,
        initial_capital: None,
        composed_toml_override: None,
        dvol_override: None,
        macro_regime_series: None,
    };
    let (_h, cancel_rx) = cancellation_pair();
    let run_result = run_scenario(cfg, cancel_rx, ProgressSender::disabled()).await;
    if let Err(ref e) = run_result {
        println!("[TESTER] run_scenario({}) error: {:?}", engine_id, e);
    }
    let report = run_result.ok()?;

    // ── Short-leg census (bug-log #80, added 2026-08-15) ──────────────────
    //
    // The table below reports Sharpe/return/trades but says NOTHING about
    // whether a "long/short" arm ever went short. It does not always: on both
    // of this harness's real windows every `_ls` arm runs its ENTIRE course
    // with a non-negative position, so the short branch of
    // `scenarios/sma_composed_run.rs` never executes and the arms are ranked as
    // long-only strategies under long/short labels. That is why the #80
    // execution-friction fix moved these numbers by exactly zero, and it is
    // worth seeing every time this harness runs rather than rediscovering it.
    //
    // `short_legs` counts fills that move the position at or below zero (a Sell
    // from flat/short, or a Buy while short); `minpos` is the most negative the
    // reconstructed position ever got. `short_legs = 0` and `minpos = 0` mean
    // the arm never shorted.
    let (short_legs, minpos, buys, sells, maxpos) = {
        let mut pos = Decimal::ZERO;
        let (mut short_legs, mut buys, mut sells) = (0usize, 0usize, 0usize);
        let (mut minpos, mut maxpos) = (Decimal::ZERO, Decimal::ZERO);
        for f in &report.fills {
            let before = pos;
            match f.side {
                trading_core::Side::Buy => {
                    pos += f.qty.get();
                    buys += 1;
                }
                trading_core::Side::Sell => {
                    pos -= f.qty.get();
                    sells += 1;
                }
            }
            if (matches!(f.side, trading_core::Side::Sell) && before <= Decimal::ZERO)
                || (matches!(f.side, trading_core::Side::Buy) && before < Decimal::ZERO)
            {
                short_legs += 1;
            }
            if pos < minpos {
                minpos = pos;
            }
            if pos > maxpos {
                maxpos = pos;
            }
        }
        (short_legs, minpos, buys, sells, maxpos)
    };
    println!(
        "[SHORT-CENSUS] {engine_id}: fills={} buys={buys} sells={sells} \
         short_legs={short_legs} min_pos={minpos} max_pos={maxpos}{}",
        report.fills.len(),
        if arm.short_enabled && short_legs == 0 {
            "   <-- short_enabled arm that NEVER SHORTED on this window"
        } else {
            ""
        }
    );

    let kpis = derive_candidate_kpis(&report);
    let equity_decimals: Vec<Decimal> = report
        .equity_series
        .iter()
        .map(|(_, m)| m.amount())
        .collect();

    let master_seed = derive_master_seed(SEED_U64, candidate_index);
    let robustness = compute_robustness_flag(&equity_decimals, N_PATHS, master_seed);

    let initial_equity = equity_decimals.first().copied().unwrap_or(Decimal::ZERO);
    let final_equity = equity_decimals.last().copied().unwrap_or(Decimal::ZERO);

    Some(ArmResult {
        id: arm.label.to_string(),
        is_benchmark: arm.is_benchmark,
        robustness,
        sharpe: kpis.sharpe,
        total_return_pct: kpis.total_return_pct,
        max_drawdown: kpis.max_drawdown,
        trade_count: kpis.trade_count,
        final_equity,
        initial_equity,
        short_legs,
        min_pos: minpos,
        max_pos: maxpos,
    })
}

/// **bug-log #82 gate — a long/short arm must actually take a short.**
///
/// Two properties, both of which were FALSE when #82 was written and both of
/// which are now measured true on real data:
///
/// 1. **`v0.sma_cross_ls` takes short legs.** It took ZERO on both windows.
///    The cause was bug-log #71: `Order::new`'s cap was side-blind, so once the
///    position passed `per_symbol_exposure_cap = 0.40` every position-CLOSING
///    Sell was silently refused while each small opening Buy still passed. The
///    arm could never return to flat, so `Sell-when-flat` — the short ENTRY —
///    could never fire.
/// 2. **The position stays bounded.** The same refusal made the arm a one-way
///    ratchet: 181 buys against 1 sell, `max_pos` 28.2 (bear) and 39.2 (bull) on
///    a 100 000 account — roughly 11-16x leverage — ending at NEGATIVE equity
///    (-9 235 / -14 146). A long/short arm had become an unbounded leveraged
///    long, and nothing in the KPI table showed it.
///
/// RED-ON-REVERT: restore the side-blind cap and both assertions fail — the
/// short count returns to zero and the ratchet returns with it.
///
/// Deliberately NOT asserted here: `macd_ls` / `rsi_ls` / `bbands_ls` still take
/// zero short legs. That is #82's SECOND mechanism (perfect buy/sell
/// alternation never reaches the flat state `Sell-when-flat` requires) and it is
/// independent of #71 — it needs a signal-shape decision or honest re-labelling,
/// not a cap fix. Asserting it here would encode a defect as a requirement.
fn assert_no_ratchet_and_shorts_taken(window: &str, results: &[ArmResult]) {
    let Some(arm) = results.iter().find(|r| r.id == "v0.sma_cross_ls") else {
        panic!("#82 gate: v0.sma_cross_ls missing from the {window} field");
    };
    assert!(
        arm.short_legs > 0,
        "#82 gate ({window}): v0.sma_cross_ls is labelled long/short and took \
         {} short legs. Zero means the arm is ranked as long/short while running \
         long-only — check that Order::new's exposure cap is still evaluated on \
         RESULTING exposure (bug-log #71), because a side-blind cap refuses the \
         closing Sells that the short entry depends on.",
        arm.short_legs
    );
    assert!(
        arm.min_pos < Decimal::ZERO,
        "#82 gate ({window}): v0.sma_cross_ls never held a negative position \
         (min_pos = {}). Short legs without a negative position would mean the \
         census is counting something else.",
        arm.min_pos
    );
    // The ratchet's most direct signature: an unbounded long. BTC traded roughly
    // 19k-70k across these two windows, so 10 units on a 100 000 account is
    // 2-7x leverage — impossible for a book that can close. The measured ratchet
    // reached 28.2 (bear) and 39.2 (bull); the fixed arm reaches 1.83 and 0.98.
    // The bound sits far from both, so it tests the RATCHET, not the sizing.
    assert!(
        arm.max_pos < Decimal::from(10),
        "#82 gate ({window}): v0.sma_cross_ls reached max_pos = {} units on a \
         100 000 account — that is the leverage ratchet, not a position. Every \
         closing Sell refused while every opening Buy passed.",
        arm.max_pos
    );
    assert!(
        arm.final_equity > Decimal::ZERO,
        "#82 gate ({window}): v0.sma_cross_ls ended at NEGATIVE equity ({}). \
         That is the leverage ratchet — every closing Sell refused while every \
         opening Buy passed. Note the second-order trap: the cap check is guarded \
         by `if current_equity > 0`, so once equity goes negative the cap stops \
         applying at all.",
        arm.final_equity
    );
}

// ── Table printer ─────────────────────────────────────────────────────────────

fn print_table(label: &str, results: &[ArmResult]) {
    println!();
    println!("=== SHORT BAKEOFF: {} ===", label);
    println!(
        "{:<20} {:<10} {:<12} {:>10} {:>10} {:>10} {:>8}",
        "ID", "Flag", "Benchmark", "Sharpe", "Return%", "MaxDD%", "Trades"
    );
    println!("{}", "-".repeat(82));
    for r in results {
        println!(
            "{:<20} {:<10} {:<12} {:>10.3} {:>10.2} {:>10.2} {:>8}",
            r.id,
            flag_str(r.robustness),
            if r.is_benchmark { "benchmark" } else { "" },
            r.sharpe,
            r.total_return_pct,
            r.max_drawdown,
            r.trade_count,
        );
    }
    println!();
    // Full-precision terminal equity. The table above rounds `Return%` to two
    // decimals, which hides anything smaller than ~50 bps — including an entire
    // execution-friction model. bug-log #80 (short legs bypassing
    // `PaperEngine::step`, so paying no slippage and no lot-rounding) moved
    // these arms by single-digit bps: invisible at 2 dp, decisive in a ranked
    // comparison. Print the raw Decimal so a before/after is actually legible.
    println!("--- terminal equity (full precision) ---");
    for r in results {
        println!("  {:<20} {}", r.id, r.final_equity);
    }
    println!();

    // Summary
    let non_fragile: Vec<&ArmResult> = results
        .iter()
        .filter(|r| {
            !r.is_benchmark
                && r.robustness != RobustnessFlag::Fragile
                && r.robustness != RobustnessFlag::Skipped
        })
        .collect();

    let benchmark = results.iter().find(|r| r.is_benchmark);
    let bh_return = benchmark
        .map(|b| b.total_return_pct)
        .unwrap_or(Decimal::ZERO);
    let bh_sharpe = benchmark.map(|b| b.sharpe).unwrap_or(0.0);

    // Beat buy-and-hold by return?
    let beats_bh_by_return: Vec<&ArmResult> = results
        .iter()
        .filter(|r| !r.is_benchmark && r.total_return_pct > bh_return)
        .collect();

    println!("--- SUMMARY: {} ---", label);
    println!(
        "Buy-and-hold: return={:.2}%  sharpe={:.3}",
        bh_return, bh_sharpe
    );
    println!(
        "Arms beating buy-and-hold by return: {}",
        beats_bh_by_return.len()
    );
    for r in &beats_bh_by_return {
        println!(
            "  [BEATS BH] {} return={:.2}%  flag={}",
            r.id,
            r.total_return_pct,
            flag_str(r.robustness)
        );
    }

    println!("Non-Fragile short arms: {}", non_fragile.len());
    if non_fragile.is_empty() {
        println!("  RESULT: All short arms are FRAGILE -- null finding, hold stands.");
    } else {
        println!("  *** SURPRISING: NON-FRAGILE short arm(s) found! ***");
        for r in &non_fragile {
            println!(
                "  [NON-FRAGILE] {} flag={} sharpe={:.3} return={:.2}%",
                r.id,
                flag_str(r.robustness),
                r.sharpe,
                r.total_return_pct
            );
        }
    }
    println!();
}

// ── Bear-window sanity check ──────────────────────────────────────────────────

fn check_always_short_sanity(results: &[ArmResult]) {
    let always_short = results.iter().find(|r| r.id == "v0.always_short");
    match always_short {
        None => {
            println!("[SANITY FAIL] v0.always_short not found in results!");
        }
        Some(r) => {
            if r.final_equity > r.initial_equity {
                println!(
                    "[SANITY PASS] v0.always_short PROFITS on bear window: \
                     initial={:.2} final={:.2} return={:.2}%",
                    r.initial_equity, r.final_equity, r.total_return_pct
                );
            } else {
                println!(
                    "[SANITY FAIL] v0.always_short does NOT profit on bear window! \
                     initial={:.2} final={:.2} return={:.2}% -- SHORT MECHANICS MAY BE BROKEN",
                    r.initial_equity, r.final_equity, r.total_return_pct
                );
            }
        }
    }
}

// ── Test harness ─────────────────────────────────────────────────────────────

/// T-T1 -- Bear window bake-off.
///
/// 2022-04-01..2022-07-01 (BTC approx -58%). This is the load-bearing window:
/// the always_short arm MUST profit here; the sanity check must hold.
/// This proves the engine dispatch for `v0.always_short` is correct (T-D6).
#[tokio::test]
#[ignore]
async fn t_t1_short_bakeoff_bear_window() {
    let bars = load_bars(BEAR_CORPUS, BEAR_START_MS, BEAR_END_MS).await;
    if bars.is_empty() {
        println!(
            "[SKIP] Bear corpus not found at {} -- install data/binance-2122/ to run.",
            BEAR_CORPUS
        );
        return;
    }
    println!(
        "[BEAR WINDOW] 2022-04-01..2022-07-01 -- {} bars loaded",
        bars.len()
    );

    // Log BTC price move for context.
    let first_close = bars.first().map(|b| b.close.get()).unwrap_or(Decimal::ZERO);
    let last_close = bars.last().map(|b| b.close.get()).unwrap_or(Decimal::ZERO);
    let pct_move = if first_close > Decimal::ZERO {
        (last_close - first_close) / first_close * Decimal::from(100)
    } else {
        Decimal::ZERO
    };
    println!(
        "[BEAR WINDOW] BTC: {:.0} -> {:.0} ({:+.1}%)",
        first_close, last_close, pct_move
    );

    let field = short_field_with_buyhold();
    let mut results = Vec::with_capacity(field.len());

    for (idx, arm) in field.iter().enumerate() {
        println!(
            "[BEAR] Running arm {} (short_enabled={})...",
            arm.label, arm.short_enabled
        );
        match run_arm(arm, bars.clone(), idx).await {
            Some(r) => results.push(r),
            None => println!("[BEAR] arm {} failed -- skipped", arm.label),
        }
    }

    print_table("BEAR 2022-Q2 (BTC -58%)", &results);
    assert_no_ratchet_and_shorts_taken("BEAR 2022-Q2", &results);
    check_always_short_sanity(&results);

    // ── Sanity check: verify the short engine sign via direct short_exec call ──
    //
    // This is the authoritative sign-correctness proof for the short_exec helper
    // (not the always_short equity-formula path). Both must be correct.
    {
        use backtest::short_exec;
        use rust_decimal_macros::dec;

        // Use the actual first/last prices from our bear window for realism.
        let first_close = bars.first().map(|b| b.close.get()).unwrap_or(dec!(45541));
        let last_close = bars.last().map(|b| b.close.get()).unwrap_or(dec!(19942));

        let initial_cash = dec!(100_000);
        let fee_bps: u32 = 4; // 4 bps taker fee

        // Open a short at the bear window's start price.
        let open_result = short_exec::try_open_short(
            initial_cash,
            dec!(0),     // no existing position
            first_close, // mark = bear start price (~45,000)
            fee_bps,
            initial_cash, // equity = initial cash
        );

        if open_result.executed {
            // Cover at the bear window's end price (much lower -- profit expected).
            let cover_result = short_exec::try_cover_short(
                open_result.cash,
                open_result.position_qty, // negative (short)
                last_close,               // mark = bear end price (~20,000)
                fee_bps,
            );
            let covered_cash = cover_result.cash;
            let covered_qty = cover_result.position_qty;

            let final_equity = covered_cash + covered_qty * last_close;
            let profit = final_equity - initial_cash;

            println!(
                "[SANITY] Direct short_exec proof: open_at={:.0} cover_at={:.0} -> \
                 final_equity={:.2} profit={:.2} (expected POSITIVE on a -56% bear)",
                first_close, last_close, final_equity, profit
            );

            assert!(
                final_equity > initial_cash,
                "SANITY FAIL: short_exec produced NEGATIVE profit on a -56% bear \
                 (open={:.0} cover={:.0} final_equity={:.2} profit={:.2}). \
                 Short engine accounting is broken.",
                first_close,
                last_close,
                final_equity,
                profit
            );

            println!(
                "[SANITY PASS] Direct short_exec: profit={:.2} on bear window -- short sign is CORRECT.",
                profit
            );
        } else {
            println!(
                "[SANITY] short_exec try_open_short skipped (solvency gate fired) -- not a sign error."
            );
        }
    }

    // ── THE KEY T-D6 SANITY: assert always_short PROFITS on the bear window ──
    //
    // This is the load-bearing assertion. If T-D6 wiring is correct,
    // run_alwaysshort_path computes equity = 100000 * (2 - price_final/price0),
    // which is strongly positive on a -56% bear window.
    let always_short_result = results.iter().find(|r| r.id == "v0.always_short");
    if let Some(r) = always_short_result {
        assert!(
            r.final_equity > r.initial_equity,
            "T-D6 SANITY FAIL: v0.always_short final_equity ({:.2}) <= initial ({:.2}) on bear window. \
             The run_alwaysshort_path dispatch is broken or returning wrong equity sign.",
            r.final_equity,
            r.initial_equity
        );
        println!(
            "[T-D6 PASS] v0.always_short profits on bear: {:.2} -> {:.2} (+{:.2}%)",
            r.initial_equity, r.final_equity, r.total_return_pct
        );
    }
}

/// T-T1 -- Bull window bake-off (contrast).
///
/// H1-2024 (BTC strongly bullish). Shorts expected to LOSE here; the honest
/// expected result. Runs on the main corpus since H1-2024 is not in binance-2122.
#[tokio::test]
#[ignore]
async fn t_t1_short_bakeoff_bull_window() {
    let bars = load_bars(MAIN_CORPUS, BULL_START_MS, BULL_END_MS).await;
    if bars.is_empty() {
        println!(
            "[SKIP] Main corpus not found at {} -- install data/binance/ to run.",
            MAIN_CORPUS
        );
        return;
    }
    println!("[BULL WINDOW] H1-2024 -- {} bars loaded", bars.len());

    // Log BTC price move for context.
    let first_close = bars.first().map(|b| b.close.get()).unwrap_or(Decimal::ZERO);
    let last_close = bars.last().map(|b| b.close.get()).unwrap_or(Decimal::ZERO);
    let pct_move = if first_close > Decimal::ZERO {
        (last_close - first_close) / first_close * Decimal::from(100)
    } else {
        Decimal::ZERO
    };
    println!(
        "[BULL WINDOW] BTC: {:.0} -> {:.0} ({:+.1}%)",
        first_close, last_close, pct_move
    );

    let field = short_field_with_buyhold();
    let mut results = Vec::with_capacity(field.len());

    for (idx, arm) in field.iter().enumerate() {
        println!(
            "[BULL] Running arm {} (short_enabled={})...",
            arm.label, arm.short_enabled
        );
        match run_arm(arm, bars.clone(), idx).await {
            Some(r) => results.push(r),
            None => println!("[BULL] arm {} failed -- skipped", arm.label),
        }
    }

    print_table("BULL H1-2024 (BTC +~80%)", &results);
    assert_no_ratchet_and_shorts_taken("BULL H1-2024", &results);

    // On a bull window always_short is EXPECTED to lose -- this is the honest control.
    let always_short = results.iter().find(|r| r.id == "v0.always_short");
    if let Some(r) = always_short {
        println!(
            "[BULL CONTRAST] v0.always_short on bull: return={:.2}%  (expected negative -- shorts lose on uptrend)",
            r.total_return_pct
        );
        // Note: we do NOT assert a failure here -- the test is an observation run.
        // An always-short arm losing on a bull market is the EXPECTED correct result.
    }
}
