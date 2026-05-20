---
generated: 2026-04-18
updated: 2026-04-30
author: ui-designer
feature: v0-paper-sma, v05-composed-strategies, v1-cross-sectional-momentum, v15a-mean-reversion-pairs
scope: T_FINAL_B (v0) + T_FINAL_B (v0.5) + T_FINAL_B_v1 + T_FINAL_B_v15a — cockpit smoke
---

# Cockpit Smoke + Kill-Switch Drill (T_FINAL_B)

This is the operator-run checklist for Week 2's cockpit acceptance.
It pairs the **panel state reference** in
[`screenshots/v0-paper-sma/README.md`](screenshots/v0-paper-sma/README.md)
(compacted from 16 per-state artifacts) with the two runtime smokes
(fixtures-driven and live against a running agent).

The sandbox that generated this report is **headless** — real PNG
screenshots are marked `_deferred_manual_` and must be captured on the
operator's workstation. The acceptance gates that _can_ be verified
headless are covered by automated tests and by the logical-state
artifacts; together they catch every regression except pixel-layout
drift.

---

## Sandbox-verifiable gates (automated)

| Gate | Command | What it checks |
|------|---------|----------------|
| Build (fixtures) | `cargo build -p ui --bin cockpit --features fixtures` | Cockpit boots against in-memory fixtures. |
| Build (live)     | `cargo build -p ui --bin cockpit --features live`     | Cockpit compiles against `agent::EventBus`. |
| Unit + integration | `cargo test -p ui --features live`                  | 3 live-subscription integration tests (T32) + all Week 1 tests. |
| Consistency audit | `cargo test -p ui`                                   | Zero inline strings, zero inline hex (T14 gate still green). |
| Workspace no-regression | `cargo test --workspace`                        | Developer's T31/T_FINAL_A tests still green. |

Each of these must pass before the operator runs the manual steps below.

---

## Panel state reference (committed)

All 16 panel-state descriptions (`tape|positions|pnl|kill` × `loading|empty|error|ready`) now live in a single compacted document:
[`spec/v0-paper-sma/reports/screenshots/README.md`](screenshots/v0-paper-sma/README.md).

That README captures, for each state, the exact copy shown, the `strings::*`
key backing each label, and the `theme::*` tokens driving each color.

The kill panel's states collapse: "loading" and "empty" are both the idle
button view, "error" is the halted sticky banner, "ready" is the confirm
dialog with the safety phrase matched.

These mappings are also asserted on every test run by
`crates/ui/tests/panel_snapshots.rs` — the README doubles as human-readable
documentation of what those snapshots cover.

---

## Manual steps — fixtures smoke (headless ok for build, display for visuals)

1. [ ] `cargo run --bin cockpit --features fixtures`
   - Window opens with title `Trading Cockpit`.
   - Left column top-to-bottom: P&L card (ready), Feed latency badge
     (OK green, 120 ms), Stop-trading button (idle).
   - Right column: Open positions (one BTC row), Live tape (12 fills).
   - Theme: Dark.
   - `[ ]` screenshot-fixtures-main.png _deferred_manual_
2. [ ] Click `Pause` on the tape. Label changes to `Resume`. Paused
   banner appears above the row list.
   - `[ ]` screenshot-fixtures-tape-paused.png _deferred_manual_
3. [ ] Click `Resume`. Banner disappears. Any fills buffered during the
   pause appear at the top (fixtures do not publish, so the tape is
   unchanged but the pause state machine is exercised).
4. [ ] Click the big red `Stop trading` button.
   - Dialog opens with `Confirm stop trading` title and the full body
     copy from `strings::KILL_DIALOG_BODY`.
   - Input placeholder reads `Type HALT BTC to confirm`.
   - Confirm button is greyed out (no `on_press`); Cancel is active.
   - `[ ]` screenshot-fixtures-kill-dialog-empty.png _deferred_manual_
5. [ ] Type `HALT` (partial). The mismatch hint appears below the
   input in amber (`color::WARN`). Confirm still disabled.
   - `[ ]` screenshot-fixtures-kill-dialog-mismatch.png _deferred_manual_
6. [ ] Complete the phrase to `HALT BTC`. Mismatch hint disappears.
   Confirm button becomes active.
   - `[ ]` screenshot-fixtures-kill-dialog-matched.png _deferred_manual_
7. [ ] Press Cancel. Dialog closes, panel returns to idle.
8. [ ] Quit the binary (`Cmd+Q` on macOS).

> Why deferred PNGs: the sandbox that runs the quality gates is headless
> (no display). All pixel captures must happen on the operator's desktop.
> Capture using the OS screenshot tool (`Cmd+Shift+4` on macOS,
> `gnome-screenshot -i` on Linux) and commit the PNGs into
> `spec/v0-paper-sma/reports/screenshots/` with the filenames above.

---

## Manual steps — kill-switch drill (live against running agent)

**Terminals:** one for the agent, one for the cockpit, one for drills.

1. [ ] Terminal 1: start the agent.
   ```
   cargo run --bin agent -- --config config/agent.toml --mode research
   ```
   Expect subsystem-init log lines and `/metrics` on `:9100`.

2. [ ] Terminal 2: start the cockpit wired to the live bus.
   ```
   cargo run --bin cockpit --features live
   ```
   With the agent running, the tape begins advancing within 2s of the
   next replay bar. The latency badge flips from `—` to a green `OK`
   value once the first tick arrives.

   _Note on same-process IPC:_ the T32 `ui::live` module shares an
   `agent::EventBus` in memory. In v0, the `cockpit` binary creates an
   **empty** bus on startup — to get actual data, you need to run the
   agent+cockpit in a **unified binary** that hands a shared
   `Arc<EventBus>` to both. If you see panels stuck in `Loading`
   forever, this is why. A unified `agent+cockpit` binary is a v0.5
   deliverable; for v0 the manual drill uses the fixtures build to
   exercise UI flow, and the live-bus path is validated by the
   integration test `crates/ui/tests/live_subscription.rs`.

   `[ ]` screenshot-live-tape-advancing.png _deferred_manual_

3. [ ] **Halt trigger A — `.halt` file drill.**
   - Terminal 3: `touch .halt` (use the configured `halt_file` path
     from `config/agent.toml`; default `./.halt`).
   - Within ~1.5s the agent's file watcher detects the file, trips the
     kill switch, broadcasts `AgentMode::Halted { reason: "halt_file" }`
     on the mode channel.
   - Cockpit receives the event, flips the mode banner to `AGENT HALTED`
     (red, `color::NEG`), shows the runbook hint.
   - `[ ]` screenshot-live-halt-file-banner.png _deferred_manual_
   - Clean up: stop both processes, `rm .halt`.

4. [ ] **Halt trigger B — cockpit button typed-phrase confirm.**
   - Restart agent + cockpit (clean `.halt` gone).
   - Click `Stop trading` in the cockpit.
   - Type `HALT BTC` in the confirm dialog. Confirm button activates.
   - Press Confirm. Cockpit enters `Flattening` state. (In v0 there
     is no round-trip to the agent for operator-initiated halts; the
     full agent-stops-when-confirmed wiring is T_FINAL_B-post work.
     The halted banner appears when the agent independently
     publishes `AgentMode::Halted`.)
   - `[ ]` screenshot-live-kill-confirmed.png _deferred_manual_

5. [ ] **Runbook link verification (automated-friendly).**
   - Open `spec/runbooks/kill-switch.md`.
   - Confirm `strings::KILL_RUNBOOK_LINK_PATH` value matches the path
     (`spec/runbooks/kill-switch.md`). Grep check:
     ```
     grep -n "spec/runbooks/kill-switch.md" crates/ui/src/strings.rs
     ```
     Must return exactly one line.

---

## Acceptance checklist for T_FINAL_B

- [x] Smoke checklist committed (this file).
- [x] Panel state reference committed at
  `spec/v0-paper-sma/reports/screenshots/README.md` (compacted from the
  original 16 per-state artifacts on 2026-04-19).
- [x] Kill-switch drill section documents both triggers (`.halt` file
  file-watcher + cockpit button typed-phrase confirm).
- [x] Runbook link verified: `ui::strings::KILL_RUNBOOK_LINK_PATH`
  resolves to `spec/runbooks/kill-switch.md` and
  `strings::KILL_RUNBOOK_LINK_LABEL` is rendered from the halted-panel
  view.
- [ ] PNG screenshots — **deferred to operator workstation run** (see
  placeholders above). The CI gate is the logical-state artifacts; the
  operator PR review adds the PNGs.

---

## Deferred PNG list (capture instructions)

Captured on the operator's workstation with OS screenshot tool, saved
into `spec/v0-paper-sma/reports/screenshots/`:

| File                                     | How to capture |
|------------------------------------------|----------------|
| screenshot-fixtures-main.png             | `cargo run --bin cockpit --features fixtures`; wait for render; full-window. |
| screenshot-fixtures-tape-paused.png      | same binary; click Pause; full-window. |
| screenshot-fixtures-kill-dialog-empty.png| same binary; click Stop trading; dialog only. |
| screenshot-fixtures-kill-dialog-mismatch.png | type `HALT` partially; dialog only. |
| screenshot-fixtures-kill-dialog-matched.png | complete `HALT BTC`; dialog only. |
| screenshot-live-tape-advancing.png       | run agent in term 1, cockpit `--features live` in term 2; full-window. |
| screenshot-live-halt-file-banner.png     | `touch .halt`; wait 2s; cockpit full-window. |
| screenshot-live-kill-confirmed.png       | kill-switch flow in live mode, post-confirm. |

---

## v0.5 — strategies panel smoke + hot-swap drill

Scope extension for **T_FINAL_B (v0.5)**. Adds operator verification for
the new `strategies` panel (tasks T522–T528) plus the R7 hot-swap and R8
invalid-config drills from
[v05-composed-strategies.md → Verification V7](../../v05-composed-strategies/feature.md#verification).

The developer's T_FINAL_A landed four v0.5 backtest reports under
`spec/reports/`, unblocking this section:

- [backtest-20260419-125532-btc-2023-1m-sma-baseline-refresh.md](./backtest-20260419-125532-btc-2023-1m-sma-baseline-refresh.md)
  — baseline; body-SHA256 matches v0 `btc-2023-1m-sma-cross`, proving
  v0.5 is purely additive.
- [backtest-20260419-125508-btc-2023-1m-macd-trend.md](./backtest-20260419-125508-btc-2023-1m-macd-trend.md)
  — MACD trend recipe loaded from `config/strategies/btc_macd_trend.toml`.
- [backtest-20260419-125458-btc-2023-1m-rsi-reversion.md](./backtest-20260419-125458-btc-2023-1m-rsi-reversion.md)
  — RSI mean-reversion recipe.
- [backtest-20260419-125501-btc-2023-1m-bbands-mean-revert.md](./backtest-20260419-125501-btc-2023-1m-bbands-mean-revert.md)
  — Bollinger bands mean-reversion recipe.

Each report's `Strategy` section carries the id + content hash + source
path the operator cross-checks against the cockpit's `Hash` column
tooltip during the live drill below.

### Sandbox-verifiable gates (automated) — v0.5 extension

| Gate | Command | What it checks |
|------|---------|----------------|
| Build (fixtures, strategies panel) | `cargo build -p ui --bin cockpit --features fixtures` | Cockpit boots with the strategies panel populated by `ui::fixtures::fake_cockpit_with_strategies` (T525). |
| Live-subscription (strategies) | `cargo test -p ui --features live` | 3 new T526 integration tests (`t526_strategy_loaded_stream_refreshes_cockpit` / `_swapped_` / `_error_`) drive the panel from a fake `EventBus`. Total ≥ 70. |
| Panel snapshots | `cargo test -p ui` | `insta` snapshots for each of the four strategies-panel states + the per-row-error visual (T524). Total ≥ 57. |
| Consistency audit | `cargo test -p ui` | `no_inline_user_visible_strings_in_widgets` + `no_inline_hex_colors_in_widgets_or_state` still zero on the `widgets::strategies` module. |

All gates above must be green before running the manual steps below.

### Manual steps — fixtures walkthrough (four-state contract)

Visual contract for each state: section 4.5 of the panel-state reference
in [`screenshots/v0-paper-sma/README.md`](./screenshots/v0-paper-sma/README.md#45-strategies--loaded-strategies--swap-log)
(the v0.5 addition you read before running this drill). Copy keys
(`STRATEGIES_*`) and theme tokens are pinned there; compare the render
against that table line-by-line.

1. [ ] Terminal 1: `cargo run --bin cockpit --features fixtures`.
   - The right column's top slot shows the `strategies` panel (Q4
     resolution: above Open positions). Left column (P&L, latency, kill
     switch) unchanged from v0.
   - With fixtures, three rows render: one Ready, one Loading, one
     Error. Visual must match README §4.5 `ready` row description:
     columns `Strategy  Hash  Status  Last event  Signals / 60s  Holds
     position`, right-aligned monospace numbers, status-pill colors
     (`POS` / `FG_MUTED` / `NEG`).
   - `[ ]` screenshot-strategies-ready.png _deferred_manual_
2. [ ] Drive the panel to the `loading` state (fixtures helper —
   `fake_cockpit_with_strategies_loading()`; in production this state
   only flashes briefly at cockpit boot before the first bus message
   arrives). Copy `Loading active strategies…` renders in
   `color::FG_MUTED` per README §4.5 `loading` row.
   - `[ ]` screenshot-strategies-loading.png _deferred_manual_
3. [ ] Drive to `empty` (no rows). Copy `No strategies loaded. Drop a
   TOML under config/strategies/ to begin.` renders in `FG_MUTED`. The
   `config/strategies/` path is carried verbatim so the operator knows
   exactly where to add a TOML.
   - `[ ]` screenshot-strategies-empty.png _deferred_manual_
4. [ ] Drive to `error` (close the bus). Copy `Can't read strategies:
   Trading agent disconnected. Check the agent log and restart it.`
   renders with `NEG` prefix via the shared `error_body` frame.
   - `[ ]` screenshot-strategies-error.png _deferred_manual_
5. [ ] Per-row error (R8 visual rehearsal): use the fixtures
   `error_row` variant — the row shows a caption-sized `NEG` badge
   beneath it carrying `error_summary`; other rows stay Ready. This is
   the same state the R8 drill below reaches against a real agent.

### Hot-swap observation drill (R7) — live against running agent

**Terminals:** one for the agent, one for the cockpit, one for
file-edit drills. The `--features live` flag swaps the cockpit's
fixtures subscription for the T526 `ui::live` subscribers that listen
on the three `EventBus` broadcast channels.

1. [ ] Terminal 1: start the agent against the three canonical recipes.
   ```
   cargo run --bin trading -- --config config/agent.toml --mode research
   ```
   Expect log lines `strategy_watcher started` and three
   `StrategyLoaded` events, one per TOML under `config/strategies/`
   (`btc_macd_trend`, `btc_rsi_reversion`, `btc_bbands_mean_revert`).
2. [ ] Terminal 2: start the cockpit wired to the live bus.
   ```
   cargo run --bin cockpit --features live
   ```
   Within 2s of start the strategies panel transitions from `Loading`
   to `Ready` with three rows — ids `btc_macd_trend`,
   `btc_rsi_reversion`, `btc_bbands_mean_revert` — each carrying a
   7-char short hash and a `loaded` last-event value. Hover a row's
   `Hash` cell: tooltip shows the full 64-char hash and the source
   path `config/strategies/<id>.toml` (matches the `Strategy` section
   of the backtest reports linked above).
3. [ ] Terminal 3: edit `config/strategies/btc_macd_trend.toml`. For
   example flip `fast_len` in the signal expression —
   `signal = "macd_hist(12,26,9) > 0 AND close > ema(200)"` →
   `signal = "macd_hist(8,21,9) > 0 AND close > ema(200)"`. Save.
4. [ ] Observe the cockpit: within **2 seconds** the
   `btc_macd_trend` row's short hash flips to a new value, the
   `Last event` cell changes to `swapped`, and a new row appears at
   the top of the recent-events footer colored `WARN` (per §4.5
   footer color map: `swapped` → `WARN`). The other two rows
   (`btc_rsi_reversion`, `btc_bbands_mean_revert`) are unchanged —
   their hashes, statuses and last-event values stay put.
   - `[ ]` screenshot-strategies-hot-swap-after.png _deferred_manual_
5. [ ] Cross-check via the audit ledger: the `strategy_events` table
   now contains exactly one `Swap` row for `btc_macd_trend` with
   distinct `from_hash` / `to_hash` values, in addition to the three
   `Load` rows from boot. (Operator can grep the agent log for
   `StrategySwapped` or query
   `audit::query::strategy_history("btc_macd_trend")`.)
6. [ ] Revert the edit and confirm the panel flips back to the
   original short hash within 2s (second `Swap` row appears in the
   footer).

### Invalid-config drill (R8) — malformed TOML rejection

Immediately after the R7 drill, keep both terminals running and move
to the R8 drill. The goal: verify that a bad edit to one strategy's
TOML flips only that row to the error state, while the other two
strategies keep running (R8 guarantee: fail-closed at the offender,
do not take the registry down).

1. [ ] Terminal 3: introduce a malformed edit to
   `config/strategies/btc_rsi_reversion.toml`. For example delete the
   required `signal` line, or set `signal = "rsi(14) < 30 AND"` (a
   dangling `AND` — parser rejects). Save.
2. [ ] Observe the cockpit: within 2 seconds the `btc_rsi_reversion`
   row's status pill flips from Ready (`POS`) to `Error` (`NEG`), the
   `Last event` cell shows `rejected`, and a caption-sized `NEG`
   error badge appears beneath the row carrying the
   `error_summary` (e.g. `unexpected token at line …` or `missing
   required key "signal"`). A new row appears at the top of the
   recent-events footer colored `NEG` (`rejected` → `NEG`).
3. [ ] Crucially confirm the other two rows (`btc_macd_trend`,
   `btc_bbands_mean_revert`) remain `Ready` with their previous
   hashes — the registry rejected the malformed TOML without
   touching the good strategies. The overall panel stays in
   `Ready` state (not the panel-wide `error` body); only the
   offending row carries the error badge.
4. [ ] Cross-check the agent log: one `StrategyLoadError` event
   published on the `strategy_error` channel; one `Reject` row
   written to `strategy_events` for `btc_rsi_reversion`; the
   registry's pointer for `btc_rsi_reversion` still references the
   previous good strategy (confirmable via the short hash in the
   row, which is unchanged from its pre-edit value).
5. [ ] Revert the TOML to the canonical content. Within 2s the row
   flips back to Ready and the error badge disappears; a fresh
   `Load` / `Swap` row appears in the footer (whichever the
   watcher chooses — a full Load if the previous registration was
   rejected, otherwise a Swap).
6. [ ] Reconciler sanity: over the full drill, the v0 R3.5
   minute-boundary invariant
   (`ledger_imbalance_total == 0`) must hold at every bar —
   `strategy_events` rows do not perturb journal balance
   (enforced by T510 / T518).

### Deferred PNG list — v0.5 additions

Captured on the operator's workstation, saved into
`spec/v0-paper-sma/reports/screenshots/` (or the sibling dir
`screenshots/v05-composed-strategies/` if the v0 dir grows unwieldy —
ui-designer's call on PR review):

| File                                        | How to capture |
|---------------------------------------------|----------------|
| screenshot-strategies-loading.png           | `cargo run --bin cockpit --features fixtures`; drive to the `loading` variant; panel crop. |
| screenshot-strategies-empty.png             | same binary; drive to the `empty` variant; panel crop. |
| screenshot-strategies-error.png             | same binary; drive to the `error` variant (closed bus); panel crop. |
| screenshot-strategies-ready.png             | same binary default run — three rows render; panel crop. |
| screenshot-strategies-hot-swap-after.png    | R7 live drill: after the `btc_macd_trend.toml` edit, cockpit full-window (captures the fresh short-hash + the new `WARN`-colored footer row). |
| screenshot-v1-positions-three-rows.png      | `cargo run --bin cockpit --features fixtures` (v1 default fixture is `fake_cockpit_v1_steady_state`); positions panel crop showing BTC/ETH/SOL with `POS` / `NEG` / `FG_MUTED` P&L colors. |

### Acceptance checklist for T_FINAL_B (v0.5)

- [ ] Four-state fixtures walkthrough performed
  (`cargo run --bin cockpit --features fixtures`); render matches
  `screenshots/v0-paper-sma/README.md` §4.5 line-by-line.
- [ ] R7 hot-swap drill performed against `--features live`; swap
  visible in the strategies panel within 2 seconds of the TOML
  rewrite; `StrategySwapped` event visible in the recent-events
  footer.
- [ ] R8 invalid-config drill performed; offending row flips to
  `Error` with `error_summary` badge; other rows unchanged;
  `StrategyLoadError` event visible in the footer;
  `ledger_imbalance_total == 0` across the drill.
- [ ] Automated gates green: `cargo fmt -p ui -- --check`,
  `cargo clippy -p ui --all-targets --all-features -- -D warnings`,
  `cargo test -p ui` (≥ 57), `cargo test -p ui --features live`
  (≥ 70), workspace consistency audits.
- [ ] Four v0.5 backtest reports cross-checked: each report's
  `Strategy` section id + hash + source matches the cockpit's
  `Hash` tooltip for the same strategy during the live drill
  (baseline report's `Strategy` section is `compiled-in` for the
  refreshed `sma_crossover` baseline; the other three carry the
  composed id + content hash).
- [ ] Deferred PNG screenshots captured on the operator display
  and committed (see v0.5 list above). CI gate is the logical-state
  artifacts (README §4.5 + `insta` snapshots); the operator PR
  review adds the PNGs.

---

## v1 — multi-symbol positions smoke

Scope extension for **T_FINAL_B_v1** (V8 from
[v1-cross-sectional-momentum.md → Verification](../../v1-cross-sectional-momentum/feature.md#verification)).
This is a **negative-confirmation drill** for R11 — the v0 positions
panel already supports N rows; v1 ships zero widget code. Acceptance is
"the cockpit shows up to 3 simultaneous rows in the steady state of the
top-3 long-only momentum strategy."

The fixture `ui::fixtures::fake_v1_three_symbol_portfolio()` (T623) is
tuned to exercise every branch of `theme::color_for_delta` in one
screen so the visual contract is densely covered:

- **BTCUSDT** — long, +$150 unrealized → `POS` green.
- **ETHUSDT** — long, −$200 unrealized → `NEG` red.
- **SOLUSDT** — long, $0 unrealized → `FG_MUTED` neutral.

The matching strategies row is `top10_momentum_h1`, `Holds position =
yes`, signals/60s = 3 (the most recent rebalance fired).

### Sandbox-verifiable gates (automated) — v1 extension

| Gate | Command | What it checks |
|------|---------|----------------|
| Build (fixtures, v1 portfolio) | `cargo build -p ui --bin cockpit --features fixtures` | Cockpit boots against `fake_cockpit_v1_steady_state()` (T623) — the new default fixture for the cockpit binary. |
| Multi-row snapshot | `cargo test -p ui` | New `panel_snapshots__positions_v1_three_rows` snapshot pins three rows with the correct color tokens (`pos` / `neg` / `fg_muted`). Total ≥ 58. |
| Consistency audit | `cargo test -p ui` | `no_inline_user_visible_strings_in_widgets` + `no_inline_hex_colors_in_widgets_or_state` still zero on the positions widget (no widget edits in v1). |
| Workspace no-regression | `cargo test --workspace` | All v0 + v0.5 + v1 backend tests still green. |

All gates above must be green before running the manual steps.

### Manual steps — fixtures walkthrough (multi-row positions)

1. [ ] Terminal 1: `cargo run --bin cockpit --features fixtures`.
   - Window opens; right column (top-to-bottom) shows the strategies
     panel with one row (`top10_momentum_h1`), then the positions
     panel with **three rows** (BTCUSDT, ETHUSDT, SOLUSDT in that
     order — highest momentum first), then the live tape.
   - `[ ]` screenshot-v1-positions-three-rows.png _deferred_manual_
2. [ ] Inspect the positions panel rows:
   - BTCUSDT row: `P&L` column reads `+150.00` in `POS` green;
     `P&L %` reads `+1.25%` also in `POS`.
   - ETHUSDT row: `P&L` reads `-200.00` in `NEG` red; `P&L %` reads
     `-1.82%` also in `NEG`.
   - SOLUSDT row: `P&L` reads `0.00` in `FG_MUTED` neutral; `P&L %`
     reads `0.00%` also in `FG_MUTED`. This row is the one that
     exercises `theme::color_for_delta`'s zero branch — visually
     muted relative to the two flanking rows.
3. [ ] Resize the window narrower (drag the right edge inward until
   the positions panel hits its minimum width). Expected behavior:
   the panel's inner `Scrollable` keeps the rows clipped cleanly to
   the panel frame; row text remains right-aligned monospaced; no
   text overflows the panel border. If the window is dragged
   shorter than the row count needs, the scrollable shows a
   vertical scrollbar — the rows do not fall off the panel into the
   neighboring tape panel.
4. [ ] Quit the binary (`Cmd+Q` on macOS).

> Why deferred PNG: the sandbox is headless. The capture instruction
> is in the deferred-PNG table at the end of the v0.5 section above.

### Acceptance for T_FINAL_B_v1

- [ ] Multi-row fixtures walkthrough performed (step 1 above);
  three position rows visible in the positions panel under
  `cargo run --bin cockpit --features fixtures`.
- [ ] Color contract validated (step 2): `POS` / `NEG` /
  `FG_MUTED` each rendered on the corresponding row's P&L cell —
  proves the v0 widget's `color_for_delta` path handles all three
  signs over an N-row table.
- [ ] Resize/clip behavior sane (step 3): rows clip inside the
  panel via the existing `Scrollable`; no overflow into adjacent
  panels.
- [ ] Strategies panel shows one row for `top10_momentum_h1`,
  `Holds position = yes` (R11.4 cross-link to V8).
- [ ] Automated gates green: `cargo fmt -p ui -- --check`,
  `cargo clippy -p ui --all-targets --all-features -- -D
  warnings`, `cargo test -p ui` (≥ 58, including the new
  `positions_v1_three_rows` snapshot), `cargo test -p ui
  --features live` (≥ 71), workspace consistency audits.
- [ ] Deferred PNG `screenshot-v1-positions-three-rows.png`
  captured on the operator display and committed under
  `spec/v0-paper-sma/reports/screenshots/` (sibling pattern from
  v0.5; ui-designer's call on PR review whether to fork into a
  `screenshots/v1-cross-sectional-momentum/` dir if the v0 dir
  grows unwieldy).
- [ ] ui-designer signoff: no widget code changed for v1 (R11
  negative confirmation). The only diff in `crates/ui/` is in
  `fixtures.rs` (data), `bin/cockpit.rs` (default-fixture wiring),
  and `tests/panel_snapshots.rs` (the new multi-row snapshot test).

---

## v1.5a — pairs strategy smoke

Scope extension for **T_FINAL_B_v15a** (V8 from
[v15a-mean-reversion-pairs.md → Verification](../../v15a-mean-reversion-pairs/feature.md#verification)).
This is a **negative-confirmation drill** for R11 — the v0 multi-row
positions panel and the v0.5 strategies panel already render the v1.5a
steady state without a widget code change. T719 ships a new fixtures
preset; zero new strings, zero new theme tokens.

The fixture `ui::fixtures::fake_cockpit_v15a_pairs_steady_state()`
(T719) is tuned to the canonical 3-pair config from `pairs_mr_h1.toml`
(T714). Per architecture.md Q3 (formulation C), only the long `a` legs
of each pair appear on-book; the would-have-shorted `b` legs surface as
`pair_short_observation` rows in the recent-events footer with zero
money columns.

Pairs (lex-sorted `BTreeMap<PairKey, _>` iteration per R9.3):

- `(BTCUSDT, ETHUSDT)` → BTCUSDT long-leg position (`POS` green, +$225)
- `(BNBUSDT, BTCUSDT)` → BNBUSDT long-leg position (`FG_MUTED` flat, $0)
- `(ETHUSDT, SOLUSDT)` → ETHUSDT long-leg position (`NEG` red, −$300)

The matching strategies row is `pairs_mr_h1`, kind
`mean_reversion_pairs` (rendered via the strategy-id and source path),
`Holds position = yes`, signals/60s = 6 (most recent 3 entries × 2
emitted Signals — the long-leg `OpenPairLong` plus the
`PairShortObservation`).

### Sandbox-verifiable gates (automated) — v1.5a extension

| Gate | Command | What it checks |
|------|---------|----------------|
| Build (fixtures, v1.5a portfolio) | `cargo build -p ui --bin cockpit --features fixtures` | Cockpit boots against `fake_cockpit_v15a_pairs_steady_state()` (T719) — the new default fixture for the cockpit binary. |
| Multi-pair snapshot | `cargo test -p ui` | New `panel_snapshots__cockpit_v15a_pairs_steady_state` snapshot pins three long-leg position rows + one strategies row + recent-events footer covering both v1.5a `StrategyEventKind` variants. Total ≥ 59. |
| Consistency audit | `cargo test -p ui` | `no_inline_user_visible_strings_in_widgets` + `no_inline_hex_colors_in_widgets_or_state` still zero on every widget (no widget edits in v1.5a). |
| Workspace no-regression | `cargo test --workspace` | All v0 + v0.5 + v1 + v1.5a backend tests still green; `pairs-2023-zscore-mr` = `90591a0e…` and `pairs-2024-h1-zscore-mr` = `14f50a59…` body-SHA256 anchors hold. |

All gates above must be green before running the manual steps.

### Manual steps — fixtures walkthrough (multi-pair steady state)

1. [ ] Terminal 1: `cargo run --bin cockpit --features fixtures`.
   - Window opens; right column (top-to-bottom) shows the strategies
     panel with **one row** (`pairs_mr_h1`), then the positions panel
     with **three rows** (BTCUSDT, BNBUSDT, ETHUSDT — lex-sorted `a`
     legs of the three pairs), then the live tape with mixed buy/sell
     fills.
   - `[ ]` screenshot-v15a-pairs-steady-state.png _deferred_manual_
2. [ ] Inspect the positions panel rows (formulation-C invariant):
   - BTCUSDT row: `P&L` reads `+225.00` in `POS` green; `P&L %` reads
     `+1.25%` also in `POS`. This is the long leg of `(BTCUSDT,
     ETHUSDT)`.
   - BNBUSDT row: `P&L` reads `0.00` in `FG_MUTED` neutral; `P&L %`
     reads `0.00%` also in `FG_MUTED`. Long leg of `(BNBUSDT, BTCUSDT)`.
   - ETHUSDT row: `P&L` reads `-300.00` in `NEG` red; `P&L %` reads
     `-1.64%` also in `NEG`. Long leg of `(ETHUSDT, SOLUSDT)`.
   - **Crucially:** no SOLUSDT row, no short rows of any kind. Per
     formulation C, only the `a` leg of each pair trades; the `b` leg
     is observation-only and surfaces in the recent-events footer
     (next step), not on the positions panel.
3. [ ] Inspect the strategies panel:
   - Single row id `pairs_mr_h1`, short hash `90591a0`, status pill
     `Ready` in `POS` green, last-event `loaded`, `Signals / 60s = 6`,
     `Holds position = yes`.
   - Recent-events footer (newest first): three observation rows in
     `FG_MUTED` (the v1.5a `PairShortObservation` + `MeanReversionStop`
     kinds map to the `STRATEGIES_EVENT_LOAD` label per the widget's
     `match` arm — Q8 informational kinds, not control events) plus
     one `loaded` row in `ACCENT` for the canonical Load. Total: 4 rows.
   - The widget's exhaustive `match` over `StrategyEventKind` (Load /
     Swap / Unload / Reject / RebalanceRejected / MeanReversionStop /
     PairShortObservation) renders without panic.
4. [ ] Spot-check theme-token contract (no inline hex / no inline
   strings introduced by v1.5a):
   - `cargo test -p ui` — consistency audits stay green.
   - The recent-events footer renders the new Q8 kinds with the
     existing `STRATEGIES_EVENT_LOAD` constant (no new strings) in
     `color::FG_MUTED` (no new theme tokens). The visual signal of
     "informational, not a control transition" comes from the muted
     color alone — pairs with the existing `loaded` / `swapped` /
     `rejected` color map.
5. [ ] Resize the window narrower (drag the right edge inward until
   the positions panel hits its minimum width). Expected behavior:
   the panel's inner `Scrollable` keeps the rows clipped cleanly to
   the panel frame; row text remains right-aligned monospaced; no
   text overflows the panel border. Same as v1's R11 multi-row drill —
   v1.5a adds zero new layout surface.
6. [ ] Quit the binary (`Cmd+Q` on macOS).

> Why deferred PNG: the sandbox is headless. Capture instructions in
> the deferred-PNG table below.

### Acceptance for T_FINAL_B_v15a

- [ ] Multi-pair fixtures walkthrough performed (step 1 above);
  three long-leg position rows + one strategy row visible under
  `cargo run --bin cockpit --features fixtures`.
- [ ] Formulation-C invariant validated (step 2): positions panel
  shows ONLY long-leg rows on the `a` legs of each pair. No
  short-leg rows appear, even though `pair_short_observation`
  events are visible in the recent-events footer.
- [ ] Strategies panel shows one row for `pairs_mr_h1`,
  `Holds position = yes`, recent-events footer carries both
  `MeanReversionStop` and `PairShortObservation` kinds (step 3).
- [ ] Theme-token / strings contract intact (step 4): zero new
  `ui::strings` constants, zero new `ui::theme` tokens added by
  v1.5a — the new event kinds map onto existing copy + colors.
- [ ] Resize/clip behavior sane (step 5): rows clip inside the
  panel via the existing `Scrollable`; no overflow into adjacent
  panels. Same widget code path as v0/v1 per R11 negative
  confirmation.
- [ ] Automated gates green: `cargo fmt -p ui -- --check`,
  `cargo clippy -p ui --all-targets --all-features -- -D
  warnings`, `cargo test -p ui` (≥ 59, including the new
  `cockpit_v15a_pairs_steady_state` snapshot), `cargo test -p ui
  --features live` (≥ 71), workspace consistency audits.
- [ ] Workspace no-regression: `cargo test --workspace` green;
  the seven v0/v0.5/v1 anchor reports + the two new v1.5a anchor
  reports (`pairs-2023-zscore-mr` = `90591a0e…`,
  `pairs-2024-h1-zscore-mr` = `14f50a59…`) byte-identical across
  two runs at seed `0xC0FFEE`.
- [ ] Deferred PNG `screenshot-v15a-pairs-steady-state.png`
  captured on the operator display and committed under
  `spec/<slug>/reports/screenshots/v15a-mean-reversion-pairs/` (or
  appended to the v0 dir per the v1 sibling pattern;
  ui-designer's call on PR review).
- [ ] ui-designer signoff: no widget code changed for v1.5a (R11
  negative confirmation). The only diffs in `crates/ui/` are in
  `fixtures.rs` (new v1.5a presets), `bin/cockpit.rs`
  (default-fixture wiring updated to the v1.5a steady state),
  and `tests/panel_snapshots.rs` (new multi-pair snapshot test).
  No new `ui::strings` constants, no new `ui::theme` tokens.

### Deferred PNG list — v1.5a additions

Captured on the operator's workstation, saved into
`spec/<slug>/reports/screenshots/v15a-mean-reversion-pairs/` (sibling of
the v0 / v1 dirs):

| File | How to capture |
|---|---|
| screenshot-v15a-pairs-steady-state.png | `cargo run --bin cockpit --features fixtures`; cockpit full-window. Captures the 3 long-leg position rows + the `pairs_mr_h1` strategy row + the recent-events footer carrying both v1.5a `StrategyEventKind` variants. |
