---
slug: live-equity-history-durable
status: tester-done
owner: tester
updated: 2026-06-11
---

# Tasks — live-equity-history-durable

**Architect-refined (2026-06-11).** Q1–Q7 resolved as A1–A7 in
[feature.md § Architecture](feature.md#architecture); full decision record in
[ADR-0052](../architecture/adr/0052-durable-live-equity-series.md). Exec-led,
**~L**, ≈ 60% exec (audit + agent) / 40% UI. **Developer ‖ ui-designer run in
PARALLEL once Wave 0 lands** (see the parallelization gate below).

**Settled invariants the design MUST NOT reopen** (feature.md § Why): the
two-timestamp contract (`as_of` = delivery key, `bar_ts` = plotted x-coord;
approach A — do NOT re-conflate); `push_live_equity_point`'s
guard/clamp/ring/`is_all_absent`-trap; and **the render harness
(`crates/ui/tests/live_equity_render.rs`) is THE gate** — model-layer tests
are necessary but not sufficient (project law: verify UI at the render layer).

**Project-law reminders (binding):** every external I/O behind a trait (the
`LiveEquityStore` trait, A1); money is `Decimal` / `Money<Usdt>`, never `f64`
(A3); the `013` migration is additive `CREATE TABLE IF NOT EXISTS` —
**anchor-safe by construction** (19 backtest body-SHA-256 anchors stay
byte-unchanged, A3/AC7); timestamps RFC3339 6-digit-fractional (ADR-0004). No
strategy / sizing math → the **baseline-equity-divergence e2e gate does NOT
apply** (A3, stated explicitly). The design touches **no anchored report
file**.

---

## Wave 0 — Pin the contract FIRST (blocks the parallel split)

These two tasks define the trait boundary (exec) and the message/view
contract (UI) the two tracks code against. **Land both, then the tracks
parallelize.** Keep them small and mergeable.

- [x] **T1 — `LiveEquityStore` trait + `EquitySnapshotRow` DTO (`crates/audit`).**
  Define the durable-equity-store trait (the external-I/O-behind-a-trait
  boundary, A1) and the `EquitySnapshotRow` DTO carrying `(bar_ts, as_of,
  total_equity, cash, realized, unrealized, mode)` as `Money<Usdt>` + two
  `Timestamp`s. Provide a `Fake` impl (in-memory `Vec`) for tests. Signature
  per feature.md § "The one trait". _acceptance: trait + `Fake` impl + DTO
  compile; `async_trait` reused (no new dep); the real impl stub selectable in
  `cockpit_live` / headless boot. **Gate: `cargo check -p audit`.**_
- [x] **T7-contract — `Message::PnlHydrated` variant + the seed rule (`crates/ui/src/state.rs`).**
  Add the batch `Message::PnlHydrated(Vec<(Timestamp /*bar_ts*/, Timestamp
  /*as_of*/, Money<Usdt>)>)` variant (A5) and its `update` arm: seed
  `live_equity_buffer` through `push_live_equity_point` (x-coord = `bar_ts`),
  seed `live_equity_last_as_of` from the **MAX hydrated `as_of`** (A4), build
  the KPI strip `Ready` when ≥2 rows. This is the model-layer half of the UI
  contract — it does NOT yet wire the boot query (that is T7). _acceptance:
  **AC4** (model) — after `PnlHydrated` with M≥2 rows,
  `live_equity_buffer.len() == min(M, 2880)`, curve + strip `Ready`, before any
  live tick; **AC5** (model) — a subsequent live `PnlRefreshed(now())` is
  appended, not dropped. **Gate: `cargo test -p ui --lib`** (the new
  `state.rs` tests)._

> **PARALLELIZATION GATE.** After T1 + T7-contract merge, the **Exec track
> (T2–T6)** and the **UI track (T7–T9)** have no ordering dependency and run
> concurrently (developer ‖ ui-designer, per AGENT.md). The exec track wires
> the real impl behind T1's trait; the UI track wires the boot query behind
> T7-contract's message. They reconverge only at T10 (tester).

---

## Exec track (developer) — parallel after Wave 0; resolves A1/A2/A3/A6/A7

- [x] **T2 — Migration `013_equity_snapshots.sql` (A3).** Additive
  `CREATE TABLE IF NOT EXISTS equity_snapshots` + indexes (`ts`, `bar_ts`),
  the `010_training_events.sql` template (copy its anchor-safety header).
  Columns `(id, ts, bar_ts, as_of, total_equity, cash, realized, unrealized,
  mode)`; money Decimal-as-TEXT (ADR-0003); ts RFC3339-micros (ADR-0004).
  _acceptance: migration applies idempotently on a fresh + a re-run DB;
  **AC7** — the backtest anchors are byte-unchanged (additive table, never
  read by the backtest binary). **Gate: `cargo test -p audit` migration test +
  `scripts/verify_anchors.sh` (or `rust-validate`) green; explicit anchor-count
  assertion in the close-out report.**_
- [x] **T3 — Writer + real `LiveEquityStore` impl (`crates/audit`).**
  `journal::post_equity_snapshot` (sibling of `post_training_*`, 6-digit ts
  format, Decimal binding) and the production `LiveEquityStore` impl wrapping
  `Arc<Ledger>`. _acceptance: writes one row per call; Decimal/ts round-trip
  lossless. **Gate: `cargo test -p audit`** (writer unit test on an in-memory
  `Ledger`)._
- [x] **T4 — Reader (`crates/audit`).** `query::equity_snapshot_tail` (sibling
  of `recent_training_events`), monotone `bar_ts` order, `LIMIT ≤ 2880`. Also
  the `ui`-boundary helper the hydrate task calls (so `ui` keeps its no-sqlx
  edge — the `reflection::query::open_and_list_recent` precedent). _acceptance:
  **AC3** round-trip — write N → read tail → values + monotone `bar_ts` order +
  limit correct (in-memory `Ledger`). **Gate: `cargo test -p audit`.**_
- [x] **T5 — Mint-site wiring + mode gate (`crates/agent`).** Call the T1 trait
  from `reconciler.rs::after_bar_close` + `runtime.rs::spawn_research_trading_loop`,
  **gated `config.mode != Research`** (A2), per-bar **fire-and-forget**: a write
  error logs + continues, never blocks/panics the loop (A6, the `bus = None`
  tolerance). Thread the store handle via `RunHandles`/`runtime::run` so BOTH
  the headless `trading` bin and `cockpit_live` persist (A7). _acceptance:
  **AC1** — paper mode (faked store) writes one row/bar with the expected
  Decimal `(bar_ts, as_of, total_equity, cash, realized, unrealized)`; **AC2** —
  research mode writes ZERO rows (the duplication gate). **Gate: `cargo test -p
  agent`** (integration test driving the loop in each mode against the `Fake`
  store)._
- [x] **T6 — Retention purge (R7 / A3).** Age/row-capped `DELETE WHERE ts < …`
  task mirroring the nightly ledger-backup task, aligned with the 30-day
  snapshot horizon. _acceptance: **AC8** — purge removes rows past the horizon;
  store bounded. **Gate: `cargo test -p audit`** (purge test: insert past +
  within horizon → only within-horizon survive)._

## UI track (ui-designer) — parallel after Wave 0; resolves A4 (boot) / A5 / R6

- [x] **T7 — Boot hydrate seam (`crates/ui/src/bin/cockpit_live.rs`).** Add an
  `equity_hydrate_task` to the boot `Task::batch` (`cockpit_live.rs:764`),
  mirroring `memory_task`: `#[cfg(feature = "live")]` `iced::Task::perform` →
  `rt.spawn(audit::query::equity_snapshot_tail(...))` → `Message::PnlHydrated`,
  fail-soft (`Ok(vec![])` on a missing/empty table). **Gated on `cfg.mode !=
  Research`** at the boot site (research issues no hydrate → session-scoped
  curve, R6). Consumes the T4 helper + the T7-contract message. _acceptance:
  hydrate fires only in paper/live mode; fixtures/non-`live` build issues no
  hydrate. **Render-layer verification: covered by T8** (this task is the wire;
  T8 is the pixel gate). **Gate: `cargo build -p ui --features live` +
  `cargo test -p ui --features live`** (a seam test that the task is issued in
  paper mode, skipped in research)._
- [x] **T8 — Render-harness gate (R5 / AC6 — THE gate). RENDER-LAYER
  VERIFICATION.** Extend `crates/ui/tests/live_equity_render.rs`: build a
  cockpit, drive ONE `Message::PnlHydrated(faked tail of ≥2 rows)` through the
  production `update` path (zero `PnlRefreshed`), render the REAL Live screen
  via `iced_test::screenshot`, and assert the `ACCENT` polyline drew
  (`count ≥ CURVE_DREW_MIN_ACCENT`, `x_span ≥ CURVE_X_SPAN_MIN`) — a
  model-Ready-but-blank-canvas regression fails here. **Add a second render
  case: hydrate, THEN deliver one live `PnlRefreshed(now())`, and assert the
  curve still draws + extends** (the A4 `as_of`-guard reconciliation proven at
  the pixel layer, AC5). _acceptance: **AC6** hydrated boot rasterizes a
  non-empty curve with zero live snapshots; **AC5** post-hydrate live append
  renders. **Gate: `cargo test -p ui --features live --test live_equity_render`.**_
- [x] **T9 — Since-inception caption + mode-correctness (R6). RENDER/STRING
  VERIFICATION.** Add the `LIVE_*` since-inception caption string (the
  `LIVE_SESSION_RETURN_CAPTION` precedent) and ensure the Live screen renders
  it under a hydrated (multi-session) history; research mode keeps the
  session-scoped curve / no history affordance. _acceptance: caption honest
  (not annualized, not "characterized"); a string-content + (if the caption is
  on the rendered Live screen) a render/`panel_snapshots` check. **Gate:
  `cargo test -p ui`** (string test) **+ the T8 render harness** if the caption
  lands in the curve crop band._

## Close-out (tester)

- [x] **T10 — Full gate.** AC1–AC9 green; **AC6** render harness green
  (including the AC5 post-hydrate-live-append render case); **AC7** anchor count
  byte-unchanged (explicit assertion in the report — `scripts/verify_anchors.sh`
  / `rust-validate`); **AC9** fixtures-mode `cockpit` smoke byte-unchanged (no
  `live` feature → no hydrate → empty buffer → curve/strip `Loading`, no panic,
  within the existing smoke window) + review confirms the store is reached
  through the `LiveEquityStore` trait + no new dep beyond `async_trait`
  (already a workspace dep); new-code `cargo clippy -- -D warnings` + `cargo fmt`
  clean. Tester report per the `rust-test` template. _Note: the
  baseline-equity-divergence e2e gate is **N/A** for this read-only monitor
  feature (A3) — record that explicitly in the report, do not file it as a
  missing gate._

---

## Wave summary (the parallelization picture)

```
Wave 0 (serial):  T1 (trait/DTO) ─┐
                  T7-contract ─────┴─► [PARALLELIZATION GATE]
                                          │
              ┌───────────────────────────┴───────────────────────────┐
   Exec track (developer)                          UI track (ui-designer)
   T2 migration → T3 writer → T4 reader            T7 boot hydrate seam
   T5 mint+mode-gate → T6 purge                    T8 render gate (AC6+AC5)
                                                   T9 caption + mode-correct
              └───────────────────────────┬───────────────────────────┘
                                          │
                              T10 tester full gate (AC1–AC9)
```

Exec ↔ UI reconverge only at T10. T8 (the render gate) depends on
T7-contract (the message) but NOT on the exec track — it uses the faked tail
— so the UI track's pixel gate runs without waiting for the real writer.
