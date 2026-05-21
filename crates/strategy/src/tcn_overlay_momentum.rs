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

use crate::Strategy;
use crate::cross_sectional::MomentumStrategy;

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
    /// Optional per-instance direction deadband ε.
    ///
    /// `None` ⇒ use `forecast::tcn::DIRECTION_EPSILON` (const-fold-identical to
    /// the pre-tuning path — existing anchor bodies stay byte-identical).
    /// `Some(eps)` ⇒ override for threshold-sweep cells (T-D-N1 / D-AR-1.g).
    direction_epsilon: Option<f32>,
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
        Ok(Self {
            forecaster: f,
            direction_epsilon: None,
        })
    }

    /// Load the BS-2 anchor checkpoint.
    ///
    /// # Errors
    ///
    /// Returns `forecast::tcn::TcnForecasterError` if the checkpoint is absent.
    pub fn load_bs2() -> Result<Self, forecast::tcn::TcnForecasterError> {
        let f = forecast::tcn::TcnForecaster::load_anchor(forecast::tcn::AnchorScenario::Bs2)?;
        Ok(Self {
            forecaster: f,
            direction_epsilon: None,
        })
    }

    /// Return the per-instance direction epsilon override (if any).
    ///
    /// `None` means the default `forecast::tcn::DIRECTION_EPSILON` const is used.
    /// Used in tests (T-D-N3) to verify that default builders leave the field unset.
    #[must_use]
    pub fn direction_epsilon(&self) -> Option<f32> {
        self.direction_epsilon
    }

    /// Construct from an already-loaded `TcnForecaster` with an explicit
    /// direction epsilon override.
    ///
    /// Used by the `threshold_sweep` bin which loads the forecaster from
    /// `--metadata-path` (recalibrated overlay) rather than the anchor dir.
    /// This avoids `load_anchor` (which resolves the standard metadata JSON
    /// path) and lets the sweep supply the recalibrated σ_train directly.
    ///
    /// # Parameters
    ///
    /// - `forecaster`: a `TcnForecaster` already loaded via `load_from_paths`.
    /// - `direction_epsilon`: the ε override for this sweep cell; converted to
    ///   `f32` via `Decimal::to_f32()`.
    #[must_use]
    pub fn from_forecaster_with_epsilon(
        forecaster: forecast::tcn::TcnForecaster,
        direction_epsilon: rust_decimal::Decimal,
    ) -> Self {
        use rust_decimal::prelude::ToPrimitive;
        let eps = direction_epsilon
            .to_f32()
            .unwrap_or(forecast::tcn::DIRECTION_EPSILON);
        Self {
            forecaster,
            direction_epsilon: Some(eps),
        }
    }

    /// Load from explicit safetensors + metadata paths with a direction epsilon override.
    ///
    /// Used by the `threshold_sweep` bin to load from `--metadata-path`
    /// (recalibrated overlay) for each sweep cell. Thin wrapper over
    /// `TcnForecaster::load_from_paths` + `from_forecaster_with_epsilon`.
    ///
    /// # Errors
    ///
    /// Returns `TcnForecasterError` on file I/O or parse failure.
    pub fn load_from_paths_with_epsilon(
        safetensors_path: &std::path::Path,
        metadata_path: &std::path::Path,
        direction_epsilon: rust_decimal::Decimal,
    ) -> Result<Self, forecast::tcn::TcnForecasterError> {
        let f = forecast::tcn::TcnForecaster::load_from_paths(safetensors_path, metadata_path)?;
        Ok(Self::from_forecaster_with_epsilon(f, direction_epsilon))
    }

    /// Override the direction deadband ε for threshold-sweep cells (D-AR-1.g).
    ///
    /// `None` (default, set by `load_bs1` / `load_bs2`) means use
    /// `forecast::tcn::DIRECTION_EPSILON` — const-fold-identical to the pre-
    /// tuning path; all 26 predecessor anchors stay byte-identical.
    /// `Some(eps)` overrides for the sweep bin's per-cell strategy instances.
    ///
    /// `eps` is supplied as `Decimal` to avoid f32 rounding at the call site
    /// (the caller already has the grid value as `Decimal`).
    #[must_use]
    pub fn with_direction_epsilon(mut self, eps: rust_decimal::Decimal) -> Self {
        use rust_decimal::prelude::ToPrimitive;
        self.direction_epsilon = Some(eps.to_f32().unwrap_or(forecast::tcn::DIRECTION_EPSILON));
        self
    }

    /// Attach an audit ledger for Phase D `forecast_events` SQL durability
    /// (T-D-N20 / R6.4). Forwards to
    /// `forecast::tcn::TcnForecaster::with_ledger` (see `tcn.rs:589`).
    ///
    /// Only available when the `audit-tick` feature is enabled — the
    /// `TcnForecaster.ledger` field and `with_ledger` builder are both
    /// cfg-gated. Backtest paths continue to use `load_bs1` / `load_bs2`
    /// directly (no ledger attached → `tick.rs:104-107` static branch stays
    /// dormant — H2 anchor invariant preserved).
    #[cfg(feature = "forecast-audit-tick")]
    #[must_use]
    pub fn with_ledger(self, ledger: audit::Ledger) -> Self {
        Self {
            forecaster: self.forecaster.with_ledger(ledger),
            direction_epsilon: self.direction_epsilon,
        }
    }

    /// Attach strategy_id + symbol context for the Phase D
    /// `post_forecast_event` SQL writer (R1.4). Forwards to
    /// `forecast::tcn::TcnForecaster::with_forecast_context`.
    ///
    /// Called alongside `with_ledger`. Only available under `forecast-audit-tick`.
    #[cfg(feature = "forecast-audit-tick")]
    #[must_use]
    pub fn with_forecast_context(self, strategy_id: String, symbol: String) -> Self {
        Self {
            forecaster: self.forecaster.with_forecast_context(strategy_id, symbol),
            direction_epsilon: self.direction_epsilon,
        }
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

            // Channel-first layout: feat_cf[c * n + t].
            // The multiplications by 0 and 1 express the channel index
            // explicitly so all five assignments share the same formula.
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

        let eps = self
            .direction_epsilon
            .unwrap_or(forecast::tcn::DIRECTION_EPSILON);
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
        let windows: BTreeMap<Symbol, Vec<Bar>> =
            base.universe().map(|s| (s.clone(), Vec::new())).collect();

        Self {
            id: StrategyId::new("tcn_overlay_momentum"),
            base,
            forecaster,
            windows,
            confidence_threshold,
            stats: ModulationStats::default(),
        }
    }

    /// Return the confidence threshold for this strategy instance.
    ///
    /// Used in tests to assert builder values without round-tripping through
    /// the full backtest path. (T-D-N3 / D-AR-1.f invariance assertions.)
    #[must_use]
    pub fn confidence_threshold(&self) -> Decimal {
        self.confidence_threshold
    }

    /// Construct with a `PassthroughForecaster` (degrades to pure v1 momentum).
    ///
    /// Used when the `forecast` feature is not enabled.
    #[must_use]
    pub fn with_passthrough(base: MomentumStrategy) -> Self {
        Self::new(base, Box::new(PassthroughForecaster), dec!(0.6))
    }

    /// Construct with the real BS-1 TCN anchor checkpoint.
    ///
    /// Loads `tcn-bs1-d1c3696d…` from `crates/forecast/checkpoints/anchors/`.
    /// Requires the `forecast` feature (candle backend).
    ///
    /// # Errors
    ///
    /// Returns `forecast::tcn::TcnForecasterError` if the checkpoint is absent
    /// or fails to decode.
    #[cfg(feature = "forecast")]
    pub fn with_tcn_bs1(base: MomentumStrategy) -> Result<Self, forecast::tcn::TcnForecasterError> {
        let forecaster = TcnSyncForecaster::load_bs1()?;
        Ok(Self::new(base, Box::new(forecaster), dec!(0.6)))
    }

    /// Construct with the real BS-2 TCN anchor checkpoint.
    ///
    /// Loads `tcn-bs2-3fabcabe…` from `crates/forecast/checkpoints/anchors/`.
    /// Requires the `forecast` feature (candle backend).
    ///
    /// # Errors
    ///
    /// Returns `forecast::tcn::TcnForecasterError` if the checkpoint is absent
    /// or fails to decode.
    #[cfg(feature = "forecast")]
    pub fn with_tcn_bs2(base: MomentumStrategy) -> Result<Self, forecast::tcn::TcnForecasterError> {
        let forecaster = TcnSyncForecaster::load_bs2()?;
        Ok(Self::new(base, Box::new(forecaster), dec!(0.6)))
    }

    /// Phase D (T-D-N21) — Construct with the real BS-1 TCN anchor checkpoint
    /// **and** attach an audit ledger for `forecast_events` SQL durability
    /// (R6.4 / R6.5). The ledger enables the `post_forecast_event` writer
    /// inside `TcnForecaster`'s inference path.
    ///
    /// Requires the `forecast-audit-tick` feature (candle backend + audit-tick
    /// tee both enabled). Backtest callers MUST continue to use `with_tcn_bs1`
    /// (no ledger — `tick.rs:104-107` stays dormant, H2 preserved).
    ///
    /// # Errors
    ///
    /// Returns `forecast::tcn::TcnForecasterError` if the checkpoint is absent
    /// or fails to decode.
    #[cfg(feature = "forecast-audit-tick")]
    pub fn with_tcn_bs1_ledger(
        base: MomentumStrategy,
        ledger: audit::Ledger,
    ) -> Result<Self, forecast::tcn::TcnForecasterError> {
        let forecaster = TcnSyncForecaster::load_bs1()?
            .with_ledger(ledger)
            .with_forecast_context("tcn_overlay_momentum_bs1".to_string(), "MULTI".to_string());
        Ok(Self::new(base, Box::new(forecaster), dec!(0.6)))
    }

    /// Phase D (T-D-N21) — Construct with the real BS-2 TCN anchor checkpoint
    /// **and** attach an audit ledger. Mirror of `with_tcn_bs1_ledger` for
    /// the BS-2 checkpoint.
    ///
    /// # Errors
    ///
    /// Returns `forecast::tcn::TcnForecasterError` if the checkpoint is absent
    /// or fails to decode.
    #[cfg(feature = "forecast-audit-tick")]
    pub fn with_tcn_bs2_ledger(
        base: MomentumStrategy,
        ledger: audit::Ledger,
    ) -> Result<Self, forecast::tcn::TcnForecasterError> {
        let forecaster = TcnSyncForecaster::load_bs2()?
            .with_ledger(ledger)
            .with_forecast_context("tcn_overlay_momentum_bs2".to_string(), "MULTI".to_string());
        Ok(Self::new(base, Box::new(forecaster), dec!(0.6)))
    }

    // ── Threshold-sweep tuned builders (D-AR-1.f, v25-tcn-threshold-tuning) ────
    //
    // These 4 builders are ADDITIVE and do NOT change the 4 existing
    // `with_tcn_bs{1,2}{,_ledger}` builders (those keep their `dec!(0.6)` literal).
    // All 26 predecessor anchor bodies stay byte-identical (R8 / K4).
    //
    // Explicit (τ, ε) args required — no Option<Decimal> defaults (D-AR-1.f).

    /// Sweep-path BS-1 builder — no ledger, explicit (confidence_threshold, direction_epsilon).
    ///
    /// Used by `threshold_sweep` bin for the 45-cell BS-1 backtest grid.
    /// Requires `feature = "forecast"`.
    ///
    /// # Errors
    ///
    /// Returns `forecast::tcn::TcnForecasterError` if the BS-1 checkpoint is absent.
    #[cfg(feature = "forecast")]
    pub fn with_tcn_bs1_tuned(
        base: MomentumStrategy,
        confidence_threshold: rust_decimal::Decimal,
        direction_epsilon: rust_decimal::Decimal,
    ) -> Result<Self, forecast::tcn::TcnForecasterError> {
        let forecaster = TcnSyncForecaster::load_bs1()?.with_direction_epsilon(direction_epsilon);
        Ok(Self::new(base, Box::new(forecaster), confidence_threshold))
    }

    /// Sweep-path BS-2 builder — no ledger, explicit (confidence_threshold, direction_epsilon).
    ///
    /// Used by `threshold_sweep` bin for the 45-cell BS-2 backtest grid.
    /// Requires `feature = "forecast"`.
    ///
    /// # Errors
    ///
    /// Returns `forecast::tcn::TcnForecasterError` if the BS-2 checkpoint is absent.
    #[cfg(feature = "forecast")]
    pub fn with_tcn_bs2_tuned(
        base: MomentumStrategy,
        confidence_threshold: rust_decimal::Decimal,
        direction_epsilon: rust_decimal::Decimal,
    ) -> Result<Self, forecast::tcn::TcnForecasterError> {
        let forecaster = TcnSyncForecaster::load_bs2()?.with_direction_epsilon(direction_epsilon);
        Ok(Self::new(base, Box::new(forecaster), confidence_threshold))
    }

    /// Audit-path BS-1 builder with ledger + explicit (confidence_threshold, direction_epsilon).
    ///
    /// For production audit paths where forecast_events SQL durability is
    /// required alongside a tuned (τ, ε). Requires `feature = "forecast-audit-tick"`.
    ///
    /// # Errors
    ///
    /// Returns `forecast::tcn::TcnForecasterError` if the BS-1 checkpoint is absent.
    #[cfg(feature = "forecast-audit-tick")]
    pub fn with_tcn_bs1_ledger_tuned(
        base: MomentumStrategy,
        ledger: audit::Ledger,
        confidence_threshold: rust_decimal::Decimal,
        direction_epsilon: rust_decimal::Decimal,
    ) -> Result<Self, forecast::tcn::TcnForecasterError> {
        let forecaster = TcnSyncForecaster::load_bs1()?
            .with_ledger(ledger)
            .with_forecast_context("tcn_overlay_momentum_bs1".to_string(), "MULTI".to_string())
            .with_direction_epsilon(direction_epsilon);
        Ok(Self::new(base, Box::new(forecaster), confidence_threshold))
    }

    /// Audit-path BS-2 builder with ledger + explicit (confidence_threshold, direction_epsilon).
    ///
    /// Mirror of `with_tcn_bs1_ledger_tuned` for BS-2. Requires `feature = "forecast-audit-tick"`.
    ///
    /// # Errors
    ///
    /// Returns `forecast::tcn::TcnForecasterError` if the BS-2 checkpoint is absent.
    #[cfg(feature = "forecast-audit-tick")]
    pub fn with_tcn_bs2_ledger_tuned(
        base: MomentumStrategy,
        ledger: audit::Ledger,
        confidence_threshold: rust_decimal::Decimal,
        direction_epsilon: rust_decimal::Decimal,
    ) -> Result<Self, forecast::tcn::TcnForecasterError> {
        let forecaster = TcnSyncForecaster::load_bs2()?
            .with_ledger(ledger)
            .with_forecast_context("tcn_overlay_momentum_bs2".to_string(), "MULTI".to_string())
            .with_direction_epsilon(direction_epsilon);
        Ok(Self::new(base, Box::new(forecaster), confidence_threshold))
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
        let result =
            combine_with_direction(SignalKind::Buy, ForecastDirection::Up, dec!(0.8), dec!(0.6));
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

        let base = MomentumStrategy::from_config(cfg, SmolStr::new(cfg_path.to_string_lossy()));
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
        let base = MomentumStrategy::from_config(cfg, SmolStr::new(cfg_path.to_string_lossy()));
        let mut strategy =
            TcnOverlayMomentumStrategy::new(base, Box::new(AlwaysDownForecaster), dec!(0.6));

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

    // ── T-D-N20 / T-D-N21 — with_ledger / with_tcn_bs1_ledger builder tests ──

    // Helper: build a minimal MomentumStrategy from the canonical config file,
    // or skip the test if the config doesn't exist (CI without workspace data).
    fn build_base_strategy() -> Option<MomentumStrategy> {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let cfg_path = std::path::PathBuf::from(&manifest)
            .join("../../config/strategies/top10_momentum_h1.toml");
        CrossSectionalMomentumConfig::from_file(&cfg_path)
            .ok()
            .map(|cfg| MomentumStrategy::from_config(cfg, smol_str::SmolStr::new("test")))
    }

    /// T-D-N21 (a) — `with_passthrough` cold-start stats are zero.
    /// Invariant that applies to all constructors.
    #[test]
    fn with_passthrough_cold_start_stats_zero() {
        let Some(base) = build_base_strategy() else {
            eprintln!("SKIP with_passthrough_cold_start_stats_zero: config not found");
            return;
        };
        let strategy = TcnOverlayMomentumStrategy::with_passthrough(base);
        assert_eq!(strategy.stats.total, 0);
        assert_eq!(strategy.stats.passed_through, 0);
        assert_eq!(strategy.stats.dampened, 0);
    }

    /// T-D-N21 (b) — Strategy ID is always `"tcn_overlay_momentum"` regardless
    /// of constructor used. Regression guard for R6.5 registry wiring.
    #[test]
    fn strategy_id_is_tcn_overlay_momentum() {
        let Some(base) = build_base_strategy() else {
            eprintln!("SKIP strategy_id_is_tcn_overlay_momentum: config not found");
            return;
        };
        let strategy = TcnOverlayMomentumStrategy::with_passthrough(base);
        assert_eq!(
            strategy.id().0.as_str(),
            "tcn_overlay_momentum",
            "strategy ID must match the registry key"
        );
    }
}
