---
slug: cockpit-live-dashboard-wiring
status: in-progress
owner: ui-designer
updated: 2026-06-09
---

# Tasks — cockpit-live-dashboard-wiring

**Solo: ui-designer.** This is ~100% UI / 0% agent-exec work (D1=(a)) —
mirror how `cockpit-baseline-panel` was implemented solo. No new crate
edge, no new bus channel, no new widget, no new theme token, no `core`
math. The live `pnl` feed already arrives at the UI every bar
(`live.rs:215 stream_pnl` → `Message::PnlRefreshed` → `state.rs:1808`);
this feature renders it on the two stubbed panels instead of dropping it.

Read the resolved `## Design` in [feature.md](feature.md) first — it has
the D1–D5 resolutions, the equity-buffer contract, the KPI-source mapping,
the four `PanelState` transitions, and two **must-honor** edges (the
`is_all_absent` 1-point trap and the `from_points` monotone-timestamp
guard).

Lint convention (binding): new code follows the crate's existing
per-module `#![allow(...)]` pattern, adds **zero** new warnings, and does
**not** touch the ~140 pre-existing `crates/ui` pedantic lints (out of
scope).

---

- [ ] **T1 — Buffer-cap const.** Add `LIVE_EQUITY_BUFFER_CAP: usize =
  2_880;` to `crates/ui/src/theme.rs` (`layout` module, beside
  `SPARKLINE_POINT_CAP`), with a doc-comment: 48 h of 1-min bars, ring
  bound for the session equity buffer; render still `downsample`s to
  `SPARKLINE_POINT_CAP`. — _acceptance: const compiles, documented; no
  other behavior change. (supports R1/D-buffer)_

- [ ] **T2 — Model fields.** Add three fields to `struct Cockpit`
  (`crates/ui/src/state.rs`, beside `pnl`/`positions`/`strategy_equity`):
  `live_equity_buffer: VecDeque<(Timestamp, Money<Usdt>)>`,
  `live_equity_curve: PanelState<EquitySeries>`,
  `live_kpi: PanelState<BacktestMetrics>`. Initialize all three empty /
  `PanelState::Loading` in `Cockpit::new()` (`state.rs:1160`) and every
  other constructor/`Debug` site that lists fields (grep
  `baseline_screen_state:` to find them all — there are ≥2). The buffer is
  **not** `Serialize`d. — _acceptance: `cargo build -p ui` green; buffer +
  both panels start empty/Loading on a fresh `Cockpit`. (R1/R3, D1/D5)_

- [ ] **T3 — Append + derive in the `PnlRefreshed` arm.** Extend the
  existing `Message::PnlRefreshed(snap)` arm (`state.rs:1808`, keep
  `model.pnl = PanelState::Ready(snap)`):
  1. **Monotone guard:** push `(snap.as_of, snap.total_equity)` only if the
     buffer is empty OR `snap.as_of >= buffer.back().0` (drop a strictly
     earlier late point — do not append, do not error).
  2. **Ring bound:** after push, `pop_front()` while
     `buffer.len() > LIVE_EQUITY_BUFFER_CAP`.
  3. **Curve (≥1 pt):** `EquitySeries::from_points(buffer cloned →
     Vec)` → `Ok(s)` ⇒ `live_equity_curve = Ready(s)`; `Err` (only the
     guarded-empty case) ⇒ leave `Loading`.
  4. **Strip (≥2 pts, `is_all_absent` trap):** if `buffer.len() < 2` ⇒
     `live_kpi = Loading`; else build `BacktestMetrics` with
     `total_return_pct = (latest − first)/first` (Decimal, guard first≠0),
     `max_drawdown_pct = s.max_drawdown_pct`, `trades = 0`,
     `cagr_present = sharpe_present = win_rate_present = false`, remaining
     numerics zero ⇒ `live_kpi = Ready(m)`.
  — _acceptance: a headless sequence of `PnlRefreshed` grows
  `live_equity_buffer`, transitions curve Loading→Ready at point 1 and
  strip Loading→Ready at point 2; an out-of-order (earlier `as_of`)
  snapshot is dropped (buffer length unchanged, no error). (AC1, R1/R2/R4)_

- [ ] **T4 — Error/Empty propagation.** Extend the `Message::PnlError(e)`
  arm (`state.rs:1818`, keep `model.pnl = Error(e)`) to also set
  `live_equity_curve = Error(e.clone())` and `live_kpi = Error(e)`. (Empty:
  if a closed-with-zero-points path is distinguishable in the impl, map it
  to `PanelState::Empty`; otherwise `Error` is acceptable — both render a
  non-blank body.) — _acceptance: `PnlError` drives both new panels to
  `Error`, no panic. (AC2, R3)_

- [ ] **T5 — Wire `screens/live.rs`.** Replace the two hard-wired refs:
  `live.rs:58` → `let equity_state = &model.live_equity_curve;` and
  `live.rs:66` → `let kpi_state = &model.live_kpi;`. Leave the
  `equity_curve::view(...).map(|_| Message::ChartMarkerHoverEnded)` and
  `kpi_strip::view(...).map(...)` bridges unchanged. Update the module
  header (lines 8-13, 55-57, 64-66) to drop the "no live feed yet / Design
  § A7/A8 / Phase F" annotations and reference this feature. Leave the
  LLM-spend tile (line 69) as-is. — _acceptance: `cargo build -p ui
  --features live` green; Live screen reads the model-backed states; no
  remaining `&PanelState::Loading` literal for these two panels. (R1/R2,
  AC7)_

- [ ] **T6 — Strings (R6).** If a Total-return caption is added (recommended
  per R5/AC5), add `LIVE_SESSION_RETURN_CAPTION: &str = "Session to date";`
  to `crate::strings` (`LIVE_*` block) and register it in the `strings.rs`
  test-table. No hardcoded string literals in any new code; no new theme
  token. — _acceptance: any new copy comes from `strings.rs` and is in the
  registration table; `cargo test -p ui strings` green. (R5/R6, AC4/AC5)_

- [ ] **T7 — Headless feed-drive + four-state test.** Add a unit/headless
  test (in `state.rs` `#[cfg(test)]`, mirroring the existing
  `pause_buffers_fills_and_resume_flushes` fixture at `state.rs:3052` for
  constructing inputs, and the `bus.publish_pnl` test at `state.rs:1019`):
  drive a sequence of `Message::PnlRefreshed` into `update(&mut c, ...)`
  and assert (a) `live_equity_buffer.len()` grows per snapshot; (b) the
  curve `PanelState` is Loading at 0, Ready at ≥1; (c) the strip is Loading
  at 1, Ready at ≥2 (the `is_all_absent` proof); (d) `PnlError` → both
  Error (no panic); (e) a same-timestamp snapshot appends, a strictly
  earlier one is dropped. — _acceptance: the test passes and is the core
  wiring proof that a `PnlRefreshed` sequence populates the curve. (AC1,
  AC2)_

- [ ] **T8 — Fixtures smoke (Loading-default, R7).** Confirm
  `headless_emulator_smoke.rs` boots the default Live route with **no live
  agent** and paints frame 1 with both panels in their **Loading** body, no
  panic, in the existing smoke window. (The existing
  `headless_emulator_boots_cockpit_and_renders` already boots the default
  route — verify the buffer's empty path renders Loading without a feed; add
  a targeted `Screen::Live` first-frame assertion only if the default-route
  test does not already cover it, mirroring
  `headless_emulator_paints_baseline_route` at line 87.) — _acceptance:
  fixtures `cockpit` smoke is green; Live curve + strip paint Loading
  bodies with no feed, no panic. (AC3, R7)_

- [ ] **T9 — Panel snapshots (both themes) + update the existing baseline.**
  In `crates/ui/tests/panel_snapshots.rs` `mod live_screen`
  (line 2637): the existing `live_screen_summary` helper hard-codes the
  equity-curve + kpi-strip as Loading placeholders (lines 2648-2657) — update
  it to read the model-backed states (`c.live_equity_curve.variant_name()` /
  `c.live_kpi.variant_name()` plus the rendered card/placeholder copy). Add a
  **Ready** snapshot (seed `live_equity_buffer` with ≥2 monotone points via
  `update(... PnlRefreshed ...)` so curve+strip are Ready) in **both**
  `ThemeMode::Dark` and `ThemeMode::Light`, keeping the existing **Loading**
  `live_snapshot__steady_state` as the default-state baseline. Regenerate the
  affected `.snap` baselines (was: two Loading placeholders). Keep
  `tests/consistency.rs` / `tests/contrast.rs` / `tests/layout_invariants.rs`
  green. — _acceptance: Live-screen snapshots cover Loading (default) +
  Ready (seeded) in both themes; Lumen-consistency suites green; no
  hardcoded colors/strings. (AC4, AC6)_

## Notes

- **Why solo (no developer):** D1=(a) means 0% agent/exec work — the `pnl`
  feed, reconciler publish, bus channel, subscription, and `PnlRefreshed`
  message are all already shipped (T903c / live-cockpit-unified). The entire
  surface is `crates/ui` state + one screen wiring + tests, exactly the
  `cockpit-baseline-panel` solo shape (and smaller — no loader / no CSV /
  no embedded const / no new screen / no sidebar IA).

- **Two must-honor edges (do not skip — they are correctness, not polish):**
  1. **`is_all_absent` 1-point trap** (`widgets/kpi_strip.rs:79`): a
     1-point series yields `total_return=0, max_dd=0, trades=0` ==
     all-absent sentinel → the strip would wrongly render six dashes. T3
     keeps the **strip Loading until ≥2 points** (the curve renders from 1).
  2. **`from_points` monotone-timestamp guard** (`equity_series.rs:83`):
     a non-monotone `(as_of)` pair makes `from_points` return
     `Err(NonMonotoneTimestamps)`. T3's append-guard drops strictly-earlier
     late points so the build never errors on out-of-order snapshots.

- **No baseline-divergence e2e gate** — this is a read-only live-monitor UI
  wiring (no strategy overlay, no sizing math, no decision variable), so the
  CLAUDE.md baseline-equity-divergence requirement does **not** apply
  (confirmed in feature.md § Acceptance criteria / § Backtest Scenarios).

- **Trades deferred:** v0.1.0 renders Trades `= 0` (the UI keeps no fill
  counter — `FillReceived` only feeds the capped `tape` VecDeque). A true
  session fill-counter (a `u64` on the model, ++ in the `FillReceived` arm,
  0 on boot) + its test is the named follow-on, not this feature.

- **Watch recipe** (T7/T9 run cargo > 2 min on a cold `crates/ui` build):
  ```
  watch -n 10 'cargo test -p ui --features live live_screen 2>&1 | tail -20'
  ```
