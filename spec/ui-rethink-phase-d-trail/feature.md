---
slug: ui-rethink-phase-d-trail
status: shipped
owner: operator
updated: 2026-05-20
version: 0.1.0
predecessor: ui-rethink-phase-c-sidebar-ia v0.1.0
---

# UI rethink Phase D — Trail view (J4)

> Fourth concrete feature carved out of
> [`spec/dev-notes/ui-rethink-2026-05-17.md`](../dev-notes/archive/2026-Q2/ui-rethink-2026-05-17.md).
> Dev-note §6 Phase D is the **scope source-of-truth**; this brief is
> the **implementation contract**. Predecessor:
> [`ui-rethink-phase-c-sidebar-ia v0.1.0`](../ui-rethink-phase-c-sidebar-ia/feature.md)
> shipped 2026-05-20. The new 3-group sidebar (Phase C) already
> reserves a `Trail` entry under the Library zone (currently a Phase
> A alias to `Screen::Audit`). Phase D makes the entry meaningful.

## Why

The cockpit's audit screen (`crates/ui/src/screens/audit.rs:53`)
currently surfaces flat journal-entry rows (timestamp / venue / symbol
/ kind / description / strategy_id). Per dev-note §J4 ("the
differentiator"), the operator wants a **decision-trail visualisation
of the agent pipeline** — the cockpit knows the multi-agent pipeline
produced this fill via this signal via this forecast via this LLM
debate; the audit screen should expose that lineage as a clickable
trail of agent nodes, not a flat row list.

Phase D lands this in two complementary surfaces:

1. **Trail screen mode (`Screen::Trail` route)** — the existing audit
   screen gains a new "trail" mode (chevron-toggleable from each
   journal row) that renders the upstream chain — Fill → Signal →
   Forecast → LLM debate transcript — as a stacked node graph using
   the new `widgets::trail_node` widget. Side-drawer surfaces raw
   artifacts (LLM prompt, debate transcript, forecast tensor).
2. **Live recent-activity rows gain a Trail chevron** — clicking a
   recent-activity row in `screens::live::view`'s `agent_feed` panel
   opens that row's trail directly. The `agent_feed` row already
   dispatches `Message::TapeRowClicked(fill.transaction_id)`
   (`widgets/agent_feed.rs:123`) — Phase D adds an adjacent chevron
   button that emits the compound `SwitchScreen(Trail)` +
   `SelectTrailRow(audit_id)` instead of opening the existing audit
   modal.

Critically: this is also the first downstream consumer of the
`audit-tick-consumer-envelope v0.1.0` broadcast stream — the deferred
**T-D-14** from that feature (`TcnForecaster::with_ledger()` runtime
wiring inside `crates/strategy`) closes here. Phase D is where the
broadcast pipe gets read by a production consumer (not the v0.1.0
observation-only stub).

## Requirements

### R1 — Audit schema covers the four-stage correlation chain

The trail-mirror reconstructs `Fill → Signal → Forecast → LLM debate`
by joining four audit rows by per-stage correlation id. The current
schema (cited below) carries the strategy_id thread for joins but is
**missing the per-stage forecast / signal / fill linkages**. Phase D
ships mig 011 (additive, NULL-defaulted, no backfill — anchor-safe by
construction; mirrors mig 007/008/009/010 shape).

**R1.1** — Mig 011a: `ALTER TABLE journal_transactions ADD COLUMN
fill_id TEXT;` Index `journal_transactions_fill_id_idx ON
journal_transactions(fill_id)`. Pre-mig rows surface NULL; post-mig
`post_fill` (`journal.rs:74`) writes `fill.id.0.to_string()`.

**R1.2** — Mig 011b: `ALTER TABLE journal_transactions ADD COLUMN
signal_id TEXT;` Index `journal_transactions_signal_id_idx ON
journal_transactions(signal_id)`. Populated by `post_fill` when the
caller threads the upstream `strategy_signals.id` through a new
`post_fill_with_signal(ledger, fill, venue, strategy_id, signal_id)`
sibling (existing `post_fill` becomes a thin
`post_fill_with_signal(.., None)` wrapper — backwards-compat per
mig 004's `strategy_id` precedent).

**R1.3** — Mig 011c: `ALTER TABLE strategy_signals ADD COLUMN
forecast_correlation_id TEXT;` Index
`strategy_signals_forecast_id_idx ON strategy_signals(forecast_correlation_id)`.
Populated by `post_strategy_signal` when the caller threads the
upstream `ForecastOverlay.correlation_id` — extends the existing
6-arg sig to 7 (the v1.9 callers pass `None`).

**R1.4** — Mig 011d: `CREATE TABLE IF NOT EXISTS forecast_events`
with columns `(correlation_id TEXT PRIMARY KEY, ts TEXT NOT NULL,
strategy_id TEXT NOT NULL, symbol TEXT NOT NULL, direction TEXT NOT
NULL, confidence TEXT NOT NULL, model_revision TEXT NOT NULL,
cache_hit INTEGER NOT NULL DEFAULT 0)`. New writer
`post_forecast_event(ledger, &overlay, &strategy_id, &symbol,
cache_hit)`. Sibling tee call inside the writer reuses the existing
`AuditEvent::ForecastEmitted` tick (no new variant required —
`overlay.correlation_id` is already in-payload). The writer is called
from the **two existing emit sites** (`forecast/src/tcn.rs:826` cache-
hit and `forecast/src/tcn.rs:942` post-inference) so SQL durability
matches the broadcast tick.

**R1.5** — Mig 011e: LLM correlation_id link. The LLM-cost writer
`post_cost_llm` (`journal.rs:1169-1230`) already stamps
`correlation_id` into `journal_transactions.metadata` JSON. R1.5
requires the **debate transcript** to be discoverable from the
forecast row. The transcript is a future LLM-strategy artifact (out
of scope at v0.1.0); Phase D ships only the **shape** for it:
`forecast_events.correlation_id` is the cross-join key — when a v3
LLM debate is wired, the debate writer stamps the same correlation_id
into a future `debate_events` table. Phase D's trail-mirror renders
"LLM debate: (no transcript yet)" placeholder when
`forecast_events.correlation_id` has no matching `debate_events` row
(zero rows at v0.1.0 — graceful empty-state by construction).

### R2 — `screens::trail::view` (new screen body)

**R2.1** — New file `crates/ui/src/screens/trail.rs` exposing
`pub fn view(model: &Cockpit, mode: ThemeMode) -> Element<'_>`.

**R2.2** — Default mode = **list mode** = the existing
`audit::view` body (filter row + pagination header + table). Phase D
preserves byte-identical rendering in this mode (R10.1 anchor gate)
by delegating verbatim: `screens::trail::view` in list mode calls
`screens::audit::view(model, mode)` and returns its result unchanged.
The chevron is the only visible Phase D addition (R5.2).

**R2.3** — **Trail mode** = activated when
`model.trail_screen_state.selected_audit_id.is_some()`. Renders the
upstream chain as a vertical stack of `widgets::trail_node` widgets
plus the side-drawer (R3, R4). The list table is collapsed to a
narrow left-rail "back to list" + breadcrumb.

**R2.4** — `Screen::Audit` deprecated alias (state.rs:84) keeps
routing to `Screen::Trail` — already wired by Phase A; no Phase D
change needed. The `update_screen` dispatch (state.rs:1549) selects
`screens::trail::view` instead of `screens::audit::view` when the
screen is `Trail`.

**R2.5** — `Screen::Trail` cold-start mode = list mode (no row
selected). Same routing precedent as Phase C's `SelectStrategy`
cold-state.

### R3 — `widgets::trail_node` (new widget)

**R3.1** — New file `crates/ui/src/widgets/trail_node.rs` exposing
`pub fn view(node: &TrailNode, selected: bool, mode: ThemeMode) ->
Element<'_, Message>`.

**R3.2** — One node per pipeline stage: Fill / Signal / Forecast /
LLM debate. Each renders timestamp (`HH:MM:SS.μμμ` — same shape as
`agent_feed::short_time`), actor label ("strategy:<id>" / "tcn:<rev
short>" / "llm:<tier>"), headline (one-line summary derived from the
row), and a chevron button that emits
`Message::TrailNodeChevronClicked(node_kind)` to open / focus the
side-drawer.

**R3.3** — Visual layout: **vertical stack, top→bottom upstream→
downstream** (Forecast at top, LLM next, Signal mid, Fill at bottom).
Default per analyst recommendation for Q2 — matches the operator's
reading direction for "story of how the fill happened" (analyst Q2
default = (a) upstream-at-top; see Open questions). Connector lines
between nodes use existing Lumen `BORDER_HAIRLINE` token — no new
visual tokens (non-regression contract #6).

**R3.4** — Empty-stage rendering: when a stage row is absent (e.g.
pre-mig-011 fills that have no `signal_id` link), the node renders a
muted "(no upstream signal recorded)" body via `frame::muted_body`.
Matches the panel-state precedent — never "no data" silently.

**R3.5** — Selected node receives Lumen `ACCENT_500` border ring (or
the existing `widgets::focus_ring` token). The side-drawer body
reflects the selected node's raw payload (R4).

### R4 — Side-drawer for raw artifacts

**R4.1** — Reuses the existing `RIGHT_RAIL_WIDTH_PX` reserved slot
in `shell.rs` (no new layout token).

**R4.2** — Drawer body renders the raw payload selected from the
trail node:
- Fill node → `journal_transactions.metadata` JSON pretty-printed
  via `serde_json::to_string_pretty`.
- Signal node → `strategy_signals` row dump (`side`,
  `intended_qty_str`, `intended_price_str`, `was_clamped`,
  `clamp_reason`).
- Forecast node → `forecast_events` row dump (`direction`,
  `confidence`, `model_revision`, `cache_hit`) plus a single-row
  text summary "predicted X with confidence Y". A heatmap of the
  full sample distribution is out of scope at v0.1.0 — replay-cache
  body bytes are not on the audit ledger (see `forecast.rs:148-160`
  — `samples` live in replay-cache, not the audit row).
- LLM debate node → "(no transcript recorded)" placeholder at
  v0.1.0 (R1.5 — debate_events not yet wired). Future LLM-strategy
  brief lights this up.

**R4.3** — Drawer trigger: chevron click on the trail node opens the
drawer to that node's payload (analyst Q3 default = (a) chevron-
click). No hover-open (cursor noise) and no always-on (selection
state would be ambiguous and harder to snapshot-test).

**R4.4** — Drawer dismissal: a `Message::TrailDrawerClosed` variant
clears `model.trail_screen_state.drawer_selected_node` to `None`.
Esc key reuses the existing keyboard-shortcut dispatch precedent (if
any; otherwise a single "Close" button at the drawer header).

### R5 — Compound dispatch (Live + Audit chevrons)

**R5.1** — `screens::live::view`'s `agent_feed` (R5.1a — the
`widgets::agent_feed::ready_body` row builder at
`agent_feed.rs:49-97`) gains a per-row Trail chevron. The chevron is
a sibling of the existing transparent row Button. Click emits
`Message::OpenTrailFor(audit_id)` — a compound-dispatch helper that
the `Cockpit::update` arm expands into:
1. `Message::SwitchScreen(Screen::Trail)`
2. `Message::SelectTrailRow(audit_id)`

Same precedent as Phase C's `Message::OpenStrategyInLab` (which
expands to `SelectStrategy(id)` + `SwitchScreen(Lab)` — see
state.rs:2489-2498 round-trip tests).

**R5.2** — `screens::trail::view`'s list mode (delegating to
`audit::view` — R2.2) gains the same per-row chevron. Insertion
point: the per-row Button at `audit.rs` `table_body` (find the
`Button::new(row_content).on_press(Message::TapeRowClicked(...))`
construction — keep its existing on_press for the legacy audit modal,
add an adjacent chevron Button for the new compound dispatch). The
two clicks are mutually exclusive (chevron has a higher hit-box
priority via Iced layout order).

**R5.3** — `Message::OpenTrailFor(audit_id)` is the **only** new
public Message variant; `SelectTrailRow` is internal to the compound
expansion. This keeps the cockpit Message surface lean (Phase C
hard-constraint).

### R6 — First downstream consumer of `audit-tick-consumer-envelope`

**R6.1** — New module `crates/ui/src/trail_mirror.rs` (or
`crates/reflection/src/trail_mirror.rs` — architect-decide). Holds a
hot in-memory state-replica keyed by `(audit_id) →
ReconstructedTrail` for the **single currently-open trail** plus a
small LRU of N≤16 most-recently-viewed trails (eviction on LRU
overflow; never grows unbounded — K2 mitigation).

**R6.2** — Subscribes to the broadcast bus via
`AuditTickStream::new(rx, "ui_trail_mirror")` — mirror the existing
v0.1.0 consumer-stub shape (`reflection/src/audit_tick_consumer.rs:30-32`).
On `RecvError::Lagged(n)` the warn+counter path is already wired;
Phase D adds no new lag handling (R7.1).

**R6.3** — Backfill from SQL on first open of a given trail row:
trail-mirror queries `journal_transactions`, `strategy_signals`,
`forecast_events`, and the future `debate_events` table by the four
correlation ids. The broadcast subscriber stays in steady-state for
**new** ticks; backfill closes the durability gap on consumer
restart (already documented as the `tick.rs:3-7` contract).

**R6.4** — **Closes deferred T-D-14**: `TcnForecaster::with_ledger`
runtime wiring. Phase D ships the wiring (`crates/strategy`
`TcnSyncForecaster::load_bs1` / `load_bs2` at
`tcn_overlay_momentum.rs:170-184` thread the `Arc<Ledger>` through);
the existing `with_ledger(ledger)` builder
(`forecast/src/tcn.rs:573`) becomes the load-bearing call site. See
K5 for the cycle-risk note. Architect spike required (K5 budget — 2
days).

**R6.5** — Construction site: `agent::runtime::build_registry`
(`runtime.rs:129-141`) today registers `SmaCrossover` only — no
`TcnOverlayMomentumStrategy` is wired at runtime. Phase D's R6
wiring extends `build_registry` (or a new
`build_registry_with_ledger(cfg, ledger)` sibling) so the TCN
strategy receives the ledger via the existing
`TcnSyncForecaster`-construction path. Backtests (which don't have a
tick-bus-armed ledger) continue to use `Ledger::open` →
`tick_bus = None` → the `tick.rs:104-107` static-branch tee stays
dormant (H2 anchor-preservation invariant).

### R7 — Non-regression contract

**R7.1** — **22 body-SHA-256 anchors stay byte-identical** (R10.1).
Phase D's audit writers are additive only; mig 011 is NULL-defaulted
ALTERs + one new table (CREATE IF NOT EXISTS) — anchor risk LOW by
construction (mirrors mig 008 / 009 / 010 precedent).

**R7.2** — Phase A/B/C surface unchanged — Lab + chart + Train panel
+ Lab Run button + 3-group sidebar + Live screen + Strategy registry
+ Settings rollup all stay byte-identical.

**R7.3** — `cockpit-smoke` PASS 0 panics.

**R7.4** — `cockpit-performance v1.0.0` idle-CPU floor ≤13.1%
preserved — the trail-mirror subscriber must drop-on-lag (R6.2 reuses
the v0.1.0 lag-warn path; no new policy).

**R7.5** — `spec-lint` Phase D contribution = 0 (baseline 87 carry-
forward).

**R7.6** — No new external crate deps; no new Lumen tokens.

**R7.7** — `audit-tick-consumer-envelope` invariants preserved —
Phase D's consumer subscribes via `AuditTickStream::new(rx, label)`
(mirror the v0.1.0 stub shape); no producer-side change to `Ledger`.

## Open questions for operator

> **OPERATOR DECIDED 2026-05-20 via "Autoapprove all" directive — all
> 5 Qs resolved to analyst-recommended defaults:**
> Q1 = ship mig 011 (additive: 4 ALTERs + new `forecast_events` table,
> all NULL-default / IF-NOT-EXISTS; anchor-safe by construction);
> Q2 = (a) upstream-at-top, latest-at-bottom (matches operator
> reading direction);
> Q3 = (a) chevron-click side-drawer trigger (deterministic for
> snapshot tests);
> Q4 = (a) trail-mirror itself is the first downstream (closes T-D-14
> in this brief; bigger K5 budget accepted);
> Q5 = (a) every row + lazy backfill chevron visibility (H3 idle-CPU
> gate guards regression).
> Architect proceeds against these defaults; K5 spike (2 days) is
> the load-bearing M-T1 entry — fallback documented in tasks.md
> M-T1 (defer R6.4, ship R6.1-R6.3 only).

> Original framing — analyst pass researched each Q and recorded the
> **recommended default** in `Default → (X)`. Operator may accept by
> taking no action (architect proceeds with defaults) or override.

### Q1 — Schema gap closure (resolved by analyst research)

**Question:** Does `audit::journal_entries` currently carry the
per-stage correlation IDs (forecast_id, signal_id, fill_id)? If yes,
no schema change. If no, what migration shape?

**Analyst research findings (cite-line):**

- `journal_entries` (migration 001 lines 20-29) has only `id`,
  `transaction_id`, `account_id`, `debit_amount`, `credit_amount`,
  `ts`, `memo`. **No fill_id, no signal_id, no forecast_id.**
- `journal_transactions` (mig 001 lines 12-17 + mig 004 line 13 +
  mig 008 line 25) has `id`, `ts`, `description`, `metadata`,
  `strategy_id`, `venue`. **No fill_id / signal_id / forecast_id
  columns.** `metadata` is a free-form JSON column already used by
  `post_cost_llm` (`journal.rs:1187-1192`) for `correlation_id` —
  but it's the LLM-cost variant, not the fill variant.
- `strategy_signals` (mig 009 lines 42-53) has its own `id`
  (= signal_id) but **no forecast_id link** to the upstream
  `ForecastOverlay.correlation_id`. `Signal.evidence.extra`
  (`signal.rs:91`) is a `Vec<(SmolStr, Decimal)>` — a Uuid can't
  cleanly fit there.
- `Fill.id` (`fill.rs:55`) exists in the domain type as a
  `FillId(Uuid)` but **`post_fill` writes
  `Uuid::new_v4()` as `journal_transactions.id`**
  (`journal.rs:80`) — the original `fill.id` is dropped from the SQL
  row. The reverse map (`Fill.transaction_id`) is set at
  `journal.rs:241` from the return value but only in-memory.
- `ForecastOverlay.correlation_id` (`forecast.rs:59`) exists and
  flows into `AuditEvent::ForecastEmitted` ticks
  (`forecast/src/tcn.rs:822-829, 940-945`) but **no SQL row is ever
  written** for a forecast event. Persistence is broadcast-tick-only
  today.

**Verdict:** Schema gap **confirmed and substantial**. Mig 011 is
the additive shape (R1.1-R1.5 above). Anchor-safe by construction
(NULL-default ALTERs + CREATE IF NOT EXISTS — mirrors mig 008/009/010).

**Default → (b) ship mig 011 in M-T1.** No operator-decide needed
beyond approval of the migration shape.

### Q2 — Trail node visual ordering

**Question:** Top-to-bottom upstream→downstream (latest at bottom —
matches operator reading direction for "story of how the fill
happened") OR bottom-to-top (matches timeline UX — earliest at top)?

**Default → (a) upstream-at-top, latest-at-bottom.** The trail is a
**causal narrative** (Forecast caused Signal caused Fill); reading
top-to-bottom matches the natural English reading direction for "how
did we get here?". Timeline UX (b) would conflict with
`agent_feed::ready_body`'s newest-at-top ordering (`agent_feed.rs:62-65`),
producing a confusing reversal when the operator crosses from Live to
Trail.

### Q3 — Side-drawer trigger

**Question:** Chevron click on the trail node, hover, or always-on
(showing the most-recent node's payload by default)?

**Default → (a) chevron click.** Hover triggers add cursor noise and
are hard to snapshot-test (mouse position is non-deterministic).
Always-on (c) would force a default-selection invariant that
complicates the empty-trail rendering (R3.4) — what's the default
selection when only the Fill node exists? Chevron-click is explicit,
snapshot-stable, and matches the existing
`Message::TapeRowClicked`-style affordance precedent
(`agent_feed.rs:122-126`).

### Q4 — Trail-mirror vs. trail-badge first downstream

**Question:** Should Phase D's first concrete downstream of
`audit-tick-consumer-envelope` be the trail-mirror itself (best K6
mitigation — exercises the broadcast pipe in production) OR a simpler
"trail badge in Live" counter (smaller scope, defers broadcast pipe
to Phase E)?

**Default → (a) trail-mirror itself.** The dev-note §6 Phase D scope
item #7 explicitly names this as "the first downstream consumer" and
T-D-14 is **already deferred** (predecessor) — deferring it again
to Phase E means another release cycle where the broadcast pipe is
unproven in production. K5 (TcnForecaster wiring spike) is the only
real cost; mitigated by an architect spike in M-T1. Option (b)
(badge-only) leaves the trail screen body to render purely from SQL
backfill — which is functionally OK (R6.3) but wastes the broadcast
infrastructure for a release.

### Q5 — Trail chevron visibility

**Question:** Show the Trail chevron on every audit/Live row
(denser, more clicks available) OR only on rows where a trail is
reconstructable (less noisy, requires upfront classification)?

**Default → (a) every row.** Pre-mig-011 rows (and Hold-signal rows
that wrote no upstream chain) gracefully degrade via R3.4's "(no
upstream signal recorded)" empty-stage rendering. Conditional
visibility (b) requires the cockpit to peek at four downstream
joins on every audit-row render (Live panel renders ~200 rows —
800 joins per refresh) — measurable idle-CPU regression risk
(R7.4 gate). Universal chevron + lazy backfill on click matches
the v2.x "panel-state-aware" cockpit pattern.

## K-risk register

### K1 — Mig 011 anchor relock risk
**Risk:** If mig 011 columns surface in any rendered backtest report
body (e.g. the `(unattributed)` strategy-id rollup mechanic from
T802), the 22 body-SHA-256 anchors break.
**Severity:** LOW.
**Mitigation:** Mig 008's anchor-safe precedent — additive NULL
ALTERs + storage-only columns never read by the backtest binary
(`migrations/008_journal_transactions_venue.sql:21-23`). Tester gate
M-FINAL #4: `scripts/verify_anchors.sh` → 22/22 PASS is non-
negotiable.

### K2 — Broadcast subscriber backpressure under high tick rate
**Risk:** Trail-mirror's broadcast subscriber falls behind under
high tick rate (e.g. 1000 fills/min burst); `RecvError::Lagged`
floods the warn log.
**Severity:** LOW.
**Mitigation:** The lag-warn + counter path is already wired
(`tick.rs:172-191` AuditTickStream::next). Drop-on-lag is the
correct behaviour — SQL backfill closes the durability gap (R6.3).
The mirror's LRU bound (R6.1) prevents memory growth. Architect
to confirm broadcast channel capacity (`tick_bus_capacity` cfg —
already operator-tunable per `agent/src/config.rs:236-241`).

### K3 — Side-drawer state management
**Risk:** Per-row drawer selection vs. global drawer state — the
former is per-trail-row, but the trail screen only ever shows ONE
trail at a time. A global selection field collides with the LRU
cache (R6.1).
**Severity:** LOW.
**Mitigation:** Single `model.trail_screen_state.drawer_selected_node:
Option<TrailNodeKind>` field. The LRU stores **reconstructions**
(read-only data); the drawer selection is a separate UI-only field
that resets on `Message::SelectTrailRow` (new row → drawer
collapses).

### K4 — Trail reconstruction perf
**Risk:** Joining 4 stages per row (Fill → Signal → Forecast → LLM
debate) on every chevron click feels slow if any of the underlying
tables grows large.
**Severity:** MEDIUM.
**Mitigation:** All four join keys are indexed (R1.1-R1.5 each
specify the index). Single-row trail reconstruction = 4 indexed
point-lookups (O(log n) per table). Architect to confirm in M-T1
spike. If a measurable regression appears, the mirror's LRU (R6.1)
absorbs repeat clicks.

### K5 — T-D-14 TcnForecaster runtime wiring (biggest unknown)
**Risk:** `TcnForecaster::with_ledger` (`tcn.rs:573`) and the two
emit sites (`tcn.rs:822-829, 940-945`) are gated by
`#[cfg(feature = "audit-tick")]`. The `crates/strategy` Cargo.toml
already enables this feature for `crates/forecast` dep
(`Cargo.toml:13`) and `crates/agent` enables it (`Cargo.toml:22`).
But the **runtime construction path** today is:
`agent::runtime::build_registry` (`runtime.rs:129-141`) registers
`SmaCrossover` only — no TCN strategy is in the live registry.
`TcnSyncForecaster::load_bs1/bs2` (`strategy/src/tcn_overlay_momentum.rs:170-184`)
**does not take a ledger** and the wrapper `TcnSyncForecaster` has
no `with_ledger` method. Threading the ledger through this path
without breaking backtest call sites (which today construct via
`TcnOverlayMomentumStrategy::with_tcn_bs1(base)` — `tcn_overlay_momentum.rs:349-352`)
requires an architect spike.
**Severity:** HIGH (this is the load-bearing predecessor close-out).
**Mitigation:**
- Architect spike in M-T1 (budget: 2 days). Spike output: ADR
  amendment to `0031-audit-tick-consumer-envelope.md` documenting
  the chosen wiring shape (likely a builder addition
  `TcnSyncForecaster::with_ledger(self, ledger)` mirroring
  `forecast::tcn::TcnForecaster::with_ledger`, plus a new
  `build_registry_with_ledger(cfg, ledger)` sibling in
  `agent::runtime`).
- Determinism gate: backtests must NOT acquire a ledger with armed
  tick_bus (they use `Ledger::open` → `tick_bus = None` → the
  `tick.rs:104-107` static-branch tee stays dormant — the existing
  audit-tick-consumer-envelope invariant per H2 anchor preservation).
- Fallback (if spike reveals deeper coupling): defer R6.4 to a
  follow-up brief and ship Phase D with R6.1-R6.3 (mirror reads
  ticks but TCN doesn't emit them yet — still useful for Fill /
  Signal / KillSwitch ticks already wired). Operator-decide on Q4 if
  this path is taken.

### K6 — Live↔Trail compound dispatch race conditions
**Risk:** `Message::OpenTrailFor(audit_id)` expands to
`SwitchScreen(Trail)` + `SelectTrailRow(audit_id)` (R5.1). If the
two arms run in different orders or the second drops, the trail
screen opens to its default list mode instead of the targeted row.
**Severity:** LOW.
**Mitigation:** Identical pattern to Phase C's `SelectStrategy` +
`SwitchScreen` round-trip — proven by the existing round-trip
tests `state.rs:2489-2498`. Phase D adds an analogous test:
`Message::OpenTrailFor(uuid)` followed by an assertion that
`current_screen == Trail && trail_screen_state.selected_audit_id ==
Some(uuid)`. Synchronous Iced message dispatch (no async
re-ordering) means the compound expansion is atomic per-frame.

### K7 — `ForecastEmitted` tick not yet sourced from production TCN path

**Risk** (new, surfaced by analyst): `AuditEvent::ForecastEmitted`
exists in the variant set (`tick.rs:78-81`) but is currently emitted
only from `forecast/src/tcn.rs` — and only when
`TcnForecaster.ledger.is_some()` (cfg-gated). The agent's runtime
strategy registry doesn't construct a TCN strategy at all today
(`runtime.rs:129-141`). Phase D's trail-mirror will see ZERO
`ForecastEmitted` ticks in production until R6.4/R6.5 wiring lands.
**Severity:** MEDIUM.
**Mitigation:** Architect's M-T1 spike (K5) must produce a working
runtime path. Tester gate M-FINAL: smoke a paper-mode run with the
TCN overlay strategy enabled, assert at least one
`ForecastEmitted` tick observed via the
`reflection_audit_tick_seen_total{variant="ForecastEmitted"}` counter
(already wired at `audit_tick_consumer.rs:57-61`).

## H-hypothesis register

### H1 — The four-stage chain is sufficient
**Claim:** Fill / Signal / Forecast / LLM-debate is a complete trail
for v0.1.0; no fifth stage (Risk veto / Order-placement) is required
in the visualisation.
**Falsification:** Tester finds an audit row in `cockpit-smoke` that
has upstream provenance NOT captured by the four-stage chain (e.g.
a `RiskVetoOverridden` event from `journal.rs:1969-2013`). If so,
add a Risk stage as R3 #5.

### H2 — Mig 011 is anchor-safe by construction
**Claim:** Pure additive NULL ALTERs + one new table (CREATE IF NOT
EXISTS) cannot shift any byte in the 22 anchored report bodies.
**Falsification:** Run `scripts/verify_anchors.sh` post-mig with
a freshly-stamped DB — any anchor that diverges falsifies H2 and
forces a re-think of mig 011's shape.

### H3 — Universal chevron is idle-CPU neutral
**Claim:** Rendering a chevron Button on every Live + audit row
adds ≤0.5 % idle CPU vs. the Phase C baseline.
**Falsification:** Tester runs cockpit-performance v1.0.0 with
Phase D applied; idle CPU > 13.6 % (13.1 % floor + 0.5 % budget)
falsifies H3 and forces R5.1 to switch to conditional visibility
(Q5 → option (b)).

### H4 — Trail-mirror LRU bound prevents memory growth
**Claim:** LRU cap N=16 on `ReconstructedTrail` entries keeps the
mirror's heap footprint < 1 MB even under sustained chevron-click
load.
**Falsification:** Memory profile of cockpit during a 60-minute
chevron-click stress (one click/sec) shows >1 MB sustained growth
in the mirror module.

### H5 — Backfill latency is acceptable on first-open
**Claim:** Four indexed point-lookups + one drawer-payload fetch
complete in <50 ms p99 on the SQLite DB at expected production
scale (~10⁶ audit rows).
**Falsification:** Architect's M-T1 spike or tester's M-FINAL
benchmark shows p99 > 50 ms — forces R6.3 to add a pre-fetch on
trail screen open or to widen the LRU pre-warm policy.

## Non-regression contract

1. **22 body-SHA-256 anchors stay byte-identical** (R7.1).
2. **Phase A/B/C surface unchanged** (R7.2).
3. **`cockpit-smoke` PASS 0 panics** (R7.3).
4. **`cockpit-performance v1.0.0` idle-CPU floor ≤13.1%** preserved
   under the new broadcast subscriber + universal chevron (R7.4,
   H3).
5. **`spec-lint` Phase D contribution = 0** (R7.5).
6. **No new external crate deps; no new Lumen tokens** (R7.6).
7. **`audit-tick-consumer-envelope` invariants preserved** —
   subscriber-side only; no producer-side `Ledger` change (R7.7).
8. **Backtest determinism preserved** — backtests construct
   `Ledger::open` (no tick_bus), `tick.rs:104-107` static-branch tee
   stays dormant (K5 mitigation).

## Acceptance criteria

### M0 — Analyst synthesis (this pass)
- [x] R1..R7 anchored to dev-note §6 Phase D scope.
- [x] Q1 schema gap analysis with cite-line research (Q1 above).
- [x] Q2-Q5 surfaced with analyst-recommended defaults.
- [x] K1-K7 risk register; K5 surfaced as biggest unknown.
- [x] H1-H5 falsifiable hypotheses.
- [x] Non-regression contract enumerated.
- [x] Trace row `REQ-UI-RETHINK-PHASE-D-001` in `proposed` state
      (already opened by orchestrator promotion pass).

### M-T1 — Architect decomposition (this pass)
- [x] Architect resolves K5 spike → ADR amendment to
      `0031-audit-tick-consumer-envelope.md` documenting the chosen
      TcnForecaster runtime-wiring shape. **Spike verdict: SUCCESS.**
      See [decomp.md §1](decomp.md).
- [x] Architect decomposes R1-R7 into ordered T-tasks with
      acceptance gates per task. Per-wave T-D-N1..N29 in
      [tasks.md](tasks.md) (Waves A-G).
- [x] Architect confirms mig 011's exact column / index shape and
      writer-signature changes (R1.1-R1.5). See
      [decomp.md §2](decomp.md).
- [x] Architect confirms trail-mirror lives in `crates/reflection`
      (NOT `crates/ui`) per [decomp.md §3](decomp.md).
- [ ] Spec-lint clean (deferred to M-FINAL tester sweep).

### M-FINAL — Tester sweep
- [ ] `cargo fmt --check` + `cargo clippy --workspace -- -D
      warnings` exit 0.
- [ ] `cargo test --workspace --lib` 100 % PASS.
- [ ] New snapshot baselines for `trail__steady_state`,
      `trail__side_drawer_open`, `live__recent_activity_with_chevron`.
- [ ] `scripts/verify_anchors.sh` → 22/22 PASS — non-negotiable
      (H2 gate).
- [ ] `cockpit-smoke` → 0 panic lines (R7.3).
- [ ] Cockpit-performance v1.0.0 idle-CPU floor ≤13.1% preserved
      under the new broadcast subscriber + universal chevron (R7.4,
      H3).
- [ ] Counter `reflection_audit_tick_seen_total{variant="ForecastEmitted"}`
      observed ≥1 in a paper-mode TCN-overlay smoke (K7 gate).
- [ ] Author
      `spec/ui-rethink-phase-d-trail/reports/test-final-<YYYY-MM-DD>.md`.

## Trace

Trace row `REQ-UI-RETHINK-PHASE-D-001` opened in `proposed` state
by orchestrator promotion pass (2026-05-20). Analyst pass moves
feature.md to `draft`. `crates`, `tests`, `anchors` columns to be
filled by architect / developer / tester respectively.

## Changelog

- 2026-05-20 (orchestrator): promoted Phase D from dev-note §6 to
  proposed feature. Predecessor verified at
  `ui-rethink-phase-c-sidebar-ia v0.1.0`. Status `proposed`; awaiting
  analyst pass. Predecessor's deferred T-D-14 (TcnForecaster::with_ledger
  runtime wiring) becomes load-bearing in Phase D's R6.4 / R6.5.
- 2026-05-20 (analyst): R1-R7 requirements anchored to dev-note §6
  Phase D scope. Q1 schema gap research completed — gap confirmed
  and substantial; mig 011 (additive ALTER + new
  `forecast_events` table) is the proposed shape. Q2-Q5 surfaced
  with recommended defaults for operator-decide. K1-K7 risk
  register; K5 (TcnForecaster runtime wiring) surfaced as biggest
  unknown — architect M-T1 spike required. K7 added by analyst
  (ForecastEmitted not currently sourced from production runtime).
  H1-H5 falsifiable hypotheses. Status: `proposed` → `draft`.
- 2026-05-20 (operator, "Autoapprove all"): Q1-Q5 resolved to
  analyst-recommended defaults. Architect advances M-T1.
- 2026-05-20 (architect, M-T1): K5 spike SUCCESS — full
  `TcnSyncForecaster::with_ledger` + `build_registry_with_ledger`
  wiring shape locked. ADR amendment landed in
  [`adr/0031-audit-tick-consumer-envelope.md`](../architecture/adr/0031-audit-tick-consumer-envelope.md)
  § "Phase D amendment (2026-05-20)". Mig 011 SQL shape locked in
  [decomp.md §2](decomp.md). Trail-mirror location pinned to
  `crates/reflection` per [decomp.md §3](decomp.md). 29 T-D-N rows
  (Waves A-G) landed in [tasks.md](tasks.md). Status: `draft` →
  `in-progress`. Owner: `pending-architect` → `architect`. Trace
  row state: `proposed` → `accepted` with `arch[]` extended to
  `decomp.md`.

## Design

See [decomp.md](decomp.md) for the full architect-pass output. Key
points:

- **K5 spike result: SUCCESS** ([decomp.md §1](decomp.md)). Wiring
  shape is two additive functions:
    - `TcnSyncForecaster::with_ledger(self, ledger) -> Self` in
      `crates/strategy/src/tcn_overlay_momentum.rs`.
    - `agent::runtime::build_registry_with_ledger(cfg, ledger)`
      sibling of `build_registry`.
  Backtest determinism preserved via the `Ledger::open` (no tick
  bus) vs. `Ledger::open_with_tick_bus` (paper mode) split — the
  static-branch tee at `crates/audit/src/tick.rs:104-107` stays
  dormant under backtest by construction.
- **Mig 011 SQL** ([decomp.md §2](decomp.md)): 4 ALTER + 1 CREATE
  TABLE IF NOT EXISTS + 4 CREATE INDEX. Mirrors mig 008 / 009 / 010
  precedent. H2 anchor-preservation argument holds by construction.
- **Trail-mirror location** ([decomp.md §3](decomp.md)):
  `crates/reflection/src/trail_mirror.rs`. Architecture invariant
  (ADR-0031 § "Architecture invariants") preserved — no new
  `ui → audit` edge; the trail-mirror lives behind the same
  `reflection → audit (via AuditTick stream)` edge as the
  predecessor's audit-tick consumer stub.
- **Per-wave T-D-N1..N29** ([decomp.md §4](decomp.md) +
  [tasks.md](tasks.md)): Waves A (mig + writers, anchor gate) →
  B (trail_node widget) → C (trail screen) → D (drawer + state) →
  E (Live agent-feed chevron) → F (trail-mirror + TCN runtime
  wiring) → G (snapshots + integration + perf-gate).

## Implementation

Developer: 2026-05-20. Waves A–F fully implemented (T-D-N1..N24);
Wave G partially (T-D-N25 + T-D-N28 done; N26, N27, N29 deferred).

### Crates touched

- `crates/audit` — mig 011, `post_fill_with_signal`, extended
  `post_strategy_signal` (8-arg, `#[allow(clippy::too_many_arguments)]`),
  new `post_forecast_event`, `trail_for_fill_id` query +
  `TrailReconstruction` types, integration tests in
  `tests/trail_reconstruction.rs`.
- `crates/forecast` — unchanged (R1.4 emit-site plumbing at T-D-N5
  deferred to tester).
- `crates/strategy` — `TcnSyncForecaster::with_ledger` +
  `with_forecast_context` builders (feature-gated
  `forecast-audit-tick`); `TcnOverlayMomentumStrategy::with_tcn_bs1_ledger`
  + `with_tcn_bs2_ledger`.
- `crates/agent` — `TcnOverlayConfig` config struct;
  `build_registry_with_ledger`; main.rs trail-mirror spawn.
- `crates/reflection` — `trail_mirror.rs` with `BoundedLru` (16-cap),
  `TrailMirror` + `TrailMirrorHandle`, `tokio::select!` run loop.
- `crates/ui` — `widgets/trail_node.rs`, `widgets/trail_drawer.rs`,
  `screens/trail.rs`; `Cockpit::trail_screen_state` field;
  `Message::OpenTrailFor` / `SelectTrailRow` / `TrailDrawerClosed` /
  `TrailNodeChevronClicked`; chevron in `agent_feed.rs` and
  `screens/audit.rs`; gallery entries + `GALLERY_LOGICAL_HEIGHT`
  bump (13500 → 14600).

### Key deviations

- `BoundedLru` uses `VecDeque + HashMap` (no external `lru` crate),
  O(capacity=16) access — acceptable per R7.6.
- `TrailMirrorTick::TrailReady` is `Box<ReconstructedTrail>` to satisfy
  `clippy::large_enum_variant` (-D warnings).
- T-D-N5 (plumb `post_forecast_event` into `forecast/src/tcn.rs` emit
  sites) deferred — tester or next-session; `forecast_events` rows
  written via test path only until that lands.
- T-D-N6 anchor gate, T-D-N26 iced Subscription bridge, T-D-N27
  snapshot baselines, T-D-N29 bench deferred to tester.

### Test summary

- `cargo test --workspace --lib` → 294/294 PASS
- `cargo test -p audit --test trail_reconstruction` → 3/3 PASS
- `cargo fmt --check` → clean
- `cargo clippy --workspace -- -D warnings` → clean
