# TODO — cockpit Live view & follow-ups

_As of 2026-06-11, HEAD `40f5de9`. Starting points + gotchas for picking this back up._

The cockpit's **Live view works**: `cargo run -p ui --release --bin cockpit_live --features live`
boots the paper agent in research mode (`config/agent.toml`), replays `data/binance`
through the `sma_crossover` strategy, and shows a live fills tape, positions, a growing
equity curve, and a KPI strip. Total-return units are correct, and the agent trades real
~$10k clips. The items below are what's left.

**Read first:** [`spec/cockpit-live-dashboard-wiring/feature.md`](spec/cockpit-live-dashboard-wiring/feature.md)
(design) + [`spec/cockpit-live-dashboard-wiring/tasks.md`](spec/cockpit-live-dashboard-wiring/tasks.md)
(v0.1.x follow-ups, incl. the reverted I1). Relevant commits: `40f5de9` (bar-time revert),
`72d0138` (Live wiring), `40c9be3` (paced replay), `4b478ad` (replay path/pace fix).

---

## ⚠️ Read this before touching the Live equity curve

Every blind fix to the live curve broke it, because it was verified at the **agent / unit-test
layer** (which passed) while the **actual rendered curve** was broken. The unit test publishing
a `PnlSnapshot` is NOT proof the curve renders.

**Before changing the curve or x-axis: build a render-verifiable test first.** It should feed a
realistic sequence of `Message::PnlRefreshed` into the equity buffer and assert the curve
actually renders — non-empty, expected point count, expected time span. The panel-snapshot
harness (`crates/ui/tests/panel_snapshots.rs`) renders a screen to a buffer; that's where it
goes. Once you can SEE the curve break in a test, the fixes below become safe.

---

## 1. Equity-curve x-axis shows wallclock time, not data dates  ← main open item

**Symptom:** during a replay the x-axis labels all show the current wallclock minute (e.g. "7:51")
instead of the historical 2023–24 dates.

**Why it's stuck:** the obvious fix — stamp `PnlSnapshot.as_of` with `bar.close_ts` (data time) —
**breaks the entire curve** and was reverted in `40f5de9`. The UI equity buffer's **monotone guard**
(`push_live_equity_point`) drops any point whose `as_of < back.ts`. With bar-time (2023) values,
those points get dropped relative to any wallclock-stamped point → empty curve ("no graph").

**The right fix:** give the equity point a **separate data-time x-coordinate**, distinct from
`as_of`. Keep `as_of` = wallclock `now()` (the monotone guard + any freshness logic rely on it);
add the bar/data time as the x-axis coordinate the chart plots. `format_time_axis_label` already
does span-adaptive `MMM 'YY` / `MMM DD` labels, so the labels will be right once the x-coord is the
data time. **Verify with the render test above before shipping.**

**Start here:**
- `crates/ui/src/state.rs` — `push_live_equity_point` (the monotone guard; buffer = `live_equity_buffer`) and the `PnlRefreshed` message arm.
- `crates/agent/src/runtime.rs` (~line 1112, `spawn_research_trading_loop`) — where `as_of` is set; `bar.close_ts` is the data time available right there.
- `crates/core/src/views.rs` — `PnlSnapshot` (likely needs a data-time field added).
- `crates/ui/src/widgets/{chart.rs, equity_curve.rs}` — x-axis tick + label rendering (`format_time_axis_label`, `time_axis_tick_count`).

---

## 2. Trades KPI always shows 0

**Symptom:** the Live KPI strip's "Trades" card shows 0.
**Why:** there is no live fill counter — `FillReceived` only pushes into the capped/evicting `tape`
deque, so `tape.len()` is a sliding window, not a session total. This is honest (0, not faked), but
a real counter is the follow-on.
**Start here:** `crates/ui/src/state.rs` (`FillReceived` arm, ~line 1782) — add a `u64` session
counter; render it in `crates/ui/src/widgets/kpi_strip.rs`.

---

## 3. Equity curve resets on every restart  (larger, exec-side)

**Symptom:** the curve is session-scoped — it starts empty each `cockpit_live` boot.
**Why:** the agent keeps only a scalar equity, no durable series. Deferred as a follow-on
(`live-equity-history-durable`, exec-side, ~L effort).
**Start here:** `crates/agent/src/reconciler.rs` (the per-bar equity); the architect's `D1=(b)`
note in `spec/cockpit-live-dashboard-wiring/feature.md`.

---

## 4. Trivial: deprecated `Screen::Home`

`crates/ui/src/bin/cockpit.rs:185` uses the deprecated `Screen::Home` alias → change to
`Screen::Live`. One line; currently just a build warning.

---

## Notes / knobs (not bugs)

- **Replay speed:** `config/agent.toml` → `[data.historical] replay_pace_ms = 30` (≈8.8 min full
  replay; set `5` for ≈1.5 min). The cockpit replays `data/binance` in research mode; the headless
  `trading` bin can pass `--fast-replay` for as-fast-as-possible.
- **The "$4 buys/sells" were never a bug** — that's the 4 bps taker fee on ~$10k clips
  (`fixed_fraction = 0.10` × $100k). The **Notional** column shows the real ~$10k trade size.
- **Parked:** [`spec/cockpit-reports-viewer/feature.md`](spec/cockpit-reports-viewer/feature.md)
  (candidate — no backtest report has equity-curve companion data, so it'd be a metrics/writeup
  browser only). [`spec/cockpit-chart-cache/feature.md`](spec/cockpit-chart-cache/feature.md)
  (measured NO-GO — `canvas::Cache` saves <0.1% of a frame).

---

## Broader project state

The active-vs-passive **research program is concluded** — across price/OHLCV, derivatives-
positioning, and on-chain, no active strategy beat passive buy-and-hold net of cost; the shipped
approach is passive. See [`spec/product.md`](spec/product.md) (terminal verdict) and
[`spec/dev-notes/presentations/program-capstone-2026-06-08.md`](spec/dev-notes/presentations/program-capstone-2026-06-08.md).
The cockpit is the post-research build-out: a Baseline panel (the shipped BH result), the Live
monitor (this file's subject), a 24–40× interaction-perf fix, and a repaired visual-regression gate.
[`spec/backlog.md`](spec/backlog.md) holds the wind-down state.
