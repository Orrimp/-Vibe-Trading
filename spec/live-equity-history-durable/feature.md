---
slug: live-equity-history-durable
status: shipped
owner: tester
updated: 2026-06-11
version: 0.2.0
trace: REQ-LIVE-EQUITY-HISTORY-001
---

# Durable live equity history — survive `cockpit_live` restart

## Changelog

- 2026-06-12 (orchestrator): **T6b — purge scheduling wired**, closing the
  ADR-0052 D5 deferral (operator-ratified "wire the hook"). `purge_older_than`
  on the `LiveEquityStore` trait + `spawn_equity_purge_task` (nightly, boot
  catch-up first tick, fire-and-forget) spawned only where rows are minted
  (paper/live, store = Some). Proven by `fake_store_purge_removes_old_keeps_recent`
  + `equity_purge_task_boot_tick_trims_past_horizon`; anchors 119/119.
- 2026-06-11 (ui-designer): UI track landed (T7-contract, T8, T9, T7). Added
  `Message::PnlHydrated` batch arm + `live_equity_hydrated` honesty flag
  (T7-contract, A4/A5 — guard seeded from MAX hydrated `as_of`); extended the
  render harness with the AC6 hydrated-boot + AC5 post-hydrate-live-append pixel
  gates (T8); added `LIVE_SINCE_INCEPTION_CAPTION` + the mode-correct caption
  (T9, R6); wired the boot hydrate seam + `RunHandles.equity_store` write store
  in `cockpit_live.rs` (T7, gated `mode != Research` via
  `should_hydrate_equity_on_boot`). Filled `## UI`. Non-live gates green
  (`--lib` 447, `--test live_equity_render` 7, `--test panel_snapshots` 103);
  the `--features live` gate is blocked on the parallel developer's in-flight
  `agent`/`audit` compile (T2–T5) — re-run when exec lands. Zero new theme
  token; one new string.
- 2026-06-11 (architect): v0.2.0 — added `## Architecture` (A1–A7 resolving
  Q1–Q7) + drafted [ADR-0052](../architecture/adr/0052-durable-live-equity-series.md)
  (registered in the ADR README same pass). A1 audit-ledger `equity_snapshots`
  table behind a `LiveEquityStore` trait; A2 mint-site write gated
  `mode != Research`; A3 additive `013` migration (19 anchors byte-safe by
  construction) + since-inception return + age-capped retention; A4 hydrate via
  `query → Task::perform → batch PnlHydrated`, guard seeded from MAX hydrated
  `as_of` (backwards-clock edge + mode-switch-mid-deployment accepted +
  documented); A5 batch arm; A6 per-bar fire-and-forget; A7 headless bin
  persists. Status → architect-done. Tasks refined into M-DEV waves (exec ‖ UI
  parallel once T1+T7-contract land). No disagreement with the analyst's
  recommended defaults.
- 2026-06-11 (analyst): initial draft (v0.1.0). Scoped from repo-root
  `TODO.md` item #3 — the last open cockpit Live-view item. Honors the
  `cockpit-live-dashboard-wiring` D1=(b) deferral framing: the
  session-scoped UI buffer there was the correct proportionate ship; THIS
  feature is the named exec-side follow-on it deferred. R1–R7, AC1–AC9,
  Q1–Q7 below; recommended defaults attached to every open question.

## Why

The cockpit Live screen's equity curve is **session-scoped**: it starts
empty on every `cockpit_live` boot and grows only as the agent trades the
current session. There is no durable equity *series* anywhere — the agent
keeps a scalar equity (`ReconcilerState::equity()`); the UI accumulates a
`live_equity_buffer: VecDeque<(Timestamp, Money<Usdt>)>` (cap 2880 = 48 h
of 1-min bars) that is **NOT serialized** and is initialized empty in every
`Cockpit` constructor. Quit the cockpit, reopen it, and the curve is blank
until the agent re-trades.

This was a **deliberate, documented deferral**. The
`cockpit-live-dashboard-wiring` feature (shipped, v0.1.2) resolved its
**D1=(a)** to "UI-accumulate a session-scoped series" and explicitly
**rejected D1=(b)** — a durable agent/exec-side equity history — as "a
larger exec-side change (~60% exec / 40% UI, re-scopes the feature to **L**)
outside this UI-wiring feature's scope. **Deferred to a named follow-on:
`live-equity-history-durable` (exec-side, unscheduled)**". This feature IS
that follow-on. (See `spec/cockpit-live-dashboard-wiring/feature.md` § Design
D1, and the same § Out of scope first bullet.)

### What is settled and MUST NOT be reopened (inherited constraints)

The Live equity render path went through ~6 round-trips this session
(`TODO.md` item #1 + the reverted I1). The hard-won invariants below are
**locked** — the architect MUST design around them, not relitigate them:

1. **The two-timestamp contract (approach A, 2026-06-11) is settled.** Each
   `PnlSnapshot` carries BOTH `as_of: Timestamp` (wallclock
   `Timestamp::now()`, the **delivery / freshness** key) AND `bar_ts:
   Option<Timestamp>` (`#[serde(default)]`, the bar/data **close time** the
   chart plots on its x-axis). `crates/core/src/views.rs` documents this in
   full; `crates/agent/src/{reconciler.rs::after_bar_close,
   runtime.rs::spawn_research_trading_loop}` both stamp both fields. **Do
   NOT propose re-conflating `as_of` and `bar_ts`** — that exact change
   (stamping `as_of = bar.close_ts`) broke the curve and was reverted in
   `40f5de9`. The UI delivery guard keys on `as_of` monotonicity; the
   plotted coordinate is `bar_ts ?? as_of` (`state.rs:2039`).

2. **The UI buffer's append guard is `push_live_equity_point`**
   (`state.rs:1374`): a delivery guard on `as_of` (drop strictly-earlier
   deliveries), a monotone clamp on the stored x-coordinate (so
   `EquitySeries::from_points` can never error), a 2880-cap ring, and the
   `is_all_absent` 1-point KPI trap (the KPI strip stays `Loading` until ≥2
   points because a 1-point series is byte-identical to the all-absent
   sentinel; the curve renders from ≥1). Hydration MUST flow through (or
   exactly mirror) this guard — see R4.

3. **The render-verifiable harness is the gate, not unit tests.**
   `crates/ui/tests/live_equity_render.rs` rasterizes the real Live screen
   via `iced_test::screenshot` and counts the curve's `ACCENT` polyline
   pixels, with a self-proving pair that confirms it distinguishes a broken
   curve from a healthy one. Project law (MEMORY.md *Verify UI at the render
   layer*): cockpit/UI changes verify at the rendered layer. The
   hydrated-boot render MUST be gated here (R5 / AC6), not only at the model
   layer.

### Not a strategy or sizing feature — say it plainly

This is a **read-only monitor persistence** feature. It runs no strategy,
computes no sizing, and introduces no decision variable. Per CLAUDE.md the
**baseline-equity-divergence e2e gate applies to strategy overlays / sizing
modifiers** — this is neither, so that gate does **NOT** apply here (stated
explicitly, exactly as `cockpit-live-dashboard-wiring` § Acceptance criteria
and § Backtest Scenarios did). The relevant non-negotiables that DO apply:
every external I/O behind a trait; `Decimal` / `Money<Usdt>` for money,
never `f64`; anchored reports in `spec/*/reports/` are byte-immutable
(this feature adds a migration that MUST be anchor-safe — see R3 / AC7).

## Requirements

The running agent persists its per-bar equity series to a durable store in
**paper/live mode**; on `cockpit_live` boot the Live screen's equity curve
+ KPI strip are **hydrated** from that store so the curve is non-empty
before the first new bar — while **research-replay mode stays
session-scoped** (persisting replayed 2023 equity would duplicate/overlap
series across boots).

- **R1 — Durable per-bar equity series (paper/live only).** When the agent
  runs in **paper or live mode**, every per-bar `PnlSnapshot` the reconciler
  publishes is also **persisted** to a durable store (Q1) keyed by a
  monotone timeline. The persisted row carries at minimum `(bar_ts, as_of,
  total_equity)` and SHOULD carry `(cash, realized, unrealized)` (Q3). The
  write is single-writer (the agent process) and crash-consistent (Q1).

- **R2 — Research-replay mode stays session-scoped (no persistence).** In
  **research mode** the agent MUST NOT persist equity to the durable series
  (default — Q2). Research replay restarts the 2023 replay from scratch each
  boot, so its `bar_ts` ranges *repeat*; naively persisting them would
  produce overlapping/duplicate series and a meaningless hydrated curve. The
  mode discriminator already exists (`agent::config::Mode {Research, Paper}`,
  `config.rs:525`). The persistence writer is gated on `mode != Research`.

- **R3 — Persistence behind a trait; additive, anchor-safe schema.** The
  durable store is reached through a **trait** (per the "every external I/O
  behind a trait" non-negotiable) so tests fake it and the production impl is
  swappable. If Q1 resolves to the audit SQLite ledger, the schema change is
  a **purely additive `CREATE TABLE IF NOT EXISTS` migration** (the `010`
  precedent) — no `ALTER`, no backfill, no `UPDATE` on existing rows — so the
  19 anchored backtest body-SHA-256 report anchors are **byte-safe by
  construction** (the backtest binary instantiates the reconciler with `bus =
  None` and never touches this table). **AC7 gates this.**

- **R4 — Boot hydration seeds the UI buffer (delivery-guard-safe).** On
  `cockpit_live` boot in paper/live mode, the Live subscription **hydrates**
  `live_equity_buffer` from the durable store's tail (Q4) **before / as** the
  first live bar arrives, so the curve renders a real history immediately.
  Hydration MUST respect the settled invariants:
  - The historical rows seed the buffer's **plotted x-coordinate** from their
    persisted `bar_ts` (the chart x-axis), exactly as a live append does.
  - The `as_of` **delivery guard** (`live_equity_last_as_of`,
    `state.rs:1385`) must not reject the *first live* snapshot after a
    historical hydrate. Historical `as_of` values are older than `now()`, so
    seeding `live_equity_last_as_of` from the **max hydrated `as_of`** (or
    leaving it `None` and letting the first live snapshot set it) keeps the
    live path monotone. The architect MUST pin which (Q4 sub-decision) and
    the render harness MUST prove the post-hydrate live append still lands.
  - Hydration seeds **enough points to clear the `is_all_absent` ≥2-point KPI
    rule** when ≥2 rows exist, so the KPI strip comes up `Ready` on boot
    rather than `Loading` (the curve already renders from ≥1).
  - The buffer cap (2880) bounds the hydrate: seed **at most the last 2880
    rows** (the store downsamples / the query `LIMIT`s — Q3 retention).

- **R5 — Hydrated boot verified at the render layer.** The hydrated-boot
  curve MUST be proven to rasterize by extending
  `crates/ui/tests/live_equity_render.rs` (or a sibling in the same file):
  construct a cockpit hydrated from a faked store tail, render the real Live
  screen, assert the `ACCENT` polyline drew (count ≥ `CURVE_DREW_MIN_ACCENT`,
  x-span ≥ `CURVE_X_SPAN_MIN`) **with zero live snapshots delivered yet**.
  Model-layer + text-summary assertions are necessary but **not sufficient**
  (the explicit lesson of this file's harness). **AC6.**

- **R6 — Mode-correct, honest, no overclaim.** The hydrated curve is a
  **real paper/live history** — it MAY legitimately span multiple sessions /
  days. Any caption stays honest (it is a continuous paper/live equity
  history, not a "characterized result" / not annualized). In research mode
  the curve remains the session-scoped one and any "history" affordance is
  absent or disabled. The KPI strip's session-return semantics under a
  multi-session hydrate are a defined decision (Q3 sub-point: is "return"
  measured from the **first hydrated point** = account inception, or from the
  **session open**? — recommend account-inception with an honest caption).

- **R7 — Retention / compaction is bounded and defined.** The durable store
  does not grow without bound. A retention horizon + downsampling rule is
  defined (Q3): paper/live at 1-min bars is 1440 rows/day; the UI only ever
  reads the last ≤2880 for the curve. The store keeps a defined horizon (e.g.
  full-resolution recent + downsampled older, or a hard row/age cap aligned
  with the 30-day ledger-snapshot retention in
  `spec/architecture/08-recovery-and-backups.md`).

### Out of scope (explicit)

- **Persisting research-replay equity** — deliberately excluded (R2 / Q2);
  the repeating 2023 `bar_ts` ranges make it meaningless. If a future feature
  wants per-replay-run history, it needs a `run_id`-keyed series (a different
  schema + a run picker UI) — name it separately, do not bundle.
- **Live Sharpe / CAGR / win-rate math** — still absent in `core`
  (unchanged from `cockpit-live-dashboard-wiring` D2). The hydrated KPI strip
  renders the same honest cards: Total-return + Max-DD live (now over the
  hydrated history), Sharpe / CAGR / Win-rate `—`. No new `core` math here.
- **Cross-machine / cloud equity sync** — `08-recovery-and-backups.md`
  defers off-site sync / `litestream` to the real-money follow-on. Local
  durable store only.
- **A new equity-history *screen* / multi-account picker** — this feature
  hydrates the existing Live curve; it does not add a history browser. A
  richer history UI is a separate feature.
- **Backfilling history from the existing audit journal** (reconstructing
  equity from fills/journal rows) — tempting (the data exists), but
  re-deriving equity from the double-entry ledger is its own correctness
  problem (cost-basis replay, fee attribution). v0.1.0 persists the
  reconciler's already-computed equity forward from feature-land; it does NOT
  backfill pre-existing sessions. (Flagged as a possible v0.2.0 — Q1 note.)

## Architecture findings (for the architect — analysis, not hand-waving)

### Q1 candidates weighed — where the durable series lives

Three candidate homes, weighed on single-writer discipline, crash
consistency, the UI read/hydrate path, and retention/compaction:

| Candidate | Single-writer | Crash consistency | UI read/hydrate path | Retention | Verdict |
|-----------|---------------|-------------------|----------------------|-----------|---------|
| **(a) New table in the existing audit SQLite ledger** | **Yes — one writer already exists.** The agent owns `Arc<audit::Ledger>` (`cockpit_live.rs:295`, constructed via `Ledger::open[_with_tick_bus]`); it is the single audit writer. A new `equity_snapshots` table rides that same handle/pool. | **Strong** — sqlx/SQLite gives atomic-commit + the project already snapshots the ledger nightly with 24h RPO (`08-recovery-and-backups.md`). | **Direct** — a `query.rs` reader (sibling of `recent_training_events` / `recent_fills_filtered`, same RFC3339 `[since,until)` window) returns the tail; `cockpit_live` already does `audit::query` → `Task::perform` → `PnlRefreshed` (the exact pattern at `state.rs:2022` "the binary's Subscription issues audit::query calls and routes the result back as PnlRefreshed"). | Additive migration (`010` precedent) + a `DELETE WHERE ts <` purge task (mirrors the nightly backup task). | **RECOMMENDED.** Reuses the established single-writer, the migration precedent, the query+`Task::perform` hydrate seam, and the backup story. |
| **(b) Flat append-only file (CSV/parquet) beside the ledger** | Possible but introduces a **second** durable writer with its own fsync/rotation discipline the project doesn't have for the live path. | Append-only is simple but partial-write/rotation handling is new bespoke code. | Needs a new file reader + parse path the UI/agent doesn't have for this shape (parquet readers exist for market data, not for an agent-written equity log). | Manual file rotation. | **Fallback.** Cheaper to reason about per-row, but spawns a new I/O surface + retention mechanism with no existing precedent; weaker than reusing the ledger. |
| **(c) Agent's own state dir (e.g. a JSON/sidecar the agent owns)** | Single-writer, but bespoke. | Whole-file rewrite risks torn writes unless done atomically (tmp+rename); a growing JSON is the worst shape for an append workload. | New reader; no precedent. | Whole-file → no natural compaction. | **Rejected.** Worst of both — bespoke writer AND a poor append/retention shape. |

**Recommendation: (a) the audit ledger, behind a trait (R3).** It is the
durable store the project already runs, with the single-writer discipline,
crash consistency, backup retention, additive-migration precedent (`010`),
and the `query → Task::perform → PnlRefreshed` hydrate seam all already in
place. **This is the durable, composable choice** — and per the
durable-over-quick rule it earns the `(Recommended)` tag (Q1). The architect
can PROVE it is anchor-safe (additive `CREATE TABLE IF NOT EXISTS`, backtest
reconciler uses `bus = None` and never writes the table), so the cheap-vs-
durable tension does not arise here — (a) is both the durable AND the
lowest-new-surface option.

**If-budget-tightens fallback:** (b) a flat append-only file. It avoids the
migration entirely and is per-row trivial, but it adds a second durable
writer + a bespoke retention path. Name it as the fallback only if the
operator wants to avoid any ledger-schema change.

### Single-writer reality (the crucial topology fact)

The persistence writer MUST be the **agent process**, NOT the UI. In
`cockpit_live` the agent and iced run in one process sharing one
`Arc<EventBus>` and one `Arc<audit::Ledger>` — but the **agent side-thread**
owns the runtime (`cockpit_live.rs:463` "Side thread: own the runtime, drive
`agent::runtime::run`"). The reconciler (`after_bar_close`) and the
research-replay loop (`spawn_research_trading_loop`) are where the snapshot
is *minted* — that is the correct, single write site. The UI only **reads**
(hydrates). This keeps the audit single-writer invariant intact: the UI never
writes the equity table. (Headless `trading` bin: same agent, same writer,
no UI — persistence happens there too, which is desirable.)

### The mode gate is the load-bearing correctness decision

`agent::config::Mode` (`config.rs:525`) is `{Research, Paper}` (live folds
into paper-style execution at this stage). The publish sites already branch
on `config.mode` (`runtime.rs:471`). The persistence call is gated `mode !=
Research`. This is the single most important line in the feature: it is what
prevents the repeating-2023-`bar_ts` duplication. The architect MUST place
the gate at the **mint** site (reconciler / trading loop), not at the UI, so
research replay never even writes a row.

### Effort honesty — touched crates + test surface

This is genuinely **~L**, exec-led (matches the wiring feature's D1=(b)
estimate of ~60% exec / 40% UI):

- **`crates/audit`** (if Q1=(a)): new migration `013_equity_snapshots.sql`
  (additive); a `journal.rs` writer (`post_equity_snapshot`, sibling of
  `post_training_*`); a `query.rs` reader (`recent_equity_snapshots` /
  `equity_snapshot_tail`, sibling of `recent_training_events`); a purge for
  retention (R7). All behind the persistence trait (R3).
- **`crates/agent`** (`reconciler.rs` + `runtime.rs`): call the persistence
  trait at the mint site, gated `mode != Research`. The reconciler already
  computes the snapshot; this adds a fire-and-forget durable write (must not
  block the trading loop; must not panic the loop on a write error — log +
  continue, mirroring the `bus = None` backtest tolerance).
- **`crates/core`**: likely **no change** — `PnlSnapshot` already carries
  every persisted field (`bar_ts`, `as_of`, `total_equity`, `cash`,
  `realized`, `unrealized`). If a dedicated row DTO is wanted it is a new
  `views.rs` struct (sibling of `TrainingEventRow`), no behavior change.
- **`crates/ui`** (`state.rs` + `live.rs`/`bin/cockpit_live.rs`): a hydrate
  path — a boot-time `audit::query` → seed `live_equity_buffer` via (or
  mirroring) `push_live_equity_point`; reconcile the `as_of` delivery guard
  with historical rows (R4 sub-decision). No new widget, no new theme token
  (the curve/strip are reused verbatim, exactly as the wiring feature).
- **Test surface:** the audit writer/reader round-trip (unit, in-memory
  `Ledger`); the mode-gate (research writes nothing / paper writes a row);
  the anchor-safety proof (AC7 — backtest anchors unchanged); the
  **hydrated-boot render** in `live_equity_render.rs` (R5 / AC6 — the gate);
  the post-hydrate-live-append-still-lands render (the delivery-guard
  reconciliation). Cockpit fixtures smoke stays green (no live feature → no
  hydrate → empty buffer → Loading, unchanged).

## Open questions for the architect

- **Q1 — Where does the durable series live?**
  - **(a) New table in the existing audit SQLite ledger, behind a
    persistence trait. (Recommended)** — durable + composable: reuses the
    single existing audit writer, the additive-migration precedent (`010`),
    the `query → Task::perform → PnlRefreshed` hydrate seam, and the nightly
    backup/retention story. Provably anchor-safe (additive
    `CREATE TABLE IF NOT EXISTS`; backtest reconciler writes nothing). No
    competing durable writer introduced.
  - **(b) Flat append-only file (CSV/parquet) beside the ledger** —
    *if-budget-tightens fallback.* Avoids any ledger-schema change and is
    per-row trivial, but adds a second durable writer + bespoke rotation/
    retention with no existing precedent.
  - **(c) Agent's own state dir (sidecar JSON)** — rejected: bespoke writer
    AND a poor append/retention shape (whole-file rewrite, torn-write risk).
  - **Default: (a).**

- **Q2 — Persist in research-replay mode?**
  - **(a) No — research stays session-scoped; persist only paper/live.
    (Recommended)** — research replays repeating 2023 `bar_ts` ranges each
    boot; persisting them overlaps/duplicates series into a meaningless
    hydrate. Gate the writer `mode != Research` at the mint site.
  - **(b) Yes, keyed by a per-replay `run_id`** — only if a future feature
    wants per-run replay history; needs a `run_id` column + a run-picker UI.
    Out of v0.1.0 scope. *(Durability does NOT favor (b) here — persisting
    junk-overlapping series is not "more durable", it is wrong. (a) is both
    the correct AND the durable answer.)*
  - **Default: (a).**

- **Q3 — Schema + retention + the "return" baseline.**
  - **Columns:** recommend `(id, bar_ts, as_of, total_equity, cash,
    realized, unrealized, mode)` — all money as Decimal-as-TEXT (ADR-0003),
    timestamps RFC3339-micros (ADR-0004), mirroring the `training_events`
    column conventions. `fill_count` is **out** (the UI's `live_fill_count`
    is already a session counter; a durable cumulative fill count is a
    separate concern). Persisting `cash/realized/unrealized` (vs
    `total_equity` only) is cheap and lets a future surface decompose P&L —
    **recommend persist all three** (durable-over-minimal).
  - **Retention:** recommend a hard age/row cap aligned with the 30-day
    ledger-snapshot retention; the UI reads only the last ≤2880 rows
    (`LIMIT`/downsample at query time so the hydrate never exceeds the buffer
    cap). A purge task mirrors the nightly backup task.
  - **Return baseline (R6):** recommend Total-return measured from the
    **first persisted point (account inception)** with an honest
    "Since inception" caption, NOT the session open — because once history is
    durable, "session" return is the less meaningful number. The architect
    pins the caption string (a new `LIVE_*` string, the wiring feature's
    `LIVE_SESSION_RETURN_CAPTION` precedent).
  - **Default: persist all three P&L components; age-capped retention;
    query-time `LIMIT 2880` + downsample; return = since-inception.**

- **Q4 — Boot-hydration mechanism + the `as_of` delivery-guard
    reconciliation.**
  - **Mechanism:** **(a) a boot-time `audit::query` at subscribe time →
    `Task::perform` → a `PnlHydrated(Vec<…>)` (or a batch of `PnlRefreshed`)
    that seeds the buffer. (Recommended)** — reuses the exact
    query→`Task::perform`→message seam `cockpit_live` already uses; the UI
    stays a pure consumer; no reconciler-replays-the-tail-through-the-bus
    coupling.
  - **(b) Reconciler replays the persisted tail through the `pnl` bus on
    boot** — rejected: couples the agent to the UI's hydrate need and
    re-emits historical snapshots as if live (muddies freshness/latency).
  - **`as_of` guard sub-decision (MUST be pinned):** seed
    `live_equity_last_as_of` from the **max hydrated `as_of`** so the first
    *live* snapshot (wallclock `now()`, strictly newer) still passes the
    guard, while a late/duplicate historical re-delivery is dropped. The
    historical rows seed the **plotted x-coord** from their `bar_ts`. The
    render harness (R5) MUST prove a live append after hydrate still lands
    and draws.
  - **`is_all_absent` interaction:** when ≥2 rows hydrate, build the KPI
    strip `Ready` immediately (clears the ≥2-point rule); ≤1 row → `Loading`
    (unchanged).
  - **Default: (a) query-at-subscribe + seed-guard-from-max-`as_of`.**

- **Q5 — Hydrate as one batch message or replay N `PnlRefreshed`?** A single
  `PnlHydrated(Vec<(bar_ts, as_of, equity)>)` arm that seeds the buffer in
  one mutation is cheaper and avoids N re-renders, but adds one message
  variant; replaying N `PnlRefreshed` reuses the existing arm verbatim but
  re-derives the curve N times on boot. **Recommend the batch arm** (one
  rebuild, explicit hydrate semantics distinct from a live tick).
  - **Default: batch `PnlHydrated` arm.**

- **Q6 — Write cadence / coupling to the trading loop.** The durable write
  is per-bar (low-frequency, minute bars). **Recommend a fire-and-forget
  write that never blocks or panics the trading loop** (a write error logs +
  continues, exactly as the backtest path tolerates `bus = None`). Does the
  architect want a small batched/buffered writer (flush every N bars) or a
  per-bar insert? Per-bar is simplest and the cadence is low; batching is a
  premature optimization unless a future sub-second config lands.
  - **Default: per-bar fire-and-forget insert behind the trait.**

- **Q7 — Headless `trading` bin behavior.** The headless `trading` bin runs
  the same agent/reconciler with no UI. In paper/live mode it WILL now
  persist equity (desirable — the history accrues whether or not the cockpit
  is open). Confirm this is intended (it is the natural consequence of gating
  at the mint site, and is the *right* behavior: the durable series should
  not depend on a UI being attached).
  - **Default: yes — persist from the agent regardless of UI attachment.**

## Acceptance criteria

Proportionate + testable. This is a **read-only monitor persistence**
feature (no strategy overlay / sizing math) → the CLAUDE.md
baseline-equity-divergence e2e gate does **NOT** apply (stated explicitly,
per the wiring feature's precedent).

- **AC1 — Paper/live mode persists a per-bar equity row.** An integration
  test driving the reconciler / trading loop in paper mode with a faked store
  (the trait, R3) asserts one durable row per bar with the expected
  `(bar_ts, as_of, total_equity, cash, realized, unrealized)` Decimal values.
- **AC2 — Research mode persists NOTHING.** The same loop in research mode
  writes zero rows to the durable store (the mode gate, R2). This is the
  duplication-prevention guarantee — asserted directly.
- **AC3 — Writer/reader round-trip.** A unit test (in-memory `Ledger` if
  Q1=(a)) round-trips: write N snapshots → read the tail → values + ordering
  (monotone `bar_ts`) survive losslessly (Decimal-as-TEXT, RFC3339-micros).
- **AC4 — Hydration seeds the buffer.** A headless test builds a cockpit,
  hydrates from a faked store tail of M (≥2) rows, and asserts
  `live_equity_buffer.len() == min(M, 2880)`, the curve is `Ready`, AND the
  KPI strip is `Ready` (≥2-point rule cleared) — all **before any live
  `PnlRefreshed`**.
- **AC5 — Post-hydrate live append still lands (delivery-guard
  reconciliation).** After hydrating, deliver one live `PnlRefreshed`
  (wallclock `now()`); assert it is appended (not dropped by the `as_of`
  guard) and the curve/KPI update. This pins the Q4 `as_of` sub-decision.
- **AC6 — Hydrated boot renders at the pixel layer (THE gate).** Extend
  `crates/ui/tests/live_equity_render.rs`: render the real Live screen of a
  hydrated cockpit (zero live snapshots) and assert the `ACCENT` polyline
  drew (`count ≥ CURVE_DREW_MIN_ACCENT`, `x_span ≥ CURVE_X_SPAN_MIN`). A
  model-Ready-but-blank-canvas regression fails here. (Project law: verify UI
  at the render layer.)
- **AC7 — Anchor-safe migration.** If Q1=(a): the new migration is additive
  (`CREATE TABLE IF NOT EXISTS`, no `ALTER`/backfill/`UPDATE`); the 19
  anchored backtest body-SHA-256 reports are **byte-unchanged** (the
  `rust-validate` / anchor gate stays green; the backtest reconciler uses
  `bus = None` and never writes the table). Explicit anchor-count assertion
  in the test report.
- **AC8 — Retention is bounded.** A test (or documented purge) shows the
  store does not grow without bound: the purge removes rows past the horizon
  and the hydrate query `LIMIT`s to ≤2880 (R7).
- **AC9 — Fixtures `cockpit` smoke unchanged + every I/O behind a trait.**
  The fixtures-mode cockpit (no `live` feature, no agent, no store) hydrates
  nothing → buffer empty → curve/strip `Loading`, no panic, within the
  existing smoke window — byte-identical to today. Review confirms the durable
  store is reached through a trait (R3); flag any new dep explicitly.

## Size estimate (S/M/L) + exec-vs-UI split

**Estimate: L**, exec-led — **≈ 60% exec (audit + agent), 40% UI** (matches
the `cockpit-live-dashboard-wiring` D1=(b) projection exactly). The decisive
facts:

- **New durable write surface** (migration + writer + reader + purge, all
  behind a trait) in `crates/audit` — the bulk of the work, with the
  anchor-safety proof obligation (AC7).
- **Mint-site wiring + mode gate** in `crates/agent` (`reconciler.rs` +
  `runtime.rs`) — small but correctness-critical (the gate is the whole
  duplication-prevention story).
- **Hydrate path** in `crates/ui` — moderate: a boot query →
  `Task::perform` → seed-buffer arm, plus the `as_of` delivery-guard
  reconciliation, gated by the render harness.
- **`crates/core`:** likely zero (the `PnlSnapshot` fields already exist); a
  row DTO is optional.

**Bottom line for the operator:** this is the honest L the wiring feature
deferred. The data the agent already computes per bar gets written to the
store it already runs (the audit ledger), gated to paper/live so research
replay can't pollute it, and read back on boot so the curve is no longer
blank on every restart — verified, as required, at the rendered pixel layer.

## Architecture

_Architect-owned (2026-06-11). Decisions A1–A7 resolve the analyst's
Q1–Q7. The full decision record is [ADR-0052](../architecture/adr/0052-durable-live-equity-series.md);
this section is the feature-local summary + the seam map the developer ‖
ui-designer execute against. The three settled invariants in § Why (the
two-timestamp contract, `push_live_equity_point`, the render harness) are
inputs, not open questions — the design routes through them._

### Verified crate-edge reality (the seams, with line anchors)

- **Mint site** — `crates/agent/src/reconciler.rs::after_bar_close`
  (`reconciler.rs:122`) already computes the snapshot and returns it; the
  research loop `runtime.rs::spawn_research_trading_loop` mints it inline
  (`runtime.rs:1115-1124`). `runtime::run` already branches on
  `config.mode` (`runtime.rs:471`: `Mode::Research` vs `Mode::Paper`).
  `RunHandles` carries `config: Arc<Config>`, `ledger: Arc<audit::Ledger>`,
  `bus` (`runtime.rs:91-113`) — the persistence handle threads here for
  BOTH the headless `trading` bin (`crates/agent` `[[bin]] name="trading"`)
  and `cockpit_live`, since both call `runtime::run`.
- **Store** — `crates/audit`: the journal-writer pattern
  (`journal.rs::post_training_*`, `post_strategy_signal`) and the reader
  pattern (`query.rs::recent_training_events` at `query.rs:1998`, RFC3339
  half-open window; `equity_curve_for_strategy` at `query.rs:1326` for the
  `Money<Usdt>` Decimal-as-TEXT round-trip) are the exact siblings to copy.
  Migrations stop at `012_llm_forecast.sql` → ours is `013`. The `010`
  migration header is the additive-anchor-safety template verbatim.
- **Hydrate seam** — `crates/ui/src/bin/cockpit_live.rs`: the boot-time
  cold-hydrate tasks (`memory_task` at `cockpit_live.rs:696-745`,
  `models_task` at `:754-761`, batched into `boot_task` at `:764`) are the
  exact mirror — a `#[cfg(feature = "live")]` `iced::Task::perform` that
  delegates the query to a crate-boundary helper (so `ui` keeps its
  no-direct-sqlx edge), is fail-soft (`Ok(vec![])` on a missing file), and
  fires a hydrate `Message`. `cfg.mode` is in scope at boot
  (`cockpit_live.rs:241-253`). The `Arc<audit::Ledger>` is in `AppState`
  (`:641`).
- **UI append target** — `crates/ui/src/state.rs::push_live_equity_point`
  (`state.rs:1374`), buffer/guard fields at `:1020`/`:1031`,
  `LIVE_EQUITY_BUFFER_CAP = 2880` (`theme.rs:805`), the `Message::PnlRefreshed`
  arm at `:2029`, the `Message` enum at `:1540`. Caption precedent
  `LIVE_SESSION_RETURN_CAPTION = "Session to date"` (`strings.rs:1789`).
- **`crates/core`** — **zero change** (confirmed): `PnlSnapshot` already
  carries `bar_ts, as_of, total_equity, cash, realized, unrealized`. A row
  DTO, if wanted, is a new `views.rs` struct (sibling of `TrainingEventRow`)
  — no behaviour change.

### A1 (→ Q1) — Durable store = the audit SQLite ledger, behind a `LiveEquityStore` trait. **ACCEPT (a).**

A new additive `equity_snapshots` table on the existing `Arc<audit::Ledger>`
handle, reached through a `LiveEquityStore` trait (the
external-I/O-behind-a-trait boundary; the production impl wraps the
`Ledger`, tests use a fake). Reuses the single existing audit writer, the
`010` additive-migration precedent, the `query → Task::perform → message`
hydrate seam, and the nightly-backup retention story. Provably anchor-safe
(A3). Fallback (b) flat file and rejected (c) sidecar JSON are recorded in
ADR-0052 § Alternatives. _No disagreement with the analyst — this is both
the durable AND the lowest-new-surface option._

### A2 (→ Q2) — Research-replay mode persists nothing. **ACCEPT (a).**

The writer is gated `config.mode != Research` at the **mint** site (NOT the
UI). Research replays repeating 2023 `bar_ts` ranges each boot; persisting
them overlaps/duplicates into a meaningless hydrate. This is the single
load-bearing correctness line. AC2 asserts research writes zero rows.

### A3 (→ Q3) — Schema, retention, and the return baseline.

- **Columns:** `(id, ts, bar_ts, as_of, total_equity, cash, realized,
  unrealized, mode)`. `id` = UUID v4 PRIMARY KEY (every audit table's
  shape). Money = Decimal-as-TEXT (ADR-0003); `ts`/`bar_ts`/`as_of` =
  RFC3339 6-digit-fractional (ADR-0004 — the `subsecond digits:6` format the
  `post_strategy_signal` writer uses, NOT `Rfc3339` second precision — keeps
  `ORDER BY` stable under sub-second writes). `ts` = the row's mint
  wallclock; `bar_ts` and `as_of` are the snapshot's two timestamps stored
  verbatim. **Persist all three P&L components** (cash/realized/unrealized) —
  cheap, lets a future surface decompose P&L (durable-over-minimal).
  `fill_count` is **out** (the UI's `live_fill_count` is a session counter; a
  durable cumulative count is a separate concern). `mode` is stored for
  forensics/filtering even though only paper/live rows are ever written.
- **Retention (R7):** an age/row-capped `DELETE WHERE ts < …` purge task
  mirroring the nightly ledger-backup task, aligned with the 30-day
  ledger-snapshot horizon (`08-recovery-and-backups.md`). The hydrate query
  `LIMIT`s to ≤2880 so a hydrate never exceeds the buffer cap. AC8.
- **Return baseline (R6):** Total-return measured from the **first buffered
  point (account inception)**, NOT the session open — once history is
  durable, "session" return is the less meaningful number. The
  `ledger_inception_ts` (`query.rs:1626`) is the conceptual precedent. The
  caption is a new `LIVE_*` string (e.g. `"Since inception"`) — pinned by
  the ui-designer in T9, honest (not annualized, not "characterized").

### A4 (→ Q4) — Hydrate via boot query → `Task::perform` → batch `PnlHydrated`; seed the guard from the max hydrated `as_of`. **ACCEPT (a).** *(The riskiest decision — pinned exactly.)*

- **Mechanism:** a boot-time `audit::query::equity_snapshot_tail` →
  `iced::Task::perform` → a new batch `Message::PnlHydrated(Vec<(bar_ts,
  as_of, equity)>)` arm that seeds `live_equity_buffer` through (or exactly
  mirroring) `push_live_equity_point` in ONE mutation. Each historical row's
  plotted x-coordinate is its persisted `bar_ts`. Reuses the exact
  Memory/Models cold-boot seam; the UI stays a pure consumer; no
  reconciler-replays-the-tail coupling. **Gated on `cfg.mode != Research`**
  at the boot site — in research mode the hydrate task is not issued, so the
  curve stays session-scoped (R6).
- **The `as_of` guard contract (the load-bearing sub-decision):** seed
  `live_equity_last_as_of` from the **MAX hydrated `as_of`**. Historical
  `as_of` values are prior-session wallclock stamps, all `≤ now()`; the first
  LIVE snapshot's fresh `as_of = now()` is `≥` the max and therefore passes
  the guard and lands, while a late/duplicate re-delivery of an
  already-hydrated row is dropped (the guard's purpose). AC5 proves the
  post-hydrate live append lands at the **render** layer.
- **Backwards-clock edge (explicit):** if the host clock moved backwards
  across the restart, a fresh live `as_of` could be < the max hydrated
  `as_of` and be dropped until wallclock catches up. **Accepted, not
  defended-against in the guard.** It is bounded (~1 bar/min; the curve
  simply does not extend until the clock passes the stale max), it cannot
  corrupt or reorder the stored series (the x-coordinate is `bar_ts`,
  independent of `as_of`), and it never panics. Keying the guard on `bar_ts`
  instead was already tried and reverted (`40f5de9`) — it re-introduces the
  fast-replay drop bug. A backwards clock across a paper/live restart is a
  host-monitoring fault, not a UI concern. (Stated so the tester does not
  file it as a defect.)
- **Mode-switch mid-deployment (explicit):** a research↔paper switch between
  boots is correct by construction — a paper boot persists forward + hydrates
  prior paper history; a research boot persists/hydrates nothing. No `run_id`
  in v0.1.0, so two *different* paper deployments on one ledger would
  interleave — out of scope (named follow-on).
- **`is_all_absent` interaction:** ≥2 hydrated rows → build the KPI strip
  `Ready` immediately (clears the ≥2-point trap); ≤1 row → `Loading`
  (unchanged). The curve renders from ≥1.

### A5 (→ Q5) — One batch `PnlHydrated` arm, not N `PnlRefreshed`. **ACCEPT.**

A single `Message::PnlHydrated(Vec<…>)` arm seeds the buffer and rebuilds the
derived curve + KPI strip ONCE (vs re-deriving N times by replaying N
`PnlRefreshed`). It also makes "hydrate" explicitly distinct from a live
tick. One new message variant is the cost; worth it.

### A6 (→ Q6) — Per-bar fire-and-forget write. **ACCEPT.**

Per-bar insert behind the trait; a write error logs and continues, never
blocks and never panics the trading loop (the `bus = None` backtest
tolerance). The minute-bar cadence is low; batching/buffering is a premature
optimization until a sub-second config lands (then revisit behind the same
trait).

### A7 (→ Q7) — Headless `trading` bin persists too. **ACCEPT — confirmed intended.**

Gating at the mint site means the headless `trading` bin (same
`runtime::run`, no UI) persists equity in paper/live mode. This is the right
behaviour: the durable series should not depend on a UI being attached. The
headless bin issues no hydrate (no UI) — it only writes.

### Anchor-safety proof (A3 / AC7) — by construction

`013_equity_snapshots.sql` is `CREATE TABLE IF NOT EXISTS` + indexes — no
`ALTER`, no backfill, no `UPDATE` on any pre-existing row (the `010`
template). The backtest binary instantiates the reconciler with `bus = None`
(`reconciler.rs:72` doc + `runtime.rs:860` note) and never touches this
table, so the 19 anchored backtest body-SHA-256 reports are byte-unchanged.
This feature adds **no** row to `spec/anchors.toml` and mutates **none** of
the 9 anchor SHAs. AC7 asserts the anchor count is byte-unchanged.

### Project-law compliance (stated, per house convention)

- **Every external I/O behind a trait** — the `LiveEquityStore` trait (A1);
  the `ui` hydrate delegates to an `audit::query` helper (no direct sqlx in
  `ui`).
- **Money is `Decimal`/`Money<Usdt>`, never `f64`** — all four money columns
  Decimal-as-TEXT (ADR-0003); the reader parses to `Money<Usdt>`.
- **Determinism** — RFC3339 6-digit-fractional timestamps (ADR-0004); no
  RNG, no report bytes touched.
- **Baseline-equity-divergence e2e gate — N/A (explicit).** This is a
  read-only monitor-persistence feature: no strategy, no sizing, no decision
  variable. Per CLAUDE.md that gate applies to strategy overlays / sizing
  modifiers only — stated here exactly as `cockpit-live-dashboard-wiring`
  stated it.
- **Anchored reports byte-immutable** — the design touches no anchored
  report file (A3 proves it by construction).

### The one trait (the only real new API)

```rust
/// crates/audit — the durable live-equity-series boundary (R3 / A1).
/// External-I/O-behind-a-trait: the production impl wraps `Arc<Ledger>`;
/// tests use a fake. Money is `Money<Usdt>` (Decimal), never f64.
#[async_trait::async_trait]
pub trait LiveEquityStore: Send + Sync {
    /// Persist one per-bar snapshot. Fire-and-forget at the call site:
    /// the agent logs + continues on Err, never blocks/panics the loop (A6).
    async fn append_equity_snapshot(
        &self,
        row: &EquitySnapshotRow,   // (bar_ts, as_of, total_equity, cash, realized, unrealized, mode)
    ) -> Result<(), audit::LedgerError>;

    /// Read the tail for boot hydration, newest-bounded, LIMIT ≤ 2880,
    /// returned in monotone `bar_ts` order (A4 / R4).
    async fn equity_snapshot_tail(
        &self,
        limit: usize,
    ) -> Result<Vec<EquitySnapshotRow>, audit::LedgerError>;
}
```

(`async_trait` is already a workspace dep — no new dependency. `EquitySnapshotRow`
is an `audit`-crate DTO carrying `Money<Usdt>` + the two `Timestamp`s; the
`ui` hydrate consumes it via the `audit::query` helper, keeping `ui`'s
no-sqlx edge.)

## Implementation

_developer (exec track, 2026-06-11): T1–T6 landed. Summary below._

### Exec track (T1–T6) — what was built

**T1 — `LiveEquityStore` trait + `EquitySnapshotRow` DTO + `FakeLiveEquityStore`**

- `crates/audit/src/equity_store.rs` (new file):
  - `EquitySnapshotRow` carries `(id, ts, bar_ts, as_of, total_equity, cash, realized, unrealized, mode)` — all money as `Money<Usdt>`, all timestamps as `Timestamp`.
  - `LiveEquityStore` async trait (two methods: `append_equity_snapshot`, `equity_snapshot_tail`).
  - `LedgerEquityStore` (production impl wrapping `Arc<Ledger>`) and `FakeLiveEquityStore` (test fake: `Arc<Mutex<Vec<EquitySnapshotRow>>>`, thread-safe; `equity_snapshot_tail` sorts by `bar_ts` ascending and returns the last `limit` rows).
- `crates/audit/Cargo.toml`: added `async-trait.workspace = true`.
- `crates/audit/src/lib.rs`: added `pub mod equity_store;` + re-exports.

**T2 — Additive migration `013_equity_snapshots.sql`**

- `crates/audit/migrations/013_equity_snapshots.sql` (new): `CREATE TABLE IF NOT EXISTS equity_snapshots` — nine columns, two indexes (`ts` for purge; `bar_ts` for tail query). No `ALTER`, no backfill. Idempotent by construction.

**T3 — Writer `journal::post_equity_snapshot` + `LedgerEquityStore` production impl**

- `crates/audit/src/journal.rs`: added `format_ts_micros` helper (RFC3339 6-digit-fractional, per ADR-0004) and `post_equity_snapshot` (single INSERT, Decimal-as-TEXT per ADR-0003).

**T4 — Reader `query::equity_snapshot_tail` + `purge_old_equity_snapshots`**

- `crates/audit/src/query.rs`: added `equity_snapshot_tail` (fetches newest `limit` rows DESC then reverses to ascending `bar_ts` — the UI boundary helper, no direct sqlx in `ui`), `purge_old_equity_snapshots` (DELETE WHERE ts < cutoff, 30-day horizon), and unit tests (`equity_snapshot_round_trip_ac3`, `equity_snapshot_tail_limit_ac3`, `equity_snapshot_purge_ac8`, `equity_snapshot_tail_empty_table`).

**T5 — Mint-site wiring in `crates/agent`**

- `crates/agent/src/reconciler.rs`:
  - `equity_store: Option<Arc<dyn audit::LiveEquityStore>>` field on `ReconcilerTask`.
  - `with_equity_store` builder method.
  - `after_bar_close` fires a `tokio::spawn` fire-and-forget write when `equity_store.is_some()`; write errors are `tracing::warn`-logged, never propagated.
  - `build_snapshot_row` helper derives `total_equity`, `cash`, `realized`, `unrealized` from `ReconcilerState` (Decimal arithmetic, no f64).
- `crates/agent/src/runtime.rs`:
  - `RunHandles::equity_store: Option<Arc<dyn audit::LiveEquityStore>>` field.
  - Research branch: `drop(equity_store)` with A2-gate comment — research writes zero rows.
  - Paper branch: wires the store into `ReconcilerTask::with_equity_store` when `Some`.
- `crates/agent/src/main.rs`:
  - Constructs `equity_store: Option<Arc<dyn audit::LiveEquityStore>>` before `RunHandles` — `Some(LedgerEquityStore)` for paper/live, `None` for research.

**T6 — Retention purge (30-day horizon, AC8)**

- `purge_old_equity_snapshots` in `query.rs` (above). Retention hook ready for the nightly task; the `PURGE_EQUITY_HORIZON_DAYS = 30` constant is documented in the function signature.

### Test coverage

- `crates/audit/src/query.rs` (`#[cfg(test)]`): `equity_snapshot_round_trip_ac3`, `equity_snapshot_tail_limit_ac3`, `equity_snapshot_purge_ac8`, `equity_snapshot_tail_empty_table` (AC3, AC8).
- `crates/audit/src/equity_store.rs` (`#[cfg(test)]`): `fake_store_append_and_tail_monotone_order`, `fake_store_tail_respects_limit`, `fake_store_is_empty_and_len`.
- `crates/agent/tests/equity_store_integration.rs` (new): `ac1_paper_mode_persists_one_row_per_bar`, `ac2_research_mode_writes_zero_rows`, `ac1_faked_store_tail_is_monotone` (AC1, AC2).
- `RunHandles` initializers in test files updated to include `equity_store: None` (no behavior change to existing tests).

### Verification

- `cargo test -p audit`: all 46+ tests pass (0 failed).
- `cargo test -p agent`: all tests pass (0 failed across all integration test files).
- `cargo clippy -p audit -p agent -- -D warnings`: 0 warnings.
- `scripts/verify_anchors.sh`: 119/119 PASS — migration is anchor-safe by construction.

### Deviations from the architecture

None. The implementation follows A1–A7 exactly as specified.

## UI

_ui-designer-owned (2026-06-11). Resolves A4 (boot hydrate seam) / A5 (batch
arm) / R6 (since-inception caption). The Live screen reuses the EXISTING equity
curve + KPI strip verbatim — **no new widget, no new theme token**. The only
operator-visible change is the boot-hydrated curve (non-empty before the first
new bar) and a mode-correct return caption._

### What the operator sees

On a **paper/live** `cockpit_live` boot with prior durable history, the Live
screen's equity curve is **non-empty immediately** (hydrated from the audit
ledger's `equity_snapshots` tail) instead of blank-until-first-bar, and the
return caption reads **"Since inception"** (the figure spans the durable
multi-session history, measured from account inception). In **research** mode
nothing is hydrated — the curve stays session-scoped and the caption stays
**"Session to date"**. An **empty** durable table (fresh ledger) is a no-op:
curve/strip stay `Loading`, caption stays session-scoped — no blank-screen, no
overclaim.

### Wireframe (Live screen — hydrated paper/live boot)

```
┌──────────┬──────────────────────────────────────────────────────────┐
│ Sidebar  │  Live                                                      │
│  (Live)  │  System health · latency · server-time · market-health    │
│          │  ┌──────────────────────────────────────────────────────┐ │
│          │  │  Equity curve  ── HYDRATED on boot (paper/live) ──    │ │
│          │  │     /\    (ACCENT polyline; spans bar_ts x-axis,      │ │
│          │  │    /  \  /   = durable 2023→now history, not blank)   │ │
│          │  │ __/    \/                                             │ │
│          │  └──────────────────────────────────────────────────────┘ │
│          │  [Total return] [CAGR —] [Sharpe —] [Max-DD] [Win —] [Tr.] │
│          │  Since inception          ← caption flips on hydrate (R6)  │
│          │  ┌── Positions ──────────┐ ┌── Agent feed ──────────────┐  │
│          │  └───────────────────────┘ └────────────────────────────┘  │
└──────────┴──────────────────────────────────────────────────────────┘
       (research mode: curve session-scoped, caption "Session to date")
```

### New screens / panels / widgets

- **None.** The Live screen (`screens/live.rs`), the equity curve
  (`widgets::equity_curve`), and the KPI strip (`widgets::kpi_strip`) are reused
  verbatim. The change is purely in what *seeds* the model state on boot.

### Model / message contract (T7-contract, A4/A5)

- **`Message::PnlHydrated(Vec<(Timestamp /*bar_ts*/, Timestamp /*as_of*/,
  Money<Usdt>)>)`** — the batch hydrate variant (A5). One `update` arm seeds
  `live_equity_buffer` through the existing `push_live_equity_point` (x-coord =
  `bar_ts`) in a single mutation, seeds `live_equity_last_as_of` from the **MAX
  hydrated `as_of`** (A4 — so the first live `PnlRefreshed(now())` still lands,
  never dropped by the delivery guard), and rebuilds the curve `Ready` (≥1 row)
  + KPI strip `Ready` (≥2 rows) / `Loading` (1 row). Empty hydrate = no-op.
  The batch delivers every row "as of" the batch max so the per-point guard
  never drops a hydrate row, even when a row's own `as_of` is non-monotone vs.
  its `bar_ts` (a backed-up-clock prior session).
- **`Cockpit.live_equity_hydrated: bool`** — the honesty switch for the return
  caption. Set `true` only by the `PnlHydrated` arm (a real durable history was
  loaded); never by a live tick; reset `false` each boot (session-scoped, like
  the buffer). Drives the "Since inception" vs "Session to date" choice.

### Boot hydrate seam (T7, A4) — `crates/ui/src/bin/cockpit_live.rs`

- `equity_hydrate_task` added to the boot `Task::batch` (mirrors `memory_task`):
  a `#[cfg(feature = "live")]` `iced::Task::perform` that spawns
  `audit::query::equity_snapshot_tail(&ledger, LIVE_EQUITY_BUFFER_CAP)` on the
  side-thread runtime (so `ui` keeps its no-direct-sqlx edge — the query lives
  in `audit`), maps each `EquitySnapshotRow → (bar_ts, as_of, total_equity)`,
  and fires `Message::PnlHydrated`. Fail-soft (`unwrap_or_default()` → empty
  tail → no-op). **Gated `mode != Research`** at the boot site via
  `ui::live::should_hydrate_equity_on_boot(&mode)` (the named, testable mode
  seam). The `RunHandles.equity_store` write store is wired symmetrically:
  `Some(LedgerEquityStore)` in paper/live, `None` in research — research writes
  nothing AND hydrates nothing.

### New strings added to `ui::strings`

- `LIVE_SINCE_INCEPTION_CAPTION = "Since inception"` — the durable-history
  return caption (R6). Honest scope label: measured from the first persisted
  point (account inception), spans sessions/days; **never** annualized /
  "characterized" / a baseline result. A string test
  (`since_inception_caption_is_honest_no_overclaim`) bans overclaim tokens.
  (`LIVE_SESSION_RETURN_CAPTION = "Session to date"` is the unchanged
  session-scoped precedent.)

### New theme tokens

- **Zero.** The curve/strip/caption reuse `text::SMALL`, `color::FG_3`,
  `layout::LIVE_EQUITY_BUFFER_CAP`, `color::ACCENT` — all existing tokens.

### Render-harness gate (T8, R5 / AC6 / AC5)

`crates/ui/tests/live_equity_render.rs` extended with two pixel-layer tests
(the project-law render gate — model-Ready is necessary but not sufficient):

- **`hydrated_boot_curve_actually_renders` (AC6)** — one `Message::PnlHydrated`
  (faked ≥2-row 2023-`bar_ts` tail), **zero** `PnlRefreshed`, render the real
  Live screen, assert the `ACCENT` polyline drew (`count ≥
  CURVE_DREW_MIN_ACCENT`, `x_span ≥ CURVE_X_SPAN_MIN`). A blank-canvas
  regression fails here.
- **`live_append_after_hydrate_still_renders_and_grows` (AC5)** — hydrate (all
  `as_of` in the past), THEN one live `PnlRefreshed(now())`; assert the live
  point landed (model) AND the curve still rasterizes + extends its x-span (the
  A4 `as_of` guard-reconciliation proven at the pixel layer — the riskiest
  decision). Uses the rescale-invariant x-span signal (a new higher peak
  rescales the Y-axis, so a raw pixel-count comparison is not a valid "grew"
  proof).

### Accessibility notes

- **Keyboard map:** unchanged — the Live curve + strip + caption are read-only
  display surfaces (no new interactive element; the hydrate is a boot-time data
  seed, not an operator action). Existing sidebar/screen keyboard nav is
  untouched.
- **Contrast:** the caption uses `color::FG_3` on the panel background — the
  same token the existing `LIVE_SESSION_RETURN_CAPTION` already uses (contrast
  verified in `theme`, ≥ 4.5:1). No new color pairing introduced.
- **Color is not the only signal:** the caption text itself ("Since inception"
  vs "Session to date") names the scope — the operator never relies on color to
  distinguish hydrated from session-scoped state.
- **No blank screens:** the empty-hydrate path keeps the curve/strip honest
  `Loading` (waiting for first bar), not a "no data" dead-end; the existing
  `VIEWER_NO_EQUITY_DATA` placeholder copy is unchanged.

### Both-theme coverage

The caption renders correctly under `--theme dark` and `--theme light` — the
`panel_snapshots` live-screen summary asserts the caption in dark; the
token-driven `color::FG_3.current(mode)` resolves per theme. The existing
`live_snapshot__ready_dark` / `__ready_light` snapshots stay byte-unchanged
(they don't hydrate → caption stays "Session to date").
