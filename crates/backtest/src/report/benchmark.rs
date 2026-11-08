//! Equal-weight buy-and-hold control for the lane report bodies — bug-log `#151`.
//!
//! ## Why this exists
//!
//! Of the seven report writers in this directory, **not one** emitted a benchmark. Every
//! lane body stated a total return and left the reader no way to judge it. The live case
//! was `top10-2023-fy-patchtst-overlay-realdata`, whose canonical body moved from
//! +5.97 % to +41.81 % in the `#111` re-lock: is +41.81 % on a 10-symbol crypto basket
//! over FY2023 a result worth attention, or a disappointment against a passive hold? The
//! corpus could not answer it. A number with no comparator is not a weak result; it is
//! not a result.
//!
//! The bake-off has always been held to this standard — buy-and-hold is wired in as *its*
//! control precisely because a crown means nothing without one. The lane reports predate
//! that standard and nothing went back.
//!
//! ## Why it costs no compute
//!
//! `#151` listed this fix as "a BH run per anchored window" and parked it on that budget.
//! That budget does not exist: the run results already carry `bars: Arc<Vec<Bar>>`, and an
//! equal-weight buy-and-hold is a pure function of each symbol's FIRST and LAST close. No
//! second backtest, no data load, no engine. That is why this is a render-time module and
//! not a scenario.
//!
//! ## What it does and does not model
//!
//! - **Equal weight at entry, then held.** `initial_capital / N` per symbol, bought at
//!   that symbol's first observed close, marked at its last observed close. No
//!   rebalancing — rebalancing is not buy-and-hold.
//! - **Entry friction only.** One taker fee per leg on the way in. There is no exit fee
//!   because there is no exit: the terminal value is a MARK, exactly as the arm's own
//!   `final_equity` is a mark of its open book. Charging the control an exit the arm is
//!   not charged would bias the comparison toward the arm.
//! - **First and last OBSERVED close**, not the window's calendar bounds. A symbol that
//!   starts late in the merged stream is held for less than the full window; this is the
//!   same stream the arm traded, so the two see the same data.

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use trading_core::{Bar, Symbol};

/// The buy-and-hold control, as the writers print it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuyAndHold {
    /// Capital the control was given — the same figure the arm started with.
    pub initial_equity: Decimal,
    /// Mark-to-market value of the held basket at each symbol's last observed close.
    pub final_equity: Decimal,
    /// `(final - initial) / initial * 100`.
    pub total_return_pct: Decimal,
    /// How many symbols the capital was split across.
    pub legs: usize,
    /// Entry taker fees paid, summed over legs.
    pub fees_paid: Decimal,
}

/// Compute the equal-weight buy-and-hold control over the bars the lane actually traded.
///
/// Returns `None` when there is nothing to benchmark — no bars, no capital, or a symbol
/// whose first close is non-positive (which would make the leg's quantity meaningless).
/// `None` means "not computed", and the writers print that rather than a zero, because a
/// zero here would read as "passive returned nothing".
#[must_use]
pub fn equal_weight_buy_and_hold(
    bars: &[Bar],
    initial_capital: Decimal,
    taker_fee_bps: u32,
) -> Option<BuyAndHold> {
    if bars.is_empty() || initial_capital <= Decimal::ZERO {
        return None;
    }

    // First and last OBSERVED close per symbol. `bars` is time-ordered, so the first
    // insertion wins for `first` and every later one overwrites `last`.
    let mut legs: BTreeMap<Symbol, (Decimal, Decimal)> = BTreeMap::new();
    for bar in bars {
        let close = bar.close.get();
        legs.entry(bar.symbol.clone())
            .and_modify(|(_, last)| *last = close)
            .or_insert((close, close));
    }
    if legs.is_empty() {
        return None;
    }

    let n = Decimal::from(u64::try_from(legs.len()).ok()?);
    let budget = initial_capital / n;
    let fee_rate = Decimal::from(taker_fee_bps) / Decimal::from(10_000u32);

    let mut final_equity = Decimal::ZERO;
    let mut fees_paid = Decimal::ZERO;
    for (first, last) in legs.values() {
        if *first <= Decimal::ZERO {
            return None;
        }
        let fee = budget * fee_rate;
        let qty = (budget - fee) / first;
        final_equity += qty * last;
        fees_paid += fee;
    }

    Some(BuyAndHold {
        initial_equity: initial_capital,
        final_equity,
        total_return_pct: (final_equity - initial_capital) / initial_capital
            * Decimal::from(100u32),
        legs: legs.len(),
        fees_paid,
    })
}

/// The two table cells the writers print, rendered once so all of them agree.
///
/// Kept in `Decimal` end to end (AD-9): the arm's own return is recomputed here from its
/// initial and final equity rather than taking the `f64` the writers already have, so the
/// delta is not a difference of two roundings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BhCells {
    /// The control itself, e.g. `+33.31% ($133311.29 USDT, 10 legs, $50.00 fees)`.
    pub control: String,
    /// The arm minus the control, in percentage POINTS, e.g. `-8.50 pp`.
    pub delta: String,
}

/// Render the buy-and-hold cells for a lane body.
///
/// `None` renders as an explicit "not computed" rather than a dash or a zero: a zero in
/// this column would read as "passive returned nothing", which is a claim, and a dash
/// leaves the reader where bug-log `#151` found them.
#[must_use]
pub fn bh_cells(bh: Option<&BuyAndHold>, arm_initial: Decimal, arm_final: Decimal) -> BhCells {
    let Some(bh) = bh else {
        return BhCells {
            control: "not computed (no bars in the run result)".to_string(),
            delta: "n/a".to_string(),
        };
    };
    let control = format!(
        "{:+}% (${} USDT, {} legs, ${} fees)",
        bh.total_return_pct.round_dp(2),
        bh.final_equity.round_dp(2),
        bh.legs,
        bh.fees_paid.round_dp(2),
    );
    let delta = if arm_initial > Decimal::ZERO {
        let arm_pct = (arm_final - arm_initial) / arm_initial * Decimal::from(100u32);
        format!("{:+} pp", (arm_pct - bh.total_return_pct).round_dp(2))
    } else {
        "n/a".to_string()
    };
    BhCells { control, delta }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use trading_core::{Price, Quantity, Timeframe, Timestamp, Venue};

    fn bar(sym: &str, close: Decimal, secs: i64) -> Bar {
        let ts =
            Timestamp::new(time::OffsetDateTime::from_unix_timestamp(secs).expect("valid unix ts"));
        Bar {
            symbol: Symbol::new(sym),
            tf: Timeframe::OneHour,
            open_ts: ts,
            close_ts: ts,
            open: Price::new(close).unwrap(),
            high: Price::new(close).unwrap(),
            low: Price::new(close).unwrap(),
            close: Price::new(close).unwrap(),
            volume: Quantity::new(dec!(1)).unwrap(),
            trade_count: 1,
            local_recv_ts: ts,
            venue: Venue::Binance,
        }
    }

    /// Two legs, DERIVED by hand, and the two legs differ in return so the test
    /// distinguishes an equal-weight average from "whatever the first symbol did".
    ///
    /// $1000 over 2 legs → $500 each, 0 bps.
    /// A: 100 → 200 ⇒ qty 5   ⇒ 5 × 200 = 1000
    /// B:  50 →  50 ⇒ qty 10  ⇒ 10 × 50 =  500
    /// total 1500 ⇒ +50 %. Not +100 % (A alone) and not 0 % (B alone).
    #[test]
    fn equal_weight_averages_the_legs_rather_than_following_one() {
        let bars = vec![
            bar("AAA", dec!(100), 0),
            bar("BBB", dec!(50), 0),
            bar("AAA", dec!(200), 3600),
            bar("BBB", dec!(50), 3600),
        ];
        let bh = equal_weight_buy_and_hold(&bars, dec!(1000), 0).expect("computable");
        assert_eq!(bh.legs, 2);
        assert_eq!(bh.final_equity, dec!(1500));
        assert_eq!(bh.total_return_pct, dec!(50));
        assert_eq!(bh.fees_paid, dec!(0));
    }

    /// FIRST and LAST close, not the extremes. A 10× spike in the middle must not move
    /// the answer — otherwise the control would report the best moment, not the hold.
    #[test]
    fn an_intermediate_spike_does_not_change_the_hold() {
        let flat = vec![bar("AAA", dec!(100), 0), bar("AAA", dec!(120), 7200)];
        let spiked = vec![
            bar("AAA", dec!(100), 0),
            bar("AAA", dec!(1000), 3600),
            bar("AAA", dec!(120), 7200),
        ];
        let a = equal_weight_buy_and_hold(&flat, dec!(1000), 0).expect("computable");
        let b = equal_weight_buy_and_hold(&spiked, dec!(1000), 0).expect("computable");
        assert_eq!(
            a, b,
            "a mid-window spike is not part of a buy-and-hold result"
        );
        assert_eq!(a.total_return_pct, dec!(20));
    }

    /// The fee dimension, pinned from BOTH sides: the same bars at 0 bps and at 50 bps
    /// must give DIFFERENT equity, and the difference must be the fee's effect.
    ///
    /// $1000, 1 leg, 50 bps ⇒ fee $5, invested $995 at 100 ⇒ qty 9.95 ⇒ 9.95 × 120 = 1194.
    #[test]
    fn the_taker_fee_changes_the_answer_and_by_the_derived_amount() {
        let bars = vec![bar("AAA", dec!(100), 0), bar("AAA", dec!(120), 3600)];
        let free = equal_weight_buy_and_hold(&bars, dec!(1000), 0).expect("computable");
        let charged = equal_weight_buy_and_hold(&bars, dec!(1000), 50).expect("computable");
        assert_eq!(free.final_equity, dec!(1200));
        assert_eq!(charged.final_equity, dec!(1194));
        assert_eq!(charged.fees_paid, dec!(5));
        assert_ne!(
            free.final_equity, charged.final_equity,
            "if the fee rate does not move the result, the fee is not wired"
        );
    }

    /// A falling market must come out NEGATIVE. Pins the sign, which a single rising
    /// fixture cannot.
    #[test]
    fn a_falling_market_returns_a_negative_control() {
        let up = vec![bar("AAA", dec!(100), 0), bar("AAA", dec!(150), 3600)];
        let down = vec![bar("AAA", dec!(100), 0), bar("AAA", dec!(60), 3600)];
        assert_eq!(
            equal_weight_buy_and_hold(&up, dec!(1000), 0)
                .expect("computable")
                .total_return_pct,
            dec!(50)
        );
        assert_eq!(
            equal_weight_buy_and_hold(&down, dec!(1000), 0)
                .expect("computable")
                .total_return_pct,
            dec!(-40)
        );
    }

    /// `None` means "not computed", and each `None` path is paired with the Some control
    /// that proves the fixture is otherwise valid — or a `None` would just mean "wrong
    /// fixture".
    #[test]
    fn the_uncomputable_cases_are_none_and_the_control_case_is_some() {
        let good = vec![bar("AAA", dec!(100), 0), bar("AAA", dec!(120), 3600)];
        assert!(
            equal_weight_buy_and_hold(&good, dec!(1000), 0).is_some(),
            "non-vacuity: the valid fixture must compute, or the None cases prove nothing"
        );
        assert!(
            equal_weight_buy_and_hold(&[], dec!(1000), 0).is_none(),
            "no bars"
        );
        assert!(
            equal_weight_buy_and_hold(&good, dec!(0), 0).is_none(),
            "zero capital"
        );
        assert!(
            equal_weight_buy_and_hold(&good, dec!(-1), 0).is_none(),
            "negative capital"
        );
    }

    /// The delta's SIGN is the whole point of the column, so it is pinned from both
    /// sides: an arm that beats the control and an arm that loses to it.
    #[test]
    fn the_delta_sign_distinguishes_beating_the_control_from_losing_to_it() {
        let bars = vec![bar("AAA", dec!(100), 0), bar("AAA", dec!(120), 3600)];
        let bh = equal_weight_buy_and_hold(&bars, dec!(1000), 0).expect("computable");
        assert_eq!(bh.total_return_pct, dec!(20));

        // Arm +50 % against a +20 % control → +30 pp.
        let won = bh_cells(Some(&bh), dec!(1000), dec!(1500));
        assert_eq!(won.delta, "+30.00 pp");
        // Arm −10 % against the same control → −30 pp.
        let lost = bh_cells(Some(&bh), dec!(1000), dec!(900));
        assert_eq!(lost.delta, "-30.00 pp");
        assert_ne!(
            won.delta, lost.delta,
            "if both arms render the same delta the column carries no information"
        );
    }

    /// `None` must say so in words. A dash or a zero here is what `#151` is about.
    #[test]
    fn an_uncomputed_control_says_so_rather_than_printing_a_number() {
        let cells = bh_cells(None, dec!(1000), dec!(1500));
        assert!(cells.control.contains("not computed"), "{}", cells.control);
        assert_eq!(cells.delta, "n/a");
        // And the computed case must NOT say that — or the check above is vacuous.
        let bars = vec![bar("AAA", dec!(100), 0), bar("AAA", dec!(120), 3600)];
        let bh = equal_weight_buy_and_hold(&bars, dec!(1000), 0).expect("computable");
        let real = bh_cells(Some(&bh), dec!(1000), dec!(1500));
        assert!(!real.control.contains("not computed"), "{}", real.control);
        assert!(real.control.contains("+20.00%"), "{}", real.control);
    }

    /// Adding a symbol REDISTRIBUTES the capital rather than adding to it: total invested
    /// is conserved. Pins that `budget` divides by the leg count.
    #[test]
    fn a_third_leg_splits_the_same_capital_three_ways() {
        let two = vec![
            bar("AAA", dec!(100), 0),
            bar("BBB", dec!(100), 0),
            bar("AAA", dec!(100), 3600),
            bar("BBB", dec!(100), 3600),
        ];
        let mut three = two.clone();
        three.push(bar("CCC", dec!(100), 0));
        three.push(bar("CCC", dec!(100), 3600));

        let a = equal_weight_buy_and_hold(&two, dec!(900), 0).expect("computable");
        let b = equal_weight_buy_and_hold(&three, dec!(900), 0).expect("computable");
        assert_eq!(a.legs, 2);
        assert_eq!(b.legs, 3);
        // All flat, so both must return exactly the starting capital — the leg count
        // changes the split, never the total.
        assert_eq!(a.final_equity, dec!(900));
        assert_eq!(b.final_equity, dec!(900));
    }
}
