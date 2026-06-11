---
adr: 0052
title: Durable live equity series — audit-ledger table, mint-site mode gate, and the hydration as_of contract
status: accepted
date: 2026-06-11
supersedes: none
superseded-by: none
---

# ADR-0052: Durable live equity series — audit-ledger table, mint-site mode gate, and the hydration `as_of` contract

## Context

The cockpit Live screen's equity curve is session-scoped: the agent
keeps only a scalar equity (`ReconcilerState::equity()`), and the UI
accumulates a `live_equity_buffer: VecDeque<(Timestamp, Money<Usdt>)>`
(cap 2880) that is never serialized and starts empty on every boot.
Quit `cockpit_live`, reopen it, and the curve is blank until the agent
re-trades. This was a documented deferral: `cockpit-live-dashboard-wiring`
(v0.1.2) resolved its D1 to "UI-accumulate a session series" and named
this exec-side follow-on (`live-equity-history-durable`) to make the
series durable.

The decision space is loaded by three hard, already-shipped invariants
that this ADR must design around, not relitigate:

1. **The two-timestamp contract (approach A, 2026-06-11).** Each
   `PnlSnapshot` carries BOTH `as_of: Timestamp` (wallclock
   `Timestamp::now()`, the delivery/freshness key the UI guard uses) AND
   `bar_ts: Option<Timestamp>` (the bar close time the chart plots).
   Conflating them — stamping `as_of = bar.close_ts` — broke the live
   render and was reverted in `40f5de9`. `crates/core/src/views.rs`,
   `reconciler.rs::after_bar_close`, and
   `runtime.rs::spawn_research_trading_loop` all document and uphold this.

2. **`push_live_equity_point` is the single UI append guard**
   (`state.rs:1374`): a delivery guard on `as_of` (drop strictly-earlier
   deliveries), a monotone clamp on the stored x-coordinate, a 2880-cap
   ring, and the `is_all_absent` ≥2-point KPI trap.

3. **The render-verifiable harness is the gate**, not unit tests
   (`crates/ui/tests/live_equity_render.rs`; MEMORY.md *Verify UI at the
   render layer*).

The persistence introduces a new durable write surface to an audit
ledger that anchors 19 backtest body-SHA-256 reports — so anchor-safety
is a first-class correctness obligation, and "every external I/O behind
a trait" applies.

## Decision

### D1 — The durable series lives in the audit SQLite ledger, reached through a persistence trait

A new additive table `equity_snapshots` rides the existing
`Arc<audit::Ledger>` handle/pool. The store is reached through a
`LiveEquityStore` trait (the external-I/O-behind-a-trait boundary) so
tests fake it and the production impl is swappable. The audit ledger is
the durable store the project already runs with single-writer discipline
(the agent owns the only `Ledger` write handle), crash-consistent commit
(sqlx/SQLite), an additive-migration precedent (`010_training_events`),
the `query → Task::perform → message` hydrate seam (mirrors the Memory /
Models cold-boot tasks in `cockpit_live.rs`), and the nightly
ledger-snapshot backup story (`08-recovery-and-backups.md`). A flat
append-only sidecar file and an agent-owned JSON state file were both
weighed and rejected (see Alternatives).

### D2 — The write is minted in the agent at the per-bar site, gated `mode != Research`, fire-and-forget

The persistence call lives at the snapshot **mint** site — the
reconciler (`after_bar_close`) and the research-replay loop
(`spawn_research_trading_loop`) — NOT in the UI. The UI only reads
(hydrates); this keeps the audit single-writer invariant intact. The
write is gated `config.mode != Research` at the mint site so research
replay never even writes a row: research restarts the 2023 replay each
boot, so its `bar_ts` ranges repeat, and persisting them would produce an
overlapping/duplicate, meaningless hydrated series. This mode gate is the
single load-bearing correctness line of the feature. The write is
**per-bar, fire-and-forget**: a write error logs and continues, never
blocking and never panicking the trading loop — exactly the tolerance the
backtest path already grants `bus = None`. The headless `trading` bin
(same `runtime::run`, no UI) persists too — desirable: the history should
not depend on a UI being attached.

### D3 — The migration is purely additive (`CREATE TABLE IF NOT EXISTS`), so the 19 anchors are byte-safe by construction

`013_equity_snapshots.sql` is `CREATE TABLE IF NOT EXISTS` + indexes —
no `ALTER`, no backfill, no `UPDATE` on any pre-existing row — following
the `010` precedent verbatim. Columns:
`(id, ts, bar_ts, as_of, total_equity, cash, realized, unrealized, mode)`.
Money is Decimal-as-TEXT (ADR-0003); `ts`/`bar_ts`/`as_of` are RFC3339
6-digit-fractional-second (ADR-0004). The backtest binary instantiates
the reconciler with `bus = None` and never touches this table, so the
backtest report bytes are unchanged. This ADR adds NO row to
`spec/anchors.toml` and changes none of the 9 anchor SHAs.

### D4 — Hydration seeds `live_equity_last_as_of` from the max hydrated `as_of`; the writer is monotone-bounded by wallclock

On boot in paper/live mode, a boot-time `audit::query::equity_snapshot_tail`
→ `iced::Task::perform` → a new batch `Message::PnlHydrated(Vec<…>)` arm
seeds `live_equity_buffer` through (or exactly mirroring)
`push_live_equity_point`: each historical row's plotted x-coordinate is
its persisted `bar_ts`, and the buffer is rebuilt once. The delivery
guard is reconciled by seeding `live_equity_last_as_of` from the **max
hydrated `as_of`**, NOT leaving it `None`:

- Historical `as_of` values are wallclock stamps from prior sessions, all
  `≤ now()`. The first LIVE snapshot after a hydrate carries a fresh
  `as_of = Timestamp::now()`, which is `≥` every hydrated `as_of`, so it
  passes the guard and lands. Seeding from the max (rather than `None`)
  additionally drops a late/duplicate re-delivery of an already-hydrated
  historical row — the guard's purpose.
- **Backwards-clock edge.** The guard compares wallclock `as_of` values.
  Persisted `as_of` is the *minting* wallclock; if the machine clock moved
  backwards across a restart, a fresh live `as_of` could be < the max
  hydrated `as_of` and be dropped until wallclock catches up. We accept
  this. It is bounded (live bars are ~1/min; the curve simply does not
  extend until the clock passes the stale max), it cannot corrupt or
  reorder the stored series (the x-coordinate is `bar_ts`, independent of
  `as_of`), and it never panics. The alternative — keying the guard on
  `bar_ts` — was already tried and reverted (it conflates the delivery key
  with the plotted coordinate and re-introduces the fast-replay drop bug).
  A clock that runs backwards across a paper/live restart is an operational
  fault to be surfaced by host monitoring, not papered over in the UI
  guard. (Stated explicitly so the tester does not file it as a defect.)
- **`is_all_absent` interaction.** When ≥2 rows hydrate, the rebuild
  yields a `Ready` KPI strip immediately (the ≥2-point trap clears); ≤1
  row keeps it `Loading`. The curve renders from ≥1 point.
- **Cap bound.** The hydrate query `LIMIT`s to ≤2880 (the buffer cap), so
  a hydrate can never exceed the ring.

### D5 — Return baseline is "since inception"; retention is age-capped, aligned with the 30-day ledger snapshot horizon

Because the hydrated curve may legitimately span multiple sessions, the
Live KPI strip's Total-return is measured from the **first buffered point
(account inception)**, with an honest caption (a new `LIVE_*` string, the
`LIVE_SESSION_RETURN_CAPTION = "Session to date"` precedent — e.g.
`"Since inception"`). No new `core` math: Max-DD stays the live-derived
fraction ×100; Sharpe / CAGR / Win-rate stay `—` (unchanged from the
wiring feature's D2). Retention is a hard age/row-capped `DELETE WHERE ts
< …` purge task mirroring the nightly ledger-backup task, aligned with the
30-day ledger-snapshot retention; the UI only ever reads the last ≤2880
rows via the query `LIMIT` (and downsamples at query time if a future
sub-minute cadence ever exceeds the cap).

### Mode-switch-mid-deployment note (the research↔paper transition)

If an operator switches a deployment's mode between boots
(research→paper or paper→research), the gate behaves correctly by
construction: a paper boot persists forward from that point; a research
boot persists nothing and hydrates nothing (it keeps the session-scoped
curve). A paper boot that follows earlier paper boots hydrates the
accumulated paper history. There is no run-id keying in v0.1.0, so two
*different* paper deployments pointed at the same ledger would interleave
their series — out of scope (a `run_id`-keyed series + run picker is named
as a separate future feature, not bundled).

## Alternatives considered

- **(b) Flat append-only file (CSV/parquet) beside the ledger** — rejected
  for v0.1.0: introduces a *second* durable writer with its own
  fsync/rotation/partial-write discipline the live path does not have, plus
  a new file-reader + retention mechanism with no existing precedent. It is
  the named if-budget-tightens fallback only (it avoids the schema change
  entirely), but it is strictly higher new-surface than reusing the ledger.
- **(c) Agent-owned sidecar JSON state file** — rejected: worst of both —
  a bespoke writer AND a poor append/retention shape (whole-file rewrite,
  torn-write risk, no natural compaction).
- **Reconciler replays the persisted tail through the `pnl` bus on boot**
  (instead of a UI query) — rejected: couples the agent to the UI's
  hydrate need and re-emits historical snapshots as if live, muddying the
  freshness/latency semantics the `as_of` key exists to protect.
- **Replay N `PnlRefreshed` for hydration** (instead of one batch
  `PnlHydrated`) — rejected: re-derives the curve N times on boot; the
  batch arm rebuilds once and makes "hydrate" explicitly distinct from a
  live tick.
- **Persist research-replay equity, keyed by `run_id`** — rejected for
  v0.1.0: the repeating 2023 `bar_ts` ranges make an un-keyed series
  meaningless, and a `run_id`-keyed series needs a new column + a run
  picker UI; named as a separate feature.
- **Backfill history from the existing journal** (reconstruct equity from
  fills) — rejected: re-deriving equity from the double-entry ledger
  (cost-basis replay, fee attribution) is its own correctness problem;
  v0.1.0 persists the reconciler's already-computed equity forward only.
- **Leave `live_equity_last_as_of = None` after hydrate** — rejected: a
  late/duplicate re-delivery of an already-hydrated historical row would
  re-append; seeding from the max hydrated `as_of` is the correct guard.

## Consequences

- **Anchor safety is by construction (D3).** If a future change makes the
  migration non-additive or routes the backtest reconciler through the
  table, the `rust-validate` / anchor gate goes red. AC7 asserts the
  19-anchor count is byte-unchanged.
- **The mode gate (D2) is the duplication-prevention guarantee.** AC2
  asserts research mode writes zero rows; if the gate moves to the UI or is
  dropped, research replay pollutes the series and AC2 fails.
- **The hydrated boot is render-gated (D4).** AC6 extends
  `live_equity_render.rs`: a cockpit hydrated from a faked store tail must
  rasterize a non-empty `ACCENT` polyline (`count ≥ CURVE_DREW_MIN_ACCENT`,
  `x_span ≥ CURVE_X_SPAN_MIN`) with zero live snapshots delivered. A
  model-Ready-but-blank-canvas regression fails here. AC5 proves the first
  post-hydrate live append still lands (the D4 `as_of` contract).
- **Every external I/O is behind the `LiveEquityStore` trait (D1).** The
  fake impl drives AC1/AC2/AC4/AC5 without a real DB; the real impl is
  selected in the `cockpit_live` / headless boot.
- **No new dependency.** The store reuses the existing `audit`/sqlx stack;
  the `ui` crate keeps its no-direct-sqlx boundary by delegating the
  hydrate query to an `audit::query` helper (the Memory cold-boot
  precedent). No money `f64` anywhere — `Decimal`/`Money<Usdt>` throughout
  (ADR-0003), timestamps RFC3339-micros (ADR-0004).
- **Not a strategy/sizing feature.** No decision variable, no overlay; the
  CLAUDE.md baseline-equity-divergence e2e gate does NOT apply (stated
  explicitly, per the `cockpit-live-dashboard-wiring` precedent).

## Changelog
- 2026-06-11 (architect): initial accept. D1 audit-ledger table behind a
  `LiveEquityStore` trait; D2 mint-site write gated `mode != Research`,
  fire-and-forget; D3 additive `013` migration, 19 anchors byte-safe by
  construction; D4 hydrate seeds `live_equity_last_as_of` from max hydrated
  `as_of` (backwards-clock edge accepted + documented); D5 since-inception
  return + age-capped retention. Resolves feature Q1–Q7.
