//! ADR-0071 advisor-signal-library-expansion — day-1 divergence e2e gate.
//!
//! ## Gate (CLAUDE.md non-negotiable + R-SL.5)
//!
//! Every new arm ships with a baseline-equity-divergence e2e test. For each of
//! the 5 new pre-registered arms, this test asserts:
//!
//! 1. **Diverges from ≥ 1 existing base arm** — final equity differs from at
//!    least one of SMA/MACD/RSI/BBands by ≥ 1 bp of initial capital.
//!    Proves no silent alias/no-op.
//!
//! 2. **Diverges from buy-and-hold** — each new arm's equity differs from
//!    always-long by ≥ 1 bp. Proves it actually gates trades.
//!
//! 3. **No two new arms produce identical curves** — the 5 new arms are
//!    pairwise distinct on the same series.
//!
//! 4. **FAIL-before / PASS-after** — a never-firing breakout arm (e.g.
//!    `close > max(high, 20)` which is structurally impossible with current-bar-
//!    inclusive RollingMax) would sit in cash = 100_000 (initial capital). The
//!    test detects this vacuous case by asserting each arm's terminal equity
//!    differs from the initial capital by ≥ 1 bp (meaning it actually TRADED).
//!    The corrected signal `high >= max(high, 20)` fires on the spike bar and
//!    produces equity ≠ 100_000.
//!
//! 5. **Factory smoke** — each real `config/strategies/<stem>.toml` loads via
//!    `ComposedStrategyConfig::from_file` and its parsed `id` equals the stem.
//!
//! ## Series construction (101 bars, corrected for breakout signals)
//!
//! The series is purpose-built so each of the 5 new arms fires AND exits at a
//! structurally distinct price, producing pairwise-distinct terminal equity (≥ 1 bp):
//!
//! - **Flat (bars 0-49)**: close=high=low=100, volume=50.
//!   Builds all indicators: avg/max/min windows, sma(50), obv=0, obv_avg(20)=0.
//!
//! - **Spike (bar 50)**: close=200, high=220, low=100, volume=400.
//!   All 5 arms enter:
//!   - `high(220) >= max(high,20)=100` → breakout arms BUY.
//!   - `vol(400) > 2*avg(vol,20)=100` → vol_breakout BUY.
//!   - `close(200) > avg(close,10)×1.05=115.5` → roc_momentum BUY.
//!   - `close(200) > min(low,20)=100` (rule was false on flat bars) → donchian_floor BUY.
//!   - `OBV(400) > obv_avg(20)=20 AND close(200) > sma(50)=100` → OBV arm BUY.
//!
//! - **Bar 51** (new high, low volume): close=205, high=225, low=200, volume=30.
//!   - `high(225) >= max(high,20)=225` → donchian_break stays long.
//!   - `vol(30) < 2*avg(vol,20)≈133` → vol_breakout SELLS at close=205.
//!   - Other arms stay long.
//!
//! - **Bar 52** (decline): close=190, high=195, low=180, volume=50.
//!   - `high(195) < max(high,20)=225` → donchian_break SELLS at close=190.
//!   - roc_momentum stays long (avg(close,10)≈128, 190 > 134.4).
//!   - OBV arm stays long.
//!   - donchian_floor stays long.
//!
//! - **Bar 53** (sharp drop): close=130, high=135, low=120, volume=50.
//!   - roc_momentum: avg(close,10)=132.5, 130 < 139.1 → SELLS at close=130.
//!   - OBV arm stays long (OBV=330 > obv_avg(20)≈77).
//!   - donchian_floor stays long.
//!
//! - **Decline (bars 54-100)**: close drops 2/bar (128→36), low=close-10, high=close.
//!   - OBV: accumulates -50/bar on each down bar.
//!   - OBV arm SELLS at bar 59 (close=118) when OBV(30) < obv_avg(20)≈119.5.
//!   - donchian_floor NEVER exits (close > min(low,20) = close-10, always true).
//!
//! Terminal equities (buy=200, qty=50, cash_base=90000):
//!   - vol_breakout:   sell @205 → 90000 + 50×205 = 100250
//!   - donchian_break: sell @190 → 90000 + 50×190 =  99500
//!   - roc_momentum:   sell @130 → 90000 + 50×130 =  96500
//!   - obv:            sell @118 → 90000 + 50×118 =  95900
//!   - donchian_floor: hold @36  → 90000 + 50×36  =  91800
//!   - buy-and-hold:   hold @36, qty=100 → 90000 + 100×36 = 93600

#![allow(clippy::float_arithmetic, clippy::unwrap_used)]

use std::path::Path;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{Bar, Price, Quantity, SignalKind, Symbol, Timeframe, Timestamp, Venue};

use strategy::traits::Strategy;
use strategy::{ComposedStrategy, ComposedStrategyConfig};

// ── Constants ─────────────────────────────────────────────────────────────────

const INITIAL_CAPITAL: Decimal = dec!(100_000);
/// 1 basis point of initial capital.
const ONE_BP: Decimal = dec!(10); // 0.01% of 100_000

// ── Bar builder ────────────────────────────────────────────────────────────────

fn make_bar(idx: usize, close: Decimal, high: Decimal, low: Decimal, volume: Decimal) -> Bar {
    let epoch = OffsetDateTime::UNIX_EPOCH;
    let open_ts = Timestamp::new(epoch + time::Duration::hours(idx as i64));
    let close_ts =
        Timestamp::new(epoch + time::Duration::hours(idx as i64) + time::Duration::minutes(59));
    let mk_p = |v: Decimal| Price::new(v.max(dec!(0.01))).unwrap();
    let mk_q = |v: Decimal| Quantity::new(v.max(dec!(0.01))).unwrap();
    Bar {
        symbol: Symbol::new("BTCUSDT"),
        tf: Timeframe::OneHour,
        open: mk_p(close),
        high: mk_p(high),
        low: mk_p(low),
        close: mk_p(close),
        volume: mk_q(volume),
        open_ts,
        close_ts,
        trade_count: 10,
        local_recv_ts: open_ts,
        venue: Venue::Binance,
    }
}

/// Build the purpose-built deterministic bar series (101 bars).
///
/// Designed so each of the 5 new arms fires AND exits at a distinct close price,
/// producing pairwise-distinct terminal equity (≥ 1 bp). See module-level doc for
/// the full design rationale and per-bar breakdown.
fn build_bars() -> Vec<Bar> {
    let mut bars = Vec::with_capacity(101);

    // ── Flat phase (bars 0-49) ──────────────────────────────────────────────
    // close=high=low=100, vol=50. Builds all 50-bar indicators:
    // avg(close,20)=100, avg(vol,20)=50, max(high,20)=100, min(low,20)=100,
    // sma(50)=100 at bar 49, obv=0, obv_avg(20)=0.
    // donchian_floor rule: close(100) > min(low,20)(100) → FALSE (100 > 100 is false).
    for i in 0..50 {
        bars.push(make_bar(i, dec!(100), dec!(100), dec!(100), dec!(50)));
    }

    // ── Spike bar (bar 50) ─────────────────────────────────────────────────
    // All 5 arms enter simultaneously:
    // - donchian_break: high(220) >= max(high,20)=100 → TRUE → BUY at 200
    // - vol_breakout: high(220)>=100 AND vol(400)>2*50=100 → TRUE → BUY at 200
    // - roc_momentum: close(200) > avg(close,20)=105 × 1.05=110.25 → TRUE → BUY at 200
    // - donchian_floor: rule was FALSE on flat bars; close(200)>min(low,20)=100 → TRUE → BUY at 200
    // - OBV arm: OBV(400)>obv_avg(20)=20 AND close(200)>sma(50)=100 → TRUE → BUY at 200
    bars.push(make_bar(50, dec!(200), dec!(220), dec!(100), dec!(400)));

    // ── Bar 51: new 20-bar high, low volume ────────────────────────────────
    // high=225 > previous max=220 → still a new 20-bar high.
    // donchian_break: high(225)>=max(high,20)=225 → stays long.
    // vol_breakout: high(225)>=225 AND vol(30). avg(vol,20) of bars 32-51:
    //   (18×50 + 400 + 30)/20 = 1330/20 = 66.5. 2×66.5=133. 30>133? NO → SELLS at close=205.
    // Other arms remain long (close > their respective thresholds).
    bars.push(make_bar(51, dec!(205), dec!(225), dec!(200), dec!(30)));

    // ── Bar 52: decline — donchian_break exits ─────────────────────────────
    // high(195) < max(high,20)=225 (spike bar's 225 stays in window until bar 71).
    // donchian_break: 195>=225? NO → SELLS at close=190.
    // roc_momentum: avg(close,10) of bars 43-52 = (7×100+200+205+190)/10=
    //   (700+595)/10=129.5. 190 > 129.5×1.05=135.975? YES → stays long.
    // OBV arm: stays long (OBV declining but above obv_avg(20)).
    // donchian_floor: min(low,20)=100 (historical bars still in window). stays long.
    bars.push(make_bar(52, dec!(190), dec!(195), dec!(180), dec!(50)));

    // ── Bar 53: sharp drop — roc_momentum exits ────────────────────────────
    // avg(close,10) of bars 44-53 = (6×100+200+205+190+130)/10=
    //   (600+725)/10=132.5. 130 > 132.5×1.05=139.125? NO → SELLS at close=130.
    // OBV at bar 53: 400+30-50-50=330. obv_avg(20)≈77. 330>77 → stays long.
    // donchian_floor: min(low,20)=100. 130>100 → stays long.
    bars.push(make_bar(53, dec!(130), dec!(135), dec!(120), dec!(50)));

    // ── Decline phase (bars 54-100): close drops 2/bar ────────────────────
    // close = 128, 126, 124, …  low = close - 10  high = close  vol = 50
    // OBV arm exits at bar 59 (close=118) when OBV(30) < obv_avg(20)≈119.5.
    // donchian_floor never exits: close > min(low,20) = close-10 (always true).
    for i in 0..47usize {
        let close = dec!(128) - Decimal::from(i) * dec!(2);
        let close = close.max(dec!(5));
        let low = (close - dec!(10)).max(dec!(1));
        bars.push(make_bar(54 + i, close, close, low, dec!(50)));
    }

    bars
}

// ── Position simulator ────────────────────────────────────────────────────────

/// Simple long-only position sim. Returns the equity curve.
/// Fixed fraction: buys 10% of cash on Buy signal, liquidates on Sell.
fn run_composed_equity(strategy: &mut ComposedStrategy, bars: &[Bar]) -> Vec<Decimal> {
    let mut cash = INITIAL_CAPITAL;
    let mut qty = Decimal::ZERO;
    let mut curve = vec![cash];

    for bar in bars {
        let signals = strategy.on_bar(bar);
        let close = bar.close.get();

        for sig in &signals {
            match sig.kind {
                SignalKind::Buy if qty <= Decimal::ZERO => {
                    let spend = cash * dec!(0.1);
                    if spend > Decimal::ZERO && close > Decimal::ZERO {
                        qty += spend / close;
                        cash -= spend;
                    }
                }
                SignalKind::Sell if qty > Decimal::ZERO => {
                    cash += qty * close;
                    qty = Decimal::ZERO;
                }
                _ => {}
            }
        }

        curve.push(cash + qty * close);
    }

    curve
}

/// Count the number of Buy + Sell trades executed by the strategy.
fn count_trades(strategy: &mut ComposedStrategy, bars: &[Bar]) -> usize {
    let mut count = 0;
    for bar in bars {
        let signals = strategy.on_bar(bar);
        count += signals.len();
    }
    count
}

/// Buy-and-hold equity: buys 10% at bar 0, never sells.
fn buyhold_final_equity(bars: &[Bar]) -> Decimal {
    let mut cash = INITIAL_CAPITAL;
    let mut qty = Decimal::ZERO;

    for (i, bar) in bars.iter().enumerate() {
        let close = bar.close.get();
        if i == 0 && close > Decimal::ZERO {
            let spend = cash * dec!(0.1);
            qty += spend / close;
            cash -= spend;
        }
    }

    if let Some(last) = bars.last() {
        cash + qty * last.close.get()
    } else {
        cash
    }
}

/// Build a `ComposedStrategy` from an inline TOML string.
fn strategy_from_toml(toml: &str, id: &str) -> ComposedStrategy {
    let cfg = ComposedStrategyConfig::from_str(toml, id).expect("TOML must parse");
    ComposedStrategy::from_config(cfg, smol_str::SmolStr::new(id))
}

// ── TOML strings for the 5 new arms (mirrors config/strategies/*.toml) ────────

/// Corrected: `high >= max(high, 20)` — fires when the current bar makes a new 20-bar high.
const TOML_DONCHIAN_BREAK: &str = r#"
id     = "btc_donchian_break"
kind   = "composed"
symbol = "BTCUSDT"
stage  = "research"
signal = "high >= max(high, 20)"
size   = "fixed_fraction(0.1)"
"#;

const TOML_DONCHIAN_FLOOR: &str = r#"
id     = "btc_donchian_floor"
kind   = "composed"
symbol = "BTCUSDT"
stage  = "research"
signal = "close > min(low, 20)"
size   = "fixed_fraction(0.1)"
"#;

/// Corrected: `high >= max(high, 20) AND volume > 2 * avg(volume, 20)`.
const TOML_VOL_BREAKOUT: &str = r#"
id     = "btc_vol_breakout"
kind   = "composed"
symbol = "BTCUSDT"
stage  = "research"
signal = "high >= max(high, 20) AND volume > 2 * avg(volume, 20)"
size   = "fixed_fraction(0.1)"
"#;

const TOML_ROC_MOMENTUM: &str = r#"
id     = "btc_roc_momentum"
kind   = "composed"
symbol = "BTCUSDT"
stage  = "research"
signal = "close > avg(close, 10) * 1.05"
size   = "fixed_fraction(0.1)"
"#;

/// Restored to architect-ratified obv_avg(20) (ADR-0071). Period 10 was a test-accommodation hack.
const TOML_OBV: &str = r#"
id     = "btc_obv"
kind   = "composed"
symbol = "BTCUSDT"
stage  = "research"
signal = "obv() > obv_avg(20) AND close > sma(50)"
size   = "fixed_fraction(0.1)"
"#;

// ── Existing base arm TOMLs (for comparison) ──────────────────────────────────

const TOML_SMA: &str = r#"
id     = "btc_sma_cross"
kind   = "composed"
symbol = "BTCUSDT"
stage  = "research"
signal = "close > sma(50)"
size   = "fixed_fraction(0.1)"
"#;

const TOML_MACD: &str = r#"
id     = "btc_macd_trend"
kind   = "composed"
symbol = "BTCUSDT"
stage  = "research"
signal = "macd_hist(12,26,9) > 0 AND close > ema(200)"
size   = "fixed_fraction(0.1)"
"#;

const TOML_RSI: &str = r#"
id     = "btc_rsi_reversion"
kind   = "composed"
symbol = "BTCUSDT"
stage  = "research"
signal = "rsi(14) < 30 AND close > min(low, 20)"
size   = "fixed_fraction(0.1)"
"#;

const TOML_BBANDS: &str = r#"
id     = "btc_bbands_mean_revert"
kind   = "composed"
symbol = "BTCUSDT"
stage  = "research"
signal = "close < bollinger_lower(20, 2) AND volume > 1.5 * avg(volume, 20)"
size   = "fixed_fraction(0.1)"
"#;

// ── Test 0: FAIL-before — each arm actually TRADED (non-vacuous divergence) ───

/// Anti-vacuous gate: every new arm must execute ≥ 1 trade on this series.
///
/// A never-firing breakout arm (e.g. the infeasible `close > max(high, 20)`)
/// stays in cash = initial_capital the whole time. Its equity differs from
/// buy-and-hold by ~6400 (> 1bp), so the divergence test would PASS vacuously
/// for a no-op arm. This gate blocks that: if terminal equity == initial_capital
/// (never traded), the arm is a no-op and this test FAILS.
///
/// With the corrected signals, each arm fires a Buy on bar 50, ensuring at
/// least 1 trade and equity ≠ 100_000.
#[test]
fn each_new_arm_actually_traded_not_vacuous() {
    let bars = build_bars();

    let new_arms: &[(&str, &str)] = &[
        (TOML_DONCHIAN_BREAK, "btc_donchian_break"),
        (TOML_DONCHIAN_FLOOR, "btc_donchian_floor"),
        (TOML_VOL_BREAKOUT, "btc_vol_breakout"),
        (TOML_ROC_MOMENTUM, "btc_roc_momentum"),
        (TOML_OBV, "btc_obv"),
    ];

    for (toml, id) in new_arms {
        let eq = *run_composed_equity(&mut strategy_from_toml(toml, id), &bars)
            .last()
            .unwrap();

        // Terminal equity must differ from initial capital — if equal, the arm
        // never traded (no-op / never-firing signal).
        assert!(
            (eq - INITIAL_CAPITAL).abs() >= ONE_BP,
            "arm {id}: terminal equity ({eq}) == initial capital ({INITIAL_CAPITAL}); \
             the arm never traded — signal must fire at least once on this series. \
             FAIL-before: a structurally-impossible breakout signal (e.g. `close > max(high, 20)`) \
             would stay in cash and fail this check."
        );
    }
}

// ── Test 1: each new arm diverges from ≥ 1 existing base arm ─────────────────

#[test]
fn each_new_arm_diverges_from_at_least_one_existing_arm() {
    let bars = build_bars();
    let bh = buyhold_final_equity(&bars);

    // Run existing arms.
    let eq_sma = *run_composed_equity(&mut strategy_from_toml(TOML_SMA, "btc_sma_cross"), &bars)
        .last()
        .unwrap();
    let eq_macd = *run_composed_equity(&mut strategy_from_toml(TOML_MACD, "btc_macd_trend"), &bars)
        .last()
        .unwrap();
    let eq_rsi = *run_composed_equity(
        &mut strategy_from_toml(TOML_RSI, "btc_rsi_reversion"),
        &bars,
    )
    .last()
    .unwrap();
    let eq_bb = *run_composed_equity(
        &mut strategy_from_toml(TOML_BBANDS, "btc_bbands_mean_revert"),
        &bars,
    )
    .last()
    .unwrap();

    let existing = [eq_sma, eq_macd, eq_rsi, eq_bb];
    let existing_min = existing.iter().copied().reduce(Decimal::min).unwrap();
    let existing_max = existing.iter().copied().reduce(Decimal::max).unwrap();

    // Run each new arm.
    let new_arms: &[(&str, &str)] = &[
        (TOML_DONCHIAN_BREAK, "btc_donchian_break"),
        (TOML_DONCHIAN_FLOOR, "btc_donchian_floor"),
        (TOML_VOL_BREAKOUT, "btc_vol_breakout"),
        (TOML_ROC_MOMENTUM, "btc_roc_momentum"),
        (TOML_OBV, "btc_obv"),
    ];

    for (toml, id) in new_arms {
        let eq = *run_composed_equity(&mut strategy_from_toml(toml, id), &bars)
            .last()
            .unwrap();

        // Diverges from buy-and-hold.
        assert!(
            (eq - bh).abs() >= ONE_BP,
            "arm {id}: terminal equity ({eq}) must differ from buy-and-hold ({bh}) by ≥ 1 bp; \
             diff = {}",
            (eq - bh).abs()
        );

        // Diverges from at least one existing arm.
        let diverges = (eq - eq_sma).abs() >= ONE_BP
            || (eq - eq_macd).abs() >= ONE_BP
            || (eq - eq_rsi).abs() >= ONE_BP
            || (eq - eq_bb).abs() >= ONE_BP;
        assert!(
            diverges,
            "arm {id}: equity ({eq}) must differ from at least one existing arm by ≥ 1 bp; \
             existing range [{existing_min}, {existing_max}]"
        );
    }
}

// ── Test 2: buy-and-hold divergence (separately spelled out) ─────────────────

#[test]
fn each_new_arm_diverges_from_buyhold() {
    let bars = build_bars();
    let bh = buyhold_final_equity(&bars);

    let new_arms: &[(&str, &str)] = &[
        (TOML_DONCHIAN_BREAK, "btc_donchian_break"),
        (TOML_DONCHIAN_FLOOR, "btc_donchian_floor"),
        (TOML_VOL_BREAKOUT, "btc_vol_breakout"),
        (TOML_ROC_MOMENTUM, "btc_roc_momentum"),
        (TOML_OBV, "btc_obv"),
    ];

    for (toml, id) in new_arms {
        let eq = *run_composed_equity(&mut strategy_from_toml(toml, id), &bars)
            .last()
            .unwrap();
        assert!(
            (eq - bh).abs() >= ONE_BP,
            "arm {id}: equity ({eq}) must differ from buy-and-hold ({bh}) by ≥ 1 bp; \
             diff = {}",
            (eq - bh).abs()
        );
    }
}

// ── Test 3: no two new arms produce identical equity curves ───────────────────

#[test]
fn no_two_new_arms_produce_identical_curves() {
    let bars = build_bars();

    let mut curves: Vec<(&str, Vec<Decimal>)> = vec![
        (
            "btc_donchian_break",
            run_composed_equity(
                &mut strategy_from_toml(TOML_DONCHIAN_BREAK, "btc_donchian_break"),
                &bars,
            ),
        ),
        (
            "btc_donchian_floor",
            run_composed_equity(
                &mut strategy_from_toml(TOML_DONCHIAN_FLOOR, "btc_donchian_floor"),
                &bars,
            ),
        ),
        (
            "btc_vol_breakout",
            run_composed_equity(
                &mut strategy_from_toml(TOML_VOL_BREAKOUT, "btc_vol_breakout"),
                &bars,
            ),
        ),
        (
            "btc_roc_momentum",
            run_composed_equity(
                &mut strategy_from_toml(TOML_ROC_MOMENTUM, "btc_roc_momentum"),
                &bars,
            ),
        ),
        (
            "btc_obv",
            run_composed_equity(&mut strategy_from_toml(TOML_OBV, "btc_obv"), &bars),
        ),
    ];

    // Pairwise: no two curves are identical (at least ONE_BP apart in terminal equity).
    for i in 0..curves.len() {
        for j in (i + 1)..curves.len() {
            let (id_a, curve_a) = &curves[i];
            let (id_b, curve_b) = &curves[j];
            let eq_a = *curve_a.last().unwrap();
            let eq_b = *curve_b.last().unwrap();
            assert!(
                (eq_a - eq_b).abs() >= ONE_BP,
                "arms {id_a} and {id_b} produce identical terminal equity ({eq_a}); \
                 they must be pairwise distinct (≥ 1 bp apart)"
            );
        }
    }
    // Suppress unused_mut warning.
    let _ = curves.iter_mut();
}

// ── Test 4: FAIL-before contract (aliasing a signal → identical curve) ────────

/// Prove FAIL-before: the corrected breakout arms produce DISTINCT curves.
///
/// If `donchian_break` were aliased to the SMA signal (`close > avg(close, 20)`),
/// it would trade on a MUCH longer horizon (exits when the 20-bar avg catches close),
/// producing different equity from the real breakout signal. The test confirms the
/// real signals are pairwise-distinct, which would be violated by aliasing to any
/// existing arm's logic.
///
/// The critical FAIL-before case is captured by `each_new_arm_actually_traded_not_vacuous`:
/// an infeasible signal (never fires) fails that test; a valid breakout signal
/// (fires on bar 50 at the spike) passes it. These two tests together constitute
/// the complete FAIL-before / PASS-after gate.
#[test]
fn fail_before_aliasing_donchian_break_to_floor_would_be_identical() {
    let bars = build_bars();
    // The two real arms must NOT be identical — if one were aliased to the other's
    // signal, the pairwise test above would fail. Confirm they differ here.
    let eq_break = *run_composed_equity(
        &mut strategy_from_toml(TOML_DONCHIAN_BREAK, "btc_donchian_break"),
        &bars,
    )
    .last()
    .unwrap();
    let eq_floor = *run_composed_equity(
        &mut strategy_from_toml(TOML_DONCHIAN_FLOOR, "btc_donchian_floor"),
        &bars,
    )
    .last()
    .unwrap();
    assert!(
        (eq_break - eq_floor).abs() >= ONE_BP,
        "donchian_break ({eq_break}) and donchian_floor ({eq_floor}) must diverge on this series; \
         if they appear equal then they are effectively aliased (FAIL-before would be triggered)"
    );
}

/// Prove FAIL-before: the vol_breakout and donchian_break arms diverge.
///
/// Vol_breakout is strictly tighter (requires BOTH new high AND volume surge).
/// On bar 51 (new high, low volume), vol_breakout exits while donchian_break stays.
/// Their distinct exit prices (205 vs 190) produce distinct terminal equity.
#[test]
fn fail_before_vol_breakout_and_donchian_break_are_distinct() {
    let bars = build_bars();
    let eq_vol = *run_composed_equity(
        &mut strategy_from_toml(TOML_VOL_BREAKOUT, "btc_vol_breakout"),
        &bars,
    )
    .last()
    .unwrap();
    let eq_break = *run_composed_equity(
        &mut strategy_from_toml(TOML_DONCHIAN_BREAK, "btc_donchian_break"),
        &bars,
    )
    .last()
    .unwrap();
    assert!(
        (eq_vol - eq_break).abs() >= ONE_BP,
        "vol_breakout ({eq_vol}) and donchian_break ({eq_break}) must diverge — \
         vol_breakout adds a volume-surge gate that causes an earlier exit on bar 51 \
         (new high but low volume). If they are equal, the volume gate is not working."
    );
}

// ── Test 5: factory smoke — real TOMLs load from disk with id == stem ─────────

#[test]
fn factory_smoke_real_tomls_load_with_correct_id() {
    let workspace = std::env::var("CARGO_MANIFEST_DIR")
        .map(|d| {
            // crates/strategy → workspace root
            Path::new(&d)
                .parent()
                .and_then(|p| p.parent())
                .unwrap()
                .to_path_buf()
        })
        .unwrap();

    let stems_and_ids: &[(&str, &str)] = &[
        ("btc_donchian_break", "btc_donchian_break"),
        ("btc_donchian_floor", "btc_donchian_floor"),
        ("btc_vol_breakout", "btc_vol_breakout"),
        ("btc_roc_momentum", "btc_roc_momentum"),
        ("btc_obv", "btc_obv"),
    ];

    for (stem, expected_id) in stems_and_ids {
        let path = workspace
            .join("config")
            .join("strategies")
            .join(format!("{stem}.toml"));
        let cfg = ComposedStrategyConfig::from_file(&path)
            .unwrap_or_else(|e| panic!("TOML {stem}.toml must load: {e}"));
        assert_eq!(
            cfg.id, *expected_id,
            "TOML {stem}.toml: parsed id `{}` must equal stem `{expected_id}`",
            cfg.id
        );
    }
}

// ── Test 6: factory smoke — real TOMLs load AND actually fire signals ──────────

/// Verify that the real on-disk TOMLs fire at least 1 signal on the test series.
/// This catches a mismatch between the inline test TOMLs (above) and the on-disk
/// config files — if the on-disk TOML has a structurally-infeasible signal, this fails.
#[test]
fn factory_smoke_real_tomls_fire_at_least_one_signal() {
    let workspace = std::env::var("CARGO_MANIFEST_DIR")
        .map(|d| {
            Path::new(&d)
                .parent()
                .and_then(|p| p.parent())
                .unwrap()
                .to_path_buf()
        })
        .unwrap();

    let bars = build_bars();

    let stems: &[&str] = &[
        "btc_donchian_break",
        "btc_donchian_floor",
        "btc_vol_breakout",
        "btc_roc_momentum",
        "btc_obv",
    ];

    for stem in stems {
        let path = workspace
            .join("config")
            .join("strategies")
            .join(format!("{stem}.toml"));
        let cfg = ComposedStrategyConfig::from_file(&path)
            .unwrap_or_else(|e| panic!("TOML {stem}.toml must load: {e}"));
        let mut strategy = ComposedStrategy::from_config(cfg, smol_str::SmolStr::new(*stem));
        let trade_count = count_trades(&mut strategy, &bars);
        assert!(
            trade_count >= 1,
            "TOML {stem}.toml fired 0 signals on the test series — \
             the signal may be structurally infeasible (never fires). \
             Check that the on-disk TOML matches the corrected ADR-0071 signal."
        );
    }
}

// ── Supplemental: describe_plan no-panic gate (from T13) ─────────────────────

/// Assert `describe_plan` returns the `SmaCross` fallback (no panic) for
/// each new arm id (ADR-0071 Q-SL-4: describe_plan deferred to follow-on;
/// fallback is the safety net — node.rs:1358).
#[test]
fn describe_plan_no_panic_for_new_arm_ids() {
    use strategy::{PlanContext, PlanDescribe, PlanRuleShape};

    let new_ids: &[(&str, &str)] = &[
        (TOML_DONCHIAN_BREAK, "btc_donchian_break"),
        (TOML_DONCHIAN_FLOOR, "btc_donchian_floor"),
        (TOML_VOL_BREAKOUT, "btc_vol_breakout"),
        (TOML_ROC_MOMENTUM, "btc_roc_momentum"),
        (TOML_OBV, "btc_obv"),
    ];

    use trading_core::{Money, Price, Timestamp, Usdt};
    let bars = build_bars();
    let ctx = PlanContext {
        last_close: Price::new(dec!(35_000)).unwrap(),
        last_bar_ts: Timestamp::new(
            OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(1_700_000_000),
        ),
        budget: Money::<Usdt>::from_decimal(dec!(200)),
        budget_cap: Money::<Usdt>::from_decimal(dec!(1000)),
    };

    for (toml, id) in new_ids {
        let mut strategy = strategy_from_toml(toml, id);
        // Run at least one bar so `last_rule_value` is Some.
        if let Some(bar) = bars.first() {
            strategy.on_bar(bar);
        }
        // `describe_plan` must NOT panic for any new arm id.
        let plan = strategy.describe_plan(&ctx);
        // The fallback is SmaCross{20,50} — verify it returns that shape
        // (no panic, and returns the generic shape, not a custom one we haven't
        // implemented yet).
        assert!(
            matches!(
                plan.rule,
                PlanRuleShape::SmaCross {
                    fast_len: 20,
                    slow_len: 50
                }
            ),
            "arm {id}: describe_plan must return the SmaCross fallback (no panic); \
             got {plan:?}"
        );
    }
}
