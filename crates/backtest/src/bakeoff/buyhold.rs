//! Buy-and-hold equity path helper (extracted from `bin/param_robustness_sweep.rs`).
//!
//! This is a **behaviour-preserving relocation** of `run_buyhold_path` from the
//! sweep bin into the `backtest` library so the bake-off orchestrator can use it
//! uniformly.  The sweep bin is updated to call this copy.  Logic is byte-identical
//! to the original; only the module path changed.
//!
//! # Contract
//!
//! Equal-weight buy-and-hold for `n_symbols` on the given bar series:
//! - Allocates `initial_capital / n_symbols` per symbol at bar-0 close price.
//! - Marks to market at every subsequent timestep.
//! - Returns `(equity_curve_decimals, final_equity)`.
//!   The curve has `n_bars + 1` entries (entry [0] = `initial_capital`).
//!
//! For the single-coin bake-off arm `n_symbols = 1`, so the whole budget buys
//! the coin at bar-0 close and holds.

#![allow(clippy::float_arithmetic)] // Decimal arithmetic only; no f64 here

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use trading_core::Bar;

/// Run the equal-weight buy-and-hold path on the given bars.
///
/// `n_symbols` controls the equal-weight allocation per symbol.
/// For a single-coin bake-off, pass `1`.
///
/// # Returns
///
/// `(equity_curve, final_equity)` where `equity_curve` is a `Vec<Decimal>`
/// of length `n_distinct_timestamps + 1` (first entry = `initial_capital`).
///
/// Returns `(vec![initial_capital], initial_capital)` when bars are empty or
/// `n_symbols == 0`.
#[must_use]
pub fn run_buyhold_path(
    bars: &[Bar],
    initial_capital: Decimal,
    n_symbols: usize,
) -> (Vec<Decimal>, Decimal) {
    if bars.is_empty() || n_symbols == 0 {
        return (vec![initial_capital], initial_capital);
    }

    // Equal-weight allocation per symbol.
    #[allow(clippy::cast_precision_loss)]
    let weight = initial_capital / Decimal::try_from(n_symbols as f64).unwrap_or(dec!(10));

    // Group bars by symbol (in BTreeMap order — deterministic).
    let mut by_symbol: std::collections::BTreeMap<String, Vec<Decimal>> =
        std::collections::BTreeMap::new();
    for bar in bars {
        by_symbol
            .entry(bar.symbol.to_string())
            .or_default()
            .push(bar.close.get());
    }

    // Buy at bar 0 close; compute qty per symbol.
    let mut qtys: std::collections::BTreeMap<String, Decimal> = std::collections::BTreeMap::new();
    for (sym, prices) in &by_symbol {
        let buy_price = *prices.first().unwrap_or(&dec!(1));
        if buy_price > Decimal::ZERO {
            qtys.insert(sym.clone(), weight / buy_price);
        }
    }

    // Determine number of distinct timestamps.
    let n_bars = {
        #[allow(clippy::cast_possible_truncation)]
        let bar_ts: std::collections::BTreeSet<i64> = bars
            .iter()
            .map(|b| b.open_ts.inner().unix_timestamp_nanos() as i64)
            .collect();
        bar_ts.len()
    };

    // Group bars by timestamp → price map per symbol.
    let mut bar_map: std::collections::BTreeMap<i128, std::collections::BTreeMap<String, Decimal>> =
        std::collections::BTreeMap::new();
    for bar in bars {
        let ts = bar.open_ts.inner().unix_timestamp_nanos();
        bar_map
            .entry(ts)
            .or_default()
            .insert(bar.symbol.to_string(), bar.close.get());
    }

    let mut equity_curve: Vec<Decimal> = Vec::with_capacity(n_bars + 1);
    equity_curve.push(initial_capital);

    // Carry last known price so we handle missing bars gracefully.
    let mut last_prices: std::collections::BTreeMap<String, Decimal> =
        std::collections::BTreeMap::new();

    for prices_at_ts in bar_map.values() {
        for (sym, price) in prices_at_ts {
            last_prices.insert(sym.clone(), *price);
        }
        let equity: Decimal = qtys
            .iter()
            .map(|(sym, qty)| {
                let p = last_prices.get(sym).copied().unwrap_or(dec!(0));
                qty * p
            })
            .sum();
        equity_curve.push(equity);
    }

    let final_eq = *equity_curve.last().unwrap_or(&initial_capital);
    (equity_curve, final_eq)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;
    use trading_core::{Bar, Symbol, Timeframe, Timestamp, Venue};

    fn make_bar(ts_offset_hours: i64, symbol: &str, close: Decimal) -> Bar {
        use trading_core::money::{Price, Quantity};
        let ts =
            Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::hours(ts_offset_hours));
        let price = Price::new(close)
            .unwrap_or_else(|_| Price::new(dec!(1)).expect("dec!(1) is always a valid price"));
        let qty = Quantity::new(Decimal::ZERO).expect("zero is always valid qty");
        Bar {
            symbol: Symbol::new(symbol),
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

    /// Empty bars → returns (vec![initial], initial).
    #[test]
    fn empty_bars_returns_capital() {
        let (curve, final_eq) = run_buyhold_path(&[], dec!(1000), 1);
        assert_eq!(curve, vec![dec!(1000)]);
        assert_eq!(final_eq, dec!(1000));
    }

    /// `n_symbols=0` → returns (vec![initial], initial).
    #[test]
    fn zero_symbols_returns_capital() {
        let bars = vec![make_bar(0, "BTCUSDT", dec!(50000))];
        let (curve, final_eq) = run_buyhold_path(&bars, dec!(1000), 0);
        assert_eq!(curve, vec![dec!(1000)]);
        assert_eq!(final_eq, dec!(1000));
    }

    /// Single symbol, price doubles → equity doubles.
    #[test]
    fn single_symbol_price_doubles() {
        let bars = vec![
            make_bar(0, "BTCUSDT", dec!(100)),
            make_bar(1, "BTCUSDT", dec!(200)),
        ];
        let (curve, final_eq) = run_buyhold_path(&bars, dec!(1000), 1);
        // bar 0: buy 10 BTC at 100. bar 1: equity = 10 * 200 = 2000.
        assert_eq!(curve.len(), 3); // initial + 2 timesteps
        assert_eq!(curve[0], dec!(1000));
        // curve[1] = 10 * 100 = 1000 (first mark at bar-0 price)
        assert_eq!(curve[1], dec!(1000));
        // curve[2] = 10 * 200 = 2000
        assert_eq!(curve[2], dec!(2000));
        assert_eq!(final_eq, dec!(2000));
    }

    /// Determinism: two calls on the same bars produce identical results.
    #[test]
    fn deterministic() {
        let bars = vec![
            make_bar(0, "BTCUSDT", dec!(50000)),
            make_bar(1, "BTCUSDT", dec!(51000)),
            make_bar(2, "BTCUSDT", dec!(49000)),
        ];
        let (curve1, final1) = run_buyhold_path(&bars, dec!(100_000), 1);
        let (curve2, final2) = run_buyhold_path(&bars, dec!(100_000), 1);
        assert_eq!(curve1, curve2);
        assert_eq!(final1, final2);
    }
}
