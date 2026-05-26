//! Vol kill-switch overlay strategy (v3.0.0-volatility R6.b secondary).
//!
//! Wraps the v1 cross-sectional momentum strategy (`MomentumStrategy`) with a
//! GARCH(1,1) kill-switch that flattens exposure on a symbol when its predicted
//! σ exceeds `threshold_multiplier × historical_median(σ̂)`.  The symbol's
//! signals are held as `Hold` for `cooldown_bars` bars after the trigger.
//!
//! ## Kill-switch logic
//!
//! ```text
//! if sigma_hat > threshold_multiplier * rolling_median_sigma:
//!     emit Hold for ALL symbols in the basket (Q4=(p3) broadened filter)
//!     start cooldown_bars countdown
//! else:
//!     pass base signal through unchanged
//! ```
//!
//! ## Cross-sectional basket semantic (Q4=(p3), 2026-05-26)
//!
//! When the kill-switch fires for ANY symbol in the basket, ALL signals in the
//! rebalance basket are converted to `Hold` — not just the triggering symbol's.
//! This is the "belt+suspenders" cross-sectional-basket semantic: a vol spike on
//! BTCUSDT halts the whole basket (BTCUSDT + ETHUSDT + …).
//! Caveat (K2): this overlay is designed for `MomentumStrategy` (cross-sectional);
//! wrapping a single-symbol inner strategy with this overlay would over-suppress
//! signals on unrelated symbols.  See `spec/vol-killswitch-overlay-noop-fix/feature.md`.
//!
//! ## Determinism
//!
//! - No `SystemTime::now()` in `on_bar()`.
//! - Rolling median over a fixed-capacity buffer (deterministic insertion order).
//! - No RNG.
//!
//! ## Cross-references
//!
//! - ADR-0038 § D5 — strategy-side composition lock; risk-engine deferred.
//! - `crates/strategy/src/vol_targeting_overlay.rs` — sibling strategy (R6.a).
//! - `crates/strategy/src/cross_sectional/momentum.rs` — inner v1 strategy.

use std::collections::BTreeMap;

use trading_core::{Bar, Signal, SignalKind, StrategyId, Symbol, Tick};

use crate::Strategy;
use crate::cross_sectional::MomentumStrategy;
use crate::vol_targeting_overlay::GarchParams;

// ── VolKillSwitchConfig ───────────────────────────────────────────────────────

/// Configuration for `VolKillSwitchOverlay`.
#[derive(Debug, Clone)]
pub struct VolKillSwitchConfig {
    /// Multiplier on the rolling median σ̂ — kill-switch fires when
    /// `sigma_hat > threshold_multiplier × rolling_median_sigma`.
    /// Default: 3.0.
    pub threshold_multiplier: f64,
    /// Number of bars to hold flat after a kill-switch trigger.
    /// Default: 4 hours = 4 bars (hourly cadence).
    pub cooldown_bars: u32,
    /// Capacity of the rolling sigma buffer for median computation.
    /// Default: 720 bars (30 days at hourly cadence).
    pub rolling_window: usize,
    /// Floor on rolling_median_sigma to prevent zero-threshold (default 1e-8).
    pub min_median_floor: f64,
}

impl Default for VolKillSwitchConfig {
    fn default() -> Self {
        Self {
            threshold_multiplier: 3.0,
            cooldown_bars: 4,
            rolling_window: 720,
            min_median_floor: 1e-8,
        }
    }
}

// ── Per-symbol kill-switch state ──────────────────────────────────────────────

#[derive(Debug, Clone)]
struct KillSwitchState {
    /// GARCH recurrence state.
    r_prev: f64,
    sigma_prev: f64,
    prev_close: f64,
    /// Rolling buffer of recent sigma_hat values for median computation.
    sigma_buffer: Vec<f64>,
    /// Cooldown countdown (0 = not in cooldown).
    cooldown_remaining: u32,
}

// ── VolKillSwitchOverlay ──────────────────────────────────────────────────────

/// Vol kill-switch overlay: wraps `MomentumStrategy` and holds signals flat
/// when sigma_hat spikes above `threshold_multiplier × rolling_median(σ̂)`.
pub struct VolKillSwitchOverlay {
    id: StrategyId,
    inner: MomentumStrategy,
    models: BTreeMap<Symbol, GarchParams>,
    state: BTreeMap<Symbol, KillSwitchState>,
    config: VolKillSwitchConfig,
    /// Kill-switch trigger count (diagnostics).
    pub kill_switch_count: u64,
    /// Total bars processed.
    pub bars_total: u64,
}

impl std::fmt::Debug for VolKillSwitchOverlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VolKillSwitchOverlay")
            .field("id", &self.id)
            .field("config", &self.config)
            .field("kill_switch_count", &self.kill_switch_count)
            .finish()
    }
}

impl VolKillSwitchOverlay {
    /// Construct from an inner momentum strategy and a set of GARCH models.
    #[must_use]
    pub fn new(
        inner: MomentumStrategy,
        models: BTreeMap<String, GarchParams>,
        config: VolKillSwitchConfig,
    ) -> Self {
        let state: BTreeMap<Symbol, KillSwitchState> = models
            .iter()
            .map(|(sym, m)| {
                (
                    Symbol::new(sym.as_str()),
                    KillSwitchState {
                        r_prev: 0.0,
                        sigma_prev: m.init_sigma(),
                        prev_close: 0.0,
                        sigma_buffer: Vec::with_capacity(config.rolling_window),
                        cooldown_remaining: 0,
                    },
                )
            })
            .collect();

        let models_by_sym: BTreeMap<Symbol, GarchParams> = models
            .into_iter()
            .map(|(k, v)| (Symbol::new(k.as_str()), v))
            .collect();

        Self {
            id: StrategyId::new("vol_killswitch_overlay_momentum"),
            inner,
            models: models_by_sym,
            state,
            config,
            kill_switch_count: 0,
            bars_total: 0,
        }
    }

    /// Compute the rolling median of the sigma buffer (sorted copy).
    fn rolling_median(buf: &[f64]) -> f64 {
        if buf.is_empty() {
            return 0.0;
        }
        let mut sorted = buf.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mid = sorted.len() / 2;
        if sorted.len().is_multiple_of(2) {
            (sorted[mid - 1] + sorted[mid]) / 2.0
        } else {
            sorted[mid]
        }
    }
}

impl Strategy for VolKillSwitchOverlay {
    fn id(&self) -> StrategyId {
        self.id.clone()
    }

    fn on_bar(&mut self, bar: &Bar) -> Vec<Signal> {
        self.bars_total += 1;

        // Determine if the kill-switch is active for this symbol.
        let kill_active = if let Some(model) = self.models.get(&bar.symbol) {
            let state = self
                .state
                .entry(bar.symbol.clone())
                .or_insert_with(|| KillSwitchState {
                    r_prev: 0.0,
                    sigma_prev: model.init_sigma(),
                    prev_close: 0.0,
                    sigma_buffer: Vec::with_capacity(self.config.rolling_window),
                    cooldown_remaining: 0,
                });

            // Compute log-return.
            use rust_decimal::prelude::ToPrimitive;
            let close_f64 = bar.close.get().to_f64().unwrap_or(0.0);
            let r_curr = if state.prev_close > 0.0 && close_f64 > 0.0 {
                (close_f64 / state.prev_close).ln()
            } else {
                0.0
            };

            // GARCH step.
            let sh = model.forecast_step(state.r_prev, state.sigma_prev);

            // Update rolling buffer.
            if state.sigma_buffer.len() >= self.config.rolling_window {
                state.sigma_buffer.remove(0); // sliding window
            }
            state.sigma_buffer.push(sh);

            // Advance state.
            state.sigma_prev = sh;
            state.r_prev = r_curr;
            state.prev_close = close_f64;

            // Check kill-switch condition.
            let median_sigma =
                Self::rolling_median(&state.sigma_buffer).max(self.config.min_median_floor);
            let threshold = self.config.threshold_multiplier * median_sigma;

            if state.cooldown_remaining > 0 {
                // In cooldown: decrement and keep kill active.
                state.cooldown_remaining -= 1;
                true
            } else if sh > threshold {
                // Trigger: start cooldown.
                state.cooldown_remaining = self.config.cooldown_bars;
                self.kill_switch_count += 1;
                true
            } else {
                false
            }
        } else {
            false // No model → pass through.
        };

        let base_signals = self.inner.on_bar(bar);

        if kill_active {
            // Q4=(p3) broadened filter (2026-05-26): when the kill-switch fires for
            // ANY symbol in the basket, convert ALL signals in the rebalance basket
            // to Hold — not just the triggering symbol's signals.
            //
            // Rationale: `VolKillSwitchOverlay` wraps a cross-sectional momentum
            // strategy (`MomentumStrategy`).  At rebalance time the inner strategy
            // emits signals for multiple basket symbols simultaneously.  If vol
            // spikes on BTCUSDT, the operator's intent is to halt the WHOLE basket
            // (belt+suspenders), not just the spiking symbol.  See K2 in the feature
            // brief for the single-symbol-strategy caveat — this overlay is
            // cross-sectional-basket-only at v0.1.0.
            //
            // Before (Q4=(p1)-default — narrow, trigger-symbol-only):
            //   if sig.symbol == bar.symbol { sig.kind = SignalKind::Hold; }
            // After (Q4=(p3) — broaden to cross-sectional basket):
            base_signals
                .into_iter()
                .map(|mut sig| {
                    sig.kind = SignalKind::Hold;
                    sig
                })
                .collect()
        } else {
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
            "threshold_multiplier": { "type": "number", "default": 3.0 },
            "cooldown_bars": { "type": "integer", "default": 4 },
            "rolling_window": { "type": "integer", "default": 720 },
            "momentum_config_id": { "type": "string", "default": "top10_momentum" },
            "forecaster_id": { "type": "string", "default": "garch-bs1" }
        })
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
    fn rolling_median_odd() {
        let buf = [1.0, 3.0, 2.0];
        let m = VolKillSwitchOverlay::rolling_median(&buf);
        assert!((m - 2.0).abs() < 1e-9, "median of [1,2,3] is 2.0, got {m}");
    }

    #[test]
    fn rolling_median_even() {
        let buf = [1.0, 2.0, 3.0, 4.0];
        let m = VolKillSwitchOverlay::rolling_median(&buf);
        assert!(
            (m - 2.5).abs() < 1e-9,
            "median of [1,2,3,4] is 2.5, got {m}"
        );
    }

    #[test]
    fn rolling_median_empty() {
        let m = VolKillSwitchOverlay::rolling_median(&[]);
        assert_eq!(m, 0.0, "empty buffer median should be 0.0");
    }

    #[test]
    fn vol_killswitch_new_initialises() {
        let mut models = BTreeMap::new();
        models.insert("BTCUSDT".to_string(), stub_model());
        let inner = make_inner();
        let overlay = VolKillSwitchOverlay::new(inner, models, VolKillSwitchConfig::default());
        let sym = Symbol::new("BTCUSDT");
        assert!(overlay.state.contains_key(&sym));
        assert_eq!(overlay.kill_switch_count, 0);
    }

    fn make_inner() -> MomentumStrategy {
        use crate::cross_sectional::CrossSectionalMomentumConfig;
        let toml = r#"
id    = "top10_momentum_h1"
kind  = "cross_sectional_momentum"
stage = "research"
universe = ["BTCUSDT", "ETHUSDT"]
lookback_minutes = 60
rebalance_minutes = 60
k_long = 1
k_short = 0
exposure_cap = 0.50
drift_rebalance_threshold = 0.10
vol_floor = 0.000001
size = "equal_weight"
"#;
        let cfg = CrossSectionalMomentumConfig::from_str(toml).expect("valid");
        MomentumStrategy::from_config(cfg, smol_str::SmolStr::new("stub"))
    }
}
