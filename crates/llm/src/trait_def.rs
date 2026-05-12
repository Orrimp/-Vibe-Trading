//! `LlmProvider` trait + request / response types (R1, Q4 a-c).
//!
//! All types are `Serialize + Deserialize + Clone + Debug + PartialEq` so
//! the record/replay layer (R6) and the audit memo renderer (R11.1) can
//! round-trip them without bespoke encoders.

use async_trait::async_trait;
use cost::{AgentRole, LlmTier};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::LlmError;
use crate::tools::ToolSchema;
use crate::ProviderKind;

/// Newtype around a provider-specific model identifier (e.g.
/// `claude-3-5-sonnet-20241022`, `gpt-4o-mini`, `llama3:8b`).
///
/// Kept as an opaque `String` because each provider's model registry has
/// its own naming rules; pricing lookups (`crates/llm/src/pricing.rs`,
/// T1906) do the `(ProviderKind, ModelId) → rate-card` match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelId(pub String);

impl ModelId {
    /// Construct a `ModelId` from a string slice.
    #[must_use]
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// View the underlying string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for ModelId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for ModelId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::fmt::Display for ModelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Cache-breakpoint marker on a [`SystemBlock`].
///
/// At v2.0.0 only `Ephemeral` (5-minute Anthropic TTL) is supported. The
/// provider impls translate this to provider-specific wire format
/// (Anthropic: `cache_control: {"type": "ephemeral"}`; OpenAI / Ollama:
/// silently flattened to plain text — T1903 / T1904 / T1905).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheBreakpoint {
    /// Anthropic-style 5-minute TTL cache marker.
    Ephemeral,
}

/// A single segment of the system prompt, optionally cache-marked.
///
/// Composed by the `CachedSystemPrompt` builder (T1906) into a
/// `Vec<SystemBlock>` carried by [`ChatRequest::system`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemBlock {
    /// Plain text — no cache control.
    Plain(String),
    /// Text with a cache breakpoint marker after it.
    Cached(String, CacheBreakpoint),
}

/// Conversational role of a [`ChatMessage`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    /// Operator / orchestrator turn.
    User,
    /// Model turn.
    Assistant,
}

/// A response or message-content fragment.
///
/// Both `ChatRequest::messages[*].content` and `ChatResponse::content`
/// are `Vec<ContentBlock>` so tool-use payloads and text fragments can be
/// interleaved in the same turn (Anthropic returns them as separate
/// blocks; OpenAI surfaces tool-calls as a sibling field, normalized into
/// `ContentBlock::ToolUse` by the OpenAI provider impl in T1904).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentBlock {
    /// Plain text content.
    Text(String),
    /// A structured tool-use invocation matching one of the
    /// [`ChatRequest::tools`] schemas.
    ToolUse {
        /// Name of the tool — matches `ToolSchema::name`.
        name: String,
        /// Tool-call arguments, validated against `ToolSchema::input_schema`
        /// by [`crate::tools::validate_tool_use`].
        input: serde_json::Value,
        /// Provider-issued correlation id for this tool-use invocation.
        id: String,
    },
}

/// A single message in the conversation history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Conversational role.
    pub role: MessageRole,
    /// One or more content blocks (interleaved text / tool-use).
    pub content: Vec<ContentBlock>,
}

/// Why the provider stopped generating.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// Model finished its turn naturally.
    EndTurn,
    /// `max_tokens` reached.
    MaxTokens,
    /// Model emitted a tool-use block and stopped.
    ToolUse,
    /// One of the configured stop sequences matched.
    StopSequence,
}

/// Token-usage breakdown for cost / cache observability.
///
/// Field shape mirrors `cost::CostEvent::Llm` so the cost sink (R10) can
/// copy fields one-for-one without translation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Total uncached input tokens billed.
    pub tokens_in: u64,
    /// Output tokens billed.
    pub tokens_out: u64,
    /// Input tokens served from cache (subset of `tokens_in` semantics
    /// varies per provider; Anthropic reports this in
    /// `cache_read_input_tokens`).
    pub tokens_cached_in: u64,
}

/// A request to [`LlmProvider::complete`].
///
/// Construct via [`ChatRequest::new`] (sensible defaults) then mutate
/// fields. The `tools` field is non-`Option` by design — an empty `Vec`
/// is the "free-text response" signal, so consumers can't forget to
/// pass tools at the type level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatRequest {
    /// Provider-specific model identifier.
    pub model: ModelId,
    /// Tier driving deep- vs. quick-think routing (re-uses `cost::LlmTier`).
    pub tier: LlmTier,
    /// Agent role attributing the cost (re-uses `cost::AgentRole`).
    pub role: AgentRole,
    /// Layered system prompt with optional cache breakpoints.
    pub system: Vec<SystemBlock>,
    /// Conversation history.
    pub messages: Vec<ChatMessage>,
    /// Available tools (empty `Vec` = free-text response).
    pub tools: Vec<ToolSchema>,
    /// Maximum tokens the model may emit.
    pub max_tokens: u32,
    /// Optional sampling temperature.
    pub temperature: Option<f32>,
    /// Operator-side correlation id, echoed in `ChatResponse`.
    pub correlation_id: Uuid,
}

impl ChatRequest {
    /// Construct a `ChatRequest` with sensible defaults: `max_tokens =
    /// 4096`, `temperature = None`, empty `tools / system / messages`,
    /// a fresh `correlation_id`.
    ///
    /// Mutate the returned value to set tools / system prompt / messages.
    #[must_use]
    pub fn new(model: ModelId, tier: LlmTier, role: AgentRole) -> Self {
        Self {
            model,
            tier,
            role,
            system: Vec::new(),
            messages: Vec::new(),
            tools: Vec::new(),
            max_tokens: 4096,
            temperature: None,
            correlation_id: Uuid::new_v4(),
        }
    }
}

/// A response from [`LlmProvider::complete`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatResponse {
    /// One or more content blocks — text and / or tool-use invocations.
    pub content: Vec<ContentBlock>,
    /// Why the model stopped generating.
    pub stop_reason: StopReason,
    /// Token-usage breakdown for cost / cache observability.
    pub usage: TokenUsage,
    /// Model the provider actually served — may differ from the request's
    /// `model` for OpenAI-compatible providers that route across models.
    pub model: ModelId,
    /// Echoed from the request.
    pub correlation_id: Uuid,
}

/// The trait every LLM provider implements.
///
/// Async because every consumer is a tokio task and reqwest is async-only.
/// Non-streaming at v2.0.0 (streaming is a v3 follow-up brief —
/// `v2-llm-streaming`). Tool-use is mandatory at the type level via
/// `ChatRequest::tools`.
///
/// Consumers receive `Arc<dyn LlmProvider>` from
/// `LlmProviderFactory::build` (T1913); they never construct providers
/// directly and never touch credentials.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Stable lowercase name of the provider (`"anthropic"`,
    /// `"openai"`, `"ollama"`, …). Used in log / metric labels.
    fn name(&self) -> &str;

    /// `ProviderKind` enum value driving downstream
    /// `match (ProviderKind, ModelId)` decisions (pricing lookup,
    /// cache-marker translation, ...).
    fn provider_kind(&self) -> ProviderKind;

    /// Issue a non-streaming completion request.
    async fn complete(&self, request: ChatRequest) -> Result<ChatResponse, LlmError>;
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// T1901 acceptance (a): `ChatRequest::new(model, tier, role)` builds
    /// with sensible defaults.
    #[test]
    fn t1901_chat_request_new_has_sensible_defaults() {
        let req = ChatRequest::new(
            ModelId::from("test-model"),
            LlmTier::DeepThink,
            AgentRole::Trader,
        );

        assert_eq!(req.model, ModelId::new("test-model"));
        assert_eq!(req.tier, LlmTier::DeepThink);
        assert_eq!(req.role, AgentRole::Trader);
        assert_eq!(req.max_tokens, 4096);
        assert_eq!(req.temperature, None);
        assert!(req.tools.is_empty());
        assert!(req.system.is_empty());
        assert!(req.messages.is_empty());

        // Two fresh requests must produce distinct correlation IDs.
        let req2 = ChatRequest::new(
            ModelId::from("test-model"),
            LlmTier::DeepThink,
            AgentRole::Trader,
        );
        assert_ne!(req.correlation_id, req2.correlation_id);
    }

    /// Smoke-test that all shape types round-trip through serde JSON so the
    /// record/replay layer (R6) can serialize them without bespoke encoders.
    #[test]
    fn t1901_shape_types_round_trip_through_serde_json() {
        let req = ChatRequest {
            model: ModelId::from("claude-3-5-sonnet-20241022"),
            tier: LlmTier::DeepThink,
            role: AgentRole::Trader,
            system: vec![
                SystemBlock::Plain("project context".to_string()),
                SystemBlock::Cached("role context".to_string(), CacheBreakpoint::Ephemeral),
            ],
            messages: vec![ChatMessage {
                role: MessageRole::User,
                content: vec![ContentBlock::Text("hello".to_string())],
            }],
            tools: vec![],
            max_tokens: 256,
            temperature: Some(0.7),
            correlation_id: Uuid::nil(),
        };
        let json = serde_json::to_string(&req).expect("serialize ChatRequest");
        let back: ChatRequest = serde_json::from_str(&json).expect("deserialize ChatRequest");
        assert_eq!(back, req);

        let resp = ChatResponse {
            content: vec![
                ContentBlock::Text("ok".to_string()),
                ContentBlock::ToolUse {
                    name: "buy".to_string(),
                    input: serde_json::json!({"symbol": "BTC"}),
                    id: "toolu_01".to_string(),
                },
            ],
            stop_reason: StopReason::ToolUse,
            usage: TokenUsage {
                tokens_in: 1000,
                tokens_out: 200,
                tokens_cached_in: 500,
            },
            model: ModelId::from("claude-3-5-sonnet-20241022"),
            correlation_id: Uuid::nil(),
        };
        let json = serde_json::to_string(&resp).expect("serialize ChatResponse");
        let back: ChatResponse = serde_json::from_str(&json).expect("deserialize ChatResponse");
        assert_eq!(back, resp);
    }
}
