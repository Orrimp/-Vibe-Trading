//! T1709 — Audit-screen filter chip click emits the expected
//! `Message::AuditFilterChanged(filter)` value (Phase 3 R9.2 / R10).
//!
//! The chip handlers in `crates/ui/src/screens/audit.rs` build a fresh
//! `AuditFilter` value via the `with_*` helpers and emit it via
//! `Message::AuditFilterChanged(...)`. This test exercises the
//! `update`-arm contract: feed a series of synthesized filter changes
//! and assert each lands in `cockpit.audit_screen_state.filter` with
//! the expected field values + a `page` reset to 0 + `rows = Loading`.

#![cfg(feature = "fixtures")]
// Test helpers construct a `Cockpit::default()` and override individual
// fields for clarity over a giant `Cockpit { ..Default::default() }`
// initializer; the lint is over-eager for this test shape.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::field_reassign_with_default
)]

use trading_core::{AuditKindFilter, Symbol, Venue};
use ui::state::{AuditFilter, AuditTimeRange, Cockpit, Message, PanelState, Screen, update};

#[test]
fn audit_filter_changed_resets_page_and_rows_to_loading() {
    let mut cockpit = Cockpit::default();
    cockpit.current_screen = Screen::Trail;
    cockpit.audit_screen_state.page = 5;
    cockpit.audit_screen_state.rows = PanelState::Ready(ui::fixtures::fake_journal_rows(5));

    let new_filter = AuditFilter::default().with_venues(vec![Venue::Coinbase]);
    update(
        &mut cockpit,
        Message::AuditFilterChanged(new_filter.clone()),
    );

    assert_eq!(
        cockpit.audit_screen_state.filter, new_filter,
        "filter must round-trip verbatim"
    );
    assert_eq!(
        cockpit.audit_screen_state.page, 0,
        "page must reset to 0 on filter change (R10)"
    );
    assert!(
        matches!(cockpit.audit_screen_state.rows, PanelState::Loading),
        "rows must flip to Loading until the binary's Task::perform re-fetches"
    );
}

#[test]
fn audit_filter_with_kind_chip_isolates_kind_field() {
    let mut cockpit = Cockpit::default();
    cockpit.current_screen = Screen::Trail;
    let initial = AuditFilter::default();
    let kind_changed = initial.with_kind(AuditKindFilter::Fill);
    update(&mut cockpit, Message::AuditFilterChanged(kind_changed));

    assert_eq!(
        cockpit.audit_screen_state.filter.kind,
        AuditKindFilter::Fill,
    );
    assert!(cockpit.audit_screen_state.filter.venues.is_empty());
    assert!(cockpit.audit_screen_state.filter.symbol.is_none());
}

#[test]
fn audit_filter_chip_chain_is_compositional() {
    // Three chip clicks in sequence — each `with_*` returns a fresh
    // filter; the cockpit's filter ends with all three changes applied.
    let mut cockpit = Cockpit::default();
    cockpit.current_screen = Screen::Trail;

    let f1 = AuditFilter::default().with_venues(vec![Venue::Binance, Venue::Coinbase]);
    update(&mut cockpit, Message::AuditFilterChanged(f1));
    let f2 = cockpit
        .audit_screen_state
        .filter
        .with_symbol(Some(Symbol::new("BTCUSDT")));
    update(&mut cockpit, Message::AuditFilterChanged(f2));
    let f3 = cockpit
        .audit_screen_state
        .filter
        .with_time_range(AuditTimeRange::Last1H);
    update(&mut cockpit, Message::AuditFilterChanged(f3));

    let f = &cockpit.audit_screen_state.filter;
    assert_eq!(f.venues, vec![Venue::Binance, Venue::Coinbase]);
    assert_eq!(f.symbol.as_ref().map(|s| s.0.as_str()), Some("BTCUSDT"));
    assert_eq!(f.time_range, AuditTimeRange::Last1H);
}
