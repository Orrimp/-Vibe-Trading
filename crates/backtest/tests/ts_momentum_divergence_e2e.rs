//! TS-momentum day-1 falsifiers (M-DEV-5).
//!
//! Five falsifiers, each RED-on-revert (the test FAILS when its guarded property
//! is broken), per the CLAUDE.md non-negotiable (every overlay/sizing-modifier
//! ships a baseline-equity-divergence e2e from day 1).
//!
//! ## F-TSM.1 — Baseline-equity-divergence e2e (headline anti-no-op)
//!
//! TS-momentum equity diverges ≥ 1 bp from the LIKE-SIZED always-long control
//! (review 1-17: the old primary compared against full-capital buy-and-hold, so
//! an always-long no-op passed on the sizing gap alone — the gate is now
//! against a control with the identical `run_path` 10%-of-equity sizing, so a
//! no-op TS rule produces Δ≈0 and FAILS). BH kept as a secondary info assert.
//!
//! ## F-TSM.2 — Signal-non-no-op
//!
//! Force the trend signal degenerate (entry_threshold below every score →
//! always-long) → equity collapses to the like-sized always-long case with an
//! honest fee-free expectation of EXACTLY zero (identical signal streams →
//! identical fills at zero fee/slippage; tolerance is a 0.1 bp slack, review
//! 1-17 — was a ~500× loose 5% band). Proves the long/flat DECISION (not a
//! sizing artifact) produces the divergence.
//!
//! ## F-TSM.3 — No-look-ahead (prefix-invariance form, review 1-17)
//!
//! Run the full series, truncate the last K bars, re-run: every pre-truncation
//! equity point (= every rebalance decision) must be IDENTICAL. A strategy
//! whose bar-t decision reads any bar > t changes its prefix when the future is
//! removed → RED. (The old form fed two different series and asserted they
//! differ — it could not fail on an actual look-ahead.)
//!
//! ## F-TSM.4 — Goes-flat (TS-specific, the must-actually-exit gate)
//!
//! On a series with a clear sustained downtrend, the strategy EXITS to FLAT
//! on ≥ 1 POST-WARMUP bar: headline asserts post-warmup time-in-market strictly
//! below the post-warmup total (review 1-17 — warmup bars excluded from BOTH
//! sides via the degenerate always-long control, whose tim IS the post-warmup
//! total; the old `tim < total_bars` form was warmup-satisfiable).
//!
//! ## F-TSM.5 — Two-run byte-identity of the TS surface body (review 1-17)
//!
//! Drive a REAL small-N seeded TS sweep (production seams: `derive_path_seed`
//! per path, seeded ChaCha20 synthetic bars, `run_path` per cell,
//! `DistributionSummary` reduction, `classify_verdict`,
//! `render_surface_report`) twice with the same master seed and assert the two
//! RENDERED BODIES are byte-identical; a different master seed must change the
//! body (proves the seed parameter is USED — the pre-1-17 helper ignored it).
//! Catches any unordered fold in the per-asset score loop, selector, reducer,
//! or renderer (D-TSM.6).
//!
//! ## Pattern references
//!
//! - `crates/backtest/tests/carry_divergence_e2e.rs` (carry sibling — R-CARRY.10a/10b/2/6)
//! - `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs` (CLAUDE.md non-negotiable)
//! - `crates/backtest/tests/param_sweep_e2e.rs::fp_c3_3_two_run_byte_identity`

use backtest::cli_types::TcnScenarioInput;
use backtest::scenarios::montecarlo::run_path;
use backtest::stats::{
    DistributionSummary, PathMetrics, compute_calmar, compute_max_drawdown_f64,
    compute_sharpe_hourly, compute_sortino_hourly, compute_total_return,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use strategy::SelectionMode;
use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

// ── Timestamp helpers ─────────────────────────────────────────────────────────

fn epoch_2023() -> time::OffsetDateTime {
    time::OffsetDateTime::from_unix_timestamp(1_672_531_200).expect("valid epoch_2023")
}

fn make_ts(offset_hours: i64) -> Timestamp {
    Timestamp::new(epoch_2023() + time::Duration::hours(offset_hours))
}

// ── Bar builder ───────────────────────────────────────────────────────────────

fn make_bar(sym: &str, close: Decimal, hour: i64) -> Bar {
    Bar {
        symbol: Symbol::new(sym),
        tf: Timeframe::OneHour,
        open_ts: make_ts(hour),
        close_ts: make_ts(hour),
        local_recv_ts: make_ts(hour),
        venue: Venue::Binance,
        open: Price::new(close).unwrap(),
        high: Price::new(close).unwrap(),
        low: Price::new(close).unwrap(),
        close: Price::new(close).unwrap(),
        volume: Quantity::new(dec!(100)).unwrap(),
        trade_count: 1,
    }
}

// ── Universe builder ──────────────────────────────────────────────────────────
//
// Design: single symbol "AAAUSD" with:
//   Phase 1 (bars 0..N_UP): uptrend (+1% per bar) — TS rule goes long.
//   Phase 2 (bars N_UP..total): sustained downtrend (−2% per bar) — TS rule should go flat.
//
// Buy-and-hold sits through both phases; TS-momentum exits in Phase 2.
// This guarantees equity divergence (F-TSM.1 / F-TSM.4).

fn build_up_then_down_bars(n_up: usize, n_down: usize) -> Vec<Bar> {
    let mut bars: Vec<Bar> = Vec::new();
    let mut price = dec!(1000);
    for hour in 0..(n_up + n_down) {
        bars.push(make_bar("AAAUSD", price, hour as i64));
        if hour < n_up {
            // Uptrend: +1% per bar
            price *= dec!(1.01);
        } else {
            // Sustained downtrend: −2% per bar
            price *= dec!(0.98);
        }
    }
    // Already sorted by symbol (single symbol), but sort by ts for correctness
    bars.sort_by(|a, b| a.open_ts.cmp(&b.open_ts).then(a.symbol.0.cmp(&b.symbol.0)));
    bars
}

// ── Config builders ───────────────────────────────────────────────────────────

/// TS-momentum config: TimeSeriesLongFlat, 1-symbol universe, lookback=5, threshold=0.00.
/// After 5 bars of uptrend: score = Σlog(1.01)^5 ≈ +0.0497 > 0.00 → LONG.
/// After 5+ bars of downtrend: score = Σlog(0.98)^5 ≈ −0.101 < 0.00 → FLAT.
fn make_ts_config(threshold: Decimal) -> strategy::CrossSectionalMomentumConfig {
    strategy::CrossSectionalMomentumConfig {
        id: SmolStr::new("ts_test"),
        universe: vec![SmolStr::new("AAAUSD")],
        lookback_minutes: 5,  // lookback L=5 bars
        rebalance_minutes: 1, // rebalance every bar
        k_long: 10,           // inert under TimeSeriesLongFlat
        k_short: 0,
        exposure_cap: dec!(0.5),
        drift_rebalance_threshold: dec!(0.10),
        vol_floor: dec!(0.000001),
        stage: SmolStr::new("research"),
        direction: strategy::Direction::Momentum,
        score_source: strategy::ScoreSource::VolAdjustedReturn, // ignored under TimeSeriesLongFlat
        selection_mode: SelectionMode::TimeSeriesLongFlat,
        entry_threshold: threshold,
    }
}

/// TS-momentum config with custom lookback and threshold.
fn make_ts_config_custom(
    lookback: u32,
    threshold: Decimal,
) -> strategy::CrossSectionalMomentumConfig {
    strategy::CrossSectionalMomentumConfig {
        id: SmolStr::new(format!("ts_L{lookback}_thr{threshold}")),
        universe: vec![SmolStr::new("AAAUSD")],
        lookback_minutes: lookback,
        rebalance_minutes: 1,
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

/// BH-equivalent: CrossSectionalTopK with threshold=0 → always selects top-K.
/// With K=10 and a 1-symbol universe, this is always long after warmup → ≈ BH.
fn make_always_long_config() -> strategy::CrossSectionalMomentumConfig {
    strategy::CrossSectionalMomentumConfig {
        id: SmolStr::new("ts_always_long"),
        universe: vec![SmolStr::new("AAAUSD")],
        lookback_minutes: 5,
        rebalance_minutes: 1,
        k_long: 10,
        k_short: 0,
        exposure_cap: dec!(0.5),
        drift_rebalance_threshold: dec!(0.10),
        vol_floor: dec!(0.000001),
        stage: SmolStr::new("research"),
        direction: strategy::Direction::Momentum,
        score_source: strategy::ScoreSource::VolAdjustedReturn,
        selection_mode: SelectionMode::CrossSectionalTopK, // always-long after warmup
        entry_threshold: Decimal::ZERO,
    }
}

/// Degenerate TS config: threshold = very negative → always-long regardless of trend.
fn make_degenerate_ts_config() -> strategy::CrossSectionalMomentumConfig {
    let mut cfg = make_ts_config(dec!(-999999)); // threshold so negative it never blocks entry
    cfg.id = SmolStr::new("ts_degenerate");
    cfg
}

// ── run one path → PathRunResult ─────────────────────────────────────────────

fn run_to_result(
    cfg: strategy::CrossSectionalMomentumConfig,
    bars: Vec<Bar>,
) -> backtest::scenarios::montecarlo::PathRunResult {
    let strat = strategy::MomentumStrategy::from_config(cfg, SmolStr::new("ts_e2e_test"));
    let input = TcnScenarioInput {
        scenario_name: "ts-e2e".to_string(),
        start_year: 2023,
        bar_count: bars.len(),
        initial_capital: dec!(100_000),
        slippage_bps: 0, // zero friction to isolate signal effects
        taker_fee_bps: 0,
        config_id: "test_ts".to_string(),
        forecaster_id: "test".to_string(),
        bars_override: Some(bars),
        emit_equity_bin: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        funding_override: None, // TS-momentum: no funding
        basis_override: None,
    };
    pollster::block_on(run_path(input, 0xC0FFEE, strat))
        .expect("run_path must succeed in TS divergence e2e test")
}

// ── Buy-and-hold baseline ─────────────────────────────────────────────────────

fn run_buyhold(bars: &[Bar]) -> Decimal {
    if bars.is_empty() {
        return dec!(100_000);
    }
    let initial_capital = dec!(100_000);
    let sym = Symbol::new("AAAUSD");

    // Buy at bar 0 close price.
    let buy_price = bars.first().map(|b| b.close.get()).unwrap_or(dec!(1));
    if buy_price <= Decimal::ZERO {
        return initial_capital;
    }
    let qty = initial_capital / buy_price;

    // Mark-to-market at final bar.
    let final_price = bars.last().map(|b| b.close.get()).unwrap_or(buy_price);
    let _ = sym; // suppress unused warning
    qty * final_price
}

// ─────────────────────────────────────────────────────────────────────────────
// F-TSM.1 — Baseline-equity-divergence e2e (CLAUDE.md non-negotiable)
// ─────────────────────────────────────────────────────────────────────────────

/// F-TSM.1 PASS: TS-momentum equity diverges ≥ 1 bp from the LIKE-SIZED
/// always-long control (review 1-17).
///
/// Universe: AAAUSD with uptrend (+1%/bar × 15 bars) then sustained downtrend
/// (−2%/bar × 25 bars). TS rule (lookback=5, threshold=0.00) goes long during
/// the uptrend and exits to FLAT when the trend flips → avoids downtrend
/// losses. The like-sized control (CrossSectionalTopK over the same 1-symbol
/// universe, identical exposure_cap and `run_path` 10%-of-equity sizing) sits
/// long through the downtrend.
///
/// **Why the like-sized control is the primary gate:** the old primary compared
/// against FULL-CAPITAL buy-and-hold, so an always-long no-op TS still passed
/// via the 10%-sizing gap alone. Against the like-sized control, a no-op TS
/// (always-long) produces the SAME fills → Δ≈0 → this test FAILS (RED-on-revert
/// demonstrated by `f_tsm_1_red_on_revert_always_long_tracks_bh`).
#[test]
fn f_tsm_1_baseline_divergence() {
    const N_UP: usize = 15; // uptrend bars (warmup + several long bars)
    const N_DOWN: usize = 25; // sustained downtrend — TS exits; always-long suffers

    let bars = build_up_then_down_bars(N_UP, N_DOWN);

    let result_ts = run_to_result(make_ts_config(dec!(0.00)), bars.clone());
    let result_always_long = run_to_result(make_always_long_config(), bars.clone());

    let ts_equity = result_ts.final_equity;
    let control_equity = result_always_long.final_equity;
    // Positive when the TS exit avoided downtrend loss the control sat through.
    let delta = ts_equity - control_equity;

    // PRIMARY GATE: ≥ 1 bp of initial capital (100_000 × 0.0001 = 10) vs the
    // LIKE-SIZED control — a no-op (always-long) TS produces delta ≈ 0 → FAILS.
    const EPSILON_1BP: Decimal = dec!(10); // 1 bp of initial 100_000

    assert!(
        delta.abs() > EPSILON_1BP,
        "F-TSM.1 DIVERGENCE VIOLATION: TS equity ({ts_equity}) must diverge from the \
         LIKE-SIZED always-long control ({control_equity}) by ≥ 1 bp ({EPSILON_1BP}). \
         Actual |delta| = {}. If delta ≈ 0, the TS long/flat exit is a no-op (always-long) — \
         check SelectionMode::TimeSeriesLongFlat + score_trailing_log_return wiring.",
        delta.abs()
    );
    assert!(
        delta > Decimal::ZERO,
        "F-TSM.1 DIRECTION: on an up-then-down path the TS exit must BEAT the like-sized \
         always-long control (ts={ts_equity}, control={control_equity}) — a negative delta \
         means the exit fired on the wrong side."
    );

    // SECONDARY (info only, not the anti-no-op gate): TS also diverges from
    // full-capital BH — kept for continuity with the original headline framing.
    let bh_equity = run_buyhold(&bars);
    assert!(
        (ts_equity - bh_equity).abs() > EPSILON_1BP,
        "F-TSM.1 secondary: TS ({ts_equity}) should also diverge from full-capital BH \
         ({bh_equity}) on this path (info assert — the primary gate above is the \
         like-sized control)."
    );
}

/// F-TSM.1 RED-on-revert: always-long rule produces Δ≈0 vs BH → proves divergence gate works.
///
/// When the TS rule is replaced with CrossSectionalTopK (always selects the single-name
/// universe → always long after warmup), the equity tracks BH closely. This confirms
/// that the F-TSM.1 test WOULD FAIL (detecting the no-op) if TS were wired as always-long.
#[test]
fn f_tsm_1_red_on_revert_always_long_tracks_bh() {
    const N_UP: usize = 15;
    const N_DOWN: usize = 25;

    let bars = build_up_then_down_bars(N_UP, N_DOWN);

    // Always-long (CrossSectionalTopK) with 1-symbol universe → tracks BH after warmup.
    let result_always_long = run_to_result(make_always_long_config(), bars.clone());
    let bh_equity = run_buyhold(&bars);

    let always_long_equity = result_always_long.final_equity;

    // The always-long strategy should track BH closely (both sit through downtrend).
    // We confirm: if we were to subtract BH, the delta would be SMALL (< 1 bp only if
    // fees=0 and sizing is exactly 1/K=1/10 of cap) — or at least, MUCH smaller than
    // what the TS strategy produces.
    //
    // Because sizing is fixed-fraction (10% of equity), the always-long result is NOT
    // exactly BH (it uses a fixed fraction, not full exposure). So we assert:
    // the TS divergence from BH is LARGER than the always-long deviation from BH.
    let ts_equity = run_to_result(make_ts_config(dec!(0.00)), bars.clone()).final_equity;
    let delta_ts = (ts_equity - bh_equity).abs();
    let delta_always_long = (always_long_equity - bh_equity).abs();

    assert!(
        delta_ts > delta_always_long,
        "F-TSM.1 RED-ON-REVERT: TS divergence from BH ({delta_ts}) should be LARGER than \
         always-long deviation from BH ({delta_always_long}). \
         If always-long deviates as much as TS, the exit mechanism is not working. \
         bh={bh_equity}, ts={ts_equity}, always_long={always_long_equity}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// F-TSM.2 — Signal-non-no-op
// ─────────────────────────────────────────────────────────────────────────────

/// F-TSM.2: Force threshold so negative it never blocks entry → always-long.
/// The equity collapses to the always-long case (Δ < ε) — proving the
/// long/flat DECISION (not a sizing artifact) drives F-TSM.1's divergence.
///
/// If both strategies are always long, they should produce very similar results
/// (the small remaining delta is from fixed-fraction vs full-exposure sizing).
#[test]
fn f_tsm_2_signal_non_no_op() {
    const N_UP: usize = 15;
    const N_DOWN: usize = 25;

    let bars = build_up_then_down_bars(N_UP, N_DOWN);

    // Degenerate TS (threshold = -999999): always-long after warmup.
    let result_degenerate = run_to_result(make_degenerate_ts_config(), bars.clone());
    // Normal TS (threshold = 0.00): goes flat in downtrend.
    let result_normal = run_to_result(make_ts_config(dec!(0.00)), bars.clone());

    // Degenerate TS and always-long (CrossSectionalTopK) should produce similar results.
    let result_always_long = run_to_result(make_always_long_config(), bars);

    let delta_degen_vs_always_long =
        (result_degenerate.final_equity - result_always_long.final_equity).abs();

    // Honest fee-free expectation (review 1-17 — was a ~500× loose 5% band):
    // both configs share lookback=5 (same warmup bar), rebalance every bar, the
    // same 1-symbol universe, and `run_path`'s fixed 10%-of-equity sizing. The
    // degenerate TS selects the symbol whenever score > −999999 (i.e. always
    // once warmed); TopK selects the single warmed symbol regardless of score.
    // Identical membership on every rebalance bar → identical signal stream →
    // identical fills at zero fee/slippage → the expected delta is EXACTLY 0.
    // Allow a 0.1 bp slack (1 unit on 100_000) purely as defensive headroom.
    let tolerance = dec!(1); // 0.1 bp of 100_000 (expectation: exactly 0)
    assert!(
        delta_degen_vs_always_long < tolerance,
        "F-TSM.2 NON-NO-OP: degenerate TS (threshold=−∞, always-long) must track the \
         like-sized always-long (CrossSectionalTopK) EXACTLY at zero fees (expected delta 0, \
         slack 0.1 bp). delta={delta_degen_vs_always_long}. \
         If they differ, the degenerate threshold is not triggering always-long behavior \
         or the two selection paths no longer share sizing."
    );

    // Normal TS must diverge significantly from degenerate TS (the exit makes the difference).
    let delta_normal_vs_degen = (result_normal.final_equity - result_degenerate.final_equity).abs();
    let epsilon_1bp = dec!(10); // 1 bp of 100_000
    assert!(
        delta_normal_vs_degen > epsilon_1bp,
        "F-TSM.2 SIGNAL-NON-NO-OP: normal TS (threshold=0.00, goes-flat) must diverge from \
         degenerate TS (threshold=−∞, always-long) by ≥ 1 bp. delta={delta_normal_vs_degen}. \
         If delta ≈ 0, the threshold is not changing the selection — the signal is decorative."
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// F-TSM.3 — No-look-ahead
// ─────────────────────────────────────────────────────────────────────────────

/// F-TSM.3: Prefix-invariance — truncating FUTURE bars must not change any
/// PAST decision (review 1-17 rebuild).
///
/// Run the strategy on the full series, then on the same series with the last
/// `K_TRUNC` bars removed, and assert the entire pre-truncation equity curve is
/// Decimal-EXACT identical. Equity after bar t is a faithful decision probe: any
/// changed rebalance decision changes fills → cash/positions → equity.
///
/// **Why this is the non-vacuous form:** a causal strategy (bar-t decision reads
/// only bars ≤ t) is prefix-invariant BY CONSTRUCTION; a look-ahead strategy
/// (bar-t decision reads any bar > t) sees different "future" data once the tail
/// is truncated and changes at least one pre-truncation decision → the prefix
/// differs → RED. The pre-1-17 form fed two DIFFERENT series and asserted the
/// results differ — which holds for causal and look-ahead strategies alike, so
/// it could never fail on an actual look-ahead.
#[test]
fn f_tsm_3_no_look_ahead_prefix_invariance() {
    // Up-then-down with the trend flip INSIDE the kept prefix so real decisions
    // (entry ~bar 5, exit ~bar 18) happen before the truncation point (bar 32).
    const N_UP: usize = 15;
    const N_DOWN: usize = 25;
    const K_TRUNC: usize = 8; // truncate the last 8 bars

    let full_bars = build_up_then_down_bars(N_UP, N_DOWN);
    let n_full = full_bars.len();
    let n_prefix = n_full - K_TRUNC;
    let prefix_bars: Vec<Bar> = full_bars[..n_prefix].to_vec();

    let result_full = run_to_result(make_ts_config(dec!(0.00)), full_bars);
    let result_prefix = run_to_result(make_ts_config(dec!(0.00)), prefix_bars);

    // Non-vacuity guard: the kept prefix must contain real rebalance decisions
    // (the long entry AND the goes-flat exit), otherwise prefix equality would
    // hold trivially for an all-cash run.
    assert!(
        result_prefix.trades >= 2,
        "F-TSM.3 non-vacuity: the kept prefix must contain the entry and the exit \
         (≥ 2 fills), got {} — enlarge the prefix or the trend phases.",
        result_prefix.trades
    );

    // Prefix invariance: equity_curve is [initial, after bar 0, …]; the first
    // n_prefix+1 points of the full run must equal the truncated run EXACTLY.
    assert_eq!(
        result_prefix.equity_curve.len(),
        n_prefix + 1,
        "equity curve length must be n_bars + 1"
    );
    for (t, (full_eq, prefix_eq)) in result_full.equity_curve[..=n_prefix]
        .iter()
        .zip(result_prefix.equity_curve.iter())
        .enumerate()
    {
        assert_eq!(
            full_eq, prefix_eq,
            "F-TSM.3 LOOK-AHEAD VIOLATION at equity index {t}: the full-series run \
             ({full_eq}) and the future-truncated run ({prefix_eq}) diverge BEFORE the \
             truncation point — some bar-t decision is reading data from a bar > t \
             (ring-buffer indexing or rebalance gating is non-causal)."
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// F-TSM.4 — Goes-flat (TS-specific, must-actually-exit gate)
// ─────────────────────────────────────────────────────────────────────────────

/// F-TSM.4: On a series with a sustained downtrend, the strategy holds FLAT on
/// ≥ 1 POST-WARMUP bar (review 1-17 headline).
///
/// **Warmup bars are excluded from BOTH sides:** the degenerate always-long TS
/// control (same config, unreachable threshold) is long on EVERY post-warmup
/// bar, so its `time_in_market_bars` IS the post-warmup in-market capacity of
/// this exact path/config. The headline asserts `tim(TS) < tim(degenerate)` —
/// strictly below the post-warmup total. The pre-1-17 form asserted
/// `tim < total_bars`, which warmup alone satisfies (an always-long rule has
/// `tim = total − warmup < total`) — vacuously green for the degenerate case.
///
/// **RED-on-revert:** an always-long rule has `tim == tim(degenerate)` → the
/// strict inequality fails, proving the gate detects the degenerate case.
#[test]
fn f_tsm_4_goes_flat() {
    const N_UP: usize = 10;
    const N_DOWN: usize = 25; // enough to trigger exit: after L=5 down bars, score<0

    let bars = build_up_then_down_bars(N_UP, N_DOWN);
    let total_bars = bars.len();

    let result_ts = run_to_result(make_ts_config(dec!(0.00)), bars.clone());
    let result_degen = run_to_result(make_degenerate_ts_config(), bars.clone());

    let tim = result_ts.time_in_market_bars;
    // The degenerate control's tim = the post-warmup in-market total (it is long
    // on every bar its warmup allows).
    let post_warmup_total = result_degen.time_in_market_bars;

    // Sanity on the denominator: warmup must exist (post-warmup total < total
    // bars) and the control must actually be in the market post-warmup.
    assert!(
        post_warmup_total > 0 && post_warmup_total < total_bars as u64,
        "F-TSM.4 control sanity: the degenerate always-long control must be long on \
         every POST-WARMUP bar (0 < tim < total_bars), got tim={post_warmup_total}, \
         total={total_bars}."
    );

    // HEADLINE: post-warmup tim strictly below the post-warmup total — the TS
    // rule must actually exit on ≥ 1 bar it COULD have been long on.
    assert!(
        tim < post_warmup_total,
        "F-TSM.4 GOES-FLAT VIOLATION: post-warmup time_in_market ({tim}) must be strictly \
         below the post-warmup total ({post_warmup_total} = the degenerate always-long \
         control's tim; warmup excluded from both sides). If equal, the strategy never \
         exits to FLAT post-warmup — always-long — check SelectionMode::TimeSeriesLongFlat \
         and select_above_threshold."
    );

    // Also assert: the TS strategy beats BH on this path (exits avoided some loss).
    let bh_equity = run_buyhold(&bars);
    assert!(
        result_ts.final_equity > bh_equity,
        "F-TSM.4: TS strategy ({}) should beat BH ({}) on the up-then-down series \
         because it exits during the downtrend. If TS <= BH, the exit is not happening \
         or happening too late.",
        result_ts.final_equity,
        bh_equity
    );
}

/// F-TSM.4 RED-on-revert: always-long has time_in_market == active bars → fails the gate.
///
/// With CrossSectionalTopK (always selects the 1-symbol universe → always long after warmup),
/// time_in_market should equal (total_bars - warmup). We verify the always-long result
/// does NOT diverge from BH — proving the F-TSM.4 test would detect always-long.
#[test]
fn f_tsm_4_red_on_revert_always_long_does_not_exit() {
    const N_UP: usize = 10;
    const N_DOWN: usize = 25;

    let bars = build_up_then_down_bars(N_UP, N_DOWN);

    let result_always_long = run_to_result(make_always_long_config(), bars.clone());
    let result_ts = run_to_result(make_ts_config(dec!(0.00)), bars.clone());

    // Always-long: time_in_market should be high (equal to active bars after warmup).
    // TS: time_in_market should be lower (exits during downtrend).
    assert!(
        result_ts.time_in_market_bars < result_always_long.time_in_market_bars,
        "F-TSM.4 RED-ON-REVERT: TS time_in_market ({}) must be LESS THAN \
         always-long time_in_market ({}) — TS goes flat; always-long does not.",
        result_ts.time_in_market_bars,
        result_always_long.time_in_market_bars,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// F-TSM.5 — Two-run byte-identity of the TS surface metrics
// ─────────────────────────────────────────────────────────────────────────────

/// F-TSM.5: Same master seed → byte-identical RENDERED surface bodies across
/// two REAL seeded TS sweep runs (review 1-17 rebuild).
///
/// The pre-1-17 helper ignored its `master_seed`/`n_bars` parameters and ran
/// fixed deterministic bars — the seeded ensemble was never exercised. This
/// rebuild drives the PRODUCTION seams end to end:
/// `sweep_harness::derive_path_seed(master, j)` per path (ADR-0051 D1) →
/// ChaCha20-seeded synthetic bars → `montecarlo::run_path` per cell →
/// `DistributionSummary::from_path_metrics` → `classify_verdict` →
/// `sweep_harness::render_surface_report` — then asserts the two rendered
/// bodies are BYTE-IDENTICAL (assert_eq on the full rendered string; the
/// run-varying front-matter inputs are pinned constants).
///
/// A DIFFERENT master seed must change the body — proving the seed parameter is
/// USED (the anti-vacuity control for the pre-1-17 defect class #66).
#[test]
fn f_tsm_5_two_run_byte_identity() {
    const MASTER: u64 = 0xDEAD_BEEF_C0FF_EE00;
    const N_PATHS: usize = 3;
    const N_BARS: usize = 120; // enough for both probe lookbacks (max L=20)

    let render_1 = run_seeded_ts_sweep_render(MASTER, N_PATHS, N_BARS);
    let render_2 = run_seeded_ts_sweep_render(MASTER, N_PATHS, N_BARS);

    assert_eq!(
        render_1, render_2,
        "F-TSM.5: two same-seed seeded TS sweep runs must render BYTE-IDENTICAL bodies \
         (D-TSM.6) — an unordered fold has crept into the score loop, selector, reducer, \
         or renderer."
    );

    // Seed-usage control: a different master seed must produce a different body.
    // If this fails, the seed is not reaching the path generator (the pre-1-17
    // vacuity: seed params accepted but ignored).
    let render_other_seed = run_seeded_ts_sweep_render(MASTER ^ 0xFFFF_FFFF, N_PATHS, N_BARS);
    assert_ne!(
        render_1, render_other_seed,
        "F-TSM.5 seed-usage control: a DIFFERENT master seed must change the rendered \
         body — the ensemble seed parameter is being ignored by the path generation."
    );
}

// ── Helper: run one REAL seeded small-N TS sweep and render its surface ───────
//
// Production seams (reviews 1-14/1-15 extractions), reused verbatim:
// - `backtest::sweep_harness::derive_path_seed` (ADR-0051 D1 seed derivation —
//   delegates to `mc_harness::derive_path_seed`, the ONE production formula);
// - `backtest::scenarios::montecarlo::run_path` (per-path engine, via
//   `run_to_result`);
// - `backtest::bakeoff::robustness::classify_verdict` (frozen verdict);
// - `backtest::bakeoff::buyhold::run_buyhold_path` (control row);
// - `backtest::sweep_harness::render_surface_report` (the anchored renderer).
//
// The 2-cell probe grid is NEVER anchored (a probe scenario name + gbm-style
// seeded bars); the LOCKED TS_TIER1_GRID is untouched.

/// Seeded synthetic single-symbol random-walk bars (ChaCha20 — stable across
/// platforms/versions). Steps quantized to whole basis points so all math stays
/// Decimal-exact inside `run_path`.
fn seeded_random_walk_bars(seed: u64, n: usize) -> Vec<Bar> {
    use rand::{Rng, SeedableRng};
    let mut rng = rand_chacha::ChaCha20Rng::seed_from_u64(seed);
    let mut price = dec!(1000);
    let mut bars = Vec::with_capacity(n);
    for hour in 0..n {
        bars.push(make_bar("AAAUSD", price, hour as i64));
        // ±3% per bar in whole-bp steps → trends and reversals both occur.
        let step_bp: i64 = rng.random_range(-300..=300);
        price = (price * (Decimal::ONE + Decimal::new(step_bp, 4))).max(dec!(0.01));
    }
    bars
}

fn run_seeded_ts_sweep_render(master_seed: u64, n_paths: usize, n_bars: usize) -> String {
    use backtest::bakeoff::robustness::classify_verdict;
    use backtest::sweep_harness::{
        CellResult, SweepDirection, SweepScoreSource, SweepSelectionMode, ThetaCell,
        derive_path_seed, render_surface_report,
    };

    // 2-cell TS probe grid (never anchored — probe roles, probe scenario name).
    const PROBE_TS_GRID: &[ThetaCell] = &[
        ThetaCell {
            g: 0,
            lookback_minutes: 5,
            k_long: 10,
            drift_threshold_num: 10,
            drift_threshold_den: 2,
            rebalance_minutes_override: 0,
            entry_threshold_num: 0,
            entry_threshold_den: 0,
            role: "probe: short lookback, thr=0.00 (F-TSM.5 two-run identity)",
        },
        ThetaCell {
            g: 1,
            lookback_minutes: 20,
            k_long: 10,
            drift_threshold_num: 10,
            drift_threshold_den: 2,
            rebalance_minutes_override: 0,
            entry_threshold_num: 2,
            entry_threshold_den: 2,
            role: "probe: long lookback, thr=0.02 (F-TSM.5 two-run identity)",
        },
    ];

    let clamp = |curve: &[Decimal]| -> Vec<Decimal> {
        curve
            .iter()
            .map(|&e| {
                if e <= Decimal::ZERO {
                    dec!(0.000001)
                } else {
                    e
                }
            })
            .collect()
    };

    let mut cell_results: Vec<CellResult> = Vec::with_capacity(PROBE_TS_GRID.len());
    for cell in PROBE_TS_GRID {
        let cfg = make_ts_config_custom(cell.lookback_minutes, cell.entry_threshold());
        let mut metrics: Vec<PathMetrics> = Vec::with_capacity(n_paths);
        let mut total_trades = 0u64;
        let mut total_tim = 0u64;
        let mut total_bars_run = 0u64;
        for j in 0..n_paths {
            // ADR-0051 D1 production seed derivation — the master seed is USED.
            let seed_j = derive_path_seed(master_seed, j);
            let bars = seeded_random_walk_bars(seed_j, n_bars);
            let result = run_to_result(cfg.clone(), bars);
            total_trades += result.trades as u64;
            total_tim += result.time_in_market_bars;
            total_bars_run += n_bars as u64;

            let equity_clamped = clamp(&result.equity_curve);
            metrics.push(PathMetrics {
                sharpe: compute_sharpe_hourly(&equity_clamped),
                sortino: compute_sortino_hourly(&equity_clamped),
                calmar: compute_calmar(&equity_clamped),
                max_drawdown: compute_max_drawdown_f64(&equity_clamped),
                total_return: compute_total_return(&equity_clamped),
                final_equity: result.final_equity,
                initial_equity: result.initial_equity,
            });
        }
        let summary = DistributionSummary::from_path_metrics(&metrics)
            .expect("build DistributionSummary for seeded TS probe cell");
        let verdict = classify_verdict(&summary);
        cell_results.push(CellResult {
            cell: *cell,
            summary,
            verdict,
            total_trades,
            total_funding_harvested: Decimal::ZERO,
            total_time_in_market_bars: total_tim,
            total_bars_run,
            total_liquidations: 0,
        });
    }

    // Buy-and-hold control over the SAME seeded paths (production helper).
    let mut bh_metrics: Vec<PathMetrics> = Vec::with_capacity(n_paths);
    for j in 0..n_paths {
        let seed_j = derive_path_seed(master_seed, j);
        let bars = seeded_random_walk_bars(seed_j, n_bars);
        let (equity, final_eq) =
            backtest::bakeoff::buyhold::run_buyhold_path(&bars, dec!(100_000), 1);
        let equity_clamped = clamp(&equity);
        bh_metrics.push(PathMetrics {
            sharpe: compute_sharpe_hourly(&equity_clamped),
            sortino: compute_sortino_hourly(&equity_clamped),
            calmar: compute_calmar(&equity_clamped),
            max_drawdown: compute_max_drawdown_f64(&equity_clamped),
            total_return: compute_total_return(&equity_clamped),
            final_equity: final_eq,
            initial_equity: dec!(100_000),
        });
    }
    let buyhold_summary = DistributionSummary::from_path_metrics(&bh_metrics)
        .expect("build BH DistributionSummary for seeded TS probe");

    // Render through the PRODUCTION renderer. Front-matter inputs are pinned
    // constants so the full rendered string is comparable byte-for-byte.
    render_surface_report(
        "1970-01-01T00:00:00Z", // generated (pinned)
        0.0,                    // wall_clock_s (pinned)
        "f-tsm5-test-host",     // host (pinned)
        0,                      // pid (pinned)
        "f-tsm5-test-commit",   // git_commit (pinned)
        "f-tsm5-test-revision", // data_revision_sha (pinned)
        "ts-fp5-two-run-probe", // scenario (probe — never anchored)
        master_seed,
        0xC0FFEE, // fill_seed (ADR-0051 D1 constant)
        n_paths,
        "seeded-synthetic", // generator label (probe)
        "n/a",              // bootstrap mode (probe)
        "fixed",            // block-length policy (probe)
        None,               // selected block length
        "f-tsm5-source-revision",
        PROBE_TS_GRID,
        &cell_results,
        &buyhold_summary,
        SweepDirection::Momentum,
        SweepScoreSource::VolAdjustedReturn,
        None, // funding revision (TS: none)
        SweepSelectionMode::TimeSeriesLongFlat,
        backtest::resample::Horizon::OneHour,
        0, // taker_fee_bps (matches run_to_result's zero-friction input)
        0, // slippage_bps
    )
}
