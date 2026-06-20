//! Integration tests for the bake-off + ranking engine (advisor F1+F2, ADR-0059).
//!
//! # Test catalogue
//!
//! T2.2 — buyhold arm parity: `run_scenario("v0.buyhold")` equity series ==
//!         `run_buyhold_path` directly on the same synthetic bars.
//!
//! T6.1 — deterministic bake-off on real Binance data (requires `--features realdata`).
//!         `#[ignore]` by default; run with `cargo test -p backtest --features realdata
//!         --test bakeoff_e2e -- --ignored`.

#[cfg(test)]
mod bakeoff_arm_parity {
    use backtest::{
        DateRange, ScenarioConfig,
        bakeoff::buyhold::run_buyhold_path,
        cancel::cancellation_pair,
        cli_types::LatencySlippageSimConfig,
        engine::{ScenarioDataSource, run_scenario},
        progress::ProgressSender,
    };
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;
    use trading_core::{Bar, Price, Quantity, StrategyId, Symbol, Timeframe, Timestamp, Venue};

    fn make_bar(ts_offset_hours: i64, close: Decimal) -> Bar {
        let ts =
            Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::hours(ts_offset_hours));
        let price = Price::new(close)
            .unwrap_or_else(|_| Price::new(dec!(1)).expect("dec!(1) is valid price"));
        let qty = Quantity::new(Decimal::ZERO).expect("zero qty is valid");
        Bar {
            symbol: Symbol::new("BTCUSDT"),
            tf: Timeframe::OneHour,
            venue: Venue::Binance,
            open_ts: ts,
            close_ts: ts,
            open: price,
            high: price,
            low: price,
            close: price,
            volume: qty,
            trade_count: 0,
            local_recv_ts: ts,
        }
    }

    /// T2.2 — The equity series produced by the `"v0.buyhold"` `run_scenario`
    /// arm must equal what `run_buyhold_path` produces on the same bar list when
    /// both use the same initial capital.
    ///
    /// Verification strategy:
    /// - Build a small bar slice (5 bars, price [100, 110, 120, 130, 140]).
    /// - Call `run_buyhold_path` directly → get `(curve, final_eq)`.
    /// - Call `run_scenario("v0.buyhold", bars_override = same slice)` → `RunReport`.
    /// - Assert `RunReport::final_equity` ≈ `final_eq` (Decimal-exact).
    /// - Assert the reported trade count is 0 (buyhold has no algo-driven trades).
    #[tokio::test]
    async fn t2_2_buyhold_arm_parity() {
        let bars = vec![
            make_bar(0, dec!(100)),
            make_bar(1, dec!(110)),
            make_bar(2, dec!(120)),
            make_bar(3, dec!(130)),
            make_bar(4, dec!(140)),
        ];

        // The v0.buyhold arm in engine.rs uses INITIAL_CAPITAL = 100_000.
        let initial_capital = dec!(100_000);

        // Direct path: run_buyhold_path with the same initial capital.
        let (_curve, final_eq) = run_buyhold_path(&bars, initial_capital, 1);

        // Engine path: run_scenario with bars_override.
        let mut seed = [0u8; 32];
        seed[0] = 0xBA; // non-zero seed required

        let (_handle, cancel_rx) = cancellation_pair();
        let progress_tx = ProgressSender::disabled();

        let cfg = ScenarioConfig {
            strategy: StrategyId("v0.buyhold".into()),
            pair: (Venue::Binance, Symbol::new("BTCUSDT")),
            range: DateRange::Last90d,
            params: None,
            seed,
            write_report: false,
            data_source: ScenarioDataSource::Synthetic,
            bars_override: Some(bars.clone()),
            sma_fast_len: None,
            sma_slow_len: None,
            latency_slippage_sim: LatencySlippageSimConfig::default(),
            reports_dir: None,
        };

        let report = run_scenario(cfg, cancel_rx, progress_tx)
            .await
            .expect("run_scenario v0.buyhold should succeed");

        // The engine produces final equity as Money<Usdt>; compare as Decimal.
        let engine_final: Decimal = report.kpis.final_equity.amount();

        // Decimal-exact comparison (no f64 path in buyhold arm).
        assert_eq!(
            engine_final, final_eq,
            "buyhold arm parity failed: engine={engine_final}, direct={final_eq}"
        );

        // Buyhold has no algo-driven trades.
        assert_eq!(
            report.kpis.trade_count, 0,
            "buyhold arm should report 0 trades, got {}",
            report.kpis.trade_count
        );
    }
}

/// T6.1 — deterministic bake-off on real Binance data.
///
/// Run with:
/// ```
/// cargo test -p backtest --features realdata --test bakeoff_e2e t6_1 -- --ignored
/// ```
#[cfg(feature = "realdata")]
#[cfg(test)]
mod bakeoff_realdata {
    use backtest::{
        BakeoffRequest, DateRange, RobustnessMode, bakeoff::BakeoffConfig as BakeoffCfg,
        cancel::cancellation_pair, engine::ScenarioDataSource, progress::ProgressSender,
        run_bakeoff,
    };
    use trading_core::Symbol;

    /// Resolve the workspace root from `CARGO_MANIFEST_DIR` (the crate root).
    ///
    /// `CARGO_MANIFEST_DIR` = `crates/backtest`; workspace root is two levels up.
    fn workspace_root() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("crate has parent")
            .parent()
            .expect("crates/ has parent (workspace root)")
            .to_path_buf()
    }

    /// T6.1 — two runs with the same seed on the same date range must produce
    /// the byte-identical ranked order and crowned winner.
    #[ignore]
    #[tokio::test]
    async fn t6_1_bakeoff_deterministic_on_real_data() {
        // `run_bakeoff` with BinanceCache resolves "data/binance" relative to cwd.
        // Mirror the survey test pattern: pin cwd to workspace root before running.
        let ws_root = workspace_root();
        std::env::set_current_dir(&ws_root).expect("set_current_dir to workspace root");

        // Skip gracefully when corpus is absent (CI without data/ mounted).
        if !ws_root
            .join("data/binance/BTCUSDT/2024/01.parquet")
            .is_file()
        {
            eprintln!("T6.1 SKIP: data/binance corpus absent");
            return;
        }

        let range = DateRange::Custom {
            start_ms: time::macros::datetime!(2024-01-01 00:00:00 UTC).unix_timestamp() * 1000,
            end_ms: time::macros::datetime!(2024-03-31 23:59:00 UTC).unix_timestamp() * 1000,
        };
        let seed = {
            let mut s = [0u8; 32];
            s[0] = 0x42;
            s
        };
        let symbol = Symbol::new("BTCUSDT");
        let field = backtest::bakeoff::BakeoffConfig::default_field();

        let make_cfg = || BakeoffCfg {
            request: BakeoffRequest {
                symbol: symbol.clone(),
                range: range.clone(),
                seed,
                field: field.clone(),
            },
            data_source: ScenarioDataSource::BinanceCache,
            robustness: RobustnessMode::Skip,
        };

        let run = |cfg: BakeoffCfg| async move {
            let (_handle, cancel_rx) = cancellation_pair();
            let progress_tx = ProgressSender::disabled();
            run_bakeoff(cfg, cancel_rx, progress_tx)
                .await
                .expect("bakeoff should succeed")
        };

        let r1 = run(make_cfg()).await;

        // Visible leaderboard (shown with `--nocapture`) — doubles as the
        // operator-facing sample of a real bake-off.
        eprintln!(
            "\n=== BAKE-OFF  {}  {:?} ===",
            r1.request.symbol.0.as_str(),
            r1.request.range
        );
        for &i in &r1.ranked {
            let c = &r1.candidates[i];
            let crown = if r1.crowned == Some(i) {
                "  <== CROWN"
            } else {
                ""
            };
            eprintln!(
                "  {:<13}  sharpe={:>7.3}  return={:>8}%  maxDD={:>8}%  trades={:<4}{}",
                c.strategy.0.as_str(),
                c.kpis.sharpe,
                (c.kpis.total_return_pct * rust_decimal::Decimal::from(100)).round_dp(2),
                (c.kpis.max_drawdown * rust_decimal::Decimal::from(100)).round_dp(2),
                c.kpis.trade_count,
                crown,
            );
        }
        eprintln!(
            "  outcome={:?}  winner={}  reasons={:?}\n",
            r1.rationale.outcome,
            r1.rationale.winner.0.as_str(),
            r1.rationale.reasons
        );

        let r2 = run(make_cfg()).await;

        // Ranked order must be byte-identical.
        assert_eq!(r1.ranked, r2.ranked, "ranked order must be deterministic");

        // Crowned winner must be identical.
        assert_eq!(
            r1.crowned, r2.crowned,
            "crowned winner must be deterministic"
        );
    }

    /// T6.2 — bake-off real-data sanity guard: buy-and-hold on a KNOWN-BULL
    /// window (BTCUSDT 2024-Q1, ~+65% real rally) must report a clearly POSITIVE
    /// total_return_pct (≥ +20%).
    ///
    /// This is the durable regression guard against the synthetic-fallback bug
    /// (ADR-0059 §bug-fix 2026-06-19): when `BinanceCache` + `bars_override: None`
    /// silently generated synthetic GBM bars, buy-and-hold reported -3.24% on a
    /// quarter that actually rallied ~+65%.  This test would have caught it.
    ///
    /// Run with:
    /// ```
    /// cargo test -p backtest --features realdata --test bakeoff_e2e t6_2 -- --ignored --nocapture
    /// ```
    #[ignore]
    #[tokio::test]
    async fn t6_2_bakeoff_buyhold_positive_on_bull_window() {
        use rust_decimal_macros::dec;

        // `run_bakeoff` with BinanceCache resolves "data/binance" relative to cwd.
        let ws_root = workspace_root();
        std::env::set_current_dir(&ws_root).expect("set_current_dir to workspace root");

        // Skip gracefully when corpus is absent (CI without data/ mounted).
        if !ws_root
            .join("data/binance/BTCUSDT/2024/01.parquet")
            .is_file()
        {
            eprintln!("T6.2 SKIP: data/binance corpus absent");
            return;
        }

        // BTCUSDT 2024-Q1: confirmed ~+65% real-data bull rally.
        let range = DateRange::Custom {
            start_ms: time::macros::datetime!(2024-01-01 00:00:00 UTC).unix_timestamp() * 1000,
            end_ms: time::macros::datetime!(2024-03-31 23:59:00 UTC).unix_timestamp() * 1000,
        };
        let seed = {
            let mut s = [0u8; 32];
            s[0] = 0x42;
            s
        };
        let symbol = Symbol::new("BTCUSDT");

        let cfg = BakeoffCfg {
            request: BakeoffRequest {
                symbol: symbol.clone(),
                range: range.clone(),
                seed,
                field: backtest::bakeoff::BakeoffConfig::default_field(),
            },
            data_source: ScenarioDataSource::BinanceCache,
            robustness: RobustnessMode::Skip,
        };

        let (_handle, cancel_rx) = cancellation_pair();
        let progress_tx = ProgressSender::disabled();
        let report = run_bakeoff(cfg, cancel_rx, progress_tx)
            .await
            .expect("bakeoff on bull window should succeed");

        // Find the buy-and-hold arm.
        let buyhold = report
            .candidates
            .iter()
            .find(|c| c.is_benchmark)
            .expect("bake-off must include a buy-and-hold arm");

        eprintln!(
            "\n=== T6.2 SANITY GUARD — {} 2024-Q1 ===",
            symbol.0.as_str()
        );
        eprintln!(
            "  buy-and-hold total_return = {:.2}%  (must be > +20%)",
            (buyhold.kpis.total_return_pct * rust_decimal::Decimal::from(100)).round_dp(2),
        );
        eprintln!("  buy-and-hold sharpe       = {:.3}", buyhold.kpis.sharpe);

        // Guard: real BTC 2024-Q1 rallied ~+65%; we require at least +20% to
        // prove real data reached the engine (not synthetic GBM garbage).
        assert!(
            buyhold.kpis.total_return_pct > dec!(0.20),
            "T6.2 FAIL: buy-and-hold total_return_pct = {:.4} is not > +20%; \
             synthetic-fallback bug may have regressed (expected real BTC 2024-Q1 ~+65%)",
            buyhold.kpis.total_return_pct,
        );
    }
}
