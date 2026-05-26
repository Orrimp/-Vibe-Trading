//! `propose_forecast` tool schema — the structured output contract for
//! the LLM-forecaster (T-D-N(B3)).
//!
//! ## Design
//!
//! The `propose_forecast` tool is the ONLY tool advertised to the LLM.
//! Requiring the LLM to call this tool enforces structured output:
//! - A 5-tier `rating` enum (`STRONG_BUY | BUY | HOLD | SELL | STRONG_SELL`).
//! - A `confidence` float in `[0, 1]`.
//! - A `horizon` string (always `"short"` at v0.1.0).
//! - A `reasoning_trace` string (`50`–`2000` chars; mechanical L4 gate per
//!   ADR-0039 § D1.b `short_frac > 0.50`).
//! - An optional `cited_lesson_ids` array of lesson card ID strings.
//!
//! Free-form text responses (without a tool call) are caught by
//! `LlmForecasterImpl::decode_response` → `LlmForecasterError::InvalidResponse`.
//!
//! ## JSON Schema
//!
//! The schema is Draft 2020-12 compatible and is passed verbatim to
//! [`llm::ToolSchema::input_schema`]. The `validate_tool_use` function in
//! `crates/llm/src/tools.rs` validates every response against this schema
//! before `LlmForecasterImpl` decodes the typed fields.
//!
//! ## Cross-references
//!
//! - `spec/v3-llm-forecaster/decomp.md § T-AR-1` — the 5-tier rating + confidence
//!   + reasoning trace fields.
//! - `spec/v3-llm-forecaster/decomp.md § T-AR-2` — the `propose_forecast` contract.
//! - `spec/architecture/adr/0039-llm-forecaster-verdict-criteria.md § D1.b` — L4
//!   `short_frac`/`duplicate_frac` thresholds depend on `minLength: 50`.

use llm::ToolSchema;

/// Name of the tool the LLM must call to emit its forecast.
pub const PROPOSE_FORECAST_TOOL_NAME: &str = "propose_forecast";

/// Build the `propose_forecast` [`ToolSchema`] definition.
///
/// This is the single tool advertised in every `ChatRequest` sent by
/// `LlmForecasterImpl`. The LLM **must** call this tool; any response
/// that does not contain a `ToolUse` block with this name is an
/// `LlmForecasterError::InvalidResponse`.
///
/// ## Schema fields
///
/// | Field              | Type               | Constraints                              |
/// |--------------------|--------------------|------------------------------------------|
/// | `rating`           | string enum        | one of the 5-tier values                 |
/// | `confidence`       | number             | `[0, 1]` inclusive                       |
/// | `horizon`          | string enum        | `"short"` only at v0.1.0                 |
/// | `reasoning_trace`  | string             | 50–2000 chars (L4 gate)                  |
/// | `cited_lesson_ids` | array of strings   | optional; subset of retrieved top-K IDs  |
///
/// ## Temperature pin note
///
/// The temperature pin (`temperature = Some(0.0)`) is enforced on the
/// `ChatRequest`, not here in the schema. See `LlmForecasterImpl::build_request`.
#[must_use]
pub fn propose_forecast_schema() -> ToolSchema {
    ToolSchema {
        name: PROPOSE_FORECAST_TOOL_NAME.to_string(),
        description: concat!(
            "Emit a directional forecast for the given symbol. ",
            "Call this tool with a structured payload containing the 5-tier rating, ",
            "confidence score, horizon, reasoning trace, and any cited lesson card IDs. ",
            "Do NOT emit free-form text — call this tool instead."
        )
        .to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "rating": {
                    "type": "string",
                    "enum": ["STRONG_BUY", "BUY", "HOLD", "SELL", "STRONG_SELL"],
                    "description": concat!(
                        "5-tier directional rating. ",
                        "STRONG_BUY = high-conviction bullish (>1% up next 1h); ",
                        "BUY = moderate bullish (0.3–1% up); ",
                        "HOLD = no directional edge; ",
                        "SELL = moderate bearish (0.3–1% down); ",
                        "STRONG_SELL = high-conviction bearish (>1% down)."
                    )
                },
                "confidence": {
                    "type": "number",
                    "minimum": 0.0,
                    "maximum": 1.0,
                    "description": concat!(
                        "Calibrated confidence in [0, 1]. ",
                        "This value is correlated against realized outcome. ",
                        "High-confidence wrong forecasts are penalized — be honest about uncertainty."
                    )
                },
                "horizon": {
                    "type": "string",
                    "enum": ["short"],
                    "description": "Forecast horizon. Always 'short' (next 1h candle) at v0.1.0."
                },
                "reasoning_trace": {
                    "type": "string",
                    "minLength": 50,
                    "maxLength": 2000,
                    "description": concat!(
                        "50–2000 characters of structured reasoning explaining WHICH signals ",
                        "drove the rating. Reference specific indicator values (RSI, MACD, etc.), ",
                        "lesson card insights, and recent decision patterns."
                    )
                },
                "cited_lesson_ids": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": concat!(
                        "Optional list of lesson card IDs that influenced this forecast. ",
                        "Use the exact card_id strings from the 'Retrieved lesson cards' section. ",
                        "Pass an empty array if no lesson cards are relevant."
                    )
                }
            },
            "required": ["rating", "confidence", "horizon", "reasoning_trace"]
        }),
    }
}

// ── Unit tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use llm::validate_tool_use;

    /// Schema validates a fully-populated well-formed payload.
    #[test]
    fn schema_accepts_well_formed_payload() {
        let schema = propose_forecast_schema();
        let input = serde_json::json!({
            "rating": "BUY",
            "confidence": 0.75,
            "horizon": "short",
            "reasoning_trace": "RSI(14) = 62.3, trending above 60 for 3 consecutive bars. MACD histogram positive at 0.0023. BB upper band not yet breached. Lesson card lc_abc123 (BTC Bull regime, +1.2% outcome) cited.",
            "cited_lesson_ids": ["lc_abc123"]
        });
        assert!(
            validate_tool_use(&schema, &input).is_ok(),
            "well-formed payload must validate"
        );
    }

    /// Schema accepts payloads without the optional `cited_lesson_ids` field.
    #[test]
    fn schema_accepts_payload_without_cited_lesson_ids() {
        let schema = propose_forecast_schema();
        let input = serde_json::json!({
            "rating": "HOLD",
            "confidence": 0.42,
            "horizon": "short",
            "reasoning_trace": "Mixed signals: RSI at 50.1 (neutral), MACD near-zero histogram. No strong lesson card match. No directional edge — HOLD."
        });
        assert!(
            validate_tool_use(&schema, &input).is_ok(),
            "payload without cited_lesson_ids must be valid"
        );
    }

    /// Schema rejects payloads with an unknown rating value.
    #[test]
    fn schema_rejects_unknown_rating() {
        let schema = propose_forecast_schema();
        let input = serde_json::json!({
            "rating": "STRONG_MAYBE",
            "confidence": 0.5,
            "horizon": "short",
            "reasoning_trace": "Some trace that is long enough to pass the 50-char minimum check here."
        });
        assert!(
            validate_tool_use(&schema, &input).is_err(),
            "unknown rating must be rejected"
        );
    }

    /// Schema rejects reasoning_trace shorter than 50 chars.
    #[test]
    fn schema_rejects_short_reasoning_trace() {
        let schema = propose_forecast_schema();
        let input = serde_json::json!({
            "rating": "BUY",
            "confidence": 0.8,
            "horizon": "short",
            "reasoning_trace": "Too short."
        });
        assert!(
            validate_tool_use(&schema, &input).is_err(),
            "reasoning_trace < 50 chars must be rejected"
        );
    }

    /// Schema rejects confidence outside [0, 1].
    #[test]
    fn schema_rejects_confidence_out_of_range() {
        let schema = propose_forecast_schema();
        for bad_confidence in [serde_json::json!(-0.1_f64), serde_json::json!(1.5_f64)] {
            let input = serde_json::json!({
                "rating": "BUY",
                "confidence": bad_confidence,
                "horizon": "short",
                "reasoning_trace": "Some trace that is long enough to pass the 50-char minimum check here."
            });
            assert!(
                validate_tool_use(&schema, &input).is_err(),
                "confidence {bad_confidence} must be rejected"
            );
        }
    }

    /// Schema rejects missing required fields.
    #[test]
    fn schema_rejects_missing_required_fields() {
        let schema = propose_forecast_schema();
        // Missing `rating`
        let input = serde_json::json!({
            "confidence": 0.5,
            "horizon": "short",
            "reasoning_trace": "Some trace that is long enough to pass the 50-char minimum check here."
        });
        assert!(
            validate_tool_use(&schema, &input).is_err(),
            "missing 'rating' must be rejected"
        );
    }

    /// Tool name constant matches the schema name.
    #[test]
    fn tool_name_constant_matches_schema() {
        let schema = propose_forecast_schema();
        assert_eq!(schema.name, PROPOSE_FORECAST_TOOL_NAME);
    }

    /// All five valid rating enum values are accepted.
    #[test]
    fn schema_accepts_all_five_rating_values() {
        let schema = propose_forecast_schema();
        for rating in ["STRONG_BUY", "BUY", "HOLD", "SELL", "STRONG_SELL"] {
            let input = serde_json::json!({
                "rating": rating,
                "confidence": 0.5,
                "horizon": "short",
                "reasoning_trace": "Some trace that is long enough to pass the 50-char minimum check here."
            });
            assert!(
                validate_tool_use(&schema, &input).is_ok(),
                "valid rating '{rating}' must be accepted"
            );
        }
    }
}
