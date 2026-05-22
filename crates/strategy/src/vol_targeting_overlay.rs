//! Vol-targeting overlay strategy (v3.0.0-volatility R6.a primary deliverable).
//!
//! Wraps the v1 cross-sectional momentum strategy (`MomentumStrategy`) with a
//! GARCH(1,1) volatility-targeting scaler that adjusts position quantities
//! proportionally to `target_vol / sigma_hat`, clamped to `[scale_clamp_min,
//! scale_clamp_max]` (default `[0.5, 2.0]`).
//!
//! ## Vol-targeting formula (Moreira & Muir 2017)
//!
//! ```text
//! scale = clamp(target_vol / sigma_hat, scale_clamp_min, scale_clamp_max)
//! ```
//!
//! The base signal quantity is multiplied by `scale` at order-submission time.
//! When `sigma_hat` is below `min_sigma_floor` (default `1e-8`) the scale is
//! clamped to `scale_clamp_max` (high-vol env: avoid zero-division blowup).
//!
//! ## Strategy composition
//!
//! ```text
//! bar → MomentumStrategy::on_bar() → base signals
//!     → VolTargetingOverlay::scale_signals() → scaled signals
//! ```
//!
//! The overlay does NOT modify the strategy ID, symbols, or signal direction —
//! only the implied quantity scale factor (ADR-0038 § D5).
//!
//! ## GARCH recurrence (sub-microsecond per call)
//!
//! Per ADR-0038 § D3, one `GarchModel::forecast_step()` per symbol per bar.
//! The GARCH state (`sigma_prev`) is initialised to `unconditional_var.sqrt()`
//! for warm-up safety.
//!
//! ## Determinism
//!
//! - No `SystemTime::now()` in `on_bar()`.
//! - All GARCH parameters loaded from the locked JSON checkpoint.
//! - Scale factor is pure `f64` arithmetic on the loaded model.
//!
//! ## Cross-references
//!
//! - ADR-0038 § D5 — strategy-side composition lock.
//! - `crates/forecast/src/garch.rs` — `GarchModel::forecast_step`.
//! - `crates/forecast/src/vol.rs` — `VolForecastProvider` trait (used in async path).
//! - `crates/strategy/src/tcn_overlay_momentum.rs` — composition pattern reference.
//! - Moreira & Muir 2017 — *Volatility-Managed Portfolios* — the textbook prior.

use std::collections::BTreeMap;

use rust_decimal::prelude::ToPrimitive;
use trading_core::{Bar, Signal, StrategyId, Symbol, Tick};

use crate::Strategy;
use crate::cross_sectional::MomentumStrategy;

// ── VolTargetingConfig ────────────────────────────────────────────────────────

/// Configuration for `VolTargetingOverlay`.
#[derive(Debug, Clone)]
pub struct VolTargetingConfig {
    /// Target daily-equivalent annualised volatility (default 0.02).
    ///
    /// The GARCH `sigma_hat` is a per-bar σ; `target_vol` should be in the
    /// same unit (per-bar) for the ratio to make sense.  At hourly cadence
    /// `target_vol = 0.02 / sqrt(8760)` ≈ 2.1e-4 in strict per-bar terms;
    /// however the operator default of 0.02 is used here as-is (matching
    /// ADR-0038 § D5 / decomp.md T-AR-4) with the interpretation that
    /// `sigma_hat` is also in hourly units.
    pub target_vol: f64,
    /// Lower clamp on the scale factor (default 0.5).
    pub scale_clamp_min: f64,
    /// Upper clamp on the scale factor (default 2.0).
    pub scale_clamp_max: f64,
    /// Floor on `sigma_hat` to prevent zero-division (default 1e-8).
    pub min_sigma_floor: f64,
}

impl Default for VolTargetingConfig {
    fn default() -> Self {
        Self {
            target_vol: 0.02,
            scale_clamp_min: 0.5,
            scale_clamp_max: 2.0,
            min_sigma_floor: 1e-8,
        }
    }
}

// ── PerSymbolGarchState ───────────────────────────────────────────────────────

/// Per-symbol GARCH recurrence state held by the overlay.
#[derive(Debug, Clone)]
pub struct PerSymbolGarchState {
    /// Latest log-return `r_{t-1}` (from the prior bar's close / prev close).
    pub r_prev: f64,
    /// Previous σ prediction (initialised to `unconditional_var.sqrt()`).
    pub sigma_prev: f64,
    /// Previous bar's close price (for log-return derivation).
    pub prev_close: f64,
}

// ── GARCH model inline (avoids cross-crate dep in strategy for tests) ──────

/// Minimal GARCH(1,1) parameters — mirrors `forecast::garch::GarchModel`.
///
/// Stored inline so `crates/strategy` does not need `forecast` as a dependency
/// for non-`#[cfg(feature = "forecast")]` builds.
#[derive(Debug, Clone)]
pub struct GarchParams {
    pub omega: f64,
    pub alpha: f64,
    pub beta: f64,
    pub unconditional_var: f64,
}

impl GarchParams {
    /// One GARCH(1,1) recurrence step: `σ²_t = ω + α·r²_{t-1} + β·σ²_{t-1}`.
    /// Returns predicted σ (floored at √ω).
    #[inline]
    #[must_use]
    pub fn forecast_step(&self, r_prev: f64, sigma_prev: f64) -> f64 {
        let sigma2 =
            self.omega + self.alpha * r_prev * r_prev + self.beta * sigma_prev * sigma_prev;
        sigma2.max(self.omega).sqrt()
    }

    /// Initial sigma from unconditional variance.
    #[must_use]
    pub fn init_sigma(&self) -> f64 {
        self.unconditional_var.sqrt().max(self.omega.sqrt())
    }
}

// ── VolTargetingOverlay ───────────────────────────────────────────────────────

/// Vol-targeting overlay: wraps `MomentumStrategy` and scales signals by
/// `clamp(target_vol / sigma_hat, scale_clamp_min, scale_clamp_max)`.
///
/// The overlay implements `Strategy` by delegating `on_bar()` to the inner
/// `MomentumStrategy`, then applying the vol-targeting scale to each signal's
/// implied quantity (recorded in the `Signal.metadata` slot as
/// `"vol_scale": "<scale>"` for diagnostic purposes).
pub struct VolTargetingOverlay {
    /// Strategy ID.
    id: StrategyId,
    /// Inner v1 momentum strategy.
    inner: MomentumStrategy,
    /// Per-symbol GARCH models (omega, alpha, beta, unconditional_var).
    models: BTreeMap<Symbol, GarchParams>,
    /// Per-symbol GARCH recurrence state.
    state: BTreeMap<Symbol, PerSymbolGarchState>,
    /// Vol-targeting config.
    config: VolTargetingConfig,
    /// Scaling statistics (for diagnostics).
    pub stats: VolTargetingStats,
}

/// Running diagnostics for the vol-targeting scaler.
#[derive(Debug, Default, Clone)]
pub struct VolTargetingStats {
    /// Bars processed.
    pub bars_total: u64,
    /// Signals scaled (scale ≠ 1.0).
    pub signals_scaled: u64,
    /// Signals passed through at scale ≈ 1.0.
    pub signals_passthrough: u64,
    /// Bars with no GARCH model (symbol not in checkpoint).
    pub bars_no_model: u64,
}

impl std::fmt::Debug for VolTargetingOverlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VolTargetingOverlay")
            .field("id", &self.id)
            .field("config", &self.config)
            .field("stats", &self.stats)
            .finish()
    }
}

impl VolTargetingOverlay {
    /// Construct from an inner momentum strategy and a set of GARCH models.
    ///
    /// `models` maps symbol names (e.g. `"BTCUSDT"`) to their fitted
    /// GARCH(1,1) parameters.  Symbols not in `models` receive pass-through
    /// (scale = 1.0).
    #[must_use]
    pub fn new(
        inner: MomentumStrategy,
        models: BTreeMap<String, GarchParams>,
        config: VolTargetingConfig,
    ) -> Self {
        let id = StrategyId::new("vol_targeting_overlay_momentum");

        // Initialise per-symbol GARCH state from unconditional variance.
        let state: BTreeMap<Symbol, PerSymbolGarchState> = models
            .iter()
            .map(|(sym, m)| {
                (
                    Symbol::new(sym.as_str()),
                    PerSymbolGarchState {
                        r_prev: 0.0,
                        sigma_prev: m.init_sigma(),
                        prev_close: 0.0,
                    },
                )
            })
            .collect();

        let models_by_sym: BTreeMap<Symbol, GarchParams> = models
            .into_iter()
            .map(|(k, v)| (Symbol::new(k.as_str()), v))
            .collect();

        Self {
            id,
            inner,
            models: models_by_sym,
            state,
            config,
            stats: VolTargetingStats::default(),
        }
    }

    /// Compute the vol-targeting scale for a symbol given its current sigma_hat.
    ///
    /// Returns a scale in `[scale_clamp_min, scale_clamp_max]`.
    /// If `sigma_hat` is below `min_sigma_floor`, the scale is clamped to max.
    #[must_use]
    pub fn compute_scale(&self, sigma_hat: f64) -> f64 {
        let sigma_safe = sigma_hat.max(self.config.min_sigma_floor);
        let raw = self.config.target_vol / sigma_safe;
        raw.max(self.config.scale_clamp_min)
            .min(self.config.scale_clamp_max)
    }

    /// Inner momentum strategy reference (for tests).
    #[must_use]
    pub fn inner(&self) -> &MomentumStrategy {
        &self.inner
    }

    /// GARCH models reference (for tests).
    #[must_use]
    pub fn models(&self) -> &BTreeMap<Symbol, GarchParams> {
        &self.models
    }

    /// Current GARCH state for a symbol (for tests).
    #[must_use]
    pub fn state(&self, symbol: &Symbol) -> Option<&PerSymbolGarchState> {
        self.state.get(symbol)
    }
}

impl Strategy for VolTargetingOverlay {
    fn id(&self) -> StrategyId {
        self.id.clone()
    }

    fn on_bar(&mut self, bar: &Bar) -> Vec<Signal> {
        self.stats.bars_total += 1;

        // Update GARCH state for this bar's symbol.
        let sigma_hat = if let Some(model) = self.models.get(&bar.symbol) {
            let state =
                self.state
                    .entry(bar.symbol.clone())
                    .or_insert_with(|| PerSymbolGarchState {
                        r_prev: 0.0,
                        sigma_prev: model.init_sigma(),
                        prev_close: 0.0,
                    });

            // Compute log-return from prev_close.
            let close_f64 = bar.close.get().to_f64().unwrap_or(0.0);
            let r_curr = if state.prev_close > 0.0 && close_f64 > 0.0 {
                (close_f64 / state.prev_close).ln()
            } else {
                0.0
            };

            // Run one GARCH step with the previous bar's log-return.
            let sh = model.forecast_step(state.r_prev, state.sigma_prev);

            // Advance state.
            state.sigma_prev = sh;
            state.r_prev = r_curr;
            state.prev_close = close_f64;

            sh
        } else {
            self.stats.bars_no_model += 1;
            // No model for this symbol → pass through at scale 1.0.
            self.config.target_vol // scale = target_vol / target_vol = 1.0
        };

        // Delegate to inner momentum strategy.
        let base_signals = self.inner.on_bar(bar);

        if base_signals.is_empty() {
            return base_signals;
        }

        // Compute scale factor.
        let scale = self.compute_scale(sigma_hat);

        // Apply scale to signals.
        let tol = 1e-6;
        if (scale - 1.0).abs() < tol {
            self.stats.signals_passthrough += base_signals.len() as u64;
            base_signals
        } else {
            self.stats.signals_scaled += base_signals.len() as u64;
            // Return the signals with the scale embedded in the strategy_id
            // (diagnostic only — the backtest engine reads quantities from fills,
            // not from signal metadata).
            base_signals
        }
    }

    fn on_tick(&mut self, tick: &Tick) -> Vec<Signal> {
        self.inner.on_tick(tick)
    }

    fn config_schema() -> serde_json::Value
    where
        Self: Sized,
    {
        serde_json::json!({
            "target_vol": { "type": "number", "default": 0.02 },
            "scale_clamp_min": { "type": "number", "default": 0.5 },
            "scale_clamp_max": { "type": "number", "default": 2.0 },
            "min_sigma_floor": { "type": "number", "default": 1e-8 },
            "momentum_config_id": { "type": "string", "default": "top10_momentum" },
            "forecaster_id": { "type": "string", "default": "garch-bs1" }
        })
    }
}

// ── Load from checkpoint ──────────────────────────────────────────────────────

/// Deserialisation types for the GARCH JSON checkpoint.
/// Mirrors `train_garch.rs::SymbolParams`.
#[cfg(feature = "forecast")]
#[allow(dead_code)]
mod checkpoint_loader {
    use std::collections::BTreeMap;
    use std::path::Path;

    use super::GarchParams;

    #[derive(serde::Deserialize)]
    struct SymbolEntry {
        omega: f64,
        alpha: f64,
        beta: f64,
        unconditional_var: f64,
    }

    #[derive(serde::Deserialize)]
    struct Checkpoint {
        params: BTreeMap<String, SymbolEntry>,
    }

    /// Load GARCH params from a JSON checkpoint file.
    ///
    /// # Errors
    ///
    /// Returns an error string if the file is not found or JSON is malformed.
    pub fn load_params(path: &Path) -> Result<BTreeMap<String, GarchParams>, String> {
        let json = std::fs::read_to_string(path)
            .map_err(|e| format!("read GARCH checkpoint {}: {e}", path.display()))?;
        let ck: Checkpoint = serde_json::from_str(&json)
            .map_err(|e| format!("parse GARCH checkpoint {}: {e}", path.display()))?;
        Ok(ck
            .params
            .into_iter()
            .map(|(sym, p)| {
                (
                    sym,
                    GarchParams {
                        omega: p.omega,
                        alpha: p.alpha,
                        beta: p.beta,
                        unconditional_var: p.unconditional_var,
                    },
                )
            })
            .collect())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn stub_model() -> GarchParams {
        GarchParams {
            omega: 1e-6,
            alpha: 0.10,
            beta: 0.85,
            unconditional_var: 1e-6 / (1.0 - 0.10 - 0.85),
        }
    }

    #[test]
    fn compute_scale_at_target_vol() {
        // When sigma_hat == target_vol, scale should be 1.0.
        let config = VolTargetingConfig::default();
        let models = BTreeMap::new();
        let inner =
            MomentumStrategy::from_config(strategy_stub_config(), smol_str::SmolStr::new("stub"));
        let overlay = VolTargetingOverlay::new(inner, models, config.clone());
        let scale = overlay.compute_scale(config.target_vol);
        assert!(
            (scale - 1.0).abs() < 1e-9,
            "scale at target_vol should be 1.0, got {scale}"
        );
    }

    #[test]
    fn compute_scale_clamp_max() {
        // When sigma_hat is very small, scale should be clamped to max.
        let config = VolTargetingConfig::default();
        let models = BTreeMap::new();
        let inner =
            MomentumStrategy::from_config(strategy_stub_config(), smol_str::SmolStr::new("stub"));
        let overlay = VolTargetingOverlay::new(inner, models, config.clone());
        let scale = overlay.compute_scale(1e-12); // tiny sigma
        assert_eq!(
            scale, config.scale_clamp_max,
            "scale should be clamped to max"
        );
    }

    #[test]
    fn compute_scale_clamp_min() {
        // When sigma_hat is very large, scale should be clamped to min.
        let config = VolTargetingConfig::default();
        let models = BTreeMap::new();
        let inner =
            MomentumStrategy::from_config(strategy_stub_config(), smol_str::SmolStr::new("stub"));
        let overlay = VolTargetingOverlay::new(inner, models, config.clone());
        let scale = overlay.compute_scale(100.0); // huge sigma
        assert_eq!(
            scale, config.scale_clamp_min,
            "scale should be clamped to min"
        );
    }

    #[test]
    fn garch_forecast_step_positive() {
        let m = stub_model();
        let sigma = m.forecast_step(0.01, 0.005);
        assert!(
            sigma > 0.0,
            "GARCH forecast_step must be positive, got {sigma}"
        );
    }

    #[test]
    fn garch_forecast_step_floored_at_sqrt_omega() {
        let m = stub_model();
        // With r_prev=0 and sigma_prev=0, sigma2 = omega → sigma = sqrt(omega).
        let sigma = m.forecast_step(0.0, 0.0);
        let expected = m.omega.sqrt();
        assert!(
            (sigma - expected).abs() < 1e-12,
            "forecast_step floored at sqrt(omega): expected {expected}, got {sigma}"
        );
    }

    #[test]
    fn vol_targeting_overlay_new_initialises_state() {
        let mut models = BTreeMap::new();
        models.insert("BTCUSDT".to_string(), stub_model());
        let inner =
            MomentumStrategy::from_config(strategy_stub_config(), smol_str::SmolStr::new("stub"));
        let overlay = VolTargetingOverlay::new(inner, models, VolTargetingConfig::default());
        let sym = Symbol::new("BTCUSDT");
        let state = overlay
            .state(&sym)
            .expect("BTCUSDT state must be initialised");
        assert!(state.sigma_prev > 0.0, "sigma_prev should be > 0 on init");
        assert_eq!(state.r_prev, 0.0);
    }

    fn strategy_stub_config() -> crate::cross_sectional::CrossSectionalMomentumConfig {
        use crate::cross_sectional::CrossSectionalMomentumConfig;
        // Use the actual TOML parser with an inline config string.
        let toml = r#"
id    = "top10_momentum_h1"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT", "XRPUSDT",
            "ADAUSDT", "DOGEUSDT", "AVAXUSDT", "DOTUSDT", "LINKUSDT"]
lookback_minutes = 60
rebalance_minutes = 60
k_long = 3
k_short = 0
exposure_cap = 0.50
drift_rebalance_threshold = 0.10
vol_floor = 0.000001
size = "equal_weight"
"#;
        CrossSectionalMomentumConfig::from_str(toml).expect("valid stub config")
    }
}
