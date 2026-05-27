//! cockpit-toast-queue v0.1.0 — integration tests (ADR-0046 § Tests / T-D-N9).
//!
//! These tests exercise the `state::update` path for the toast queue.
//! No iced application is constructed — tests operate on `Cockpit` directly
//! and dispatch `Message` variants through the pure `state::update` function.
//!
//! ## Test inventory (4 tests, all in T-D-N9)
//!
//! | Test name                                     | ADR-0046 gate |
//! |-----------------------------------------------|---------------|
//! | `queue_displays_multiple`                     | R2 / R4.1     |
//! | `auto_dismiss_after_timeout`                  | R2.5 / K5     |
//! | `two_completions_in_rapid_succession_both_visible` | R4.1 / K5 |
//! | `overflow_drops_oldest_keeps_newest`          | R1.3          |
//!
//! ## Clock-injection seam
//!
//! `auto_dismiss_after_timeout` constructs a synthetic `Instant` by adding
//! `TOAST_AUTODISMISS + 1 ms` to the entry's `created_at` and passes it as
//! the `Message::ToastTick(...)` payload. No fake-clock infrastructure needed —
//! the ADR-0046 clock-injection via message payload is the designed test seam.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::Duration;

use smol_str::SmolStr;
use ui::state::{Cockpit, MAX_TOAST_QUEUE_LEN, Message, TOAST_AUTODISMISS, ToastSeverity, update};

// ── Test 1: queue_displays_multiple ──────────────────────────────────────────

/// R2 / R4.1 acceptance: dispatch 3 `ShowToast` messages; assert queue len == 3
/// and ordering matches dispatch (FIFO — first dispatched is FRONT).
#[test]
fn queue_displays_multiple() {
    let mut c = Cockpit::new();

    update(&mut c, Message::ShowToast(SmolStr::new("first")));
    update(&mut c, Message::ShowToast(SmolStr::new("second")));
    update(&mut c, Message::ShowToast(SmolStr::new("third")));

    assert_eq!(
        c.toast_queue.len(),
        3,
        "queue must hold all 3 enqueued entries"
    );
    // FIFO: first dispatched = front = oldest.
    assert_eq!(
        c.toast_queue[0].message,
        SmolStr::new("first"),
        "front entry must be the first dispatched"
    );
    assert_eq!(
        c.toast_queue[1].message,
        SmolStr::new("second"),
        "middle entry must be the second dispatched"
    );
    assert_eq!(
        c.toast_queue[2].message,
        SmolStr::new("third"),
        "back entry must be the third dispatched"
    );
}

// ── Test 2: auto_dismiss_after_timeout ───────────────────────────────────────

/// R2.5 / K5 clock-injection acceptance: enqueue 1 toast, then dispatch a
/// `ToastTick` with a synthetic `Instant` that is `TOAST_AUTODISMISS + 1 ms`
/// AFTER the entry's `created_at`. Assert queue is empty.
///
/// No fake-clock field needed: the `Message::ToastTick(Instant)` payload IS
/// the clock-injection seam per ADR-0046.
#[test]
fn auto_dismiss_after_timeout() {
    let mut c = Cockpit::new();

    update(&mut c, Message::ShowToast(SmolStr::new("ephemeral")));
    assert_eq!(
        c.toast_queue.len(),
        1,
        "queue must have 1 entry before tick"
    );

    // Grab created_at of the entry, then advance past TOAST_AUTODISMISS.
    let created = c.toast_queue.front().expect("entry exists").created_at;
    let synthetic_now = created + TOAST_AUTODISMISS + Duration::from_millis(1);

    update(&mut c, Message::ToastTick(synthetic_now));

    assert!(
        c.toast_queue.is_empty(),
        "queue must be empty after ToastTick past TOAST_AUTODISMISS"
    );
}

// ── Test 3: two_completions_in_rapid_succession_both_visible ─────────────────

/// R4.1 / K5 stronger contract: two `ShowToastWithSeverity` dispatches in
/// rapid succession (same logical instant) — both entries must be queue-
/// resident. The back-compat `toast_message()` shim returns the FRONT (oldest)
/// entry. This is the "graduated K5" test that the old single-slot REPLACE
/// semantic would have FAILED.
#[test]
fn two_completions_in_rapid_succession_both_visible() {
    let mut c = Cockpit::new();

    update(
        &mut c,
        Message::ShowToastWithSeverity(SmolStr::new("run_completed"), ToastSeverity::Success),
    );
    update(
        &mut c,
        Message::ShowToastWithSeverity(SmolStr::new("training_completed"), ToastSeverity::Success),
    );

    assert_eq!(
        c.toast_queue.len(),
        2,
        "both completion toasts must be queue-resident (NO silent REPLACE)"
    );

    // Both entries are individually reachable by id.
    let front_id = c.toast_queue.front().expect("front exists").id;
    let back_id = c.toast_queue.back().expect("back exists").id;
    assert_ne!(front_id, back_id, "entries must have distinct ids");

    // back-compat shim returns FRONT (first enqueued = oldest visible).
    assert_eq!(
        c.toast_message().map(SmolStr::as_str),
        Some("run_completed"),
        "toast_message() shim must return the FRONT (first) entry"
    );
}

// ── Test 4: overflow_drops_oldest_keeps_newest ───────────────────────────────

/// R1.3 / ADR-0046 overflow policy: enqueue 6 messages with cap 5.
/// The oldest (id=0, "msg1") is dropped; the 5 newest are retained;
/// `front().id` corresponds to the 2nd enqueued entry.
#[test]
fn overflow_drops_oldest_keeps_newest() {
    let mut c = Cockpit::new();

    // Enqueue MAX_TOAST_QUEUE_LEN + 1 entries.
    for i in 1..=(MAX_TOAST_QUEUE_LEN + 1) as u32 {
        update(&mut c, Message::ShowToast(SmolStr::new(format!("msg{i}"))));
    }

    assert_eq!(
        c.toast_queue.len(),
        MAX_TOAST_QUEUE_LEN,
        "queue must be capped at MAX_TOAST_QUEUE_LEN after overflow"
    );

    // The oldest entry ("msg1", id=0) must have been dropped.
    assert!(
        c.toast_queue
            .iter()
            .all(|t| t.message != SmolStr::new("msg1")),
        "oldest entry (msg1) must have been dropped"
    );

    // Front is "msg2" (second enqueued = oldest retained).
    assert_eq!(
        c.toast_queue.front().map(|t| t.message.as_str()),
        Some("msg2"),
        "front must be msg2 (second enqueued = oldest retained)"
    );

    // Back is "msg6" (last enqueued = newest).
    assert_eq!(
        c.toast_queue.back().map(|t| t.message.as_str()),
        Some(format!("msg{}", MAX_TOAST_QUEUE_LEN + 1).as_str()),
        "back must be the last enqueued (newest)"
    );
}
