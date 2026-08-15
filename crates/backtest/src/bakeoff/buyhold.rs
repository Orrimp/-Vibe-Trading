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

/// Buy-and-hold gated by an exogenous per-timestamp macro regime mask.
///
/// ADR-0073 D4 — the `v0.macro_riskon` arm's equity-path function, a sibling
/// of `run_buyhold_path`. Holds the coin (passive long) when the daily macro
/// regime is risk-ON; goes flat (cash) when risk-OFF or during warm-up.
///
/// # Regime join (look-ahead-free by construction)
///
/// At each distinct coin-bar timestamp `t`, the regime is read as:
/// `regime.as_of_value(TimestampMs(t_ms)).unwrap_or(false)`.
/// `as_of_value` returns the most-recent record with `close_ts ≤ t` (ADR-0058
/// `PitSeries<bool>`) — a macro daily close dated `D` is visible only to coin
/// bars at/after `D`'s close. Weekend/holiday gaps carry Friday's close
/// forward across Sat/Sun/holiday crypto bars. Look-ahead is structurally
/// unrepresentable (no `ts > query` accessor).
///
/// # Transitions
///
/// - **flat → ON:** buy `cash / price(t)` coin at `t`'s close, `cash = 0`.
/// - **ON → flat:** sell all coin at `t`'s close, `cash = coin_qty × price(t)`,
///   `coin_qty = 0`. Realistic: the regime flip is observed at the daily close
///   ≤ `t`, the trade executes at the coin bar `t` — look-ahead-free.
///
/// # ⚠ Costs: this path trades for FREE — a pre-registration departure
///
/// Every transition above executes at the bar's own close with **no taker fee,
/// no slippage, no lot rounding and no min-notional filter**; the arm's
/// `total_fees` is `Money::zero()`. The transition is therefore
/// equity-*neutral* at the instant it happens (selling at the mark neither gains
/// nor loses), which is also why a regime flip on the final bar is invisible in
/// the equity curve.
///
/// This **contradicts the feature's own pre-registered clause** — *"transition
/// trades pay the standard taker fee … the macro arm is NOT cost-advantaged vs
/// the always-long benchmark"* — and ADR-0073 records no decision to drop it
/// (review 3-16 HIGH). The 18 sibling arms this one is RANKED AGAINST pay 4 bps
/// a leg through `PaperEngine`, plus lot rounding since bug-log #79, so this is
/// bug-log #80's shape — asymmetric friction inside a ranked comparison — on a
/// new axis.
///
/// Direction, stated fairly: the departure **flatters** this arm, so charging
/// the pre-registered fee would strengthen the recorded null rather than
/// reverse it. It is left unchanged here deliberately: changing the economics of
/// a ranked arm is a measured product change, not a review patch. Until it is
/// decided, **no verdict computed on this path may be described as "net of
/// costs"** — that phrase is literally false for this arm.
///
/// # Returns
///
/// `(equity_curve, final_equity)` where:
/// - `equity_curve` has `n_distinct_timestamps + 1` entries; entry[0] = `initial_capital`.
/// - Empty bars or all-warm-up → `(vec![initial_capital], initial_capital)`.
///
/// # Determinism / Decimal
///
/// All arithmetic is `rust_decimal::Decimal` — no `f64`.
/// Uses `BTreeMap`-ordered iteration — deterministic.
#[must_use]
pub fn run_macro_gated_buyhold_path(
    bars: &[trading_core::Bar],
    regime: &trading_core::PitSeries<bool>,
    initial_capital: Decimal,
) -> (Vec<Decimal>, Decimal) {
    use trading_core::pit::TimestampMs;

    if bars.is_empty() {
        return (vec![initial_capital], initial_capital);
    }

    // Collect distinct timestamps in BTreeMap order → price at that ts.
    // Use close price (same as run_buyhold_path).
    let mut bar_map: std::collections::BTreeMap<i64, Decimal> = std::collections::BTreeMap::new();
    for bar in bars {
        let ts_ms = bar.open_ts.unix_millis();
        // Last bar wins on tie (deterministic).
        bar_map.insert(ts_ms, bar.close.get());
    }

    if bar_map.is_empty() {
        return (vec![initial_capital], initial_capital);
    }

    let mut equity_curve: Vec<Decimal> = Vec::with_capacity(bar_map.len() + 1);
    equity_curve.push(initial_capital);

    let mut cash = initial_capital;
    let mut coin_qty = Decimal::ZERO;
    let mut prev_on = false; // start flat (warm-up default)

    for (&ts_ms, &price) in &bar_map {
        // Look-ahead-free regime read via ADR-0058 primitive.
        let on = regime.as_of_value(TimestampMs(ts_ms)).unwrap_or(false); // warm-up → flat

        // Transitions.
        if on && !prev_on {
            // flat → ON: buy coin.
            if price > Decimal::ZERO && cash > Decimal::ZERO {
                coin_qty = cash / price;
                cash = Decimal::ZERO;
            }
        } else if !on && prev_on {
            // ON → flat: sell coin.
            if coin_qty > Decimal::ZERO {
                cash = coin_qty * price;
                coin_qty = Decimal::ZERO;
            }
        }
        prev_on = on;

        // Mark to market.
        let equity = cash + coin_qty * price;
        equity_curve.push(equity);
    }

    let final_eq = *equity_curve.last().unwrap_or(&initial_capital);
    (equity_curve, final_eq)
}

/// Run the always-short equity path on the given bars (ADR-0068 T-D6).
///
/// This is the **exact inverse** of `run_buyhold_path` for a single-coin, 1×
/// fully-collateralized short:
///
/// ```text
///   equity[i] = initial_capital × (2 − price[i] / price0)
/// ```
///
/// - `price0` is the close price of the first bar (bar-0).
/// - A short opened at `price0` profits as `price[i] < price0` and loses as
///   `price[i] > price0`.
/// - Loss is **UNBOUNDED and NEGATIVE** — do NOT clamp at 0.  A 2× price move
///   (price doubles) wipes out the whole position and then some.
///
/// # Returns
///
/// `(equity_curve, final_equity)` where:
/// - `equity_curve` has `n_bars + 1` entries; `curve[0] = initial_capital`.
/// - Empty bars → `(vec![initial_capital], initial_capital)`.
///
/// # Sign contract
///
/// | Price move    | Equity at final bar               |
/// |---------------|-----------------------------------|
/// | halved (−50%) | `initial_capital × 1.5` (+50%)    |
/// | doubled (+100%) | `0` (−100%, wipe)              |
/// | tripled (+200%) | `−initial_capital` (−200%)     |
/// | unchanged      | `initial_capital` (0%)           |
///
/// This function is **paper/sim only** — it models a simulated short position
/// with no real margin, no real orders, and no real money at risk.
#[must_use]
pub fn run_alwaysshort_path(bars: &[Bar], initial_capital: Decimal) -> (Vec<Decimal>, Decimal) {
    if bars.is_empty() {
        return (vec![initial_capital], initial_capital);
    }

    // Collect distinct bar timestamps in order (same dedup as run_buyhold_path).
    let mut seen: std::collections::BTreeSet<i128> = std::collections::BTreeSet::new();
    let mut prices_ordered: Vec<Decimal> = Vec::new();

    for bar in bars {
        let ts = bar.open_ts.inner().unix_timestamp_nanos();
        if seen.insert(ts) {
            prices_ordered.push(bar.close.get());
        }
    }

    if prices_ordered.is_empty() {
        return (vec![initial_capital], initial_capital);
    }

    let price0 = prices_ordered[0];
    if price0 == Decimal::ZERO {
        // Edge: zero open price — no meaningful short; return flat.
        return (
            vec![initial_capital; prices_ordered.len() + 1],
            initial_capital,
        );
    }

    // Build the curve: entry[0] = initial_capital; entry[i+1] = formula applied to bar i.
    let mut equity_curve: Vec<Decimal> = Vec::with_capacity(prices_ordered.len() + 1);
    equity_curve.push(initial_capital);

    for &price in &prices_ordered {
        // equity = initial_capital × (2 − price / price0)
        // Rearranged to minimise precision loss: initial_capital × 2 − initial_capital × (price / price0)
        let ratio = price / price0;
        let eq = initial_capital * (dec!(2) - ratio);
        equity_curve.push(eq);
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

    // ── Tests for run_alwaysshort_path ────────────────────────────────────────

    /// Empty bars → returns (vec![initial], initial).
    #[test]
    fn alwaysshort_empty_bars_returns_capital() {
        let (curve, final_eq) = run_alwaysshort_path(&[], dec!(1000));
        assert_eq!(curve, vec![dec!(1000)]);
        assert_eq!(final_eq, dec!(1000));
    }

    /// Price halves (−50%) → equity +50% (1.5× initial). Short profits on down-move.
    #[test]
    fn alwaysshort_price_halves_equity_plus_50pct() {
        // price0 = 100; price1 = 50 (halved).
        // equity[1] = 1000 × (2 − 50/100) = 1000 × 1.5 = 1500.
        let bars = vec![
            make_bar(0, "BTCUSDT", dec!(100)),
            make_bar(1, "BTCUSDT", dec!(50)),
        ];
        let (curve, final_eq) = run_alwaysshort_path(&bars, dec!(1000));
        assert_eq!(curve.len(), 3); // initial + 2 timesteps
        assert_eq!(curve[0], dec!(1000));
        assert_eq!(curve[1], dec!(1000)); // bar-0 = price0 → equity unchanged
        assert_eq!(curve[2], dec!(1500)); // bar-1: price halved → +50%
        assert_eq!(final_eq, dec!(1500));
    }

    /// Price doubles (+100%) → equity 0 (full wipe). Short loses on up-move.
    #[test]
    fn alwaysshort_price_doubles_equity_zero() {
        // price0 = 100; price1 = 200 (doubled).
        // equity[1] = 1000 × (2 − 200/100) = 1000 × 0 = 0.
        let bars = vec![
            make_bar(0, "BTCUSDT", dec!(100)),
            make_bar(1, "BTCUSDT", dec!(200)),
        ];
        let (curve, final_eq) = run_alwaysshort_path(&bars, dec!(1000));
        assert_eq!(curve[0], dec!(1000));
        assert_eq!(curve[2], dec!(0));
        assert_eq!(final_eq, dec!(0));
    }

    /// Price triples (+200%) → equity NEGATIVE (−initial). Unbounded loss — no clamp.
    #[test]
    fn alwaysshort_price_triples_equity_negative() {
        // price0 = 100; price1 = 300 (tripled).
        // equity[1] = 1000 × (2 − 300/100) = 1000 × (2 − 3) = 1000 × −1 = −1000.
        let bars = vec![
            make_bar(0, "BTCUSDT", dec!(100)),
            make_bar(1, "BTCUSDT", dec!(300)),
        ];
        let (curve, final_eq) = run_alwaysshort_path(&bars, dec!(1000));
        assert_eq!(curve[0], dec!(1000));
        assert_eq!(curve[2], dec!(-1000));
        assert!(
            final_eq < dec!(0),
            "equity must be NEGATIVE on a 3× up-move (no clamp)"
        );
    }

    /// Determinism: two calls on the same bars produce identical results.
    #[test]
    fn alwaysshort_deterministic() {
        let bars = vec![
            make_bar(0, "BTCUSDT", dec!(50000)),
            make_bar(1, "BTCUSDT", dec!(45000)),
            make_bar(2, "BTCUSDT", dec!(40000)),
        ];
        let (curve1, final1) = run_alwaysshort_path(&bars, dec!(100_000));
        let (curve2, final2) = run_alwaysshort_path(&bars, dec!(100_000));
        assert_eq!(curve1, curve2);
        assert_eq!(final1, final2);
        // Sanity: bear trend → short profits.
        assert!(
            final1 > dec!(100_000),
            "always_short must profit on a bear trend"
        );
    }
}
