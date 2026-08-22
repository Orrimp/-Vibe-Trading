//! ADR-0089 D5 — the two portfolio controls, proven to BIND.
//!
//! Bug-log **#68** (drift hold band inert) and **#69** (portfolio exposure cap
//! inert) are ONE defect: `risk::size_portfolio_target` implements both, and it
//! had zero production callers. Both were configured, range-validated, and
//! printed into hashed report bodies; neither was ever consulted.
//!
//! A gate for a limit that cannot fail is exactly what produced #69, so every
//! test here is a PAIR over the same fixture with ONE parameter changed. The
//! pairing is the evidence: if the control were still inert both halves would
//! return the same numbers and the test would fail. Nothing here asserts a
//! property that holds when the control is switched off.
//!
//! | test | control | the pair |
//! |---|---|---|
//! | `gross_cap_refuses_the_whole_rebalance_and_the_count_is_surfaced` | #69 cap | `exposure_cap` 0.25 (breach) vs 0.50 (fits) |
//! | `drift_band_suppresses_resizes_that_a_tight_band_performs` | #68 band | `drift_rebalance_threshold` 0.001 vs 0.90 |
//! | `a_held_leg_does_not_accumulate_across_rebalances` | #94 | absolute vs delta order sizing |

use backtest::cli_types::TcnScenarioInput;
use backtest::scenarios::montecarlo::{PathRunResult, run_path};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

const INITIAL_CAPITAL: Decimal = dec!(100_000);

fn make_ts(hour: i64) -> Timestamp {
    Timestamp::new(
        time::OffsetDateTime::from_unix_timestamp(1_672_531_200).expect("valid epoch_2023")
            + time::Duration::hours(hour),
    )
}

fn make_bar(sym: &str, close: Decimal, hour: i64) -> Bar {
    Bar {
        symbol: Symbol::new(sym),
        tf: Timeframe::OneHour,
        open_ts: make_ts(hour),
        close_ts: make_ts(hour),
        local_recv_ts: make_ts(hour),
        venue: Venue::Binance,
        open: Price::new(close).expect("positive"),
        high: Price::new(close).expect("positive"),
        low: Price::new(close).expect("positive"),
        close: Price::new(close).expect("positive"),
        volume: Quantity::new(dec!(100)).expect("positive"),
        trade_count: 1,
    }
}

/// `n_symbols` series, each rising `per_bar_pct` per bar, merged and sorted by
/// (ts, symbol) exactly as the production harness feeds `run_path`.
fn build_bars(n_symbols: usize, n_bars: usize, per_bar_pct: Decimal) -> Vec<Bar> {
    let mut bars = Vec::new();
    for s in 0..n_symbols {
        let sym = format!("{}AAUSD", (b'A' + u8::try_from(s).expect("small")) as char);
        // Distinct starting prices so the cross-sectional scores are not tied.
        let mut price = dec!(1000) + Decimal::from(s) * dec!(7);
        for hour in 0..n_bars {
            bars.push(make_bar(&sym, price, i64::try_from(hour).expect("small")));
            price *= Decimal::ONE + per_bar_pct;
        }
    }
    bars.sort_by(|a, b| a.open_ts.cmp(&b.open_ts).then(a.symbol.0.cmp(&b.symbol.0)));
    bars
}

fn make_config(
    n_symbols: usize,
    k_long: u32,
    exposure_cap: Decimal,
    drift: Decimal,
) -> strategy::CrossSectionalMomentumConfig {
    let universe = (0..n_symbols)
        .map(|s| {
            SmolStr::new(format!(
                "{}AAUSD",
                (b'A' + u8::try_from(s).expect("small")) as char
            ))
        })
        .collect();
    strategy::CrossSectionalMomentumConfig {
        id: SmolStr::new("portfolio_controls_bind"),
        universe,
        lookback_minutes: 3,
        rebalance_minutes: 1,
        k_long,
        k_short: 0,
        exposure_cap,
        drift_rebalance_threshold: drift,
        vol_floor: dec!(0.000001),
        stage: SmolStr::new("research"),
        direction: strategy::Direction::Momentum,
        score_source: strategy::ScoreSource::VolAdjustedReturn,
        selection_mode: strategy::SelectionMode::CrossSectionalTopK,
        entry_threshold: Decimal::ZERO,
    }
}

fn run(cfg: strategy::CrossSectionalMomentumConfig, bars: Vec<Bar>) -> PathRunResult {
    let strat = strategy::MomentumStrategy::from_config(cfg, SmolStr::new("portfolio_controls"));
    let input = TcnScenarioInput {
        scenario_name: "portfolio-controls-bind".to_string(),
        start_year: 2023,
        bar_count: bars.len(),
        initial_capital: INITIAL_CAPITAL,
        // Zero friction: any equity or trade-count difference between the halves
        // of a pair is the CONTROL, never fee drag.
        slippage_bps: 0,
        taker_fee_bps: 0,
        config_id: "portfolio_controls_bind".to_string(),
        forecaster_id: "test".to_string(),
        bars_override: Some(bars),
        emit_equity_bin: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        funding_override: None,
        bar_span_hours: 1,
    };
    pollster::block_on(run_path(input, 0xC0FFEE, strat)).expect("run_path must succeed")
}

/// **#69 — the portfolio exposure cap actually REFUSES an over-cap vector, and
/// the refusal is COUNTED (ADR-0089 D2).**
///
/// Three legs at `FIXED_FRACTION = 0.10` propose 0.30 gross. Against a 0.25 cap
/// that is a breach on the very first rebalance, so the whole vector is skipped
/// and nothing is ever bought. Against a 0.50 cap the same vector fits.
///
/// RED WITHOUT THE CONTROL: with the cap inert (its state before this change,
/// and its state for the entire anchored corpus) the 0.25 half trades exactly
/// like the 0.50 half — `breaches == 0`, `trades > 0` — and BOTH assertions on
/// the breach half fail. The pair cannot pass unless the cap is consulted.
#[test]
fn gross_cap_refuses_the_whole_rebalance_and_the_count_is_surfaced() {
    let bars = build_bars(3, 40, dec!(0.01));

    let breached = run(make_config(3, 3, dec!(0.25), dec!(0.10)), bars.clone());
    let fits = run(make_config(3, 3, dec!(0.50), dec!(0.10)), bars);

    assert!(
        breached.portfolio_breaches > 0,
        "#69: a 0.30-gross vector against a 0.25 cap must be REFUSED and the \
         refusal counted, got portfolio_breaches = {}",
        breached.portfolio_breaches
    );
    assert_eq!(
        breached.trades, 0,
        "ADR-0089 D2 is all-or-nothing: a breached rebalance must skip the ENTIRE \
         vector, not part of it — got {} fills",
        breached.trades
    );
    assert_eq!(
        breached.final_equity, INITIAL_CAPITAL,
        "nothing was ever bought, so equity must sit at initial capital"
    );

    // The control half — same bars, same k_long, same drift, ONLY the cap differs.
    assert_eq!(
        fits.portfolio_breaches, 0,
        "0.30 gross fits under a 0.50 cap — no breach expected, got {}",
        fits.portfolio_breaches
    );
    assert!(
        fits.trades > 0,
        "the control half must actually trade, otherwise the breach half proves \
         nothing about the cap"
    );
}

/// **#68 — the drift hold band actually SUPPRESSES rebalances.**
///
/// One leg on a steady uptrend, rebalancing every bar. The leg's notional drifts
/// away from `FIXED_FRACTION x equity` as the price rises; a tight band re-marks
/// it constantly, a wide band leaves it alone. Same bars, same cap, same k_long
/// — only `drift_rebalance_threshold` differs.
///
/// This is the Tier-1 grid's third advertised axis
/// (`lookback x k_long x drift_rebalance_threshold`). Until the sizer was wired
/// nothing consumed it, so 54 of 58 cells sat at 0.10 and the axis changed
/// nothing. RED WITHOUT THE CONTROL: an inert band gives both halves identical
/// trade counts.
#[test]
fn drift_band_suppresses_resizes_that_a_tight_band_performs() {
    let bars = build_bars(1, 60, dec!(0.02));

    let tight = run(make_config(1, 1, dec!(0.50), dec!(0.001)), bars.clone());
    let wide = run(make_config(1, 1, dec!(0.50), dec!(0.90)), bars);

    assert!(
        tight.trades > wide.trades,
        "#68: a 0.001 band must re-mark the leg far more often than a 0.90 band. \
         Equal counts mean the band is still inert. tight = {}, wide = {}",
        tight.trades,
        wide.trades
    );
    assert!(
        wide.trades > 0,
        "the wide-band half must still open the leg — otherwise the comparison is \
         'traded' vs 'did nothing', which would pass with the band inert too"
    );
    assert_ne!(
        tight.final_equity, wide.final_equity,
        "the band changes the executed book, so it must change the equity path; \
         identical equity means the extra orders were no-ops"
    );
}

/// **#94 — a resize order is the DELTA, so the book CONVERGES on its target.**
///
/// `size_portfolio_target` used to emit the FULL target quantity on every
/// action, including a same-side resize. That is only correct against a "set
/// position to X" venue API; every execution path here fills INCREMENTALLY, so
/// the order overshot by whatever was already held.
///
/// The observable is CONVERGENCE, and it is what ties #94 to #68: a delta-sized
/// order lands the leg exactly on target, so the drift band then holds it until
/// real drift accumulates. An absolute-sized order overshoots, so the leg is
/// outside the band again on the very next bar and re-trades forever — the band
/// can suppress nothing because nothing ever settles.
///
/// RED WITHOUT THE FIX, MEASURED: 60 bars, one leg, 0.10 band, +2 %/bar.
/// Delta-sized → **10 fills** in ~59 rebalance opportunities. Absolute-sized →
/// **50**. The bound below (under half the opportunities) sits far from both.
///
/// The defect was unreachable while the sizer had no caller (#69) and invisible
/// to its own unit tests, which only exercise flat -> open and -> close, where
/// delta and target coincide. The first fixture ever driven through the resize
/// path lost 74 % of equity with `min_cash_seen` at 43.8 out of 100 000.
#[test]
fn a_resize_converges_instead_of_overshooting_every_bar() {
    const N_BARS: usize = 60;
    let result = run(
        make_config(1, 1, dec!(0.50), dec!(0.10)),
        build_bars(1, N_BARS, dec!(0.02)),
    );

    // Rebalance is every bar, so opportunities = bars - 1 (the first bar cannot
    // drift from a position that does not exist yet).
    let opportunities = N_BARS - 1;

    assert!(
        result.trades > 1,
        "the fixture must open the leg and resize it at least once, or it cannot \
         distinguish convergence from inertia — got {} fills",
        result.trades
    );
    assert!(
        result.trades * 2 < opportunities,
        "#94: an order sized to the DELTA lands on target, so the 0.10 band holds \
         the leg for several bars at a time. {} fills against {opportunities} \
         rebalance opportunities means the leg is leaving the band every bar — \
         i.e. the order overshot and the book never converged",
        result.trades
    );
}
