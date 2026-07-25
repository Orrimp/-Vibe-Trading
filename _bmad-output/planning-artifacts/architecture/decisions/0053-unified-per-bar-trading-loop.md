---
adr: 0053
title: Unify the per-bar trading loop across research and paper — one feed-parameterized spawn_trading_loop, mint-site mode gate, and the live-money carry-forward
status: accepted
date: 2026-06-11
supersedes: none
superseded-by: none
---

# ADR-0053: Unify the per-bar trading loop across research and paper

## Context

Paper mode's cockpit equity curve is a **flat line at
`initial_capital_usdt`** — it never moves no matter how long the agent
runs. The presenter caught this on 2026-06-11 demoing the just-shipped
durable equity history (ADR-0052): the persistence rails are real (the
`equity_snapshots` table, the `LiveEquityStore` trait, boot hydration),
but the **value being persisted and re-hydrated is a constant**.

Root cause (verified against `crates/agent/src/runtime.rs` @ `abdb5dc`):
the `Mode::Paper` arm (runtime.rs:544-684) spawns per-venue WS feeds, a
stale-data watchdog, and a **periodic reconciler whose `state_tx` is
explicitly dropped** (runtime.rs:669-674). No code calls
`registry.on_bar` in paper mode — there is no strategy loop, no order
placement, no paper fills. So `after_bar_close` computes
`equity = cash + position_qty * last_mark = initial_capital + 0 * 0 =
initial_capital` every tick forever, and ADR-0052 faithfully persists
that constant once per minute.

The complete, correct per-bar equity pipeline **already exists** as
`spawn_research_trading_loop` (runtime.rs:945-1192): it subscribes
directly to `feed.subscribe_bars` (bypassing the broadcast bus so
fast-replay bars are never dropped), runs `registry.on_bar` →
`risk::size_and_validate` → `PaperEngine::step` (seed `0x00C0_FFEE`),
maintains `cash`/`position`/`realized_pnl`/`cost_basis` from its own
fills, publishes fills + positions via `paper_engine_publisher`, and
publishes a rich `PnlSnapshot { …, as_of: now(), bar_ts:
Some(bar.close_ts) }` per bar. It runs against the **replay** feed; paper
mode needs it against the **live** feed.

**Verified load-bearing facts** (the design rests on these):

1. `spawn_research_trading_loop` takes `feed: Arc<dyn MarketDataSource>`
   as its **first parameter** (runtime.rs:946) and its loop body has
   **NO `config.mode` branch** — it is genuinely mode-agnostic and
   feed-source-agnostic. The live `BinanceFeed` and the `ReplayFeed`
   are both `Arc<dyn MarketDataSource>` exposing `subscribe_bars`.
2. There are exactly **two call sites**: runtime.rs:527 (research arm)
   and `crates/agent/tests/paced_replay_late_subscriber.rs:164` (the
   late-subscriber regression guard). Small, fully-enumerated blast
   radius.
3. The research loop **does not persist** — it only `publish_pnl` to the
   bus (runtime.rs:1180). It has no `LiveEquityStore` parameter today.
   The research arm `drop(equity_store)`s the store (runtime.rs:541).
   This is the one place the analyst's "behavior-preserving rename"
   understated the work: the unify must **add** a persistence parameter,
   not merely rename.
4. The Binance live feed forwards only `is_closed` klines
   (binance.rs:339) with real `close_ts` (binance.rs:348), so in paper
   mode `bar.close_ts ≈ now()` and the two-timestamp contract degrades
   gracefully.

This is loaded by ADR-0052's settled, already-shipped invariants this
ADR designs **around**, not against: the two-timestamp contract
(`as_of` wallclock / `bar_ts` data time — stamping `as_of = bar.close_ts`
broke the render and was reverted in `40f5de9`); the `LiveEquityStore`
trait as the sole durable-write API; the mint-site `mode != Research`
write gate; `Decimal`/`Money<Usdt>` never `f64`; every external I/O
behind a trait; the render-verifiable harness (`live_equity_render.rs`)
as the gate. The audit ledger anchors 19 backtest body-SHA-256 reports,
so research byte-stability is a first-class correctness obligation.

## Decision

### D1 — One feed-parameterized `spawn_trading_loop` for research AND paper

Rename and generalize `spawn_research_trading_loop` into a single
`spawn_trading_loop(feed, bus, registry, backtest_cfg, risk_cfg,
feed_symbol, feed_tf, equity_store: Option<Arc<dyn LiveEquityStore>>,
mode_label: &str, set, cancel)`. The equity math, sizing, paper-engine,
fill/position publish, and per-bar `PnlSnapshot` publish all live in
**one function** for the whole agent. The research arm calls it with the
replay feed and `None`; the paper arm calls it with the Binance feed and
`Some(store)`. The loop body is unchanged from the verified research
body except for the two additive seams in D2/D3.

This is the **highest-durability** shape: one per-bar equity path for
research, paper, and the eventual live-money path — no "research does it
one way, paper another" drift, and the next equity/sizing/exec change
touches one function. It is precisely the surface the CLAUDE.md
baseline-equity-divergence non-negotiable exists to protect ("compute a
value but never apply it" — the `v3-volatility-forecaster-noop-fix`
class); keeping ONE path is the structural defense.

**Rejected alternatives** (see Alternatives considered): the
shared-inner-body seam (b) and full duplication (c).

### D2 — The loop gains `equity_store: Option<Arc<dyn LiveEquityStore>>`; persistence is loop-direct, per bar, gated by `Some(store)`, fire-and-forget

The unified loop persists each per-bar `PnlSnapshot` directly via
`LiveEquityStore::append_equity_snapshot` immediately after
`publish_pnl`, **only when the store is `Some`**. The mode gate is
therefore **structurally enforced at the caller**, identical in spirit
to ADR-0052 D2: research passes `None` (the existing `drop(equity_store)`
semantic, made explicit as a parameter), paper passes `Some(store)`.
There is no `if mode != Research` inside the loop — the absence of a
store IS the gate.

The persist is **fire-and-forget** (ADR-0052 A6, unchanged): wrap the
write in `tokio::spawn`, log + discard on `Err`, never block or panic
the loop. The row is built by the existing
`reconciler::build_snapshot_row(snap, mode_label)` helper — which is
promoted from private to `pub(crate)` so the loop (same crate) reuses it
verbatim rather than duplicating row construction. `mode_label` is
`"paper"` for the paper arm and is never reached as a write in research
(store is `None`).

This makes the unified loop the **single per-bar writer** (publish +
persist) — one series, no double-mint. The research loop's outputs are
byte-unchanged: with `equity_store = None` the persist branch never
executes, so the only change to the research path is a wider function
signature.

### D3 — The bar stream is the cadence; the paper idle-reconciler's mint role retires; the periodic reconciler keeps imbalance-only

In the paper arm: remove the `drop(state_tx)` idle-reconciler stub
(runtime.rs:655-683) — the unified loop subsumes its minting role. The
**live closed-bar stream is the equity cadence** (mark to `bar.close`
per closed bar, exactly as research does), NOT the free-running 60 s
timer divorced from bar arrivals. Between bars or on feed stall, equity
holds its last value; the existing stale-data watchdog
(runtime.rs:633) already surfaces feed staleness — this feature adds no
separate marking timer.

The periodic `ReconcilerTask` is **not respawned as a mint site in paper
mode** under this ADR. Its imbalance-check role (reconciler.rs:214-231)
is retained as a named follow-on if/when paper mode grows a ledger-vs-
book reconciliation need; for v0.1.0 it is not wired (it was idle
anyway — it read a never-updated state). No intra-bar tick marking in
v0.1.0 (a future "live tick marking" is a named follow-on): bar-close
marking matches research, keeps `bar_ts` clean, and avoids a tick-rate
equity firehose.

### D4 — Research byte-stability is discharged by named regression guards, not assertion-by-assertion review

The unify is behavior-preserving for research mode, proven by:

- **The paced-replay late-subscriber test**
  (`paced_replay_late_subscriber.rs:164`) — its call site is updated to
  the new signature (passing the replay feed + `None` for the store);
  it still asserts fills/positions/pnl reach a late bus subscriber. This
  is the proof the unify preserves the late-subscriber + two-timestamp +
  fills-tape contract the cockpit Live view depends on.
- **The existing research-mode integration tests** (the agent test
  suite) — green and unchanged.
- **`equity_store_integration.rs` AC2** — research mode (store `None`)
  writes ZERO rows; the unify preserves the construction-time gate.
- **The anchor gate's structural independence from `runtime::run`** —
  the backtest binary computes equity via `backtest::scenarios` /
  `MatchingEngine`, never through `runtime::run` or `spawn_trading_loop`.
  The 19 backtest anchors (119 anchor rows total in `anchors.toml`) are
  byte-safe **by construction**: no anchor binds `runtime.rs`. An
  explicit anchor-count assertion in the test report (AC7) confirms it.

This is anchor-ADDITIVE (no `anchors.toml` row added; the feature changes
no hashed report body). It does NOT mutate any of the 9 anchor SHAs in
`spec/anchors.toml`.

### D5 — The baseline-equity-divergence gate's INTENT applies and is satisfied by the divergence proofs — NOT stamped N/A

Unlike ADR-0052 (a read-only monitor persistence feature with no
strategy, sizing, or decision variable — a genuine N/A), this feature
**introduces a strategy + sizing + execution decision into paper mode**
(`registry.on_bar` → `risk::size_and_validate` → `PaperEngine::step`).
There IS a decision variable (the strategy signal driving position
changes), and the entire point is that equity **diverges from the flat
`initial_capital` baseline** when the strategy trades. This is precisely
the `v3-volatility-forecaster-noop-fix` failure class the CLAUDE.md
non-negotiable guards against: a value computed but never applied
(`state_tx` dropped, equity never updated), catchable only by an e2e
divergence assertion.

**Ruling: the gate's intent applies and is satisfied by two halves of a
baseline-equity-divergence assertion — recorded as such, NOT as N/A**, so
the tester gates on the divergence explicitly:

- **Data-layer half (AC1):** a paper-loop integration test against a
  faked moving feed asserts the per-bar `PnlSnapshot.total_equity` values
  are **not all equal** (diverge from the flat baseline by ≥ a testable
  epsilon) and the persisted rows carry those non-constant values.
- **Render-layer half (AC6):** the existing `count`/`x_span` ACCENT
  signals do NOT discriminate flat from non-flat — the `equity_curve.rs`
  flat-line guard (equity_curve.rs:178) renders a degenerate series as a
  **centered, full-width horizontal line** that PASSES both
  `CURVE_DREW_MIN_ACCENT` and `CURVE_X_SPAN_MIN`. The only valid
  discriminator is the ACCENT bounding-box **height**: assert
  `(max_y - min_y) ≥ CURVE_Y_VAR_MIN` (a NEW threshold; `AccentStats`
  already tracks `min_y`/`max_y` at live_equity_render.rs:142-148), with
  a **self-proving contrast** rendering a flat `initial_capital` series
  and asserting its bbox height `< CURVE_Y_VAR_MIN`. The baseline is the
  flat line (the current bug); the diverged curve is the real paper
  session; the epsilon is `CURVE_Y_VAR_MIN`.

### D6 — No real orders; determinism preserved

Paper mode constructs **no live exchange-execution client** in the
trading loop — fills originate only from the in-process `PaperEngine`
(seed `0x00C0_FFEE`, the research-loop value), marking to the live
feed's `bar.close`. No `SystemTime::now()` in the trade-decision path
(the fill timestamp is `bar.close_ts`, as in `PaperEngine::step`). The
no-real-orders invariant is the entire point of "paper" and is asserted
structurally (AC3). The strategy that trades is the one already in the
registry (`SmaCrossover` from `config/agent.toml`) — paper does not
silently invent a different strategy than the operator configured.

## Alternatives considered

- **(b) Shared-inner-body seam: keep `spawn_research_trading_loop`
  public, extract a private `run_trading_loop`, add a thin
  `spawn_paper_trading_loop` wrapper.** Keeps the research public entry
  point untouched (smallest blast radius on the just-shipped research
  loop) while still sharing the equity math through one inner body — so
  it does NOT spawn a v0.2.0 de-dup. **Rejected as the primary** because
  the blast radius of (a) is already fully enumerated (two call sites,
  one a test we own), and (a) yields ONE public per-bar path for the
  whole agent vs two public spawns that can drift in their wrappers.
  Retained as the **if-budget-tightens fallback**: if the operator wants
  zero churn on the research public signature, (b) achieves the same
  durability (one inner equity body) at the cost of a second public
  wrapper.

- **(c) Duplicate the research loop into a new paper loop.** Research
  literally untouched (trivially byte-safe), but two ~240-line equity
  loops to keep in sync forever — the exact "computed but never applied"
  bug class the CLAUDE.md non-negotiable exists to prevent (a fix to one
  silently skips the other). **Rejected**: it is the literal
  anti-pattern, and it spawns a v0.2.0 "de-duplicate the trading loops"
  cleanup commitment.

- **Mark a static operator-entered position book** (Q1's considered "mark
  a static book"). **Rejected**: a different product surface (a
  position-entry form) that does not make paper a rehearsal of live the
  way running the real strategy loop does.

- **Persist via `state_tx` → the existing `after_bar_close` (keep the
  reconciler as the paper mint site).** Reuses the shipped persist path
  verbatim but couples loop → watch channel → reconciler and forces a
  re-pace of the reconciler's free 60 s timer to the bar stream to avoid
  between-bar double-mints. **Rejected** in favor of loop-direct persist
  (D2) — one writer, the bar stream is the cadence, no channel coupling,
  no double-mint risk.

- **Intra-bar tick marking** (mark to live ticks between bar closes).
  **Rejected for v0.1.0** — bar-close marking matches research, keeps the
  `bar_ts` two-timestamp contract clean, and avoids a tick-rate equity
  firehose. Named as a future follow-on.

## Consequences

- **Positive.** One per-bar equity path for the whole agent — research,
  paper, and the carry-forward to live-money execution. Paper mode
  becomes a true rehearsal of live: equity moves with real paper fills,
  the Live fills tape + positions panel populate, and the durable series
  (ADR-0052) persists/re-hydrates real values. The `drop(state_tx)` stub
  is gone. The divergence gate's intent is honestly satisfied, not
  rubber-stamped.

- **Negative / risk.** The unify edits the just-shipped, anchor-adjacent
  research loop. Mitigation: D4's named guards (paced-replay test +
  research tests + AC2 + anchor independence). The signature widens by
  two parameters (`equity_store`, `mode_label`); the paced-replay test
  call site MUST be updated in the same change (compile-enforced — the
  test will not build otherwise, which is the desired forcing function).

- **Scope held.** No change to the `LiveEquityStore` trait, the `013`
  migration, the A2 gate's location (still construction-time, now via the
  loop's `Option` param), captions, `PnlSnapshot`, or the UI widget. No
  new crate, no new dependency (feed/exec/sizer/store/publisher all
  reused). The single UI-surface change is a render-TEST assertion
  (`CURVE_Y_VAR_MIN`), not a widget. Live (real-money) execution stays
  out of scope (a separate feature with its own risk/compliance gates) —
  this ADR only makes the topology ready for it.

Resolves feature `paper-mode-equity-wiring` Q1–Q6 (Q1=a unify, Q2=a
retire paper reconciler mint role, Q3 bar-stream cadence / mark
`bar.close` / no intra-bar ticks, Q4 loop-direct persist gated by
`Some(store)`, Q5 configured registry `SmaCrossover`, Q6=a gate intent
satisfied by AC1 + AC6).
