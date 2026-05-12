#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T1917 — `LedgerCostSink::record(CostEvent::Llm { … })` end-to-end
//! acceptance.
//!
//! Acceptance criterion from `spec/v2-llm-strategy/tasks.md` (M5 T1917):
//! > fires one LLM cost event through `LedgerCostSink` with tokens
//! > (1000, 200, 500); reads back the journal entry meta and asserts
//! > the token fields round-trip. \[R9.1, R9.4\]
//!
//! This is the cost-crate half of the T1917 contract — the audit-crate
//! half pins `journal::post_cost_llm` in isolation
//! (`crates/audit/tests/llm_cost_meta_test.rs`); together they prove the
//! `CostEvent::Llm.tokens_*` fields land on the
//! `journal_transactions.metadata` JSON column so T1910's
//! `cache_hit_ratio_since` query returns the right number.

use std::sync::Arc;

use audit::{bootstrap, Ledger};
use cost::{AgentRole, CostEvent, CostSink, LedgerCostSink, LlmTier, ProviderKind};
use rust_decimal_macros::dec;
use uuid::Uuid;

async fn open_ledger() -> Arc<Ledger> {
    let ledger = Ledger::in_memory().await.expect("open in-memory ledger");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap chart of accounts");
    Arc::new(ledger)
}

/// T1917 acceptance — `LedgerCostSink::record(CostEvent::Llm { … })`
/// round-trips token fields through `journal_transactions.metadata`.
#[tokio::test]
async fn t1917_sink_llm_meta_round_trips() {
    let ledger = open_ledger().await;
    let sink = LedgerCostSink::new(Arc::clone(&ledger));

    let corr_id = Uuid::new_v4();
    let event = CostEvent::Llm {
        provider: ProviderKind::Anthropic,
        model: "claude-opus-4-7".to_string(),
        tier: LlmTier::DeepThink,
        role: AgentRole::Trader,
        tokens_in: 1_000,
        tokens_out: 200,
        tokens_cached_in: 500,
        usd: dec!(0.05),
        correlation_id: corr_id,
    };
    sink.record(event).expect("sink record ok");

    // Fire-and-forget — let the spawned writer land.
    tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;

    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT metadata FROM journal_transactions \
         WHERE description LIKE 'llm_cost:%'",
    )
    .fetch_all(ledger.pool())
    .await
    .expect("select metadata");
    assert_eq!(rows.len(), 1, "exactly one llm_cost row should land");

    let meta: serde_json::Value = serde_json::from_str(&rows[0].0).expect("metadata is JSON");
    assert_eq!(meta["tokens_in"], 1_000);
    assert_eq!(meta["tokens_out"], 200);
    assert_eq!(meta["tokens_cached_in"], 500);
    assert_eq!(meta["correlation_id"], corr_id.to_string());
}
