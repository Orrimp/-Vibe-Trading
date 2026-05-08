//! T1711 — Audit screen row click opens the journal-transaction modal
//! (Phase 3 R11 / R14.1).
//!
//! The audit-screen rows in `crates/ui/src/screens/audit.rs` emit
//! `Message::TapeRowClicked(row.tx_id.clone())` — the literal Phase 1
//! variant per R11.4 / Q11. The modal-state transition is handled by
//! `ui::state::update`'s existing `TapeRowClicked` arm (set
//! `tape_audit_modal = Some(JournalModalState::Loading { tx_id })`).
//! This test asserts the same arm fires when the row click originates
//! on the Audit screen — the audit row is just another caller of
//! the same `Message` variant.

#![cfg(feature = "fixtures")]
// Test helpers construct a `Cockpit::default()` and override individual
// fields for clarity over a giant `Cockpit { ..Default::default() }`
// initializer; the lint is over-eager for this test shape.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::field_reassign_with_default
)]

use ui::state::{update, Cockpit, JournalModalState, Message, PanelState, Screen};

#[test]
fn audit_row_click_flips_modal_to_loading() {
    let mut cockpit = Cockpit::default();
    cockpit.current_screen = Screen::Audit;
    let rows = ui::fixtures::fake_journal_rows(3);
    let target_tx_id = rows[0].tx_id.clone();
    let total = u64::try_from(rows.len()).unwrap_or(0);
    cockpit.audit_screen_state.rows = PanelState::Ready(rows);
    cockpit.audit_screen_state.total_count = Some(total);

    assert!(cockpit.tape_audit_modal.is_none());

    update(&mut cockpit, Message::TapeRowClicked(target_tx_id.clone()));

    let modal: &JournalModalState = cockpit
        .tape_audit_modal
        .as_ref()
        .expect("modal must open after audit row click");
    assert_eq!(
        modal.tx_id, target_tx_id,
        "modal carries the clicked row's tx_id verbatim"
    );
    assert!(
        matches!(modal.entries, PanelState::Loading),
        "modal body starts in Loading until the binary's Task::perform returns"
    );
}

#[test]
fn audit_row_click_does_not_affect_audit_screen_state() {
    // Clicking an audit row opens the modal but does NOT mutate the
    // audit panel's filter, page, or rows — the modal overlays the
    // screen, the screen state is preserved underneath.
    let mut cockpit = Cockpit::default();
    cockpit.current_screen = Screen::Audit;
    let rows = ui::fixtures::fake_journal_rows(5);
    let target_tx_id = rows[0].tx_id.clone();
    let initial_total = u64::try_from(rows.len()).unwrap_or(0);
    cockpit.audit_screen_state.rows = PanelState::Ready(rows.clone());
    cockpit.audit_screen_state.total_count = Some(initial_total);
    cockpit.audit_screen_state.page = 0;

    update(&mut cockpit, Message::TapeRowClicked(target_tx_id));

    assert_eq!(cockpit.audit_screen_state.page, 0);
    assert_eq!(cockpit.audit_screen_state.total_count, Some(initial_total));
    assert!(matches!(
        cockpit.audit_screen_state.rows,
        PanelState::Ready(_)
    ));
}
