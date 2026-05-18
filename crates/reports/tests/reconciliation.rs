#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T808 — reconciliation engine integration tests.
//!
//! Three cases per the acceptance criterion:
//!
//! 1. All-zero deltas → `all_passed == true`.
//! 2. One-cent injected delta in one row → `all_passed == false`,
//!    only that row's `passed == false`.
//! 3. `to_failure_json` round-trips through `serde_json::Value`.

use reports::{ReconciliationInputs, reconcile};
use rust_decimal_macros::dec;

fn balanced() -> ReconciliationInputs {
    ReconciliationInputs {
        headline_return: dec!(150.00),
        realized: dec!(100.00),
        unrealized: dec!(50.00),
        sum_by_strategy: dec!(100.00),
        sum_by_symbol: dec!(100.00),
        equity_delta: dec!(140.00),
        equity_check_sum: dec!(140.00),
    }
}

#[test]
fn t808_case_1_all_zero_deltas_all_passed_true() {
    let r = reconcile::compute(&balanced());
    assert!(r.all_passed());
    assert!(r.headline.passed);
    assert!(r.by_strategy.passed);
    assert!(r.by_symbol.passed);
    assert!(r.equity.passed);
    for row in [&r.headline, &r.by_strategy, &r.by_symbol, &r.equity] {
        assert_eq!(row.delta, rust_decimal::Decimal::ZERO);
    }
}

#[test]
fn t808_case_2_one_cent_imbalance_only_that_row_fails() {
    // Inject a one-cent delta into the headline identity.
    let mut inp = balanced();
    inp.headline_return = dec!(150.01);
    let r = reconcile::compute(&inp);

    assert!(!r.all_passed());
    assert!(!r.headline.passed);
    assert_eq!(r.headline.delta, dec!(0.01));

    // Other three rows still pass.
    assert!(r.by_strategy.passed);
    assert!(r.by_symbol.passed);
    assert!(r.equity.passed);
}

#[test]
fn t808_case_3_to_failure_json_round_trips_through_serde_value() {
    let mut inp = balanced();
    inp.headline_return = dec!(150.05);
    inp.sum_by_strategy = dec!(99.99);
    let r = reconcile::compute(&inp);

    let body = r.to_failure_json(
        "deadbeefcafebabe",
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "7d",
        "2026-04-24T00:00:00.000000Z",
        "2026-05-01T00:00:00.000000Z",
    );

    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["run_id"], "deadbeefcafebabe");
    assert_eq!(v["period"], "7d");
    assert_eq!(v["period_start"], "2026-04-24T00:00:00.000000Z");
    assert_eq!(v["period_end"], "2026-05-01T00:00:00.000000Z");

    let rows = v["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 4);
    // headline failed, by_strategy failed, by_symbol passed, equity passed.
    assert_eq!(rows[0]["passed"], false);
    assert_eq!(rows[1]["passed"], false);
    assert_eq!(rows[2]["passed"], true);
    assert_eq!(rows[3]["passed"], true);
    // Deltas are TEXT-form decimals (no scientific notation).
    assert_eq!(rows[0]["delta"], "0.05");
    assert_eq!(rows[1]["delta"], "-0.01");
}
