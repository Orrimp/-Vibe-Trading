//! Reflection audit-tick consumer stub (R4 / v0.1.0).
//!
//! `ReflectionAuditTickConsumer` is an **observation-only** stub that receives
//! `AuditTick<AuditEvent>` from the broadcast bus and logs + counts each
//! variant. It does NOT write `LessonCard`s at v0.1.0 — the lesson-write
//! migration is an explicit follow-up brief.
//!
//! The existing `ReflectionWriter` (mpsc tap from `crates/exec`) stays
//! untouched and remains the v2.x production write path (R4.2). Both paths
//! coexist; this stub is observation-only (R4.1).
//!
//! Spawned only when `[reflection] audit_tick_consumer_enabled = true` in
//! `config/agent.toml` (default `false` — R4.3). Default builds are
//! bit-identical.

use std::sync::Arc;

use audit::tick::{AuditEvent, AuditTickStream};

/// v0.1.0 stub consumer — logs and counts variants. Does NOT write
/// `LessonCard`s (R4.1). The lesson-write migration is a follow-up brief.
pub struct ReflectionAuditTickConsumer<S> {
    stream: AuditTickStream,
    #[allow(dead_code)] // kept for the follow-up brief that writes lessons
    store: Arc<S>,
}

impl<S: Send + Sync + 'static> ReflectionAuditTickConsumer<S> {
    /// Construct with a `AuditTickStream` and a shared store handle.
    pub fn new(stream: AuditTickStream, store: Arc<S>) -> Self {
        Self { stream, store }
    }

    /// Drain ticks until the sender drops. Emits one
    /// `tracing::info!` + `reflection_audit_tick_seen_total{variant=…}`
    /// counter increment per tick.
    pub async fn run(mut self) {
        while let Some(tick) = self.stream.next().await {
            #[allow(unreachable_patterns)] // non_exhaustive: future variants land here
            let variant = match &tick.event {
                AuditEvent::Fill { .. } => "Fill",
                AuditEvent::StrategySignal { .. } => "StrategySignal",
                AuditEvent::StrategyEvent { .. } => "StrategyEvent",
                AuditEvent::ForecastEmitted { .. } => "ForecastEmitted",
                AuditEvent::KillSwitchTripped { .. } => "KillSwitchTripped",
                AuditEvent::FeedReconnect { .. } => "FeedReconnect",
                AuditEvent::UptimeIntervalOpened { .. } => "UptimeIntervalOpened",
                AuditEvent::UptimeIntervalClosed { .. } => "UptimeIntervalClosed",
                _ => "Unknown",
            };
            tracing::info!(
                target: "reflection::audit_tick",
                variant,
                run_id = %tick.context.run_id,
                "audit tick observed"
            );
            metrics::counter!(
                "reflection_audit_tick_seen_total",
                "variant" => variant,
            )
            .increment(1);
        }
    }
}
