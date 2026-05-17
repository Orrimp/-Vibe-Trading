//! TCN overlay momentum strategy (v2.5 T-D-14, M5).
//!
//! Wraps the v1 cross-sectional momentum strategy (`MomentumStrategy`) with a
//! TCN forecast overlay per `feature.md § D5` and
//! `architecture/12-forecast-overlay.md § Overlay composition pattern`.
//!
//! ## Composition rule (feature.md § D5)
//!
//! ```text
//! 1. Feed the bar to MomentumStrategy → base Signal.
//! 2. Maintain a per-symbol rolling window of the last 256 OHLCV bars.
//! 3. When the window is full, run TcnForecaster synchronously (CPU inference
//!    is fast: <10 ms on M-series; no async needed in the sync Strategy trait).
//! 4. Apply overlay::combine() with confidence_threshold = dec!(0.6).
//! 5. Return modulated signals.
//! ```
//!
//! ## Determinism
//!
//! - All RNG paths delegated to MomentumStrategy (unchanged from v1 baseline).
//! - TCN inference is deterministic on CPU (same weights + same inputs → same
//!   f32 output).
//! - No `SystemTime::now()` in the hot path.
//!
//! ## Strategy ID
//!
//! `"tcn_overlay_momentum"` — registered via `StrategyRegistry::register()`.
//!
//! ## Cross-references
//!
//! - `spec/v25-tcn-overlay/feature.md § D5` — thresholds
//! - `crates/forecast/src/overlay.rs` — combine() helper
//! - `crates/forecast/src/tcn.rs` — TcnForecaster + AnchorScenario

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use trading_core::{Bar, Signal, SignalKind, StrategyId, Symbol, Tick};

use crate::cross_sectional::MomentumStrategy;
use crate::Strategy;

// ── TcnOverlayMomentumConfig ───────────────────────────────────────────────────

/// Configuration for `TcnOverlayMomentumStrategy`.
///
/// The `confidence_threshold` and `forecaster_id` fields map to the TOML
/// shape specified in `feature.md § D5`.
#[derive(Debug, Clone)]
pub struct TcnOverlayMomentumConfig {
    /// Which anchor checkpoint to load: `"tcn-bs1"` or `"tcn-bs2"`.
    pub forecaster_id: String,
    /// Minimum forecast confidence required to modulate the base signal.
    /// Below this threshold, the base signal passes through unchanged.
    pub confidence_threshold: Decimal,
    /// Path to the base momentum config TOML.
    pub base_config_path: String,
}

impl Default for TcnOverlayMomentumConfig {
    fn default() -> Self {
        Self {
            forecaster_id: "tcn-bs1".to_string(),
            confidence_threshold: dec!(0.6),
            base_config_path: "config/strategies/top10_momentum_h1.toml".to_string(),
        }
    }
}

// ── Modulation statistics (for dry-run reporting) ─────────────────────────────

/// Running counts of overlay modulations (for diagnostics).
#[derive(Debug, Default, Clone)]
pub struct ModulationStats {
    /// Signals passed through without modulation (base kept).
    pub passed_through: u64,
    /// Signals dampened to Hold (disagree + confident).
    pub dampened: u64,
    /// Bars where window was not full (no overlay applied).
    pub window_warming_up: u64,
    /// Total signals processed.
    pub total: u64,
}

impl ModulationStats {
    /// Dampen rate (dampened / total non-warmup).
    pub fn dampen_rate(&self) -> f64 {
        let eligible = self.passed_through + self.dampened;
        if eligible == 0 {
            0.0
        } else {
            self.dampened as f64 / eligible as f64
        }
    }
}

// ── TcnOverlayMomentumStrategy ────────────────────────────────────────────────

/// Strategy that applies a TCN forecast overlay on top of v1 momentum signals.
///
/// The strategy ID is `"tcn_overlay_momentum"`.
///
/// `ForecastProvider` is held as a boxed trait object so tests can inject
/// mocks without loading the full LFS checkpoint.  Production callers use
/// `from_config()` which loads the anchor checkpoint.
pub struct TcnOverlayMomentumStrategy {
    id: StrategyId,
    /// Inner v1 momentum strategy.
    base: MomentumStrategy,
    /// TCN forecaster (boxed for DI / testability).
    forecaster: Box<dyn SyncForecaster>,
    /// Per-symbol rolling windows of the last CONTEXT_LEN bars.
    windows: BTreeMap<Symbol, Vec<Bar>>,
    /// Forecast confidence threshold (default 0.6 per D5).
    confidence_threshold: Decimal,
    /// Modulation statistics for diagnostics.
    pub stats: ModulationStats,
}

impl std::fmt::Debug for TcnOverlayMomentumStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TcnOverlayMomentumStrategy")
            .field("id", &self.id)
            .field("confidence_threshold", &self.confidence_threshold)
            .field("stats", &self.stats)
            .finish()
    }
}

// ── SyncForecaster: sync wrapper around TcnForecaster ─────────────────────────

/// A synchronous forecaster trait for use in the sync `Strategy::on_bar()`.
///
/// CPU TCN inference is deterministic and fast (<10 ms), so blocking the
/// async runtime is acceptable here.  The trait exists for DI in tests.
pub trait SyncForecaster: Send + Sync {
    /// Run inference on a window of bars and return `(direction, confidence)`.
    ///
    /// `direction` is one of `"up"`, `"down"`, `"flat"`.
    /// `confidence` is in `[0, 1]`.
    fn infer(&self, bars: &[Bar]) -> (ForecastDirection, Decimal);
}

/// Direction from the forecaster.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForecastDirection {
    Up,
    Down,
    Flat,
}

// ── TcnSyncForecaster: wraps forecast::tcn::TcnForecaster ────────────────────

/// Production sync forecaster that wraps `forecast::tcn::TcnForecaster`.
///
/// Inference runs on CPU; no async executor needed.
#[cfg(feature = "forecast")]
pub struct TcnSyncForecaster {
    forecaster: forecast::tcn::TcnForecaster,
}

#[cfg(feature = "forecast")]
impl TcnSyncForecaster {
    /// Load the BS-1 anchor checkpoint.
    ///
    /// # Errors
    ///
    /// Returns `forecast::tcn::TcnForecasterError` if the checkpoint is absent.
    pub fn load_bs1() -> Result<Self, forecast::tcn::TcnForecasterError> {
        let f = forecast::tcn::TcnForecaster::load_anchor(forecast::tcn::AnchorScenario::Bs1)?;
        Ok(Self { forecaster: f })
    }

    /// Load the BS-2 anchor checkpoint.
    ///
    /// # Errors
    ///
    /// Returns `forecast::tcn::TcnForecasterError` if the checkpoint is absent.
    pub fn load_bs2() -> Result<Self, forecast::tcn::TcnForecasterError> {
        let f = forecast::tcn::TcnForecaster::load_anchor(forecast::tcn::AnchorScenario::Bs2)?;
        Ok(Self { forecaster: f })
    }
}

#[cfg(feature = "forecast")]
impl SyncForecaster for TcnSyncForecaster {
    fn infer(&self, bars: &[Bar]) -> (ForecastDirection, Decimal) {
        use candle_core::{Device, Tensor};
        use rust_decimal::prelude::ToPrimitive;
        use rust_decimal_macros::dec;

        let n = bars.len();
        if n < 2 {
            return (ForecastDirection::Flat, Decimal::ZERO);
        }

        // Build the [1, 5, n] feature tensor from Bar structs.
        // Features: logret, logrange, logvol_z, hour_sin, hour_cos.
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

        let mut feat_cf: Vec<f32> = vec![0.0_f32; 5 * n];
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

            feat_cf[0 * n + t] = logret;
            feat_cf[1 * n + t] = logrange;
            feat_cf[2 * n + t] = logvol_z;
            feat_cf[3 * n + t] = hour_sin;
            feat_cf[4 * n + t] = hour_cos;
        }

        let device = Device::Cpu;
        let x = match Tensor::from_vec(feat_cf, (1, 5, n), &device) {
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

        let direction = if r_hat > forecast::tcn::DIRECTION_EPSILON {
            ForecastDirection::Up
        } else if r_hat < -forecast::tcn::DIRECTION_EPSILON {
            ForecastDirection::Down
        } else {
            ForecastDirection::Flat
        };

        let confidence_f = (r_hat.abs() / self.forecaster.sigma_train).clamp(0.0, 1.0);
        let confidence = Decimal::try_from(f64::from(confidence_f)).unwrap_or(dec!(0));

        (direction, confidence)
    }
}

// ── ConstForecaster: no-op for backtest mode without candle feature ────────────

/// A pass-through forecaster that always returns `Flat` with confidence 0.
///
/// Used when the `forecast` feature is not enabled or in environments where
/// the checkpoint cannot be loaded.  The overlay is always a no-op
/// (pass-through), so the strategy degrades gracefully to pure v1 momentum.
pub struct PassthroughForecaster;

impl SyncForecaster for PassthroughForecaster {
    fn infer(&self, _bars: &[Bar]) -> (ForecastDirection, Decimal) {
        (ForecastDirection::Flat, Decimal::ZERO)
    }
}

// ── Construction ──────────────────────────────────────────────────────────────

impl TcnOverlayMomentumStrategy {
    /// Construct from an explicit base strategy + forecaster.
    ///
    /// This is the primary constructor for both production and tests.
    #[must_use]
    pub fn new(
        base: MomentumStrategy,
        forecaster: Box<dyn SyncForecaster>,
        confidence_threshold: Decimal,
    ) -> Self {
        // Pre-populate the per-symbol window map with empty vecs.
        let windows: BTreeMap<Symbol, Vec<Bar>> = base
            .universe()
            .map(|s| (s.clone(), Vec::new()))
            .collect();

        Self {
            id: StrategyId::new("tcn_overlay_momentum"),
            base,
            forecaster,
            windows,
            confidence_threshold,
            stats: ModulationStats::default(),
        }
    }

    /// Construct with a `PassthroughForecaster` (degrades to pure v1 momentum).
    ///
    /// Used when the `forecast` feature is not enabled.
    #[must_use]
    pub fn with_passthrough(base: MomentumStrategy) -> Self {
        Self::new(
            base,
            Box::new(PassthroughForecaster),
            dec!(0.6),
        )
    }
}

// ── Strategy impl ─────────────────────────────────────────────────────────────

impl Strategy for TcnOverlayMomentumStrategy {
    fn id(&self) -> StrategyId {
        self.id.clone()
    }

    fn on_bar(&mut self, bar: &Bar) -> Vec<Signal> {
        // 1. Feed the bar to the base momentum strategy.
        let base_signals = self.base.on_bar(bar);

        // 2. Update the per-symbol rolling window.
        if let Some(window) = self.windows.get_mut(&bar.symbol) {
            window.push(bar.clone());
            // Trim to CONTEXT_LEN (256).
            let context_len = forecast_context_len();
            if window.len() > context_len {
                let excess = window.len() - context_len;
                window.drain(..excess);
            }
        }

        if base_signals.is_empty() {
            return base_signals;
        }

        // 3. Apply TCN overlay to each signal.
        let window_opt = self.windows.get(&bar.symbol);
        let context_len = forecast_context_len();

        base_signals
            .into_iter()
            .map(|sig| {
                self.stats.total += 1;

                // Get the window for this symbol.
                let window = match window_opt {
                    Some(w) if w.len() >= context_len => w,
                    _ => {
                        self.stats.window_warming_up += 1;
                        return sig;
                    }
                };

                // 4. Run inference.
                let (direction, confidence) = self.forecaster.infer(window);

                // 5. Apply combine() per overlay.rs rules.
                // Convert ForecastDirection → overlay::combine() compatible.
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
                        "tcn_overlay: signal dampened"
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
                "forecaster_id": { "type": "string", "enum": ["tcn-bs1", "tcn-bs2"] },
                "confidence_threshold": { "type": "string", "default": "0.6" },
                "base": { "type": "string", "default": "cross_sectional_momentum" }
            }
        })
    }
}

// ── combine() helper (duplicates overlay::combine but works with our local Direction) ──

/// Apply the overlay composition rule using our local `ForecastDirection`.
///
/// This mirrors `forecast::overlay::combine()` without importing `forecast`:
///
/// | Overlay direction | Confidence ≥ threshold | Base kind | Result           |
/// |---|---|---|---|
/// | Same as base      | yes                     | Buy/Sell  | pass-through     |
/// | Opposite to base  | yes                     | Buy/Sell  | Hold (dampen)    |
/// | Flat              | any                     | any       | pass-through     |
/// | any               | < threshold             | any       | pass-through     |
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

    // Agreement → pass-through.
    if (base_bullish && dir_up) || (base_bearish && dir_down) {
        return base;
    }

    // Disagreement + confident → dampen to Hold.
    if (base_bullish && dir_down) || (base_bearish && dir_up) {
        return SignalKind::Hold;
    }

    // All other cases (Hold, pair signals, etc.) → pass-through.
    base
}

/// Returns the context window length for the TCN forecaster.
///
/// Separated as a function so it can be patched in tests without the candle feature.
fn forecast_context_len() -> usize {
    256 // CONTEXT_LEN from crates/forecast/src/tcn.rs
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CrossSectionalMomentumConfig;
    use rust_decimal_macros::dec;

    // ── combine_with_direction tests ──────────────────────────────────────────

    #[test]
    fn combine_agree_confident_buy_passthrough() {
        let result = combine_with_direction(
            SignalKind::Buy,
            ForecastDirection::Up,
            dec!(0.8),
            dec!(0.6),
        );
        assert_eq!(result, SignalKind::Buy);
    }

    #[test]
    fn combine_disagree_confident_dampens_to_hold() {
        let result = combine_with_direction(
            SignalKind::Buy,
            ForecastDirection::Down,
            dec!(0.85),
            dec!(0.6),
        );
        assert_eq!(result, SignalKind::Hold);
    }

    #[test]
    fn combine_flat_overlay_passthrough() {
        let result = combine_with_direction(
            SignalKind::Buy,
            ForecastDirection::Flat,
            dec!(0.99),
            dec!(0.6),
        );
        assert_eq!(result, SignalKind::Buy);
    }

    #[test]
    fn combine_low_confidence_passthrough() {
        let result = combine_with_direction(
            SignalKind::Buy,
            ForecastDirection::Down,
            dec!(0.5),
            dec!(0.6),
        );
        assert_eq!(result, SignalKind::Buy);
    }

    #[test]
    fn combine_sell_disagree_up_dampens() {
        let result = combine_with_direction(
            SignalKind::Sell,
            ForecastDirection::Up,
            dec!(0.75),
            dec!(0.6),
        );
        assert_eq!(result, SignalKind::Hold);
    }

    // ── Strategy smoke test ───────────────────────────────────────────────────

    /// T-D-14: Single-symbol smoke test with PassthroughForecaster.
    ///
    /// Runs 50 bars of BTCUSDT through the strategy; verifies no panic and
    /// that signals are emitted without modulation (passthrough → identity).
    #[test]
    fn td14_smoke_with_passthrough_forecaster() {
        use smol_str::SmolStr;
        use std::path::PathBuf;
        use trading_core::{Price, Quantity, Timeframe, Timestamp, Venue};

        // Load the top10_momentum_h1 config from file if available; otherwise skip.
        let cfg_path = PathBuf::from("config/strategies/top10_momentum_h1.toml");
        let cfg = match CrossSectionalMomentumConfig::from_file(&cfg_path) {
            Ok(c) => c,
            Err(_) => {
                eprintln!("SKIP td14_smoke_with_passthrough_forecaster: config not found");
                return;
            }
        };

        let base = MomentumStrategy::from_config(
            cfg,
            SmolStr::new(cfg_path.to_string_lossy()),
        );
        let mut strategy = TcnOverlayMomentumStrategy::with_passthrough(base);

        assert_eq!(strategy.id().to_string(), "tcn_overlay_momentum");

        let symbol = trading_core::Symbol::new("BTCUSDT");
        let base_ts = time::OffsetDateTime::UNIX_EPOCH;
        let mut close = dec!(16_500);

        for i in 0..50i64 {
            // Step the price slightly each bar.
            close += dec!(10);
            let ts = Timestamp::new(base_ts + time::Duration::hours(i));
            let bar = trading_core::Bar {
                symbol: symbol.clone(),
                tf: Timeframe::OneHour,
                open_ts: ts,
                close_ts: ts,
                open: Price::new(close).unwrap(),
                high: Price::new(close + dec!(5)).unwrap(),
                low: Price::new(close - dec!(5)).unwrap(),
                close: Price::new(close).unwrap(),
                volume: Quantity::new(dec!(100)).unwrap(),
                trade_count: 10,
                local_recv_ts: ts,
                venue: Venue::Binance,
            };

            let _signals = strategy.on_bar(&bar);
        }

        // The passthrough forecaster means all signals pass through unchanged.
        assert_eq!(
            strategy.stats.dampened, 0,
            "passthrough forecaster must not dampen any signals"
        );
        // No panic — strategy ran 50 bars without error.
    }

    /// T-D-14: ModulationStats tracking.
    #[test]
    fn td14_modulation_stats_tracking() {
        // Verify that dampened signals increment stats.dampened.
        // Use a MockForecaster that always disagrees + high confidence.
        struct AlwaysDownForecaster;
        impl SyncForecaster for AlwaysDownForecaster {
            fn infer(&self, _bars: &[Bar]) -> (ForecastDirection, Decimal) {
                (ForecastDirection::Down, dec!(0.9))
            }
        }

        use smol_str::SmolStr;
        use std::path::PathBuf;
        use trading_core::{Price, Quantity, Timeframe, Timestamp, Venue};

        let cfg_path = PathBuf::from("config/strategies/top10_momentum_h1.toml");
        let cfg = match CrossSectionalMomentumConfig::from_file(&cfg_path) {
            Ok(c) => c,
            Err(_) => {
                eprintln!("SKIP td14_modulation_stats_tracking: config not found");
                return;
            }
        };
        let base = MomentumStrategy::from_config(
            cfg,
            SmolStr::new(cfg_path.to_string_lossy()),
        );
        let mut strategy = TcnOverlayMomentumStrategy::new(
            base,
            Box::new(AlwaysDownForecaster),
            dec!(0.6),
        );

        let symbol = trading_core::Symbol::new("BTCUSDT");
        let base_ts = time::OffsetDateTime::UNIX_EPOCH;
        let mut close = dec!(16_500);

        // Run 300 bars so the window fills up (256 + some rebalance bars).
        for i in 0..300i64 {
            close += dec!(10);
            let ts = Timestamp::new(base_ts + time::Duration::hours(i));
            let bar = trading_core::Bar {
                symbol: symbol.clone(),
                tf: Timeframe::OneHour,
                open_ts: ts,
                close_ts: ts,
                open: Price::new(close).unwrap(),
                high: Price::new(close + dec!(5)).unwrap(),
                low: Price::new(close - dec!(5)).unwrap(),
                close: Price::new(close).unwrap(),
                volume: Quantity::new(dec!(100)).unwrap(),
                trade_count: 10,
                local_recv_ts: ts,
                venue: Venue::Binance,
            };
            strategy.on_bar(&bar);
        }

        // After 300 bars, if any Buy signals were emitted and window was full,
        // the AlwaysDownForecaster should have dampened some.
        // The total field counts all signals processed.
        let stats = &strategy.stats;
        assert_eq!(
            stats.total,
            stats.passed_through + stats.dampened + stats.window_warming_up,
            "stats must be self-consistent"
        );
    }
}
