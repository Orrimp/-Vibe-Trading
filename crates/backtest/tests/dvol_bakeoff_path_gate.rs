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
//! ## Why the original assertion was vacuous (review 3-15 CRITICAL)
//!
//! Both bake-off tests asserted that `v0.dvol_regime`'s final equity DIFFERS
//! from `v0.buyhold`'s by ≥ 1 bp. But `v0.dvol_regime` runs at
//! `FixedFractionSizer(0.10)` — **10% invested** — while `v0.buyhold` is **100%
//! invested**. On H1-2024 (BTC ≈ +47.8%) those two curves are ~48,000 USDT apart
//! *by construction*, with the signal completely dead: measured under the exact
//! defect the tests were written to catch (`resolve_dvol_override` returning the
//! `None` stub) they still passed by ~4,778×.
//!
//! **The replacement discriminator** is the arm's own fill history:
//! `trade_count >= 2`. A dispatched arm handed an empty DVOL series runs
//! permanent warm-up, which (after the 3-15 warm-up fix) means it enters the coin
//! once and never moves again — **exactly one** fill. Two or more fills can only
//! come from a regime flip, which can only come from a real DVOL series. And with
//! the bug-log #78 ABSENCE fix the arm is now DROPPED from the field entirely
//! when the series cannot be resolved, so the `expect("v0.dvol_regime must be
//! present")` above it is a second, independent red under the same defect.
//!
//! ## Tests
//!
//! 1. `resolve_dvol_override_returns_some_with_real_corpus` — fast unit test:
//!    calls `resolve_dvol_override` directly and asserts `Some` + non-empty.
//!    Checks the loader plumbing is wired before launching the full bakeoff.
//!
//! 2. `dvol_regime_bakeoff_ran_a_real_series` / `..._eth_...` — full `run_bakeoff`
//!    on BTCUSDT/ETHUSDT H1-2024 (the frozen robustness window); asserts the arm
//!    is PRESENT and completed at least one round trip.
//!
//! 3. `solusdt_bakeoff_runs_clean_without_dvol_arm` — corpus-INDEPENDENT; the arm
//!    must be absent for a non-BTC/ETH coin. Not `#[ignore]`d (review 3-15).
//!
//! 4. `corpus_gated_tests_are_declared_and_counted` — corpus-independent
//!    skip-visibility guard: prints a loud banner naming every `#[ignore]`d test
//!    in this file and whether the corpus that would let them run is present.
//!
//! ## Corpus dependency
//!
//! Tests 1 and 2 require `data/deribit-dvol/{BTC,ETH}/{2023,2024}.parquet` on
//! disk (gitignored; aggregate SHA pinned to `EXPECTED_DVOL_REVISION_SHA`). They
//! are `#[ignore]` by default and tagged as **corpus-dependent on-machine
//! tests** — and, per review 3-15 MEDIUM, no CI job runs `--ignored`, so test 4
//! exists to make that skip visible instead of silent.
//!
//! ## Run
//!
//! ```sh
//! cargo test -p backtest --test dvol_bakeoff_path_gate -- --include-ignored 2>&1 \
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

    // **Review 3-15 MEDIUM — the pre-extension witness.**
    //
    // The load span used to be exactly the backtest range, so the very first
    // evaluated bars had NO as-of DVOL value at all and the W=30 median ring
    // filled INSIDE the evaluation window (~16% of H1-2024 structurally
    // pre-signal). `resolve_dvol_override` now pre-extends the LOAD span by
    // `DVOL_WARMUP_DAYS` while the returned series stays aligned to the
    // evaluation bars, so EVERY bar — including bar 0 — must carry a value.
    assert_eq!(
        none_count, 0,
        "every evaluated bar must have an as-of DVOL value: {none_count} of \
         {bar_count} are None. A non-zero count means the load span was not \
         pre-extended and the median ring is warming up inside the evaluation \
         window (review 3-15 MEDIUM)."
    );
    assert_eq!(
        series.iter().position(|v| v.is_some()),
        Some(0),
        "the FIRST evaluated bar must already see a DVOL close"
    );
    assert_eq!(some_count, bar_count);

    println!("resolve_dvol_override_returns_some_with_real_corpus: PASS");
}

// ── Test 2: full bakeoff run — v0.dvol_regime ran a REAL series ──────────────

/// Full bakeoff-path regression gate: proves `dvol_override` is `Some` + non-trivial.
///
/// Runs `run_bakeoff` for `BTCUSDT` on `H1_2024` with the real Binance corpus +
/// real DVOL corpus and asserts two independent things about the
/// `v0.dvol_regime` candidate:
///
/// 1. **It is PRESENT.** With the bug-log #78 ABSENCE fix, a missing/stale/
///    unresolvable DVOL series drops the arm from the ranked field entirely, so
///    the `expect` below is itself a red under the `None`-stub defect.
/// 2. **It completed a round trip** (`trade_count >= 2`). The arm handed an empty
///    series runs permanent warm-up = holds the coin from bar 0 and never moves
///    → exactly ONE fill. A second fill requires a regime flip, which requires a
///    real DVOL series.
///
/// The `v0.buyhold` comparison that used to be the assertion is kept as a
/// PRINTED DIAGNOSTIC only: the arm is 10%-invested and the benchmark is
/// 100%-invested, so their gap is structural sizing (~48k USDT on H1-2024) and
/// passes with the signal fully dead. See the module docs.
///
/// The test does NOT assert that `v0.dvol_regime` WINS (the honest prior is
/// FRAGILE / `BenchmarkWins`). It only asserts that the arm ran with a real DVOL
/// series, not the empty warm-up stub.
///
/// **Corpus-dependent on-machine test.** Requires:
/// - `data/binance/BTCUSDT/*.parquet` (Binance H1-2024 bars).
/// - `data/deribit-dvol/BTC/{2023,2024}.parquet` (DVOL corpus, SHA-pinned; 2023
///   is needed since the load span is pre-extended for median warm-up).
///
/// Expected run time: ~2–5 min (11 arms × H1_2024 data × 1000 bootstrap paths).
/// To skip bootstrap and run faster (still proves wiring), set `RobustnessMode::Skip`.
#[tokio::test]
#[ignore = "corpus-dependent on-machine test: requires real Binance + DVOL corpus; ~2-5 min"]
async fn dvol_regime_bakeoff_ran_a_real_series() {
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
    // RED #1 under the `None`-stub defect: with the bug-log #78 ABSENCE fix an
    // unresolvable DVOL series removes the arm from the field entirely.
    let dvol_candidate = report
        .candidates
        .iter()
        .find(|c| c.strategy.0.as_str() == "v0.dvol_regime")
        .expect(
            "v0.dvol_regime must be present in candidates for BTCUSDT. Absent means \
             resolve_dvol_override returned None (corpus missing, SHA mismatch, or \
             the window is not covered) and the arm was correctly DROPPED — which is \
             the honest behaviour, but it means this gate has no arm to check.",
        );

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

    // ── RED #2: the arm completed a ROUND TRIP ───────────────────────────────
    //
    // This is the non-vacuous discriminator (review 3-15 CRITICAL). An arm handed
    // an empty/all-None series runs permanent warm-up: it enters the coin on bar 0
    // and never moves → trade_count == 1. Two or more fills require a regime flip,
    // which requires a real DVOL series joined to the bars.
    assert!(
        dvol_candidate.kpis.trade_count >= 2,
        "FAIL: v0.dvol_regime completed {} fill(s) over H1-2024. The arm must ENTER \
         and EXIT at least once — trade_count <= 1 is the signature of an empty \
         DVOL series (permanent warm-up), i.e. the `dvol_override: None` no-op trap \
         this gate exists to catch. Check resolve_dvol_override in bakeoff/mod.rs.",
        dvol_candidate.kpis.trade_count
    );

    // ── Diagnostic ONLY: the buy-and-hold gap ────────────────────────────────
    //
    // v0.dvol_regime is 10%-invested (FixedFractionSizer(0.10)); v0.buyhold is
    // 100%-invested. On H1-2024 (BTC ≈ +47.8%) the gap is ~48,000 USDT with the
    // signal fully dead, which is why this is printed and NOT asserted.
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
    println!(
        "dvol_bakeoff_path_gate: [diagnostic, NOT a gate] |dvol_equity - buyhold_equity| \
         = {diff} ({diff_pct:.4}%) — 10%-invested vs 100%-invested, structural"
    );
    println!(
        "dvol_bakeoff_path_gate: [GATE] v0.dvol_regime trade_count = {} (must be >= 2)",
        dvol_candidate.kpis.trade_count
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

    println!("\ndvol_regime_bakeoff_ran_a_real_series: PASS");
    println!("The v0.dvol_regime arm ran with a real DVOL series (not the None stub).");
    println!("Honest verdict: FRAGILE / BenchmarkWins is the pre-registered expected outcome.");
}

// ── Test 2b: ETHUSDT bakeoff — honest ETH verdict ────────────────────────────

/// Mirror of `dvol_regime_bakeoff_ran_a_real_series` for ETHUSDT (T8 requires
/// both BTC and ETH). Reports the honest ETH verdict.
///
/// Corpus-dependent: requires `data/binance/ETHUSDT/*.parquet` +
/// `data/deribit-dvol/ETH/{2023,2024}.parquet`.
#[tokio::test]
#[ignore = "corpus-dependent on-machine test: requires real Binance + DVOL corpus (ETH); ~2-5 min"]
async fn dvol_regime_bakeoff_eth_ran_a_real_series() {
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

    // RED #1: absent → resolve_dvol_override returned None → arm correctly dropped.
    let dvol_candidate = report
        .candidates
        .iter()
        .find(|c| c.strategy.0.as_str() == "v0.dvol_regime")
        .expect(
            "v0.dvol_regime must be present for ETHUSDT. Absent means the DVOL series \
             could not be resolved and the arm was DROPPED (bug-log #78 ABSENCE fix).",
        );

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
    println!(
        "dvol_bakeoff_path_gate[ETH]: [diagnostic, NOT a gate] \
         |dvol_equity - buyhold_equity| = {diff} (10%- vs 100%-invested, structural)"
    );

    // RED #2 — the non-vacuous discriminator (review 3-15 CRITICAL): an empty
    // DVOL series yields permanent warm-up = exactly ONE fill (enter and hold).
    assert!(
        dvol_candidate.kpis.trade_count >= 2,
        "FAIL: v0.dvol_regime completed {} fill(s) on ETHUSDT H1-2024. \
         trade_count <= 1 is the empty-series / no-op signature.",
        dvol_candidate.kpis.trade_count
    );

    let outcome = &report.rationale.outcome;
    println!("dvol_bakeoff_path_gate[ETH]: recommendation = {outcome:?}");
    println!("dvol_regime_bakeoff_eth_ran_a_real_series: PASS");
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
///
/// Review 3-15 MEDIUM (skip-visibility): this was `#[ignore]`d **only for being
/// slow** while being the one test in this file that needs no corpus — so the
/// entire loader→join→orchestrator chain had zero automated coverage, including
/// the ADR-0072 D8 absence guard it exercises. Un-ignored: ~30s is a price worth
/// paying for the only non-corpus gate here.
#[tokio::test]
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

// ── Test 3b: ABSENCE for a SUPPORTED coin when the series is unavailable ─────

/// **bug-log #78 gate — corpus-INDEPENDENT.**
///
/// The arm used to be dropped only for an unsupported *coin*. When the DVOL
/// series could not be resolved for a SUPPORTED coin — corpus missing (the
/// parquets are gitignored, so: every fresh clone and CI box), SHA mismatch, or
/// a window the frozen corpus no longer covers — `resolve_dvol_override`
/// returned `None`, the arm was dispatched anyway, and
/// `cfg.dvol_override.unwrap_or_default()` handed the engine an empty series.
/// The result sat in the ranked field labelled *"Implied-vol regime (hold when
/// DVOL < 30-day median)"* having tested nothing.
///
/// This test reaches that path without needing any corpus: on
/// `ScenarioDataSource::Synthetic` there are no preloaded real bars, so there is
/// no grid to join a DVOL series onto and `dvol_override` is `None` — for
/// **BTCUSDT**, a fully supported coin. The arm must be ABSENT.
///
/// RED before the fix: the arm was present with `trade_count == 0`.
#[tokio::test]
async fn btcusdt_bakeoff_drops_dvol_arm_when_series_unavailable() {
    let field = BakeoffConfig::default_field(); // includes v0.dvol_regime

    let cfg = BakeoffConfig {
        request: BakeoffRequest {
            symbol: Symbol::new("BTCUSDT"), // SUPPORTED coin — not the D8 filter
            range: DateRange::H2_2024,
            seed: LAB_SEED,
            field,
            timeframe: Horizon::OneHour,
            initial_capital: dec!(100_000),
        },
        // Synthetic → `preloaded_bars` is None → no DVOL series can be resolved.
        data_source: ScenarioDataSource::Synthetic,
        robustness: backtest::RobustnessMode::Skip,
    };

    let (handle, cancel_rx) = cancellation_pair();
    let report = run_bakeoff(
        cfg,
        cancel_rx,
        ProgressSender::disabled(),
        BakeoffProgressSender::disabled(),
    )
    .await
    .expect("bakeoff must complete without error when the DVOL series is unavailable");
    drop(handle);

    let dvol_row = report
        .candidates
        .iter()
        .find(|c| c.strategy.0.as_str() == "v0.dvol_regime");

    assert!(
        dvol_row.is_none(),
        "v0.dvol_regime must be ABSENT from the ranked field when its series cannot \
         be resolved — degrade to ABSENCE, never to a substitute wearing the probe's \
         label (bug-log #78). Found it present with trade_count={}, \
         total_return%={}. An operator reading that row is told the implied-vol \
         channel was tested on their coin; nothing was tested.",
        dvol_row.map_or(0, |c| c.kpis.trade_count),
        dvol_row.map_or(rust_decimal::Decimal::ZERO, |c| c.kpis.total_return_pct),
    );

    // The rest of the field is untouched — ABSENCE removes one arm, it does not
    // break the run.
    assert!(
        report
            .candidates
            .iter()
            .any(|c| c.strategy.0.as_str() == "v0.buyhold"),
        "v0.buyhold benchmark must always be present"
    );
    assert!(
        report.candidates.len() >= 9,
        "only the DVOL arm may be dropped; got {} candidates",
        report.candidates.len()
    );
    println!(
        "btcusdt_bakeoff_drops_dvol_arm_when_series_unavailable: {} candidates, \
         v0.dvol_regime ABSENT (correct).",
        report.candidates.len()
    );
}

// ── Test 4: skip-visibility — the ignored tests are declared and counted ──────

/// Every corpus-gated test in this file, by name. Kept in sync with the
/// `#[ignore]` attributes by `corpus_gated_tests_are_declared_and_counted`.
const CORPUS_GATED_TESTS: &[&str] = &[
    "resolve_dvol_override_returns_some_with_real_corpus",
    "dvol_regime_bakeoff_ran_a_real_series",
    "dvol_regime_bakeoff_eth_ran_a_real_series",
];

/// **Review 3-15 MEDIUM — make the skips LOUD and COUNTED.**
///
/// Every test that touches the real corpus, the loader, or
/// `resolve_dvol_override` is `#[ignore]`d, and no CI job runs `--ignored`. The
/// loader→join→orchestrator chain therefore had zero automated coverage while
/// the suite reported green. A silent skip is indistinguishable from a pass.
///
/// This test is corpus-INDEPENDENT and always runs. It:
/// 1. asserts the declared inventory above matches the `#[ignore]` attributes
///    actually present in this source file (so adding a fourth ignored test
///    without declaring it fails here), and
/// 2. prints a loud banner saying whether the corpus that would unlock them is
///    present, and the exact command to run them.
///
/// It deliberately does NOT fail when the corpus is absent — that is a legitimate
/// state for a fresh clone. It fails when the skips become *invisible*.
#[test]
fn corpus_gated_tests_are_declared_and_counted() {
    let this_file = include_str!("dvol_bakeoff_path_gate.rs");
    // Count real ATTRIBUTES only — a line that *starts* with `#[ignore`. Prose
    // in the doc comments mentions `#[ignore]` many times over.
    let ignored_attrs = this_file
        .lines()
        .filter(|l| l.trim_start().starts_with("#[ignore"))
        .count();

    assert_eq!(
        ignored_attrs,
        CORPUS_GATED_TESTS.len(),
        "this file has {ignored_attrs} `#[ignore]`d test(s) but CORPUS_GATED_TESTS \
         declares {}. Every skip must be declared here so it is counted and \
         reported instead of silently vanishing (review 3-15 skip-visibility). \
         Declared: {CORPUS_GATED_TESTS:?}",
        CORPUS_GATED_TESTS.len()
    );
    for name in CORPUS_GATED_TESTS {
        assert!(
            this_file.contains(&format!("fn {name}(")),
            "declared corpus-gated test `{name}` does not exist in this file — the \
             inventory has drifted from the tests"
        );
    }

    let manifest = workspace_root().join("data/deribit-dvol/REVISION.toml");
    let corpus_present = manifest.exists();

    eprintln!("┌───────────────────────────────────────────────────────────────────────");
    eprintln!("│ DVOL bake-off path gate — SKIP REPORT");
    eprintln!(
        "│ corpus `data/deribit-dvol/REVISION.toml`: {}",
        if corpus_present {
            "PRESENT"
        } else {
            "ABSENT (gitignored — fetch it with `cargo run -p data --bin fetch_deribit_dvol`)"
        }
    );
    eprintln!(
        "│ {} corpus-gated test(s) are #[ignore]d and DID NOT RUN in this invocation:",
        CORPUS_GATED_TESTS.len()
    );
    for name in CORPUS_GATED_TESTS {
        eprintln!("│   - {name}");
    }
    eprintln!(
        "│ Run them with: cargo test -p backtest --features realdata \\\n\
         │                  --test dvol_bakeoff_path_gate -- --include-ignored --nocapture"
    );
    if corpus_present {
        eprintln!("│ ⚠ the corpus IS on this machine, so these SHOULD be run before any verdict.");
    }
    eprintln!("└───────────────────────────────────────────────────────────────────────");
}
