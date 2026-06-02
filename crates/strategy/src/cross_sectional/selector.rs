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

/// Select every warmed asset whose OWN score exceeds `entry_threshold` (D-TSM.1, M-DEV-3).
///
/// This is the time-series long/flat selector: NO cross-sectional ranking, NO top-K.
/// Each warmed asset (score = `Some(_)`) independently decides long/flat on its own
/// score vs the entry threshold. Cardinality is variable (0..N); all below-threshold
/// or still-warming names are absent from the result (→ flat / cash).
///
/// # Behaviour
///
/// 1. Filters out `None` (warmup-incomplete) entries — warming names are always flat.
/// 2. Keeps only entries where `score > entry_threshold`.
/// 3. Counts `n_above`; assigns each a nominal weight `exposure_cap / n_above`.
///    **This weight is a membership sentinel** — `run_path` books the fixed fraction;
///    the map is used for *membership*, not for re-sizing the engine (D-TSM.2).
/// 4. Returns empty map when `n_above == 0` (→ all-flat → the goes-flat path, F-TSM.4).
///
/// **Determinism:** iterates `BTreeMap` in alphabetical order (no `HashMap`, no
/// unstable sort) — two-run byte-identity by construction (D-TSM.6, F-TSM.5).
#[must_use]
pub fn select_above_threshold(
    scores: &BTreeMap<Symbol, Option<Decimal>>,
    entry_threshold: Decimal,
    exposure_cap: Decimal,
) -> BTreeMap<Symbol, Decimal> {
    // Collect symbols above threshold, in BTreeMap (alphabetical) order.
    // BTreeMap iteration is already deterministic; no extra sort needed.
    let above: Vec<Symbol> = scores
        .iter()
        .filter_map(|(s, v)| {
            v.and_then(|score| {
                if score > entry_threshold {
                    Some(s.clone())
                } else {
                    None
                }
            })
        })
        .collect();

    let n_above = above.len();
    if n_above == 0 {
        return BTreeMap::new();
    }

    let leg_weight = exposure_cap / Decimal::from(n_above as u64);

    above.into_iter().map(|s| (s, leg_weight)).collect()
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

    // ── M-DEV-3: select_above_threshold unit tests ────────────────────────────

    /// M-DEV-3 (a): 3 symbols — one above, one below, one at the threshold.
    /// Only the above-threshold symbol is selected; at-threshold and below are flat.
    #[test]
    fn m_dev3_above_below_at_threshold() {
        let sc = scores(&[
            ("ADAUSDT", Some(0.05)),  // above threshold (0.02)
            ("BTCUSDT", Some(0.02)),  // AT threshold — NOT above (strict >)
            ("ETHUSDT", Some(-0.01)), // below threshold
        ]);
        let result = select_above_threshold(&sc, dec!(0.02), dec!(0.5));
        assert_eq!(result.len(), 1, "only one symbol above threshold 0.02");
        assert!(
            result.contains_key(&sym("ADAUSDT")),
            "ADAUSDT (score=0.05 > 0.02) must be selected"
        );
        assert!(
            !result.contains_key(&sym("BTCUSDT")),
            "BTCUSDT (score=0.02, AT threshold, NOT above) must NOT be selected"
        );
        assert!(
            !result.contains_key(&sym("ETHUSDT")),
            "ETHUSDT (score=-0.01 < 0.02) must NOT be selected"
        );
    }

    /// M-DEV-3 (b): All symbols below threshold → empty map (→ all-flat, goes-flat path).
    /// This is the F-TSM.4 selector unit test: the goes-flat falsifier at selector level.
    #[test]
    fn m_dev3_all_below_threshold_returns_empty() {
        let sc = scores(&[
            ("BTCUSDT", Some(-0.05)),
            ("ETHUSDT", Some(-0.10)),
            ("BNBUSDT", Some(0.01)),
        ]);
        // Threshold = 0.02 → all below
        let result = select_above_threshold(&sc, dec!(0.02), dec!(0.5));
        assert!(
            result.is_empty(),
            "all below threshold → empty result (goes-flat path, F-TSM.4)"
        );
    }

    /// M-DEV-3 (c): All above threshold → all selected, weights = exposure_cap / n_above.
    #[test]
    fn m_dev3_all_above_threshold_all_selected() {
        let sc = scores(&[
            ("BTCUSDT", Some(0.10)),
            ("ETHUSDT", Some(0.05)),
            ("BNBUSDT", Some(0.03)),
        ]);
        let result = select_above_threshold(&sc, dec!(0.00), dec!(0.5));
        assert_eq!(result.len(), 3, "all 3 symbols above threshold 0.00");
        let expected_weight = dec!(0.5) / dec!(3);
        for w in result.values() {
            assert_eq!(
                *w, expected_weight,
                "weight should be exposure_cap / n_above"
            );
        }
    }

    /// M-DEV-3 (d): Warmup-incomplete (None) entries are excluded (same as top_k_long).
    #[test]
    fn m_dev3_warmup_incomplete_excluded() {
        let sc = scores(&[
            ("BTCUSDT", None),        // warming up — excluded regardless of threshold
            ("ETHUSDT", Some(0.10)),  // above threshold — included
            ("BNBUSDT", Some(-0.05)), // below threshold — excluded
        ]);
        let result = select_above_threshold(&sc, dec!(0.02), dec!(0.5));
        assert_eq!(
            result.len(),
            1,
            "only warmed AND above-threshold symbols selected"
        );
        assert!(result.contains_key(&sym("ETHUSDT")));
        assert!(!result.contains_key(&sym("BTCUSDT")));
        assert!(!result.contains_key(&sym("BNBUSDT")));
    }

    /// M-DEV-3 (e): Two-run identity — two calls on the same input produce identical results.
    /// This guards the determinism of the alphabetical BTreeMap iteration (D-TSM.6).
    #[test]
    fn m_dev3_two_run_identity() {
        let sc = scores(&[
            ("ADAUSDT", Some(0.05)),
            ("BTCUSDT", Some(0.03)),
            ("ETHUSDT", Some(-0.02)),
            ("LINKUSDT", Some(0.01)),
        ]);
        let run1 = select_above_threshold(&sc, dec!(0.02), dec!(0.5));
        let run2 = select_above_threshold(&sc, dec!(0.02), dec!(0.5));
        assert_eq!(
            run1, run2,
            "two runs on the same input must produce byte-identical results (D-TSM.6)"
        );
    }

    /// M-DEV-3 (f): Alphabetical determinism — symbols are returned in alphabetical order.
    /// BTreeMap iteration guarantees this; verify the collected keys are sorted.
    #[test]
    fn m_dev3_alphabetical_order() {
        let sc = scores(&[
            ("ZCUSDT", Some(0.10)),  // Z first in input (but last alphabetically)
            ("ADAUSDT", Some(0.08)), // A
            ("BTCUSDT", Some(0.06)), // B
        ]);
        let result = select_above_threshold(&sc, dec!(0.00), dec!(0.5));
        assert_eq!(result.len(), 3);
        // BTreeMap keys are always sorted alphabetically.
        let keys: Vec<String> = result.keys().map(|s| s.to_string()).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(
            keys, sorted,
            "result keys must be in alphabetical order (BTreeMap guarantee)"
        );
    }
}
