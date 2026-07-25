---
adr: 0060
title: Budget-aware sizing modifier + forward-paper-run selection seam
status: accepted
date: 2026-06-20
updated: 2026-06-21
supersedes: none
superseded-by: none
---

# ADR-0060: Budget-aware sizing modifier + forward-paper-run selection seam

## Context

The single-coin investment-advisor MVP (product pivot 2026-06-19) closes with
journey steps 4 + 5 (feature `advisor-forward-paper`, roadmap F4 + F5): the user
picks a strategy from the bake-off leaderboard and watches it **paper-trade
their fixed €200 forward** on real data, with running P/L. The engine already
ships everything heavy: the risk/sizing layer (`crates/risk`:
`FixedFractionSizer`, `size_and_validate`), the real-time paper-mode trading
loop (`agent::runtime::spawn_trading_loop`, ADR-0053), the durable equity store
(ADR-0052), the reflection-wired loop (commit `e9da47f`), and the cockpit Live
view (`crates/ui/src/live.rs`). Three new decisions are needed to wire them into
the journey without violating the layering invariant (`ui` must not import
`strategy`/`exec`/`forecast`/`llm`) or the CLAUDE.md sizing-modifier
non-negotiable. Today the runtime hardcodes `feed_symbol = Symbol::new("BTCUSDT")`
(`runtime.rs:490`) and builds the sizer un-capped from
`risk_cfg.sizing.fixed_fraction` (`runtime.rs:1138`) on the default
`initial_capital_usdt` — neither knows about a per-user budget or a selected
coin/strategy.

## Decision

**D1 — F4 budget-aware sizing lives in `crates/risk::sizing` as an additive
modifier on the existing sizer.** Add `budget_cap: Option<Money<Usdt>>` to
`FixedFractionSizer` + a `with_budget_cap(fraction, budget)` ctor; in
`compute_qty`, after the existing per-symbol exposure-cap clamp, apply a
**composed budget clamp** `qty = qty.min(budget.amount() / price)` (Decimal-exact;
the tighter of {exposure cap, budget} binds). `new(fraction)` sets
`budget_cap: None` and is **byte-identical** to today; `size_and_validate` is
unchanged (the cap rides inside the sizer). The budget is a **permanent notional
ceiling** ("never deploy more than €200" even after equity grows), not an
equity re-scale.

**D2 — F4 ships with the day-1 baseline-equity-divergence e2e (NON-NEGOTIABLE).**
`crates/risk/tests/budget_sizing_divergence_end_to_end.rs`, modelled on
`crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`: run the sizing path
twice (budget arm: `cash=200`, `with_budget_cap(.,200)`; baseline arm:
`cash=100_000`, `::new`) over a fixture bar series producing ≥ 1 fill; assert the
**return paths diverge ≥ 1 bp**. Under a no-op cap the return paths are equal →
the assertion FAILS (the forensic FAIL-before / PASS-after gate). This lands in
the same PR as the modifier.

**D3 — F5 selection→forward seam: a `core`-typed `ForwardRunConfig` + a widened
registry builder, both on the `agent` side of the wall.** Add
`agent::config::ForwardRunConfig { strategy: StrategyId, symbol: Symbol, budget:
Money<Usdt>, lookback: Option<DateRange> }` (built UI-side from the crowned/picked
`LeaderRow` + the F3 budget — `core` types only, **no `ui → strategy` edge**).
Widen the existing `build_registry(cfg)` seam into `build_registry_for(cfg,
&ForwardRunConfig)` that resolves the selected `StrategyId` to a concrete
strategy (the same id set the bake-off field uses: `v0.sma`/`v0.5.macd`/
`v0.5.rsi`/`v0.5.bbands`/`v0.buyhold`; unknown → warn + config-default fallback).
Add `RunHandles.forward: Option<ForwardRunConfig>`; in the Mode::Paper branch
derive `feed_symbol` from the selection (replacing the hardcoded `BTCUSDT`),
build the registry via `build_registry_for`, and pass the budget to
`spawn_trading_loop`.

**D4 — the MVP forward run is real-time-only; the replay preview is deferred.**
Paper mode is live Binance WS, real-time (confirmed during the soak). The Live
view filling in over time IS journey step 5. A "what it would have done" static
fast-replay preview (cheap — `run_scenario` over the recent window with budget
sizing) is an additive convenience, not a gate; deferred to v0.2 (F5b, OQ-1).

**D5 — the budget becomes the loop's starting capital, so the existing equity
publish IS the budget equity.** Add a trailing `budget: Option<Money<Usdt>>` arg
to `spawn_trading_loop`; when `Some(b)` set `initial_capital = b.amount()`
(start `cash` at the budget) and build the sizer via `with_budget_cap`. Every
per-bar `equity = cash + position·mark` already published to the `EventBus` PnL
channel + the durable store (ADR-0052) is then the budget equity — **zero new
equity plumbing**. Live computes running P/L = equity − budget. When `None`, the
loop is byte-identical to today (research + legacy paper unaffected; ADR-0053
unified-loop determinism intact).

**D6 — the launch lifecycle is a POST-BOOT hot-swap of the trading-loop task,
NOT the boot-config field (2026-06-21 amendment — supersedes the D3/D5
`RunHandles.forward` boot-config assumption).** The original D3/D5 put the
selection on `RunHandles.forward`, read once when `run()` spawns the paper loop
at boot. **This is structurally insufficient**: `run(handles, cancel)` consumes
`RunHandles` exactly once at boot, but the user's selection arrives in the iced
thread *after* the bake-off completes (post-boot). A once-consumed boot field
can never carry a post-boot selection — which is why both 2026-06-20/21 dev
passes shipped a FAKE (the default loop kept running; the UI relabelled
`equity − 200` of the 100 000-capital default equity). The real mechanism:

1. **`ForwardCommand::Launch(ForwardRunConfig)`** (a new `core`-typed enum) flows
   from the iced thread into the runtime over a new **`mpsc`**;
   `RunHandles.forward_rx: Option<Receiver<ForwardCommand>>` **replaces**
   `RunHandles.forward`. The headless `trading` bin + the soak harness pass
   `None` (byte-identical to today).
2. **`runtime::run`'s `Mode::Paper` branch becomes a `paper_loop_supervisor`**
   that retains the loop's spawn context (bus, ledger, `equity_store`,
   `reflection_writer`, risk/backtest config, `btc_closes_seed`, the
   paper-feed-builder, the per-loop cancel token, and the loop's `AbortHandle`).
   It spawns the **initial** loop, then — only when `forward_rx.is_some()` —
   `select!`s on `forward_rx.recv()` + `cancel.cancelled()`. On `Launch(cfg)` it
   **(a) cancels the current loop's child token, (b) awaits its `AbortHandle`
   drain (the no-double-equity-writer guarantee — the loop is the sole paper
   equity writer), (c) spawns a fresh `spawn_trading_loop`** with
   `build_registry_for(Some(&cfg))` (selected strategy), a fresh feed on
   `cfg.symbol`, and `Some(cfg.budget)` — **reusing the same `bus`/`ledger`/
   `equity_store`/`boot_id`**. `spawn_trading_loop` is widened to **return its
   `AbortHandle`**.
3. **EventBus/subscription continuity is the load-bearing property.** The Live
   view's `stream_pnl` holds `bus.pnl()` — a long-lived `broadcast::Receiver`
   polled in a `recv()` loop — anchored to the `Arc<EventBus>`, **not** to any
   producer task. Swapping the producer keeps the consumer intact; the swapped
   loop's first `publish_pnl` (now the budget equity) reaches Live unchanged.
4. **`cockpit_live` holds the `Sender`** (`forward_tx` in `AppState`) and **sends
   `ForwardCommand::Launch(cfg)`** from the `BakeoffRunCompleted`-with-crowned-row
   arm (strategy = crowned/picked id, symbol = `mirror.coin`, budget =
   `budget_eur() ?? 200`), **alongside** the existing
   `ForwardPaperTradeStarted(budget)` — which is now ONLY the UI-frame trigger
   (sets `forward_budget` so the P/L card paints); the real *launch* is the send.

**Why (A) over (B)/(C):** (B) whole-runtime relaunch creates a NEW `EventBus`
that breaks the iced subscription (the recipe holds the old `Arc<EventBus>`) and
a new `boot_id` that fragments the T806/ADR-0052 audit trail — reusing the bus
collapses (B) into (A). (C) a separate forward-run read from `LiveEquityStore`
adds a second equity surface + a per-run reader the cockpit's live-bus-anchored
Live view does not have — deferred to v0.2 (F5c). `cargo tree -p ui` stays
unchanged (the command payload is `core` types); `forward_rx = None` is
byte-identical to today (research + soak + reflection wiring + 119/119 anchors
hold by construction).

## Alternatives considered

- **F4 as a new sizer type / new call path** — rejected: the budget is a second
  hard notional cap that composes trivially with the existing exposure-cap
  clamp; a parallel sizer duplicates the validated `compute_qty` →
  `size_and_validate` → `Order::new` path for no gain.
- **F4 as an equity re-scale (`equity = budget` into the fraction)** — rejected
  as the *cap*: it conflates budget with account equity and lets a winning
  streak compound deployment above €200. (We do seed starting capital at the
  budget per D5, but the permanent cap per D1 is what enforces the ceiling.)
- **`ui` builds the strategy / a `ui → strategy` edge** — rejected: violates the
  layering invariant. The `build_registry` seam exists precisely so `ui` names
  only a `StrategyId` (a `core` type); `build_registry_for` widens that seam, it
  does not breach it. `cargo tree -p ui` stays unchanged.
- **Forward-run home in a new `crates/advisor` or in `ui`** — rejected: the
  runtime + the registry builder + `RunHandles` already live in `agent`, which
  already depends on `strategy`. Homing F5 anywhere else forces a new dependency.
- **Replay preview as MVP** — rejected for MVP: it adds a second equity surface
  (replayed vs live) the Live view must disambiguate, and the journey is the
  real-time forward run. Deferred, not dropped.
- **(D6) Boot-config `RunHandles.forward` field as the launch mechanism** —
  rejected (2026-06-21): `run()` consumes `RunHandles` once at boot, but the
  selection is post-boot, so the field can never launch the real run. This is
  the exact gap that produced the two prior FAKE passes.
- **(D6) Whole-runtime relaunch (B)** — rejected: a new `EventBus` breaks the
  iced subscription; a new `boot_id` fragments the audit trail; reusing the bus
  collapses it into the hot-swap (A).
- **(D6) Separate forward-run read from `LiveEquityStore` (C)** — deferred to
  v0.2: the cockpit Live view is anchored to the live `EventBus` PnL stream, not
  a per-run equity reader; (C) adds a second equity surface + a run-selector for
  no MVP benefit when the runtime is already in-process.

## Consequences

- **CLAUDE.md non-negotiable satisfied:** F4 (a sizing modifier) ships with the
  day-1 baseline-equity-divergence e2e (D2). Skipping or weakening it to a
  PASS-only test re-opens the v3-vol-overlay no-op class — the tester MUST
  independently confirm the FAIL-before.
- **Layering invariant held by construction:** the bridge passes `core` types
  (`StrategyId`/`Symbol`/`Money`); resolution is in `agent`. The gate is
  `cargo tree -p ui` unchanged (no `strategy`/`exec`/`forecast`/`llm` edge).
- **Determinism + anchor neutrality:** `compute_qty` stays `Decimal`-pure (no f64
  boundary added); the `budget=None` paths are byte-identical to today, so
  research determinism (ADR-0053) and the 119/119 anchored backtest body-SHAs
  stay byte-identical (`scripts/verify_anchors.sh` is the gate, before+after).
  No `spec/*/reports/` body is written by a forward run; no `spec/anchors.toml`
  SHA changes → ADR-0038 § D6 + the anchor-mutation-requires-an-ADR rule are
  untriggered.
- **Paper-only hard cap enforced:** `compute_qty` never returns
  qty·price > budget; the budget is the simulated ceiling (product § Risk).
- **Free reuse:** the durable equity store (ADR-0052) + the reflection-wired
  loop (commit `e9da47f`) are inherited via the existing `Some(...)` args — the
  forward run's budget equity is durable and its closed trades produce lesson
  cards with no new wiring.
- **Live render-layer proof required:** the €200 P/L surface is verified at the
  rendered-pixel layer with a populated fixture + a negative control (the
  Live-view-saga precedent); a no-panic boot is not proof. **(D6 amendment) the
  proof must trace the rendered P/L to a `Some(budget)` REAL forward loop (cash
  starts at budget), not an isolated `PnlSnapshot` fixture** — an isolated
  fixture passes even with the fake.
- **(D6) Hot-swap consequences:** `spawn_trading_loop` returns its `AbortHandle`;
  `runtime::run`'s paper branch becomes a supervisor select-loop when
  `forward_rx.is_some()`; the swap awaits the old loop's drain → no
  double-equity-writer. The same `boot_id`/ledger/store/bus carry the forward
  run → one continuous audit trail. The pre-selection default loop's warm-up
  bars are honestly attributed to the default strategy id (OQ-4).
- **Open for the operator:** OQ-1 (real-time-only vs add the replay preview),
  OQ-2 (default-to-crowned vs force a pick), **OQ-4 (swap-boundary warm-up
  bars), OQ-5 (forward-run lesson cards)** — all carry recommended defaults
  (real-time-only; default-to-crowned; leave-warm-up-bars; wire-forward-lesson-
  cards); none is a build gate.

## Changelog

- 2026-06-21 (architect): **§ D6 launch-lifecycle amendment.** Recorded that the
  D3/D5 boot-config `RunHandles.forward` field is structurally insufficient for
  the POST-boot bake-off selection (the cause of two FAKE dev passes), and that
  the real launch is a **hot-swap of the trading-loop task on the already-running
  runtime (mechanism A)**: a `core`-typed `ForwardCommand::Launch` over a new
  `mpsc` (`RunHandles.forward_rx`, replacing `forward`) → a `paper_loop_supervisor`
  that aborts-old + spawns-new on the SAME `EventBus`/ledger/store/boot_id;
  `spawn_trading_loop` returns its `AbortHandle`; the Live subscription survives
  because it is anchored to the `Arc<EventBus>` broadcast receiver, not the
  producer task. Rejected (B) relaunch + deferred (C) durable-store-read.
  Upgraded the render-proof requirement to trace to a real `Some(budget)` loop.
  See `spec/advisor-forward-paper/feature.md` § 4.0–4.4 + the rewritten F5 tasks.
