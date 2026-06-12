---
slug: paper-mode-equity-wiring
status: shipped
owner: tester
updated: 2026-06-12
version: 0.2.0
trace: REQ-LIVE-EQUITY-PAPER-001
---

# Tasks — paper-mode-equity-wiring

> Architect-refined M-DEV waves (v0.2.0), resolving Q1=(a) unify. Design:
> **[ADR-0053](../architecture/adr/0053-unified-per-bar-trading-loop.md)** +
> `feature.md` § Architecture (A1–A6). Exec-led (~90% `crates/agent`); the
> lone UI surface is a render-TEST assertion (no widget, no design-system
> work). **No new crate, no new dependency** — feed, exec engine, sizer,
> store, trait, publisher, and UI are all already in use.

## Shape decision (single developer track — structured honestly)

The UI surface is **one render-test assertion** in an existing test file
(`live_equity_render.rs`) — not a widget, theme token, string, or layout
change. Forcing a parallel ui-designer track for a single test extension
would be artificial. **This feature runs as one developer track**; the
render-test task (M-DEV-6) is the developer's, flagged with its
render-layer verification. The render-layer-verify discipline still
binds (MEMORY.md *Verify UI at the render layer*): M-DEV-6 asserts on
rasterized ACCENT pixels, never on model/agent state.

## Waves

```
Wave 0 (blocking skeleton)
  └─ M-DEV-1  rename + widen spawn_trading_loop; research call site
Wave 1 (developer, sequential within the track — all touch runtime.rs)
  ├─ M-DEV-2  loop-direct persist + build_snapshot_row pub(crate)
  ├─ M-DEV-3  wire the paper arm; delete the drop(state_tx) stub
  ├─ M-DEV-4  data-layer divergence + fills + no-real-orders tests
  └─ M-DEV-6  render-layer divergence gate (the UI test)
Wave 2 (reconverge — proofs)
  ├─ M-DEV-5  research byte-safety + anchor-count proof
  └─ M-DEV-7  fixtures smoke + no-live-feature build unchanged
```

## Task list

- [x] **M-DEV-1 — Rename + widen the loop; research call site (Q1, A1).**
  Rename `spawn_research_trading_loop → spawn_trading_loop`; add two
  additive params: `equity_store: Option<Arc<dyn audit::LiveEquityStore>>`
  and `mode_label: &'static str`. Body unchanged (the verified research
  body). Update the research arm call site (runtime.rs:527) to pass
  `replay_feed, …, None, "research", …` and **remove** `drop(equity_store)`
  (runtime.rs:541). Update the paced-replay test call site
  (`paced_replay_late_subscriber.rs:164`) to the new signature
  (`None, "research"`) — this is compile-enforced and must build + pass
  before the wave closes. **Gate:** `cargo build -p agent`;
  `paced_replay_late_subscriber` green; research integration tests green.
  (R1, R4 · AC7)

- [x] **M-DEV-2 — Loop-direct persist + `build_snapshot_row` `pub(crate)`
  (Q4, A2).** Promote `reconciler::build_snapshot_row` (reconciler.rs:255)
  from private to `pub(crate)` and have it take `mode_label` (drop the
  hardcoded `"paper"`). In `spawn_trading_loop`, immediately after
  `publish_pnl(snap)`, persist the same `snap` via
  `store.append_equity_snapshot` **only when `equity_store` is `Some`**,
  fire-and-forget (`tokio::spawn`, log + discard on `Err`, never
  block/panic — ADR-0052 A6). NO `if mode != Research` inside the loop —
  the `Some`/`None` IS the gate. **Gate:** `cargo build -p agent`;
  `equity_store_integration.rs` AC2 (research store=`None` → 0 rows) still
  green. (R2 · AC1, AC2)

- [x] **M-DEV-3 — Wire the paper arm; delete the idle-reconciler stub
  (Q2, A3 · AC4).** In the `Mode::Paper` branch (runtime.rs:544), call
  `spawn_trading_loop(binance_feed, …, equity_store, "paper", …)` using the
  Binance feed built at runtime.rs:571. **Delete the entire
  `drop(state_tx)` idle-reconciler block (runtime.rs:655-683)** — the loop
  is now the sole paper mint site; the free 60 s `reconciler_interval_ms`
  marking is gone; the bar stream is the cadence. Do NOT respawn the
  periodic `ReconcilerTask` as a mint site (imbalance-only is an un-wired
  follow-on). **Gate:** `cargo build -p agent`; `cargo clippy -p agent --
  -D warnings`; no `drop(state_tx)` remains (grep clean). (R1, R2, R3 ·
  AC4)

- [x] **M-DEV-4 — Data-layer divergence + fills + no-real-orders tests
  (A5, A6 data-half · AC1, AC2, AC3, AC5).** Extend
  `crates/agent/tests/equity_store_integration.rs` with a paper-loop
  integration test: drive `spawn_trading_loop` against a `data::FakeFeed`
  emitting closed bars whose price MOVES and triggers `SmaCrossover`
  signals, with a `FakeLiveEquityStore` and a recording publisher. Assert:
  (i) paper fills are produced (recording publisher non-empty) and reach
  `bus.fills()`/`bus.positions()` (AC5); (ii) the per-bar
  `PnlSnapshot.total_equity` values are **NOT all equal** — diverge from
  the flat `initial_capital` baseline by ≥ a testable epsilon (AC1
  data-half); (iii) the persisted rows carry those non-constant
  `total_equity` values + **row count == bar count** (one writer, no
  double-mint — AC2); (iv) no live exchange-execution client is
  constructed — fills originate only from `PaperEngine` (seed
  `0x00C0_FFEE`) (AC3). **Gate:** the new test green; deterministic
  (two-run identical). (R7 · AC1, AC2, AC3, AC5)

- [x] **M-DEV-5 — Research byte-safety + anchor-count proof (R4 · AC7).**
  Confirm the research-replay loop's outputs (fills, snapshots, equity)
  are unchanged by the unify: research + paced-replay tests green; run the
  anchor gate (`rust-validate` / `verify_anchors.sh`) — **119 anchor rows
  byte-identical** (the backtest binary never calls `runtime::run`, so
  anchors are structurally independent). Record an **explicit
  anchor-count assertion** in the test report. **Gate:** anchor gate green
  + the count is stated, not assumed. (R4 · AC7)

- [x] **M-DEV-6 — Render-layer divergence gate (the UI test, A6
  render-half · AC6).** Extend `crates/ui/tests/live_equity_render.rs`:
  add `const CURVE_Y_VAR_MIN` (a Y-variation threshold — pick from the
  `diag_accent_bounding_box` empirics: a real session bbox is tall, a flat
  line ~1px; floor it well above AA noise and well below the real height).
  Feed the real Live screen a **moving** paper equity series (live and/or
  `PnlHydrated` from a moving tail) and assert
  `(max_y - min_y) ≥ CURVE_Y_VAR_MIN` **in addition to** the existing
  `count ≥ CURVE_DREW_MIN_ACCENT` + `x_span ≥ CURVE_X_SPAN_MIN`. Add a
  **self-proving contrast**: render a FLAT `initial_capital` series and
  assert its bbox height `< CURVE_Y_VAR_MIN` (it PASSES `count`/`x_span`
  via the `equity_curve.rs:178` centered-full-width-line guard — proving
  Y-variation is the only valid discriminator). **Render-layer verify:
  assert on rasterized ACCENT pixels, never on model state.** **Gate:**
  both assertions green; the flat contrast fails Y-variation but passes
  count/x_span (proving the gate discriminates). (R5 · AC6)

- [x] **M-DEV-7 — Fixtures smoke + no-live-feature build unchanged
  (AC8).** The fixtures-mode cockpit (no `live` feature, no agent) is
  byte-identical to today (it never runs the paper loop); a build without
  the `live` feature is unaffected. Confirm zero new external I/O (the
  feed/exec/store are all reused behind existing traits). **Gate:**
  fixtures `cockpit` smoke green; `cargo build` (no `live` feature) green;
  no new dep in any `Cargo.toml` (`scripts/precheck.sh` clean). (AC8)

## Per-task gate summary (for the tester's report)

| Task | AC | Gate command(s) |
|---|---|---|
| M-DEV-1 | AC7 | `cargo build -p agent`; paced-replay + research tests green |
| M-DEV-2 | AC1, AC2 | `cargo build -p agent`; equity_store AC2 green |
| M-DEV-3 | AC4 | `cargo clippy -p agent -- -D warnings`; grep no `drop(state_tx)` |
| M-DEV-4 | AC1, AC2, AC3, AC5 | new paper-loop test green + deterministic |
| M-DEV-5 | AC7 | anchor gate green; explicit anchor-count in report |
| M-DEV-6 | AC6 | render test: Y-variation + self-proving flat contrast |
| M-DEV-7 | AC8 | fixtures smoke + no-`live` build + `precheck.sh` clean |

## Wave hint (Q1=(a) unify — confirmed)

- **Wave 0:** M-DEV-1 (skeleton/signature — unblocks everything; the
  compile-enforced paced-replay call-site update lands here).
- **Wave 1:** M-DEV-2 → M-DEV-3 → M-DEV-4 → M-DEV-6 (single developer
  track; M-DEV-2/3 are sequential — both edit `runtime.rs`; M-DEV-4/6 are
  the test halves of the divergence gate).
- **Wave 2:** M-DEV-5 + M-DEV-7 (proofs; reconverge for the tester).

## `[[req]]` row for `spec/trace.toml` (analyst-authored; orchestrator to apply via `spec-update`)

> The analyst owns `[[req]]` row creation but this task is folder-scoped (no
> direct `trace.toml` edits). The row below is ready to insert; `arch` lists
> the analyst-draft files (architect appends the ADR), `crates`/`tests` are
> analyst seeds the developer/tester refine, `anchors` stays empty (additive —
> backtest anchors byte-unchanged by construction, AC7). State `proposed`.

```toml
[[req]]
id          = "REQ-LIVE-EQUITY-PAPER-001"
title       = "Paper-mode equity wiring — make paper-mode cockpit equity REAL (not a flat line at initial_capital_usdt). Root cause (presenter 2026-06-11): the Mode::Paper arm of runtime.rs connects a live Binance WS feed + an idle reconciler whose state_tx is DROPPED (runtime.rs:674), so equity = cash + 0*0 = initial_capital forever, and the shipped live-equity-history-durable (ADR-0052) faithfully persists/re-hydrates that flat line. NO strategy loop, order placement, or paper fills run in paper mode today (registry.on_bar is called only by spawn_research_trading_loop). Fix: run the existing per-bar equity pipeline (registry.on_bar -> risk::size_and_validate -> PaperEngine::step -> publish fills/positions + rich PnlSnapshot, total_equity = cash + base_qty*mark) against the LIVE feed in paper mode — recommended via UNIFYING spawn_research_trading_loop into one feed-parameterized spawn_trading_loop(feed, mode, store) for research AND paper (Q1=a; the loop body is already mode-agnostic + feed-parameterized, runtime.rs:946; behavior-preserving rename guarded by research tests + anchor gate). Remove drop(state_tx); ONE per-bar writer (loop-direct LiveEquityStore persist, gated mode != Research, fire-and-forget per A6); bar-stream cadence (mark to bar.close), not the free 60s timer. Research replay + 19 backtest anchors byte-unchanged (R4/AC7 — backtest never calls runtime::run). Settled law NOT reopened: two-timestamp contract (as_of wallclock / bar_ts data time), LiveEquityStore trait + A2 research-writes-nothing gate, Decimal/Money<Usdt> never f64, every external I/O behind a trait. CLAUDE.md baseline-equity-divergence gate INTENT APPLIES here (this adds a strategy/sizing/exec decision into paper mode, unlike the durable feature's genuine N/A) and is satisfied by AC6 (Y-variation render proof — a flat line PASSES the existing ACCENT count/x_span because equity_curve.rs centers a degenerate series as a full-width horizontal line; only the ACCENT bbox HEIGHT (max_y-min_y) >= CURVE_Y_VAR_MIN discriminates flat from moving) + AC1's data-layer non-constant-total_equity assertion. Effort ~M, ~90% exec (crates/agent runtime.rs/reconciler.rs); store/trait/migration/hydrate/captions/PnlSnapshot all shipped by ADR-0052; the lone UI change is the Y-variation render assertion."
feature     = "paper-mode-equity-wiring"
product     = "spec/product.md"
arch        = [
  # ANALYST (2026-06-11) — v0.1.0 brief, Q1-Q6 with recommended defaults.
  "spec/paper-mode-equity-wiring/feature.md",
  "spec/paper-mode-equity-wiring/tasks.md",
]
crates      = ["crates/agent"]   # developer refines; exec-led, ~90% runtime.rs/reconciler.rs
tests       = [
  # render-layer divergence gate (AC6) — Y-variation assertion turns "a curve drew" into "the curve moved".
  "crates/ui/tests/live_equity_render.rs",
  # data-layer divergence (AC1/AC2) — paper loop produces non-constant total_equity + one-row-per-bar.
  "crates/agent/tests/equity_store_integration.rs",
]
anchors     = []   # N/A — additive only; backtest anchors byte-unchanged by construction (AC7). Baseline-divergence gate INTENT satisfied by AC6 + AC1 data-layer assertion (NOT rubber-stamped N/A — Q6).
state       = "proposed"
```
