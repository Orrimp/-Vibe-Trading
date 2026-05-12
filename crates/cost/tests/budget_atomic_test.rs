//! T1907 acceptance — `CostBudget` atomic-cents refactor.
//!
//! Verifies the four T1907 acceptance criteria from
//! `spec/v2-llm-strategy/tasks.md`:
//!
//! - (a) seed budget at $179.99 / $200, `try_reserve(0.01)` returns Ok.
//! - (b) seed at $200.01, `try_reserve(any)` returns BudgetExceeded.
//! - (c) 100 parallel `add_spend(0.10)` calls produce final spent ==
//!   $10.00 (1000 cents — no torn writes).
//! - (d) `remaining()` reads consistent.

use std::sync::Arc;

use cost::{BudgetError, CostBudget};
use rust_decimal_macros::dec;

#[test]
fn t1907_a_within_ceiling_reservation_ok() {
    let b = CostBudget::new(dec!(200.00));
    b.add_spend(dec!(179.99));
    assert!(
        b.try_reserve(dec!(0.01)).is_ok(),
        "$179.99 + $0.01 estimate ≤ $200 ceiling — should reserve cleanly"
    );
}

#[test]
fn t1907_b_over_ceiling_returns_budget_exceeded() {
    let b = CostBudget::new(dec!(200.00));
    b.add_spend(dec!(200.01));
    let err = b
        .try_reserve(dec!(0.00))
        .expect_err("over-ceiling spend should fail any reservation");
    let BudgetError::BudgetExceeded {
        spent_usd,
        ceiling_usd,
    } = err;
    assert_eq!(spent_usd, dec!(200.01));
    assert_eq!(ceiling_usd, dec!(200.00));
}

#[test]
fn t1907_c_parallel_add_spend_no_torn_writes() {
    let b = Arc::new(CostBudget::new(dec!(1_000.00)));
    let mut handles = Vec::new();
    for _ in 0..100 {
        let b2 = Arc::clone(&b);
        handles.push(std::thread::spawn(move || {
            b2.add_spend(dec!(0.10));
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    assert_eq!(b.spent(), dec!(10.00), "100 × $0.10 == $10.00 exact");
}

#[test]
fn t1907_d_remaining_reads_consistent() {
    let b = CostBudget::new(dec!(200.00));
    assert_eq!(b.remaining(), dec!(200.00));
    b.add_spend(dec!(50.00));
    assert_eq!(b.remaining(), dec!(150.00));
    b.add_spend(dec!(0.00));
    assert_eq!(b.remaining(), dec!(150.00));
}
