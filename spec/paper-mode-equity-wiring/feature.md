---
slug: paper-mode-equity-wiring
status: draft
owner: analyst
updated: 2026-06-11
version: 0.1.0
trace: REQ-LIVE-EQUITY-PAPER-001
---

# Paper-mode equity wiring — make paper-mode cockpit equity REAL (not a flat line)

## Changelog

- 2026-06-11 (analyst): initial draft (v0.1.0). Scoped from the presenter-
  discovered gap (2026-06-11): in PAPER mode the cockpit equity curve is a
  flat line at `initial_capital_usdt` because the paper branch of
  `runtime.rs` connects a live Binance WS feed + an idle reconciler whose
  `state_tx` is **dropped**, so equity never moves — while the freshly-shipped
  `live-equity-history-durable` (ADR-0052) faithfully persists and re-hydrates
  that flat line. This feature makes paper-mode equity real so paper mode
  becomes a true rehearsal of live. R1–R7, AC1–AC8, Q1–Q6 below; recommended
  default attached to every open question. Adjacent context + inherited
  invariants from `spec/live-equity-history-durable/feature.md`.

## Why

The cockpit Live screen's equity curve is **flat at `initial_capital_usdt`
in paper mode** — it never moves no matter how long the agent runs. The
presenter caught this on 2026-06-11 while demoing the just-shipped durable
equity history: the persistence rails are real (the `equity_snapshots` table,
the `LiveEquityStore` trait, boot hydration), but the **value being persisted
and re-hydrated is a constant**. Paper mode therefore is NOT a rehearsal of
live — it is a rehearsal of "the account never trades".

### Root cause — evidence with file:line

Paper mode today (`crates/agent/src/runtime.rs`, `Mode::Paper` arm, lines
544–684) spawns exactly four things:

1. **Per-venue WS feed supervisors** (`spawn_venue_supervisor`,
   `runtime.rs:611-623`) — Binance always, Coinbase/Kraken opt-in. These run
   `spawn_feed_taps` which republish bars + ticks onto the `EventBus` `bars`
   / `ticks` channels (`runtime.rs:817-881`). **Bars and ticks reach the bus.
   Nothing consumes them for trading.**
2. **A stale-data watchdog** (`spawn_market_health_watchdog`,
   `runtime.rs:633-642`) — read-only health monitor; does not trade.
3. **An idle periodic reconciler** (`runtime.rs:655-683`) seeded at
   `ReconcilerState { cash: initial_capital_usdt, position_qty: 0,
   last_mark: 0, realized_pnl: 0, cost_basis: 0 }`. Its `state_tx` is
   **explicitly dropped** (`runtime.rs:669-674`):

   ```rust
   let (state_tx, state_rx) = tokio::sync::watch::channel(initial_state);
   // The state_tx is dropped here (no external updater in the
   // current paper-mode stub); the reconciler reads the initial
   // value which is correct at boot. A future paper-mode trading
   // loop will use state_tx to push equity updates.
   drop(state_tx);
   ```

   So the reconciler's `after_bar_close` (`reconciler.rs:157`) computes
   `equity = cash + position_qty * last_mark = initial_capital + 0 * 0 =
   initial_capital` **every tick, forever**, publishes that constant
   `PnlSnapshot`, and (since the equity store is wired, A6) persists the
   constant row once per minute.
4. **Risk-telemetry + mode-forwarder** (`runtime.rs:692-711`) — placeholders /
   plumbing, not trading.

**There is no strategy loop, no order placement, and no paper fills in paper
mode.** The `registry` (carrying `SmaCrossover`) is constructed and
file-watched (`runtime.rs:370-395`) but in paper mode **nothing calls
`registry.on_bar(&bar)`**. The `paper_engine_publisher` helper
(`runtime.rs:762`, routing fills → `bus.fills()` + `bus.positions()`) exists
and is exercised **only by the research loop** (`spawn_research_trading_loop`,
`runtime.rs:945`, the lone `registry.on_bar` call site at `runtime.rs:1037`).

### The reference implementation already exists

`spawn_research_trading_loop` (`runtime.rs:945-1192`) is the complete,
correct per-bar equity pipeline we need — it just runs against the **replay
feed** instead of the **live feed**:

- subscribes directly to `feed.subscribe_bars(symbol, tf)` (bypassing the
  broadcast bus so fast-replay bars are never dropped — `runtime.rs:1000`);
- runs `registry.on_bar(&bar)` → `risk::size_and_validate` → `PaperEngine::step`
  (the same matching engine, seed `0x00C0_FFEE`, `runtime.rs:1008`);
- maintains `cash`, `position.base_qty`, `realized_pnl`, `cost_basis` from its
  own fills;
- publishes fills + positions via `paper_engine_publisher`
  (`runtime.rs:1122`);
- computes `total_equity = cash + position.base_qty * mark` per bar and
  publishes a rich `PnlSnapshot { …, as_of: now(), bar_ts: Some(bar.close_ts) }`
  (`runtime.rs:1169-1180`).

The Binance live feed already produces real per-bar `bar_ts`: `subscribe_bars`
forwards only `is_closed` klines (`binance.rs:339`) with `close_ts =
millis_to_timestamp(k.close_time)` (`binance.rs:348`). In paper/live mode
`bar.close_ts ≈ now()` (a minute bar closes at wallclock), so the
two-timestamp contract degrades gracefully (`as_of` and `bar_ts` nearly
coincide) — exactly as `reconciler.rs:146-148` and `views.rs:108` document.

**So the fix is fundamentally a wiring problem: run the existing per-bar
equity pipeline against the live feed in paper mode, gated to paper mode.**
The open design question (Q1) is whether to do that by *unifying* the
research loop into one shared per-bar equity path (preferred if honest) or by
*generalizing the loop's feed source* with a thin paper-specific seam.

### What is settled and MUST NOT be reopened (inherited constraints)

These are locked by `live-equity-history-durable` (ADR-0052) + the
`cockpit-live-equity-render-guard` saga (~6 round-trips, `40f5de9`). The
architect designs **around** them, not against them:

1. **The two-timestamp contract (approach A) is settled.** Each `PnlSnapshot`
   carries BOTH `as_of: Timestamp` (wallclock `Timestamp::now()`, the
   delivery/freshness key, monotone) AND `bar_ts: Option<Timestamp>`
   (`#[serde(default)]`, the bar/data close time the chart plots on its
   x-axis). Do NOT re-conflate them — stamping `as_of = bar.close_ts` broke
   the curve and was reverted (`40f5de9`). The new paper loop stamps both
   exactly as the research loop does (`runtime.rs:1177-1178`).

2. **The `LiveEquityStore` trait boundary is the only durable-write API**
   (ADR-0052 A1) and the **research-writes-nothing gate (A2)** is law: the
   persistence writer is gated `mode != Research` at the mint site. This
   feature does NOT touch the store, the trait, the migration, or the gate —
   it only makes the VALUE the (paper) mint site already persists be real
   instead of constant. The mode gate stays exactly where it is.

3. **The render-verifiable harness is the gate, not unit tests.**
   `crates/ui/tests/live_equity_render.rs` rasterizes the real Live screen and
   counts ACCENT polyline pixels. Project law (MEMORY.md *Verify UI at the
   render layer*): the "equity actually moved" proof lands here — but see R5 /
   AC6: the existing `count`/`x_span` signals do NOT distinguish a flat curve
   from a non-flat one (the `equity_curve.rs` flat-line guard centers a
   degenerate series as a full-width horizontal line, `equity_curve.rs:178,195`).
   The non-flat proof needs the **Y-variation** signal.

4. **Money is `Decimal` / `Money<Usdt>`, never `f64`; every external I/O
   behind a trait; determinism (no RNG in the decision path, fixed paper-engine
   seed).** The research loop already honors all three; the paper loop inherits
   them verbatim.

## Requirements

In **paper mode** the agent runs a strategy + paper-execution loop against the
**live** market feed, so the per-bar equity it publishes (and the durable
series it persists via ADR-0052) **moves with paper fills** instead of staying
flat at `initial_capital_usdt`. Research-replay mode is **unchanged**. The
cockpit Live curve — live and hydrated — then renders a **non-flat** equity
history.

- **R1 — Paper mode runs a strategy + paper-execution loop.** When the agent
  runs in **paper mode**, a trading loop consumes the live feed's closed bars,
  runs the strategy registry (`registry.on_bar`), sizes orders via the risk
  sizer, executes them through the paper matching engine, and maintains
  `cash` / `position` / `realized_pnl` / `cost_basis` from the resulting
  fills — exactly the pipeline `spawn_research_trading_loop` already runs for
  replay. **Fills + positions are published** (via `paper_engine_publisher`)
  so the Live fills tape + positions panel populate, and **a per-bar
  `PnlSnapshot` with the real `total_equity = cash + base_qty * mark` is
  published** (Q1 decides the structural shape: unified path vs. feed-source
  seam).

- **R2 — The reconciler's equity reflects the paper book (no more dropped
  `state_tx`).** The paper-mode equity that drives the durable write must be
  the **real** mark-to-market of the paper position book, not the boot
  constant. The architect picks the mechanism (Q1/Q2): either (a) the paper
  trading loop becomes the single mint site (publishing + persisting per bar,
  like the research loop does — the reconciler's periodic role narrows or
  retires in paper mode), or (b) the trading loop drives `state_tx` and the
  existing periodic reconciler stays the mint site. **The `drop(state_tx)`
  stub at `runtime.rs:674` is removed either way.** Whichever path, exactly
  ONE writer mints the per-bar paper snapshot (no double-write, no two
  conflicting equity series).

- **R3 — Mark-to-market source + cadence (paper).** Equity marks to the
  **live feed's closed-bar close price** (`bar.close`), the same mark the
  research loop uses (`runtime.rs:1031`). Cadence is **per closed bar**
  (1-min on the BTCUSDT/1m feed), driven by the bar stream — NOT a free-
  running 60 s timer divorced from bar arrivals. `bar_ts = bar.close_ts`
  (real, from the live kline); `as_of = now()`. In paper mode bar close ≈
  wallclock, so the two nearly coincide (settled, R6 of the durable feature).
  Between bars (or if the feed stalls) equity holds its last value — the
  watchdog (`runtime.rs:633`) already surfaces feed staleness; this feature
  does not add a separate marking timer (Q3).

- **R4 — Research mode is byte-for-byte unchanged.** This feature adds a
  paper-mode loop; it MUST NOT alter `spawn_research_trading_loop`'s behavior,
  the replay path, or any backtest. The 19 anchored backtest body-SHA-256
  reports stay byte-unchanged (the backtest binary never calls `runtime::run`;
  research replay's equity math + `bus`/`store` wiring are untouched). If Q1
  resolves to a unified path, the refactor MUST preserve the research loop's
  exact outputs (proven by the existing research tests + the anchor gate).
  **AC7.**

- **R5 — The persisted + hydrated paper curve is NON-FLAT, proven at the
  render layer.** A paper session whose fills move equity produces a durable
  series with **Y-variation**, and on boot the hydrated curve renders
  **non-flat**. The existing render harness (`live_equity_render.rs`) asserts
  ACCENT `count ≥ CURVE_DREW_MIN_ACCENT` + `x_span ≥ CURVE_X_SPAN_MIN` — but
  the `equity_curve.rs` flat-line guard (`:178`) renders a *flat* series as a
  **centered full-width horizontal line**, which PASSES both of those. The
  non-flat proof therefore needs a **Y-variation signal**: assert the ACCENT
  bounding-box height `(max_y - min_y) ≥ CURVE_Y_VAR_MIN` (the harness already
  tracks `min_y`/`max_y` in `AccentStats`, `live_equity_render.rs:142-148`).
  A flat-line regression (the current bug) fails this; a real moving curve
  passes. **AC6 — THE gate.**

- **R6 — Honest semantics; no overclaim.** The paper curve is a **real paper
  trading history** (paper fills against the live feed) — it is NOT a live-
  money result and NOT a "characterized" backtest. Any caption stays honest
  (the inherited `LIVE_SINCE_INCEPTION_CAPTION` / `LIVE_SESSION_RETURN_CAPTION`
  apply unchanged — this feature adds no new caption; it makes the existing
  ones reflect real movement). The strategy that trades in paper mode is the
  one already registered (`SmaCrossover` from `config/agent.toml`); paper mode
  does not silently invent a different strategy than the operator configured.

- **R7 — Determinism + no-real-orders guarantee preserved.** Paper mode
  places **no real exchange orders** — fills come only from the in-process
  `PaperEngine` (seed `0x00C0_FFEE`, the research-loop value), marking to the
  live feed. No `SystemTime::now()` in the trade-decision path (the fill
  timestamp is `bar.close_ts`, as in the research loop / `PaperEngine::step`).
  The no-real-orders invariant is the entire point of "paper" and is asserted
  (no live exchange-execution client is constructed in the paper trading loop).

### Out of scope (explicit)

- **Changing the durable store, the `LiveEquityStore` trait, the `013`
  migration, or the A2 research-writes-nothing gate** — all settled law
  (ADR-0052). This feature changes only the VALUE that the existing paper mint
  site persists.
- **Live (real-money) order execution** — paper mode stays paper (in-process
  `PaperEngine`, no exchange execution client). A real-money execution path is
  a separate, much larger feature with its own risk/compliance gates.
- **New strategies or strategy selection UI** — paper mode trades whatever the
  registry already holds (`SmaCrossover`); multi-strategy paper books / a
  strategy picker are separate features.
- **A manual/static position-book entry UI** (operator types in positions to
  mark-to-market) — this was a candidate scope (Q1 option (a) "mark a static
  book") but is **rejected** as the primary path: it is a different product
  surface (a position-entry form) and does not make paper a rehearsal of live
  the way running the real strategy loop does. Named here only as the
  considered-and-rejected alternative.
- **Live Sharpe / CAGR / win-rate math** — still absent in `core` (unchanged
  from the durable feature); the KPI strip's Sharpe/CAGR/Win cards stay `—`.
  This feature makes Total-return + Max-DD reflect real movement; it adds no
  new KPI math.
- **Multi-symbol paper trading** — the live feed + research loop are hardcoded
  BTCUSDT/1m (`runtime.rs:480-481`); a multi-symbol paper book is a separate
  feature.
- **Re-deriving / backfilling historical paper equity** — out of scope exactly
  as in the durable feature; this persists forward from feature-land.

## Architecture findings (for the architect — analysis, not hand-waving)

### The decisive question: unify the per-bar equity path, or seam the feed source?

The research loop (`spawn_research_trading_loop`) is ~95% the code paper mode
needs. The only real difference is the **feed**: research subscribes to
`ReplayFeed`, paper would subscribe to `BinanceFeed`. Both are
`Arc<dyn MarketDataSource>`; both expose `subscribe_bars(symbol, tf)`. The
loop already takes `feed: Arc<dyn MarketDataSource>` as its first parameter
(`runtime.rs:946`) — **it is already feed-source-agnostic.** Three structural
options, weighed on durability (does the architect's lock carry forward), the
research byte-stability obligation (R4), and effort:

| Option | What it is | Research byte-safety | Durability | Effort |
|--------|-----------|----------------------|------------|--------|
| **(a) Unify: one `spawn_trading_loop(feed, mode, …)` for research AND paper.** Rename/generalize `spawn_research_trading_loop`; the paper arm calls it with the Binance feed; the research arm calls it with the replay feed. The equity math, sizing, paper-engine, publish, and (via the already-wired store) persist all live in ONE place. | **Must be proven**: the unified fn MUST produce byte-identical research outputs (same fills, same snapshots). The loop body is already mode-agnostic (no `config.mode` branch inside it), so a careful rename + a `feed`/`store` parameter is behavior-preserving. Guarded by the existing research tests + the anchor gate (the backtest path never calls `runtime::run`, so anchors are structurally safe). | **HIGHEST.** One per-bar equity path for the whole agent. No "research does it one way, paper another" drift. The next equity/sizing/exec change touches one function. The architect's M-T1 lock carries forward across research, paper, and the eventual live path. | **M.** Rename + generalize one fn (~30-60 LoC of signature/wiring churn); wire the paper arm to call it; retire the idle-reconciler stub in paper; reconcile the now-redundant periodic reconciler's role (it becomes paper's imbalance-checker only, or the loop subsumes it). |
| **(b) Feed-source seam only: keep `spawn_research_trading_loop` as-is, add a thin `spawn_paper_trading_loop` that delegates to a shared inner body.** Extract the loop body into a private `run_trading_loop(feed, publisher, store, …)`; both public spawns wrap it. | Same proof obligation as (a) but the research **public** entry point is untouched (lowest blast radius on the research call site). | **HIGH** (shared inner body = one equity path) but spawns two public fns that can drift in their wrappers. | **S-M.** Extract inner body; add the paper wrapper + its feed/publisher/store wiring; retire the stub. |
| **(c) Duplicate: copy the research loop into a new paper loop, wire the live feed.** No shared body. | Research literally untouched (copy, don't move) — trivially byte-safe. | **LOWEST.** Two ~240-line equity loops to keep in sync forever — the exact "compute scale but never apply it" class of bug the CLAUDE.md non-negotiable exists to prevent (a fix to one silently skips the other). Spawns a v0.2.0 "de-duplicate the trading loops" cleanup brief. | **S now**, but **+~M v0.2.0 cleanup commitment.** |

**Recommendation: (a) unify — one `spawn_trading_loop` for research AND paper
(Recommended).** Per the durable-over-quick rule the `(Recommended)` tag goes
on the most durable choice: a single per-bar equity path for the whole agent
is the architecture that carries forward to the eventual live-money path
without a third copy. The loop body is **already** mode-agnostic and
feed-parameterized (`runtime.rs:946`), so unification is a behavior-preserving
rename + a `feed`/`store`/`mode-label` parameter, not a rewrite — the research
byte-safety obligation (R4) is dischargeable by the existing research tests +
the anchor gate. **This is exactly the "equity math errors are what the gate
exists to catch" surface** — keeping ONE path is the structural defense.

**If-budget-tightens fallback: (b) the shared-inner-body seam.** It keeps the
research public entry point untouched (smallest blast radius on the proven
research path) while still sharing the equity math through one inner body — so
it does NOT spawn the v0.2.0 de-dup cleanup that (c) would. Name (b) as the
fallback if the operator wants to minimize churn on the just-shipped research
loop. **(c) duplication is rejected** — it is the literal anti-pattern the
CLAUDE.md baseline-equity-divergence non-negotiable was written against.

### Mark-to-market + cadence (Q3) — the bar stream IS the clock

The research loop marks to `bar.close` per closed bar and the bar stream is
its clock (`runtime.rs:1018`, `bar_stream.next()`). Paper mode should be
identical: the **live closed-bar stream** drives the cadence. The current
paper-mode reconciler runs on a **free 60 s timer** (`runtime.rs:677`,
`reconciler_interval_ms = 60_000`) that is NOT coupled to bar arrivals — that
is a second reason the curve is flat-and-dumb (it ticks on a timer reading a
never-updated state). Under the recommended unify (Q1=a), the **bar stream
becomes the cadence** and the free timer's marking role goes away; the
periodic reconciler's only remaining job is the imbalance check
(`reconciler.rs:214-231`), which can stay on its timer reading the loop's
live state (Q2 sub-decision: keep the imbalance reconciler, feed it the real
state, or fold its check into the loop).

### Interaction with the shipped persistence (Q4 / AC framing)

The ADR-0052 persist site is `ReconcilerTask::after_bar_close`
(`reconciler.rs:181`) — it persists whatever `ReconcilerState` holds. TODAY
that state is the boot constant (flat). Two clean ways to make the persisted
value real, depending on Q1:

- **If Q1=(a) unify:** the unified trading loop already publishes the rich
  `PnlSnapshot` per bar (`runtime.rs:1171`). The cleanest persist site is then
  the **loop itself** (it has `cash`/`position`/`realized`/`cost_basis` in
  hand) calling the `LiveEquityStore` directly per bar, gated `mode !=
  Research` — mirroring how the research loop *would* persist if it weren't
  gated off. The idle reconciler's persist path (`reconciler.rs:181`) then is
  **not** the paper mint site (it retires or becomes imbalance-only). This is
  the honest shape: ONE writer (the loop), one series.
- **If Q1=(b)/keep-reconciler-as-mint:** the trading loop drives `state_tx`
  with the real `ReconcilerState` each bar; the existing
  `after_bar_close` persist path then writes real values unchanged. This
  reuses the shipped persist site verbatim but couples the loop → watch
  channel → reconciler, and the reconciler's free 60 s timer must be re-paced
  to the bar stream (or it double-mints between bars).

**Recommendation: persist from the unified loop (Q1=a → one writer), retire
the paper idle-reconciler's mint role.** The architect pins the exact persist
seam. Either way the AC1 contract of the durable feature ("one row per bar
with REAL values") is the natural extension — `equity_store_integration.rs`
already asserts one-row-per-bar; the new assertion is that across a multi-fill
paper session the rows' `total_equity` values are **not all equal** (Y-variation
at the data layer), the necessary precursor to the R5 render proof.

### The render proof needs Y-variation, not just ACCENT-count (the AC6 subtlety)

`equity_curve.rs:173-201`: when the series y-range collapses
(`y_range_degenerate`, the flat-line guard), every point renders at
`frac_y = 0.5` — a **centered, full-width horizontal line**. That line draws
**plenty** of ACCENT pixels and spans the **full** x-axis. So the existing
`live_equity_render.rs` assertions (`count ≥ CURVE_DREW_MIN_ACCENT = 200`,
`x_span ≥ CURVE_X_SPAN_MIN = 400`, `live_equity_render.rs:228,235`) **PASS on a
flat curve** — they prove "a curve drew", not "the curve moved". The current
paper-mode bug (flat at initial capital) would sail through them. The available
discriminator is the ACCENT **bounding-box height**: `AccentStats` already
tracks `min_y`/`max_y` (`live_equity_render.rs:142-148`); a real moving curve
has a tall bbox, a flat line a ~1px-tall bbox. **R5/AC6 introduce
`CURVE_Y_VAR_MIN`** and assert `(max_y - min_y) ≥ CURVE_Y_VAR_MIN` for the
real paper curve, plus a self-proving contrast (a flat series renders bbox
height `< CURVE_Y_VAR_MIN`) so the gate provably distinguishes flat from
non-flat — the same belt-and-braces pattern the existing harness uses for
healthy-vs-broken.

### Effort honesty — touched crates + test surface

This is **~M**, exec-led, **almost entirely in `crates/agent`** — the UI and
store are already done by the durable feature:

- **`crates/agent` (`runtime.rs`)** — the bulk. Under Q1=(a): rename +
  generalize `spawn_research_trading_loop → spawn_trading_loop(feed, mode,
  store, …)`; call it from the paper arm with the Binance feed +
  `Some(store)`; remove the `drop(state_tx)` idle-reconciler stub
  (`runtime.rs:655-683`); reconcile the periodic reconciler's role (imbalance-
  only, fed real state, or folded in). Pin the per-bar persist seam (loop-
  direct vs. via `state_tx` → `after_bar_close`).
- **`crates/agent` (`reconciler.rs`)** — small: the periodic reconciler either
  loses its paper mint role (loop persists instead) or keeps it with real
  state + bar-paced cadence. The `build_snapshot_row` helper
  (`reconciler.rs:255`) is reused if the reconciler stays the writer.
- **`crates/exec` / `crates/data` / `crates/risk` / `crates/backtest`** — **no
  change**: `paper_engine_publisher`, `BinanceFeed`, the sizer, and
  `PaperEngine` are all already in use by the research loop. This feature
  re-points an existing pipeline at the live feed.
- **`crates/audit` / `crates/ui` / `crates/core`** — **no change**: the store,
  trait, migration, hydrate path, `PnlHydrated` arm, captions, and
  `PnlSnapshot` are all shipped (ADR-0052). The UI renders whatever values
  arrive; making them real needs zero UI change. **The only UI-test change is
  extending `live_equity_render.rs` with the Y-variation assertion (R5/AC6)** —
  a test, not a widget.
- **Test surface:** a paper-mode trading-loop integration test (live feed
  faked via `data::FakeFeed`/`MockFeed` emitting bars that move price, asserting
  fills are produced + the per-bar snapshots' `total_equity` is non-constant);
  the research-byte-safety proof (R4 — the unified loop's research outputs
  unchanged, anchor gate green); the **non-flat render gate** in
  `live_equity_render.rs` (R5/AC6); the no-real-orders assertion (R7). The
  existing `equity_store_integration.rs` AC1 extends to assert non-constant
  `total_equity` across a moving session.

### Does the CLAUDE.md baseline-equity-divergence non-negotiable apply? — assessed honestly, NOT rubber-stamped

The durable feature stamped this gate **N/A** because it was a read-only
monitor persistence feature (no strategy, no sizing, no decision variable).
**This feature is different and the assessment must be different.** This
feature *introduces a strategy + sizing + execution loop into paper mode* —
`registry.on_bar` → `risk::size_and_validate` → `PaperEngine::step`. There IS
a decision variable here (the strategy signal driving position changes), and
the entire point is that equity **diverges from the flat baseline** when the
strategy trades.

The CLAUDE.md non-negotiable: *"Every strategy overlay or sizing-modifier ships
with a baseline-equity-divergence end-to-end test from day 1 … an e2e test that
asserts the overlay's output equity diverges from the un-targeted baseline
equity by ≥ 1 bp when the strategy decision variable is non-trivial."* The
precedent (`v3-volatility-forecaster-noop-fix`) is **exactly this bug class**:
a value computed but never applied, caught only by an e2e divergence assertion,
not by unit/anchor tests.

**Assessment: the SPIRIT of the gate applies, and R5/AC6 IS its instantiation
here.** This feature is not adding a *new overlay on top of an existing
strategy* (the literal trigger), but it IS making equity move with a strategy
decision where it was previously a flat constant — the precise "computed but
never applied" failure mode (`state_tx` dropped, equity never updated). The
honest call is: **do NOT rubber-stamp N/A.** Instead, **satisfy the gate's
intent directly** — R5/AC6's "the paper curve is provably non-flat (Y-variation
≥ epsilon) when the strategy trades, vs a flat `initial_capital` baseline" IS a
baseline-equity-divergence e2e assertion. The baseline is the flat
`initial_capital` line (the current bug); the diverged curve is the real paper
session; the epsilon is `CURVE_Y_VAR_MIN` at the render layer (and a non-
constant-`total_equity` assertion at the data layer in
`equity_store_integration.rs`). The architect should record this explicitly in
the ADR (not as "N/A" but as "satisfied by AC6 + the data-layer non-constant
assertion") so the tester gates on it. **This is the single most important
correctness statement in the feature.** (Q6 surfaces this for an explicit
operator/architect ruling rather than an analyst fiat.)

## Open questions for the architect

- **Q1 — Structural shape: unify the trading loop, seam the feed, or
  duplicate?**
  - **(a) Unify into one `spawn_trading_loop(feed, mode, store, …)` for
    research AND paper. (Recommended)** — durable: one per-bar equity path for
    the whole agent, carrying forward to the live-money path. The loop body is
    already mode-agnostic + feed-parameterized (`runtime.rs:946`), so this is a
    behavior-preserving rename + parameter, guarded by the research tests +
    anchor gate (R4). ~M.
  - **(b) Shared-inner-body seam: keep `spawn_research_trading_loop` public,
    extract a private `run_trading_loop`, add a thin `spawn_paper_trading_loop`
    wrapper.** — *if-budget-tightens fallback.* Smallest blast radius on the
    just-shipped research entry point while still sharing the equity math (one
    inner body); does NOT spawn a v0.2.0 de-dup. ~S-M.
  - **(c) Duplicate the loop for paper.** — rejected: two ~240-line equity
    loops to keep in sync = the exact "computed but never applied" bug class
    the CLAUDE.md non-negotiable guards against; spawns a v0.2.0 cleanup brief.
  - **Default: (a).**

- **Q2 — The periodic reconciler's fate in paper mode.** Today it is the
  (idle) mint site. Under Q1=(a):
  - **(a) Retire its paper MINT role; the unified loop becomes the single
    per-bar writer (publish + persist); the reconciler keeps ONLY its
    imbalance check, fed the loop's real state. (Recommended)** — one writer,
    one series; the bar stream is the cadence; no free-timer double-mint.
  - **(b) Keep the reconciler as the mint site; the loop drives `state_tx`
    with real state each bar; re-pace the reconciler from the 60 s free timer
    to the bar stream.** — reuses the shipped `after_bar_close` persist path
    verbatim but couples loop → watch channel → reconciler and needs the
    re-pace to avoid between-bar double-mints.
  - **Default: (a).** (`drop(state_tx)` at `runtime.rs:674` is removed in
    BOTH.)

- **Q3 — Mark cadence + between-bar behavior.** Recommend **bar-stream-driven**
  (the live closed-bar stream is the clock, marking to `bar.close`), NOT the
  current free 60 s timer. Between bars / on feed stall, equity holds its last
  value; the existing watchdog (`runtime.rs:633`) surfaces staleness. Does the
  architect want any intra-bar marking (mark to live *ticks* between bar
  closes)? Recommend **no** for v0.1.0 — bar-close marking matches the research
  loop, keeps the two-timestamp `bar_ts` clean, and avoids a tick-rate equity
  firehose. (A future "live tick marking" is a named follow-on.)
  - **Default: bar-stream-driven, mark to `bar.close`, no intra-bar ticks.**

- **Q4 — Per-bar persist seam (where the real value is written).** Recommend
  **the unified loop persists directly via the `LiveEquityStore` trait per bar,
  gated `mode != Research`** (Q1=a → loop is the writer). If Q1=(b)/keep-
  reconciler, persist stays at `after_bar_close` with real `state_tx`. Either
  way: fire-and-forget, never blocks/panics the loop (A6 of ADR-0052,
  unchanged); ONE writer only.
  - **Default: loop-direct persist (paired with Q1=a / Q2=a).**

- **Q5 — Which strategy trades in paper mode.** Recommend **the registry's
  already-configured strategy** (`SmaCrossover` from `config/agent.toml`,
  seeded at `runtime.rs:141`) — paper mode trades what the operator
  configured, exactly as research does (`runtime.rs:529` passes the same
  `registry`). No new strategy, no paper-only strategy selection. (R6 honesty:
  paper does not silently trade a different strategy than configured.)
  - **Default: the configured registry strategy (`SmaCrossover`).**

- **Q6 — Baseline-equity-divergence gate ruling (the load-bearing
  correctness call).** The analyst's § assessment is: this feature introduces
  a strategy/sizing/exec decision into paper mode, so the gate's INTENT applies
  (unlike the durable feature's genuine N/A), and **R5/AC6 + the data-layer
  non-constant-`total_equity` assertion ARE its instantiation** (baseline = the
  flat `initial_capital` line = the current bug; diverged = the real paper
  session; epsilon = `CURVE_Y_VAR_MIN` / non-constant rows). Recommend the
  architect **record it as "satisfied by AC6 + data-layer divergence
  assertion", NOT as "N/A"**, so the tester gates on the divergence explicitly.
  - **(a) Treat as the gate's intent satisfied by AC6 + the data-layer
    non-constant assertion (the divergence IS the feature). (Recommended)** —
    honest: equity moving with a strategy decision vs a flat baseline is
    precisely what the gate exists to verify.
  - **(b) Stamp N/A like the durable feature** — rejected as a rubber-stamp:
    the durable feature added no decision variable; this one does.
  - **Default: (a).**

## Acceptance criteria

Proportionate + testable. **The baseline-equity-divergence gate's intent
APPLIES here** (this feature introduces a strategy/sizing/exec decision into
paper mode — unlike the read-only durable feature's genuine N/A) and is
satisfied by AC6 + AC1's non-constant assertion (Q6 — to be ruled by the
architect, recommended (a)).

- **AC1 — Paper-mode loop produces moving equity (data-layer divergence).** An
  integration test drives the paper trading loop against a faked live feed
  (`data::FakeFeed`/`MockFeed`) emitting closed bars whose price MOVES and
  triggers `SmaCrossover` signals; asserts (i) paper fills are produced (via a
  recording publisher), (ii) the per-bar `PnlSnapshot.total_equity` values are
  **NOT all equal** (the series diverges from the flat `initial_capital`
  baseline by ≥ a testable epsilon), and (iii) the persisted rows (faked
  `LiveEquityStore`) carry those non-constant `total_equity` values. This is
  the data-layer half of the divergence gate (Q6).
- **AC2 — One writer, one series (no double-mint).** With the paper loop
  running, exactly ONE per-bar writer mints the snapshot — the test asserts row
  count == bar count (no second writer from a re-paced/idle reconciler
  doubling rows). Pins Q2/Q4.
- **AC3 — No real orders (paper invariant).** A test asserts the paper trading
  loop constructs **no live exchange-execution client** — fills originate only
  from the in-process `PaperEngine` (seed `0x00C0_FFEE`). The no-real-orders
  guarantee is structural, not incidental (R7).
- **AC4 — `drop(state_tx)` stub is gone.** A structural assertion (or the
  absence of the boot-constant path): the paper-mode equity is the real
  mark-to-market of the paper book, not `ReconcilerState { cash:
  initial_capital, qty: 0, mark: 0 }`. The `runtime.rs:674` stub is removed.
- **AC5 — Fills + positions populate the Live panels.** With the paper loop
  running against a moving faked feed, fills reach `bus.fills()` and positions
  reach `bus.positions()` (the `paper_engine_publisher` is wired) — the Live
  fills tape + positions panel are no longer empty in paper mode.
- **AC6 — Non-flat curve renders at the pixel layer (THE gate — the
  render-layer divergence proof).** Extend `crates/ui/tests/live_equity_render.rs`:
  feed the real Live screen a moving paper equity series (live and/or via
  `PnlHydrated` from a moving tail) and assert the ACCENT polyline is
  **non-flat** — `(max_y - min_y) ≥ CURVE_Y_VAR_MIN` (a NEW Y-variation
  threshold), in addition to the existing `count`/`x_span`. **A self-proving
  contrast** renders a FLAT `initial_capital` series and asserts its ACCENT
  bbox height `< CURVE_Y_VAR_MIN` (the flat-line guard centers it as a
  full-width horizontal line that PASSES `count`/`x_span` — so Y-variation is
  the only valid discriminator). This is the render-layer half of the
  divergence gate (Q6) and the proof the operator's flat-line bug is fixed.
- **AC7 — Research mode + backtests byte-unchanged.** The research replay
  loop's outputs (fills, snapshots, equity) are unchanged by the
  unify/seam refactor (R4); the existing research/paced-replay tests stay
  green; the 19 anchored backtest body-SHA-256 reports are byte-identical
  (the `rust-validate`/anchor gate stays green — backtest never calls
  `runtime::run`). Explicit anchor-count assertion in the test report.
- **AC8 — Fixtures `cockpit` smoke + no-feature build unchanged.** The
  fixtures-mode cockpit (no `live` feature, no agent) is byte-identical to
  today (it never runs the paper loop); a build without the live feature is
  unaffected. Every new external I/O (none expected — the feed/exec/store are
  all reused) is behind a trait; flag any new dep explicitly.

## Size estimate (S/M/L) + exec-vs-UI split

**Estimate: M**, exec-led — **≈ 90% exec (`crates/agent`), ~10% UI-test
(`live_equity_render.rs` Y-variation assertion)**. The decisive facts:

- The per-bar equity pipeline **already exists** (`spawn_research_trading_loop`)
  and is **already feed-parameterized** — this feature re-points it at the live
  feed under a mode gate, not a from-scratch build. The store, trait, migration,
  hydrate path, captions, and `PnlSnapshot` are **all shipped** (ADR-0052).
- The real work is the structural decision (Q1: unify vs seam) + removing the
  `drop(state_tx)` idle-reconciler stub + reconciling the periodic reconciler's
  role + pinning the per-bar persist seam — all in `runtime.rs`/`reconciler.rs`,
  with the **research byte-safety obligation** (R4) as the main correctness
  constraint.
- The ONE UI-surface change is a test: the Y-variation render assertion (R5/AC6)
  that turns the existing harness from "a curve drew" into "the curve moved" —
  no new widget, no new theme token, no new string.

**Bottom line for the operator:** paper mode currently rehearses "an account
that never trades" — a flat line the durable-history feature now faithfully
persists and re-hydrates. This feature runs the strategy loop the agent already
has, against the live feed it already connects, through the paper engine it
already ships, so paper-mode equity **moves with real paper fills** — making
paper a true rehearsal of live. Verified, as the bug demands, with a Y-variation
render assertion that the existing ACCENT-count gate could not provide (a flat
line passes ACCENT-count; only Y-variation proves movement). The
baseline-equity-divergence intent applies and is satisfied by that proof — not
rubber-stamped N/A.
