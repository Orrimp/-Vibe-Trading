---
slug: live-equity-history-durable
status: analyst-draft
owner: analyst
updated: 2026-06-11
---

# Tasks — live-equity-history-durable

**Stub — the architect refines this after resolving Q1–Q7 in
[feature.md](feature.md).** Exec-led, **~L**, ≈ 60% exec (audit + agent) /
40% UI (the `cockpit-live-dashboard-wiring` D1=(b) split). Developer ‖
ui-designer once the design lands.

Read the resolved `## Open questions` + `## Architecture findings` in
[feature.md](feature.md) first. The load-bearing decisions: **Q1** (store =
audit ledger, recommended), **Q2** (research mode persists nothing — the
duplication gate), **Q4** (hydrate via `query → Task::perform`, and the
`as_of` delivery-guard reconciliation).

**Settled invariants the design MUST NOT reopen** (see feature.md § Why):
the two-timestamp contract (`as_of` = delivery key, `bar_ts` = plotted
x-coord; approach A, 2026-06-11 — do NOT re-conflate); `push_live_equity_point`'s
guard/clamp/ring/`is_all_absent`-trap; and **the render harness
(`crates/ui/tests/live_equity_render.rs`) is the gate** — model-layer tests
are necessary but not sufficient (project law: verify UI at the render
layer).

**Project-law reminders (binding):** every external I/O behind a trait;
money is `Decimal` / `Money<Usdt>`, never `f64`; the persistence migration
(if Q1=(a)) is additive `CREATE TABLE IF NOT EXISTS` — **anchor-safe** (the
19 backtest body-SHA-256 anchors stay byte-unchanged). No strategy / sizing
math → the baseline-equity-divergence e2e gate does NOT apply.

---

## Exec track (developer) — gated on Q1/Q2/Q3

- [ ] **T1 — Persistence trait (R3).** Define the durable-equity-store trait
  (the external-I/O-behind-a-trait boundary) so tests fake it. _acceptance:
  trait + a fake impl compile; the real impl is selected in `cockpit_live` /
  headless boot._
- [ ] **T2 — Schema migration (Q1=(a), Q3).** Additive
  `013_equity_snapshots.sql` (`CREATE TABLE IF NOT EXISTS` + indexes), the
  `010_training_events.sql` precedent: columns `(id, bar_ts, as_of,
  total_equity, cash, realized, unrealized, mode)`, money Decimal-as-TEXT
  (ADR-0003), ts RFC3339-micros (ADR-0004). _acceptance: migration applies
  idempotently; **AC7** — backtest anchors byte-unchanged._
- [ ] **T3 — Writer (`crates/audit`).** `journal::post_equity_snapshot`
  (sibling of `post_training_*`), behind the T1 trait. _acceptance: writes
  one row per call; Decimal/ts round-trip lossless._
- [ ] **T4 — Reader (`crates/audit`).** `query::equity_snapshot_tail` /
  `recent_equity_snapshots` (sibling of `recent_training_events`,
  RFC3339 window), `LIMIT ≤ 2880`, monotone `bar_ts` order. _acceptance:
  **AC3** round-trip; tail ordering + limit correct._
- [ ] **T5 — Mint-site wiring + mode gate (`crates/agent`).** Call the trait
  from `reconciler.rs::after_bar_close` + `runtime.rs::spawn_research_trading_loop`,
  **gated `mode != Research`**; fire-and-forget, never blocks/panics the loop
  (log + continue, like the `bus = None` backtest tolerance). _acceptance:
  **AC1** paper writes a row/bar; **AC2** research writes nothing._
- [ ] **T6 — Retention purge (R7).** Age/row-capped purge task (mirrors the
  nightly ledger-backup task). _acceptance: **AC8** store bounded._

## UI track (ui-designer) — gated on Q4/Q5

- [ ] **T7 — Hydrate seam (`crates/ui`).** Boot-time `audit::query` →
  `Task::perform` → batch `PnlHydrated` arm (Q5) that seeds
  `live_equity_buffer` via / mirroring `push_live_equity_point`; seed
  `live_equity_last_as_of` from the **max hydrated `as_of`** (Q4 sub-decision)
  so the first live snapshot still passes the guard; build the KPI strip
  `Ready` when ≥2 rows hydrate. _acceptance: **AC4** buffer + curve + strip
  Ready before any live tick; **AC5** post-hydrate live append still lands._
- [ ] **T8 — Render-harness gate (R5 — THE gate).** Extend
  `crates/ui/tests/live_equity_render.rs`: render the real Live screen of a
  hydrated cockpit (zero live snapshots) and assert the `ACCENT` polyline
  drew (`count ≥ CURVE_DREW_MIN_ACCENT`, `x_span ≥ CURVE_X_SPAN_MIN`).
  _acceptance: **AC6** — hydrated boot rasterizes; a model-Ready-but-blank
  regression fails here._
- [ ] **T9 — Honest caption + mode-correctness (R6).** Since-inception
  return caption (new `LIVE_*` string, `LIVE_SESSION_RETURN_CAPTION`
  precedent); research mode keeps the session-scoped curve / no history
  affordance. _acceptance: caption honest (not annualized / not
  "characterized"); string-content test if added._

## Close-out (tester)

- [ ] **T10 — Full gate.** AC1–AC9 green; fixtures `cockpit` smoke
  byte-unchanged (**AC9**); anchor count unchanged (**AC7**); render harness
  green (**AC6**) including the post-hydrate-live-append case; new-code
  clippy/fmt clean. Tester report per the rust-test template.
