//! Audit tick consumer envelope — read-direction broadcast over the audit journal.
//!
//! # Durability contract (K6)
//!
//! Ticks are **in-memory only**. SQL rows are durable. On consumer restart,
//! backfill from the `journal_entries` / `strategy_events` tables. A receiver
//! that observes `RecvError::Closed` should drain and restart.
//!
//! # Tee opt-in convention (K5 mitigation)
//!
//! Every `journal::*` writer that calls `db_txn.commit()` (or `execute(...)` on
//! a single-shot row) and represents an event a consumer might care about MUST
//! grow a `crate::tick::emit(ledger, AuditEvent::…)` call **after** the commit.
//! The in-scope writers at v0.1.0 are enumerated in
//! `spec/audit-tick-consumer-envelope/decomp.md §3`. Adding a new variant
//! requires an ADR amendment.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use time::OffsetDateTime;
use tokio::sync::broadcast;
use trading_core::{Fill, ForecastOverlay, Signal, StrategyId, Venue};
use uuid::Uuid;

use crate::Ledger;

/// Generic envelope over an audit event plus run-time context.
/// Mirrors the barter-rs `AuditTick` shape (no crate dep — shape only).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditTick<Event, Context = AuditContext> {
    pub event: Event,
    pub context: Context,
}

/// Run-time context attached to every tick. Pre-seeded on the `Ledger` at
/// session start (Q5). `agent_pid` is set once via `std::process::id()`;
/// `run_id` defaults to `Uuid::nil()` for the live agent's startup-time uuid
/// OR is overridden per-backtest via `Ledger::with_run_id(uuid)` (K4 mitigation).
/// `posted_at` is stamped at each `emit()` call site.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditContext {
    pub run_id: Uuid,
    pub posted_at: OffsetDateTime,
    pub agent_pid: u32,
}

/// Variants emitted at v0.1.0. `#[non_exhaustive]` is MANDATORY so v3 can add
/// `PartialFill`, `OrderPlaced`, etc., without breaking downstream consumers
/// (R1.3 / K5).
///
/// # Size budget (H5)
///
/// `AuditEvent` must stay ≤ 256 bytes (`tick_event_size.rs` guards this).
/// `Fill` and `Signal` are boxed so the enum discriminant + pointer fits in
/// the budget even as those domain types grow.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEvent {
    /// Emitted from `journal::post_fill` (R2.5).
    /// `fill` is boxed because `Fill` is ~120 bytes; keeping the enum ≤ 256B
    /// requires the pointer indirection (H5).
    Fill { fill: Box<Fill>, fees: Decimal },
    /// Emitted from `journal::post_strategy_signal` (R2.5).
    /// `signal` is boxed for the same size-budget reason (H5).
    StrategySignal {
        strategy_id: StrategyId,
        signal: Box<Signal>,
    },
    /// Emitted from `journal::strategy_event` (R2.5). Covers the four
    /// delegating writers (`feed_reconnect`, `rebalance_rejected`,
    /// `mean_reversion_stop`, `pair_short_observation`) through the `kind`
    /// discriminator.
    StrategyEvent { kind: SmolStr, payload_json: String },
    /// Emitted from `crates/forecast/src/tcn.rs` cache-hit + post-inference
    /// call sites (decomp §5A). Guarded by the `audit-tick` feature on the
    /// `forecast` crate.
    ForecastEmitted {
        overlay: ForecastOverlay,
        cache_hit: bool,
    },
    /// Emitted from `journal::kill_switch_tripped` (R2.5). Owns its own
    /// transaction — does NOT delegate to `strategy_event`.
    KillSwitchTripped { reason: SmolStr },
    /// Reserved for a future v3 typed-feed refactor. Not emitted at v0.1.0;
    /// `feed_reconnect` produces `StrategyEvent { kind = "FeedReconnect" }`
    /// via the `strategy_event` delegation today.
    FeedReconnect {
        venue: Venue,
        symbol: trading_core::Symbol,
        gap_ms: u64,
    },
    /// Emitted from `journal::open_uptime_interval` (R2.5).
    UptimeIntervalOpened { run_id: Uuid },
    /// Emitted from `journal::close_uptime_interval` (R2.5).
    UptimeIntervalClosed { run_id: Uuid, duration_s: u64 },
}

// ── pub(crate) helper: journal-side tee (single-line call at each writer) ──

/// Emit an `AuditTick` post-commit. Returns immediately — drops silently on
/// no-subscribers or lag overflow (R2.3 / H1 / H3). The `None` arm of
/// `tick_bus` is a single predictable static branch (H2 anchor preservation).
pub(crate) fn emit(ledger: &Ledger, event: AuditEvent) {
    let Some(bus) = ledger.tick_bus.as_ref() else {
        return;
    };
    let tick = AuditTick {
        event,
        context: AuditContext {
            run_id: bus.run_id,
            posted_at: time::OffsetDateTime::now_utc(),
            agent_pid: bus.agent_pid,
        },
    };
    let variant = variant_label(&tick.event);
    metrics::counter!("audit_tick_emitted_total", "variant" => variant).increment(1);
    tracing::debug!(target: "audit::tick", variant, "audit tick emitted");
    // Silently drop Err(Lagged) or Err(Closed) — never propagates as LedgerError.
    let _ = bus.sender.send(tick);
}

/// Public variant of `emit` for cross-crate callers (e.g. `crates/forecast`
/// with the `audit-tick` feature). Same semantics as `pub(crate) emit`.
pub fn emit_public(ledger: &Ledger, event: AuditEvent) {
    emit(ledger, event);
}

fn variant_label(event: &AuditEvent) -> &'static str {
    #[allow(unreachable_patterns)] // non_exhaustive: future variants land here
    match event {
        AuditEvent::Fill { .. } => "Fill",
        AuditEvent::StrategySignal { .. } => "StrategySignal",
        AuditEvent::StrategyEvent { .. } => "StrategyEvent",
        AuditEvent::ForecastEmitted { .. } => "ForecastEmitted",
        AuditEvent::KillSwitchTripped { .. } => "KillSwitchTripped",
        AuditEvent::FeedReconnect { .. } => "FeedReconnect",
        AuditEvent::UptimeIntervalOpened { .. } => "UptimeIntervalOpened",
        AuditEvent::UptimeIntervalClosed { .. } => "UptimeIntervalClosed",
        _ => "Unknown",
    }
}

// ── Consumer-side newtype ────────────────────────────────────────────────────

/// Newtype wrapping a `broadcast::Receiver`. Provides an async `next()` with
/// explicit lag handling (R3.1) and a blocking-iterator adaptor for synchronous
/// consumers (R3.2).
pub struct AuditTickStream {
    rx: broadcast::Receiver<AuditTick<AuditEvent>>,
    consumer_label: SmolStr,
}

impl AuditTickStream {
    /// Wrap a fresh receiver. `consumer_label` flows into the
    /// `audit_tick_lagged_total{consumer=label}` Prometheus counter on
    /// `RecvError::Lagged` (R6.1).
    pub fn new(
        rx: broadcast::Receiver<AuditTick<AuditEvent>>,
        consumer_label: impl Into<SmolStr>,
    ) -> Self {
        Self {
            rx,
            consumer_label: consumer_label.into(),
        }
    }

    /// Returns the next tick asynchronously (R3.1).
    /// - `Ok(tick)` → `Some(tick)`.
    /// - `Err(Lagged(n))` → log warn + counter increment, continue to next recv.
    /// - `Err(Closed)` → `None` (sender dropped; consumer should wind down).
    pub async fn next(&mut self) -> Option<AuditTick<AuditEvent>> {
        loop {
            match self.rx.recv().await {
                Ok(tick) => return Some(tick),
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(
                        target: "audit::tick",
                        consumer = %self.consumer_label,
                        lagged = n,
                        "audit tick stream lagged"
                    );
                    metrics::counter!(
                        "audit_tick_lagged_total",
                        "consumer" => self.consumer_label.to_string(),
                    )
                    .increment(n);
                }
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    }

    /// Blocking adaptor for synchronous consumers (R3.2). Uses
    /// `tokio::runtime::Handle::block_on` internally. Panics if called from
    /// inside an async runtime context. Intended for `crates/reports`
    /// synchronous renderers.
    pub fn into_iter_blocking(mut self) -> impl Iterator<Item = AuditTick<AuditEvent>> {
        let handle = tokio::runtime::Handle::current();
        std::iter::from_fn(move || handle.block_on(self.next()))
    }
}
