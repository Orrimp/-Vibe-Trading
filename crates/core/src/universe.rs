//! Universe types for v1 cross-sectional momentum (T601).
//!
//! `SymbolSet` is BTreeSet-backed so iteration is always alphabetical —
//! this property is load-bearing for R12.2 / R12.4 determinism.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use crate::asset::Asset;
use crate::symbol::Symbol;

/// Error constructing a universe.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UniverseError {
    #[error("universe is empty")]
    Empty,
    #[error("universe contains unknown symbol: {0}")]
    UnknownSymbol(Symbol),
    #[error("universe must contain at least 2 symbols for cross-sectional ranking, got {0}")]
    TooSmall(usize),
}

/// Sorted set of `Symbol`s — alphabetical iteration order.
///
/// `BTreeSet`-backed so iteration is always deterministic (alphabetical),
/// matching the Q5 strategy-side filter and R12.2 `(venue_ts, symbol ASC)` merge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SymbolSet(BTreeSet<Symbol>);

impl SymbolSet {
    /// Create a `SymbolSet`, returning `Err(UniverseError::Empty)` if empty.
    ///
    /// # Errors
    ///
    /// Returns [`UniverseError::Empty`] if the input iterator is empty.
    pub fn new(symbols: impl IntoIterator<Item = Symbol>) -> Result<Self, UniverseError> {
        let set: BTreeSet<_> = symbols.into_iter().collect();
        if set.is_empty() {
            return Err(UniverseError::Empty);
        }
        Ok(Self(set))
    }

    /// Returns `true` if the symbol is in this set.
    #[must_use]
    pub fn contains(&self, s: &Symbol) -> bool {
        self.0.contains(s)
    }

    /// Iterate over symbols in alphabetical order.
    pub fn iter(&self) -> impl Iterator<Item = &Symbol> {
        self.0.iter()
    }

    /// Number of symbols.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the set is empty (only possible if constructed via `Default`).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Display for SymbolSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let symbols: Vec<&str> = self.0.iter().map(|s| s.0.as_str()).collect();
        write!(f, "[{}]", symbols.join(", "))
    }
}

/// The v1 strategy universe: a frozen symbol set + base-asset mapping.
///
/// Captured at strategy load time from `MarketDataSource::exchange_info`.
/// Stable for the life of the strategy instance (R1.1 — membership is frozen).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Universe {
    /// Sorted set of universe symbols (`BTreeSet` for deterministic iteration).
    pub symbols: SymbolSet,
    /// Symbol → base-asset mapping captured at load time (R8.3 — side table).
    /// `BTreeMap` for deterministic serialization.
    pub base_asset: BTreeMap<Symbol, Asset>,
}

impl Universe {
    /// Create a `Universe` from symbols and a base-asset mapping.
    ///
    /// # Errors
    ///
    /// Returns [`UniverseError::Empty`] if `symbols` is empty.
    pub fn new(
        symbols: impl IntoIterator<Item = Symbol>,
        base_asset: BTreeMap<Symbol, Asset>,
    ) -> Result<Self, UniverseError> {
        let symbol_set = SymbolSet::new(symbols)?;
        Ok(Self {
            symbols: symbol_set,
            base_asset,
        })
    }

    /// Create a universe from a list of `"<BASE>USDT"` symbol strings.
    ///
    /// Base-asset mapping is derived by stripping the `"USDT"` suffix
    /// (sufficient for the v1 default universe which is all USDT-quoted).
    ///
    /// # Errors
    ///
    /// Returns [`UniverseError::Empty`] if the list is empty.
    pub fn from_usdt_symbols(symbols: &[&str]) -> Result<Self, UniverseError> {
        let syms: Vec<Symbol> = symbols.iter().map(|s| Symbol::new(*s)).collect();
        let base_asset: BTreeMap<Symbol, Asset> = syms
            .iter()
            .map(|sym| {
                // Strip "USDT" suffix; fall back to "Other(<full symbol>)".
                let base = sym
                    .0
                    .as_str()
                    .strip_suffix("USDT")
                    .unwrap_or(sym.0.as_str());
                let asset = match base {
                    "BTC" => Asset::Btc,
                    "ETH" => Asset::Eth,
                    other => Asset::Other(SmolStr::new(other)),
                };
                (sym.clone(), asset)
            })
            .collect();
        Self::new(syms, base_asset)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    fn sym(s: &str) -> Symbol {
        Symbol::new(s)
    }

    #[test]
    fn t601_symbol_set_empty_error() {
        let result = SymbolSet::new(std::iter::empty::<Symbol>());
        assert!(
            matches!(result, Err(UniverseError::Empty)),
            "empty set must error"
        );
    }

    #[test]
    fn t601_symbol_set_alphabetical_iter() {
        let set = SymbolSet::new(vec![sym("XRPUSDT"), sym("BTCUSDT"), sym("ETHUSDT")]).unwrap();
        let symbols: Vec<&Symbol> = set.iter().collect();
        assert_eq!(symbols[0].0.as_str(), "BTCUSDT");
        assert_eq!(symbols[1].0.as_str(), "ETHUSDT");
        assert_eq!(symbols[2].0.as_str(), "XRPUSDT");
    }

    #[test]
    fn t601_symbol_set_round_trip() {
        let set = SymbolSet::new(vec![sym("BTCUSDT"), sym("ETHUSDT")]).unwrap();
        let json = serde_json::to_string(&set).expect("serialize");
        let back: SymbolSet = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(set, back);
    }

    #[test]
    fn t601_universe_from_usdt_symbols() {
        let universe = Universe::from_usdt_symbols(&["BTCUSDT", "ETHUSDT", "BNBUSDT"]).unwrap();
        assert_eq!(universe.symbols.len(), 3);
        assert!(universe.symbols.contains(&sym("BTCUSDT")));
        let btc_asset = universe.base_asset.get(&sym("BTCUSDT")).unwrap();
        assert_eq!(btc_asset, &Asset::Btc);
        let eth_asset = universe.base_asset.get(&sym("ETHUSDT")).unwrap();
        assert_eq!(eth_asset, &Asset::Eth);
    }

    #[test]
    fn t601_universe_round_trip() {
        let universe = Universe::from_usdt_symbols(&["BTCUSDT", "ETHUSDT"]).unwrap();
        let json = serde_json::to_string(&universe).expect("serialize");
        let back: Universe = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(universe, back);
    }
}
