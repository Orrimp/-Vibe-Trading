//! Panel widgets.
//!
//! One module per cockpit panel. Each module exposes a single `view`
//! function that takes a `&Cockpit` (and sometimes the panel's own sub-state)
//! and returns an `iced::Element<Message>`. Business logic never lives here.
//!
//! Consistency contract (enforced in tests `tests/no_inline_strings.rs` and
//! `tests/no_inline_hex.rs`):
//! - No string literals — use `crate::strings::*`.
//! - No hex colors or magic-number `Length::Units(N)` — use `crate::theme::*`.

pub mod agent_feed;
pub(crate) mod canvas_chart;
pub mod chart;
pub mod chart_legend;
pub mod chart_tooltip;
// ui-quality-gate-overhaul M2-B (T-M2-B-1/-2) — diagnostic
// `DebugRenderer` newtype gated behind the `render-debug` feature.
// Default builds compile this module away entirely (the file's
// `#![cfg(feature = "render-debug")]` floor-gate guarantees zero
// production surface). See `widgets/debug_renderer.rs` for the
// design + the architect Q3 build-time-only lifecycle.
#[cfg(feature = "render-debug")]
pub mod debug_renderer;
pub mod drawdown_band;
pub mod equity_curve;
pub mod focus_ring;
pub mod frame;
pub mod human_control;
pub mod journal_transaction_modal;
pub mod kill;
pub mod kpi_strip;
pub mod latency;
pub mod num;
pub mod override_risk_veto;
pub mod pnl;
pub mod positions;
pub mod sidebar_nav;
pub mod sparkline;
pub mod status_bar;
pub mod strategies;
// cockpit-performance-and-input-responsiveness M1 Candidate A — local
// 10 fps wrap of `iced_aw::Spinner`. Used by `frame::loading_with_spinner`;
// see `widgets/throttled_spinner.rs` for context.
/// Phase A (T-D-7) — date-range picker widget for the Lab top-bar.
/// Preset chips + inline Custom editor with parse-error highlight (R5.1).
pub mod date_range;
/// Phase A (T-D-5) — pair chip widget for the Lab top-bar pair-chip row.
/// Renders a `(Venue, Symbol)` as a Lumen chip; dispatches
/// `Message::LabSelectPair` on press.
pub mod pair_chip;
/// Phase A (T-D-3) — empty-state placeholder card for routes not yet
/// implemented. Used by `shell::screen_body` for Compare / Memory /
/// Models / Trail / Settings.
pub mod placeholder;
/// Phase A (T-D-14b) — Run backtest button widget for the Lab screen.
/// Big primary button per Lumen Phase 1 tokens; disabled while a run is
/// in-flight (at-most-one-in-flight per Design § 4).
pub mod run_button;
/// Phase A (T-D-6) — strategy chip widget for the Lab strategy-chip row.
/// Renders a strategy id + family badge; two emit paths (primary select +
/// compare toggle).
pub mod strategy_chip;
pub mod throttled_spinner;
/// cockpit-training-control T-D-N2 — training log ring-buffer widget.
/// 200-entry VecDeque<SmolStr> with auto-scroll + click-to-freeze.
pub mod training_log;
pub mod volume_histogram;
