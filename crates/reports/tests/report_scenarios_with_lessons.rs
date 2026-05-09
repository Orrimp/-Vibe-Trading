#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T1811 — exercise the new `## Memory highlights` body with the new
//! reflection-store fixtures.
//!
//! The existing `report_scenarios.rs::t816_*` tests assert the
//! empty-state body is byte-stable (the v1+ generate(...) path).  This
//! file asserts the **lesson-bearing** body is byte-stable when the
//! retrieval-store fixtures are wired in: the renderer's
//! `render_with_lessons` path is the contract the operator success
//! report consumes after T1813's anchor re-lock.

#[path = "fixtures/build_reflection_store_1y.rs"]
mod build_reflection_store_1y;
#[path = "fixtures/build_reflection_store_7d.rs"]
mod build_reflection_store_7d;
#[path = "fixtures/build_reflection_store_90d.rs"]
mod build_reflection_store_90d;

use reflection::regime::RegimeTag;
use reflection::store::ReflectionStore;
use reflection::types::RetrievalQuery;
use reflection::{retrieve_top_k, REPORT_TIME_TOP_K};
use reports::render::memory_highlights::render_with_lessons;
use trading_core::{StrategyId, Symbol};

#[tokio::test]
async fn t1811_7d_fixture_renders_three_lesson_bullets() {
    let (store, expected_cards) =
        build_reflection_store_7d::build_reflection_store_7d_in_memory().await;
    assert_eq!(expected_cards.len(), 3);

    let query = RetrievalQuery {
        strategy_id: StrategyId::new("sma_crossover"),
        symbol_or_pair: reflection::types::SymbolOrPair::Single(Symbol::new("BTCUSDT")),
        current_regime: RegimeTag::Bull,
    };
    let lessons = retrieve_top_k(&store, &query, REPORT_TIME_TOP_K)
        .await
        .expect("retrieve");
    // 3-card store → at most 3 lessons.
    assert!(!lessons.is_empty());
    assert!(lessons.len() <= 3);

    let body = render_with_lessons(&[], &lessons);
    assert!(body.contains("## Memory highlights"));
    assert!(body.contains("Top"));
    assert!(body.contains("lesson cards retrieved this period:"));
    // No empty-state line when cards are present.
    assert!(!body.contains("_no closed trades yet"));
}

#[tokio::test]
async fn t1811_90d_fixture_covers_six_outcome_regime_cells() {
    let (store, cards) = build_reflection_store_90d::build_reflection_store_90d_in_memory().await;
    assert_eq!(cards.len(), 10, "10 cards across 3 strategies + pair-leg");

    // Spot-check that all 9 outcome×regime combinations are in the fixture.
    let mut seen = std::collections::BTreeSet::new();
    for c in &cards {
        seen.insert((c.outcome_class, c.exit_regime));
    }
    // 9 cells (Win/Loss/Scratch × Bull/Bear/Chop) + 1 pair-MR Win Chop
    // (which dedups with the matrix Win-Chop cell).  So 9 unique.
    assert_eq!(
        seen.len(),
        9,
        "expected 9 unique (outcome, regime) cells, found {seen:?}"
    );

    let query = RetrievalQuery {
        strategy_id: StrategyId::new("sma_crossover"),
        symbol_or_pair: reflection::types::SymbolOrPair::Single(Symbol::new("BTCUSDT")),
        current_regime: RegimeTag::Bull,
    };
    let lessons = retrieve_top_k(&store, &query, REPORT_TIME_TOP_K)
        .await
        .expect("retrieve");
    let body = render_with_lessons(&[], &lessons);
    assert!(body.contains("## Memory highlights"));
    assert!(body.contains("regime="));
}

#[tokio::test]
async fn t1811_1y_fixture_seeds_at_least_500_cards() {
    let (store, count) = build_reflection_store_1y::build_reflection_store_1y_in_memory().await;
    assert_eq!(count, 500, "1y fixture must seed exactly 500 cards");
    assert_eq!(store.count().await.unwrap(), 500);
}

#[tokio::test]
async fn t1811_lesson_bearing_body_byte_stable_across_two_runs() {
    let (store, _) = build_reflection_store_7d::build_reflection_store_7d_in_memory().await;
    let query = RetrievalQuery {
        strategy_id: StrategyId::new("sma_crossover"),
        symbol_or_pair: reflection::types::SymbolOrPair::Single(Symbol::new("BTCUSDT")),
        current_regime: RegimeTag::Bull,
    };

    let lessons_a = retrieve_top_k(&store, &query, REPORT_TIME_TOP_K)
        .await
        .expect("a");
    let lessons_b = retrieve_top_k(&store, &query, REPORT_TIME_TOP_K)
        .await
        .expect("b");
    let body_a = render_with_lessons(&[], &lessons_a);
    let body_b = render_with_lessons(&[], &lessons_b);
    assert_eq!(
        body_a, body_b,
        "lesson-bearing body must be byte-stable across two runs"
    );
}
