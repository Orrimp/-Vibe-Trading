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

/// Live broadcast-bus subscription (T32). Gated behind the `live` feature
/// so `cargo build -p ui` stays fast and iced remains the only required
/// heavy dep. See `live.rs` for the channel list and handoff contract.
#[cfg(feature = "live")]
pub mod live;

pub use state::{
    update, AgentMode, ChartBuffer, Cockpit, KillState, Latency, MarketHealthState, Message,
    PanelState, Screen, CHART_BUFFER_CAPACITY,
};

/// Crate-wide convenience: the iced `Element` type specialized to our
/// [`Message`]. Avoids repeating `iced::Element<'_, Message>` at every
/// widget boundary.
pub type Element<'a> = iced::Element<'a, Message>;
