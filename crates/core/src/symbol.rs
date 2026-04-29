//! Identifier newtypes: `Symbol`, `StrategyId`, `AccountId`, `Side`.
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

/// Exchange-native symbol, e.g. `"BTCUSDT"`.
/// Uses slash-free format as required by spec (not `BTC/USDT`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Symbol(pub SmolStr);

impl Symbol {
    /// Create a symbol from any string.
    pub fn new(s: impl Into<SmolStr>) -> Self {
        Self(s.into())
    }
}

impl std::fmt::Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a strategy, e.g. `"sma_crossover"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StrategyId(pub SmolStr);

impl StrategyId {
    pub fn new(s: impl Into<SmolStr>) -> Self {
        Self(s.into())
    }
}

impl std::fmt::Display for StrategyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Ledger account identifier, e.g. `"assets:cash:USDT"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AccountId(pub SmolStr);

impl AccountId {
    pub fn new(s: impl Into<SmolStr>) -> Self {
        Self(s.into())
    }
}

impl std::fmt::Display for AccountId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Trade side: the aggressor side or the intended direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Buy,
    Sell,
}

impl std::fmt::Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Side::Buy => write!(f, "buy"),
            Side::Sell => write!(f, "sell"),
        }
    }
}
