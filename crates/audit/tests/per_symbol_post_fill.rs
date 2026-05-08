#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T1105 — V1 + V2 + V5 + V8 verification gates for the per-symbol
//! position-accounts feature
//! (`spec/features/per-symbol-position-accounts.md`).
//!
//! Per the architect's task spec
//! (`spec/tasks/per-symbol-position-accounts.md` → T1105):
//!
//! V1 — `post_fill` writes per-symbol account. Boot empty in-memory ledger
//! (chart_of_accounts + migrations 001–006 auto-applied via
//! `Ledger::open(":memory:")`). Post one ETHUSDT Buy + one BTCUSDT Buy + one
//! SOLUSDT Buy via `audit::journal::post_fill`. Assert the resulting
//! `journal_entries` rows on the position side group by exactly the three
//! per-pair account-ids (`assets:position:ETHUSDT`,
//! `assets:position:BTCUSDT`, `assets:position:SOLUSDT`) and zero rows
//! reference the legacy `assets:position:BTC` bucket.
//!
//! V2 — Pre-migration legacy rows still readable. Hand-craft a legacy-shape
//! txn directly via raw SQL: an ETHUSDT Sell whose `journal_entries` rows
//! reference the legacy `assets:position:BTC` account-id (the pre-T1102
//! hardcode) but whose `journal_transactions.description` carries the
//! symbol `ETHUSDT`. Assert: (a) the legacy row's `account_id` is unchanged
//! after migration `006`; (b) `audit::journal::verify_balance` returns
//! `Ok(())` (R6); (c) `audit::query::pnl_by_symbol` correctly buckets the
//! realized-P&L under `Symbol::new("ETHUSDT")` via the description-parse
//! fallback (R7).
//!
//! V5 — Reconciliation invariant. Re-uses the T1106/T1104 mixed fixture
//! (`build_ledger_mixed_legacy_and_per_symbol_7d`) which contains BOTH
//! legacy `assets:position:BTC` rows AND a post-006 per-pair row. For every
//! transaction id, assert `verify_balance(...)` returns `Ok(())`. Then
//! assert `Σ debit_amount == Σ credit_amount` globally on `journal_entries`
//! (R6).
//!
//! V8 — Universe coverage. Parse `config/agent.toml`'s `[funding].universe`
//! array directly (cannot import `agent::Config` here — the `agent` crate
//! depends on `audit`, cycle); for every symbol, assert
//! `SELECT 1 FROM accounts WHERE id = 'assets:position:<SYM>'` returns one
//! row (R11).
//!
//! The mixed fixture file
//! (`crates/reports/tests/fixtures/build_ledger_with_open_positions_7d.rs`)
//! is mounted via `#[path]` — the same pattern T1005 uses for V1/V4/V7
//! against the open-positions reader. Avoids fixture duplication and
//! keeps T1105 V5 and T1106 V3 over the same source of ledger truth.

use audit::{bootstrap, journal, query, Ledger};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::Deserialize;
use time::OffsetDateTime;
use trading_core::{
    FeeTier, Fill, FillId, Liquidity, Money, OrderId, Price, Quantity, Side, Symbol, Timestamp,
    Usdt, Venue,
};

// The mounted fixture exports the closed/dangling 7d-plan helpers
// (`build_ledger_with_open_positions_7d`, `build_fill_plan`, etc.) that
// T1005's `open_positions_at.rs` consumes; this test only needs the
// mixed-fixture builder, so silence the unused-symbol warnings on the
// rest of the file. Same `#[path]` re-mount pattern as T1005 — keeps the
// fixture's seed/UUID/timestamp constants in one source of truth and
// avoids a cycle through `reports`.
#[allow(dead_code)]
#[path = "../../reports/tests/fixtures/build_ledger_with_open_positions_7d.rs"]
mod fixture;

// ── helpers ───────────────────────────────────────────────────────────────────

/// Open a fresh in-memory ledger with all 6 migrations applied AND the
/// chart-of-accounts bootstrap rows seeded (so the legacy
/// `assets:position:BTC` and `assets:cash:USDT` rows exist for raw-SQL
/// inserts in V2).
async fn open_ledger() -> Ledger {
    let ledger = Ledger::in_memory().await.expect("open in-memory ledger");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap chart of accounts");
    ledger
}

fn ts_offset_secs(secs: i64) -> Timestamp {
    Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(secs))
}

fn ts_far_future() -> Timestamp {
    Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::days(36_500))
}

fn ts_unix_epoch() -> Timestamp {
    Timestamp::new(OffsetDateTime::UNIX_EPOCH)
}

fn make_fill(symbol: &str, side: Side, qty: Decimal, price: Decimal, venue_ts_secs: i64) -> Fill {
    Fill {
        id: FillId::new(),
        order_id: OrderId::new(),
        symbol: Symbol::new(symbol),
        side,
        qty: Quantity::new(qty).expect("qty ok"),
        price: Price::new(price).expect("price ok"),
        fee: Money::from_decimal(dec!(0)),
        fee_tier: FeeTier::Taker,
        venue_ts: ts_offset_secs(venue_ts_secs),
        local_ts: ts_offset_secs(venue_ts_secs),
        liquidity: Liquidity::Taker,
        transaction_id: None,
    }
}

/// Minimal typed view onto `config/agent.toml`'s `[funding]` table — the V8
/// universe-coverage gate. Only the `universe` field is consumed; everything
/// else in the file is ignored. Mirrors `agent::config::FundingConfig` shape
/// so a future schema change in agent's config flags this test at compile
/// time (deserialize-failure surface).
#[derive(Debug, Deserialize)]
struct FundingTomlSlice {
    #[serde(default)]
    universe: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct AgentTomlSlice {
    funding: FundingTomlSlice,
}

/// Repo-relative path to `config/agent.toml`. The audit crate manifest dir
/// is `<repo>/crates/audit`; the config file lives at `<repo>/config/agent.toml`.
fn agent_toml_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("config")
        .join("agent.toml")
}

// ── tests ─────────────────────────────────────────────────────────────────────

/// V1 — `post_fill` writes per-pair `assets:position:<SYMBOL>` accounts.
///
/// Empty in-memory DB; post 3 Buys (ETHUSDT, BTCUSDT, SOLUSDT). The position
/// side of every fill MUST land on its per-pair account-id; zero rows must
/// reference the legacy `assets:position:BTC` bucket. Encodes the V1
/// acceptance verbatim.
#[tokio::test]
async fn t1105_v1_post_fill_writes_per_symbol_account() {
    let ledger = open_ledger().await;

    journal::post_fill(
        &ledger,
        &make_fill("ETHUSDT", Side::Buy, dec!(0.5), dec!(2_000), 100),
        Venue::Binance,
        Some("strat_eth"),
    )
    .await
    .expect("post ETHUSDT fill");
    journal::post_fill(
        &ledger,
        &make_fill("BTCUSDT", Side::Buy, dec!(0.01), dec!(60_000), 200),
        Venue::Binance,
        Some("strat_btc"),
    )
    .await
    .expect("post BTCUSDT fill");
    journal::post_fill(
        &ledger,
        &make_fill("SOLUSDT", Side::Buy, dec!(10), dec!(100), 300),
        Venue::Binance,
        Some("strat_sol"),
    )
    .await
    .expect("post SOLUSDT fill");

    // Per-symbol distribution: exactly three per-pair account-ids on the
    // position side, sorted lexicographically. BTreeMap-equivalent
    // (SQL `ORDER BY`) so the assertion is byte-comparable.
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT account_id, COUNT(*) AS n \
         FROM journal_entries \
         WHERE account_id LIKE 'assets:position:%' \
         GROUP BY account_id \
         ORDER BY account_id",
    )
    .fetch_all(ledger.pool())
    .await
    .expect("group by per-pair account_id");
    assert_eq!(
        rows,
        vec![
            ("assets:position:BTCUSDT".to_string(), 1),
            ("assets:position:ETHUSDT".to_string(), 1),
            ("assets:position:SOLUSDT".to_string(), 1),
        ],
        "expected exactly 3 per-pair `assets:position:<SYMBOL>` account-ids \
         with one row each (BTCUSDT, ETHUSDT, SOLUSDT); got {rows:?}"
    );

    // Legacy BTC bucket — zero post-T1102 rows. The chart-of-accounts row
    // still EXISTS (bootstrap seeds it for backwards compat) but no
    // `journal_entries` row from the three `post_fill` calls above
    // references it.
    let legacy_btc_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM journal_entries WHERE account_id = 'assets:position:BTC'",
    )
    .fetch_one(ledger.pool())
    .await
    .expect("count legacy BTC bucket rows");
    assert_eq!(
        legacy_btc_count.0, 0,
        "expected zero post-T1102 journal_entries rows on the legacy \
         `assets:position:BTC` bucket; got {} (regression: the BTC \
         hardcode is back at journal.rs:82 / journal.rs:135)",
        legacy_btc_count.0
    );
}

/// V2 — Pre-migration legacy rows stay readable post-006.
///
/// Hand-craft an ETHUSDT closing Sell whose entries reference the legacy
/// `assets:position:BTC` account (the pre-T1102 hardcode) but whose
/// `journal_transactions.description` carries `ETHUSDT`. Assert:
///   1. the legacy `account_id` is preserved verbatim (R3 — additive
///      migration; no UPDATE),
///   2. `verify_balance(transaction_id) == Ok(())` (R6 — Σ Dr == Σ Cr),
///   3. `pnl_by_symbol` buckets the row under `Symbol::new("ETHUSDT")` via
///      description-parse (R7 — fallback path retained for legacy reads).
#[tokio::test]
async fn t1105_v2_legacy_row_readable_after_migration() {
    // Migration `006` is auto-applied by `Ledger::in_memory()` (sqlx::migrate!
    // runs every `.sql` file under `migrations/` at open). The migration is
    // purely additive — no UPDATE on existing rows — so seeding a legacy
    // row AFTER all migrations applied is byte-equivalent to seeding it
    // pre-006 and then running 006 on top (Q3: "no backfill").
    let ledger = open_ledger().await;

    // Synthesise a legacy ETHUSDT Sell that mirrors the pre-T1102 writer
    // shape: `description` carries `ETHUSDT` but every position-side leg
    // references `assets:position:BTC` (the literal pre-T1102 hardcode).
    let legacy_txn_id = "00000000-0000-0000-0000-00000000eth1".to_string();
    let legacy_ts = "2026-04-27T20:00:00Z";
    let legacy_desc = "sell 0.5 ETHUSDT @ 2200"; // closing fill, +100 USDT realized
    let strategy_id = "strat_eth_legacy";

    sqlx::query(
        "INSERT INTO journal_transactions (id, ts, description, strategy_id) \
         VALUES (?, ?, ?, ?)",
    )
    .bind(&legacy_txn_id)
    .bind(legacy_ts)
    .bind(legacy_desc)
    .bind(strategy_id)
    .execute(ledger.pool())
    .await
    .expect("insert legacy txn header");

    // Mimic the v0 Sell-side double-entry shape (cf. `journal.rs::post_fill`
    // Sell branch, lines 122–187). Cost basis = price (v0 simplification);
    // notional = 0.5 * 2_200 = 1_100; cost = 0.5 * 2_200 = 1_100; realized
    // = 0. To exercise pnl_by_symbol's bucketing we instead post a
    // sell-with-profit shape: notional = 1_100, cost = 1_000 (cost basis
    // 2_000 from a prior unobserved Buy), realized = +100. The test
    // asserts the symbol bucket, not the dollar amount, so the synthetic
    // cost basis is fine.
    //
    // Legs (legacy hardcode — every position-side leg on `assets:position:BTC`):
    //   Dr assets:cash:USDT          1_100
    //   Cr assets:position:BTC       1_000   (legacy hardcode, NOT ETHUSDT)
    //   Cr income:realized_pnl       100
    let legs: &[(&str, &str, &str, &str)] = &[
        // (entry_id, account_id, debit, credit)
        (
            "00000000-0000-0000-0000-00000000eth2",
            "assets:cash:USDT",
            "1100",
            "0",
        ),
        (
            "00000000-0000-0000-0000-00000000eth3",
            "assets:position:BTC",
            "0",
            "1000",
        ),
        (
            "00000000-0000-0000-0000-00000000eth4",
            "income:realized_pnl",
            "0",
            "100",
        ),
    ];
    for (entry_id, account_id, dr, cr) in legs {
        sqlx::query(
            "INSERT INTO journal_entries \
             (id, transaction_id, account_id, debit_amount, credit_amount, ts) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(entry_id)
        .bind(&legacy_txn_id)
        .bind(account_id)
        .bind(dr)
        .bind(cr)
        .bind(legacy_ts)
        .execute(ledger.pool())
        .await
        .expect("insert legacy entry leg");
    }

    // (1) Legacy `account_id` preserved verbatim — migration 006 did NOT
    // rewrite the row (R3 / Q3 — purely additive).
    let position_account_id: (String,) = sqlx::query_as(
        "SELECT account_id FROM journal_entries \
         WHERE transaction_id = ? AND account_id LIKE 'assets:position:%'",
    )
    .bind(&legacy_txn_id)
    .fetch_one(ledger.pool())
    .await
    .expect("select legacy position account_id");
    assert_eq!(
        position_account_id.0, "assets:position:BTC",
        "legacy row's position-side account_id must remain `assets:position:BTC` \
         post-006 (R3 — purely additive migration; no UPDATE on existing rows)"
    );

    // (2) `verify_balance` PASSes — the legacy txn satisfies Σ Dr == Σ Cr
    // (R6 — reconciliation invariant holds across the migration boundary).
    journal::verify_balance(&ledger, &legacy_txn_id)
        .await
        .expect("verify_balance(legacy_txn) returns Ok(())");

    // (3) `pnl_by_symbol` buckets the row under `Symbol::new("ETHUSDT")` via
    // the description-parse fallback (R7). The position-side leg references
    // the legacy `assets:position:BTC` account (NOT ETHUSDT), so the only
    // way the reader picks `ETHUSDT` is by parsing the description token.
    let pnl = query::pnl_by_symbol(&ledger, ts_unix_epoch(), ts_far_future())
        .await
        .expect("pnl_by_symbol on legacy fixture");
    assert_eq!(
        pnl,
        vec![(
            Symbol::new("ETHUSDT"),
            Money::<Usdt>::from_decimal(dec!(100)),
        )],
        "expected one bucket — `(ETHUSDT, +100 USDT)` — derived from the legacy \
         txn's description-parse path (R7); got {pnl:?}"
    );
}

/// V5 — Reconciliation invariant holds across the migration boundary on
/// the mixed fixture.
///
/// The mixed fixture (`build_ledger_mixed_legacy_and_per_symbol_7d`)
/// contains: 2 raw-SQL legacy rows on `assets:position:BTC` (BTCUSDT +
/// ETHUSDT Buys) plus 1 post-006 row on `assets:position:SOLUSDT` (via
/// `post_fill`). For every transaction id, `verify_balance(...)` MUST
/// return `Ok(())`. Then assert `Σ debit_amount == Σ credit_amount`
/// globally — the migration adds account rows only; no money moves (R6).
#[tokio::test]
async fn t1105_v5_balance_invariant_pre_and_post_migration() {
    let ledger = open_ledger().await;

    // Build the mixed fixture (pre-006 BTC + ETH legacy rows + post-006
    // SOL row). The fixture itself is non-anchored (T1004) and shared
    // with T1106 V3 / V7.
    fixture::build_ledger_mixed_legacy_and_per_symbol_7d(&ledger)
        .await
        .expect("build mixed legacy/per-symbol fixture");

    // Per-transaction reconciliation — every txn in the mixed fixture
    // satisfies Σ Dr == Σ Cr. Sorted by ts ASC, id ASC for byte-deterministic
    // iteration order across runs.
    let txn_ids: Vec<(String,)> =
        sqlx::query_as("SELECT id FROM journal_transactions ORDER BY ts ASC, id ASC")
            .fetch_all(ledger.pool())
            .await
            .expect("list transaction ids");
    assert!(
        !txn_ids.is_empty(),
        "fixture must produce at least one transaction"
    );
    for (txn_id,) in &txn_ids {
        journal::verify_balance(&ledger, txn_id)
            .await
            .unwrap_or_else(|e| {
                panic!("verify_balance failed on txn `{txn_id}` in mixed fixture: {e}")
            });
    }

    // Global reconciliation — Σ debit_amount == Σ credit_amount across the
    // entire journal. Sums are pulled as `Decimal`-parseable strings so
    // integer/decimal precision is preserved (no f64 in money math).
    let totals: Vec<(String, String)> =
        sqlx::query_as("SELECT debit_amount, credit_amount FROM journal_entries")
            .fetch_all(ledger.pool())
            .await
            .expect("read all entries for global Σ");
    let mut total_debit = Decimal::ZERO;
    let mut total_credit = Decimal::ZERO;
    for (dr_str, cr_str) in &totals {
        let dr: Decimal = dr_str.parse().expect("parse debit decimal");
        let cr: Decimal = cr_str.parse().expect("parse credit decimal");
        total_debit += dr;
        total_credit += cr;
    }
    assert_eq!(
        total_debit, total_credit,
        "global Σ debit_amount ({total_debit}) must equal Σ credit_amount \
         ({total_credit}) across the migration boundary (R6)"
    );
}

/// V8 — Universe coverage smoke.
///
/// Parse `config/agent.toml`'s `[funding].universe` array directly (cannot
/// import `agent::Config` here — the `agent` crate depends on `audit`,
/// cycle); for every symbol, assert the post-migration `accounts` table
/// contains a row with `id = 'assets:position:<SYMBOL>'` (R11). The
/// `accounts` row is the FK target for every post-T1102 `post_fill`
/// write — missing one means the first fill for that symbol fails the
/// FK reference at runtime.
#[tokio::test]
async fn t1105_v8_universe_coverage() {
    let ledger = open_ledger().await;

    let path = agent_toml_path();
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read config/agent.toml at {}: {e}", path.display()));
    let cfg: AgentTomlSlice =
        toml::from_str(&raw).expect("parse config/agent.toml [funding].universe");

    assert!(
        !cfg.funding.universe.is_empty(),
        "config/agent.toml [funding].universe is empty — V8 cannot assert \
         coverage of a zero-length set"
    );

    // BTreeMap-style sorted iteration so the failure message is
    // deterministic across runs (no HashMap iteration order leak).
    let mut symbols: Vec<String> = cfg.funding.universe.clone();
    symbols.sort();

    for symbol in &symbols {
        let id = format!("assets:position:{symbol}");
        let row: Option<(String,)> = sqlx::query_as("SELECT id FROM accounts WHERE id = ?")
            .bind(&id)
            .fetch_optional(ledger.pool())
            .await
            .expect("query account by id");
        assert!(
            row.is_some(),
            "config/agent.toml [funding].universe declares `{symbol}` but the \
             post-006 `accounts` table has no row with id = `{id}` (R11 — \
             FK reference would fail at first fill for this symbol)"
        );
    }
}
