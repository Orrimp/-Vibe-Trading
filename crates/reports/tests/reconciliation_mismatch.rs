#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T814 — Reconciliation FAIL integration test (R11.4 / V5).
//!
//! Constructs a deliberate one-cent imbalance between the inception
//! `realized_pnl_since` query and the period-windowed
//! `pnl_by_strategy` / `pnl_by_symbol` queries by inserting an
//! `income:realized_pnl` journal row dated **before** the report's
//! `period_start`.  The headline identity still balances (both sides
//! see the same `realized + 0`), but the by-strategy and by-symbol
//! identities fail because the period-windowed sums omit the
//! pre-period entry while the inception-side `realized` includes it.
//!
//! Asserts the four R11.4 acceptance criteria:
//!
//! - (a) `*** RECONCILIATION FAILURE …` banner present in the body.
//! - (b) `FAIL` cell present in the R11 appendix table.
//! - (c) Sibling `_reconciliation_failure.json` written next to the
//!   markdown with the expected schema fields.
//! - (d) The `report` binary exits with code 1.

use std::process::Command;

use audit::{bootstrap, journal, Ledger};
use reports::{FrozenMarkSource, ReportError, ReportWindow};
use rust_decimal_macros::dec;
use tempfile::TempDir;
use uuid::Uuid;

/// Build a ledger that has a realized-pnl entry dated **before** the
/// 7d window's `period_start = now - 7d`.  The entry is dated 100 days
/// ago so it's safely outside any 7d window the test may resolve.
async fn build_imbalanced_ledger(path: &std::path::Path) {
    let url_path = path.to_str().unwrap();
    let ledger = Ledger::open(url_path).await.unwrap();
    bootstrap::chart_of_accounts(&ledger).await.unwrap();

    // Bootstrap transaction at a stable past timestamp so
    // `ledger_inception_ts` resolves cleanly.  Use a date well outside
    // the 7d window.
    let bootstrap_txn = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO journal_transactions (id, ts, description) VALUES (?, ?, ?)")
        .bind(&bootstrap_txn)
        .bind("2026-01-01T00:00:00Z")
        .bind("bootstrap")
        .execute(ledger.pool())
        .await
        .unwrap();
    journal::registry_event(&ledger, "Bootstrap", "initial seed", "{}")
        .await
        .ok();

    // Insert a separate realized-pnl transaction whose `ts` lies
    // BEFORE `period_start = now - 7d`.  We pick `2026-01-15T00:00:00Z`
    // — far outside any 7d window relative to the test's `now`.
    let pnl_txn = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO journal_transactions (id, ts, description, strategy_id) \
         VALUES (?, ?, ?, ?)",
    )
    .bind(&pnl_txn)
    .bind("2026-01-15T00:00:00.000000Z")
    .bind("pre-period realized pnl")
    .bind("test_strategy")
    .execute(ledger.pool())
    .await
    .unwrap();

    // Single `income:realized_pnl` entry with a $0.01 credit.  The
    // period-windowed `pnl_by_strategy` / `pnl_by_symbol` queries
    // reject this (ts < period_start) while the inception-side
    // `realized_pnl_since(inception)` includes it.
    let entry_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO journal_entries \
         (id, transaction_id, account_id, debit_amount, credit_amount, ts) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&entry_id)
    .bind(&pnl_txn)
    .bind("income:realized_pnl")
    .bind(dec!(0).to_string())
    .bind(dec!(0.01).to_string())
    .bind("2026-01-15T00:00:00.000000Z")
    .execute(ledger.pool())
    .await
    .unwrap();
}

#[tokio::test]
async fn t814_reconciliation_fail_writes_banner_table_and_sibling_json() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("audit.db");
    build_imbalanced_ledger(&db_path).await;

    let frozen = FrozenMarkSource::from_csv_str("symbol,close_time,close\n").unwrap();
    let out = dir.path().join("report.md");

    let result =
        reports::generate(ReportWindow::Days7, &db_path, &frozen, &out, Some(0xC0FFEE)).await;

    // ── (d-precursor) lib returns ReportError::Reconciliation ──────
    let sibling_path = match result {
        Err(ReportError::Reconciliation { sibling_path }) => sibling_path,
        Err(other) => panic!("expected ReportError::Reconciliation, got: {other:?}"),
        Ok(_) => panic!("expected reconciliation failure, got Ok"),
    };

    // Markdown still landed despite the error — operators see the
    // broken report.
    assert!(
        out.exists(),
        "markdown should be atomic-written even on FAIL"
    );
    let full = std::fs::read_to_string(&out).unwrap();

    // (a) FAIL banner in the body.
    assert!(
        full.contains("*** RECONCILIATION FAILURE — see Reconciliation section ***"),
        "missing FAIL banner. body:\n{full}"
    );

    // (b) `FAIL` cell present in the R11 appendix table.  The
    // appendix renders cells as `| FAIL |` (literal uppercase).
    let r11_offset = full
        .find("## Reconciliation")
        .expect("R11 appendix section missing");
    let r11_section = &full[r11_offset..];
    assert!(
        r11_section.contains("| FAIL |"),
        "R11 appendix should contain a `FAIL` cell. section:\n{r11_section}"
    );

    // (c) Sibling JSON exists at the expected path next to the
    // markdown, with the documented schema fields.
    let expected_sibling = dir.path().join("report_reconciliation_failure.json");
    assert_eq!(
        sibling_path, expected_sibling,
        "sibling path mismatch: expected {expected_sibling:?}, got {sibling_path:?}"
    );
    assert!(
        sibling_path.exists(),
        "sibling JSON not written at {sibling_path:?}"
    );

    let json_text = std::fs::read_to_string(&sibling_path).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json_text).expect("sibling JSON should parse");
    assert_eq!(v["schema_version"], 1, "schema_version != 1");
    assert!(v["run_id"].is_string(), "run_id missing");
    assert!(
        v["ledger_snapshot_sha"].is_string(),
        "ledger_snapshot_sha missing"
    );
    assert_eq!(v["period"], "7d");
    assert!(v["period_start"].is_string(), "period_start missing");
    assert!(v["period_end"].is_string(), "period_end missing");
    let rows = v["rows"].as_array().expect("rows should be an array");
    assert_eq!(
        rows.len(),
        4,
        "rows should describe all four R11 identities"
    );

    // At least one row reports `passed: false` — that is the failing
    // identity (by_strategy and/or by_symbol against the pre-period
    // realized pnl).
    let any_failed = rows.iter().any(|r| r["passed"].as_bool() == Some(false));
    assert!(any_failed, "at least one row should report passed: false");
}

#[tokio::test]
async fn t814_reconciliation_fail_bin_exits_one() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("audit.db");
    build_imbalanced_ledger(&db_path).await;

    let out = dir.path().join("report.md");
    // `CARGO_BIN_EXE_<bin_name>` is set by Cargo for integration tests
    // of crates that build a binary.
    let bin_path = env!("CARGO_BIN_EXE_report");

    let status = Command::new(bin_path)
        .arg("--period")
        .arg("7d")
        .arg("--ledger")
        .arg(&db_path)
        .arg("--output")
        .arg(&out)
        .arg("--seed")
        .arg("0xC0FFEE")
        .status()
        .expect("spawn report bin");

    // (d) bin exits 1 per R11.4 / R1.6.
    assert_eq!(
        status.code(),
        Some(1),
        "bin should exit 1 on reconciliation failure, got {status:?}"
    );

    // The bin still wrote the markdown and the sibling JSON before
    // exiting non-zero.
    assert!(out.exists(), "markdown should exist after FAIL exit");
    let sibling = dir.path().join("report_reconciliation_failure.json");
    assert!(
        sibling.exists(),
        "sibling JSON should exist after FAIL exit"
    );
}
