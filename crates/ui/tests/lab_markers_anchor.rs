//! Integration test: fill markers fall inside the run's own bars window.
//!
//! ## Purpose
//!
//! Proves the operator-facing wiring: when a Lab run completes, the bars
//! surfaced on `SmaComposedRunResult.bars` contain all fills produced by
//! the same run.  The chart's `anchor_for_ts` precondition is therefore
//! met and triangle markers will render.
//!
//! ## Scope
//!
//! This test calls `backtest::scenarios::sma_composed_run::run` directly
//! (the same code path used by `engine::run_scenario` for `v0.sma` /
//! `sma_crossover`).  It does NOT spin up an iced runtime or render
//! anything — it validates the data contract the UI depends on.
//!
//! ## Forensic gate
//!
//! If `result.bars` is empty (pre-fix behaviour where bars were consumed
//! by `into_iter()` before the result struct was built), assertions on
//! `result.bars.len()` would fail — the fix is verifiable by reverting
//! the `bars_arc` introduction in `sma_composed_run.rs`.

use backtest::cli_types::SmaComposedRunInput;
use backtest::scenarios::sma_composed_run;
use rust_decimal_macros::dec;
use trading_core::Symbol;

const TEST_SEED: u64 = 0xC0FFEE;

/// Fills fall inside the bars window — the `anchor_for_ts` precondition.
///
/// For each fill:
///   `fill.venue_ts.unix_millis()` must be ≥ `bars.first().open_ts.unix_millis()`
///   AND ≤ `bars.last().close_ts.unix_millis()`
#[tokio::test]
async fn fills_anchor_within_run_bars() {
    let input = SmaComposedRunInput {
        strategy_id: "sma_crossover".to_string(),
        symbol: Symbol::new("BTCUSDT"),
        start_year: 2023,
        bar_count: 1_440, // one day of minute bars — fast
        initial_capital: dec!(100_000),
        slippage_bps: 2,
        taker_fee_bps: 4,
    };

    let result = sma_composed_run::run(&input, None, TEST_SEED)
        .await
        .expect("sma_crossover run must succeed");

    // Bars must be surfaced.
    assert_eq!(
        result.bars.len(),
        1_440,
        "result.bars must contain exactly bar_count bars (got {})",
        result.bars.len()
    );

    // At least one fill so the anchor test is meaningful.
    assert!(
        !result.fills.is_empty(),
        "sma_crossover on 1440 bars must produce at least one fill; got 0"
    );

    let first_bar_open_ms = result.bars.first().unwrap().open_ts.unix_millis();
    let last_bar_close_ms = result.bars.last().unwrap().close_ts.unix_millis();

    for (i, fill) in result.fills.iter().enumerate() {
        let fill_ts_ms = fill.venue_ts.unix_millis();
        assert!(
            fill_ts_ms >= first_bar_open_ms,
            "fill[{i}] ts {fill_ts_ms} is before the first bar open {first_bar_open_ms}"
        );
        assert!(
            fill_ts_ms <= last_bar_close_ms,
            "fill[{i}] ts {fill_ts_ms} is after the last bar close {last_bar_close_ms}"
        );
    }
}

/// Determinism: two identical runs produce identical bars + fills.
#[tokio::test]
async fn run_bars_are_deterministic() {
    let input = SmaComposedRunInput {
        strategy_id: "sma_crossover".to_string(),
        symbol: Symbol::new("BTCUSDT"),
        start_year: 2023,
        bar_count: 1_440,
        initial_capital: dec!(100_000),
        slippage_bps: 2,
        taker_fee_bps: 4,
    };

    let r1 = sma_composed_run::run(&input, None, TEST_SEED)
        .await
        .expect("first run must succeed");
    let r2 = sma_composed_run::run(&input, None, TEST_SEED)
        .await
        .expect("second run must succeed");

    assert_eq!(
        r1.bars.len(),
        r2.bars.len(),
        "bars length must be deterministic"
    );
    assert_eq!(
        r1.fills.len(),
        r2.fills.len(),
        "fills count must be deterministic"
    );

    // Spot-check: first and last bar open prices match.
    if !r1.bars.is_empty() {
        assert_eq!(
            r1.bars.first().unwrap().open,
            r2.bars.first().unwrap().open,
            "first bar open price must be deterministic"
        );
        assert_eq!(
            r1.bars.last().unwrap().close,
            r2.bars.last().unwrap().close,
            "last bar close price must be deterministic"
        );
    }
}
