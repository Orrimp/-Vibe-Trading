#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T1917 — LLM cost-event token-tag plumbing acceptance (audit half).
//!
//! Acceptance criterion from `spec/v1/v2-llm-strategy/tasks.md` (M5 T1917):
//! > fires one LLM cost event through `LedgerCostSink` with tokens
//! > (1000, 200, 500); reads back the journal entry meta and asserts
//! > the token fields round-trip. \[R9.1, R9.4\]
//!
//! The cost-crate half (the `LedgerCostSink::record(CostEvent::Llm { … })`
//! end-to-end test) lives at `crates/cost/tests/ledger_sink_llm_meta_test.rs`
//! because `audit` cannot depend on `cost` (cost → audit is the existing
//! direction). This file pins the direct `audit::journal::post_cost_llm`
//! contract so the audit crate's surface is testable in isolation.

use audit::{Ledger, bootstrap, journal};
use rust_decimal_macros::dec;
use uuid::Uuid;

async fn open_ledger() -> Ledger {
    let ledger = Ledger::in_memory().await.expect("open in-memory ledger");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap chart of accounts");
    ledger
}

/// Read the (single) `journal_transactions.metadata` row for the most-
/// recent `llm_cost:%` description; returns the parsed JSON or panics.
async fn fetch_only_llm_cost_meta(ledger: &Ledger) -> serde_json::Value {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT metadata FROM journal_transactions \
         WHERE description LIKE 'llm_cost:%' ORDER BY ts DESC",
    )
    .fetch_all(ledger.pool())
    .await
    .expect("select llm_cost metadata rows");
    assert_eq!(
        rows.len(),
        1,
        "expected exactly one llm_cost row; got {}",
        rows.len()
    );
    serde_json::from_str(&rows[0].0).expect("metadata is JSON")
}

/// T1917 — `post_cost_llm(tier, usd, 1000, 200, 500, uuid)` writes the
/// four token / correlation fields into the metadata JSON.
#[tokio::test]
async fn t1917_post_cost_llm_writes_token_meta() {
    let ledger = open_ledger().await;

    let corr_id = Uuid::new_v4();
    journal::post_cost_llm(
        &ledger,
        "deep_think",
        dec!(0.05),
        1_000, // tokens_in
        200,   // tokens_out
        500,   // tokens_cached_in
        corr_id,
    )
    .await
    .expect("post_cost_llm ok");

    let meta = fetch_only_llm_cost_meta(&ledger).await;
    assert_eq!(meta["tokens_in"], 1_000);
    assert_eq!(meta["tokens_out"], 200);
    assert_eq!(meta["tokens_cached_in"], 500);
    assert_eq!(meta["correlation_id"], corr_id.to_string());
}

/// T1917 — the legacy 3-arg `post_cost` still compiles and writes zero
/// tokens (backwards-compat: existing non-LLM callers stay green).
#[tokio::test]
async fn t1917_legacy_post_cost_writes_zero_tokens() {
    let ledger = open_ledger().await;

    journal::post_cost(&ledger, "deep_think", dec!(0.01))
        .await
        .expect("legacy post_cost ok");

    let meta = fetch_only_llm_cost_meta(&ledger).await;
    assert_eq!(meta["tokens_in"], 0);
    assert_eq!(meta["tokens_out"], 0);
    assert_eq!(meta["tokens_cached_in"], 0);
    // correlation_id is Uuid::nil() in the legacy wrapper.
    assert_eq!(meta["correlation_id"], Uuid::nil().to_string());
}

/// T1917 — feeding the T1910 `cache_hit_ratio_since` query through a
/// `post_cost_llm`-written ledger returns the right ratio. This pins the
/// integration between T1917 (writer) and T1910 (reader).
#[tokio::test]
async fn t1917_post_cost_llm_feeds_cache_hit_ratio_since() {
    use audit::query;
    use time::OffsetDateTime;
    use trading_core::Timestamp;

    let ledger = open_ledger().await;

    // Three events, each (tokens_in = 1000, tokens_cached_in = 500) →
    // ratio = 1500 / 3000 = 0.5 (same shape as T1910's acceptance).
    for _ in 0..3 {
        journal::post_cost_llm(
            &ledger,
            "deep_think",
            dec!(0.05),
            1_000,
            200,
            500,
            Uuid::new_v4(),
        )
        .await
        .expect("post_cost_llm ok");
    }

    let since = Timestamp::new(OffsetDateTime::UNIX_EPOCH);
    let ratio = query::cache_hit_ratio_since(&ledger, since)
        .await
        .expect("cache_hit_ratio_since ok");
    assert_eq!(ratio, dec!(0.5));
}
