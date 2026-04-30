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
    Timeframe, Timestamp, Usdt,
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
                if let Some(order) = ord {
                    if let Ok(fills) = engine.step(bar, vec![order]).await {
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

    // Use a temp directory for report output so we don't pollute spec/reports.
    let tmp = tempfile::tempdir().expect("create tempdir");
    let reports_dir = tmp.path().join("spec/reports");
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

        // The binary prints "Report written: spec/reports/backtest-<stamp>-<scenario>.md"
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
    let reports_dir = tmp.path().join("spec/reports");
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
// Anchor hashes (locked per spec/tasks/v1-cross-sectional-momentum.md T622):
//   btc-2023-1m-sma-cross         fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
//   btc-2023-1m-sma-baseline-refresh  (same body as sma-cross)
//   btc-2023-1m-macd-trend        ef9c5e48… (abbreviated in spec — verified at T521 ship)
//   btc-2023-1m-rsi-reversion     bc56d20d… (abbreviated in spec)
//   btc-2023-1m-bbands-mean-revert d8a08a23… (abbreviated in spec)
//
// NOTE: The abbreviated hashes (ef9c5e48…, bc56d20d…, d8a08a23…) are shortened
// in the spec.  For the regression gate we use a "starts_with" check rather than
// a full 64-character equality so the test still compiles even before the full
// hashes are recorded.  Once a full hash is observed, replace the 8-char prefix
// below with the 64-char value.

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
    let reports_dir = tmp.path().join("spec/reports");
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

fn scenario_body_hex(scenario: &str) -> String {
    let report = run_scenario_once(scenario);
    let hash = backtest::report_body_hash(&report);
    hash.iter().map(|b| format!("{b:02x}")).collect::<String>()
}

/// T622 — v0 SMA anchor hash unchanged after v1 changes.
///
/// Locked anchor: `fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c`
#[test]
fn t622_sma_cross_anchor_hash_unchanged() {
    const ANCHOR: &str = "fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c";
    let hex = scenario_body_hex("btc-2023-1m-sma-cross");
    assert_eq!(
        hex, ANCHOR,
        "T622 REGRESSION: btc-2023-1m-sma-cross body-SHA256 changed.\n\
         Expected: {ANCHOR}\n\
         Got:      {hex}"
    );
}

/// T622 — v0.5 SMA baseline-refresh anchor (same body as sma-cross).
#[test]
fn t622_sma_baseline_refresh_anchor_hash_unchanged() {
    const ANCHOR: &str = "fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c";
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
/// Spec anchor prefix: `ef9c5e48` — full hash recorded at T521 ship.
#[test]
fn t622_macd_trend_anchor_hash_unchanged() {
    const ANCHOR_PREFIX: &str = "ef9c5e48";
    let hex = scenario_body_hex("btc-2023-1m-macd-trend");
    assert!(
        hex.starts_with(ANCHOR_PREFIX),
        "T622 REGRESSION: btc-2023-1m-macd-trend body-SHA256 changed.\n\
         Expected prefix: {ANCHOR_PREFIX}\n\
         Got:             {hex}"
    );
}

/// T622 — v0.5 RSI reversion anchor hash unchanged.
///
/// Spec anchor prefix: `bc56d20d` — full hash recorded at T521 ship.
#[test]
fn t622_rsi_reversion_anchor_hash_unchanged() {
    const ANCHOR_PREFIX: &str = "bc56d20d";
    let hex = scenario_body_hex("btc-2023-1m-rsi-reversion");
    assert!(
        hex.starts_with(ANCHOR_PREFIX),
        "T622 REGRESSION: btc-2023-1m-rsi-reversion body-SHA256 changed.\n\
         Expected prefix: {ANCHOR_PREFIX}\n\
         Got:             {hex}"
    );
}

/// T622 — v0.5 BBands mean-revert anchor hash unchanged.
///
/// Spec anchor prefix: `d8a08a23` — full hash recorded at T521 ship.
#[test]
fn t622_bbands_mean_revert_anchor_hash_unchanged() {
    const ANCHOR_PREFIX: &str = "d8a08a23";
    let hex = scenario_body_hex("btc-2023-1m-bbands-mean-revert");
    assert!(
        hex.starts_with(ANCHOR_PREFIX),
        "T622 REGRESSION: btc-2023-1m-bbands-mean-revert body-SHA256 changed.\n\
         Expected prefix: {ANCHOR_PREFIX}\n\
         Got:             {hex}"
    );
}

// ── T717 — v0 + v0.5 + v1 full-hash regression gate ──────────────────────────
//
// These tests extend T622 with the complete 64-char anchor hashes for all 7
// v0/v0.5/v1 scenarios.  The v1.5a backend changes must not affect any of
// these anchors (architecture determinism contract R9.4).
//
// Anchor hashes locked per spec/tasks/v15a-mean-reversion-pairs.md T717:
//   btc-2023-1m-sma-cross             fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
//   btc-2023-1m-sma-baseline-refresh  fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
//   btc-2023-1m-macd-trend            ef9c5e483fa079f670a7aa15671643fce3b39a5ce35df8cb6d797887053f8805
//   btc-2023-1m-rsi-reversion         bc56d20d608c680e534bf6764ce8e0e568f0d4ffdf847a539c53fef65170d7aa
//   btc-2023-1m-bbands-mean-revert    d8a08a23d3629556c5fca39d6af89d7e0f99418e642af0b86fce22ff4d2792e3
//   top10-2023-1h-momentum            a20431e3f5765cefbdfed7d1157654bcbec90d90e4bd178cdd37ce084cba55af
//   top10-2024-h1-momentum            38b576335c9a7a45b7f4a74ecf82ca8310b89ae025c2ba33c56f79e62c22ba2c

/// T717 — SMA cross anchor unchanged after v1.5a backend changes.
#[test]
fn t717_sma_cross_anchor_hash_unchanged() {
    const ANCHOR: &str = "fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c";
    let hex = scenario_body_hex("btc-2023-1m-sma-cross");
    assert_eq!(
        hex, ANCHOR,
        "T717 REGRESSION: btc-2023-1m-sma-cross body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

/// T717 — SMA baseline-refresh anchor unchanged after v1.5a backend changes.
#[test]
fn t717_sma_baseline_refresh_anchor_hash_unchanged() {
    const ANCHOR: &str = "fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c";
    let hex = scenario_body_hex("btc-2023-1m-sma-baseline-refresh");
    assert_eq!(
        hex, ANCHOR,
        "T717 REGRESSION: btc-2023-1m-sma-baseline-refresh body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

/// T717 — MACD trend full anchor hash unchanged.
#[test]
fn t717_macd_trend_anchor_hash_unchanged() {
    const ANCHOR: &str = "ef9c5e483fa079f670a7aa15671643fce3b39a5ce35df8cb6d797887053f8805";
    let hex = scenario_body_hex("btc-2023-1m-macd-trend");
    assert_eq!(
        hex, ANCHOR,
        "T717 REGRESSION: btc-2023-1m-macd-trend body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

/// T717 — RSI reversion full anchor hash unchanged.
#[test]
fn t717_rsi_reversion_anchor_hash_unchanged() {
    const ANCHOR: &str = "bc56d20d608c680e534bf6764ce8e0e568f0d4ffdf847a539c53fef65170d7aa";
    let hex = scenario_body_hex("btc-2023-1m-rsi-reversion");
    assert_eq!(
        hex, ANCHOR,
        "T717 REGRESSION: btc-2023-1m-rsi-reversion body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

/// T717 — BBands mean-revert full anchor hash unchanged.
#[test]
fn t717_bbands_mean_revert_anchor_hash_unchanged() {
    const ANCHOR: &str = "d8a08a23d3629556c5fca39d6af89d7e0f99418e642af0b86fce22ff4d2792e3";
    let hex = scenario_body_hex("btc-2023-1m-bbands-mean-revert");
    assert_eq!(
        hex, ANCHOR,
        "T717 REGRESSION: btc-2023-1m-bbands-mean-revert body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

/// T717 — top10-2023-1h-momentum anchor hash unchanged.
#[test]
fn t717_top10_2023_momentum_anchor_hash_unchanged() {
    const ANCHOR: &str = "a20431e3f5765cefbdfed7d1157654bcbec90d90e4bd178cdd37ce084cba55af";
    let hex = scenario_body_hex("top10-2023-1h-momentum");
    assert_eq!(
        hex, ANCHOR,
        "T717 REGRESSION: top10-2023-1h-momentum body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}

/// T717 — top10-2024-h1-momentum anchor hash unchanged.
#[test]
fn t717_top10_2024_momentum_anchor_hash_unchanged() {
    const ANCHOR: &str = "38b576335c9a7a45b7f4a74ecf82ca8310b89ae025c2ba33c56f79e62c22ba2c";
    let hex = scenario_body_hex("top10-2024-h1-momentum");
    assert_eq!(
        hex, ANCHOR,
        "T717 REGRESSION: top10-2024-h1-momentum body-SHA256 changed.\n\
         Expected: {ANCHOR}\nGot:      {hex}"
    );
}
