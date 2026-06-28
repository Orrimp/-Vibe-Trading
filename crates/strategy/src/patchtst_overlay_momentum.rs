//! PatchTST overlay momentum strategy (v2.5a Wave A.4, T-D-N22).
//!
//! Sibling of `tcn_overlay_momentum.rs` for PatchTST (Nie et al 2022).
//! Wraps the v1 cross-sectional momentum strategy (`MomentumStrategy`) with a
//! PatchTST forecast overlay. Mirrors the TCN strategy shape verbatim per
//! ADR-0036 § D7 + decomp.md § T-AR-4 + Q8=(a).
//!
//! ## Key differences from TCN overlay
//!
//! | Attribute           | TCN overlay          | PatchTST overlay                |
//! |---------------------|---------------------|---------------------------------|
//! | context_len         | 256 bars             | 336 bars (14 days hourly)       |
//! | model family        | tcn                  | patchtst                        |
//! | checkpoint prefix   | tcn-bs{1,2}-<sha>    | patchtst-bs1-<sha>              |
//! | target horizon      | 1 bar                | 24 bars (24h)                   |
//! | σ_train derivation  | in-loop (deprecated) | post-training frozen pass       |
//!
//! ## Composition rule
//!
//! Identical to `tcn_overlay_momentum.rs`:
//! 1. Feed bar → `MomentumStrategy` → base `Signal`.
//! 2. Maintain per-symbol rolling window of last 336 OHLCV bars.
//! 3. When window full, run `PatchTstSyncForecaster::infer()`.
//! 4. Apply same `combine_with_direction()` rule (confidence_threshold = 0.6).
//! 5. Return modulated signals.
//!
//! ## Strategy ID
//!
//! `"patchtst_overlay_momentum"` — registered via `StrategyRegistry::register()`.
//!
//! ## Cross-references
//!
//! - `spec/v1/v25a-patchtst-overlay/feature.md § Q8=(a)` — Q8 decision
//! - `spec/architecture/adr/0036-patchtst-training-contract.md § D7`
//! - `crates/forecast/src/patchtst.rs` — `PatchTstForecaster` + `AnchorScenario`
//! - `crates/strategy/src/tcn_overlay_momentum.rs` — TCN sibling (mirror source)

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use trading_core::{Bar, Signal, SignalKind, StrategyId, Symbol, Tick};

use crate::Strategy;
use crate::cross_sectional::MomentumStrategy;
// Re-export the shared direction types from the TCN overlay module.
use crate::tcn_overlay_momentum::{ForecastDirection, SyncForecaster};

// ── PatchTstOverlayMomentumConfig ─────────────────────────────────────────────

/// Configuration for `PatchTstOverlayMomentumStrategy`.
///
/// Mirrors `TcnOverlayMomentumConfig` with PatchTST defaults.
#[derive(Debug, Clone)]
pub struct PatchTstOverlayMomentumConfig {
    /// Which anchor checkpoint to load: `"patchtst-bs1"` (only option at v0.1.0).
    pub forecaster_id: String,
    /// Minimum forecast confidence required to modulate the base signal.
    pub confidence_threshold: Decimal,
    /// Path to the base momentum config TOML.
    pub base_config_path: String,
}

impl Default for PatchTstOverlayMomentumConfig {
    fn default() -> Self {
        Self {
            forecaster_id: "patchtst-bs1".to_string(),
            confidence_threshold: dec!(0.6),
            base_config_path: "config/strategies/top10_momentum_h1.toml".to_string(),
        }
    }
}

// ── ModulationStats (re-used from tcn_overlay_momentum) ───────────────────────
// We re-export rather than duplicate so the two modules share the type.
pub use crate::tcn_overlay_momentum::ModulationStats;
pub use crate::tcn_overlay_momentum::PassthroughForecaster;

// ── PatchTstSyncForecaster: wraps forecast::patchtst::PatchTstForecaster ──────

/// Production sync forecaster that wraps `forecast::patchtst::PatchTstForecaster`.
///
/// Inference runs on CPU; no async executor needed.
#[cfg(feature = "forecast")]
pub struct PatchTstSyncForecaster {
    forecaster: forecast::patchtst::PatchTstForecaster,
}

#[cfg(feature = "forecast")]
impl PatchTstSyncForecaster {
    /// Load the BS-1 anchor checkpoint.
    ///
    /// # Errors
    ///
    /// Returns `forecast::patchtst::PatchTstForecasterError` if the checkpoint
    /// is absent or fails to decode.
    pub fn load_bs1() -> Result<Self, forecast::patchtst::PatchTstForecasterError> {
        let f = forecast::patchtst::PatchTstForecaster::load_anchor(
            forecast::patchtst::AnchorScenario::Bs1,
        )?;
        Ok(Self { forecaster: f })
    }
}

#[cfg(feature = "forecast")]
impl SyncForecaster for PatchTstSyncForecaster {
    fn infer(&self, bars: &[Bar]) -> (ForecastDirection, Decimal) {
        use candle_core::{Device, Tensor};
        use rust_decimal::prelude::ToPrimitive;
        use rust_decimal_macros::dec;

        let n = bars.len();
        if n < 2 {
            return (ForecastDirection::Flat, Decimal::ZERO);
        }

        // Build the [1, CHANNELS, context_len] feature tensor from Bar structs.
        // Features: logret, logrange, logvol_z, hour_sin, hour_cos
        // (identical order to TCN features per Q5=(c) carry-forward).
        use std::f32::consts::PI;

        let log_vols: Vec<f32> = bars
            .iter()
            .map(|b| (1.0_f32 + b.volume.get().to_f32().unwrap_or(0.0)).ln())
            .collect();
        let mu_vol = log_vols.iter().sum::<f32>() / n as f32;
        let sigma_vol = {
            let var = log_vols.iter().map(|v| (v - mu_vol).powi(2)).sum::<f32>() / n as f32;
            var.sqrt().max(1e-6)
        };

        // Channel-first layout: feat_cf[c * n + t].
        let channels = 5;
        let mut feat_cf: Vec<f32> = vec![0.0_f32; channels * n];
        for t in 0..n {
            let bar = &bars[t];
            let close = bar.close.get().to_f32().unwrap_or(1.0).max(1e-8);
            let high = bar.high.get().to_f32().unwrap_or(close);
            let low = bar.low.get().to_f32().unwrap_or(close);

            let logret = if t == 0 {
                0.0
            } else {
                let prev = bars[t - 1].close.get().to_f32().unwrap_or(1.0).max(1e-8);
                (close / prev).ln()
            };
            let logrange = (1.0_f32 + (high - low) / close).ln();
            let logvol_z = (log_vols[t] - mu_vol) / sigma_vol;

            // hour_of_week from Bar close_ts.
            let ts = bar.close_ts.inner();
            let weekday_offset = {
                use time::Weekday;
                match ts.weekday() {
                    Weekday::Monday => 0,
                    Weekday::Tuesday => 24,
                    Weekday::Wednesday => 48,
                    Weekday::Thursday => 72,
                    Weekday::Friday => 96,
                    Weekday::Saturday => 120,
                    Weekday::Sunday => 144,
                }
            };
            let hour_of_week = (weekday_offset + ts.hour() as usize) as f32;
            let hour_sin = (2.0 * PI * hour_of_week / 168.0).sin();
            let hour_cos = (2.0 * PI * hour_of_week / 168.0).cos();

            #[allow(clippy::erasing_op, clippy::identity_op)]
            {
                feat_cf[0 * n + t] = logret;
                feat_cf[1 * n + t] = logrange;
            }
            feat_cf[2 * n + t] = logvol_z;
            feat_cf[3 * n + t] = hour_sin;
            feat_cf[4 * n + t] = hour_cos;
        }

        let device = Device::Cpu;
        let x = match Tensor::from_vec(feat_cf, (1, channels, n), &device) {
            Ok(t) => t,
            Err(_) => return (ForecastDirection::Flat, Decimal::ZERO),
        };

        let y = match self.forecaster.forward(&x, false) {
            Ok(t) => t,
            Err(_) => return (ForecastDirection::Flat, Decimal::ZERO),
        };

        let r_hat = match y.flatten_all().and_then(|t| t.to_vec1::<f32>()) {
            Ok(v) => v.first().copied().unwrap_or(0.0),
            Err(_) => return (ForecastDirection::Flat, Decimal::ZERO),
        };

        // PatchTST uses the same ε deadband as TCN (same FeatureConfig epsilon).
        let eps = forecast::patchtst::DIRECTION_EPSILON;
        let direction = if r_hat > eps {
            ForecastDirection::Up
        } else if r_hat < -eps {
            ForecastDirection::Down
        } else {
            ForecastDirection::Flat
        };

        let confidence_f = (r_hat.abs() / self.forecaster.sigma_train).clamp(0.0, 1.0);
        let confidence = Decimal::try_from(f64::from(confidence_f)).unwrap_or(dec!(0));

        (direction, confidence)
    }
}

// ── PatchTstOverlayMomentumStrategy ──────────────────────────────────────────

/// Strategy that applies a PatchTST forecast overlay on top of v1 momentum signals.
///
/// The strategy ID is `"patchtst_overlay_momentum"`.
pub struct PatchTstOverlayMomentumStrategy {
    id: StrategyId,
    /// Inner v1 momentum strategy.
    base: MomentumStrategy,
    /// PatchTST forecaster (boxed for DI / testability).
    forecaster: Box<dyn SyncForecaster>,
    /// Per-symbol rolling windows of the last CONTEXT_LEN (336) bars.
    windows: BTreeMap<Symbol, Vec<Bar>>,
    /// Forecast confidence threshold (default 0.6).
    confidence_threshold: Decimal,
    /// Modulation statistics for diagnostics.
    pub stats: ModulationStats,
}

impl std::fmt::Debug for PatchTstOverlayMomentumStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PatchTstOverlayMomentumStrategy")
            .field("id", &self.id)
            .field("confidence_threshold", &self.confidence_threshold)
            .field("stats", &self.stats)
            .finish()
    }
}

// ── Construction ──────────────────────────────────────────────────────────────

impl PatchTstOverlayMomentumStrategy {
    /// Construct from an explicit base strategy + forecaster.
    #[must_use]
    pub fn new(
        base: MomentumStrategy,
        forecaster: Box<dyn SyncForecaster>,
        confidence_threshold: Decimal,
    ) -> Self {
        let windows: BTreeMap<Symbol, Vec<Bar>> =
            base.universe().map(|s| (s.clone(), Vec::new())).collect();

        Self {
            id: StrategyId::new("patchtst_overlay_momentum"),
            base,
            forecaster,
            windows,
            confidence_threshold,
            stats: ModulationStats::default(),
        }
    }

    /// Return the confidence threshold for this strategy instance.
    #[must_use]
    pub fn confidence_threshold(&self) -> Decimal {
        self.confidence_threshold
    }

    /// Construct with a `PassthroughForecaster` (degrades to pure v1 momentum).
    #[must_use]
    pub fn with_passthrough(base: MomentumStrategy) -> Self {
        Self::new(base, Box::new(PassthroughForecaster), dec!(0.6))
    }

    /// Construct with the real BS-1 PatchTST anchor checkpoint.
    ///
    /// Loads `patchtst-bs1-<sha>` from `crates/forecast/checkpoints/anchors/`.
    /// Requires the `forecast` feature (candle backend).
    ///
    /// # Errors
    ///
    /// Returns `forecast::patchtst::PatchTstForecasterError` if the checkpoint
    /// is absent or fails to decode.
    #[cfg(feature = "forecast")]
    pub fn with_patchtst_bs1(
        base: MomentumStrategy,
    ) -> Result<Self, forecast::patchtst::PatchTstForecasterError> {
        let forecaster = PatchTstSyncForecaster::load_bs1()?;
        Ok(Self::new(base, Box::new(forecaster), dec!(0.6)))
    }
}

// ── Strategy impl ─────────────────────────────────────────────────────────────

impl Strategy for PatchTstOverlayMomentumStrategy {
    fn id(&self) -> StrategyId {
        self.id.clone()
    }

    fn on_bar(&mut self, bar: &Bar) -> Vec<Signal> {
        // 1. Feed bar to base momentum strategy.
        let base_signals = self.base.on_bar(bar);

        // 2. Update per-symbol rolling window (CONTEXT_LEN = 336 for PatchTST).
        let context_len = patchtst_context_len();
        if let Some(window) = self.windows.get_mut(&bar.symbol) {
            window.push(bar.clone());
            if window.len() > context_len {
                let excess = window.len() - context_len;
                window.drain(..excess);
            }
        }

        if base_signals.is_empty() {
            return base_signals;
        }

        // 3. Apply PatchTST overlay to each signal.
        let window_opt = self.windows.get(&bar.symbol);

        base_signals
            .into_iter()
            .map(|sig| {
                self.stats.total += 1;

                let window = match window_opt {
                    Some(w) if w.len() >= context_len => w,
                    _ => {
                        self.stats.window_warming_up += 1;
                        return sig;
                    }
                };

                let (direction, confidence) = self.forecaster.infer(window);

                let modulated_kind = combine_with_direction(
                    sig.kind,
                    direction,
                    confidence,
                    self.confidence_threshold,
                );

                if modulated_kind == sig.kind {
                    self.stats.passed_through += 1;
                } else {
                    self.stats.dampened += 1;
                    tracing::debug!(
                        symbol = %sig.symbol,
                        base_kind = ?sig.kind,
                        modulated = ?modulated_kind,
                        direction = ?direction,
                        confidence = %confidence,
                        "patchtst_overlay: signal dampened"
                    );
                }

                Signal {
                    kind: modulated_kind,
                    ..sig
                }
            })
            .collect()
    }

    fn on_tick(&mut self, _tick: &Tick) -> Vec<Signal> {
        Vec::new()
    }

    fn config_schema() -> serde_json::Value
    where
        Self: Sized,
    {
        serde_json::json!({
            "type": "object",
            "properties": {
                "forecaster_id": { "type": "string", "enum": ["patchtst-bs1"] },
                "confidence_threshold": { "type": "string", "default": "0.6" },
                "base": { "type": "string", "default": "cross_sectional_momentum" }
            }
        })
    }
}

// ── combine helper (mirrors tcn_overlay_momentum) ────────────────────────────

fn combine_with_direction(
    base: SignalKind,
    direction: ForecastDirection,
    confidence: Decimal,
    threshold: Decimal,
) -> SignalKind {
    if confidence < threshold {
        return base;
    }
    if direction == ForecastDirection::Flat {
        return base;
    }

    let base_bullish = matches!(base, SignalKind::Buy);
    let base_bearish = matches!(base, SignalKind::Sell);
    let dir_up = direction == ForecastDirection::Up;
    let dir_down = direction == ForecastDirection::Down;

    if (base_bullish && dir_up) || (base_bearish && dir_down) {
        return base;
    }
    if (base_bullish && dir_down) || (base_bearish && dir_up) {
        return SignalKind::Hold;
    }
    base
}

/// Returns the context window length for the PatchTST forecaster.
fn patchtst_context_len() -> usize {
    336 // CONTEXT_LEN from crates/forecast/src/patchtst.rs
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use smol_str::SmolStr;

    // ── combine_with_direction tests ──────────────────────────────────────────

    #[test]
    fn patchtst_combine_agree_buy_passthrough() {
        let result =
            combine_with_direction(SignalKind::Buy, ForecastDirection::Up, dec!(0.8), dec!(0.6));
        assert_eq!(result, SignalKind::Buy);
    }

    #[test]
    fn patchtst_combine_disagree_confident_dampens_to_hold() {
        let result = combine_with_direction(
            SignalKind::Buy,
            ForecastDirection::Down,
            dec!(0.85),
            dec!(0.6),
        );
        assert_eq!(result, SignalKind::Hold);
    }

    #[test]
    fn patchtst_combine_flat_overlay_passthrough() {
        let result = combine_with_direction(
            SignalKind::Buy,
            ForecastDirection::Flat,
            dec!(0.99),
            dec!(0.6),
        );
        assert_eq!(result, SignalKind::Buy);
    }

    #[test]
    fn patchtst_combine_low_confidence_passthrough() {
        let result = combine_with_direction(
            SignalKind::Sell,
            ForecastDirection::Up,
            dec!(0.4),
            dec!(0.6),
        );
        assert_eq!(result, SignalKind::Sell);
    }

    #[test]
    fn patchtst_context_len_is_336() {
        assert_eq!(patchtst_context_len(), 336);
    }

    /// Load the MomentumStrategy from the standard config file if present;
    /// otherwise skip gracefully (CI without config files).
    fn try_load_base() -> Option<MomentumStrategy> {
        use crate::CrossSectionalMomentumConfig;
        let cfg_path = std::path::PathBuf::from("config/strategies/top10_momentum_h1.toml");
        CrossSectionalMomentumConfig::from_file(&cfg_path)
            .ok()
            .map(|cfg| MomentumStrategy::from_config(cfg, SmolStr::new(cfg_path.to_string_lossy())))
    }

    #[test]
    fn patchtst_strategy_id() {
        let Some(base) = try_load_base() else {
            eprintln!("SKIP patchtst_strategy_id: config not found");
            return;
        };
        let strategy = PatchTstOverlayMomentumStrategy::with_passthrough(base);
        assert_eq!(strategy.id().to_string(), "patchtst_overlay_momentum");
    }

    #[test]
    fn patchtst_confidence_threshold_default_06() {
        let Some(base) = try_load_base() else {
            eprintln!("SKIP patchtst_confidence_threshold_default_06: config not found");
            return;
        };
        let strategy = PatchTstOverlayMomentumStrategy::with_passthrough(base);
        assert_eq!(strategy.confidence_threshold(), dec!(0.6));
    }
}
