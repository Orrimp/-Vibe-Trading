//! UI crate — iced desktop app: ops cockpit and backtest viewer.
//!
//! **v0 scope:** `cockpit` binary only. `viewer` is deferred to v0.5.
//!
//! Dependencies are deliberately narrow: `core` (types) and `audit`
//! (read-only ledger queries via `audit::query`). **Never** `strategy`,
//! `exec`, `models`, or `llm`. This is enforced by the architect — see
//! [`architecture.md`][0] — so the cockpit is swappable without touching
//! trading logic.
//!
//! [0]: ../../spec/architecture.md
//!
//! ### Design-system contract
//!
//! - All user-visible copy lives in [`strings`] (no inline literals).
//! - All colors / spacing / font sizes flow from [`theme`] (no inline hex).
//! - Every panel has explicit loading / empty / error / ready states via
//!   [`state::PanelState`] — no blank screens.
//! - Destructive actions (kill switch) go through a typed-phrase confirm
//!   dialog. See [`widgets::kill`].

// Lint policy: deny unwraps at the crate boundary.
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

/// Phase E — Compare-matrix feature module (ui-rethink-phase-e-compare).
/// Houses `compare::state` (CompareScreenState / CachedCell / CompareKpiAxis)
/// and `compare::cache` (scan_spec_tree + lookup_cell + parse_frontmatter).
pub mod compare;

/// Phase F — Memory-screen feature module (ui-rethink-phase-f-memory-models-assistant).
/// Houses `memory::state` (MemoryScreenState / LessonCardCard / MemoryViewMode /
/// MemoryFilter) and `memory::drawer` (Memory entry side-drawer widget).
pub mod memory;

/// Phase F — Models-screen feature module (ui-rethink-phase-f-memory-models-assistant).
/// Houses `models::state` (ModelsScreenState / CheckpointMeta / ModelFamily / ModelStatus)
/// and `models::registry_read` (discover_checkpoints + CheckpointMetadata serde structs).
pub mod models;

/// Phase F — Assistant-slot feature module (Lumen Phase 6 wake, Q4=(a) stub-only).
/// Houses `assistant::state` (AssistantState / AssistantMode) and
/// `assistant::view` (right-rail slot view fn).
pub mod assistant;

/// cockpit-baseline-panel v0.1.0 — passive buy-and-hold baseline feature.
/// Houses `baseline::loader` (CSV → EquitySeries + embedded §7.1 metrics
/// const) and `baseline::state` (BaselineScreenState + boot-load helper).
/// Pure-`ui` over `core` + `std::fs`; no new crate edge (AC7). The screen
/// body lives at `screens::baseline`.
pub mod baseline;
pub mod lab;
/// cockpit-reports-viewer v0.1.0 — browse + render committed backtest
/// reports in-cockpit. Houses `reports::loader` (the shared report-load
/// parse, lifted from `bin/viewer.rs` so the viewer bin + the Reports
/// screen call ONE implementation — D2/AC5), `reports::body_render` (the
/// markdown heading pre-pass), and `reports::state` (ReportsScreenState +
/// ReportEntry + boot-load helper). Pure-`ui` over `core` + `reports` +
/// `std::fs`; no new crate edge (AC7). The screen body lives at
/// `screens::reports`. (Distinct from the `reports` extern crate — see
/// `reports/mod.rs`.)
pub mod reports;
pub mod screens;
pub mod shell;
pub mod state;
pub mod strings;
pub mod theme;
pub mod viewer;
pub mod widgets;
pub mod window_icon;

// Fixtures module is always compiled so unit + integration snapshot tests
// can access deterministic generators without `--features fixtures`. The
// binary still only pulls fixtures into the default state under
// `#[cfg(feature = "fixtures")]` — see `bin/cockpit.rs`.
pub mod fixtures;

// ui-test-harness-bootstrap v0.1 (T4012) — test-only cockpit factory
// for `iced_test::screenshot`-driven visual snapshots. Always-compiled
// for the same reason as `fixtures` (integration tests can only see
// `pub` items); the production builds incur a one-function compile
// cost. See `crates/ui/src/test_support.rs` for the Q1+Q6 resolution
// rationale.
pub mod test_support;

// ui-gallery-bin v0.1 — widget gallery module. Always-compiled
// (matches the `fixtures` + `test_support` pattern above) so
// integration tests under `crates/ui/tests/gallery_*.rs` can import
// it without `--features fixtures` — `cfg(test)` in the lib does NOT
// propagate to integration-test crates. The bin
// (`crates/ui/src/bin/ui_gallery.rs`) still requires `--features
// fixtures` via `Cargo.toml [[bin]] required-features`.
// Q-GALLERY-SCOPE: imports `crate::fixtures::*` directly; no local
// state builders inside `gallery/`. See spec/ui-gallery-bin/feature.md.
pub mod gallery;

/// Live broadcast-bus subscription (T32). Gated behind the `live` feature
/// so `cargo build -p ui` stays fast and iced remains the only required
/// heavy dep. See `live.rs` for the channel list and handoff contract.
#[cfg(feature = "live")]
pub mod live;

pub use state::{
    AgentMode, CHART_BUFFER_CAPACITY, ChartBuffer, Cockpit, KillState, LAB_PAIR_ORDER, Latency,
    MarketHealthState, Message, PanelState, Screen, update,
};

/// Crate-wide convenience: the iced `Element` type specialized to our
/// [`Message`]. Avoids repeating `iced::Element<'_, Message>` at every
/// widget boundary.
pub type Element<'a> = iced::Element<'a, Message>;

/// Integration-test helper: force all chart time-zone formatting to UTC
/// for the lifetime of this process.
///
/// Call this once from any integration-test binary that renders chart
/// widgets (instead of `unsafe { std::env::set_var(CHART_FORCE_UTC_ENV, "1") }`).
/// The underlying mechanism is a `SeqCst` `AtomicBool` — no env-var
/// data race, safe to call from parallel test threads.
///
/// See `crates/ui/src/widgets/chart.rs::force_chart_utc_for_tests` for
/// the full motivation comment.
pub use widgets::chart::force_chart_utc_for_tests;
