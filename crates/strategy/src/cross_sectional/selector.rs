//! Top-K symbol selector for cross-sectional momentum (T604 — v1 R4).
//!
//! `top_k_long` takes a BTreeMap of `Option<Decimal>` scores (None = warming up),
//! filters incomplete entries, sorts descending with alphabetical tie-break,
//! takes the first `k`, and assigns equal weight `exposure_cap / k` per leg.

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use trading_core::Symbol;

/// Select the top-K symbols by momentum score, returning their target weights.
///
/// # Behaviour
///
/// 1. Filters out `None` (warmup-incomplete) entries per R4.5.
/// 2. Sorts descending by score with alphabetical tie-break (R4.4).
///    BTreeMap iteration is already alphabetical so a stable sort by score
///    (descending) gives the correct tie-break without extra sorting.
/// 3. Takes the first `k` entries.
/// 4. Assigns each a weight of `exposure_cap / k`.
///
/// Returns an empty map if fewer than 1 warmed symbol is available or `k == 0`.
#[must_use]
pub fn top_k_long(
    scores: &BTreeMap<Symbol, Option<Decimal>>,
    k: u32,
    exposure_cap: Decimal,
) -> BTreeMap<Symbol, Decimal> {
    if k == 0 {
        return BTreeMap::new();
    }

    let k_dec = Decimal::from(k);
    let leg_weight = if k_dec > Decimal::ZERO {
        exposure_cap / k_dec
    } else {
        Decimal::ZERO
    };

    // Collect (symbol, score) pairs with Some scores, preserving BTreeMap
    // (alphabetical) iteration order so that when we stable-sort by score
    // descending, equal scores retain alphabetical order.
    let mut warmed: Vec<(Symbol, Decimal)> = scores
        .iter()
        .filter_map(|(s, v)| v.map(|score| (s.clone(), score)))
        .collect();

    // Stable sort descending by score — equal scores keep alphabetical order
    // (which comes from BTreeMap iteration).
    warmed.sort_by(|a, b| b.1.cmp(&a.1));

    warmed
        .into_iter()
        .take(k as usize)
        .map(|(s, _score)| (s, leg_weight))
        .collect()
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn sym(s: &str) -> Symbol {
        Symbol::new(s)
    }

    fn scores(pairs: &[(&str, Option<f64>)]) -> BTreeMap<Symbol, Option<Decimal>> {
        pairs
            .iter()
            .map(|(s, v)| {
                (
                    sym(s),
                    v.map(|f| Decimal::try_from(f).expect("test f64 to Decimal")),
                )
            })
            .collect()
    }

    #[test]
    fn t604_top3_from_ten_synthetic_scores() {
        // 10 symbols with increasing scores; top-3 should be the three highest.
        let sc = scores(&[
            ("ADAUSDT", Some(0.1)),
            ("AVAXUSDT", Some(0.2)),
            ("BNBUSDT", Some(0.3)),
            ("BTCUSDT", Some(0.9)), // highest
            ("DOGEUSDT", Some(0.4)),
            ("DOTUSDT", Some(0.5)),
            ("ETHUSDT", Some(0.8)), // second
            ("LINKUSDT", Some(0.6)),
            ("SOLUSDT", Some(0.7)), // third
            ("XRPUSDT", Some(0.05)),
        ]);
        let result = top_k_long(&sc, 3, dec!(0.5));
        assert_eq!(result.len(), 3);
        assert!(result.contains_key(&sym("BTCUSDT")));
        assert!(result.contains_key(&sym("ETHUSDT")));
        assert!(result.contains_key(&sym("SOLUSDT")));
        // Weight = 0.5 / 3
        for w in result.values() {
            let expected = dec!(0.5) / dec!(3);
            assert_eq!(*w, expected, "weight should be exposure_cap/k");
        }
    }

    #[test]
    fn t604_alphabetical_tie_break() {
        // Two symbols with identical top score — alphabetically first wins.
        let sc = scores(&[
            ("BTCUSDT", Some(1.0)), // tied, alphabetically first
            ("ETHUSDT", Some(1.0)), // tied
            ("BNBUSDT", Some(0.5)),
        ]);
        let result = top_k_long(&sc, 1, dec!(0.5));
        assert_eq!(result.len(), 1);
        // BTreeMap iteration gives BNBUSDT < BTCUSDT < ETHUSDT, stable sort
        // preserves alphabetical order for ties → BTCUSDT wins.
        assert!(
            result.contains_key(&sym("BTCUSDT")),
            "alphabetical tie-break failed"
        );
    }

    #[test]
    fn t604_warmup_incomplete_excluded() {
        let sc = scores(&[
            ("BTCUSDT", None), // warming up — excluded
            ("ETHUSDT", None), // warming up — excluded
            ("BNBUSDT", Some(0.5)),
            ("SOLUSDT", Some(0.3)),
        ]);
        let result = top_k_long(&sc, 3, dec!(0.5));
        // Only 2 warmed symbols, so at most 2 selected (not 3)
        assert_eq!(result.len(), 2);
        assert!(!result.contains_key(&sym("BTCUSDT")));
        assert!(!result.contains_key(&sym("ETHUSDT")));
    }

    #[test]
    fn t604_k_zero_returns_empty() {
        let sc = scores(&[("BTCUSDT", Some(1.0))]);
        let result = top_k_long(&sc, 0, dec!(0.5));
        assert!(result.is_empty());
    }

    #[test]
    fn t604_all_warmup_incomplete_returns_empty() {
        let sc = scores(&[("BTCUSDT", None), ("ETHUSDT", None)]);
        let result = top_k_long(&sc, 2, dec!(0.5));
        assert!(result.is_empty());
    }
}
