//! Core types — `LessonCard`, `ClosedTrade`, `RetrievalQuery`,
//! `LessonCardWriteRequest`, `card_id` content-hash.
//!
//! The `card_id` is a sha256 over a deterministic field set (sorted
//! by name) so the same closed-trade fixture produces the same
//! `card_id` across runs — load-bearing for R2.4 idempotency.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use trading_core::{Money, PairKey, StrategyId, Symbol, Timestamp, Usdt};

use crate::outcome::OutcomeClass;
use crate::regime::RegimeTag;

/// One side's identity — single symbol or a pair (a-b ordered).
///
/// `Display` emits `BTCUSDT` for a single symbol; `(BTCUSDT, ETHUSDT)`
/// for a pair (matching `PairKey::Display`). Used in the rendered
/// card-line body; load-bearing for R4.2.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SymbolOrPair {
    Single(Symbol),
    Pair(PairKey),
}

impl std::fmt::Display for SymbolOrPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Single(s) => write!(f, "{s}"),
            Self::Pair(p) => write!(f, "{p}"),
        }
    }
}

/// Closed-trade view consumed by `post_mortem_analyst::generate_card`.
///
/// This is a **derived** view over the audit ledger — it's not a
/// chart-of-accounts row.  The fields source from
/// `journal_transactions` + `realized_pnl_for_trade`; the
/// `closed_at` is the close-side `journal_transactions.ts`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosedTrade {
    /// Close-side transaction id (the sell-side fill that brought
    /// the per-symbol position to zero).
    pub close_transaction_id: String,
    /// Open-side transaction id (the most-recent prior buy-side
    /// transaction for the same symbol).  Used for `opening_capital`
    /// snapshot.
    pub open_transaction_id: String,
    /// Symbol or pair; pair-MR records the `a` leg's symbol-or-pair
    /// view.
    pub symbol_or_pair: SymbolOrPair,
    /// Strategy id (or `(unattributed)` if the close-side row had no
    /// strategy tag).
    pub strategy_id: StrategyId,
    /// Net signed P&L over fees.  Sourced from
    /// `audit::query::realized_pnl_for_trade`.
    pub signed_pnl: Money<Usdt>,
    /// Ledger close timestamp (RFC3339 microsecond precision via the
    /// journal-format helper).  Body byte source for the rendered
    /// card line.
    pub closed_at: Timestamp,
    /// Ledger open timestamp.  Used for `entry_regime` classification
    /// and `holding_period_bars` compute.
    pub opened_at: Timestamp,
    /// Number of 1-minute bars between `opened_at` and `closed_at`.
    /// Computed by the caller (paper-engine / fixture builder).
    pub holding_period_bars: u32,
}

/// One lesson card.  Persisted in `lesson_cards` table; rendered as
/// one bullet line in `## Memory highlights`.
///
/// All fields are deterministic over the closed-trade input.  The
/// `note` field is reserved for v2 LLM enrichment (Q1 = Option A
/// keeps it `None`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LessonCard {
    /// Content hash of the deterministic fields (sorted by name).
    /// Load-bearing for R2.4 idempotency.
    pub card_id: String,
    /// Trade close timestamp (sourced from the audit ledger).
    pub closed_at: Timestamp,
    /// Symbol or pair traded.
    pub symbol_or_pair: SymbolOrPair,
    /// Strategy id (or `(unattributed)` for tag-less trades).
    pub strategy_id: StrategyId,
    /// Signed P&L over fees.
    pub signed_pnl: Money<Usdt>,
    /// Cash balance at the open-side timestamp.
    pub opening_capital: Money<Usdt>,
    /// Holding period in 1m bars.
    pub holding_period_bars: u32,
    /// Regime classification at the open timestamp.
    pub entry_regime: RegimeTag,
    /// Regime classification at the close timestamp.
    pub exit_regime: RegimeTag,
    /// Win / Loss / Scratch per Q3c thresholds.
    pub outcome_class: OutcomeClass,
    /// Reserved for the LLM-enrichment v2.  v1 always emits `None`.
    pub note: Option<String>,
}

/// Retrieval query — filters cards by strategy / symbol / regime
/// before the cosine ranking.  Built at report time per Q3f's
/// largest-abs-PnL rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetrievalQuery {
    pub strategy_id: StrategyId,
    pub symbol_or_pair: SymbolOrPair,
    pub current_regime: RegimeTag,
}

/// Write-request enqueued by the executor's fill-handler tap on a
/// trade-close fill.  Consumed by `ReflectionWriterTask::run` which
/// turns it into a `LessonCard` via `post_mortem_analyst::generate_card`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LessonCardWriteRequest {
    pub closed_trade: ClosedTrade,
    pub opening_capital: Money<Usdt>,
    pub btc_closes: Vec<(Timestamp, Decimal)>,
}

/// Compute a sha256 content hash over the deterministic fields of a
/// closed trade.  Same fixture in twice → same `card_id`; different
/// `signed_pnl` → different `card_id`.
///
/// Fields are concatenated in **lex-sorted name order** to keep the
/// hash stable as the field set evolves.  Format:
/// `field_name=value;field_name=value;...`.
#[must_use]
pub fn card_id(
    closed_at: &Timestamp,
    holding_period_bars: u32,
    opening_capital: &Money<Usdt>,
    signed_pnl: &Money<Usdt>,
    strategy_id: &StrategyId,
    symbol_or_pair: &SymbolOrPair,
) -> String {
    use time::format_description::well_known::Rfc3339;
    let closed_at_str = closed_at
        .inner()
        .format(&Rfc3339)
        .unwrap_or_else(|_| String::from("invalid"));
    // Sorted by field name ASC. Using a Vec<(name, value)> makes the
    // sort explicit + grep-able.
    let mut fields: Vec<(&'static str, String)> = vec![
        ("closed_at", closed_at_str),
        ("holding_period_bars", holding_period_bars.to_string()),
        (
            "opening_capital",
            opening_capital.amount().normalize().to_string(),
        ),
        ("signed_pnl", signed_pnl.amount().normalize().to_string()),
        ("strategy_id", strategy_id.0.to_string()),
        ("symbol_or_pair", symbol_or_pair.to_string()),
    ];
    fields.sort_by_key(|(k, _)| *k);
    let mut hasher = Sha256::new();
    for (name, value) in fields {
        hasher.update(name.as_bytes());
        hasher.update(b"=");
        hasher.update(value.as_bytes());
        hasher.update(b";");
    }
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;
    use trading_core::{Symbol, Usdt};

    fn ts(unix_secs: i64) -> Timestamp {
        Timestamp::new(OffsetDateTime::from_unix_timestamp(unix_secs).expect("ts"))
    }

    #[test]
    fn card_id_is_deterministic() {
        let a = card_id(
            &ts(1_700_000_000),
            42,
            &Money::<Usdt>::from_decimal(dec!(10000)),
            &Money::<Usdt>::from_decimal(dec!(123.45)),
            &StrategyId::new("sma_crossover"),
            &SymbolOrPair::Single(Symbol::new("BTCUSDT")),
        );
        let b = card_id(
            &ts(1_700_000_000),
            42,
            &Money::<Usdt>::from_decimal(dec!(10000)),
            &Money::<Usdt>::from_decimal(dec!(123.45)),
            &StrategyId::new("sma_crossover"),
            &SymbolOrPair::Single(Symbol::new("BTCUSDT")),
        );
        assert_eq!(a, b);
        assert_eq!(a.len(), 64); // sha256 hex
    }

    #[test]
    fn card_id_changes_with_signed_pnl() {
        let a = card_id(
            &ts(1_700_000_000),
            42,
            &Money::<Usdt>::from_decimal(dec!(10000)),
            &Money::<Usdt>::from_decimal(dec!(123.45)),
            &StrategyId::new("sma_crossover"),
            &SymbolOrPair::Single(Symbol::new("BTCUSDT")),
        );
        let b = card_id(
            &ts(1_700_000_000),
            42,
            &Money::<Usdt>::from_decimal(dec!(10000)),
            &Money::<Usdt>::from_decimal(dec!(123.46)),
            &StrategyId::new("sma_crossover"),
            &SymbolOrPair::Single(Symbol::new("BTCUSDT")),
        );
        assert_ne!(a, b);
    }
}
