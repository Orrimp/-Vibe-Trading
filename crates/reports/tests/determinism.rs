#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T814 — Determinism (R10.3 / V4).
//!
//! Runs `lib::generate` twice against the same fixture ledger at seed
//! `0xC0FFEE`, ten seconds apart, and asserts:
//!
//! 1. Front-matter `generated:` differs between the two renders
//!    (wall-clock advanced).
//! 2. Body bytes (post-`---\n\n` fence) are byte-identical via SHA-256.
//!
//! The "10 seconds apart" cadence matches the v1+ acceptance criterion
//! recorded in `spec/features/operator-success-reports.md` under R10
//! → Acceptance: "running the binary twice against the same fixture
//! ledger at the same `--period` and `--seed`, ten seconds apart,
//! produces two files with different front-matter but byte-identical
//! body".

use std::time::Duration;

use audit::{Ledger, bootstrap, journal};
use reports::{FrozenMarkSource, ReportWindow};
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use uuid::Uuid;

/// Build a tiny deterministic ledger that produces zero realized P&L
/// and an empty position set — enough for `lib::generate` to drive
/// every body section to a stable output.
async fn build_minimal_ledger(path: &std::path::Path) {
    let url_path = path.to_str().unwrap();
    let ledger = Ledger::open(url_path).await.unwrap();
    bootstrap::chart_of_accounts(&ledger).await.unwrap();

    // One bootstrap transaction so `ledger_inception_ts` succeeds.
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

/// Split a markdown report into `(front_matter, body)`.  The body
/// starts immediately after the closing `---\n\n` fence — the same
/// byte range the SHA-256 anchor convention hashes.
fn split_front_matter_and_body(full: &str) -> (&str, &str) {
    // Skip the opening fence "---\n", then find "\n---\n\n".
    let after_open = full.strip_prefix("---\n").unwrap_or(full);
    let close_marker = "---\n\n";
    if let Some(pos) = after_open.find(close_marker) {
        let body_start_in_after = pos + close_marker.len();
        let body_start = full.len() - after_open.len() + body_start_in_after;
        let body = &full[body_start..];
        let front_matter = &full[..body_start];
        (front_matter, body)
    } else {
        (full, "")
    }
}

fn body_sha256(body: &str) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(body.as_bytes());
    h.finalize().into()
}

/// Extract the value of a `key:` line from the YAML front-matter slice.
fn front_matter_value(fm: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    for line in fm.lines() {
        if let Some(rest) = line.strip_prefix(&prefix) {
            return Some(rest.trim().to_string());
        }
    }
    None
}

#[tokio::test]
async fn t814_determinism_two_runs_same_seed_byte_identical_body() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("audit.db");
    build_minimal_ledger(&db_path).await;

    // ── Why this test is safe to run against the WALL CLOCK (bug-log #158) ──
    //
    // The body assertion below is "two renders ten seconds apart produce
    // byte-identical bodies". Since 2026-11-07 the equity curve is derived from
    // the ledger and sampled on a grid running to `period_end`, which `Days7`
    // resolves to `now` — so a ten-second shift moves every tick by ten seconds
    // and a movement the grid steps across changes the sampled series. This test
    // is immune only because `build_minimal_ledger` writes a bootstrap row and
    // NO journal entries: zero movements, so the curve is constant whatever the
    // grid does.
    //
    // That is exactly the undeclared accident #158 was about, so it is declared
    // and CHECKED here. Add fees or fills to the fixture and this assertion fires
    // with the reason, instead of the test flaking on a cadence boundary a few
    // runs out of a hundred. (Pinning the clock is not an option: `generated:`
    // is `fmt_ts_micros(now)`, so a pinned clock would defeat assertion 1.)
    {
        let ledger = Ledger::open(db_path.to_str().unwrap()).await.unwrap();
        let inception = audit::query::ledger_inception_ts(&ledger).await.unwrap();
        let now = trading_core::Timestamp::new(time::OffsetDateTime::now_utc());
        let points = audit::query::equity_series(
            &ledger,
            inception,
            now,
            rust_decimal::Decimal::ZERO,
            rust_decimal::Decimal::ZERO,
        )
        .await
        .unwrap();
        assert!(
            points.is_empty(),
            "this test asserts byte-identical BODIES across a ten-second wall-clock \
             shift, which only holds while the ledger has no movements for the \
             equity curve to sample ({} found). The fixture gained journal entries: \
             either keep it movement-free, or move the body assertion to a pinned \
             clock via `reports::generate_at` and drop the `generated:` check.",
            points.len(),
        );
    }

    let frozen = FrozenMarkSource::from_csv_str("symbol,close_time,close\n").unwrap();
    let out_a = dir.path().join("a.md");
    let out_b = dir.path().join("b.md");

    // First render at seed 0xC0FFEE.
    reports::generate(
        ReportWindow::Days7,
        &db_path,
        &frozen,
        &out_a,
        Some(0xC0FFEE),
    )
    .await
    .expect("first render");

    // Sleep 10s so wall-clock-bound front-matter fields advance.
    std::thread::sleep(Duration::from_secs(10));

    // Second render at the same seed.
    reports::generate(
        ReportWindow::Days7,
        &db_path,
        &frozen,
        &out_b,
        Some(0xC0FFEE),
    )
    .await
    .expect("second render");

    let full_a = std::fs::read_to_string(&out_a).unwrap();
    let full_b = std::fs::read_to_string(&out_b).unwrap();
    let (fm_a, body_a) = split_front_matter_and_body(&full_a);
    let (fm_b, body_b) = split_front_matter_and_body(&full_b);

    // ── R10.3 — body bytes byte-identical ──────────────────────────
    let sha_a = body_sha256(body_a);
    let sha_b = body_sha256(body_b);
    assert_eq!(
        sha_a, sha_b,
        "body SHA-256 differs across two runs at same (period, ledger, seed)\n\
         body A:\n{body_a}\n\nbody B:\n{body_b}"
    );

    // ── R10.3 / V4 — front-matter `generated:` differs ─────────────
    let gen_a =
        front_matter_value(fm_a, "generated").expect("missing generated: in front-matter A");
    let gen_b =
        front_matter_value(fm_b, "generated").expect("missing generated: in front-matter B");
    assert_ne!(
        gen_a, gen_b,
        "front-matter `generated:` should differ across two renders 10s apart"
    );
}
