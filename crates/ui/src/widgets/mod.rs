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

/// cockpit-activity-status-bar v0.1.0 Wave B (T-D-N6) — activity tape region.
/// Renders in-flight + failed activities to the left of the server-time field.
pub mod activity_tape;
pub mod agent_feed;
/// cockpit-training-control T-D-N17 — shared axis-rendering helpers.
/// Tick spacing, label formatting, and coordinate mapping for use by
/// `widgets::training_plot` and potentially `widgets::chart`.
pub(crate) mod axis;
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
/// lab-yahoo-realdata Wave D-followup (T-D2) — Cache-state badge widget.
/// Three-state pill (Fresh / Stale / Empty) for the active Yahoo ticker.
/// Only rendered when `data_source == YahooCache`.
pub mod cache_state_badge;
/// lab-yahoo-realdata T-C3.3 / T-AR4 / R-UI-1.4 — Cadence badge widget.
/// Small chip showing the adaptive bar cadence ("1m" / "1h" / "1d").
/// Only rendered when `data_source == YahooCache`.
pub mod cadence_badge;
/// Phase A (T-D-7) — date-range picker widget for the Lab top-bar.
/// Preset chips + inline Custom editor with parse-error highlight (R5.1).
pub mod date_range;
/// Phase E — Compare matrix widget (ui-rethink-phase-e-compare R2.1-R2.6).
/// Strategies-as-rows × pairs-as-columns; populated / empty / blanked cells.
pub mod matrix;
/// Phase A (T-D-5) — pair chip widget for the Lab top-bar pair-chip row.
/// Renders a `(Venue, Symbol)` as a Lumen chip; dispatches
/// `Message::LabSelectPair` on press.
pub mod pair_chip;
/// Phase A (T-D-3) — empty-state placeholder card for routes not yet
/// implemented. Used by `shell::screen_body` for Compare / Memory /
/// Models / Trail / Settings.
pub mod placeholder;
/// lab-polish-round-2 R1 — position-curve stepped-polyline widget for the Lab
/// screen. Shows base-asset position quantity over time for the active pair.
/// Mirrors `volume_histogram` structure. Lumen-token'd, anchor-additive.
pub mod position_curve;
/// lab-end-to-end-v2 Wave D-4 T-AR-6 / R8 — Determinate progress bar
/// for the Lab run flow. Lumen-token'd 8px bar with optional label overlay.
/// Renders indeterminate (30% sentinel) when `progress == None`.
pub mod progress_bar;
/// Phase A (T-D-14b) — Run backtest button widget for the Lab screen.
/// Big primary button per Lumen Phase 1 tokens; disabled while a run is
/// in-flight (at-most-one-in-flight per Design § 4).
pub mod run_button;
/// Phase B (T-D-N13) — Run delta badge showing signed Δ(P&L / MaxDD / SR)
/// between the last two completed runs on the same (strategy, pair, range) tuple.
/// Rendered to the right of the Run button in `screens/lab.rs::run_button_row`
/// when both `lab_state.last_run_report` and `lab_state.prev_run_report` are Some
/// and share the same tuple.
pub mod run_delta_badge;
/// Phase C — settings tab-strip widget (ui-rethink-phase-c-sidebar-ia T-D-N19).
/// Three-tab chrome strip: Risk · Control · Debug, with T1609 active-chip
/// bottom-edge accent on the active tab.
pub mod settings_tabs;
/// lab-yahoo-realdata T-C3.2 / R-UI-1.1 — Source toggle widget.
/// Two-state chip toggle between Synthetic GBM and YahooCache real data.
/// Dispatches `Message::LabSelectDataSource(LabDataSource)` on chip press.
pub mod source_toggle;
/// Phase C — strategy registry card widget (ui-rethink-phase-c-sidebar-ia T-D-N15).
/// One card per registered strategy in the registry list view (R6.1).
pub mod strategy_card;
/// Phase A (T-D-6) — strategy chip widget for the Lab strategy-chip row.
/// Renders a strategy id + family badge; two emit paths (primary select +
/// compare toggle).
pub mod strategy_chip;
pub mod throttled_spinner;
/// Phase D — Trail side-drawer widget (ui-rethink-phase-d-trail R4.1-R4.4).
/// Renders Fill / Signal / Forecast / LLM-placeholder drawer bodies.
pub mod trail_drawer;
/// Phase D — Trail node widget (ui-rethink-phase-d-trail R3.1-R3.5).
/// One node per pipeline stage; vertical stack; chevron button emits
/// `Message::TrailNodeChevronClicked(TrailNodeKind)`.
pub mod trail_node;
/// cockpit-training-control T-D-N2 — training log ring-buffer widget.
/// 200-entry VecDeque<SmolStr> with auto-scroll + click-to-freeze.
pub mod training_log;
/// cockpit-training-control T-D-N12 — loss-curve plot inside Train panel.
/// Renders (epoch, train_loss, val_loss) series as two lines.
pub mod training_plot;
pub mod volume_histogram;
