//! T14 — decisive signal-library bake-off (ADR-0071 Phase 6).
//!
//! Runs the full 18-arm advisor bake-off (9 single engines + 8 vote ensembles +
//! buy-and-hold) on the real Binance H1-2024 corpus and reports honest per-arm
//! robustness outcomes.
//!
//! ## What this proves
//!
//! - The 5 ADR-0071 arms (donchian_break, donchian_floor, vol_breakout,
//!   roc_momentum, obv) execute without error on real-data (no DSL parse/eval panic).
//! - Each new arm's Sharpe is reported. Fragile is the EXPECTED, VALID,
//!   pre-registered outcome per tasks.md: "most new signals are also Fragile —
//!   a null result is valid + shippable."
//! - The overall recommendation (BenchmarkWins / single best / ensemble) is
//!   reported honestly.
//!
//! ## Run
//!
//! ```sh
//! cargo test -p backtest --test signal_library_bakeoff_t14 -- --ignored 2>&1 \
//!   | tee /tmp/signal-library-bakeoff.log
//! ```
//!
//! This test is `#[ignore]` by default (requires real Binance corpus + ~2-5 min).

#![allow(clippy::float_arithmetic, clippy::unwrap_used)]

use backtest::{
    BakeoffConfig, BakeoffRequest, DateRange, RobustnessMode,
    bakeoff::RobustnessFlag,
    cancel::cancellation_pair,
    engine::ScenarioDataSource,
    progress::{BakeoffProgressSender, ProgressSender},
};
use rust_decimal_macros::dec;
use trading_core::{StrategyId, Symbol};

/// T14 — decisive bake-off on real Binance H1-2024 data.
///
/// Bootstrap: 1000 paths, LAB_DEFAULT_SEED (matches the advisor path in runner.rs).
/// Field: all 17 arms from `advisor_field()` in leaderboard/runner.rs
///   = 9 single engines (4 original + 5 ADR-0071) + 8 vote ensembles.
/// Benchmark: buy-and-hold (appended by `run_bakeoff`).
/// Total: 18 arms.
///
/// Outcome reported in println — read from /tmp/signal-library-bakeoff.log.
/// Expected: most/all ADR-0071 arms are Fragile → BenchmarkWins.
/// This is the VALID, PRE-REGISTERED outcome. Report it honestly.
#[tokio::test]
#[ignore = "requires real Binance corpus + ~2-5 min; run with -- --ignored"]
async fn t14_decisive_signal_library_bakeoff() {
    // Mirror the LAB_DEFAULT_SEED from crates/ui/src/lab/defaults.rs
    // (hardcoded here to avoid a ui → backtest circular dep).
    // The actual value doesn't matter for the test assertion, only for
    // reproducibility of the bootstrap.
    let lab_seed: [u8; 32] = [
        0x42, 0x49, 0x4e, 0x41, // "BINA"
        0x4e, 0x43, 0x45, 0x31, // "NCE1"
        0x42, 0x49, 0x4e, 0x41, // "BINA"
        0x4e, 0x43, 0x45, 0x31, // "NCE1"
        0x42, 0x49, 0x4e, 0x41, // "BINA"
        0x4e, 0x43, 0x45, 0x31, // "NCE1"
        0x42, 0x49, 0x4e, 0x41, // "BINA"
        0x4e, 0x43, 0x45, 0x31, // "NCE1"
    ];

    // Field: 10 single engines + 8 vote ensembles (default_field + default_ensemble_field).
    // advisor_field() in runner.rs additionally carries the v0.macro_riskon overlay (= 19);
    // this signal-library bake-off excludes it by design.
    let mut field: Vec<StrategyId> = BakeoffConfig::default_field();
    field.extend(BakeoffConfig::default_ensemble_field());
    let expected_field_len = 18; // 10 single (incl. v0.obv + v0.dvol_regime) + 8 ensemble
    assert_eq!(
        field.len(),
        expected_field_len,
        "expected {expected_field_len} arms in field (10 single + 8 ensemble)"
    );

    // Bootstrap: 1000 paths, same seed as the advisor path.
    let seed_u64 = u64::from_le_bytes([
        lab_seed[0],
        lab_seed[1],
        lab_seed[2],
        lab_seed[3],
        lab_seed[4],
        lab_seed[5],
        lab_seed[6],
        lab_seed[7],
    ]);

    let cfg = BakeoffConfig {
        request: BakeoffRequest {
            symbol: Symbol::new("BTCUSDT"),
            range: DateRange::H1_2024,
            seed: lab_seed,
            field: field.clone(),
            timeframe: backtest::resample::Horizon::OneHour,
            initial_capital: dec!(100_000),
        },
        data_source: ScenarioDataSource::BinanceCache,
        robustness: RobustnessMode::Bootstrap {
            paths: 1000,
            seed: seed_u64,
        },
    };

    let (handle, cancel_rx) = cancellation_pair();
    let progress_tx = ProgressSender::disabled();
    let bakeoff_progress_tx = BakeoffProgressSender::disabled();

    println!("T14: Starting decisive bake-off — 18 arms × 1000 bootstrap paths");
    println!("T14: Symbol=BTCUSDT, Range=H1_2024, DataSource=BinanceCache");
    println!("T14: Field ({} arms + buyhold):", field.len());
    for id in &field {
        println!("  - {}", id.0.as_str());
    }
    println!("--- RUNNING ---");

    let report = backtest::bakeoff::run_bakeoff(cfg, cancel_rx, progress_tx, bakeoff_progress_tx)
        .await
        .expect("bakeoff must complete without error on real Binance H1-2024 data");

    drop(handle);

    // ── Report honest results ──────────────────────────────────────────────────
    println!("\n=== T14 BAKE-OFF RESULTS (BTCUSDT H1-2024) ===");
    println!(
        "{:<35} {:>8} {:>8} {:>8} {:>12} {:>10}",
        "arm", "sharpe", "sortino", "calmar", "total_ret%", "robustness"
    );

    let adr0071_ids = [
        "v0.donchian_break",
        "v0.donchian_floor",
        "v0.vol_breakout",
        "v0.roc_momentum",
        "v0.obv",
    ];

    let mut adr0071_results: Vec<(&str, f64, Option<RobustnessFlag>)> = Vec::new();

    for (idx, candidate) in report.candidates.iter().enumerate() {
        let id = candidate.strategy.0.as_str();
        let kpis = &candidate.kpis;
        let rob = match candidate.robustness {
            Some(RobustnessFlag::Robust) => "Robust",
            Some(RobustnessFlag::Marginal) => "Marginal",
            Some(RobustnessFlag::Fragile) => "Fragile",
            Some(RobustnessFlag::Skipped) => "Skipped",
            None => "—",
        };
        let is_crowned = report.crowned == Some(idx);
        let crown_marker = if is_crowned { " <== CROWNED" } else { "" };
        println!(
            "{:<35} {:>8.3} {:>8.3} {:>8.3} {:>12.4} {:>10}{}",
            id, kpis.sharpe, kpis.sortino, kpis.calmar, kpis.total_return_pct, rob, crown_marker
        );

        if adr0071_ids.contains(&id) {
            adr0071_results.push((id, kpis.sharpe, candidate.robustness));
        }
    }

    println!("\n--- ADR-0071 NEW ARM SUMMARY ---");
    for (id, sharpe, flag) in &adr0071_results {
        let flag_str = match flag {
            Some(RobustnessFlag::Robust) => "Robust",
            Some(RobustnessFlag::Marginal) => "Marginal",
            Some(RobustnessFlag::Fragile) => "Fragile",
            Some(RobustnessFlag::Skipped) => "Skipped",
            None => "—",
        };
        println!("  {:<35} sharpe={:.3}  flag={}", id, sharpe, flag_str);
    }

    // ── Recommendation ─────────────────────────────────────────────────────────
    let rec = &report.rationale;
    println!("\n--- RECOMMENDATION ---");
    println!("  outcome: {:?}", rec.outcome);
    if let Some(crown_idx) = report.crowned
        && let Some(crown) = report.candidates.get(crown_idx)
    {
        println!(
            "  crowned: {} (sharpe={:.3})",
            crown.strategy.0.as_str(),
            crown.kpis.sharpe
        );
    }

    // ── Assertion: all 5 new arms ran without error (present in results) ───────
    for adr_id in &adr0071_ids {
        let found = report
            .candidates
            .iter()
            .any(|c| c.strategy.0.as_str() == *adr_id);
        assert!(
            found,
            "ADR-0071 arm {adr_id} must appear in bakeoff results — \
             it may have failed to run or wasn't included in the field"
        );
    }

    // ── Assertion: total candidate count = field + buyhold ────────────────────
    assert_eq!(
        report.candidates.len(),
        field.len() + 1, // +1 for buy-and-hold
        "expected {} candidates (field={} + buyhold=1); got {}",
        field.len() + 1,
        field.len(),
        report.candidates.len()
    );

    println!(
        "\nT14: DONE — all {} arms completed, all 5 ADR-0071 arms present.",
        report.candidates.len()
    );
    println!("Expected: ADR-0071 arms are Fragile (pre-registered null result is VALID).");

    // The expected outcome for ADR-0071 arms on H1-2024 data:
    // Most are Fragile (consistent with the 2026-06-08 finding that all families
    // are Fragile under the frozen gate). BenchmarkWins or a non-ADR-0071 arm
    // is crowned. This is VALID — the null result was explicitly pre-registered.
    // DO NOT adjust bands to manufacture a passing ADR-0071 arm.
}
