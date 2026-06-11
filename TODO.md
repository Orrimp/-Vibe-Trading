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

**The render-verifiable harness now EXISTS:** `crates/ui/tests/live_equity_render.rs` rasterizes
the real Live screen via `iced_test::screenshot` and counts the curve's ACCENT polyline pixels —
with a self-proving pair that confirms it can tell a broken curve from a healthy one. **Run it
(`cargo test -p ui --test live_equity_render`) before AND after any change to the curve / x-axis /
equity buffer.** If you can't first make it go RED for your bug, you can't trust it GREEN for your
fix. (This is what closed item #1 below.)

---

## 1. ✅ DONE (2026-06-11) — Equity-curve x-axis now plots data dates (approach A)

**Landed + render-verified.** `PnlSnapshot.bar_ts` (the bar/data time) is now the chart x-coord,
kept SEPARATE from the wallclock `as_of` (which stays the equity buffer's out-of-order delivery
guard). During a fast replay the axis shows real 2023-24 dates instead of one repeated wallclock
minute. The render harness above (`live_equity_render.rs`, 5/5 green) proves the curve rasterizes;
the unit test `live_equity_curve_plots_bar_ts_not_wallclock` proves it plots `bar_ts`. A **bonus
NaN-panic** in `equity_curve.rs` (flat / 1-point curve → `lyon_path: p.y.is_finite()`, the cockpit
crashing on the FIRST live bar) was caught by the harness and fixed in the same pass. Full record:
`spec/cockpit-live-dashboard-wiring/tasks.md` I1.

> **Operator still owes the human-render confirmation:** run
> `cargo run -p ui --release --bin cockpit_live --features live` and confirm the equity-curve
> x-axis labels read as **2023-24 dates** (e.g. `Jan '23`, `Mar '23`), not a wallclock minute.
> The automated harness proves the polyline draws and stores `bar_ts`; only your eyes confirm the
> rendered *labels*.

<details><summary>Original investigation notes (how it was solved — kept for reference)</summary>

**Symptom (was):** during a replay the x-axis labels all showed the current wallclock minute
(e.g. "7:51") instead of the historical 2023-24 dates.

**Why the first attempt got stuck:** the obvious fix — stamp `PnlSnapshot.as_of` with
`bar.close_ts` — **broke the entire curve** (reverted in `40f5de9`). The buffer's monotone guard
(`push_live_equity_point`) drops any point whose `as_of < back.ts`; bar-time (2023) values got
dropped relative to wallclock-stamped points → empty curve.

**The fix that landed (approach A):** a separate data-time x-coordinate (`bar_ts`), distinct from
`as_of`. `as_of` stays wallclock `now()` (the guard relies on it); `bar_ts` is the plotted x-coord.
`format_time_axis_label` already does span-adaptive `MMM 'YY` / `MMM DD` labels, so the labels are
right once the x-coord is the data time. Files touched: `crates/core/src/views.rs` (`bar_ts` field),
`crates/agent/src/{runtime.rs,reconciler.rs}` (publish `bar_ts`), `crates/ui/src/state.rs` (guard
keys on `as_of`, plots `x_coord`), `crates/ui/src/widgets/equity_curve.rs` (NaN guard).

</details>

---

## 2. ✅ DONE (2026-06-11) — Trades KPI shows the live session fill count

`Cockpit.live_fill_count` (u64, session-scoped) increments on every `FillReceived` — including
fills buffered while the tape display is paused — updates a Ready KPI strip **in place** (current
the instant a fill lands, not only at the next per-bar rebuild), and feeds the per-bar KPI rebuild.
Semantic: **fills**, not round-trips (honest for a monitor; round-trip pairing would be exec-side).
Verified at the model layer (`live_trades_counter_counts_session_fills`) AND the summary/render
layer (`panel_snapshots::live_kpi_trades_card_shows_session_fill_count` asserts the rendered
"Trades: 3" card text); cockpit-smoke PASS.

---

## 3. Equity curve resets on every restart  (larger, exec-side)

**Symptom:** the curve is session-scoped — it starts empty each `cockpit_live` boot.
**Why:** the agent keeps only a scalar equity, no durable series. Deferred as a follow-on
(`live-equity-history-durable`, exec-side, ~L effort).
**Start here:** `crates/agent/src/reconciler.rs` (the per-bar equity); the architect's `D1=(b)`
note in `spec/cockpit-live-dashboard-wiring/feature.md`.

---

## 4. ✅ DONE (2026-06-11) — deprecated `Screen::Home` → `Screen::Live`

`crates/ui/src/bin/cockpit.rs:185` now uses `Screen::Live` (the alias target — behavior-identical).
Deprecation warning gone; cockpit-smoke PASS on the fixtures boot.

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
