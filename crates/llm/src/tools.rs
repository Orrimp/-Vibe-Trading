//! Tool-use schema + boundary validator (R5, Q4e).
//!
//! `ToolSchema` is the trait-level shape carried by `ChatRequest::tools`.
//! `validate_tool_use` is the free function provider impls call to refuse
//! malformed tool-use payloads before surfacing them to the consumer.
//! Living as a free function (not a trait default-method) lets a future
//! provider that does its own server-side validation (e.g. Anthropic's
//! `tools.beta.schema-strict`) opt out cleanly.

use serde::{Deserialize, Serialize};

use crate::error::LlmError;

/// Declarative tool definition advertised to the LLM.
///
/// `input_schema` is a `serde_json::Value` — the foundation trait stays
/// narrow; consumers that prefer compile-time-typed schemas use the
/// `schemars` crate to generate the `Value` (no schemars dep in `llm`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSchema {
    /// Tool name (matches the `ContentBlock::ToolUse::name` the model
    /// returns).
    pub name: String,
    /// Human-readable description shown to the model.
    pub description: String,
    /// JSON Schema (Draft 2020-12) describing the tool's input shape.
    pub input_schema: serde_json::Value,
}

/// Validate a tool-use payload against its declared schema.
///
/// Provider impls call this from their `complete()` response path before
/// surfacing a `ContentBlock::ToolUse` to the caller. Validation failures
/// surface as [`LlmError::InvalidResponse`] so the consumer-side error
/// route ("ping the prompt author") is unambiguous.
///
/// # Errors
///
/// Returns [`LlmError::InvalidResponse`] if the schema is itself malformed
/// or if `input` does not conform.
pub fn validate_tool_use(schema: &ToolSchema, input: &serde_json::Value) -> Result<(), LlmError> {
    let validator = jsonschema::validator_for(&schema.input_schema).map_err(|e| {
        LlmError::InvalidResponse(format!("tool '{}' has invalid schema: {e}", schema.name))
    })?;
    if let Err(err) = validator.validate(input) {
        return Err(LlmError::InvalidResponse(format!(
            "tool '{}' input validation failed: {err}",
            schema.name
        )));
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn buy_tool() -> ToolSchema {
        ToolSchema {
            name: "buy".to_string(),
            description: "Buy a given symbol".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "symbol": {"type": "string"},
                    "qty": {"type": "number", "minimum": 0}
                },
                "required": ["symbol", "qty"]
            }),
        }
    }

    #[test]
    fn t1901_validate_tool_use_accepts_conforming_input() {
        let schema = buy_tool();
        let input = serde_json::json!({"symbol": "BTC", "qty": 0.5});
        assert!(validate_tool_use(&schema, &input).is_ok());
    }

    #[test]
    fn t1901_validate_tool_use_rejects_missing_required_field() {
        let schema = buy_tool();
        let input = serde_json::json!({"symbol": "BTC"});
        let err = validate_tool_use(&schema, &input).unwrap_err();
        match err {
            LlmError::InvalidResponse(msg) => {
                assert!(msg.contains("buy"), "msg should name the tool: {msg}");
            }
            other => panic!("expected InvalidResponse, got {other:?}"),
        }
    }

    #[test]
    fn t1901_validate_tool_use_rejects_wrong_type() {
        let schema = buy_tool();
        let input = serde_json::json!({"symbol": "BTC", "qty": "not a number"});
        let err = validate_tool_use(&schema, &input).unwrap_err();
        assert!(matches!(err, LlmError::InvalidResponse(_)));
    }
}
