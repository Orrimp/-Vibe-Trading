---
slug: cockpit-live-dashboard-wiring
status: tester-done
owner: ui-designer
updated: 2026-06-11
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

- [x] **T1 — Buffer-cap const.** Add `LIVE_EQUITY_BUFFER_CAP: usize =
  2_880;` to `crates/ui/src/theme.rs` (`layout` module, beside
  `SPARKLINE_POINT_CAP`), with a doc-comment: 48 h of 1-min bars, ring
  bound for the session equity buffer; render still `downsample`s to
  `SPARKLINE_POINT_CAP`. — _acceptance: const compiles, documented; no
  other behavior change. (supports R1/D-buffer)_
  **DONE** — `theme::layout::LIVE_EQUITY_BUFFER_CAP = 2_880`
  (`crates/ui/src/theme.rs:805`), doc-commented (48 h of 1-min bars, ring
  bound, render still downsamples to `SPARKLINE_POINT_CAP`).

- [x] **T2 — Model fields.** Add three fields to `struct Cockpit`
  (`crates/ui/src/state.rs`, beside `pnl`/`positions`/`strategy_equity`):
  `live_equity_buffer: VecDeque<(Timestamp, Money<Usdt>)>`,
  `live_equity_curve: PanelState<EquitySeries>`,
  `live_kpi: PanelState<BacktestMetrics>`. Initialize all three empty /
  `PanelState::Loading` in `Cockpit::new()` (`state.rs:1160`) and every
  other constructor/`Debug` site that lists fields (grep
  `baseline_screen_state:` to find them all — there are ≥2). The buffer is
  **not** `Serialize`d. — _acceptance: `cargo build -p ui` green; buffer +
  both panels start empty/Loading on a fresh `Cockpit`. (R1/R3, D1/D5)_
  **DONE** — three fields added after `strategy_equity`
  (`state.rs:1013`). Initialized in **both** struct-literal constructors —
  `impl Default` (`state.rs:1163`) and `Cockpit::ready()` (`state.rs:1283`);
  `new()`/`boot()` delegate via `..Self::default()` so they inherit. Added
  to the manual `Debug` impl (`live_equity_buffer_len` + both panels). The
  `Cockpit` struct has exactly **two** struct literals (the brief's "≥2"):
  no other site lists fields — see seam note in feature.md § UI.

- [x] **T3 — Append + derive in the `PnlRefreshed` arm.** Extend the
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
  **DONE** — extracted to a private `Cockpit::push_live_equity_point(ts,
  equity)` helper (`state.rs:1318`) called from the `PnlRefreshed` arm
  (`state.rs:1811`) BEFORE `model.pnl = Ready(snap)` (captures `as_of` /
  `total_equity` before the move). Monotone guard + ring bound + curve
  rebuild (`from_points`, ≥1 pt) + strip rebuild (Loading until ≥2 pts,
  session-return Decimal divide-guarded). Verified by the T7 tests.

- [x] **T4 — Error/Empty propagation.** Extend the `Message::PnlError(e)`
  arm (`state.rs:1818`, keep `model.pnl = Error(e)`) to also set
  `live_equity_curve = Error(e.clone())` and `live_kpi = Error(e)`. (Empty:
  if a closed-with-zero-points path is distinguishable in the impl, map it
  to `PanelState::Empty`; otherwise `Error` is acceptable — both render a
  non-blank body.) — _acceptance: `PnlError` drives both new panels to
  `Error`, no panic. (AC2, R3)_
  **DONE** — `PnlError(e)` arm (`state.rs:1825`) sets
  `live_equity_curve = Error(e.clone())`, `live_kpi = Error(e.clone())`,
  then `pnl = Error(e)`. `PnlError(SmolStr)` is `Clone` so the two clones
  are cheap. Empty: the closed-channel path routes through `PnlError`
  (not separately distinguishable in the UI `update`), so it maps to
  `Error` per the design's accepted fallback — both render a non-blank
  body. Verified by `pnl_error_drives_live_panels_to_error_no_panic`.

- [x] **T5 — Wire `screens/live.rs`.** Replace the two hard-wired refs:
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
  **DONE** — `equity_state = &model.live_equity_curve` (`live.rs:67`),
  `kpi_state = &model.live_kpi` (`live.rs:74`); both `.map(|_|
  Message::ChartMarkerHoverEnded)` bridges unchanged. Module header
  rewritten to reference this feature (dropped the Phase-F / no-feed-yet
  annotations). Removed the now-unused `PanelState` / `EquitySeries` /
  `BacktestMetrics` imports (the swap deleted the only construction site).
  LLM-spend tile untouched. Also added the honest **session caption**
  (T6). `cargo build -p ui --features live --bin cockpit_live` green.

- [x] **T6 — Strings (R6).** If a Total-return caption is added (recommended
  per R5/AC5), add `LIVE_SESSION_RETURN_CAPTION: &str = "Session to date";`
  to `crate::strings` (`LIVE_*` block) and register it in the `strings.rs`
  test-table. No hardcoded string literals in any new code; no new theme
  token. — _acceptance: any new copy comes from `strings.rs` and is in the
  registration table; `cargo test -p ui strings` green. (R5/R6, AC4/AC5)_
  **DONE** — `LIVE_SESSION_RETURN_CAPTION = "Session to date"` added to the
  `LIVE_*` block (`strings.rs:1745`), wired into `screens/live.rs` under
  the KPI row + asserted by the Live panel snapshots. No hardcoded string
  literals, no new theme token. (Seam note: the `consistency.rs` "no inline
  strings" gate scans only `src/widgets/*.rs`; there is no separate
  `strings.rs` registration table to add to — the snapshot helper IS the
  reference site. `consistency.rs` green.)

- [x] **T7 — Headless feed-drive + four-state test.** Add a unit/headless
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
  **DONE** — 7 new `state.rs` `#[cfg(test)]` tests (all green) with a
  parameterized `pnl_snap_at(secs, equity)` helper:
  `pnl_refresh_sequence_populates_live_equity_curve` (a/b — the core proof),
  `live_kpi_strip_loading_at_one_point_ready_at_two` (c — the
  `is_all_absent` proof + Total-return/Trades/absent-flag assertions),
  `live_kpi_strip_max_drawdown_is_live` (live Max-DD = 0.25 + negative
  session-return), `live_equity_buffer_drops_out_of_order_and_allows_equal_ts`
  (e — monotone guard), `pnl_error_drives_live_panels_to_error_no_panic`
  (d), `live_equity_buffer_is_bounded_ring` (ring cap + eviction),
  `live_panels_reset_on_fresh_cockpit` (session-scoped reset).

- [x] **T8 — Fixtures smoke (Loading-default, R7).** Confirm
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
  **DONE** — added `headless_emulator_paints_live_route`
  (`headless_emulator_smoke.rs:129`) mirroring the baseline-route test:
  asserts the fixtures cockpit at `Screen::Live` starts with an empty
  buffer + both panels Loading, then boots through the Emulator and asserts
  a non-empty first-frame screenshot (no panic, no feed). All 3
  `headless_emulator_smoke` tests green.

- [x] **T9 — Panel snapshots (both themes) + update the existing baseline.**
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
  **DONE** — `live_screen_summary(c, mode)` now reads `c.live_equity_curve`
  + `c.live_kpi` via `equity_curve_line` / `kpi_strip_lines` helpers
  (rendering the real card text via `num::format_pct_*` / `format_count`).
  Three snapshots: `live_snapshot__steady_state` (Loading default,
  regenerated — diff is ONLY the `theme:` + `session_caption:` lines, the
  curve/strip copy is byte-identical to the old Loading baseline),
  `live_snapshot__ready_dark`, `live_snapshot__ready_light` (seed ≥2
  monotone points via `PnlRefreshed`; Total-return 0.10% / Max-DD 0.00% /
  Sharpe·CAGR·Win `—` / Trades 0). consistency/contrast/layout green.
  **Visual baselines:** the `live__recent_activity_with_chevron` PNG triple
  (floor/typical/operator) regenerated — diff is ONLY the new "Session to
  date" caption + the wired panels (still Loading in that feedless fixture);
  48 other visual snapshots unchanged.

- [x] **M-TEST — tester gate** (tester, 2026-06-09) — VERDICT PASS. `cargo test -p ui` 435 unit + all integration suites GREEN (one pre-existing `lab_run_engine::h3_…` network-dependent failure confirmed pre-existing on parent commit, whitelisted). Static: all 3 builds (default / `--features live` / fixtures) clean; fmt pre-existing bench diff in `chart_build_probe.rs` (not a feature file — carries over from commit 07f71be); zero new warnings from feature's changed files (state.rs/live.rs/theme.rs/strings.rs). 7 wiring tests all GREEN; 1-point trap proven; visual gate 51/51 (only `live__recent_activity_with_chevron` triple changed). `verify-anchors` 119/119. `spec-lint` exit 0 (2 missing-frontmatter self-corrected to `tester-done`). `git diff crates/` empty. Both open questions ratified (Trades = 0 honest; Empty-on-channel-close → Error, non-blank, no panic). Report: `spec/cockpit-live-dashboard-wiring/reports/test-2026-06-09-cockpit-live-dashboard-wiring.md`.

## Follow-up polish — v0.1.1 (ui-designer, 2026-06-10)

Three display issues surfaced by the operator's first real session
(cockpit_live, paced replay 30 ms/bar, sma_crossover BTCUSDT 2023-24).
Diagnosed against code + run; two UI fixes + one correct-as-configured verdict.

- [x] **F1 — Equity-curve X-axis label smear (rendering bug).**
  `widgets/chart::time_axis_tick_count` returned `(bar_count−1)/fixed_step`
  intervals → label count scaled with series length (2880-pt Live ring → ~575
  labels; 367-pt Baseline → ~73). Capped at the width budget
  (`raw_intervals.min(clamp(width/96, 4, 12))`) → ≤ 12 labels for any length;
  **no-op for the ≤ 60-bar price chart** (baseline-preserving). Added
  span-adaptive `format_time_axis_label` (HH:MM / MMM DD HH:MM / MMM DD /
  MMM 'YY) shared by `equity_curve` + `drawdown_band`; month names via new
  `strings::MONTH_ABBREVS` / `month_abbrev`. Tests:
  `time_axis_tick_count_bounded_for_long_series` (pins n=367 & n=2880 ≤
  budget) + `format_time_axis_label_span_bands`.
  Files: `crates/ui/src/widgets/{chart,equity_curve,drawdown_band}.rs`,
  `crates/ui/src/strings.rs`.

- [x] **F2 — "Total return 0.01–0.02" — fraction-vs-percent unit bug.**
  `state.rs:1360` fed a fraction `(latest−first)/first` into
  `BacktestMetrics.total_return_pct` (PERCENT semantics) → +1.5 % rendered
  "0.01%". `max_drawdown_pct` had the same bug (`EquitySeries` fraction →
  percent-expecting `format_pct_max_dd`). ×100 both at the seam. Corrected the
  wiring test's `0.10`→`10` and `-0.10`→`-10`, `0.25`→`25` assertions (they
  encoded the bug); added `live_kpi_units_render_percent_not_fraction` pinning
  the rendered card text. Files: `crates/ui/src/state.rs`.

- [x] **F3 — "~$4 buys" — display ambiguity, NOT an exec bug.** Config
  `[risk.sizing] fixed_fraction = 0.10` on $100k → ~$10k clips; 4 bps taker
  fee × $10k = exactly $4.00. The agent_feed tape's rightmost USDT-suffixed
  column was the **fee**; qty showed BTC unlabeled. Added a **Notional**
  column (qty × price, USDT — derived in-widget, no backend change) so the
  ~$10k clip is visible and the ~$4 fee is unmistakably a fee. New string
  `TAPE_COL_NOTIONAL`; extended `tape_summary` panel snapshot to pin
  notional+fee. Files: `crates/ui/src/widgets/agent_feed.rs`,
  `crates/ui/src/strings.rs`, `crates/ui/tests/panel_snapshots.rs`.
  **Sizing is correct as configured — no agent/exec change.**

- [x] **F-TEST — self-verified** (ui-designer, 2026-06-10) — 437 lib tests
  pass (3 new); consistency/contrast/layout green; 4 panel snaps regenerated
  intentionally (2 live = "10.00%"; 2 agent_feed = notional+fee); the
  `live__recent_activity_with_chevron` visual triple regenerated (notional
  column); 52 other visual baselines + all `charts_screen_*` unchanged;
  new-code clippy/fmt clean; `verify_anchors.sh` 119/119. Tester to confirm
  the full integration suite + the operator's eyeball re-test recipe.

## Follow-up — v0.1.2 (developer, 2026-06-11)

Two issues surfaced from the operator's second live cockpit session (HEAD `3f9fd63`,
after the Notional-column fix landed).

- [x] **I1 — X-axis "now" not data time — FIXED via approach A (separate data-time x-coord).**
  **Landed 2026-06-11 (cockpit-live-equity-render-guard, completed + render-verified by the
  orchestrator after the implementing agent left it uncommitted & non-compiling).** The live
  equity curve now plots a SEPARATE data-time coordinate on its x-axis while the delivery
  guard keeps the wallclock key — so a fast replay shows real 2023-24 dates instead of one
  repeated wallclock minute.
  - `crates/core/src/views.rs` — new `PnlSnapshot.bar_ts: Option<Timestamp>`
    (`#[serde(default)]`, back-compat). `as_of` stays wallclock; `bar_ts` is the chart x-coord.
  - `crates/agent/src/runtime.rs` (`spawn_research_trading_loop`) + `reconciler.rs`
    (`after_bar_close`) — publish `as_of: Timestamp::now()`, `bar_ts: Some(bar.close_ts)`.
  - `crates/ui/src/state.rs` — new `live_equity_last_as_of` delivery-guard key (separate from
    the buffer); `push_live_equity_point(as_of, x_coord, equity)` keys the out-of-order guard
    on `as_of` but stores `x_coord = snap.bar_ts.unwrap_or(snap.as_of)` as the plotted ts,
    with a monotone clamp so `EquitySeries::from_points` can never error.
  - `crates/ui/src/widgets/equity_curve.rs` — **bonus fix the render harness caught:** guards
    the y-range denominator → fixes a `lyon_path: p.y.is_finite()` panic on a flat / 1-point
    curve (the cockpit was crashing on the FIRST live bar after boot).

  **Why the first attempt was reverted (history):** stamping `as_of: bar.close_ts` directly
  passed t903c but BROKE the live render — the buffer's monotone guard dropped the 2023
  bar-time points → empty curve ("no graph", operator-reported). Reverted in `40f5de9`.
  Approach A fixes it properly by keeping the two timestamps SEPARATE.

  **Render-verified (the lesson applied — verify at the rasterized layer, not unit tests):**
  - `crates/ui/tests/live_equity_render.rs` (NEW) — rasterizes the real Live screen via
    `iced_test::screenshot` and counts ACCENT polyline pixels; a self-proving pair
    (`harness_catches_dropped_points_empty_curve` + `healthy_curve_draws_far_more_than_broken`)
    proves it discriminates a broken curve from a healthy one. **5/5 green** (1198 ACCENT px,
    x-span ~947px on the healthy case).
  - `crates/ui/src/state.rs::live_equity_curve_plots_bar_ts_not_wallclock` (NEW unit test) —
    proves the buffer plots `bar_ts` (2023 data time), never the wallclock `as_of`.
  - `t903c_after_bar_close_publishes_pnl` updated: asserts `snap.bar_ts == Some(bar_ts)` and
    `snap.as_of >= bar_ts` (wallclock).
  - **Test cmd:** `cargo test -p ui --test live_equity_render && cargo test -p ui --lib live_equity && cargo test -p agent reconciler`
  - **Output:** all green.
  - **Orchestrator completion note:** the implementing agent updated only 4 of 10
    `PnlSnapshot` initializers (workspace didn't compile) and referenced a unit test it never
    wrote. The orchestrator fixed the 6 missing initializers, wrote the missing
    `live_equity_curve_plots_bar_ts_not_wallclock`, and confirmed the render harness runs green
    before landing. **Operator still owes the human-render confirmation** (run `cockpit_live`,
    confirm the x-axis shows 2023-24 dates) — see TODO.md.

- [x] **I2 — "$4 buys/sells" — CONFIRMED display/perception, not sizing bug.**
  Ran the headless agent with `RUST_LOG=agent=info` and captured real fills.
  Fill #1 (Buy): `price=16,680.94 USDT, qty=0.5996 BTC, notional=10,002.00 USDT,
  fee=4.0008 USDT, running_equity=99,995.99 USDT`. Config:
  `fixed_fraction=0.10 × $100k equity → ~$10k notional; 4bps taker × $10k = exactly $4.00 fee`.
  The Notional column added in `3f9fd63` (`agent_feed.rs:118`) computes
  `qty × price` correctly (= ~$10k USDT). The `$4` the operator sees is the
  **Fee** column, not Notional. **No sizing bug; no agent/exec change needed.**
  Added `tracing::debug!` logging with `notional_usdt` + `fee_usdt` per fill
  (always-on at debug level; info-level at first-5 + every-100) so the fill
  detail is surfaced for operators running with `RUST_LOG=agent=debug`.
  **For the ui-designer:** the Notional column is computing correctly (~$10k).
  The display prominence recommendation: the column order `Price | Qty | Notional | Fee`
  is already correct (Notional = big ~$10k number; Fee = small ~$4 number).
  If the operator is still seeing only $4, they may be looking at the Fee column
  or have a stale binary. No code change needed for Issue 2 beyond the debug log.

- [x] **I3 — Trades KPI always 0 → live session fill counter (TODO #2, 2026-06-11).**
  Added `Cockpit.live_fill_count: u64` (session-scoped, NOT serialized): incremented on
  every `Message::FillReceived` — including fills routed to the paused-tape buffer
  (pausing the display doesn't pause trading) — with an **in-place** `m.trades` update on
  a Ready strip (count is current the instant the fill lands, between per-bar rebuilds),
  and `trades: self.live_fill_count` in the per-bar KPI rebuild. Semantic: FILLS, not
  round-trips (honest for a monitor; round-trip pairing is exec-side, out of scope).
  - **file:line:** `crates/ui/src/state.rs` (field by `live_equity_last_as_of`;
    `FillReceived` arm; KPI rebuild in `push_live_equity_point`).
  - **Tests:** model layer `state.rs::live_trades_counter_counts_session_fills`
    (counts paused fills; in-place update; rebuild carries count) + render/summary layer
    `panel_snapshots.rs::live_kpi_trades_card_shows_session_fill_count` (asserts the
    rendered "card Trades: 3" text through the production `update` path). Full ui suite
    440 lib + 100 panel snapshots green; `live_equity_render` harness 5/5; cockpit-smoke
    PASS (`reports/cockpit-smoke-2026-06-11T19-05Z.log`).

- [x] **I4 — deprecated `Screen::Home` boot screen (TODO #4, 2026-06-11).**
  `crates/ui/src/bin/cockpit.rs:185` `Screen::Home` → `Screen::Live` (alias target,
  behavior-identical; removes the deprecation warning). Covered by the same
  cockpit-smoke PASS above.

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
