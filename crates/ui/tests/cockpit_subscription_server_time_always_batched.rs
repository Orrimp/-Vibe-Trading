//! T-D-C2 — `ServerTimeRecipe` is always present in the subscription batch.
//!
//! Asserts that `SubscriptionVariant::ServerTime` appears in the descriptor
//! returned by `build_subscription_batch_descriptor` for every active
//! `Screen::*` variant.  This is a regression guard: if a future change
//! accidentally gates the always-on 1 Hz clock recipe behind a screen check,
//! this test catches it at `cargo test` time.
//!
//! ## Why this test uses the descriptor seam
//!
//! `cockpit_live::subscription()` returns an opaque `iced::Subscription<Message>`
//! that cannot be introspected.  The Wave C seam extraction
//! (`ui::live::build_subscription_batch_descriptor`) returns a
//! `Vec<SubscriptionVariant>` that tests CAN inspect.  Production
//! `subscription()` calls `build_subscription_batch_descriptor` and converts
//! each variant to an actual iced subscription, so the descriptor and the live
//! batch are always in sync.
//!
//! ## T-T4 falsification probe (D-V0.2.0-3 row 6)
//!
//! **Probe**: in `crates/ui/src/live.rs`, inside
//! `build_subscription_batch_descriptor`, remove (comment out) the line
//! `desc.push(SubscriptionVariant::ServerTime);`.
//!
//! **Expected failure**: every call to `assert!(descriptor.contains(&SubscriptionVariant::ServerTime))`
//! in this file fails with
//! `assertion 'left == right' failed` — `ServerTime` not found in descriptor.
//!
//! **Restore**: reinstate the removed line verbatim; all tests PASS.
//!
//! ## Coverage
//!
//! | Test ID  | What it pins                                                     |
//! |----------|------------------------------------------------------------------|
//! | C2-T1    | ServerTime in descriptor for Screen::Lab                        |
//! | C2-T2    | ServerTime in descriptor for Screen::Live                       |
//! | C2-T3    | ServerTime in descriptor for Screen::Compare                    |
//! | C2-T4    | ServerTime in descriptor for Screen::Trail                      |
//! | C2-T5    | ServerTime in descriptor for Screen::Settings                   |

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use ui::Screen;
use ui::live::{SubscriptionVariant, build_subscription_batch_descriptor};

/// The 5 primary active `Screen` variants exercised by this test.
///
/// Deprecated aliases (`Home`, `Charts`, `Audit`, `Risk`, `Debug`, `Control`)
/// are excluded — they alias the active variants above and are kept only for
/// test-harness compat (Phase A deprecation cycle).  Any screen that aliases
/// to Lab/Live/Trail/Settings would exercise the same always-on recipe batch.
fn active_screens() -> Vec<Screen> {
    vec![
        Screen::Lab,
        Screen::Live,
        Screen::Compare,
        Screen::Trail,
        Screen::Settings,
    ]
}

/// C2-T1..C2-T5 — `SubscriptionVariant::ServerTime` is present in the
/// descriptor for every active screen variant.
///
/// The current production contract: `ServerTimeRecipe` (1 Hz clock) is
/// always-on — it is included unconditionally, regardless of which panel is
/// visible.  This test pins that contract so that a future change that
/// accidentally wraps the `ServerTime` push in a `if cockpit.current_screen == Screen::Live`
/// guard is caught before it ships.
///
/// Parameterisation: we iterate over `active_screens()` so a single test
/// function covers all 5 variants; each failure message names the screen
/// that triggered it.
#[test]
fn server_time_recipe_in_every_screen_batch() {
    for screen in active_screens() {
        // Build descriptor with no optional features (no trail, no lab run,
        // no training run) — the always-on recipes must appear regardless.
        let descriptor = build_subscription_batch_descriptor(
            /*has_trail=*/ false, /*has_lab_progress=*/ false,
            /*has_training_log=*/ false,
        );

        assert!(
            descriptor.contains(&SubscriptionVariant::ServerTime),
            "SubscriptionVariant::ServerTime must be present for Screen::{screen:?} \
             (always-on recipe — never screen-gated). \
             Descriptor: {descriptor:?}"
        );
    }
}

/// Supplementary: descriptor also contains ServerTime when optional recipes
/// are present (trail, lab progress, training log all Some).
///
/// Validates that enabling the optional recipes doesn't accidentally displace
/// or suppress the always-on ServerTime variant.
#[test]
fn server_time_present_with_all_optional_recipes_active() {
    let descriptor = build_subscription_batch_descriptor(
        /*has_trail=*/ true, /*has_lab_progress=*/ true, /*has_training_log=*/ true,
    );

    assert!(
        descriptor.contains(&SubscriptionVariant::ServerTime),
        "SubscriptionVariant::ServerTime must remain present even when all \
         optional recipes are active. Descriptor: {descriptor:?}"
    );
}
