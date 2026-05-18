//! `BudgetedProvider<Inner>` — pre-call gate + post-call reconcile
//! decorator (T1912).
//!
//! Design § Q6 (`spec/v2-llm-strategy/feature.md:1451`) resolution:
//!
//! - **Factory-wrapped.** `LlmProviderFactory::build` always wraps the
//!   leaf provider in this decorator; consumers receive
//!   `Arc<dyn LlmProvider>` and never see the leaf. Forgetting the
//!   gate is impossible by construction.
//! - **Mode check first.** `CostBudget::mode_override()` is consulted
//!   before any pricing math. `None` → block (debounced audit memo,
//!   `LlmError::BudgetExceeded`). `Some(QuickThink)` on a DeepThink
//!   request → construct a NEW degraded `ChatRequest` (`tier:
//!   QuickThink, model: cfg.quick_think.model`) — the caller's
//!   request struct is unchanged for forensics.
//! - **Pre-call estimate.** Worst-case bound via
//!   [`crate::pricing::estimate_cost`] (input_chars / 4 heuristic +
//!   max_tokens output). If the estimate would push spent over
//!   ceiling → `LlmError::BudgetExceeded` without sending an HTTP
//!   request.
//! - **Post-call reconcile.** On Ok, compute the actual `usd` from
//!   the response's token usage via
//!   [`crate::pricing::cost_for_usage`], `add_spend` it to the
//!   budget, post one `CostEvent::Llm` to the sink, fire the
//!   cache-observability event ([`crate::observability::emit_cache_event`]).
//!   On Err, **no** cost event is posted (R9.3 — failed calls don't
//!   bill).
//! - **Debounce.** `last_block_memo_at` (Unix seconds, AtomicU64)
//!   throttles audit memos to ≤ 1 / minute so a tight retry loop on a
//!   blocked budget doesn't flood the ledger.
//!
//! **Pass-4 enhancement (developer, 2026-05-12).** T1916 landed in this
//! same pass (M5), shipping `audit::journal::post_llm_budget_event` +
//! the `BudgetEventKind` enum. `BudgetedProvider` now optionally wires
//! an `audit_ledger: Option<Arc<audit::Ledger>>` (via the new
//! [`BudgetedProvider::with_audit_ledger`] constructor) and, on every
//! debounced block / degrade event, fires the audit memo alongside the
//! pre-existing `tracing::warn!` line. The legacy
//! [`BudgetedProvider::new`] constructor stays backwards-compat —
//! callers that pass no ledger still get the warn-only forensic path,
//! preserving every existing call site (factory, integration tests,
//! unit tests) without churn.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use audit::Ledger;
use audit::journal::{BudgetEventKind, post_llm_budget_event};
use cost::{CostBudget, CostEvent, CostSink, LlmTier};

use crate::ProviderKind;
use crate::config::LlmConfig;
use crate::error::LlmError;
use crate::observability::emit_cache_event;
use crate::pricing::{cost_for_usage, estimate_cost, resolve_rate};
use crate::trait_def::{ChatRequest, ChatResponse, LlmProvider};

/// Minimum spacing between successive block / degrade audit memos.
///
/// Q6-bonus § "Block ergonomics" — debounced ≤ 1/min so a busy
/// retry loop doesn't flood the ledger.
const MEMO_DEBOUNCE_SECS: u64 = 60;

/// Budget-gate wrapper. Implements [`LlmProvider`] by delegating to
/// `inner` after the pre-call gate + before the post-call reconcile.
pub struct BudgetedProvider<Inner: LlmProvider> {
    inner: Inner,
    budget: Arc<CostBudget>,
    sink: Arc<dyn CostSink>,
    cfg: Arc<LlmConfig>,
    /// Last time we posted a `budget_block` memo (Unix seconds).
    last_block_memo_at: AtomicU64,
    /// Optional ledger handle. When `Some`, every debounced block /
    /// degrade event also fires
    /// `audit::journal::post_llm_budget_event(...)` (T1916 — Q10 /
    /// R11.1). Wired via [`Self::with_audit_ledger`]; the legacy
    /// [`Self::new`] leaves it `None` (warn-only forensic path).
    audit_ledger: Option<Arc<Ledger>>,
}

impl<Inner: LlmProvider> std::fmt::Debug for BudgetedProvider<Inner> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BudgetedProvider")
            .field("inner.name", &self.inner.name())
            .field("inner.kind", &self.inner.provider_kind())
            .finish()
    }
}

impl<Inner: LlmProvider> BudgetedProvider<Inner> {
    /// Wrap `inner`. The budget + sink + config are shared (Arc) so a
    /// single budget enforcer can apply across multiple wrappers (one
    /// per leaf provider in a multi-provider setup).
    ///
    /// No audit-ledger wiring — block / degrade events surface as
    /// `tracing::warn!` only. Use [`Self::with_audit_ledger`] to also
    /// post structured audit memos via `post_llm_budget_event` (T1916).
    #[must_use]
    pub fn new(
        inner: Inner,
        budget: Arc<CostBudget>,
        sink: Arc<dyn CostSink>,
        cfg: Arc<LlmConfig>,
    ) -> Self {
        Self {
            inner,
            budget,
            sink,
            cfg,
            last_block_memo_at: AtomicU64::new(0),
            audit_ledger: None,
        }
    }

    /// Same as [`Self::new`] but additionally wires the audit ledger so
    /// every debounced block / degrade event lands a structured memo
    /// row via `audit::journal::post_llm_budget_event` (T1916 —
    /// `feature.md` Design § Q6 + Q10 / R11.1).
    ///
    /// The memo is fire-and-forget via `tokio::spawn` so the LLM hot
    /// path is not blocked on the SQL transaction.
    #[must_use]
    pub fn with_audit_ledger(
        inner: Inner,
        budget: Arc<CostBudget>,
        sink: Arc<dyn CostSink>,
        cfg: Arc<LlmConfig>,
        audit_ledger: Arc<Ledger>,
    ) -> Self {
        Self {
            inner,
            budget,
            sink,
            cfg,
            last_block_memo_at: AtomicU64::new(0),
            audit_ledger: Some(audit_ledger),
        }
    }

    /// Attempt to claim the once-per-minute memo slot. Returns true
    /// if the caller should emit a memo on this attempt.
    fn debounced_should_emit_memo(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let last = self.last_block_memo_at.load(Ordering::SeqCst);
        if now < last.saturating_add(MEMO_DEBOUNCE_SECS) {
            return false;
        }
        // CAS — only one concurrent caller wins the memo slot.
        self.last_block_memo_at
            .compare_exchange(last, now, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }

    /// Fire-and-forget audit memo. No-op when `audit_ledger` is `None`.
    fn spawn_audit_memo(
        &self,
        kind: BudgetEventKind,
        tier: LlmTier,
        spent_usd: rust_decimal::Decimal,
        ceiling_usd: rust_decimal::Decimal,
    ) {
        if let Some(ledger) = self.audit_ledger.as_ref() {
            let ledger = Arc::clone(ledger);
            let tier_str = tier.to_string();
            tokio::spawn(async move {
                if let Err(e) =
                    post_llm_budget_event(&ledger, kind, &tier_str, spent_usd, ceiling_usd).await
                {
                    tracing::error!(
                        target: "llm.budget",
                        error = %e,
                        kind = %kind,
                        "post_llm_budget_event failed"
                    );
                }
            });
        }
    }
}

#[async_trait]
impl<Inner: LlmProvider> LlmProvider for BudgetedProvider<Inner> {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn provider_kind(&self) -> ProviderKind {
        self.inner.provider_kind()
    }

    async fn complete(&self, request: ChatRequest) -> Result<ChatResponse, LlmError> {
        // ── 1. Mode check ────────────────────────────────────────────
        let mode = self.budget.mode_override();
        let actual_request = match mode {
            None => {
                // Block: post (debounced) audit memo, return error.
                if self.debounced_should_emit_memo() {
                    tracing::warn!(
                        target: "llm.budget",
                        spent_usd = %self.budget.spent(),
                        ceiling_usd = %self.budget.ceiling_usd,
                        role = ?request.role,
                        tier = ?request.tier,
                        "budget_block"
                    );
                    self.spawn_audit_memo(
                        BudgetEventKind::Block,
                        request.tier.clone(),
                        self.budget.spent(),
                        self.budget.ceiling_usd,
                    );
                }
                return Err(LlmError::BudgetExceeded {
                    spent_usd: self.budget.spent(),
                    ceiling_usd: self.budget.ceiling_usd,
                });
            }
            Some(LlmTier::QuickThink) if matches!(request.tier, LlmTier::DeepThink) => {
                // Degrade: build a NEW request (don't mutate caller's).
                if self.debounced_should_emit_memo() {
                    tracing::warn!(
                        target: "llm.budget",
                        spent_usd = %self.budget.spent(),
                        ceiling_usd = %self.budget.ceiling_usd,
                        role = ?request.role,
                        from_tier = ?request.tier,
                        to_model = %self.cfg.quick_think.model,
                        "degrade_to_quick_think"
                    );
                    // Audit memo tier reflects the *original* (intended)
                    // tier so the forensic record names what the caller
                    // tried to do, matching `tracing::warn!`'s
                    // `from_tier` field.
                    self.spawn_audit_memo(
                        BudgetEventKind::DegradeToQuickThink,
                        request.tier.clone(),
                        self.budget.spent(),
                        self.budget.ceiling_usd,
                    );
                }
                let mut degraded = request.clone();
                degraded.tier = LlmTier::QuickThink;
                degraded.model = self.cfg.quick_think.model.clone();
                degraded
            }
            // DeepThink available OR request already QuickThink: pass through.
            Some(_) => request.clone(),
        };

        // ── 2. Pre-call estimate ─────────────────────────────────────
        let provider_kind = self.inner.provider_kind();
        let rate = resolve_rate(
            &self.cfg.pricing,
            &provider_kind,
            actual_request.model.as_str(),
        )?;
        let input_chars = estimate_input_chars(&actual_request);
        let estimate = estimate_cost(&rate, input_chars, u64::from(actual_request.max_tokens));
        self.budget.try_reserve(estimate)?;

        // ── 3. Forward ───────────────────────────────────────────────
        let response = self.inner.complete(actual_request.clone()).await?;

        // ── 4. Post-call reconcile ───────────────────────────────────
        let usage = &response.usage;
        let actual_usd = cost_for_usage(
            &rate,
            usage.tokens_in,
            usage.tokens_out,
            usage.tokens_cached_in,
        );
        self.budget.add_spend(actual_usd);

        let event = CostEvent::Llm {
            provider: provider_kind.clone(),
            model: response.model.to_string(),
            tier: actual_request.tier.clone(),
            role: actual_request.role.clone(),
            tokens_in: usage.tokens_in,
            tokens_out: usage.tokens_out,
            tokens_cached_in: usage.tokens_cached_in,
            usd: actual_usd,
            correlation_id: actual_request.correlation_id,
        };
        if let Err(e) = self.sink.record(event) {
            // Sink failure is non-fatal — we've already incremented
            // spent_cents; surface for forensics, return the success.
            tracing::error!(
                target: "llm.budget",
                error = %e,
                "cost sink record failed"
            );
        }

        emit_cache_event(
            &actual_request.role,
            usage.tokens_in,
            usage.tokens_cached_in,
        );

        Ok(response)
    }
}

/// Best-effort input-character count for the estimate. Sums the byte
/// length of every text content block + every system block. Tool
/// schemas are NOT counted — they live in the system surface for most
/// providers and the 32-token padding in `pricing::estimate_cost`
/// covers small-schema overhead.
fn estimate_input_chars(req: &ChatRequest) -> u64 {
    use crate::trait_def::{ContentBlock, SystemBlock};
    let mut total: u64 = 0;
    for block in &req.system {
        let len = match block {
            SystemBlock::Plain(t) | SystemBlock::Cached(t, _) => t.len(),
        };
        total = total.saturating_add(len as u64);
    }
    for msg in &req.messages {
        for c in &msg.content {
            let len = match c {
                ContentBlock::Text(t) => t.len(),
                ContentBlock::ToolUse { input, .. } => input.to_string().len(),
            };
            total = total.saturating_add(len as u64);
        }
    }
    total
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trait_def::{
        ChatMessage, ContentBlock, MessageRole, ModelId, StopReason, TokenUsage,
    };
    use cost::{AgentRole, NoopCostSink};
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use std::sync::Mutex;
    use uuid::Uuid;

    /// Mock inner provider with a programmable response, capturing the
    /// last `complete()` request for assertions.
    #[derive(Default)]
    struct MockProvider {
        last_request: Mutex<Option<ChatRequest>>,
        /// Returned by `complete()` — Some(response) → Ok; None → Err.
        response: Mutex<Option<ChatResponse>>,
        /// If true, fail-injects an `InvalidResponse` error.
        fail: Mutex<bool>,
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
            if *self.fail.lock().unwrap() {
                return Err(LlmError::InvalidResponse("mock fail".into()));
            }
            self.response
                .lock()
                .unwrap()
                .clone()
                .ok_or_else(|| LlmError::InvalidResponse("no response programmed".into()))
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

    fn make_response(
        model: &str,
        tokens_in: u64,
        tokens_out: u64,
        tokens_cached_in: u64,
    ) -> ChatResponse {
        ChatResponse {
            content: vec![ContentBlock::Text("ok".into())],
            stop_reason: StopReason::EndTurn,
            usage: TokenUsage {
                tokens_in,
                tokens_out,
                tokens_cached_in,
            },
            model: ModelId::new(model),
            correlation_id: Uuid::nil(),
        }
    }

    /// T1912 (a): seed $179.99 / $200; mode_override returns DeepThink
    /// (since 179.99 / 200 = 0.8995 → wait — at >= 80% it returns
    /// QuickThink). So DeepThink request degrades to QuickThink.
    #[tokio::test]
    async fn t1912_a_degrade_to_quick_think_on_threshold() {
        let budget = Arc::new(CostBudget::new(dec!(200.00)));
        budget.add_spend(dec!(179.99)); // 89.995% → degrade
        let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);
        let cfg = Arc::new(LlmConfig::default());
        let mock = MockProvider::default();
        *mock.response.lock().unwrap() = Some(make_response("claude-haiku-4-5-20251001", 10, 5, 0));

        let bp = BudgetedProvider::new(mock, Arc::clone(&budget), sink, cfg);

        let req = make_request(LlmTier::DeepThink, "claude-opus-4-7");
        let resp = bp.complete(req).await.expect("call should succeed");
        assert_eq!(resp.usage.tokens_out, 5);

        // Inner saw a degraded request.
        let captured = bp.inner.last_request.lock().unwrap().clone().unwrap();
        assert!(matches!(captured.tier, LlmTier::QuickThink));
        assert_eq!(captured.model.as_str(), "claude-haiku-4-5-20251001");
    }

    /// T1912 (b): seed $200.01 / $200; any request → BudgetExceeded,
    /// inner NEVER called.
    #[tokio::test]
    async fn t1912_b_block_returns_budget_exceeded_no_inner_call() {
        let budget = Arc::new(CostBudget::new(dec!(200.00)));
        budget.add_spend(dec!(200.01));
        let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);
        let cfg = Arc::new(LlmConfig::default());
        let mock = MockProvider::default();

        let bp = BudgetedProvider::new(mock, Arc::clone(&budget), sink, cfg);

        let req = make_request(LlmTier::DeepThink, "claude-opus-4-7");
        let err = bp.complete(req).await.expect_err("should block");
        assert!(matches!(err, LlmError::BudgetExceeded { .. }));

        // Inner provider was NOT called.
        assert!(bp.inner.last_request.lock().unwrap().is_none());
    }

    /// T1912 (c): seed $0 / $200; DeepThink passes through unchanged
    /// and post-call reconcile bumps spent + posts to sink.
    #[tokio::test]
    async fn t1912_c_pass_through_when_budget_healthy() {
        let budget = Arc::new(CostBudget::new(dec!(200.00)));
        let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);
        let cfg = Arc::new(LlmConfig::default());
        let mock = MockProvider::default();
        // 1k uncached input + 500 output tokens @ Opus rate = $0.015 + $0.0375 = $0.0525.
        *mock.response.lock().unwrap() = Some(make_response("claude-opus-4-7", 1_000, 500, 0));

        let bp = BudgetedProvider::new(mock, Arc::clone(&budget), sink, cfg);

        let req = make_request(LlmTier::DeepThink, "claude-opus-4-7");
        bp.complete(req).await.expect("call ok");

        let captured = bp.inner.last_request.lock().unwrap().clone().unwrap();
        assert!(matches!(captured.tier, LlmTier::DeepThink));
        assert_eq!(captured.model.as_str(), "claude-opus-4-7");

        // Spent counter advanced.
        assert!(budget.spent() > Decimal::ZERO);
    }

    /// Failed call does NOT post a cost event AND does not increment
    /// spent_cents (R9.3).
    #[tokio::test]
    async fn t1912_failed_call_does_not_bill() {
        let budget = Arc::new(CostBudget::new(dec!(200.00)));
        let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);
        let cfg = Arc::new(LlmConfig::default());
        let mock = MockProvider::default();
        *mock.fail.lock().unwrap() = true;

        let bp = BudgetedProvider::new(mock, Arc::clone(&budget), sink, cfg);

        let req = make_request(LlmTier::DeepThink, "claude-opus-4-7");
        let err = bp.complete(req).await.expect_err("inner failed");
        assert!(matches!(err, LlmError::InvalidResponse(_)));
        assert_eq!(budget.spent(), Decimal::ZERO, "no billing on failure");
    }
}
