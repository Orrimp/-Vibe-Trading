//! `CostSink` trait and implementations (T30).
//!
//! v0: only `NoopCostSink` is used (zero LLM calls).
//! `LedgerCostSink` is fully wired so v0.5 drops in real LLM events.
use std::sync::Arc;

use rust_decimal::Decimal;
use trading_core::CostError;

use crate::event::CostEvent;

/// Receives cost events and records them.
pub trait CostSink: Send + Sync {
    fn record(&self, event: CostEvent) -> Result<(), CostError>;
}

/// No-op cost sink for v0 (no LLM calls, nothing to record).
pub struct NoopCostSink;

impl CostSink for NoopCostSink {
    fn record(&self, _event: CostEvent) -> Result<(), CostError> {
        Ok(())
    }
}

/// Cost sink that writes events as journal entries to the audit ledger (T30).
///
/// Writes to `expense:llm:<tier>` and `liabilities:llm_accrued`.
/// Because `record()` is sync but the ledger is async, this sink spawns a
/// fire-and-forget task per event.  In v0 this is never called.
pub struct LedgerCostSink {
    pub ledger: Arc<audit::Ledger>,
}

impl LedgerCostSink {
    /// Create a new `LedgerCostSink` backed by `ledger`.
    #[must_use]
    pub fn new(ledger: Arc<audit::Ledger>) -> Self {
        Self { ledger }
    }
}

impl CostSink for LedgerCostSink {
    fn record(&self, event: CostEvent) -> Result<(), CostError> {
        let usd = event.usd();
        if usd == Decimal::ZERO {
            return Ok(());
        }

        let ledger = Arc::clone(&self.ledger);

        // T1917 — `CostEvent::Llm` pulls the four token / correlation
        // fields and forwards them to `post_cost_llm` so the meta JSON
        // on the `journal_transactions` row carries the tokens the
        // T1910 `cache_hit_ratio_since` reader needs. Non-LLM events
        // fall through to the legacy 3-arg `post_cost` shape (zeros in
        // the meta JSON, which the reader treats as no contribution).
        match event {
            CostEvent::Llm {
                tier,
                tokens_in,
                tokens_out,
                tokens_cached_in,
                correlation_id,
                ..
            } => {
                let tier_str = tier.to_string();
                tokio::spawn(async move {
                    if let Err(e) = audit::journal::post_cost_llm(
                        &ledger,
                        &tier_str,
                        usd,
                        tokens_in,
                        tokens_out,
                        tokens_cached_in,
                        correlation_id,
                    )
                    .await
                    {
                        tracing::error!(error = %e, "LedgerCostSink: failed to write LLM journal entries");
                    }
                });
            }
            _ => {
                tokio::spawn(async move {
                    if let Err(e) = audit::journal::post_cost(&ledger, "other", usd).await {
                        tracing::error!(error = %e, "LedgerCostSink: failed to write journal entries");
                    }
                });
            }
        }

        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use uuid::Uuid;

    use crate::event::{AgentRole, LlmTier, ProviderKind};

    fn make_llm_event(usd: Decimal) -> CostEvent {
        CostEvent::Llm {
            provider: ProviderKind::Anthropic,
            model: "claude-3-opus".to_string(),
            tier: LlmTier::DeepThink,
            role: AgentRole::Trader,
            tokens_in: 1000,
            tokens_out: 200,
            tokens_cached_in: 500,
            usd,
            correlation_id: Uuid::new_v4(),
        }
    }

    #[tokio::test]
    async fn t30_ledger_sink_writes_balanced_entries() {
        let ledger = Arc::new(audit::Ledger::in_memory().await.unwrap());
        audit::bootstrap::chart_of_accounts(&ledger).await.unwrap();

        let sink = LedgerCostSink::new(Arc::clone(&ledger));

        // Emit 5 events summing to $0.50
        for _ in 0..5 {
            sink.record(make_llm_event(dec!(0.10))).unwrap();
        }

        // Allow fire-and-forget tasks to complete
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        // Global debits == global credits (balance preserved)
        let (total_dr, total_cr) = audit::query::global_debit_credit_sum(&ledger)
            .await
            .unwrap();
        let diff = (total_dr - total_cr).abs();
        assert!(
            diff <= dec!(0.00000001),
            "ledger should be balanced: dr={total_dr} cr={total_cr}"
        );
    }

    #[test]
    fn t30_noop_sink_accepts_events() {
        let sink = NoopCostSink;
        let event = make_llm_event(dec!(0.25));
        assert!(sink.record(event).is_ok());
    }
}
