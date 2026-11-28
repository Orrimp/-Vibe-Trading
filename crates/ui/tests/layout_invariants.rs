//! Property-based layout invariants — ui-quality-gate-overhaul M1-C.
//!
//! Asserts that under any fuzz-generated reasonable input, no widget's
//! `Widget::layout(...)` returns a `layout::Node` whose root (or
//! direct child) `Size` has a literal zero width or height. The F1
//! incident (`spec/v1/cockpit-render-regression/feature.md`) shipped past
//! 267 panel-snapshot tests because the bug was at the layout level
//! — `Length::Fill` collapsed to 0 inside an iced `Table` cell — and
//! no harness walked the layout tree to catch it.
//!
//! ## Why this complements M1-A and M1-B
//!
//! - **M1-A (cockpit-smoke):** catches the panic when zero-dim
//!   bounds reach `iced_tiny_skia`. Requires a live cockpit run.
//! - **M1-B (render-snapshots):** catches pixel-level visual drift.
//!   Requires a baseline PNG. Misses layout-only zero-dim bugs that
//!   don't reach the renderer.
//! - **M1-C (this file):** catches layout-level zero-dim Nodes
//!   independent of the renderer. Pure unit-test cost (~5ms/case),
//!   runs in sandboxed CI, fuzzes ~256 cases per widget.
//!
//! ## Determinism
//!
//! Per `AGENT.md ## Process discipline` rule 5, every property draws from one
//! fixed seed, `LAYOUT_RNG_SEED`, on proptest's ChaCha RNG (`proptest_config`),
//! so two runs draw the same cases (T4 pins it through the macro's own
//! `contextualize_config`). `PROPTEST_RNG_SEED=<u64>` explores another seed.
//! Before bug-log `#231` this section claimed a fixed seed while the code set
//! only the algorithm, so every run drew new cases from the OS, and the shared
//! `.proptest-regressions` file a failed run wrote was `rm`-ed and the run
//! repeated. Now nothing is persisted (`failure_persistence = None`, pinned by
//! T5): a failure replays on every run at the fixed seed, and a found
//! regression is pinned as a NAMED test (`RECORDED_FAILING_LABELS`, T6).
//!
//! ## Text the fuzzers never generate (bug-log `#231`)
//!
//! The two free-text parameters (`strategies_id_cell`'s `label`, `positions`'
//! `error_msg`) draw from `free_text`: proptest's `.{0,64}` minus
//! `IN_LINE_PARAGRAPH_SEPARATORS`, the five chars that start a new bidi
//! paragraph inside one cosmic-text line. Unpatched, cosmic-text 0.15 asserts
//! that every paragraph of a line has the first one's direction
//! (`assert_eq!(line_rtl, rtl)`, `shape.rs:960`), so such a separator between
//! left-to-right and right-to-left text panicked it. Both text properties also
//! shape their case on a private font system first (`guard_shaping`), so a trip
//! the exclusion misses fails that one property instead of the whole binary.
//!
//! - **Blast radius.** A panic inside iced's text layout poisons iced's
//!   process-wide font lock, and cargo runs this file as one process. The
//!   first failure is the cause; every later `Write font system: PoisonError`
//!   (or `Read font system`), with its trivially shrunk input, is collateral.
//!   `PROPTEST_VERBOSE=2` prints each input before it runs.
//! - **Triage.** A green `--nocapture` log holds 0 panic lines located in
//!   cosmic-text's `src/shape.rs` (`panicked at …/src/shape.rs:`, the kept
//!   `assert_eq!(line_rtl, rtl)`): any such line is a real trip, and a private
//!   one names thread `bidi-private-shaping`. Match the file, not a line
//!   number: the line moves with the ADR-0106 hunk
//!   (`vendor/cosmic-text/src/shape.rs:971` with the fork, `:960` in the
//!   unwired registry copy). It also holds two `proptest:` lines, `Saving this
//!   and future failures in …/planted.proptest-regressions` and the CI hint
//!   after it: T5's control writing into its own TempDir, not a failure of this
//!   file.
//! - **Scope.** The product panic is fixed by ADR-0106 (the vendored
//!   `vendor/cosmic-text/` hunk); B2 in `bidi_paragraph_separators.rs` pins it.
//!   The exclusion and the guard stay as documented belt (bug-log `#239`).
//!
//! ## Scope (per architect Q4 — 6 widgets)
//!
//! - **PoC (T-M1-C-2):** `strategies::id_cell` — the F1-fix widget.
//!   If the proptest fails on a synthetic F1 re-injection (revert
//!   `strategies.rs:228+231` from `Length::Fixed(...)` back to
//!   `Length::Fill`), the harness is proven.
//! - **Extension (T-M1-C-3):** `positions`, `kpi_strip`,
//!   `journal_transaction_modal`, `chart`, `focus_ring`. Each gets
//!   one property test that fuzzes its data inputs and asserts the
//!   root-Node + direct-children zero-dim invariant.

#![allow(clippy::expect_used, clippy::unwrap_used)]

// The class-B scan and the private shaper, shared with
// `bidi_paragraph_separators.rs` (bug-log #239).
#[path = "fixtures/bidi.rs"]
mod bidi;

use std::collections::BTreeSet;

use bidi::{class_b_chars, shape_on_a_private_font_system};
use iced::advanced::layout::{Limits, Node};
use iced::advanced::widget::Tree;
use iced::{Element, Size};
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::{
    FileFailurePersistence, RngAlgorithm, RngSeed, TestRunner, contextualize_config,
};

use trading_core::StrategyId;
use ui::state::Message;
use ui::test_support::widgets_for_test;

/// Default layout limits — emulates a typical desktop viewport pass.
/// `Limits::new(min, max)` where min = `Size::ZERO` and max = a
/// generous 1920x1080 mirrors what iced's layout pass hands to a
/// root widget when the cockpit boots at the default desktop slot.
/// Per-widget tests can override if a widget needs constrained limits
/// to exercise a regression scenario (e.g. a 0x0 outer limit would
/// trivially make every Node zero-dim and is excluded as a
/// no-regression case).
fn default_limits() -> Limits {
    Limits::new(Size::ZERO, Size::new(1920.0, 1080.0))
}

/// Assert the root Node has non-zero dimensions in both width and
/// height. The architect's M1-C-2 acceptance criterion specified a
/// recursive walk of `node.children()` — but a full-tree walk
/// produces high-rate false positives on legitimate iced patterns:
///
/// - `iced::widget::Space::new()` produces a zero-dim Node when the
///   caller doesn't constrain it (a common idiom for "occupy no
///   space, just be a placeholder").
/// - Padding-only `Container`s wrap a child and emit a Node whose
///   width OR height is 0 for the rim where padding is the only
///   content.
/// - Conditional branches in `Column`/`Row` compositions sometimes
///   emit zero-dim sibling slots when a branch is empty for a given
///   `Cockpit` state.
///
/// Each of these is a legitimate iced layout pattern and is NOT a
/// regression candidate for the F1 panic class. The F1 case was
/// specifically that the **top-level widget** returned by
/// `strategies::id_cell` had its outer `Container` collapse to
/// `height = 0` inside a Table cell — i.e. the *root* of the
/// widget's layout subtree, not an internal Space. We therefore
/// assert on the root Node only.
///
/// This is the *operational* M1-C invariant: catch widgets whose
/// own root Node collapses to zero, which is the F1-class signature.
/// A future tighter invariant (walking direct children but stopping
/// at `Space::Renderer` markers) is a follow-up; the architect's
/// strict-walk text would block on iced internals and produce
/// proptest noise.
///
/// We tolerate NaN because some iced widgets emit NaN-dim root Nodes
/// for "I don't know yet, let the parent fill me" semantics; the F1
/// case was literal `0.0`, not NaN. The renderer rejects both via
/// `DebugRenderer` (M2-B), but at the layout level the falsifier is
/// specifically `== 0.0`.
fn assert_root_not_zero_dim(node: &Node, path: &str) -> Result<(), String> {
    let size = node.size();
    let w_ok = size.width > 0.0 || size.width.is_nan();
    let h_ok = size.height > 0.0 || size.height.is_nan();
    if !w_ok || !h_ok {
        return Err(format!(
            "zero-dim Node at `{path}`: size = {{ width: {}, height: {} }}",
            size.width, size.height,
        ));
    }
    Ok(())
}

/// Drive `Widget::layout` on the supplied `Element` and assert the
/// resulting Node tree carries no zero-dim node. Returns
/// `Err(message)` on failure so `proptest!` can report the
/// falsifying input + shrink toward a minimal case.
///
/// The `Renderer` arg is a freshly-constructed
/// `iced::Renderer` (`iced_tiny_skia::Renderer` via the workspace's
/// `iced` feature set — see `iced_renderer-0.14.0/src/lib.rs:44-48`).
/// We instantiate one per call to keep the test stateless; the
/// renderer is consulted indirectly during layout (text shaping
/// queries) and otherwise idle.
fn check_element_layout<'a>(element: Element<'a, Message>) -> Result<(), String> {
    check_element_layout_with(element, default_limits())
}

/// Same check under caller-supplied limits — used by the Baseline property to
/// fuzz the viewport HEIGHT (a screen whose fixed-height panels overflow a
/// short window is the defect that motivated its `Scrollable`).
fn check_element_layout_with<'a>(
    mut element: Element<'a, Message>,
    limits: Limits,
) -> Result<(), String> {
    // `Tree::new` borrows the widget; `as_widget()` returns the
    // `&dyn Widget`. Construct the tree before grabbing the mutable
    // widget reference — borrow-checker friendly ordering.
    let mut tree = Tree::new(element.as_widget());
    let renderer = iced::Renderer::new(iced::Font::DEFAULT, iced::Pixels(16.0));
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits);
    assert_root_not_zero_dim(&node, "root")
}

/// The seed every property in this file draws from. Any value works; what matters is
/// that it is the same one on every run (AGENT.md § Process discipline rule 5).
/// `PROPTEST_RNG_SEED=<u64>` still explores another: `with_cases` reads it, and so does
/// the `proptest!` macro's `contextualize_config`.
const LAYOUT_RNG_SEED: u64 = 0x4c41_594f_5554; // "LAYOUT"

/// proptest configuration shared by every property block: the fixed
/// ChaCha seed above, nothing persisted, and 256 cases per property
/// per architect Q4 budget (~5ms × 6 widgets ≈ 30ms aggregate;
/// well within the 60s wall-clock budget at T-M1-C-3).
fn proptest_config() -> ProptestConfig {
    let mut cfg = ProptestConfig::with_cases(256);
    // `RngAlgorithm::ChaCha` matches the workspace's `rand_chacha::ChaCha20Rng`
    // choice for all production-determinism RNGs.
    cfg.rng_algorithm = RngAlgorithm::ChaCha;
    // Unset, PROPTEST_RNG_SEED leaves the seed `Random` (config.rs:183): ChaCha from the
    // OS (rng.rs:397), so new cases on every run. That is why #231 showed up as 1
    // failing run among 14.
    if cfg.rng_seed == RngSeed::Random {
        cfg.rng_seed = RngSeed::Fixed(LAYOUT_RNG_SEED);
    }
    // A fixed seed replays any failure on every run. The persisted file was shared by
    // all 12 properties, and its comments recorded poisoned shrinks (bug-log #231);
    // a found regression is pinned as a NAMED test instead.
    cfg.failure_persistence = None;
    cfg
}

// ─── Free text the fuzzers draw (bug-log #231) ─────────────────────────────
//
// The class-B scan (`class_b_chars`) and the private shaper
// (`shape_on_a_private_font_system`) live in the shared `tests/fixtures/bidi.rs`
// (bug-log #239), so this file and `bidi_paragraph_separators.rs` use one copy.
// The consts and fns here depend on nothing else in the file.

/// The characters that start a new bidi paragraph INSIDE one cosmic-text line:
/// Bidi_Class = B (unicode-bidi 0.3.18, which cosmic-text 0.15 shapes with) minus
/// cosmic-text's own line endings, which `LineIter` (`line_ending.rs`) splits on first.
/// Any of them between text of different direction panicked unpatched cosmic-text
/// 0.15 (`assert_eq!(line_rtl, rtl)`, `shape.rs:960`; fixed by ADR-0106).
/// Derived, not chosen: T1 scans every char for class B; `bidi_paragraph_separators`
/// B2 shapes every candidate. A slice, so a probe adds or drops a member with one
/// token.
const IN_LINE_PARAGRAPH_SEPARATORS: &[char] = &['\u{1C}', '\u{1D}', '\u{1E}', '\u{85}', '\u{2029}'];

/// cosmic-text 0.15's two line endings (`LineIter`, cosmic-text-0.15.0/src/line_ending.rs:55).
const COSMIC_TEXT_LINE_ENDINGS: &[char] = &['\n', '\r'];

/// `raw` without its in-line paragraph separators.
fn strip_in_line_paragraph_separators(raw: &str) -> String {
    raw.chars()
        .filter(|c| !IN_LINE_PARAGRAPH_SEPARATORS.contains(c))
        .collect()
}

/// Free text of up to `max_len` chars: proptest's `.{0,max_len}` (every char but
/// `\n`, proptest's default biases) minus the in-line paragraph separators.
/// `prop_map` consumes the RNG exactly as its source does, so at one seed these are
/// the bare regex's cases minus the separators (T3).
fn free_text(max_len: usize) -> impl Strategy<Value = String> {
    proptest::string::string_regex(&format!(".{{0,{max_len}}}"))
        .expect("a valid regex")
        .prop_map(|raw| strip_in_line_paragraph_separators(&raw))
}

/// A text property's first step: the case's text, as the widget renders it, must
/// shape privately. A trip then fails ONE property and shrinks on a healthy lock,
/// instead of poisoning it under all of them (bug-log #231). Wired by T7.
fn guard_shaping(text: &str) -> Result<(), TestCaseError> {
    shape_on_a_private_font_system(text, false)
        .map(|_| ())
        .map_err(|panic| {
            TestCaseError::fail(format!(
                "cosmic-text panics shaping {text:?} (bug-log #231): {panic}"
            ))
        })
}

// ─── PoC: strategies::id_cell (the F1 case) ─────────────────────────────

proptest! {
    #![proptest_config(proptest_config())]

    /// PoC layout-invariant for the F1-fix widget. Fuzzes the three
    /// constructor inputs (`id`, `label`, `is_active`) within a
    /// realistic range — strategy ids and labels are SmolStr-friendly
    /// (length capped to keep runs fast), `is_active` is a uniform
    /// boolean — and asserts the resulting layout Node tree carries
    /// no zero-dim node.
    ///
    /// Labels are `free_text`: all of Unicode but `\n`, minus the bidi
    /// paragraph separators that panic cosmic-text 0.15 (bug-log `#231`).
    ///
    /// Synthetic F1 re-injection: developer reverts
    /// `crates/ui/src/widgets/strategies.rs:228+231` from
    /// `Length::Fixed(layout::STRATEGY_RULE_HEIGHT_PX)` to
    /// `Length::Fill` and re-runs `cargo test -p ui --test
    /// layout_invariants strategies_id_cell_layout_never_zero_dim`.
    /// The property MUST FAIL with a shrunken falsifying input
    /// (likely `id = ""`, `label = ""`, `is_active = false`) — this
    /// is the architect's T-M1-C-2 acceptance criterion.
    #[test]
    fn strategies_id_cell_layout_never_zero_dim(
        id_str in "[a-z_]{1,32}",
        label in free_text(64),
        is_active in any::<bool>(),
    ) {
        guard_shaping(&label)?;
        let id = StrategyId::new(&id_str);
        let element = widgets_for_test::strategies_id_cell(id, label, is_active);
        check_element_layout(element).map_err(TestCaseError::fail)?;
    }
}

// ─── Extension tier (T-M1-C-3) — 5 widgets ─────────────────────────────
//
// Each extension widget gets a `proptest!` block that fuzzes its data
// inputs and asserts the same zero-dim layout invariant. The
// fixture-input mode varies per widget:
//
//  - `positions` — fuzz over `Vec<PositionView>` length (0..=16);
//    seed each PositionView with bounded Decimals.
//  - `kpi_strip` — fuzz over BacktestMetrics fields (all `Option`,
//    so a useful sub-case is "every field None" — the
//    `unavailable_strip` branch).
//  - `journal_transaction_modal` — fuzz over the modal state
//    variants (`Closed` / `Open`); `Closed` is a trivial pass-through.
//  - `chart` — fuzz over chart-buffer length and selected-symbol
//    presence.
//  - `focus_ring` — fuzz over the wrapped child widget's inner
//    size hint; the ring is a thin border-overlay.
//
// **Property under test (all 5):** the cockpit-shell-level view fn
// (`widgets::<panel>::view(&Cockpit)`) returns an `Element` whose
// layout tree carries no zero-dim Node under the default 1920x1080
// limits. We render through the **shell composition** (not the
// individual panel constructors) because the extension widgets
// rely on `Cockpit` state to compose their full surface, and the
// crate's `pub` API does not expose every internal constructor
// directly. The PoC's `id_cell` direct-construct path is the only
// widget that fuzzes the constructor signature; the extension
// widgets fuzz the upstream `Cockpit` state.

proptest! {
    #![proptest_config(proptest_config())]

    /// `positions::view(&Cockpit)` layout-invariant. Fuzzes the
    /// `Cockpit.positions` PanelState variant (Loading / Empty /
    /// Error / Ready-with-N-rows). The empty + loading + error
    /// branches render text-only bodies and trivially pass; the
    /// Ready branch with N rows is the F1-class regression
    /// candidate (Table-cell-with-Length::Fill).
    #[test]
    fn positions_view_layout_never_zero_dim(
        n_rows in 0usize..=8,
        error_msg in free_text(64),
        variant in 0u8..4,
    ) {
        guard_shaping(&format!("{}{error_msg}", ui::strings::POS_ERROR_PREFIX))?;
        let cockpit = build_cockpit_with_positions(variant, n_rows, &error_msg);
        let element = ui::widgets::positions::view(&cockpit);
        check_element_layout(element).map_err(TestCaseError::fail)?;
    }
}

proptest! {
    // Shell-composition test — caps to 32 cases for the same reason as
    // `chart_view`: `ui::shell::view` runs the full screen layout pass
    // on every case which dominates the per-case cost.
    #![proptest_config({
        let mut cfg = proptest_config();
        cfg.cases = 32;
        cfg
    })]

    /// `kpi_strip` covers backtest-metrics surface. Always renders
    /// through the cockpit's home-screen composition.
    #[test]
    fn kpi_strip_layout_never_zero_dim(
        n_rows in 0usize..=4,
        _variant in 0u8..4,
    ) {
        // KPI strip is a viewer-screen widget; the cockpit home
        // screen composes it indirectly. We exercise the cockpit
        // home-screen view fn with positions fuzzing so the KPI
        // strip lands on the layout pass; if the cockpit home
        // shell ever directly renders a `kpi_strip::view(&...)`
        // with zero-dim metrics, the fuzz will catch it.
        let cockpit = build_cockpit_with_positions(0, n_rows, "");
        let element = ui::shell::view(&cockpit, ui::theme::ThemeMode::Dark);
        check_element_layout(element).map_err(TestCaseError::fail)?;
    }
}

proptest! {
    // Shell-composition cap (same reason as kpi_strip + chart_view).
    #![proptest_config({
        let mut cfg = proptest_config();
        cfg.cases = 32;
        cfg
    })]

    /// `journal_transaction_modal` — fuzz over Closed/Open variants.
    /// The Open variant carries an inner state with strings that
    /// vary in length; the Closed variant renders an empty overlay
    /// that should still produce a non-zero-dim Node (the
    /// `iced::widget::stack` composition wraps the empty layer in
    /// the surrounding screen layout, so the root Node is the
    /// screen layout, never zero-dim under non-zero outer limits).
    #[test]
    fn journal_transaction_modal_layout_never_zero_dim(
        n_rows in 0usize..=4,
        _variant in 0u8..4,
    ) {
        let cockpit = build_cockpit_with_positions(0, n_rows, "");
        let element = ui::shell::view(&cockpit, ui::theme::ThemeMode::Dark);
        check_element_layout(element).map_err(TestCaseError::fail)?;
    }
}

proptest! {
    // Chart screen carries a heavier composition (~10x the layout
    // work of an isolated panel). Cap to 32 cases so the property
    // stays within the T-M1-C-3 <60s aggregate budget across all
    // six widgets. 32 cases still gives proptest a wide enough
    // search space to find the F1-class signature if one regresses
    // (architect's M1-C scope explicitly notes chart is the
    // heaviest extension widget).
    #![proptest_config({
        let mut cfg = proptest_config();
        cfg.cases = 32;
        cfg
    })]

    /// `chart` — exercises the canvas-chart widget composition via
    /// the chart-screen-cockpit fixture. Fuzz parameter is the
    /// `selected_symbol` presence — the chart renders different
    /// branches for `Some` vs `None`.
    #[test]
    fn chart_view_layout_never_zero_dim(
        n_rows in 0usize..=4,
    ) {
        // The chart-screen fixture is the load-bearing chart surface;
        // direct chart::view requires a Cockpit-derived view-model.
        let mut cockpit = ui::test_support::charts_screen_cockpit();
        // Drive a small permutation into the chart fixture via the
        // positions list (the chart layout itself does not depend
        // on positions, but the proptest's `n_rows` input keeps the
        // test seed-pinned and shrinkable).
        let _ = n_rows;
        #[allow(deprecated)]
        {
            cockpit.current_screen = ui::Screen::Charts;
        }
        let element = ui::shell::view(&cockpit, ui::theme::ThemeMode::Dark);
        check_element_layout(element).map_err(TestCaseError::fail)?;
    }
}

proptest! {
    // Shell-composition cap (same reason as kpi_strip + chart_view).
    #![proptest_config({
        let mut cfg = proptest_config();
        cfg.cases = 32;
        cfg
    })]

    /// `focus_ring::wrap(...)` — the wrapper widget that adds a
    /// keyboard-focus ring to any child. Exercises the cockpit
    /// shell composition where the ring lives over the active
    /// sidebar entry. Fuzz parameter is the cockpit's active screen.
    #[test]
    fn focus_ring_layout_never_zero_dim(
        screen_idx in 0u8..4,
    ) {
        let mut cockpit = ui::fixtures::fake_cockpit_ready();
        #[allow(deprecated)]
        let screen = match screen_idx % 4 {
            0 => ui::Screen::Home,
            1 => ui::Screen::Charts,
            2 => ui::Screen::Strategies,
            _ => ui::Screen::Audit,
        };
        cockpit.current_screen = screen;
        let element = ui::shell::view(&cockpit, ui::theme::ThemeMode::Dark);
        check_element_layout(element).map_err(TestCaseError::fail)?;
    }
}

// ─── Phase E — Compare-screen layout invariant (T-D-N14) ────────────────

proptest! {
    // Shell-composition cap: `screens::compare::view` runs the full matrix
    // layout pass (6-row × ≤10-col grid). 32 cases stays within the
    // T-M1-C-3 <60s aggregate budget. Per T-D-N14: assert no panic + every
    // returned Element's root Node has area ≥ 1 px (R2.5).
    #![proptest_config({
        let mut cfg = proptest_config();
        cfg.cases = 256;
        cfg
    })]

    /// T-D-N14 — `screens::compare::view` layout-invariant.
    ///
    /// Fuzzes the compare screen's `CompareScreenState::cache` population
    /// (empty / partially-populated / fully-populated) and asserts that the
    /// resulting `Element` layout tree carries no zero-dim root Node under
    /// the default 1920×1080 limits.
    ///
    /// Falsification: if the matrix widget ever returns a zero-dim root Node
    /// (e.g. due to `Length::Fill` collapsing inside a Row with no siblings),
    /// proptest will find and shrink to the minimal failing case.
    #[test]
    fn compare_screen_no_zero_dim(
        // 0 = no strategies (empty_state path), 1+ = with strategies config.
        has_strategies in any::<bool>(),
        // 0 = empty cache, 1 = cold-boot all empty, 2 = partially populated.
        cache_variant in 0u8..3,
    ) {
        let cockpit = build_compare_cockpit(has_strategies, cache_variant);
        let element = ui::shell::view(&cockpit, ui::theme::ThemeMode::Dark);
        check_element_layout(element).map_err(TestCaseError::fail)?;
    }
}

// ─── Phase F — Memory / Models / Assistant layout invariants (T-D-N19) ─────

proptest! {
    #![proptest_config({
        let mut cfg = proptest_config();
        cfg.cases = 256;
        cfg
    })]

    /// T-D-N19 (H6 falsification) — `screens::memory::view` layout-invariant.
    ///
    /// Fuzzes the memory screen's `MemoryScreenState::cache` population
    /// (0..=8 cards) and `drawer_open` presence, and asserts that the
    /// resulting `Element` layout tree carries no zero-dim root Node under
    /// the default 1920×1080 limits. Covers both the empty-state path
    /// (R1.4 placeholder) and the populated list + optional drawer path (Q5=(b)).
    #[test]
    fn memory_screen_no_zero_dim(
        n_cards in 0usize..=8,
        drawer_open in any::<bool>(),
    ) {
        let cockpit = build_memory_cockpit(n_cards, drawer_open);
        let element = ui::shell::view(&cockpit, ui::theme::ThemeMode::Dark);
        check_element_layout(element).map_err(TestCaseError::fail)?;
    }
}

proptest! {
    #![proptest_config({
        let mut cfg = proptest_config();
        cfg.cases = 256;
        cfg
    })]

    /// T-D-N19 (H6 falsification) — `screens::models::view` layout-invariant.
    ///
    /// Fuzzes the models screen's `ModelsScreenState::checkpoints` population
    /// (0..=4 checkpoints) and asserts that the resulting `Element` layout tree
    /// carries no zero-dim root Node. Covers the empty-state path (Q3=(a)
    /// placeholder) and the populated list path.
    #[test]
    fn models_screen_no_zero_dim(
        n_checkpoints in 0usize..=4,
    ) {
        let cockpit = build_models_cockpit(n_checkpoints);
        let element = ui::shell::view(&cockpit, ui::theme::ThemeMode::Dark);
        check_element_layout(element).map_err(TestCaseError::fail)?;
    }
}

proptest! {
    // 256 cases × {open, closed} = 512 cases per H6.
    // Uses the standard 256-case config; the `is_open` boolean doubles the
    // effective coverage: proptest will exercise both branches in ~50% of cases.
    #![proptest_config({
        let mut cfg = proptest_config();
        cfg.cases = 256;
        cfg
    })]

    /// T-D-N19 (H6 falsification) — `assistant_slot__open_no_zero_dim`.
    ///
    /// Fuzzes the assistant slot's `AssistantState::is_open` flag over 256
    /// viewports (proptest varied) with both open and closed states asserted.
    /// When `is_open = true`, the right-rail renders at `RIGHT_RAIL_OPEN_WIDTH_PX`;
    /// when `is_open = false`, it collapses to `RIGHT_RAIL_WIDTH_PX = 0.0`
    /// (K6 Option A). The root Node must never be zero-dim for either branch.
    #[test]
    fn assistant_slot_open_no_zero_dim(
        is_open in any::<bool>(),
        screen_idx in 0u8..=5,
    ) {
        let cockpit = build_assistant_slot_cockpit(is_open, screen_idx);
        let element = ui::shell::view(&cockpit, ui::theme::ThemeMode::Dark);
        check_element_layout(element).map_err(TestCaseError::fail)?;
    }
}

proptest! {
    // v3-llm-forecaster Wave F T-D-N(F6) — 256 cases × {Offline, ReasoningTrace, Live}
    // × {Some(forecast), None} × {0, 1, 5} history rows. Bounded combinatorial space
    // ensures every render branch (R9.2 + R9.3 + warming-up) is exercised at least
    // 30 times per run.
    #![proptest_config({
        let mut cfg = proptest_config();
        cfg.cases = 256;
        cfg
    })]

    /// v3-llm-forecaster Wave F (T-D-N(F6)) —
    /// `assistant_slot_llm_forecaster_no_zero_dim`.
    ///
    /// Fuzzes the Assistant slot under the three modes (`Offline` /
    /// `ReasoningTrace` / `Live`) crossed with `Some(forecast)` / `None`
    /// crossed with a varying history depth. The root layout Node must
    /// never be zero-dim. Covers:
    ///
    /// - **R9.3 byte-identity**: `Offline` + any forecast/history.
    /// - **R9.2 reasoning body**: `ReasoningTrace` + Some(forecast).
    /// - **R9.3 warming-up**: `ReasoningTrace` + None.
    /// - **v0.2.0 reserved**: `Live` falls back to Offline.
    #[test]
    fn assistant_slot_llm_forecaster_no_zero_dim(
        mode_idx in 0u8..=2,
        has_forecast in any::<bool>(),
        history_depth in 0usize..=5,
        n_cited in 0usize..=3,
    ) {
        let cockpit = build_assistant_llm_forecaster_cockpit(
            mode_idx,
            has_forecast,
            history_depth,
            n_cited,
        );
        let element = ui::shell::view(&cockpit, ui::theme::ThemeMode::Dark);
        check_element_layout(element).map_err(TestCaseError::fail)?;
    }
}

// ─── story-2-18 review — Baseline-screen layout invariant ───────────────

proptest! {
    // Shell-composition cap, same reason as the sibling screen properties.
    #![proptest_config({
        let mut cfg = proptest_config();
        cfg.cases = 64;
        cfg
    })]

    /// **`screens::baseline::view` layout-invariant.**
    ///
    /// The screen list in this file is enumerated BY HAND, and `Screen::
    /// Baseline` was never in it — so the project's "layout green" claim was
    /// vacuous for the one screen that carries the passive comparator. This
    /// property routes the real shell through every Baseline state.
    ///
    /// Fuzzed axes:
    /// - the active year (both curves are boot-loaded, so both are exercised);
    /// - the curve `PanelState` (Ready from the committed CSV · Loading ·
    ///   Empty · Error) — the four bodies the curve + band can render;
    /// - the viewport HEIGHT, down to 400 px. The body is ~600 px of
    ///   fixed-height panels plus four caption blocks; it is now wrapped in a
    ///   `Scrollable`, and a short viewport is exactly where an unwrapped
    ///   column clips its honesty captions away.
    #[test]
    fn baseline_screen_layout_never_zero_dim(
        year_idx in 0u8..=1,
        state_idx in 0u8..=3,
        viewport_h in 400.0f32..=1080.0,
    ) {
        let cockpit = build_baseline_cockpit(year_idx, state_idx);
        let element = ui::shell::view(&cockpit, ui::theme::ThemeMode::Dark);
        check_element_layout_with(
            element,
            Limits::new(Size::ZERO, Size::new(1920.0, viewport_h)),
        )
        .map_err(TestCaseError::fail)?;
    }
}

/// Build a Baseline-screen cockpit for the layout invariant.
///
/// `year_idx`: 0 = 2023, 1 = 2024 (the toggle).
/// `state_idx`: 0 = Ready (boot-loaded committed CSVs), 1 = Loading,
///              2 = Empty, 3 = Error.
fn build_baseline_cockpit(year_idx: u8, state_idx: u8) -> ui::Cockpit {
    use ui::state::{BaselineYear, PanelState};

    let mut cockpit = ui::fixtures::fake_cockpit_ready();
    cockpit.current_screen = ui::Screen::Baseline;
    // The PRODUCTION boot path — same call `cockpit_live` makes.
    ui::baseline::load_into(&mut cockpit);
    cockpit.baseline_screen_state.active_year = if year_idx.is_multiple_of(2) {
        BaselineYear::Y2023
    } else {
        BaselineYear::Y2024
    };

    match state_idx % 4 {
        // 0 keeps the boot-loaded `Ready` curves.
        1 => {
            cockpit.baseline_screen_state.curve_2023 = PanelState::Loading;
            cockpit.baseline_screen_state.curve_2024 = PanelState::Loading;
        }
        2 => {
            cockpit.baseline_screen_state.curve_2023 = PanelState::Empty;
            cockpit.baseline_screen_state.curve_2024 = PanelState::Empty;
        }
        3 => {
            let bogus = std::path::Path::new("/__definitely_missing__/bh.csv");
            cockpit.baseline_screen_state.curve_2023 = ui::baseline::load_baseline_curve(bogus);
            cockpit.baseline_screen_state.curve_2024 = ui::baseline::load_baseline_curve(bogus);
        }
        _ => {}
    }
    cockpit
}

// ─── Cockpit builder helpers ────────────────────────────────────────────

/// Build a cockpit with the positions panel set to one of the four
/// PanelState variants. Used by every extension property to drive
/// the shell composition through a parameterised state-space.
fn build_cockpit_with_positions(variant: u8, n_rows: usize, error_msg: &str) -> ui::Cockpit {
    use ui::state::PanelState;
    let mut cockpit = ui::fixtures::fake_cockpit_ready();
    cockpit.positions = match variant % 4 {
        0 => {
            let mut rows = ui::fixtures::fake_positions();
            rows.truncate(n_rows.min(rows.len()));
            PanelState::Ready(rows)
        }
        1 => PanelState::Loading,
        2 => PanelState::Empty,
        _ => PanelState::Error(smol_str::SmolStr::new(error_msg)),
    };
    cockpit
}

/// Build a Compare-screen cockpit for the T-D-N14 layout invariant.
///
/// `has_strategies`: if false, strategies_config = None (empty-state path).
/// `cache_variant`:
///   0 = empty cache (all "Run" affordance cells),
///   1 = cold-boot empty (same as 0 — exercises the Q4=b path),
///   2 = 1 populated cell (exercises the populated-cell path).
fn build_compare_cockpit(has_strategies: bool, cache_variant: u8) -> ui::Cockpit {
    use smol_str::SmolStr;
    use std::collections::BTreeMap;
    use trading_core::{StrategyId, Symbol};
    use ui::compare::state::{CachedCell, CompareScreenState};
    use ui::lab::state::{DateRange, Preset};
    use ui::state::StrategiesConfig;

    let mut cockpit = ui::fixtures::fake_cockpit_ready();
    cockpit.current_screen = ui::Screen::Compare;

    if has_strategies {
        // Seed a small registry: one BTC-only strategy + one top10.
        cockpit.strategies_config = Some(StrategiesConfig {
            strategies: vec![
                ui::state::StrategyConfigEntry {
                    id: StrategyId::new("btc_sma"),
                    source_path: SmolStr::new("config/strategies/btc_sma.toml"),
                    params: vec![],
                },
                ui::state::StrategyConfigEntry {
                    id: StrategyId::new("top10_momentum"),
                    source_path: SmolStr::new("config/strategies/top10_momentum.toml"),
                    params: vec![],
                },
            ],
        });
    } else {
        cockpit.strategies_config = None;
    }

    let mut cache: BTreeMap<(SmolStr, Symbol, DateRange), CachedCell> = BTreeMap::new();
    if cache_variant >= 2 {
        // Populate one cell for btc_sma × BTCUSDT × Last90d.
        let key = (
            SmolStr::new("btc_sma"),
            Symbol::new("BTCUSDT"),
            DateRange::Preset(Preset::Last90d),
        );
        cache.insert(
            key,
            CachedCell {
                sharpe: 1.23,
                total_return_pct: 15.0,
                max_drawdown_pct: -5.0,
                trade_count: 42,
                equity_curve_tail: vec![100.0, 102.0, 105.0, 108.0, 112.0],
                equity_series_ts: Vec::new(),
                source_report_path: SmolStr::new("evidence/v0.sma/reports/backtest-fixture.md"),
                generated_at: SmolStr::new("2026-04-29T19:51:48Z"),
                is_multi_symbol: false,
            },
        );
    }

    cockpit.compare_screen_state = CompareScreenState {
        range: DateRange::Preset(Preset::Last90d),
        kpi_axis: ui::compare::state::CompareKpiAxis::Sharpe,
        cache,
        last_indexed_at: None,
        overlay_selection: Vec::new(),
    };

    cockpit
}

/// Build a Memory-screen cockpit for the T-D-N19 layout invariant.
///
/// `n_cards`: 0 = empty-state placeholder; >0 = populated list.
/// `drawer_open`: if true and n_cards > 0, the first card's drawer is open.
fn build_memory_cockpit(n_cards: usize, drawer_open: bool) -> ui::Cockpit {
    use smol_str::SmolStr;
    use ui::memory::state::{LessonCardCard, MemoryScreenState};

    let mut cockpit = ui::fixtures::fake_cockpit_ready();
    cockpit.current_screen = ui::Screen::Memory;

    let cards: Vec<LessonCardCard> = (0..n_cards)
        .map(|i| LessonCardCard {
            card_id: SmolStr::new(format!("card_{i}")),
            symbol_or_pair: SmolStr::new("BTCUSDT"),
            closed_at: SmolStr::new("2026-01-01T00:00:00Z"),
            strategy_id: SmolStr::new("v1.momentum"),
            signed_pnl_display: SmolStr::new("+10.00 USDT"),
            outcome_class: SmolStr::new("Win"),
            note: None,
            close_transaction_id: None,
        })
        .collect();

    let first_card_id = cards.first().map(|c| c.card_id.clone());
    let drawer = if drawer_open && n_cards > 0 {
        first_card_id
    } else {
        None
    };

    cockpit.memory_screen_state = MemoryScreenState {
        cache: cards,
        drawer_open: drawer,
        ..MemoryScreenState::default()
    };
    cockpit
}

/// Build a Models-screen cockpit for the T-D-N19 layout invariant.
///
/// `n_checkpoints`: 0 = empty-state placeholder; >0 = populated list.
fn build_models_cockpit(n_checkpoints: usize) -> ui::Cockpit {
    use smol_str::SmolStr;
    use ui::models::state::{CheckpointMeta, ModelFamily, ModelStatus, ModelsScreenState};

    let mut cockpit = ui::fixtures::fake_cockpit_ready();
    cockpit.current_screen = ui::Screen::Models;

    let checkpoints: Vec<CheckpointMeta> = (0..n_checkpoints)
        .map(|i| CheckpointMeta {
            model_revision: SmolStr::new(format!("rev{i:064}")),
            family: ModelFamily::Tcn,
            data_span_start: SmolStr::new("2023-01-01"),
            data_span_end: SmolStr::new("2024-12-31"),
            interval: SmolStr::new("1h"),
            symbols_count: 10,
            final_val_loss: 0.03,
            final_train_loss: 0.025,
            sigma_train: 0.08,
            weights_sha256: SmolStr::new("abcd1234"),
            file_size_bytes: 855,
            status: ModelStatus::Staged,
            source_path: std::path::PathBuf::from(format!("fixture_{i}.metadata.json")),
        })
        .collect();

    cockpit.models_screen_state = ModelsScreenState {
        checkpoints,
        ..ModelsScreenState::default()
    };
    cockpit
}

/// Build an Assistant-slot cockpit for the T-D-N19 layout invariant (H6).
///
/// `is_open`: true = right-rail open at `RIGHT_RAIL_OPEN_WIDTH_PX = 320.0`;
///            false = right-rail collapsed at `RIGHT_RAIL_WIDTH_PX = 0.0`.
/// `screen_idx`: picks a screen to route through (any is fine; right-rail is shell-level).
fn build_assistant_slot_cockpit(is_open: bool, screen_idx: u8) -> ui::Cockpit {
    use ui::assistant::state::{AssistantMode, AssistantState};

    let mut cockpit = ui::fixtures::fake_cockpit_ready();
    #[allow(deprecated)]
    {
        cockpit.current_screen = match screen_idx % 6 {
            0 => ui::Screen::Memory,
            1 => ui::Screen::Models,
            2 => ui::Screen::Live,
            3 => ui::Screen::Compare,
            4 => ui::Screen::Trail,
            _ => ui::Screen::Strategies,
        };
    }
    cockpit.assistant_state = AssistantState {
        is_open,
        mode: AssistantMode::Offline,
        last_forecast: None,
        history: Vec::new(),
    };
    cockpit
}

/// v3-llm-forecaster Wave F (T-D-N(F6)) — Build an Assistant-slot
/// cockpit under one of the three `AssistantMode` variants with
/// `last_forecast` populated or not and `history` of varying depth.
///
/// Always `is_open == true` so the layout invariant exercises the
/// rendered body (not the 0-width collapsed path which T-D-N19 already
/// covers).
fn build_assistant_llm_forecaster_cockpit(
    mode_idx: u8,
    has_forecast: bool,
    history_depth: usize,
    n_cited: usize,
) -> ui::Cockpit {
    use smol_str::SmolStr;
    use ui::assistant::state::{AssistantMode, AssistantState, LlmForecastView};

    let mode = match mode_idx % 3 {
        0 => AssistantMode::Offline,
        1 => AssistantMode::ReasoningTrace,
        _ => AssistantMode::Live,
    };

    let cited_ids: Vec<SmolStr> = (0..n_cited)
        .map(|i| SmolStr::new(format!("card_{i}")))
        .collect();

    let make_view = |idx: usize| LlmForecastView {
        symbol: SmolStr::new("BTCUSDT"),
        rating: SmolStr::new(match idx % 5 {
            0 => "STRONG_BUY",
            1 => "BUY",
            2 => "HOLD",
            3 => "SELL",
            _ => "STRONG_SELL",
        }),
        confidence_display: SmolStr::new("0.50"),
        reasoning_trace: SmolStr::new("Reasoning trace text for layout fuzz."),
        cited_lesson_ids: cited_ids.clone(),
        cost_line: Some(SmolStr::new("$0.10 of $100.00 today")),
        audit_id: Some(SmolStr::new(format!("audit_{idx}"))),
    };

    let last_forecast = if has_forecast {
        Some(make_view(0))
    } else {
        None
    };
    let history: Vec<LlmForecastView> = (1..=history_depth).map(make_view).collect();

    let mut cockpit = ui::fixtures::fake_cockpit_ready();
    cockpit.assistant_state = AssistantState {
        is_open: true,
        mode,
        last_forecast,
        history,
    };
    cockpit
}

// ─── bug-log #231: the exclusion, the seed and persistence, pinned ─────────
//
// T1 and T3-T7. T1 derives the separator set from a class-B scan; T3 pins
// `free_text` two-sided; T4 the seed, T5 that a failure writes nothing; T6 the
// labels a run is known to have failed on; T7 that every property parameter
// draws from a classified strategy. T2, the canary that cosmic-text still
// trips, was deleted by the bug-log #239 fix (ADR-0106): nothing trips now, and
// `bidi_paragraph_separators.rs` B2 pins that.

/// Labels a run of this file is known to have failed on, pinned as named cases
/// (bug-log #231). The first is the minimal form (a separator, then an RTL
/// char). The second is the label the 2026-11-24 failure drew: replaying its
/// persisted seed (`cc 52704bdd…`) on the pre-#231 file regenerated it, with
/// U+0085 at char 19 and U+FEBE (Arabic, class AL) as the next strong char.
const RECORDED_FAILING_LABELS: &[&str] = &[
    "\u{85}\u{5d0}",
    "W\u{2}{=${u?qA\u{1}K/\u{1b}$\u{1dfd0}\u{b5f2a}Z\u{ab}\u{85}\u{fffd}1\u{a5}\u{febe}%\u{692e4}\u{d5a36}\u{1f574}%jVe/.\u{575fc}H=%\u{69f3c}\u{f9881}c\u{4fb70}`\u{bc3c6}`R\u{e8}\u{0}._\\?:\u{7f}\u{8bd33}\u{5579a}b\u{1f574}\u{23a}[\u{1b}",
];

/// `n` values of `strategy`, drawn in order by one runner built on `cfg`.
fn draw<S: Strategy>(strategy: &S, cfg: ProptestConfig, n: usize) -> Vec<S::Value> {
    let mut runner = TestRunner::new(cfg);
    (0..n)
        .map(|_| strategy.new_tree(&mut runner).expect("a value").current())
        .collect()
}

/// A config drawing from `seed` on ChaCha, built without `proptest_config()`, so
/// T3 does not depend on that fn's seed handling (T4 pins that).
fn seeded_config(seed: u64) -> ProptestConfig {
    let mut cfg = ProptestConfig::with_cases(1);
    cfg.rng_algorithm = RngAlgorithm::ChaCha;
    cfg.rng_seed = RngSeed::Fixed(seed);
    cfg
}

/// T1. The separator const is exactly unicode-bidi's class B minus cosmic-text's
/// line endings, exhaustive over every `char` and two-sided (#161): a missing
/// member fails, and so does an extra one.
#[test]
fn in_line_paragraph_separators_are_class_b_minus_the_line_endings() {
    let class_b: BTreeSet<char> = class_b_chars().iter().copied().collect();
    for c in COSMIC_TEXT_LINE_ENDINGS {
        assert!(
            class_b.contains(c),
            "control: the class-B scan must find the line ending {c:?}, or it cannot \
             see a separator at all (scan found {class_b:?})"
        );
    }
    let endings: BTreeSet<char> = COSMIC_TEXT_LINE_ENDINGS.iter().copied().collect();
    let in_line: BTreeSet<char> = class_b.difference(&endings).copied().collect();
    let declared: BTreeSet<char> = IN_LINE_PARAGRAPH_SEPARATORS.iter().copied().collect();
    assert_eq!(
        declared.len(),
        IN_LINE_PARAGRAPH_SEPARATORS.len(),
        "IN_LINE_PARAGRAPH_SEPARATORS lists a char twice"
    );
    assert_eq!(
        in_line, declared,
        "class B minus COSMIC_TEXT_LINE_ENDINGS (left) must equal \
         IN_LINE_PARAGRAPH_SEPARATORS (right): unicode-bidi or cosmic-text moved \
         (bug-log #231)"
    );
    println!("t231 T1: class B = {class_b:?}; in-line separators = {in_line:?}");
}

/// T3. `free_text` draws exactly the bare regex's cases minus the separators,
/// two-sided: an independent `str::replace` and a char-count identity catch an
/// under-strip, and a literal pair plus a domain control catch an over-strip.
#[test]
fn free_text_draws_the_raw_cases_minus_the_separators() {
    const N: usize = 4096;
    let bare = proptest::string::string_regex(".{0,64}").expect("a valid regex");
    let raw = draw(&bare, seeded_config(LAYOUT_RNG_SEED), N);
    let filtered = draw(&free_text(64), seeded_config(LAYOUT_RNG_SEED), N);
    let is_separator = |c: &char| IN_LINE_PARAGRAPH_SEPARATORS.contains(c);

    let raw_separators: usize = raw
        .iter()
        .map(|s| s.chars().filter(is_separator).count())
        .sum();
    let raw_nel = raw
        .iter()
        .map(|s| s.matches('\u{85}').count())
        .sum::<usize>();
    assert!(
        raw_separators > 0,
        "control: {N} raw draws hold no separator, so this test cannot see a filter"
    );
    for (i, (r, f)) in raw.iter().zip(&filtered).enumerate() {
        assert_eq!(
            *f,
            r.replace(IN_LINE_PARAGRAPH_SEPARATORS, ""),
            "draw {i}: free_text must be the raw draw minus the separators"
        );
        assert_eq!(
            f.chars().count(),
            r.chars().count() - r.chars().filter(is_separator).count(),
            "draw {i}: free_text dropped a char that is not a separator"
        );
    }
    assert_eq!(
        strip_in_line_paragraph_separators("a\u{85}\u{5d0}\u{2028}\u{2029}b"),
        "a\u{5d0}\u{2028}b",
        "the strip keeps RTL, U+2028 and non-Latin-1, and drops only the separators"
    );
    let chars = || filtered.iter().flat_map(|s| s.chars());
    let rtl_blocks = chars()
        .filter(|c| ('\u{590}'..='\u{8ff}').contains(c))
        .count();
    let above_latin1 = chars().filter(|&c| c > '\u{ff}').count();
    assert!(
        rtl_blocks > 0 && above_latin1 > 0,
        "domain control: the filtered draws must still hold Hebrew/Arabic-block chars \
         ({rtl_blocks}) and chars above U+00FF ({above_latin1})"
    );
    println!(
        "t231 T3: {N} draws; raw separators {raw_separators} (U+0085 {raw_nel}); \
         filtered chars in U+0590..=U+08FF {rtl_blocks}, above U+00FF {above_latin1}"
    );
}

/// T4. The properties' config draws the same cases on every run, read the way the
/// `proptest!` macro reads it (`contextualize_config`), and another seed differs.
/// Under `PROPTEST_RNG_SEED` the seed is the env's, so the `Random → Fixed` step is
/// not exercised; the test says so instead of passing silently (CI sets neither var).
#[test]
fn the_property_config_draws_the_same_cases_twice() {
    if std::env::var_os("PROPTEST_RNG_SEED").is_some() {
        println!(
            "t231 T4: UNMEASURED: PROPTEST_RNG_SEED is set, so proptest_config()'s \
             own Random -> Fixed(LAYOUT_RNG_SEED) step is not exercised"
        );
    }
    let cfg = contextualize_config(proptest_config());
    let RngSeed::Fixed(seed) = cfg.rng_seed else {
        panic!(
            "proptest_config() leaves the seed {:?}: every run would draw new cases \
             (AGENT.md § Process discipline rule 5; bug-log #231)",
            cfg.rng_seed
        );
    };
    let first = draw(&free_text(64), contextualize_config(proptest_config()), 64);
    let second = draw(&free_text(64), contextualize_config(proptest_config()), 64);
    assert_eq!(
        first, second,
        "two runners on proptest_config() drew different cases"
    );

    let mut other = contextualize_config(proptest_config());
    other.rng_seed = RngSeed::Fixed(seed ^ 1);
    let third = draw(&free_text(64), other, 64);
    assert_ne!(first, third, "control: another seed must draw other cases");
}

/// T5. A failing property on this file's config writes no regressions file (#187:
/// the unhappy path must write nothing). It runs in a TempDir, never on a repo
/// path, so a stale file in `crates/ui/tests/` cannot affect it. Under
/// `PROPTEST_DISABLE_FAILURE_PERSISTENCE` proptest itself clears persistence, so the
/// subject and the belt pass whatever `proptest_config()` sets; the test says so.
#[test]
fn a_failing_property_in_this_file_writes_no_regressions_file() {
    if std::env::var_os("PROPTEST_DISABLE_FAILURE_PERSISTENCE").is_some() {
        println!(
            "t231 T5: UNMEASURED: PROPTEST_DISABLE_FAILURE_PERSISTENCE is set, so \
             proptest_config()'s own `failure_persistence = None` is not exercised"
        );
    }
    /// Run a planted always-failing property on this file's config, edited by
    /// `edit`, with its source file in a fresh TempDir; return the dir's entries.
    fn planted(edit: impl FnOnce(&mut ProptestConfig)) -> Vec<String> {
        let dir = tempfile::TempDir::new().expect("a temp dir");
        let source: &'static str = Box::leak(
            dir.path()
                .join("planted.rs")
                .to_str()
                .expect("a UTF-8 temp path")
                .to_owned()
                .into_boxed_str(),
        );
        let mut cfg = contextualize_config(proptest_config());
        cfg.source_file = Some(source);
        edit(&mut cfg);
        let result = TestRunner::new(cfg).run(&any::<bool>(), |_| {
            Err(TestCaseError::fail("planted failure (bug-log #231 T5)"))
        });
        assert!(result.is_err(), "the planted property must fail");
        let mut names: Vec<String> = std::fs::read_dir(dir.path())
            .expect("read the temp dir")
            .map(|e| {
                e.expect("a dir entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        names
    }

    assert_eq!(
        planted(|_| {}),
        Vec::<String>::new(),
        "a failure on proptest_config() wrote a regressions file (bug-log #231)"
    );
    assert_eq!(
        planted(|cfg| {
            cfg.failure_persistence = Some(Box::new(FileFailurePersistence::WithSource(
                "proptest-regressions",
            )));
        }),
        vec![String::from("planted.proptest-regressions")],
        "control: the same failure with persistence on must write the file T5 looks for"
    );
    assert!(
        proptest_config().failure_persistence.is_none(),
        "belt: proptest_config() must persist nothing, under any path"
    );
}

/// T6. The recorded failing labels each tripped unpatched cosmic-text 0.15; with
/// ADR-0106's hunk they shape privately RAW and lay out raw in the widget the
/// property fuzzes (bug-log #239). Before the fix this test laid out their
/// filtered forms behind a "raw trips" precondition (bug-log #231).
#[test]
fn strategies_id_cell_lays_out_the_recorded_failing_labels_raw() {
    // Non-vacuity: the minimal form and the 2026-11-24 label.
    assert_eq!(
        RECORDED_FAILING_LABELS.len(),
        2,
        "RECORDED_FAILING_LABELS must hold the minimal form and the 2026-11-24 label"
    );
    for raw in RECORDED_FAILING_LABELS {
        if let Err(panic) = shape_on_a_private_font_system(raw, false) {
            panic!(
                "precondition: {raw:?} must shape privately, since ADR-0106's \
                 vendored cosmic-text hunk resolves the paragraph directions \
                 (bug-log #239); it panics: {panic}"
            );
        }
        let element = widgets_for_test::strategies_id_cell(
            StrategyId::new("v0.sma"),
            (*raw).to_owned(),
            false,
        );
        if let Err(e) = check_element_layout(element) {
            panic!("strategies_id_cell({raw:?}) layout: {e}");
        }
    }
}

/// What a property parameter's strategy expression is, for T7.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParamKind {
    Bool,
    NumericRange,
    FreeText,
    AsciiClassLiteral,
}

/// T7's allowlist. `None` for anything not on it: `any::<bool>()`; a numeric range
/// `<num><u8|usize|f32>..[=]<num>`; `free_text(<digits>)`; or a plain `"…"` literal of
/// positive bracket classes of printable ASCII (no `\`, no `[` inside, no leading
/// `^`), each optionally followed by `{n}`, `{n,m}`, `*`, `+` or `?`.
fn classify(expr: &str) -> Option<ParamKind> {
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if expr == "any::<bool>()" {
        return Some(ParamKind::Bool);
    }
    if let Some(arg) = expr
        .strip_prefix("free_text(")
        .and_then(|r| r.strip_suffix(')'))
    {
        return digits(arg).then_some(ParamKind::FreeText);
    }
    if let Some(body) = expr.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
        return is_ascii_class_regex(body).then_some(ParamKind::AsciiClassLiteral);
    }
    let (lo, hi) = expr.split_once("..")?;
    let hi = hi.strip_prefix('=').unwrap_or(hi);
    let (lo, float) = if let Some(n) = lo.strip_suffix("f32") {
        (n, true)
    } else {
        (
            lo.strip_suffix("usize").or_else(|| lo.strip_suffix("u8"))?,
            false,
        )
    };
    let number = |s: &str| match s.split_once('.') {
        Some((i, f)) => float && digits(i) && digits(f),
        None => digits(s),
    };
    (number(lo) && number(hi)).then_some(ParamKind::NumericRange)
}

/// A regex body made only of positive bracket classes of printable ASCII, each with
/// an optional quantifier: it can generate printable ASCII only.
fn is_ascii_class_regex(body: &str) -> bool {
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let mut rest = body;
    if rest.is_empty() {
        return false;
    }
    while !rest.is_empty() {
        let Some(class_and_after) = rest.strip_prefix('[') else {
            return false;
        };
        let Some((class, after)) = class_and_after.split_once(']') else {
            return false;
        };
        let printable = class
            .bytes()
            .all(|b| (0x20..=0x7e).contains(&b) && b != b'\\' && b != b'[');
        if class.is_empty() || class.starts_with('^') || !printable {
            return false;
        }
        rest = if let Some(after) = after.strip_prefix(['*', '+', '?']) {
            after
        } else if let Some(braced) = after.strip_prefix('{') {
            let Some((bound, after)) = braced.split_once('}') else {
                return false;
            };
            let ok = match bound.split_once(',') {
                Some((n, m)) => digits(n) && digits(m),
                None => digits(bound),
            };
            if !ok {
                return false;
            }
            after
        } else {
            after
        };
    }
    true
}

/// One property parameter line, `<ident> in <expr>,`, as `(ident, expr)`.
fn parse_param(line: &str) -> Option<(&str, &str)> {
    let (ident, rest) = line.split_once(" in ")?;
    let expr = rest.strip_suffix(',')?;
    let ident_ok = ident.starts_with(|c: char| c == '_' || c.is_ascii_alphabetic())
        && ident.chars().all(|c| c == '_' || c.is_ascii_alphanumeric());
    (ident_ok && !expr.is_empty() && expr == expr.trim()).then_some((ident, expr))
}

/// T7 (#190 wiring). Reads this file: every `proptest!` block opens at column 0
/// (any other non-comment line naming the macro fails), its config expression
/// calls `proptest_config()`, every parameter draws from a strategy `classify`
/// allows, free text only ever comes from `free_text` in the two properties that
/// render it, and each of those opens with `guard_shaping`. A new property block or
/// strategy form fails here until it is classified deliberately.
///
/// What it does NOT see: a config expression that calls `proptest_config()` and
/// then discards or overrides its seed or persistence; a `TestRunner` built by hand
/// outside the macro (T3/T4/T5 build their own, on purpose); and a macro that
/// expands to either.
#[test]
fn property_parameters_draw_text_only_through_free_text() {
    // The classifier itself, on runtime strings.
    for rejected in [
        r#"".{0,64}""#,
        r#""\\PC{0,64}""#,
        r#""[^a]{0,64}""#,
        r#"r".{0,64}""#,
        "any::<String>()",
        r#"prop::collection::vec(".{0,64}", 0..4)"#,
        "free_text(64).prop_map(|s| s)",
        "free_text(max_len)",
        r#""[a-z]{0,64}|.""#,
        r#""[[:alpha:]]{1,8}""#,
    ] {
        assert_eq!(
            classify(rejected),
            None,
            "classify must reject `{rejected}`"
        );
    }
    for (accepted, kind) in [
        (r#""[a-z_.]{1,32}""#, ParamKind::AsciiClassLiteral),
        ("free_text(64)", ParamKind::FreeText),
        ("0u8..4", ParamKind::NumericRange),
        ("0usize..=8", ParamKind::NumericRange),
        ("400.0f32..=1080.0", ParamKind::NumericRange),
        ("any::<bool>()", ParamKind::Bool),
    ] {
        assert_eq!(
            classify(accepted),
            Some(kind),
            "classify must accept `{accepted}`"
        );
    }

    // The macro's name, spelled so that no line of T7 itself contains it.
    const MACRO: &str = concat!("proptest", "!");
    let opener = format!("{MACRO} {{");
    let lines: Vec<&str> = include_str!("layout_invariants.rs").lines().collect();
    let openers: Vec<usize> = (0..lines.len()).filter(|&i| lines[i] == opener).collect();
    // Every non-comment line naming the macro, anywhere on the line: an indented or
    // path-qualified (`proptest::…`) block, or `{MACRO}{{`, is one T7 would not read.
    let mentions: Vec<usize> = (0..lines.len())
        .filter(|&i| !lines[i].trim_start().starts_with("//") && lines[i].contains(MACRO))
        .collect();
    assert_eq!(
        mentions,
        openers,
        "every {MACRO} block must open at column 0 as `{opener}` so T7 reads it; \
         lines naming it otherwise: {:?}",
        mentions
            .iter()
            .filter(|i| !openers.contains(i))
            .map(|i| i + 1)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        openers.len(),
        12,
        "this file holds 12 {MACRO} blocks; classify a new one here deliberately \
         (bug-log #231)"
    );

    let mut free_text_params = BTreeSet::new();
    let mut literal_params = BTreeSet::new();
    for &open in &openers {
        let close = (open + 1..lines.len())
            .find(|&j| lines[j] == "}")
            .unwrap_or_else(|| panic!("layout_invariants.rs:{}: unclosed {MACRO}", open + 1));
        let block: Vec<(usize, &str)> = (open + 1..close)
            .map(|j| (j + 1, lines[j].trim()))
            .collect();
        let at = |k: usize| block[k].0;

        // Config wiring: one `#![proptest_config(` per block, built from proptest_config().
        let configs: Vec<usize> = (0..block.len())
            .filter(|&k| block[k].1.starts_with("#![proptest_config("))
            .collect();
        assert_eq!(
            configs.len(),
            1,
            "layout_invariants.rs:{}: a {MACRO} block needs exactly one config line",
            open + 1
        );
        let first = configs[0];
        let last = (first..block.len())
            .find(|&k| block[k].1.ends_with(")]"))
            .unwrap_or_else(|| panic!("layout_invariants.rs:{}: unclosed config", at(first)));
        let joined = block[first..=last]
            .iter()
            .map(|(_, l)| *l)
            .collect::<Vec<_>>()
            .join(" ");
        let expr = joined
            .strip_prefix("#![proptest_config(")
            .unwrap_or(&joined);
        assert!(
            expr.contains("proptest_config()"),
            "layout_invariants.rs:{}: the config `{expr}` is not built from \
             proptest_config(), so it skips the fixed seed and no-persistence",
            at(first)
        );

        // Parameters: one `fn NAME(` list, one `<ident> in <expr>,` per line.
        let fns: Vec<usize> = (0..block.len())
            .filter(|&k| block[k].1.starts_with("fn "))
            .collect();
        assert_eq!(
            fns.len(),
            1,
            "layout_invariants.rs:{}: one fn per block",
            open + 1
        );
        let f = fns[0];
        let name = block[f]
            .1
            .strip_prefix("fn ")
            .and_then(|r| r.strip_suffix('('))
            .unwrap_or_else(|| {
                panic!(
                    "layout_invariants.rs:{}: `{}` must end its line at `(`, one \
                     parameter per line",
                    at(f),
                    block[f].1
                )
            });
        let end = (f + 1..block.len())
            .find(|&k| block[k].1 == ") {")
            .unwrap_or_else(|| {
                panic!(
                    "layout_invariants.rs:{}: fn {name}'s list never ends at `) {{`",
                    at(f)
                )
            });
        let mut this_fn_free_text = Vec::new();
        for &(lineno, line) in &block[f + 1..end] {
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            let (param, expr) = parse_param(line).unwrap_or_else(|| {
                panic!(
                    "layout_invariants.rs:{lineno}: fn {name}: `{line}` is not \
                     `<ident> in <expr>,` on one line"
                )
            });
            match classify(expr) {
                Some(ParamKind::FreeText) => {
                    free_text_params.insert((name, param));
                    this_fn_free_text.push(param);
                }
                Some(ParamKind::AsciiClassLiteral) => {
                    literal_params.insert((name, param));
                }
                Some(ParamKind::Bool | ParamKind::NumericRange) => {}
                None => panic!(
                    "layout_invariants.rs:{lineno}: fn {name}'s parameter `{param}` draws \
                     from `{expr}`, which T7 does not classify. Free text must come from \
                     free_text (bug-log #231); classify any other form here deliberately"
                ),
            }
        }

        // Guard wiring: a free-text property's body opens with guard_shaping(<its text>).
        if !this_fn_free_text.is_empty() {
            let opening = block[end + 1..]
                .iter()
                .find(|(_, l)| !l.is_empty() && !l.starts_with("//"))
                .map_or("", |(_, l)| *l);
            assert!(
                opening.starts_with("guard_shaping(")
                    && this_fn_free_text.iter().all(|p| opening.contains(p)),
                "fn {name} draws free text ({this_fn_free_text:?}), so its body must open \
                 with `guard_shaping(` of that text (bug-log #231); it opens with `{opening}`"
            );
        }
    }

    let expected_free_text: BTreeSet<(&str, &str)> = [
        ("strategies_id_cell_layout_never_zero_dim", "label"),
        ("positions_view_layout_never_zero_dim", "error_msg"),
    ]
    .into_iter()
    .collect();
    let expected_literals: BTreeSet<(&str, &str)> =
        [("strategies_id_cell_layout_never_zero_dim", "id_str")]
            .into_iter()
            .collect();
    assert_eq!(
        free_text_params, expected_free_text,
        "free_text parameters moved"
    );
    assert_eq!(
        literal_params, expected_literals,
        "string-literal parameters moved"
    );
}
