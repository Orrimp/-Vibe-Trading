//! T-D-D1 — `TrailMirrorRecipe` is present in the subscription batch **only
//! when** the trail-mirror handle is `Some`; it is absent when the handle is
//! `None`.
//!
//! This is the Surface 2 gating test for `TrailMirrorRecipe` (Wave D,
//! D-V0.2.0-3 row 9).  Surface 1 (`trail_mirror_recipe_stream.rs`) already
//! covers the stream boundary; this test covers the subscription-batch
//! presence / absence gate.
//!
//! ## Why this test uses the descriptor seam
//!
//! `cockpit_live::subscription()` returns an opaque `iced::Subscription<Message>`
//! that cannot be introspected.  The Wave C seam extraction
//! (`ui::live::build_subscription_batch_descriptor`) returns a
//! `Vec<SubscriptionVariant>` that tests CAN inspect.  Production
//! `subscription()` calls `build_subscription_batch_descriptor(has_trail, ...)`
//! where `has_trail = self.trail_mirror_handle.is_some()`, so the descriptor
//! and the live subscription are always in sync.
//!
//! ## T-T4 falsification probe (D-V0.2.0-3 row 9)
//!
//! **Probe**: in `crates/ui/src/bin/cockpit_live.rs`, inside the
//! `subscription()` `match variant` loop, replace the `Trail =>` arm body:
//!
//! ```text
//! // Original:
//! self.trail_mirror_handle
//!     .as_ref()
//!     .map(|h| ui::live::trail_mirror_subscription(h.clone()))
//!     .unwrap_or_else(iced::Subscription::none)
//! // Probe: replace the entire arm body with:
//! iced::Subscription::none()
//! ```
//!
//! AND in `crates/ui/src/live.rs`, inside `build_subscription_batch_descriptor`,
//! remove the conditional push:
//!
//! ```text
//! // Original:
//! if has_trail {
//!     desc.push(SubscriptionVariant::Trail);
//! }
//! // Probe: delete the above 3 lines.
//! ```
//!
//! **Expected failure** (after the descriptor probe):
//! `trail_mirror_batched_when_handle_present` fails with:
//! `"descriptor lacks Trail variant when handle is Some"`.
//!
//! **Restore**: reinstate the removed lines verbatim; both tests PASS.
//!
//! ## Coverage
//!
//! | Test ID | What it pins                                                         |
//! |---------|----------------------------------------------------------------------|
//! | D1-T1   | `Trail` variant present in descriptor when `has_trail = true`        |
//! | D1-T2   | `Trail` variant absent in descriptor when `has_trail = false`        |

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use ui::live::{SubscriptionVariant, build_subscription_batch_descriptor};

// ── Test D1-T1 ───────────────────────────────────────────────────────────────

/// D1-T1 — descriptor includes `TrailMirrorRecipe` when handle is present.
///
/// When the trail-mirror handle is `Some` (i.e. `has_trail = true`), the
/// `SubscriptionVariant::Trail` variant MUST appear in the descriptor.
///
/// Production wiring: `cockpit_live::subscription()` passes
/// `self.trail_mirror_handle.is_some()` as `has_trail` to
/// `build_subscription_batch_descriptor`.  This test pins that path.
///
/// ## T-T4 falsification
///
/// Remove the `if has_trail { desc.push(SubscriptionVariant::Trail); }` block
/// from `build_subscription_batch_descriptor` → this test fails with
/// `"descriptor lacks Trail variant when handle is Some"`.
#[test]
fn trail_mirror_batched_when_handle_present() {
    // has_trail = true simulates AppState::trail_mirror_handle = Some(_).
    let descriptor = build_subscription_batch_descriptor(
        /*has_trail=*/ true, /*has_lab_progress=*/ false,
        /*has_training_log=*/ false,
    );

    assert!(
        descriptor.contains(&SubscriptionVariant::Trail),
        "SubscriptionVariant::Trail must be present in the descriptor when \
         has_trail = true (trail_mirror_handle is Some). \
         Descriptor: {descriptor:?}"
    );
}

// ── Test D1-T2 ───────────────────────────────────────────────────────────────

/// D1-T2 — descriptor omits `TrailMirrorRecipe` when handle is absent.
///
/// When the trail-mirror handle is `None` (i.e. `has_trail = false`), the
/// `SubscriptionVariant::Trail` variant MUST NOT appear in the descriptor.
///
/// This pins the absence contract: a future change that accidentally includes
/// `Trail` unconditionally would be caught here.
#[test]
fn trail_mirror_omitted_when_handle_absent() {
    // has_trail = false simulates AppState::trail_mirror_handle = None.
    let descriptor = build_subscription_batch_descriptor(
        /*has_trail=*/ false, /*has_lab_progress=*/ false,
        /*has_training_log=*/ false,
    );

    assert!(
        !descriptor.contains(&SubscriptionVariant::Trail),
        "SubscriptionVariant::Trail must NOT be present in the descriptor when \
         has_trail = false (trail_mirror_handle is None). \
         Descriptor: {descriptor:?}"
    );
}
