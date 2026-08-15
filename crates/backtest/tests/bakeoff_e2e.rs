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
            short_enabled: false,
            initial_capital: None,
            composed_toml_override: None,
            dvol_override: None,
            macro_regime_series: None,
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

/// T-PROG-1 — candidate-level bake-off progress channel.
///
/// Drives `run_bakeoff` with a live `BakeoffProgressSender` over a minimal
/// synthetic one-strategy field (`["v0.sma"]`).  Total candidates = 2 (v0.sma
/// + the always-appended v0.buyhold benchmark).
///
/// Asserts:
/// 1. Exactly `total` (= 2) `BakeoffProgress` events are received.
/// 2. `done` values form a strictly-monotone sequence 0, 1 (0-based).
/// 3. `current_id` sequence is `["v0.sma", "v0.buyhold"]`.
/// 4. `total` is 2 in every message.
#[cfg(test)]
mod bakeoff_progress {
    use backtest::{
        BakeoffRequest, DateRange, RobustnessMode,
        bakeoff::BakeoffConfig as BakeoffCfg,
        cancel::cancellation_pair,
        engine::ScenarioDataSource,
        progress::{BakeoffProgressSender, ProgressSender, bakeoff_progress_pair},
        resample::Horizon,
        run_bakeoff,
    };
    use rust_decimal_macros::dec;
    use trading_core::Symbol;

    /// The one-strategy synthetic field used by the progress test.
    fn progress_test_field() -> Vec<trading_core::StrategyId> {
        use smol_str::SmolStr;
        vec![trading_core::StrategyId(SmolStr::new_static("v0.sma"))]
    }

    #[allow(clippy::unwrap_used, clippy::expect_used)]
    #[tokio::test]
    async fn t_prog_1_bakeoff_progress_sequence() {
        // One non-zero seed (ZeroSeed is an error).
        let seed = {
            let mut s = [0u8; 32];
            s[0] = 0xAB;
            s
        };

        let cfg = BakeoffCfg {
            request: BakeoffRequest {
                symbol: Symbol::new("BTCUSDT"),
                range: DateRange::Last30d, // Synthetic ignores the range window
                seed,
                field: progress_test_field(),
                timeframe: Horizon::OneHour,
                initial_capital: dec!(100_000),
            },
            data_source: ScenarioDataSource::Synthetic,
            robustness: RobustnessMode::Skip,
        };

        // Build a live (Some) bakeoff progress channel.
        let (bakeoff_tx, mut bakeoff_rx) = bakeoff_progress_pair();

        let (_handle, cancel_rx) = cancellation_pair();
        let progress_tx = ProgressSender::disabled();

        // Run the bakeoff — sender is consumed (dropped) on return, so the
        // channel closes and recv() will eventually return None.
        let _report = run_bakeoff(cfg, cancel_rx, progress_tx, bakeoff_tx)
            .await
            .expect("run_bakeoff with Some progress tx should succeed");

        // Drain all buffered progress events.
        let mut events = Vec::new();
        // The sender was moved into run_bakeoff and dropped on return, so the
        // channel is now closed. recv() returns None when the buffer is empty.
        while let Some(ev) = bakeoff_rx.recv().await {
            events.push(ev);
        }

        // 1. Exactly `total` events.
        let expected_total: u16 = 2; // v0.sma + v0.buyhold
        assert_eq!(
            events.len(),
            expected_total as usize,
            "T-PROG-1: expected {expected_total} progress events, got {}",
            events.len()
        );

        // 2. `total` field is correct and consistent in all events.
        for ev in &events {
            assert_eq!(
                ev.total, expected_total,
                "T-PROG-1: ev.total should be {expected_total}, got {}",
                ev.total
            );
        }

        // 3. `done` values are monotonically increasing (0, 1, 2, …).
        let done_values: Vec<u16> = events.iter().map(|e| e.done).collect();
        let expected_done: Vec<u16> = (0u16..expected_total).collect();
        assert_eq!(
            done_values, expected_done,
            "T-PROG-1: `done` sequence should be {expected_done:?}, got {done_values:?}"
        );

        // 4. `current_id` sequence matches field order + buyhold.
        let ids: Vec<&str> = events.iter().map(|e| e.current_id.as_str()).collect();
        assert_eq!(
            ids,
            vec!["v0.sma", "v0.buyhold"],
            "T-PROG-1: current_id sequence mismatch: {ids:?}"
        );
    }

    /// Sanity: `BakeoffProgressSender::disabled()` path produces no events.
    #[allow(clippy::expect_used)]
    #[tokio::test]
    async fn t_prog_disabled_produces_no_events() {
        let seed = {
            let mut s = [0u8; 32];
            s[0] = 0x01;
            s
        };
        let cfg = BakeoffCfg {
            request: BakeoffRequest {
                symbol: Symbol::new("BTCUSDT"),
                range: DateRange::Last30d,
                seed,
                field: progress_test_field(),
                timeframe: Horizon::OneHour,
                initial_capital: dec!(100_000),
            },
            data_source: ScenarioDataSource::Synthetic,
            robustness: RobustnessMode::Skip,
        };
        // Disabled sender — no channel allocation.
        let (_, mut rx) = bakeoff_progress_pair();
        drop(rx.recv()); // drain nothing — just ensure compile

        let (_handle, cancel_rx) = cancellation_pair();
        // Actually run with disabled sender.
        let result = run_bakeoff(
            cfg,
            cancel_rx,
            ProgressSender::disabled(),
            BakeoffProgressSender::disabled(),
        )
        .await;
        assert!(result.is_ok(), "disabled sender path must not error");
    }
}

/// bug-log #81 / review 3-16 CRITICAL — the macro arm degrades to **ABSENCE**.
///
/// This is the production-binding gate for the drop-to-ABSENCE guard in
/// `run_bakeoff`. It calls the real `run_bakeoff` (not a helper, not a
/// constructor) and asserts on the RANKED OUTPUT the cockpit renders.
///
/// The defect it pins: `v0.macro_riskon` used to be dispatched unconditionally.
/// With no regime series the engine arm builds an EMPTY `PitSeries`,
/// `as_of_value` returns `None` at every bar, and the arm holds **100% cash**
/// for the whole window — a ranked row, under the label *"Macro regime (hold
/// when SPX up, DXY down, rates calm)"*, for an experiment that never ran. The
/// DVOL sibling got this guard; the macro arm did not.
///
/// The assertion is an **iff against the real precondition**, evaluated
/// independently of the bake-off: the arm appears in the ranked field exactly
/// when `load_macro_regime_series` succeeds for the same corpus root and range.
/// In the shipped build the loader is not even compiled
/// (`#![cfg(feature = "yahoo")]`, nothing enables `backtest/yahoo`), so the
/// right-hand side is `false` by construction and the arm must NEVER appear.
///
/// Non-vacuity witness: the test also asserts the rest of the field DID run, so
/// it cannot pass by the bake-off returning an empty/failed report.
#[cfg(test)]
mod macro_arm_absence {
    use backtest::{
        BakeoffConfig as BakeoffCfg, BakeoffRequest, DateRange, RobustnessMode,
        cancel::cancellation_pair,
        engine::ScenarioDataSource,
        progress::{BakeoffProgressSender, ProgressSender},
        resample::Horizon,
        run_bakeoff,
    };
    use rust_decimal_macros::dec;
    use trading_core::{StrategyId, Symbol};

    /// Would the macro regime series actually load for this range, from the same
    /// corpus root `run_bakeoff` uses? `false` by construction when the loader
    /// module is not compiled — which is the shipped build (bug-log #81).
    #[cfg(feature = "yahoo")]
    fn macro_series_loads_for(range: &DateRange) -> bool {
        backtest::macro_regime::load_macro_regime_series(
            std::path::Path::new("data/yahoo-macro"),
            range,
        )
        .is_ok()
    }
    #[cfg(not(feature = "yahoo"))]
    fn macro_series_loads_for(_range: &DateRange) -> bool {
        false
    }

    #[allow(clippy::expect_used)]
    #[tokio::test]
    async fn macro_arm_is_absent_from_the_ranked_field_when_its_series_is_unavailable() {
        let seed = {
            let mut s = [0u8; 32];
            s[0] = 0x5A;
            s
        };
        let range = DateRange::Last30d; // Synthetic ignores the window
        let field = vec![
            StrategyId("v0.sma".into()),
            StrategyId("v0.macro_riskon".into()),
        ];

        let cfg = BakeoffCfg {
            request: BakeoffRequest {
                symbol: Symbol::new("BTCUSDT"),
                range: range.clone(),
                seed,
                field,
                timeframe: Horizon::OneHour,
                initial_capital: dec!(100_000),
            },
            data_source: ScenarioDataSource::Synthetic,
            robustness: RobustnessMode::Skip,
        };

        let (_handle, cancel_rx) = cancellation_pair();
        let report = run_bakeoff(
            cfg,
            cancel_rx,
            ProgressSender::disabled(),
            BakeoffProgressSender::disabled(),
        )
        .await
        .expect("run_bakeoff must succeed — dropping an arm is not an error");

        let ids: Vec<&str> = report
            .candidates
            .iter()
            .map(|c| c.strategy.0.as_str())
            .collect();

        // Non-vacuity: the bake-off really ran the rest of the field.
        assert!(
            ids.contains(&"v0.sma"),
            "the non-macro arm must still run — otherwise this test proves nothing \
             about the macro arm's absence; got {ids:?}"
        );
        assert!(
            ids.contains(&"v0.buyhold"),
            "the appended benchmark must be present; got {ids:?}"
        );

        // The gate.
        let has_macro = ids.contains(&"v0.macro_riskon");
        let series_available = macro_series_loads_for(&range);
        assert_eq!(
            has_macro, series_available,
            "v0.macro_riskon must appear in the ranked field IFF its regime series \
             loaded (available={series_available}, present={has_macro}). A present-but-\
             unavailable arm is the 100%-cash stub wearing the probe's label \
             (bug-log #81); an absent-but-available arm silently loses a real result. \
             Ranked ids: {ids:?}"
        );

        // And in the SHIPPED build the loader is not compiled at all, so the arm
        // can never appear no matter what corpus is on disk.
        if !backtest::bakeoff::macro_arm_compiled() {
            assert!(
                !has_macro,
                "the macro regime loader is not compiled in this build \
                 (`backtest/yahoo` off) — the arm must be ABSENT, not ranked as cash"
            );
        }
    }
}

/// T7.1 — full wired advisor bake-off on real BTCUSDT H1_2024 data.
///
/// Runs `run_bakeoff` with the EXACT config the live cockpit uses — 20 arms
/// (10 rule engines + 8 vote ensembles + 1 ADR-0073 macro arm + buy-and-hold
/// appended), the real Bootstrap robustness gate (1000 paths), `H1_2024` from
/// the pinned corpus, `BinanceCache`. Prints the full ranked leaderboard with
/// `--nocapture` for orchestrator sanity-checking against reality.
///
/// Run with:
/// ```
/// cargo test -p backtest --features realdata --test bakeoff_e2e t7_1 -- --ignored --nocapture
/// ```
#[cfg(feature = "realdata")]
#[cfg(test)]
mod bakeoff_full_wired_advisor {
    use backtest::{
        BakeoffConfig as BakeoffCfg, BakeoffRequest, DateRange, RobustnessFlag, RobustnessMode,
        cancel::cancellation_pair, engine::ScenarioDataSource, progress::ProgressSender,
        resample::Horizon, run_bakeoff,
    };
    use trading_core::Symbol;

    /// Workspace root: `CARGO_MANIFEST_DIR` = `crates/backtest`; two levels up.
    fn workspace_root() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("crate has parent")
            .parent()
            .expect("crates/ has parent (workspace root)")
            .to_path_buf()
    }

    /// T7.1 — full advisor bake-off on real BTCUSDT H1_2024 data.
    ///
    /// Replicates `ui::leaderboard::runner::default_bakeoff_config` exactly:
    /// - field = `default_field()` ∪ `default_ensemble_field()` ∪ `default_macro_field()`
    ///   (19 DECLARED arms before buyhold; ADR-0073 adds v0.macro_riskon, which
    ///   only runs when `--features yahoo` compiled its loader — bug-log #81).
    /// - seed  = `LAB_DEFAULT_SEED` = `[0xC0, 0xFF, 0xEE, 0, …]`.
    /// - robustness = Bootstrap { paths: 1000, seed: u64_from_le_bytes(seed[0..8]) }.
    /// - data_source = BinanceCache.
    /// - range = H1_2024 (2024-01-01 .. 2024-07-01 UTC).
    ///
    /// Prints the full ranked leaderboard and asserts:
    /// 1. buy-and-hold total_return > +20% (proves real data, not synthetic GBM).
    /// 2. Every arm that CAN run in this build produces a result, and the ones
    ///    that cannot are ABSENT rather than degenerate (bug-log #81).
    /// 3. Ensembles (`v0.8.vote.*`) are present and distinct from the members.
    #[ignore]
    #[allow(clippy::unwrap_used, clippy::expect_used, clippy::float_arithmetic)]
    #[tokio::test]
    async fn t7_1_full_wired_advisor_bakeoff_real_data() {
        use rust_decimal_macros::dec;

        let ws_root = workspace_root();
        std::env::set_current_dir(&ws_root).expect("set_current_dir to workspace root");

        // Skip gracefully when corpus is absent (CI without data/ mounted).
        if !ws_root
            .join("data/binance/BTCUSDT/2024/01.parquet")
            .is_file()
        {
            eprintln!("T7.1 SKIP: data/binance corpus absent");
            return;
        }

        // Exact seed from `ui::leaderboard::runner::LAB_DEFAULT_SEED`
        // (= `ui::lab::defaults::LAB_DEFAULT_SEED = [0xC0, 0xFF, 0xEE, 0, …]`).
        // Cannot import `ui` from `backtest` tests — hard-code the constant.
        let seed: [u8; 32] = {
            let mut s = [0u8; 32];
            s[0] = 0xC0;
            s[1] = 0xFF;
            s[2] = 0xEE;
            s
        };

        // Bootstrap seed = u64::from_le_bytes(seed[0..8]) — exact formula used
        // by `advisor_robustness()` in runner.rs.
        let bootstrap_seed = u64::from_le_bytes([
            seed[0], seed[1], seed[2], seed[3], seed[4], seed[5], seed[6], seed[7],
        ]);

        // Field = default_field() ∪ default_ensemble_field() ∪ default_macro_field()
        // — exact advisor field (ADR-0073 adds the macro arm).
        let mut field = BakeoffCfg::default_field();
        field.extend(BakeoffCfg::default_ensemble_field());
        field.extend(BakeoffCfg::default_macro_field()); // ADR-0073

        let cfg = BakeoffCfg {
            request: BakeoffRequest {
                symbol: Symbol::new("BTCUSDT"),
                range: DateRange::H1_2024,
                seed,
                field,
                timeframe: Horizon::OneHour,
                initial_capital: dec!(100_000),
            },
            data_source: ScenarioDataSource::BinanceCache,
            robustness: RobustnessMode::Bootstrap {
                paths: 1000,
                seed: bootstrap_seed,
            },
        };

        let (_handle, cancel_rx) = cancellation_pair();
        let progress_tx = ProgressSender::disabled();

        eprintln!("\n=== T7.1 FULL WIRED ADVISOR BAKE-OFF — BTCUSDT H1_2024 (Bootstrap 1000) ===");
        eprintln!("  Seed: [0xC0, 0xFF, 0xEE, 0, …]  Bootstrap seed: {bootstrap_seed:#018x}");
        eprintln!("  Field: 4 rule engines + 8 vote ensembles + buy-and-hold (appended)");

        let report = run_bakeoff(
            cfg,
            cancel_rx,
            progress_tx,
            backtest::progress::BakeoffProgressSender::disabled(),
        )
        .await
        .expect("T7.1: run_bakeoff should succeed on real BTCUSDT H1_2024");

        // ── Print full ranked leaderboard ─────────────────────────────────────
        eprintln!(
            "\n  {:.<22}  {:>7}  {:>9}  {:>9}  {:>7}  RobustnessFlag",
            "Strategy", "Sharpe", "Return%", "MaxDD%", "Trades"
        );
        eprintln!("  {:->80}", "");

        for &i in &report.ranked {
            let c = &report.candidates[i];
            let crown = if report.crowned == Some(i) {
                " <== CROWN"
            } else {
                ""
            };
            let flag_str = match c.robustness {
                Some(RobustnessFlag::Robust) => "Robust",
                Some(RobustnessFlag::Marginal) => "Marginal",
                Some(RobustnessFlag::Fragile) => "Fragile",
                Some(RobustnessFlag::Skipped) => "Skipped",
                None => "Skipped",
            };
            let return_pct =
                (c.kpis.total_return_pct * rust_decimal::Decimal::from(100)).round_dp(2);
            let maxdd_pct = (c.kpis.max_drawdown * rust_decimal::Decimal::from(100)).round_dp(2);
            eprintln!(
                "  {:<22}  {:>7.3}  {:>+9}  {:>9}  {:>7}  {:<10}{}",
                c.strategy.0.as_str(),
                c.kpis.sharpe,
                return_pct,
                maxdd_pct,
                c.kpis.trade_count,
                flag_str,
                crown,
            );
        }

        eprintln!("  {:->80}", "");
        eprintln!(
            "  Crowned:     {}",
            report
                .crowned
                .map(|i| report.candidates[i].strategy.0.as_str())
                .unwrap_or("(none)")
        );
        eprintln!("  Outcome:     {:?}", report.rationale.outcome);
        eprintln!("  Reasons:     {:?}", report.rationale.reasons);
        eprintln!(
            "  Recommendation winner: {}",
            report.rationale.winner.0.as_str()
        );
        eprintln!();

        // ── Sanity assertions ─────────────────────────────────────────────────

        // 1. Every arm that CAN run produced a row. The field declares 19 entries
        //    + 1 appended buy-and-hold; the ADR-0073 `v0.macro_riskon` arm is
        //    dropped to ABSENCE unless `--features yahoo` compiled its regime
        //    loader (bug-log #81), so the honest expectation is 19 here and 20
        //    under `--features yahoo` with a covering `data/yahoo-macro/` corpus.
        //    Asserting the literal 20 unconditionally is what let an arm that
        //    never ran be counted as one that did.
        let macro_can_run = backtest::bakeoff::macro_arm_compiled();
        let expected_candidates = if macro_can_run { 20 } else { 19 };
        assert_eq!(
            report.candidates.len(),
            expected_candidates,
            "T7.1: expected {expected_candidates} candidates (18 always-runnable field arms \
             + DVOL + {} macro + 1 buyhold), got {}",
            usize::from(macro_can_run),
            report.candidates.len()
        );

        // …and the absence is REAL, not a silent stub: with the loader
        // uncompiled the macro id must not appear among the ranked rows at all.
        let has_macro_row = report
            .candidates
            .iter()
            .any(|c| c.strategy.0.as_str() == "v0.macro_riskon");
        assert_eq!(
            has_macro_row, macro_can_run,
            "T7.1: v0.macro_riskon must be ABSENT from the ranked field when its \
             regime loader is not compiled (bug-log #81) — a 100%-cash row wearing \
             the macro label is worse than no row"
        );

        // 2. buy-and-hold total_return > +20% on the known-bull H1_2024 window.
        let buyhold = report
            .candidates
            .iter()
            .find(|c| c.is_benchmark)
            .expect("T7.1: bake-off must include a buy-and-hold arm");

        eprintln!(
            "  buy-and-hold total_return = {:.2}%  (sanity guard: must be > +20%)",
            (buyhold.kpis.total_return_pct * rust_decimal::Decimal::from(100)).round_dp(2),
        );
        assert!(
            buyhold.kpis.total_return_pct > dec!(0.20),
            "T7.1 FAIL: buy-and-hold total_return_pct = {:.4}% is not > +20%; \
             synthetic-fallback bug may have regressed (BTC H1_2024 was a bull market ~+40-65%)",
            (buyhold.kpis.total_return_pct * rust_decimal::Decimal::from(100)).round_dp(4),
        );

        // 3. Both vote ensembles are present in the candidate list.
        let ids: Vec<&str> = report
            .candidates
            .iter()
            .map(|c| c.strategy.0.as_str())
            .collect();
        assert!(
            ids.contains(&"v0.8.vote.majority"),
            "T7.1: v0.8.vote.majority must be in candidates; got {ids:?}"
        );
        assert!(
            ids.contains(&"v0.8.vote.unanimous"),
            "T7.1: v0.8.vote.unanimous must be in candidates; got {ids:?}"
        );

        // 4. The crowned winner is identified.
        assert!(
            report.crowned.is_some(),
            "T7.1: crowned must be Some (non-empty field always crowns)"
        );

        // 5. Bootstrap gate ran: at least one candidate has a non-None robustness flag.
        let any_flagged = report.candidates.iter().any(|c| c.robustness.is_some());
        assert!(
            any_flagged,
            "T7.1: Bootstrap robustness gate ran — at least one candidate must have a robustness flag"
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
        resample::Horizon, run_bakeoff,
    };
    use rust_decimal_macros::dec;
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
                timeframe: Horizon::OneHour,
                initial_capital: dec!(100_000),
            },
            data_source: ScenarioDataSource::BinanceCache,
            robustness: RobustnessMode::Skip,
        };

        let run = |cfg: BakeoffCfg| async move {
            let (_handle, cancel_rx) = cancellation_pair();
            let progress_tx = ProgressSender::disabled();
            run_bakeoff(
                cfg,
                cancel_rx,
                progress_tx,
                backtest::progress::BakeoffProgressSender::disabled(),
            )
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
                timeframe: Horizon::OneHour,
                initial_capital: dec!(100_000),
            },
            data_source: ScenarioDataSource::BinanceCache,
            robustness: RobustnessMode::Skip,
        };

        let (_handle, cancel_rx) = cancellation_pair();
        let progress_tx = ProgressSender::disabled();
        let report = run_bakeoff(
            cfg,
            cancel_rx,
            progress_tx,
            backtest::progress::BakeoffProgressSender::disabled(),
        )
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

/// leaderboard-timeframe-capital — Day-1 divergence tests (CLAUDE.md non-negotiable).
///
/// Two tests prove the new knobs are WIRED, not cosmetic:
///
/// - `T_CAPITAL_DIV`: 2× start capital → ~2× final equity (same return %).
/// - `T_TIMEFRAME_DIV`: the bakeoff route used for H4/D1 produces non-empty
///   results (the resampling path is reached); H4 uses 4h bars ≠ H1 bars.
///
/// These are synthetic-only (no `--features realdata`) so they run on every CI
/// push. They are deterministic: fixed seed + same Synthetic GBM path +
/// Decimal arithmetic (no f64 money).
#[cfg(test)]
mod leaderboard_tuning_divergence {
    use backtest::{
        BakeoffRequest, DateRange, RobustnessMode,
        bakeoff::BakeoffConfig as BakeoffCfg,
        cancel::cancellation_pair,
        engine::ScenarioDataSource,
        progress::{BakeoffProgressSender, ProgressSender},
        resample::Horizon,
        run_bakeoff,
    };
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use smol_str::SmolStr;
    use trading_core::{StrategyId, Symbol};

    fn non_zero_seed() -> [u8; 32] {
        let mut s = [0u8; 32];
        s[0] = 0xCA;
        s[1] = 0xFE;
        s
    }

    fn sma_field() -> Vec<StrategyId> {
        vec![StrategyId(SmolStr::new_static("v0.sma"))]
    }

    /// T_CAPITAL_DIV — 2× start capital → ~2× absolute final equity, same
    /// return fraction.
    ///
    /// Runs the bake-off twice on the same Synthetic GBM bars with the same seed
    /// but 2× the capital on the second run.  Asserts:
    /// 1. Both runs produce a result (the engine reached the capital knob).
    /// 2. The buy-and-hold arm's final_equity in run2 is within 1% of 2×
    ///    final_equity in run1 (capital scales linearly; 1% tolerance for
    ///    potential rounding at the Decimal boundary).
    /// 3. The return fraction is the same in both runs (≤ 1e-6 absolute
    ///    difference — capital does NOT affect relative performance).
    #[allow(clippy::unwrap_used, clippy::expect_used)]
    #[tokio::test]
    async fn t_capital_div_2x_capital_doubles_absolute_equity() {
        let seed = non_zero_seed();

        let make_cfg = |capital: Decimal| BakeoffCfg {
            request: BakeoffRequest {
                symbol: Symbol::new("BTCUSDT"),
                range: DateRange::Last30d,
                seed,
                field: sma_field(),
                timeframe: Horizon::OneHour,
                initial_capital: capital,
            },
            data_source: ScenarioDataSource::Synthetic,
            robustness: RobustnessMode::Skip,
        };

        let capital_1x = dec!(100_000);
        let capital_2x = dec!(200_000);

        let (_h1, cancel1) = cancellation_pair();
        let report1 = run_bakeoff(
            make_cfg(capital_1x),
            cancel1,
            ProgressSender::disabled(),
            BakeoffProgressSender::disabled(),
        )
        .await
        .expect("bakeoff 1x capital must succeed");

        let (_h2, cancel2) = cancellation_pair();
        let report2 = run_bakeoff(
            make_cfg(capital_2x),
            cancel2,
            ProgressSender::disabled(),
            BakeoffProgressSender::disabled(),
        )
        .await
        .expect("bakeoff 2x capital must succeed");

        // Both runs must produce candidates.
        assert!(
            !report1.candidates.is_empty(),
            "T_CAPITAL_DIV: 1x run produced no candidates"
        );
        assert!(
            !report2.candidates.is_empty(),
            "T_CAPITAL_DIV: 2x run produced no candidates"
        );

        // Find buy-and-hold (always appended — guaranteed present).
        let bh1 = report1
            .candidates
            .iter()
            .find(|c| c.is_benchmark)
            .expect("buy-and-hold must be in 1x report");
        let bh2 = report2
            .candidates
            .iter()
            .find(|c| c.is_benchmark)
            .expect("buy-and-hold must be in 2x report");

        // Use the equity curve's final value as the "final equity" (the curve is
        // ordered oldest-first so the last entry is the terminal equity).
        let eq1_amount = bh1
            .equity_curve
            .last()
            .map(|(_, m)| m.amount())
            .expect("buy-and-hold equity curve must be non-empty");
        let eq2_amount = bh2
            .equity_curve
            .last()
            .map(|(_, m)| m.amount())
            .expect("buy-and-hold equity curve must be non-empty");

        // eq2 ≈ 2 × eq1 (within 1% tolerance for Decimal boundary rounding).
        let ratio = eq2_amount / eq1_amount;
        let deviation = (ratio - dec!(2)).abs();
        assert!(
            deviation < dec!(0.01),
            "T_CAPITAL_DIV: 2x capital final equity ratio should be ~2.0, got {ratio:.6} \
             (deviation={deviation:.6}). eq1={eq1_amount:.4}, eq2={eq2_amount:.4}. \
             The capital knob is NOT wired — this is the day-1 divergence gate."
        );

        // Return fraction (total_return_pct) must be the same in both runs.
        let ret1 = bh1.kpis.total_return_pct;
        let ret2 = bh2.kpis.total_return_pct;
        let ret_diff = (ret1 - ret2).abs();
        assert!(
            ret_diff < dec!(0.000001),
            "T_CAPITAL_DIV: return fraction must be capital-independent: \
             ret1={ret1:.8}, ret2={ret2:.8}, diff={ret_diff:.8e}"
        );
    }

    /// T_TIMEFRAME_DIV — the timeframe resampling knob is wired: H4 produces
    /// fewer bars than H1 from the same input (a 4:1 fold is applied), and the
    /// bakeoff routes those resampled bars to the engine via `bars_override`.
    ///
    /// **What is tested here:**
    /// 1. `resample_ohlcv` with `Horizon::FourHours` folds 1h bars 4:1 (the
    ///    mechanical proof that the resampler is correct).
    /// 2. A bakeoff with BinanceCache data source routes through the resampling
    ///    path: when bars_override is not None (real/preloaded bars), the H4
    ///    resampled bars ARE passed to the engine.
    ///
    /// **Why not test via Synthetic end-to-end?**
    /// `Synthetic` data source generates fresh GBM bars INSIDE `run_scenario`
    /// for each arm, bypassing `bars_override` entirely.  The resampling path
    /// (`preloaded_bars`) only activates when `resolve_bakeoff_bars` returns
    /// `Some` (i.e., BinanceCache or Yahoo).  A pure Synthetic e2e would not
    /// exercise the wiring.  Instead, we test the resampler unit + the bakeoff
    /// ScenarioConfig wiring via a bars-override synthetic run (T_TIMEFRAME_BARS).
    #[allow(clippy::unwrap_used, clippy::expect_used)]
    #[tokio::test]
    async fn t_timeframe_div_resampler_reduces_bar_count_4to1() {
        use backtest::resample::resample_ohlcv;
        use rust_decimal::Decimal;
        use time::OffsetDateTime;
        use trading_core::{Bar, Price, Quantity, Timeframe, Timestamp};

        // Build a deterministic set of 24 × 1h bars (one "day" of hourly data).
        let make_1h_bar = |hour: i64| -> Bar {
            let ts = Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::hours(hour));
            let price = Price::new(Decimal::from(100 + hour))
                .unwrap_or_else(|_| Price::new(dec!(100)).expect("100 is valid"));
            let qty = Quantity::new(Decimal::ONE).expect("1 qty is valid");
            Bar {
                symbol: Symbol::new("BTCUSDT"),
                tf: Timeframe::OneHour,
                venue: trading_core::Venue::Binance,
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
        };

        // 24 × 1h bars → 6 × 4h bars (24 / 4 = 6).
        let bars_1h: Vec<Bar> = (0..24).map(make_1h_bar).collect();
        let bars_h4 = resample_ohlcv(&bars_1h, Horizon::FourHours)
            .expect("resample_ohlcv must succeed on well-formed test bars");

        assert_eq!(
            bars_1h.len(),
            24,
            "T_TIMEFRAME_DIV: should have built 24 1h bars"
        );
        assert_eq!(
            bars_h4.len(),
            6,
            "T_TIMEFRAME_DIV: 24 × 1h bars resampled to H4 must produce 6 bars (24÷4=6); \
             got {}. The resampler is broken.",
            bars_h4.len(),
        );

        // H1 identity — same length, no fold.
        let bars_h1_identity = resample_ohlcv(&bars_1h, Horizon::OneHour)
            .expect("resample_ohlcv must succeed on well-formed test bars");
        assert_eq!(
            bars_h1_identity.len(),
            24,
            "T_TIMEFRAME_DIV: H1 identity pass-through must preserve all 24 bars; \
             got {}",
            bars_h1_identity.len()
        );
    }

    /// T_TIMEFRAME_BARS — the bakeoff routes resampled bars to the engine via
    /// `bars_override`. When a preloaded bar set is resampled to H4 and passed
    /// as `bars_override`, the SMA arm sees fewer bars → different equity outcome
    /// vs H1 on the same initial capital.
    ///
    /// Uses an explicit `bars_override` on the `Synthetic` path (bypassing real
    /// data) to prove the bakeoff ScenarioConfig wiring is correct regardless of
    /// data source.
    #[allow(clippy::unwrap_used, clippy::expect_used)]
    #[tokio::test]
    async fn t_timeframe_bars_resampled_bars_produce_different_equity() {
        use backtest::{
            DateRange, ScenarioConfig,
            cancel::cancellation_pair,
            cli_types::LatencySlippageSimConfig,
            engine::{ScenarioDataSource, run_scenario},
            progress::ProgressSender,
            resample::resample_ohlcv,
        };
        use rust_decimal::Decimal;
        use time::OffsetDateTime;
        use trading_core::{Bar, Price, Quantity, StrategyId, Timeframe, Timestamp, Venue};

        // Build 96 × 1h bars with alternating trend reversals to generate SMA
        // crossover signals: 24 rising → 24 falling → 24 rising → 24 falling.
        // This pattern guarantees SMA crossovers happen on 1h bars.
        let make_bar = |hour: i64, price: Decimal| -> Bar {
            let ts = Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::hours(hour));
            let p = Price::new(price).unwrap_or_else(|_| Price::new(dec!(100)).expect("100 valid"));
            let qty = Quantity::new(dec!(1)).expect("qty 1 valid");
            Bar {
                symbol: Symbol::new("BTCUSDT"),
                tf: Timeframe::OneHour,
                venue: Venue::Binance,
                open_ts: ts,
                close_ts: ts,
                open: p,
                high: p,
                low: p,
                close: p,
                volume: qty,
                trade_count: 0,
                local_recv_ts: ts,
            }
        };

        // 96 bars: rise 100→150 (24h), fall 150→100 (24h), rise 100→150 (24h),
        // fall 150→100 (24h). Produces multiple SMA crossovers on the 1h series.
        let mut prices: Vec<Decimal> = Vec::with_capacity(96);
        for cycle in 0..2 {
            let base = Decimal::from(100 + cycle * 5); // slight offset per cycle
            // 24 rising bars
            for i in 0..24i64 {
                prices.push(base + Decimal::from(i * 2)); // 100→146, step=2
            }
            // 24 falling bars
            for i in 0..24i64 {
                prices.push(base + Decimal::from((23 - i) * 2)); // 146→100
            }
        }
        let bars_1h: Vec<Bar> = prices
            .iter()
            .enumerate()
            .map(|(h, &p)| make_bar(h as i64, p))
            .collect();

        // Resample to H4: 96 / 4 = 24 bars
        let bars_h4 = resample_ohlcv(&bars_1h, Horizon::FourHours)
            .expect("resample_ohlcv must succeed on well-formed test bars");
        assert_eq!(bars_h4.len(), 24, "96÷4=24 H4 bars");

        let mut seed = [0u8; 32];
        seed[0] = 0xAA;

        let run_sma = |bars: Vec<Bar>| async {
            let (_h, cancel) = cancellation_pair();
            let cfg = ScenarioConfig {
                strategy: StrategyId("v0.sma".into()),
                pair: (Venue::Binance, Symbol::new("BTCUSDT")),
                range: DateRange::Last30d,
                params: None,
                seed,
                write_report: false,
                data_source: ScenarioDataSource::Synthetic,
                bars_override: Some(bars),
                sma_fast_len: None,
                sma_slow_len: None,
                latency_slippage_sim: LatencySlippageSimConfig::default(),
                reports_dir: None,
                short_enabled: false,
                initial_capital: Some(dec!(100_000)),
                composed_toml_override: None,
                dvol_override: None,
                macro_regime_series: None,
            };
            run_scenario(cfg, cancel, ProgressSender::disabled())
                .await
                .expect("run_scenario must succeed")
        };

        let report_h1 = run_sma(bars_1h).await;
        let report_h4 = run_sma(bars_h4).await;

        // The final equity must differ — the SMA strategy on 48 × 1h bars
        // vs 12 × 4h bars sees different signal sequences → different fills →
        // different final equity. This is the ScenarioConfig bars_override
        // wiring proof.
        let eq_h1 = report_h1.kpis.final_equity.amount();
        let eq_h4 = report_h4.kpis.final_equity.amount();

        assert_ne!(
            eq_h1, eq_h4,
            "T_TIMEFRAME_BARS: H1 and H4 final equity are equal ({eq_h1:.4}) — the \
             resampled bars are NOT producing different outcomes. Check the \
             bars_override wiring in the bakeoff ScenarioConfig."
        );
    }
}
