//! Report rendering — front-matter writer + body assembly.
//!
//! ## Body-vs-front-matter discipline (R10.2–R10.5)
//!
//! Anything that varies between two equivalent runs (timestamps,
//! wall-clock, host, pid, git commit, generated:, `data_source`
//! variants, reconciliation outcome) lives in
//! [`front_matter::FrontMatter`] —
//! the body is the deterministic, hashable part.
//!
//! Each `body/*.rs` module returns a pure `String` from a `render`
//! function over its inputs.  No `SystemTime::now()`, no
//! `Instant::now()`, no clock access of any kind.  The orchestrator in
//! [`crate::generate`] does the I/O once per query and hands
//! `Decimal` / `Vec<...>` / pre-frozen data to each renderer.

pub mod equity_curve;
pub mod front_matter;
pub mod headline;
pub mod memory_highlights;
pub mod open_risks;
pub mod reconciliation;
pub mod risk_metrics;
pub mod strategy_attribution;
pub mod system_health;
pub mod what_changed;

/// What was written by [`crate::generate`].
///
/// The doc comment here read "Used by tests and the bin to surface the run-id
/// and the body SHA" until 2026-11-08. Only the first half was true: both
/// production callers print `run_id` and `markdown_path` and nothing else
/// (`bin/report.rs`, `agent/src/cron.rs`).
///
/// A `body_sha256: [u8; 32]` field was removed in the same pass (bug-log `#162`).
/// `grep -rn '\.body_sha256' --include='*.rs'` found **zero** readers, and the
/// value was computed on the wrong side of a one-byte boundary: it hashed the
/// in-memory body, while `scripts/hash_report.py` — the canonical body-hash
/// function the 119-anchor gate execs — strips `^---\n.*?\n---\n` and so keeps
/// the blank line that follows the closing fence. On a real anchored body the
/// two digests are `e1b286d5…` (canonical) and `532b09de…` (the field), exactly
/// one leading newline apart.
///
/// It was deleted rather than corrected because the repo already has two
/// definitions of this convention that agree — `hash_report.py` and
/// `tests/report_scenarios.rs::body_after_fence` — and a third with no reader is
/// a trap, not a cross-check. `tests/body_hash_convention.rs` now gates the
/// agreement between those two, which is what the anchors actually rest on.
#[derive(Debug, Clone)]
pub struct ReportArtifacts {
    /// Path to the rendered markdown.
    pub markdown_path: std::path::PathBuf,
    /// Run-id (16 hex chars).
    pub run_id: String,
    /// Companion CSVs written under `artifacts/<run-id>/`.
    pub csv_paths: Vec<std::path::PathBuf>,
}
