//! OpenAI-compatible provider impl (T1904).
//!
//! Covers OpenAI, OpenRouter, DeepSeek, and LM Studio via a common
//! base-URL parameter. Implements [`crate::LlmProvider`] against the
//! Chat Completions shape (`POST {base_url}/chat/completions`).
//!
//! Differences from Anthropic:
//!
//! - **No prompt cache.** [`SystemBlock::Cached`] silently flattens to
//!   plain text; one `tracing::debug!(target: "llm.cache",
//!   "cache_markers_dropped_for_provider")` line per `complete()`
//!   invocation that drops markers.
//! - **`tokens_cached_in` is always 0.** OpenAI-compat doesn't surface
//!   cached input tokens in the response.
//! - **Tool-use as `tools: [{type: "function", function: {...}}]`.**
//!   Surface-side, response `tool_calls[].function.arguments` is a
//!   stringified JSON object — we parse it before
//!   [`crate::tools::validate_tool_use`].
//!
//! Retries follow the same shared [`crate::retry::run_with_backoff`]
//! curve as Anthropic (max 3, full jitter, honor `Retry-After`).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::error::LlmError;
use crate::providers::anthropic::parse_retry_after;
use crate::retry::{run_with_backoff, RetryError};
use crate::tools::{validate_tool_use, ToolSchema};
use crate::trait_def::{
    ChatMessage, ChatRequest, ChatResponse, ContentBlock, LlmProvider, MessageRole, ModelId,
    StopReason, SystemBlock, TokenUsage,
};
use crate::ProviderKind;

/// OpenAI-compatible provider.
///
/// `base_url` covers OpenAI (`https://api.openai.com/v1`), OpenRouter
/// (`https://openrouter.ai/api/v1`), DeepSeek (`https://api.deepseek.com`),
/// LM Studio (`http://localhost:1234/v1`), etc.
#[derive(Debug, Clone)]
pub struct OpenAiProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    default_model: ModelId,
    /// Stable `provider_kind()` value — defaults to `OpenAi` but
    /// operators routing through OpenRouter / DeepSeek can override
    /// at factory time so pricing lookups land on the right rate card.
    kind: ProviderKind,
}

impl OpenAiProvider {
    /// Construct with default `https://api.openai.com/v1` base URL and
    /// `ProviderKind::OpenAi`.
    #[must_use]
    pub fn new(api_key: impl Into<String>, default_model: ModelId) -> Self {
        Self::new_with_base_url("https://api.openai.com/v1", api_key, default_model)
    }

    /// Construct with an explicit base URL. `provider_kind` defaults to
    /// `OpenAi`; use [`OpenAiProvider::with_provider_kind`] post-build
    /// if you need to override (OpenRouter / DeepSeek pricing).
    #[must_use]
    pub fn new_with_base_url(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        default_model: ModelId,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            api_key: api_key.into(),
            default_model,
            kind: ProviderKind::OpenAi,
        }
    }

    /// Override `provider_kind()` — used by the factory when the operator
    /// configures an OpenRouter or DeepSeek route.
    #[must_use]
    pub fn with_provider_kind(mut self, kind: ProviderKind) -> Self {
        self.kind = kind;
        self
    }

    fn effective_model<'a>(&'a self, req: &'a ChatRequest) -> &'a ModelId {
        if req.model.as_str().is_empty() {
            &self.default_model
        } else {
            &req.model
        }
    }
}

// ── Wire format ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, PartialEq)]
struct WireMessage {
    role: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<WireToolCall>>,
}

#[derive(Debug, Serialize, PartialEq)]
struct WireToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: &'static str,
    function: WireFunctionCall,
}

#[derive(Debug, Serialize, PartialEq)]
struct WireFunctionCall {
    name: String,
    /// OpenAI represents tool-call arguments as a **JSON-encoded string**,
    /// not a nested object.
    arguments: String,
}

#[derive(Debug, Serialize, PartialEq)]
struct WireToolDef<'a> {
    #[serde(rename = "type")]
    tool_type: &'static str,
    function: WireFunctionDef<'a>,
}

#[derive(Debug, Serialize, PartialEq)]
struct WireFunctionDef<'a> {
    name: &'a str,
    description: &'a str,
    parameters: &'a serde_json::Value,
}

#[derive(Debug, Serialize, PartialEq)]
pub(crate) struct WireRequestBody<'a> {
    model: &'a str,
    messages: Vec<WireMessage>,
    max_tokens: u32,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<WireToolDef<'a>>,
}

/// Build the OpenAI-shaped JSON request body.
///
/// **`SystemBlock::Cached` flattens to plain text** — Anthropic-isms
/// don't make it onto the wire. If the system prompt is non-empty, we
/// prepend a single `role: "system"` message; otherwise omit it.
///
/// Returns the body alongside a bool reporting whether any cache
/// markers were dropped — caller emits the `tracing::debug!` line.
pub(crate) fn build_request_body<'a>(
    req: &'a ChatRequest,
    model: &'a ModelId,
) -> (WireRequestBody<'a>, bool) {
    let mut markers_dropped = false;
    let mut system_text = String::new();
    for block in &req.system {
        if !system_text.is_empty() {
            system_text.push_str("\n\n");
        }
        match block {
            SystemBlock::Plain(t) => system_text.push_str(t),
            SystemBlock::Cached(t, _) => {
                markers_dropped = true;
                system_text.push_str(t);
            }
        }
    }

    let mut messages: Vec<WireMessage> = Vec::with_capacity(req.messages.len() + 1);
    if !system_text.is_empty() {
        messages.push(WireMessage {
            role: "system",
            content: Some(system_text),
            tool_calls: None,
        });
    }
    for m in &req.messages {
        messages.push(wire_message(m));
    }

    let body = WireRequestBody {
        model: model.as_str(),
        messages,
        max_tokens: req.max_tokens,
        stream: false,
        temperature: req.temperature,
        tools: req.tools.iter().map(wire_tool_def).collect(),
    };
    (body, markers_dropped)
}

fn wire_message(msg: &ChatMessage) -> WireMessage {
    // Flatten content blocks: concatenate `Text` segments; collect
    // `ToolUse` invocations into `tool_calls`. For assistant turns this
    // is the standard OpenAI shape; for user turns we send all text.
    let mut text = String::new();
    let mut tool_calls: Vec<WireToolCall> = Vec::new();
    for block in &msg.content {
        match block {
            ContentBlock::Text(t) => {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(t);
            }
            ContentBlock::ToolUse { name, input, id } => {
                tool_calls.push(WireToolCall {
                    id: id.clone(),
                    call_type: "function",
                    function: WireFunctionCall {
                        name: name.clone(),
                        arguments: serde_json::to_string(input).unwrap_or_else(|_| "{}".into()),
                    },
                });
            }
        }
    }
    WireMessage {
        role: match msg.role {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
        },
        content: if text.is_empty() { None } else { Some(text) },
        tool_calls: if tool_calls.is_empty() {
            None
        } else {
            Some(tool_calls)
        },
    }
}

fn wire_tool_def<'a>(t: &'a ToolSchema) -> WireToolDef<'a> {
    WireToolDef {
        tool_type: "function",
        function: WireFunctionDef {
            name: &t.name,
            description: &t.description,
            parameters: &t.input_schema,
        },
    }
}

// ── Response parsing ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct WireResponseBody {
    #[serde(default)]
    model: Option<String>,
    choices: Vec<WireChoice>,
    #[serde(default)]
    usage: Option<WireUsage>,
}

#[derive(Debug, Deserialize)]
struct WireChoice {
    #[serde(default)]
    finish_reason: Option<String>,
    message: WireResponseMessage,
}

#[derive(Debug, Deserialize)]
struct WireResponseMessage {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<WireResponseToolCall>>,
}

#[derive(Debug, Deserialize)]
struct WireResponseToolCall {
    id: String,
    function: WireResponseFunction,
}

#[derive(Debug, Deserialize)]
struct WireResponseFunction {
    name: String,
    /// JSON-encoded string per OpenAI shape.
    arguments: String,
}

#[derive(Debug, Deserialize, Default)]
struct WireUsage {
    #[serde(default)]
    prompt_tokens: u64,
    #[serde(default)]
    completion_tokens: u64,
}

/// Parse OpenAI's response body into a [`ChatResponse`].
pub(crate) fn parse_response(
    body: &str,
    req: &ChatRequest,
    served_model_fallback: &ModelId,
) -> Result<ChatResponse, LlmError> {
    let raw: WireResponseBody = serde_json::from_str(body)
        .map_err(|e| LlmError::InvalidResponse(format!("openai body parse: {e}")))?;

    let choice = raw
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| LlmError::InvalidResponse("openai response had no choices".into()))?;

    let mut content: Vec<ContentBlock> = Vec::new();
    if let Some(text) = choice.message.content
        && !text.is_empty()
    {
        content.push(ContentBlock::Text(text));
    }
    let had_tool_calls = choice.message.tool_calls.is_some();
    if let Some(calls) = choice.message.tool_calls {
        for call in calls {
            let input: serde_json::Value =
                serde_json::from_str(&call.function.arguments).map_err(|e| {
                    LlmError::InvalidResponse(format!(
                        "tool_call '{}' arguments not JSON: {e}",
                        call.function.name
                    ))
                })?;
            let schema = req
                .tools
                .iter()
                .find(|t| t.name == call.function.name)
                .ok_or_else(|| {
                    LlmError::InvalidResponse(format!(
                        "tool_call references undeclared tool '{}'",
                        call.function.name
                    ))
                })?;
            validate_tool_use(schema, &input)?;
            content.push(ContentBlock::ToolUse {
                name: call.function.name,
                input,
                id: call.id,
            });
        }
    }

    let stop_reason = match choice.finish_reason.as_deref() {
        Some("stop") | None => StopReason::EndTurn,
        Some("length") => StopReason::MaxTokens,
        Some("tool_calls") | Some("function_call") => StopReason::ToolUse,
        Some("content_filter") => StopReason::StopSequence,
        Some(_) if had_tool_calls => StopReason::ToolUse,
        Some(other) => {
            return Err(LlmError::InvalidResponse(format!(
                "openai unknown finish_reason '{other}'"
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
            tokens_in: usage.prompt_tokens,
            tokens_out: usage.completion_tokens,
            tokens_cached_in: 0,
        },
        model: served_model,
        correlation_id: req.correlation_id,
    })
}

/// Classify HTTP error into [`RetryError`]. Same routing matrix as
/// Anthropic, just with provider kind threaded through.
pub(crate) fn classify_http_error(
    status: reqwest::StatusCode,
    retry_after: Option<Duration>,
    sanitized_body: &str,
    provider: ProviderKind,
) -> RetryError {
    match status.as_u16() {
        429 => RetryError::RateLimited { retry_after },
        503 => RetryError::Transient,
        401 | 403 => RetryError::Fatal(LlmError::Auth(format!(
            "openai-compat auth failure ({status}): {sanitized_body}"
        ))),
        _ => RetryError::Fatal(LlmError::Provider {
            provider,
            message: format!("HTTP {status}: {sanitized_body}"),
        }),
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    fn name(&self) -> &str {
        match self.kind {
            ProviderKind::OpenAi => "openai",
            ProviderKind::OpenRouter => "openrouter",
            ProviderKind::DeepSeek => "deepseek",
            _ => "openai_compat",
        }
    }

    fn provider_kind(&self) -> ProviderKind {
        self.kind.clone()
    }

    async fn complete(&self, request: ChatRequest) -> Result<ChatResponse, LlmError> {
        let model = self.effective_model(&request).clone();
        let (body, markers_dropped) = build_request_body(&request, &model);
        if markers_dropped {
            tracing::debug!(
                target: "llm.cache",
                provider = "openai_compat",
                "cache_markers_dropped_for_provider"
            );
        }
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        let response = run_with_backoff(3, || async {
            let resp = self
                .client
                .post(&url)
                .header("authorization", format!("Bearer {}", self.api_key))
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
            let body_text = resp.text().await.unwrap_or_default();
            let sanitized: String = body_text.replace(['\n', '\r'], " ");
            Err(classify_http_error(
                status,
                retry_after,
                &sanitized,
                self.kind.clone(),
            ))
        })
        .await?;

        parse_response(&response, &request, &model)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trait_def::CacheBreakpoint;
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
            ModelId::from("gpt-4o-mini"),
            LlmTier::QuickThink,
            AgentRole::Trader,
        );
        req.system = vec![
            SystemBlock::Cached("project ctx".into(), CacheBreakpoint::Ephemeral),
            SystemBlock::Cached("role ctx".into(), CacheBreakpoint::Ephemeral),
            SystemBlock::Plain("dynamic ctx".into()),
        ];
        req.messages = vec![ChatMessage {
            role: MessageRole::User,
            content: vec![ContentBlock::Text("hi".into())],
        }];
        req.max_tokens = 128;
        req
    }

    /// T1904 wire-shape: `cache_control` markers are NOT serialized;
    /// the cached + plain text segments flatten into a single
    /// `role: "system"` message. `markers_dropped == true` so the
    /// caller emits the `tracing::debug!` line.
    #[test]
    fn t1904_build_request_body_drops_cache_markers() {
        let req = base_request();
        let (body, dropped) = build_request_body(&req, &req.model);
        assert!(dropped, "markers_dropped flag set");
        let json = serde_json::to_value(&body).unwrap();
        assert!(
            !json.to_string().contains("cache_control"),
            "wire body contains NO cache_control markers"
        );
        // First message is the flattened system prompt.
        let messages = json["messages"].as_array().unwrap();
        assert_eq!(messages[0]["role"], "system");
        let sys_text = messages[0]["content"].as_str().unwrap();
        assert!(sys_text.contains("project ctx"));
        assert!(sys_text.contains("role ctx"));
        assert!(sys_text.contains("dynamic ctx"));
    }

    /// T1904 wire-shape: tool definitions use OpenAI's
    /// `{type: "function", function: {...}}` envelope.
    #[test]
    fn t1904_build_request_body_tool_envelope() {
        let mut req = base_request();
        req.tools = vec![buy_tool()];
        let (body, _) = build_request_body(&req, &req.model);
        let json = serde_json::to_value(&body).unwrap();
        let tools = json["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["type"], "function");
        assert_eq!(tools[0]["function"]["name"], "buy");
        assert_eq!(tools[0]["function"]["parameters"]["type"], "object");
    }

    /// T1904 wire-shape: no system blocks → no system message inserted.
    #[test]
    fn t1904_build_request_body_no_system_when_empty() {
        let req = ChatRequest::new(
            ModelId::from("gpt-4o"),
            LlmTier::QuickThink,
            AgentRole::Trader,
        );
        let (body, dropped) = build_request_body(&req, &req.model);
        assert!(!dropped, "no markers to drop");
        // No system message in body.
        for m in &body.messages {
            assert_ne!(m.role, "system");
        }
    }

    /// T1904 response parsing: `tokens_cached_in` is always 0.
    #[test]
    fn t1904_parse_response_tokens_cached_in_is_zero() {
        let req = base_request();
        let body = serde_json::json!({
            "id": "chatcmpl-1",
            "object": "chat.completion",
            "model": "gpt-4o-mini-2024-07-18",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "hi back"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 100, "completion_tokens": 20, "total_tokens": 120}
        })
        .to_string();
        let resp = parse_response(&body, &req, &req.model).expect("parses");
        assert_eq!(resp.usage.tokens_in, 100);
        assert_eq!(resp.usage.tokens_out, 20);
        assert_eq!(resp.usage.tokens_cached_in, 0);
        assert_eq!(resp.stop_reason, StopReason::EndTurn);
        // Model gets carried through from the response.
        assert_eq!(resp.model.as_str(), "gpt-4o-mini-2024-07-18");
    }

    /// T1904 response parsing: tool_calls.function.arguments (a JSON
    /// string) is parsed and validated against the declared schema.
    #[test]
    fn t1904_parse_response_validates_tool_calls() {
        let mut req = base_request();
        req.tools = vec![buy_tool()];
        let body = serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_01",
                        "type": "function",
                        "function": {
                            "name": "buy",
                            "arguments": "{\"symbol\": \"BTC\", \"qty\": 0.5}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5}
        })
        .to_string();
        let resp = parse_response(&body, &req, &req.model).expect("parses");
        match &resp.content[0] {
            ContentBlock::ToolUse { name, input, id } => {
                assert_eq!(name, "buy");
                assert_eq!(id, "call_01");
                assert_eq!(input["qty"], 0.5);
            }
            other => panic!("expected ToolUse, got {other:?}"),
        }
        assert_eq!(resp.stop_reason, StopReason::ToolUse);
    }

    /// T1904 response parsing: bad arguments JSON surfaces as
    /// `InvalidResponse`.
    #[test]
    fn t1904_parse_response_rejects_unparseable_arguments() {
        let mut req = base_request();
        req.tools = vec![buy_tool()];
        let body = serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "tool_calls": [{
                        "id": "call_01",
                        "type": "function",
                        "function": {
                            "name": "buy",
                            "arguments": "not valid json"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1}
        })
        .to_string();
        let err = parse_response(&body, &req, &req.model).expect_err("rejected");
        assert!(matches!(err, LlmError::InvalidResponse(_)));
    }

    /// HTTP classification: same matrix as Anthropic, but `provider`
    /// is threaded through (so OpenRouter / DeepSeek route correctly).
    #[test]
    fn t1904_classify_http_error_threads_provider() {
        let r = classify_http_error(
            reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            None,
            "boom",
            ProviderKind::DeepSeek,
        );
        match r {
            RetryError::Fatal(LlmError::Provider { provider, .. }) => {
                assert_eq!(provider, ProviderKind::DeepSeek);
            }
            other => panic!("expected Fatal(Provider DeepSeek), got {other:?}"),
        }
    }

    /// Provider name varies by `kind` so log/metric labels are correct.
    #[test]
    fn t1904_name_changes_with_kind() {
        let p = OpenAiProvider::new("k".to_string(), ModelId::from("gpt-4o"));
        assert_eq!(p.name(), "openai");
        let p2 = p.clone().with_provider_kind(ProviderKind::OpenRouter);
        assert_eq!(p2.name(), "openrouter");
        let p3 = p.clone().with_provider_kind(ProviderKind::DeepSeek);
        assert_eq!(p3.name(), "deepseek");
    }
}
