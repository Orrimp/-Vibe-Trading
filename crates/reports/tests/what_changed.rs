#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T813 — R8 what-changed integration test.
//!
//! Verifies the lifecycle filter (R8.1) and the empty-period sentinel
//! (R8.3).

use reports::render::what_changed::{render, WhatChangedInputs};
use smol_str::SmolStr;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use trading_core::{StrategyEventKind, StrategyEventView, StrategyId, Timestamp};

fn ev(kind: StrategyEventKind, sid: &str, ts_str: &str) -> StrategyEventView {
    StrategyEventView {
        id: SmolStr::new("evt-1"),
        ts: Timestamp::new(OffsetDateTime::parse(ts_str, &Rfc3339).unwrap()),
        kind,
        strategy_id: Some(StrategyId::new(sid)),
        old_hash: None,
        new_hash: None,
        source_path: None,
        operator: SmolStr::new("system"),
        error_code: None,
        error_summary: None,
    }
}

#[test]
fn t813_r8_empty_period_renders_sentinel() {
    let inp = WhatChangedInputs { events: vec![] };
    let body = render(&inp);
    assert!(body.contains("_no strategy lifecycle events in this period._"));
}

#[test]
fn t813_r8_load_swap_chronological_order_with_strategy_id() {
    let load = ev(
        StrategyEventKind::Load,
        "alpha",
        "2026-04-29T00:00:00.000000Z",
    );
    let swap = ev(
        StrategyEventKind::Swap,
        "alpha",
        "2026-04-30T00:00:00.000000Z",
    );

    let inp = WhatChangedInputs {
        events: vec![load, swap],
    };
    let body = render(&inp);

    assert!(body.contains("[Load] strategy_id=alpha"));
    assert!(body.contains("[Swap] strategy_id=alpha"));
    let lp = body.find("[Load]").unwrap();
    let sp = body.find("[Swap]").unwrap();
    assert!(lp < sp);
}

#[test]
fn t813_r8_filters_non_lifecycle_events() {
    let rebal = ev(
        StrategyEventKind::RebalanceRejected,
        "beta",
        "2026-04-29T00:00:00.000000Z",
    );
    let feed = ev(
        StrategyEventKind::FeedReconnect,
        "gamma",
        "2026-04-30T00:00:00.000000Z",
    );
    let inp = WhatChangedInputs {
        events: vec![rebal, feed],
    };
    let body = render(&inp);
    // No lifecycle events in the input → sentinel.
    assert!(body.contains("_no strategy lifecycle events in this period._"));
}
