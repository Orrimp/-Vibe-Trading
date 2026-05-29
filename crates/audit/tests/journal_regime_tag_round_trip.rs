//! Integration test for `post_regime_tag` (v3-regime-classifier Wave D — T-D-D1).
//!
//! Verifies that:
//! 1. A `strategy_events` row with `kind = "RegimeTag"` round-trips through
//!    the DB (all fields verbatim).
//! 2. `AuditEvent::RegimeTag` tick fires after commit.
//! 3. Serde byte-identity: JSON serialisation of the tick is stable.
//! 4. Existing fixture ledgers do NOT emit a RegimeTag row (additive variant
//!    contract — byte-identity preserved).

use audit::{
    Ledger,
    journal::post_regime_tag,
    tick::{AuditEvent, AuditTickStream},
};

/// Open an in-memory ledger for tests.
async fn open_ledger() -> Ledger {
    Ledger::in_memory().await.expect("in-memory ledger opens")
}

/// Open an in-memory ledger with a tick bus so we can observe tick emissions.
async fn open_ledger_with_bus() -> (Ledger, AuditTickStream) {
    let (ledger, sender) = Ledger::open_with_tick_bus(":memory:", 64)
        .await
        .expect("in-memory ledger with tick bus");
    let stream = AuditTickStream::new(sender.subscribe(), "test");
    (ledger, stream)
}

/// T-D-D1 — basic round-trip: `strategy_events` row persists with correct fields.
#[tokio::test]
async fn journal_entry_regime_tag_round_trips() {
    let ledger = open_ledger().await;

    post_regime_tag(
        &ledger,
        "BTCUSDT",
        "bull",
        "0.823456",
        Some("2026-05-28T12:00:00.000000Z"),
    )
    .await
    .expect("post_regime_tag must succeed");

    // Read back the row from strategy_events.
    let row: (String, String, String, String, String) = sqlx::query_as(
        "SELECT kind, error_code, error_summary, operator, venue \
         FROM strategy_events \
         WHERE kind = 'RegimeTag' \
         ORDER BY ts DESC \
         LIMIT 1",
    )
    .fetch_one(ledger.pool())
    .await
    .expect("RegimeTag row must exist");

    assert_eq!(row.0, "RegimeTag", "kind column");
    assert_eq!(row.1, "regime_tag", "error_code column");
    assert_eq!(row.3, "system", "operator column");
    assert!(row.4.is_empty(), "venue must be NULL/empty");

    // Verify the error_summary JSON contains all three expected fields.
    let summary: serde_json::Value =
        serde_json::from_str(&row.2).expect("error_summary must be valid JSON");
    assert_eq!(summary["symbol"], "BTCUSDT", "symbol in error_summary");
    assert_eq!(summary["regime"], "bull", "regime in error_summary");
    assert_eq!(
        summary["max_confidence"], "0.823456",
        "max_confidence in error_summary"
    );
}

/// T-D-D1 — all four regime values round-trip verbatim.
#[tokio::test]
async fn regime_tag_all_regime_variants_persist() {
    let ledger = open_ledger().await;

    for (symbol, regime, confidence) in [
        ("BTCUSDT", "bull", "0.810000"),
        ("ETHUSDT", "bear", "0.750000"),
        ("BNBUSDT", "volatile", "0.720000"),
        ("SOLUSDT", "calm", "0.700000"),
    ] {
        post_regime_tag(&ledger, symbol, regime, confidence, None)
            .await
            .unwrap_or_else(|e| panic!("post_regime_tag failed for {regime}: {e}"));

        // Confirm the regime string persists verbatim in the JSON.
        let row: (String,) = sqlx::query_as(
            "SELECT error_summary FROM strategy_events \
             WHERE kind = 'RegimeTag' \
             ORDER BY rowid DESC \
             LIMIT 1",
        )
        .fetch_one(ledger.pool())
        .await
        .expect("row must exist");

        let summary: serde_json::Value = serde_json::from_str(&row.0).expect("valid JSON");
        assert_eq!(summary["regime"], regime, "regime round-trip for {regime}");
        assert_eq!(summary["symbol"], symbol, "symbol round-trip for {symbol}");
        assert_eq!(
            summary["max_confidence"], confidence,
            "confidence round-trip"
        );
    }
}

/// T-D-D1 / T-D-D2 audit-tick gate — `AuditEvent::RegimeTag` fires post-commit.
///
/// `post_regime_tag` delegates to `strategy_event` (which fires a
/// `StrategyEvent` tick) and then fires its own `RegimeTag` tick.
/// This test drains ticks until a `RegimeTag` arrives (tolerates the
/// `StrategyEvent` prefix tick without failing).
#[tokio::test]
async fn regime_tag_emits_audit_tick() {
    let (ledger, mut stream) = open_ledger_with_bus().await;

    post_regime_tag(
        &ledger,
        "BTCUSDT",
        "bear",
        "0.780000",
        Some("2026-05-28T12:01:00.000000Z"),
    )
    .await
    .expect("post_regime_tag must succeed");

    // Drain ticks until we see a RegimeTag (the StrategyEvent prefix tick
    // is emitted first by the inner `strategy_event()` call).
    let regime_tag_tick = loop {
        let tick = tokio::time::timeout(std::time::Duration::from_secs(1), stream.next())
            .await
            .expect("tick must arrive within 1s")
            .expect("tick stream not closed");

        if matches!(tick.event, AuditEvent::RegimeTag { .. }) {
            break tick;
        }
        // Tolerate the StrategyEvent prefix — keep draining.
    };

    match regime_tag_tick.event {
        AuditEvent::RegimeTag {
            symbol,
            regime,
            max_confidence,
        } => {
            assert_eq!(symbol.as_str(), "BTCUSDT", "symbol in tick");
            assert_eq!(regime.as_str(), "bear", "regime in tick");
            assert_eq!(
                max_confidence.as_str(),
                "0.780000",
                "max_confidence in tick"
            );
        }
        other => panic!("expected AuditEvent::RegimeTag, got {:?}", other),
    }
}

/// T-D-D1 — serde byte-identity: JSON serialise → deserialise → fields match.
#[tokio::test]
async fn regime_tag_audit_event_serde_byte_identity() {
    let event = AuditEvent::RegimeTag {
        symbol: smol_str::SmolStr::new("BTCUSDT"),
        regime: smol_str::SmolStr::new("volatile"),
        max_confidence: smol_str::SmolStr::new("0.720000"),
    };

    // Serialise to JSON.
    let json = serde_json::to_string(&event).expect("AuditEvent must serialise");

    // Deserialise back.
    let round_tripped: AuditEvent =
        serde_json::from_str(&json).expect("AuditEvent must deserialise");

    match round_tripped {
        AuditEvent::RegimeTag {
            symbol,
            regime,
            max_confidence,
        } => {
            assert_eq!(symbol.as_str(), "BTCUSDT");
            assert_eq!(regime.as_str(), "volatile");
            assert_eq!(max_confidence.as_str(), "0.720000");
        }
        other => panic!("expected RegimeTag after round-trip, got {:?}", other),
    }

    // Verify JSON is deterministic (same string on second serialise).
    let json2 = serde_json::to_string(&AuditEvent::RegimeTag {
        symbol: smol_str::SmolStr::new("BTCUSDT"),
        regime: smol_str::SmolStr::new("volatile"),
        max_confidence: smol_str::SmolStr::new("0.720000"),
    })
    .expect("second serialise");
    assert_eq!(json, json2, "JSON serialisation must be byte-identical");
}

/// T-D-D1 — additive variant contract: existing `strategy_events` rows
/// produced by a fresh ledger (no RegimeTag calls) do NOT contain a RegimeTag row.
#[tokio::test]
async fn empty_ledger_has_no_regime_tag_rows() {
    let ledger = open_ledger().await;

    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM strategy_events WHERE kind = 'RegimeTag'")
            .fetch_one(ledger.pool())
            .await
            .expect("count query must succeed");

    assert_eq!(count.0, 0, "fresh ledger must have zero RegimeTag rows");
}
