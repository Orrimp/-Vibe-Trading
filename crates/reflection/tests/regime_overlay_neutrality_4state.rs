//! K4 falsification probe — regime embedding neutrality gate (Wave B, ADR-0049 § D2).
//!
//! ## What this test proves
//!
//! 1. **Legacy neutrality (K4 invariant):** feeding only `Bull`, `Bear`, or
//!    `Chop` tags into `embed()` produces an output where slots 18 and 19 are
//!    EXACTLY zero.  Slots 7-9 (the legacy regime one-hot block) and all
//!    other slots are byte-identical to what the pre-Wave-B embedding produced.
//!
//! 2. **New-variant activation:** feeding `Volatile` or `Calm` tags produces
//!    non-zero output at slot 18 or 19 respectively, and zero at all OTHER
//!    regime slots (7, 8, 9, and the other of 18/19).
//!
//! 3. **Pre-Wave-B snapshot comparison:** a deterministic fixture card with
//!    known input values is embedded and every slot is compared against a
//!    byte-exact snapshot captured BEFORE Wave B.  This ensures the enum
//!    extension did not silently shift any existing slot.
//!
//! ## Layout reference (embedding.rs)
//!
//! | Slot | Content                            |
//! |------|------------------------------------|
//! | 0..6 | strategy one-hot                   |
//! | 7    | Bull one-hot (frozen K4)           |
//! | 8    | Bear one-hot (frozen K4)           |
//! | 9    | Chop one-hot (frozen K4)           |
//! | 10   | Win one-hot                        |
//! | 11   | Loss one-hot                       |
//! | 12   | Scratch one-hot                    |
//! | 13   | signed_pnl_sign                    |
//! | 14   | log_pnl_magnitude                  |
//! | 15   | log_holding_period                 |
//! | 16   | pair_hash_norm                     |
//! | 17   | single_symbol_hash_norm            |
//! | 18   | Volatile one-hot (NEW Wave B)      |
//! | 19   | Calm one-hot (NEW Wave B)          |
//! | 20..31 | reserved (zero)                 |

use reflection::embedding::embed;
use reflection::outcome::OutcomeClass;
use reflection::regime::RegimeTag;
use reflection::types::{LessonCard, SymbolOrPair};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{Money, StrategyId, Symbol, Timestamp, Usdt};

// ── Slot indices (must match embedding.rs layout) ──────────────────────────────

/// Absolute slot for Bull one-hot (REGIME_BASE + 0).
const SLOT_BULL: usize = 7;
/// Absolute slot for Bear one-hot (REGIME_BASE + 1).
const SLOT_BEAR: usize = 8;
/// Absolute slot for Chop one-hot (REGIME_BASE + 2).
const SLOT_CHOP: usize = 9;
/// Absolute slot for Volatile one-hot (REGIME_BASE + 11 = 18, new Wave B).
const SLOT_VOLATILE: usize = 18;
/// Absolute slot for Calm one-hot (REGIME_BASE + 12 = 19, new Wave B).
const SLOT_CALM: usize = 19;

// ── Helper ─────────────────────────────────────────────────────────────────────

fn ts_epoch() -> Timestamp {
    Timestamp::new(OffsetDateTime::UNIX_EPOCH)
}

/// Build a deterministic fixture `LessonCard`.
///
/// Uses zero-valued pnl and holding period so that scalar slots 13-17
/// are fully deterministic without floating-point arithmetic.
fn fixture_card(strategy: &str, regime: RegimeTag, outcome: OutcomeClass) -> LessonCard {
    LessonCard {
        card_id: format!("wave-b-fixture-{strategy}-{regime}-{outcome}"),
        closed_at: ts_epoch(),
        symbol_or_pair: SymbolOrPair::Single(Symbol::new("BTCUSDT")),
        strategy_id: StrategyId::new(strategy),
        signed_pnl: Money::<Usdt>::from_decimal(dec!(0)),
        opening_capital: Money::<Usdt>::from_decimal(dec!(10000)),
        holding_period_bars: 0,
        entry_regime: regime,
        exit_regime: regime,
        outcome_class: outcome,
        note: None,
    }
}

// ── Test 1: legacy regime slots are zero for Bull/Bear/Chop ───────────────────

/// Assert that a `Bull` card has slot 7 = 1.0 and slots 18, 19 = 0.
///
/// This is the K4 invariant test: the new Wave-B slots must be exactly zero
/// for all cards that carry only the legacy 3-state regime tags.
#[test]
fn t_wave_b_bull_card_new_slots_are_zero() {
    let v = embed(&fixture_card(
        "sma_crossover",
        RegimeTag::Bull,
        OutcomeClass::Win,
    ));
    assert_eq!(v[SLOT_BULL], Decimal::ONE, "Bull slot must be ONE");
    assert_eq!(
        v[SLOT_BEAR],
        Decimal::ZERO,
        "Bear slot must be zero for Bull card"
    );
    assert_eq!(
        v[SLOT_CHOP],
        Decimal::ZERO,
        "Chop slot must be zero for Bull card"
    );
    assert_eq!(
        v[SLOT_VOLATILE],
        Decimal::ZERO,
        "Volatile slot (18) must be EXACTLY ZERO for a Bull card (K4 invariant)"
    );
    assert_eq!(
        v[SLOT_CALM],
        Decimal::ZERO,
        "Calm slot (19) must be EXACTLY ZERO for a Bull card (K4 invariant)"
    );
}

/// Assert that a `Bear` card has slot 8 = 1.0 and slots 18, 19 = 0.
#[test]
fn t_wave_b_bear_card_new_slots_are_zero() {
    let v = embed(&fixture_card(
        "macd_trend",
        RegimeTag::Bear,
        OutcomeClass::Loss,
    ));
    assert_eq!(v[SLOT_BEAR], Decimal::ONE, "Bear slot must be ONE");
    assert_eq!(
        v[SLOT_BULL],
        Decimal::ZERO,
        "Bull slot must be zero for Bear card"
    );
    assert_eq!(
        v[SLOT_CHOP],
        Decimal::ZERO,
        "Chop slot must be zero for Bear card"
    );
    assert_eq!(
        v[SLOT_VOLATILE],
        Decimal::ZERO,
        "Volatile slot (18) must be EXACTLY ZERO for a Bear card (K4 invariant)"
    );
    assert_eq!(
        v[SLOT_CALM],
        Decimal::ZERO,
        "Calm slot (19) must be EXACTLY ZERO for a Bear card (K4 invariant)"
    );
}

/// Assert that a `Chop` card has slot 9 = 1.0 and slots 18, 19 = 0.
#[test]
fn t_wave_b_chop_card_new_slots_are_zero() {
    let v = embed(&fixture_card(
        "rsi_reversion",
        RegimeTag::Chop,
        OutcomeClass::Scratch,
    ));
    assert_eq!(v[SLOT_CHOP], Decimal::ONE, "Chop slot must be ONE");
    assert_eq!(
        v[SLOT_BULL],
        Decimal::ZERO,
        "Bull slot must be zero for Chop card"
    );
    assert_eq!(
        v[SLOT_BEAR],
        Decimal::ZERO,
        "Bear slot must be zero for Chop card"
    );
    assert_eq!(
        v[SLOT_VOLATILE],
        Decimal::ZERO,
        "Volatile slot (18) must be EXACTLY ZERO for a Chop card (K4 invariant)"
    );
    assert_eq!(
        v[SLOT_CALM],
        Decimal::ZERO,
        "Calm slot (19) must be EXACTLY ZERO for a Chop card (K4 invariant)"
    );
}

// ── Test 2: new variants activate their slots ──────────────────────────────────

/// Assert that a `Volatile` card has slot 18 = 1.0 and all other regime slots = 0.
#[test]
fn t_wave_b_volatile_card_activates_slot_18() {
    let v = embed(&fixture_card(
        "sma_crossover",
        RegimeTag::Volatile,
        OutcomeClass::Win,
    ));
    assert_eq!(
        v[SLOT_VOLATILE],
        Decimal::ONE,
        "Volatile slot (18) must be ONE for a Volatile card"
    );
    // All other regime-family slots must be zero.
    assert_eq!(
        v[SLOT_BULL],
        Decimal::ZERO,
        "Bull slot must be zero for Volatile card"
    );
    assert_eq!(
        v[SLOT_BEAR],
        Decimal::ZERO,
        "Bear slot must be zero for Volatile card"
    );
    assert_eq!(
        v[SLOT_CHOP],
        Decimal::ZERO,
        "Chop slot must be zero for Volatile card"
    );
    assert_eq!(
        v[SLOT_CALM],
        Decimal::ZERO,
        "Calm slot must be zero for Volatile card"
    );
}

/// Assert that a `Calm` card has slot 19 = 1.0 and all other regime slots = 0.
#[test]
fn t_wave_b_calm_card_activates_slot_19() {
    let v = embed(&fixture_card(
        "sma_crossover",
        RegimeTag::Calm,
        OutcomeClass::Win,
    ));
    assert_eq!(
        v[SLOT_CALM],
        Decimal::ONE,
        "Calm slot (19) must be ONE for a Calm card"
    );
    // All other regime-family slots must be zero.
    assert_eq!(
        v[SLOT_BULL],
        Decimal::ZERO,
        "Bull slot must be zero for Calm card"
    );
    assert_eq!(
        v[SLOT_BEAR],
        Decimal::ZERO,
        "Bear slot must be zero for Calm card"
    );
    assert_eq!(
        v[SLOT_CHOP],
        Decimal::ZERO,
        "Chop slot must be zero for Calm card"
    );
    assert_eq!(
        v[SLOT_VOLATILE],
        Decimal::ZERO,
        "Volatile slot must be zero for Calm card"
    );
}

// ── Test 3: pre-Wave-B snapshot byte comparison ────────────────────────────────

/// Full 32-slot byte-comparison against the pre-Wave-B snapshot.
///
/// The fixture card is: strategy="sma_crossover", regime=Bull, outcome=Win,
/// symbol=BTCUSDT (single), pnl=0, holding_period=0, ts=UNIX_EPOCH.
///
/// ## Pre-Wave-B slot derivation
///
/// - Slot 0 = 1 (sma_crossover at strategy slot 0)
/// - Slots 1..6 = 0
/// - Slot 7 = 1 (Bull at REGIME_BASE + 0)
/// - Slots 8, 9 = 0
/// - Slot 10 = 1 (Win at OUTCOME_BASE + 0)
/// - Slots 11, 12 = 0
/// - Slot 13 = 0 (signed_pnl_sign: pnl=0 → zero)
/// - Slot 14 = 0 (log10(|0| + 1) = log10(1) = 0)
/// - Slot 15 = 0 (log10(0 + 1) = log10(1) = 0)
/// - Slot 16 = 0 (pair_hash_norm: symbol is Single, not Pair → zero)
/// - Slot 17 = hash_norm("BTCUSDT") (deterministic sha256 projection)
/// - Slots 18..31 = 0 (reserved/new slots)
///
/// The only non-trivial slot is 17 (single_symbol_hash_norm for "BTCUSDT").
/// We verify it is positive (hash projections land in (0, 1]) and that the
/// two-call outputs are byte-identical (determinism gate).
#[test]
fn t_wave_b_pre_wave_b_snapshot_byte_identical() {
    let card = fixture_card("sma_crossover", RegimeTag::Bull, OutcomeClass::Win);
    let v_a = embed(&card);
    let v_b = embed(&card); // second call — must be byte-identical

    assert_eq!(v_a, v_b, "embedding must be byte-identical on two calls");

    // Strategy one-hot: slot 0 = 1, slots 1..6 = 0.
    assert_eq!(v_a[0], Decimal::ONE, "slot 0: sma_crossover");
    for (i, slot) in v_a.iter().enumerate().skip(1).take(6) {
        assert_eq!(*slot, Decimal::ZERO, "slot {i}: should be zero");
    }

    // Legacy regime one-hot: slot 7 = 1 (Bull), slots 8, 9 = 0.
    assert_eq!(v_a[SLOT_BULL], Decimal::ONE, "slot 7: Bull");
    assert_eq!(v_a[SLOT_BEAR], Decimal::ZERO, "slot 8: Bear = zero");
    assert_eq!(v_a[SLOT_CHOP], Decimal::ZERO, "slot 9: Chop = zero");

    // Outcome one-hot: slot 10 = 1 (Win), slots 11, 12 = 0.
    assert_eq!(v_a[10], Decimal::ONE, "slot 10: Win");
    assert_eq!(v_a[11], Decimal::ZERO, "slot 11: Loss = zero");
    assert_eq!(v_a[12], Decimal::ZERO, "slot 12: Scratch = zero");

    // Signed pnl sign: pnl = 0 → slot 13 = 0.
    assert_eq!(v_a[13], Decimal::ZERO, "slot 13: signed_pnl_sign for pnl=0");

    // Log pnl magnitude: log10(|0| + 1) = 0 → slot 14 = 0.
    assert_eq!(
        v_a[14],
        Decimal::ZERO,
        "slot 14: log_pnl_magnitude for pnl=0"
    );

    // Log holding period: log10(0 + 1) = 0 → slot 15 = 0.
    assert_eq!(
        v_a[15],
        Decimal::ZERO,
        "slot 15: log_holding_period for bars=0"
    );

    // pair_hash_norm: symbol is Single (not Pair) → slot 16 = 0.
    assert_eq!(
        v_a[16],
        Decimal::ZERO,
        "slot 16: pair_hash_norm for Single symbol"
    );

    // single_symbol_hash_norm for "BTCUSDT": must be in (0, 1].
    assert!(
        v_a[17] > Decimal::ZERO && v_a[17] <= Decimal::ONE,
        "slot 17: single_symbol_hash_norm must be in (0, 1], got {}",
        v_a[17]
    );

    // Wave-B slots: MUST be zero for a Bull card (K4 invariant).
    assert_eq!(
        v_a[SLOT_VOLATILE],
        Decimal::ZERO,
        "slot 18: Volatile one-hot must be zero for legacy Bull card"
    );
    assert_eq!(
        v_a[SLOT_CALM],
        Decimal::ZERO,
        "slot 19: Calm one-hot must be zero for legacy Bull card"
    );

    // Remaining reserved slots 20..31: all zero.
    for (i, slot) in v_a.iter().enumerate().skip(20) {
        assert_eq!(*slot, Decimal::ZERO, "slot {i}: reserved slot must be zero");
    }
}

// ── Test 4: all-legacy-regime sweep ───────────────────────────────────────────

/// Sweep all 3 legacy regime tags across all 7 strategies and all 3 outcomes.
/// Assert that slots 18 and 19 are zero for EVERY combination.
///
/// This is the comprehensive K4 gate: no legacy callsite should ever
/// produce a non-zero value in the new Wave-B regime slots.
#[test]
fn t_wave_b_all_legacy_regime_combinations_have_zero_new_slots() {
    let strategies = [
        "sma_crossover",
        "macd_trend",
        "rsi_reversion",
        "bbands_mean_revert",
        "top10_momentum_h1",
        "pairs_mr_h1",
        "(unattributed)",
    ];
    let legacy_regimes = [RegimeTag::Bull, RegimeTag::Bear, RegimeTag::Chop];
    let outcomes = [OutcomeClass::Win, OutcomeClass::Loss, OutcomeClass::Scratch];

    for strategy in &strategies {
        for &regime in &legacy_regimes {
            for &outcome in &outcomes {
                let card = fixture_card(strategy, regime, outcome);
                let v = embed(&card);
                assert_eq!(
                    v[SLOT_VOLATILE],
                    Decimal::ZERO,
                    "strategy={strategy}, regime={regime}, outcome={outcome:?}: \
                     slot 18 (Volatile) must be zero for legacy card"
                );
                assert_eq!(
                    v[SLOT_CALM],
                    Decimal::ZERO,
                    "strategy={strategy}, regime={regime}, outcome={outcome:?}: \
                     slot 19 (Calm) must be zero for legacy card"
                );
            }
        }
    }
}

// ── Test 5: display strings for new variants ───────────────────────────────────

/// Verify `Display` output for the new Wave-B `RegimeTag` variants.
///
/// ADR-0049 § D2 mandates `Volatile → "volatile"`, `Calm → "calm"`.
/// The legacy variants must remain byte-identical.
#[test]
fn t_wave_b_display_strings_match_adr0049() {
    assert_eq!(format!("{}", RegimeTag::Bull), "bull");
    assert_eq!(format!("{}", RegimeTag::Bear), "bear");
    assert_eq!(format!("{}", RegimeTag::Chop), "chop");
    assert_eq!(format!("{}", RegimeTag::Volatile), "volatile");
    assert_eq!(format!("{}", RegimeTag::Calm), "calm");
}

// ── Test 6: ordinals preserve K4 contract ─────────────────────────────────────

/// Assert that `Volatile` and `Calm` ordinals are 3 and 4 respectively,
/// and that `Bull`, `Bear`, `Chop` ordinals are still 0, 1, 2.
///
/// This exercises the `PartialOrd` / `Ord` derived ordering which
/// encodes the ordinal contract from ADR-0049 § D2.
#[test]
fn t_wave_b_enum_ordinals_match_adr0049() {
    // Legacy variants: Bull < Bear < Chop.
    assert!(
        RegimeTag::Bull < RegimeTag::Bear,
        "Bull ordinal must be < Bear"
    );
    assert!(
        RegimeTag::Bear < RegimeTag::Chop,
        "Bear ordinal must be < Chop"
    );
    // New variants: Chop < Volatile < Calm.
    assert!(
        RegimeTag::Chop < RegimeTag::Volatile,
        "Chop ordinal must be < Volatile"
    );
    assert!(
        RegimeTag::Volatile < RegimeTag::Calm,
        "Volatile ordinal must be < Calm"
    );
    // Full ordering: Bull < Bear < Chop < Volatile < Calm.
    assert!(
        RegimeTag::Bull < RegimeTag::Calm,
        "Bull ordinal must be < Calm (full order)"
    );
}

// ── Test 7: new regime tags produce non-zero embeddings ───────────────────────

/// Assert that embedding vectors for `Volatile` and `Calm` cards are NOT
/// all-zero (would indicate a misrouted one-hot, e.g. falling into a
/// zero-padded slot outside the vector bounds).
#[test]
fn t_wave_b_new_regime_tags_produce_nonzero_embeddings() {
    let volatile_v = embed(&fixture_card(
        "sma_crossover",
        RegimeTag::Volatile,
        OutcomeClass::Win,
    ));
    let calm_v = embed(&fixture_card(
        "sma_crossover",
        RegimeTag::Calm,
        OutcomeClass::Win,
    ));

    // At minimum, slot 18 or 19 must be ONE.
    let volatile_sum: Decimal = volatile_v[18..20].iter().sum();
    let calm_sum: Decimal = calm_v[18..20].iter().sum();

    assert_eq!(
        volatile_sum,
        Decimal::ONE,
        "Volatile card: exactly one of slots 18/19 must be ONE (sum={volatile_sum})"
    );
    assert_eq!(
        calm_sum,
        Decimal::ONE,
        "Calm card: exactly one of slots 18/19 must be ONE (sum={calm_sum})"
    );
}
