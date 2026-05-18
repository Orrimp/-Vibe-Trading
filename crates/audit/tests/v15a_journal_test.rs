//! T707 + T708 — v1.5a audit integration tests.
//!
//! T707: `mean_reversion_stop` + `pair_short_observation` writers.
//! T708: `pnl_by_pair` reader — single-pair and overlapping-`a` cases.

use audit::{Ledger, bootstrap, journal, query};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{Pair, PairMembership, StrategyEventKind, StrategyId, Symbol, Timestamp};

async fn open_ledger() -> Ledger {
    let ledger = Ledger::in_memory().await.expect("open in-memory ledger");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap chart of accounts");
    ledger
}

fn ts_epoch() -> Timestamp {
    Timestamp::new(OffsetDateTime::UNIX_EPOCH)
}

fn ts_far_future() -> Timestamp {
    Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::days(36500))
}

/// Inject a raw `income:realized_pnl` row via a journal transaction.
///
/// This is the equivalent of a closed-trade P&L write.  We bypass the
/// simplified v0 `post_fill` cost-basis (which always produces zero realized
/// P&L) so we can test the `pnl_by_symbol` / `pnl_by_pair` aggregation logic.
async fn inject_realized_pnl(ledger: &Ledger, symbol: &str, amount: Decimal) {
    use uuid::Uuid;

    let txn_id = Uuid::new_v4().to_string();
    // Use a timestamp in the far-future so `[ts_epoch, ts_far_future]` captures it.
    let ts_str = "2030-01-01T00:00:00Z";

    // The description pattern must match `"<side> <qty> <symbol> @ <price>"` for
    // `extract_symbol_from_description` to work.
    let side = if amount >= Decimal::ZERO {
        "sell"
    } else {
        "buy"
    };
    let description = format!("{side} 1 {symbol} @ 1000");

    sqlx::query("INSERT INTO journal_transactions (id, ts, description) VALUES (?, ?, ?)")
        .bind(&txn_id)
        .bind(ts_str)
        .bind(&description)
        .execute(ledger.pool())
        .await
        .expect("insert transaction");

    // Credit income:realized_pnl for a profit (or debit for a loss).
    let (debit, credit) = if amount >= Decimal::ZERO {
        (Decimal::ZERO, amount)
    } else {
        (amount.abs(), Decimal::ZERO)
    };

    let entry_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO journal_entries \
         (id, transaction_id, account_id, debit_amount, credit_amount, ts) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&entry_id)
    .bind(&txn_id)
    .bind("income:realized_pnl")
    .bind(debit.to_string())
    .bind(credit.to_string())
    .bind(ts_str)
    .execute(ledger.pool())
    .await
    .expect("insert journal entry");

    // Balancing entry to preserve debit == credit in the transaction.
    let bal_id = Uuid::new_v4().to_string();
    let (bal_debit, bal_credit) = if amount >= Decimal::ZERO {
        (amount, Decimal::ZERO)
    } else {
        (Decimal::ZERO, amount.abs())
    };
    sqlx::query(
        "INSERT INTO journal_entries \
         (id, transaction_id, account_id, debit_amount, credit_amount, ts) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&bal_id)
    .bind(&txn_id)
    .bind("assets:cash:USDT")
    .bind(bal_debit.to_string())
    .bind(bal_credit.to_string())
    .bind(ts_str)
    .execute(ledger.pool())
    .await
    .expect("insert balancing entry");
}

fn make_pair_membership(a: &str, b: &str) -> PairMembership {
    let pair = Pair::new(Symbol::new(a), Symbol::new(b), dec!(1.0)).expect("valid pair");
    PairMembership::from_pair(&pair)
}

// ── T707: mean_reversion_stop writer ─────────────────────────────────────────

#[tokio::test]
async fn t707_mean_reversion_stop_writes_and_reads() {
    let ledger = open_ledger().await;

    journal::mean_reversion_stop(&ledger, "pairs_mr_h1", "(BTCUSDT, ETHUSDT)", "4.23", None)
        .await
        .expect("write mean_reversion_stop");

    let history = query::strategy_history(&ledger, StrategyId::new("pairs_mr_h1"))
        .await
        .expect("strategy_history");

    assert_eq!(history.len(), 1, "expected exactly 1 event");
    let ev = &history[0];
    assert_eq!(
        ev.kind,
        StrategyEventKind::MeanReversionStop,
        "kind must be MeanReversionStop"
    );
    assert_eq!(
        ev.error_code.as_deref(),
        Some("mean_reversion_stop"),
        "error_code must be 'mean_reversion_stop'"
    );

    // error_summary must be JSON containing pair_key and z_at_stop.
    let summary = ev.error_summary.as_deref().expect("error_summary present");
    let json: serde_json::Value =
        serde_json::from_str(summary).expect("error_summary is valid JSON");
    assert_eq!(
        json["pair_key"].as_str(),
        Some("(BTCUSDT, ETHUSDT)"),
        "pair_key in summary"
    );
    assert_eq!(
        json["z_at_stop"].as_str(),
        Some("4.23"),
        "z_at_stop in summary"
    );
}

// ── T707: pair_short_observation writer ──────────────────────────────────────

#[tokio::test]
async fn t707_pair_short_observation_writes_and_reads() {
    let ledger = open_ledger().await;

    journal::pair_short_observation(&ledger, "pairs_mr_h1", "(BTCUSDT, ETHUSDT)", "-2.34", None)
        .await
        .expect("write pair_short_observation");

    let history = query::strategy_history(&ledger, StrategyId::new("pairs_mr_h1"))
        .await
        .expect("strategy_history");

    assert_eq!(history.len(), 1, "expected exactly 1 event");
    let ev = &history[0];
    assert_eq!(
        ev.kind,
        StrategyEventKind::PairShortObservation,
        "kind must be PairShortObservation"
    );
    assert_eq!(
        ev.error_code.as_deref(),
        Some("pair_short_observation"),
        "error_code must be 'pair_short_observation'"
    );

    let summary = ev.error_summary.as_deref().expect("error_summary present");
    let json: serde_json::Value =
        serde_json::from_str(summary).expect("error_summary is valid JSON");
    assert_eq!(
        json["pair_key"].as_str(),
        Some("(BTCUSDT, ETHUSDT)"),
        "pair_key in summary"
    );
    assert_eq!(
        json["z_at_entry"].as_str(),
        Some("-2.34"),
        "z_at_entry in summary"
    );
}

// ── T707: both events in strategy_history + ledger imbalance == 0 ────────────

#[tokio::test]
async fn t707_both_events_no_ledger_imbalance() {
    let ledger = open_ledger().await;

    // Capture global debit/credit before.
    let (dr_before, cr_before) = query::global_debit_credit_sum(&ledger)
        .await
        .expect("global sum before");

    journal::mean_reversion_stop(&ledger, "pairs_mr_h1", "(BTCUSDT, ETHUSDT)", "4.50", None)
        .await
        .expect("write mean_reversion_stop");

    journal::pair_short_observation(&ledger, "pairs_mr_h1", "(BTCUSDT, ETHUSDT)", "-2.10", None)
        .await
        .expect("write pair_short_observation");

    // The global debit/credit sums must be UNCHANGED — strategy_events carry
    // no money (reconciler invariant preserved per T707 acceptance criteria).
    let (dr_after, cr_after) = query::global_debit_credit_sum(&ledger)
        .await
        .expect("global sum after");

    assert_eq!(
        dr_before, dr_after,
        "mean_reversion_stop must not affect debits"
    );
    assert_eq!(
        cr_before, cr_after,
        "pair_short_observation must not affect credits"
    );

    // Both events appear in strategy_history in write order.
    let history = query::strategy_history(&ledger, StrategyId::new("pairs_mr_h1"))
        .await
        .expect("strategy_history");
    assert_eq!(history.len(), 2, "expected 2 events");
    assert_eq!(history[0].kind, StrategyEventKind::MeanReversionStop);
    assert_eq!(history[1].kind, StrategyEventKind::PairShortObservation);
}

// ── T708: pnl_by_pair — single pair ──────────────────────────────────────────

#[tokio::test]
async fn t708_pnl_by_pair_single_pair() {
    let ledger = open_ledger().await;

    // Inject known P&L for BTCUSDT: 3 trades × $100 profit each = $300 total.
    for _ in 0..3 {
        inject_realized_pnl(&ledger, "BTCUSDT", dec!(100)).await;
    }

    let since = ts_epoch();
    let until = ts_far_future();
    let memberships = vec![make_pair_membership("BTCUSDT", "ETHUSDT")];

    let pairs_pnl = query::pnl_by_pair(&ledger, &memberships, since, until)
        .await
        .expect("pnl_by_pair");

    // Should have exactly 1 row for the single pair.
    assert_eq!(pairs_pnl.len(), 1, "expected 1 pair P&L row");
    let (key, pnl) = &pairs_pnl[0];
    assert_eq!(key.a, Symbol::new("BTCUSDT"), "a-leg symbol");
    assert_eq!(key.b, Symbol::new("ETHUSDT"), "b-leg symbol");
    assert_eq!(pnl.amount(), dec!(300), "P&L should be 3 × 100 = 300");

    // Sum invariant (k == 1): pnl_by_pair sum must equal pnl_by_symbol sum.
    let by_symbol = query::pnl_by_symbol(&ledger, since, until)
        .await
        .expect("pnl_by_symbol");
    let symbol_btc_pnl = by_symbol
        .iter()
        .find(|(sym, _)| *sym == Symbol::new("BTCUSDT"))
        .map(|(_, m)| m.amount())
        .unwrap_or(Decimal::ZERO);

    assert_eq!(
        pnl.amount(),
        symbol_btc_pnl,
        "pnl_by_pair[(BTCUSDT,ETHUSDT)] must equal pnl_by_symbol[BTCUSDT] when k==1"
    );
}

// ── T708: pnl_by_pair — overlapping a-leg (k=2 multiplicity) ─────────────────

#[tokio::test]
async fn t708_pnl_by_pair_overlapping_a_leg() {
    let ledger = open_ledger().await;

    // Inject P&L for BTCUSDT: $500.
    inject_realized_pnl(&ledger, "BTCUSDT", dec!(500)).await;

    let since = ts_epoch();
    let until = ts_far_future();

    // k=2: BTCUSDT appears in both (BTCUSDT, ETHUSDT) AND (BTCUSDT, SOLUSDT).
    let memberships = vec![
        make_pair_membership("BTCUSDT", "ETHUSDT"),
        make_pair_membership("BTCUSDT", "SOLUSDT"),
    ];

    let pairs_pnl = query::pnl_by_pair(&ledger, &memberships, since, until)
        .await
        .expect("pnl_by_pair overlapping");

    // Both pairs should appear (same BTCUSDT P&L assigned to each).
    assert_eq!(
        pairs_pnl.len(),
        2,
        "expected 2 pair rows (k=2 multiplicity)"
    );

    // Verify aggregate BTCUSDT P&L from pnl_by_symbol.
    let by_symbol = query::pnl_by_symbol(&ledger, since, until)
        .await
        .expect("pnl_by_symbol");
    let btc_pnl = by_symbol
        .iter()
        .find(|(sym, _)| *sym == Symbol::new("BTCUSDT"))
        .map(|(_, m)| m.amount())
        .unwrap_or(Decimal::ZERO);

    // Each pair row reports the same aggregate BTCUSDT P&L (architect risk #3).
    for (key, pair_pnl) in &pairs_pnl {
        assert_eq!(
            pair_pnl.amount(),
            btc_pnl,
            "overlapping-a: pnl_by_pair[{key}] should equal aggregate pnl_by_symbol[BTCUSDT]"
        );
    }

    // Document: Σ pnl_by_pair (k=2) == 2 * pnl_by_symbol[BTCUSDT] — this is
    // the expected multiplicity behavior (architect risk #3).
    let sum_pair_pnl: Decimal = pairs_pnl.iter().map(|(_, m)| m.amount()).sum();
    assert_eq!(
        sum_pair_pnl,
        btc_pnl * Decimal::from(2u32),
        "k=2 multiplicity: Σ pnl_by_pair == 2 * pnl_by_symbol (documented behavior)"
    );
}

// ── T708: pnl_by_pair returns empty for zero-P&L pairs ───────────────────────

#[tokio::test]
async fn t708_pnl_by_pair_empty_when_no_fills() {
    let ledger = open_ledger().await;

    let memberships = vec![make_pair_membership("BTCUSDT", "ETHUSDT")];
    let result = query::pnl_by_pair(&ledger, &memberships, ts_epoch(), ts_far_future())
        .await
        .expect("pnl_by_pair empty");

    assert!(result.is_empty(), "zero-P&L pairs should be omitted");
}

// ── T708: pnl_by_pair is lex-sorted ──────────────────────────────────────────

#[tokio::test]
async fn t708_pnl_by_pair_lex_sorted() {
    let ledger = open_ledger().await;

    // Inject P&L for two symbols.
    inject_realized_pnl(&ledger, "BTCUSDT", dec!(200)).await;
    inject_realized_pnl(&ledger, "ETHUSDT", dec!(50)).await;

    // Memberships in non-lex order: ETHUSDT pair first, then BTCUSDT pair.
    let memberships = vec![
        make_pair_membership("ETHUSDT", "SOLUSDT"),
        make_pair_membership("BTCUSDT", "ETHUSDT"),
    ];

    let result = query::pnl_by_pair(&ledger, &memberships, ts_epoch(), ts_far_future())
        .await
        .expect("pnl_by_pair sorted");

    // Result should be lex-sorted: BTCUSDT < ETHUSDT.
    assert_eq!(result.len(), 2, "expected 2 pair rows");
    assert!(
        result[0].0 < result[1].0,
        "pnl_by_pair result must be lex-sorted by PairKey"
    );
    assert_eq!(
        result[0].0.a,
        Symbol::new("BTCUSDT"),
        "first key should be BTCUSDT"
    );
    assert_eq!(
        result[1].0.a,
        Symbol::new("ETHUSDT"),
        "second key should be ETHUSDT"
    );
}

// ── T708: pnl_by_pair sum invariant — 30-fill stress test ────────────────────

#[tokio::test]
async fn t708_pnl_by_pair_30_fill_sum_invariant() {
    let ledger = open_ledger().await;

    // Inject 10 fills for each of 3 symbols (30 total).
    let symbols = ["BTCUSDT", "ETHUSDT", "SOLUSDT"];
    let amounts: [Decimal; 3] = [dec!(100), dec!(50), dec!(75)];

    for (sym, amt) in symbols.iter().zip(amounts.iter()) {
        for _ in 0..10 {
            inject_realized_pnl(&ledger, sym, *amt).await;
        }
    }

    let since = ts_epoch();
    let until = ts_far_future();

    // One pair per symbol (unique a-legs → k==1 for each).
    let memberships = vec![
        make_pair_membership("BTCUSDT", "ETHUSDT"),
        make_pair_membership("ETHUSDT", "SOLUSDT"),
        make_pair_membership("SOLUSDT", "BNBUSDT"),
    ];

    let pairs_pnl = query::pnl_by_pair(&ledger, &memberships, since, until)
        .await
        .expect("pnl_by_pair 30-fill");

    let by_symbol = query::pnl_by_symbol(&ledger, since, until)
        .await
        .expect("pnl_by_symbol 30-fill");

    // Sum invariant: each pair's P&L must match its a-symbol's P&L.
    for (key, pair_pnl) in &pairs_pnl {
        let sym_pnl = by_symbol
            .iter()
            .find(|(sym, _)| *sym == key.a)
            .map(|(_, m)| m.amount())
            .unwrap_or(Decimal::ZERO);

        assert_eq!(
            pair_pnl.amount(),
            sym_pnl,
            "pnl_by_pair[{key}] must equal pnl_by_symbol[{}]",
            key.a
        );
    }
}

// ── T708: PairKey ordering in result ─────────────────────────────────────────

#[tokio::test]
async fn t708_pnl_by_pair_key_ordering_matches_btreemap() {
    let ledger = open_ledger().await;

    inject_realized_pnl(&ledger, "BNBUSDT", dec!(10)).await;
    inject_realized_pnl(&ledger, "BTCUSDT", dec!(20)).await;
    inject_realized_pnl(&ledger, "ETHUSDT", dec!(30)).await;

    // Input memberships in reverse order.
    let memberships = vec![
        make_pair_membership("ETHUSDT", "SOLUSDT"),
        make_pair_membership("BTCUSDT", "BNBUSDT"),
        make_pair_membership("BNBUSDT", "BTCUSDT"),
    ];

    let result = query::pnl_by_pair(&ledger, &memberships, ts_epoch(), ts_far_future())
        .await
        .expect("pnl_by_pair ordering");

    // Verify the output is in strictly ascending PairKey order.
    for w in result.windows(2) {
        assert!(
            w[0].0 < w[1].0,
            "pnl_by_pair output must be strictly ascending: {:?} < {:?}",
            w[0].0,
            w[1].0
        );
    }
}
