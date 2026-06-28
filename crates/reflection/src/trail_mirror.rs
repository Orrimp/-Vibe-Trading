//! Trail-mirror — hot in-memory state-replica for the Phase D Trail view.
//!
//! Phase D (ui-rethink-phase-d-trail T-D-N23 / R6.1-R6.3).
//!
//! ## Responsibilities
//!
//! 1. **Broadcast subscriber** — wraps `AuditTickStream::new(rx,
//!    "ui_trail_mirror")` and processes incoming `AuditEvent` variants.
//!    On `RecvError::Lagged(n)` the warn+counter path inside
//!    `AuditTickStream::next` handles it; Phase D adds no new lag policy.
//!
//! 2. **LRU cache** — holds up to `LRU_CAPACITY` (16) reconstructed
//!    trail entries (`String → ReconstructedTrail`). Eviction on
//!    overflow prevents unbounded growth (H4 gate: heap < 1 MB at N=16
//!    under sustained chevron-click load).
//!
//! 3. **SQL backfill on `Open` request** — when the operator clicks a
//!    trail chevron, the UI sends `TrailMirrorRequest::Open(audit_id)`.
//!    The mirror first checks its LRU; on miss it queries the four
//!    correlation tables (`journal_transactions`, `strategy_signals`,
//!    `forecast_events`) and populates `ReconstructedTrail`. This
//!    closes the durability gap on consumer restart (R6.3).
//!
//! 4. **`TrailMirrorTick` broadcast** — after each update the mirror
//!    sends a `TrailMirrorTick` on its output sender so the iced
//!    `Subscription` bridge (`state.rs:~1213`, T-D-N26) delivers the
//!    update to the cockpit.
//!
//! ## Architecture invariant
//!
//! The trail-mirror lives in `crates/reflection` (NOT `crates/ui`) per
//! [decomp.md §3](../../spec/v1/ui-rethink-phase-d-trail/decomp.md).  This
//! preserves the ADR-0031 architecture edge `reflection → audit` and
//! ensures no `ui → audit` dep is added (R7.7).
//!
//! ## v0.1.0 scope
//!
//! The `run` loop is implemented; SQL backfill stubs return `None` for
//! all four stages (Wave G T-D-N25 wires the real query). The LRU cap
//! and request/response plumbing are fully functional.

use std::collections::HashMap;

use tokio::sync::{broadcast, mpsc};
use tracing::{debug, info};

use audit::tick::{AuditEvent, AuditTick, AuditTickStream};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Maximum number of `ReconstructedTrail` entries the LRU holds.
/// H4 gate: at ~60 kB per trail (generous upper bound), 16 entries = ~1 MB.
pub const LRU_CAPACITY: usize = 16;

// ── Domain types ──────────────────────────────────────────────────────────────

/// One reconstructed stage of the trail pipeline.
///
/// All fields are `Option` — absent when the stage has no audit row (R3.4).
#[derive(Debug, Clone, Default)]
pub struct TrailStage {
    /// Timestamp string already formatted (`HH:MM:SS.μμμ`).
    pub timestamp: Option<String>,
    /// Actor label ("strategy:<id>", "tcn:<rev_short>", etc.).
    pub actor: Option<String>,
    /// One-line headline summarising this stage's payload.
    pub headline: Option<String>,
    /// Raw payload as a serialised string (used by the drawer body).
    pub raw_payload: Option<String>,
}

/// Reconstructed four-stage decision trail for one fill.
///
/// Built from the four correlation tables (journal_transactions,
/// strategy_signals, forecast_events, future debate_events).
/// `debate` is always `None` at v0.1.0 (R1.5).
#[derive(Debug, Clone, Default)]
pub struct ReconstructedTrail {
    /// The audit_id (= `journal_transactions.id`) identifying this trail.
    pub audit_id: String,
    /// Fill stage — the most-downstream node.
    pub fill: TrailStage,
    /// Signal stage — the strategy signal that caused the fill.
    pub signal: TrailStage,
    /// Forecast stage — the TCN forecast that influenced the signal.
    pub forecast: TrailStage,
    /// LLM debate stage — always absent at v0.1.0 (R1.5 placeholder).
    pub debate: TrailStage,
}

// ── Request / response ────────────────────────────────────────────────────────

/// Request variants that the UI sends to the trail-mirror task.
#[derive(Debug, Clone)]
pub enum TrailMirrorRequest {
    /// Open / hydrate the trail for the given audit_id.
    /// The mirror checks its LRU first; on miss queries SQL backfill.
    Open(String),
}

/// Tick emitted by the trail-mirror to the iced `Subscription` bridge.
#[derive(Debug, Clone)]
pub enum TrailMirrorTick {
    /// A reconstructed trail is ready (LRU hit or SQL backfill completed).
    /// Boxed to reduce enum size (clippy::large_enum_variant).
    TrailReady(Box<ReconstructedTrail>),
    /// Steady-state update: the mirror saw a new `ForecastEmitted` / `Fill`
    /// tick for an audit_id currently in the LRU. The UI may re-fetch.
    TrailUpdated(String),
}

// ── Minimal LRU (no external crate) ──────────────────────────────────────────

/// A bounded LRU cache backed by a `VecDeque` (access-order) + `HashMap`.
///
/// `capacity` = `LRU_CAPACITY` (16). On overflow the least-recently-used
/// entry is evicted. Access (get/put) is O(capacity) — acceptable for N=16.
struct BoundedLru {
    capacity: usize,
    /// Insertion/access order: front = most recently used.
    order: std::collections::VecDeque<String>,
    map: HashMap<String, ReconstructedTrail>,
}

impl BoundedLru {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            order: std::collections::VecDeque::with_capacity(capacity + 1),
            map: HashMap::with_capacity(capacity + 1),
        }
    }

    fn get(&mut self, key: &str) -> Option<&ReconstructedTrail> {
        if self.map.contains_key(key) {
            // Move to front (most-recently-used).
            self.order.retain(|k| k != key);
            self.order.push_front(key.to_string());
            self.map.get(key)
        } else {
            None
        }
    }

    fn put(&mut self, key: String, value: ReconstructedTrail) {
        if self.map.contains_key(&key) {
            self.order.retain(|k| *k != key);
        } else if self.map.len() >= self.capacity {
            // Evict LRU (back of deque).
            if let Some(evict_key) = self.order.pop_back() {
                self.map.remove(&evict_key);
            }
        }
        self.order.push_front(key.clone());
        self.map.insert(key, value);
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.map.len()
    }
}

// ── TrailMirror ───────────────────────────────────────────────────────────────

/// The trail-mirror task.
///
/// Construct with [`TrailMirror::new`], then `.await` [`TrailMirror::run`]
/// in a spawned tokio task. The returned `TrailMirrorHandle` provides the
/// request sender and tick receiver for the iced `Subscription` bridge
/// (T-D-N26).
pub struct TrailMirror {
    /// Broadcast-tick stream from the audit ledger.
    stream: AuditTickStream,
    /// Audit ledger handle for SQL backfill (R6.3).
    ledger: std::sync::Arc<audit::Ledger>,
    /// LRU: `audit_id → ReconstructedTrail`.
    lru: BoundedLru,
    /// Incoming requests from the UI.
    req_rx: mpsc::Receiver<TrailMirrorRequest>,
    /// Outgoing ticks to the iced subscription bridge.
    tick_tx: broadcast::Sender<TrailMirrorTick>,
}

/// Handle returned to the caller; passed into the iced `Subscription` bridge.
#[derive(Clone)]
pub struct TrailMirrorHandle {
    /// Send `TrailMirrorRequest::Open(audit_id)` to hydrate a trail.
    pub req_tx: mpsc::Sender<TrailMirrorRequest>,
    /// Subscribe to receive `TrailMirrorTick` updates.
    pub tick_tx: broadcast::Sender<TrailMirrorTick>,
}

impl TrailMirror {
    /// Construct a `TrailMirror` and return both the task struct and its
    /// handle (caller passes to iced Subscription bridge).
    ///
    /// `audit_tick_rx` must be a fresh receiver from the audit ledger's
    /// broadcast bus (created via `sender.subscribe()` in `main.rs`).
    pub fn new(
        audit_tick_rx: broadcast::Receiver<AuditTick<AuditEvent>>,
        ledger: std::sync::Arc<audit::Ledger>,
    ) -> (Self, TrailMirrorHandle) {
        let stream = AuditTickStream::new(audit_tick_rx, "ui_trail_mirror");
        let (req_tx, req_rx) = mpsc::channel(64);
        // tick_tx capacity = 16 (one per LRU slot; cockpit rarely needs more).
        let (tick_tx, _) = broadcast::channel(16);
        let handle = TrailMirrorHandle {
            req_tx,
            tick_tx: tick_tx.clone(),
        };
        let mirror = Self {
            stream,
            ledger,
            lru: BoundedLru::new(LRU_CAPACITY),
            req_rx,
            tick_tx,
        };
        (mirror, handle)
    }

    /// Run the trail-mirror event loop until the broadcast bus closes.
    ///
    /// Processes:
    /// - `TrailMirrorRequest::Open(audit_id)` → LRU check → SQL backfill.
    /// - `AuditEvent::Fill` / `ForecastEmitted` → update LRU if the
    ///   affected audit_id is cached; emit `TrailMirrorTick::TrailUpdated`.
    pub async fn run(mut self) {
        info!(target: "trail_mirror", "subscribed");

        loop {
            tokio::select! {
                // ── Incoming audit-tick ───────────────────────────────────
                maybe_tick = self.stream.next() => {
                    let Some(tick) = maybe_tick else {
                        info!(target: "trail_mirror", "audit tick stream closed — stopping");
                        break;
                    };
                    self.handle_audit_tick(tick).await;
                }

                // ── UI request ───────────────────────────────────────────
                maybe_req = self.req_rx.recv() => {
                    let Some(req) = maybe_req else {
                        debug!(target: "trail_mirror", "request channel closed");
                        break;
                    };
                    self.handle_request(req).await;
                }
            }
        }

        info!(target: "trail_mirror", "trail mirror stopped");
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    async fn handle_audit_tick(&mut self, tick: AuditTick<AuditEvent>) {
        match &tick.event {
            AuditEvent::Fill { fill, .. } => {
                // Derive the audit_id from the fill's transaction_id if present.
                if let Some(ref tx_id) = fill.transaction_id {
                    let audit_id = tx_id.to_string();
                    if self.lru.get(&audit_id).is_some() {
                        self.emit_updated(audit_id);
                    }
                }
            }
            AuditEvent::ForecastEmitted { overlay, .. } => {
                let audit_id = overlay.correlation_id.to_string();
                if self.lru.get(&audit_id).is_some() {
                    self.emit_updated(audit_id);
                }
            }
            _ => {} // Other events: not relevant to the trail view.
        }
    }

    async fn handle_request(&mut self, req: TrailMirrorRequest) {
        match req {
            TrailMirrorRequest::Open(audit_id) => {
                debug!(target: "trail_mirror", %audit_id, "Open request received");

                // Check LRU first.
                if let Some(trail) = self.lru.get(&audit_id) {
                    let trail = trail.clone();
                    self.emit_ready(trail);
                    return;
                }

                // SQL backfill — Wave G T-D-N25 wires the real query.
                // At v0.1.0 we return an empty-stage stub (R3.4 graceful degradation).
                let trail = self.backfill(&audit_id).await;
                self.lru.put(audit_id, trail.clone());
                self.emit_ready(trail);
            }
        }
    }

    /// SQL backfill: query the four correlation tables and reconstruct the trail.
    ///
    /// At v0.1.0 this is a stub (all stages `None`). T-D-N25 (`audit::query::
    /// trail_for_fill_id`) wires the real four-table join.
    async fn backfill(&self, audit_id: &str) -> ReconstructedTrail {
        // v0.1.0 stub — returns empty-stage trail.
        // T-D-N25 replaces this body with the real `audit::query::trail_for_fill_id` call.
        debug!(
            target: "trail_mirror",
            audit_id,
            "backfill stub (v0.1.0) — all stages empty"
        );
        // `self.ledger` is kept for the T-D-N25 real backfill path.
        // Suppress unused warning at the stub stage.
        let _ = &self.ledger;
        ReconstructedTrail {
            audit_id: audit_id.to_string(),
            ..Default::default()
        }
    }

    fn emit_ready(&self, trail: ReconstructedTrail) {
        let _ = self
            .tick_tx
            .send(TrailMirrorTick::TrailReady(Box::new(trail)));
    }

    fn emit_updated(&self, audit_id: String) {
        let _ = self.tick_tx.send(TrailMirrorTick::TrailUpdated(audit_id));
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// T-D-N23 (a) — `BoundedLru` respects the cap: inserting 17 entries into
    /// a capacity-16 cache evicts the oldest (LRU), never grows beyond 16.
    /// H4 gate: heap footprint never unbounded.
    #[test]
    fn lru_cap_enforced() {
        let mut lru = BoundedLru::new(16);
        for i in 0..17u32 {
            let key = format!("audit-{i}");
            let val = ReconstructedTrail {
                audit_id: key.clone(),
                ..Default::default()
            };
            lru.put(key, val);
        }
        // After 17 inserts into a cap-16 cache, exactly 16 entries remain.
        assert_eq!(lru.len(), 16, "LRU must evict oldest on overflow (H4 gate)");
        // The first entry (audit-0) should be evicted (LRU).
        assert!(lru.get("audit-0").is_none(), "oldest entry must be evicted");
        // The most-recently inserted entry (audit-16) must be present.
        assert!(
            lru.get("audit-16").is_some(),
            "most-recent entry must survive"
        );
    }

    /// T-D-N23 (b) — accessing an entry promotes it above the eviction line.
    #[test]
    fn lru_access_promotes_entry() {
        let mut lru = BoundedLru::new(4);
        for i in 0..4u32 {
            let key = format!("a-{i}");
            lru.put(
                key.clone(),
                ReconstructedTrail {
                    audit_id: key,
                    ..Default::default()
                },
            );
        }
        // Access a-0 (was oldest).
        let _ = lru.get("a-0");
        // Insert a-4 — should evict a-1 (now oldest), not a-0.
        let k = "a-4".to_string();
        lru.put(
            k.clone(),
            ReconstructedTrail {
                audit_id: k,
                ..Default::default()
            },
        );
        assert!(
            lru.get("a-0").is_some(),
            "accessed entry must survive eviction"
        );
        assert!(lru.get("a-1").is_none(), "unaccessed entry must be evicted");
    }

    /// T-D-N23 (c) — `ReconstructedTrail::default()` has all stages empty
    /// (R3.4 graceful degradation invariant).
    #[test]
    fn reconstructed_trail_default_all_none() {
        let trail = ReconstructedTrail::default();
        assert!(trail.audit_id.is_empty());
        assert!(trail.fill.timestamp.is_none());
        assert!(trail.signal.actor.is_none());
        assert!(trail.forecast.headline.is_none());
        assert!(trail.debate.raw_payload.is_none());
    }
}
