//! Ollama provider impl (T1905).
//!
//! Local Ollama daemon — no auth, no rate-limiting, no prompt cache.
//! Implements [`crate::LlmProvider`] against `POST {base_url}/api/chat`.
//!
//! Differences from the cloud providers:
//!
//! - **`max_retries = 0`.** HTTP failures surface immediately as
//!   [`LlmError::Network`] (Ollama is local — retries don't fix a
//!   wrong-port misconfig or a stopped daemon).
//! - **No cache.** [`SystemBlock::Cached`] flattens to plain text.
//! - **Best-effort tool-use (R5.4).** Ollama has no native tool-call
//!   surface; when `request.tools` is non-empty we append a "respond
//!   in JSON matching this schema" tail to the system prompt and parse
//!   the assistant's text response as a JSON tool-use object. Schema
//!   validation failures surface as `LlmError::InvalidResponse`.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::ProviderKind;
use crate::error::LlmError;
use crate::tools::{ToolSchema, validate_tool_use};
use crate::trait_def::{
    ChatMessage, ChatRequest, ChatResponse, ContentBlock, LlmProvider, MessageRole, ModelId,
    StopReason, SystemBlock, TokenUsage,
};

/// Ollama provider — local daemon, no auth.
#[derive(Debug, Clone)]
pub struct OllamaProvider {
    client: reqwest::Client,
    base_url: String,
    default_model: ModelId,
}

impl OllamaProvider {
    /// Construct with default `http://localhost:11434` base URL.
    #[must_use]
    pub fn new(default_model: ModelId) -> Self {
        Self::with_base_url("http://localhost:11434", default_model)
    }

    /// Construct with a custom base URL (CI mock server, remote LAN host).
    #[must_use]
    pub fn with_base_url(base_url: impl Into<String>, default_model: ModelId) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            default_model,
        }
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
    content: String,
}

#[derive(Debug, Serialize, PartialEq, Default)]
struct WireOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Serialize, PartialEq)]
pub(crate) struct WireRequestBody<'a> {
    model: &'a str,
    messages: Vec<WireMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "is_default_options")]
    options: WireOptions,
}

fn is_default_options(o: &WireOptions) -> bool {
    o.num_predict.is_none() && o.temperature.is_none()
}

/// Build the Ollama JSON request body.
///
/// If the request has declared tools, we append a tool-schema tail to
/// the system message (best-effort R5.4). The schema list serializes
/// as compact JSON for the LLM to mirror.
pub(crate) fn build_request_body<'a>(
    req: &'a ChatRequest,
    model: &'a ModelId,
) -> WireRequestBody<'a> {
    let mut system_text = String::new();
    for block in &req.system {
        if !system_text.is_empty() {
            system_text.push_str("\n\n");
        }
        match block {
            SystemBlock::Plain(t) | SystemBlock::Cached(t, _) => system_text.push_str(t),
        }
    }

    if !req.tools.is_empty() {
        if !system_text.is_empty() {
            system_text.push_str("\n\n");
        }
        system_text.push_str(&tool_schema_tail(&req.tools));
    }

    let mut messages: Vec<WireMessage> = Vec::with_capacity(req.messages.len() + 1);
    if !system_text.is_empty() {
        messages.push(WireMessage {
            role: "system",
            content: system_text,
        });
    }
    for m in &req.messages {
        messages.push(wire_message(m));
    }

    WireRequestBody {
        model: model.as_str(),
        messages,
        stream: false,
        options: WireOptions {
            num_predict: Some(req.max_tokens),
            temperature: req.temperature,
        },
    }
}

fn wire_message(msg: &ChatMessage) -> WireMessage {
    let mut text = String::new();
    for block in &msg.content {
        match block {
            ContentBlock::Text(t) => {
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(t);
            }
            ContentBlock::ToolUse { name, input, .. } => {
                // Render a prior tool-use as a JSON line so Ollama sees
                // the conversation history coherently in best-effort mode.
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(&format!(
                    "[tool_use {name}] {}",
                    serde_json::to_string(input).unwrap_or_default()
                ));
            }
        }
    }
    WireMessage {
        role: match msg.role {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
        },
        content: text,
    }
}

/// Render the tool-schema tail appended to the system prompt for
/// best-effort tool-use (R5.4). One tool: just emit the schema. Many
/// tools: emit them as a JSON array with `name` and `input_schema`.
pub(crate) fn tool_schema_tail(tools: &[ToolSchema]) -> String {
    let list: Vec<serde_json::Value> = tools
        .iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name,
                "description": t.description,
                "input_schema": t.input_schema,
            })
        })
        .collect();
    let pretty = serde_json::to_string(&list).unwrap_or_else(|_| "[]".into());
    format!(
        "Respond with a single JSON object matching one of these tool schemas: \
         {{\"name\": <tool name>, \"input\": <input object>}}. Tools: {pretty}"
    )
}

// ── Response parsing ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct WireResponseBody {
    #[serde(default)]
    model: Option<String>,
    message: WireResponseMessage,
    #[serde(default)]
    done: bool,
    #[serde(default)]
    done_reason: Option<String>,
    #[serde(default)]
    prompt_eval_count: u64,
    #[serde(default)]
    eval_count: u64,
}

#[derive(Debug, Deserialize)]
struct WireResponseMessage {
    #[serde(default)]
    content: String,
}

/// Parse Ollama's response body into a [`ChatResponse`].
///
/// If the request declared tools, the text content is JSON-parsed as
/// `{"name": "<tool>", "input": {...}}` and validated against the
/// declared schema. Otherwise the text passes through as
/// `ContentBlock::Text`.
pub(crate) fn parse_response(
    body: &str,
    req: &ChatRequest,
    served_model_fallback: &ModelId,
) -> Result<ChatResponse, LlmError> {
    let raw: WireResponseBody = serde_json::from_str(body)
        .map_err(|e| LlmError::InvalidResponse(format!("ollama body parse: {e}")))?;

    let text = raw.message.content;
    let mut content: Vec<ContentBlock> = Vec::new();
    let mut stop_reason = match raw.done_reason.as_deref() {
        Some("stop") | None => StopReason::EndTurn,
        Some("length") => StopReason::MaxTokens,
        _ => StopReason::EndTurn,
    };
    // Best-effort tool-use parse when tools are declared (R5.4).
    if !req.tools.is_empty() {
        let parsed: serde_json::Value = serde_json::from_str(text.trim()).map_err(|e| {
            LlmError::InvalidResponse(format!(
                "ollama best-effort tool-use schema-mismatch: not JSON: {e}"
            ))
        })?;
        let name = parsed
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                LlmError::InvalidResponse(
                    "ollama best-effort tool-use schema-mismatch: missing 'name'".into(),
                )
            })?
            .to_string();
        let input = parsed.get("input").cloned().ok_or_else(|| {
            LlmError::InvalidResponse(
                "ollama best-effort tool-use schema-mismatch: missing 'input'".into(),
            )
        })?;
        let schema = req.tools.iter().find(|t| t.name == name).ok_or_else(|| {
            LlmError::InvalidResponse(format!(
                "ollama best-effort tool-use schema-mismatch: undeclared tool '{name}'"
            ))
        })?;
        validate_tool_use(schema, &input).map_err(|e| match e {
            LlmError::InvalidResponse(msg) => LlmError::InvalidResponse(format!(
                "ollama best-effort tool-use schema-mismatch: {msg}"
            )),
            other => other,
        })?;
        content.push(ContentBlock::ToolUse {
            name,
            input,
            id: "ollama-best-effort".to_string(),
        });
        stop_reason = StopReason::ToolUse;
    } else if !text.is_empty() {
        content.push(ContentBlock::Text(text));
    }

    // `done == false` only on stream chunks — we forbid streaming on
    // the request, so a falsey `done` is an unexpected partial response.
    if !raw.done {
        tracing::debug!(
            target: "llm.ollama",
            "ollama response carried done=false but streaming was disabled"
        );
    }

    let served_model = raw
        .model
        .map(ModelId::from)
        .unwrap_or_else(|| served_model_fallback.clone());

    Ok(ChatResponse {
        content,
        stop_reason,
        usage: TokenUsage {
            tokens_in: raw.prompt_eval_count,
            tokens_out: raw.eval_count,
            tokens_cached_in: 0,
        },
        model: served_model,
        correlation_id: req.correlation_id,
    })
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    fn provider_kind(&self) -> ProviderKind {
        ProviderKind::Other("ollama".to_string())
    }

    async fn complete(&self, request: ChatRequest) -> Result<ChatResponse, LlmError> {
        let model = self.effective_model(&request).clone();
        let body = build_request_body(&request, &model);
        let url = format!("{}/api/chat", self.base_url.trim_end_matches('/'));

        // No retries for local Ollama — transport failures surface as
        // LlmError::Network immediately.
        let resp = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body_text = resp.text().await.unwrap_or_default();
            let sanitized: String = body_text.replace(['\n', '\r'], " ");
            return Err(LlmError::Provider {
                provider: ProviderKind::Other("ollama".into()),
                message: format!("HTTP {status}: {sanitized}"),
            });
        }
        let text = resp.text().await?;
        parse_response(&text, &request, &model)
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
            ModelId::from("llama3:8b"),
            LlmTier::QuickThink,
            AgentRole::Trader,
        );
        req.messages = vec![ChatMessage {
            role: MessageRole::User,
            content: vec![ContentBlock::Text("buy 0.5 BTC".into())],
        }];
        req.max_tokens = 64;
        req
    }

    /// T1905 wire-shape: `options.num_predict = max_tokens`,
    /// `stream = false`.
    #[test]
    fn t1905_build_request_body_options_mapping() {
        let req = base_request();
        let json = serde_json::to_value(build_request_body(&req, &req.model)).unwrap();
        assert_eq!(json["model"], "llama3:8b");
        assert_eq!(json["stream"], serde_json::Value::Bool(false));
        assert_eq!(json["options"]["num_predict"], 64);
    }

    /// T1905 wire-shape: when tools are declared, the system message
    /// tail is appended.
    #[test]
    fn t1905_build_request_body_appends_tool_tail() {
        let mut req = base_request();
        req.tools = vec![buy_tool()];
        let json = serde_json::to_value(build_request_body(&req, &req.model)).unwrap();
        let sys = json["messages"][0].clone();
        assert_eq!(sys["role"], "system");
        let text = sys["content"].as_str().unwrap();
        assert!(text.contains("tool schemas"));
        assert!(text.contains("buy"));
    }

    /// T1905 response parsing: usage fields map from
    /// `prompt_eval_count` / `eval_count`; `tokens_cached_in == 0`.
    #[test]
    fn t1905_parse_response_usage_mapping() {
        let req = base_request();
        let body = serde_json::json!({
            "model": "llama3:8b",
            "created_at": "2026-05-12T00:00:00Z",
            "message": {"role": "assistant", "content": "hello"},
            "done": true,
            "done_reason": "stop",
            "prompt_eval_count": 50,
            "eval_count": 12
        })
        .to_string();
        let resp = parse_response(&body, &req, &req.model).expect("parses");
        assert_eq!(resp.usage.tokens_in, 50);
        assert_eq!(resp.usage.tokens_out, 12);
        assert_eq!(resp.usage.tokens_cached_in, 0);
        assert!(matches!(&resp.content[0], ContentBlock::Text(t) if t == "hello"));
        assert_eq!(resp.stop_reason, StopReason::EndTurn);
    }

    /// T1905 best-effort tool-use: response JSON `{name, input}` is
    /// validated and surfaces as `ContentBlock::ToolUse`.
    #[test]
    fn t1905_best_effort_tool_use_happy_path() {
        let mut req = base_request();
        req.tools = vec![buy_tool()];
        let body = serde_json::json!({
            "model": "llama3:8b",
            "message": {
                "role": "assistant",
                "content": "{\"name\": \"buy\", \"input\": {\"symbol\": \"BTC\", \"qty\": 0.5}}"
            },
            "done": true,
            "done_reason": "stop",
            "prompt_eval_count": 30,
            "eval_count": 18
        })
        .to_string();
        let resp = parse_response(&body, &req, &req.model).expect("parses");
        match &resp.content[0] {
            ContentBlock::ToolUse { name, input, id } => {
                assert_eq!(name, "buy");
                assert_eq!(id, "ollama-best-effort");
                assert_eq!(input["qty"], 0.5);
            }
            other => panic!("expected ToolUse, got {other:?}"),
        }
        assert_eq!(resp.stop_reason, StopReason::ToolUse);
    }

    /// T1905 best-effort tool-use: a response that fails JSON-Schema
    /// validation surfaces as `InvalidResponse` with the
    /// "schema-mismatch" prefix.
    #[test]
    fn t1905_best_effort_tool_use_schema_mismatch() {
        let mut req = base_request();
        req.tools = vec![buy_tool()];
        // Missing required `qty` field.
        let body = serde_json::json!({
            "message": {
                "role": "assistant",
                "content": "{\"name\": \"buy\", \"input\": {\"symbol\": \"BTC\"}}"
            },
            "done": true,
            "prompt_eval_count": 1,
            "eval_count": 1
        })
        .to_string();
        let err = parse_response(&body, &req, &req.model).expect_err("rejected");
        match err {
            LlmError::InvalidResponse(msg) => {
                assert!(
                    msg.contains("schema-mismatch"),
                    "msg should mention schema-mismatch: {msg}"
                );
            }
            other => panic!("expected InvalidResponse, got {other:?}"),
        }
    }

    /// T1905 best-effort tool-use: non-JSON response surfaces as
    /// `InvalidResponse`.
    #[test]
    fn t1905_best_effort_tool_use_non_json() {
        let mut req = base_request();
        req.tools = vec![buy_tool()];
        let body = serde_json::json!({
            "message": {"role": "assistant", "content": "hello, world"},
            "done": true,
            "prompt_eval_count": 1,
            "eval_count": 1
        })
        .to_string();
        let err = parse_response(&body, &req, &req.model).expect_err("rejected");
        assert!(matches!(err, LlmError::InvalidResponse(_)));
    }

    /// `name()` and `provider_kind()` are stable.
    #[test]
    fn t1905_name_and_kind() {
        let p = OllamaProvider::new(ModelId::from("llama3:8b"));
        assert_eq!(p.name(), "ollama");
        assert_eq!(p.provider_kind(), ProviderKind::Other("ollama".into()));
    }
}
