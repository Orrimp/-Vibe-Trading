---
slug: cockpit-live-dashboard-wiring
status: presenter-done
owner: ui-designer
updated: 2026-06-12
version: 0.1.2
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

_Architect-owned (2026-06-09). Decisions D1–D5 resolved below against the
verified crate edges and code seams. **No ADR** — this is additive UI
wiring fully within the established `cockpit_live` subscription +
`PanelState` precedent (`cockpit-baseline-panel`); it adds no new crate
edge, no new bus channel, no new `core` math, and mutates no anchor. The
decisions live here, not in an ADR (proportionate to an S–M, ~100%-UI
feature)._

### Crate-edge reality (verified — AC7 satisfied by construction)

- `crates/ui/Cargo.toml` `[dependencies]` already lists
  `trading_core = { path = "../core" }` (for `PnlSnapshot`, `EquitySeries`,
  `BacktestMetrics`, `Money<Usdt>`, `Timestamp`) **and** `agent` (pulled in
  the `live` feature for the `cockpit_live` binary). **No new crate edge is
  introduced.** The two render widgets (`widgets::equity_curve`,
  `widgets::kpi_strip`) and the `pnl`/`positions` channels are all already
  subscribed and consumed (`live.rs:215` `stream_pnl` →
  `Message::PnlRefreshed` → `state.rs:1808`). This feature renders state the
  UI already receives every bar.
- New code follows the crate's existing per-module `#![allow(...)]`
  pattern (`screens/live.rs:23`), introduces **zero new warnings**, and does
  **not** attempt to clear the ~140 pre-existing `crates/ui` pedantic lints
  (out of scope — matches the Baseline panel convention).

### D1 — Equity-series location → **(a) UI-accumulate (CONFIRMED for v0.1.0)**

The UI accumulates a **session-scoped** equity series from the existing
per-bar `pnl` feed. Each `Message::PnlRefreshed(snap)` appends one
`(snap.as_of, snap.total_equity)` point to a UI-side buffer on the cockpit
model; the buffer is converted to an `EquitySeries` via
`EquitySeries::from_points(...)` for the widget. This is pure-`ui`, adds no
crate edge, no exec change, no new bus message — and reuses the exact
`EquitySeries` type the existing `strategy_equity: HashMap<StrategyId,
PanelState<EquitySeries>>` (`state.rs:1011`) already stores (a different,
backtest-fetched source of the same shape).

**Rejected for v0.1.0 (deferred): (b) durable agent/exec-side equity
history.** A reconciler/engine-side ring buffer (or ledger-backed series)
exposed via a new bus message/query would survive restart and be reusable
by other surfaces — but it is a **larger exec-side change** (~60% exec /
40% UI, re-scopes the feature to **L**) outside this UI-wiring feature's
scope. **Deferred to a named follow-on: `live-equity-history-durable`
(exec-side, unscheduled).** The session-scoped buffer here is the correct,
honest behavior for a live monitor: it starts empty each `cockpit_live`
boot and grows as the agent trades.

### D2 — KPI-source mapping (which cards are live vs `—`)

The KPI strip's six fixed `BacktestMetrics` cards are populated **honestly**.
The strip is built from the **accumulated `EquitySeries`** (not the raw
buffer) plus the present-flags:

| Card | Field | v0.1.0 source | Live? |
|------|-------|---------------|-------|
| **Total return** | `total_return_pct` | `(latest_equity − first_equity) / first_equity` — the **session** return (first accumulated point = session open) | **LIVE** |
| **Max DD** | `max_drawdown_pct` | `series.max_drawdown_pct` — free from `from_points`'s O(N) walk | **LIVE** |
| **Trades** | `trades` | **`0`** (see Trades finding below) | absent (`0`) |
| **Sharpe** | `sharpe` / `sharpe_present=false` | `—` — no live Sharpe math in `core` (verified grep); annualization methodology is out of scope for a monitor | `—` |
| **CAGR** | `cagr_pct` / `cagr_present=false` | `—` — no CAGR math in `core`; a single session is not a multi-year base | `—` |
| **Win rate** | `win_rate_pct` / `win_rate_present=false` | `—` — no closed-position win/loss tally wired | `—` |

So `cagr_present = sharpe_present = win_rate_present = false`;
`total_return_pct` and `max_drawdown_pct` carry real session numbers;
`trades = 0`. **Do NOT fabricate Sharpe/CAGR/Win-rate** — they render the
honest `—` (matching the Lab strip's existing `—` Sharpe and the Baseline
panel's same `core`-has-no-Sharpe finding).

**Trades source — FINDING (analyst evidence refined).** The analyst's R2/D2
floated "live count of fills observed this session (the UI already sees
`Channel::Fills`)". **Verified: the UI maintains NO fill *counter*.**
`Message::FillReceived` (`state.rs:1782`) only `push_front`s into the
`tape: PanelState<VecDeque<FillView>>`, which is **capped at `TAPE_MAX_ROWS`
and evicts oldest** — so `tape.len()` is a bounded window, **not** a session
total. There is no `fill_count` field anywhere in the model (verified grep).
**Decision: Trades renders `0` in v0.1.0** (absent). Wiring a true
session fill-counter (a `u64` on the model incremented in the
`FillReceived` arm, reset on boot) is a small but distinct add — **deferred
to the same follow-on** rather than bundled, keeping this feature the
clean two-panel swap. *(If the operator wants it now it is ~10 lines + one
test; flagged, not assumed.)*

> **Critical edge — the `is_all_absent` bootstrap trap (must-honor).**
> `kpi_strip::is_all_absent` (`widgets/kpi_strip.rs:79`) renders the
> *unavailable* (six-dash) strip when **`total_return_pct == 0 AND
> max_drawdown_pct == 0 AND trades == 0`** and the three present-flags are
> false. At the **very first** `PnlRefreshed` (a single accumulated point):
> session-return `= (e − e)/e = 0`, max-DD `= 0` (one point, no drawdown),
> trades `= 0` → the live `BacktestMetrics` is **byte-identical to the
> all-absent sentinel**, so the strip would wrongly show all dashes instead
> of "Total return 0.00% / Max DD 0.00%". **Resolution: the KPI strip stays
> `PanelState::Loading` until the buffer holds ≥ 2 points** (i.e. at least
> one real session delta exists); on the ≥2-point transition it becomes
> `Ready(metrics)`. The **equity curve** has no such constraint — it renders
> `Ready` from ≥ 1 point (a single dot is a valid 1-point curve). This
> 1-point-curve / 2-point-strip split is intentional and is asserted by
> AC2.

### D3 — Update cadence / throttle → **naive append-and-rerender (no throttle)**

The `pnl` feed is per-bar (reconciler `interval_ms`; minute bars in the
paper config → ~1/min steady state). This is low-frequency; a naive
"append the point + rebuild the derived state + rerender" on each
`PnlRefreshed` is correct and cheap. **No throttle / coalescing in
v0.1.0.** The buffer bound (D-buffer) caps the per-append `from_points`
cost at O(cap). *Forward note:* if a future config emits sub-second `pnl`,
revisit with a coalescing throttle (e.g. rebuild at most every N ms) — but
that is not this feature.

### D4 — Drawdown band on the Live screen → **curve-only (no band)**

The current Live layout (`screens/live.rs`) has **no band slot** — it
stacks `health_strip → equity → kpi_row → bottom_row`. The Baseline screen
pairs the curve with a `drawdown_band`, but adding a band row to Live is a
**layout change** beyond this wiring feature. **Decision: curve-only for
v0.1.0**, preserving the existing Live layout. (The drawdown data is free
from the accumulated series, so a band is a cheap future add if the
operator wants one — deferred, not assumed.)

### D5 — Where `BacktestMetrics` is derived → **on-append (cached on the model)**

Derive the `BacktestMetrics` (and the `EquitySeries`) **on each append**,
in the `PnlRefreshed` arm, and store the result as a `PanelState` on the
model — the view reads the cached state (the `strategy_equity` /
`baseline_screen_state.active_metrics()` pattern: the screen `view` reads a
model-stored `PanelState`, it does not compute). Rationale: (1) consistent
with how every other panel on the Live screen reads pre-derived model
state at view time; (2) the iced `view` runs every frame (incl. on
unrelated messages / hover), so view-time derivation would rebuild the
series on frames where nothing changed — on-append rebuilds **only when a
new bar arrives** (≈ 1/min), which is strictly cheaper. The widget
borrows the model-stored `PanelState<EquitySeries>` /
`PanelState<BacktestMetrics>` directly (the lifetime pattern
`kpi_strip::view(&self.model...)` already uses).

### The equity buffer (the one piece of real design)

**Location.** Two sibling fields on the cockpit model (`state.rs`,
alongside `pnl`, `positions`, `strategy_equity`):

```text
/// cockpit-live-dashboard-wiring — session-scoped live equity buffer.
/// Raw (Timestamp, Money<Usdt>) points appended one-per-PnlRefreshed,
/// bounded ring (LIVE_EQUITY_BUFFER_CAP). Empty on each cockpit_live boot
/// (session-scoped — correct for a live monitor; durable history deferred,
/// D1 follow-on). NOT serialized.
live_equity_buffer: VecDeque<(Timestamp, Money<Usdt>)>,

/// Derived-on-append (D5) render state for the Live equity curve.
/// Loading until the first point; Ready(series) from ≥1 point;
/// Error(msg) on PnlError; Empty only if the channel closes with 0 points.
live_equity_curve: PanelState<EquitySeries>,

/// Derived-on-append (D5) render state for the Live KPI strip.
/// Loading until ≥2 points (is_all_absent trap, D2); Ready(metrics) after;
/// Error(msg) on PnlError; Empty mirrors the curve.
live_kpi: PanelState<BacktestMetrics>,
```

**Append (in the `Message::PnlRefreshed(snap)` arm — extend the existing
arm at `state.rs:1808`, do not add a message).** The existing line
`model.pnl = PanelState::Ready(snap)` stays. Then:

1. **Monotone guard (must-honor — `from_points` rejects non-monotone
   timestamps, `equity_series.rs:83`).** Push `(snap.as_of,
   snap.total_equity)` **only if** the buffer is empty OR `snap.as_of >=
   buffer.back().ts`. If `snap.as_of < back.ts` (a late/out-of-order
   snapshot), **drop the point** (do not append, do not error) — a dropped
   late point is invisible on a monitor and strictly preferable to an
   `EquitySeries` build error. (Equal timestamps are allowed by
   `from_points`'s `<` check, so a duplicate-`as_of` bar appends fine.)
2. **Bound (ring).** After push, while `buffer.len() >
   LIVE_EQUITY_BUFFER_CAP`, `pop_front()` (drop oldest). **Cap =
   `LIVE_EQUITY_BUFFER_CAP = 2_880`** — a new `theme::layout` const.
   Rationale: 2_880 minute-bars = **48 h** of continuous 1-min session at
   full resolution before any eviction; a session longer than that quietly
   slides a 48 h window. The chart consumer already `downsample`s to
   `SPARKLINE_POINT_CAP = 120` for rendering, so the buffer cap governs
   *retention/memory* (2_880 × ~48 B ≈ 140 KB worst-case — negligible),
   not pixels. **This is a bounded ring by design — not unbounded.** (A
   2_880-cap + 120-render-downsample mirrors the established
   `downsample(SPARKLINE_POINT_CAP)` path the `strategy_equity` curve uses
   at `cockpit_live.rs:1464`.)
3. **Rebuild derived state (D5).**
   - `live_equity_curve`: `EquitySeries::from_points(buffer.iter().cloned()
     .collect())` → on `Ok(series)` set `Ready(series)`; on `Err` (only
     possible if the buffer is unexpectedly empty — guarded) leave/clear to
     `Loading`. Build from ≥1 point.
   - `live_kpi`: if `buffer.len() < 2` → `Loading` (bootstrap trap, D2);
     else build `BacktestMetrics { total_return_pct: session_return,
     max_drawdown_pct: series.max_drawdown_pct, trades: 0,
     cagr_present:false, sharpe_present:false, win_rate_present:false,
     ..zeros }` and set `Ready(metrics)`. `session_return =
     (latest.amount() − first.amount()) / first.amount()` using
     `rust_decimal::Decimal` (guard `first != 0`; the agent's starting
     equity is non-zero, but divide-guard anyway → `Decimal::ZERO` if
     first is zero).

**Session-scoped reset.** The buffer is **not serialized** and is
initialized empty in `Cockpit::new()` (`state.rs:1160`) and the `Debug`/
clone constructors — so it starts empty every `cockpit_live` boot. There is
no explicit "reset" event; a fresh process = a fresh empty buffer = the
correct live-monitor session-open state.

### The four `PanelState` transitions (R3 — both panels)

| State | Fires when | Curve | Strip |
|-------|-----------|-------|-------|
| **Loading** | 0 points (fresh boot, pre-first-bar) **or** fixtures-mode `cockpit` (no `live` feature → no agent → no feed → buffer never fills). Strip also Loading at **1 point** (bootstrap trap, D2). | skeleton (`VIEWER_NO_EQUITY_DATA`) | unavailable strip |
| **Ready** | Curve: ≥ 1 point. Strip: ≥ 2 points. Streaming — grows per bar. | growing curve | live cards (Total-return/Max-DD live; Sharpe/CAGR/Win `—`; Trades 0) |
| **Empty** | The `pnl` channel **closes** (`RecvError::Closed` → `PnlError`) with **0** points accumulated — i.e. agent went away before any bar. *(Distinct from Loading: Loading = "no feed yet / waiting"; Empty = "feed ended, nothing seen".)* In practice the closed-channel path routes through `PnlError`; treat a closed-with-zero-points as `Empty` if the impl distinguishes, else `Error` is acceptable (both render a non-blank body). | empty body | empty body |
| **Error** | `Message::PnlError(e)` (`state.rs:1818`) — extend that arm to also set `live_equity_curve = Error(e.clone())` and `live_kpi = Error(e)`. No panic. | muted error body | muted error body |

**Fixtures-mode determinism (R7 / AC3 — smoke-safe).** `cockpit --features
fixtures` has no `live` feature, no agent, no `pnl` feed → `PnlRefreshed`
**never fires** → the buffer stays empty → both panels stay **`Loading`**
and render their built-in skeleton bodies. The Live screen is the default
route (`cockpit_live.rs:583`; smoke boots Home→Live and paints frame 1),
so the smoke (`headless_emulator_smoke.rs`) hits exactly the **Loading**
state with no feed — deterministic, no panic, no feed required to render.

### `screens/live.rs` wiring (the two-line swap)

Replace the two hard-wired refs:

- `live.rs:58` `let equity_state: &PanelState<EquitySeries> =
  &PanelState::Loading;` → `let equity_state = &model.live_equity_curve;`
- `live.rs:66` `let kpi_state: &PanelState<BacktestMetrics> =
  &PanelState::Loading;` → `let kpi_state = &model.live_kpi;`

Everything downstream (the `equity_curve::view(equity_state, mode).map(...)`
and `kpi_strip::view(kpi_state, mode).map(...)` bridges) is **unchanged** —
the widgets already accept `&PanelState<T>` and the never-fired
`.map(|_| Message::ChartMarkerHoverEnded)` adapter still applies (the live
curve/strip emit no interactions, same as Baseline). Update the module
header (lines 8-13, 55-57, 64-66) to drop the "no live feed yet / Phase F"
annotations and reference this feature. The LLM-spend tile (line 69)
remains a separate Phase-F placeholder — **untouched**.

### Strings (R6)

Any new copy lives in `crate::strings` under the `LIVE_*` block (the
existing `LIVE_HEADLINE`/`LIVE_SYSTEM_HEALTH_LABEL`/`LIVE_LLM_*` family) and
is registered in the `strings.rs` test-table. **No new theme token** — the
reused widgets are already token-correct, and the strip/curve render in
both `--theme dark` and `--theme light` for free. If a "session" caption is
added under the Total-return card (R5/AC5 — optional), it MUST convey
session-to-date scope (e.g. a `LIVE_SESSION_RETURN_CAPTION` like
`"Session to date"`) and MUST NOT imply an annualized/characterized result;
absent cards render `—`, never a fabricated number. **Recommendation:** add
the short "Session to date" caption so the Total-return card is
unambiguous (cheap honesty; satisfies R5/AC5 affirmatively).

### Summary table — every constant/decision introduced

| New thing | Where | Value / rule |
|-----------|-------|--------------|
| `LIVE_EQUITY_BUFFER_CAP` | `theme::layout` | `2_880` (48 h of 1-min bars; ring) |
| `live_equity_buffer` | `state::Cockpit` | `VecDeque<(Timestamp, Money<Usdt>)>`, empty on boot, not serialized |
| `live_equity_curve` | `state::Cockpit` | `PanelState<EquitySeries>`, derived on-append |
| `live_kpi` | `state::Cockpit` | `PanelState<BacktestMetrics>`, derived on-append, Loading until ≥2 pts |
| `LIVE_SESSION_RETURN_CAPTION` (optional) | `strings` | `"Session to date"` — R5/AC5 honest caption |
| (no new) crate edge / bus channel / message / widget / theme token / `core` math | — | AC7 |

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

_ui-designer (2026-06-09). Implemented solo per tasks T1–T9 (~100% UI,
D1=(a)). Files-only; orchestrator commits. All tests green; both default and
`--features live` builds clean; anchors 119/119._

### Wireframe (the Live screen, wired)

```text
┌─ Live ─────────────────────────────────────────────────────────────────────┐
│ System health   [Feed latency OK 120ms]            Server — UTC  binance …  │
│                                                                              │
│ ┌─ equity curve (model.live_equity_curve) ─────────────────────────────────┐│
│ │  Loading → "No equity data"  ·  Ready → growing live line (≥1 pt)         ││
│ │  Empty → empty body          ·  Error → muted error body                 ││
│ └──────────────────────────────────────────────────────────────────────────┘│
│ ┌─ KPI strip (model.live_kpi) ───────────────────────────────┐ ┌ LLM spend ┐│
│ │ Total return │ CAGR │ Sharpe │ Max DD │ Win rate │ Trades   │ │    —      ││
│ │   +0.10%     │  —   │   —    │ 0.00%  │    —     │   0      │ │ (Phase F) ││
│ └────────────────────────────────────────────────────────────┘ └───────────┘│
│ Session to date                       ← honest scope caption (R5/AC5)         │
│ ┌─ Open positions ──────────────┐ ┌─ Agent activity ───────────────────────┐ │
│ │ (already-live)                │ │ (already-live)                          │ │
│ └───────────────────────────────┘ └─────────────────────────────────────────┘│
└──────────────────────────────────────────────────────────────────────────────┘
```

The **only** two panels this feature touches are the equity curve and the KPI
strip (previously hard-wired `&PanelState::Loading`); everything else on the
Live screen was already live-wired.

### Screens / panels / widgets

- **No new screen, no new widget, no new panel.** The two stubbed panels on
  the existing **Live** screen (`crates/ui/src/screens/live.rs`) are pointed
  at model-backed state. The render widgets (`widgets::equity_curve`,
  `widgets::kpi_strip`) are reused **verbatim** — same `&PanelState<T>`
  signature, same never-fired `.map(|_| Message::ChartMarkerHoverEnded)`
  bridge as `screens/baseline.rs`.
- **New model state** (`crates/ui/src/state.rs`, siblings of `strategy_equity`):
  `live_equity_buffer: VecDeque<(Timestamp, Money<Usdt>)>` (session-scoped
  ring), `live_equity_curve: PanelState<EquitySeries>` and
  `live_kpi: PanelState<BacktestMetrics>` (both derived on-append, D5).
- **One UI-element addition:** an honest "Session to date" scope caption under
  the KPI row (R5/AC5) — a static label clarifying the Total-return card is a
  session figure, never an annualized/characterized result.

### New strings (`ui::strings`)

- `LIVE_SESSION_RETURN_CAPTION = "Session to date"` (the `LIVE_*` block) — the
  only new copy. No other user-visible string introduced; no hardcoded
  literals in the new code.

### New theme tokens

- `theme::layout::LIVE_EQUITY_BUFFER_CAP = 2_880` — **not a visual token**; a
  retention/memory ring-cap constant (48 h of 1-min bars). **Zero new
  color/spacing/radius/type tokens** — the reused widgets are already
  token-correct and render in both `--theme dark` and `--theme light` for
  free (verified by the dark+light Ready snapshots + the regenerated dark
  visual PNGs).

### Accessibility notes

- **Keyboard / focus:** no new interactive element. The session caption is
  static text; the two panels emit no interactions (the `.map` bridge is
  never fired). Focus order is unchanged from the shipped Live screen.
- **Contrast:** all colors flow from `theme` tokens already verified ≥ 4.5:1
  by `tests/contrast.rs` (green). The KPI cards use the existing
  sentiment/`FG_3`-dash treatment; the absent cards render `—` in `FG_3`.
- **Color is never the only signal:** P&L sentiment on the Total-return card
  is paired with the sign (`+`/`−`) and the value text; absent cards show the
  `—` glyph, not merely a muted color. Max-DD keeps the established minus
  prefix.
- **No blank states:** Loading → skeleton/`VIEWER_NO_EQUITY_DATA` + the
  unavailable strip; Empty → empty body; Error → muted error body. Every
  state has explicit copy (R3).

### Seams flagged (architect design vs. code)

The design matched the code well; three small clarifications worth recording:

1. **Constructor sites — exactly two struct literals, not "≥2 plus `new()`".**
   The brief said grep `baseline_screen_state:` (≥2 sites). There are two
   **struct-literal** constructors — `impl Default for Cockpit` and
   `Cockpit::ready(...)` — and the three fields are initialized in both.
   `Cockpit::new()` and `Cockpit::boot()` do **not** list fields; they
   delegate via `..Self::default()`, so they inherit the empty/Loading state.
   The manual `Debug` impl also lists fields (added `live_equity_buffer_len`
   + both panels there). No site was missed.
2. **`PnlError` payload is `SmolStr` (Clone).** The arm sets the curve +
   strip to `Error(e.clone())` then `pnl = Error(e)` — `e.clone()` is cheap
   (`SmolStr`), exactly as the design assumed. Empty-on-channel-close is not
   separately distinguishable in the UI `update` (the closed `pnl` channel
   routes through `PnlError`), so it maps to `Error` per the design's
   accepted both-render-a-non-blank-body fallback.
3. **The `live_screen` snapshot helper is a text summary, not a render.**
   The existing `live_screen_summary` builds a `String` (it never called
   `screens::live::view`), and it **hard-coded** the curve/strip as Loading.
   It is now `(c, mode)` and reads the model-backed states. The
   regeneration of `live_snapshot__steady_state` is intentional and its diff
   is **only** the two added lines (`theme:` + `session_caption:`) — the
   curve/strip placeholder copy is byte-identical to the prior Loading
   baseline, confirming the Loading default render is unchanged. The
   `consistency.rs` "no inline strings" gate scans only `src/widgets/*.rs`,
   so there is no separate `strings.rs` registration table — the snapshot
   helper is the reference site for the new caption.

## UI — v0.1.1 follow-up (3 operator-session display fixes)

_ui-designer (2026-06-10). Files-only; orchestrator commits. 437 lib tests +
panel/consistency/contrast/layout/visual green; new-code clippy/fmt clean;
anchors 119/119._

### New widgets / panels / columns

- **No new screen or widget.** One new **column** on the existing
  `widgets::agent_feed` tape: **Notional** (`qty × price`, USDT), inserted
  between Qty and Fee. Derived in-widget from existing `FillView` fields — no
  `core`/`agent` change. Resolves the "$4 buys" misread (the rightmost
  USDT-suffixed number was the fee; the operator read it as the trade size).
- **One shared helper added** to `widgets::chart`:
  `format_time_axis_label(local_ts, span_seconds)` — span-adaptive granularity
  (`HH:MM` < 6 h / `MMM DD HH:MM` < 14 d / `MMM DD` < 18 mo / `MMM 'YY` ≥ 18
  mo). Consumed by `equity_curve` + `drawdown_band` (shared, not forked).
- **`time_axis_tick_count` bug fix** (same shared function): interval count now
  capped at the width budget (`clamp(width/96, 4, 12)`) so the label count is
  bounded regardless of series length — fixes the Live (2880-pt) + Baseline
  (367-pt) X-axis smear. No-op for the ≤ 60-bar price chart.

### New strings (`ui::strings`)

- `TAPE_COL_NOTIONAL = "Notional"` — the agent_feed Notional column header.
- `MONTH_ABBREVS: [&str; 12]` (`"Jan".."Dec"`) + `month_abbrev(u8)` helper —
  for the adaptive time-axis labels (no inline month literals in widgets).
  Registered in `strings::all()` as `MONTH_JAN..MONTH_DEC` + `TAPE_COL_NOTIONAL`.

### New theme tokens

- **Zero.** (As required — most additions would be a smell.) The fix reuses
  existing `text::MICRO`, `space::*`, `color::FG_3`/`BORDER_1` tokens.

### Unit-semantics correction (no token, behavioral)

- `state.rs` live-KPI wiring now scales the session return and Max-DD
  **fraction → percent** (×100) to match `BacktestMetrics`'s percent
  semantics (the baseline const + `format_pct_sentiment`/`format_pct_max_dd`
  all expect percent). A +1.5 % session now renders "1.50%", not "0.01%".

### Accessibility notes

- **Notional column:** right-aligned monospaced digits with thousands
  separators + explicit "USDT" unit (matches the Fee column treatment). The
  header is keyboard/screen-reader legible copy from `strings`. No new
  interactive element; focus order unchanged.
- **Axis labels:** `text::MICRO` on `FG_3` (existing, contrast-verified);
  fewer, non-overlapping labels improve legibility. Month/time text is never
  color-only — it is literal copy.
- **Both themes:** all changes flow through existing tokens → render correctly
  in `--theme dark` and `--theme light` (verified by the regenerated dark +
  light `live_snapshot__ready_*` panel snaps + the visual triple).

### Baselines regenerated (intentional)

- Panel snaps (4): `live_snapshot__ready_dark`, `live_snapshot__ready_light`
  (Total-return "0.10%"→"10.00%" — the unit fix); `agent_feed_ready_three_fills`,
  `agent_feed_paused` (new `notional=…  fee=…` columns).
- Visual baselines (3): `live__recent_activity_with_chevron__{floor,typical,
  operator}` (Notional column now rendered). All `charts_screen_*` + 49 other
  visual baselines **unchanged** (price-chart tick-count cap is a no-op).

## Implementation v0.1.2 (developer, 2026-06-11)

### ISSUE 1 — PnlSnapshot.as_of used wallclock instead of bar data time

Root cause: `crates/agent/src/runtime.rs` `spawn_research_trading_loop` (line 1094 in v0.1.1)
stamped `as_of: Timestamp::now()` on every `PnlSnapshot`. During 30 ms/bar fast replay all ~500k
bars stamp the current wallclock minute → the equity-curve x-axis shows the same minute for
every point → span-adaptive axis formatter collapses every label to the current HH:MM.

Fix:
- `crates/agent/src/runtime.rs:1094`: `as_of: bar.close_ts` (bar data timestamp, not wallclock).
- `crates/agent/src/reconciler.rs:115`: updated `after_bar_close(&self, bar_ts: Timestamp)`
  signature so the reconciler path also takes a bar timestamp rather than calling `Timestamp::now()`.
- `reconciler.rs:268`: test `t903c_after_bar_close_publishes_pnl` now passes a fixed historical
  timestamp (2023-01-15 12:30:00 UTC) and asserts `snap.as_of == bar_ts`.
- Removed unused `Timestamp` from the `spawn_research_trading_loop` local import.
- Correct in all modes: in live/paper `bar.close_ts ≈ now()` anyway; in replay it's the
  historical 2023-24 bar time.
- Wallclock-consumer audit: `as_of` only flows to `state.rs:1946` `push_live_equity_point`
  (equity-curve x-coord). The status-bar server-time/latency reads `Tick::local_recv_ts`
  (separate path, unaffected).

### ISSUE 2 — "$4 buys/sells" — CONFIRMED display/perception issue

Real fill data captured from headless agent run (`RUST_LOG=agent=info`, BTCUSDT 2023 replay):

| Fill | Side | Price (USDT) | Qty (BTC) | Notional (USDT) | Fee (USDT) | Running equity (USDT) |
|------|------|-------------|-----------|----------------|-----------|----------------------|
| 1 | Buy | 16,680.94 | 0.5996 | 10,002.00 | 4.0008 | 99,995.99 |
| 2 | Sell | 16,698.63 | 0.5996 | 10,012.61 | 4.0050 | 100,002.60 |
| 3 | Buy | 16,881.56 | 0.5925 | 10,002.26 | 4.0009 | 99,998.60 |

Verdict: **Notional ~$10k (correct); Fee ~$4 (correct, 4bps × $10k).** The Notional column
in `agent_feed.rs:118` (`fill.qty.get() * fill.price.get()`) is computing correctly.
The `$4` the operator sees is the Fee column. No sizing bug.

The agent log now emits `notional_usdt` + `fee_usdt` per fill at `debug!` level (always) and
at `info!` level (first 5 + every 100). This makes the fill detail surfaced when running with
`RUST_LOG=agent=debug` or reviewing INFO logs.

For the ui-designer: Notional is correct. If the operator is still reading $4 as the trade size
after refreshing the cockpit binary, they may be looking at the Fee column.
The column order `Price | Qty | Notional | Fee` is already optimal.

### Test results

- `cargo test -p agent`: 69/69 PASS (including `t903c_after_bar_close_publishes_pnl`)
- `cargo test -p data`: 79+14+12+11/... PASS
- `cargo clippy -p agent -- -D warnings`: 0 warnings, exit 0
- `scripts/verify_anchors.sh`: 119/119 PASS

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
- 2026-06-09 (architect): **Design resolved, no ADR** (additive UI wiring
  within the `cockpit_live` subscription + `PanelState` precedent). **D1=(a)
  UI-accumulate** (session-scoped buffer; durable agent-side history
  deferred to follow-on `live-equity-history-durable`). **D2:** Max-DD +
  session Total-return LIVE from the accumulated `EquitySeries`;
  Sharpe/CAGR/Win-rate `—` (no `core` math, verified); **Trades = `0`** —
  *finding: the UI keeps NO fill counter* (`FillReceived` only pushes the
  capped `tape` VecDeque, `state.rs:1782`; `tape.len()` is a bounded window,
  not a session total), so a true counter is deferred. **D3:** no throttle
  (per-bar feed). **D4:** curve-only (Live layout has no band slot). **D5:**
  derive on-append, cache `PanelState` on the model. **Equity buffer:**
  `live_equity_buffer: VecDeque<(Timestamp, Money<Usdt>)>` on the model,
  appended `(as_of, total_equity)` per `PnlRefreshed` with a **monotone
  guard** (`from_points` rejects non-monotone ts, `equity_series.rs:83` —
  drop late points) and a **bounded ring** `LIVE_EQUITY_BUFFER_CAP = 2_880`
  (48 h of 1-min bars); empty on each boot (session-scoped, not
  serialized). **Critical edge flagged:** `kpi_strip::is_all_absent`
  (`kpi_strip.rs:79`) masks a 1-point strip (all-zero == sentinel) →
  **strip stays Loading until ≥2 points**; curve renders from ≥1.
  Verified **no new crate edge** (`ui→core` + `ui→agent` already exist).
  `screens/live.rs:58,66` swap the two `&PanelState::Loading` for
  `&model.live_equity_curve` / `&model.live_kpi`. tasks.md emitted (T1–T9,
  ui-designer-solo). trace REQ-COCKPIT-LIVE-DASHBOARD-001 proposed →
  arch-done. HANDOFF → ui-designer.
- 2026-06-09 (ui-designer): **Implemented T1–T9 solo (all green).** Three
  model fields added (`live_equity_buffer` / `live_equity_curve` /
  `live_kpi`) initialized in both struct-literal constructors + the `Debug`
  impl; `Cockpit::push_live_equity_point` derives the curve + strip on each
  `PnlRefreshed` (monotone guard drops late points; ring cap
  `LIVE_EQUITY_BUFFER_CAP = 2_880`; strip held Loading until ≥2 pts for the
  `is_all_absent` trap); `PnlError` degrades both panels to `Error` (no
  panic); `screens/live.rs:58,66` swapped to the model-backed states + an
  honest "Session to date" caption (`LIVE_SESSION_RETURN_CAPTION`). KPI
  mapping: Total-return (session) + Max-DD LIVE; Sharpe/CAGR/Win-rate `—`;
  Trades 0 (no live counter — deferred). **Tests:** 7 new `state.rs` lib
  tests (PnlRefreshed-populates-curve, 1-pt→Loading-strip, live Max-DD,
  monotone-guard, PnlError-no-panic, ring-cap, session-reset) + a
  `headless_emulator_paints_live_route` fixtures smoke + 3 Live panel
  snapshots (steady-state regenerated; ready_dark + ready_light added). 435
  lib tests pass; panel/consistency/contrast/layout green; `--features live`
  `cockpit_live` + fixtures `cockpit` builds clean; new-code clippy/fmt
  clean; anchors 119/119. Regenerated the `live__recent_activity_with_chevron`
  visual PNG triple (diff = only the new caption + wired panels; 48 others
  unchanged). Pre-existing unrelated failure: `lab_run_engine::h3_…`
  (network-dependent `--features live` backtest, fails identically on clean
  HEAD). § UI filled. status → ui-done. HANDOFF → tester.
- 2026-06-10 (ui-designer): **v0.1.1 — three post-ship display fixes from the
  operator's first real session** (cockpit_live, paced replay 30 ms/bar,
  sma_crossover BTCUSDT 2023-24). All three diagnosed against code + the
  shipped run; two UI-layer fixes + one verdict-of-correct-with-a-display-fix.
  **(1) Equity-curve X-axis label smear** — root cause
  `widgets/chart::time_axis_tick_count` (`chart.rs:125`) derived the interval
  count as `(bar_count − 1) / fixed_step` (step 5/10/15), so the **label count
  scaled with the series length**: the 2880-pt Live ring → ~575 labels, the
  367-pt Baseline year-series → ~73 (a shared bug — the Baseline curve had it
  too, just milder). The `time_axis_tick_count_adaptive` test only covered
  `n = 60`, which masked it. **Fix:** cap the interval count at the
  width-derived label budget `clamp(width/96, 4, 12)` —
  `raw_intervals.min(max_labels)` — so any series is bounded to ≤ 12 labels;
  **strict no-op for the ≤ 60-bar price chart** (`(59)/5 = 11 ≤ 12`), so the
  `charts_screen_*` baselines stay byte-identical. Added adaptive **span-based
  label formatting** (`format_time_axis_label`, shared by `equity_curve` +
  `drawdown_band`): `HH:MM` < 6 h, `MMM DD HH:MM` < 14 d, `MMM DD` < 18 mo,
  `MMM 'YY` ≥ 18 mo (the 2-year replay) — month names from new
  `strings::MONTH_ABBREVS`/`month_abbrev` (no inline literals). **(2) "Total
  return 0.01–0.02"** — **fraction-vs-percent unit bug** at the wiring seam:
  `state.rs:1360` fed `(latest−first)/first` (a FRACTION, 0.015) into
  `BacktestMetrics.total_return_pct`, whose **PERCENT** semantics the baseline
  const (`196.22` → "+196.22%") and `format_pct_sentiment` (appends `%`
  verbatim) establish → a +1.5 % session rendered "0.01%" (100× too small).
  `max_drawdown_pct` had the **same bug** (`EquitySeries::max_drawdown_pct` is
  a fraction 0.40; the strip's `format_pct_max_dd` expects percent). **Fix:**
  ×100 both at the seam. The wiring test's `total_return = 0.10` assertion was
  the bug encoded as fact (a 1000→1100 = +10 % session); corrected to `10`
  (percent), + added `live_kpi_units_render_percent_not_fraction` pinning the
  *rendered card text* ("1.50%", "−2.46%") through the real formatters. **(3)
  "~$4 buys, never larger / smaller"** — **NOT an exec bug; a display
  ambiguity.** Config is `[risk.sizing] fixed_fraction = 0.10` on
  `initial_capital_usdt = 100000` → real clips are ~$10k notional; `taker_fee
  = 4 bps × $10k = exactly $4.00`. The agent_feed tape's rightmost column was
  the **fee** (`fmt_usdt(fill.fee)`), the only USDT-suffixed number in the row;
  the qty column showed BTC (e.g. "0.1") with no unit. The operator read the
  $4 fee as the trade size. "Never larger/smaller" = correct fixed-10 % clips
  (≈ constant ~$4 fee until equity moves). **Fix (UI-only):** added a
  **Notional** column (qty × price, USDT — derived in the widget from existing
  `FillView` fields, no backend change) between Qty and Fee, so the ~$10k clip
  is visible and the ~$4 fee is unmistakably a fee. **Tests:** 437 lib (3 new:
  axis-density-for-long-series, span-band formatting, unit-pinning); 2 state
  assertions corrected to percent; consistency/contrast/layout green;
  regenerated 4 panel snaps (2 live = "10.00%"; 2 agent_feed = notional+fee
  columns) + the `live__recent_activity_with_chevron` visual triple (notional
  column now rendered); 52 other visual baselines + all `charts_screen_*`
  unchanged (price-chart cap is a no-op). new-code clippy/fmt clean; anchors
  119/119. **No agent/exec change** — sizing is correct as configured.
