//! Forecast overlay domain types (v2.5 — Kronos).
//!
//! These types live in `crates/core` next to `Signal` because they are
//! domain primitives crossed by every consumer: `crates/forecast/`,
//! `crates/strategy/`, and the audit ledger.
//!
//! ## Design
//!
//! - [`ForecastOverlay`] is the wire format between a `ForecastProvider`
//!   and a consuming `Strategy` impl. It is deliberately small and
//!   serde-stable so it can land in audit rows and replay-cache values
//!   without dragging the full sampled OHLCV distribution into the journal.
//! - [`Direction`] is a three-way enum: `Up` / `Down` / `Flat`.
//! - [`ForecastRequest`] carries the OHLCV window and sampling parameters.
//! - [`ForecastResponse`] carries the forecast distribution + summary.
//! - [`ForecastError`] enumerates failure modes (no `RateLimited` — local
//!   inference; no `Network`/`Auth` — no HTTP hop).
//!
//! Cross-references:
//! - [`spec/architecture/12-forecast-overlay.md`](../../../spec/architecture/12-forecast-overlay.md)
//! - [`spec/v25-kronos-forecast-overlay/feature.md`](../../../spec/v25-kronos-forecast-overlay/feature.md)

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

// ── Direction ─────────────────────────────────────────────────────────────────

/// Forecast direction: the model's predicted net movement for the next bar.
///
/// `Flat` means the confidence is below the strategy's threshold or the
/// model's distribution is bimodal/uncertain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Model predicts upward price movement for the next bar.
    Up,
    /// Model predicts downward price movement for the next bar.
    Down,
    /// Model predicts flat / uncertain movement; overlay pass-through.
    Flat,
}

// ── ForecastOverlay ───────────────────────────────────────────────────────────

/// Overlay value type: the summary a `ForecastProvider` emits for a
/// consuming strategy.
///
/// This is the small, audit-row-safe summary. The full sampled distribution
/// lives in the replay-cache row keyed by the same `correlation_id`'s
/// request hash.
///
/// All numeric fields use [`Decimal`] per ADR-0003 — no `f64`.
/// `sampled_at` uses 6-digit fractional seconds per ADR-0004.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForecastOverlay {
    /// Ties this overlay to the corresponding audit row + trade close.
    pub correlation_id: Uuid,
    /// Model confidence score in `[0, 1]`.
    pub confidence: Decimal,
    /// Predicted direction for the next bar.
    pub direction: Direction,
    /// Number of bars the forecast covers. `1` at v2.5 (single-bar only).
    pub horizon_bars: u32,
    /// Hugging Face revision SHA pin of the model that produced this
    /// forecast.
    pub model_revision: String,
    /// Timestamp at which the forecast was sampled (6-digit fractional
    /// seconds).
    #[serde(with = "time::serde::rfc3339")]
    pub sampled_at: OffsetDateTime,
}

// ── ForecastRequest ───────────────────────────────────────────────────────────

/// A single OHLCV bar for inclusion in a forecast window.
///
/// All price/volume fields are `Decimal` (no `f64`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OhlcvBar {
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    /// Bar close timestamp.
    #[serde(with = "time::serde::rfc3339")]
    pub ts: OffsetDateTime,
}

/// Sampling parameters for the Kronos forecaster.
///
/// All parameters are part of the canonical-JSON cache key
/// (including `sampling_seed`) per ADR-0027 Q6.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SamplingParams {
    /// Softmax temperature. `1.0` = unscaled; `0.0` = argmax.
    pub temperature: Decimal,
    /// Nucleus (top-p) sampling cutoff. `1.0` = no nucleus filter.
    pub top_p: Decimal,
    /// Top-k cutoff. `0` = disabled.
    pub top_k: u32,
    /// Maximum tokens to sample.
    pub max_tokens: u32,
    /// Explicit seed for `ChaCha20Rng` — part of the cache key so two
    /// seeds produce two independent deterministic forecasts.
    pub sampling_seed: u64,
}

impl Default for SamplingParams {
    fn default() -> Self {
        Self {
            temperature: Decimal::ONE,
            top_p: Decimal::ONE,
            top_k: 0,
            max_tokens: 128,
            // Default seed `0xC0FFEE` matches the project-wide fixture seed
            // (ADR-0002 ChaCha20Rng convention).
            sampling_seed: 0x00C0_FFEE,
        }
    }
}

/// A forecast request: OHLCV window + sampling parameters + model pin.
///
/// The full set of fields is hashed (minus `correlation_id`) to produce the
/// replay-cache key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForecastRequest {
    /// Hugging Face revision SHA that identifies the model checkpoint.
    pub model_revision: String,
    /// The OHLCV bars to feed as the context window.
    pub ohlcv_window: Vec<OhlcvBar>,
    /// Sampling configuration.
    pub sampling: SamplingParams,
    /// Per-call correlation id (excluded from the cache key — same request
    /// at different times shares a cache entry).
    pub correlation_id: Uuid,
}

// ── ForecastResponse ──────────────────────────────────────────────────────────

/// The model's forecast for the next bar, including distribution samples.
///
/// The full `samples` array is stored in the replay-cache. The summary
/// `overlay` is what gets posted to the audit ledger and consumed by the
/// `Strategy::tick()` composition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForecastResponse {
    /// Correlation id echoed from the request — ties forecast to audit row.
    pub correlation_id: Uuid,
    /// The model revision that produced this response (echoed from request).
    pub model_revision: String,
    /// Summary overlay for the consuming strategy.
    pub overlay: ForecastOverlay,
    /// Raw sampled OHLCV bars (the full distribution). Stored in the
    /// replay-cache; not posted to the audit ledger in v2.5.
    pub samples: Vec<OhlcvBar>,
}

// ── ForecastError ─────────────────────────────────────────────────────────────

/// Error variants for [`ForecastProvider::forecast`].
///
/// Mirrors the v2 [`LlmError`](crates::llm::error::LlmError) pattern but
/// scoped to local inference: no `RateLimited`, no `Network`, no `Auth`.
#[derive(Debug, thiserror::Error)]
pub enum ForecastError {
    /// The underlying provider (e.g. `tract` runner) failed.
    #[error("forecast provider error: {0}")]
    Provider(String),

    /// Inference exceeded the time limit.
    #[error("forecast timeout: {0}")]
    Timeout(String),

    /// The input OHLCV window was invalid (e.g. empty, wrong type).
    #[error("forecast invalid input: {0}")]
    InvalidInput(String),

    /// The model produced an output that could not be decoded.
    #[error("forecast invalid response: {0}")]
    InvalidResponse(String),

    /// Strict-replay mode: no cache entry for the given request hash.
    #[error("forecast replay miss: hash={hash}")]
    ReplayMiss { hash: String },

    /// `tract` inference error (op not supported, tensor shape mismatch, etc).
    #[error("forecast inference error: {0}")]
    Inference(String),

    /// The energy-cost budget for this call was exceeded (future gate;
    /// not triggered at default `energy_cost_per_kwh = 0`).
    #[error("forecast budget exceeded: {0}")]
    BudgetExceeded(String),
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;

    fn sample_overlay() -> ForecastOverlay {
        ForecastOverlay {
            correlation_id: Uuid::nil(),
            confidence: dec!(0.75),
            direction: Direction::Up,
            horizon_bars: 1,
            model_revision: "abc123".to_string(),
            sampled_at: OffsetDateTime::UNIX_EPOCH,
        }
    }

    fn sample_bar() -> OhlcvBar {
        OhlcvBar {
            open: dec!(100.0),
            high: dec!(105.0),
            low: dec!(99.0),
            close: dec!(103.0),
            volume: dec!(1000.0),
            ts: OffsetDateTime::UNIX_EPOCH,
        }
    }

    /// T-M1-2 (a): ForecastOverlay serde round-trip.
    #[test]
    fn forecast_overlay_serde_roundtrip() {
        let overlay = sample_overlay();
        let json = serde_json::to_string(&overlay).expect("serialize");
        let back: ForecastOverlay = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, overlay);
    }

    /// T-M1-2 (b): Direction serde round-trip for all variants.
    #[test]
    fn direction_serde_all_variants() {
        for dir in [Direction::Up, Direction::Down, Direction::Flat] {
            let json = serde_json::to_string(&dir).unwrap();
            let back: Direction = serde_json::from_str(&json).unwrap();
            assert_eq!(back, dir, "Direction round-trip failed for {dir:?}");
        }
    }

    /// T-M1-2 (c): ForecastRequest serde round-trip.
    #[test]
    fn forecast_request_serde_roundtrip() {
        let req = ForecastRequest {
            model_revision: "sha256:deadbeef".to_string(),
            ohlcv_window: vec![sample_bar()],
            sampling: SamplingParams::default(),
            correlation_id: Uuid::nil(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: ForecastRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back, req);
    }

    /// T-M1-2 (d): ForecastResponse serde round-trip.
    #[test]
    fn forecast_response_serde_roundtrip() {
        let resp = ForecastResponse {
            correlation_id: Uuid::nil(),
            model_revision: "sha256:deadbeef".to_string(),
            overlay: sample_overlay(),
            samples: vec![sample_bar()],
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: ForecastResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(back, resp);
    }

    /// T-M1-2 (e): confidence field uses Decimal, not f64.
    /// Verify precision is preserved (no float rounding).
    #[test]
    fn confidence_decimal_precision_preserved() {
        let mut overlay = sample_overlay();
        // Set to a value that f64 cannot represent exactly.
        overlay.confidence = dec!(0.333333333333333333333333);
        let json = serde_json::to_string(&overlay).unwrap();
        let back: ForecastOverlay = serde_json::from_str(&json).unwrap();
        assert_eq!(back.confidence, overlay.confidence);
    }

    /// T-M1-2 (f): SamplingParams default seed matches project convention.
    #[test]
    fn sampling_params_default_seed_is_project_convention() {
        let params = SamplingParams::default();
        assert_eq!(params.sampling_seed, 0x00C0_FFEE);
    }

    /// T-M1-2 (g): ForecastError variants are Debug + Display.
    #[test]
    fn forecast_error_display() {
        let err = ForecastError::ReplayMiss {
            hash: "abc".to_string(),
        };
        let display = format!("{err}");
        assert!(display.contains("abc"), "Display should include hash");

        let err2 = ForecastError::Provider("tract failed".into());
        assert!(format!("{err2}").contains("tract failed"));
    }
}
