#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T813 — R9 open-risks integration test.
//!
//! All-clear renders `_no open risks._`; fired risks render the
//! threshold + observed value; `Result::Err` cell renders `unknown
//! — see logs` per R9.3.

use reports::render::open_risks::{OpenRisksInputs, RiskOutcome, render};

fn clear() -> Result<RiskOutcome, String> {
    Ok(RiskOutcome {
        fired: false,
        threshold: String::new(),
        observed: String::new(),
    })
}

#[test]
fn t813_r9_all_clear_renders_no_open_risks_sentinel() {
    let inp = OpenRisksInputs {
        drawdown: clear(),
        llm_budget: clear(),
        strategy_decay: clear(),
        rebalance_rejections: clear(),
        mr_stops: clear(),
    };
    let body = render(&inp);
    assert!(body.contains("_no open risks._"));
}

#[test]
fn t813_r9_drawdown_fired_renders_threshold_and_observed() {
    let inp = OpenRisksInputs {
        drawdown: Ok(RiskOutcome {
            fired: true,
            threshold: "current_drawdown >= 11.25%".into(),
            observed: "12.10%".into(),
        }),
        llm_budget: clear(),
        strategy_decay: clear(),
        rebalance_rejections: clear(),
        mr_stops: clear(),
    };
    let body = render(&inp);
    assert!(body.contains(
        "- Drawdown approaching limit: threshold current_drawdown >= 11.25% (observed 12.10%)"
    ));
    assert!(!body.contains("_no open risks._"));
}

#[test]
fn t813_r9_err_cell_does_not_swallow_risk_renders_unknown() {
    let inp = OpenRisksInputs {
        drawdown: Err("query failed".into()),
        llm_budget: clear(),
        strategy_decay: clear(),
        rebalance_rejections: clear(),
        mr_stops: clear(),
    };
    let body = render(&inp);
    assert!(body.contains("- Drawdown approaching limit: unknown — see logs"));
    // R9.3 — even with one risk in unknown state, the binary still must
    // not be in the all-green path.
    assert!(!body.contains("_no open risks._"));
}
