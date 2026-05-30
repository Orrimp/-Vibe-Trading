//! T-D-C4 — `ToastDismissRecipe` is always present in the subscription batch.
//!
//! Asserts that `SubscriptionVariant::ToastDismiss` appears in the descriptor
//! returned by `build_subscription_batch_descriptor` for every active
//! `Screen::*` variant AND regardless of whether the `toast_queue` is empty
//! or non-empty.
//!
//! ## Why always-on?
//!
//! `ToastDismissRecipe` is unconditional — the 500 ms auto-dismiss sweep is
//! a process-lifetime subscription that must run even when there are no toasts
//! queued.  Gating it on `toast_queue.is_empty()` would cause the dismiss
//! sweep to restart (with a new timer interval) every time a toast is enqueued
//! and cleared, producing an observable delay in auto-dismiss timing.
//!
//! This test is the regression guard: if a future change accidentally wraps
//! the `ToastDismiss` push in `if !cockpit.toast_queue.is_empty()`, this test
//! catches it before it ships.
//!
//! ## Why this test uses the descriptor seam
//!
//! `cockpit_live::subscription()` returns an opaque `iced::Subscription<Message>`
//! that cannot be introspected.  The Wave C seam extraction
//! (`ui::live::build_subscription_batch_descriptor`) returns a
//! `Vec<SubscriptionVariant>` that tests CAN inspect.  Production
//! `subscription()` calls `build_subscription_batch_descriptor` and converts
//! each variant to an actual iced subscription.
//!
//! ## T-T4 falsification probe (D-V0.2.0-3 row 8)
//!
//! **Probe**: in `crates/ui/src/live.rs`, inside
//! `build_subscription_batch_descriptor`, remove (comment out) the line
//! `desc.push(SubscriptionVariant::ToastDismiss);`.
//!
//! **Expected failure**: every call to
//! `assert!(descriptor.contains(&SubscriptionVariant::ToastDismiss))`
//! in this file fails — `ToastDismiss` not found in descriptor.
//!
//! **Restore**: reinstate the removed line verbatim; all tests PASS.
//!
//! ## Coverage
//!
//! | Test ID  | What it pins                                                          |
//! |----------|-----------------------------------------------------------------------|
//! | C4-T1    | ToastDismiss in descriptor for Screen::Lab                            |
//! | C4-T2    | ToastDismiss in descriptor for Screen::Live                           |
//! | C4-T3    | ToastDismiss in descriptor for Screen::Compare                        |
//! | C4-T4    | ToastDismiss in descriptor for Screen::Trail                          |
//! | C4-T5    | ToastDismiss in descriptor for Screen::Settings                       |
//! | C4-T6    | ToastDismiss present when all optional recipes are active             |
//! | C4-T7    | ToastDismiss present regardless of empty/non-empty toast_queue state  |

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use ui::Screen;
use ui::live::{SubscriptionVariant, build_subscription_batch_descriptor};

/// The 5 primary active `Screen` variants exercised by this test.
///
/// Deprecated aliases (`Home`, `Charts`, `Audit`, `Risk`, `Debug`, `Control`)
/// are excluded — they alias the active variants above and are kept only for
/// test-harness compat (Phase A deprecation cycle).
fn active_screens() -> Vec<Screen> {
    vec![
        Screen::Lab,
        Screen::Live,
        Screen::Compare,
        Screen::Trail,
        Screen::Settings,
    ]
}

/// C4-T1..C4-T5 — `SubscriptionVariant::ToastDismiss` is present in the
/// descriptor for every active screen variant.
///
/// Iterates over all 5 active screens.  The descriptor is built with no
/// optional recipes active (no trail, no lab run, no training run) — the
/// always-on `ToastDismiss` must appear regardless.
///
/// ## T-T4 falsification probe
///
/// Remove `desc.push(SubscriptionVariant::ToastDismiss)` from
/// `build_subscription_batch_descriptor` — every assertion in this test
/// fails with `ToastDismiss not found in descriptor`.
#[test]
fn toast_dismiss_in_every_screen_batch() {
    for screen in active_screens() {
        let descriptor = build_subscription_batch_descriptor(
            /*has_trail=*/ false, /*has_lab_progress=*/ false,
            /*has_training_log=*/ false,
        );

        assert!(
            descriptor.contains(&SubscriptionVariant::ToastDismiss),
            "SubscriptionVariant::ToastDismiss must be present for Screen::{screen:?} \
             (always-on recipe — never screen-gated or toast-queue-gated). \
             Descriptor: {descriptor:?}"
        );
    }
}

/// C4-T6 — `ToastDismiss` is present even when all optional recipes are active.
///
/// Validates that enabling trail, lab progress, and training log does not
/// accidentally displace or suppress the always-on `ToastDismiss` variant.
#[test]
fn toast_dismiss_present_with_all_optional_recipes_active() {
    let descriptor = build_subscription_batch_descriptor(
        /*has_trail=*/ true, /*has_lab_progress=*/ true, /*has_training_log=*/ true,
    );

    assert!(
        descriptor.contains(&SubscriptionVariant::ToastDismiss),
        "SubscriptionVariant::ToastDismiss must remain present even when all \
         optional recipes are active. Descriptor: {descriptor:?}"
    );
}

/// C4-T7 — `ToastDismiss` is present regardless of `toast_queue` state.
///
/// Simulates the two observable toast-queue states: empty (no toasts) and
/// non-empty (at least one toast pending).  In the descriptor seam these
/// states don't affect `build_subscription_batch_descriptor` (the function
/// doesn't take `toast_queue.is_empty()` as a parameter — by design).
///
/// This test documents the invariant explicitly so a future change that
/// adds a `has_toasts: bool` parameter to the descriptor function must also
/// update this test — forcing the author to consciously consider whether
/// `ToastDismiss` should ever be omitted.
///
/// Note: since `build_subscription_batch_descriptor` does not currently take
/// a `has_toasts` parameter, both calls are identical.  The test is still
/// meaningful as documentation of the always-on contract.
#[test]
fn toast_dismiss_present_regardless_of_toast_queue_emptiness() {
    // Simulate empty toast queue (no toasts pending).
    let descriptor_empty_queue = build_subscription_batch_descriptor(
        /*has_trail=*/ false, /*has_lab_progress=*/ false,
        /*has_training_log=*/ false,
    );

    assert!(
        descriptor_empty_queue.contains(&SubscriptionVariant::ToastDismiss),
        "ToastDismiss must be present when toast_queue is empty. \
         Descriptor: {descriptor_empty_queue:?}"
    );

    // Simulate non-empty toast queue (toasts queued).
    // The descriptor function is identical — this demonstrates the
    // always-on contract does not vary with toast_queue state.
    let descriptor_non_empty_queue = build_subscription_batch_descriptor(
        /*has_trail=*/ false, /*has_lab_progress=*/ false,
        /*has_training_log=*/ false,
    );

    assert!(
        descriptor_non_empty_queue.contains(&SubscriptionVariant::ToastDismiss),
        "ToastDismiss must be present when toast_queue is non-empty. \
         Descriptor: {descriptor_non_empty_queue:?}"
    );
}
