//! `KronosForecaster` — stub implementation of [`ForecastProvider`].
//!
//! ## Milestone status
//!
//! - **M1 (T-M1-3)**: Stub only. The struct compiles and implements
//!   `ForecastProvider`, returning `ForecastError::Inference` immediately.
//! - **M2 (T-M2-*)**: ONNX vendoring + checksum gate.
//! - **M3 (T-M3-*)**: `tract` integration — actual model loading + forward
//!   pass. `KronosForecaster::new()` will load the ONNX at construction time.
//!
//! Do NOT add any `tract` dependency or model-loading code here before the M3
//! milestone lands. The stub intentionally keeps the M1 compile surface small.

use async_trait::async_trait;
use tracing::warn;
use trading_core::forecast::{ForecastError, ForecastRequest, ForecastResponse};

use crate::ForecastProvider;

/// Configuration for the Kronos forecaster.
///
/// Model revision is pinned as a Hugging Face SHA per ADR-0027 Q3 /
/// feature.md R1.2. Sampling defaults come from [`SamplingParams::default`].
#[derive(Debug, Clone)]
pub struct KronosConfig {
    /// Hugging Face revision SHA for the `NeoQuasar/Kronos-base` checkpoint.
    /// Example: `"main"` (for testing) or a full SHA like
    /// `"a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0"`.
    pub model_revision: String,

    /// Path to the ONNX checkpoint file (relative to workspace root or
    /// absolute). Defaults to `crates/forecast/assets/kronos-base.onnx`.
    pub onnx_path: std::path::PathBuf,

    /// Confidence threshold below which the overlay direction is treated
    /// as `Flat` (pass-through). Default: `0.6`.
    pub overlay_confidence_threshold: rust_decimal::Decimal,

    /// kWh energy cost per forecast call. Default: `0` (no energy billing).
    /// Operators who want non-zero energy accounting set this via config;
    /// fixture backtest reports stay byte-identical at the default.
    pub energy_cost_per_kwh: rust_decimal::Decimal,
}

impl Default for KronosConfig {
    fn default() -> Self {
        Self {
            model_revision: "main".to_string(),
            onnx_path: std::path::PathBuf::from("crates/forecast/assets/kronos-base.onnx"),
            overlay_confidence_threshold: rust_decimal::Decimal::new(6, 1), // 0.6
            energy_cost_per_kwh: rust_decimal::Decimal::ZERO,
        }
    }
}

/// Kronos ONNX-backed forecast provider.
///
/// **Stub at M1.** Call [`KronosForecaster::new_stub`] to construct a
/// no-op instance for testing. `M3` will add `new(config)` with actual
/// `tract` model loading.
#[derive(Debug)]
pub struct KronosForecaster {
    config: KronosConfig,
}

impl KronosForecaster {
    /// Construct a stub `KronosForecaster` for tests and scaffolding.
    ///
    /// Returns `Inference` error on every `forecast()` call until M3
    /// lands the actual model loader.
    #[must_use]
    pub fn new_stub(config: KronosConfig) -> Self {
        Self { config }
    }

    /// Expose the config for inspection (used in tests).
    #[must_use]
    pub fn config(&self) -> &KronosConfig {
        &self.config
    }
}

#[async_trait]
impl ForecastProvider for KronosForecaster {
    async fn forecast(&self, req: ForecastRequest) -> Result<ForecastResponse, ForecastError> {
        // STUB — M3 will replace this with actual tract inference.
        // Log a warning so anyone hitting this in a non-test context knows it's unimplemented.
        warn!(
            target: "forecast.kronos",
            model_revision = %self.config.model_revision,
            window_bars = %req.ohlcv_window.len(),
            "KronosForecaster is a stub — M3 tract integration not yet implemented"
        );
        Err(ForecastError::Inference(
            "KronosForecaster stub: tract model not loaded (M3 pending)".to_string(),
        ))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use trading_core::forecast::{OhlcvBar, SamplingParams};
    use rust_decimal::Decimal;
    use time::OffsetDateTime;
    use uuid::Uuid;

    fn sample_request() -> ForecastRequest {
        ForecastRequest {
            model_revision: "stub-v0".to_string(),
            ohlcv_window: vec![OhlcvBar {
                open: Decimal::new(100, 0),
                high: Decimal::new(105, 0),
                low: Decimal::new(99, 0),
                close: Decimal::new(103, 0),
                volume: Decimal::new(1000, 0),
                ts: OffsetDateTime::UNIX_EPOCH,
            }],
            sampling: SamplingParams::default(),
            correlation_id: Uuid::nil(),
        }
    }

    /// M1 stub returns Inference error.
    #[tokio::test]
    async fn stub_returns_inference_error() {
        let provider = KronosForecaster::new_stub(KronosConfig::default());
        let req = sample_request();
        let err = provider.forecast(req).await.unwrap_err();
        assert!(
            matches!(err, ForecastError::Inference(_)),
            "stub must return Inference error, got {err:?}"
        );
    }

    /// KronosConfig default values match spec.
    #[test]
    fn kronos_config_defaults() {
        let cfg = KronosConfig::default();
        assert_eq!(cfg.overlay_confidence_threshold, Decimal::new(6, 1));
        assert_eq!(cfg.energy_cost_per_kwh, Decimal::ZERO);
        assert_eq!(cfg.model_revision, "main");
    }

    /// KronosForecaster implements ForecastProvider (object-safe).
    #[test]
    fn kronos_is_dyn_compatible() {
        let _provider: &dyn ForecastProvider = &KronosForecaster::new_stub(KronosConfig::default());
    }
}
