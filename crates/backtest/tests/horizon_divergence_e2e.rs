//! Horizon retest day-1 falsifiers F-HR.4 + F-HR.5 (M-DEV-5).
//!
//! F-HR.4 — the carried-forward per-family falsifiers AT the coarser horizon
//! (4h and daily). Each sub-falsifier is RED-on-revert per CLAUDE.md
//! (the test FAILS when its guarded property is broken).
//!
//! ## F-HR.4 sub-falsifiers
//!
//! (a) Baseline-equity-divergence: a coarse-resampled TS run diverges by ≥ 1 bp
//!     from a **like-sized** always-long control (identical config except
//!     `selection_mode`), so a no-op long/flat exit fails. Repeated at the
//!     anchored 4/2 bps friction, where the fee path must also bite.
//! (b) Signal-non-no-op: a degenerate always-long threshold tracks BH closely;
//!     normal TS diverges → the threshold is load-bearing at the coarse horizon.
//! (c) No-look-ahead: **prefix-invariance** — `resample_ohlcv(&bars[..k])`
//!     equals the prefix of `resample_ohlcv(&bars)` on every COMPLETE bucket,
//!     compared field-by-field. Plus partial-bucket visibility.
//! (d) Goes-flat: a coarse-bar downtrend produces ≥ 1 flat bar
//!     (time_in_market_bars < total).
//!
//! ## F-HR.5 — two-run byte-identity of each horizon θ-surface REPORT BODY
//!
//! Renders the LOCKED `ts-4h` / `ts-daily` surfaces twice through the real
//! chain (`resample_ohlcv` → `run_path` → periodic metrics →
//! `DistributionSummary` → `classify_verdict` → `render_surface_report`) and
//! compares the hashed body — the exact substring the anchors SHA-256 — with a
//! negative control proving the two cadences render differently. The explicit
//! coverage boundary (what these do NOT exercise) is stated at that section.
//!
//! ## Review 1-18 (falsifier rebuild)
//!
//! Five of these were vacuous — see each test's own doc-comment for the revert
//! it now catches. (a) compared a 10 %-sized TS run against a 100 %-deployed
//! buy-and-hold, so the sizing gap alone met the gate; (c) substituted input
//! prices and asserted the outputs differed, which is true of ANY function that
//! reads its input; the two F-HR.5 tests asserted `f(x) == f(x)` on a
//! hand-built config with no LOCKED grid and no renderer. No test in this file
//! may claim coverage it lacks.
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
    run_to_result_with_fees(cfg, bars, 0, 0)
}

/// The anchored surfaces' friction: taker 4 bps + slippage 2 bps
/// (`param_robustness_sweep --taker-fee-bps 4 --slippage-bps 2`, the defaults
/// every non-basis anchored invocation uses).
const ANCHORED_TAKER_FEE_BPS: u32 = 4;
/// See [`ANCHORED_TAKER_FEE_BPS`].
const ANCHORED_SLIPPAGE_BPS: u32 = 2;

/// Run one path at an explicit friction level (review 1-18 fee realism).
///
/// The 0/0 default of [`run_to_result`] keeps each falsifier's *mechanism*
/// isolated (any divergence is the signal, not the fee bleed), but every
/// anchored surface runs at 4/2 bps — the regime in which the FRAGILE verdicts
/// were actually reached. The `*_at_anchored_fees` variants below exercise that
/// regime so a fee-path regression cannot hide behind a frictionless harness.
fn run_to_result_with_fees(
    cfg: strategy::CrossSectionalMomentumConfig,
    bars: Vec<Bar>,
    taker_fee_bps: u32,
    slippage_bps: u32,
) -> backtest::scenarios::montecarlo::PathRunResult {
    let n = bars.len();
    let strat = strategy::MomentumStrategy::from_config(cfg, SmolStr::new("hr_e2e_test"));
    let input = TcnScenarioInput {
        scenario_name: "hr-e2e".to_string(),
        start_year: 2023,
        bar_count: n,
        initial_capital: dec!(100_000),
        slippage_bps,
        taker_fee_bps,
        config_id: "test_hr".to_string(),
        forecaster_id: "test".to_string(),
        bars_override: Some(bars),
        emit_equity_bin: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        funding_override: None,
        bar_span_hours: 1,
    };
    pollster::block_on(run_path(input, 0xC0FFEE, strat))
        .expect("run_path must succeed in horizon e2e test")
}

/// `resample_ohlcv` is fallible since review 1-18 (the bucket emitter's three
/// `panic!`s became a `ResampleError`). Tests want the bars.
fn resample(bars_1h: &[Bar], horizon: Horizon) -> Vec<Bar> {
    resample_ohlcv(bars_1h, horizon).expect("resample_ohlcv must succeed on well-formed test bars")
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

/// F-HR.4.a: TS at the 4h horizon diverges ≥ 1 bp from a **like-sized**
/// always-long control.
///
/// ## The revert this catches
///
/// A TS long/flat rule that never goes flat at the 4h cadence — i.e. the
/// `SelectionMode::TimeSeriesLongFlat` exit silently degrading to "always
/// long", the failure mode a no-op overlay produces (CLAUDE.md's
/// baseline-equity-divergence non-negotiable). Under that revert the TS run and
/// the control run become the SAME strategy on the SAME bars, delta collapses
/// to exactly 0, and this test goes RED.
///
/// ## Why the control is not buy-and-hold (review 1-18)
///
/// It used to be: the test compared a TS run (`exposure_cap = 0.5`, so ~10 % of
/// equity deployed on this single-name universe after the top-K split) against
/// `run_buyhold_final_equity`, which deploys 100 % of capital at the first
/// close. Those two differ by ~90 % of the equity **before any signal fires**,
/// so the ≥ 1 bp gate was met by the sizing gap alone and a no-op TS rule would
/// still have passed — a vacuous falsifier (bug-log #66 class). The control is
/// now [`make_always_long_config`]: the same engine, the same bars, the same
/// `exposure_cap`, the same `k_long`, differing ONLY in `selection_mode`. Any
/// surviving divergence is the exit mechanism.
#[test]
fn f_hr_4_baseline_divergence_4h() {
    // N_UP large enough for L=3 4h-bars to see a full uptrend (need 3*4=12 1h-bars of uptrend).
    // N_DOWN large enough for 3+ 4h-bars of downtrend (need 3*4=12 1h-bars minimum).
    const N_UP_1H: usize = 20; // 5 full 4h-bars of uptrend
    const N_DOWN_1H: usize = 32; // 8 full 4h-bars of downtrend (TS exits after L=3 bars)

    let bars_1h = build_1h_up_down_bars(N_UP_1H, N_DOWN_1H);
    let bars_4h = resample(&bars_1h, Horizon::FourHours);

    // TS with lookback L=3 coarse bars, threshold=0.00.
    let result_ts = run_to_result(make_ts_config(3, dec!(0.00)), bars_4h.clone());
    // LIKE-SIZED control: identical config except selection_mode → always long.
    let result_control = run_to_result(make_always_long_config(3), bars_4h.clone());

    // Proof the control really is always-long (otherwise the comparison is
    // meaningless): it must hold strictly more bars than the TS run.
    assert!(
        result_control.time_in_market_bars > result_ts.time_in_market_bars,
        "F-HR.4.a CONTROL INVALID (4h): the always-long control held \
         {control_tim} bars and TS held {ts_tim} — the control must never exit, \
         so if these are equal the TS exit is not firing and the divergence \
         assertion below would be vacuous.",
        control_tim = result_control.time_in_market_bars,
        ts_tim = result_ts.time_in_market_bars,
    );

    let ts_equity = result_ts.final_equity;
    let control_equity = result_control.final_equity;
    let delta = (ts_equity - control_equity).abs();
    const EPSILON_1BP: Decimal = dec!(10); // 1 bp of 100_000

    assert!(
        delta > EPSILON_1BP,
        "F-HR.4.a DIVERGENCE VIOLATION (4h): TS equity ({ts_equity}) must diverge from the \
         LIKE-SIZED always-long control ({control_equity}) by ≥ 1 bp ({EPSILON_1BP}). \
         Actual |delta| = {delta}. delta ≈ 0 means the TS long/flat exit is a no-op at the \
         4h horizon — check resample_ohlcv + the TimeSeriesLongFlat wiring. \
         (This control is deliberately NOT buy-and-hold: a 100 %-deployed BH would meet the \
         gate on the sizing gap alone.)",
        ts_equity = ts_equity,
        control_equity = control_equity,
        delta = delta,
    );
}

/// F-HR.4.a at the ANCHORED friction level (4 bps taker + 2 bps slippage).
///
/// ## The revert this catches
///
/// The same no-op-exit revert as `f_hr_4_baseline_divergence_4h`, but in the
/// regime the anchored surfaces actually ran in. Review 1-18: every e2e helper
/// here ran at 0/0 bps while every anchored θ-surface runs at 4/2 — so the fee
/// bleed the FRAGILE verdicts hinge on was never exercised by a falsifier. It
/// additionally catches a fee path that stops charging: the TS run turns over
/// (it exits and would re-enter), so its realised friction must be strictly
/// positive and its equity must sit strictly below the frictionless run of the
/// identical config.
#[test]
fn f_hr_4_baseline_divergence_4h_at_anchored_fees() {
    const N_UP_1H: usize = 20;
    const N_DOWN_1H: usize = 32;

    let bars_1h = build_1h_up_down_bars(N_UP_1H, N_DOWN_1H);
    let bars_4h = resample(&bars_1h, Horizon::FourHours);

    let result_ts = run_to_result_with_fees(
        make_ts_config(3, dec!(0.00)),
        bars_4h.clone(),
        ANCHORED_TAKER_FEE_BPS,
        ANCHORED_SLIPPAGE_BPS,
    );
    let result_control = run_to_result_with_fees(
        make_always_long_config(3),
        bars_4h.clone(),
        ANCHORED_TAKER_FEE_BPS,
        ANCHORED_SLIPPAGE_BPS,
    );

    assert!(
        result_control.time_in_market_bars > result_ts.time_in_market_bars,
        "F-HR.4.a(fees) CONTROL INVALID: control held {} bars, TS held {} — \
         friction must not suppress the exit itself",
        result_control.time_in_market_bars,
        result_ts.time_in_market_bars,
    );

    let delta = (result_ts.final_equity - result_control.final_equity).abs();
    const EPSILON_1BP: Decimal = dec!(10);
    assert!(
        delta > EPSILON_1BP,
        "F-HR.4.a(fees) DIVERGENCE VIOLATION (4h @ {taker}/{slip} bps): TS equity ({ts}) must \
         diverge from the like-sized always-long control ({ctl}) by ≥ 1 bp. delta={delta}",
        taker = ANCHORED_TAKER_FEE_BPS,
        slip = ANCHORED_SLIPPAGE_BPS,
        ts = result_ts.final_equity,
        ctl = result_control.final_equity,
        delta = delta,
    );

    // The friction must actually bite: the same TS config run frictionless must
    // finish strictly ahead. RED-on-revert: a fee path that silently stops
    // charging (or a `taker_fee_bps` that never reaches MatchConfig) makes these
    // two runs identical.
    let result_ts_free = run_to_result(make_ts_config(3, dec!(0.00)), bars_4h);
    assert!(
        result_ts.trades > 0,
        "F-HR.4.a(fees): the TS run must trade for the fee regime to be exercised; trades=0"
    );
    assert!(
        result_ts.final_equity < result_ts_free.final_equity,
        "F-HR.4.a(fees) FEE PATH INERT: the same TS config finished at {with_fees} at \
         {taker}/{slip} bps and {without_fees} at 0/0 bps. With {trades} trades the friction \
         MUST reduce equity — equal values mean the fee/slippage inputs are not reaching the \
         match engine.",
        with_fees = result_ts.final_equity,
        without_fees = result_ts_free.final_equity,
        taker = ANCHORED_TAKER_FEE_BPS,
        slip = ANCHORED_SLIPPAGE_BPS,
        trades = result_ts.trades,
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
    let bars_daily = resample(&bars_1h, Horizon::OneDay);

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

/// F-HR.4.c: **prefix-invariance** of the resampler — the causality property
/// that actually constrains the fold.
///
/// For every truncation point `k`, `resample_ohlcv(&bars[..k])` must equal the
/// corresponding prefix of `resample_ohlcv(&bars)` on every COMPLETE bucket.
/// Equivalently: a completed coarse bar is a function of the 1h bars that
/// precede its close and NOTHING later. Buckets are compared field-by-field
/// (open/high/low/close/volume/open_ts/close_ts) — not via a downstream equity
/// scalar.
///
/// ## The revert this catches
///
/// Any fold that reaches forward: `open = last`, a high/low computed over a
/// centred window, a bucket that borrows the next bucket's close, a
/// look-ahead-normalised volume. Under any of those, extending the input
/// changes an ALREADY-COMPLETE bucket and the field comparison goes RED at the
/// exact bucket index.
///
/// The trailing bucket of each prefix is deliberately excluded from the
/// comparison: it is partial by construction (the prefix cut mid-bucket), and
/// emitting a partial bucket is the resampler's documented behaviour, not a
/// look-ahead. `f_hr_4_partial_bucket_is_visible` covers that case instead.
///
/// ## What it replaces (review 1-18)
///
/// The old test substituted different input prices and asserted the outputs
/// differed — trivially true of ANY function that reads its input, including
/// one with a blatant look-ahead. It could not go RED under the bug its name
/// claims to catch.
#[test]
fn f_hr_4_no_look_ahead_coarse() {
    const N_BARS_1H: usize = 40;
    let bars_1h = build_1h_alternating_bars(N_BARS_1H);

    for horizon in [Horizon::FourHours, Horizon::OneDay] {
        let ratio = horizon.ratio() as usize;
        let full = resample(&bars_1h, horizon);

        for k in 1..=N_BARS_1H {
            let prefix_out = resample(&bars_1h[..k], horizon);
            // Buckets fully covered by the first k source bars. The prefix's
            // LAST bucket is partial whenever k is not a bucket boundary.
            let complete = k / ratio;
            let comparable = complete.min(prefix_out.len()).min(full.len());
            assert!(
                prefix_out.len() >= comparable,
                "F-HR.4.c ({horizon}): prefix of {k} bars produced {} buckets, \
                 fewer than the {comparable} complete buckets it contains",
                prefix_out.len(),
            );
            for (i, (p, f)) in prefix_out
                .iter()
                .zip(full.iter())
                .take(comparable)
                .enumerate()
            {
                assert_eq!(
                    (
                        p.open_ts.unix_millis(),
                        p.close_ts.unix_millis(),
                        p.open.get(),
                        p.high.get(),
                        p.low.get(),
                        p.close.get(),
                        p.volume.get(),
                        p.trade_count,
                    ),
                    (
                        f.open_ts.unix_millis(),
                        f.close_ts.unix_millis(),
                        f.open.get(),
                        f.high.get(),
                        f.low.get(),
                        f.close.get(),
                        f.volume.get(),
                        f.trade_count,
                    ),
                    "F-HR.4.c NO-LOOK-AHEAD ({horizon}): bucket {i} changed when the input \
                     grew from {k} to {n} source bars. A completed coarse bar must be a \
                     function of the 1h bars up to its close and NOTHING after it — a \
                     difference here means the fold reads forward (e.g. `open = last`, a \
                     centred high/low window, or a borrowed next-bucket close).",
                    n = N_BARS_1H,
                );
            }
        }
    }
}

/// Partial buckets are EMITTED but no longer invisible (review 1-18).
///
/// ## The revert this catches
///
/// Silencing the partial-bucket census: a bucket folded from fewer than
/// `ratio` source bars used to be byte-indistinguishable from a complete one,
/// so a truncated corpus month produced a short coarse bar that looked
/// perfectly normal to every downstream consumer. The test pins BOTH halves of
/// the contract — the partial is still in `bars` (dropping it would change the
/// coarse source series and move the locked horizon anchors) AND it is reported
/// in `partial_buckets` with its bucket timestamp and true source count.
#[test]
fn f_hr_4_partial_bucket_is_visible() {
    // 6 hourly bars at 4h → bucket 0 complete (4 bars), bucket 1 partial (2).
    let bars_1h = build_1h_alternating_bars(6);
    let detailed = backtest::resample::resample_ohlcv_detailed(&bars_1h, Horizon::FourHours)
        .expect("well-formed bars must resample");

    assert_eq!(
        detailed.bars.len(),
        2,
        "the partial trailing bucket must still be EMITTED (dropping it would change \
         the coarse source series and move the locked horizon anchors)"
    );
    assert!(
        !detailed.is_complete(),
        "a 6-bar input at 4h has an incomplete trailing bucket — is_complete() must say so"
    );
    assert_eq!(
        detailed.partial_buckets.len(),
        1,
        "exactly one partial bucket expected, got {:?}",
        detailed.partial_buckets
    );
    let p = detailed.partial_buckets[0];
    assert_eq!(p.index, 1, "the SECOND bucket is the partial one");
    assert_eq!(p.source_bar_count, 2, "it was folded from 2 source bars");
    assert_eq!(p.expected_bar_count, 4, "a 4h bucket wants 4 source bars");
    assert_eq!(
        p.open_ts_ms,
        detailed.bars[1].open_ts.unix_millis(),
        "the census must NAME the bucket by its open timestamp"
    );

    // A whole number of buckets reports nothing.
    let complete = backtest::resample::resample_ohlcv_detailed(
        &build_1h_alternating_bars(8),
        Horizon::FourHours,
    )
    .expect("well-formed bars must resample");
    assert!(
        complete.is_complete() && complete.partial_buckets.is_empty(),
        "8 bars at 4h are exactly 2 complete buckets — no partial may be reported: {:?}",
        complete.partial_buckets
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
    let bars_4h = resample(&bars_1h, Horizon::FourHours);
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
    let bars_4h = resample(&bars_1h, Horizon::FourHours);

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
// F-HR.5 — Two-run byte-identity of each horizon surface REPORT BODY
// ─────────────────────────────────────────────────────────────────────────────
//
// ## What these cover, and what they do not (review 1-18)
//
// COVERED: the whole in-process rendering chain for a LOCKED horizon grid —
// `resample_ohlcv` → `run_path` (real engine, real fills) → the periodic metric
// fns → `DistributionSummary` → `classify_verdict` → `family_any_non_fragile`
// → `render_surface_report`. The comparison is on the RENDERED BODY (the exact
// substring `scripts/hash_report.py` SHA-256s — front-matter stripped), so any
// unordered fold, unstable reduction, HashMap iteration, float-format drift, or
// verdict/grid desync anywhere in that chain changes bytes and fails.
//
// NOT COVERED: the bin's own pre-render stages — CLI parsing, the real-corpus
// loader, `BlockBootstrapPathGen` seeding, and rayon's cross-cell scheduling.
// A full `param_robustness_sweep` invocation needs the pinned Binance corpus
// and runs for minutes, which is not a unit-test shape; the anchored surfaces
// themselves (`scripts/verify_anchors.sh`, 119/119) are the gate on that outer
// layer. What these tests add is that everything from the resampler inward is
// deterministic, which is where a coarse-horizon fold could realistically go
// unstable.
//
// ## What they replace
//
// The old versions asserted `f(x) == f(x)` on a HAND-BUILT `make_ts_config`
// pair and compared three formatted `DistributionSummary` floats — no LOCKED
// grid, no renderer, no verdict. They could not have gone RED for any drift in
// the report body, which is the artifact the anchors actually hash.

/// One cell of a real horizon surface: run N paths, reduce, classify.
///
/// `cell` is a LOCKED grid cell (`grid_for_kind(GridKind::Ts4h | ::TsDaily)`);
/// its `lookback_minutes` / `entry_threshold` are injected into the strategy
/// config exactly as `cell_config` does for the production sweep.
fn run_locked_cell(
    cell: &backtest::sweep_harness::ThetaCell,
    n_paths: usize,
    horizon: Horizon,
    year: i32,
) -> backtest::sweep_harness::CellResult {
    let ppy = horizon.periods_per_year(year);
    // Read the θ off the LOCKED cell through the PRODUCTION accessor — never a
    // local re-derivation of the num/den encoding.
    let cfg = make_ts_config(cell.lookback_minutes, cell.entry_threshold());

    let mut all_metrics: Vec<PathMetrics> = Vec::with_capacity(n_paths);
    let mut total_trades = 0u64;
    let mut total_time_in_market_bars = 0u64;
    let mut total_bars_run = 0u64;

    for j in 0..n_paths {
        // Deterministic per-path fixture variation (stands in for the
        // block-bootstrap ensemble, which needs the pinned corpus).
        let n_up_1h = 20 + j * 4;
        let n_down_1h = 24 + j * 4;
        let bars_1h = build_1h_up_down_bars(n_up_1h, n_down_1h);
        let bars_coarse = resample(&bars_1h, horizon);
        let n_coarse = bars_coarse.len() as u64;

        let result = run_to_result_with_fees(
            cfg.clone(),
            bars_coarse,
            ANCHORED_TAKER_FEE_BPS,
            ANCHORED_SLIPPAGE_BPS,
        );
        total_trades += result.trades as u64;
        total_time_in_market_bars += result.time_in_market_bars;
        total_bars_run += n_coarse;

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

        // The periodic metric fns — the coarse-horizon branch (D-HR.1).
        all_metrics.push(PathMetrics {
            sharpe: compute_sharpe_periodic(&equity_clamped, ppy),
            sortino: compute_sortino_periodic(&equity_clamped, ppy),
            calmar: compute_calmar_periodic(&equity_clamped, ppy),
            max_drawdown: compute_max_drawdown_f64(&equity_clamped),
            total_return: compute_total_return(&equity_clamped),
            final_equity: result.final_equity,
            initial_equity: result.initial_equity,
        });
    }

    let summary = DistributionSummary::from_path_metrics(&all_metrics)
        .expect("build DistributionSummary for horizon cell");
    let verdict = backtest::bakeoff::robustness::classify_verdict(&summary);
    backtest::sweep_harness::CellResult {
        cell: *cell,
        summary,
        verdict,
        total_trades,
        total_funding_harvested: Decimal::ZERO,
        total_time_in_market_bars,
        total_bars_run,
        total_liquidations: 0,
    }
}

/// Render a full horizon surface through the PRODUCTION renderer.
///
/// Front-matter values that legitimately vary run-to-run (`generated`,
/// `wall_clock_s`, `host`, `pid`, `git_commit`) are held at fixed literals so
/// the two runs are comparable; the front-matter is stripped before comparison
/// anyway (see [`report_body`]).
fn render_locked_surface(
    grid_kind: backtest::sweep_harness::GridKind,
    horizon: Horizon,
    year: i32,
    n_paths: usize,
) -> String {
    let grid = backtest::sweep_harness::grid_for_kind(grid_kind);
    let cell_results: Vec<backtest::sweep_harness::CellResult> = grid
        .iter()
        .map(|c| run_locked_cell(c, n_paths, horizon, year))
        .collect();
    // Buy-and-hold control distribution, built from the same fixture family.
    let bh_metrics: Vec<PathMetrics> = (0..n_paths)
        .map(|j| {
            let bars_1h = build_1h_up_down_bars(20 + j * 4, 24 + j * 4);
            let bars_coarse = resample(&bars_1h, horizon);
            let equity: Vec<Decimal> = bars_coarse
                .iter()
                .map(|b| dec!(100_000) * b.close.get() / bars_coarse[0].close.get())
                .collect();
            let ppy = horizon.periods_per_year(year);
            PathMetrics {
                sharpe: compute_sharpe_periodic(&equity, ppy),
                sortino: compute_sortino_periodic(&equity, ppy),
                calmar: compute_calmar_periodic(&equity, ppy),
                max_drawdown: compute_max_drawdown_f64(&equity),
                total_return: compute_total_return(&equity),
                final_equity: *equity.last().unwrap_or(&dec!(100_000)),
                initial_equity: dec!(100_000),
            }
        })
        .collect();
    let buyhold_summary =
        DistributionSummary::from_path_metrics(&bh_metrics).expect("build BH control summary");

    let scenario = backtest::sweep_harness::build_scenario_name(
        grid_kind,
        backtest::sweep_harness::SweepDirection::Momentum,
        backtest::sweep_harness::SweepScoreSource::VolAdjustedReturn,
        backtest::sweep_harness::SweepSelectionMode::TimeSeriesLongFlat,
        horizon,
        year,
        "block-bootstrap-real",
        ANCHORED_TAKER_FEE_BPS,
    );

    backtest::sweep_harness::render_surface_report(
        "2026-08-06T00:00:00Z",
        1.0,
        "testhost",
        1,
        "deadbeef",
        "test-data-revision-sha",
        &scenario,
        0xC0FFEE,
        0xC0FFEE,
        n_paths,
        "block-bootstrap-real",
        "stationary",
        "auto",
        Some(7),
        "test-source-revision-sha",
        grid,
        &cell_results,
        &buyhold_summary,
        backtest::sweep_harness::SweepDirection::Momentum,
        backtest::sweep_harness::SweepScoreSource::VolAdjustedReturn,
        None,
        backtest::sweep_harness::SweepSelectionMode::TimeSeriesLongFlat,
        horizon,
        ANCHORED_TAKER_FEE_BPS,
        ANCHORED_SLIPPAGE_BPS,
    )
}

/// The hashed part of a report: everything after the closing `---` of the YAML
/// front-matter. This is exactly what `scripts/hash_report.py` /
/// `scripts/verify_anchors.sh` SHA-256 into `evidence/anchors.toml`.
fn report_body(report: &str) -> &str {
    let rest = report
        .strip_prefix("---\n")
        .expect("report must open with YAML front-matter");
    let end = rest
        .find("\n---\n")
        .expect("report front-matter must be closed by a `---` line");
    &rest[end + "\n---\n".len()..]
}

/// F-HR.5 (4h) — two renders of the LOCKED `ts-4h` surface are byte-identical.
///
/// ## The revert this catches
///
/// Any non-determinism between the 1h source bars and the hashed report body at
/// the 4h cadence: an unordered fold or `HashMap` bucket keying in
/// `resample_ohlcv`, an unstable percentile/sort in `DistributionSummary`, a
/// verdict that reads uninitialised state, a renderer row whose order depends
/// on iteration order, or a float formatted at native precision instead of the
/// ADR-0051 D3 fixed precision. Any of those makes run 1 and run 2 differ and
/// this test goes RED — which is precisely the failure that would make the
/// anchored 4h surfaces (#92/#93) unreproducible.
///
/// See the section header for the explicit coverage boundary (the corpus loader
/// and rayon scheduling are NOT exercised here).
#[test]
fn f_hr_5_two_run_byte_identity_4h() {
    const N: usize = 3;
    let body_1 = render_locked_surface(
        backtest::sweep_harness::GridKind::Ts4h,
        Horizon::FourHours,
        2023,
        N,
    );
    let body_2 = render_locked_surface(
        backtest::sweep_harness::GridKind::Ts4h,
        Horizon::FourHours,
        2023,
        N,
    );

    let b1 = report_body(&body_1);
    let b2 = report_body(&body_2);
    if b1 != b2 {
        let first_diff = b1
            .lines()
            .zip(b2.lines())
            .enumerate()
            .find(|(_, (l1, l2))| l1 != l2)
            .map_or_else(
                || "line counts differ".to_string(),
                |(i, (l1, l2))| format!("line {i}:\n  run1: {l1}\n  run2: {l2}"),
            );
        panic!(
            "F-HR.5 (4h) NON-DETERMINISM: two runs of the LOCKED ts-4h surface produced \
             different hashed report bodies. The anchored 4h surfaces (#92/#93) would be \
             unreproducible. First difference — {first_diff}"
        );
    }

    // The body must actually be the horizon surface (not an empty/1h render):
    // these three lines are what make it the artifact the anchors hash.
    assert!(
        b1.contains("| horizon                  | 4h"),
        "the 4h body must carry the hashed `horizon` row: {b1}"
    );
    assert!(
        b1.contains("FAMILY-"),
        "the body must carry the pre-registered family-verdict line: {b1}"
    );
    // The TS lane renders its grid through `ts_grid_def_string` (it carries the
    // TS-only entry_threshold column) — assert against THAT formatter, not the
    // momentum `grid_def_string`.
    assert!(
        b1.contains(&backtest::sweep_harness::ts_grid_def_string(
            backtest::sweep_harness::grid_for_kind(backtest::sweep_harness::GridKind::Ts4h)
        )),
        "the body must embed the LOCKED ts-4h grid definition (K3/D6.3)"
    );
}

/// F-HR.5 (daily) — two renders of the LOCKED `ts-daily` surface are
/// byte-identical. Mirrors the 4h test; same revert, daily cadence (the
/// anchored surfaces are #94/#95).
#[test]
fn f_hr_5_two_run_byte_identity_daily() {
    const N: usize = 3;
    let body_1 = render_locked_surface(
        backtest::sweep_harness::GridKind::TsDaily,
        Horizon::OneDay,
        2023,
        N,
    );
    let body_2 = render_locked_surface(
        backtest::sweep_harness::GridKind::TsDaily,
        Horizon::OneDay,
        2023,
        N,
    );

    let b1 = report_body(&body_1);
    let b2 = report_body(&body_2);
    if b1 != b2 {
        let first_diff = b1
            .lines()
            .zip(b2.lines())
            .enumerate()
            .find(|(_, (l1, l2))| l1 != l2)
            .map_or_else(
                || "line counts differ".to_string(),
                |(i, (l1, l2))| format!("line {i}:\n  run1: {l1}\n  run2: {l2}"),
            );
        panic!(
            "F-HR.5 (daily) NON-DETERMINISM: two runs of the LOCKED ts-daily surface produced \
             different hashed report bodies. The anchored daily surfaces (#94/#95) would be \
             unreproducible. First difference — {first_diff}"
        );
    }

    assert!(
        b1.contains("| horizon                  | daily"),
        "the daily body must carry the hashed `horizon` row: {b1}"
    );
    assert!(
        b1.contains(&backtest::sweep_harness::ts_grid_def_string(
            backtest::sweep_harness::grid_for_kind(backtest::sweep_harness::GridKind::TsDaily)
        )),
        "the body must embed the LOCKED ts-daily grid definition (K3/D6.3)"
    );
}

/// F-HR.5 negative control: the 4h and daily surfaces must NOT render the same
/// body.
///
/// ## The revert this catches
///
/// A byte-identity test passes trivially if the renderer collapses everything
/// to a constant. This proves the two horizon renders are genuinely distinct
/// artifacts — the `horizon` row, the grid definition, and the metrics all move
/// with the cadence — so the identity assertions above have real content.
#[test]
fn f_hr_5_horizon_surfaces_are_distinct() {
    const N: usize = 3;
    let four_h = render_locked_surface(
        backtest::sweep_harness::GridKind::Ts4h,
        Horizon::FourHours,
        2023,
        N,
    );
    let daily = render_locked_surface(
        backtest::sweep_harness::GridKind::TsDaily,
        Horizon::OneDay,
        2023,
        N,
    );
    assert_ne!(
        report_body(&four_h),
        report_body(&daily),
        "F-HR.5 control: the 4h and daily surfaces must render DIFFERENT bodies — \
         identical bodies would mean the byte-identity tests are vacuous"
    );
}
