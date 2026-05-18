#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T1916 — `audit::journal::post_llm_budget_event` acceptance.
//!
//! Acceptance criterion from `spec/v2-llm-strategy/tasks.md` (M5 T1916):
//! > fires `post_llm_budget_event(...)` against an in-memory ledger;
//! > asserts (a) one row lands at `expense:llm:deep_think` with the
//! > expected tag, (b) global debit-credit sum balanced
//! > (Δ ≤ 1e-8). \[R11.1, Q10\]

use audit::{
    Ledger, bootstrap,
    journal::{BudgetEventKind, post_llm_budget_event},
    query,
};
use rust_decimal_macros::dec;

async fn open_ledger() -> Ledger {
    let ledger = Ledger::in_memory().await.expect("open in-memory ledger");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap chart of accounts");
    ledger
}

/// T1916 (a) — Block memo lands one row tagged
/// `expense:llm:deep_think` with `description = "llm_budget:budget_block"`.
#[tokio::test]
async fn t1916_a_block_memo_lands_on_expense_llm_deep_think() {
    let ledger = open_ledger().await;

    post_llm_budget_event(
        &ledger,
        BudgetEventKind::Block,
        "deep_think",
        dec!(200.01),
        dec!(200.00),
    )
    .await
    .expect("post_llm_budget_event Block");

    // The single transaction we just wrote has two zero-amount entries:
    // one on expense:llm:deep_think, one on liabilities:llm_accrued. Both
    // are debit=0/credit=0, so they don't move the global sum — but the
    // expense-side row MUST exist (R11.1: cockpit + reports grep here).
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT t.description, e.account_id, t.metadata \
         FROM journal_transactions t \
         JOIN journal_entries e ON e.transaction_id = t.id \
         WHERE t.description LIKE 'llm_budget:%' \
           AND e.account_id = 'expense:llm:deep_think'",
    )
    .fetch_all(ledger.pool())
    .await
    .expect("select memo row");

    assert_eq!(
        rows.len(),
        1,
        "exactly one expense-side memo row should exist; got {}",
        rows.len()
    );
    let (description, _account, metadata) = &rows[0];
    assert_eq!(
        description, "llm_budget:budget_block",
        "description must carry the R11.1 tag"
    );
    let meta: serde_json::Value = serde_json::from_str(metadata).expect("metadata is JSON");
    assert_eq!(meta["kind"], "budget_block");
    assert_eq!(meta["tier"], "deep_think");
    assert_eq!(meta["spent_usd"], "200.01");
    assert_eq!(meta["ceiling_usd"], "200.00");
}

/// T1916 (b) — Global debit-credit sum stays balanced after the memo
/// write (the memo writes Dr 0 / Cr 0 across two entries → Δ = 0).
#[tokio::test]
async fn t1916_b_global_dr_cr_sum_balanced_post_memo() {
    let ledger = open_ledger().await;

    // Bootstrap establishes the chart with zero balance; emit one memo.
    post_llm_budget_event(
        &ledger,
        BudgetEventKind::DegradeToQuickThink,
        "deep_think",
        dec!(180.00),
        dec!(200.00),
    )
    .await
    .expect("post_llm_budget_event Degrade");

    let (dr, cr) = query::global_debit_credit_sum(&ledger)
        .await
        .expect("read global sums");
    let delta = (dr - cr).abs();
    assert!(
        delta <= dec!(0.00000001),
        "reconciler invariant: Σdr - Σcr ≤ 1e-8 (got {delta})"
    );
}

/// `BudgetEventKind::Display` projects the canonical R11.1 tag strings.
#[test]
fn t1916_display_emits_r11_1_tags() {
    assert_eq!(
        BudgetEventKind::DegradeToQuickThink.to_string(),
        "budget_degrade_to_quick_think"
    );
    assert_eq!(BudgetEventKind::Block.to_string(), "budget_block");
}

/// Both memo kinds land balanced zero-amount entries; the description
/// discriminates without re-parsing metadata.
#[tokio::test]
async fn t1916_both_kinds_round_trip_through_journal() {
    let ledger = open_ledger().await;

    post_llm_budget_event(
        &ledger,
        BudgetEventKind::DegradeToQuickThink,
        "deep_think",
        dec!(160.00),
        dec!(200.00),
    )
    .await
    .expect("post Degrade");

    post_llm_budget_event(
        &ledger,
        BudgetEventKind::Block,
        "quick_think",
        dec!(201.00),
        dec!(200.00),
    )
    .await
    .expect("post Block");

    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT description FROM journal_transactions \
         WHERE description LIKE 'llm_budget:%' ORDER BY description",
    )
    .fetch_all(ledger.pool())
    .await
    .expect("select descriptions");

    let descriptions: Vec<String> = rows.into_iter().map(|(d,)| d).collect();
    assert_eq!(
        descriptions,
        vec![
            "llm_budget:budget_block".to_string(),
            "llm_budget:budget_degrade_to_quick_think".to_string(),
        ],
        "both tags should round-trip"
    );
}
