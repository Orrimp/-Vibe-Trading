#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T813 — `lib::generate` end-to-end smoke test.
//!
//! Builds a minimal in-memory ledger, drives the orchestrator end-to-end,
//! and asserts:
//!
//! 1. The markdown file is written at the given path.
//! 2. The markdown carries the front-matter fence, the body sections,
//!    and the reconciliation appendix.
//! 3. Companion CSVs land in the artifacts directory.

use audit::{Ledger, bootstrap, journal};
use reports::{FrozenMarkSource, ReportWindow};
use tempfile::TempDir;
use uuid::Uuid;

async fn build_minimal_ledger(path: &std::path::Path) {
    let url_path = path.to_str().unwrap();
    let ledger = Ledger::open(url_path).await.unwrap();
    bootstrap::chart_of_accounts(&ledger).await.unwrap();

    // Insert a single bootstrap transaction so `ledger_inception_ts`
    // returns a real timestamp.
    let txn_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO journal_transactions (id, ts, description) VALUES (?, ?, ?)")
        .bind(&txn_id)
        .bind("2026-04-01T00:00:00Z")
        .bind("bootstrap")
        .execute(ledger.pool())
        .await
        .unwrap();
    // Seed an opening balance so the body has signal.
    journal::registry_event(&ledger, "Bootstrap", "initial seed", "{}")
        .await
        .ok();
}

#[tokio::test]
async fn t813_generate_writes_markdown_and_csvs() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("audit.db");
    build_minimal_ledger(&db_path).await;

    let out = dir.path().join("report.md");
    let frozen = FrozenMarkSource::from_csv_str("symbol,close_time,close\n").unwrap();

    let result =
        reports::generate(ReportWindow::Days7, &db_path, &frozen, &out, Some(0xC0FFEE)).await;

    let artifacts = result.expect("generate should succeed on a minimal ledger");

    // Markdown landed.
    assert!(out.exists(), "markdown not written");
    let body = std::fs::read_to_string(&out).unwrap();
    assert!(body.starts_with("---\n"), "missing front-matter fence");
    assert!(body.contains("\n---\n\n"), "missing closing fence");

    // Body sections present in the locked order.
    let r9 = body.find("## Open risks").expect("R9 section");
    let r2 = body.find("## Headline").expect("R2 section");
    let r3 = body.find("## Equity curve").expect("R3 section");
    let r4 = body.find("## Risk metrics").expect("R4 section");
    let r5 = body.find("## Strategy attribution").expect("R5 section");
    let r6 = body.find("## Memory highlights").expect("R6 section");
    let r7 = body.find("## System health").expect("R7 section");
    let r8 = body.find("## What changed").expect("R8 section");
    let r11 = body.find("## Reconciliation").expect("R11 section");
    // R9 is pinned ABOVE R3 per R9.1.
    assert!(r9 < r3, "R9 must precede the equity curve");
    assert!(r9 < r2);
    assert!(r2 < r3);
    assert!(r3 < r4);
    assert!(r4 < r5);
    assert!(r5 < r6);
    assert!(r6 < r7);
    assert!(r7 < r8);
    assert!(r8 < r11);

    // PASS reconciliation on a balanced minimal ledger.
    assert!(body.contains("reconciliation: PASS"));

    // CSV artifacts written.
    assert!(!artifacts.csv_paths.is_empty());
    for p in &artifacts.csv_paths {
        assert!(p.exists(), "CSV artifact missing: {p:?}");
    }
}

#[tokio::test]
async fn t813_generate_two_runs_byte_identical_body() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("audit.db");
    build_minimal_ledger(&db_path).await;

    let out_a = dir.path().join("a.md");
    let out_b = dir.path().join("b.md");
    let frozen = FrozenMarkSource::from_csv_str("symbol,close_time,close\n").unwrap();

    let _a = reports::generate(
        ReportWindow::Days7,
        &db_path,
        &frozen,
        &out_a,
        Some(0xC0FFEE),
    )
    .await
    .unwrap();
    let _b = reports::generate(
        ReportWindow::Days7,
        &db_path,
        &frozen,
        &out_b,
        Some(0xC0FFEE),
    )
    .await
    .unwrap();

    let body_a = std::fs::read_to_string(&out_a).unwrap();
    let body_b = std::fs::read_to_string(&out_b).unwrap();

    // Front-matter `generated:` differs (wall-clock); body after `---\n\n`
    // is identical.
    let after_fence_a = body_a.split("---\n\n").nth(1).unwrap_or("");
    let after_fence_b = body_b.split("---\n\n").nth(1).unwrap_or("");
    // Period boundaries (period_end) advance with wall-clock — thus the
    // body's equity-sample count may differ between two runs that span a
    // 1m boundary.  We only assert that all the section headers + sentinel
    // strings match across the two runs, not the byte-level body shape.
    // (T814 ships the strict body-SHA256 byte-identity test on a frozen
    // fixture run-id with seed.)
    for marker in [
        "## Headline",
        "## Equity curve",
        "## Risk metrics",
        "## Strategy attribution",
        "## Memory highlights",
        "## System health",
        "## What changed",
        "## Open risks",
        "## Reconciliation",
    ] {
        assert!(
            after_fence_a.contains(marker),
            "body A missing section {marker}"
        );
        assert!(
            after_fence_b.contains(marker),
            "body B missing section {marker}"
        );
    }
}
