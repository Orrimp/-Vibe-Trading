//! Vol mean-reversion standalone strategy (v3.0.0-volatility R6.c tertiary).
//!
//! Emits a position proportional to the vol surprise:
//! `1 - sigma_predicted / sigma_realized` when `sigma_realized > sigma_predicted`
//! (i.e. vol is higher than expected → expect reversion).
//!
//! **v0.1.0 scope:** unit-tested only. No backtest scenario in v0.1.0 per
//! feature.md § Q-anchors-sub = 3 default and ADR-0038 § D5.
//!
//! ## Signal logic
//!
//! For each bar:
//! 1. Compute `sigma_predicted = GarchModel::forecast_step(r_prev, sigma_prev)`.
//! 2. Compute `sigma_realized = realized_vol_proxy(bar)` — Parkinson single-bar.
//! 3. If `sigma_realized > sigma_predicted`:
//!    - vol_surprise = `sigma_realized / sigma_predicted - 1.0` (∈ [0, ∞))
//!    - Emit `SignalKind::Buy` with `vol_surprise` as the intensity.
//! 4. Else: emit `SignalKind::Hold`.
//!
//! ## Determinism
//!
//! - No `SystemTime::now()` in `on_bar()`.
//! - Parkinson formula is closed-form + deterministic.
//!
//! ## Cross-references
//!
//! - ADR-0038 § D5 — strategy-side composition lock.
//! - `crates/strategy/src/vol_targeting_overlay.rs` — sibling (R6.a).
//! - `crates/strategy/src/vol_killswitch_overlay.rs` — sibling (R6.b).

use std::collections::BTreeMap;

use rust_decimal::prelude::ToPrimitive;
use trading_core::{Bar, Signal, SignalEvidence, SignalKind, StrategyId, Symbol, Tick};

use crate::Strategy;
use crate::vol_targeting_overlay::GarchParams;

// ── VolMeanReversionConfig ────────────────────────────────────────────────────

/// Configuration for `VolMeanReversionStrategy`.
#[derive(Debug, Clone)]
pub struct VolMeanReversionConfig {
    /// Minimum vol_surprise required to emit a Buy signal (default 0.0).
    pub min_surprise_threshold: f64,
    /// Floor on sigma_predicted to prevent zero-division (default 1e-8).
    pub min_sigma_floor: f64,
}

impl Default for VolMeanReversionConfig {
    fn default() -> Self {
        Self {
            min_surprise_threshold: 0.0,
            min_sigma_floor: 1e-8,
        }
    }
}

// ── Per-symbol state ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct MeanRevState {
    r_prev: f64,
    sigma_prev: f64,
    prev_close: f64,
}

// ── VolMeanReversionStrategy ──────────────────────────────────────────────────

/// Standalone vol mean-reversion strategy.
///
/// Emits Buy when realized vol (Parkinson single-bar) > predicted vol
/// (GARCH recurrence). Signal intensity proportional to vol_surprise.
pub struct VolMeanReversionStrategy {
    id: StrategyId,
    models: BTreeMap<Symbol, GarchParams>,
    state: BTreeMap<Symbol, MeanRevState>,
    config: VolMeanReversionConfig,
    /// Diagnostics: number of vol-surprise signals emitted.
    pub signals_emitted: u64,
    /// Diagnostics: total bars processed.
    pub bars_total: u64,
}

impl std::fmt::Debug for VolMeanReversionStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VolMeanReversionStrategy")
            .field("id", &self.id)
            .field("config", &self.config)
            .field("signals_emitted", &self.signals_emitted)
            .finish()
    }
}

impl VolMeanReversionStrategy {
    /// Construct from a set of GARCH models and symbols to trade.
    #[must_use]
    pub fn new(
        symbols: &[&str],
        models: BTreeMap<String, GarchParams>,
        config: VolMeanReversionConfig,
    ) -> Self {
        let state: BTreeMap<Symbol, MeanRevState> = symbols
            .iter()
            .map(|&sym| {
                let sigma_init = models.get(sym).map(GarchParams::init_sigma).unwrap_or(1e-4);
                (
                    Symbol::new(sym),
                    MeanRevState {
                        r_prev: 0.0,
                        sigma_prev: sigma_init,
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
            id: StrategyId::new("vol_meanreversion"),
            models: models_by_sym,
            state,
            config,
            signals_emitted: 0,
            bars_total: 0,
        }
    }

    /// Compute Parkinson single-bar realized vol estimate.
    ///
    /// `σ̂_P = |ln(high / low)| / sqrt(4 · ln 2)`.
    /// Returns 0.0 if high <= 0 or low <= 0.
    #[must_use]
    fn parkinson_single_bar(high: f64, low: f64) -> f64 {
        if high <= 0.0 || low <= 0.0 || high < low {
            return 0.0;
        }
        let ln_hl = (high / low).ln();
        let parkinson_var = (1.0 / (4.0 * f64::ln(2.0))) * (ln_hl * ln_hl);
        parkinson_var.sqrt()
    }
}

impl Strategy for VolMeanReversionStrategy {
    fn id(&self) -> StrategyId {
        self.id.clone()
    }

    fn on_bar(&mut self, bar: &Bar) -> Vec<Signal> {
        self.bars_total += 1;

        let close_f64 = bar.close.get().to_f64().unwrap_or(0.0);
        let high_f64 = bar.high.get().to_f64().unwrap_or(0.0);
        let low_f64 = bar.low.get().to_f64().unwrap_or(0.0);

        let Some(model) = self.models.get(&bar.symbol) else {
            return vec![];
        };

        let state = self
            .state
            .entry(bar.symbol.clone())
            .or_insert_with(|| MeanRevState {
                r_prev: 0.0,
                sigma_prev: model.init_sigma(),
                prev_close: 0.0,
            });

        // Compute log-return.
        let r_curr = if state.prev_close > 0.0 && close_f64 > 0.0 {
            (close_f64 / state.prev_close).ln()
        } else {
            0.0
        };

        // GARCH step.
        let sigma_predicted = model
            .forecast_step(state.r_prev, state.sigma_prev)
            .max(self.config.min_sigma_floor);

        // Advance GARCH state.
        state.sigma_prev = sigma_predicted;
        state.r_prev = r_curr;
        state.prev_close = close_f64;

        // Parkinson realized vol for this bar.
        let sigma_realized = Self::parkinson_single_bar(high_f64, low_f64);

        // Emit signal if vol surprise exceeds threshold.
        if sigma_realized > sigma_predicted {
            let vol_surprise = sigma_realized / sigma_predicted - 1.0;
            if vol_surprise >= self.config.min_surprise_threshold {
                self.signals_emitted += 1;
                let score = rust_decimal::Decimal::try_from(vol_surprise).unwrap_or_default();
                return vec![Signal {
                    strategy_id: self.id.clone(),
                    symbol: bar.symbol.clone(),
                    ts: bar.close_ts,
                    kind: SignalKind::Buy,
                    evidence: SignalEvidence::momentum("vol_surprise", score),
                    pair_data: None,
                }];
            }
        }

        // No vol surprise above threshold → Hold.
        vec![Signal {
            strategy_id: self.id.clone(),
            symbol: bar.symbol.clone(),
            ts: bar.close_ts,
            kind: SignalKind::Hold,
            evidence: SignalEvidence::momentum("hold", rust_decimal::Decimal::ZERO),
            pair_data: None,
        }]
    }

    fn on_tick(&mut self, _tick: &Tick) -> Vec<Signal> {
        vec![]
    }

    fn config_schema() -> serde_json::Value
    where
        Self: Sized,
    {
        serde_json::json!({
            "min_surprise_threshold": { "type": "number", "default": 0.0 },
            "min_sigma_floor": { "type": "number", "default": 1e-8 },
            "forecaster_id": { "type": "string", "default": "garch-bs1" }
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parkinson_single_bar_known_value() {
        // high=e, low=1 → ln(e/1)=1 → sigma = 1/sqrt(4*ln2)
        let sigma = VolMeanReversionStrategy::parkinson_single_bar(std::f64::consts::E, 1.0);
        let expected = 1.0 / (4.0 * f64::ln(2.0)).sqrt();
        assert!(
            (sigma - expected).abs() < 1e-10,
            "got {sigma}, expected {expected}"
        );
    }

    #[test]
    fn parkinson_single_bar_zero_spread() {
        let sigma = VolMeanReversionStrategy::parkinson_single_bar(1.5, 1.5);
        assert_eq!(sigma, 0.0, "zero spread → sigma = 0");
    }

    #[test]
    fn parkinson_single_bar_invalid_guard() {
        assert_eq!(
            VolMeanReversionStrategy::parkinson_single_bar(0.0, 1.0),
            0.0
        );
        assert_eq!(
            VolMeanReversionStrategy::parkinson_single_bar(1.0, 0.0),
            0.0
        );
        assert_eq!(
            VolMeanReversionStrategy::parkinson_single_bar(0.5, 1.0),
            0.0
        ); // high < low
    }
}
