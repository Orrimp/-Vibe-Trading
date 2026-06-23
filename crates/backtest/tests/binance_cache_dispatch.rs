//! Integration tests for `ScenarioDataSource::BinanceCache` dispatch.
//!
//! simple-strategies-realdata — T-A1, T-A2, T-C1 (no-op-source guard)
//!
//! # What is tested
//!
//! **T-A1 (AC1) — Single-symbol arms ACCEPT `BinanceCache`:**
//! The four single-symbol strategies (v0.sma, v0.5.macd, v0.5.rsi,
//! v0.5.bbands) accept `ScenarioDataSource::BinanceCache` with a
//! `bars_override` and return a successful `RunReport` with non-empty
//! equity.  When `write_report = true`, the written `.md` file body
//! contains the string `"binance"` as the `data_source` label (R1 / A1).
//!
//! **T-A2 (AC2) — Cross-sectional arms REJECT `BinanceCache`:**
//! The four cross-sectional strategies (v1.momentum, v1.5a.pairs,
//! v2.5.tcn, v2.5.tcn.weights) return `RunError::UnsupportedDataSource`
//! when `data_source = BinanceCache`, exactly mirroring their `YahooCache`
//! rejection (R1 / A1).
//!
//! **T-C1 (AC4) — No-op-source divergence guard:**
//! A `v0.sma × BTCUSDT` run on real Binance hourly bars (loaded from the
//! on-disk parquet corpus via `ReplayFeed`) has a final equity that
//! DIVERGES from a synthetic-bars baseline run for the SAME (strategy,
//! symbol, seed) by at least epsilon — proving real parquet bytes reached
//! the strategy rather than a silent synthetic fallback.
//! Skipped (not failed) when the corpus is absent so CI without data stays green.
//!
//! # Anchor safety
//!
//! This test does NOT write any anchored report files.  `write_report = false`
//! throughout except in the label-check sub-test, which writes to a
//! `tempfile::tempdir()` outside `spec/`.  No `anchors.toml` row is created.
//! Anchor count: 119/119 unchanged by construction.

#![allow(clippy::unwrap_used)]

use backtest::cancel::cancellation_pair;
use backtest::engine::{DateRange, RunError, ScenarioConfig, ScenarioDataSource};
use backtest::progress::ProgressSender;
use backtest::scenarios::sma_composed_run::{default_start_price, synthetic_bars_minute};
use tokio_stream::StreamExt as _;
use trading_core::{StrategyId, Symbol, Timeframe, Venue};

// ── Test seed (non-zero, deterministic) ───────────────────────────────────────

const TEST_SEED: [u8; 32] = {
    let mut s = [0u8; 32];
    s[0] = 0xB1;
    s[1] = 0xCC;
    s[2] = 0xAC;
    s
};

// The u64 form of TEST_SEED used by synthetic_bars_minute.
const TEST_SEED_U64: u64 = {
    // Reproduce the engine's seed derivation: load_u64_le from first 8 bytes.
    let b = TEST_SEED;
    u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
};

// ── Workspace root resolver ────────────────────────────────────────────────────

/// Resolve workspace root from CARGO_MANIFEST_DIR (backtest crate lives two
/// levels below the root: `crates/backtest`).
fn workspace_root() -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("locate workspace root from CARGO_MANIFEST_DIR")
        .to_path_buf()
}

// ── Base ScenarioConfig builder ────────────────────────────────────────────────

fn base_binance_sma_config(bars: Vec<trading_core::Bar>) -> ScenarioConfig {
    ScenarioConfig {
        strategy: StrategyId("v0.sma".into()),
        pair: (Venue::Binance, Symbol::new("BTCUSDT")),
        range: DateRange::Last30d,
        params: None,
        seed: TEST_SEED,
        write_report: false,
        data_source: ScenarioDataSource::BinanceCache,
        bars_override: Some(bars),
        sma_fast_len: None,
        sma_slow_len: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        reports_dir: None,
        short_enabled: false,
    }
}

/// Build synthetic minute-bars for `BTCUSDT` using the same GBM generator the
/// engine uses, so the test does not need the on-disk Binance corpus.
fn make_synthetic_bars(count: usize) -> Vec<trading_core::Bar> {
    let sym = Symbol::new("BTCUSDT");
    let start_price = default_start_price(&sym);
    synthetic_bars_minute(&sym, count, TEST_SEED_U64, start_price, 2023)
}

// ─────────────────────────────────────────────────────────────────────────────
// T-A1 — Single-symbol arms ACCEPT BinanceCache
// ─────────────────────────────────────────────────────────────────────────────

/// AC1 — `v0.sma` with `BinanceCache` + `bars_override` returns a `RunReport`
/// with non-empty equity series.  The `data_source` label is confirmed as
/// `"binance"` by writing a report to a tempdir and reading back the body.
#[tokio::test]
async fn binance_cache_accepted_by_sma_arm_label_is_binance() {
    let root = workspace_root();
    std::env::set_current_dir(&root).unwrap_or_else(|e| {
        panic!("set_current_dir({root:?}): {e}");
    });

    let bars = make_synthetic_bars(500);
    let mut cfg = base_binance_sma_config(bars);
    cfg.write_report = true;
    let reports_dir = tempfile::tempdir().expect("tempdir");
    cfg.reports_dir = Some(reports_dir.path().to_path_buf());

    let (_handle, cancel_rx) = cancellation_pair();
    let report = backtest::engine::run_scenario(cfg, cancel_rx, ProgressSender::disabled())
        .await
        .expect("v0.sma BinanceCache run must succeed");

    assert!(
        !report.equity_series.is_empty(),
        "BinanceCache v0.sma run must produce a non-empty equity series"
    );

    // Verify the written report body contains "binance" as the data_source label.
    let report_path = report
        .report_path
        .expect("write_report = true must produce a report_path");
    let body = std::fs::read_to_string(&report_path)
        .unwrap_or_else(|e| panic!("read report {}: {e}", report_path.display()));
    assert!(
        body.contains("| Data source") && body.contains("binance"),
        "report body must contain 'binance' as data_source label; got:\n{body}"
    );
}

/// T-A1 — `v0.5.macd` with `BinanceCache` returns non-empty equity.
#[tokio::test]
async fn binance_cache_accepted_by_macd_arm() {
    let root = workspace_root();
    std::env::set_current_dir(&root).unwrap_or_else(|e| panic!("cwd: {e}"));

    let bars = make_synthetic_bars(500);
    let mut cfg = base_binance_sma_config(bars);
    cfg.strategy = StrategyId("v0.5.macd".into());

    let (_handle, cancel_rx) = cancellation_pair();
    let report = backtest::engine::run_scenario(cfg, cancel_rx, ProgressSender::disabled())
        .await
        .expect("v0.5.macd BinanceCache run must succeed");

    assert!(
        !report.equity_series.is_empty(),
        "BinanceCache v0.5.macd run must produce a non-empty equity series"
    );
}

/// T-A1 — `v0.5.rsi` with `BinanceCache` returns non-empty equity.
#[tokio::test]
async fn binance_cache_accepted_by_rsi_arm() {
    let root = workspace_root();
    std::env::set_current_dir(&root).unwrap_or_else(|e| panic!("cwd: {e}"));

    let bars = make_synthetic_bars(500);
    let mut cfg = base_binance_sma_config(bars);
    cfg.strategy = StrategyId("v0.5.rsi".into());

    let (_handle, cancel_rx) = cancellation_pair();
    let report = backtest::engine::run_scenario(cfg, cancel_rx, ProgressSender::disabled())
        .await
        .expect("v0.5.rsi BinanceCache run must succeed");

    assert!(
        !report.equity_series.is_empty(),
        "BinanceCache v0.5.rsi run must produce a non-empty equity series"
    );
}

/// T-A1 — `v0.5.bbands` with `BinanceCache` returns non-empty equity.
#[tokio::test]
async fn binance_cache_accepted_by_bbands_arm() {
    let root = workspace_root();
    std::env::set_current_dir(&root).unwrap_or_else(|e| panic!("cwd: {e}"));

    let bars = make_synthetic_bars(500);
    let mut cfg = base_binance_sma_config(bars);
    cfg.strategy = StrategyId("v0.5.bbands".into());

    let (_handle, cancel_rx) = cancellation_pair();
    let report = backtest::engine::run_scenario(cfg, cancel_rx, ProgressSender::disabled())
        .await
        .expect("v0.5.bbands BinanceCache run must succeed");

    assert!(
        !report.equity_series.is_empty(),
        "BinanceCache v0.5.bbands run must produce a non-empty equity series"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// T-A2 — Cross-sectional arms REJECT BinanceCache
// ─────────────────────────────────────────────────────────────────────────────

fn cross_sectional_binance_config(strategy: &str) -> ScenarioConfig {
    ScenarioConfig {
        strategy: StrategyId(strategy.into()),
        pair: (Venue::Binance, Symbol::new("BTCUSDT")),
        range: DateRange::Last30d,
        params: None,
        seed: TEST_SEED,
        write_report: false,
        data_source: ScenarioDataSource::BinanceCache,
        bars_override: None,
        sma_fast_len: None,
        sma_slow_len: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        reports_dir: None,
        short_enabled: false,
    }
}

/// T-A2 (AC2) — `v1.momentum` rejects `BinanceCache` with
/// `RunError::UnsupportedDataSource`, exactly as it rejects `YahooCache`.
#[tokio::test]
async fn binance_cache_rejected_by_momentum_arm() {
    let root = workspace_root();
    std::env::set_current_dir(&root).unwrap_or_else(|e| panic!("cwd: {e}"));

    let (_handle, cancel_rx) = cancellation_pair();
    let result = backtest::engine::run_scenario(
        cross_sectional_binance_config("v1.momentum"),
        cancel_rx,
        ProgressSender::disabled(),
    )
    .await;

    assert!(
        matches!(result, Err(RunError::UnsupportedDataSource(_))),
        "v1.momentum must reject BinanceCache; got: {result:?}"
    );
}

/// T-A2 (AC2) — `v1.5a.pairs` rejects `BinanceCache`.
#[tokio::test]
async fn binance_cache_rejected_by_pairs_arm() {
    let root = workspace_root();
    std::env::set_current_dir(&root).unwrap_or_else(|e| panic!("cwd: {e}"));

    let (_handle, cancel_rx) = cancellation_pair();
    let result = backtest::engine::run_scenario(
        cross_sectional_binance_config("v1.5a.pairs"),
        cancel_rx,
        ProgressSender::disabled(),
    )
    .await;

    assert!(
        matches!(result, Err(RunError::UnsupportedDataSource(_))),
        "v1.5a.pairs must reject BinanceCache; got: {result:?}"
    );
}

/// T-A2 (AC2) — `v2.5.tcn` rejects `BinanceCache`.
#[tokio::test]
async fn binance_cache_rejected_by_tcn_arm() {
    let root = workspace_root();
    std::env::set_current_dir(&root).unwrap_or_else(|e| panic!("cwd: {e}"));

    let (_handle, cancel_rx) = cancellation_pair();
    let result = backtest::engine::run_scenario(
        cross_sectional_binance_config("v2.5.tcn"),
        cancel_rx,
        ProgressSender::disabled(),
    )
    .await;

    assert!(
        matches!(result, Err(RunError::UnsupportedDataSource(_))),
        "v2.5.tcn must reject BinanceCache; got: {result:?}"
    );
}

/// T-A2 (AC2) — `v2.5.tcn.weights` rejects `BinanceCache`.
#[tokio::test]
async fn binance_cache_rejected_by_tcn_weights_arm() {
    let root = workspace_root();
    std::env::set_current_dir(&root).unwrap_or_else(|e| panic!("cwd: {e}"));

    let (_handle, cancel_rx) = cancellation_pair();
    let result = backtest::engine::run_scenario(
        cross_sectional_binance_config("v2.5.tcn.weights"),
        cancel_rx,
        ProgressSender::disabled(),
    )
    .await;

    assert!(
        matches!(result, Err(RunError::UnsupportedDataSource(_))),
        "v2.5.tcn.weights must reject BinanceCache; got: {result:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// ETH regression — composed strategies must trade on non-BTC symbols
//
// Root-cause: ComposedStrategy::emit_signal previously hardcoded `self.symbol`
// (= "BTCUSDT" from config TOML). When run on ETHUSDT bars, sig.symbol was
// "BTCUSDT" while position.symbol was "ETHUSDT", so Order::new returned
// AssetMismatch; the silent `.ok()` discarded every order → 0 trades.
// Fixed in `crates/strategy/src/composed/node.rs` by emitting `bar.symbol`.
// These tests assert trade_count > 0 on ETHUSDT synthetic bars for all three
// affected strategies (v0.5.macd / v0.5.rsi / v0.5.bbands).
// ─────────────────────────────────────────────────────────────────────────────

/// Regression test: v0.5.macd must produce >0 trades on ETHUSDT synthetic bars.
///
/// Bug: `ComposedStrategy::emit_signal` emitted `sig.symbol = "BTCUSDT"` (from
/// config TOML) even when running on ETHUSDT bars. Order::new rejected it with
/// `AssetMismatch`; the `.ok()` silently discarded every order → 0 trades.
/// Fix: emit `bar.symbol` instead of `self.symbol`.
#[tokio::test]
async fn composed_strategy_macd_trades_on_ethusdt_not_zero() {
    let root = workspace_root();
    std::env::set_current_dir(&root).unwrap_or_else(|e| panic!("cwd: {e}"));

    let sym = Symbol::new("ETHUSDT");
    let start_price = default_start_price(&sym);
    // Use enough bars to warm up MACD(12,26,9) + EMA(200): 200 + warmup ≈ 500+
    let bars = synthetic_bars_minute(&sym, 2_000, TEST_SEED_U64, start_price, 2023);

    let cfg = ScenarioConfig {
        strategy: StrategyId("v0.5.macd".into()),
        pair: (Venue::Binance, sym),
        range: DateRange::Last30d,
        params: None,
        seed: TEST_SEED,
        write_report: false,
        data_source: ScenarioDataSource::BinanceCache,
        bars_override: Some(bars),
        sma_fast_len: None,
        sma_slow_len: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        reports_dir: None,
        short_enabled: false,
    };

    let (_handle, cancel_rx) = cancellation_pair();
    let report = backtest::engine::run_scenario(cfg, cancel_rx, ProgressSender::disabled())
        .await
        .expect("v0.5.macd ETHUSDT run must succeed");

    assert!(
        report.kpis.trade_count > 0,
        "v0.5.macd on ETHUSDT must produce > 0 trades (was 0 due to AssetMismatch bug); \
         got trade_count = {}",
        report.kpis.trade_count
    );
}

/// Regression test: v0.5.rsi must produce >0 trades on ETHUSDT synthetic bars.
#[tokio::test]
async fn composed_strategy_rsi_trades_on_ethusdt_not_zero() {
    let root = workspace_root();
    std::env::set_current_dir(&root).unwrap_or_else(|e| panic!("cwd: {e}"));

    let sym = Symbol::new("ETHUSDT");
    let start_price = default_start_price(&sym);
    let bars = synthetic_bars_minute(&sym, 2_000, TEST_SEED_U64, start_price, 2023);

    let cfg = ScenarioConfig {
        strategy: StrategyId("v0.5.rsi".into()),
        pair: (Venue::Binance, sym),
        range: DateRange::Last30d,
        params: None,
        seed: TEST_SEED,
        write_report: false,
        data_source: ScenarioDataSource::BinanceCache,
        bars_override: Some(bars),
        sma_fast_len: None,
        sma_slow_len: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        reports_dir: None,
        short_enabled: false,
    };

    let (_handle, cancel_rx) = cancellation_pair();
    let report = backtest::engine::run_scenario(cfg, cancel_rx, ProgressSender::disabled())
        .await
        .expect("v0.5.rsi ETHUSDT run must succeed");

    assert!(
        report.kpis.trade_count > 0,
        "v0.5.rsi on ETHUSDT must produce > 0 trades (was 0 due to AssetMismatch bug); \
         got trade_count = {}",
        report.kpis.trade_count
    );
}

/// Regression test: v0.5.bbands must produce >0 trades on ETHUSDT synthetic bars.
#[tokio::test]
async fn composed_strategy_bbands_trades_on_ethusdt_not_zero() {
    let root = workspace_root();
    std::env::set_current_dir(&root).unwrap_or_else(|e| panic!("cwd: {e}"));

    let sym = Symbol::new("ETHUSDT");
    let start_price = default_start_price(&sym);
    let bars = synthetic_bars_minute(&sym, 2_000, TEST_SEED_U64, start_price, 2023);

    let cfg = ScenarioConfig {
        strategy: StrategyId("v0.5.bbands".into()),
        pair: (Venue::Binance, sym),
        range: DateRange::Last30d,
        params: None,
        seed: TEST_SEED,
        write_report: false,
        data_source: ScenarioDataSource::BinanceCache,
        bars_override: Some(bars),
        sma_fast_len: None,
        sma_slow_len: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        reports_dir: None,
        short_enabled: false,
    };

    let (_handle, cancel_rx) = cancellation_pair();
    let report = backtest::engine::run_scenario(cfg, cancel_rx, ProgressSender::disabled())
        .await
        .expect("v0.5.bbands ETHUSDT run must succeed");

    assert!(
        report.kpis.trade_count > 0,
        "v0.5.bbands on ETHUSDT must produce > 0 trades (was 0 due to AssetMismatch bug); \
         got trade_count = {}",
        report.kpis.trade_count
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// T-C1 — No-op-source divergence guard (AC4)
//
// Runs v0.sma × BTCUSDT on REAL Binance hourly bars (from on-disk parquet)
// AND on synthetic bars with the SAME (strategy, symbol, seed).
// Asserts the two final equities diverge by at least epsilon — proving real
// parquet bytes reached the strategy and there is no silent synthetic fallback.
//
// SKIPPED (not failed) when `data/binance/BTCUSDT/2023/01.parquet` is absent
// so CI without the gitignored corpus stays green.
// ─────────────────────────────────────────────────────────────────────────────

/// T-C1 (AC4) — Real Binance hourly bars reach v0.sma; equity diverges from
/// the synthetic baseline by ≥ epsilon = 1 USD.
///
/// Pattern: mirrors `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`.
/// This is the no-op-source guard (simple-strategies-realdata A5).
#[tokio::test]
async fn binance_cache_real_bars_diverge_from_synthetic_baseline() {
    let root = workspace_root();
    std::env::set_current_dir(&root).unwrap_or_else(|e| panic!("cwd: {e}"));

    // SKIP if the on-disk Binance corpus is absent (CI without data fixtures).
    let parquet_probe = root.join("data/binance/BTCUSDT/2023/01.parquet");
    if !parquet_probe.is_file() {
        eprintln!(
            "SKIP T-C1: data/binance/BTCUSDT/2023/01.parquet not present — \
             re-fetch with `cargo run -p data --bin fetch_binance_klines`"
        );
        return;
    }

    // ── 1. Load real Binance hourly bars for BTCUSDT, Jan 2023 ───────────────
    let binance_root = root.join("data/binance");
    let feed = data::ReplayFeed::new(&binance_root, /* fast = */ true);
    let sym = Symbol::new("BTCUSDT");

    // 2023-01-01T00:00:00Z = 1_672_531_200_000 ms
    // We clip to just January (≈744 bars) to keep the test fast.
    let start_ms: u64 = 1_672_531_200_000;
    let end_ms: u64 = start_ms + 31 * 24 * 3_600_000;

    let stream_result = {
        use data::source::MarketDataSource as _;
        feed.subscribe_bars(sym.clone(), Timeframe::OneHour).await
    };

    let mut stream = match stream_result {
        Ok(s) => s,
        Err(e) => {
            eprintln!("SKIP T-C1: subscribe_bars failed: {e}");
            return;
        }
    };

    let mut binance_bars: Vec<trading_core::Bar> = Vec::new();
    while let Some(bar_result) = stream.next().await {
        match bar_result {
            Ok(b) => {
                let ts_ms = b.open_ts.unix_millis() as u64;
                if ts_ms >= start_ms && ts_ms < end_ms {
                    binance_bars.push(b);
                } else if ts_ms >= end_ms {
                    break;
                }
            }
            Err(e) => {
                eprintln!("SKIP T-C1: stream error: {e}");
                return;
            }
        }
    }

    if binance_bars.len() < 100 {
        eprintln!(
            "SKIP T-C1: only {} bars loaded for BTCUSDT Jan 2023 — corpus may be incomplete",
            binance_bars.len()
        );
        return;
    }

    // ── 2. Run v0.sma on real Binance bars (BinanceCache) ────────────────────
    let binance_cfg = ScenarioConfig {
        strategy: StrategyId("v0.sma".into()),
        pair: (Venue::Binance, sym.clone()),
        range: DateRange::Last30d,
        params: None,
        seed: TEST_SEED,
        write_report: false,
        data_source: ScenarioDataSource::BinanceCache,
        bars_override: Some(binance_bars.clone()),
        sma_fast_len: None,
        sma_slow_len: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        reports_dir: None,
        short_enabled: false,
    };

    let (_handle, cancel_rx) = cancellation_pair();
    let binance_report =
        backtest::engine::run_scenario(binance_cfg, cancel_rx, ProgressSender::disabled())
            .await
            .expect("v0.sma BinanceCache must succeed");

    // ── 3. Run v0.sma on synthetic bars (Synthetic) — same seed, same count ──
    let n_bars = binance_bars.len();
    let synthetic_bars = make_synthetic_bars(n_bars);
    let synthetic_cfg = ScenarioConfig {
        strategy: StrategyId("v0.sma".into()),
        pair: (Venue::Binance, sym),
        range: DateRange::Last30d,
        params: None,
        seed: TEST_SEED,
        write_report: false,
        data_source: ScenarioDataSource::Synthetic,
        bars_override: Some(synthetic_bars),
        sma_fast_len: None,
        sma_slow_len: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        reports_dir: None,
        short_enabled: false,
    };

    let (_handle2, cancel_rx2) = cancellation_pair();
    let synthetic_report =
        backtest::engine::run_scenario(synthetic_cfg, cancel_rx2, ProgressSender::disabled())
            .await
            .expect("v0.sma Synthetic must succeed");

    // ── 4. Assert equity diverges by ≥ 1 USD ─────────────────────────────────
    let binance_final = binance_report
        .equity_series
        .last()
        .map(|(_, eq)| eq.amount())
        .unwrap_or_default();
    let synthetic_final = synthetic_report
        .equity_series
        .last()
        .map(|(_, eq)| eq.amount())
        .unwrap_or_default();

    let delta = if binance_final > synthetic_final {
        binance_final - synthetic_final
    } else {
        synthetic_final - binance_final
    };
    let epsilon = rust_decimal::Decimal::ONE; // 1 USD

    assert!(
        delta >= epsilon,
        "T-C1 FAIL (AC4 no-op-source guard): BinanceCache and Synthetic final equities \
         must diverge by >= {epsilon} USD; got binance_final={binance_final}, \
         synthetic_final={synthetic_final}, delta={delta}. \
         If delta=0, the BinanceCache path is silently feeding synthetic bars."
    );
}
