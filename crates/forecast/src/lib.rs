//! `crates/forecast` — model-agnostic `ForecastProvider` trait + overlay logic.
//!
//! ## Overview
//!
//! This crate provides:
//!
//! - [`ForecastProvider`]: the async trait all forecast backends implement.
//!   Shape mirrors `LlmProvider` but narrower (one method, no streaming,
//!   no tool-use).
//! - [`overlay`]: pure-function combine logic for fusing a base-strategy
//!   signal with a forecast overlay.
//!
//! The concrete forecaster implementation lands per-feature. v2.5 targets
//! a small custom Transformer/TCN trained in `candle` (the project's named
//! prototyping framework per CLAUDE.md); the trait + overlay scaffolding
//! here are intentionally model-agnostic.
//!
//! Cross-references:
//! - `spec/architecture/12-forecast-overlay.md` — overlay design pattern.
//! - `spec/v25-dl-forecast-overlay/feature.md` — current v2.5 brief.

use async_trait::async_trait;
use trading_core::forecast::{ForecastError, ForecastRequest, ForecastResponse};

pub mod features;
pub mod overlay;
pub mod provenance;
#[cfg(feature = "candle")]
pub mod tcn;

/// The async trait all forecast backends implement.
///
/// Intentionally narrower than `LlmProvider`: one method, one request,
/// one response, one error. No tool-use, no streaming, no prompt cache.
///
/// Implementations MUST be `Send + Sync` for use in multi-threaded
/// async runtimes (tokio multi-thread).
///
/// In research (replay) mode, implementations MUST return
/// [`ForecastError::ReplayMiss`] on a cache miss — no live fallthrough.
#[async_trait]
pub trait ForecastProvider: Send + Sync {
    /// Issue a forecast request and return a response.
    ///
    /// # Errors
    ///
    /// See [`ForecastError`] for all failure modes. In strict-replay
    /// (backtest) mode, a cache miss returns `ForecastError::ReplayMiss`.
    async fn forecast(
        &self,
        request: ForecastRequest,
    ) -> Result<ForecastResponse, ForecastError>;
}

/// Re-export core forecast types for consumers of this crate.
pub use trading_core::forecast::{
    Direction, ForecastOverlay, ForecastRequest as Request, ForecastResponse as Response,
    OhlcvBar, SamplingParams,
};

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use time::OffsetDateTime;
    use uuid::Uuid;

    use trading_core::forecast::{Direction, ForecastOverlay, OhlcvBar, SamplingParams};

    /// A minimal mock `ForecastProvider` for unit tests.
    struct MockForecaster {
        response: ForecastResponse,
    }

    #[async_trait]
    impl ForecastProvider for MockForecaster {
        async fn forecast(&self, req: ForecastRequest) -> Result<ForecastResponse, ForecastError> {
            let mut resp = self.response.clone();
            resp.correlation_id = req.correlation_id;
            Ok(resp)
        }
    }

    fn sample_overlay() -> ForecastOverlay {
        ForecastOverlay {
            correlation_id: Uuid::nil(),
            confidence: Decimal::new(75, 2), // 0.75
            direction: Direction::Up,
            horizon_bars: 1,
            model_revision: "stub-v0".to_string(),
            sampled_at: OffsetDateTime::UNIX_EPOCH,
        }
    }

    fn sample_bar() -> OhlcvBar {
        OhlcvBar {
            open: Decimal::new(100, 0),
            high: Decimal::new(105, 0),
            low: Decimal::new(99, 0),
            close: Decimal::new(103, 0),
            volume: Decimal::new(1000, 0),
            ts: OffsetDateTime::UNIX_EPOCH,
        }
    }

    fn sample_request() -> ForecastRequest {
        ForecastRequest {
            model_revision: "stub-v0".to_string(),
            ohlcv_window: vec![sample_bar()],
            sampling: SamplingParams::default(),
            correlation_id: Uuid::nil(),
        }
    }

    fn sample_response() -> ForecastResponse {
        ForecastResponse {
            correlation_id: Uuid::nil(),
            model_revision: "stub-v0".to_string(),
            overlay: sample_overlay(),
            samples: vec![sample_bar()],
        }
    }

    /// T-M1-3: ForecastProvider trait compiles + mock impl works.
    #[tokio::test]
    async fn forecast_provider_mock_impl_compiles() {
        let provider = MockForecaster {
            response: sample_response(),
        };
        let req = sample_request();
        let resp = provider.forecast(req).await.expect("mock should succeed");
        assert_eq!(resp.overlay.direction, Direction::Up);
        assert_eq!(resp.overlay.horizon_bars, 1);
    }

    /// T-M1-3: ForecastProvider is object-safe (can be boxed as dyn trait).
    #[tokio::test]
    async fn forecast_provider_is_object_safe() {
        let provider: Box<dyn ForecastProvider> = Box::new(MockForecaster {
            response: sample_response(),
        });
        let req = sample_request();
        let resp = provider.forecast(req).await.expect("boxed trait object works");
        assert_eq!(resp.overlay.direction, Direction::Up);
    }

    /// T-M1-3: ForecastProvider::forecast returns ReplayMiss when appropriate.
    #[tokio::test]
    async fn forecast_provider_replay_miss_variant() {
        struct StrictMissProvider;

        #[async_trait]
        impl ForecastProvider for StrictMissProvider {
            async fn forecast(
                &self,
                _req: ForecastRequest,
            ) -> Result<ForecastResponse, ForecastError> {
                Err(ForecastError::ReplayMiss {
                    hash: "deadbeef".to_string(),
                })
            }
        }

        let provider: Box<dyn ForecastProvider> = Box::new(StrictMissProvider);
        let req = sample_request();
        let err = provider.forecast(req).await.unwrap_err();
        assert!(
            matches!(err, ForecastError::ReplayMiss { .. }),
            "expected ReplayMiss, got {err:?}"
        );
    }
}
