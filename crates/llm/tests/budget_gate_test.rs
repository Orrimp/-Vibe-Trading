//! T1912 acceptance — `BudgetedProvider<Inner>` integration test.
//!
//! Three acceptance cases from `spec/v2-llm-strategy/tasks.md`:
//!
//! - (a) seed $179.99 / $200; DeepThink request → degraded to
//!   QuickThink + `cfg.quick_think.model`; warn line emits (assert
//!   via the inner mock seeing the degraded request).
//! - (b) seed $200.01 / $200; any request → `LlmError::BudgetExceeded`,
//!   inner provider NEVER called.
//! - (c) seed $0.00 / $200; DeepThink → passes through untouched,
//!   post-call reconcile bumps `budget.spent()`.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use cost::{AgentRole, CostBudget, CostSink, LlmTier, NoopCostSink};
use llm::{
    BudgetedProvider, ChatMessage, ChatRequest, ChatResponse, ContentBlock, LlmConfig, LlmError,
    LlmProvider, MessageRole, ModelId, ProviderKind, StopReason, TokenUsage,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use uuid::Uuid;

#[derive(Default)]
struct MockProvider {
    last_request: Mutex<Option<ChatRequest>>,
    response: Mutex<Option<ChatResponse>>,
}

#[async_trait]
impl LlmProvider for MockProvider {
    fn name(&self) -> &str {
        "mock"
    }
    fn provider_kind(&self) -> ProviderKind {
        ProviderKind::Anthropic
    }
    async fn complete(&self, req: ChatRequest) -> Result<ChatResponse, LlmError> {
        *self.last_request.lock().unwrap() = Some(req.clone());
        self.response
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| LlmError::InvalidResponse("no programmed response".into()))
    }
}

fn make_request(tier: LlmTier, model: &str) -> ChatRequest {
    let mut r = ChatRequest::new(ModelId::new(model), tier, AgentRole::Trader);
    r.max_tokens = 100;
    r.messages.push(ChatMessage {
        role: MessageRole::User,
        content: vec![ContentBlock::Text("hi".into())],
    });
    r
}

fn make_response(model: &str, tokens_in: u64, tokens_out: u64) -> ChatResponse {
    ChatResponse {
        content: vec![ContentBlock::Text("ok".into())],
        stop_reason: StopReason::EndTurn,
        usage: TokenUsage {
            tokens_in,
            tokens_out,
            tokens_cached_in: 0,
        },
        model: ModelId::new(model),
        correlation_id: Uuid::nil(),
    }
}

#[tokio::test]
async fn t1912_a_degrade_path_inner_sees_quick_think_model() {
    let budget = Arc::new(CostBudget::new(dec!(200.00)));
    budget.add_spend(dec!(179.99));
    let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);
    let cfg = Arc::new(LlmConfig::default());

    let mock = Arc::new(MockProvider::default());
    *mock.response.lock().unwrap() = Some(make_response("claude-haiku-4-5-20251001", 10, 5));

    // Wrap a *shared* mock: BudgetedProvider takes ownership of an
    // `Inner: LlmProvider`. To inspect post-call we use a wrapper that
    // delegates to the Arc.
    struct SharedMock(Arc<MockProvider>);
    #[async_trait]
    impl LlmProvider for SharedMock {
        fn name(&self) -> &str {
            self.0.name()
        }
        fn provider_kind(&self) -> ProviderKind {
            self.0.provider_kind()
        }
        async fn complete(&self, req: ChatRequest) -> Result<ChatResponse, LlmError> {
            self.0.complete(req).await
        }
    }

    let bp = BudgetedProvider::new(SharedMock(Arc::clone(&mock)), budget, sink, cfg);
    let req = make_request(LlmTier::DeepThink, "claude-opus-4-7");
    bp.complete(req).await.expect("ok");

    let captured = mock.last_request.lock().unwrap().clone().unwrap();
    assert!(
        matches!(captured.tier, LlmTier::QuickThink),
        "tier degraded"
    );
    assert_eq!(
        captured.model.as_str(),
        "claude-haiku-4-5-20251001",
        "model remapped to quick_think.model"
    );
}

#[tokio::test]
async fn t1912_b_block_returns_budget_exceeded_no_inner_call() {
    let budget = Arc::new(CostBudget::new(dec!(200.00)));
    budget.add_spend(dec!(200.01));
    let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);
    let cfg = Arc::new(LlmConfig::default());

    let mock = Arc::new(MockProvider::default());
    struct SharedMock(Arc<MockProvider>);
    #[async_trait]
    impl LlmProvider for SharedMock {
        fn name(&self) -> &str {
            self.0.name()
        }
        fn provider_kind(&self) -> ProviderKind {
            self.0.provider_kind()
        }
        async fn complete(&self, req: ChatRequest) -> Result<ChatResponse, LlmError> {
            self.0.complete(req).await
        }
    }

    let bp = BudgetedProvider::new(SharedMock(Arc::clone(&mock)), budget, sink, cfg);
    let req = make_request(LlmTier::DeepThink, "claude-opus-4-7");
    let err = bp.complete(req).await.expect_err("should block");
    assert!(matches!(err, LlmError::BudgetExceeded { .. }));
    assert!(
        mock.last_request.lock().unwrap().is_none(),
        "inner must not have been called"
    );
}

#[tokio::test]
async fn t1912_c_pass_through_when_budget_healthy() {
    let budget = Arc::new(CostBudget::new(dec!(200.00)));
    let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);
    let cfg = Arc::new(LlmConfig::default());

    let mock = Arc::new(MockProvider::default());
    *mock.response.lock().unwrap() = Some(make_response("claude-opus-4-7", 1_000, 500));

    struct SharedMock(Arc<MockProvider>);
    #[async_trait]
    impl LlmProvider for SharedMock {
        fn name(&self) -> &str {
            self.0.name()
        }
        fn provider_kind(&self) -> ProviderKind {
            self.0.provider_kind()
        }
        async fn complete(&self, req: ChatRequest) -> Result<ChatResponse, LlmError> {
            self.0.complete(req).await
        }
    }

    let bp = BudgetedProvider::new(
        SharedMock(Arc::clone(&mock)),
        Arc::clone(&budget),
        sink,
        cfg,
    );
    let req = make_request(LlmTier::DeepThink, "claude-opus-4-7");
    bp.complete(req).await.expect("ok");

    let captured = mock.last_request.lock().unwrap().clone().unwrap();
    assert!(
        matches!(captured.tier, LlmTier::DeepThink),
        "tier preserved"
    );
    assert_eq!(
        captured.model.as_str(),
        "claude-opus-4-7",
        "model preserved"
    );
    assert!(
        budget.spent() > Decimal::ZERO,
        "post-call reconcile must increment spent: got {}",
        budget.spent()
    );
}
