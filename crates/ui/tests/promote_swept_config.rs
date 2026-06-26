//! advisor-param-promotion (ADR-0070 § D4) — `PromoteSweptConfig` pure-state
//! behaviour relay (the LOAD-BEARING wiring proof).
//!
//! ## Why this file is the wiring proof (not the render PNG)
//!
//! Per ADR-0070 § T12 + the CLAUDE.md v3-vol-overlay-noop precedent: the
//! render-pixel proof shows the "Use this config" affordance paints as an enabled
//! accent button — but a button that paints accent yet drops its message on the
//! floor would STILL pass that pixel guard (the old visual-only pill looked
//! identical). The proof that the click actually CARRIES the tuned config forward
//! is THIS pure-state test: driving `ui::state::update` with
//! `Message::PromoteSweptConfig(params)` and asserting the model preseeds the
//! forward-launch target + navigates. RED before the message/handler existed
//! (the affordance was a visual `Container` pill with no `on_press`); GREEN after.
//! Reverting the handler — or weakening it to a no-op — breaks these.
//!
//! The binary-layer `ForwardCommand::Launch` dispatch off
//! `pending_forward_promotion` is exercised at integration; here we pin the PURE
//! half (the preseed + the nav), which is the contract the binary reads.

#![cfg(feature = "fixtures")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use ui::state::{Cockpit, Message, PanelState, Screen, update};
use ui::tune::PromoteParams;

/// Core proof: `PromoteSweptConfig(SMA)` sets the forward-launch target (which is
/// also the provenance signal) + navigates to the forward plan. This is the exact
/// path the now-wired "Use this config" button drives — a no-op before.
#[test]
fn promote_swept_config_preseeds_target_and_navigates() {
    let mut cockpit = Cockpit::default();
    // The button lives on the Tune screen with a Ready result grid.
    cockpit.current_screen = Screen::Tune;
    cockpit.tune_screen_state.result = PanelState::Ready(ui::fixtures::fake_sweep_report_mirror());

    let params = PromoteParams::Sma {
        fast_len: 10,
        slow_len: 20,
    };
    update(&mut cockpit, Message::PromoteSweptConfig(params.clone()));

    // (a) Navigated to the forward plan (the operator reviews, then their €200
    // begins).
    assert_eq!(
        cockpit.current_screen,
        Screen::ForwardPlan,
        "PromoteSweptConfig must navigate to the forward-plan screen"
    );
    // (b)+(c) The forward-launch target is set — the binary launches from it AND
    // the forward-plan view renders the "you tuned this" provenance header from
    // its Some-ness (one field is the launch target AND the provenance signal).
    let promotion = cockpit
        .pending_forward_promotion
        .as_ref()
        .expect("PromoteSweptConfig must set pending_forward_promotion");
    // The structured tuned params carry through verbatim (the self-contained
    // message — not a mirror re-lookup).
    assert_eq!(
        promotion.params, params,
        "the preseeded promotion must carry the exact tuned params"
    );
    // SMA → the StrategyId the forward resolvers dispatch on.
    assert_eq!(
        promotion.strategy_id.0.as_str(),
        "v0.5.sma",
        "SMA promotion must resolve to the v0.5.sma forward id"
    );
    // The coin is the Tune sweep's authoritative coin.
    assert_eq!(
        promotion.coin.0.as_str(),
        cockpit.tune_coin.0.as_str(),
        "the promotion carries the Tune coin"
    );
    // (d) The plan flips to Loading so the operator never sees a stale crowned
    // plan while the agent resolves the tuned one.
    assert!(
        matches!(cockpit.forward_plan_screen_state.plan, PanelState::Loading),
        "the forward plan must flip to Loading on promote (no stale crowned plan)"
    );
}

/// The family → forward `StrategyId` map is exhaustive and correct for every
/// family (the SAME ids `build_registry_for` matches). A wrong id would launch the
/// wrong (or no) strategy.
#[test]
fn promote_swept_config_maps_every_family_to_its_forward_id() {
    let cases = [
        (
            PromoteParams::Sma {
                fast_len: 7,
                slow_len: 14,
            },
            "v0.5.sma",
        ),
        (
            PromoteParams::Macd {
                fast: 8,
                slow: 20,
                signal: 5,
            },
            "v0.5.macd",
        ),
        (
            PromoteParams::Rsi {
                period: 10,
                oversold: 25,
            },
            "v0.5.rsi",
        ),
        (
            PromoteParams::Bollinger {
                period: 20,
                k_tenths: 25,
            },
            "v0.5.bbands",
        ),
    ];

    for (params, expected_id) in cases {
        let mut cockpit = Cockpit::default();
        cockpit.current_screen = Screen::Tune;
        cockpit.tune_screen_state.result =
            PanelState::Ready(ui::fixtures::fake_sweep_report_mirror());

        update(&mut cockpit, Message::PromoteSweptConfig(params.clone()));

        let promotion = cockpit
            .pending_forward_promotion
            .expect("every family must preseed a promotion");
        assert_eq!(
            promotion.strategy_id.0.as_str(),
            expected_id,
            "{params:?} must map to the {expected_id} forward id"
        );
        assert_eq!(
            promotion.params, params,
            "params must carry through verbatim"
        );
    }
}

/// The window label is carried from the Ready sweep mirror (the "robust on THIS
/// window" honesty copy reads it). Pins that the promotion names the window the
/// gate actually scored, not a default.
#[test]
fn promote_swept_config_carries_the_swept_window_label() {
    let mut cockpit = Cockpit::default();
    cockpit.current_screen = Screen::Tune;
    // The fixture mirror's range_label is "2024 H1".
    cockpit.tune_screen_state.result = PanelState::Ready(ui::fixtures::fake_sweep_report_mirror());

    update(
        &mut cockpit,
        Message::PromoteSweptConfig(PromoteParams::Sma {
            fast_len: 10,
            slow_len: 20,
        }),
    );

    let promotion = cockpit.pending_forward_promotion.expect("preseeded");
    assert_eq!(
        promotion.window_label.as_str(),
        "2024 H1",
        "the promotion must carry the window the sweep scored (for the honesty copy)"
    );
}
