//! advisor-handoff-export v0.1.0 (remediation-plan P5, ADR-0088) — the
//! SUGGEST → manual hand-off export.
//!
//! Journey terminus of the single-coin investment-advisor (product §
//! journey: pick coin + budget → bake off → rank & pick → plan → **export**
//! → watch paper-trade). This module houses the pure, deterministic,
//! offline serialiser that turns a crowned plan into a portable markdown
//! checklist a reader can take away from the cockpit — with its full
//! honesty context (the credibility verdict, the survivorship caveat, the
//! short unbounded-loss disclaimer) attached in-body.
//!
//! ## The layering discipline (INVARIANT — ADR-0088 § D1)
//!
//! `ui` must NOT import `strategy` / `exec` / `models` / `llm`. The
//! serialiser is a **pure function over pure-`ui` mirror types only**
//! ([`crate::forward_plan::ForwardPlanView`],
//! [`crate::leaderboard::BakeoffReportMirror`],
//! [`crate::leaderboard::NarrationState`], `trading_core::FxNote`) — it
//! reads no new engine type, so `cargo tree -p ui` is unchanged.
//!
//! Module layout:
//!
//! ```text
//! export/
//! ├── mod.rs          — this file, re-exports
//! └── plan_export.rs  — serialize_plan_export + export_filename (the ONLY
//!                       code in this module) + inline golden/determinism
//!                       tests
//! ```
//!
//! **No file I/O here.** `serialize_plan_export` returns a `String`;
//! `export_filename` returns a `String`. The single `std::fs::write` leaf
//! + the `Message::ExportPlan` handler + the "Export this plan" trigger
//! button live at the ui-designer's seam (`Screen::ForwardPlan`, T6) — this
//! module never touches the filesystem or a wall-clock.

pub mod plan_export;

pub use plan_export::{export_filename, serialize_plan_export};
