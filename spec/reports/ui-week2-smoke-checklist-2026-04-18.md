---
generated: 2026-04-18
author: ui-designer
feature: v0-paper-sma
scope: T_FINAL_B — cockpit smoke + kill-switch drill
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
[`spec/reports/screenshots/v0-paper-sma/README.md`](screenshots/v0-paper-sma/README.md).

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
> `spec/reports/screenshots/v0-paper-sma/` with the filenames above.

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
  `spec/reports/screenshots/v0-paper-sma/README.md` (compacted from the
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
into `spec/reports/screenshots/v0-paper-sma/`:

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
