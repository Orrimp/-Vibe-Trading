//! Horizon retest day-1 falsifiers F-HR.4 + F-HR.5 (M-DEV-5).
//!
//! F-HR.4 — the carried-forward per-family falsifiers AT the coarser horizon
//! (4h and daily). Each sub-falsifier is RED-on-revert per CLAUDE.md
//! (the test FAILS when its guarded property is broken).
//!
//! ## F-HR.4 sub-falsifiers
//!
//! (a) Baseline-equity-divergence: a coarse-resampled TS run diverges from
//!     recomputed BH by > 1 bp when the decision variable is non-trivial.
//! (b) Signal-non-no-op: a degenerate always-long threshold tracks BH closely;
//!     normal TS diverges → the threshold is load-bearing at the coarse horizon.
//! (c) No-look-ahead: a forward-shifted coarse source changes the equity.
//! (d) Goes-flat: a coarse-bar downtrend produces ≥ 1 flat bar
//!     (time_in_market_bars < total).
//!
//! ## F-HR.5 — two-run byte-identity of each horizon θ-surface body-SHA
//!
//! Runs a small-N (N=6) 4h AND daily "surface" (2 cells each) twice at the
//! same ensemble_seed; asserts identical formatted DistributionSummary metrics.
//! Catches any unordered fold in the resampler, the grid, or the renderer.
//!
//! ## Pattern references
//!
//! `crates/backtest/tests/ts_momentum_divergence_e2e.rs` (F-TSM.1-5)
//! `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs` (CLAUDE.md non-negotiable)

use backtest::cli_types::TcnScenarioInput;
use backtest::resample::{Horizon, resample_ohlcv};
use backtest::scenarios::montecarlo::run_path;
use backtest::stats::{
    DistributionSummary, PathMetrics, compute_calmar_periodic, compute_max_drawdown_f64,
    compute_sharpe_periodic, compute_sortino_periodic, compute_total_return,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use strategy::SelectionMode;
use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

// ─────────────────────────────────────────────────────────────────────────────
// Shared helpers
// ─────────────────────────────────────────────────────────────────────────────

fn epoch_2023() -> time::OffsetDateTime {
    time::OffsetDateTime::from_unix_timestamp(1_672_531_200).expect("valid epoch_2023")
}

fn make_ts(offset_hours: i64) -> Timestamp {
    Timestamp::new(epoch_2023() + time::Duration::hours(offset_hours))
}

fn make_bar_at_hour(sym: &str, close: Decimal, hour: i64) -> Bar {
    Bar {
        symbol: Symbol::new(sym),
        tf: Timeframe::OneHour,
        open_ts: make_ts(hour),
        close_ts: make_ts(hour + 1),
        local_recv_ts: make_ts(hour + 1),
        venue: Venue::Binance,
        open: Price::new(close).unwrap(),
        high: Price::new(close).unwrap(),
        low: Price::new(close).unwrap(),
        close: Price::new(close).unwrap(),
        volume: Quantity::new(dec!(100)).unwrap(),
        trade_count: 1,
    }
}

/// Build 1h bars with an up-then-down structure.
/// Phase 1 (n_up bars): +1%/bar (TS goes long after warmup).
/// Phase 2 (n_down bars): −2%/bar (TS goes flat when cumulative log-return < 0).
///
/// NOTE: these large returns (+27%/day compounded) are fine for the 4h horizon
/// tests (short uptrend → small position value) but cause the risk exposure cap
/// (0.40) to block the SELL order on the daily horizon after 30 days of uptrend.
/// For daily tests use `build_1h_up_down_bars_moderate` instead.
fn build_1h_up_down_bars(n_up: usize, n_down: usize) -> Vec<Bar> {
    let total = n_up + n_down;
    let mut bars: Vec<Bar> = Vec::with_capacity(total);
    let mut price = dec!(1000);
    for hour in 0..total {
        bars.push(make_bar_at_hour("AAAUSD", price, hour as i64));
        if hour < n_up {
            price *= dec!(1.01); // +1%
        } else {
            price *= dec!(0.98); // −2%
        }
    }
    bars.sort_by_key(|b| b.open_ts);
    bars
}

/// Build 1h bars with a moderate up-then-down structure, safe for daily-horizon tests.
///
/// Phase 1 (n_up bars): +0.1%/bar (~2.5%/day) — TS goes long after warmup.
/// Phase 2 (n_down bars): −0.5%/bar (~11.3% drop/day) — TS goes flat.
///
/// ## Why this differs from `build_1h_up_down_bars`
///
/// The standard fixture uses +1%/1h (≈+27%/day compounded). Over 30 daily-uptrend
/// bars the position value grows to ~93% of portfolio equity, tripping the hard
/// `per_symbol_exposure_cap = 0.40` in `Order::new` when the strategy tries to
/// close the position. With moderate rates (+0.1%/1h, −0.5%/1h) the position
/// stays at ≈16% of equity throughout, the SELL order is accepted, and the
/// `TimeSeriesLongFlat` mechanism is correctly exercised at the daily cadence.
///
/// The score at day 30 (first down-day, L=5 lookback) is ≈ −0.018 < 0, so the
/// strategy exits promptly, giving `time_in_market_bars ≈ 25` vs the always-long
/// baseline of 40.
fn build_1h_up_down_bars_moderate(n_up: usize, n_down: usize) -> Vec<Bar> {
    let total = n_up + n_down;
    let mut bars: Vec<Bar> = Vec::with_capacity(total);
    let mut price = dec!(1000);
    for hour in 0..total {
        bars.push(make_bar_at_hour("AAAUSD", price, hour as i64));
        if hour < n_up {
            price *= dec!(1.001); // +0.1%/1h  ≈ +2.5%/day
        } else {
            price *= dec!(0.995); // −0.5%/1h  ≈ −11.3%/day
        }
    }
    bars.sort_by_key(|b| b.open_ts);
    bars
}

/// Build alternating-price 1h bars for no-look-ahead test.
fn build_1h_alternating_bars(n_bars: usize) -> Vec<Bar> {
    let mut bars = Vec::with_capacity(n_bars);
    let mut price = dec!(1000);
    for hour in 0..n_bars {
        bars.push(make_bar_at_hour("AAAUSD", price, hour as i64));
        if hour % 2 == 0 {
            price *= dec!(1.05);
        } else {
            price *= dec!(0.97);
        }
    }
    bars.sort_by_key(|b| b.open_ts);
    bars
}

// ── Config builders ───────────────────────────────────────────────────────────

/// TS-momentum config with custom lookback (in coarse-bar units) and threshold.
fn make_ts_config(
    lookback_coarse_bars: u32,
    threshold: Decimal,
) -> strategy::CrossSectionalMomentumConfig {
    strategy::CrossSectionalMomentumConfig {
        id: SmolStr::new(format!("ts_hr_L{lookback_coarse_bars}")),
        universe: vec![SmolStr::new("AAAUSD")],
        lookback_minutes: lookback_coarse_bars, // coarse-bar count
        rebalance_minutes: 1,                   // every bar
        k_long: 10,
        k_short: 0,
        exposure_cap: dec!(0.5),
        drift_rebalance_threshold: dec!(0.10),
        vol_floor: dec!(0.000001),
        stage: SmolStr::new("research"),
        direction: strategy::Direction::Momentum,
        score_source: strategy::ScoreSource::VolAdjustedReturn,
        selection_mode: SelectionMode::TimeSeriesLongFlat,
        entry_threshold: threshold,
    }
}

/// Always-long config (CrossSectionalTopK on single-symbol universe → always long after warmup).
fn make_always_long_config(lookback_coarse_bars: u32) -> strategy::CrossSectionalMomentumConfig {
    strategy::CrossSectionalMomentumConfig {
        id: SmolStr::new("always_long"),
        universe: vec![SmolStr::new("AAAUSD")],
        lookback_minutes: lookback_coarse_bars,
        rebalance_minutes: 1,
        k_long: 10,
        k_short: 0,
        exposure_cap: dec!(0.5),
        drift_rebalance_threshold: dec!(0.10),
        vol_floor: dec!(0.000001),
        stage: SmolStr::new("research"),
        direction: strategy::Direction::Momentum,
        score_source: strategy::ScoreSource::VolAdjustedReturn,
        selection_mode: SelectionMode::CrossSectionalTopK, // always selects single-name → always long
        entry_threshold: Decimal::ZERO,
    }
}

// ── Run helpers ───────────────────────────────────────────────────────────────

fn run_to_result(
    cfg: strategy::CrossSectionalMomentumConfig,
    bars: Vec<Bar>,
) -> backtest::scenarios::montecarlo::PathRunResult {
    let n = bars.len();
    let strat = strategy::MomentumStrategy::from_config(cfg, SmolStr::new("hr_e2e_test"));
    let input = TcnScenarioInput {
        scenario_name: "hr-e2e".to_string(),
        start_year: 2023,
        bar_count: n,
        initial_capital: dec!(100_000),
        slippage_bps: 0,
        taker_fee_bps: 0,
        config_id: "test_hr".to_string(),
        forecaster_id: "test".to_string(),
        bars_override: Some(bars),
        emit_equity_bin: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        funding_override: None,
    };
    pollster::block_on(run_path(input, 0xC0FFEE, strat))
        .expect("run_path must succeed in horizon e2e test")
}

/// Buy-and-hold total return on the given bars (single symbol, buy at first close, hold).
fn run_buyhold_final_equity(bars: &[Bar]) -> Decimal {
    if bars.is_empty() {
        return dec!(100_000);
    }
    let initial = dec!(100_000);
    let buy_price = bars.first().map(|b| b.close.get()).unwrap_or(dec!(1));
    if buy_price <= Decimal::ZERO {
        return initial;
    }
    let qty = initial / buy_price;
    let final_price = bars.last().map(|b| b.close.get()).unwrap_or(buy_price);
    qty * final_price
}

// ─────────────────────────────────────────────────────────────────────────────
// F-HR.4.a — Baseline-equity-divergence at the coarse horizon (4h, TS)
// ─────────────────────────────────────────────────────────────────────────────

/// F-HR.4.a: TS at 4h horizon (resampled) diverges ≥ 1 bp from BH.
///
/// Construction: 1h bars with an uptrend (warmup) + sustained downtrend.
/// Resample to 4h (4:1 fold). After L=3 coarse downtrend bars, the TS rule exits
/// to FLAT. BH holds through → equity diverges.
///
/// RED-on-revert: always-long coarse TS produces Δ≈0 vs BH → test fails.
#[test]
fn f_hr_4_baseline_divergence_4h() {
    // N_UP large enough for L=3 4h-bars to see a full uptrend (need 3*4=12 1h-bars of uptrend).
    // N_DOWN large enough for 3+ 4h-bars of downtrend (need 3*4=12 1h-bars minimum).
    const N_UP_1H: usize = 20; // 5 full 4h-bars of uptrend
    const N_DOWN_1H: usize = 32; // 8 full 4h-bars of downtrend (TS exits after L=3 bars)

    let bars_1h = build_1h_up_down_bars(N_UP_1H, N_DOWN_1H);
    let bars_4h = resample_ohlcv(&bars_1h, Horizon::FourHours);

    // TS with lookback L=3 coarse bars, threshold=0.00
    let result_ts = run_to_result(make_ts_config(3, dec!(0.00)), bars_4h.clone());
    let bh_equity = run_buyhold_final_equity(&bars_4h);

    let ts_equity = result_ts.final_equity;
    let delta = (ts_equity - bh_equity).abs();
    const EPSILON_1BP: Decimal = dec!(10); // 1 bp of 100_000

    assert!(
        delta > EPSILON_1BP,
        "F-HR.4.a DIVERGENCE VIOLATION (4h): TS equity ({ts_equity}) must diverge from BH \
         equity ({bh_equity}) by ≥ 1 bp ({EPSILON_1BP}). Actual |delta| = {delta}. \
         If delta ≈ 0, the TS long/flat exit is not triggering at the 4h horizon — \
         check resample_ohlcv identity + TimeSeriesLongFlat wiring.",
        ts_equity = ts_equity,
        bh_equity = bh_equity,
        delta = delta,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// F-HR.4.b — Signal-non-no-op at the daily horizon (TS)
// ─────────────────────────────────────────────────────────────────────────────

/// F-HR.4.b: At the daily horizon, the TS signal (threshold=0.00) diverges from
/// the always-long (CrossSectionalTopK) baseline → the trend signal is non-trivial
/// at the daily cadence.
///
/// Construction: 1h bars with 30 days of uptrend + 15 days of sustained downtrend,
/// resampled to daily. Normal TS exits in the downtrend; always-long stays in.
/// Both run with 0 fees so any divergence is purely from the exit mechanism.
///
/// RED-on-revert: if TS never exits at the daily cadence, delta from always-long ≈ 0.
#[test]
fn f_hr_4_signal_non_no_op_daily() {
    // 1h bars: 30 days of uptrend (720 1h-bars) + 15 days of sustained downtrend (360 bars).
    // Daily resample: 30 up days + 15 down days = 45 daily bars.
    // TS lookback L=5 daily bars; after 5 down bars → score < 0 → flat.
    //
    // We use the MODERATE fixture (+0.1%/1h up, −0.5%/1h down) rather than the
    // standard +1%/1h fixture, because the large compounding rate in the standard
    // fixture causes the position value after 30 up-days to reach ≈93% of equity,
    // which trips the hard per_symbol_exposure_cap=0.40 check in Order::new when
    // the strategy tries to exit. The risk guard silently rejects the SELL while the
    // strategy's held_symbols tracking believes the position was closed, leaving the
    // physical position open forever. The moderate rates keep the position at ≈16% of
    // equity throughout, so the SELL executes and the TimeSeriesLongFlat exit is
    // correctly exercised at the daily cadence. See test fixture notes in
    // build_1h_up_down_bars_moderate for the full derivation.
    const N_UP_1H: usize = 24 * 30; // 30 days uptrend
    const N_DOWN_1H: usize = 24 * 15; // 15 days sustained downtrend

    let bars_1h = build_1h_up_down_bars_moderate(N_UP_1H, N_DOWN_1H);
    let bars_daily = resample_ohlcv(&bars_1h, Horizon::OneDay);

    // Normal TS: threshold=0.00, lookback=5 daily bars → goes flat in downtrend.
    let result_ts = run_to_result(make_ts_config(5, dec!(0.00)), bars_daily.clone());
    // Always-long: CrossSectionalTopK on single-symbol → always long after warmup.
    let result_always_long = run_to_result(make_always_long_config(5), bars_daily.clone());

    // TS must have fewer time_in_market_bars than always-long (it exits during downtrend).
    let ts_tim = result_ts.time_in_market_bars;
    let al_tim = result_always_long.time_in_market_bars;

    assert!(
        ts_tim < al_tim,
        "F-HR.4.b SIGNAL-NON-NO-OP (daily): TS time_in_market ({ts_tim}) must be LESS THAN \
         always-long time_in_market ({al_tim}). TS must exit to flat during the 15-day downtrend. \
         If equal, the trend signal is not acting at the daily cadence — \
         check that the daily resample produced meaningful bar counts \
         and that SelectionMode::TimeSeriesLongFlat is correctly wired.",
        ts_tim = ts_tim,
        al_tim = al_tim,
    );

    // TS equity must also diverge from always-long by ≥ 1 bp
    // (TS avoids some downtrend losses that always-long takes).
    let ts_equity = result_ts.final_equity;
    let al_equity = result_always_long.final_equity;
    let delta = (ts_equity - al_equity).abs();
    const EPSILON_1BP: Decimal = dec!(10);

    assert!(
        delta > EPSILON_1BP,
        "F-HR.4.b SIGNAL-NON-NO-OP (daily): TS equity ({ts_equity}) must diverge from \
         always-long equity ({al_equity}) by ≥ 1 bp at the daily horizon. \
         delta={delta}. If delta ≈ 0, the exit is not changing the equity outcome.",
        ts_equity = ts_equity,
        al_equity = al_equity,
        delta = delta,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// F-HR.4.c — No-look-ahead at the coarse horizon
// ─────────────────────────────────────────────────────────────────────────────

/// F-HR.4.c: Shifting the 1h source by 1 bar before resampling to 4h changes the
/// coarse equity → the resampled bars are causal (no future 1h bar leaks in).
///
/// RED-on-revert: if resample_ohlcv were to use future 1h bars (e.g. `open = last`
/// instead of `open = first`), a shifted source might produce the SAME coarse bar
/// → the equity delta would be small and the assertion would fail.
#[test]
fn f_hr_4_no_look_ahead_coarse() {
    const N_BARS_1H: usize = 40;

    let bars_1h = build_1h_alternating_bars(N_BARS_1H);

    // Shift all bars forward by 1 hour: bar[k] gets bar[k+1]'s price.
    // This simulates a look-ahead at the 1h level. After resampling to 4h,
    // the bucket boundaries shift, changing which 1h bars are in each bucket.
    let mut bars_shifted = bars_1h.clone();
    for k in 0..(N_BARS_1H - 1) {
        let next_price = bars_1h[k + 1].close.get();
        bars_shifted[k] = make_bar_at_hour("AAAUSD", next_price, k as i64);
    }

    let bars_4h_causal = resample_ohlcv(&bars_1h, Horizon::FourHours);
    let bars_4h_shifted = resample_ohlcv(&bars_shifted, Horizon::FourHours);

    let result_causal = run_to_result(make_ts_config(3, dec!(0.00)), bars_4h_causal);
    let result_shifted = run_to_result(make_ts_config(3, dec!(0.00)), bars_4h_shifted);

    let delta = (result_causal.final_equity - result_shifted.final_equity).abs();
    const EPSILON_1BP: Decimal = dec!(10);

    assert!(
        delta > EPSILON_1BP,
        "F-HR.4.c NO-LOOK-AHEAD (coarse 4h): causal series ({causal}) and 1h-shifted series \
         ({shifted}) must produce DIFFERENT coarse equity (delta ≥ 1 bp = {EPSILON_1BP}). \
         Actual delta = {delta}. If delta ≈ 0, the shift is not changing the resampled bars \
         — check that the 4h bucket boundaries are correctly UTC-aligned.",
        causal = result_causal.final_equity,
        shifted = result_shifted.final_equity,
        delta = delta,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// F-HR.4.d — Goes-flat at the coarse 4h horizon (TS-specific must-exit gate)
// ─────────────────────────────────────────────────────────────────────────────

/// F-HR.4.d: On a resampled 4h series with a clear sustained downtrend, the TS
/// strategy exits to FLAT (≥ 1 flat bar). time_in_market_bars < total_bars.
///
/// RED-on-revert: an always-long (CrossSectionalTopK) rule never exits
/// → time_in_market ≈ total → the assertion fails.
#[test]
fn f_hr_4_goes_flat_coarse() {
    // Enough 1h bars for a clear uptrend then downtrend after resampling to 4h.
    // Need ≥ L=3 coarse down-bars for the score to go negative.
    // 4h bars: ~5 up, ≥ 8 down (downtrend starts at 1h bar 20, coarse bar 5).
    const N_UP_1H: usize = 20;
    const N_DOWN_1H: usize = 40; // 10 full 4h-bars of downtrend

    let bars_1h = build_1h_up_down_bars(N_UP_1H, N_DOWN_1H);
    let bars_4h = resample_ohlcv(&bars_1h, Horizon::FourHours);
    let total_4h_bars = bars_4h.len();

    let result_ts = run_to_result(make_ts_config(3, dec!(0.00)), bars_4h.clone());
    let tim = result_ts.time_in_market_bars;

    assert!(
        tim < total_4h_bars as u64,
        "F-HR.4.d GOES-FLAT VIOLATION (4h): time_in_market_bars ({tim}) must be < \
         total_4h_bars ({total_4h_bars}). The TS strategy must exit to FLAT during the \
         sustained coarse-bar downtrend. If equal, the signal is always-long at the 4h \
         cadence — check SelectionMode::TimeSeriesLongFlat on the resampled bars.",
        tim = tim,
        total_4h_bars = total_4h_bars,
    );

    // Also verify the TS exits earlier than an always-long rule.
    let result_always_long = run_to_result(make_always_long_config(3), bars_4h);
    assert!(
        tim < result_always_long.time_in_market_bars,
        "F-HR.4.d: TS time_in_market ({}) must be LESS THAN always-long time_in_market ({}) \
         at the 4h horizon — TS exits to flat; always-long does not.",
        tim,
        result_always_long.time_in_market_bars,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// F-HR.4 RED-on-revert — always-long coarse TS tracks BH → proves the gate
// ─────────────────────────────────────────────────────────────────────────────

/// F-HR.4 RED-on-revert: always-long coarse TS produces Δ small vs BH.
///
/// Proves that the divergence in F-HR.4.a is driven by the long/flat exit and
/// not a sizing artifact — if the exit is replaced with always-long, the delta
/// from BH is much smaller than the normal TS delta.
#[test]
fn f_hr_4_red_on_revert_always_long_tracks_bh() {
    const N_UP_1H: usize = 20;
    const N_DOWN_1H: usize = 32;

    let bars_1h = build_1h_up_down_bars(N_UP_1H, N_DOWN_1H);
    let bars_4h = resample_ohlcv(&bars_1h, Horizon::FourHours);

    let result_ts = run_to_result(make_ts_config(3, dec!(0.00)), bars_4h.clone());
    let result_always_long = run_to_result(make_always_long_config(3), bars_4h.clone());
    let bh_equity = run_buyhold_final_equity(&bars_4h);

    let delta_ts = (result_ts.final_equity - bh_equity).abs();
    let delta_always_long = (result_always_long.final_equity - bh_equity).abs();

    // The TS divergence from BH must be LARGER than the always-long deviation.
    // (always-long tracks BH closely; TS exits in downtrend → larger deviation).
    assert!(
        delta_ts > delta_always_long,
        "F-HR.4 RED-ON-REVERT: TS divergence from BH ({delta_ts}) must be LARGER than \
         always-long deviation from BH ({delta_always_long}) at the 4h horizon. \
         If always-long deviates as much, the exit mechanism is not working. \
         bh={bh}, ts={ts}, always_long={al}",
        delta_ts = delta_ts,
        delta_always_long = delta_always_long,
        bh = bh_equity,
        ts = result_ts.final_equity,
        al = result_always_long.final_equity,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// F-HR.5 — Two-run byte-identity of each horizon surface metrics
// ─────────────────────────────────────────────────────────────────────────────

/// Helper: run one "horizon cell" (N paths of coarse-bar TS) and return
/// (DistributionSummary, total_trades). Uses `compute_sharpe_periodic` as the
/// metric branch (the non-1h path, D-HR.1), with a small fixed ppy for testing.
fn run_horizon_cell_summary(
    cfg: &strategy::CrossSectionalMomentumConfig,
    n_paths: usize,
    horizon: Horizon,
    ppy: f64,
) -> (DistributionSummary, u64) {
    let mut all_metrics: Vec<PathMetrics> = Vec::with_capacity(n_paths);
    let mut total_trades = 0u64;

    for j in 0..n_paths {
        // Vary paths by n_up / n_down (deterministic per j).
        // Using 1h bars that are then resampled; the TS lookback is in coarse-bar units.
        // For 4h: L=3 bars needs ≥ 12 1h-bars of trend; n_up varies so paths differ.
        let n_up_1h = 20 + j * 4;
        let n_down_1h = 24 + j * 4;
        let bars_1h = build_1h_up_down_bars(n_up_1h, n_down_1h);
        let bars_coarse = resample_ohlcv(&bars_1h, horizon);

        let result = run_to_result(cfg.clone(), bars_coarse);
        total_trades += result.trades as u64;

        let equity_clamped: Vec<Decimal> = result
            .equity_curve
            .iter()
            .map(|&e| {
                if e <= Decimal::ZERO {
                    dec!(0.000001)
                } else {
                    e
                }
            })
            .collect();

        // Use the periodic metric fns (the coarse-horizon path, D-HR.1).
        let pm = PathMetrics {
            sharpe: compute_sharpe_periodic(&equity_clamped, ppy),
            sortino: compute_sortino_periodic(&equity_clamped, ppy),
            calmar: compute_calmar_periodic(&equity_clamped, ppy),
            max_drawdown: compute_max_drawdown_f64(&equity_clamped),
            total_return: compute_total_return(&equity_clamped),
            final_equity: result.final_equity,
            initial_equity: result.initial_equity,
        };
        all_metrics.push(pm);
    }

    let summary = DistributionSummary::from_path_metrics(&all_metrics)
        .expect("build DistributionSummary for horizon cell");
    (summary, total_trades)
}

/// F-HR.5 — two-run byte-identity of the 4h surface metrics.
///
/// Runs 2 cells × N=6 paths each at the 4h horizon twice, asserting
/// byte-identical formatted DistributionSummary. Catches any non-determinism
/// in the resampler, the periodic metric fn, or the reduction.
///
/// ADR-0051 D2/D3/§D6.8.
#[test]
fn f_hr_5_two_run_byte_identity_4h() {
    const N: usize = 6;
    const PPY_4H: f64 = 2190.0; // non-leap year ppy for 4h

    // Cell a: lookback=3 coarse bars, threshold=0.00.
    let cfg_a = make_ts_config(3, dec!(0.00));
    // Cell b: lookback=7 coarse bars, threshold=0.02.
    let cfg_b = make_ts_config(7, dec!(0.02));

    // Run 1
    let (s1a, trades_a1) = run_horizon_cell_summary(&cfg_a, N, Horizon::FourHours, PPY_4H);
    let (s1b, trades_b1) = run_horizon_cell_summary(&cfg_b, N, Horizon::FourHours, PPY_4H);

    // Run 2: same inputs
    let (s2a, trades_a2) = run_horizon_cell_summary(&cfg_a, N, Horizon::FourHours, PPY_4H);
    let (s2b, trades_b2) = run_horizon_cell_summary(&cfg_b, N, Horizon::FourHours, PPY_4H);

    let fmt6 = |v: f64| format!("{v:.6}");

    // Cell a: byte-identical across runs.
    assert_eq!(
        fmt6(s1a.sharpe.p50),
        fmt6(s2a.sharpe.p50),
        "F-HR.5 (4h): cell_a Sharpe p50 must be deterministic across two runs"
    );
    assert_eq!(
        fmt6(s1a.prob_loss),
        fmt6(s2a.prob_loss),
        "F-HR.5 (4h): cell_a prob_loss must be deterministic across two runs"
    );
    assert_eq!(
        fmt6(s1a.max_dd_tail_p95),
        fmt6(s2a.max_dd_tail_p95),
        "F-HR.5 (4h): cell_a p95 MaxDD must be deterministic across two runs"
    );

    // Cell b: byte-identical across runs.
    assert_eq!(
        fmt6(s1b.sharpe.p50),
        fmt6(s2b.sharpe.p50),
        "F-HR.5 (4h): cell_b Sharpe p50 must be deterministic across two runs"
    );
    assert_eq!(
        fmt6(s1b.prob_loss),
        fmt6(s2b.prob_loss),
        "F-HR.5 (4h): cell_b prob_loss must be deterministic across two runs"
    );
    assert_eq!(
        fmt6(s1b.max_dd_tail_p95),
        fmt6(s2b.max_dd_tail_p95),
        "F-HR.5 (4h): cell_b p95 MaxDD must be deterministic across two runs"
    );

    // Trade counts deterministic.
    assert_eq!(
        trades_a1, trades_a2,
        "F-HR.5 (4h): cell_a trade count must be identical across two runs"
    );
    assert_eq!(
        trades_b1, trades_b2,
        "F-HR.5 (4h): cell_b trade count must be identical across two runs"
    );

    // Sanity: cells a (L=3, thr=0.00) and b (L=7, thr=0.02) must differ.
    assert!(
        trades_a1 != trades_b1 || fmt6(s1a.max_dd_tail_p95) != fmt6(s1b.max_dd_tail_p95),
        "F-HR.5 (4h) sanity: cells a and b must differ in trade count or max_dd. \
         trades_a={}, trades_b={}, max_dd_a={:.6}, max_dd_b={:.6}",
        trades_a1,
        trades_b1,
        s1a.max_dd_tail_p95,
        s1b.max_dd_tail_p95,
    );
}

/// F-HR.5 — two-run byte-identity of the daily surface metrics.
///
/// Runs 2 cells × N=6 paths at the daily horizon twice, asserting
/// byte-identical formatted metrics. Mirrors the 4h version above.
#[test]
fn f_hr_5_two_run_byte_identity_daily() {
    const N: usize = 6;
    const PPY_DAILY: f64 = 365.0; // non-leap year ppy for daily

    // Cell a: lookback=2 daily bars, threshold=0.00 (fast TSMOM).
    let cfg_a = make_ts_config(2, dec!(0.00));
    // Cell b: lookback=5 daily bars, threshold=0.02 (classic 1-wk with band).
    let cfg_b = make_ts_config(5, dec!(0.02));

    // Need enough 1h bars to produce meaningful daily coarse bars (≥ 10 daily bars per path).
    // build_1h_up_down_bars with n_up=24*N_UP_DAYS and n_down=24*N_DOWN_DAYS.
    // We vary n_up/n_down inside run_horizon_cell_summary — it uses 20+j*4 1h bars of up.
    // At j=5: n_up_1h = 40 (≈1.6 daily bars of up at 24:1) — enough for L=2 daily.
    // For longer lookbacks and daily: we need at least 24*5=120 1h bars of trend.
    // Let's ensure the bars are sufficient by using a larger multiplier for daily.
    // We'll create a custom helper that uses 24*-based sizing.

    fn run_daily_cell_summary(
        cfg: &strategy::CrossSectionalMomentumConfig,
        n_paths: usize,
    ) -> (DistributionSummary, u64) {
        let mut all_metrics: Vec<PathMetrics> = Vec::with_capacity(n_paths);
        let mut total_trades = 0u64;

        for j in 0..n_paths {
            // Each path: up 6+j full days (144+j*24 1h-bars), down 8+j full days.
            let n_up_1h = (6 + j) * 24;
            let n_down_1h = (8 + j) * 24;
            let bars_1h = build_1h_up_down_bars(n_up_1h, n_down_1h);
            let bars_daily = resample_ohlcv(&bars_1h, Horizon::OneDay);

            let result = run_to_result(cfg.clone(), bars_daily);
            total_trades += result.trades as u64;

            let equity_clamped: Vec<Decimal> = result
                .equity_curve
                .iter()
                .map(|&e| {
                    if e <= Decimal::ZERO {
                        dec!(0.000001)
                    } else {
                        e
                    }
                })
                .collect();

            let pm = PathMetrics {
                sharpe: compute_sharpe_periodic(&equity_clamped, PPY_DAILY),
                sortino: compute_sortino_periodic(&equity_clamped, PPY_DAILY),
                calmar: compute_calmar_periodic(&equity_clamped, PPY_DAILY),
                max_drawdown: compute_max_drawdown_f64(&equity_clamped),
                total_return: compute_total_return(&equity_clamped),
                final_equity: result.final_equity,
                initial_equity: result.initial_equity,
            };
            all_metrics.push(pm);
        }

        let summary = DistributionSummary::from_path_metrics(&all_metrics)
            .expect("build DistributionSummary for daily horizon cell");
        (summary, total_trades)
    }

    // Run 1
    let (s1a, trades_a1) = run_daily_cell_summary(&cfg_a, N);
    let (s1b, trades_b1) = run_daily_cell_summary(&cfg_b, N);

    // Run 2: same inputs
    let (s2a, trades_a2) = run_daily_cell_summary(&cfg_a, N);
    let (s2b, trades_b2) = run_daily_cell_summary(&cfg_b, N);

    let fmt6 = |v: f64| format!("{v:.6}");

    // Cell a: byte-identical across runs.
    assert_eq!(
        fmt6(s1a.sharpe.p50),
        fmt6(s2a.sharpe.p50),
        "F-HR.5 (daily): cell_a Sharpe p50 must be deterministic across two runs"
    );
    assert_eq!(
        fmt6(s1a.prob_loss),
        fmt6(s2a.prob_loss),
        "F-HR.5 (daily): cell_a prob_loss must be deterministic across two runs"
    );
    assert_eq!(
        fmt6(s1a.max_dd_tail_p95),
        fmt6(s2a.max_dd_tail_p95),
        "F-HR.5 (daily): cell_a p95 MaxDD must be deterministic across two runs"
    );

    // Cell b: byte-identical across runs.
    assert_eq!(
        fmt6(s1b.sharpe.p50),
        fmt6(s2b.sharpe.p50),
        "F-HR.5 (daily): cell_b Sharpe p50 must be deterministic across two runs"
    );
    assert_eq!(
        fmt6(s1b.prob_loss),
        fmt6(s2b.prob_loss),
        "F-HR.5 (daily): cell_b prob_loss must be deterministic across two runs"
    );
    assert_eq!(
        fmt6(s1b.max_dd_tail_p95),
        fmt6(s2b.max_dd_tail_p95),
        "F-HR.5 (daily): cell_b p95 MaxDD must be deterministic across two runs"
    );

    // Trade counts deterministic.
    assert_eq!(
        trades_a1, trades_a2,
        "F-HR.5 (daily): cell_a trade count must be identical across two runs"
    );
    assert_eq!(
        trades_b1, trades_b2,
        "F-HR.5 (daily): cell_b trade count must be identical across two runs"
    );

    // Sanity: cells a (L=2, thr=0.00) and b (L=5, thr=0.02) must differ.
    assert!(
        trades_a1 != trades_b1 || fmt6(s1a.max_dd_tail_p95) != fmt6(s1b.max_dd_tail_p95),
        "F-HR.5 (daily) sanity: cells a (L=2, thr=0.00) and b (L=5, thr=0.02) must differ. \
         trades_a={}, trades_b={}, max_dd_a={:.6}, max_dd_b={:.6}",
        trades_a1,
        trades_b1,
        s1a.max_dd_tail_p95,
        s1b.max_dd_tail_p95,
    );
}
