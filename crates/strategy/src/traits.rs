//! The `Strategy` trait — contract per architecture.md.
use trading_core::{Bar, Signal, StrategyId, Symbol, Tick};

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

    /// Per-symbol quantity scale factor applied at sizing time.
    ///
    /// Default returns `1.0` (no scaling). Overlays that adjust position
    /// sizes (e.g., vol-targeting) override this to expose their cached
    /// per-symbol scale factor. Queried by the sizing pipeline at order-
    /// construction time. The scale is cached from the most recent
    /// `on_bar` call; calling `quantity_scale` before any `on_bar` for
    /// this symbol returns the default `1.0`.
    fn quantity_scale(&self, _symbol: &Symbol) -> f64 {
        1.0
    }
}
