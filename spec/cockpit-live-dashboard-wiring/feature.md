---
slug: cockpit-live-dashboard-wiring
status: proposed
owner: analyst
updated: 2026-06-09
version: 0.1.0
---

# Cockpit Live dashboard wiring — equity curve + KPI strip from the live agent

## Why

The cockpit's **Live screen** (`crates/ui/src/screens/live.rs`) is the
operator-facing window onto the running paper-trading agent. Its
"recent activity", positions, P&L, latency, and market-health panels are
**already live-wired** (they receive the agent's broadcast events via the
shared `Arc<EventBus>` that `cockpit_live` constructs). But its two
headline panels — the **equity curve** and the **KPI strip** — are
**hard-wired to `PanelState::Loading`** and never update:

```rust
// crates/ui/src/screens/live.rs:58
let equity_state: &PanelState<EquitySeries> = &PanelState::Loading;
...
// crates/ui/src/screens/live.rs:66
let kpi_state: &PanelState<BacktestMetrics> = &PanelState::Loading;
```

The module header (lines 8-13) literally annotates this: *"`PanelState::Loading`
placeholder (Design § A7 — no live feed yet)"*, and the LLM-spend tile
carries *"real wiring in Phase F"* (line 69). This is the Phase-F gap.

**Wiring = connect those two panels to the agent's live state so they
update as the agent trades the passive baseline.** This is the natural
follow-on to `cockpit-baseline-panel` (which surfaced the *historical*
BH result from a committed CSV): that feature proved the
`equity_curve` / `kpi_strip` widgets render a real `EquitySeries` +
`BacktestMetrics`; this feature points the **same widgets** at the
**live** agent feed instead of a file.

### Viability — established BEFORE scoping (the make-or-break finding)

The `cockpit-reports-viewer` feature stalled on a viability finding (no
data existed). This feature was scoped data-first to avoid repeating that.
**Verdict: GREEN for KPIs + the curve, with one honest structural nuance
the architect must resolve (the curve needs an equity *history* that does
not exist on the live side today — it must be accumulated).** Evidence:

1. **The agent emits a live equity *point* every bar, but NO equity
   *series*.** `agent::ReconcilerTask::after_bar_close`
   (`crates/agent/src/reconciler.rs:106-124`) builds a
   `trading_core::PnlSnapshot` on **every bar close** and publishes it on
   the `pnl` bus channel (`bus.publish_pnl`, line 121). That snapshot
   (`crates/core/src/views.rs:88-95`) carries exactly the live KPI inputs:
   - `total_equity: Money<Usdt>` — **the equity scalar** (`= cash +
     position_qty * last_mark`, `reconciler.rs:51-53`).
   - `cash`, `unrealized`, `realized` — P&L decomposition.
   - `as_of: Timestamp` — **the time coordinate for an equity point.**
   - `daily_return` — **currently hard-wired to `Decimal::ZERO`**
     (`reconciler.rs:117`, comment: *"requires a roll-over baseline the
     reconciler does not yet track … T912 future work"*). So `daily_return`
     is **not** a usable live KPI today (see § Data contract).
   So each `PnlSnapshot` is a **single `(Timestamp, equity)` point** plus
   P&L scalars. **There is NO accumulated equity history anywhere on the
   live/exec/agent side** — an exhaustive grep for `EquitySeries` /
   `equity_history` / `equity_curve` / `Vec<(Timestamp, …)>` across
   `crates/agent`, `crates/exec`, and the portfolio path returned **zero**
   live accumulators. The reconciler keeps only `last_equity: Decimal`
   (`reconciler.rs:130,152`) as a scalar for its imbalance heuristic, then
   overwrites it each tick — no buffer.

2. **The agent→UI plumbing the panels would use already exists and is
   already consumed.** `cockpit_live` runs the agent + iced in one process
   sharing one `Arc<EventBus>` (no IPC — `cockpit_live.rs:551-556`). The
   UI subscribes via `ui::live::subscription` (`crates/ui/src/live.rs:65`),
   which already batches a **`Channel::Pnl`** recipe
   (`live.rs:69` + `stream_pnl`, `live.rs:215-227`) that maps each
   `PnlSnapshot` → **`Message::PnlRefreshed(snap)`**. The UI **already
   handles** that message: `state.rs:1808` `model.pnl =
   PanelState::Ready(snap)`. So the equity scalar + P&L already flow to the
   UI every bar — the Live screen simply does not *render them as a curve /
   KPI strip* (the "recent activity" / positions panels consume the same
   feed family). **No new channel is required for the scalar feed.** The
   `TrailMirror` the brief mentioned (`cockpit_live.rs:389,503`) is a
   separate audit-tape bridge (tick-bus → ledger reflection), **not** the
   P&L path — the panels ride the existing `pnl` channel, not the mirror.
   Positions (for position-count / exposure KPIs) ride the already-wired
   `Channel::Positions` → `Message::PositionsRefreshed`
   (`state.rs:1811`, stored at `state.rs:811` `pub positions:
   PanelState<Vec<PositionView>>`).

3. **The honest data-availability reality (state it plainly — it's a live
   monitor, this is expected).** The dashboard has data **only when
   `cockpit_live` is running AND the agent has traded at least one bar.**
   On a fresh boot the panels are `Loading` until the first `PnlRefreshed`
   arrives; in **fixtures-mode `cockpit`** (no live agent, no `live`
   feature) there is no feed at all, so the panels stay `Loading`/`Empty`
   forever — and that is the correct, honest state for a live monitor with
   no live source. We do **not** oversell this as an always-populated
   screen; empty-until-running is the contract.

4. **What is wireable today vs what needs upstream work — the split that
   drives the size estimate:**
   - **Wireable NOW (pure-`ui`, from the existing feed):**
     - *Equity scalar, P&L (cash/unrealized/realized/total)* — already in
       `model.pnl` every bar.
     - *Position count + exposure* — `model.positions`
       (`Vec<PositionView>`; each `PositionView` carries `exposure_pct`,
       `core/src/views.rs:107`).
     - *Live drawdown* — derivable from the accumulated equity series via
       `EquitySeries::from_points` (which computes per-point + max
       drawdown, `equity_series.rs`), **once the UI accumulates the
       points** (D1).
   - **NOT wireable as a true live number today (needs derivation /
     upstream work):**
     - *Sharpe / total-return / CAGR* — **there is NO Sharpe/CAGR/return
       math in `crates/core`** (exhaustive grep — confirmed, same finding
       the Baseline panel hit; `cockpit-baseline-panel/feature.md` D1).
       That math lives only in `crates/backtest` + `crates/forecast`. A
       *live* Sharpe also needs a return-series + annualization-base
       methodology decision that is out of scope for a monitor panel. So
       the KPI strip's Sharpe/return cards must render the honest absent
       state (`—`) in v0.1.0 unless the architect chooses a defined
       live-derivation (D2). The Lab strip already renders Sharpe as `—`
       with the standing comment "engine not yet computing"
       (`kpi_strip.rs` precedent).
     - *A persistent equity history across restarts* — the live series is
       in-memory only; it starts empty each boot. Out of scope (D1 note).

**Net viability:** the **KPI strip is wireable now** for the
equity/P&L/position/exposure/drawdown cards; the **equity curve is
wireable now too, but only by accumulating the per-bar points in the UI**
(a small ring buffer), because no equity *history* exists upstream. The
honest options for where the series lives are D1 below. **No green light is
forced:** if the operator/architect wants the series to live agent-side
(exec) for persistence/reuse, that is a larger exec-side change — flagged,
not assumed.

This feature is a follow-on build-out candidate after `cockpit-baseline-panel`
(the ui-designer's #1, shipped) — it reuses that feature's render path and
the `cockpit_live` plumbing wholesale.

## Requirements

The Live screen's **equity curve** and **KPI strip** render the **running
agent's live state**, updating as new agent state arrives, reusing the
existing `equity_curve` / `kpi_strip` widgets verbatim (the Baseline-panel
pattern, `crates/ui/src/screens/baseline.rs`).

- **R1 — Live equity curve.** Replace the hard-wired
  `&PanelState::Loading` at `live.rs:58` with a `PanelState<EquitySeries>`
  backed by a **live-accumulated equity series**: each `PnlRefreshed`
  appends a `(snap.as_of, snap.total_equity)` point; the widget renders the
  growing curve via `equity_curve::view`. The drawdown band is **out of
  scope for the Live screen v0.1.0** (the Baseline screen pairs a band; the
  Live screen's existing layout does not — adding one is the architect's
  call, default: curve only, mirroring the current Live layout which has no
  band slot).
- **R2 — Live KPI strip.** Replace the hard-wired `&PanelState::Loading`
  at `live.rs:66` with a `PanelState<BacktestMetrics>` built from live
  state. The six fixed `kpi_strip` cards (Total return / CAGR / Sharpe /
  Max DD / Win rate / Trades) are populated **honestly**:
  - **Max DD** — from the accumulated `EquitySeries::max_drawdown_pct`
    (live, real). **Renderable now.**
  - **Total return** — `(total_equity − starting_equity) / starting_equity`,
    where `starting_equity` is the first accumulated point's equity (the
    session open). **Renderable now** as a *session* return (caption must
    say "session", not "annualized" — see R5).
  - **Trades** — live count of fills observed this session (the UI already
    sees `Channel::Fills`); OR `0` if the architect scopes fills out of the
    KPI (D2). **Renderable now if fills are counted.**
  - **Sharpe / CAGR** — render the honest absent state (`—`,
    `*_present = false`) in v0.1.0, because no live Sharpe/CAGR math exists
    in `core` and a methodology decision is out of scope (D2). **Do NOT
    fabricate.** (Matches the Lab strip's existing `—` Sharpe.)
  - **Win rate** — `—` (`win_rate_present = false`) unless the architect
    wires a live win/loss tally from closed positions (D2 — likely out of
    scope for v0.1.0).
- **R3 — Four `PanelState` states (both panels).** Honor `PanelState<T>`:
  - **Loading** — before the first `PnlRefreshed` (fresh boot, or
    fixtures-mode with no agent). Widgets render their built-in skeleton.
    This is the **default** state and the one the fixtures smoke hits.
  - **Ready** — streaming: curve grows, KPIs update each bar.
  - **Empty** — agent idle / channel closed with zero points accumulated
    → widgets render their empty body, never a blank.
  - **Error** — the `pnl` channel reports an error (`Message::PnlError`,
    already handled at `state.rs:1818`) → `PanelState::Error(msg)`; widgets
    render the muted error body. (The existing P&L panel already maps
    `PnlError`; the curve/strip should degrade consistently.)
- **R4 — Updates as agent state arrives (the "live" contract).** The
  panels update on each `PnlRefreshed` (≈ once per bar; cadence = the
  reconciler `interval_ms`). No new timer/tick is needed — the existing
  per-bar `pnl` cadence drives the refresh. Throttle/coalescing is the
  architect's call (D3) — the bar cadence is already low-frequency
  (minute bars), so a naive append-and-rerender is likely fine.
- **R5 — Honest live captions / labels.** Any added label or caption MUST
  NOT overclaim a session monitor as a characterized result:
  - The Total-return card is a **session-to-date** return, not an
    annualized / multi-year figure — if a caption or tooltip is added it
    says "session" (binding, mirrors the Baseline panel's no-overclaim
    rule R3/A3). MUST NOT imply the live session is the "baseline result".
  - Absent cards (Sharpe/CAGR/Win rate) render `—`, never a placeholder
    number.
- **R6 — Lumen consistency.** Theme tokens only (zero new tokens — the
  reused widgets are already theme-correct). All new copy in `strings.rs`
  (`LIVE_*` block — no hardcoded strings). Renders in both `--theme dark`
  and `--theme light` for free.
- **R7 — Smoke safety (fixtures `cockpit`).** The Live screen is the
  cockpit's **default route** (`cockpit_live.rs:583` sets
  `Screen::Live`; the fixtures `cockpit` smoke boots Home→Live and paints
  the first frame). With no live agent (fixtures has no `live` feature, no
  feed), the curve + strip MUST paint their **Loading** body, **no panic**,
  within the existing smoke window. The live-accumulation path must not
  require a feed to render the empty/Loading state.

### Out of scope (explicit)

- **Agent-side / exec-side persistence of the equity series** — the live
  series is in-memory only and starts empty each boot. A durable
  agent-side equity history (survives restart, reusable by other surfaces)
  is a larger exec change → out of scope (flagged in D1 as the
  durable-but-expensive option for the operator to weigh).
- **Live Sharpe / CAGR math** — needs a return-series + annualization
  methodology decision and new `core` math; not a monitor-panel concern in
  v0.1.0 (D2). Cards render `—`.
- **`daily_return` KPI** — the field exists on `PnlSnapshot` but is
  hard-wired to zero upstream (`reconciler.rs:117`, "T912 future work").
  Surfacing it would display a fake zero → out of scope until T912 wires
  the roll-over baseline.
- **Drawdown band on the Live screen** — the current Live layout has no
  band slot; adding one is a layout change deferred to the architect (R1).
- **LLM-spend tile real wiring** — the `live.rs:69` LLM-spend placeholder
  is a separate Phase-F item; not in this feature (this feature is the
  equity curve + KPI strip only).
- **New widgets / new theme tokens** — if either is needed it is a smell
  and must be challenged in review (R6 / AC).

## Design

_Architect-owned. Resolve D1–D3 below against the actual crate edges
(`ui` depends on `core` + `agent` for the live build; the `pnl` channel +
`Message::PnlRefreshed` already exist). No new crate edge is expected —
the panels consume state the UI already receives. Record decisions inline
+ in the Changelog. Likely no ADR (additive, within the existing
`cockpit_live` subscription precedent) — but confirm, since this touches
the agent→UI live contract._

## Data contract

The two panels subscribe to **the agent state the UI already receives** —
no new channel for the scalar feed. Specifics:

### What flows today (already wired — reuse)

| Source | Bus channel | UI message | UI store | Carries |
|--------|-------------|------------|----------|---------|
| `ReconcilerTask::after_bar_close` (`reconciler.rs:106`) | `pnl` (`bus.publish_pnl`, cap 256) | `Message::PnlRefreshed(PnlSnapshot)` (`live.rs:215` `stream_pnl`) | `model.pnl: PanelState<PnlSnapshot>` (`state.rs:812`) | `total_equity`, `cash`, `unrealized`, `realized`, `as_of`, (`daily_return`=0) |
| paper engine fills | `positions` (cap 256) | `Message::PositionsRefreshed(Vec<PositionView>)` (`live.rs:185`) | `model.positions` (`state.rs:811`) | per-position `exposure_pct`, `pnl_pct`, qty |
| paper engine fills | `fills` (cap 1024) | `Message::FillReceived(FillView)` (`live.rs:162`) | `model.tape` | individual fills (for a live trade count, if D2 wires it) |

### What must be NEW (UI-side accumulation — the curve)

- **A live equity series.** Since no equity *history* exists upstream
  (viability finding #1), the UI accumulates one: a `Vec<(Timestamp,
  Money<Usdt>)>` (or a bounded ring buffer) that **appends one point per
  `PnlRefreshed`** from `(snap.as_of, snap.total_equity)`, then builds an
  `EquitySeries::from_points(...)` for the widget. This is the **only new
  state** the feature introduces UI-side. Precedent: the UI already stores
  a `strategy_equity: HashMap<StrategyId, PanelState<EquitySeries>>`
  (`state.rs:1011`) for **backtest-fetched** curves — the same
  `EquitySeries` type, a different (live, accumulated) source.
- **The KPI `BacktestMetrics`** is **derived at view time** (or on each
  append) from the accumulated series + the latest `PnlSnapshot`:
  - `max_drawdown_pct` ← `series.max_drawdown_pct()` (free from
    `from_points`).
  - `total_return_pct` ← `(latest_equity − first_equity) / first_equity`
    (session return).
  - `trades` ← live fill count (D2) or `0`.
  - `sharpe`/`cagr`/`win_rate` ← `*_present = false` → `—` (no live math).

### How it updates

- On each `Message::PnlRefreshed(snap)` (already handled at
  `state.rs:1808`): the existing arm sets `model.pnl`; this feature
  **extends** that arm (or adds a sibling) to also append the point to the
  live equity buffer and recompute the derived `BacktestMetrics`. No new
  message, no new subscription, no new channel — the equity curve + KPI
  strip ride the **existing `pnl` cadence** (≈ per bar).
- The Live screen `view` (`screens/live.rs`) swaps the two hard-wired
  `&PanelState::Loading` references for the model-backed states.

### Update cadence

- Driven by the reconciler `interval_ms` (per-bar; minute bars in the
  paper config → ~1/min steady state). No high-frequency path. A throttle
  is almost certainly unnecessary (D3) but is the architect's call.

## Open decisions for the architect

- **D1 — Where does the live equity *series* live?** No equity history
  exists upstream (viability #1); it must be accumulated somewhere.
  - **(a) UI accumulates from the existing `pnl` feed** — append
    `(as_of, total_equity)` per `PnlRefreshed` into a UI-side buffer; build
    `EquitySeries::from_points`. **Pure-`ui`, no new crate edge, no exec
    change. (Recommended for v0.1.0)** — it's the proportionate,
    self-contained wiring this feature is scoped for, and it reuses the
    exact `EquitySeries` path the `strategy_equity` HashMap already uses.
    The series is session-scoped (empty each boot) — acceptable for a live
    monitor. *If-budget-tightens fallback is also (a): it is already the
    cheap path.*
  - **(b) Agent/exec accumulates a durable equity series** (ring buffer or
    ledger-backed) and exposes it (new bus message or a query). **More
    durable** — survives restart, reusable by other surfaces, and is where
    a "real" equity history arguably belongs. But it is a **larger
    exec-side change** (new state on the reconciler/engine + a new bus
    channel or query + tests) that exceeds this feature's UI-wiring scope.
    Recommend **deferring (b) to a future exec feature** if persistence is
    wanted; do not bundle it here. *(This is the durable option, but it is
    a different feature's worth of work — calling it out per the
    durable-over-quick rule, while noting the proportionate ship here is
    (a). If the operator wants persistence now, that re-scopes the feature
    to L and is exec-side, not UI-side.)*
  - **Decision needed:** confirm (a) for v0.1.0 (session-scoped UI
    accumulation), explicitly deferring durable agent-side history to a
    named follow-on.
- **D2 — KPI source: which cards are live in v0.1.0?** The Baseline panel
  found `core` has **no Sharpe/return math**. For the *live* strip:
  - Confirm **Max DD + Total return (session)** are wired from the
    accumulated series (real, recommended).
  - Decide **Trades**: wire a live session fill-count (the UI sees
    `Channel::Fills`) or render `0`/absent in v0.1.0.
  - Confirm **Sharpe / CAGR / Win rate** render `—` (`*_present = false`)
    — i.e. do NOT introduce live Sharpe math in this monitor feature. (If
    the architect *wants* a live Sharpe, that is a methodology decision +
    new `core` math = out of the UI-wiring scope; flag it as a follow-on.)
- **D3 — Update cadence / throttle.** The `pnl` feed is per-bar
  (low-frequency). Confirm a naive append-and-rerender is acceptable, or
  specify a coalescing throttle if the architect anticipates a faster feed
  (e.g. sub-second pnl in a future config). Default: no throttle.
- **D4 — Drawdown band on the Live screen?** The Baseline screen pairs the
  curve with a `drawdown_band`; the current Live layout does not. Decide
  whether to add a band slot to the Live layout (the data is free from the
  accumulated series) or keep curve-only for v0.1.0 (default: curve-only,
  preserving the current Live layout).
- **D5 — Where the derived `BacktestMetrics` is computed.** At view time
  (cheap, recomputed per frame) vs on-append (cached on the model). Mirror
  whatever the `strategy_equity` path does for consistency. Pure-`ui`
  either way; no `core` math (D2).

## Acceptance criteria

Proportionate + testable. The tester closes the loop against these. This
is a **live-monitor UI wiring** (no strategy overlay / sizing math) → the
CLAUDE.md baseline-equity-divergence e2e gate does **NOT** apply.

- **AC1 — Live panels render the agent feed.** With a live feed
  (integration test publishing `PnlSnapshot`s on the bus, mirroring the
  existing `live.rs` pnl test at line 1019 `bus.publish_pnl(...)`), the
  Live screen's equity curve grows point-by-point and the KPI strip's live
  cards (Max DD, Total-return-session, and Trades if D2 wires it) update as
  snapshots arrive. (Headless: drive `Message::PnlRefreshed` into the model
  and assert the equity buffer length + the curve/strip `PanelState`
  transition Loading→Ready.)
- **AC2 — Four panel states behave** (R3). A unit/headless test covers
  Loading (no points), Ready (≥1 point streaming), Empty (zero-point /
  idle), and Error (`PnlError` → `PanelState::Error`, no panic) for both
  the curve and the strip.
- **AC3 — Fixtures `cockpit` smoke passes** (R7) — first-frame render of
  the default Live route with **no live agent**: curve + strip paint their
  **Loading** body, no panic, within the existing smoke window
  (`headless_emulator_smoke.rs`). The live-accumulation path renders the
  empty/Loading state without a feed.
- **AC4 — Lumen-consistent** — `tests/consistency.rs` / `tests/contrast.rs`
  / `tests/layout_invariants.rs` stay green; **no hardcoded colors** (theme
  tokens only) and **no hardcoded strings** (any new copy via `strings.rs`
  `LIVE_*`). Renders in both themes.
- **AC5 — Honest live labels** (R5) — any added Total-return caption/tooltip
  conveys "session" scope and does not overclaim the live session as a
  characterized/annualized result; absent cards render `—`, never a fake
  number. Asserted by a string-content test if a caption is added.
- **AC6 — Panel-snapshot test** per the cockpit's panel-snapshot
  convention: a Live-screen snapshot in both themes covering at least the
  Loading state (default) and a Ready state (seeded equity buffer). The
  existing Live-screen snapshot baseline is updated to reflect the wired
  panels (was: two Loading placeholders).
- **AC7 — No new crate edge, no new widget, no new theme token.** Review
  confirms the wiring consumes state the UI already receives (`pnl` /
  `positions` channels, already subscribed); the two render widgets are
  reused verbatim. **Flag explicitly if any new dep is taken** — none is
  expected (the `ui` crate already depends on `core` for `PnlSnapshot` /
  `EquitySeries` and on `agent` for the live build).

### Lint convention (pre-existing tech-debt — do not fix-all)

`crates/ui` carries ~140 pre-existing pedantic clippy lints. New Live-wiring
code follows the crate's existing per-module allow-pattern; it introduces no
new warnings and does not attempt to clear the pre-existing 140 (out of
scope), matching the Baseline panel's convention.

## Size estimate (S/M/L) + UI-vs-agent-work split

**Estimate: S–M** (comparable to `cockpit-baseline-panel`, slightly
smaller on the data side because **no loader / no CSV / no embedded const**
— the feed already exists and is already consumed).

**The decisive scoping fact: ~100% UI wiring, 0% agent/exec work — IF D1=(a).**

- **UI work (the whole feature under D1=(a)):**
  - Add a live equity buffer + derived-metrics field(s) to the cockpit
    model (`state.rs`) — small (one `Vec`/buffer + the derive).
  - Extend the existing `Message::PnlRefreshed` arm (`state.rs:1808`) to
    append the point + recompute `BacktestMetrics` — small.
  - Swap the two hard-wired `&PanelState::Loading` refs in
    `screens/live.rs:58,66` for the model-backed states — trivial.
  - Tests: headless feed-drive (AC1/AC2), smoke Loading-path (AC3),
    panel snapshots both themes (AC6), string test if a caption is added
    (AC5).
  - **No new module, no loader, no new crate edge, no new widget/token.**
    Smaller than Baseline's `baseline/loader.rs` + `screens/baseline.rs` +
    sidebar IA touchpoints (the Live screen already exists and is
    default-routed — no new screen, no sidebar entry, no IA change).
- **Agent/exec work: NONE under D1=(a).** The `pnl` feed, the reconciler
  publish, the bus channel, the subscription, and the UI message are **all
  already shipped and live** (T903c / live-cockpit-unified). This feature
  consumes them.
- **IF D1=(b) is chosen instead (durable agent-side equity history):**
  re-scope to **L** and split ≈ 60% exec / 40% UI — new reconciler/engine
  state, a new bus channel or query, exec tests, *then* the UI wiring. That
  is a different (exec-side) feature; recommend deferring it.

**Bottom line for the operator:** under the recommended D1=(a), this is a
**small, self-contained UI-wiring feature** — the live data already
arrives at the UI every bar; we're rendering it instead of dropping it on
the floor. The only structural caveat is honest: the equity *curve* is
**session-scoped** (resets each `cockpit_live` boot) because no durable
upstream history exists — which is the correct behavior for a live monitor,
and exactly what the operator expects when they "run the paper agent via
`cockpit_live`".

## Backtest Scenarios

N/A — this is a read-only **live-monitor** UI wiring feature. It runs no
new strategy, produces no new backtest, and reads no anchored file. It
renders the running agent's live state via the existing `pnl` / `positions`
broadcast feed. Per CLAUDE.md, the baseline-equity-divergence e2e gate
applies to **strategy overlays / sizing modifiers** — this is neither (no
overlay, no sizing math, no decision variable).

## Implementation

_developer ‖ ui-designer fill this._

## UI

_ui-designer fills this._

## Verification

_tester links to reports here._

## Changelog

- 2026-06-09 (analyst): initial brief. **Viability established data-first
  (GREEN with one structural nuance).** Confirmed via file:line evidence:
  the agent emits a live equity *point* every bar
  (`PnlSnapshot.total_equity` + `as_of`, `reconciler.rs:106-124`) on the
  already-wired `pnl` channel the UI **already consumes**
  (`live.rs:215 stream_pnl` → `Message::PnlRefreshed` → `state.rs:1808`
  `model.pnl`), so the scalar feed + plumbing exist with **no new channel**.
  But **no equity *history* exists anywhere on the live/exec/agent side**
  (exhaustive grep empty) — so the *curve* must accumulate the per-bar
  points UI-side (D1=(a), recommended), making it **session-scoped**.
  Sharpe/CAGR/return have **no live math in `core`** (same finding as the
  Baseline panel) → those KPI cards render `—`; Max DD + session Total-return
  + Trades are wireable now. `TrailMirror` (`cockpit_live.rs:389`) is the
  audit-tape bridge, NOT the P&L path — clarified. Teed up D1 (series
  location: UI-accumulate vs durable agent-side), D2 (which KPI cards are
  live), D3 (cadence/throttle), D4 (drawdown band on Live?), D5 (derive
  site). Size **S–M, ~100% UI / 0% agent under D1=(a)**; **L, ~60% exec
  under D1=(b)** — recommend (a) and defer durable history to a future exec
  feature. Opened REQ-COCKPIT-LIVE-DASHBOARD-001 (proposed). HANDOFF →
  architect.
