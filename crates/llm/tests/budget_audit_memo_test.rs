#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T1912 (M5 enhancement, pass 4) — `BudgetedProvider::with_audit_ledger`
//! wires `audit::journal::post_llm_budget_event` (T1916) so every
//! debounced `Block` / `DegradeToQuickThink` event lands a structured
//! audit-memo row alongside the pre-existing `tracing::warn!` line.
//!
//! Tests cover both event kinds end-to-end (block + degrade) and assert
//! the legacy `BudgetedProvider::new` path stays warn-only (no audit
//! row written when no ledger was wired).

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use audit::{bootstrap, Ledger};
use cost::{AgentRole, CostBudget, CostSink, LlmTier, NoopCostSink};
use llm::{
    BudgetedProvider, ChatMessage, ChatRequest, ChatResponse, ContentBlock, LlmConfig, LlmError,
    LlmProvider, MessageRole, ModelId, ProviderKind, StopReason, TokenUsage,
};
use rust_decimal_macros::dec;
use uuid::Uuid;

#[derive(Default)]
struct MockProvider {
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
    async fn complete(&self, _req: ChatRequest) -> Result<ChatResponse, LlmError> {
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

async fn open_ledger() -> Arc<Ledger> {
    let ledger = Ledger::in_memory().await.expect("open ledger");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap chart of accounts");
    Arc::new(ledger)
}

/// Read the descriptions of every `llm_budget:*` row in the ledger.
async fn fetch_budget_memo_descriptions(ledger: &Ledger) -> Vec<String> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT description FROM journal_transactions \
         WHERE description LIKE 'llm_budget:%' ORDER BY ts",
    )
    .fetch_all(ledger.pool())
    .await
    .expect("select memo descriptions");
    rows.into_iter().map(|(d,)| d).collect()
}

/// T1912 enhancement (a) — `Block` event posts a
/// `llm_budget:budget_block` audit row when ledger is wired.
#[tokio::test]
async fn t1912_audit_memo_block_lands_with_ledger() {
    let ledger = open_ledger().await;
    let budget = Arc::new(CostBudget::new(dec!(200.00)));
    budget.add_spend(dec!(200.01)); // mode_override() → None (Block)
    let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);
    let cfg = Arc::new(LlmConfig::default());

    let mock = MockProvider::default();
    let bp = BudgetedProvider::with_audit_ledger(
        mock,
        Arc::clone(&budget),
        sink,
        cfg,
        Arc::clone(&ledger),
    );

    let req = make_request(LlmTier::DeepThink, "claude-opus-4-7");
    let err = bp.complete(req).await.expect_err("should block");
    assert!(matches!(err, LlmError::BudgetExceeded { .. }));

    // Spawn task takes a beat to land — wait briefly.
    tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;

    let descriptions = fetch_budget_memo_descriptions(&ledger).await;
    assert_eq!(
        descriptions,
        vec!["llm_budget:budget_block".to_string()],
        "block event must post one R11.1-tagged audit memo"
    );
}

/// T1912 enhancement (b) — `Degrade` event posts a
/// `llm_budget:budget_degrade_to_quick_think` audit row when ledger is
/// wired.
#[tokio::test]
async fn t1912_audit_memo_degrade_lands_with_ledger() {
    let ledger = open_ledger().await;
    let budget = Arc::new(CostBudget::new(dec!(200.00)));
    budget.add_spend(dec!(179.99)); // 89.995% → degrade to QuickThink
    let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);
    let cfg = Arc::new(LlmConfig::default());

    let mock = MockProvider::default();
    *mock.response.lock().unwrap() = Some(ChatResponse {
        content: vec![ContentBlock::Text("ok".into())],
        stop_reason: StopReason::EndTurn,
        usage: TokenUsage {
            tokens_in: 10,
            tokens_out: 5,
            tokens_cached_in: 0,
        },
        model: ModelId::new("claude-haiku-4-5-20251001"),
        correlation_id: Uuid::nil(),
    });

    let bp = BudgetedProvider::with_audit_ledger(
        mock,
        Arc::clone(&budget),
        sink,
        cfg,
        Arc::clone(&ledger),
    );

    let req = make_request(LlmTier::DeepThink, "claude-opus-4-7");
    bp.complete(req).await.expect("degraded call should succeed");

    tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;

    let descriptions = fetch_budget_memo_descriptions(&ledger).await;
    assert_eq!(
        descriptions,
        vec!["llm_budget:budget_degrade_to_quick_think".to_string()],
        "degrade event must post one R11.1-tagged audit memo"
    );
}

/// T1912 enhancement (c) — legacy `BudgetedProvider::new` (no ledger
/// wired) keeps the warn-only path: NO audit memo row.
#[tokio::test]
async fn t1912_no_audit_memo_when_ledger_absent() {
    // Use an out-of-band ledger purely to read its (empty) state.
    let ledger = open_ledger().await;

    let budget = Arc::new(CostBudget::new(dec!(200.00)));
    budget.add_spend(dec!(200.01));
    let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);
    let cfg = Arc::new(LlmConfig::default());

    let mock = MockProvider::default();
    let bp = BudgetedProvider::new(mock, Arc::clone(&budget), sink, cfg);

    let req = make_request(LlmTier::DeepThink, "claude-opus-4-7");
    let _ = bp.complete(req).await;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let descriptions = fetch_budget_memo_descriptions(&ledger).await;
    assert!(
        descriptions.is_empty(),
        "legacy ::new (no ledger wired) must NOT post audit memos; got {descriptions:?}"
    );
}
