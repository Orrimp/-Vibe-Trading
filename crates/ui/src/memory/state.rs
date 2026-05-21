//! Phase F — Memory-screen per-session state.
//!
//! Sibling of `crates/ui/src/compare/state.rs` (Phase E) and
//! `crates/ui/src/state.rs::TrailScreenState` (Phase D). All fields
//! are session-scoped; no on-disk persistence at v0.1.0
//! (cold-boot-only per R5.3).

use smol_str::SmolStr;

/// Phase F — Memory screen view mode (R1.2).
///
/// `Cards` is the default per Q1=(a) (reverse-chronological list).
/// `Cluster` is reserved for v0.2.0 (reflection-memory-distillation
/// brief) and renders as a disabled toolbar toggle with tooltip
/// `MEMORY_CLUSTER_MODE_DISABLED_TOOLTIP`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MemoryViewMode {
    #[default]
    Cards,
    /// Reserved v0.2.0 — disabled at v0.1.0 per R1.2 bullet 3.
    Cluster,
}

/// Phase F — Memory entry filter (R1.2 toolbar).
///
/// Applied to the card list to narrow by strategy or symbol.
/// `None` = no filter (show all). At v0.1.0, filters are wired
/// in the UI; clearing is always possible (chip deselect).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryFilter {
    /// Narrow by strategy id.
    ByStrategy(SmolStr),
    /// Narrow by symbol or pair display string.
    BySymbol(SmolStr),
}

/// Phase F — UI view-model for one memory card (R8.3).
///
/// Distinct from `reflection::LessonCard` to avoid leaking the
/// reflection-crate type into the UI layer. Populated by
/// `cockpit_live` at the async/sync boundary from the
/// `Message::MemoryHydrate` arm.
#[derive(Debug, Clone)]
pub struct LessonCardCard {
    /// Unique card identifier (from `LessonCard::card_id`).
    pub card_id: SmolStr,
    /// Display string for the trade context line (e.g. `"BTCUSDT"`).
    pub symbol_or_pair: SmolStr,
    /// ISO-8601 close timestamp string (for rendering).
    pub closed_at: SmolStr,
    /// Strategy id string (for filter + display).
    pub strategy_id: SmolStr,
    /// Signed P&L formatted for display (e.g. `"+12.50 USDT"`).
    pub signed_pnl_display: SmolStr,
    /// Outcome class label: `"Win"` / `"Loss"` / `"Scratch"`.
    pub outcome_class: SmolStr,
    /// Note / lesson body text. At v0.1.0 this is the deterministic
    /// note from the reflection writer (no LLM enrichment). Renders
    /// as plain text (R1.2 markdown-deferred).
    pub note: Option<SmolStr>,
    /// `close_transaction_id` for the Memory→Trail cross-link (R6.1).
    /// `None` when the correlation column is missing (older rows).
    pub close_transaction_id: Option<SmolStr>,
}

/// Phase F — Memory-screen per-session state (R4.1).
///
/// Added as `pub memory_screen_state: MemoryScreenState` on `Cockpit`
/// at `state.rs:~884`, immediately after `compare_screen_state`
/// (three-touchpoint pattern: struct field + Debug + Default).
#[derive(Debug, Clone, Default)]
pub struct MemoryScreenState {
    /// Active view mode. `Cards` is the default per Q1=(a).
    pub mode: MemoryViewMode,
    /// Active filter. `None` = show all.
    pub filter: Option<MemoryFilter>,
    /// Lesson card cache populated by `Message::MemoryHydrate`.
    /// Empty until the first hydrate fires (cold-boot-only per R5.3).
    pub cache: Vec<LessonCardCard>,
    /// ISO-8601 timestamp of the last successful hydrate. `None` until
    /// first hydrate fires.
    pub last_indexed: Option<SmolStr>,
    /// Which card's drawer is open. `None` = drawer closed (Q5=(b)).
    pub drawer_open: Option<SmolStr>,
}
