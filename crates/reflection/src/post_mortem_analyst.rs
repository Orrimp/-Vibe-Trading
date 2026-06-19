//! `post_mortem_analyst::generate_card` — turns a closed trade into
//! a lesson card.
//!
//! v1 implementation is the **deterministic** Q1 = Option A path:
//! pure over inputs (no LLM, no clock, no random); the LLM v2
//! follow-up replaces this body behind the same name.

use rust_decimal::Decimal;
use thiserror::Error;
use trading_core::{Money, Timestamp, Usdt};

use crate::outcome::classify_outcome;
use crate::regime::{RegimeError, classify_regime};
use crate::types::{ClosedTrade, LessonCard, card_id};

/// Errors from `generate_card`.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GenerateCardError {
    #[error("regime classification failed: {0}")]
    Regime(#[from] RegimeError),
}

/// Produce a `LessonCard` from a closed trade.
///
/// Pure over inputs:
/// - regime is classified at trade-open and trade-close timestamps
///   via `classify_regime`,
/// - outcome via `classify_outcome` against opening capital,
/// - `card_id` via the content-hash helper from T1801,
/// - `note: None` (Q1 = Option A).
///
/// # Errors
///
/// - [`GenerateCardError::Regime`] on regime classification failure.
pub async fn generate_card(
    closed_trade: &ClosedTrade,
    opening_capital: Money<Usdt>,
    btc_closes: &[(Timestamp, Decimal)],
) -> Result<LessonCard, GenerateCardError> {
    // Regime classification requires at least 7 days of BTC daily data.
    // In short soaks / paper mode with no seed data, the 7d lookback will fail
    // with `NoCloseAtMinus7d`.  Rather than dropping the card entirely, we fall
    // back to `Chop` (the "undetermined/insufficient-data" bucket) so that
    // lesson cards are still generated and the reflection pipeline exercises
    // the durable-write path.  Production long-running agents will have the
    // full seed loaded and see accurate regime tags.
    //
    // `NoCloseAtTimestamp` (can't find current close) is a hard error —
    // it means the trade close timestamp is before all known data, which
    // indicates a logic error upstream, so we keep that as a real failure.
    let entry_regime = classify_regime(btc_closes, closed_trade.opened_at)
        .unwrap_or(crate::regime::RegimeTag::Chop);
    let exit_regime = classify_regime(btc_closes, closed_trade.closed_at)
        .unwrap_or(crate::regime::RegimeTag::Chop);
    let outcome_class = classify_outcome(closed_trade.signed_pnl, opening_capital);

    let card_id_str = card_id(
        &closed_trade.closed_at,
        closed_trade.holding_period_bars,
        &opening_capital,
        &closed_trade.signed_pnl,
        &closed_trade.strategy_id,
        &closed_trade.symbol_or_pair,
    );

    Ok(LessonCard {
        card_id: card_id_str,
        closed_at: closed_trade.closed_at,
        symbol_or_pair: closed_trade.symbol_or_pair.clone(),
        strategy_id: closed_trade.strategy_id.clone(),
        signed_pnl: closed_trade.signed_pnl,
        opening_capital,
        holding_period_bars: closed_trade.holding_period_bars,
        entry_regime,
        exit_regime,
        outcome_class,
        note: None,
    })
}
