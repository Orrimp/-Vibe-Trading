#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Story 3.21 AC2 — **crash-consistency proof** for the paper portfolio's
//! money side.
//!
//! The story's task-1 inventory (2026-08-04) found that crash atomicity is
//! *already* provided by the storage layer: every fill is one balanced
//! double-entry SQLite transaction (`audit::journal::post_fill_with_signal`
//! → `pool.begin()` … `commit()`), money is `Decimal`-as-TEXT throughout
//! (AD-9), and the live equity series rides the same ledger (ADR-0052).
//! So AC2 is **not** "build crash-consistency" — it is "prove the existing
//! claim with a test that could fail if the claim were false". This file is
//! that proof. It adds **no** production code and changes no writer.
//!
//! ## What this file DOES simulate
//!
//! **A hard process kill (SIGKILL / `TerminateProcess`) while a fill's write
//! transaction is open, with the partial rows already physically on disk.**
//!
//! [`ac2_killed_process_reloads_to_whole_fills`] re-execs *this very test
//! binary* as a child (`std::process::Command` on `current_exe()`, driving
//! the `#[ignore]`d helper [`crash_writer_child_helper_do_not_run_directly`]).
//! The child opens a **file-backed** ledger, commits a realistic paper
//! session of [`COMMITTED_FILLS`] fills through the real production writer,
//! then opens one more transaction shaped like a **half-applied Buy** — the
//! header plus the `Dr assets:position:…` leg and **no `Cr assets:cash:USDT`
//! leg** — and never commits it. Before parking it shrinks the page cache and
//! inserts [`SPILL_ROWS`] further rows *inside that same uncommitted
//! transaction*, specifically to force SQLite to spill dirty, uncommitted
//! pages out of the page cache onto the disk. The parent then kills it,
//! reopens the same file, and asserts the reload is coherent.
//!
//! Where those spilled bytes land depends on the journal mode, and the test
//! does not assume one: `Ledger::open` currently yields SQLite's `delete`
//! (rollback-journal) mode, in which the *modified* pages go into the **main
//! database file** and their pre-images into `<db>-journal`, so recovery is an
//! undo; under WAL the modified pages go into `<db>-wal` with no commit frame
//! and recovery simply ignores them. [`on_disk_bytes`] therefore measures the
//! main file *and* both sidecars, which makes the guard below correct in
//! either mode, and the child reports the mode it actually ran under.
//!
//! **The forced spill is the anti-vacuity guard on the crash itself.** Without
//! it, a kill during a small transaction may leave *nothing* on disk to roll
//! back, and "no half-fill survived" would be true for the boring reason that
//! no half-fill was ever written — a passing test that proves nothing (the
//! failure mode recorded as bug-log #66). The child therefore *measures* how
//! much uncommitted data reached the disk and refuses to signal ready below
//! [`MIN_SPILL_BYTES`]; the parent re-asserts that number and re-measures at
//! the moment of the kill. Observed on macOS / SQLite in `delete` mode:
//! ~1.4 MB of uncommitted pages in the main file plus ~55 KiB of rollback
//! journal, all of it gone after recovery.
//!
//! [`ac2_aborted_transaction_leaves_no_trace`] is the cheap in-process sibling:
//! the same partial write, abandoned by dropping the `sqlx::Transaction`
//! (rollback), then re-opened through a fresh pool. It is deliberately the
//! *weaker* of the two — it exercises sqlx's `Drop` and SQLite's
//! `ROLLBACK`, not the OS-level crash path — and it is here only because it
//! localises a regression to the writer shape in one second instead of ten.
//!
//! ## What this file does NOT simulate
//!
//! - **Media-level torn writes / lying `fsync`.** A kill leaves the sidecar
//!   in a state SQLite itself wrote; it does not tear a 4 KiB sector in half,
//!   flip bits, or model a drive that acknowledges a barrier it did not honour.
//!   Recovery from a physically corrupt page is SQLite's own tested domain,
//!   not this repository's, and faking it by truncating the file would prove a
//!   property of `std::fs`, not of our writer.
//! - **A kill inside SQLite's own commit-marker `fsync`.** Landing the kill in
//!   that microsecond window is not reproducible from a parent process, so the
//!   test does not pretend to. The covered case — killed with the transaction
//!   open — is the one the paper session actually faces.
//! - **Filesystem or whole-machine loss**, and **concurrent multi-process
//!   writers** (the paper session has exactly one writer).
//! - **The non-money half of the paper state.** `lab::persistence` (strategy,
//!   pair, range, compare set) is a serde blob outside `crates/audit` and is
//!   AC3's subject, not AC2's.
//!
//! ## The reload contract, and the proof that it can fail
//!
//! [`reload_violations`] is the single assertion surface: it reloads the
//! ledger and returns every breach of the coherence contract —
//! `Σ debits == Σ credits` at exact `Decimal` cents (never `f64`, AD-9), the
//! same per transaction, the cash/position/fee/income family reconciliation,
//! no orphan header or orphan entry, and — for every transaction the writer
//! marked as a fill (`fill_id IS NOT NULL`) — both a cash leg and a position
//! leg and never fewer than the 4 entries a whole fill writes.
//!
//! An assertion that cannot fail proves nothing, so three **negative
//! controls** commit the exact shapes the crash must not leave behind and
//! prove [`reload_violations`] reports them:
//! [`negative_control_committed_half_fill_is_caught`] (position leg with no
//! cash leg), [`negative_control_orphan_header_is_caught`] (header with no
//! entries — the literal torn-write shape, since the writer INSERTs the
//! header first), and
//! [`negative_control_one_cent_imbalance_is_caught`] (a whole-shaped,
//! four-entry fill that is off by exactly one cent — the AD-9 exact-cent
//! claim, which a loose tolerance would wave through).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use audit::{Ledger, bootstrap, journal, query};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tempfile::tempdir;
use time::OffsetDateTime;
use trading_core::{
    FeeTier, Fill, FillId, Liquidity, Money, OrderId, Price, Quantity, Side, Symbol, Timestamp,
    Venue,
};

// ── the paper session fixture ─────────────────────────────────────────────────

/// Fills the child commits before it is killed. Shaped like a real forward
/// paper session — alternating symbols, one Sell closing every third leg —
/// rather than a single synthetic Buy, so the reload has both open positions
/// and realised legs to reconstruct.
const COMMITTED_FILLS: usize = 12;

/// Strategy id stamped on every committed paper fill.
const PAPER_STRATEGY: &str = "ac2-paper-session";

/// Entries the writer emits for one Buy: Dr position, Cr cash, Dr fee, Cr cash.
const BUY_ENTRIES: usize = 4;
/// Entries the writer emits for one Sell: Dr cash, Cr position, realised P&L,
/// Dr fee, Cr cash.
const SELL_ENTRIES: usize = 5;

fn ts_offset_secs(secs: i64) -> Timestamp {
    Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(secs))
}

/// The deterministic paper session both the parent and the child agree on.
///
/// Money is fixed (`Decimal` literals) so the parent can hand-compute the
/// post-crash cash balance to the cent; only the UUID ids differ per call,
/// and nothing asserts on those.
fn paper_session_fills() -> Vec<Fill> {
    (0..COMMITTED_FILLS)
        .map(|i| {
            let sell = i % 3 == 2;
            let symbol = if i % 2 == 0 { "BTCUSDT" } else { "ETHUSDT" };
            let price = if i % 2 == 0 {
                dec!(50000) + Decimal::from(i) * dec!(125)
            } else {
                dec!(3000) + Decimal::from(i) * dec!(11)
            };
            let qty = if i % 2 == 0 { dec!(0.004) } else { dec!(0.07) };
            // Fees are fixed cent-scale literals, not a percentage of the
            // notional — a derived fee would drag sub-cent scale into the
            // hand-computed expectation for no test value.
            let fee = dec!(0.11) + Decimal::from(i) * dec!(0.01);
            Fill {
                id: FillId::new(),
                order_id: OrderId::new(),
                symbol: Symbol::new(symbol),
                side: if sell { Side::Sell } else { Side::Buy },
                qty: Quantity::new(qty).expect("qty ok"),
                price: Price::new(price).expect("price ok"),
                fee: Money::from_decimal(fee),
                fee_tier: FeeTier::Taker,
                #[allow(clippy::cast_possible_wrap)]
                venue_ts: ts_offset_secs(1_000 + i as i64 * 60),
                #[allow(clippy::cast_possible_wrap)]
                local_ts: ts_offset_secs(1_000 + i as i64 * 60),
                liquidity: Liquidity::Taker,
                transaction_id: None,
            }
        })
        .collect()
}

/// Journal entry rows a fully applied paper session writes.
fn expected_entry_rows() -> i64 {
    paper_session_fills()
        .iter()
        .map(|f| match f.side {
            Side::Buy => BUY_ENTRIES as i64,
            Side::Sell => SELL_ENTRIES as i64,
        })
        .sum()
}

/// Hand-computed `assets:cash:USDT` balance after a fully applied session.
///
/// `query::cash_balance` reports credits − debits, so a Buy (cash paid out)
/// adds `notional + fee` and a Sell (cash received) adds `fee − notional`.
fn expected_cash_balance() -> Decimal {
    paper_session_fills()
        .iter()
        .map(|f| {
            let notional = f.qty.get() * f.price.get();
            let fee = f.fee.amount();
            match f.side {
                Side::Buy => notional + fee,
                Side::Sell => fee - notional,
            }
        })
        .sum()
}

async fn open_file_ledger(db_path: &Path) -> Ledger {
    let url = db_path.to_str().expect("utf-8 db path");
    let ledger = Ledger::open(url).await.expect("open file-backed ledger");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap chart of accounts");
    ledger
}

async fn write_paper_session(ledger: &Ledger) {
    for fill in paper_session_fills() {
        journal::post_fill(ledger, &fill, Venue::Binance, Some(PAPER_STRATEGY))
            .await
            .expect("post_fill");
    }
}

// ── the reload-coherence contract ─────────────────────────────────────────────

/// One breach of the contract the paper portfolio must satisfy on reload.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Violation {
    /// `Σ debits != Σ credits` across the whole ledger, at exact cents.
    GlobalImbalance { debits: Decimal, credits: Decimal },
    /// One transaction does not balance, at exact cents.
    TransactionImbalance {
        txn_id: String,
        debits: Decimal,
        credits: Decimal,
    },
    /// The position / fee / income side is not exactly funded by the cash side.
    CashLegReconciliation {
        position_net_dr: Decimal,
        fees_net_dr: Decimal,
        income_net_dr: Decimal,
        cash_net_cr: Decimal,
    },
    /// A fill moved a position account but never touched cash.
    FillWithoutCashLeg { txn_id: String },
    /// A fill touched cash but never moved a position account.
    FillWithoutPositionLeg { txn_id: String },
    /// A transaction header survived with no entries at all — the literal
    /// torn-write shape, since the writer INSERTs the header first.
    OrphanHeader { txn_id: String },
    /// An entry survived whose transaction header did not.
    OrphanEntry { txn_id: String },
    /// A fill landed with fewer entries than any whole fill writes.
    PartialFill { txn_id: String, entries: usize },
}

impl Violation {
    /// Stable discriminant, so a negative control can assert *which* check
    /// fired without pinning the amounts in the message.
    fn kind(&self) -> &'static str {
        match self {
            Self::GlobalImbalance { .. } => "GlobalImbalance",
            Self::TransactionImbalance { .. } => "TransactionImbalance",
            Self::CashLegReconciliation { .. } => "CashLegReconciliation",
            Self::FillWithoutCashLeg { .. } => "FillWithoutCashLeg",
            Self::FillWithoutPositionLeg { .. } => "FillWithoutPositionLeg",
            Self::OrphanHeader { .. } => "OrphanHeader",
            Self::OrphanEntry { .. } => "OrphanEntry",
            Self::PartialFill { .. } => "PartialFill",
        }
    }
}

fn kinds(violations: &[Violation]) -> Vec<&'static str> {
    violations.iter().map(Violation::kind).collect()
}

const CASH_ACCOUNT: &str = "assets:cash:USDT";
const POSITION_PREFIX: &str = "assets:position:";
const FEE_PREFIX: &str = "expense:fees:";
const INCOME_PREFIX: &str = "income:";

fn parse_amount(raw: &str) -> Decimal {
    raw.parse::<Decimal>()
        .unwrap_or_else(|_| panic!("journal amount `{raw}` must parse as Decimal (AD-9)"))
}

/// Reload the ledger and return every breach of the coherence contract.
///
/// All money comparisons are exact `Decimal` equality — no epsilon, no
/// `f64`. `rust_decimal`'s `PartialEq` compares by value across scales, so
/// `1.10` and `1.1000` reconcile while a one-cent drift does not.
async fn reload_violations(ledger: &Ledger) -> Vec<Violation> {
    let headers: Vec<(String, Option<String>)> =
        sqlx::query_as("SELECT id, fill_id FROM journal_transactions ORDER BY id")
            .fetch_all(ledger.pool())
            .await
            .expect("read journal_transactions");

    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT transaction_id, account_id, debit_amount, credit_amount \
         FROM journal_entries ORDER BY id",
    )
    .fetch_all(ledger.pool())
    .await
    .expect("read journal_entries");

    let mut by_txn: BTreeMap<String, Vec<(String, Decimal, Decimal)>> = BTreeMap::new();
    for (txn_id, account_id, dr, cr) in &rows {
        by_txn.entry(txn_id.clone()).or_default().push((
            account_id.clone(),
            parse_amount(dr),
            parse_amount(cr),
        ));
    }

    let mut out = Vec::new();

    // (1) Global identity: Σ debits == Σ credits, exact cents.
    let total_dr: Decimal = rows.iter().map(|(_, _, dr, _)| parse_amount(dr)).sum();
    let total_cr: Decimal = rows.iter().map(|(_, _, _, cr)| parse_amount(cr)).sum();
    if total_dr != total_cr {
        out.push(Violation::GlobalImbalance {
            debits: total_dr,
            credits: total_cr,
        });
    }

    // (2) The same identity projected onto the account families, so a failure
    //     message names the money rather than two opaque totals: the position,
    //     fee and income legs together must be exactly funded by the cash leg.
    //     This is a *projection* of check (1), not an independent axiom — it
    //     earns its place because it is the form the AC states ("no position
    //     without its cash leg") and the form a human can read.
    let mut position_net_dr = dec!(0);
    let mut fees_net_dr = dec!(0);
    let mut income_net_dr = dec!(0);
    let mut cash_net_cr = dec!(0);
    for (_, account_id, dr, cr) in &rows {
        let (dr, cr) = (parse_amount(dr), parse_amount(cr));
        if account_id == CASH_ACCOUNT {
            cash_net_cr += cr - dr;
        } else if account_id.starts_with(POSITION_PREFIX) {
            position_net_dr += dr - cr;
        } else if account_id.starts_with(FEE_PREFIX) {
            fees_net_dr += dr - cr;
        } else if account_id.starts_with(INCOME_PREFIX) {
            income_net_dr += dr - cr;
        }
    }
    if position_net_dr + fees_net_dr + income_net_dr != cash_net_cr {
        out.push(Violation::CashLegReconciliation {
            position_net_dr,
            fees_net_dr,
            income_net_dr,
            cash_net_cr,
        });
    }

    // (3) Per-transaction balance + per-fill shape.
    let header_ids: BTreeSet<&str> = headers.iter().map(|(id, _)| id.as_str()).collect();
    for (txn_id, fill_id) in &headers {
        let Some(entries) = by_txn.get(txn_id) else {
            out.push(Violation::OrphanHeader {
                txn_id: txn_id.clone(),
            });
            continue;
        };

        let dr: Decimal = entries.iter().map(|(_, dr, _)| *dr).sum();
        let cr: Decimal = entries.iter().map(|(_, _, cr)| *cr).sum();
        if dr != cr {
            out.push(Violation::TransactionImbalance {
                txn_id: txn_id.clone(),
                debits: dr,
                credits: cr,
            });
        }

        // Only transactions the writer marked as fills (`fill_id` — migration
        // 011) carry the whole-fill shape. Memo rows (kill-switch trips,
        // registry events) are single zero-amount entries by design and must
        // not be judged against it.
        if fill_id.is_none() {
            continue;
        }
        let has_cash = entries.iter().any(|(acct, _, _)| acct == CASH_ACCOUNT);
        let has_position = entries
            .iter()
            .any(|(acct, _, _)| acct.starts_with(POSITION_PREFIX));
        if has_position && !has_cash {
            out.push(Violation::FillWithoutCashLeg {
                txn_id: txn_id.clone(),
            });
        }
        if has_cash && !has_position {
            out.push(Violation::FillWithoutPositionLeg {
                txn_id: txn_id.clone(),
            });
        }
        // A Buy writes 4 entries and a Sell writes 5; anything shorter cannot
        // be a whole fill. The floor (rather than an exact count) is the
        // durability claim — asserting the exact shape belongs to the writer's
        // own tests, and pinning it here would make this gate brittle against
        // a legitimate future leg.
        if entries.len() < BUY_ENTRIES {
            out.push(Violation::PartialFill {
                txn_id: txn_id.clone(),
                entries: entries.len(),
            });
        }
    }

    // (4) Entries whose header did not survive.
    for txn_id in by_txn.keys() {
        if !header_ids.contains(txn_id.as_str()) {
            out.push(Violation::OrphanEntry {
                txn_id: txn_id.clone(),
            });
        }
    }

    out
}

// ── child-process protocol ────────────────────────────────────────────────────

const CHILD_DB_ENV: &str = "AC2_CRASH_CHILD_DB";
const CHILD_READY_ENV: &str = "AC2_CRASH_CHILD_READY";
const CHILD_TEST_NAME: &str = "crash_writer_child_helper_do_not_run_directly";

/// Fixed id for the transaction the child opens and never commits, so the
/// parent can assert its absence by primary key and not merely by count.
const PHANTOM_TXN_ID: &str = "3a21ac02-0000-4000-8000-000000000001";
const PHANTOM_FILL_ID: &str = "3a21ac02-0000-4000-8000-0000000000f1";

/// Rows the child inserts *inside* the uncommitted transaction to force
/// SQLite to spill dirty uncommitted pages out of the page cache. Paired with
/// a deliberately tiny page cache (`PRAGMA cache_size`), this reliably puts
/// uncommitted bytes on the disk before the kill.
const SPILL_ROWS: usize = 4_000;

/// Minimum on-disk growth the child must observe before it will declare itself
/// ready. Below this the kill would have nothing to recover from and the
/// parent's assertions would be vacuous (bug-log #66), so the child fails
/// loudly instead. Set well under the ~1.4 MB actually observed, so the guard
/// catches "nothing happened" without being brittle across SQLite builds.
const MIN_SPILL_BYTES: u64 = 32 * 1024;

fn file_len(path: &Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

/// Length of a SQLite sidecar (`-wal`, `-journal`) next to `db`.
fn sidecar_len(db: &Path, suffix: &str) -> u64 {
    let mut raw = db.as_os_str().to_os_string();
    raw.push(suffix);
    file_len(Path::new(&raw))
}

/// Every byte SQLite is currently holding for this database: the main file
/// plus both possible journal sidecars.
///
/// Measuring all three is what keeps the spill guard honest across journal
/// modes. In `delete` mode (what `Ledger::open` yields today) the uncommitted
/// pages land in the **main file** and only their pre-images in `-journal`, so
/// a sidecar-only probe would under-report by an order of magnitude; in WAL
/// mode it is the other way round.
fn on_disk_bytes(db: &Path) -> u64 {
    file_len(db) + sidecar_len(db, "-wal") + sidecar_len(db, "-journal")
}

/// The writer the parent kills.
///
/// `#[ignore]`d because it never returns on its own: it parks holding an open
/// transaction and waits to be killed. It asserts nothing — every claim is
/// made by the parent, which reads this process's sentinel file and then its
/// on-disk wreckage, so a failure here still fails a gated test rather than
/// hiding in an ignored one. Run directly (`cargo test -- --ignored`) without
/// [`CHILD_DB_ENV`] set, it returns immediately as a no-op.
#[tokio::test]
#[ignore = "child writer driven by ac2_killed_process_reloads_to_whole_fills; parked until killed"]
async fn crash_writer_child_helper_do_not_run_directly() {
    let Ok(db_path) = std::env::var(CHILD_DB_ENV) else {
        // Not invoked by the parent — nothing to do.
        return;
    };
    let ready_path = PathBuf::from(
        std::env::var(CHILD_READY_ENV).expect("parent must set the ready-sentinel path"),
    );
    let db_path = PathBuf::from(db_path);

    match child_write_and_park(&db_path, &ready_path).await {
        Ok(()) => {}
        Err(msg) => {
            let _ = std::fs::write(&ready_path, format!("FAIL {msg}\n"));
        }
    }
}

/// Commit a paper session, open a half-applied fill, force it onto the disk,
/// signal the parent, and park without committing.
async fn child_write_and_park(db_path: &Path, ready_path: &Path) -> Result<(), String> {
    let ledger = open_file_ledger(db_path).await;
    write_paper_session(&ledger).await;

    let journal_mode: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(ledger.pool())
        .await
        .map_err(|e| format!("read journal_mode: {e}"))?;

    // Pin one connection so the cache pragma and the transaction share it.
    let mut conn = ledger
        .pool()
        .acquire()
        .await
        .map_err(|e| format!("acquire: {e}"))?;
    // A 16-page cache guarantees the uncommitted rows below overflow it and
    // spill to the sidecar rather than sitting in RAM until COMMIT.
    sqlx::query("PRAGMA cache_size = 16")
        .execute(&mut *conn)
        .await
        .map_err(|e| format!("set cache_size: {e}"))?;

    let baseline = on_disk_bytes(db_path);

    let mut db_txn = sqlx::Connection::begin(&mut *conn)
        .await
        .map_err(|e| format!("begin: {e}"))?;

    let ts = "1970-01-01T02:00:00Z";
    // The header the real writer INSERTs first…
    sqlx::query(
        "INSERT INTO journal_transactions \
         (id, ts, description, strategy_id, venue, fill_id, signal_id) \
         VALUES (?, ?, ?, ?, ?, ?, NULL)",
    )
    .bind(PHANTOM_TXN_ID)
    .bind(ts)
    .bind("Buy 0.5 BTCUSDT @ 51000")
    .bind(PAPER_STRATEGY)
    .bind(Venue::Binance.to_string())
    .bind(PHANTOM_FILL_ID)
    .execute(&mut *db_txn)
    .await
    .map_err(|e| format!("insert phantom header: {e}"))?;

    // …and only the position leg of the pair. No `Cr assets:cash:USDT`: this
    // is exactly the "position without its cash leg" AC2 forbids on reload.
    sqlx::query(
        "INSERT INTO journal_entries \
         (id, transaction_id, account_id, debit_amount, credit_amount, ts, memo) \
         VALUES (?, ?, ?, ?, '0', ?, 'ac2 half-applied fill')",
    )
    .bind(format!("{PHANTOM_TXN_ID}-leg"))
    .bind(PHANTOM_TXN_ID)
    .bind("assets:position:BTCUSDT")
    .bind("25500.00")
    .bind(ts)
    .execute(&mut *db_txn)
    .await
    .map_err(|e| format!("insert phantom position leg: {e}"))?;

    // Push the uncommitted transaction past the page cache so its pages are
    // physically written to the disk before the kill lands.
    for i in 0..SPILL_ROWS {
        sqlx::query(
            "INSERT INTO journal_entries \
             (id, transaction_id, account_id, debit_amount, credit_amount, ts, memo) \
             VALUES (?, ?, 'assets:position:BTCUSDT', '0.01', '0', ?, 'ac2 spill')",
        )
        .bind(format!("{PHANTOM_TXN_ID}-spill-{i:06}"))
        .bind(PHANTOM_TXN_ID)
        .bind(ts)
        .execute(&mut *db_txn)
        .await
        .map_err(|e| format!("insert spill row {i}: {e}"))?;
    }

    let grown = on_disk_bytes(db_path).saturating_sub(baseline);
    if grown < MIN_SPILL_BYTES {
        return Err(format!(
            "only {grown} uncommitted bytes reached the disk (need >= {MIN_SPILL_BYTES}, \
             journal_mode={journal_mode}) — the kill would have nothing to recover \
             from and the parent's assertions would be vacuous"
        ));
    }

    std::fs::write(
        ready_path,
        format!("READY spill_bytes={grown} journal_mode={journal_mode}\n"),
    )
    .map_err(|e| format!("write ready sentinel: {e}"))?;

    // Park holding the open transaction. The parent kills us here. The cap is
    // only a safety net so a stranded child cannot outlive the suite; reaching
    // it drops `db_txn`, which rolls back cleanly and is NOT the case under
    // test — the parent fails first on its own poll timeout.
    let deadline = Instant::now() + Duration::from_secs(120);
    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
    }
    drop(db_txn);
    Ok(())
}

// ── AC2 proper: the kill ──────────────────────────────────────────────────────

/// **AC2 — a hard kill mid-transaction reloads to a whole number of fills.**
///
/// Spawns the child writer, waits for it to report that uncommitted bytes are
/// on the disk, kills it, and reopens the same database file. The reload must
/// show exactly [`COMMITTED_FILLS`] fills, exactly [`expected_entry_rows`]
/// entries, the hand-computed cash balance to the cent, no trace of the
/// phantom transaction, and zero coherence violations.
#[tokio::test]
async fn ac2_killed_process_reloads_to_whole_fills() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("ac2-crash.sqlite");
    let ready_path = dir.path().join("child-ready.txt");

    let exe = std::env::current_exe().expect("current_exe");
    let mut child = Command::new(&exe)
        .args([
            CHILD_TEST_NAME,
            "--exact",
            "--ignored",
            "--test-threads",
            "1",
        ])
        .env(CHILD_DB_ENV, &db_path)
        .env(CHILD_READY_ENV, &ready_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|e| panic!("spawn child writer {}: {e}", exe.display()));

    // Wait for the child to have uncommitted bytes on the disk.
    let deadline = Instant::now() + Duration::from_secs(90);
    let sentinel = loop {
        if ready_path.exists() {
            break std::fs::read_to_string(&ready_path).expect("read ready sentinel");
        }
        if let Some(status) = child.try_wait().expect("try_wait child") {
            panic!(
                "child writer exited ({status}) before signalling ready; \
                 sentinel present: {}",
                ready_path.exists()
            );
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("child writer never signalled ready within 90s");
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    };

    if let Some(reason) = sentinel.strip_prefix("FAIL ") {
        let _ = child.kill();
        let _ = child.wait();
        panic!(
            "child writer could not set up an honest crash: {}",
            reason.trim()
        );
    }
    assert!(
        sentinel.starts_with("READY "),
        "unexpected sentinel content: {sentinel:?}"
    );

    // Anti-vacuity, re-asserted by the parent: uncommitted data really is on
    // the disk at the moment of the kill, so the kill has something to undo.
    let spill_bytes: u64 = sentinel
        .split_whitespace()
        .find_map(|tok| tok.strip_prefix("spill_bytes="))
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| panic!("sentinel missing spill_bytes: {sentinel:?}"));
    let journal_mode = sentinel
        .split_whitespace()
        .find_map(|tok| tok.strip_prefix("journal_mode="))
        .unwrap_or_else(|| panic!("sentinel missing journal_mode: {sentinel:?}"))
        .to_string();
    assert!(
        spill_bytes >= MIN_SPILL_BYTES,
        "child reported only {spill_bytes} uncommitted bytes on disk \
         (need >= {MIN_SPILL_BYTES}); the crash would be a no-op and the \
         assertions below vacuous — see bug-log #66"
    );

    let bytes_at_kill = on_disk_bytes(&db_path);
    // Visible under `--nocapture`: the evidence that this run exercised a real
    // crash rather than an empty one.
    eprintln!(
        "AC2 crash proof: journal_mode={journal_mode}, {spill_bytes} uncommitted \
         bytes on disk, {bytes_at_kill} total bytes at kill"
    );
    child.kill().expect("kill child writer");
    let status = child.wait().expect("wait for killed child");

    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(
            status.signal(),
            Some(9),
            "child must die by SIGKILL, not exit cleanly (status {status}) — a \
             clean exit would mean sqlx rolled the transaction back through Drop, \
             which is the weaker sibling test, not this one"
        );
    }
    #[cfg(not(unix))]
    assert!(
        !status.success(),
        "child must be terminated abruptly, not exit cleanly (status {status})"
    );

    assert!(
        bytes_at_kill >= spill_bytes,
        "the database shrank between the child's measurement and the kill \
         ({bytes_at_kill} < {spill_bytes}); the uncommitted work was not on \
         disk when the process died"
    );

    // ── reload ────────────────────────────────────────────────────────────
    let ledger = open_file_ledger(&db_path).await;

    let violations = reload_violations(&ledger).await;
    assert!(
        violations.is_empty(),
        "paper portfolio reloaded incoherent after a mid-transaction kill \
         (journal_mode={journal_mode}, {spill_bytes} uncommitted bytes were on \
         disk): {violations:#?}"
    );

    // The interrupted fill left no trace — neither header nor any entry.
    let phantom_headers: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM journal_transactions WHERE id = ?")
            .bind(PHANTOM_TXN_ID)
            .fetch_one(ledger.pool())
            .await
            .expect("count phantom headers");
    assert_eq!(
        phantom_headers.0, 0,
        "the never-committed transaction header survived the kill"
    );
    let phantom_entries: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM journal_entries WHERE transaction_id = ?")
            .bind(PHANTOM_TXN_ID)
            .fetch_one(ledger.pool())
            .await
            .expect("count phantom entries");
    assert_eq!(
        phantom_entries.0, 0,
        "entries from the never-committed transaction survived the kill"
    );

    // A whole number of applied fills — not one row more or fewer.
    let fill_txns: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM journal_transactions WHERE fill_id IS NOT NULL")
            .fetch_one(ledger.pool())
            .await
            .expect("count fill transactions");
    assert_eq!(
        fill_txns.0, COMMITTED_FILLS as i64,
        "expected exactly {COMMITTED_FILLS} committed fills after the kill"
    );
    let entry_rows: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM journal_entries")
        .fetch_one(ledger.pool())
        .await
        .expect("count journal entries");
    assert_eq!(
        entry_rows.0,
        expected_entry_rows(),
        "entry count must be exactly the whole-fill total; a partial fill or a \
         surviving spill row would move it"
    );

    // AD-9: the money reconciles to the cent against a hand-computed figure.
    let cash = query::cash_balance(&ledger).await.expect("cash_balance");
    assert_eq!(
        cash.amount(),
        expected_cash_balance(),
        "reloaded cash must equal the hand-computed paper-session balance exactly"
    );

    // Every surviving transaction balances by the production reconciler too.
    for txn_id in query::all_transaction_ids(&ledger)
        .await
        .expect("all_transaction_ids")
    {
        query::verify_transaction_balance(&ledger, &txn_id)
            .await
            .unwrap_or_else(|e| panic!("reloaded transaction {txn_id} does not balance: {e}"));
    }

    // The position side reloaded as well — otherwise "no position without its
    // cash leg" would hold trivially on an empty position set.
    let positions = query::open_positions_at(&ledger, ts_offset_secs(100_000))
        .await
        .expect("open_positions_at");
    assert!(
        !positions.is_empty(),
        "the paper session must leave open positions to reload; an empty set \
         would make the cash-leg assertions vacuous"
    );
    for p in &positions {
        assert_ne!(
            p.qty,
            Decimal::ZERO,
            "reloaded position for {} must carry a non-zero qty",
            p.symbol
        );
    }
}

// ── the weaker in-process sibling ─────────────────────────────────────────────

/// An abandoned (never-committed) transaction leaves no trace across a reopen.
///
/// Deliberately the weaker proof: this exercises sqlx's `Drop` → SQLite
/// `ROLLBACK`, **not** the OS-level crash path. It is here because it
/// localises a writer-shape regression in about a second, where the kill test
/// takes ten.
#[tokio::test]
async fn ac2_aborted_transaction_leaves_no_trace() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("ac2-abort.sqlite");

    {
        let ledger = open_file_ledger(&db_path).await;
        write_paper_session(&ledger).await;

        let mut db_txn = ledger.pool().begin().await.expect("begin");
        sqlx::query(
            "INSERT INTO journal_transactions \
             (id, ts, description, strategy_id, venue, fill_id, signal_id) \
             VALUES (?, '1970-01-01T02:00:00Z', 'Buy 0.5 BTCUSDT @ 51000', ?, ?, ?, NULL)",
        )
        .bind(PHANTOM_TXN_ID)
        .bind(PAPER_STRATEGY)
        .bind(Venue::Binance.to_string())
        .bind(PHANTOM_FILL_ID)
        .execute(&mut *db_txn)
        .await
        .expect("insert phantom header");
        sqlx::query(
            "INSERT INTO journal_entries \
             (id, transaction_id, account_id, debit_amount, credit_amount, ts, memo) \
             VALUES (?, ?, 'assets:position:BTCUSDT', '25500.00', '0', \
                     '1970-01-01T02:00:00Z', 'ac2 half-applied fill')",
        )
        .bind(format!("{PHANTOM_TXN_ID}-leg"))
        .bind(PHANTOM_TXN_ID)
        .execute(&mut *db_txn)
        .await
        .expect("insert phantom position leg");

        // Abandon it. No commit, no explicit rollback.
        drop(db_txn);
    }

    // Fresh pool over the same file — the reload the operator would get.
    let ledger = open_file_ledger(&db_path).await;
    let violations = reload_violations(&ledger).await;
    assert!(
        violations.is_empty(),
        "abandoned transaction left the reloaded portfolio incoherent: {violations:#?}"
    );
    let phantom: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM journal_transactions WHERE id = ?")
        .bind(PHANTOM_TXN_ID)
        .fetch_one(ledger.pool())
        .await
        .expect("count phantom headers");
    assert_eq!(phantom.0, 0, "abandoned transaction header survived");
    assert_eq!(
        query::cash_balance(&ledger)
            .await
            .expect("cash_balance")
            .amount(),
        expected_cash_balance(),
        "cash must be unchanged by the abandoned transaction"
    );
}

// ── positive control ──────────────────────────────────────────────────────────

/// A fully applied paper session reloads with zero violations. The baseline the
/// negative controls below are read against.
#[tokio::test]
async fn positive_control_clean_session_has_no_violations() {
    let ledger = Ledger::in_memory().await.expect("open in-memory");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap");
    write_paper_session(&ledger).await;

    let violations = reload_violations(&ledger).await;
    assert!(
        violations.is_empty(),
        "a clean paper session must produce no violations: {violations:#?}"
    );
}

// ── negative controls: proof the contract can fail ────────────────────────────
//
// Each control commits — deliberately and durably — one of the shapes a
// non-atomic writer would leave behind, then asserts `reload_violations`
// reports it. Without these, "no violations after the kill" would be
// indistinguishable from "the check never fires" (bug-log #66). These run
// in-memory: the subject is the contract, not the storage.

async fn ledger_with_clean_session() -> Ledger {
    let ledger = Ledger::in_memory().await.expect("open in-memory");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap");
    write_paper_session(&ledger).await;
    ledger
}

async fn commit_header(ledger: &Ledger, txn_id: &str) {
    sqlx::query(
        "INSERT INTO journal_transactions \
         (id, ts, description, strategy_id, venue, fill_id, signal_id) \
         VALUES (?, '1970-01-01T02:00:00Z', 'Buy 0.5 BTCUSDT @ 51000', ?, ?, ?, NULL)",
    )
    .bind(txn_id)
    .bind(PAPER_STRATEGY)
    .bind(Venue::Binance.to_string())
    .bind(PHANTOM_FILL_ID)
    .execute(ledger.pool())
    .await
    .expect("insert header");
}

async fn commit_entry(ledger: &Ledger, txn_id: &str, leg: &str, account: &str, dr: &str, cr: &str) {
    sqlx::query(
        "INSERT INTO journal_entries \
         (id, transaction_id, account_id, debit_amount, credit_amount, ts, memo) \
         VALUES (?, ?, ?, ?, ?, '1970-01-01T02:00:00Z', 'ac2 negative control')",
    )
    .bind(format!("{txn_id}-{leg}"))
    .bind(txn_id)
    .bind(account)
    .bind(dr)
    .bind(cr)
    .execute(ledger.pool())
    .await
    .expect("insert entry");
}

/// NEGATIVE CONTROL 1 — a **committed** half-applied fill (position leg, no
/// cash leg) is caught.
///
/// This is precisely the state the kill test asserts does *not* survive. Here
/// it is written durably, and the contract must report it.
#[tokio::test]
async fn negative_control_committed_half_fill_is_caught() {
    let ledger = ledger_with_clean_session().await;
    assert!(
        reload_violations(&ledger).await.is_empty(),
        "control must start clean"
    );

    let txn_id = "ac2-nc1-half-fill";
    commit_header(&ledger, txn_id).await;
    commit_entry(
        &ledger,
        txn_id,
        "pos",
        "assets:position:BTCUSDT",
        "25500.00",
        "0",
    )
    .await;

    let violations = reload_violations(&ledger).await;
    let found = kinds(&violations);
    for expected in [
        "GlobalImbalance",
        "CashLegReconciliation",
        "TransactionImbalance",
        "FillWithoutCashLeg",
        "PartialFill",
    ] {
        assert!(
            found.contains(&expected),
            "a committed half-applied fill must trip {expected}; got {violations:#?}"
        );
    }
}

/// NEGATIVE CONTROL 2 — an orphan transaction header (no entries) is caught.
///
/// The literal torn-write shape: the writer INSERTs the header before any leg,
/// so a header standing alone is the first thing a non-atomic crash would leave.
#[tokio::test]
async fn negative_control_orphan_header_is_caught() {
    let ledger = ledger_with_clean_session().await;
    assert!(
        reload_violations(&ledger).await.is_empty(),
        "control must start clean"
    );

    let txn_id = "ac2-nc2-orphan-header";
    commit_header(&ledger, txn_id).await;

    let violations = reload_violations(&ledger).await;
    assert!(
        kinds(&violations).contains(&"OrphanHeader"),
        "a header with no entries must trip OrphanHeader; got {violations:#?}"
    );
    // The money still balances — which is exactly why the shape check has to
    // exist alongside the balance check.
    assert!(
        !kinds(&violations).contains(&"GlobalImbalance"),
        "an orphan header moves no money, so the balance check alone would miss \
         it: {violations:#?}"
    );
}

/// NEGATIVE CONTROL 3 — a whole-shaped fill that is off by **exactly one cent**
/// is caught.
///
/// Four entries, both legs present, nothing structurally wrong: only the money
/// is short, by 0.01 USDT. This is the AD-9 exact-cent claim — a reconciliation
/// written against a tolerance of a cent or more would wave this through.
#[tokio::test]
async fn negative_control_one_cent_imbalance_is_caught() {
    let ledger = ledger_with_clean_session().await;
    assert!(
        reload_violations(&ledger).await.is_empty(),
        "control must start clean"
    );

    let txn_id = "ac2-nc3-one-cent";
    commit_header(&ledger, txn_id).await;
    commit_entry(
        &ledger,
        txn_id,
        "pos",
        "assets:position:BTCUSDT",
        "100.00",
        "0",
    )
    .await;
    commit_entry(&ledger, txn_id, "cash", CASH_ACCOUNT, "0", "100.00").await;
    commit_entry(&ledger, txn_id, "fee", "expense:fees:taker", "0.10", "0").await;
    // The cash leg for the fee is one cent short.
    commit_entry(&ledger, txn_id, "feecash", CASH_ACCOUNT, "0", "0.09").await;

    let violations = reload_violations(&ledger).await;
    let found = kinds(&violations);
    assert!(
        found.contains(&"TransactionImbalance"),
        "a one-cent drift must trip TransactionImbalance; got {violations:#?}"
    );
    assert!(
        found.contains(&"GlobalImbalance"),
        "a one-cent drift must trip GlobalImbalance; got {violations:#?}"
    );
    // The structural checks stay silent — the shape is a whole fill. This is
    // what makes the control a clean test of the money check specifically.
    assert!(
        !found.contains(&"PartialFill") && !found.contains(&"FillWithoutCashLeg"),
        "a four-entry, both-legs fill must not trip the shape checks: {violations:#?}"
    );

    let drift = violations
        .iter()
        .find_map(|v| match v {
            Violation::GlobalImbalance { debits, credits } => Some(*debits - *credits),
            _ => None,
        })
        .expect("GlobalImbalance present");
    assert_eq!(
        drift,
        dec!(0.01),
        "the contract must resolve a drift of exactly one cent (AD-9), not \
         round it away"
    );
}
