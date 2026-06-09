//! Headless Emulator smoke — `ui-headless-emulator` v0.1 (T01).
//!
//! Boots the cockpit through `iced_test::emulator::Emulator`, drains
//! events until `Ready` (or a 10-event deadline), takes a screenshot
//! at the floor viewport, and asserts dimensions. Proves the
//! Emulator boots the FULL iced subscription pump headlessly — see
//! [`spec/ui-headless-emulator/feature.md`](../../spec/ui-headless-emulator/feature.md)
//! for what this unlocks vs. the existing
//! [`visual_snapshots.rs`](visual_snapshots.rs) free-function pattern.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use iced_test::emulator::{Emulator, Event, Mode};
use iced_test::futures::futures::StreamExt;
use iced_test::futures::futures::channel::mpsc;
use iced_test::futures::futures::executor;

use ui::state::Screen;
use ui::test_support::{charts_screen_cockpit, program_from_cockpit};

/// Bounded number of event-loop ticks before we give up waiting for
/// `Event::Ready`. The cockpit's boot is single-shot (no async data
/// fetch in fixtures mode), so a healthy boot resolves within 1-3
/// events. 10 is comfortable headroom.
const READY_DEADLINE_TICKS: usize = 10;

#[test]
fn headless_emulator_boots_cockpit_and_renders() {
    let cockpit = charts_screen_cockpit();
    let program = program_from_cockpit(cockpit);
    let theme = iced::Theme::Dark;

    let (tx, mut rx) = mpsc::channel(64);
    let mut emulator = Emulator::new(tx, &program, Mode::Zen, iced::Size::new(1280.0, 720.0));

    // Drain events until Ready or deadline. Per Emulator docs:
    // - Event::Action(action) → must call emulator.perform(&program, action)
    // - Event::Ready → boot complete; safe to render
    // - Event::Failed(_) → instruction failed (we don't dispatch any in v0.1)
    executor::block_on(async {
        for tick in 0..READY_DEADLINE_TICKS {
            match rx.next().await {
                Some(Event::Ready) => {
                    eprintln!("emulator ready after {tick} tick(s)");
                    break;
                }
                Some(Event::Action(action)) => {
                    emulator.perform(&program, action);
                }
                Some(Event::Failed(instruction)) => {
                    panic!("unexpected Event::Failed for instruction: {instruction:?}");
                }
                None => {
                    eprintln!("event channel closed at tick {tick} before Ready");
                    break;
                }
            }
        }
    });

    let screenshot = emulator.screenshot(&program, &theme, 1.0);
    assert_eq!(
        screenshot.size.width, 1280,
        "expected floor viewport width 1280, got {}",
        screenshot.size.width
    );
    assert_eq!(
        screenshot.size.height, 720,
        "expected floor viewport height 720, got {}",
        screenshot.size.height
    );
    assert!(
        !screenshot.rgba.is_empty(),
        "screenshot rgba buffer must be non-empty (boot + view loop ran)"
    );
}

/// cockpit-baseline-panel v0.1.0 (AC3) — the fixtures cockpit paints the
/// **Baseline** route headlessly without panic. Boots the same fixtures
/// cockpit, navigates to `Screen::Baseline`, boot-loads the curves via the
/// production `baseline::load_into` path (the curves resolve to `Ready` when
/// the runbook CSVs are present, or degrade to `Error` in a minimal checkout
/// — never a panic, R7), drains to `Ready`, and asserts a non-empty
/// first-frame screenshot. Complements the deterministic Error-state render
/// in `tests/baseline_error_state.rs`.
#[test]
fn headless_emulator_paints_baseline_route() {
    let mut cockpit = charts_screen_cockpit();
    cockpit.current_screen = Screen::Baseline;
    // Production boot path: load both realized BH curves (Ready or Error).
    ui::baseline::load_into(&mut cockpit);

    let program = program_from_cockpit(cockpit);
    let theme = iced::Theme::Dark;

    let (tx, mut rx) = mpsc::channel(64);
    let mut emulator = Emulator::new(tx, &program, Mode::Zen, iced::Size::new(1280.0, 720.0));

    executor::block_on(async {
        for tick in 0..READY_DEADLINE_TICKS {
            match rx.next().await {
                Some(Event::Ready) => {
                    eprintln!("baseline emulator ready after {tick} tick(s)");
                    break;
                }
                Some(Event::Action(action)) => {
                    emulator.perform(&program, action);
                }
                Some(Event::Failed(instruction)) => {
                    panic!("unexpected Event::Failed for instruction: {instruction:?}");
                }
                None => {
                    eprintln!("baseline event channel closed at tick {tick} before Ready");
                    break;
                }
            }
        }
    });

    let screenshot = emulator.screenshot(&program, &theme, 1.0);
    assert_eq!(screenshot.size.width, 1280);
    assert_eq!(screenshot.size.height, 720);
    assert!(
        !screenshot.rgba.is_empty(),
        "Baseline route first-frame screenshot must be non-empty (boot + view ran, no panic)"
    );
}

/// cockpit-live-dashboard-wiring v0.1.0 (AC3 / R7) — the fixtures cockpit
/// paints the **Live** route (the cockpit's default route under
/// `cockpit_live`) headlessly without panic. Fixtures mode has no `live`
/// feature, no agent, and no `pnl` feed, so the live equity buffer stays
/// empty and the equity curve + KPI strip render their **Loading** bodies.
/// The live-accumulation path must render the empty/Loading state without a
/// feed — this proves the two wired panels are smoke-safe on first frame.
#[test]
fn headless_emulator_paints_live_route() {
    let mut cockpit = charts_screen_cockpit();
    cockpit.current_screen = Screen::Live;
    // No feed is injected — fixtures mode has no live agent, so the live
    // equity buffer is empty and both wired panels stay Loading.
    assert!(
        cockpit.live_equity_buffer.is_empty(),
        "fixtures-mode Live route starts with an empty equity buffer (no feed)"
    );
    assert_eq!(cockpit.live_equity_curve.variant_name(), "loading");
    assert_eq!(cockpit.live_kpi.variant_name(), "loading");

    let program = program_from_cockpit(cockpit);
    let theme = iced::Theme::Dark;

    let (tx, mut rx) = mpsc::channel(64);
    let mut emulator = Emulator::new(tx, &program, Mode::Zen, iced::Size::new(1280.0, 720.0));

    executor::block_on(async {
        for tick in 0..READY_DEADLINE_TICKS {
            match rx.next().await {
                Some(Event::Ready) => {
                    eprintln!("live emulator ready after {tick} tick(s)");
                    break;
                }
                Some(Event::Action(action)) => {
                    emulator.perform(&program, action);
                }
                Some(Event::Failed(instruction)) => {
                    panic!("unexpected Event::Failed for instruction: {instruction:?}");
                }
                None => {
                    eprintln!("live event channel closed at tick {tick} before Ready");
                    break;
                }
            }
        }
    });

    let screenshot = emulator.screenshot(&program, &theme, 1.0);
    assert_eq!(screenshot.size.width, 1280);
    assert_eq!(screenshot.size.height, 720);
    assert!(
        !screenshot.rgba.is_empty(),
        "Live route first-frame screenshot must be non-empty (boot + view ran, Loading panels, no panic)"
    );
}
