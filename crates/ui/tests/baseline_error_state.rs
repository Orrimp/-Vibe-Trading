//! cockpit-baseline-panel v0.1.0 — Error-state headless render test (T8).
//!
//! The deterministic stand-in for "the fixtures-only `cockpit` smoke paints
//! the Baseline Error path" (D2). Rather than couple the global cockpit-smoke
//! gate to whether the runbook CSVs are present in the checkout (a flakiness
//! source), this test pins the exact behaviour a minimal checkout hits:
//!
//! 1. The loader, pointed at a **missing** path, lands the curve in
//!    `PanelState::Error(BASELINE_DATA_UNAVAILABLE)` — never panics (R7).
//! 2. `screens::baseline::view` (via `shell::screen_body`) renders that
//!    Error state in **both** themes without panic — the curve + band show
//!    their muted Error body while the KPI strip stays populated from the
//!    `const` (honest degrade), and the layout pass produces a non-zero root
//!    (AC2 / AC3).
//!
//! The render is driven through `Widget::layout` (the same harness
//! `layout_invariants.rs` uses) because that is the pass where a render
//! panic actually surfaces — constructing the `Element` alone does not run
//! the widget tree.

use iced::Element;
use iced::advanced::widget::Tree;

use ui::baseline;
use ui::state::{BaselineYear, Cockpit, Message, PanelState, Screen};
use ui::strings::BASELINE_DATA_UNAVAILABLE;
use ui::theme::ThemeMode;

/// Drive `Widget::layout` on a screen body and confirm the root node has a
/// positive area — proof the render pass ran end-to-end without panicking.
fn render_layout_ok(element: Element<'_, Message>) {
    let mut element = element;
    let mut tree = Tree::new(element.as_widget());
    let renderer = iced::Renderer::new(iced::Font::DEFAULT, iced::Pixels(16.0));
    let limits =
        iced::advanced::layout::Limits::new(iced::Size::ZERO, iced::Size::new(1440.0, 900.0));
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits);
    let size = node.size();
    assert!(
        size.width > 0.0 && size.height > 0.0,
        "Baseline screen body laid out to a zero-dim root ({} x {})",
        size.width,
        size.height
    );
}

/// The loader at a missing path yields `Error(BASELINE_DATA_UNAVAILABLE)`
/// for both years — never panics.
#[test]
fn loader_missing_path_yields_error_both_years() {
    for year in [BaselineYear::Y2023, BaselineYear::Y2024] {
        // A path guaranteed not to exist (sibling of the real artifacts dir).
        let bogus = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("docs/runbooks/artifacts/passive-baseline-2026-06-08")
            .join(format!("__missing__-{year:?}.csv"));
        match baseline::load_baseline_curve(&bogus) {
            PanelState::Error(msg) => assert_eq!(msg.as_str(), BASELINE_DATA_UNAVAILABLE),
            other => panic!("expected Error for {year:?}, got {}", other.variant_name()),
        }
    }
}

/// Construct the Baseline screen state with both curves in the Error state
/// (loader pointed at a missing path) and render `screen_body` in **both**
/// themes without panic. This is the headless equivalent of the smoke
/// painting the Error path (AC2 / AC3).
#[test]
fn baseline_error_state_renders_without_panic() {
    let mut cockpit = Cockpit::new();
    cockpit.current_screen = Screen::Baseline;

    // Force both curves into Error by loading from a bogus path. The metrics
    // half is sourced from the const and stays `Ready` (honest degrade).
    let bogus_2023 = std::path::Path::new("/__definitely_missing__/bh-2023.csv");
    let bogus_2024 = std::path::Path::new("/__definitely_missing__/bh-2024.csv");
    cockpit.baseline_screen_state.curve_2023 = baseline::load_baseline_curve(bogus_2023);
    cockpit.baseline_screen_state.curve_2024 = baseline::load_baseline_curve(bogus_2024);

    // Both curves must be Error (the fixtures-only-checkout shape).
    assert!(matches!(
        cockpit.baseline_screen_state.curve_2023,
        PanelState::Error(_)
    ));
    assert!(matches!(
        cockpit.baseline_screen_state.curve_2024,
        PanelState::Error(_)
    ));
    // The KPI strip stays populated from the const even with the curve absent.
    assert!(matches!(
        cockpit.baseline_screen_state.active_metrics(),
        PanelState::Ready(_)
    ));

    // Render the Error state in BOTH themes — no panic, non-zero root.
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        render_layout_ok(ui::shell::screen_body(Screen::Baseline, &cockpit, mode));
    }

    // The year toggle still works in the Error state (pure assignment).
    cockpit.baseline_screen_state.active_year = BaselineYear::Y2023;
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        render_layout_ok(ui::shell::screen_body(Screen::Baseline, &cockpit, mode));
    }
}

/// The happy path renders too: when the committed CSVs are present they load
/// to `Ready` and the full screen (curve + band + strip) lays out in both
/// themes. Skipped in a minimal checkout that omits the runbook artifacts —
/// the Error path above is the unconditional gate.
#[test]
fn baseline_ready_state_renders_when_csvs_present() {
    let mut cockpit = Cockpit::new();
    cockpit.current_screen = Screen::Baseline;
    baseline::load_into(&mut cockpit);

    let has_data = matches!(
        cockpit.baseline_screen_state.curve_2024,
        PanelState::Ready(_)
    );
    if !has_data {
        // Minimal checkout — runbook CSVs absent. Error path covers this.
        return;
    }

    // 2024 default, then toggle to 2023 — both render in both themes.
    for year in [BaselineYear::Y2024, BaselineYear::Y2023] {
        cockpit.baseline_screen_state.active_year = year;
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            render_layout_ok(ui::shell::screen_body(Screen::Baseline, &cockpit, mode));
        }
    }
}
