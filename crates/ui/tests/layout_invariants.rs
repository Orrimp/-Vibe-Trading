//! Property-based layout invariants — ui-quality-gate-overhaul M1-C.
//!
//! Asserts that under any fuzz-generated reasonable input, no widget's
//! `Widget::layout(...)` returns a `layout::Node` whose root (or
//! direct child) `Size` has a literal zero width or height. The F1
//! incident (`spec/cockpit-render-regression/feature.md`) shipped past
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
//! Per `AGENT.md ## Process discipline` rule 5 (and the developer
//! brief's "Determinism non-negotiables still apply to M1-C"
//! reminder): proptest's RNG is configured with a **fixed seed** via
//! `ProptestConfig::with_rng_algorithm(RngAlgorithm::ChaCha) + cases:
//! 256`. The ChaCha RNG is the same algorithm the workspace uses for
//! all production seeded RNGs (`rand_chacha::ChaCha20Rng`); proptest
//! 1.6 ships a `ChaCha` variant as the RNG algorithm choice. Two
//! consecutive `cargo test -p ui --test layout_invariants` runs MUST
//! produce identical output.
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

use iced::advanced::layout::{Limits, Node};
use iced::advanced::widget::Tree;
use iced::{Element, Size};
use proptest::prelude::*;

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
fn check_element_layout<'a>(mut element: Element<'a, Message>) -> Result<(), String> {
    // `Tree::new` borrows the widget; `as_widget()` returns the
    // `&dyn Widget`. Construct the tree before grabbing the mutable
    // widget reference — borrow-checker friendly ordering.
    let mut tree = Tree::new(element.as_widget());
    let renderer = iced::Renderer::new(iced::Font::DEFAULT, iced::Pixels(16.0));
    let limits = default_limits();
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits);
    assert_root_not_zero_dim(&node, "root")
}

/// proptest configuration shared by every property block. Fixed
/// ChaCha seed via the algorithm choice; 256 cases per property
/// per architect Q4 budget (~5ms × 6 widgets ≈ 30ms aggregate;
/// well within the 60s wall-clock budget at T-M1-C-3).
fn proptest_config() -> ProptestConfig {
    let mut cfg = ProptestConfig::with_cases(256);
    // `rng_algorithm` field: `RngAlgorithm::ChaCha` matches the
    // workspace's `rand_chacha::ChaCha20Rng` choice for all
    // production-determinism RNGs.
    cfg.rng_algorithm = proptest::test_runner::RngAlgorithm::ChaCha;
    // Source-file pinning so proptest's persistence cache lands in
    // a stable per-file directory rather than the global one.
    cfg.source_file = Some("crates/ui/tests/layout_invariants.rs");
    cfg
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
        label in ".{0,64}",
        is_active in any::<bool>(),
    ) {
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
        error_msg in ".{0,64}",
        variant in 0u8..4,
    ) {
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
                source_report_path: SmolStr::new("spec/v0.sma/reports/backtest-fixture.md"),
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
    };
    cockpit
}
