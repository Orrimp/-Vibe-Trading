#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T814 — Body-no-volatile-metadata negative invariant (R10.4).
//!
//! Asserts that the rendered report **body** (the bytes after the
//! closing `---\n\n` fence — the byte range hashed by the anchor
//! gate) does NOT contain any of the eight forbidden substrings
//! enumerated in `spec/features/operator-success-reports.md` under
//! "Body-vs-front-matter discipline (R10.2–R10.5)":
//!
//! 1. `generated:`
//! 2. `run_id:`
//! 3. `wall_clock_s:`
//! 4. `ledger_snapshot_sha:`
//! 5. `data_source:`
//! 6. `agent_pid:`
//! 7. `host:`
//! 8. `git_commit:`
//!
//! These are run-varying fields that MUST live in the YAML
//! front-matter only.  HF-1 (`wall_clock_s` leak) and T715
//! (`data_source` leak) each cost an anchor-rotation round; this
//! invariant is the codified gate.

use audit::{bootstrap, journal, Ledger};
use reports::{FrozenMarkSource, ReportWindow};
use tempfile::TempDir;
use uuid::Uuid;

async fn build_minimal_ledger(path: &std::path::Path) {
    let url_path = path.to_str().unwrap();
    let ledger = Ledger::open(url_path).await.unwrap();
    bootstrap::chart_of_accounts(&ledger).await.unwrap();

    let txn_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO journal_transactions (id, ts, description) VALUES (?, ?, ?)")
        .bind(&txn_id)
        .bind("2026-04-01T00:00:00Z")
        .bind("bootstrap")
        .execute(ledger.pool())
        .await
        .unwrap();
    journal::registry_event(&ledger, "Bootstrap", "initial seed", "{}")
        .await
        .ok();
}

/// Slice off the front-matter and return the body bytes (after the
/// closing `---\n\n` fence).  Mirrors the byte-range convention used
/// by `crates/backtest/src/main.rs::write_report` and the
/// `scripts/hash_report.py` anchor hasher.
fn body_after_fence(full: &str) -> &str {
    let after_open = full.strip_prefix("---\n").unwrap_or(full);
    let close_marker = "---\n\n";
    match after_open.find(close_marker) {
        Some(pos) => &after_open[pos + close_marker.len()..],
        None => "",
    }
}

#[tokio::test]
async fn t814_body_does_not_contain_any_volatile_substring() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("audit.db");
    build_minimal_ledger(&db_path).await;

    let frozen = FrozenMarkSource::from_csv_str("symbol,close_time,close\n").unwrap();
    let out = dir.path().join("report.md");

    reports::generate(ReportWindow::Days7, &db_path, &frozen, &out, Some(0xC0FFEE))
        .await
        .expect("generate should succeed on a balanced minimal ledger");

    let full = std::fs::read_to_string(&out).unwrap();
    let body = body_after_fence(&full);
    assert!(!body.is_empty(), "body slice should be non-empty");

    // R10.4 — the eight forbidden substrings.  These are YAML
    // front-matter keys with their colon suffix; no body section may
    // emit them.
    let forbidden = [
        "generated:",
        "run_id:",
        "wall_clock_s:",
        "ledger_snapshot_sha:",
        "data_source:",
        "agent_pid:",
        "host:",
        "git_commit:",
    ];

    for needle in forbidden {
        assert!(
            !body.contains(needle),
            "R10.4 violation: body contains forbidden substring `{needle}`.\n\
             This field must live in the YAML front-matter only.\n\
             Body bytes:\n{body}"
        );
    }
}

/// T1812 / R5.3 extension — the new `## Memory highlights` body
/// section also contains none of the 8 forbidden substrings.
///
/// Renders both the empty-state body (no cards) AND a lesson-bearing
/// body (3 cards from the 7d sibling fixture) so the negative
/// invariant covers both code paths exposed by the reflection-memory
/// renderer rewrite.
#[tokio::test]
async fn t1812_memory_highlights_body_does_not_contain_volatile_metadata() {
    use reflection::regime::RegimeTag;
    use reflection::types::{LessonCard, RetrievalQuery, SymbolOrPair};
    use reports::render::memory_highlights::render_with_lessons;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;
    use trading_core::{Money, StrategyId, Symbol, Timestamp, Usdt};

    let _ = RetrievalQuery {
        strategy_id: StrategyId::new("sma_crossover"),
        symbol_or_pair: SymbolOrPair::Single(Symbol::new("BTCUSDT")),
        current_regime: RegimeTag::Bull,
    };

    let cards = vec![LessonCard {
        card_id: "vmtest".into(),
        closed_at: Timestamp::new(OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap()),
        symbol_or_pair: SymbolOrPair::Single(Symbol::new("BTCUSDT")),
        strategy_id: StrategyId::new("sma_crossover"),
        signed_pnl: Money::<Usdt>::from_decimal(dec!(123.45)),
        opening_capital: Money::<Usdt>::from_decimal(dec!(10000)),
        holding_period_bars: 60,
        entry_regime: RegimeTag::Bull,
        exit_regime: RegimeTag::Chop,
        outcome_class: reflection::outcome::OutcomeClass::Win,
        note: None,
    }];
    let _ = Decimal::ZERO; // silence unused-import warn

    let bodies = [
        render_with_lessons(&[], &[]),                       // empty state
        render_with_lessons(&[], &cards),                    // lesson-bearing
        render_with_lessons(&["alpha".to_string()], &cards), // with decay footer
    ];

    let forbidden = [
        "generated:",
        "run_id:",
        "wall_clock_s:",
        "ledger_snapshot_sha:",
        "data_source:",
        "agent_pid:",
        "host:",
        "git_commit:",
    ];

    for body in &bodies {
        for needle in forbidden {
            assert!(
                !body.contains(needle),
                "R5.3 violation: memory-highlights body contains forbidden \
                 substring `{needle}`.\nBody:\n{body}"
            );
        }
    }
}
