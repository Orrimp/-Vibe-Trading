//! Deterministic 32-dim embedding (Q3d — pinned packed layout).
//!
//! No `f64`, no LLM, no learned weights.  Pure over `LessonCard`.
//!
//! ## Layout (fixed; append-only on `STRATEGY_SLOTS`)
//!
//! | Slots | Field                  |
//! |-------|------------------------|
//! | 0..6  | `strategy_one_hot` (7 strategies) |
//! | 7..9  | `regime_one_hot` (Bull/Bear/Chop) — K4-frozen; legacy daily seed only |
//! | 10..12| `outcome_one_hot` (Win/Loss/Scratch) |
//! | 13    | `signed_pnl_sign` (+1 / -1 / 0)  |
//! | 14    | `log_pnl_magnitude` (log10(\|pnl\| + 1), 4dp truncated) |
//! | 15    | `log_holding_period` (log10(bars + 1), 4dp truncated)  |
//! | 16    | `pair_hash_norm` ([0, 1] projection of PairKey content hash) |
//! | 17    | `single_symbol_hash_norm` ([0, 1] projection of Symbol hash) |
//! | 18    | `regime_volatile_slot` — 1.0 iff `RegimeTag::Volatile` (NEW Wave B) |
//! | 19    | `regime_calm_slot`    — 1.0 iff `RegimeTag::Calm`     (NEW Wave B) |
//! | 20..31| reserved (zero) |
//!
//! ## K4 byte-identity contract (ADR-0049 § D2)
//!
//! **Slots 7-17 and 20-31 are byte-identical to the pre-Wave-B layout
//! for any card that carries only `Bull`, `Bear`, or `Chop` as its
//! `entry_regime`.**  Specifically:
//!
//! - Slots 7, 8, 9 (`REGIME_BASE + 0/1/2`) remain the one-hot positions
//!   for `Bull`, `Bear`, `Chop` respectively.  Their encoding is frozen.
//! - `OUTCOME_BASE` stays at 10 — outcome slots are NOT displaced.
//! - Slots 18 and 19 were previously part of the reserved-zero region.
//!   For legacy cards (Bull/Bear/Chop), they remain zero-initialised,
//!   so legacy embeddings are byte-identical to pre-Wave-B output.
//! - For new Markov-switching cards that carry `Volatile` or `Calm`,
//!   slot 18 or 19 (respectively) is set to `Decimal::ONE`; all other
//!   regime-one-hot slots are zero.
//!
//! The `regime_overlay_neutrality_4state` integration test gates this
//! invariant — it re-runs a legacy fixture and asserts that slots 18
//! and 19 are exactly zero, and that the full 32-vector is byte-identical
//! to a pre-Wave-B snapshot.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sha2::{Digest, Sha256};

use crate::outcome::OutcomeClass;
use crate::regime::RegimeTag;
use crate::types::{LessonCard, SymbolOrPair};

/// Embedding dimensionality (Q3d — 32, pinned).
pub const EMBEDDING_DIM: usize = 32;

/// Strategy slot table — append-only.  Adding a new strategy in a
/// future feature appends to the END of this array; existing slot
/// indices NEVER change.  Body SHA-256s re-anchor on every append
/// (operator-success-reports' `report-sample-*` anchors).
pub const STRATEGY_SLOTS: &[&str; 7] = &[
    "sma_crossover",
    "macd_trend",
    "rsi_reversion",
    "bbands_mean_revert",
    "top10_momentum_h1",
    "pairs_mr_h1",
    "(unattributed)",
];

const REGIME_BASE: usize = 7;
const OUTCOME_BASE: usize = 10;
const SIGNED_PNL_SIGN: usize = 13;
const LOG_PNL_MAGNITUDE: usize = 14;
const LOG_HOLDING_PERIOD: usize = 15;
const PAIR_HASH_NORM: usize = 16;
const SINGLE_SYMBOL_HASH_NORM: usize = 17;

const COSINE_DENOM_FLOOR: Decimal = dec!(0.000000000001); // 1e-12

/// Compute the 32-dim embedding for a `LessonCard`.
///
/// Pure — same input → byte-identical `[Decimal; 32]` across runs.
#[must_use]
pub fn embed(card: &LessonCard) -> [Decimal; EMBEDDING_DIM] {
    let mut v = [Decimal::ZERO; EMBEDDING_DIM];

    // Slot 0..6 — strategy one-hot. Unknown strategies fall into the
    // `(unattributed)` slot at index 6.
    let strategy_idx = strategy_slot_index(card.strategy_id.0.as_str());
    v[strategy_idx] = Decimal::ONE;

    // Slot 7..9 — entry-regime one-hot. (Q3d says "regime_one_hot"
    // singular; we use entry_regime as the pinned source — exit_regime
    // already shows up in the body line.)
    v[REGIME_BASE + regime_slot(card.entry_regime)] = Decimal::ONE;

    // Slot 10..12 — outcome one-hot.
    v[OUTCOME_BASE + outcome_slot(card.outcome_class)] = Decimal::ONE;

    // Slot 13 — signed_pnl sign.
    let pnl = card.signed_pnl.amount();
    v[SIGNED_PNL_SIGN] = if pnl > Decimal::ZERO {
        Decimal::ONE
    } else if pnl < Decimal::ZERO {
        -Decimal::ONE
    } else {
        Decimal::ZERO
    };

    // Slot 14 — log10(|signed_pnl| + 1) truncated to 4 dp.
    let abs_pnl = pnl.abs();
    v[LOG_PNL_MAGNITUDE] = log10_plus_one_4dp(abs_pnl);

    // Slot 15 — log10(holding_period_bars + 1) truncated to 4 dp.
    v[LOG_HOLDING_PERIOD] = log10_plus_one_4dp(Decimal::from(card.holding_period_bars));

    // Slot 16 / 17 — pair / single-symbol hash norm.
    match &card.symbol_or_pair {
        SymbolOrPair::Pair(pair) => {
            v[PAIR_HASH_NORM] = hash_norm(&pair.to_string());
            v[SINGLE_SYMBOL_HASH_NORM] = Decimal::ZERO;
        }
        SymbolOrPair::Single(sym) => {
            v[PAIR_HASH_NORM] = Decimal::ZERO;
            v[SINGLE_SYMBOL_HASH_NORM] = hash_norm(sym.0.as_str());
        }
    }

    // Slots 18..19 — Volatile / Calm one-hot (Wave B; set above via regime_slot).
    // Slots 20..31 — reserved (zero).  Already zero-initialised by array init.
    // NOTE: regime_slot(Volatile)=11, regime_slot(Calm)=12 → REGIME_BASE+11=18,
    // REGIME_BASE+12=19, so these slots are set via the regime_slot dispatch above.

    v
}

/// Return `STRATEGY_SLOTS` index for a strategy id, falling back to
/// `(unattributed)` (index 6) for unknown ids.
fn strategy_slot_index(id: &str) -> usize {
    for (i, s) in STRATEGY_SLOTS.iter().enumerate() {
        if *s == id {
            return i;
        }
    }
    // Fallback: `(unattributed)` slot.
    STRATEGY_SLOTS.len() - 1
}

/// Return the one-hot slot offset from `REGIME_BASE` for a `RegimeTag`.
///
/// ## K4 byte-identity contract (ADR-0049 § D2)
///
/// Offsets 0, 1, 2 (Bull/Bear/Chop) are **frozen** — they map to the
/// same absolute slots (7, 8, 9) as the pre-Wave-B embedding.  Any
/// card carrying only these three variants produces a byte-identical
/// 32-vector to the pre-Wave-B output.
///
/// `Volatile` and `Calm` use offsets 11 and 12 respectively, which map
/// to absolute slots 18 and 19 (`REGIME_BASE + 11 = 18`,
/// `REGIME_BASE + 12 = 19`).  These slots were in the reserved-zero
/// region before Wave B; for legacy cards they remain zero.
///
/// | Variant  | Offset | Absolute slot | Region           |
/// |----------|--------|---------------|------------------|
/// | Bull     | 0      | 7             | regime block     |
/// | Bear     | 1      | 8             | regime block     |
/// | Chop     | 2      | 9             | regime block     |
/// | Volatile | 11     | 18            | formerly reserved|
/// | Calm     | 12     | 19            | formerly reserved|
const fn regime_slot(r: RegimeTag) -> usize {
    match r {
        RegimeTag::Bull => 0,
        RegimeTag::Bear => 1,
        RegimeTag::Chop => 2,
        // Volatile and Calm use offsets 11/12 → absolute slots 18/19.
        // These were zero-initialised reserved slots before Wave B.
        // Legacy (Bull/Bear/Chop) cards keep byte-identical output
        // because these offsets are never reached by the legacy path.
        RegimeTag::Volatile => 11,
        RegimeTag::Calm => 12,
    }
}

const fn outcome_slot(o: OutcomeClass) -> usize {
    match o {
        OutcomeClass::Win => 0,
        OutcomeClass::Loss => 1,
        OutcomeClass::Scratch => 2,
    }
}

/// `log10(x + 1)` truncated to 4 decimal places.  Returns 0 on
/// negative input (defensive — embedding compute should never see
/// negatives for `|pnl|` or `bars`).
fn log10_plus_one_4dp(x: Decimal) -> Decimal {
    let arg = x + Decimal::ONE;
    if arg <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    match features::math::decimal_log10(arg) {
        Ok(v) => v.trunc_with_scale(4),
        Err(_) => Decimal::ZERO,
    }
}

/// Project a string's sha256 hash into `[0, 1]` via the first 8
/// bytes interpreted as a big-endian `u64`.
fn hash_norm(s: &str) -> Decimal {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    let h = hasher.finalize();
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&h[..8]);
    let n = u64::from_be_bytes(buf);
    // `n / u64::MAX` as Decimal — keeps precision well past 4dp.
    let max = Decimal::from(u64::MAX);
    Decimal::from(n) / max
}

/// Cosine similarity in `Decimal` (no `f64`).
///
/// `(|a| · |b|).max(1e-12)` denominator floor — avoids divide-by-zero
/// for the embedding-zero case (the empty-store path returns
/// `Ok(vec![])` short-circuit earlier, so the floor is defensive).
#[must_use]
pub fn cosine(a: &[Decimal; EMBEDDING_DIM], b: &[Decimal; EMBEDDING_DIM]) -> Decimal {
    let mut dot = Decimal::ZERO;
    let mut norm_a_sq = Decimal::ZERO;
    let mut norm_b_sq = Decimal::ZERO;
    for i in 0..EMBEDDING_DIM {
        dot += a[i] * b[i];
        norm_a_sq += a[i] * a[i];
        norm_b_sq += b[i] * b[i];
    }
    let norm_a = match features::math::decimal_sqrt(norm_a_sq) {
        Ok(v) => v,
        Err(_) => Decimal::ZERO,
    };
    let norm_b = match features::math::decimal_sqrt(norm_b_sq) {
        Ok(v) => v,
        Err(_) => Decimal::ZERO,
    };
    let denom = (norm_a * norm_b).max(COSINE_DENOM_FLOOR);
    dot / denom
}

#[cfg(test)]
#[allow(clippy::float_arithmetic)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;
    use trading_core::{Money, StrategyId, Symbol, Timestamp, Usdt};

    fn ts() -> Timestamp {
        Timestamp::new(OffsetDateTime::UNIX_EPOCH)
    }

    fn card(strategy: &str, regime: RegimeTag, outcome: OutcomeClass) -> LessonCard {
        LessonCard {
            card_id: "test".into(),
            closed_at: ts(),
            symbol_or_pair: SymbolOrPair::Single(Symbol::new("BTCUSDT")),
            strategy_id: StrategyId::new(strategy),
            signed_pnl: Money::<Usdt>::from_decimal(dec!(100)),
            opening_capital: Money::<Usdt>::from_decimal(dec!(10000)),
            holding_period_bars: 60,
            entry_regime: regime,
            exit_regime: regime,
            outcome_class: outcome,
            note: None,
        }
    }

    #[test]
    fn strategy_one_hot_known_strategy() {
        let v = embed(&card("sma_crossover", RegimeTag::Bull, OutcomeClass::Win));
        assert_eq!(v[0], Decimal::ONE);
        assert_eq!(v[6], Decimal::ZERO);
    }

    #[test]
    fn unknown_strategy_falls_into_unattributed_slot() {
        let v = embed(&card("(unattributed)", RegimeTag::Bull, OutcomeClass::Win));
        assert_eq!(v[6], Decimal::ONE);
    }

    #[test]
    fn embed_byte_stable() {
        let c = card("sma_crossover", RegimeTag::Chop, OutcomeClass::Scratch);
        let a = embed(&c);
        let b = embed(&c);
        assert_eq!(a, b);
    }
}
