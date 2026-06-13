//! Phase E — Compare-matrix feature module (ui-rethink-phase-e-compare).
//!
//! Houses all Compare-screen-specific logic that is not a widget or a
//! screen view: state shape, cold-boot report-cache scanner, and the
//! `OpenLabFromCompare` compound-dispatch arm.
//!
//! Module layout:
//!
//! ```text
//! compare/
//! ├── mod.rs    — this file, re-exports
//! ├── state.rs  — CompareScreenState + CachedCell + CompareKpiAxis (T-D-N1)
//! └── cache.rs  — scan_spec_tree + lookup_cell + parse_frontmatter (T-D-N2)
//! ```

pub mod cache;
pub mod state;

pub use state::{CachedCell, CompareKpiAxis, CompareScreenState, OverlaySlot};
