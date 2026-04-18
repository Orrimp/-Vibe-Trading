//! Strategy registry — full implementation (T22).
//!
//! `StrategyRegistry` holds compiled-in strategies, routes bar/tick events,
//! and journals every mutation to the audit ledger.
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use trading_core::{Bar, LedgerError, Signal, StrategyId, Tick};

use crate::Strategy;

/// One strategy entry in the TOML config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyTomlEntry {
    pub kind: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_fast")]
    pub fast_len: usize,
    #[serde(default = "default_slow")]
    pub slow_len: usize,
}

fn default_true() -> bool {
    true
}
fn default_fast() -> usize {
    20
}
fn default_slow() -> usize {
    50
}

/// Registry event kind (journaled to audit ledger).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistryEventKind {
    Load,
    Swap,
    Unload,
}

impl std::fmt::Display for RegistryEventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Load => write!(f, "load"),
            Self::Swap => write!(f, "swap"),
            Self::Unload => write!(f, "unload"),
        }
    }
}

/// A pending registry journal event — caller decides when to flush to ledger.
#[derive(Debug, Clone)]
pub struct PendingJournalEvent {
    pub kind: RegistryEventKind,
    pub strategy_id: StrategyId,
    pub metadata: String,
}

/// Holds compiled-in strategies, routes bar/tick events.
///
/// Every `load`, `swap`, `unload` operation appends a [`PendingJournalEvent`]
/// that callers should flush to the audit ledger via
/// `audit::journal::registry_event`.
pub struct StrategyRegistry {
    inner: HashMap<StrategyId, Box<dyn Strategy>>,
    pending_events: Vec<PendingJournalEvent>,
}

impl StrategyRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
            pending_events: Vec::new(),
        }
    }

    /// Load a set of strategies from a TOML-derived map.
    ///
    /// Only strategies with `enabled = true` and `kind = "sma_crossover"` are
    /// instantiated in v0.  Every load is journaled.
    pub fn load_from_toml(&mut self, entries: HashMap<String, StrategyTomlEntry>) {
        for (name, entry) in entries {
            if !entry.enabled {
                continue;
            }
            let strategy: Box<dyn Strategy> = match entry.kind.as_str() {
                "sma_crossover" => {
                    Box::new(crate::SmaCrossover::new(entry.fast_len, entry.slow_len))
                }
                other => {
                    tracing::warn!(kind = other, name = %name, "unknown strategy kind — skipping");
                    continue;
                }
            };
            let id = strategy.id();
            self.inner.insert(id.clone(), strategy);
            self.pending_events.push(PendingJournalEvent {
                kind: RegistryEventKind::Load,
                strategy_id: id,
                metadata: serde_json::json!({
                    "name": name,
                    "kind": entry.kind,
                    "fast_len": entry.fast_len,
                    "slow_len": entry.slow_len,
                })
                .to_string(),
            });
        }
    }

    /// Register a strategy.  Adds a `Load` journal event.
    pub fn register(&mut self, strategy: Box<dyn Strategy>) {
        let id = strategy.id();
        self.inner.insert(id.clone(), strategy);
        self.pending_events.push(PendingJournalEvent {
            kind: RegistryEventKind::Load,
            strategy_id: id,
            metadata: "{}".to_string(),
        });
    }

    /// Swap a strategy by id.  The old strategy is replaced atomically.
    /// Adds a `Swap` journal event.
    ///
    /// # Errors
    ///
    /// Returns `Err(id)` if the strategy id does not exist in the registry.
    pub fn swap(
        &mut self,
        id: StrategyId,
        new_strategy: Box<dyn Strategy>,
    ) -> Result<(), StrategyId> {
        if !self.inner.contains_key(&id) {
            return Err(id);
        }
        self.inner.insert(id.clone(), new_strategy);
        self.pending_events.push(PendingJournalEvent {
            kind: RegistryEventKind::Swap,
            strategy_id: id,
            metadata: "{}".to_string(),
        });
        Ok(())
    }

    /// Remove a strategy by id.  Adds an `Unload` journal event.
    ///
    /// Returns `true` if the strategy was present.
    pub fn unload(&mut self, id: &StrategyId) -> bool {
        let removed = self.inner.remove(id).is_some();
        if removed {
            self.pending_events.push(PendingJournalEvent {
                kind: RegistryEventKind::Unload,
                strategy_id: id.clone(),
                metadata: "{}".to_string(),
            });
        }
        removed
    }

    /// Fan-out a bar to all active strategies.
    pub fn on_bar(&mut self, bar: &Bar) -> Vec<Signal> {
        let mut signals = Vec::new();
        for s in self.inner.values_mut() {
            signals.extend(s.on_bar(bar));
        }
        signals
    }

    /// Fan-out a tick to all active strategies.
    pub fn on_tick(&mut self, tick: &Tick) -> Vec<Signal> {
        let mut signals = Vec::new();
        for s in self.inner.values_mut() {
            signals.extend(s.on_tick(tick));
        }
        signals
    }

    /// Drain pending journal events.
    ///
    /// The caller should flush these to `audit::journal::registry_event`.
    /// After drain the internal list is empty.
    pub fn drain_pending_events(&mut self) -> Vec<PendingJournalEvent> {
        std::mem::take(&mut self.pending_events)
    }

    /// Number of active strategies.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// True if no strategies are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl Default for StrategyRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Flush all pending journal events in the registry to the audit ledger.
///
/// This is an async helper so the `strategy` crate doesn't need to depend
/// on `audit` at compile time — callers bring their own `Ledger`.
///
/// # Errors
///
/// Returns [`LedgerError`] if any individual event write fails.
pub async fn flush_pending_to_ledger(
    registry: &mut StrategyRegistry,
    ledger: &audit::Ledger,
) -> Result<(), LedgerError> {
    let events = registry.drain_pending_events();
    for ev in events {
        audit::journal::registry_event(
            ledger,
            &ev.kind.to_string(),
            &ev.strategy_id.to_string(),
            &ev.metadata,
        )
        .await?;
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use rust_decimal_macros::dec;
    use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp};

    fn make_bar(close: rust_decimal::Decimal, ts_offset: i64) -> Bar {
        let base = time::OffsetDateTime::UNIX_EPOCH;
        let ts = Timestamp::new(base + time::Duration::minutes(ts_offset));
        Bar {
            symbol: Symbol::new("BTCUSDT"),
            tf: Timeframe::OneMinute,
            open: Price::new(close).unwrap(),
            high: Price::new(close).unwrap(),
            low: Price::new(close).unwrap(),
            close: Price::new(close).unwrap(),
            volume: Quantity::new(rust_decimal::Decimal::ONE).unwrap(),
            trade_count: 0,
            local_recv_ts: ts,
            open_ts: ts,
            close_ts: ts,
        }
    }

    #[test]
    fn t22_register_and_on_bar() {
        let mut reg = StrategyRegistry::new();
        reg.register(Box::new(crate::SmaCrossover::new(2, 3)));
        assert_eq!(reg.len(), 1);

        // Feed 3 bars to warm up slow SMA
        use rust_decimal_macros::dec;
        reg.on_bar(&make_bar(dec!(10), 1));
        reg.on_bar(&make_bar(dec!(20), 2));
        let signals = reg.on_bar(&make_bar(dec!(30), 3));
        // At bar 3: fast(2)=25, slow(3)=20 → Buy
        assert!(!signals.is_empty(), "expected a signal at bar 3");
    }

    #[test]
    fn t22_deterministic_200_bars() {
        use rust_decimal_macros::dec;

        fn run_200() -> Vec<trading_core::SignalKind> {
            let mut reg = StrategyRegistry::new();
            reg.register(Box::new(crate::SmaCrossover::new(5, 20)));
            let mut kinds = Vec::new();
            for i in 0..200i64 {
                // Deterministic price: 30000 + (i % 50) * 100
                let price = dec!(30000) + rust_decimal::Decimal::from(i % 50) * dec!(100);
                for sig in reg.on_bar(&make_bar(price, i)) {
                    kinds.push(sig.kind);
                }
            }
            kinds
        }

        let run1 = run_200();
        let run2 = run_200();
        assert_eq!(run1, run2, "signal sequence must be deterministic");
        assert!(
            !run1.is_empty(),
            "expected at least one signal over 200 bars"
        );
    }

    #[test]
    fn t22_swap_replaces_strategy() {
        let mut reg = StrategyRegistry::new();
        let id = trading_core::StrategyId::new("sma_crossover");
        reg.register(Box::new(crate::SmaCrossover::new(5, 20)));

        let result = reg.swap(id.clone(), Box::new(crate::SmaCrossover::new(10, 30)));
        assert!(result.is_ok());

        let events = reg.drain_pending_events();
        assert_eq!(events.len(), 2); // Load + Swap
        assert_eq!(events[0].kind, RegistryEventKind::Load);
        assert_eq!(events[1].kind, RegistryEventKind::Swap);
    }

    #[test]
    fn t22_unload_removes_strategy() {
        let mut reg = StrategyRegistry::new();
        reg.register(Box::new(crate::SmaCrossover::new(5, 20)));
        assert_eq!(reg.len(), 1);

        let id = trading_core::StrategyId::new("sma_crossover");
        assert!(reg.unload(&id));
        assert_eq!(reg.len(), 0);

        let events = reg.drain_pending_events();
        assert_eq!(
            events.last().map(|e| e.kind.clone()),
            Some(RegistryEventKind::Unload)
        );
    }
}
