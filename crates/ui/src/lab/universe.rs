//! XRP-first universe ordering — ui-rethink-phase-a-lab T-D-8.
//!
//! Operator-locked pair order per Design § R3.2 and Q-A3:
//!
//! - **XRPUSDT, ETHUSDT, BTCUSDT** — operator preference (first three
//!   are operator-chosen, not alphabetical).
//! - **ADAUSDT, AVAXUSDT, BNBUSDT, DOGEUSDT, DOTUSDT, LINKUSDT,
//!   SOLUSDT** — alphabetical remainder.
//!
//! The ordering is **data-driven here** (a const slice) — not hard-coded
//! into the chip widget — so a future Settings toggle can re-sort without
//! touching `widgets/pair_chip.rs`. Any re-sort of this slice will fail
//! the `xrp_first_order_pinned` test below, making the change deliberate.

use trading_core::{Symbol, Venue};

/// 10-symbol XRP-first universe in operator-locked scan order
/// (ui-rethink-phase-a-lab R3.2 / Design Q-A3).
///
/// Used by `screens::lab::view` to render the pair-chip row in the
/// correct order. The slice is `&'static` so it can be passed into
/// iced view functions without lifetime threading.
pub const XRP_FIRST_UNIVERSE: &[(Venue, &str)] = &[
    (Venue::Binance, "XRPUSDT"),
    (Venue::Binance, "ETHUSDT"),
    (Venue::Binance, "BTCUSDT"),
    (Venue::Binance, "ADAUSDT"),
    (Venue::Binance, "AVAXUSDT"),
    (Venue::Binance, "BNBUSDT"),
    (Venue::Binance, "DOGEUSDT"),
    (Venue::Binance, "DOTUSDT"),
    (Venue::Binance, "LINKUSDT"),
    (Venue::Binance, "SOLUSDT"),
];

/// Build the XRP-first universe as a `Vec<(Venue, Symbol)>` — suitable
/// for initialising `Cockpit::universe` on cold-start or for lab-specific
/// routing that needs owned `Symbol` values.
#[must_use]
pub fn xrp_first_universe_owned() -> Vec<(Venue, Symbol)> {
    XRP_FIRST_UNIVERSE
        .iter()
        .map(|(v, s)| (*v, Symbol::new(*s)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// T-D-8 — XRP-first universe order is pinned. Changing any pair or
    /// its position in the list requires a deliberate test edit.
    #[test]
    fn xrp_first_order_pinned() {
        let pairs: Vec<&str> = XRP_FIRST_UNIVERSE.iter().map(|(_, s)| *s).collect();
        assert_eq!(
            pairs,
            &[
                "XRPUSDT", "ETHUSDT", "BTCUSDT", "ADAUSDT", "AVAXUSDT", "BNBUSDT", "DOGEUSDT",
                "DOTUSDT", "LINKUSDT", "SOLUSDT",
            ],
            "XRP-first universe order must match the operator-locked spec"
        );
    }

    /// T-D-8 — universe has exactly 10 entries.
    #[test]
    fn universe_has_10_entries() {
        assert_eq!(XRP_FIRST_UNIVERSE.len(), 10);
    }

    /// T-D-8 — all venues are Binance at Phase A.
    #[test]
    fn all_venues_binance() {
        for (venue, sym) in XRP_FIRST_UNIVERSE {
            assert_eq!(
                *venue,
                Venue::Binance,
                "expected Binance for {sym} at Phase A"
            );
        }
    }

    /// T-D-8 — first three are XRP, ETH, BTC (operator preference).
    #[test]
    fn top_three_are_operator_preferred() {
        let top: Vec<&str> = XRP_FIRST_UNIVERSE.iter().take(3).map(|(_, s)| *s).collect();
        assert_eq!(top, &["XRPUSDT", "ETHUSDT", "BTCUSDT"]);
    }

    /// T-D-8 — `xrp_first_universe_owned()` produces the same symbols.
    #[test]
    fn owned_matches_static() {
        let owned = xrp_first_universe_owned();
        assert_eq!(owned.len(), XRP_FIRST_UNIVERSE.len());
        for ((v1, s1), (v2, s2)) in owned.iter().zip(XRP_FIRST_UNIVERSE.iter()) {
            assert_eq!(*v1, *v2);
            assert_eq!(s1.0.as_str(), *s2);
        }
    }
}
