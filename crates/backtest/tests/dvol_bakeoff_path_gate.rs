//! T8 / Task 3 — bake-off-PATH regression gate for the DVOL wiring (ADR-0072).
//!
//! ## Why this gate is needed
//!
//! The existing day-1 gates (`dvol_regime_divergence_end_to_end.rs`,
//! `dvol_regime_leak_check.rs`) inject a **synthetic** DVOL series straight
//! into `DvolRegimeStrategy`, bypassing the bake-off orchestrator. They CANNOT
//! catch the "None stub" trap: before Task 2, `run_bakeoff` always set
//! `dvol_override: None`, meaning the arm ran with an empty series →
//! permanent warm-up → indistinguishable from buy-and-hold.
//!
//! These tests prove the ORCHESTRATOR now feeds a real DVOL series end-to-end.
//!
//! ## Tests
//!
//! 1. `resolve_dvol_override_returns_some_with_real_corpus` — fast unit test:
//!    calls `resolve_dvol_override` directly and asserts `Some` + non-empty.
//!    Checks the loader plumbing is wired before launching the full bakeoff.
//!
//! 2. `dvol_regime_bakeoff_differs_from_buyhold` — full `run_bakeoff` on
//!    BTCUSDT H1-2024 (the frozen robustness window) and asserts that the
//!    `v0.dvol_regime` candidate's final equity DIFFERS from the `v0.buyhold`
//!    candidate's final equity. Equality → `dvol_override` is still None or
//!    the series is all-None (the no-op trap).
//!
//! ## Corpus dependency
//!
//! Both tests require `data/deribit-dvol/{BTC,ETH}/{2023,2024}.parquet` on disk
//! (gitignored; aggregate SHA pinned to `EXPECTED_DVOL_REVISION_SHA`). They are
//! `#[ignore]` by default and tagged as **corpus-dependent on-machine tests**.
//!
//! ## Run
//!
//! ```sh
//! cargo test -p backtest --test dvol_bakeoff_path_gate -- --ignored 2>&1 \
//!   | tee /tmp/dvol-bakeoff-gate.log
//! ```

#![allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::pedantic
)]

use backtest::{
    BakeoffConfig, BakeoffRequest, DateRange,
    bakeoff::{resolve_dvol_override, run_bakeoff},
    cancel::cancellation_pair,
    engine::ScenarioDataSource,
    progress::{BakeoffProgressSender, ProgressSender},
    resample::Horizon,
};
use rust_decimal_macros::dec;
use trading_core::Symbol;

/// Resolve the workspace root from CARGO_MANIFEST_DIR.
///
/// Integration tests run from `crates/backtest/` (cwd = crate dir), not the
/// workspace root. Corpus paths (`data/binance`, `data/deribit-dvol`) are
/// relative to the workspace root, so tests must set_current_dir before calling
/// any function that uses those relative paths. Pattern mirrors `bakeoff_e2e.rs`.
fn workspace_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has parent (crates/)")
        .parent()
        .expect("crates/ has parent (workspace root)")
        .to_path_buf()
}

/// Seed that mirrors the lab default (LAB_DEFAULT_SEED from crates/ui/src/lab/defaults.rs).
const LAB_SEED: [u8; 32] = [
    0x42, 0x49, 0x4e, 0x41, // "BINA"
    0x4e, 0x43, 0x45, 0x31, // "NCE1"
    0x42, 0x49, 0x4e, 0x41, // "BINA"
    0x4e, 0x43, 0x45, 0x31, // "NCE1"
    0x42, 0x49, 0x4e, 0x41, // "BINA"
    0x4e, 0x43, 0x45, 0x31, // "NCE1"
    0x42, 0x49, 0x4e, 0x41, // "BINA"
    0x4e, 0x43, 0x45, 0x31, // "NCE1"
];

// ── Test 1: resolve_dvol_override returns Some with real corpus ────────────────

/// Fast unit test: the `resolve_dvol_override` helper returns `Some` + non-empty
/// when the real DVOL corpus is on disk.
///
/// **Corpus-dependent on-machine test.** Requires:
/// `data/deribit-dvol/BTC/2024.parquet` (and the matching REVISION.toml SHA pin).
///
/// Proves: the loader plumbing (`DvolDataSource::load` → `dvol_as_of`) is wired
/// before we launch the full 2-minute bakeoff.
///
/// If the corpus is absent, `resolve_dvol_override` emits `warn!` and returns
/// `None` — this test will panic with "expected Some, got None" (correct: that
/// means the corpus isn't installed or the SHA pin needs updating).
#[test]
#[ignore = "corpus-dependent on-machine test: requires data/deribit-dvol/BTC/2024.parquet + SHA match"]
fn resolve_dvol_override_returns_some_with_real_corpus() {
    // Integration tests run with cwd = crate dir, not workspace root.
    // Corpus paths are relative to the workspace root → set cwd first.
    let ws_root = workspace_root();
    std::env::set_current_dir(&ws_root).expect("set cwd to workspace root");

    // Build a synthetic bar_open_ts_ms slice that covers 2024-01-01 through
    // 2024-06-30 (H1_2024 window). We only need the timestamps; hourly grid.
    // H1_2024 = 2024-01-01T00:00:00Z (1704067200000 ms) .. 2024-07-01 (1719792000000 ms)
    let start_ms: i64 = 1_704_067_200_000;
    let end_ms: i64 = 1_719_792_000_000;
    let one_hour_ms: i64 = 3_600_000;
    let bar_open_ts_ms: Vec<i64> = (0..((end_ms - start_ms) / one_hour_ms))
        .map(|h| start_ms + h * one_hour_ms)
        .collect();
    let bar_count = bar_open_ts_ms.len();
    println!("resolve_dvol_override: testing with {bar_count} bar timestamps (H1_2024)");

    let result = resolve_dvol_override(
        "BTCUSDT",
        &DateRange::H1_2024,
        &bar_open_ts_ms,
        "test-resolve-dvol-override",
    );

    let series = result.expect(
        "resolve_dvol_override must return Some for BTCUSDT/H1_2024 \
         when the real DVOL corpus is on disk and the SHA pin matches",
    );

    assert_eq!(
        series.len(),
        bar_count,
        "returned series must have one entry per bar timestamp: expected {bar_count}, got {}",
        series.len()
    );

    // Count bars with a Some DVOL value (past warm-up).
    let some_count = series.iter().filter(|v| v.is_some()).count();
    let none_count = series.iter().filter(|v| v.is_none()).count();

    println!("resolve_dvol_override: Some={some_count}, None={none_count} (warm-up bars)");

    // The warm-up window is W=30 daily closes = at most 30 calendar days = ~720 hourly bars.
    // H1_2024 is ~4380 hourly bars; the vast majority must have a Some value.
    assert!(
        some_count > 720,
        "expected more than 720 bars to have a real DVOL value (most of H1_2024); got {some_count}. \
         Possible causes: corpus missing / SHA mismatch / dvol_as_of join broken."
    );

    // Sanity: the first 30-ish daily bars are None (warm-up expected).
    // This is not a hard requirement but a sensible check.
    let first_some_idx = series.iter().position(|v| v.is_some());
    println!(
        "resolve_dvol_override: first Some at bar index {:?}",
        first_some_idx
    );
    assert!(
        first_some_idx.is_some(),
        "expected at least one Some value in the series"
    );

    println!("resolve_dvol_override_returns_some_with_real_corpus: PASS");
}

// ── Test 2: full bakeoff run — v0.dvol_regime differs from v0.buyhold ─────────

/// Full bakeoff-path regression gate: proves `dvol_override` is `Some` + non-trivial.
///
/// Runs `run_bakeoff` for `BTCUSDT` on `H1_2024` with the real Binance corpus +
/// real DVOL corpus. Asserts the `v0.dvol_regime` candidate's final equity
/// DIFFERS from the `v0.buyhold` candidate's final equity.
///
/// **EQUAL equities → `dvol_override` is still None or all-None → the no-op trap.**
/// This is the bake-off-path equivalent of the existing day-1 divergence gate.
///
/// The test does NOT assert that `v0.dvol_regime` WINS (the honest prior is
/// FRAGILE / `BenchmarkWins`). It only asserts that the arm ran with a real DVOL
/// series, not the empty warm-up stub.
///
/// **Corpus-dependent on-machine test.** Requires:
/// - `data/binance/BTCUSDT/*.parquet` (Binance H1-2024 bars).
/// - `data/deribit-dvol/BTC/2024.parquet` (DVOL corpus, SHA-pinned).
///
/// Expected run time: ~2–5 min (11 arms × H1_2024 data × 1000 bootstrap paths).
/// To skip bootstrap and run faster (still proves wiring), set `RobustnessMode::Skip`.
#[tokio::test]
#[ignore = "corpus-dependent on-machine test: requires real Binance + DVOL corpus; ~2-5 min"]
async fn dvol_regime_bakeoff_differs_from_buyhold() {
    // Corpus paths (`data/binance`, `data/deribit-dvol`) are relative to
    // the workspace root. Integration tests run from crates/backtest/ — set cwd.
    let ws_root = workspace_root();
    std::env::set_current_dir(&ws_root).expect("set cwd to workspace root");

    let field = BakeoffConfig::default_field(); // includes v0.dvol_regime

    let cfg = BakeoffConfig {
        request: BakeoffRequest {
            symbol: Symbol::new("BTCUSDT"),
            range: DateRange::H1_2024,
            seed: LAB_SEED,
            field: field.clone(),
            timeframe: Horizon::OneHour,
            initial_capital: dec!(100_000),
        },
        data_source: ScenarioDataSource::BinanceCache,
        // Skip bootstrap for speed (wiring check, not robustness assessment).
        robustness: backtest::RobustnessMode::Skip,
    };

    let (handle, cancel_rx) = cancellation_pair();
    let progress_tx = ProgressSender::disabled();
    let bakeoff_progress_tx = BakeoffProgressSender::disabled();

    println!(
        "dvol_bakeoff_path_gate: starting bakeoff BTCUSDT H1_2024 ({} arms + buyhold)",
        field.len()
    );
    println!(
        "dvol_bakeoff_path_gate: this verifies dvol_override is Some + non-trivial end-to-end"
    );

    let report = run_bakeoff(cfg, cancel_rx, progress_tx, bakeoff_progress_tx)
        .await
        .expect("bakeoff must complete without error");

    drop(handle);

    // ── Find v0.dvol_regime and v0.buyhold candidates ─────────────────────────
    let dvol_candidate = report
        .candidates
        .iter()
        .find(|c| c.strategy.0.as_str() == "v0.dvol_regime")
        .expect("v0.dvol_regime must be present in candidates for BTCUSDT");

    let buyhold_candidate = report
        .candidates
        .iter()
        .find(|c| c.strategy.0.as_str() == "v0.buyhold")
        .expect("v0.buyhold benchmark must always be present");

    // ── Print honest results ──────────────────────────────────────────────────
    println!(
        "\ndvol_bakeoff_path_gate: v0.dvol_regime  sharpe={:.3}  total_return%={:.4}  max_dd%={:.4}  trades={}",
        dvol_candidate.kpis.sharpe,
        dvol_candidate.kpis.total_return_pct,
        dvol_candidate.kpis.max_drawdown,
        dvol_candidate.kpis.trade_count
    );
    println!(
        "dvol_bakeoff_path_gate: v0.buyhold      sharpe={:.3}  total_return%={:.4}  max_dd%={:.4}",
        buyhold_candidate.kpis.sharpe,
        buyhold_candidate.kpis.total_return_pct,
        buyhold_candidate.kpis.max_drawdown,
    );

    // ── Core assertion: equities DIFFER ──────────────────────────────────────
    // If dvol_override was None (the old stub), the arm is a warm-up-only
    // buy-and-hold proxy → equities are EQUAL (or within fp noise). A non-trivial
    // DVOL series causes at least one position flip → final equities diverge.
    let dvol_final_equity = dvol_candidate
        .equity_curve
        .last()
        .map(|(_, m)| m.amount())
        .expect("v0.dvol_regime must have at least one equity point");

    let buyhold_final_equity = buyhold_candidate
        .equity_curve
        .last()
        .map(|(_, m)| m.amount())
        .expect("v0.buyhold must have at least one equity point");

    println!("\ndvol_bakeoff_path_gate: v0.dvol_regime final equity = {dvol_final_equity}");
    println!("dvol_bakeoff_path_gate: v0.buyhold      final equity = {buyhold_final_equity}");

    let diff = (dvol_final_equity - buyhold_final_equity).abs();
    let diff_pct = if buyhold_final_equity > rust_decimal::Decimal::ZERO {
        diff / buyhold_final_equity * rust_decimal::Decimal::from(100)
    } else {
        rust_decimal::Decimal::ZERO
    };
    println!("dvol_bakeoff_path_gate: |dvol_equity - buyhold_equity| = {diff} ({diff_pct:.4}%)");

    // Minimum epsilon: 1 bp (0.01%) divergence. With a non-trivial DVOL series
    // that triggers at least one regime flip over H1_2024, the equities will
    // diverge far more than 1 bp in practice.
    // 1 bp of initial capital 100_000 = 10 USDT (minimum absolute divergence).
    let min_abs_diff = rust_decimal::Decimal::from(10);

    assert!(
        diff >= min_abs_diff,
        "FAIL: v0.dvol_regime and v0.buyhold final equities differ by only {diff} USDT \
         ({diff_pct:.4}%), which is less than the 1-bp minimum ({min_abs_diff} USDT). \
         This strongly suggests dvol_override is None (warm-up-only) or all-None — \
         the no-op trap from before Task 2. Check resolve_dvol_override in bakeoff/mod.rs."
    );

    // ── Bonus: report all arm outcomes ───────────────────────────────────────
    println!("\ndvol_bakeoff_path_gate: all arm results:");
    println!(
        "{:<35} {:>8} {:>12} {:>10}",
        "arm", "sharpe", "total_ret%", "trades"
    );
    for candidate in &report.candidates {
        println!(
            "{:<35} {:>8.3} {:>12.4} {:>10}",
            candidate.strategy.0.as_str(),
            candidate.kpis.sharpe,
            candidate.kpis.total_return_pct,
            candidate.kpis.trade_count,
        );
    }

    let outcome = &report.rationale.outcome;
    println!(
        "\ndvol_bakeoff_path_gate: recommendation outcome = {outcome:?} (FRAGILE/BenchmarkWins is the expected null)"
    );

    println!("\ndvol_regime_bakeoff_differs_from_buyhold: PASS");
    println!("The v0.dvol_regime arm ran with a real DVOL series (not the None stub).");
    println!("Honest verdict: FRAGILE / BenchmarkWins is the pre-registered expected outcome.");
}

// ── Test 2b: ETHUSDT bakeoff — honest ETH verdict ────────────────────────────

/// Mirror of `dvol_regime_bakeoff_differs_from_buyhold` for ETHUSDT (T8 requires
/// both BTC and ETH). Reports the honest ETH verdict.
///
/// Corpus-dependent: requires `data/binance/ETHUSDT/*.parquet` +
/// `data/deribit-dvol/ETH/2024.parquet`.
#[tokio::test]
#[ignore = "corpus-dependent on-machine test: requires real Binance + DVOL corpus (ETH); ~2-5 min"]
async fn dvol_regime_bakeoff_eth_differs_from_buyhold() {
    let ws_root = workspace_root();
    std::env::set_current_dir(&ws_root).expect("set cwd to workspace root");

    let field = BakeoffConfig::default_field();
    let cfg = BakeoffConfig {
        request: BakeoffRequest {
            symbol: Symbol::new("ETHUSDT"),
            range: DateRange::H1_2024,
            seed: LAB_SEED,
            field: field.clone(),
            timeframe: Horizon::OneHour,
            initial_capital: dec!(100_000),
        },
        data_source: ScenarioDataSource::BinanceCache,
        robustness: backtest::RobustnessMode::Skip,
    };

    let (handle, cancel_rx) = cancellation_pair();
    let progress_tx = ProgressSender::disabled();
    let bakeoff_progress_tx = BakeoffProgressSender::disabled();

    println!("dvol_bakeoff_path_gate[ETH]: starting bakeoff ETHUSDT H1_2024");

    let report = run_bakeoff(cfg, cancel_rx, progress_tx, bakeoff_progress_tx)
        .await
        .expect("ETHUSDT bakeoff must complete without error");

    drop(handle);

    let dvol_candidate = report
        .candidates
        .iter()
        .find(|c| c.strategy.0.as_str() == "v0.dvol_regime")
        .expect("v0.dvol_regime must be present for ETHUSDT");

    let buyhold_candidate = report
        .candidates
        .iter()
        .find(|c| c.strategy.0.as_str() == "v0.buyhold")
        .expect("v0.buyhold must always be present");

    println!(
        "dvol_bakeoff_path_gate[ETH]: v0.dvol_regime sharpe={:.3}  total_return%={:.4}  trades={}",
        dvol_candidate.kpis.sharpe,
        dvol_candidate.kpis.total_return_pct,
        dvol_candidate.kpis.trade_count,
    );
    println!(
        "dvol_bakeoff_path_gate[ETH]: v0.buyhold     sharpe={:.3}  total_return%={:.4}",
        buyhold_candidate.kpis.sharpe, buyhold_candidate.kpis.total_return_pct,
    );

    let dvol_final = dvol_candidate
        .equity_curve
        .last()
        .map(|(_, m)| m.amount())
        .expect("v0.dvol_regime must have at least one equity point");
    let buyhold_final = buyhold_candidate
        .equity_curve
        .last()
        .map(|(_, m)| m.amount())
        .expect("v0.buyhold must have at least one equity point");

    let diff = (dvol_final - buyhold_final).abs();
    let min_abs_diff = rust_decimal::Decimal::from(10); // 1 bp of 100_000

    println!("dvol_bakeoff_path_gate[ETH]: |dvol_equity - buyhold_equity| = {diff}");

    assert!(
        diff >= min_abs_diff,
        "FAIL: v0.dvol_regime and v0.buyhold equities differ by only {diff} — the no-op trap"
    );

    let outcome = &report.rationale.outcome;
    println!("dvol_bakeoff_path_gate[ETH]: recommendation = {outcome:?}");
    println!("dvol_regime_bakeoff_eth_differs_from_buyhold: PASS");
}

// ── Test 3: SOLUSDT bakeoff runs clean with v0.dvol_regime arm ABSENT ─────────

/// Regression: bakeoff on a non-BTC/ETH symbol (SOLUSDT) must complete cleanly
/// with the `v0.dvol_regime` arm absent from results (filtered by `dvol_sym_ok`).
///
/// Proves: the graceful-degradation path works — a non-supported symbol never
/// crashes the bakeoff because of the DVOL arm.
///
/// Uses `ScenarioDataSource::Synthetic` to avoid needing the Binance corpus for SOL.
/// Robustness skipped for speed.
///
/// **Corpus-independent test** (synthetic bars; no Binance/DVOL corpus needed).
/// Still `#[ignore]` because it runs the full bakeoff loop (~10 arms) which is slow.
#[tokio::test]
#[ignore = "slow (~30s); verifies SOLUSDT bakeoff runs clean without v0.dvol_regime; corpus-independent"]
async fn solusdt_bakeoff_runs_clean_without_dvol_arm() {
    let field = BakeoffConfig::default_field(); // includes v0.dvol_regime in the field

    let cfg = BakeoffConfig {
        request: BakeoffRequest {
            symbol: Symbol::new("SOLUSDT"),
            range: DateRange::H2_2024,
            seed: LAB_SEED,
            field: field.clone(),
            timeframe: Horizon::OneHour,
            initial_capital: dec!(100_000),
        },
        // Synthetic so we don't need the corpus for SOLUSDT.
        data_source: ScenarioDataSource::Synthetic,
        robustness: backtest::RobustnessMode::Skip,
    };

    let (handle, cancel_rx) = cancellation_pair();
    let progress_tx = ProgressSender::disabled();
    let bakeoff_progress_tx = BakeoffProgressSender::disabled();

    println!(
        "solusol_bakeoff: starting SOLUSDT H2_2024 (synthetic) — v0.dvol_regime must be absent"
    );

    let report = run_bakeoff(cfg, cancel_rx, progress_tx, bakeoff_progress_tx)
        .await
        .expect("SOLUSDT bakeoff must complete without error (DVOL arm skipped gracefully)");

    drop(handle);

    // v0.dvol_regime must NOT appear in results (filtered by dvol_sym_ok).
    let dvol_in_results = report
        .candidates
        .iter()
        .any(|c| c.strategy.0.as_str() == "v0.dvol_regime");

    assert!(
        !dvol_in_results,
        "v0.dvol_regime must be ABSENT from SOLUSDT bakeoff results \
         (only BTC/ETH are supported; the arm should have been skipped via dvol_sym_ok guard)"
    );

    // v0.buyhold must always be present.
    assert!(
        report
            .candidates
            .iter()
            .any(|c| c.strategy.0.as_str() == "v0.buyhold"),
        "v0.buyhold benchmark must always be present"
    );

    println!(
        "solusdt_bakeoff: {} candidates, v0.dvol_regime absent (correct).",
        report.candidates.len()
    );
    println!("solusdt_bakeoff_runs_clean_without_dvol_arm: PASS");
}
