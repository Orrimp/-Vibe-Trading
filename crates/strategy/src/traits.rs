//! The `Strategy` trait — contract per architecture.md.
use trading_core::{Bar, Signal, StrategyId, Tick};

/// A compiled-in trading strategy.
///
/// The trait shape is fixed per architecture.md v0 — it does NOT change for
/// v0.5 config-driven composition or v1+ WASM plugins.
pub trait Strategy: Send + Sync {
    fn id(&self) -> StrategyId;
    fn on_bar(&mut self, bar: &Bar) -> Vec<Signal>;
    fn on_tick(&mut self, tick: &Tick) -> Vec<Signal>;
    fn config_schema() -> serde_json::Value
    where
        Self: Sized;
}
