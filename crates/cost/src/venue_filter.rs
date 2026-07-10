//! Static venue lot-size / min-notional filter table (ADR-0087, opt-in-forever).
//!
//! Real spot venues enforce two `exchangeInfo` filters that the paper/sim
//! fill path otherwise ignores:
//!
//! - **`LOT_SIZE`** — order quantity must be an exact multiple of a
//!   per-symbol `stepSize` (e.g. `0.00001 BTC`, `1 DOGE`); the venue
//!   **floors** the requested quantity to the nearest step.
//! - **`NOTIONAL` / `MIN_NOTIONAL`** — order notional (`qty · price`) must
//!   clear a per-symbol floor (commonly ~5 USDT on Binance spot); below it
//!   the venue **rejects** the order outright.
//!
//! This module carries a **checked-in static snapshot** of those two
//! filters for the advisor's symbol corpus — NOT a live `exchangeInfo`
//! fetch. A live fetch would violate the no-live-calls / determinism
//! constraint (CLAUDE.md) and make a backtest's result depend on
//! wall-clock fetch timing. See ADR-0087 § D3 for the full rationale.
//!
//! ## Staleness (stated limit)
//!
//! `SNAPSHOT_DATE` below records when this table was captured. Venues
//! revise `stepSize` / `minNotional` occasionally; this table will drift
//! from the live truth over time. A refresh is a one-line table edit
//! under ADR-0087 — **no anchor re-emission is owed** (D3/D6: the filter
//! table sits off every anchored CLI path).
//!
//! ## Shape (mirrors `data::SymbolInfo`, no new dep edge)
//!
//! `crates/data/src/source.rs:8` already defines `SymbolInfo { symbol,
//! base_asset, quote_asset, min_qty, lot_size, min_notional }`, populated by
//! the **live** `exchange_info()` fetch. The `cost` crate does NOT depend on
//! `crates/data` (that dependency edge does not exist and this module does
//! not add it); [`VenueFilter`] carries only the two fields this exec-sim
//! mode needs (`step_size` ~ `SymbolInfo::lot_size`, `min_notional`), as a
//! small local record.
//!
//! ## Decimal discipline (ADR-0003)
//!
//! All arithmetic here is `rust_decimal::Decimal`. Rounding is
//! `(qty / step).floor() * step`, computed entirely in `Decimal` — never
//! `f64`. Round-DOWN only (`floor`, never `round`/`ceil`): the user must
//! never be filled for more than their sized budget.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use trading_core::Symbol;

/// Date this snapshot was captured (ISO-8601, `YYYY-MM-DD`). Staleness is a
/// stated limit — see the module-level docs. Bump this alongside any table
/// edit so the staleness window is always visible at a glance.
pub const SNAPSHOT_DATE: &str = "2026-07-10";

/// A venue's lot-size + min-notional filter for one symbol.
///
/// Mirrors the shape of `data::SymbolInfo` (`crates/data/src/source.rs:8`)
/// without depending on the `data` crate (ADR-0087 § D3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VenueFilter {
    /// `LOT_SIZE` stepSize: the requested quantity must be an exact
    /// multiple of this after rounding down.
    pub step_size: Decimal,
    /// `NOTIONAL` / `MIN_NOTIONAL` floor: `qty * price` must be `>=` this
    /// or the (rounded) order is rejected.
    pub min_notional: Decimal,
}

impl VenueFilter {
    /// Round `qty` down to `step_size`, then admit iff the rounded qty is
    /// strictly positive AND its notional at `price` clears `min_notional`.
    ///
    /// Returns `Some(rounded_qty)` on admit, `None` on reject (the qty
    /// rounds to zero, or the rounded notional is sub-floor). Round-DOWN
    /// only (ADR-0087 § D3): the caller must never be filled for more than
    /// what was sized.
    #[must_use]
    pub fn admit(&self, qty: Decimal, price: Decimal) -> Option<Decimal> {
        let rounded = round_down_to_step(qty, self.step_size);
        if rounded > Decimal::ZERO && rounded * price >= self.min_notional {
            Some(rounded)
        } else {
            None
        }
    }
}

/// Round `qty` DOWN to the nearest multiple of `step` (Decimal-exact floor).
///
/// `(qty / step).floor() * step`. Round-DOWN only — never rounds up, so a
/// caller can never be filled for more than it asked for. Defensive guard:
/// a non-positive `step` (malformed filter entry) returns `qty` unchanged
/// rather than dividing by zero or silently zeroing the quantity out.
#[must_use]
pub fn round_down_to_step(qty: Decimal, step: Decimal) -> Decimal {
    if step <= Decimal::ZERO {
        return qty;
    }
    (qty / step).floor() * step
}

/// Look up the checked-in venue filter for `symbol`.
///
/// Covers the **10 Binance USDT pairs** (the advisor corpus: BTC, ETH, BNB,
/// SOL, XRP, ADA, DOGE, AVAX, DOT, LINK) plus **Coinbase `BTC-USD`** (the P2
/// second venue). Returns `None` for any symbol outside this snapshot —
/// the venue-filter mode is a **no-op** for unknown symbols: never a panic,
/// never a silently-wrong number (ADR-0087 § D3).
#[must_use]
pub fn venue_filter_for(symbol: &Symbol) -> Option<VenueFilter> {
    // Binance min-notional floor: ~5 USDT across the corpus at SNAPSHOT_DATE
    // (ADR-0087 § D4's "~5-10 USDT" range).
    let binance_min_notional = dec!(5);

    match symbol.0.as_str() {
        "BTCUSDT" => Some(VenueFilter {
            step_size: dec!(0.00001),
            min_notional: binance_min_notional,
        }),
        "ETHUSDT" => Some(VenueFilter {
            step_size: dec!(0.0001),
            min_notional: binance_min_notional,
        }),
        "BNBUSDT" => Some(VenueFilter {
            step_size: dec!(0.01),
            min_notional: binance_min_notional,
        }),
        "SOLUSDT" => Some(VenueFilter {
            step_size: dec!(0.01),
            min_notional: binance_min_notional,
        }),
        "XRPUSDT" => Some(VenueFilter {
            step_size: dec!(0.1),
            min_notional: binance_min_notional,
        }),
        "ADAUSDT" => Some(VenueFilter {
            step_size: dec!(0.1),
            min_notional: binance_min_notional,
        }),
        // DOGE: coarse whole-unit step — the ADR-0087 § D5 divergence corpus
        // (a low-price coin where lot rounding provably bites a small budget).
        "DOGEUSDT" => Some(VenueFilter {
            step_size: dec!(1),
            min_notional: binance_min_notional,
        }),
        "AVAXUSDT" => Some(VenueFilter {
            step_size: dec!(0.01),
            min_notional: binance_min_notional,
        }),
        "DOTUSDT" => Some(VenueFilter {
            step_size: dec!(0.01),
            min_notional: binance_min_notional,
        }),
        "LINKUSDT" => Some(VenueFilter {
            step_size: dec!(0.01),
            min_notional: binance_min_notional,
        }),
        // Coinbase P2 second venue — finer base_increment, lower USD floor.
        "BTC-USD" => Some(VenueFilter {
            step_size: dec!(0.00000001),
            min_notional: dec!(1),
        }),
        _ => None,
    }
}

// ── Tests (T3) ──────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // ── round_down_to_step ───────────────────────────────────────────────

    #[test]
    fn round_down_whole_doge_step_one() {
        // 12.9 DOGE flooring to whole DOGE (step_size = 1) → 12.
        assert_eq!(round_down_to_step(dec!(12.9), dec!(1)), dec!(12));
    }

    #[test]
    fn round_down_btc_five_decimals() {
        // 0.123456 BTC at step 0.00001 → 0.12345 (the 6th decimal is discarded).
        assert_eq!(
            round_down_to_step(dec!(0.123456), dec!(0.00001)),
            dec!(0.12345)
        );
    }

    #[test]
    fn round_down_never_rounds_up() {
        // 0.999 at step 1 must floor to 0, never round to 1.
        assert_eq!(round_down_to_step(dec!(0.999), dec!(1)), dec!(0));
    }

    #[test]
    fn round_down_exact_multiple_is_unchanged() {
        assert_eq!(round_down_to_step(dec!(10), dec!(1)), dec!(10));
    }

    #[test]
    fn round_down_step_zero_guard_returns_qty_unchanged() {
        // Defensive: a malformed (non-positive) step must not panic or
        // divide by zero — the qty passes through unchanged.
        assert_eq!(round_down_to_step(dec!(5.5), dec!(0)), dec!(5.5));
    }

    #[test]
    fn round_down_negative_step_guard_returns_qty_unchanged() {
        assert_eq!(round_down_to_step(dec!(5.5), dec!(-1)), dec!(5.5));
    }

    // ── VenueFilter::admit ───────────────────────────────────────────────

    #[test]
    fn admit_exactly_at_min_notional_admits() {
        let vf = VenueFilter {
            step_size: dec!(1),
            min_notional: dec!(5),
        };
        // 10 DOGE * 0.5 = notional 5.0 exactly → admits.
        assert_eq!(vf.admit(dec!(10), dec!(0.5)), Some(dec!(10)));
    }

    #[test]
    fn admit_one_tick_under_min_notional_rejects() {
        let vf = VenueFilter {
            step_size: dec!(1),
            min_notional: dec!(5),
        };
        // 9.999 rounds down to 9 DOGE; 9 * 0.5 = 4.5 < 5 → rejects.
        assert_eq!(vf.admit(dec!(9.999), dec!(0.5)), None);
    }

    #[test]
    fn admit_zero_after_round_rejects_regardless_of_price() {
        let vf = VenueFilter {
            step_size: dec!(1),
            min_notional: dec!(5),
        };
        // 0.5 DOGE rounds to 0 → rejects even at a huge price.
        assert_eq!(vf.admit(dec!(0.5), dec!(100_000)), None);
    }

    #[test]
    fn admit_just_below_notional_rejects_just_at_admits() {
        let vf = VenueFilter {
            step_size: dec!(0.00001),
            min_notional: dec!(5),
        };
        // qty fixed at 0.0001 BTC; price chosen so notional straddles 5.
        assert_eq!(vf.admit(dec!(0.0001), dec!(49_999)), None); // notional 4.9999
        assert_eq!(vf.admit(dec!(0.0001), dec!(50_000)), Some(dec!(0.0001))); // notional 5.0000
    }

    // ── venue_filter_for ─────────────────────────────────────────────────

    #[test]
    fn venue_filter_for_dogeusdt_has_whole_unit_step() {
        let vf = venue_filter_for(&Symbol::new("DOGEUSDT")).expect("DOGEUSDT must be in table");
        assert_eq!(vf.step_size, dec!(1));
        assert_eq!(vf.min_notional, dec!(5));
    }

    #[test]
    fn venue_filter_for_all_ten_binance_pairs_present() {
        for sym in [
            "BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT", "XRPUSDT", "ADAUSDT", "DOGEUSDT",
            "AVAXUSDT", "DOTUSDT", "LINKUSDT",
        ] {
            assert!(
                venue_filter_for(&Symbol::new(sym)).is_some(),
                "{sym} must resolve to a filter entry"
            );
        }
    }

    #[test]
    fn venue_filter_for_coinbase_btc_usd_present() {
        assert!(venue_filter_for(&Symbol::new("BTC-USD")).is_some());
    }

    #[test]
    fn venue_filter_for_unknown_symbol_is_none() {
        // Unknown symbol → None → the mode is a no-op for this symbol,
        // never a panic, never a silently-wrong number (ADR-0087 § D3).
        assert!(venue_filter_for(&Symbol::new("SHIBUSDT")).is_none());
        assert!(venue_filter_for(&Symbol::new("")).is_none());
    }

    #[test]
    fn decimal_literals_only_no_float_in_admit_path() {
        // Compile-time-ish sanity: this test simply exercises admit() with
        // Decimal inputs end-to-end; the type signatures of `round_down_to_step`
        // and `VenueFilter::admit` (Decimal in, Decimal out) are the actual
        // enforcement — there is no f64 anywhere on this path.
        let vf = venue_filter_for(&Symbol::new("BTCUSDT")).unwrap();
        let rounded = vf.admit(dec!(0.123456), dec!(50_000));
        assert_eq!(rounded, Some(dec!(0.12345)));
    }
}
