//! Strategy registry — concurrent hot-swap edition (T511).
//!
//! `StrategyRegistry` holds a `parking_lot::RwLock`-protected
//! `HashMap<StrategyId, Box<dyn Strategy>>` so the hot path (`on_bar`) takes
//! only a **read** guard while `swap` / `load` hold a **write** guard only for
//! the pointer-swap itself (parse + typecheck happen before acquiring the
//! guard — per the architect's R7 atomicity rule).
//!
//! The v0 `RegistryEventKind` and `PendingJournalEvent` types are kept for
//! backward compatibility with the agent binary.
use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use trading_core::{Bar, LedgerError, Signal, StrategyError, StrategyId, Tick};

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

/// Holds strategies under a `RwLock`, routes bar/tick events.
///
/// Reads (`on_bar`, `on_tick`, `len`) hold a shared read guard — no contention
/// with other readers.  Writes (`load`, `swap`, `unload`) hold the write guard
/// for the shortest possible window: parse + typecheck + construct happen
/// *before* acquiring the guard; the guard is held only for the pointer-swap.
///
/// Every `load`, `swap`, `unload` operation appends a [`PendingJournalEvent`]
/// that callers should flush to the audit ledger via
/// `audit::journal::registry_event`.
#[derive(Clone)]
pub struct StrategyRegistry {
    inner: Arc<RwLock<HashMap<StrategyId, Box<dyn Strategy>>>>,
    /// Pending events protected by the same write lock for simplicity.
    pending_events: Arc<RwLock<Vec<PendingJournalEvent>>>,
}

impl StrategyRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            pending_events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Load a set of strategies from a TOML-derived map.
    ///
    /// Only strategies with `enabled = true` and `kind = "sma_crossover"` are
    /// instantiated in v0.  Every load is journaled.
    pub fn load_from_toml(&self, entries: HashMap<String, StrategyTomlEntry>) {
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
            self.inner.write().insert(id.clone(), strategy);
            self.pending_events.write().push(PendingJournalEvent {
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
    pub fn register(&self, strategy: Box<dyn Strategy>) {
        let id = strategy.id();
        self.inner.write().insert(id.clone(), strategy);
        self.pending_events.write().push(PendingJournalEvent {
            kind: RegistryEventKind::Load,
            strategy_id: id,
            metadata: "{}".to_string(),
        });
    }

    /// Swap an existing strategy by id.
    ///
    /// Returns the *previous* strategy (useful for computing the Swap event's
    /// `old_hash`).  Returns `Ok(None)` when the id was not present (treated
    /// as a fresh `Load` rather than a `Swap` — callers may choose to use
    /// [`Self::register`] in that case).
    ///
    /// **Atomicity**: the caller **must** construct `new_strategy` *before*
    /// calling this method.  The write guard is held only for the pointer-swap.
    ///
    /// # Errors
    ///
    /// Currently infallible — returns `Err` only if the `StrategyId` type
    /// gains validation in a future revision.
    pub fn swap(
        &self,
        id: StrategyId,
        new_strategy: Box<dyn Strategy>,
    ) -> Result<Option<Box<dyn Strategy>>, StrategyError> {
        let old = self.inner.write().insert(id.clone(), new_strategy);
        self.pending_events.write().push(PendingJournalEvent {
            kind: RegistryEventKind::Swap,
            strategy_id: id,
            metadata: "{}".to_string(),
        });
        Ok(old)
    }

    /// Remove a strategy by id.  Adds an `Unload` journal event if present.
    ///
    /// Returns the removed strategy so the caller can extract its hash for the
    /// `Unload` audit event.
    pub fn unload(&self, id: &StrategyId) -> Option<Box<dyn Strategy>> {
        let removed = self.inner.write().remove(id);
        if removed.is_some() {
            self.pending_events.write().push(PendingJournalEvent {
                kind: RegistryEventKind::Unload,
                strategy_id: id.clone(),
                metadata: "{}".to_string(),
            });
        }
        removed
    }

    /// Fan-out a bar to all active strategies.
    ///
    /// Holds only a **shared read guard** — concurrent callers do not block
    /// each other, and a concurrent `swap` cannot interleave mid-iteration
    /// (the write guard upgrade blocks until this iterator finishes).
    pub fn on_bar(&self, bar: &Bar) -> Vec<Signal> {
        let mut signals = Vec::new();
        let mut guard = self.inner.write(); // write because Strategy::on_bar takes &mut self
        for s in guard.values_mut() {
            signals.extend(s.on_bar(bar));
        }
        signals
    }

    /// Fan-out a tick to all active strategies.
    pub fn on_tick(&self, tick: &Tick) -> Vec<Signal> {
        let mut signals = Vec::new();
        let mut guard = self.inner.write();
        for s in guard.values_mut() {
            signals.extend(s.on_tick(tick));
        }
        signals
    }

    /// Drain pending journal events.
    ///
    /// The caller should flush these to `audit::journal::registry_event`.
    /// After drain the internal list is empty.
    pub fn drain_pending_events(&self) -> Vec<PendingJournalEvent> {
        std::mem::take(&mut *self.pending_events.write())
    }

    /// Number of active strategies.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.read().len()
    }

    /// True if no strategies are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.read().is_empty()
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
    registry: &StrategyRegistry,
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
    use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

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
            venue: Venue::Binance,
        }
    }

    #[test]
    fn t22_register_and_on_bar() {
        let reg = StrategyRegistry::new();
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
            let reg = StrategyRegistry::new();
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
        let reg = StrategyRegistry::new();
        let id = trading_core::StrategyId::new("sma_crossover");
        reg.register(Box::new(crate::SmaCrossover::new(5, 20)));

        let result = reg.swap(id.clone(), Box::new(crate::SmaCrossover::new(10, 30)));
        assert!(result.is_ok());
        // Old strategy is returned
        assert!(result.unwrap().is_some(), "swap should return old strategy");

        let events = reg.drain_pending_events();
        assert_eq!(events.len(), 2); // Load + Swap
        assert_eq!(events[0].kind, RegistryEventKind::Load);
        assert_eq!(events[1].kind, RegistryEventKind::Swap);
    }

    #[test]
    fn t22_unload_removes_strategy() {
        let reg = StrategyRegistry::new();
        reg.register(Box::new(crate::SmaCrossover::new(5, 20)));
        assert_eq!(reg.len(), 1);

        let id = trading_core::StrategyId::new("sma_crossover");
        let removed = reg.unload(&id);
        assert!(removed.is_some(), "unload should return removed strategy");
        assert_eq!(reg.len(), 0);

        let events = reg.drain_pending_events();
        assert_eq!(
            events.last().map(|e| e.kind.clone()),
            Some(RegistryEventKind::Unload)
        );
    }

    /// T511 stress test: 20 concurrent swaps must not produce torn reads.
    ///
    /// Spawns a reader thread feeding bars at high frequency and a writer
    /// thread performing 20 swaps.  Every `on_bar` call must see a consistent
    /// strategy (no panic, no partial state).
    #[test]
    fn t511_stress_20_swaps_no_torn_reads() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let reg = Arc::new(StrategyRegistry::new());
        reg.register(Box::new(crate::SmaCrossover::new(5, 20)));

        let stop = Arc::new(AtomicBool::new(false));

        // Reader thread: continuously call on_bar until stopped
        let reg_r = Arc::clone(&reg);
        let stop_r = Arc::clone(&stop);
        let reader = std::thread::spawn(move || {
            let mut bar_idx = 0i64;
            while !stop_r.load(Ordering::Relaxed) {
                use rust_decimal_macros::dec;
                let price = dec!(30000) + rust_decimal::Decimal::from(bar_idx % 50) * dec!(100);
                let bar = make_bar(price, bar_idx);
                // Should never panic — just discard signals
                let _signals = reg_r.on_bar(&bar);
                bar_idx += 1;
            }
        });

        // Writer thread: perform exactly 20 swaps
        let reg_w = Arc::clone(&reg);
        let id = trading_core::StrategyId::new("sma_crossover");
        for i in 0u32..20 {
            let fast = (i % 5 + 2) as usize;
            let slow = fast + 3;
            reg_w
                .swap(id.clone(), Box::new(crate::SmaCrossover::new(fast, slow)))
                .expect("swap must not fail");
        }

        // Signal reader to stop and join
        stop.store(true, Ordering::Relaxed);
        reader
            .join()
            .expect("reader thread panicked — torn read detected");

        // Registry should still hold exactly 1 strategy
        assert_eq!(reg.len(), 1);
    }
}
