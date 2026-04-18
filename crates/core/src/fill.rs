//! Fill type — a completed execution.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::asset::Usdt;
use crate::money::{Money, Price, Quantity};
use crate::order::OrderId;
use crate::symbol::{Side, Symbol};
use crate::time::Timestamp;

/// Unique fill identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FillId(pub Uuid);

impl FillId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for FillId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for FillId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Whether a fill was maker or taker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeeTier {
    Taker,
    Maker,
}

/// Liquidity role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Liquidity {
    Taker,
    Maker,
}

/// A completed trade execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fill {
    pub id: FillId,
    pub order_id: OrderId,
    pub symbol: Symbol,
    pub side: Side,
    pub qty: Quantity,
    /// Post-slippage fill price.
    pub price: Price,
    /// Taker fee in USDT (v0: taker only).
    pub fee: Money<Usdt>,
    pub fee_tier: FeeTier,
    pub venue_ts: Timestamp,
    pub local_ts: Timestamp,
    pub liquidity: Liquidity,
}
