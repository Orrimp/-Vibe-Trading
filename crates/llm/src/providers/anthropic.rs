//! Anthropic provider impl (T1903).
//!
//! Implements [`crate::LlmProvider`] against Anthropic's Messages API
//! (`POST {base_url}/messages`). Handles:
//!
//! - **Prompt-cache headers (Q5b/c).** [`SystemBlock::Cached`] is
//!   serialized as `{"type": "text", "text": "...", "cache_control":
//!   {"type": "ephemeral"}}`. Plain blocks are serialized without
//!   `cache_control`.
//! - **Tool-use (R5, Q4e).** [`ToolSchema`] is forwarded as Anthropic's
//!   `tools: [{name, description, input_schema}]` shape. Tool-use
//!   response blocks are passed through [`crate::tools::validate_tool_use`]
//!   against the matching declared schema before surfacing.
//! - **Streaming-off (R1.1).** The request always carries `stream: false`.
//! - **Retries (Q9).** Wraps the HTTP call in
//!   [`crate::retry::run_with_backoff`] with `max_retries = 3`. HTTP 429
//!   (with parsed `Retry-After`) and 503 retry; HTTP 401 surfaces as
//!   [`LlmError::Auth`] (fatal); other 4xx surface as
//!   [`LlmError::Provider`] (fatal).
//!
//! No real HTTP is exercised by unit tests; the in-module tests stay on
//! the pure helpers (`build_request_body`, `parse_response`,
//! `parse_retry_after`). The integration test file
//! `crates/llm/tests/anthropic_provider_test.rs` uses `wiremock` for
//! HTTP-shape assertions (T1903 acceptance criteria).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::error::LlmError;
use crate::retry::{run_with_backoff, RetryError};
use crate::tools::{validate_tool_use, ToolSchema};
use crate::trait_def::{
    CacheBreakpoint, ChatMessage, ChatRequest, ChatResponse, ContentBlock, LlmProvider,
    MessageRole, ModelId, StopReason, SystemBlock, TokenUsage,
};
use crate::ProviderKind;

/// Anthropic API version pinned at v2.0.0 ship time (latest stable
/// per Anthropic's docs as of brief authoring).
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Anthropic provider — wraps the Messages API with prompt-cache + tool-use
/// support.
///
/// Constructed by `LlmProviderFactory` (T1913); consumers receive an
/// `Arc<dyn LlmProvider>` and never touch this struct directly.
#[derive(Debug, Clone)]
pub struct AnthropicProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    default_model: ModelId,
}

impl AnthropicProvider {
    /// Construct an Anthropic provider with default base URL
    /// (`https://api.anthropic.com/v1`).
    #[must_use]
    pub fn new(api_key: impl Into<String>, default_model: ModelId) -> Self {
        Self::with_base_url("https://api.anthropic.com/v1", api_key, default_model)
    }

    /// Construct with a custom base URL — used by `wiremock` integration
    /// tests and operators routing through a corporate proxy.
    #[must_use]
    pub fn with_base_url(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        default_model: ModelId,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            api_key: api_key.into(),
            default_model,
        }
    }

    /// Effective model — the request's model takes precedence; the
    /// provider's `default_model` is a fallback used only when the
    /// request supplies an empty `ModelId`.
    fn effective_model<'a>(&'a self, req: &'a ChatRequest) -> &'a ModelId {
        if req.model.as_str().is_empty() {
            &self.default_model
        } else {
            &req.model
        }
    }
}

// ── Wire format ──────────────────────────────────────────────────────────────

/// Anthropic system-prompt segment with optional `cache_control` marker.
///
/// `Plain` blocks omit `cache_control` entirely (serde `skip_serializing_if`).
#[derive(Debug, Serialize, PartialEq, Eq)]
struct WireSystemBlock {
    #[serde(rename = "type")]
    block_type: &'static str,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_control: Option<WireCacheControl>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct WireCacheControl {
    #[serde(rename = "type")]
    cache_type: &'static str,
}

#[derive(Debug, Serialize, PartialEq)]
struct WireToolSchema<'a> {
    name: &'a str,
    description: &'a str,
    input_schema: &'a serde_json::Value,
}

#[derive(Debug, Serialize, PartialEq)]
struct WireMessage {
    role: &'static str,
    content: Vec<WireContentBlock>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(untagged)]
enum WireContentBlock {
    Text {
        #[serde(rename = "type")]
        block_type: &'static str,
        text: String,
    },
    ToolUse {
        #[serde(rename = "type")]
        block_type: &'static str,
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

#[derive(Debug, Serialize, PartialEq)]
pub(crate) struct WireRequestBody<'a> {
    model: &'a str,
    max_tokens: u32,
    stream: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    system: Vec<WireSystemBlock>,
    messages: Vec<WireMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<WireToolSchema<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

/// Build the Anthropic JSON request body from a [`ChatRequest`].
///
/// `pub(crate)` so the in-module tests can assert against the constructed
/// shape without spinning a `wiremock` server.
pub(crate) fn build_request_body<'a>(
    req: &'a ChatRequest,
    model: &'a ModelId,
) -> WireRequestBody<'a> {
    WireRequestBody {
        model: model.as_str(),
        max_tokens: req.max_tokens,
        stream: false,
        system: req.system.iter().map(wire_system_block).collect(),
        messages: req.messages.iter().map(wire_message).collect(),
        tools: req.tools.iter().map(wire_tool).collect(),
        temperature: req.temperature,
    }
}

fn wire_system_block(block: &SystemBlock) -> WireSystemBlock {
    match block {
        SystemBlock::Plain(text) => WireSystemBlock {
            block_type: "text",
            text: text.clone(),
            cache_control: None,
        },
        SystemBlock::Cached(text, CacheBreakpoint::Ephemeral) => WireSystemBlock {
            block_type: "text",
            text: text.clone(),
            cache_control: Some(WireCacheControl {
                cache_type: "ephemeral",
            }),
        },
    }
}

fn wire_message(msg: &ChatMessage) -> WireMessage {
    WireMessage {
        role: match msg.role {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
        },
        content: msg.content.iter().map(wire_content_block).collect(),
    }
}

fn wire_content_block(block: &ContentBlock) -> WireContentBlock {
    match block {
        ContentBlock::Text(t) => WireContentBlock::Text {
            block_type: "text",
            text: t.clone(),
        },
        ContentBlock::ToolUse { name, input, id } => WireContentBlock::ToolUse {
            block_type: "tool_use",
            id: id.clone(),
            name: name.clone(),
            input: input.clone(),
        },
    }
}

fn wire_tool<'a>(t: &'a ToolSchema) -> WireToolSchema<'a> {
    WireToolSchema {
        name: &t.name,
        description: &t.description,
        input_schema: &t.input_schema,
    }
}

// ── Response parsing ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct WireResponseBody {
    content: Vec<WireResponseBlock>,
    #[serde(default)]
    stop_reason: Option<String>,
    #[serde(default)]
    usage: Option<WireUsage>,
    #[serde(default)]
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WireResponseBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

#[derive(Debug, Deserialize, Default)]
struct WireUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: u64,
}

/// Parse Anthropic's response body into a [`ChatResponse`].
///
/// `pub(crate)` so the in-module tests can exercise it without spinning
/// a `wiremock` server. Validates any tool-use payloads against the
/// matching declared `ToolSchema`.
pub(crate) fn parse_response(
    body: &str,
    req: &ChatRequest,
    served_model_fallback: &ModelId,
) -> Result<ChatResponse, LlmError> {
    let raw: WireResponseBody = serde_json::from_str(body)
        .map_err(|e| LlmError::InvalidResponse(format!("anthropic body parse: {e}")))?;

    let mut content: Vec<ContentBlock> = Vec::with_capacity(raw.content.len());
    for block in raw.content {
        match block {
            WireResponseBlock::Text { text } => content.push(ContentBlock::Text(text)),
            WireResponseBlock::ToolUse { id, name, input } => {
                let schema = req.tools.iter().find(|t| t.name == name).ok_or_else(|| {
                    LlmError::InvalidResponse(format!(
                        "tool_use response references undeclared tool '{name}'"
                    ))
                })?;
                validate_tool_use(schema, &input)?;
                content.push(ContentBlock::ToolUse { name, input, id });
            }
        }
    }

    let stop_reason = match raw.stop_reason.as_deref() {
        Some("end_turn") | None => StopReason::EndTurn,
        Some("max_tokens") => StopReason::MaxTokens,
        Some("tool_use") => StopReason::ToolUse,
        Some("stop_sequence") => StopReason::StopSequence,
        Some(other) => {
            return Err(LlmError::InvalidResponse(format!(
                "anthropic returned unknown stop_reason '{other}'"
            )));
        }
    };

    let usage = raw.usage.unwrap_or_default();
    let served_model = raw
        .model
        .map(ModelId::from)
        .unwrap_or_else(|| served_model_fallback.clone());

    Ok(ChatResponse {
        content,
        stop_reason,
        usage: TokenUsage {
            tokens_in: usage.input_tokens,
            tokens_out: usage.output_tokens,
            tokens_cached_in: usage.cache_read_input_tokens,
        },
        model: served_model,
        correlation_id: req.correlation_id,
    })
}

/// Parse a `Retry-After` header value (seconds-only — the HTTP-date
/// variant is not honored at v2; provider impls fall back to computed
/// backoff in that case).
pub(crate) fn parse_retry_after(raw: Option<&str>) -> Option<Duration> {
    raw.and_then(|v| v.trim().parse::<u64>().ok())
        .map(Duration::from_secs)
}

/// Classify an HTTP error response into the [`RetryError`] taxonomy.
///
/// `pub(crate)` for the in-module tests; production caller is `complete`.
pub(crate) fn classify_http_error(
    status: reqwest::StatusCode,
    retry_after: Option<Duration>,
    sanitized_body: &str,
) -> RetryError {
    match status.as_u16() {
        429 => RetryError::RateLimited { retry_after },
        503 => RetryError::Transient,
        401 | 403 => RetryError::Fatal(LlmError::Auth(format!(
            "anthropic auth failure ({status}): {sanitized_body}"
        ))),
        _ => RetryError::Fatal(LlmError::Provider {
            provider: ProviderKind::Anthropic,
            message: format!("HTTP {status}: {sanitized_body}"),
        }),
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn provider_kind(&self) -> ProviderKind {
        ProviderKind::Anthropic
    }

    async fn complete(&self, request: ChatRequest) -> Result<ChatResponse, LlmError> {
        let model = self.effective_model(&request).clone();
        let body = build_request_body(&request, &model);
        let url = format!("{}/messages", self.base_url.trim_end_matches('/'));

        let response = run_with_backoff(3, || async {
            let resp = self
                .client
                .post(&url)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", ANTHROPIC_VERSION)
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| RetryError::Fatal(LlmError::Network(e)))?;

            let status = resp.status();
            if status.is_success() {
                let text = resp
                    .text()
                    .await
                    .map_err(|e| RetryError::Fatal(LlmError::Network(e)))?;
                return Ok(text);
            }

            let retry_after = parse_retry_after(
                resp.headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok()),
            );
            // Read body for the error message — we drop it on success path.
            let body_text = resp.text().await.unwrap_or_default();
            // Strip newlines so log lines stay one-per-event.
            let sanitized: String = body_text.replace(['\n', '\r'], " ");
            Err(classify_http_error(status, retry_after, &sanitized))
        })
        .await?;

        parse_response(&response, &request, &model)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use cost::{AgentRole, LlmTier};

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

    fn base_request() -> ChatRequest {
        let mut req = ChatRequest::new(
            ModelId::from("claude-3-5-sonnet-20241022"),
            LlmTier::DeepThink,
            AgentRole::Trader,
        );
        req.system = vec![
            SystemBlock::Cached("project context".into(), CacheBreakpoint::Ephemeral),
            SystemBlock::Cached("role context".into(), CacheBreakpoint::Ephemeral),
            SystemBlock::Plain("dynamic context".into()),
        ];
        req.messages = vec![ChatMessage {
            role: MessageRole::User,
            content: vec![ContentBlock::Text("hello".into())],
        }];
        req.max_tokens = 256;
        req
    }

    /// T1903 wire-shape: `SystemBlock::Cached` produces exactly two
    /// `cache_control: {"type": "ephemeral"}` markers, and the `Plain`
    /// block has no `cache_control` key at all.
    #[test]
    fn t1903_build_request_body_emits_two_cache_breakpoints() {
        let req = base_request();
        let body = build_request_body(&req, &req.model);
        let json = serde_json::to_value(&body).expect("body serializes");

        let system = json
            .get("system")
            .and_then(|v| v.as_array())
            .expect("system is an array");
        assert_eq!(system.len(), 3, "system has 3 blocks");

        let cached_count = system
            .iter()
            .filter(|b| {
                b.get("cache_control")
                    .and_then(|cc| cc.get("type"))
                    .and_then(|t| t.as_str())
                    == Some("ephemeral")
            })
            .count();
        assert_eq!(
            cached_count, 2,
            "exactly 2 cache_control: ephemeral markers"
        );

        // Plain block carries no cache_control key (skip_serializing_if).
        let plain = &system[2];
        assert!(plain.get("cache_control").is_none());
        assert_eq!(plain["text"], "dynamic context");

        // Top-level wire invariants.
        assert_eq!(json["stream"], serde_json::Value::Bool(false));
        assert_eq!(json["max_tokens"], serde_json::Value::Number(256.into()));
    }

    /// T1903 wire-shape: when `system` is empty, the `system` field is
    /// omitted from the body (Anthropic accepts both shapes but cleaner
    /// to omit).
    #[test]
    fn t1903_build_request_body_omits_empty_system_and_tools() {
        let req = ChatRequest::new(
            ModelId::from("claude-3-5-sonnet-20241022"),
            LlmTier::QuickThink,
            AgentRole::SentimentAnalyst,
        );
        let json = serde_json::to_value(build_request_body(&req, &req.model)).unwrap();
        assert!(json.get("system").is_none(), "empty system omitted");
        assert!(json.get("tools").is_none(), "empty tools omitted");
        assert!(
            json.get("temperature").is_none(),
            "None temperature omitted"
        );
    }

    /// T1903 wire-shape: declared tools serialize with Anthropic's
    /// `name / description / input_schema` shape.
    #[test]
    fn t1903_build_request_body_includes_declared_tools() {
        let mut req = base_request();
        req.tools = vec![buy_tool()];
        let json = serde_json::to_value(build_request_body(&req, &req.model)).unwrap();

        let tools = json["tools"].as_array().expect("tools is array");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "buy");
        assert_eq!(tools[0]["input_schema"]["type"], "object");
    }

    /// T1903 response parsing: canned 200 body parses into the expected
    /// `ChatResponse` with correct `usage` mapping.
    #[test]
    fn t1903_parse_response_maps_usage_fields() {
        let req = base_request();
        let body = serde_json::json!({
            "id": "msg_01",
            "type": "message",
            "role": "assistant",
            "model": "claude-3-5-sonnet-20241022",
            "content": [{"type": "text", "text": "hi"}],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 1000,
                "output_tokens": 200,
                "cache_read_input_tokens": 500
            }
        })
        .to_string();

        let resp = parse_response(&body, &req, &req.model).expect("parses");
        assert_eq!(resp.usage.tokens_in, 1000);
        assert_eq!(resp.usage.tokens_out, 200);
        assert_eq!(resp.usage.tokens_cached_in, 500);
        assert_eq!(resp.stop_reason, StopReason::EndTurn);
        assert_eq!(resp.content.len(), 1);
        assert!(matches!(&resp.content[0], ContentBlock::Text(t) if t == "hi"));
        assert_eq!(resp.correlation_id, req.correlation_id);
    }

    /// T1903 response parsing: a `tool_use` block is validated against
    /// the declared `ToolSchema` and surfaces as `ContentBlock::ToolUse`.
    #[test]
    fn t1903_parse_response_validates_tool_use() {
        let mut req = base_request();
        req.tools = vec![buy_tool()];
        let body = serde_json::json!({
            "content": [{
                "type": "tool_use",
                "id": "toolu_01",
                "name": "buy",
                "input": {"symbol": "BTC", "qty": 0.5}
            }],
            "stop_reason": "tool_use",
            "usage": {"input_tokens": 10, "output_tokens": 5}
        })
        .to_string();

        let resp = parse_response(&body, &req, &req.model).expect("parses");
        match &resp.content[0] {
            ContentBlock::ToolUse { name, input, id } => {
                assert_eq!(name, "buy");
                assert_eq!(id, "toolu_01");
                assert_eq!(input["symbol"], "BTC");
            }
            other => panic!("expected ToolUse, got {other:?}"),
        }
        assert_eq!(resp.stop_reason, StopReason::ToolUse);
        // cache_read_input_tokens missing → defaults to 0.
        assert_eq!(resp.usage.tokens_cached_in, 0);
    }

    /// T1903 response parsing: a tool_use payload that fails JSON-Schema
    /// validation surfaces as `LlmError::InvalidResponse`.
    #[test]
    fn t1903_parse_response_rejects_schema_violating_tool_use() {
        let mut req = base_request();
        req.tools = vec![buy_tool()];
        // Missing required field `qty`.
        let body = serde_json::json!({
            "content": [{
                "type": "tool_use",
                "id": "toolu_01",
                "name": "buy",
                "input": {"symbol": "BTC"}
            }],
            "stop_reason": "tool_use",
            "usage": {"input_tokens": 10, "output_tokens": 5}
        })
        .to_string();

        let err = parse_response(&body, &req, &req.model).expect_err("rejected");
        assert!(matches!(err, LlmError::InvalidResponse(_)));
    }

    /// T1903 response parsing: a `tool_use` referencing an undeclared
    /// tool name surfaces as `InvalidResponse` (never matches a schema).
    #[test]
    fn t1903_parse_response_rejects_undeclared_tool_use() {
        let req = base_request(); // no tools declared
        let body = serde_json::json!({
            "content": [{
                "type": "tool_use",
                "id": "toolu_01",
                "name": "sell",
                "input": {}
            }],
            "stop_reason": "tool_use",
            "usage": {"input_tokens": 10, "output_tokens": 5}
        })
        .to_string();

        let err = parse_response(&body, &req, &req.model).expect_err("rejected");
        match err {
            LlmError::InvalidResponse(msg) => assert!(msg.contains("sell")),
            other => panic!("expected InvalidResponse, got {other:?}"),
        }
    }

    /// `Retry-After: 2` header parses to a `Duration::from_secs(2)`.
    #[test]
    fn t1903_parse_retry_after_seconds() {
        assert_eq!(parse_retry_after(Some("2")), Some(Duration::from_secs(2)));
        assert_eq!(
            parse_retry_after(Some("  30  ")),
            Some(Duration::from_secs(30))
        );
        // HTTP-date format is unsupported at v2 — falls back to None.
        assert_eq!(
            parse_retry_after(Some("Wed, 21 Oct 2015 07:28:00 GMT")),
            None
        );
        assert_eq!(parse_retry_after(None), None);
    }

    /// HTTP status classification: 429 / 503 retry; 401 → Auth (fatal);
    /// 500 → Provider (fatal).
    #[test]
    fn t1903_classify_http_error_routes_correctly() {
        let r = classify_http_error(
            reqwest::StatusCode::TOO_MANY_REQUESTS,
            Some(Duration::from_secs(2)),
            "rate limit",
        );
        assert!(
            matches!(r, RetryError::RateLimited { retry_after: Some(d) } if d == Duration::from_secs(2))
        );

        let r = classify_http_error(reqwest::StatusCode::SERVICE_UNAVAILABLE, None, "down");
        assert!(matches!(r, RetryError::Transient));

        let r = classify_http_error(reqwest::StatusCode::UNAUTHORIZED, None, "bad key");
        assert!(matches!(r, RetryError::Fatal(LlmError::Auth(_))));

        let r = classify_http_error(reqwest::StatusCode::INTERNAL_SERVER_ERROR, None, "boom");
        match r {
            RetryError::Fatal(LlmError::Provider { provider, .. }) => {
                assert_eq!(provider, ProviderKind::Anthropic);
            }
            other => panic!("expected Fatal(Provider), got {other:?}"),
        }
    }
}
