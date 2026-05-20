---
slug: ui-rethink-phase-d-trail
status: draft
owner: architect
updated: 2026-05-20
version: 0.1.0
predecessor: ui-rethink-phase-c-sidebar-ia v0.1.0
---

# Decomposition — UI rethink Phase D (Trail view)

> Architect M-T1 deliverable. Implements the contract in
> [feature.md](feature.md) (R1..R7 / Q1..Q5 resolved to analyst
> defaults via operator "Autoapprove all" 2026-05-20) against the
> predecessor contract in
> [ADR-0031](../architecture/adr/0031-audit-tick-consumer-envelope.md)
> § "Phase D amendment (2026-05-20)" (T-D-14 close-out).
>
> The developer implements against this document. Every change site
> below is pinned with `file:line`. No code lands outside the listed
> sites. The per-wave `T-D-N…` checklist lives in
> [tasks.md](tasks.md); this document is the contract those tasks
> reference.

## §0 — Operator-decided defaults (2026-05-20)

| Q  | Default       | Architect commits to                                       |
|----|---------------|-------------------------------------------------------------|
| Q1 | (b) ship mig 011 | 4 ALTER + 1 CREATE IF NOT EXISTS — locked in §2 below   |
| Q2 | (a) upstream-at-top | `widgets::trail_node` renders Forecast→LLM→Signal→Fill top→bottom |
| Q3 | (a) chevron-click | `Message::TrailNodeChevronClicked(node_kind)` is the sole drawer trigger |
| Q4 | (a) trail-mirror itself | R6.4 in scope; K5 spike outcome below dictates wiring |
| Q5 | (a) every row + lazy backfill | Chevron on every row; first-open hits SQL backfill |

## §1 — K5 spike result (T-T1-1)

**Verdict: SUCCESS.** Full wiring is feasible without breaking
backtest determinism or the H2 anchor invariant. Architect produces
the ADR amendment alongside this decomp; developer Wave F lands the
code per the file:line plan below.

### §1.1 Evidence (read-line cite)

1. `crates/forecast/src/tcn.rs:571-576` — `TcnForecaster::with_ledger`
   already exists behind `#[cfg(feature = "audit-tick")]`; it is the
   load-bearing builder.
2. `crates/forecast/src/tcn.rs:822-831` (cache-hit branch) +
   `:937-947` (post-inference branch) — the two emit sites are
   already gated by `self.ledger.as_ref()`; no new emit sites are
   needed. The producer-side audit-tick contract is fully wired in
   `crates/forecast` and the v0.1.0 predecessor closed the audit
   feature plumbing.
3. `crates/strategy/src/tcn_overlay_momentum.rs:170-184` —
   `TcnSyncForecaster::load_bs1/bs2` is the **only** runtime entry
   point used outside backtests; it currently builds the
   `forecast::tcn::TcnForecaster` with no ledger attached.
4. `crates/strategy/src/tcn_overlay_momentum.rs:349-352, 363-366` —
   `TcnOverlayMomentumStrategy::with_tcn_bs1/bs2(base)` are the
   `cfg(feature = "forecast")` entry-points and call into
   `TcnSyncForecaster::load_*`. Backtest call sites land here.
5. `crates/agent/src/runtime.rs:115-141` —
   `pub fn build_registry(cfg: &Config) -> Arc<StrategyRegistry>`
   registers `SmaCrossover` only. The function is sync (no async,
   no ledger handle, no `RunHandles`).
6. `crates/agent/src/runtime.rs:91-113` — `RunHandles` already
   carries `pub ledger: Arc<audit::Ledger>`; the ledger flowing in is
   the same `open_with_tick_bus` instance constructed in
   `crates/agent/src/main.rs:100-104`. The tick bus is live in
   paper-mode by construction (T-D-12 / predecessor).
7. `crates/audit/src/ledger.rs:33,59,83-95` — the determinism gate
   is intact: `Ledger::open` (used by backtests) sets
   `tick_bus = None`; `Ledger::open_with_tick_bus` (used by the
   paper-mode runtime) sets it to `Some(TickBus { … })`. The
   `tick.rs:104-107` static-branch tee stays dormant in backtests.

### §1.2 Wiring shape (locked)

The shape mirrors the existing `with_ledger` precedent in
`forecast::tcn::TcnForecaster`:

```rust
// crates/strategy/src/tcn_overlay_momentum.rs — append to
// the existing `impl TcnSyncForecaster { … }` block at L164.

#[cfg(feature = "forecast")]
impl TcnSyncForecaster {
    /// Attach an audit ledger so `ForecastEmitted` ticks fire on the
    /// broadcast tick bus (closes Phase D / T-D-14). The inner
    /// `TcnForecaster::with_ledger` is feature-gated on `audit-tick`
    /// (the `strategy` crate already enables it via
    /// `Cargo.toml:13` — verified during the spike).
    #[cfg(feature = "audit-tick")]
    #[must_use]
    pub fn with_ledger(mut self, ledger: audit::Ledger) -> Self {
        self.forecaster = self.forecaster.with_ledger(ledger);
        self
    }
}
```

And the matching agent-side sibling:

```rust
// crates/agent/src/runtime.rs — sibling of `build_registry` at L129.

/// Phase D close-out (R6.5). Builds the same registry as
/// `build_registry` and additionally constructs the TCN-overlay
/// strategy with the audit `Ledger` threaded through so
/// `AuditEvent::ForecastEmitted` ticks reach the broadcast bus.
///
/// Called by `crates/agent/src/main.rs:184` in paper mode (the only
/// runtime path with a tick-bus-armed ledger). Backtests continue
/// to call `build_registry(cfg)` (no ledger threaded through →
/// `tick.rs:104-107` static-branch tee stays dormant → H2 anchor
/// invariant holds by construction).
#[cfg(feature = "forecast")]
pub fn build_registry_with_ledger(
    cfg: &Config,
    ledger: &Arc<audit::Ledger>,
) -> Arc<strategy::StrategyRegistry> {
    let registry = strategy::StrategyRegistry::new();
    registry.register(Box::new(strategy::SmaCrossover::new(
        cfg.strategies.sma_crossover.fast_len,
        cfg.strategies.sma_crossover.slow_len,
    )));
    if cfg.strategies.tcn_overlay_momentum.enabled {
        let base = /* same MomentumStrategy ctor used by load_bs1 */;
        match strategy::TcnOverlayMomentumStrategy::with_tcn_bs1(base) {
            Ok(mut s) => {
                s = s.with_forecaster_ledger((**ledger).clone());
                registry.register(Box::new(s));
                tracing::info!("tcn_overlay registered with ledger (Phase D R6.4)");
            }
            Err(e) => {
                tracing::warn!(error = %e,
                    "tcn_overlay disabled — checkpoint missing; \
                     falling back to SmaCrossover-only registry");
            }
        }
    }
    Arc::new(registry)
}
```

Notes:
- `TcnOverlayMomentumStrategy` gains a small `with_forecaster_ledger`
  helper (Wave F task) that threads the ledger through to its
  `Box<dyn SyncForecaster>` — concretely a downcast / a typed
  builder variant. The downcast is **not** acceptable; the cleanest
  shape is a `TcnOverlayMomentumStrategy::with_tcn_bs1_ledger(base, ledger)`
  builder that mirrors `with_tcn_bs1` (analogous to mig 008's
  `feed_reconnect_with_venue` precedent). See Wave F T-D-N20.
- `cfg.strategies.tcn_overlay_momentum.enabled` is a **new config
  knob** (default = `false`). Adding it is additive and matches the
  existing `[signal_log] enabled` precedent (`config.rs:284`). Wave F
  T-D-N19 adds the config struct + default.
- `with_tcn_bs2` mirror: skipped for v0.1.0 — BS-2 wiring is a
  follow-up. The single-checkpoint default keeps the K7 smoke gate
  scoped (M-FINAL T-F7 asserts ≥1 `ForecastEmitted` tick; BS-1 is
  sufficient).

### §1.3 Determinism gate

Backtests **must not** acquire a ledger with `tick_bus = Some(…)`.
The proof obligation is satisfied by these two facts:

1. `crates/agent/src/main.rs:100-104` is the **only** call site for
   `Ledger::open_with_tick_bus`. All backtest harnesses
   (`crates/reports`, `crates/backtest`, `cockpit_backtest` bin) call
   `audit::Ledger::open(…)` instead (`ledger.rs:33` — the unarmed
   branch).
2. `build_registry_with_ledger` is gated on the presence of the
   `Arc<audit::Ledger>` parameter; `build_registry(cfg)` (no
   ledger) is the call site used in tests and backtests
   (`runtime.rs:1237`). Calling the wrong sibling at a backtest call
   site is a compile-error class — the parameter is required.

### §1.4 Fallback (NOT exercised)

If the wiring above falls apart in implementation, Wave F drops to
R6.1-R6.3 only: trail-mirror reads `Fill`/`Signal`/`KillSwitch` ticks
from the broadcast bus + backfills `forecast_events` rows from SQL
on first open. The mig 011 schema (§2) supports both modes; the
fallback is purely a Wave F scope cut. **The spike succeeded; this
section is documented for the post-mortem branch only.**

## §2 — Mig 011 SQL shape (T-T1-2)

Pure additive — 4 `ALTER TABLE … ADD COLUMN` + 1
`CREATE TABLE IF NOT EXISTS` + 4 `CREATE INDEX IF NOT EXISTS`. No
backfill, no `UPDATE` on any pre-existing row. Mirrors mig
008/009/010's precedent. Anchor-safe by construction (H2): pre-mig
rows surface NULL on every new column; the 22 anchored backtest
report bodies never read these columns; the
`crates/audit/migrations/008_journal_transactions_venue.sql:21-23`
precedent already proved the shape against the locked anchors.

### §2.1 File: `crates/audit/migrations/011_trail_correlation_chain.sql`

```sql
-- Migration 011 — trail-correlation chain for ui-rethink-phase-d-trail v0.1.0
-- (Q1 = (b) ship; R1.1-R1.5; ADR-0031 § Phase D amendment).
--
-- Pure additive — 4 ALTER TABLE ADD COLUMN (all NULL-default) + 1
-- CREATE TABLE IF NOT EXISTS + 4 CREATE INDEX IF NOT EXISTS. No
-- ALTER on any pre-existing column, no UPDATE on any pre-existing
-- row, no backfill. The 22 backtest body-SHA-256 anchors are
-- byte-identical post-mig by construction — none of the anchored
-- reports read the new columns or the new table.
--
-- The companion writers live at:
--   - journal.rs::post_fill_with_signal      (R1.1 + R1.2; extends post_fill)
--   - journal.rs::post_strategy_signal       (R1.3; 6-arg → 7-arg, fwd-compat callers pass None)
--   - journal.rs::post_forecast_event        (R1.4; NEW writer, sibling of post_strategy_signal)
-- And the readers at:
--   - audit::query::trail_for_fill_id        (R6.3; new — 4-way correlated lookup)
--   - audit::query::recent_forecast_events   (R6.3; new — sibling of recent_signals)
--
-- See spec/ui-rethink-phase-d-trail/decomp.md §2 for the column-by-
-- column rationale.

-- R1.1 — journal_transactions.fill_id (the source-of-truth Fill.id)
ALTER TABLE journal_transactions ADD COLUMN fill_id TEXT;
CREATE INDEX IF NOT EXISTS journal_transactions_fill_id_idx
    ON journal_transactions(fill_id);

-- R1.2 — journal_transactions.signal_id (upstream Signal lineage)
ALTER TABLE journal_transactions ADD COLUMN signal_id TEXT;
CREATE INDEX IF NOT EXISTS journal_transactions_signal_id_idx
    ON journal_transactions(signal_id);

-- R1.3 — strategy_signals.forecast_correlation_id (upstream Forecast lineage)
ALTER TABLE strategy_signals ADD COLUMN forecast_correlation_id TEXT;
CREATE INDEX IF NOT EXISTS strategy_signals_forecast_id_idx
    ON strategy_signals(forecast_correlation_id);

-- R1.4 — forecast_events table (the durable side of AuditEvent::ForecastEmitted)
CREATE TABLE IF NOT EXISTS forecast_events (
    correlation_id   TEXT PRIMARY KEY,        -- ForecastOverlay.correlation_id (UUID)
    ts               TEXT NOT NULL,           -- RFC3339 6-digit microsecond (ADR-0004)
    strategy_id      TEXT NOT NULL,           -- StrategyId.0 string
    symbol           TEXT NOT NULL,           -- "BTCUSDT" style
    direction        TEXT NOT NULL,           -- 'up' | 'down' | 'flat'
    confidence       TEXT NOT NULL,           -- Decimal as TEXT (ADR-0003)
    model_revision   TEXT NOT NULL,           -- SHA per ADR-0029
    cache_hit        INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS forecast_events_ts_idx
    ON forecast_events(ts);
CREATE INDEX IF NOT EXISTS forecast_events_strategy_id_idx
    ON forecast_events(strategy_id, ts);
```

### §2.2 Writer signatures (locked)

#### `post_fill_with_signal` (NEW; wraps existing `post_fill`)

```rust
// crates/audit/src/journal.rs — sibling of post_fill at L74.

/// Phase D extension of `post_fill` (R1.1 + R1.2). Threads the
/// upstream `signal_id` (from `strategy_signals.id` — the row id
/// returned by `post_strategy_signal`) into the
/// `journal_transactions.signal_id` column and persists the
/// canonical `fill.id` into `journal_transactions.fill_id`
/// (separate from the synthesised `journal_transactions.id` UUID
/// the writer mints internally).
///
/// `post_fill(ledger, fill, venue, strategy_id)` becomes a thin
/// `post_fill_with_signal(ledger, fill, venue, strategy_id, None)`
/// wrapper — backwards-compatible per mig 004's `strategy_id`
/// precedent (R1.2).
#[instrument(name = "ledger.post_fill_with_signal", skip(ledger, fill, signal_id),
    fields(fill_id = %fill.id, venue = %venue, strategy_id = ?strategy_id))]
pub async fn post_fill_with_signal(
    ledger: &Ledger,
    fill: &Fill,
    venue: Venue,
    strategy_id: Option<&str>,
    signal_id: Option<&str>,
) -> Result<SmolStr, LedgerError> { /* SQL identical to post_fill plus
    the two new bound columns; row count unchanged */ }

/// Existing surface preserved (R7.2 anchor gate): callers that
/// don't have a signal_id (paper-mode SmaCrossover; backtests)
/// continue to call this directly.
pub async fn post_fill(
    ledger: &Ledger,
    fill: &Fill,
    venue: Venue,
    strategy_id: Option<&str>,
) -> Result<SmolStr, LedgerError> {
    post_fill_with_signal(ledger, fill, venue, strategy_id, None).await
}
```

#### `post_strategy_signal` (extended; 6-arg → 7-arg)

```rust
// crates/audit/src/journal.rs — modify in place at L293-301.
// Existing callers (2 in tests; 0 in non-test crate code per grep)
// pass `None` for the new param. The Hold-skip early-return at
// L305-307 is preserved.
pub async fn post_strategy_signal(
    ledger: &Ledger,
    signal: &Signal,
    intended_qty: Quantity,
    intended_price: Option<Price>,
    venue: Venue,
    was_clamped: bool,
    clamp_reason: Option<&str>,
    forecast_correlation_id: Option<Uuid>,   // NEW — R1.3
) -> Result<SmolStr, LedgerError> { … }
```

Bind site: append `.bind(forecast_correlation_id.map(|u| u.to_string()))`
to the existing `sqlx::query(...)` block at `journal.rs:339-355`; the
INSERT SQL grows from 10 columns to 11.

#### `post_forecast_event` (NEW)

```rust
// crates/audit/src/journal.rs — sibling of post_strategy_signal at L293.

/// Phase D writer (R1.4). Persists a `ForecastOverlay` to the
/// `forecast_events` table and fires the **existing**
/// `AuditEvent::ForecastEmitted` tick (no new variant — the
/// payload already carries `overlay.correlation_id`).
///
/// Call sites: the two existing `crates/forecast/src/tcn.rs` emit
/// sites (cache-hit at L822-831 and post-inference at L937-947)
/// invoke this **alongside** the existing tick-emit; the tick path
/// stays the broadcast contract for live consumers while the SQL
/// row closes the durability gap (K6 — restart-consumer backfill).
///
/// **Determinism gate.** This writer takes a `&Ledger`. In
/// backtests the ledger is constructed via `Ledger::open` (no
/// tick bus → `tick.rs:104-107` static-branch tee dormant); the
/// SQL row still lands. Pre-existing 22 anchors do not read this
/// table → anchor-safe.
#[instrument(name = "ledger.post_forecast_event", skip(ledger, overlay),
    fields(correlation_id = %overlay.correlation_id,
           strategy_id, symbol, cache_hit))]
pub async fn post_forecast_event(
    ledger: &Ledger,
    overlay: &ForecastOverlay,
    strategy_id: &str,
    symbol: &str,
    cache_hit: bool,
) -> Result<(), LedgerError> {
    // 6-digit microsecond ts — mirrors post_strategy_signal at L315-323
    // (HF-3 / ADR-0004 determinism gate).
    let ts_fmt = time::format_description::parse(
        "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:6]Z",
    ).map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    let ts = overlay.sampled_at.format(&ts_fmt)
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    let direction_str = match overlay.direction {
        Direction::Up => "up",
        Direction::Down => "down",
        Direction::Flat => "flat",
    };
    let confidence_str = overlay.confidence.to_string();
    let cache_hit_i = i64::from(cache_hit);

    sqlx::query(
        "INSERT OR IGNORE INTO forecast_events
         (correlation_id, ts, strategy_id, symbol, direction,
          confidence, model_revision, cache_hit)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(overlay.correlation_id.to_string())
    .bind(&ts)
    .bind(strategy_id)
    .bind(symbol)
    .bind(direction_str)
    .bind(&confidence_str)
    .bind(&overlay.model_revision)
    .bind(cache_hit_i)
    .execute(&ledger.pool)
    .await
    .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    Ok(())
}
```

Notes:
- `INSERT OR IGNORE` (not `INSERT`) — cache-hit and post-inference
  branches may both fire for the same `correlation_id` on a
  replay-warm cache; the second emit is idempotent on the SQL row.
  This matches the producer-side broadcast tick semantics (both
  branches emit the tick; the table is a `correlation_id`-keyed
  upsert).
- The call sites at `crates/forecast/src/tcn.rs:822` and `:937`
  already have `self.ledger.as_ref()` in scope. Wave A's task
  T-D-N5 adds the `post_forecast_event(l, …, cache_hit).await`
  call adjacent to the existing `tick::emit_public(...)` line —
  both fire on every emit.
- `strategy_id` and `symbol` are NOT on `ForecastOverlay` today;
  they live on the `ForecastRequest` passed into
  `TcnForecaster::predict(...)`. Wave A T-D-N5 plumbs them through
  the closest call frame (the existing emit-site scope holds
  `request.strategy_id` and `request.symbol`).

### §2.3 Reader signatures (NEW; Wave G consumes)

```rust
// crates/audit/src/query.rs — sibling of recent_signals.

/// Reconstruct the full upstream chain for one Fill audit row.
/// Returns `(fill_row, signal_row, forecast_row, debate_placeholder)`.
/// Any stage that has no SQL row surfaces as `None` (R3.4 empty-stage
/// rendering). Called on chevron-click first-open (R6.3 lazy
/// backfill).
///
/// Indexed lookups: 4 point queries against the indexes added by
/// mig 011 (H5 — p99 < 50 ms claim).
pub async fn trail_for_fill_id(
    ledger: &Ledger,
    fill_audit_id: &str,
) -> Result<TrailReconstruction, LedgerError> { … }

pub struct TrailReconstruction {
    pub fill: Option<JournalTxRow>,           // journal_transactions row
    pub signal: Option<StrategySignalRow>,    // strategy_signals row
    pub forecast: Option<ForecastEventRow>,   // forecast_events row
    pub debate: Option<()>,                   // future debate_events; always None at v0.1.0
}
```

## §3 — Trail-mirror location (T-T1-4): `crates/reflection`

**Decision: `crates/reflection/src/trail_mirror.rs`** (NOT `crates/ui`).

### Rationale

1. **Consumer-side symmetry.** The predecessor v0.1.0 already
   landed `crates/reflection/src/audit_tick_consumer.rs` as the
   first broadcast consumer. Putting the second consumer (Phase D's
   trail-mirror) next to it keeps the consumer-side surface area in
   one crate. The `ui` crate stays a thin presenter; the mirror
   logic (LRU cache, SQL backfill, broadcast subscription) is
   stateful and benefits from being unit-testable independently of
   the iced render path.
2. **Architecture invariant preserved.** ADR-0031 § Architecture
   invariants names `reflection → audit (via AuditTick stream)` as
   the new read-only edge. Phase D adds **no** new architecture
   edge — the trail-mirror lives behind the same edge. Locating it
   in `crates/ui` would add a new `ui → audit` direct edge, which
   the current architecture already disallows (the ui crate reads
   audit only via `audit::query::*` calls dispatched through an
   iced `Task::future` — never via direct broadcast subscription).
3. **Testability.** `crates/reflection` has no iced dep; trail
   reconstruction can be unit-tested with a stub broadcast sender
   and an in-memory SQLite without spinning up the cockpit binary.
   The matching `crates/ui` integration test (snapshot test for
   `trail__steady_state`) consumes the mirror via a feature-gated
   handle — `pub fn snapshot(&self) -> TrailMirrorSnapshot` — that
   the iced Subscription wrapper translates into a `Message`.
4. **Iced Subscription bridge.** The iced side adds a thin shim
   `crates/ui/src/state.rs::trail_mirror_subscription` that wraps
   `reflection::TrailMirror::stream` (returning a `Stream<Item =
   Message::TrailMirrorTick>`). This mirrors the existing async
   Subscription precedent at `state.rs:1213` (Phase B
   `Subscription::batch`).

### Module shape (locked)

```rust
// crates/reflection/src/trail_mirror.rs — NEW.

/// Phase D's first stateful broadcast consumer (R6.1-R6.3). Holds:
///
/// 1. A `tokio::sync::broadcast::Receiver` wrapped via
///    `audit::tick::AuditTickStream::new(rx, "ui_trail_mirror")`
///    (reuses the existing v0.1.0 Lagged-warn path).
/// 2. An LRU<UUID, ReconstructedTrail> capped at N=16 (R6.1, H4
///    falsification gate).
/// 3. A handle to `audit::Ledger` for SQL backfill on first open
///    (R6.3).
///
/// The mirror runs as a single tokio task spawned at cockpit
/// startup. The ui crate communicates with it via two channels:
///
/// - **Request**: `mpsc::Sender<TrailMirrorRequest>` —
///   `{ Open(audit_id), Close(audit_id), Snapshot }`.
/// - **Response**: `broadcast::Sender<TrailMirrorTick>` consumed
///   by the iced Subscription shim.
pub struct TrailMirror { … }

pub enum TrailMirrorRequest {
    Open(SmolStr),    // audit_id from row click → backfill + cache + subscribe
    Close(SmolStr),   // chevron-collapse → eviction hint (LRU bound enforces ultimate eviction)
    Snapshot,         // ui idle-refresh → mirror sends current snapshot
}

#[derive(Debug, Clone)]
pub enum TrailMirrorTick {
    Reconstructed { audit_id: SmolStr, trail: ReconstructedTrail },
    LiveUpdate { audit_id: SmolStr, stage: TrailStage }, // broadcast-driven mutation
}
```

## §4 — Per-wave change-site index

Every code change in Phase D lands at one of the sites below. Wave
ordering matches `tasks.md`. Each `T-D-N` task references back to
this section for the file:line pin.

### Wave A — Mig 011 + audit writers (T-D-N1..N5)

| #  | File                                                        | Change                                            |
|----|-------------------------------------------------------------|---------------------------------------------------|
| W1 | `crates/audit/migrations/011_trail_correlation_chain.sql`   | NEW — exact SQL from §2.1                         |
| W2 | `crates/audit/src/journal.rs:74-244` (existing `post_fill`) | Refactor to `post_fill_with_signal(.., None)` wrapper; new sibling at L73 carries the signal_id |
| W3 | `crates/audit/src/journal.rs:293`                           | Extend `post_strategy_signal` signature (+ forecast_correlation_id) per §2.2; update bind block at L339-355 |
| W4 | `crates/audit/src/journal.rs:~440` (post-tick block)        | NEW `post_forecast_event` writer per §2.2         |
| W5 | `crates/forecast/src/tcn.rs:822-831, 937-947`               | Add `post_forecast_event(l, …, cache_hit).await` adjacent to existing `tick::emit_public(...)` call; both emit sites |

**Anchor gate after Wave A:** developer runs
`scripts/verify_anchors.sh` — 22/22 PASS is the entry condition for
Wave B. H2 falsifies here if it falsifies anywhere.

### Wave B — `widgets::trail_node` (T-D-N6..N8)

| #  | File                                            | Change                                                |
|----|-------------------------------------------------|--------------------------------------------------------|
| W6 | `crates/ui/src/widgets/trail_node.rs`           | NEW — `pub fn view(node, selected, mode) -> Element<'_, Message>`; pure render |
| W7 | `crates/ui/src/widgets/mod.rs`                  | `pub mod trail_node;`                                  |
| W8 | `crates/ui/src/state.rs` (Message enum block)   | Add `Message::TrailNodeChevronClicked(TrailNodeKind)`  |

### Wave C — `screens::trail` (T-D-N9..N12)

| # | File                                                  | Change                                              |
|---|-------------------------------------------------------|------------------------------------------------------|
| W9  | `crates/ui/src/screens/trail.rs`                    | NEW — `view(model, mode)` delegates to `audit::view` in list mode (R2.2 byte-identity gate) |
| W10 | `crates/ui/src/screens/mod.rs`                      | `pub mod trail;`                                    |
| W11 | `crates/ui/src/state.rs:84`                         | Promote `Screen::Audit` from deprecated-alias status to a deprecated alias pointing at `Screen::Trail`; add new active variant `Trail` (R2.4) |
| W12 | `crates/ui/src/state.rs:~1549` (`update_screen` dispatch) | Route `Screen::Trail` → `screens::trail::view`; route `Screen::Audit` alias to same |

### Wave D — `widgets::trail_drawer` + state (T-D-N13..N15)

| # | File                                                  | Change                                              |
|---|-------------------------------------------------------|------------------------------------------------------|
| W13 | `crates/ui/src/widgets/trail_drawer.rs`             | NEW — `pub fn view(payload, mode) -> Element<'_, Message>`; renders Fill / Signal / Forecast / LLM-placeholder bodies per R4.2 |
| W14 | `crates/ui/src/state.rs` (Cockpit struct + Message enum) | Add `trail_screen_state: TrailScreenState { selected_audit_id: Option<SmolStr>, drawer_selected_node: Option<TrailNodeKind>, lru: LruCache<SmolStr, ReconstructedTrail> }`; add `Message::SelectTrailRow(SmolStr)` (internal) + `Message::TrailDrawerClosed` |
| W15 | `crates/ui/src/state.rs` (`update` fn arms)         | Handle `TrailNodeChevronClicked`, `SelectTrailRow`, `TrailDrawerClosed` |

### Wave E — Live agent-feed chevron (T-D-N16..N18)

| # | File                                                  | Change                                              |
|---|-------------------------------------------------------|------------------------------------------------------|
| W16 | `crates/ui/src/widgets/agent_feed.rs:49-97` (ready_body) | Add per-row Trail chevron button adjacent to existing transparent row button at L62-65; chevron emits `Message::OpenTrailFor(audit_id)` |
| W17 | `crates/ui/src/state.rs` (Message enum + update)    | Add `Message::OpenTrailFor(SmolStr)`; update arm expands compound: `SelectTrailRow(id)` + `SwitchScreen(Trail)` per R5.1 (Phase C `OpenStrategyInLab` precedent at `state.rs:822, 2489-2498`) |
| W18 | `crates/ui/src/screens/audit.rs:316` (table_body)   | Add per-row Trail chevron sibling of the existing per-row Button; chevron also emits `Message::OpenTrailFor` (R5.2). Mutual-exclusivity is iced layout-order based |

### Wave F — Trail-mirror consumer + TCN runtime wiring (T-D-N19..N22)

| # | File                                                  | Change                                              |
|---|-------------------------------------------------------|------------------------------------------------------|
| W19 | `crates/agent/src/config.rs` (`StrategiesConfig`)    | Add `pub tcn_overlay_momentum: TcnOverlayConfig { pub enabled: bool }` with `Default::enabled = false` (mirrors `signal_log` precedent at L284-287) |
| W20 | `crates/strategy/src/tcn_overlay_momentum.rs:307+`  | New builder `TcnSyncForecaster::with_ledger(ledger)` per §1.2; new `TcnOverlayMomentumStrategy::with_tcn_bs1_ledger(base, ledger)` mirror of `with_tcn_bs1` at L348-352 |
| W21 | `crates/agent/src/runtime.rs:141`                   | New sibling `build_registry_with_ledger(cfg, ledger)` per §1.2; call from `crates/agent/src/main.rs:184-186` (paper-mode only — backtests keep calling `build_registry(cfg)`) |
| W22 | `crates/reflection/src/trail_mirror.rs`             | NEW module per §3; mirrors `audit_tick_consumer.rs:30-32` subscription shape; LRU<UUID, ReconstructedTrail> capped at 16 |

### Wave G — Snapshots + integration + perf-gate (T-D-N23..N28)

| # | File                                                  | Change                                              |
|---|-------------------------------------------------------|------------------------------------------------------|
| W23 | `crates/audit/src/query.rs` (new fn)                | `pub async fn trail_for_fill_id(...)` per §2.3      |
| W24 | `crates/ui/src/state.rs` (Subscription bridge ~L1213) | New `trail_mirror_subscription` returning `Stream<TrailMirrorTick>` mapped to `Message::TrailMirrorTick`; wired into `Cockpit::subscription` batch |
| W25 | `crates/ui/tests/snapshot/`                         | Add 3 baselines per M-FINAL T-F3: `trail__steady_state`, `trail__side_drawer_open`, `live__recent_activity_with_chevron` |
| W26 | `crates/audit/tests/` (integration)                 | Round-trip: write `Fill+Signal+ForecastEvent` → `trail_for_fill_id` returns all 3 stages populated, debate=None |
| W27 | `crates/ui/src/state.rs` (test mod)                 | Round-trip test per M-FINAL T-F8: `OpenTrailFor(uuid)` → `current_screen == Trail && trail_screen_state.selected_audit_id == Some(uuid)` |
| W28 | `crates/reflection/src/trail_mirror.rs` (bench)     | H5 backfill-latency benchmark: SQLite p99 first-open trail reconstruction < 50 ms at ≥10⁵ audit rows |

## §5 — Anchor-preservation invariant proof sketch

Why are the 22 anchors safe by construction?

1. **Schema is additive only.** Mig 011 adds 2 NULL columns to
   `journal_transactions`, 1 NULL column to `strategy_signals`, and
   1 new table. Pre-mig rows surface NULL on every new column; no
   `UPDATE` against pre-existing data. Mirrors mig 008 / 009 / 010
   precedent (each anchor-safe in turn).
2. **Anchored reports never read the new columns.** The 22
   body-SHA-256 anchors live in `spec/anchors.toml`; the bodies are
   hashed by `scripts/hash_report.py` and the body-content is
   sourced from `crates/reports::render_*`. None of the renderers
   reference `journal_transactions.fill_id`,
   `journal_transactions.signal_id`,
   `strategy_signals.forecast_correlation_id`, or
   `forecast_events.*`. Storage-only is the precedent (mig 008's
   `venue` column followed the same shape).
3. **Writers are signature-backwards-compat.** `post_fill` is
   preserved verbatim as a thin wrapper; `post_strategy_signal`
   grows a 7th `Option<Uuid>` arg and every existing caller in
   non-test code passes `None` (grep confirms zero non-test callers
   outside `audit::tests` at this checkpoint).
4. **Backtest ledger never arms the tick bus.** Backtests call
   `Ledger::open` (`ledger.rs:33`), which leaves
   `tick_bus = None`. The `tick.rs:104-107` `let Some(bus) = … else
   { return };` guard returns early. New writers' tick-emit calls
   are no-ops in backtest mode. The 22 anchors are
   backtest-rendered → anchor stability holds regardless of how
   many writers grow tick-emit calls.

H2 falsification path: a single anchor diverging post-Wave A
falsifies (3) or (4). Wave A's exit gate is exactly
`scripts/verify_anchors.sh → 22/22 PASS`. No subsequent wave touches
the audit-write surface, so a clean Wave A propagates.

## §6 — Open architecture follow-ups (NOT in scope)

- **Debate-events table.** R1.5 reserves the placeholder; an
  LLM-strategy follow-up brief lands the `debate_events` table and
  the writer that stamps a `correlation_id` matching
  `forecast_events.correlation_id`. v0.1.0 ships the trail-mirror
  empty-state render only.
- **BS-2 ledger wiring.** §1.2 wires BS-1 only; BS-2 wiring is a
  trivial copy of the BS-1 path inside
  `build_registry_with_ledger`. Deferred to a follow-up because the
  v0.1.0 K7 smoke gate (M-FINAL T-F7) is BS-1-only.
- **Replay-cache body persistence.** R4.2 explicitly excludes the
  full sample distribution from the drawer (Forecast node renders
  `direction / confidence / model_revision / cache_hit` only).
  Surfacing the replay-cache body bytes is an architecture call —
  the replay cache currently lives outside the audit ledger
  (`forecast.rs:148-160`) and routing those bytes through the
  ledger would invalidate the H2 anchor argument. Deferred.

## §7 — Library / crate compatibility checklist

Phase D adds **zero new external crates** (R7.6). All four
deliverables ride on existing deps:

| Need                          | Existing crate                                | Verified           |
|-------------------------------|-----------------------------------------------|--------------------|
| LRU cache                     | `lru` (already a transitive dep — check)      | NEEDS-VERIFY in Wave F |
| SQLite migration              | `sqlx` (mig 011 mirrors mig 008-010 shape)    | ✓                  |
| Broadcast subscription        | `tokio::sync::broadcast` (predecessor)        | ✓                  |
| iced Subscription bridge      | iced 0.13+ (already at lock-step)             | ✓                  |
| Uuid binding                  | `uuid` v1.x (already pulled by audit)         | ✓                  |
| Decimal-as-TEXT               | `rust_decimal` (ADR-0003)                     | ✓                  |
| RFC3339 with µs precision     | `time` v0.3 (ADR-0004)                        | ✓                  |

**LRU dependency check (Wave F entry gate).** Developer must verify
`lru` is in the workspace tree before adding the import. Fallback:
hand-rolled `IndexMap` + manual eviction (16 entries — cheap). The
spec/architecture.md library compatibility checklist in this section
is the source-of-truth.

## Changelog

- 2026-05-20 (architect, M-T1): Initial decomposition. K5 spike →
  SUCCESS (full TCN runtime wiring path); mig 011 SQL shape locked
  per §2; trail-mirror location pinned to `crates/reflection` per §3.
  Per-wave change-site index (§4) drives `tasks.md` T-D-N rows.
  Operator-decided defaults (Q1-Q5) baked in.
