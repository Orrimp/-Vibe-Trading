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

/// What was written by [`crate::generate`].  Used by tests and the bin
/// to surface the run-id and the body SHA.
#[derive(Debug, Clone)]
pub struct ReportArtifacts {
    /// Path to the rendered markdown.
    pub markdown_path: std::path::PathBuf,
    /// Run-id (16 hex chars).
    pub run_id: String,
    /// Companion CSVs written under `artifacts/<run-id>/`.
    pub csv_paths: Vec<std::path::PathBuf>,
    /// SHA-256 of the body bytes (post-fence, R10.3).
    pub body_sha256: [u8; 32],
}
