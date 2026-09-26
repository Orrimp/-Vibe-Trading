//! T33 — Determinism test.
#![allow(clippy::unwrap_used)]
//!
//! Runs the `btc-2023-1m-sma-cross` scenario twice at seed `0xC0FFEE` and
//! asserts that:
//!   1. Both runs produce identical deterministic-content sha256 hashes of
//!      their report markdown.  The hash covers the report body only,
//!      **excluding** the YAML front-matter block (which contains the
//!      wall-clock `generated:` field).  See `backtest::report_body_hash` for
//!      the canonical convention.
//!   2. Both runs produce identical final equity and trade counts (ledger proxy).
//!
//! Why body-only hashing?  The `generated:` timestamp in the front matter is
//! intentionally kept for operator readability but is non-deterministic by
//! construction.  Everything else in the report — scenario parameters, equity,
//! trade counts, fees, Sharpe, drawdown — is purely a function of the seed and
//! the scenario definition, and must be byte-identical.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use trading_core::{
    Bar, Money, Order, OrderKind, Position, Price, Quantity, RiskLimits, Side, Symbol, TimeInForce,
    Timeframe, Timestamp, Usdt, Venue,
};

const SEED: u64 = 0xC0_FFEE;

// ── Inline mini-backtest (same logic as binary, extracted to a function) ──────

struct RunResult {
    trades: usize,
    final_equity: Decimal,
    signal_count: usize,
    equity_curve_len: usize,
}

fn synthetic_bars_det(count: usize) -> Vec<Bar> {
    use rand::Rng;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    let mut rng = ChaCha20Rng::seed_from_u64(SEED);
    let mut bars = Vec::with_capacity(count);
    let mut close: f64 = 16_500.0;
    let epoch = time::OffsetDateTime::new_utc(
        time::Date::from_calendar_date(2023, time::Month::January, 1).unwrap(),
        time::Time::MIDNIGHT,
    );

    for i in 0..count {
        let u1: f64 = rng.random::<f64>().max(1e-10);
        let u2: f64 = rng.random::<f64>();
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        let next = (close * (1.0 + 0.001_10 * z + 0.000_001_9)).clamp(1_000.0, 500_000.0);

        let open_ts = Timestamp::new(epoch + time::Duration::minutes(i as i64));
        let close_ts = Timestamp::new(
            epoch + time::Duration::minutes(i as i64 + 1) - time::Duration::seconds(1),
        );

        let to_dec = |v: f64| Decimal::try_from(v.max(0.01)).unwrap_or(dec!(0.01));
        let mk_price =
            |v: f64| Price::new(to_dec(v)).unwrap_or_else(|_| Price::new(dec!(1)).unwrap());

        bars.push(Bar {
            symbol: Symbol::new("BTCUSDT"),
            tf: Timeframe::OneMinute,
            open: mk_price(close),
            high: mk_price(next.max(close)),
            low: mk_price(next.min(close)),
            close: mk_price(next),
            volume: Quantity::new(to_dec(rng.random::<f64>() * 50.0 + 1.0)).unwrap(),
            trade_count: rng.random_range(10_u32..500_u32),
            local_recv_ts: close_ts,
            open_ts,
            close_ts,
            venue: Venue::Binance,
        });
        close = next;
    }
    bars
}

/// Run a mini-backtest with 1000 bars (fast proxy for the full scenario).
#[tokio::test]
async fn t33_determinism_mini_backtest() {
    let result1 = run_mini().await;
    let result2 = run_mini().await;

    assert_eq!(
        result1.trades, result2.trades,
        "trade count must be identical"
    );
    assert_eq!(
        result1.final_equity, result2.final_equity,
        "final equity must be identical"
    );
    assert_eq!(
        result1.signal_count, result2.signal_count,
        "signal count must be identical"
    );
    assert_eq!(
        result1.equity_curve_len, result2.equity_curve_len,
        "equity curve length must be identical"
    );
}

async fn run_mini() -> RunResult {
    use backtest::MatchingEngine;

    let bars = synthetic_bars_det(1000);
    let registry = strategy::StrategyRegistry::new();
    registry.register(Box::new(strategy::SmaCrossover::new(20, 50)));

    let risk_limits = RiskLimits {
        per_symbol_exposure_cap: dec!(0.40),
        price_sanity_band: dec!(0.20),
        portfolio_exposure_cap: None,
    };
    let sizer = risk::FixedFractionSizer::new(dec!(0.10));
    let config = backtest::paper::MatchConfig {
        slippage_bps: 2,
        taker_fee_bps: 4,
        maker_fee_bps: 2,
        fill_price_mode: backtest::paper::FillPriceMode::BarClose,
    };
    let mut engine = backtest::PaperEngine::new(config, SEED);

    let initial = dec!(100_000);
    let mut cash = initial;
    let mut position_qty = Decimal::ZERO;
    let mut position_cost = Decimal::ZERO;
    let mut trades = 0usize;
    let mut signal_count = 0usize;
    let mut equity_curve = vec![initial];
    let mut position = Position::empty(Symbol::new("BTCUSDT"));

    for bar in &bars {
        let mark = bar.close.get();
        position.last_mark = bar.close;
        let equity = cash + position_qty * mark;
        equity_curve.push(equity);

        let signals = registry.on_bar(bar);
        signal_count += signals.len();

        for sig in &signals {
            let side: Option<Side> = match sig.kind {
                trading_core::SignalKind::Buy if position_qty <= Decimal::ZERO => Some(Side::Buy),
                trading_core::SignalKind::Sell if position_qty > Decimal::ZERO => Some(Side::Sell),
                _ => None,
            };
            if let Some(s) = side {
                let ord = match s {
                    Side::Buy => {
                        let eq: Money<Usdt> = Money::from_decimal(equity);
                        risk::size_and_validate(
                            &sizer,
                            sig.strategy_id.clone(),
                            sig.symbol.clone(),
                            s,
                            eq,
                            bar.close,
                            &position,
                            &risk_limits,
                        )
                        .ok()
                    }
                    Side::Sell => Quantity::new(position_qty)
                        .ok()
                        .filter(|q| q.get() > Decimal::ZERO)
                        .and_then(|q| {
                            Order::new(
                                sig.strategy_id.clone(),
                                sig.symbol.clone(),
                                Side::Sell,
                                q,
                                OrderKind::Market,
                                TimeInForce::Ioc,
                                &position,
                                bar.close,
                                &risk_limits,
                                equity,
                            )
                            .ok()
                        }),
                };
                if let Some(order) = ord
                    && let Ok(fills) = engine.step(bar, vec![order]).await
                {
                    for fill in fills {
                        trades += 1;
                        match fill.side {
                            Side::Buy => {
                                let notional = fill.qty.get() * fill.price.get();
                                cash -= notional + fill.fee.amount();
                                position_qty += fill.qty.get();
                                position_cost += notional;
                                position.base_qty = position_qty;
                                position.cost_basis = Money::from_decimal(position_cost);
                            }
                            Side::Sell => {
                                let notional = fill.qty.get() * fill.price.get();
                                cash += notional - fill.fee.amount();
                                position_qty -= fill.qty.get();
                                if position_qty < Decimal::ZERO {
                                    position_qty = Decimal::ZERO;
                                    position_cost = Decimal::ZERO;
                                }
                                position.base_qty = position_qty;
                            }
                        }
                    }
                }
            }
        }
    }

    RunResult {
        trades,
        final_equity: cash + position_qty * position.last_mark.get(),
        signal_count,
        equity_curve_len: equity_curve.len(),
    }
}

// ── T33 report sha256 — real binary-level determinism ─────────────────────────

/// Verify that the backtest binary produces byte-identical report bodies across
/// two runs at the same seed.
///
/// Strategy: spawn the `backtest` binary twice via `std::process::Command`,
/// capture the report file it writes, read the file content, compute the
/// deterministic-content hash (body only, excluding the `generated:` line in
/// the YAML front matter), and assert the two hashes are equal.
///
/// This test is marked `#[ignore]` only when the binary cannot be located (CI
/// environments that have not run `cargo build`).  In the default `cargo test
/// --workspace` run the binary is always built first, so the test runs.
#[test]
fn t33_report_sha256_deterministic() {
    // Locate the backtest binary.  `cargo test` is invoked from the workspace
    // root, so we look in the standard `target/debug` path.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // Walk up from the crate manifest to the workspace root (two levels).
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("could not locate workspace root");

    let bin_path = workspace_root.join("target/debug/backtest");
    if !bin_path.exists() {
        // Binary not built yet — build it first.
        let status = std::process::Command::new("cargo")
            .args(["build", "--bin", "backtest"])
            .current_dir(workspace_root)
            .status()
            .expect("cargo build failed");
        assert!(status.success(), "cargo build --bin backtest failed");
    }

    // Use a temp directory for report output so we don't pollute evidence/reports.
    let tmp = tempfile::tempdir().expect("create tempdir");
    let reports_dir = tmp.path().join("evidence/reports");
    std::fs::create_dir_all(&reports_dir).expect("create temp reports dir");

    let run_backtest = || -> String {
        let output = std::process::Command::new(&bin_path)
            .args(["--scenario", "btc-2023-1m-sma-cross", "--seed", "0xC0FFEE"])
            .current_dir(tmp.path())
            .output()
            .expect("spawn backtest binary");
        assert!(
            output.status.success(),
            "backtest binary exited non-zero: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // The binary prints "Report written: evidence/reports/backtest-<stamp>-<scenario>.md"
        let stdout = String::from_utf8_lossy(&output.stdout);
        let report_rel = stdout
            .lines()
            .find(|l| l.starts_with("Report written: "))
            .map(|l| l.trim_start_matches("Report written: ").trim())
            .expect("could not find 'Report written:' line in binary output");

        let report_path = tmp.path().join(report_rel);
        std::fs::read_to_string(&report_path)
            .unwrap_or_else(|e| panic!("could not read report {report_path:?}: {e}"))
    };

    let report1 = run_backtest();
    let report2 = run_backtest();

    // Hash only the report body (everything after the YAML front matter).
    // The front matter contains the `generated:` wall-clock timestamp which is
    // legitimately different between runs; everything else must be identical.
    let hash1 = backtest::report_body_hash(&report1);
    let hash2 = backtest::report_body_hash(&report2);

    let hex1 = hash1.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let hex2 = hash2.iter().map(|b| format!("{b:02x}")).collect::<String>();

    assert_eq!(
        hex1,
        hex2,
        "deterministic-content SHA-256 must be identical across two runs at the same seed.\n\
         Report body 1 (first 500 chars):\n{}\n\nReport body 2 (first 500 chars):\n{}",
        {
            let body = backtest::extract_report_body(&report1);
            &body[..500_usize.min(body.len())]
        },
        {
            let body = backtest::extract_report_body(&report2);
            &body[..500_usize.min(body.len())]
        },
    );
}

// ── T521 — extended determinism gate for v0.5 composed-strategy scenarios ────

/// Helper that runs a scenario twice and asserts byte-identical body hashes.
fn assert_scenario_deterministic(scenario: &str) {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("could not locate workspace root");

    let bin_path = workspace_root.join("target/debug/backtest");
    if !bin_path.exists() {
        let status = std::process::Command::new("cargo")
            .args(["build", "--bin", "backtest"])
            .current_dir(workspace_root)
            .status()
            .expect("cargo build failed");
        assert!(status.success(), "cargo build --bin backtest failed");
    }

    let tmp = tempfile::tempdir().expect("create tempdir");
    let reports_dir = tmp.path().join("evidence/reports");
    std::fs::create_dir_all(&reports_dir).expect("create temp reports dir");

    // Also need config/strategies/ accessible for composed scenarios.
    let config_dir = tmp.path().join("config/strategies");
    std::fs::create_dir_all(&config_dir).expect("create temp config/strategies");

    // Copy the canonical TOML recipes into the temp dir.
    let src_strategies = workspace_root.join("config/strategies");
    for entry in std::fs::read_dir(&src_strategies)
        .expect("read config/strategies")
        .flatten()
    {
        let dst = config_dir.join(entry.file_name());
        std::fs::copy(entry.path(), dst).expect("copy strategy TOML");
    }

    let run = || -> String {
        let output = std::process::Command::new(&bin_path)
            .args(["--scenario", scenario, "--seed", "0xC0FFEE"])
            .current_dir(tmp.path())
            .output()
            .expect("spawn backtest binary");
        assert!(
            output.status.success(),
            "backtest binary exited non-zero for scenario {scenario}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        let report_rel = stdout
            .lines()
            .find(|l| l.starts_with("Report written: "))
            .map(|l| l.trim_start_matches("Report written: ").trim())
            .expect("could not find 'Report written:' line");
        let report_path = tmp.path().join(report_rel);
        std::fs::read_to_string(&report_path)
            .unwrap_or_else(|e| panic!("could not read report {report_path:?}: {e}"))
    };

    let report1 = run();
    let report2 = run();

    let hash1 = backtest::report_body_hash(&report1);
    let hash2 = backtest::report_body_hash(&report2);

    let hex1 = hash1.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let hex2 = hash2.iter().map(|b| format!("{b:02x}")).collect::<String>();

    assert_eq!(
        hex1, hex2,
        "T521: scenario {scenario} body-SHA256 must be identical across two runs at seed 0xC0FFEE"
    );
}

/// T521 — sma baseline refresh is deterministic.
#[test]
fn t521_sma_baseline_refresh_deterministic() {
    assert_scenario_deterministic("btc-2023-1m-sma-baseline-refresh");
}

/// T521 — btc-2023-1m-macd-trend is deterministic.
#[test]
fn t521_macd_trend_deterministic() {
    assert_scenario_deterministic("btc-2023-1m-macd-trend");
}

/// T521 — btc-2023-1m-rsi-reversion is deterministic.
#[test]
fn t521_rsi_reversion_deterministic() {
    assert_scenario_deterministic("btc-2023-1m-rsi-reversion");
}

/// T521 — btc-2023-1m-bbands-mean-revert is deterministic.
#[test]
fn t521_bbands_mean_revert_deterministic() {
    assert_scenario_deterministic("btc-2023-1m-bbands-mean-revert");
}

// ── T622 — v0 + v0.5 anchor hash regression gate ──────────────────────────────
//
// These tests run each v0/v0.5 scenario once at seed 0xC0FFEE and compare the
// body-SHA256 against the locked anchor hashes.  If any anchor hash changes,
// the v1 changes have introduced a regression in the v0/v0.5 output.
//
// Anchor hashes re-locked 2026-05-30 to `v5-realdata-medium-2026-05` namespace
// per ADR-0045 § D6 (Decision 1). SMA rows use full-hash assert_eq.
// MACD/RSI/BBands rows (3-5) are PENDING orchestrator review — see block comment
// above the t717 section for details on the data-source mapping mismatch.
//   btc-2023-1m-sma-cross         d2fa7616c5ba763784f70eb6de5072866fe66f41bcb055f62f187e80703990e0
//   btc-2023-1m-sma-baseline-refresh  (same body as sma-cross)
//   btc-2023-1m-macd-trend        PENDING (tempdir synthetic SHA ≠ real-data anchor in anchors.toml)
//   btc-2023-1m-rsi-reversion     PENDING
//   btc-2023-1m-bbands-mean-revert PENDING

fn run_scenario_once(scenario: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("could not locate workspace root");

    let bin_path = workspace_root.join("target/debug/backtest");
    if !bin_path.exists() {
        let status = std::process::Command::new("cargo")
            .args(["build", "--bin", "backtest"])
            .current_dir(workspace_root)
            .status()
            .expect("cargo build failed");
        assert!(status.success(), "cargo build --bin backtest failed");
    }

    let tmp = tempfile::tempdir().expect("create tempdir");
    let reports_dir = tmp.path().join("evidence/reports");
    std::fs::create_dir_all(&reports_dir).expect("create temp reports dir");

    let config_dir = tmp.path().join("config/strategies");
    std::fs::create_dir_all(&config_dir).expect("create temp config/strategies");
    let src_strategies = workspace_root.join("config/strategies");
    for entry in std::fs::read_dir(&src_strategies)
        .expect("read config/strategies")
        .flatten()
    {
        let dst = config_dir.join(entry.file_name());
        std::fs::copy(entry.path(), dst).expect("copy strategy TOML");
    }

    let output = std::process::Command::new(&bin_path)
        .args(["--scenario", scenario, "--seed", "0xC0FFEE"])
        .current_dir(tmp.path())
        .output()
        .expect("spawn backtest binary");

    assert!(
        output.status.success(),
        "backtest binary exited non-zero for scenario {scenario}: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report_rel = stdout
        .lines()
        .find(|l| l.starts_with("Report written: "))
        .map(|l| l.trim_start_matches("Report written: ").trim())
        .expect("could not find 'Report written:' line");

    let report_path = tmp.path().join(report_rel);
    std::fs::read_to_string(&report_path)
        .unwrap_or_else(|e| panic!("could not read report {report_path:?}: {e}"))
}

// NOTE (2026-08-22): a `binance_corpus_present()` skip-guard was added here and
// then REMOVED in the same session. It rested on a false premise — that CI lacks
// the pinned corpus. `data/binance` is **tracked**, so a runner HAS it; the
// gitignored (and therefore CI-absent) data dirs are `audit`, `audit.db`,
// `binance-dynamic` and `reflection`. The real reason these tests fail in CI is
// code-vs-evidence drift, handled by the known-red block below. Leaving a guard
// that documents a condition which never occurs would mislead the next reader.
fn scenario_body_hex(scenario: &str) -> String {
    let report = run_scenario_once(scenario);
    let hash = backtest::report_body_hash(&report);
    hash.iter().map(|b| format!("{b:02x}")).collect::<String>()
}

/// T622 — v0 SMA anchor hash unchanged after v1 changes.
///
/// Re-locked to `v5-realdata-medium-2026-05` namespace (ADR-0045 § D6).
/// Stale noop-baseline SHA `fc2e3b4a…` replaced with canonical 8-bps SHA.
#[test]
fn t622_sma_cross_anchor_hash_unchanged() {
    const ANCHOR: &str = "d2fa7616c5ba763784f70eb6de5072866fe66f41bcb055f62f187e80703990e0";
    let hex = scenario_body_hex("btc-2023-1m-sma-cross");
    assert_eq!(
        hex, ANCHOR,
        "T622 REGRESSION: btc-2023-1m-sma-cross body-SHA256 changed.\n\
         Expected: {ANCHOR}\n\
         Got:      {hex}"
    );
}

/// T622 — v0.5 SMA baseline-refresh anchor (same body as sma-cross).
///
/// Re-locked to `v5-realdata-medium-2026-05` namespace (ADR-0045 § D6).
#[test]
fn t622_sma_baseline_refresh_anchor_hash_unchanged() {
    const ANCHOR: &str = "d2fa7616c5ba763784f70eb6de5072866fe66f41bcb055f62f187e80703990e0";
    let hex = scenario_body_hex("btc-2023-1m-sma-baseline-refresh");
    assert_eq!(
        hex, ANCHOR,
        "T622 REGRESSION: btc-2023-1m-sma-baseline-refresh body-SHA256 changed.\n\
         Expected: {ANCHOR}\n\
         Got:      {hex}"
    );
}

/// T622 — v0.5 MACD trend anchor hash unchanged.
///
/// Re-locked to SYNTHETIC SHA (ADR-0045 § D6.3). The determinism test runs
/// the v0 synthetic-fallback path (CWD=tempdir → parquet lookup misses).
/// The v5-realdata-medium-2026-05 anchors.toml row for macd-trend was emitted
/// from the REAL-DATA path (17544-bar Binance bodies); the synthetic SHA is
/// held in D7.1's SYNTHETIC_DETERMINISM_SHAS dict (check_determinism_anchors.py).
#[test]
fn t622_macd_trend_anchor_hash_unchanged() {
    const ANCHOR: &str = "4d8192af7238f5e6ab4b8c95462c402210ae846a97f2484db1c600fb6e5e9d2a";
    let hex = scenario_body_hex("btc-2023-1m-macd-trend");
    assert_eq!(
        hex, ANCHOR,
        "T622 REGRESSION: btc-2023-1m-macd-trend body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

/// T622 — v0.5 RSI reversion anchor hash unchanged.
///
/// Re-locked to SYNTHETIC SHA (ADR-0045 § D6.3). The determinism test runs
/// the v0 synthetic-fallback path (CWD=tempdir → parquet lookup misses).
/// The v5-realdata-medium-2026-05 anchors.toml row was emitted from the
/// REAL-DATA path (17544-bar Binance bodies). Synthetic SHA held in
/// SYNTHETIC_DETERMINISM_SHAS (check_determinism_anchors.py).
#[test]
fn t622_rsi_reversion_anchor_hash_unchanged() {
    const ANCHOR: &str = "4a7447885164b0b2f762402d8a580e7a546543b95ed8d6f8a52feff2ce1d8ab7";
    let hex = scenario_body_hex("btc-2023-1m-rsi-reversion");
    assert_eq!(
        hex, ANCHOR,
        "T622 REGRESSION: btc-2023-1m-rsi-reversion body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

/// T622 — v0.5 BBands mean-revert anchor hash unchanged.
///
/// Re-locked to SYNTHETIC SHA (ADR-0045 § D6.3). The determinism test runs
/// the v0 synthetic-fallback path (CWD=tempdir → parquet lookup misses).
/// The v5-realdata-medium-2026-05 anchors.toml row was emitted from the
/// REAL-DATA path (17544-bar Binance bodies). Synthetic SHA held in
/// SYNTHETIC_DETERMINISM_SHAS (check_determinism_anchors.py).
#[test]
fn t622_bbands_mean_revert_anchor_hash_unchanged() {
    const ANCHOR: &str = "5037accb3118d3aafe654c58b60878e75d884bc1ce6dbaf82748c2379c80a894";
    let hex = scenario_body_hex("btc-2023-1m-bbands-mean-revert");
    assert_eq!(
        hex, ANCHOR,
        "T622 REGRESSION: btc-2023-1m-bbands-mean-revert body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

// ── T717 — v0 + v0.5 + v1 full-hash regression gate ──────────────────────────
//
// These tests extend T622 with the complete 64-char anchor hashes for all 7
// v0/v0.5/v1 scenarios.  The v1.5a backend changes must not affect any of
// these anchors (architecture determinism contract R9.4).
//
// Anchor hashes re-locked 2026-05-30 to canonical namespace per ADR-0045 § D6.
// Original noop-baseline SHAs are preserved in evidence/anchors.toml.
// SMA/Momentum/tt1 re-locked to v5-realdata-medium-2026-05 (synthetic == real-data SHA).
// MACD/RSI/BBands re-locked to SYNTHETIC SHAs (ADR-0045 § D6.3): the v5-realdata-medium
// rows for these 3 were emitted from the REAL-DATA path (17544-bar Binance bodies);
// the determinism tests run the v0 synthetic fallback (CWD=tempdir, 525600 bars).
//   btc-2023-1m-sma-cross             d2fa7616c5ba763784f70eb6de5072866fe66f41bcb055f62f187e80703990e0
//   btc-2023-1m-sma-baseline-refresh  d2fa7616c5ba763784f70eb6de5072866fe66f41bcb055f62f187e80703990e0
//   btc-2023-1m-macd-trend            4d8192af7238f5e6ab4b8c95462c402210ae846a97f2484db1c600fb6e5e9d2a  (SYNTHETIC)
//   btc-2023-1m-rsi-reversion         4a7447885164b0b2f762402d8a580e7a546543b95ed8d6f8a52feff2ce1d8ab7  (SYNTHETIC)
//   btc-2023-1m-bbands-mean-revert    5037accb3118d3aafe654c58b60878e75d884bc1ce6dbaf82748c2379c80a894  (SYNTHETIC)
//   top10-2023-1h-momentum            0f6f6eb8d943fefa866c4883be034f1beb3caff169fe76ec73bf3c29041a8ba3
//   top10-2024-h1-momentum            78976062cf3d62b9bbb2ab579e91822cb49f0d12464dedf912edb427e66c7490
//
// NOTE: T715 (pairs backtest) introduced a data_source regression where the
// momentum scenarios were emitting "synthetic (seeded RNG, v1.5a multi-symbol)"
// instead of the v1-locked "synthetic (seeded RNG, v1 multi-symbol)".  Fixed in
// the T717 hotfix: momentum data_source restored; pairs keep the v1.5a label.

/// T717 — SMA cross anchor unchanged after v1.5a backend changes.
///
/// Re-locked to `v5-realdata-medium-2026-05` namespace (ADR-0045 § D6).
/// Stale noop-baseline SHA `fc2e3b4a…` replaced with canonical 8-bps SHA.
#[test]
fn t717_sma_cross_anchor_hash_unchanged() {
    const ANCHOR: &str = "d2fa7616c5ba763784f70eb6de5072866fe66f41bcb055f62f187e80703990e0";
    let hex = scenario_body_hex("btc-2023-1m-sma-cross");
    assert_eq!(
        hex, ANCHOR,
        "T717 REGRESSION: btc-2023-1m-sma-cross body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

/// T717 — SMA baseline-refresh anchor unchanged after v1.5a backend changes.
///
/// Re-locked to `v5-realdata-medium-2026-05` namespace (ADR-0045 § D6).
#[test]
fn t717_sma_baseline_refresh_anchor_hash_unchanged() {
    const ANCHOR: &str = "d2fa7616c5ba763784f70eb6de5072866fe66f41bcb055f62f187e80703990e0";
    let hex = scenario_body_hex("btc-2023-1m-sma-baseline-refresh");
    assert_eq!(
        hex, ANCHOR,
        "T717 REGRESSION: btc-2023-1m-sma-baseline-refresh body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

/// T717 — MACD trend full anchor hash unchanged.
///
/// Re-locked to SYNTHETIC SHA (ADR-0045 § D6.3). The determinism test runs
/// the v0 synthetic-fallback path (CWD=tempdir → parquet lookup misses).
/// The v5-realdata-medium-2026-05 anchors.toml row was emitted from the
/// REAL-DATA path (17544-bar Binance bodies). Synthetic SHA held in
/// SYNTHETIC_DETERMINISM_SHAS (check_determinism_anchors.py, D7.1).
#[test]
fn t717_macd_trend_anchor_hash_unchanged() {
    const ANCHOR: &str = "4d8192af7238f5e6ab4b8c95462c402210ae846a97f2484db1c600fb6e5e9d2a";
    let hex = scenario_body_hex("btc-2023-1m-macd-trend");
    assert_eq!(
        hex, ANCHOR,
        "T717 REGRESSION: btc-2023-1m-macd-trend body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

/// T717 — RSI reversion full anchor hash unchanged.
///
/// Re-locked to SYNTHETIC SHA (ADR-0045 § D6.3). The determinism test runs
/// the v0 synthetic-fallback path (CWD=tempdir → parquet lookup misses).
/// The v5-realdata-medium-2026-05 anchors.toml row was emitted from the
/// REAL-DATA path (17544-bar Binance bodies). Synthetic SHA held in
/// SYNTHETIC_DETERMINISM_SHAS (check_determinism_anchors.py, D7.1).
#[test]
fn t717_rsi_reversion_anchor_hash_unchanged() {
    const ANCHOR: &str = "4a7447885164b0b2f762402d8a580e7a546543b95ed8d6f8a52feff2ce1d8ab7";
    let hex = scenario_body_hex("btc-2023-1m-rsi-reversion");
    assert_eq!(
        hex, ANCHOR,
        "T717 REGRESSION: btc-2023-1m-rsi-reversion body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

/// T717 — BBands mean-revert full anchor hash unchanged.
///
/// Re-locked to SYNTHETIC SHA (ADR-0045 § D6.3). The determinism test runs
/// the v0 synthetic-fallback path (CWD=tempdir → parquet lookup misses).
/// The v5-realdata-medium-2026-05 anchors.toml row was emitted from the
/// REAL-DATA path (17544-bar Binance bodies). Synthetic SHA held in
/// SYNTHETIC_DETERMINISM_SHAS (check_determinism_anchors.py, D7.1).
#[test]
fn t717_bbands_mean_revert_anchor_hash_unchanged() {
    const ANCHOR: &str = "5037accb3118d3aafe654c58b60878e75d884bc1ce6dbaf82748c2379c80a894";
    let hex = scenario_body_hex("btc-2023-1m-bbands-mean-revert");
    assert_eq!(
        hex, ANCHOR,
        "T717 REGRESSION: btc-2023-1m-bbands-mean-revert body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// KNOWN-RED PENDING THE 1-26 RE-LOCK — these four are TELLING THE TRUTH
// ═══════════════════════════════════════════════════════════════════════════
//
// `t717_*` and `tt1_*` below RE-RUN a scenario from real data and compare the
// body-SHA against a pin. They are the ONLY gate in the repo that can observe
// CODE-vs-EVIDENCE drift: `scripts/verify_anchors.sh` hashes the COMMITTED report
// bodies and never re-runs, so it reports 119/119 green while the code that
// produced those bodies has moved underneath them.
//
// They are currently RED, and they are RIGHT to be. Measured 2026-08-22:
//   expected 0f6f6eb8…  (pinned, and present in evidence/anchors.toml)
//   got      b655e5e7…  (what the current code produces)
//
// ── CAUSE FOUND 2026-09-25 (story 1-27). The paragraph that used to stand here
// was WRONG, and it is kept below so the correction is legible next to it:
//
//     "Bisect against 83378c5 shows they failed BEFORE #67/#71/#75/#76 landed, so
//      the drift predates this session's harness fixes — those fixes moved the
//      numbers further, they did not cause the divergence."
//
// Measured, one commit either side, on the default invocation this test uses:
//   b3332d35 (2026-08-15)  ->  0f6f6eb8…   GOOD, reproduces the pin exactly
//   11acd126 (2026-08-16)  ->  b655e5e7…   BAD, and it is the hash we see today
//
// `83378c59` itself was measured GOOD, so the old claim is falsified at its own
// cited commit. `11acd126` is **the #67 fix** — "the harness was booking a ~1%
// gain for buying one symbol at another symbol's price — engine guard +
// per-symbol fill routing".
//
// The reasoning error worth keeping: the old note excluded #67 because these four
// scenarios do not go through `run_path`. True, and irrelevant — #67's fix landed
// in `engine.rs` (+20) and `paper.rs` (+98), the engine BELOW every lane. Anything
// that calls `engine.step` with a multi-symbol universe is in scope, `run_path` or
// not. Single-symbol scenarios are immune by the defect's own mechanism, which is
// why the `t622_*` gates above stayed green throughout.
//
// So these four are NOT a separate drift. The pinned bodies are #67-contaminated
// evidence, and these gates are red because the engine became CORRECT. Resolution
// is a D6.b re-lock (story 1-27), not a code fix.
//
// DO NOT re-baseline these pins to the current output. That converts a truthful
// regression gate into a rubber stamp — bug-log #77's exact failure — and it would
// silently bless whatever caused the drift. The pins are re-derived by story
// **1-26** (the re-lock), which regenerates the 34 affected surfaces under a new
// namespace and records per-scenario old-vs-new numbers in an errata.
//
// RE-MEASURED 2026-08-23, after the ADR-0089 D1 sizer wiring landed, and again
// 2026-09-25 after the 1-26 re-lock: all four produce the SAME hashes every time —
// `t717_top10_2023_1h_momentum` is still `b655e5e7…` exactly.
//
// That stability was originally read as "a separate, older drift". It is not. It
// means the move had ALREADY happened, at 11acd126 on 2026-08-16, and nothing
// since has touched it. D1 (2026-08-23) genuinely changed nothing here — that part
// of the old note holds — but the CAUSE is shared with the 34 inventory surfaces:
// both are #67.
//
// What 1-26 correctly kept separate: the 2026-09-25 regeneration RUN moved the 34
// and not these four. What it got wrong, on the strength of the note above: that
// the two therefore have different causes. Same cause, different dates.
//
// `#[ignore]` is applied so CI can verify EVERYTHING ELSE while this remains open.
// They still run on demand:
//     cargo test -p backtest --test determinism -- --ignored
// Remove the attribute in the same commit as the 1-26 re-lock.

/// T717 — top10-2023-1h-momentum anchor hash unchanged.
///
/// Re-locked to `v5-realdata-medium-2026-05` namespace (ADR-0045 § D6).
/// Stale noop-baseline SHA `3b60ef07…` replaced with canonical 8-bps SHA.
#[test]
#[ignore = "known-red pending the 1-27 D6.b re-lock: the PIN is #67-contaminated evidence (cause bisected to 11acd126); do NOT re-baseline outside the protocol — see the block above"]
fn t717_top10_2023_momentum_anchor_hash_unchanged() {
    const ANCHOR: &str = "0f6f6eb8d943fefa866c4883be034f1beb3caff169fe76ec73bf3c29041a8ba3";
    let hex = scenario_body_hex("top10-2023-1h-momentum");
    assert_eq!(
        hex, ANCHOR,
        "T717 REGRESSION: top10-2023-1h-momentum body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

/// T717 — top10-2024-h1-momentum anchor hash unchanged.
///
/// Re-locked to `v5-realdata-medium-2026-05` namespace (ADR-0045 § D6).
/// Stale noop-baseline SHA `1f33534f…` replaced with canonical 8-bps SHA.
#[test]
#[ignore = "known-red pending the 1-27 D6.b re-lock: the PIN is #67-contaminated evidence (cause bisected to 11acd126); do NOT re-baseline outside the protocol — see the block above"]
fn t717_top10_2024_momentum_anchor_hash_unchanged() {
    const ANCHOR: &str = "78976062cf3d62b9bbb2ab579e91822cb49f0d12464dedf912edb427e66c7490";
    let hex = scenario_body_hex("top10-2024-h1-momentum");
    assert_eq!(
        hex, ANCHOR,
        "T717 REGRESSION: top10-2024-h1-momentum body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

// ── T-T-1 — v2.5 TCN overlay anchor regression gate ──────────────────────────
//
// Re-locked 2026-05-30 to `v5-realdata-medium-2026-05` namespace (ADR-0045 § D6).
// Original stale noop-baseline SHAs preserved in evidence/anchors.toml.
// These anchors capture the PassthroughForecaster path (candle feature
// absent in CI). The tt1_* scenarios are bare synthetic paths (no -realdata
// suffix) and take the Linear{bps:8} fallback in the default binary.
//
//   top10-2023-fy-tcn-overlay  1460fcc70029746b650ae6f1298a7f2291603e96c54531f26bf6f24c558250fc
//   top10-2024-fy-tcn-overlay  b8e9186bb36abe6539917245f7dec99685792dcc955e11ba52380a7a5293ad1e

/// T-T-1 — top10-2023-fy-tcn-overlay (2023 full-year top-10, passthrough mode) anchor hash.
///
/// Re-locked to `v5-realdata-medium-2026-05` namespace (ADR-0045 § D6).
/// Stale noop-baseline SHA `01d02584…` replaced with canonical 8-bps SHA.
#[test]
#[ignore = "known-red pending the 1-27 D6.b re-lock: the PIN is #67-contaminated evidence (cause bisected to 11acd126); do NOT re-baseline outside the protocol — see the block above"]
fn tt1_top10_2023_fy_tcn_overlay_anchor_hash_unchanged() {
    const ANCHOR: &str = "1460fcc70029746b650ae6f1298a7f2291603e96c54531f26bf6f24c558250fc";
    let hex = scenario_body_hex("top10-2023-fy-tcn-overlay");
    assert_eq!(
        hex, ANCHOR,
        "T-T-1 REGRESSION: top10-2023-fy-tcn-overlay body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

/// T-T-1 — top10-2024-fy-tcn-overlay (2024 full-year top-10, passthrough mode) anchor hash.
///
/// Re-locked to `v5-realdata-medium-2026-05` namespace (ADR-0045 § D6).
/// Stale noop-baseline SHA `e24c85ac…` replaced with canonical 8-bps SHA.
#[test]
#[ignore = "known-red pending the 1-27 D6.b re-lock: the PIN is #67-contaminated evidence (cause bisected to 11acd126); do NOT re-baseline outside the protocol — see the block above"]
fn tt1_top10_2024_fy_tcn_overlay_anchor_hash_unchanged() {
    const ANCHOR: &str = "b8e9186bb36abe6539917245f7dec99685792dcc955e11ba52380a7a5293ad1e";
    let hex = scenario_body_hex("top10-2024-fy-tcn-overlay");
    assert_eq!(
        hex, ANCHOR,
        "T-T-1 REGRESSION: top10-2024-fy-tcn-overlay body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

// ── M3 real-weights anchor hash tests (candle feature required) ───────────────
//
// These tests require `--features candle` because the -weights scenarios call
// `TcnSyncForecaster::load_bs1()` / `load_bs2()` at runtime.  They also
// require the LFS checkpoints to be present on disk.
//
// The anchor hashes were locked by developer on 2026-05-18 after two
// deterministic runs:
//   top10-2023-fy-tcn-overlay-weights  7cb1357c0d0d25cf89766d88f1342434788c4c373e6c3b1cb77d7f8cf05acef4
//   top10-2024-fy-tcn-overlay-weights  23c24dae0873df8e808897416d9d8fab75c4bd25dcd7b2933099ff061efe9f2b
//
// NOTE: The existing 20 anchor tests (above) must remain --features candle
// independent — this `#[cfg(feature = "candle")]` block is additive only.

/// Build the backtest binary with --features candle and run `scenario` once.
///
/// Runs from the workspace root so that relative paths
/// (`crates/forecast/checkpoints/anchors/`, `config/strategies/`) resolve
/// correctly.  The report is written to `evidence/<feature>/reports/` under the
/// workspace root (the same path the production binary uses).
///
/// Returns the full report text so the caller can body-hash it.
#[cfg(feature = "candle")]
fn run_scenario_once_candle(scenario: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("could not locate workspace root");

    // Always rebuild with candle feature so the binary has real-weights support.
    let status = std::process::Command::new("cargo")
        .args(["build", "--bin", "backtest", "--features", "candle"])
        .current_dir(workspace_root)
        .status()
        .expect("cargo build failed");
    assert!(
        status.success(),
        "cargo build --bin backtest --features candle failed"
    );

    let bin_path = workspace_root.join("target/debug/backtest");

    // Run from workspace root so checkpoint + config paths resolve correctly.
    //
    // bug-log #113 — the OUTPUT used to land in `evidence/<feature>/reports/`, inside
    // the anchored corpus, and `verify_anchors.sh` resolves each anchor to the NEWEST
    // matching report there. So running this test at a commit whose output differs
    // from the pin planted a drifted body in the corpus and flipped that gate to FAIL
    // as a side effect. `--reports-dir` sends it to a tempdir instead; only the output
    // location moves, the body is unchanged.
    let reports = tempfile::tempdir().expect("create reports tempdir");
    let output = std::process::Command::new(&bin_path)
        .args(["--scenario", scenario, "--seed", "0xC0FFEE"])
        .arg("--reports-dir")
        .arg(reports.path())
        .current_dir(workspace_root)
        .output()
        .expect("spawn backtest binary");

    assert!(
        output.status.success(),
        "backtest binary exited non-zero for scenario {scenario}: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report_rel = stdout
        .lines()
        .find(|l| l.starts_with("Report written: "))
        .map(|l| l.trim_start_matches("Report written: ").trim())
        .expect("could not find 'Report written:' line");

    // Report path is relative to workspace root.
    let report_path = workspace_root.join(report_rel);
    std::fs::read_to_string(&report_path)
        .unwrap_or_else(|e| panic!("could not read report {report_path:?}: {e}"))
}

#[cfg(feature = "candle")]
fn scenario_body_hex_candle(scenario: &str) -> String {
    let report = run_scenario_once_candle(scenario);
    let hash = backtest::report_body_hash(&report);
    hash.iter().map(|b| format!("{b:02x}")).collect::<String>()
}

/// M3 — top10-2023-fy-tcn-overlay-weights real-weights anchor hash.
///
/// Requires `--features candle` + LFS checkpoints present on disk.
/// Locked anchor: `7cb1357c0d0d25cf89766d88f1342434788c4c373e6c3b1cb77d7f8cf05acef4`
#[cfg(feature = "candle")]
#[test]
fn m3_top10_2023_fy_tcn_overlay_weights_anchor_hash_unchanged() {
    const ANCHOR: &str = "7cb1357c0d0d25cf89766d88f1342434788c4c373e6c3b1cb77d7f8cf05acef4";
    let hex = scenario_body_hex_candle("top10-2023-fy-tcn-overlay-weights");
    assert_eq!(
        hex, ANCHOR,
        "M3 REGRESSION: top10-2023-fy-tcn-overlay-weights body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}\n\
         This means the real-weights backtest output changed. Investigate \
         TcnSyncForecaster, TcnOverlayMomentumStrategy, or the report writer."
    );
}

/// M3 — top10-2024-fy-tcn-overlay-weights real-weights anchor hash.
///
/// Requires `--features candle` + LFS checkpoints present on disk.
/// Locked anchor: `23c24dae0873df8e808897416d9d8fab75c4bd25dcd7b2933099ff061efe9f2b`
#[cfg(feature = "candle")]
#[test]
fn m3_top10_2024_fy_tcn_overlay_weights_anchor_hash_unchanged() {
    const ANCHOR: &str = "23c24dae0873df8e808897416d9d8fab75c4bd25dcd7b2933099ff061efe9f2b";
    let hex = scenario_body_hex_candle("top10-2024-fy-tcn-overlay-weights");
    assert_eq!(
        hex, ANCHOR,
        "M3 REGRESSION: top10-2024-fy-tcn-overlay-weights body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}\n\
         This means the real-weights backtest output changed. Investigate \
         TcnSyncForecaster, TcnOverlayMomentumStrategy, or the report writer."
    );
}

// ── T-D-13 / T-D-14 — realdata determinism gate ───────────────────────────────
//
// Run each `-realdata` scenario twice with a synthetic 10-symbol parquet fixture
// (built under `<tmpdir>/data/binance/`) and assert byte-identical body hashes.
//
// Design: we place the fixture at `<tmpdir>/data/binance/` and run the binary
// from `<tmpdir>` so the hardcoded `data/binance` relative path resolves.
// Report output goes to `<tmpdir>/evidence/v1/backtest-real-binance-data/reports/`.
// `config/strategies/` is copied from the workspace so composed-strategy TOML
// lookup does not fail.
//
// These tests require `--features realdata` and are feature-gated accordingly.

/// Build the backtest binary with `--features realdata` and return the binary
/// path. Rebuilds if necessary but caches the binary between test runs.
#[cfg(feature = "realdata")]
/// Shared mutex over the `target/debug/backtest` filesystem path. Used by
/// `ensure_realdata_binary` AND `ensure_realdata_candle_binary` to serialise
/// the cargo-build sequence — without this, the two feature-variant builds
/// race for the same output path under parallel test execution and one
/// variant's binary leaks into the other variant's test invocation.
///
/// Each call additionally copies the freshly-built binary to a UNIQUE
/// per-call path under `target/debug/` so that two tests of the same variant
/// running in parallel cannot overwrite each other's binary mid-run.
static BACKTEST_BUILD_MU: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Monotonic counter so each `ensure_*_binary` call yields a unique
/// `target/debug/backtest-realdata-<n>` (or `-candle-<n>`) path even when
/// multiple tests of the same variant run in parallel.
#[cfg(feature = "realdata")]
static BACKTEST_COPY_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[cfg(feature = "realdata")]
fn copy_to_unique(
    src: &std::path::Path,
    target_dir: &std::path::Path,
    tag: &str,
) -> std::path::PathBuf {
    let n = BACKTEST_COPY_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let dst = target_dir.join(format!("backtest-{tag}-{n}"));
    std::fs::copy(src, &dst)
        .unwrap_or_else(|e| panic!("copy backtest -> backtest-{tag}-{n} failed: {e}"));
    dst
}

#[cfg(feature = "realdata")]
fn ensure_realdata_binary() -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("could not locate workspace root");

    let _guard = BACKTEST_BUILD_MU.lock().unwrap_or_else(|p| p.into_inner());

    // Always rebuild so we pick up any changes.
    let status = std::process::Command::new("cargo")
        .args(["build", "--bin", "backtest", "--features", "realdata"])
        .current_dir(workspace_root)
        .status()
        .expect("cargo build failed");
    assert!(
        status.success(),
        "cargo build --bin backtest --features realdata failed"
    );

    // Copy to a unique per-call path BEFORE releasing the mutex so a
    // concurrent rebuild (different feature set) cannot overwrite the
    // source between our build and our copy.
    copy_to_unique(
        &workspace_root.join("target/debug/backtest"),
        &workspace_root.join("target/debug"),
        "realdata",
    )
}

/// Run a realdata scenario once from the given `run_dir` working directory.
///
/// `run_dir` should be either the workspace root (when using real `data/binance/`)
/// or a synthetic tempdir (only valid when `expected_revision_sha` is `None`).
///
/// Returns the report body text.
#[cfg(feature = "realdata")]
fn run_realdata_scenario_once(
    bin: &std::path::Path,
    run_dir: &std::path::Path,
    scenario: &str,
) -> String {
    // bug-log #113 — this used to run with NO `--reports-dir`, so the report landed
    // in `evidence/<feature>/reports/` INSIDE the anchored corpus. `verify_anchors.sh`
    // resolves each anchor to the NEWEST matching report there, so running this test
    // at a commit whose output differs from the pin planted a drifted body in the
    // corpus and flipped that gate to FAIL as a side effect — a test able to break a
    // different gate by being run.
    //
    // CWD stays the workspace root: `data/binance/`, `config/strategies/` and the
    // forecast checkpoints are all resolved relative to it. Only the OUTPUT moves.
    let reports = tempfile::tempdir().expect("create reports tempdir");
    let output = std::process::Command::new(bin)
        .args(["--scenario", scenario, "--seed", "0xC0FFEE"])
        .arg("--reports-dir")
        .arg(reports.path())
        .current_dir(run_dir)
        .output()
        .expect("spawn backtest binary");

    assert!(
        output.status.success(),
        "backtest binary exited non-zero for scenario {scenario}:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report_rel = stdout
        .lines()
        .find(|l| l.starts_with("Report written: "))
        .map(|l| l.trim_start_matches("Report written: ").trim())
        .expect("could not find 'Report written:' line");

    // `--reports-dir` makes the printed path absolute; `join` on an absolute path
    // returns it unchanged, so this covers both shapes.
    let report_path = run_dir.join(report_rel);
    std::fs::read_to_string(&report_path)
        .unwrap_or_else(|e| panic!("could not read report {report_path:?}: {e}"))
}

/// Build the `backtest` binary in **release** and return its path — for the R-REPRO
/// gates only.
///
/// Why a separate builder (bug-log #121): the `*_determinism` tests above use the
/// debug binary, which is fine for them because they are cheap. The `-realdata`
/// reproduction gates are not: `top10-2023-fy-regime-dispatcher-realdata` needs
/// **270 s in release and over an hour in debug** — un-`#[ignore]`ing it on the debug
/// binary would make `cargo test -p backtest --features realdata` unusable.
///
/// The profile is safe to change here because it was **measured, not assumed**, not to
/// affect the hashed body: `top10-{2023,2024}-fy-tcn-overlay-realdata` and their
/// `-weights` siblings each produced byte-identical SHAs from a debug and a release
/// run on 2026-09-26 (4 scenarios, 8 runs). Independent corroboration: the anchored
/// report's own `wall_clock_s` is 3.2 s and the release run reports 3.2 s, while debug
/// takes ~19 s — so the anchor itself was locked in release.
///
/// Verified on 4 of 9 scenarios. If a future R-REPRO gate disagrees between profiles,
/// the profile IS part of its condition and belongs in its doc comment — that is
/// bug-log #112's dimension list growing again, not a reason to distrust these four.
#[cfg(feature = "realdata")]
fn ensure_realdata_release_binary(with_candle: bool) -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("could not locate workspace root");

    let _guard = BACKTEST_BUILD_MU.lock().unwrap_or_else(|p| p.into_inner());

    let features = if with_candle {
        "candle,realdata"
    } else {
        "realdata"
    };
    let status = std::process::Command::new("cargo")
        .args([
            "build",
            "--release",
            "-p",
            "backtest",
            "--bin",
            "backtest",
            "--features",
            features,
        ])
        .current_dir(workspace_root)
        .status()
        .expect("cargo build --release failed");
    assert!(
        status.success(),
        "cargo build --release --bin backtest --features {features} failed"
    );

    let bin = workspace_root.join("target/release/backtest");
    assert!(
        bin.is_file(),
        "release backtest binary missing at {}",
        bin.display()
    );
    bin
}

/// Fallible sibling of `run_realdata_scenario_once`: returns the report on success,
/// or the binary's own stderr on refusal.
///
/// bug-log #114's lesson, applied: do NOT hand-roll a precondition check when the
/// thing itself will tell you. That guard built a filename shape which had never
/// existed and so reported "absent" for four months on machines where the file was
/// present. The binary already refuses with a precise message naming the missing
/// feature or checkpoint — so ask it, and repeat what it says.
#[cfg(feature = "realdata")]
fn run_realdata_scenario_try(
    bin: &std::path::Path,
    run_dir: &std::path::Path,
    scenario: &str,
) -> Result<String, String> {
    let reports = tempfile::tempdir().expect("create reports tempdir");
    let output = std::process::Command::new(bin)
        .args(["--scenario", scenario, "--seed", "0xC0FFEE"])
        .arg("--reports-dir")
        .arg(reports.path())
        .current_dir(run_dir)
        .output()
        .expect("spawn backtest binary");

    if !output.status.success() {
        return Err(format!(
            "stdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout).trim(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let report_rel = stdout
        .lines()
        .find(|l| l.starts_with("Report written: "))
        .map(|l| l.trim_start_matches("Report written: ").trim())
        .ok_or_else(|| format!("no 'Report written:' line for {scenario}"))?;

    let report_path = run_dir.join(report_rel);
    std::fs::read_to_string(&report_path).map_err(|e| format!("read {report_path:?}: {e}"))
}

/// Return the workspace root (two directories up from `CARGO_MANIFEST_DIR`).
#[cfg(feature = "realdata")]
fn workspace_root_path() -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("could not locate workspace root")
        .to_path_buf()
}

/// Return `true` when the real `data/binance/REVISION.toml` is present in the
/// workspace root.  Used to gate the `-realdata` determinism tests: after
/// T-D-17 pins the real aggregate SHA into the four scenario arms, the binary
/// validates the revision at runtime.  A synthetic tempdir fixture has a
/// *different* SHA, so the binary exits non-zero if run from a tempdir.
/// Running from the workspace root uses the real data whose SHA matches the pin.
#[cfg(feature = "realdata")]
fn real_binance_data_available() -> bool {
    let revision_toml = workspace_root_path()
        .join("data")
        .join("binance")
        .join("REVISION.toml");
    revision_toml.exists()
}

/// T-D-13 — `top10-2023-fy-tcn-overlay-realdata` body-SHA256 is identical across
/// two runs against the real `data/binance/` directory.
///
/// Pre-condition: `data/binance/REVISION.toml` must exist in the workspace root
/// (populated by T-D-16 fetch).  If absent the test skips with a clear message
/// so CI (which does not carry the 240 parquets) remains green.
///
/// After T-D-17 pins the real aggregate SHA into the scenario arm, the binary
/// validates that the SHA of the on-disk manifest matches the pin.  The test
/// therefore MUST run against real data — a synthetic tempdir fixture would
/// produce a different SHA and cause the binary to exit non-zero.
#[cfg(feature = "realdata")]
#[test]
fn realdata_2023_fy_tcn_overlay_determinism() {
    if !real_binance_data_available() {
        eprintln!(
            "T-D-13: data/binance/REVISION.toml absent — skipping realdata determinism test \
             (run `cargo run -p data --bin fetch_binance_klines -- --emit-revision-manifest ...` \
             first, then re-run this test)"
        );
        return; // soft skip — does not count as failure
    }

    let bin = ensure_realdata_binary();
    let workspace = workspace_root_path();
    let scenario = "top10-2023-fy-tcn-overlay-realdata";

    let report1 = run_realdata_scenario_once(&bin, &workspace, scenario);
    let report2 = run_realdata_scenario_once(&bin, &workspace, scenario);

    let hash1 = backtest::report_body_hash(&report1);
    let hash2 = backtest::report_body_hash(&report2);

    let hex1: String = hash1.iter().map(|b| format!("{b:02x}")).collect();
    let hex2: String = hash2.iter().map(|b| format!("{b:02x}")).collect();

    assert_eq!(
        hex1, hex2,
        "T-D-13: {scenario} body-SHA256 must be identical across two runs at seed 0xC0FFEE.\n\
         Run1: {hex1}\nRun2: {hex2}"
    );
}

/// T-D-14 — `top10-2024-fy-tcn-overlay-realdata` body-SHA256 is identical across
/// two runs against the real `data/binance/` directory.
///
/// Pre-condition: same as T-D-13 (`data/binance/REVISION.toml` must exist).
/// Skips with a clear message when data is absent.
#[cfg(feature = "realdata")]
#[test]
fn realdata_2024_fy_tcn_overlay_determinism() {
    if !real_binance_data_available() {
        eprintln!("T-D-14: data/binance/REVISION.toml absent — skipping realdata determinism test");
        return; // soft skip
    }

    let bin = ensure_realdata_binary();
    let workspace = workspace_root_path();
    let scenario = "top10-2024-fy-tcn-overlay-realdata";

    let report1 = run_realdata_scenario_once(&bin, &workspace, scenario);
    let report2 = run_realdata_scenario_once(&bin, &workspace, scenario);

    let hash1 = backtest::report_body_hash(&report1);
    let hash2 = backtest::report_body_hash(&report2);

    let hex1: String = hash1.iter().map(|b| format!("{b:02x}")).collect();
    let hex2: String = hash2.iter().map(|b| format!("{b:02x}")).collect();

    assert_eq!(
        hex1, hex2,
        "T-D-14: {scenario} body-SHA256 must be identical across two runs at seed 0xC0FFEE.\n\
         Run1: {hex1}\nRun2: {hex2}"
    );
}

// ── T-D-15 — realdata + candle (weights) determinism gate ─────────────────────
//
// These tests require both `realdata` AND `candle` features because the
// `-weights-realdata` scenarios call `TcnSyncForecaster::load_bs1()` /
// `load_bs2()` at runtime.
//
// If the TCN checkpoint files are absent (LFS not resolved), the test emits a
// clear message and exits with PASS (skipped) — no panic.  This is consistent
// with the M3 tests above which gate on file presence.

/// Build the backtest binary with both `realdata` and `candle` features.
#[cfg(all(feature = "realdata", feature = "candle"))]
fn ensure_realdata_candle_binary() -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");

    {
        let _guard = BACKTEST_BUILD_MU.lock().unwrap_or_else(|p| p.into_inner());

        let status = std::process::Command::new("cargo")
            .args([
                "build",
                "--bin",
                "backtest",
                "--features",
                "realdata,candle",
            ])
            .current_dir(workspace_root)
            .status()
            .expect("cargo build failed");
        assert!(
            status.success(),
            "cargo build --bin backtest --features realdata,candle failed"
        );

        copy_to_unique(
            &workspace_root.join("target/debug/backtest"),
            &workspace_root.join("target/debug"),
            "realdata-candle",
        )
    }
}

/// Check whether the TCN checkpoint files are present (LFS resolved).
///
/// Returns `None` if absent (test should skip), `Some(path)` if present.
#[cfg(all(feature = "realdata", feature = "candle"))]
fn tcn_checkpoint_present(checkpoint_name: &str) -> bool {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root");

    // bug-log #114 — this used to build `<name>.safetensors` and test `.exists()`.
    // The real convention is `<name>-<content-hash>.safetensors`, e.g.
    // `tcn-bs1-d1c3696d…​.safetensors`, so the un-hashed path NEVER existed and this
    // returned false on every machine, resolved LFS or not. Both `_weights` tests
    // guarded by it therefore skipped silently — and reported green — from the day
    // they were written (`ce4ccbdd`, 2026-05-18) until 2026-09-26.
    //
    // Match the prefix instead, and require a real file rather than an unresolved
    // LFS pointer (a pointer is ~130 bytes; the smallest real checkpoint is ~1.6 MB).
    let dir = workspace_root
        .join("crates")
        .join("forecast")
        .join("checkpoints")
        .join("anchors");

    let prefix = format!("{checkpoint_name}-");
    let found = std::fs::read_dir(&dir).ok().and_then(|entries| {
        entries.flatten().find_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let is_match = name.starts_with(&prefix) && name.ends_with(".safetensors");
            let big_enough = e.metadata().map(|m| m.len() > 4096).unwrap_or(false);
            (is_match && big_enough).then(|| e.path())
        })
    });

    match found {
        Some(path) => {
            eprintln!("checkpoint {checkpoint_name} resolved to {path:?}");
            true
        }
        None => {
            eprintln!(
                "checkpoint {checkpoint_name}-*.safetensors absent or unresolved under {dir:?} \
                 — weights arms cannot run (bug-log #114)"
            );
            false
        }
    }
}

/// T-D-15 — `top10-2023-fy-tcn-overlay-weights-realdata` determinism.
///
/// Requires `--features realdata,candle` + LFS checkpoints AND real
/// `data/binance/` parquets.  Skips cleanly if either is absent.
#[cfg(all(feature = "realdata", feature = "candle"))]
#[test]
fn realdata_2023_fy_tcn_overlay_weights_determinism() {
    if !tcn_checkpoint_present("tcn-bs1") {
        return; // skip — LFS not resolved
    }
    if !real_binance_data_available() {
        eprintln!(
            "T-D-15: data/binance/REVISION.toml absent — skipping weights realdata determinism test"
        );
        return; // soft skip
    }

    let bin = ensure_realdata_candle_binary();
    let workspace = workspace_root_path();
    let scenario = "top10-2023-fy-tcn-overlay-weights-realdata";

    let report1 = run_realdata_scenario_once(&bin, &workspace, scenario);
    let report2 = run_realdata_scenario_once(&bin, &workspace, scenario);

    let hash1 = backtest::report_body_hash(&report1);
    let hash2 = backtest::report_body_hash(&report2);

    let hex1: String = hash1.iter().map(|b| format!("{b:02x}")).collect();
    let hex2: String = hash2.iter().map(|b| format!("{b:02x}")).collect();

    assert_eq!(
        hex1, hex2,
        "T-D-15: {scenario} body-SHA256 must be identical across two runs at seed 0xC0FFEE.\n\
         Run1: {hex1}\nRun2: {hex2}"
    );
}

/// T-D-15 — `top10-2024-fy-tcn-overlay-weights-realdata` determinism.
///
/// Requires `--features realdata,candle` + LFS checkpoints AND real
/// `data/binance/` parquets.  Skips cleanly if either is absent.
#[cfg(all(feature = "realdata", feature = "candle"))]
#[test]
fn realdata_2024_fy_tcn_overlay_weights_determinism() {
    if !tcn_checkpoint_present("tcn-bs2") {
        return; // skip — LFS not resolved
    }
    if !real_binance_data_available() {
        eprintln!(
            "T-D-15: data/binance/REVISION.toml absent — skipping weights realdata determinism test"
        );
        return; // soft skip
    }

    let bin = ensure_realdata_candle_binary();
    let workspace = workspace_root_path();
    let scenario = "top10-2024-fy-tcn-overlay-weights-realdata";

    let report1 = run_realdata_scenario_once(&bin, &workspace, scenario);
    let report2 = run_realdata_scenario_once(&bin, &workspace, scenario);

    let hash1 = backtest::report_body_hash(&report1);
    let hash2 = backtest::report_body_hash(&report2);

    let hex1: String = hash1.iter().map(|b| format!("{b:02x}")).collect();
    let hex2: String = hash2.iter().map(|b| format!("{b:02x}")).collect();

    assert_eq!(
        hex1, hex2,
        "T-D-15: {scenario} body-SHA256 must be identical across two runs at seed 0xC0FFEE.\n\
         Run1: {hex1}\nRun2: {hex2}"
    );
}

// ── R-REPRO — reproduction gates for the `-realdata` anchors (bug-log #111/#113) ──
//
// These are NOT the `*_determinism` tests above and must not be confused with them.
// Those run a scenario twice and compare it with ITSELF: they prove the engine is
// deterministic, which passes perfectly well while the code has stopped reproducing
// the anchored evidence. Both properties are worth having; only this one can see
// code-vs-evidence drift (bug-log #93).
//
// The comparison target is the CURRENT canonical namespace, `v5-sqrt-impact-2026-05`
// (ADR-0045 § D6 / v0.5.0). Each of these scenarios also carries two historical rows
// in evidence/anchors.toml — `v2.6.0-realdata + noop-baseline` (pre-friction oracle)
// and `v2.6.0-realdata + v5-realdata-medium-2026-05` (pre-sqrt-impact) — which are
// frozen history and are NOT what current code should produce.
//
// `#[ignore]` because they are a MEASUREMENT first: story 1-27 has bisected the four
// `top10-*` in-test pins to `11acd126` (the #67 fix) and the `-realdata` family has
// never had a reproduction check at all, so their state is genuinely unknown until
// these run. Invoke explicitly:
//
//     cargo test -p backtest --test determinism --features realdata,candle \
//         -- --ignored reproduces_anchor
//
// Per bug-log #113 requirement 6, a missing precondition is UNMEASURED, not a pass:
// these panic with a loud message instead of returning green, because a test you
// invoked by name and that silently did nothing is worse than no test.
//
// When a gate here is GREEN the `#[ignore]` should come off — it is then a real
// regression gate. When it is RED it stays, with the D6.b re-lock as the resolution
// (never a re-pin to current output; that is bug-log #77).

/// R-REPRO-1 — `top10-2023-fy-tcn-overlay-realdata` reproduces its canonical anchor.
#[cfg(feature = "realdata")]
#[test]
#[ignore = "measurement first: the -realdata family has never had a reproduction check (bug-log #111/#113); run with --ignored"]
fn realdata_2023_fy_tcn_overlay_reproduces_anchor() {
    const ANCHOR: &str = "1157af76be96f4ffd3a43740366252b747ad4ee077516759aababd8100c4895a";
    assert_reproduces_canonical_anchor("top10-2023-fy-tcn-overlay-realdata", ANCHOR, false);
}

/// R-REPRO-2 — `top10-2024-fy-tcn-overlay-realdata` reproduces its canonical anchor.
#[cfg(feature = "realdata")]
#[test]
#[ignore = "measurement first: the -realdata family has never had a reproduction check (bug-log #111/#113); run with --ignored"]
fn realdata_2024_fy_tcn_overlay_reproduces_anchor() {
    const ANCHOR: &str = "39a02c7955b547963ff57898a2a78a524138a91b766e6454895da8920a9e995c";
    assert_reproduces_canonical_anchor("top10-2024-fy-tcn-overlay-realdata", ANCHOR, false);
}

/// R-REPRO-3 — `top10-2023-fy-tcn-overlay-weights-realdata` reproduces its anchor.
#[cfg(all(feature = "realdata", feature = "candle"))]
#[test]
#[ignore = "measurement first: the -realdata family has never had a reproduction check (bug-log #111/#113); run with --ignored"]
fn realdata_2023_fy_tcn_overlay_weights_reproduces_anchor() {
    const ANCHOR: &str = "38736839a3c6dab3394b59a9a831873dea5eeece5e25c79ec63b09ace16a2175";
    assert_reproduces_canonical_anchor(
        "top10-2023-fy-tcn-overlay-weights-realdata",
        ANCHOR,
        true,
    );
}

/// R-REPRO-4 — `top10-2024-fy-tcn-overlay-weights-realdata` reproduces its anchor.
#[cfg(all(feature = "realdata", feature = "candle"))]
#[test]
#[ignore = "measurement first: the -realdata family has never had a reproduction check (bug-log #111/#113); run with --ignored"]
fn realdata_2024_fy_tcn_overlay_weights_reproduces_anchor() {
    const ANCHOR: &str = "582dabab182b786aa211e0c44b29b85634cc2f77c696ee6241500e29b36447f6";
    assert_reproduces_canonical_anchor(
        "top10-2024-fy-tcn-overlay-weights-realdata",
        ANCHOR,
        true,
    );
}

/// Shared body for the R-REPRO gates.
///
/// `needs_checkpoint` selects the candle-capable binary and additionally requires the
/// LFS checkpoint, which the `-weights` arms load at runtime.
///
/// # Panics
///
/// Panics when a precondition is absent (`data/binance/REVISION.toml`, or the TCN
/// checkpoint for a weights arm) — see bug-log #113 requirement 6: an explicitly
/// invoked measurement must report UNMEASURED loudly rather than return green.
#[cfg(feature = "realdata")]
fn assert_reproduces_canonical_anchor(scenario: &str, anchor: &str, needs_checkpoint: bool) {
    assert!(
        real_binance_data_available(),
        "UNMEASURED, not passed: {scenario} needs data/binance/REVISION.toml and it is absent.\n\
         Fetch the corpus first:\n  cargo run -p data --bin fetch_binance_klines -- \
         --emit-revision-manifest ...\n\
         Reporting this as a skip would be bug-log #113 requirement 6 — a gate that \
         returns green having measured nothing."
    );

    #[cfg(feature = "candle")]
    let bin = if needs_checkpoint {
        assert!(
            tcn_checkpoint_present("tcn-bs1"),
            "UNMEASURED, not passed: {scenario} needs a resolved tcn-bs1 checkpoint under \
             crates/forecast/checkpoints/anchors/ and none was found. If the file is present, \
             check the resolver before the file (bug-log #114)."
        );
        ensure_realdata_candle_binary()
    } else {
        ensure_realdata_binary()
    };
    #[cfg(not(feature = "candle"))]
    let bin = {
        assert!(
            !needs_checkpoint,
            "UNMEASURED, not passed: {scenario} needs --features candle for its real weights."
        );
        ensure_realdata_binary()
    };

    let workspace = workspace_root_path();
    let report = run_realdata_scenario_once(&bin, &workspace, scenario);
    let hex: String = backtest::report_body_hash(&report)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();

    assert_eq!(
        hex, anchor,
        "R-REPRO: {scenario} no longer reproduces its canonical \
         `v5-sqrt-impact-2026-05` anchor.\n\
         Expected: {anchor}\nGot:      {hex}\n\
         This is code-vs-evidence drift, the thing verify_anchors.sh cannot see \
         (bug-log #93). Do NOT re-pin to the produced value — that is bug-log #77. \
         The resolution is a D6.b re-lock (story 1-27)."
    );
}

// ── R-REPRO-5..9 — the -realdata anchors that had no gate at all (bug-log #111) ──
//
// These five are anchored under `v5-sqrt-impact-2026-05` and, until now, nothing in
// the repo re-ran them: `verify_anchors.sh` hashes their committed bodies and the
// `*_determinism` tests never covered them. Their reproduction state was genuinely
// unknown, which is why bug-log #111 could only report a LOWER bound on the #67
// blast radius.
//
// They use `assert_reproduces_or_report_unmeasured`, which — per bug-log #114 —
// does NOT hand-roll a precondition check for the forecaster checkpoints each arm
// needs. The binary refuses with a precise message when a feature or checkpoint is
// missing; the helper repeats that message as UNMEASURED. A guard I write myself is
// one more thing that can be wrong in the direction of silence, and #114 is exactly
// that mistake costing four months.
//
//     cargo test -p backtest --test determinism --features realdata,candle \
//         -- --ignored reproduces_anchor

/// R-REPRO-5 — `top10-2023-fy-momentum-realdata`.
#[cfg(feature = "realdata")]
#[test]
#[ignore = "known-red: measured 0fc591e5… against pin 0867d232… (2026-09-26); D6.b re-lock, never a re-pin (#77)"]
fn realdata_2023_fy_momentum_reproduces_anchor() {
    const ANCHOR: &str = "0867d232b5d4e3813992d25b7ca23eb07bf530e41d44262e3ee2bc9c6c1c9901";
    assert_reproduces_or_report_unmeasured("top10-2023-fy-momentum-realdata", ANCHOR);
}

/// R-REPRO-6 — `top10-2023-fy-patchtst-overlay-realdata`.
#[cfg(feature = "realdata")]
#[test]
#[ignore = "known-red: measured f704c4f2… against pin b015b564… (2026-09-26); D6.b re-lock, never a re-pin (#77)"]
fn realdata_2023_fy_patchtst_overlay_reproduces_anchor() {
    const ANCHOR: &str = "b015b56420d9b20387ea988d0f7f46669ae153e97387e3a9a901fff6fec73aa4";
    assert_reproduces_or_report_unmeasured("top10-2023-fy-patchtst-overlay-realdata", ANCHOR);
}

/// R-REPRO-7 — `top10-2023-fy-regime-dispatcher-realdata` reproduces its
/// **`v3.0.0-regime`** anchor. GREEN — measured 2026-09-26.
///
/// bug-log #120 — this gate was first written against the
/// `v5-sqrt-impact-2026-05` row (`857f9494…`) on the assumption that "the canonical
/// namespace" is one global choice. It is not: it is **per scenario**. The default
/// invocation reproduces the `v3.0.0-regime` row exactly, and the sqrt-impact row's
/// invocation is not established. Pinning the wrong row would have reported a
/// perfectly reproducing scenario as drifted.
///
/// Not `#[ignore]`d: this is a real regression gate now.
#[cfg(feature = "realdata")]
#[test]
fn realdata_2023_fy_regime_dispatcher_reproduces_anchor() {
    const ANCHOR: &str = "f37bbb8d3520c7bae2ff1d48fa71d704a8b122d84a3d843d443bafa359664775";
    assert_reproduces_or_report_unmeasured("top10-2023-fy-regime-dispatcher-realdata", ANCHOR);
}

/// R-REPRO-8 — `top10-2024-fy-regime-dispatcher-realdata` reproduces its
/// **`v3.0.0-regime`** anchor. GREEN — measured 2026-09-26. See R-REPRO-7 for why
/// the namespace is not the sqrt-impact one (bug-log #120).
#[cfg(feature = "realdata")]
#[test]
fn realdata_2024_fy_regime_dispatcher_reproduces_anchor() {
    const ANCHOR: &str = "691a70568f4d0e6e74e51e7318f55236b7c3e0f97968bf6aabfdacd308ba9f4e";
    assert_reproduces_or_report_unmeasured("top10-2024-fy-regime-dispatcher-realdata", ANCHOR);
}

/// R-REPRO-9 — `top10-2023-fy-vol-target-overlay-realdata`.
#[cfg(feature = "realdata")]
#[test]
#[ignore = "known-red: measured 91848e23… against pin 6adc4334… (2026-09-26); D6.b re-lock, never a re-pin (#77)"]
fn realdata_2023_fy_vol_target_overlay_reproduces_anchor() {
    const ANCHOR: &str = "6adc4334be91269de5cf3ca2f6cdc52d5b51d0f1a2c1ec3ff25a2294029f5edf";
    assert_reproduces_or_report_unmeasured("top10-2023-fy-vol-target-overlay-realdata", ANCHOR);
}

/// Compare a `-realdata` scenario against its canonical `v5-sqrt-impact-2026-05`
/// anchor, reporting a refused run as UNMEASURED **in the binary's own words**.
///
/// # Panics
///
/// Panics on drift (the point of the gate), and on an absent precondition — which is
/// reported as UNMEASURED rather than skipped, per bug-log #113 requirement 6: a test
/// invoked by name that silently does nothing is worse than no test.
#[cfg(feature = "realdata")]
fn assert_reproduces_or_report_unmeasured(scenario: &str, anchor: &str) {
    assert!(
        real_binance_data_available(),
        "UNMEASURED, not passed: {scenario} needs data/binance/REVISION.toml and it is absent."
    );

    // Release, deliberately — see `ensure_realdata_release_binary` (bug-log #121).
    let bin = ensure_realdata_release_binary(cfg!(feature = "candle"));

    let workspace = workspace_root_path();
    let report = match run_realdata_scenario_try(&bin, &workspace, scenario) {
        Ok(r) => r,
        Err(why) => panic!(
            "UNMEASURED, not passed: {scenario} — the binary refused this run. Its own words:\n\
             {why}\n\
             Reported as unmeasured rather than skipped (bug-log #113 req 6), and deliberately \
             NOT pre-guarded by a hand-rolled precondition check (bug-log #114)."
        ),
    };

    let hex: String = backtest::report_body_hash(&report)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();

    assert_eq!(
        hex, anchor,
        "R-REPRO: {scenario} no longer reproduces its canonical \
         `v5-sqrt-impact-2026-05` anchor.\n\
         Expected: {anchor}\nGot:      {hex}\n\
         Code-vs-evidence drift — the thing verify_anchors.sh cannot see (bug-log #93). \
         Do NOT re-pin to the produced value (bug-log #77); the resolution is a D6.b \
         re-lock (story 1-27)."
    );
}
