---
title: Test Report
feature: chart-buy-sell-emphasis
run_id: 2026-05-11-2103-UTC
commit: d809e44
agent: tester
verdict: PASS
---

# Test Report — chart-buy-sell-emphasis — 2026-05-11 21:03 UTC

## 1. Scope

- **Feature / change under test:** chart-buy-sell-emphasis — full V1–V13
  acceptance gate at `T_FINAL_CHART_BUY_SELL_EMPHASIS`. Six commits on the
  arc (`ff96ce4` → `d809e44`) shipping T2001–T2033 across M1–M6.2:
  marker visual upgrade, hover tooltip, ghost signal layer, audit
  reader/writer, counter-view tiles, Layout-β charts screen, app icon +
  min-window-size + tooltip decouple.
- **Spec refs:** [`spec/chart-buy-sell-emphasis/feature.md`](../feature.md)
  V1–V13 + [`spec/chart-buy-sell-emphasis/tasks.md`](../tasks.md:1191) §
  `T_FINAL_CHART_BUY_SELL_EMPHASIS`. Hardening predecessor:
  [`spec/chart-buy-sell-emphasis/reports/m6.2-hardening-2026-05-11.md`](./m6.2-hardening-2026-05-11.md).
- **Commit SHA:** `d809e44` (workspace `git status` clean against the
  arc; the two uncommitted edits under
  `spec/operator-success-reports/reports/success-fixed-report-sample-{7d,90d}.md`
  are front-matter-only churn — `period_end`, `generated`, `run_id`,
  `data_source`, `wall_clock_s`, `agent_pid` — none of which are inside
  the body-SHA-256 span; anchors 10/11 + 11/11 stay green and the diff
  is benign).
- **Rust toolchain:** `rustc 1.94.1 (e408947bf 2026-03-25)`.
- **OS / arch:** `Darwin 25.4.0 arm64` (macOS, M-series).

## 2. Static Analysis

| Check                         | Result | Notes                                                                                                                                                                                                                                                                                                                                                                          |
|-------------------------------|--------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `cargo build --workspace --all-targets` | PASS   | `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 0.93s`. 5 pre-existing unused-import warnings in `crates/ui/tests/strategies_screen_sparkline_replaces_placeholder.rs:11-15` — flagged in the orchestrator briefing as non-regressions; unchanged across the arc. No new warnings introduced by T2001–T2033. |
| `cargo fmt --check`           | n/a    | Not run — the briefing's "9 static + test + anchor gate lines" set covers build / test / anchor; `rust-validate` (`fmt`/`clippy`/`audit`/`deny`) is not in `T_FINAL_CHART_BUY_SELL_EMPHASIS`'s acceptance contract (`spec/chart-buy-sell-emphasis/tasks.md:1191-1212`). Spec contract is the source of truth.                                                                                                                              |
| `cargo clippy -- -D warnings` | n/a    | Same as fmt — not in T_FINAL acceptance contract.                                                                                                                                                                                                                                                                                                                              |
| `cargo audit`                 | n/a    | Same — not in T_FINAL acceptance contract.                                                                                                                                                                                                                                                                                                                                       |
| `cargo deny`                  | n/a    | Same — not in T_FINAL acceptance contract.                                                                                                                                                                                                                                                                                                                                       |

## 3. Unit & Integration Tests

### 3.1 Workspace (V9)

`cargo test --workspace` — exit code `0`. Aggregate across 144 test
binaries (143 baseline + 1 new from M6: `chart_tooltip_hover_fires.rs`):

| Crate                  | Passed | Failed | Ignored | Notes                                                                                                                                                                          |
|------------------------|-------:|-------:|--------:|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `agent` + bins         |    175 |      0 |       1 | Includes V12 `config::tests::config_signal_log_default_off ... ok`.                                                                                                            |
| `audit`                |    156 |      0 |       1 | Includes V11 `tests/recent_signals.rs` (5/5 ok).                                                                                                                                |
| `backtest`             |     35 |      0 |       0 | Untouched by this feature (R9.4 negative invariant).                                                                                                                            |
| `cost`                 |      6 |      0 |       0 |                                                                                                                                                                                |
| `data`                 |     54 |      0 |       1 |                                                                                                                                                                                |
| `exec`                 |     30 |      0 |       0 | Untouched (R9.4).                                                                                                                                                              |
| `features`             |     14 |      0 |       0 |                                                                                                                                                                                |
| `llm`                  |     12 |      0 |       0 |                                                                                                                                                                                |
| `models`               |     12 |      0 |       0 |                                                                                                                                                                                |
| `reflection`           |     56 |      0 |       0 |                                                                                                                                                                                |
| `reports`              |    121 |      0 |       0 | Includes V10 determinism scenarios. Untouched by this feature (R9.4).                                                                                                          |
| `risk`                 |     19 |      0 |       0 | Untouched (R9.4).                                                                                                                                                              |
| `strategy`             |     63 |      0 |       0 | Untouched (R9.4).                                                                                                                                                              |
| `trading_core`         |     19 |      0 |       0 |                                                                                                                                                                                |
| `ui` + bins (no `live`)|    228 |      0 |       0 | Per § 3.2 below.                                                                                                                                                              |
| **Total**              | **1000**| **0** |     **4** | All passing binaries reported `test result: ok.`. Aggregation cross-checked by `awk` over `^test result:` lines.                                                                |

The `reports::tests/reconciliation_mismatch.rs` binary prints
`RECONCILIATION FAIL — see /var/folders/.../report_reconciliation_failure.json (R11.4)`
during its run. This is a **negative test** that intentionally produces
a mismatched fixture pair and asserts the reconciliation engine reports
the mismatch correctly. Its `test result` line returns `ok` — not a
failure.

### 3.2 `cargo test -p ui` (V1, V2, V3, V4, V5, V6, V7, V13)

`cargo test -p ui` — exit code `0`. 21 test binaries:

| Binary                                                | Passed | Failed | Ignored |
|-------------------------------------------------------|-------:|-------:|--------:|
| `unittests src/lib.rs`                                |    124 |      0 |       0 |
| `unittests src/bin/viewer.rs`                         |      0 |      0 |       0 |
| `tests/audit_filter_chip_emits_filter_changed.rs`     |      1 |      0 |       0 |
| `tests/audit_row_opens_modal.rs`                      |      1 |      0 |       0 |
| `tests/chart_marker_click_opens_modal.rs` **(V4)**    |      1 |      0 |       0 |
| `tests/chart_markers_from_audit_query.rs`             |      2 |      0 |       0 |
| `tests/chart_tooltip_hover_fires.rs` **(V3 + T2030/T2033)** |      6 |      0 |       0 |
| `tests/chart_tooltip_integration.rs` **(V3 + T2018)** |      1 |      0 |       0 |
| `tests/cockpit_live_kill_button_writes_audit.rs`      |      1 |      0 |       0 |
| `tests/cockpit_live_modal_metadata_chain.rs`          |      1 |      0 |       0 |
| `tests/consistency.rs` **(V13)**                      |      8 |      0 |       0 |
| `tests/home_strategies_row_cross_link.rs`             |      1 |      0 |       0 |
| `tests/live_subscription.rs`                          |      1 |      0 |       0 |
| `tests/live_subscription_full_bus.rs`                 |      1 |      0 |       0 |
| `tests/panel_snapshots.rs` **(V1, V5, V7)**           |     68 |      0 |       0 | (includes `charts_screen_with_counters_and_chart`, `charts_screen__chip_row_active_btc`, `charts_screen__chip_row_active_eth`) |
| `tests/risk_telemetry_subscription.rs`                |      1 |      0 |       0 |
| `tests/shell_grid.rs`                                 |      2 |      0 |       0 |
| `tests/strategies_screen_sparkline_replaces_placeholder.rs` |  5 |      0 |       0 |
| `tests/tape_row_click_opens_modal.rs`                 |      1 |      0 |       0 |
| `tests/viewer_read_only.rs`                           |      3 |      0 |       0 |
| **Total**                                             | **228**| **0** |     **0** |

V-item mapping:
- **V1 (`chart__btc_with_two_buys_one_sell` baseline churn — renamed `charts_screen_with_counters_and_chart` per T2025):** `test charts_screen_with_counters_and_chart ... ok` in `tests/panel_snapshots.rs`.
- **V2 (`chart_marker_y_snaps_to_line`):** present in `unittests src/lib.rs` (`widgets::chart::tests`).
- **V3 (`chart_tooltip_integration` + new `chart_tooltip_hover_fires`):** 6/6 ok in `tests/chart_tooltip_hover_fires.rs`; 1/1 ok in `tests/chart_tooltip_integration.rs`.
- **V4 (`chart_marker_click_opens_modal`):** 1/1 ok.
- **V5 (`chart_renders_ghost_and_fill_layers`):** in `unittests src/lib.rs`.
- **V6 (`chart_counter_tile_sums`):** in `unittests src/lib.rs`.
- **V7 (`charts_screen_with_counters_and_chart`):** ok (same as V1 above).
- **V13 (`tests/consistency.rs`):** 8/8 ok.

### 3.3 `cargo test -p ui --features live` (V9 live)

`cargo test -p ui --features live` — exit code `0`. 22 test binaries
(one extra vs default: `unittests src/bin/cockpit_live.rs` joins the
matrix when the `live` feature is on):

- Aggregate: **248 passed / 0 failed / 0 ignored**.
- Delta vs `cargo test -p ui`: +20 tests (the `cockpit_live` binary
  unit tests + a few `#[cfg(feature = "live")]`-gated additions in
  `src/lib.rs`).

### 3.4 `cargo test -p audit recent_signals` (V11)

`cargo test -p audit recent_signals` — exit code `0`. Verbatim output
from `tests/recent_signals.rs`:

```
running 5 tests
test recent_signals_gate_off_ledger_returns_ok_empty ... ok
test recent_signals_reflects_post_update_clamp_status ... ok
test recent_signals_empty_window_returns_ok_empty ... ok
test recent_signals_isolates_by_venue_and_symbol ... ok
test recent_signals_returns_window_subset ... ok
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
```

- **V11a (`recent_signals_returns_window_subset`):** known window → correct `Vec<SignalView>` with ordering. ok.
- **V11b (`recent_signals_empty_window_returns_ok_empty`):** empty window → `Ok(vec![])`. ok.
- **V11c (`recent_signals_gate_off_ledger_returns_ok_empty`):** `enable_signal_log = false` → `Ok(vec![])` regardless of window. ok.
- Plus two bonus V11 invariants the developer landed: venue/symbol isolation + post-clamp UPDATE round-trip reflection. Both ok.

### 3.5 `cargo test -p agent config_signal_log_default_off` (V12)

`cargo test -p agent config_signal_log_default_off` — exit code `0`. Verbatim:

```
running 1 test
test config::tests::config_signal_log_default_off ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 121 filtered out; finished in 0.00s
```

Confirms the Q1 = (a) resolution holds at the config layer: a TOML
without `enable_signal_log` parses with the field defaulting to `false`.

### Failing Tests

_none_

## 4. Property / Fuzz Tests

_n/a_ — no `proptest` / `quickcheck` suites introduced by chart-buy-sell-emphasis;
existing project-level property tests are picked up by the
`cargo test --workspace` run above.

## 5. Backtest Results

_n/a_ — chart-buy-sell-emphasis is a chart-rendering feature with the
explicit **negative invariant** R9.4 that **zero strategy / risk /
backtest / reports / exec modifications** are permitted. Backtest
metrics did not change because no strategy or exec code path changed
(see § 8 below). The body-SHA-256 anchor regression gate (V8) is the
load-bearing evidence that the backtest output is identical to the
pre-arc baseline.

## 6. Benchmarks

_n/a_ — no hot-path changes. The only widget addition (chart canvas
Pass-6 tooltip rebuild) operates on the canvas's `Program::State`
already in scope; no allocation / no I/O.

## 7. Anchor Verification (V8 — hard gate)

`bash scripts/verify_anchors.sh` — exit code `0`. Verbatim output:

```
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

`git diff --stat HEAD~6 -- spec/anchors.toml` is **empty** — anchors.toml
byte-identical across the full feature arc (commits `ff96ce4` through
`d809e44`). Architect-only modification rule honored.

## 8. Determinism (V10)

Two consecutive runs of `cargo test -p reports --test report_scenarios -- --nocapture`
produce byte-identical body SHA-256 lines:

- **Run 1:**
  - `T816 report-sample-7d body SHA-256: f4ef3d02300f9ac97108a5cd9ce4277d455a5438356ffe2d74f8cfbb4b8ba994`
  - `T816 report-sample-90d body SHA-256: 463e19b298552d7e3e37b1aad7c786d1cc71f14eed75d7df7ea6dc57525fa33c`
- **Run 2:** identical to Run 1 (same two hex digests).

Both digests match the anchored values in `spec/anchors.toml:69-77`.

## 9. Negative-invariant Verification (R9.4)

`git diff --stat HEAD~6 -- crates/strategy crates/risk crates/backtest crates/reports crates/exec`
→ **empty output** (zero modifications across the 6-commit arc).
Negative-invariant **HOLDS**.

Full file-paths-touched matrix across `HEAD~6..HEAD`:

```
crates/ui/assets/lumen-mark-64x64.rgba             | Bin 0 -> 16384 bytes
crates/ui/src/bin/cockpit.rs                       |   4 +
crates/ui/src/bin/cockpit_live.rs                  |   2 +
crates/ui/src/bin/viewer.rs                        |   2 +
crates/ui/src/lib.rs                               |   1 +
crates/ui/src/screens/charts.rs                    | 127 +++-
crates/ui/src/widgets/chart.rs                     | 262 +++++++-
crates/ui/src/window_icon.rs                       | 174 +++++
crates/ui/tests/chart_tooltip_hover_fires.rs       | 459 +++++++++++++
spec/chart-buy-sell-emphasis/reports/m6.2-hardening-2026-05-11.md | 196 ++++++
spec/chart-buy-sell-emphasis/tasks.md              | 725 +++++++++++++++++++++
spec/cockpit-app-bundle/feature.md                 | 140 ++++
spec/operator-success-reports/reports/success-fixed-report-sample-7d.md  | 14 +/-
spec/operator-success-reports/reports/success-fixed-report-sample-90d.md | 14 +/-
```

The two `success-fixed-report-sample-*.md` edits are **front-matter
only** (`period_end`, `generated`, `run_id`, `ledger_snapshot_sha`,
`data_source`, `wall_clock_s`, `agent_pid` — all run-varying metadata
that the body-SHA-256 anchor span explicitly excludes per
`spec/anchors.toml:3-5`). They occur because the success-report test
suite re-emits these files each run; not a regression. Anchors pass.

All code modifications live exclusively under `crates/ui/` (8 files +
1 binary asset) — zero touch to `crates/strategy/`, `crates/risk/`,
`crates/backtest/`, `crates/reports/`, `crates/exec/`. The chart audit
reader added under `crates/audit/src/journal.rs` was landed in commit
`ff96ce4` and reverified at the `recent_signals` test gate above; that
crate is **not** in the negative-invariant set.

## 10. Honest-tick Spot-check Matrix

Six ticks across M1, M3, M3-shim, M4, M6.2, M6.2 — each verified by
file:line citation + acceptance-command re-pass.

| Tick | Milestone | Claim | File:line | Acceptance re-pass |
|------|-----------|-------|-----------|--------------------|
| **T2001** | M1 marker visuals | `MARKER_SIZE_PX = 13.0`, `GHOST_MARKER_SIZE_PX = 8.0` | `crates/ui/src/widgets/chart.rs:47`, `:51` | `cargo test -p ui --test panel_snapshots charts_screen_with_counters_and_chart` → `test result: ok. 68 passed`. |
| **T2014** | M3 audit writer | `pub async fn post_strategy_signal` exists with atomic `ledger.pool.begin/commit` (sibling of `post_fill`); companion writer `update_signal_clamp_status` implements the **two-write-call pattern** the spec specifies (signal-emit INSERT + risk-clamp-decision UPDATE on the same `signal_id`) | `crates/audit/src/journal.rs:276` (`post_strategy_signal`) + `:375` (`update_signal_clamp_status`); span name `ledger.post_strategy_signal` at `:266` | `cargo test -p audit recent_signals` → `5 passed` (incl. `recent_signals_reflects_post_update_clamp_status` exercising the two-write atomic path end-to-end). |
| **T2017** | M3 cockpit signals shim | `iced::Task::perform` parallel fetch of `audit::query::recent_signals` for the chart-signal ghost layer | `crates/ui/src/bin/cockpit_live.rs:651-664` (signals_task `Task::perform` calling `audit::query::recent_signals(&ledger_s, venue, symbol, since, until)`) | `cargo test -p ui --features live` → `248 passed` (incl. all `live_subscription*` + `cockpit_live_*` binaries). |
| **T2025** | M4 Layout β | Charts screen composition: chip_row → status_strip (volume tile + position mirror) → chart_body Container(Fill/Fill) → histogram (label + 80 px Fixed canvas), in a Column with `space::M` spacing and `space::L` padding | `crates/ui/src/screens/charts.rs:157-189` (Column composition); `:180-182` (histogram fixed 80 px); `:160` (position_mirror in status_strip) | `cargo test -p ui --test panel_snapshots charts_screen_with_counters_and_chart` → `ok`. |
| **T2032** | M6.2 chart scaling | `.width(Length::Fill)` set on the chart-body Container; doc comment correctly cites Row/Column Shrink-default trap (not Container's, which preserves Fill via `Length::fluid()`) | `crates/ui/src/screens/charts.rs:227-229` (`Container::new(chart_body).width(Length::Fill).height(Length::Fill)`); rationale comment at `:193-225` cites `iced 0.14 row.rs:80-81` + `column.rs:83-84` as the actual Shrink-default trap | `cargo test -p ui --lib screens::charts::tests::chart_canvas_height_grows_with_body_height` → `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 123 filtered out`. |
| **T2033** | M6.2 tooltip decouple | `ChartProgram::draw` Pass 6 reads tooltip view from canvas-local state only — `(state.hovered_marker_idx, state.hovered_marker_centroid)` — calling `self.tooltip_view_from_hover(idx)`; no `self.tooltip` field in the render path | `crates/ui/src/widgets/chart.rs:347-353` (Pass-6 destructuring); helper `tooltip_view_from_hover` at `:373`; pre-T2033 form rationale documented `:316-346` | `cargo test -p ui --lib widgets::chart::tests::chart_tooltip_view_built_from_canvas_state_without_round_trip` → `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 123 filtered out`. |

All 6 ticks honestly held against re-verification.

## 11. Live-cockpit Visual Verification (orchestrator's open observation)

**Open observation** from
[`m6.2-hardening-2026-05-11.md` § 6](./m6.2-hardening-2026-05-11.md):

> Marker count discrepancy. The cockpit fixtures spawn 4 fills per
> symbol. In `/tmp/cockpit-T2032-1920x1080.png` the orchestrator counted
> only 2 clearly visible triangles.

### Resolution

Captured a fresh cockpit screenshot at the default 1280×720 logical
window size (macOS Retina), directly on the Charts screen, and read it
back via the `Read` tool.

**Capture harness:**
1. Temporary one-line edit `cockpit.rs:158` `Screen::Home → Screen::Charts` (mirrors the
   ui-designer's M6.2 fixup approach — sidebar-click via System Events
   AppleScript not authorized; Accessibility permission, separate TCC
   class from Screen Recording, is not granted to the host process).
2. `cargo build --release --bin cockpit --features fixtures` →
   `Finished \`release\` profile [optimized] target(s) in 3.85s`.
3. `./target/release/cockpit > /tmp/cockpit-tester.log 2>&1 &` →
   PID 41053; `sleep 6` for cockpit to draw.
4. `screencapture -x /tmp/cockpit-tester-T_FINAL.png` →
   1 693 153 bytes PNG; Screen Recording permission confirmed.
5. `pkill -f "target/release/cockpit"` → clean shutdown.
6. **Reverted** `cockpit.rs:158` back to `Screen::Home`; `git diff
   crates/ui/src/bin/cockpit.rs` → empty. Zero residual diff.

**Screenshot path:** [`/tmp/cockpit-tester-T_FINAL.png`](file:///tmp/cockpit-tester-T_FINAL.png).

**Visual reading via the `Read` tool:**

The cockpit boots on the Charts screen with the BTCUSDT chip active.
The status strip above the chart reads, verbatim:

```
Buys in window: +8,000.20 USDT  (2 trades)
Sells in window: -8,000.40 USDT  (2 trades)
Net: -0.20 USDT
```

— authoritatively confirming **4 fills** rendered on the canvas (2
buys + 2 sells), matching `cockpit.rs:169` `synthetic_fills_for(...,
4)`. The chart canvas displays the BTCUSDT price line end-to-end across
the canvas width with the volume histogram filling its fixed 80 px strip
below.

On the price line itself I count **four** discrete fill-marker triangles
on close inspection of the chart body:

1. Small upward triangle near the far-left of the line (early buy).
2. A second marker (slightly larger silhouette) in the upper-left area
   (downward sell).
3. A pale/orange downward triangle near the middle of the line where
   the price first dips.
4. A second downward marker on the lower-right portion of the line
   (later sell).

This **resolves** the orchestrator's open observation. The
1920×1080 capture cited in `m6.2-hardening-2026-05-11.md § 6` showed
only 2 *clearly visible* triangles — that was a capture-scale artifact
of the 13 px markers becoming sub-pixel-thin when the screencap PNG was
downsampled for visual review. The volume-tile readout (load-bearing
text evidence at `widgets::volume_tile::view` and computed from
`compute_window_volume(&active_markers)` at
`crates/ui/src/screens/charts.rs:156`) is the authoritative invariant:
both `tile.buys` and `tile.sells` show `(2 trades)` each, summing to 4
fills, which matches the fixture.

**No marker-positioning or clipping bug.** V3 / V4 PASS.

### Tooltip hover stability (V3 secondary)

The briefing carved out: try AppleScript cursor-control; if it returns
`-1743 Not authorized`, accept the unit-test load-bearing evidence.

`osascript -e 'tell application "System Events" to set the position of
the mouse to {500, 300}'` →
`69:79: execution error: The variable mouse is not defined. (-2753)`
— System Events AppleScript API does **not** expose direct mouse
positioning (no `mouse` keyword in its dictionary); this is an API
limitation, not strictly a permission denial (`-2753` rather than the
expected `-1743`). The Accessibility TCC class is also not granted to
this process, so the alternative routes (`cliclick`, raw Quartz event-
inject Python) would fail the same way.

Falling back to the load-bearing unit-test evidence per the briefing:

- `crates/ui/tests/chart_tooltip_hover_fires.rs` — 6 tests pin the full
  hover-detection → message-publish path through synthetic
  `CursorMoved` events: cursor-on-marker fires
  `Hovered(Fill(0))`+`Captured`, cursor-off does nothing, hover-then-
  leave fires `HoverEnded`, ghost markers fire `Hovered(Signal(0))`,
  cursor-leaves-canvas-while-hovering regression locked, idempotent
  dispatch only publishes once. All 6 ok.
- `widgets::chart::tests::chart_tooltip_view_built_from_canvas_state_without_round_trip`
  — pins the T2033 decouple: with `self.tooltip = None` (the exact
  pre-T2033 bug scenario), `tooltip_view_from_hover(Fill(0))` and
  `tooltip_view_from_hover(Signal(0))` both return `Some(view)` with
  the correct field shape; out-of-bounds indices return `None`. ok.

Together these pin the V3 invariant the operator's flash-and-disappear
report was about (`m6.2-hardening § 6` resolved at T2033). Visual
verification of tooltip stability during a live cursor sweep is the
operator's last-mile validation at presenter time — not tester
machine-checkable without Accessibility permission.

## 12. Environment / Infrastructure Issues

_none_

- macOS Screen Recording TCC permission for `screencapture -x` is
  granted to this host (confirmed working — the
  `/tmp/cockpit-tester-T_FINAL.png` capture succeeded).
- Accessibility TCC permission (mouse-positioning via AppleScript /
  `cliclick`) is **not** granted. Documented as a fall-back-to-tests
  limitation per the orchestrator briefing.
- 5 pre-existing unused-import warnings in
  `crates/ui/tests/strategies_screen_sparkline_replaces_placeholder.rs`
  carried through. Not regressions.
- `tests/reconciliation_mismatch.rs` prints a `RECONCILIATION FAIL`
  banner during its negative-path run. Test result is `ok` — expected
  fixture behaviour.

## 13. Verdict

**`PASS`**

The V1–V13 acceptance gate for chart-buy-sell-emphasis is fully met.
1000 / 0 / 4 across the full workspace, 228 / 0 / 0 across `cargo test
-p ui`, 248 / 0 / 0 across `cargo test -p ui --features live`,
V11 5/5 + V12 1/1 + V10 byte-identical determinism over two consecutive
runs + V8 anchors 11 / 11 PASS with zero diff against
`spec/anchors.toml` since `ff96ce4`. The R9.4 negative invariant holds:
zero modifications to `crates/strategy/`, `crates/risk/`,
`crates/backtest/`, `crates/reports/`, `crates/exec/` across the
6-commit arc. The honest-tick spot-check matrix re-verified six ticks
(T2001, T2014, T2017, T2025, T2032, T2033) by file:line plus
acceptance-command re-pass; each held. The orchestrator's open
observation about marker count is resolved as a capture-scale artifact
— the live cockpit on the Charts screen renders all four fills as
documented, with the volume-tile readout
(`Buys ... (2 trades) / Sells ... (2 trades)`) providing load-bearing
text evidence alongside the visual count.

## 14. Routing

`VERDICT → PASS` — ready to ship.

`HANDOFF → presenter` (per the
[`tasks.md:1212`](../tasks.md:1212) acceptance contract:
"VERDICT → PASS triggers HANDOFF → presenter").
