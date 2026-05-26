//! Test 3 — `progress_bar::view` label-present vs label-absent structural
//! assertion (lab-end-to-end-v2 T-AR-6 / R8.2 / Bug #63 diagnostic).
//!
//! The widget at `crates/ui/src/widgets/progress_bar.rs` falls back to a
//! 30% indeterminate fill when `progress = None`, and skips the label `Row`
//! when `label = None`. These tests assert the structural difference between
//! the two rendering modes without running a full iced application.
//!
//! ## Structural discriminant: Widget tree depth
//!
//! `progress_bar::view(Some(f), Some("…"), mode)` returns a `Row` element
//! wrapping both a `ProgressBar` child and a `Text` child. `Row` reports
//! 2 children via `Widget::children()`.
//!
//! `progress_bar::view(None, None, mode)` returns a bare `ProgressBar`
//! element. `ProgressBar` has no children → `Widget::children()` returns
//! an empty Vec (default impl).
//!
//! We use `Tree::new(element.as_widget()).children.len()` as the
//! discriminant. This follows the same pattern as `layout_invariants.rs`
//! which drives `Tree::new(element.as_widget())` to inspect the widget tree.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use iced::Element;
use iced::advanced::widget::Tree;

use ui::state::Message;
use ui::theme::ThemeMode;
use ui::widgets::progress_bar;

// ── Label-present tests ───────────────────────────────────────────────────────

/// T-PB-3a — `view(Some(f), Some(label), Dark)` wraps bar + label in a `Row`.
///
/// A `Row<[ProgressBar, Text]>` element reports 2 children via `Widget::children()`.
/// This confirms the label `Text` node is wired into the widget tree.
#[test]
fn view_with_label_has_two_children() {
    let el: Element<'static, Message> =
        progress_bar::view(Some(0.5), Some("360 / 720 bars · 1.5s"), ThemeMode::Dark);

    let tree = Tree::new(el.as_widget());
    assert_eq!(
        tree.children.len(),
        2,
        "label-present view() must produce a Row with 2 children (ProgressBar + Text label); \
         got {} children. This means the label is NOT in the widget tree.",
        tree.children.len()
    );
}

/// T-PB-3b — `view(Some(f), Some(label), Light)` also has two children.
#[test]
fn view_with_label_light_mode_has_two_children() {
    let el: Element<'static, Message> =
        progress_bar::view(Some(0.572), Some("412 / 720 bars · 3.4s"), ThemeMode::Light);

    let tree = Tree::new(el.as_widget());
    assert_eq!(
        tree.children.len(),
        2,
        "light-mode label-present view() must produce a Row with 2 children; got {}",
        tree.children.len()
    );
}

/// T-PB-3c — `view(Some(1.0), Some(label), Dark)` at 100% still has two children.
#[test]
fn view_with_label_100pct_has_two_children() {
    let el: Element<'static, Message> =
        progress_bar::view(Some(1.0), Some("720 / 720 bars · 3.0s"), ThemeMode::Dark);

    let tree = Tree::new(el.as_widget());
    assert_eq!(
        tree.children.len(),
        2,
        "100% complete label-present view() must produce a Row with 2 children; got {}",
        tree.children.len()
    );
}

// ── Label-absent tests ────────────────────────────────────────────────────────

/// T-PB-3d — `view(None, None, Dark)` produces a bare ProgressBar with 0 children.
///
/// The indeterminate variant must NOT include a label Row. A bare `ProgressBar`
/// widget has no children (`Widget::children()` returns the default empty Vec).
#[test]
fn view_indeterminate_no_label_has_no_children() {
    let el: Element<'static, Message> = progress_bar::view(None, None, ThemeMode::Dark);

    let tree = Tree::new(el.as_widget());
    assert_eq!(
        tree.children.len(),
        0,
        "indeterminate no-label view() must produce a bare ProgressBar with 0 children; \
         got {} children. This means a label Row was unexpectedly added.",
        tree.children.len()
    );
}

/// T-PB-3e — `view(Some(0.0), None, Dark)` is determinate but label-absent → 0 children.
#[test]
fn view_determinate_no_label_has_no_children() {
    let el: Element<'static, Message> = progress_bar::view(Some(0.0), None, ThemeMode::Dark);

    let tree = Tree::new(el.as_widget());
    assert_eq!(
        tree.children.len(),
        0,
        "determinate no-label view() must produce a bare ProgressBar with 0 children; got {}",
        tree.children.len()
    );
}

/// T-PB-3f — `view(Some(0.5), None, Light)` is determinate but label-absent → 0 children.
#[test]
fn view_determinate_no_label_light_has_no_children() {
    let el: Element<'static, Message> = progress_bar::view(Some(0.5), None, ThemeMode::Light);

    let tree = Tree::new(el.as_widget());
    assert_eq!(
        tree.children.len(),
        0,
        "light-mode determinate no-label view() must have 0 children; got {}",
        tree.children.len()
    );
}

// ── Structural distinction guard ──────────────────────────────────────────────

/// T-PB-3g — label-present vs label-absent differ in child count.
///
/// This is the key regression gate: if someone accidentally makes `view(None, None)`
/// return a Row wrapper (or makes `view(Some, Some)` drop the label), this
/// test catches it by asserting the two paths have different child counts.
#[test]
fn label_present_vs_absent_differ_in_child_count() {
    let with_label: Element<'static, Message> =
        progress_bar::view(Some(0.5), Some("360 / 720 bars · 1.5s"), ThemeMode::Dark);
    let without_label: Element<'static, Message> = progress_bar::view(None, None, ThemeMode::Dark);

    let tree_with = Tree::new(with_label.as_widget());
    let tree_without = Tree::new(without_label.as_widget());

    assert!(
        tree_with.children.len() > tree_without.children.len(),
        "label-present path must have more children than label-absent path: \
         with={}, without={}",
        tree_with.children.len(),
        tree_without.children.len()
    );
}

// ── Regression guard ─────────────────────────────────────────────────────────

/// T-PB-3h — `view` never panics across the full parameter matrix.
///
/// Belt-and-suspenders integration check extending the inline widget tests.
#[test]
fn view_never_panics_across_parameter_matrix() {
    let labels = [
        None,
        Some("0 / 720 bars · 0.0s"),
        Some("360 / 720 bars · 1.5s"),
        Some("720 / 720 bars · 3.0s"),
    ];
    let progresses = [None, Some(0.0f32), Some(0.5), Some(1.0)];
    let modes = [ThemeMode::Dark, ThemeMode::Light];

    for &progress in &progresses {
        for &label in &labels {
            for mode in modes {
                let _el: Element<'static, Message> = progress_bar::view(progress, label, mode);
            }
        }
    }
}
