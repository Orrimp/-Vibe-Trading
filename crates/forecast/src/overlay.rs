//! Overlay composition helpers for consuming strategies.
//!
//! This module lifts the boilerplate five steps (call provider, combine signal,
//! emit cost event, emit audit row) into helpers so the strategy's `tick()`
//! body stays short.
//!
//! ## Composition rule (architecture/12 § Overlay composition pattern)
//!
//! ```text
//! 1. Read the base strategy's Signal.
//! 2. Call ForecastProvider::forecast() with the OHLCV window.
//! 3. Combine: agree+confident → boost; disagree+confident → dampen; flat → pass-through.
//! 4. Emit one CostEvent::Infra { line: "kronos_inference", … }.
//! 5. Emit one audit row carrying ForecastOverlay + correlation_id.
//! ```
//!
//! Steps 4 and 5 are M5/M6 work. This module provides the combine logic
//! (step 3) as a pure function so it is trivially testable.

use trading_core::forecast::{Direction, ForecastOverlay};
use trading_core::signal::SignalKind;

/// Combine a base signal direction with a forecast overlay to produce a
/// modulated `SignalKind`.
///
/// ## Rules (architecture/12 § Combine step)
///
/// | Overlay direction | Confidence ≥ threshold | Base signal | Result          |
/// |---|---|---|---|
/// | Same as base      | yes                     | Buy/Sell    | `Buy`/`Sell` (pass-through; boost is a future sizing concern not a kind change) |
/// | Opposite to base  | yes                     | Buy/Sell    | `Hold` (dampen) |
/// | `Flat`            | any                     | any         | base (pass-through) |
/// | any               | < threshold             | any         | base (pass-through) |
///
/// At v2.5 the "boost" path does NOT change `SignalKind` from `Buy` to
/// `StrongBuy` (no such variant); it is a pass-through. The distinction is
/// reserved for a v2.5.x sizing-weight extension.
///
/// # Arguments
///
/// - `base`: the consuming strategy's raw signal kind.
/// - `overlay`: the `ForecastOverlay` from the provider.
/// - `confidence_threshold`: typically `KronosConfig::overlay_confidence_threshold`
///   (default `0.6`).
#[must_use]
pub fn combine(
    base: SignalKind,
    overlay: &ForecastOverlay,
    confidence_threshold: rust_decimal::Decimal,
) -> SignalKind {
    // If confidence is below threshold, pass the base signal through.
    if overlay.confidence < confidence_threshold {
        return base;
    }

    // Flat direction → pass-through.
    if overlay.direction == Direction::Flat {
        return base;
    }

    // Determine if the overlay agrees or disagrees with the base signal.
    let base_is_bullish = matches!(base, SignalKind::Buy);
    let base_is_bearish = matches!(base, SignalKind::Sell);
    let overlay_bullish = overlay.direction == Direction::Up;
    let overlay_bearish = overlay.direction == Direction::Down;

    // Agreement: pass-through (boost reserved for future sizing extension).
    if (base_is_bullish && overlay_bullish) || (base_is_bearish && overlay_bearish) {
        return base;
    }

    // Disagreement with sufficient confidence: dampen to Hold.
    if (base_is_bullish && overlay_bearish) || (base_is_bearish && overlay_bullish) {
        return SignalKind::Hold;
    }

    // All other cases (Hold base, pair signals, etc.) → pass-through.
    base
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;
    use uuid::Uuid;
    use trading_core::forecast::ForecastOverlay;
    use trading_core::signal::SignalKind;

    fn overlay(dir: Direction, confidence: Decimal) -> ForecastOverlay {
        ForecastOverlay {
            correlation_id: Uuid::nil(),
            confidence,
            direction: dir,
            horizon_bars: 1,
            model_revision: "test".into(),
            sampled_at: OffsetDateTime::UNIX_EPOCH,
        }
    }

    const THRESHOLD: Decimal = dec!(0.6);

    /// Case 1: agree + confident → pass-through (Buy stays Buy).
    #[test]
    fn combine_agree_confident_buy_passthrough() {
        let result = combine(
            SignalKind::Buy,
            &overlay(Direction::Up, dec!(0.8)),
            THRESHOLD,
        );
        assert_eq!(result, SignalKind::Buy);
    }

    /// Case 1b: agree + confident → pass-through (Sell stays Sell).
    #[test]
    fn combine_agree_confident_sell_passthrough() {
        let result = combine(
            SignalKind::Sell,
            &overlay(Direction::Down, dec!(0.9)),
            THRESHOLD,
        );
        assert_eq!(result, SignalKind::Sell);
    }

    /// Case 2: disagree + confident → dampen to Hold.
    #[test]
    fn combine_disagree_confident_dampens_to_hold() {
        let result = combine(
            SignalKind::Buy,
            &overlay(Direction::Down, dec!(0.85)),
            THRESHOLD,
        );
        assert_eq!(result, SignalKind::Hold);
    }

    /// Case 2b: disagree + confident → dampen (Sell dampened by Up overlay).
    #[test]
    fn combine_disagree_sell_up_dampens_to_hold() {
        let result = combine(
            SignalKind::Sell,
            &overlay(Direction::Up, dec!(0.75)),
            THRESHOLD,
        );
        assert_eq!(result, SignalKind::Hold);
    }

    /// Case 3: flat overlay → pass-through regardless of confidence.
    #[test]
    fn combine_flat_overlay_passthrough() {
        let result = combine(
            SignalKind::Buy,
            &overlay(Direction::Flat, dec!(0.99)),
            THRESHOLD,
        );
        assert_eq!(result, SignalKind::Buy);
    }

    /// Case 4: low confidence → pass-through.
    #[test]
    fn combine_low_confidence_passthrough() {
        // Confidence 0.5 is below default threshold 0.6.
        let result = combine(
            SignalKind::Buy,
            &overlay(Direction::Down, dec!(0.5)),
            THRESHOLD,
        );
        assert_eq!(result, SignalKind::Buy);
    }

    /// Hold base + confident forecast → pass-through (Hold stays Hold).
    #[test]
    fn combine_hold_base_passthrough() {
        let result = combine(
            SignalKind::Hold,
            &overlay(Direction::Down, dec!(0.95)),
            THRESHOLD,
        );
        assert_eq!(result, SignalKind::Hold);
    }

    /// Confidence exactly at threshold is accepted (≥ not >).
    #[test]
    fn combine_confidence_at_threshold_is_accepted() {
        let result = combine(
            SignalKind::Buy,
            &overlay(Direction::Down, dec!(0.6)),
            THRESHOLD,
        );
        // At threshold, disagree → Hold.
        assert_eq!(result, SignalKind::Hold);
    }

    /// Confidence just below threshold is rejected (pass-through).
    #[test]
    fn combine_confidence_just_below_threshold_passthrough() {
        let result = combine(
            SignalKind::Buy,
            &overlay(Direction::Down, dec!(0.59)),
            THRESHOLD,
        );
        // Below threshold → pass-through Buy.
        assert_eq!(result, SignalKind::Buy);
    }
}
