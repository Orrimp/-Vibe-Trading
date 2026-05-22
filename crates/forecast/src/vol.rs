//! `VolForecastProvider` trait + `VolRequest` / `VolResponse` types.
//!
//! ## Contract (ADR-0038 § D1.a)
//!
//! A **sibling** trait to [`crate::ForecastProvider`] — same `Send + Sync` async shape,
//! but for volatility forecasts rather than direction forecasts.
//! The two traits share no code path; the F-verdict and V-verdict algorithms
//! are entirely independent per Q4=(b) operator default.
//!
//! ## Usage pattern
//!
//! Implementations hold a loaded [`crate::garch::GarchModel`] per symbol and
//! run `forecast_step()` from cached state.  The trait is object-safe so
//! strategy-side consumers (`VolTargetingOverlay`, `VolKillSwitchOverlay`,
//! `VolMeanReversionStrategy`) hold `Arc<dyn VolForecastProvider>`.
//!
//! ## Cross-references
//!
//! - ADR-0038 § D1.a — per-symbol inputs shape.
//! - ADR-0038 § D3   — GARCH baseline contract.
//! - ADR-0038 § D4   — replay-cache namespace `"vol_forecast"`.
//! - `crates/forecast/src/garch.rs` — `GarchModel` struct.

use async_trait::async_trait;
use thiserror::Error;

// ── VolRequest ────────────────────────────────────────────────────────────────

/// Request a σ forecast for a single symbol at a given time step.
///
/// The caller supplies the most recent log-return `r_prev` and the previous
/// σ prediction `sigma_prev` (from the prior bar's output).  For the very
/// first call, callers should use the model's `unconditional_var.sqrt()` as
/// `sigma_prev`.
#[derive(Debug, Clone)]
pub struct VolRequest {
    /// Symbol being forecast (e.g. `"BTCUSDT"`).
    pub symbol: String,
    /// Log-return at the previous bar: `r_{t-1} = ln(close_{t-1} / close_{t-2})`.
    pub r_prev: f64,
    /// Previous σ prediction (not σ²): output of the prior `forecast_vol` call.
    ///
    /// On first call use `model.unconditional_var.sqrt()`.
    pub sigma_prev: f64,
    /// Horizon in bars (1 = one-step-ahead; caller scales for multi-step).
    ///
    /// The GARCH(1,1) recurrence returns σ for horizon 1; multi-step scaling
    /// is the caller's responsibility (e.g. `sigma * sqrt(H)` for i.i.d.
    /// approximation, or recursive recurrence for term-structure).
    pub horizon_bars: u32,
}

// ── VolResponse ───────────────────────────────────────────────────────────────

/// Response to a [`VolRequest`].
///
/// Contains the predicted σ for horizon 1 (not σ²).  Multi-step scaling
/// is left to the consumer (strategy layer vs. backtest layer differ here).
#[derive(Debug, Clone)]
pub struct VolResponse {
    /// Symbol this forecast applies to.
    pub symbol: String,
    /// Predicted σ for the requested horizon (σ, not σ²).
    ///
    /// Always positive (floored at ω per `GarchModel::forecast_step()`).
    pub sigma_hat: f64,
    /// Horizon this forecast covers (mirrors `VolRequest::horizon_bars`).
    pub horizon_bars: u32,
    /// Model revision string (e.g. `"garch-bs1-<sha>"`).
    pub model_revision: String,
}

// ── VolForecastError ──────────────────────────────────────────────────────────

/// Errors from the vol forecast provider.
#[derive(Debug, Error)]
pub enum VolForecastError {
    /// Strict-replay mode: cache miss for this (symbol, timestamp) key.
    #[error("replay miss: {hash}")]
    ReplayMiss { hash: String },

    /// Requested symbol is not in the loaded checkpoint.
    #[error("unknown symbol: {symbol}")]
    UnknownSymbol { symbol: String },

    /// I/O or parse error loading the GARCH checkpoint.
    #[error("checkpoint load error: {detail}")]
    CheckpointLoad { detail: String },

    /// Internal error (serialisation, unexpected state, etc.).
    #[error("internal error: {detail}")]
    Internal { detail: String },
}

// ── VolForecastProvider ───────────────────────────────────────────────────────

/// Async trait all vol-forecast backends implement.
///
/// Mirrors [`crate::ForecastProvider`] shape exactly:
/// - One method, one request, one response, one error.
/// - `Send + Sync` for multi-threaded async runtimes.
/// - In research (replay) mode, implementations MUST return
///   [`VolForecastError::ReplayMiss`] on a cache miss — no live fallthrough.
///
/// ## Object safety
///
/// The trait is object-safe; strategy consumers hold
/// `Arc<dyn VolForecastProvider>`.
#[async_trait]
pub trait VolForecastProvider: Send + Sync {
    /// Issue a vol-forecast request and return a response.
    ///
    /// # Errors
    ///
    /// See [`VolForecastError`] for all failure modes.  In strict-replay
    /// (backtest) mode, a cache miss returns
    /// [`VolForecastError::ReplayMiss`].
    async fn forecast_vol(&self, request: VolRequest) -> Result<VolResponse, VolForecastError>;
}

// ── GarchVolForecaster ────────────────────────────────────────────────────────

/// GARCH(1,1)-based `VolForecastProvider`.
///
/// Holds one fitted [`crate::garch::GarchModel`] per symbol, loaded from the
/// per-symbol JSON checkpoint (`garch-bs1-<sha>.json`, ADR-0038 § D3).
///
/// The `forecast_vol` call runs a single `GarchModel::forecast_step()` —
/// sub-microsecond per call (no heap allocation, no async overhead beyond the
/// trait machinery).
///
/// ## Thread safety
///
/// `GarchVolForecaster` is `Send + Sync` — it holds only immutable model
/// params + a read-only `Arc<HashMap>` of per-symbol models.
pub struct GarchVolForecaster {
    /// Per-symbol fitted models.  Keys are USDT-quote symbols (e.g. `"BTCUSDT"`).
    models: std::collections::HashMap<String, crate::garch::GarchModel>,
    /// Canonical model revision string embedded in every `VolResponse`.
    revision: String,
}

impl GarchVolForecaster {
    /// Construct from an existing per-symbol model map.
    ///
    /// `revision` should be the `checkpoint_revision` SHA-256 hex from the
    /// JSON checkpoint file (ADR-0038 § D3 aggregate SHA derivation).
    #[must_use]
    pub fn new(
        models: std::collections::HashMap<String, crate::garch::GarchModel>,
        revision: String,
    ) -> Self {
        Self { models, revision }
    }

    /// Return the model for a given symbol, if loaded.
    #[must_use]
    pub fn model(&self, symbol: &str) -> Option<&crate::garch::GarchModel> {
        self.models.get(symbol)
    }

    /// Return the checkpoint revision string.
    #[must_use]
    pub fn revision(&self) -> &str {
        &self.revision
    }

    /// All symbol names loaded into this forecaster, sorted alphabetically.
    #[must_use]
    pub fn symbols(&self) -> Vec<&str> {
        let mut keys: Vec<&str> = self.models.keys().map(String::as_str).collect();
        keys.sort_unstable();
        keys
    }
}

#[async_trait]
impl VolForecastProvider for GarchVolForecaster {
    async fn forecast_vol(&self, request: VolRequest) -> Result<VolResponse, VolForecastError> {
        let model =
            self.models
                .get(&request.symbol)
                .ok_or_else(|| VolForecastError::UnknownSymbol {
                    symbol: request.symbol.clone(),
                })?;

        let sigma_hat = model.forecast_step(request.r_prev, request.sigma_prev);

        Ok(VolResponse {
            symbol: request.symbol,
            sigma_hat,
            horizon_bars: request.horizon_bars,
            model_revision: self.revision.clone(),
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::garch::GarchModel;

    fn stub_model() -> GarchModel {
        GarchModel {
            omega: 1e-6,
            alpha: 0.10,
            beta: 0.85,
            unconditional_var: 1e-6 / (1.0 - 0.10 - 0.85),
            log_likelihood: 0.0,
            n_iters: 1,
            converged: true,
        }
    }

    #[tokio::test]
    async fn vol_forecast_provider_mock_impl_compiles() {
        let models = [("BTCUSDT".to_string(), stub_model())]
            .into_iter()
            .collect();
        let forecaster = GarchVolForecaster::new(models, "test-rev".to_string());

        let req = VolRequest {
            symbol: "BTCUSDT".to_string(),
            r_prev: 0.01,
            sigma_prev: 0.005,
            horizon_bars: 1,
        };
        let resp = forecaster.forecast_vol(req).await.expect("must succeed");
        assert_eq!(resp.symbol, "BTCUSDT");
        assert!(resp.sigma_hat > 0.0);
        assert_eq!(resp.horizon_bars, 1);
        assert_eq!(resp.model_revision, "test-rev");
    }

    #[tokio::test]
    async fn vol_forecast_unknown_symbol_error() {
        let models = std::collections::HashMap::new();
        let forecaster = GarchVolForecaster::new(models, "test-rev".to_string());

        let req = VolRequest {
            symbol: "UNKNOWN".to_string(),
            r_prev: 0.0,
            sigma_prev: 0.0,
            horizon_bars: 1,
        };
        let err = forecaster.forecast_vol(req).await.unwrap_err();
        assert!(
            matches!(err, VolForecastError::UnknownSymbol { .. }),
            "expected UnknownSymbol, got {err:?}"
        );
    }

    #[tokio::test]
    async fn vol_forecast_provider_is_object_safe() {
        let models = [("ETHUSDT".to_string(), stub_model())]
            .into_iter()
            .collect();
        let forecaster: Box<dyn VolForecastProvider> =
            Box::new(GarchVolForecaster::new(models, "v0".to_string()));

        let req = VolRequest {
            symbol: "ETHUSDT".to_string(),
            r_prev: -0.005,
            sigma_prev: 0.008,
            horizon_bars: 1,
        };
        let resp = forecaster
            .forecast_vol(req)
            .await
            .expect("boxed trait works");
        assert!(resp.sigma_hat > 0.0);
    }

    #[test]
    fn garch_vol_forecaster_symbols_sorted() {
        let models = [
            ("XRPUSDT".to_string(), stub_model()),
            ("BTCUSDT".to_string(), stub_model()),
            ("ADAUSDT".to_string(), stub_model()),
        ]
        .into_iter()
        .collect();
        let forecaster = GarchVolForecaster::new(models, "rev".to_string());
        let syms = forecaster.symbols();
        assert_eq!(syms, ["ADAUSDT", "BTCUSDT", "XRPUSDT"]);
    }
}
