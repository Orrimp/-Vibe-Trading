//! Activity-tape data types (bug-log #92, relocated from `agent` 2026-08-22).
//!
//! These are **plain data**: no tokio, no `Instant`, no I/O. They live here so
//! that `ui` can render an activity tape without depending on `agent`.
//!
//! # Why they moved
//!
//! `crates/ui` declares `agent` only under its `live` feature, but
//! `src/state.rs`, `src/lab/activity.rs` and `src/widgets/activity_tape.rs`
//! imported it **unconditionally**. So the `--no-default-features` build that
//! `crates/ui/Cargo.toml` documents as supported ("for the gallery-only bin")
//! failed with three `E0432` unresolved-import errors — bug-log **#92**. Nothing
//! in CI, scripts, skills, README or the runbooks built that configuration, so it
//! rotted unnoticed.
//!
//! The **producer** side — `ActivitySender`, `ActivityHandle` and the tokio
//! broadcast channel — deliberately stays in `agent`: it needs tokio, and
//! `trading_core` has no async dependency. Only the wire types move, which is
//! what `ui` actually consumes. `agent` re-exports them, so every existing
//! `agent::ActivityEvent` path keeps compiling.

use std::sync::atomic::{AtomicU64, Ordering};

// ── Global ID counter ────────────────────────────────────────────────────────

static NEXT_ACTIVITY_ID: AtomicU64 = AtomicU64::new(1);

/// A monotonic per-process activity identifier. NOT a UUID; IDs do not need
/// to survive restarts (the activity tape is purely in-memory per R-NR.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActivityId(pub u64);

impl ActivityId {
    /// Allocate the next ID from the global atomic counter.
    #[must_use]
    pub fn next() -> Self {
        Self(NEXT_ACTIVITY_ID.fetch_add(1, Ordering::Relaxed))
    }
}

// ── Kind ─────────────────────────────────────────────────────────────────────

/// The activity class. Determines icon/label prefix in the status bar tape.
///
/// Q8=(a): only the three v0.1.0 producers are active.
/// `LlmCall` / `AuditLedgerWrite` are forward-listed (R5.1 / R5.2) and
/// included as variants so future producers can add wiring without a
/// schema migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActivityKind {
    /// Yahoo data preload (cold-cache fetch + parquet read).
    YahooPreload,
    /// Lab backtest run dispatched by the cockpit Lab screen.
    LabRun,
    /// Training subprocess managed by the cockpit Train sub-panel.
    Training,
    /// Forward-listed for v0.1.1 (`v3-llm-forecaster`). Not wired at v0.1.0.
    LlmCall,
    /// Forward-listed for v0.1.1. Not wired at v0.1.0 (K3 — aggregation
    /// design required before enabling).
    AuditLedgerWrite,
}

// ── Outcome / Phase ──────────────────────────────────────────────────────────

/// Terminal outcome of a completed activity.
#[derive(Debug, Clone)]
pub enum ActivityOutcome {
    /// Activity completed normally.
    Success,
    /// Activity failed with the given human-readable reason.
    Failed(String),
    /// Activity was cancelled by the operator or the system.
    Cancelled,
}

/// Lifecycle phase carried by each `ActivityEvent`.
#[derive(Debug, Clone)]
pub enum ActivityPhase {
    /// Activity just started. `total_units` is known if the producer can
    /// estimate it upfront (e.g. total bars for a backtest).
    Start { total_units: Option<u64> },
    /// Progress heartbeat. Rate-limited to ≤ 10 events/sec per handle (R1.4).
    Tick { current: u64, elapsed_ms: u64 },
    /// Activity finished (success, failure, or cancellation).
    End(ActivityOutcome),
}

// ── Event ────────────────────────────────────────────────────────────────────

/// A single event on the activity broadcast channel.
///
/// All fields are `Clone` so the broadcast channel can fan out to multiple
/// subscribers without allocation per receiver.
#[derive(Debug, Clone)]
pub struct ActivityEvent {
    /// Stable ID for correlating Start → Tick* → End across events.
    pub id: ActivityId,
    /// Which subsystem produced this event.
    pub kind: ActivityKind,
    /// Operator-facing label (≤ 64 chars recommended per R1.2).
    pub label: String,
    /// Lifecycle phase for this event.
    pub phase: ActivityPhase,
    /// Wall-clock milliseconds since the Unix epoch (UTC) at event emission.
    pub ts_ms: i64,
}
