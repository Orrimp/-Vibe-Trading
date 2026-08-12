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
use iced::advanced::layout::Node;
use iced::advanced::widget::Tree;

use ui::baseline;
use ui::state::{BaselineYear, Cockpit, Message, PanelState, Screen};
use ui::strings::{BASELINE_DATA_CORRUPT, BASELINE_DATA_UNAVAILABLE};
use ui::theme::ThemeMode;

/// Lay out a screen body and return the root `Node` (the pass where a render
/// panic actually surfaces — constructing the `Element` alone does not walk
/// the widget tree).
fn layout_body(element: Element<'_, Message>) -> Node {
    let mut element = element;
    let mut tree = Tree::new(element.as_widget());
    let renderer = iced::Renderer::new(iced::Font::DEFAULT, iced::Pixels(16.0));
    let limits =
        iced::advanced::layout::Limits::new(iced::Size::ZERO, iced::Size::new(1440.0, 900.0));
    element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits)
}

/// Drive `Widget::layout` on a screen body and confirm the root node has a
/// positive area — proof the render pass ran end-to-end without panicking.
fn render_layout_ok(element: Element<'_, Message>) {
    let node = layout_body(element);
    let size = node.size();
    assert!(
        size.width > 0.0 && size.height > 0.0,
        "Baseline screen body laid out to a zero-dim root ({} x {})",
        size.width,
        size.height
    );
}

/// Walk down single-child wrappers (the `Scrollable` shell, containers) to the
/// first node that actually branches — the screen's composed `Column`.
fn composed_column(root: &Node) -> &Node {
    let mut cursor = root;
    while cursor.children().len() == 1 {
        cursor = &cursor.children()[0];
    }
    cursor
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
/// themes.
///
/// **No skip-if-absent** (story-2-18 review M-skips): the CSVs are committed,
/// the artifacts directory has already moved once, and a `return` here was
/// counted nowhere — a silently-skipped gate is indistinguishable from a
/// passing one in every report anybody reads.
#[test]
fn baseline_ready_state_renders_when_csvs_present() {
    let mut cockpit = Cockpit::new();
    cockpit.current_screen = Screen::Baseline;
    baseline::load_into(&mut cockpit);

    assert!(
        matches!(
            cockpit.baseline_screen_state.curve_2024,
            PanelState::Ready(_)
        ),
        "the committed 2024 BH curve must load Ready (state = {}) — it lives at \
         {}; if the artifacts moved again, fix the path rather than skipping \
         this gate",
        cockpit.baseline_screen_state.curve_2024.variant_name(),
        baseline::baseline_csv_path(BaselineYear::Y2024).display()
    );

    // 2024 default, then toggle to 2023 — both render in both themes.
    for year in [BaselineYear::Y2024, BaselineYear::Y2023] {
        cockpit.baseline_screen_state.active_year = year;
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            render_layout_ok(ui::shell::screen_body(Screen::Baseline, &cockpit, mode));
        }
    }
}

/// **The composition gate** (story-2-18 review M-mirror).
///
/// The panel-snapshot mirror re-derives the screen from model state, so it
/// cannot notice a row that stopped being composed: deleting `.push(kpi)` from
/// `screens::baseline::view` left every snapshot green. This test lays out the
/// PRODUCTION body through `shell::screen_body` and asserts the composed
/// column's shape — how many rows, in what order, at what heights — so a
/// dropped or reordered panel is a red test rather than a missing screen.
///
/// The two fixed heights are the widgets' own contracts: `equity_curve` is
/// 240 px (`CURVE_HEIGHT_PX`, R9.4) and `drawdown_band` is 100 px
/// (`BAND_HEIGHT_PX`, R7.3) — independently specified, not read back from the
/// implementation.
#[test]
fn baseline_body_composes_every_row_in_order() {
    let mut cockpit = Cockpit::new();
    cockpit.current_screen = Screen::Baseline;
    baseline::load_into(&mut cockpit);
    assert!(
        matches!(
            cockpit.baseline_screen_state.curve_2024,
            PanelState::Ready(_)
        ),
        "composition gate needs the committed curve"
    );

    let root = layout_body(ui::shell::screen_body(
        Screen::Baseline,
        &cockpit,
        ThemeMode::Dark,
    ));
    let column = composed_column(&root);
    let rows = column.children();

    // headline_row · caption · kpi_strip · sharpe_note · curve · band ·
    // sampling_note · risk_detail
    assert_eq!(
        rows.len(),
        8,
        "Baseline composes 8 rows when a curve is drawn; got {} — a panel was \
         dropped from or added to screens::baseline::view",
        rows.len()
    );

    let curve_h = rows[4].size().height;
    let band_h = rows[5].size().height;
    assert!(
        (curve_h - 240.0).abs() < 1.0,
        "row 4 must be the 240 px equity curve, got {curve_h} px — the stack \
         was reordered or a row was dropped above it"
    );
    assert!(
        (band_h - 100.0).abs() < 1.0,
        "row 5 must be the 100 px drawdown band, got {band_h} px"
    );

    // The KPI strip sits between the caption and the Sharpe note and is a real
    // band of cards, not a collapsed slot.
    let kpi_h = rows[2].size().height;
    assert!(
        kpi_h > 40.0,
        "row 2 must be the populated KPI strip, got {kpi_h} px"
    );

    // Without a drawn curve there is nothing to reconcile, so the sampling
    // note is absent — one row fewer, and the curve/band shift up by one.
    let bogus = std::path::Path::new("/__definitely_missing__/bh.csv");
    cockpit.baseline_screen_state.curve_2024 = baseline::load_baseline_curve(bogus);
    let root = layout_body(ui::shell::screen_body(
        Screen::Baseline,
        &cockpit,
        ThemeMode::Dark,
    ));
    let column = composed_column(&root);
    assert_eq!(
        column.children().len(),
        7,
        "with no curve loaded the sampling note must not be composed"
    );
}

/// A **corrupt** (present-but-unreadable) CSV must reach the operator as a
/// different statement from an absent one — and must still lay out (review
/// M-states).
#[test]
fn corrupt_csv_reports_corruption_not_absence() {
    let dir = std::env::temp_dir().join(format!("baseline_corrupt_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("bh-truncated.csv");
    // A file that exists and is bundled — but is truncated mid-header.
    std::fs::write(&path, "bar_ind").expect("write");

    match baseline::load_baseline_curve(&path) {
        PanelState::Error(msg) => assert_eq!(
            msg.as_str(),
            BASELINE_DATA_CORRUPT,
            "a bundled-but-damaged CSV must not claim it 'isn't bundled in \
             this build' — that sends the operator to the wrong fix"
        ),
        other => panic!("expected Error(corrupt), got {}", other.variant_name()),
    }

    let mut cockpit = Cockpit::new();
    cockpit.current_screen = Screen::Baseline;
    cockpit.baseline_screen_state.curve_2024 = baseline::load_baseline_curve(&path);
    for mode in [ThemeMode::Dark, ThemeMode::Light] {
        render_layout_ok(ui::shell::screen_body(Screen::Baseline, &cockpit, mode));
    }

    let _ = std::fs::remove_file(&path);
}
