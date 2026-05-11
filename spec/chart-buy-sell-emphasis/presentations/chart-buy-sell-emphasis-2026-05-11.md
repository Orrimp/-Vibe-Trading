---
slug: chart-buy-sell-emphasis
mode: release
status: draft
audience: human-operator
updated: 2026-05-11
generated: 2026-05-11T23:30:00Z
supersedes: _none — first presenter fire on this feature_
---

# Chart buy/sell emphasis — release

## TL;DR

The cockpit Charts screen now answers your question — "did the strategy buy at the right time?" — at a glance: bigger green/red triangles riding the price line, a six-field hover tooltip, click-through to the audit ledger modal, three counter views (window-volume tile, open-position strip, per-bar volume histogram), all under the Layout-β reshape — 1000 tests pass, zero anchor drift, ghost-signal layer default-OFF.

## What changed

- **Markers are now visible and tell a story.** Buy and sell triangles grew from 6 px to 13 px, picked up a 1-px `BORDER_STRONG` outline plus a 1.5-px Lumen "whisper shadow", and now render **on top of** the price line (z-order flipped). Each marker's vertical anchor is the polyline at the marker's `x`, linearly interpolated between the two bracketing 1-minute bars — the marker rides the line across slope changes instead of jumping in step-function quanta. The fill's actual execution price moves to the tooltip.
- **Hover and click both work.** Pointer rests on a marker -> a six-field tooltip surfaces (`Side`, `Price`, `Quantity`, `Notional`, `Time`, `Strategy ID`). Click a marker -> the existing tape-row audit modal opens against the fill's `transaction_id` — same modal that ships from `tape-row-audit-modal`, no new widget.
- **Three counter views surround the chart.** Cumulative window-volume tile (`Buys in window` / `Sells in window` / `Net`) and an open-position mirror sit above the chart in a status strip; an 80-px per-bar volume histogram sits below. The chart canvas keeps the full middle width.
- **Ghost-signal layer scaffolded behind a default-off config gate.** A new `strategy_signals` audit table (migration `009`), a new `audit::query::recent_signals` reader, a new `core::SignalView` type, and a new `[signal_log] enabled = false` agent-config section ship together. With the gate off (the default), zero rows are written, zero ghost triangles render, audit-DB size is unchanged.
- **Cross-bin window chrome.** A shared `window_icon::standard_window_settings` helper sets a 1280×720 minimum window size + an embedded Lumen brand mark on cockpit, cockpit_live, and viewer. (See "Notes for the operator" §4 on the macOS dock-icon limitation.)
- **Zero strategy / risk / backtest / report / exec code changed.** All 9 strategy backtest anchors and both operator-success-report anchors are byte-identical (`spec/anchors.toml` is untouched across the entire arc).

## Why

Quoting `spec/chart-buy-sell-emphasis/feature.md` lines 11–43: this brief promotes verbatim operator feedback from the **2026-05-10 cockpit review** into a real feature. The operator opened the Charts screen shipped by Lumen Phase 2 and said *"I want to visually see (maybe green and red arrow) when the strategy is buying and when it is selling. Also the current amount of buy and sell. This will help me as human to determine if the strategy buys at the right time."* The Phase 2 chart already rendered triangles — but four breakages plus one missing feature were identified: triangles too small (`MARKER_SIZE_PX = 6.0`), hidden under the line (z-order wrong), floating off the line (marker y came from `fill.price.get()` instead of from the polyline at the marker's x), no way to inspect a marker (no tooltip, no click-through), and no surface for strategy intent before risk-clamping (no ghost layer). The fix is **pure UI plus one additive audit reader plus one additive config gate**; no strategy logic changes, no edge claim, no impact on the 11 locked anchors. This is the first feature against the Charts screen since Phase 2 shipped 2026-05-05.

**Terms-of-art** (one-line glosses, used throughout):

- **Z-order** — the layering order of overlapping canvas draw passes; later passes paint on top of earlier ones. The fix moves the marker-fill pass after the line-stroke pass.
- **Snap-to-line** — placing a marker's `y` at the polyline's `y` at the marker's `x` instead of at the marker's intrinsic `y` value (the fill price).
- **Ghost layer** — a faded, reduced-opacity render of *what the strategy wanted to do* (a signal before risk-clamping) painted under the bold render of *what actually happened* (an executed fill).
- **Hover tooltip** — a small overlay that appears when the pointer rests over a target; surfaces metadata without a click.
- **Hit-rect** — an invisible 28-px square interactive rectangle around the visible 13-px triangle (Fitts's-law forgiveness for small targets).
- **Body-SHA-256** — the deterministic body-only hash of a report. The 11 locked entries in `spec/anchors.toml` are the regression gate.
- **Audit ledger** — the `data/audit/ledger.db` SQLite file the agent writes every order, fill, risk veto, and strategy event into. All cockpit reads go through `audit::query::*`.
- **In-process mpsc** — *not used in this feature*. Mentioned here only because the rejected Q1 = (b) option would have added one. Q1 = (a) writes straight to the audit ledger instead — no new bus channel.
- **Atomic write** — a write that either fully completes or doesn't appear at all. The two new writer calls (`post_strategy_signal` + `update_signal_clamp_status`) use the existing `ledger.pool.begin() / commit()` pattern that `post_fill` already uses.

## What you can do now

| Action | Command |
|---|---|
| Launch the cockpit with paper-trading fixtures (Charts screen reachable via the sidebar) | `cargo run --release --bin cockpit --features fixtures` |
| Launch the live cockpit against a real ledger + bus | `cargo run --release --bin cockpit_live --features live` |
| Launch the backtest viewer (Charts screen NOT inherited — Q8 = (b) deferred) | `cargo run --release --bin viewer -- <report-dir>` |
| Run the full chart-tooltip test suite | `cargo test -p ui --test chart_tooltip_hover_fires` |
| Run the click-through-to-modal integration test | `cargo test -p ui --test chart_marker_click_opens_modal` |
| Run the new audit reader gate (V11) | `cargo test -p audit recent_signals` |
| Run the default-off config gate (V12) | `cargo test -p agent config_signal_log_default_off` |
| Verify the full anchor table (V8 — hard gate) | `bash scripts/verify_anchors.sh` |
| Turn on the ghost-signal layer in production (default is off — see "Notes for the operator" §2) | edit `agent.toml`: under `[signal_log]`, set `enabled = true` |

## How it works (one paragraph each)

**1. Marker visuals re-baselined.** In `crates/ui/src/widgets/chart.rs`, `MARKER_SIZE_PX` bumps from `6.0` to `13.0` and a new `GHOST_MARKER_SIZE_PX = 8.0` constant appears alongside. The `draw_triangle` helper grew two optional parameters — `outline: Option<Color>` and `shadow: Option<(iced::Vector, iced::Color)>` — so the new render path is: shadow pre-pass at `(0.0, 1.5)` offset using the `shadow_1` Lumen token, then the fill, then a 1-px `BORDER_STRONG` outline stroke. The `ChartProgram::draw` body re-orders its passes: gridlines -> axis labels -> line stroke -> ghost-signal triangles -> executed-fill triangles -> tooltip overlay. Markers now sit on top of the price line, distinctly larger and visually separable at default zoom.

**2. Marker y anchored by linear interpolation.** A new `snap_price_to_line(fill_ts: i64, bars: &[Bar]) -> Option<f32>` helper binary-searches for the two bars whose `close_ts` values bracket the fill's `venue_ts`, then linearly interpolates between their `close` prices. The interpolated price flows through the existing `y_for_price` projection; the original fill price stays only in the tooltip's **Price** field. `f32` linear interpolation with two `Decimal`-derived inputs is bitwise-deterministic on the same hardware + toolchain — no transcendentals, no SIMD reduction, no FMA optimization. V10 (two consecutive runs byte-identical) protects this.

**3. Hover detection via custom canvas pointer-tracking.** `ChartProgram::State` was promoted from `()` to a `ChartHoverState` struct holding `{ hovered_marker_idx: Option<ChartMarkerIndex>, hovered_marker_centroid: Option<iced::Point> }`. `canvas::Program::update` consumes `mouse::Event::CursorMoved` + `mouse::Event::ButtonPressed`, computes a 28-px hit-rect per marker centroid, and publishes `Message::ChartMarkerHovered(ChartMarkerIndex)` / `Message::ChartMarkerHoverEnded` on hit-rect transitions. `ChartMarkerIndex` is a tagged enum `Fill(usize) | Signal(usize)` so the tooltip can route to the right source list. Click on a fill marker re-uses the existing `Message::TapeRowClicked(transaction_id)` arm from `tape-row-audit-modal` — no new modal widget per the principles three-uses rule (this is the second consumer, not the third).

**4. Tooltip renders from canvas-local state only (M6.2 hardening).** The first ship had the tooltip read from `Cockpit.chart_tooltip` — a parent-state field that flips on the next `update` tick after the canvas state flips. Result: a one-frame race where `ChartProgram::draw` saw the new `hovered_marker_idx` but the old `tooltip: None`, so the tooltip "flashed and disappeared". T2033 decoupled the render: `ChartProgram::draw` Pass 6 now calls `self.tooltip_view_from_hover(idx)` directly off the canvas's own `(state.hovered_marker_idx, state.hovered_marker_centroid)`, building the view from `self.markers[idx]` / `self.signals[idx]`. Regression-guard test `chart_tooltip_view_built_from_canvas_state_without_round_trip` constructs the exact bug scenario (`tooltip: None`) and asserts the tooltip is still buildable from canvas state alone.

**5. Ghost-signal layer scaffolded (default-OFF).** A new SQLite migration `009_strategy_signals.sql` creates a dedicated `strategy_signals` table (separate from `strategy_events`, which is a lifecycle-event table). Two new writers `journal::post_strategy_signal` (signal-emit-time INSERT) and `journal::update_signal_clamp_status` (risk-decide-time UPDATE on the same `signal_id`) use the existing `ledger.pool.begin() / commit()` atomic-transaction pattern. A new reader `audit::query::recent_signals(ledger, venue, &symbol, since, until) -> Result<Vec<SignalView>, LedgerError>` mirrors `recent_fills_filtered`. A new `core::SignalView` type lives at `crates/core/src/views.rs` (sibling of `FillView`). A new `[signal_log] enabled = false` agent-config section (`crates/agent/src/config.rs::SignalLogConfig`) gates the writer — with the gate off, zero rows are written, the reader returns `Ok(vec![])`, and the cockpit renders zero ghost triangles. This feature ships the **writer + reader + core type + config gate + cockpit read path**. The live agent-runtime tap point that actually calls `post_strategy_signal` is a parallel-track follow-up; the cockpit goes live the moment that tap point lands, without further cockpit-side change.

**6. Counter views beside the chart (Layout β).** `crates/ui/src/screens/charts.rs` reshapes into a Column composition: chip row -> status strip (volume tile + open-position mirror) -> chart body (`Container` with `Length::Fill` width and height) -> per-bar volume histogram (label + 80-px `Length::Fixed` canvas). A new `widgets::volume_tile` widget renders the three-tile arithmetic (`buys_usdt + sells_usdt` summed over the visible window). A new `widgets::volume_histogram` widget renders the per-bar paired green/red stacked bars. The `widgets::positions` widget already shipped on Home; we filter it to the active symbol via the existing `model.positions` field — no new positions logic.

## How the Charts screen looks now

ASCII sketch of the Layout-β composition. The actual rendered cockpit (per `/tmp/cockpit-tester-T_FINAL.png` and the three `/tmp/cockpit-T2032-*` screenshots) follows this shape exactly:

```
┌──────────────────────────────────────────────────────────────────────────┐
│  binance · BTCUSDT    binance · ETHUSDT    binance · SOLUSDT             │  <- chip row
├──────────────────────────────────────────────────────────────────────────┤
│  Buys in window:  +8,000.20 USDT  (2 trades)    │  Open position         │  <- status strip
│  Sells in window: -8,000.40 USDT  (2 trades)    │  BTCUSDT 0.45 18,000   │     (volume tile
│  Net:              -0.20 USDT                   │  40,500 +225 USDT 1.2% │      + position
├──────────────────────────────────────────────────────────────────────────┤     mirror)
│                                                                          │
│                    ▼ (sell, near upper-left)                             │
│  40042 ──╮                                                               │
│          ╰╮       ▲ (buy)        ▼ (sell, near upper-right)              │  <- chart canvas
│           ╰─╮         ╰─╮              ╰╮                                │     (full width
│             ╰─╮          ╰─╮             ╰╮                              │      and height,
│               ╰─╮            ╰╮            ╰╮                            │      grows with
│                 ╰─╮            ╰╮            ╰╮                          │      window)
│  39000            ╰─╮            ╰╮            ╰─╮ ▼ (sell)              │
│                                                                          │
├──────────────────────────────────────────────────────────────────────────┤
│ Per-bar volume                                                           │  <- histogram
│ ▮ ▯ ▮ ▮ ▯ ▮ ▯ ▮ ▯ ▮ ▮ ▯ ▮ ▯ ▮ ▮ ▯ ▮ ▯ ▮ ▮ ▯ ▮ ▯ ▮ ▮ ▯ ▮ ▯ ▮              │     (fixed 80 px)
└──────────────────────────────────────────────────────────────────────────┘
```

The status strip's two halves are side-by-side at full chart widths; the chart canvas grows vertically with the window (the bug T2032 closed); the histogram is fixed-height at the bottom. Each triangle on the chart is now 13 px (was 6 px) with a 1-px `BORDER_STRONG` outline + 1.5-px whisper shadow. Hovering over any triangle surfaces the six-field tooltip; clicking opens the audit-journal modal against that fill's `transaction_id`. When the ghost-signal layer is enabled (`[signal_log] enabled = true` in `agent.toml`), 8-px ghost triangles at 60% alpha render *behind* the fill triangles — strategy intent before risk-clamping, visible alongside what actually executed.

## Draw-order pass sequence (z-order, locked by test)

`ChartProgram::draw` runs exactly six passes in this order. The pass list is asserted byte-stable by the `chart_summary` test helper's `draw_order:` line — any future drift trips V1 / V10:

1. **Gridlines** — light-grey horizontal + vertical reference lines inside the inner rect.
2. **Axis labels** — y-axis price labels at the gridline pitch; x-axis time labels at the bottom edge.
3. **Line stroke** — the `ACCENT`-colored 1-px polyline through the bar `close` prices.
4. **Ghost-signal triangles** — the 8-px `UP_400 / DOWN_400` filled triangles at 60% alpha. **Empty iterator** when `chart_signals: PanelState::Ready(vec![])` (the default-off config-gate path).
5. **Executed-fill triangles** — the 13-px `UP_500 / DOWN_500` filled triangles with `BORDER_STRONG` outline + whisper shadow. Anchored to the polyline via `snap_price_to_line` linear interpolation.
6. **Tooltip overlay** — final pass; renders only when `state.hovered_marker_idx.is_some()`. Built from canvas-local state (the T2033 fix) — `tooltip_view_from_hover(idx)` reads from `self.markers[idx]` or `self.signals[idx]` directly, never round-tripping through `Cockpit.chart_tooltip`.

Before this feature the order was passes 1 + 2 + (executed fills) + (line stroke) — the line literally covered the markers. After: the line is pass 3, markers are passes 4 + 5, tooltip is pass 6. The visual change is dramatic at the same canvas density.

## Live demo

Two real captures from this machine on 2026-05-11.

### Demo 1 — fresh anchor verification (V8 — hard gate)

```
$ bash scripts/verify_anchors.sh
PASS  btc-2023-1m-sma-cross                 fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
PASS  btc-2023-1m-sma-baseline-refresh      fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
PASS  btc-2023-1m-macd-trend                ef9c5e483fa079f670a7aa15671643fce3b39a5ce35df8cb6d797887053f8805
PASS  btc-2023-1m-rsi-reversion             bc56d20d608c680e534bf6764ce8e0e568f0d4ffdf847a539c53fef65170d7aa
PASS  btc-2023-1m-bbands-mean-revert        d8a08a23d3629556c5fca39d6af89d7e0f99418e642af0b86fce22ff4d2792e3
PASS  top10-2023-1h-momentum                3b60ef0743f006867b9e52f9de154869ee170987b27560e288b2d9597d3ecf97
PASS  top10-2024-h1-momentum                1f33534fc7c6af1c04330564bec77aac620ecf6f1058f11ff90dfb66adcf05c6
PASS  pairs-2023-zscore-mr                  90591a0ecc5d56c8ff93834b127a3780a31f51634f38f12c3c412391116abbd0
PASS  pairs-2024-h1-zscore-mr               14f50a598ba8343fc9be198a78716d036407d585c641c0b054eae6c062f1507f
PASS  report-sample-7d                      f4ef3d02300f9ac97108a5cd9ce4277d455a5438356ffe2d74f8cfbb4b8ba994
PASS  report-sample-90d                     463e19b298552d7e3e37b1aad7c786d1cc71f14eed75d7df7ea6dc57525fa33c
---
ANCHORS PASS  (11 / 11)
```

All 11 anchors green. Zero bytes of `spec/anchors.toml` changed across the full six-commit arc (`ff96ce4` -> `d809e44`). The anchor-neutrality contract holds.

### Demo 2 — live cockpit launch on this machine

```
$ cargo build --release --bin cockpit --features fixtures
   Compiling ui v0.1.0 (/Users/Vitaliy.Schreibmann/.../trading/crates/ui)
    Finished `release` profile [optimized] target(s) in 3.03s

$ ./target/release/cockpit > /tmp/cockpit-presenter.log 2>&1 &
$ sleep 5
$ screencapture -x /tmp/cockpit-presenter-charts.png
$ pkill -f "target/release/cockpit"
$ ls -la /tmp/cockpit-presenter-charts.png
-rw-r--r-- 1 Vitaliy.Schreibmann  wheel  2787807 May 11 23:14 /tmp/cockpit-presenter-charts.png
```

The fresh capture (`/tmp/cockpit-presenter-charts.png`, 2.79 MB) shows the cockpit booting on the Home screen (the default) at default window size with the four Home panels — PnL `90,129.50 USDT`, Open positions (BTC/ETH/SOL rows), Strategies, and Agent activity — all rendering cleanly. The Charts screen with markers + tile strip + histogram is shown in the screenshots block below (the tester captured it directly via a one-line `Screen::Home -> Screen::Charts` edit, which was reverted before the tester report landed — the `git diff` was empty post-revert).

## Screenshots

All paths absolute. Each capture was made via `screencapture -x` on this Mac after macOS Screen Recording permission was granted to the host IDE.

| Path | Bytes | What it shows |
|------|------:|---------------|
| `/tmp/cockpit-presenter-charts.png` | 2,787,807 | Fresh presenter capture, this session. Cockpit at default window size on the Home screen — PnL `90,129.50 USDT`, Open positions, Strategies, Agent activity all rendering. Confirms the binary boots clean. |
| `/tmp/cockpit-tester-T_FINAL.png` | 1,693,153 | Tester's T_FINAL capture (2026-05-11 23:01). Cockpit on the Charts screen at default 1280×720 with the binance·BTCUSDT chip active. Status strip: `Buys in window: +8,000.20 USDT (2 trades)  Sells in window: -8,000.40 USDT (2 trades)  Net: -0.20 USDT`. Open-position strip: `BTCUSDT 0.45 18,000.00 40,500.00 +225.00 USDT 1.20%`. **Four discrete fill-marker triangles** visible on the price line — resolves the orchestrator's open observation about marker count. Per-bar volume label visible below the chart. |
| `/tmp/cockpit-T2032-1280x720.png` | 2,640,155 | Layout-β floor. Chart canvas fills its allocation; no cropping. |
| `/tmp/cockpit-T2032-1600x900.png` | 2,516,673 | Mid window size. Chart grew with the body; Buy/Sell triangles visible. |
| `/tmp/cockpit-T2032-1920x1080.png` | 2,175,326 | Large viewport. Sidebar at canonical 180 px; chart fills horizontal + vertical between fixed siblings; multiple fill markers on the price line. |
| `/tmp/cockpit-T2033-no-hover.png` | 2,624,028 | Baseline at default 1280×720; both Buy and Sell fill-marker triangles rendered on the price line. Confirms the marker render path is healthy. |

**Tooltip-hover screenshot limitation acknowledged.** A live hover-state capture requires positioning the cursor over a marker; the macOS Accessibility TCC class (separate from Screen Recording) is not granted to this host process, so `cliclick` / AppleScript `set position of mouse` / Quartz event-inject paths all fail. The load-bearing T2033 evidence is the unit-test suite (`chart_tooltip_hover_fires` 6/6 ok + `chart_tooltip_view_built_from_canvas_state_without_round_trip` 1/1 ok) plus the inline source citation at `crates/ui/src/widgets/chart.rs` Pass 6 documenting the iced 0.14 async-update-queue race the M6.2 fix closed.

## Verification

| V-id | Description | Status | Evidence |
|---|---|---|---|
| V1 | Marker visual upgrade reflected in snapshot | VERIFIED | tester §3.2 `tests/panel_snapshots.rs` 68/68 ok (includes `charts_screen_with_counters_and_chart`, renamed from `chart__btc_with_two_buys_one_sell` per T2025) |
| V2 | Marker y snaps to the polyline (linear interpolation) | VERIFIED | tester §3.2 `chart_marker_y_snaps_to_line` in `unittests src/lib.rs` (`widgets::chart::tests`) |
| V3 | Tooltip surfaces on hover | VERIFIED | tester §3.2 `tests/chart_tooltip_hover_fires.rs` 6/6 ok + `tests/chart_tooltip_integration.rs` 1/1 ok |
| V4 | Click-through opens the journal-transaction modal | VERIFIED | tester §3.2 `tests/chart_marker_click_opens_modal.rs` 1/1 ok |
| V5 | Ghost-signal layer renders behind fills | VERIFIED | tester §3.2 `chart_renders_ghost_and_fill_layers` in `unittests src/lib.rs` |
| V6 | Counter-view tile arithmetic correctness | VERIFIED | tester §3.2 `chart_counter_tile_sums` in `unittests src/lib.rs` |
| V7 | Per-bar histogram + open-position mirror render | VERIFIED | tester §3.2 `charts_screen_with_counters_and_chart` in `tests/panel_snapshots.rs` |
| V8 | Anchor regression 11/11 PASS | VERIFIED | live demo 1 above: `ANCHORS PASS (11 / 11)`; tester §7; `git diff --stat HEAD~6 -- spec/anchors.toml` empty |
| V9 | Existing UI tests stay green | VERIFIED | tester §3.2 `cargo test -p ui` 228/0/0 across 21 binaries; §3.3 `cargo test -p ui --features live` 248/0/0; §3.1 workspace 1000/0/4 |
| V10 | Determinism — two consecutive runs byte-identical | VERIFIED | tester §8: both `report-sample-7d` SHA `f4ef3d02…` and `report-sample-90d` SHA `463e19b2…` identical across two consecutive runs |
| V11 | New audit reader unit-tested | VERIFIED | tester §3.4 `cargo test -p audit recent_signals` 5/5 ok — includes V11a (window subset), V11b (empty window), V11c (gate-off ledger), plus two bonus invariants (venue/symbol isolation; post-clamp UPDATE round-trip) |
| V12 | Config gate default-off behaviour | VERIFIED | tester §3.5 `config::tests::config_signal_log_default_off ... ok` |
| V13 | Consistency tests stay green | VERIFIED | tester §3.2 `tests/consistency.rs` 8/8 ok |

## Numbers that matter

- **Tests:** **1000 passed / 0 failed / 4 ignored** across 144 test binaries (tester §3.1). The 4 `#[ignore]` cases are pre-existing and unrelated to this feature.
- **UI crate alone:** **228 / 0 / 0** across 21 test binaries; **248 / 0 / 0** across the 22 binaries with the `live` feature on. Six tooltip-hover cases in the new `chart_tooltip_hover_fires.rs` integration test cover every transition state.
- **Audit crate `recent_signals`:** 5/5 ok (V11a + V11b + V11c + 2 bonus invariants).
- **Agent crate `config_signal_log_default_off`:** 1/1 ok (V12).
- **Anchors:** **11 / 11 PASS** (live demo 1). Zero bytes of `spec/anchors.toml` changed across the six-commit arc.
- **Net new tests added by this feature:** approximately **+57** since the pre-feature baseline. Net `+10` over the M6.2 hardening arc alone (990 -> 1000).
- **New tests by file (cumulative):**
  - `chart_tooltip_hover_fires.rs` — 6 tests (T2030 + T2033 — every hover state transition).
  - `chart_marker_click_opens_modal.rs` — 1 test (V4).
  - `chart_tooltip_integration.rs` — 1 test (V3).
  - `tests/recent_signals.rs` — 5 tests (V11).
  - `tests/migration_009.rs` — 2 tests (T2013 idempotency).
  - In-lib `unittests src/lib.rs` (ui crate) — `chart_marker_y_snaps_to_line`, `chart_renders_ghost_and_fill_layers`, `chart_counter_tile_sums`, `chart_canvas_height_grows_with_body_height`, `chart_tooltip_view_built_from_canvas_state_without_round_trip`, `min_window_size_set_on_all_bins`, `window_icon_set_on_all_bins`, `chart_state_tracks_hovered_marker_idx`, `chart_draw_triangle_outline_and_shadow` + tooltip / strings tests.
  - In-lib audit `journal::tests` — `post_strategy_signal_writes_row`, `update_signal_clamp_status_flips_field`, `post_strategy_signal_skips_hold_kind`, `post_strategy_signal_persists_intended_price`.
- **Live wall-clock** for the workspace test suite (debug build): tester §3.1 shows individual binaries finishing in 0.00s to 0.30s; the dominant contributor is the reports / reflection scenario suites.
- **Cockpit binary cold-boot to first frame:** approximately 5 seconds against the fixtures feature on this Mac (the `sleep 5` between launch + `screencapture` succeeded cleanly).
- **LLM cost incurred by this feature:** **$0.00** (zero tokens, zero new bus channels, zero new LLM provider dependency).

## Notes for the operator

Seven facts a non-engineer operator should weigh in on or be aware of before approving the ship.

1. **The feature directly answers your stated need.** The 2026-05-10 ask was *"did the strategy buy at the right time?"* — the markers are now visible (bigger, outlined, shadowed), anchored to the price line (so timing reads off the x-axis cleanly), with a six-field hover tooltip that surfaces side / price / quantity / notional / time / strategy and a click-through to the existing audit-journal modal. The four ways the Phase 2 chart was visually broken are closed; the missing fifth piece (no way to inspect a marker) is filled by the tooltip + click-through pair.

2. **Q1 = (a) signal source — additive `strategy_signals` audit table, default-off.** The architect picked the audit-ledger source over the in-memory bus-channel option (which would have violated the "no new bus channel" hard constraint) and the replay-from-backtest option (which defeated the live-monitoring use case). The new `[signal_log] enabled = false` config gate is the load-bearing operator-facing decision: with the gate off (the default), zero new rows are written and the audit-DB growth budget is $0. With the gate on, audit-DB grows approximately 8 MiB / month at 4 strategies × 60 bars/hour × 24 hours × 30 days. The default is different from reflection-memory's `enable_writer = true` (operator-approved 2026-05-10) because the audit-DB growth budget here is meaningful — operators should opt in explicitly only after they want the ghost-marker layer live. The ghost-marker layer renders zero triangles until the gate flips.

3. **Three counter views all shipped (R7), Layout β kept the chart full-width.** Cumulative-window-volume tile (`Buys in window` / `Sells in window` / `Net`) and an open-position mirror sit above the chart in the status strip; the per-bar volume histogram sits below the chart at a fixed 80 px height. The chart canvas takes the full middle width and grows vertically with the window. Tester's screenshots at 1280×720 / 1600×900 / 1920×1080 verify the layout holds across the canonical size matrix.

4. **App-icon limitation documented honestly.** `iced::window::Settings::icon` does NOT drive the macOS dock / cmd-tab / Spotlight icons. `winit::Window::set_window_icon` is documented as unsupported on macOS. The fix needs `.app` bundle packaging (`cargo bundle` / `Info.plist` / `.icns` asset) — captured as a candidate follow-up brief at `spec/cockpit-app-bundle/feature.md` with seven open questions for the analyst. The iced-level icon plumbing IS correct (test `window_icon_set_on_all_bins` proves the RGBA blob loads cleanly across all three bins); the title-bar icon may render — but the dock / cmd-tab / Spotlight icons do not. This is a macOS limitation against a bare `cargo run` binary, not a code defect.

5. **Minimum window size: 1280×720 (Layout-β floor).** Operator can't shrink any of the three bins (cockpit / cockpit_live / viewer) below this size. Shared helper `window_icon::standard_window_settings` enforces this consistently — test `min_window_size_set_on_all_bins` proves all three bins honor the floor.

6. **Anchor neutrality: zero `spec/anchors.toml` changes.** All 9 strategy backtest anchors at `spec/anchors.toml:15-58` and both operator-success-report anchors at lines 67–75 are byte-identical to their pre-feature SHAs. Tester §9 confirms `git diff --stat HEAD~6 -- crates/strategy crates/risk crates/backtest crates/reports crates/exec` returns empty output — zero modifications to the five anchor-protected crates across the six-commit arc. The R9.4 negative invariant holds.

7. **Three implementation arcs, transparently logged.** This feature took longer than a typical single-pass UI ship because the headless agent cannot visually verify its own work — three iteration cycles were needed:
   - **Arc 1 (`ff96ce4`):** Initial developer + ui-designer parallel pass — T2001–T2027 (27 tasks across M1–M5).
   - **Arc 2 (`9bb5786`):** M6 follow-up after the **first operator visual verification** found three runtime gaps (tooltip-not-firing despite passing tests, missing min-window-size, missing app icon). T2028–T2030 closed them.
   - **Arc 3 (`f89f850` + `5f15696` + `d809e44`):** M6.2 + hardening after the **second operator visual verification** found three more (app-icon-still-not-showing on macOS, chart-crops-on-resize, tooltip-flashes-and-disappears). T2031–T2033 closed them with source-level research + screenshot evidence. The macOS Screen Recording TCC permission was granted to the host IDE in this final pass — the screenshot-verification gate (now mandatory for any UI follow-up) closes the headless-agent visual-verification gap going forward.

## Open decisions

_no decisions pending — feature is ready to ship_. The Q-resolution map below documents the 10 closed questions: Q1 / Q2 / Q3 / Q6 / Q7 / Q9 by the architect; Q4 / Q5 / Q8 by the operator. The new `[signal_log] enabled = false` default is the analyst-recommended + architect-confirmed safer default for this feature (the audit-DB growth cost only lands when the operator opts in — different from reflection-memory because the cost is higher and the default consumer here is the ghost-marker layer, which is itself opt-in).

## Q-resolution map

The brief opened with nine architect/operator questions plus Q10 ("anything else?"). All ten are resolved.

| Q | Topic | Resolution | Resolver |
|----|-------|------------|----------|
| Q1 | Signal source plumbing | **Option (a)** — additive `strategy_signals` audit table + config gate. New migration `009_strategy_signals.sql`, new writers `journal::post_strategy_signal` + `update_signal_clamp_status`, new reader `audit::query::recent_signals`, default `enabled = false`. | Architect |
| Q2 | Marker y-snap method | **Option (b)** — linear interpolation between bracketing bars' closes. New helper `snap_price_to_line(fill_ts, &[Bar]) -> Option<f32>`. | Architect |
| Q3 | Tooltip implementation in iced canvas | **Option (b)** — custom canvas pointer-tracking + custom-drawn tooltip overlay. `ChartProgram::State` promoted from `()` to `ChartHoverState`; 28-px hit-rect; tooltip is the final draw pass. | Architect |
| Q4 | Tooltip content fields | **Six fields**: Side (Buy/Sell badge), Price (USDT 4 dp), Quantity (base 4 dp), Notional (USDT 2 dp), Timestamp (RFC3339 UTC), Strategy ID (or `—`). **Truncated transaction ID dropped** (full UUID is one click away via R4.5). | Operator (2026-05-10) |
| Q5 | Layout for the three counter views | **Layout (β)** — chart keeps full width; volume tile + position mirror above in a status strip; 80-px volume histogram below. | Operator (2026-05-10) |
| Q6 | Marker contrast / outline / drop shadow | **13-px filled triangle + 1-px `BORDER_STRONG` outline + 1.5-px `shadow_1` whisper shadow.** Ghost layer = 8-px `UP_400 / DOWN_400` at 60% alpha, no outline, no shadow. | Architect |
| Q7 | Per-bar histogram widget shape | **Option (b)** — new `widgets::volume_histogram` (separate widget; no ripple into the `equity_curve` consumers of `widgets::sparkline`). | Architect |
| Q8 | Backtest-viewer parity | **Option (b)** — cockpit-only this round. Viewer parity is a follow-up brief named `viewer-charts-parity`. | Operator (2026-05-10) |
| Q9 | `SignalView` type home + shape | **`crates/core/src/views.rs`** (sibling of `FillView`). Shape: `{ signal_id: SmolStr, symbol: Symbol, side: Side, intended_qty: Quantity, signal_ts: Timestamp, strategy_id: StrategyId, was_clamped: bool, clamp_reason: Option<SmolStr> }`. `intended_price` added forward-compat for v2 limit-order shapes. | Architect |
| Q10 | Anything else? | **Three deferrals**: marker keyboard navigation (defer to a Phase 5-style focus-ring extension); symbol-switch animation (defer; abrupt switch matches Phase 2's existing UX); multi-strategy colour-coding for the ghost layer (defer; per-side colouring matches the fill layer and reads consistently). | Analyst (deferrals) |

## Implementation surface (where to look in the codebase)

| Area | Path | What landed |
|------|------|-------------|
| Audit migration | `crates/audit/migrations/009_strategy_signals.sql` | New `strategy_signals` table + 3 indices. Idempotent `CREATE TABLE IF NOT EXISTS`. |
| Audit writer | `crates/audit/src/journal.rs:259-369` | `pub async fn post_strategy_signal(...)` (extended signature carries `intended_qty: Quantity` + `intended_price: Option<Price>` — see tasks.md T2014 deviation note). |
| Audit writer (clamp UPDATE) | `crates/audit/src/journal.rs:385-417` | `pub async fn update_signal_clamp_status(ledger, signal_id, was_clamped, clamp_reason) -> Result<(), LedgerError>` |
| Audit reader | `crates/audit/src/query.rs` | `pub async fn recent_signals(ledger, venue, &symbol, since, until) -> Result<Vec<SignalView>, LedgerError>` — mirrors `recent_fills_filtered` shape. |
| Core type | `crates/core/src/views.rs:138-187` | `SignalView` struct + serde round-trip tests at `:189-235`; re-exported from `crates/core/src/lib.rs:53-55`. |
| Agent config | `crates/agent/src/config.rs` | New `pub struct SignalLogConfig { pub enabled: bool }` with `#[serde(default)] enabled: false` — wired to `[signal_log]` TOML section. |
| Chart widget | `crates/ui/src/widgets/chart.rs` | `MARKER_SIZE_PX = 13.0` (`:47`), `GHOST_MARKER_SIZE_PX = 8.0` (`:51`), `draw_triangle` extended with `outline` + `shadow` params, new `snap_price_to_line` helper, `ChartProgram::State` promoted to `ChartHoverState`, `canvas::Program::update` consumes pointer events, `marker_hit_rect` 28-px helper, `tooltip_view_from_hover` canvas-local builder (`:373`), Pass-6 tooltip render decouple (`:347-353`). |
| Tooltip widget | `crates/ui/src/widgets/chart_tooltip.rs` | New widget — `draw_tooltip(frame, bounds, anchor, view, mode)`. T2033 left a vestigial `ChartProgram::tooltip` field with `#[allow(dead_code)]` to preserve the public `chart::view` signature. |
| Volume histogram widget | `crates/ui/src/widgets/volume_histogram.rs` | New widget — paired green/red stacked bars per bar at fixed 80-px height. |
| Volume tile widget | `crates/ui/src/widgets/volume_tile.rs` | New widget — three-tile compose of `Buys / Sells / Net`. |
| Charts screen | `crates/ui/src/screens/charts.rs:157-189` | Column composition reshape (Layout β): chip row -> status strip -> chart-body `Container` (`.width(Length::Fill).height(Length::Fill)` per T2032) -> histogram. |
| Shared window chrome | `crates/ui/src/window_icon.rs` | New module — `standard_window_settings()` helper. Embeds the Lumen brand mark (16384-byte RGBA blob) + 1280×720 min-size floor. Documents the macOS dock-icon limitation in a module-level section. |
| Brand mark asset | `crates/ui/assets/lumen-mark-64x64.rgba` | New 16384-byte raw RGBA blob — embedded via `include_bytes!`. Pre-rasterised once out-of-tree (not a workspace dep). |
| Cockpit bin | `crates/ui/src/bin/cockpit.rs` | Calls `.window(window_icon::standard_window_settings())`. |
| Cockpit live bin | `crates/ui/src/bin/cockpit_live.rs` | Same. Plus the `Task::perform` shim for the recent_signals fetch at `:651-664`. |
| Viewer bin | `crates/ui/src/bin/viewer.rs` | Same window-chrome helper call. |
| Strings | `crates/ui/src/strings.rs` | 9 new `CHART_TOOLTIP_*` constants + 7 new `CHART_VOLUME_*` / `CHART_POSITION_*` constants. All registered in `all_strings_present` test. |
| Cockpit follow-up brief | `spec/cockpit-app-bundle/feature.md` | New stub brief — candidate status, owner pending-analyst. Seven open questions for the `.app` bundle / macOS dock icon work. |

**Files explicitly NOT modified** (R9.4 negative invariant, all verified by `git diff --stat HEAD~6`):

- `crates/strategy/` — zero changes (no `Strategy` trait change, no strategy implementations touched).
- `crates/risk/` — zero changes.
- `crates/backtest/` — zero changes.
- `crates/reports/` — zero changes.
- `crates/exec/` — zero changes.
- `crates/agent/src/bus.rs` — zero changes (no new bus channel).
- `spec/anchors.toml` — zero changes.

## Test coverage breakdown

New + extended test files across the chart-buy-sell-emphasis arc:

| Test file | Crate | What it covers | R / V item | Result |
|-----------|-------|----------------|-----------|--------|
| `tests/chart_tooltip_hover_fires.rs` | ui | 6 hover state transitions: cursor-on-marker fires `Hovered(Fill/Signal(idx)) + Captured`; cursor-off fires nothing; hover-then-leave fires `HoverEnded`; ghost-marker hover; cursor-leaves-canvas-while-hovering; idempotent dispatch | R4.1, R4.3, R4.6 | 6/6 ok |
| `tests/chart_marker_click_opens_modal.rs` | ui | End-to-end click -> `cockpit.tape_audit_modal == Some(PanelState::Ready(view))` with matching `transaction_id` | R4.5, V4 | 1/1 ok |
| `tests/chart_tooltip_integration.rs` | ui | Synthetic `CursorMoved` -> `cockpit.chart_tooltip == Some(view)` with expected fields | R4.1, V3 | 1/1 ok |
| `tests/chart_markers_from_audit_query.rs` | ui | Cockpit chart-markers fetch shim against the audit query | R5.4 | 2/2 ok |
| `tests/recent_signals.rs` | audit | V11a window-subset, V11b empty-window, V11c gate-off, plus venue/symbol isolation + post-UPDATE round-trip | V11 (R5.3, R5.7) | 5/5 ok |
| `tests/migration_009.rs` | audit | `migrations_apply_clean` + `migration_009_is_idempotent` | T2013 | 2/2 ok |
| `chart_marker_y_snaps_to_line` (lib unit) | ui | Fill at midpoint ts between two bars whose closes differ by `dec!(100)` -> rendered `y` is interpolated midpoint ± 0.5 px | R3.1, R3.2, V2 | ok |
| `chart_renders_ghost_and_fill_layers` (lib unit) | ui | Fixture with 2 `SignalView`s + 1 `FillView` at overlapping bars; chart-summary asserts `ghost_count: 2`, `fill_count: 1`, `draw_order: gridlines,labels,line,ghosts,fills` | R5, V5 | ok |
| `chart_counter_tile_sums` (lib unit) | ui | 3 buys (total $30k) + 2 sells (total $20k) -> `Buys: +$30,000.00 (3) / Sells: -$20,000.00 (2) / Net: +$10,000.00` | R7.1, V6 | ok |
| `chart_canvas_height_grows_with_body_height` (lib unit) | ui | Pure Layout-β arithmetic helper; asserts `h_1080 > h_720` with exact 360-px delta | R8.3, T2032 | ok |
| `chart_tooltip_view_built_from_canvas_state_without_round_trip` (lib unit) | ui | `ChartProgram { tooltip: None, .. }` (bug scenario) -> `tooltip_view_from_hover(Fill(0))` + `(Signal(0))` both return `Some(view)`; out-of-bounds returns `None` | T2033 | ok |
| `min_window_size_set_on_all_bins` (lib unit) | ui | All three bins set `min_size = Some(_)` via the shared helper | T2028 | ok |
| `window_icon_set_on_all_bins` (lib unit) | ui | Lumen mark blob is 16384 bytes (64×64 RGBA); `iced::window::icon::from_rgba` returns `Some(Icon)` | T2029 | ok |
| `chart_state_tracks_hovered_marker_idx` (lib unit) | ui | Synthetic `CursorMoved` -> hit-rect transition from `None` to `Some(Fill(0))` and back | T2008 | ok |
| `chart_draw_triangle_outline_and_shadow` (lib unit) | ui | `draw_triangle` helper returns expected `(Vector{0.0, 1.5}, shadow_1.color)` for dark + light modes; zero `#hex` literals | T2002, R1.2 | ok |
| `post_strategy_signal_writes_row` (lib unit) | audit | Row count 0 -> 1 after `post_strategy_signal` | T2014 | ok |
| `update_signal_clamp_status_flips_field` (lib unit) | audit | Single `UPDATE` flips `was_clamped` + `clamp_reason` on the same `signal_id` | T2014 | ok |
| `post_strategy_signal_skips_hold_kind` (lib unit) | audit | `SignalKind::Hold` is not persisted (only `Buy` / `Sell` create rows) | T2014 | ok |
| `post_strategy_signal_persists_intended_price` (lib unit) | audit | Forward-compat `intended_price: Option<Price>` round-trips (v2 limit-order shape) | T2014, Q9 | ok |
| `config::tests::config_signal_log_default_off` | agent | TOML without `enable_signal_log` parses to `false`; TOML with `enable_signal_log = true` parses to `true` | V12 | ok |
| `tests/consistency.rs` | ui | `no_inline_hex_colors_in_widgets_or_state` + `no_inline_user_visible_strings_in_widgets` + 6 more | V13 | 8/8 ok |
| `tests/panel_snapshots.rs` | ui | 68 panel snapshots, includes the renamed `charts_screen_with_counters_and_chart` + chip-row active variants | V1, V7 | 68/68 ok |

## Anchor table (first 8 chars per locked body-SHA — same as pre-feature)

| Scenario | Version | SHA-256 prefix |
|---|---|---|
| btc-2023-1m-sma-cross | v0 | `fc2e3b4a…` |
| btc-2023-1m-sma-baseline-refresh | v0 | `fc2e3b4a…` |
| btc-2023-1m-macd-trend | v0.5 | `ef9c5e48…` |
| btc-2023-1m-rsi-reversion | v0.5 | `bc56d20d…` |
| btc-2023-1m-bbands-mean-revert | v0.5 | `d8a08a23…` |
| top10-2023-1h-momentum | v1 | `3b60ef07…` |
| top10-2024-h1-momentum | v1 | `1f33534f…` |
| pairs-2023-zscore-mr | v1.5a | `90591a0e…` |
| pairs-2024-h1-zscore-mr | v1.5a | `14f50a59…` |
| report-sample-7d | v1+ | `f4ef3d02…` |
| report-sample-90d | v1+ | `463e19b2…` |

All 11 entries byte-identical to their pre-feature SHAs (`git diff --stat HEAD~6 -- spec/anchors.toml` empty). Feature is anchor-neutral by design — pure UI + additive audit reader/writer + config gate.

## Decision history (2026-05-10 to 2026-05-11)

This feature's brief, design, implementation, three hardening passes, test verdict, and presentation arc spans two calendar days. The resolution sequence:

1. **Brief opened** by the analyst (2026-05-10) after the operator's verbal cockpit-review feedback. Twelve R-items, ten open questions (Q1–Q10), three operator-decide (Q4 / Q5 / Q8). Promoted from `spec/backlog.md -> Active`.
2. **Operator answered Q4, Q5, Q8** via orchestrator chat (2026-05-10): six tooltip fields (truncated tx-id dropped), Layout (β), backtest-viewer parity deferred to a follow-up brief.
3. **Architect answered Q1, Q2, Q3, Q6, Q7, Q9** in the same Design slice (2026-05-10): additive `strategy_signals` table with default-off config gate, linear interpolation, custom canvas pointer-tracking, 13-px triangle + outline + whisper shadow, new `volume_histogram` widget, `SignalView` at `core::views`.
4. **Architect expanded tasks.md** (2026-05-10) with 27 developer tasks T2001–T2027 + `T_FINAL_CHART_BUY_SELL_EMPHASIS`. Owner tags: `[D]` developer, `[U]` ui-designer, `[D+U]` co-owned.
5. **Developer + ui-designer landed T2001–T2027 in parallel** (2026-05-11) in commit `ff96ce4`. Per-task footers cite file:line + acceptance command. Three insta snapshots re-baselined via `cargo insta accept`. `cargo test --workspace` 990 passed; anchors 11/11.
6. **Operator's first visual verification** (2026-05-11, after `ff96ce4`) surfaced three runtime gaps: tooltip not firing despite passing tests; missing min-window-size; missing app icon. Routed back as M6 (`2233f73` + `9bb5786`) — T2028 + T2029 + T2030 closed them. Net new tests +9.
7. **Operator's second visual verification** (2026-05-11, after `9bb5786`) surfaced three more: macOS dock icon still not showing; chart crops on resize; tooltip flashes and disappears. Routed back as M6.2 (`da021cb` + `f89f850`) — T2031 + T2032 + T2033 closed them. M6.2 introduced the **mandatory screenshot-verification gate** for any UI follow-up (the headless agent must `screencapture -x` and Read the screenshot before marking a UI task done).
8. **Hardening pass** (2026-05-11, `5f15696`) corrected the T2032 doc rationale (Row/Column Shrink-default trap, not Container's) and landed four screenshot evidence files at 1280×720 / 1600×900 / 1920×1080 / no-hover, after macOS Screen Recording permission was granted to the host IDE. M6.2 orchestrator-side closeout filed at `spec/chart-buy-sell-emphasis/reports/m6.2-hardening-2026-05-11.md` (`d809e44`).
9. **Tester ran the 13-gate V-pass** (2026-05-11 21:03 UTC) at `spec/chart-buy-sell-emphasis/reports/test-2026-05-11-2103-chart-buy-sell-emphasis-final.md` against commit `d809e44`. 1000 / 0 / 4 across the workspace; 228 / 0 / 0 across `cargo test -p ui`; 248 / 0 / 0 across `cargo test -p ui --features live`; V11 5/5; V12 1/1; V8 11/11. Six honest-tick spot checks all held. The orchestrator's open observation about marker count (only 2-of-4 visible in the 1920×1080 capture) was resolved by a fresh 1280×720 capture (`/tmp/cockpit-tester-T_FINAL.png`) showing all 4 markers — the prior was a capture-scale artifact. **VERDICT -> PASS at 21:03 UTC** (commit `b0cc4a5`).
10. **Presenter (this deck) ran end-to-end** (2026-05-11) — fresh `verify_anchors.sh` PASS, fresh cockpit boot + screenshot, surfaces the seven operator-relevant facts. Mechanical `check_presentation.sh` gate confirmed approval boxes UN-ticked.

The full arc spans 33 + T_FINAL ticked tasks across 6 commits, 3 visual-verification cycles, and 1 closeout hardening pass.

## What approval means / cost of "yes" vs "no"

A clean **"Approved — ship"** ratifies the feature as it stands: chart visuals upgraded, hover + click work, three counter views shipped, ghost-signal layer scaffolded behind a default-off config gate, app icon documented honestly (macOS dock-icon limitation captured in the `cockpit-app-bundle` follow-up brief stub), 1000 / 0 / 4 test count locked, 11 / 11 anchors locked, zero anchor-relock cost. After approval the operator can launch any of the three bins and exercise the live Charts screen — tooltip hover, marker click, symbol switch, window resize — against the fixtures or live ledger. The ghost-signal layer activates only when the operator flips `[signal_log] enabled = true` in `agent.toml` and a separate parallel-track follow-up wires the live agent-runtime tap point.

**"Approve with notes"** routes the deck back to the orchestrator with the override appended to the Notes block. Examples of likely notes: flip `SignalLogConfig::enabled` default to `true` (analogous to reflection-memory's operator override), bump the min-window-size floor away from 1280×720, drop the click-through to keep hover as the only interaction. Cost depends on the note — a default flip is a one-file follow-up patch with no anchor re-lock cost; a UX-shape change routes back to ui-designer.

**"Reject"** routes the entire feature back to the analyst with the rejection reason. Cost: substantial — re-scopes the brief, the architect's design, the developer + ui-designer's 27 + 6 task implementation across three arcs, the tester's 13-gate V-pass. The 1000 / 0 / 4 test count and the 11 / 11 anchor PASS are not in dispute; this would only be the right call if the operator believes the *shape* of v1.9.0 (Q1 = (a), Q5 = (β), Q8 = (b)) is wrong rather than the implementation. The three operator-decide resolutions (Q4 / Q5 / Q8) are the load-bearing scope decisions; rejecting one of those three is what would justify the cost.

## Risk register (excerpted from `feature.md`, all mitigated)

| Risk | Severity | How it is mitigated in v1.9.0 |
|------|----------|------------------------------|
| Strategy / backtest body-bytes shift via accidental code modification | high | R9.4 negative invariant — `git diff --stat HEAD~6 -- crates/strategy crates/risk crates/backtest crates/reports crates/exec` empty. Anchors 11/11 PASS. |
| Tooltip race ("flash and disappear") via parent-state round-trip | high | T2033 decoupled tooltip render — Pass 6 reads from canvas-local state via `tooltip_view_from_hover`. Regression test `chart_tooltip_view_built_from_canvas_state_without_round_trip` pins this. |
| Audit-DB growth blow-up if ghost layer accidentally default-on | medium | `SignalLogConfig::enabled` defaults `false`. V12 unit test guards the default. Reader returns `Ok(vec![])` for gate-off ledgers — no special-case branch. |
| Determinism leak via f32 non-determinism in `snap_price_to_line` | medium | f32 linear interpolation with two Decimal-derived inputs is bitwise-deterministic on same hardware + toolchain. V10 (two consecutive runs byte-identical) protects this. |
| Hit-rect math too small or too large at non-default cockpit zoom | medium | 28-px hit-rect per Fitts's-law forgiveness on a 13-px target. Unit tests pin transitions at the default density; tester ran the layout-β snapshot across 1280×720 / 1600×900 / 1920×1080. |
| Lumen brand-mark asset bloats binary | low | 16384-byte raw RGBA (64×64) — embedded via `include_bytes!`. Pre-rasterised once out-of-tree (not a workspace dep on `resvg` / `tiny-skia`). |
| macOS dock icon missing surprises operator post-ship | low | Documented honestly in module-level doc at `crates/ui/src/window_icon.rs` + a candidate follow-up brief stub at `spec/cockpit-app-bundle/feature.md`. The title-bar icon may still render (iced-level plumbing IS correct); the dock / cmd-tab / Spotlight icons require `.app` bundle packaging. |
| Ghost-layer rendering invariant breaks if a future strategy emits a `Hold` signal | low | `post_strategy_signal_skips_hold_kind` test pins the invariant — only `Buy` / `Sell` signal kinds persist. Hold is a non-event. |

## Follow-up briefs (forward look — not in this ship)

Three follow-ups are queued by this feature; each is independent and lands when its dependencies exist:

1. **Live agent-runtime tap point for `post_strategy_signal`.** This feature ships the writer + reader + config gate + cockpit read path. The live runtime tap point that actually calls `journal::post_strategy_signal` from the agent's per-bar loop, between the strategy registry's `on_bar` return and the risk engine's consume call, is a parallel-track follow-up brief (no slug yet — to be filed after the agent-runtime track architect picks scope). The backtest binary already invokes `registry.on_bar`; the live runtime does not yet. When the live tap point lands, the ghost-marker layer comes alive without any further cockpit-side change.

2. **`cockpit-app-bundle`** (`spec/cockpit-app-bundle/feature.md` — candidate, owner pending-analyst). `.app` bundle packaging for the macOS dock icon. Seven open questions for the analyst: `cargo bundle` vs raw `Info.plist`; `.icns` vs PNG-set; CI integration; signing + notarization (deferred — needs an Apple Developer ID); cross-bin scoping (one bundle per bin or one parent app); operator distribution path (DMG / Homebrew / direct download); Linux + Windows parity. Carries the dock / cmd-tab / Spotlight icon fix.

3. **`viewer-charts-parity`** (no file yet — captured as an operator-confirmed deferral in Q8). Extends the viewer binary with the same Charts-screen composition (price chart + markers + tooltip + click-through). Significant scope: viewer currently renders KPI strip + equity curve + drawdown band (no price chart). Cockpit-only this round per operator's pick.

## Honest-tick spot-check matrix (from tester report §10)

Six ticks across M1, M3, M3-shim, M4, M6.2, M6.2 — each verified by file:line citation + acceptance-command re-pass. All six held:

| Tick | Milestone | Claim | File:line | Acceptance |
|------|-----------|-------|-----------|------------|
| T2001 | M1 marker visuals | `MARKER_SIZE_PX = 13.0`, `GHOST_MARKER_SIZE_PX = 8.0` | `crates/ui/src/widgets/chart.rs:47, :51` | `cargo test -p ui --test panel_snapshots charts_screen_with_counters_and_chart` 68/0/0 ok |
| T2014 | M3 audit writer | `post_strategy_signal` + `update_signal_clamp_status` atomic two-write pattern | `crates/audit/src/journal.rs:259-369, :385-417` | `cargo test -p audit recent_signals` 5/5 ok including `recent_signals_reflects_post_update_clamp_status` |
| T2017 | M3 cockpit shim | `iced::Task::perform` parallel fetch of `recent_signals` | `crates/ui/src/bin/cockpit_live.rs:651-664` | `cargo test -p ui --features live` 248/0/0 ok |
| T2025 | M4 Layout β | Column composition with chart-body Container Fill/Fill + 80-px histogram | `crates/ui/src/screens/charts.rs:157-189` | panel_snapshots ok |
| T2032 | M6.2 chart scaling | `.width(Length::Fill)` on chart-body Container | `crates/ui/src/screens/charts.rs:227-229` | `chart_canvas_height_grows_with_body_height` 1/1 ok |
| T2033 | M6.2 tooltip decouple | Pass 6 reads canvas-local hover state via `tooltip_view_from_hover` | `crates/ui/src/widgets/chart.rs:347-353, :373` | `chart_tooltip_view_built_from_canvas_state_without_round_trip` 1/1 ok |

## Approval

- [x] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback

**2026-05-11 (operator, verbal approval via orchestrator chat):**

Approved clean — no overrides, no follow-up notes. All seven operator-relevant facts accepted as designed:

- Q1 = Option (a) audit-table signal source ships **default-OFF**. Operator opts in via `agent.toml` when ready for the ghost-marker layer to populate. Different from reflection-memory's v1.8 ship (which flipped `enable_writer` to default-on); the audit-DB growth budget (~8 MiB/month at 4 strategies × 1m cadence) drove the conservative choice.
- macOS `.app` bundle for the dock icon is **deferred** per the candidate stub at [`spec/cockpit-app-bundle/feature.md`](../../cockpit-app-bundle/feature.md). The iced-level icon plumbing is correct; macOS just doesn't honor it for a bare `cargo run` binary. Operator picks up the bundling task when the cockpit ship target firms up.
- Three implementation arcs (initial dev+ui-designer parallel + M6 + M6.2 + hardening) are reflected honestly in the deck. The multi-cycle pattern was the headless agent's inability to visually verify; Screen Recording permission grant + the documented screenshot-verification gate close that long-term.

No other notes. Deck approved as `[x] Approved — ship`.

## Changelog

- 2026-05-11 (presenter): initial draft after tester `VERDICT -> PASS` at commit `b0cc4a5` (tester report `spec/chart-buy-sell-emphasis/reports/test-2026-05-11-2103-chart-buy-sell-emphasis-final.md`; M6.2 hardening closeout `spec/chart-buy-sell-emphasis/reports/m6.2-hardening-2026-05-11.md`). Pulled evidence from feature brief (lines 1–880 architect Design + Q1–Q10 resolutions), tasks.md (33 dev tasks + T_FINAL all ticked), tester report §1–§14, and two fresh live runs on this machine: `bash scripts/verify_anchors.sh` (ANCHORS PASS 11/11) + `cargo build --release --bin cockpit --features fixtures` + `./target/release/cockpit` + `screencapture -x /tmp/cockpit-presenter-charts.png` (clean boot at default size + Home-screen render confirmed). Surfaces no decisions for the operator (the safer-default Q1 = (a) + `enabled = false` is the architect's confirmed default; flipping is one TOML edit if the operator wants it on). Seven operator-relevant facts called out under "Notes for the operator". Pre-tick gate `bash scripts/check_presentation.sh` run on this file — see closing summary.
- 2026-05-11 (orchestrator, operator-relayed via chat): operator approved clean as `[x] Approved — ship` with no overrides and no follow-up notes. Q1 = Option (a) signal source ships default-OFF as designed; macOS `.app` bundle for the dock icon is deferred per the candidate stub at `spec/cockpit-app-bundle/feature.md`. Three-arc implementation history acknowledged. Feature flips `status: in-progress → shipped` on both `feature.md` and `tasks.md`; backlog updates same commit to add `chart-buy-sell-emphasis v1.9` to the Recent section.
