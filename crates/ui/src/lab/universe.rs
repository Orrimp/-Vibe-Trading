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

/// 10-ticker Yahoo crypto-mirror universe (lab-yahoo-realdata Q2 = (a) / R4.1 / T-AR2).
///
/// **UI display contract.** Per Q6 = (a), the UI renders the Binance-style symbols
/// (`BTCUSDT`, `ETHUSDT`, ...); conversion to Yahoo-native (`BTC-USD`, ...) happens
/// at the dispatch boundary in `lab::runner::preload_yahoo_bars` via
/// `data::yahoo::binance_to_yahoo_ticker`.
///
/// Order mirrors `XRP_FIRST_UNIVERSE`: XRP, ETH, BTC first (operator preference);
/// ADA, AVAX, BNB, DOGE, DOT, LINK, SOL alphabetical remainder.
pub const YAHOO_CRYPTO_UNIVERSE: &[(Venue, &str)] = &[
    (Venue::Yahoo, "XRPUSDT"),
    (Venue::Yahoo, "ETHUSDT"),
    (Venue::Yahoo, "BTCUSDT"),
    (Venue::Yahoo, "ADAUSDT"),
    (Venue::Yahoo, "AVAXUSDT"),
    (Venue::Yahoo, "BNBUSDT"),
    (Venue::Yahoo, "DOGEUSDT"),
    (Venue::Yahoo, "DOTUSDT"),
    (Venue::Yahoo, "LINKUSDT"),
    (Venue::Yahoo, "SOLUSDT"),
];

/// Build the Yahoo crypto-mirror universe as a `Vec<(Venue, Symbol)>` — suitable
/// for pair-chip row rendering when `data_source == YahooCache`.
#[must_use]
pub fn yahoo_crypto_universe_owned() -> Vec<(Venue, Symbol)> {
    YAHOO_CRYPTO_UNIVERSE
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

    // ── T-AR2 — YAHOO_CRYPTO_UNIVERSE tests ─────────────────────────────────

    /// T-AR2 — Yahoo crypto universe order is pinned.
    /// Changing any pair or its position requires a deliberate test edit.
    #[test]
    fn yahoo_crypto_universe_order_pinned() {
        let pairs: Vec<&str> = YAHOO_CRYPTO_UNIVERSE.iter().map(|(_, s)| *s).collect();
        assert_eq!(
            pairs,
            &[
                "XRPUSDT", "ETHUSDT", "BTCUSDT", "ADAUSDT", "AVAXUSDT", "BNBUSDT", "DOGEUSDT",
                "DOTUSDT", "LINKUSDT", "SOLUSDT",
            ],
            "Yahoo crypto universe order must match the operator-locked spec (T-AR2)"
        );
    }

    /// T-AR2 — Yahoo crypto universe has exactly 10 entries.
    #[test]
    fn yahoo_crypto_universe_has_10_entries() {
        assert_eq!(YAHOO_CRYPTO_UNIVERSE.len(), 10);
    }

    /// T-AR2 — all Yahoo universe venues are `Venue::Yahoo`.
    #[test]
    fn yahoo_crypto_universe_all_venues_yahoo() {
        for (venue, sym) in YAHOO_CRYPTO_UNIVERSE {
            assert_eq!(
                *venue,
                Venue::Yahoo,
                "expected Venue::Yahoo for {sym} in YAHOO_CRYPTO_UNIVERSE"
            );
        }
    }

    /// T-AR2 — Yahoo universe symbols match XRP-first ordering (same symbol set).
    #[test]
    fn yahoo_crypto_universe_symbols_match_xrp_first() {
        let yahoo_syms: Vec<&str> = YAHOO_CRYPTO_UNIVERSE.iter().map(|(_, s)| *s).collect();
        let binance_syms: Vec<&str> = XRP_FIRST_UNIVERSE.iter().map(|(_, s)| *s).collect();
        assert_eq!(
            yahoo_syms, binance_syms,
            "YAHOO_CRYPTO_UNIVERSE symbols must match XRP_FIRST_UNIVERSE in identical order"
        );
    }

    /// T-AR2 — `yahoo_crypto_universe_owned()` produces the same symbols as the static const.
    #[test]
    fn yahoo_owned_matches_static() {
        let owned = yahoo_crypto_universe_owned();
        assert_eq!(owned.len(), YAHOO_CRYPTO_UNIVERSE.len());
        for ((v1, s1), (v2, s2)) in owned.iter().zip(YAHOO_CRYPTO_UNIVERSE.iter()) {
            assert_eq!(*v1, *v2);
            assert_eq!(s1.0.as_str(), *s2);
        }
    }
}
