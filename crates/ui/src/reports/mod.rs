//! cockpit-reports-viewer v0.1.0 — browse + render committed backtest
//! reports inside the cockpit shell.
//!
//! Pure-`ui` over `trading_core` + `reports` (the report-writer crate) +
//! `std::fs` — **no new crate edge** (AC7). A navigable `Screen::Reports`
//! list-detail screen: a left picker over the discovered
//! `evidence/*/reports/backtest-*.md` corpus + a right detail pane that renders
//! the selected report via the existing `kpi_strip` / `equity_curve` /
//! `drawdown_band` widgets verbatim, plus the markdown body.
//!
//! Module layout (sibling of `baseline/`):
//!
//! ```text
//! reports/
//! ├── mod.rs          — this file, re-exports
//! ├── loader.rs       — load_report + discover_reports + parse/strip front-matter
//! │                     (D2 lift from bin/viewer.rs — the ONE shared parse, AC5)
//! ├── body_render.rs  — markdown heading pre-pass (D2 lift from bin/viewer.rs)
//! └── state.rs        — ReportsScreenState + ReportEntry + boot-load helper (D1)
//! ```
//!
//! The screen body lives at `crate::screens::reports`; the sidebar IA +
//! `Screen::Reports` routing live in `crate::theme` / `crate::state` /
//! `crate::shell`.
//!
//! NOTE on naming: the module path `crate::reports` (this UI feature module)
//! is distinct from the `reports` **crate** (`trading`'s report-writer). The
//! `use reports::parse` extern-crate import in `loader.rs` and this
//! `crate::reports` module coexist unambiguously — Rust resolves
//! `crate::reports` to the local module and bare `reports::` to the extern
//! crate, exactly as `bin/viewer.rs` already does.

pub mod body_render;
pub mod loader;
pub mod state;

pub use state::{ReportEntry, ReportsScreenState, load_into};
