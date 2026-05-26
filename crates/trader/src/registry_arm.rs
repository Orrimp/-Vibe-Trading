//! Registry-arm free function — moved from `crates/strategy/src/registry.rs`
//! per ADR-0041 § D4.
//!
//! The `"llm_forecaster_v3"` TOML-loader arm previously lived inside
//! `strategy::registry::load_from_toml`. Post-ADR-0041, that arm has been
//! removed from strategy and this free function is the new registration path.
//!
//! ## Usage
//!
//! Application binaries call this function before calling
//! `registry.load_from_toml(...)`:
//!
//! ```rust,ignore
//! for entry in &toml_entries {
//!     if entry.kind == "llm_forecaster_v3" {
//!         trader::register_llm_forecaster_v3(&registry, entry);
//!     }
//! }
//! registry.load_from_toml(other_entries);
//! ```

use std::sync::Arc;

use reflection::NullReflectionStore;
use strategy::registry::{StrategyRegistry, StrategyTomlEntry};

use crate::llm_forecaster::{LlmForecasterConfig, LlmForecasterStrategy, StubForecaster};

/// Register an `"llm_forecaster_v3"` TOML entry into the strategy registry.
///
/// This is the ADR-0041 § D4 replacement for the removed
/// `strategy::registry::load_from_toml` `"llm_forecaster_v3"` arm.
///
/// Uses `StubForecaster` + `NullReflectionStore` for the TOML-loader path.
/// The real `LlmForecasterImpl` wiring is done by the application binary
/// via `StrategyRegistry::register` when operating in production mode.
///
/// Returns `true` if the strategy was registered, `false` if `enabled = false`.
pub fn register_llm_forecaster_v3(registry: &StrategyRegistry, entry: &StrategyTomlEntry) -> bool {
    if !entry.enabled {
        tracing::info!(
            kind = "llm_forecaster_v3",
            "llm_forecaster_v3 loaded but enabled=false (R9.3 default-disabled)"
        );
        return false;
    }
    let cfg = LlmForecasterConfig {
        enabled: true,
        ..LlmForecasterConfig::default()
    };
    let strategy = LlmForecasterStrategy::new(
        cfg,
        Arc::new(StubForecaster::default()),
        Arc::new(NullReflectionStore),
        Vec::new(), // btc_closes: empty → regime = Chop fallback
        None,       // rt: None → pollster::block_on in test path
    );
    registry.register(Box::new(strategy));
    true
}
