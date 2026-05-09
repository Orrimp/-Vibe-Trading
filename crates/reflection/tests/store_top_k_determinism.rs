//! T1809 / T1810 — top-K determinism.
//!
//! - Seed 100 cards, run `retrieve_top_k(query, 5)` twice, assert
//!   byte-identical card order across two calls.
//! - Score tie → `closed_at ASC` tie-break.
//! - Empty store → `Ok(vec![])`.

use reflection::outcome::OutcomeClass;
use reflection::regime::RegimeTag;
use reflection::store::sqlite::SqliteReflectionStore;
use reflection::store::ReflectionStore;
use reflection::types::{LessonCard, RetrievalQuery, SymbolOrPair};
use reflection::{retrieve_top_k, REPORT_TIME_TOP_K};
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{Money, StrategyId, Symbol, Timestamp, Usdt};

fn ts(unix_secs: i64) -> Timestamp {
    Timestamp::new(OffsetDateTime::from_unix_timestamp(unix_secs).expect("ts"))
}

fn mk(i: usize, strategy: &str, regime: RegimeTag, outcome: OutcomeClass) -> LessonCard {
    LessonCard {
        card_id: format!("card-{i:04}"),
        closed_at: ts(1_700_000_000 + i as i64 * 60),
        symbol_or_pair: SymbolOrPair::Single(Symbol::new("BTCUSDT")),
        strategy_id: StrategyId::new(strategy),
        signed_pnl: Money::<Usdt>::from_decimal(dec!(100)),
        opening_capital: Money::<Usdt>::from_decimal(dec!(10000)),
        holding_period_bars: u32::try_from(60 + i).unwrap_or(60),
        entry_regime: regime,
        exit_regime: regime,
        outcome_class: outcome,
        note: None,
    }
}

#[tokio::test]
async fn t1810_top_k_byte_stable_across_two_runs() {
    let store = SqliteReflectionStore::in_memory().await.expect("open");

    // Seed 100 cards across the 6 named strategies + (unattributed),
    // 3 regimes, 3 outcomes — covers every embedding cell.
    let strategies = [
        "sma_crossover",
        "macd_trend",
        "rsi_reversion",
        "bbands_mean_revert",
        "top10_momentum_h1",
        "pairs_mr_h1",
        "(unattributed)",
    ];
    let regimes = [RegimeTag::Bull, RegimeTag::Bear, RegimeTag::Chop];
    let outcomes = [OutcomeClass::Win, OutcomeClass::Loss, OutcomeClass::Scratch];

    for i in 0..100 {
        let s = strategies[i % strategies.len()];
        let r = regimes[i % regimes.len()];
        let o = outcomes[i % outcomes.len()];
        let card = mk(i, s, r, o);
        store.upsert(&card).await.expect("upsert");
    }
    assert_eq!(store.count().await.unwrap(), 100);

    let query = RetrievalQuery {
        strategy_id: StrategyId::new("sma_crossover"),
        symbol_or_pair: SymbolOrPair::Single(Symbol::new("BTCUSDT")),
        current_regime: RegimeTag::Bull,
    };
    let a = retrieve_top_k(&store, &query, REPORT_TIME_TOP_K)
        .await
        .expect("a");
    let b = retrieve_top_k(&store, &query, REPORT_TIME_TOP_K)
        .await
        .expect("b");
    assert_eq!(a.len(), REPORT_TIME_TOP_K);
    assert_eq!(b.len(), REPORT_TIME_TOP_K);
    let a_ids: Vec<_> = a.iter().map(|c| c.card_id.clone()).collect();
    let b_ids: Vec<_> = b.iter().map(|c| c.card_id.clone()).collect();
    assert_eq!(
        a_ids, b_ids,
        "top-K order must be byte-stable across two runs"
    );
}

#[tokio::test]
async fn t1810_score_tie_breaks_on_closed_at_ascending() {
    let store = SqliteReflectionStore::in_memory().await.expect("open");
    // Three cards with IDENTICAL embedding-relevant fields → identical
    // cosine scores against any query.  Must come back ordered by
    // closed_at ASC (older first).
    let mut a = mk(0, "sma_crossover", RegimeTag::Bull, OutcomeClass::Win);
    a.card_id = "first".into();
    a.closed_at = ts(1_700_000_000);
    let mut b = mk(1, "sma_crossover", RegimeTag::Bull, OutcomeClass::Win);
    b.card_id = "second".into();
    b.closed_at = ts(1_700_000_100);
    let mut c = mk(2, "sma_crossover", RegimeTag::Bull, OutcomeClass::Win);
    c.card_id = "third".into();
    c.closed_at = ts(1_700_000_200);

    // Insert in reverse order — the tie-break must still order by closed_at ASC.
    store.upsert(&c).await.unwrap();
    store.upsert(&b).await.unwrap();
    store.upsert(&a).await.unwrap();

    let query = RetrievalQuery {
        strategy_id: StrategyId::new("sma_crossover"),
        symbol_or_pair: SymbolOrPair::Single(Symbol::new("BTCUSDT")),
        current_regime: RegimeTag::Bull,
    };
    // Note: cards have non-identical scalar slots (holding_period_bars
    // differs between mk(0..2)) — the tie applies only to the one-hot
    // dimensions.  Override here to force a true tie:
    let mut tied_a = a.clone();
    tied_a.holding_period_bars = 60;
    tied_a.signed_pnl = Money::<Usdt>::from_decimal(dec!(100));
    let mut tied_b = b.clone();
    tied_b.holding_period_bars = 60;
    tied_b.signed_pnl = Money::<Usdt>::from_decimal(dec!(100));
    let mut tied_c = c.clone();
    tied_c.holding_period_bars = 60;
    tied_c.signed_pnl = Money::<Usdt>::from_decimal(dec!(100));

    // Use a fresh store with the truly-tied cards.
    let store2 = SqliteReflectionStore::in_memory().await.expect("open2");
    store2.upsert(&tied_c).await.unwrap();
    store2.upsert(&tied_b).await.unwrap();
    store2.upsert(&tied_a).await.unwrap();

    let res = retrieve_top_k(&store2, &query, 3).await.expect("retrieve");
    assert_eq!(res.len(), 3);
    let ids: Vec<_> = res.iter().map(|c| c.card_id.clone()).collect();
    assert_eq!(
        ids,
        vec![
            "first".to_string(),
            "second".to_string(),
            "third".to_string()
        ],
        "score-tie tie-break must order by closed_at ASC"
    );
}

#[tokio::test]
async fn t1809_empty_store_returns_empty_vec() {
    let store = SqliteReflectionStore::in_memory().await.expect("open");
    let query = RetrievalQuery {
        strategy_id: StrategyId::new("sma_crossover"),
        symbol_or_pair: SymbolOrPair::Single(Symbol::new("BTCUSDT")),
        current_regime: RegimeTag::Chop,
    };
    let res = retrieve_top_k(&store, &query, REPORT_TIME_TOP_K)
        .await
        .expect("retrieve");
    assert!(res.is_empty());
}
