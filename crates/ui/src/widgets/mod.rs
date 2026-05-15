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
pub mod throttled_spinner;
pub mod volume_histogram;
