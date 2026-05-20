---
slug: live-cockpit-unified
status: shipped
owner: tester
updated: 2026-05-02
version: 1.5.0
---

# Live cockpit — unified single-process binary

## Why

The current two-binary live path is **dishonest**. The
[v0-paper-sma → "What wired up"](../v0-paper-sma/feature.md) section
(lines 1338–1400) documents the deferral: v0 wired the cockpit's
`ui::live::subscription` against an `Arc<EventBus>` shared inside one
process — but no production binary actually creates that shared
process. Today an operator who runs

```
cargo run --bin cockpit --features live
```

gets a cockpit that **constructs its own empty `EventBus`** at boot
([crates/ui/src/bin/cockpit.rs:67–69](../../crates/ui/src/bin/cockpit.rs)),
which has no publisher, so every panel sits in `Loading` forever.
The companion process

```
cargo run --bin trading --config config/agent.toml
```

runs the full backend (broadcast bus + all tokio tasks) but its bus
is a *different* `Arc<EventBus>` — the two never meet.
[v0-paper-sma.md lines 1437–1441](../v0-paper-sma/feature.md) explicitly defers
the unified binary to "v0.5"; v0.5, v1, and v1.5a all shipped without
landing it. The
[ui::strings::CONNECTION_AGENT_UNREACHABLE](../../crates/ui/src/strings.rs)
copy is present and points the operator at the missing wiring, but
there is no reachable code path that would unfreeze the panel — the
unified binary is the only fix.

**What the operator can do today (live):**

- Run `cargo run --bin trading --config config/agent.toml` headless
  and watch the JSON log + Prometheus `:9100` for fills, kill-switch
  trips, uptime heartbeats (T806), feed reconnects (T805), kill-switch
  audit dual-writes (T809), and (opt-in) in-process cron (T810).
- Run `cargo run --bin cockpit --features fixtures` for a layout
  smoke against the latest fixture preset. Panels populate from
  `ui::fixtures::fake_cockpit_v15a_pairs_steady_state()`. **No live
  agent is involved.**

**What the operator cannot do today:** see live agent state in the
cockpit. There is no code path from the running agent's bus to a
running cockpit's `Subscription`. The `--features live` flag compiles
the subscription module but boots an empty bus — pre-T32 honest
behavior the developer flagged at handoff time.

**Why fix it now.** Five operator-success-reports landings in Wave 1
(T805 feed-reconnect, T806 uptime intervals, T809 kill-switch
dual-write + incident spawn, T810 in-process cron) all assume an
operator is *watching* the agent. The intended cockpit signal —
mode banner, latency badge, fills tape, P&L sparkline, kill-switch
button — is the human readout for those backend signals. Without
the unified binary the operator falls back to `tail -f` on the
JSON log; the cockpit-as-built becomes a fixtures demo, which is
not what it was specced for.

The fix is plumbing-only: a single binary that builds an
`Arc<EventBus>`, hands one clone to the agent's tokio tasks
(producers) and another to the iced cockpit (consumer via
`ui::live::subscription`), and reconciles the shutdown / kill-switch
/ uptime-ledger paths so neither side leaks.

## Requirements (R-items)

The numbered list is exhaustive on the boring parts. Where wording
implies a choice (which crate, which name) the choice is recorded
as an Open Question instead.

- **R1 — single binary, both halves.** A single `cargo run` command
  starts the agent backend (`#[tokio::main]` workload from
  `crates/agent/src/main.rs`) **and** the iced cockpit
  (`crates/ui/src/bin/cockpit.rs` view + subscription) in one OS
  process, sharing one `Arc<EventBus>`.

- **R2 — shared bus.** The cockpit subscribes via the existing
  `ui::live::subscription(Arc<EventBus>)` entry point. No new
  channel types; the bus surface stays as documented in
  [crates/agent/src/bus.rs](../../crates/agent/src/bus.rs) and
  [architecture.md → v0.5 broadcast bus extensions](../architecture.md#v05--broadcast-bus-extensions-q5--confirmed-2026-04-19).
  The fix is wiring, not API.

- **R3 — graceful shutdown, both directions.**
  - **Ctrl-C in terminal** → cancel agent tokio tasks → close the
    iced window cleanly.
  - **Operator closes the iced window** → cancel agent tokio tasks
    so they don't leak past the GUI.
  Both paths must (a) cancel the uptime heartbeat task, (b) write the
  T806 close-uptime-interval row, (c) flush any pending audit DB
  writes, (d) drop the `Arc<EventBus>` so the cockpit's subscription
  drains via `RecvError::Closed` (the existing
  `CONNECTION_CHANNEL_CLOSED` copy is the user-visible terminal
  state if the GUI lingers). Target: clean exit in **< 2s** from
  shutdown signal.

- **R4 — kill-switch unified.** When tripped via either path, both
  sides observe the same trip:
  - **File-watch trip** (operator `touch ops/.halt`) →
    `KillSwitch::trip(HaltReason::HaltFile)` → broadcast on
    `mode` channel → cockpit's `Message::AgentHaltedExternally`
    sets the halted banner.
  - **Cockpit "Flatten & Halt" button** → `Message::KillConfirmed`
    must call `KillSwitch::trip(HaltReason::ManualOperator)` on the
    *same* `Arc<KillSwitch>` the agent owns, so the trip flows
    through the existing T809 dual-write (audit memo +
    `strategy_events` row + incident-report spawn) and then comes
    back as a mode broadcast that lights up the cockpit's halted
    banner.
  Today the cockpit button only mutates `KillState::Flattening`
  in the UI model
  ([crates/ui/src/state.rs:397–402](../../crates/ui/src/state.rs)) —
  it does not call into `KillSwitch::trip`. The unified binary must
  close that gap.

- **R5 — uptime ledger writes (T806) survive both shutdown paths.**
  The agent currently opens an uptime interval at startup, runs a
  30s heartbeat task, and closes the interval on Ctrl-C
  ([crates/agent/src/main.rs:117–145, 315–321](../../crates/agent/src/main.rs)).
  In the unified binary the same writes must fire — and the
  close-interval write must specifically survive the
  iced-window-closed path, which today doesn't exist as a shutdown
  trigger anywhere in the agent. The new shutdown reconciler
  guarantees `close_uptime_interval` runs before the process exits
  whether the trigger is Ctrl-C or window close.

- **R6 — feed-reconnect (T805) still emits when running unified.**
  The Binance reconnect handler in `crates/data/` calls
  `audit::journal::feed_reconnect`. That call site is unchanged by
  this feature — the unified binary just needs to keep it running
  (i.e. the data tokio task is still spawned and not cancelled
  prematurely by the iced lifecycle).

- **R7 — Prometheus `:9100` still served, with an off switch.**
  The `metrics-exporter-prometheus` HTTP listener at
  `0.0.0.0:9100`
  ([crates/agent/src/observability.rs:96–104](../../crates/agent/src/observability.rs))
  still starts in the unified binary. **New requirement:** a config
  knob to disable the listener for the unified-binary case (some
  operators run the cockpit on a laptop where binding `:9100`
  publicly is wrong). Default behavior unchanged.

- **R8 — fixtures mode preserved, additive only.**
  `cargo run --bin cockpit --features fixtures` still works exactly
  as it does today (fixture-driven layout smoke, no agent process
  required). The unified binary is **additive**, not a replacement
  — the fixtures path remains the canonical layout-snapshot driver
  and the development entry point with no exchange dependency.

- **R9 — CLI surface.** The unified binary takes the agent's CLI
  flags (at minimum `--config <path>` and `--mode {research|paper}`)
  and any cockpit-specific flags. Recommended invocation shape
  (architect to confirm in Q1):
  ```
  cargo run --release --bin <unified> -- --config config/agent.toml
  ```
  Whatever name and crate the architect picks, it must not collide
  with the existing `trading` (agent bin) or `cockpit` (cockpit bin)
  names — both stay live for backwards compatibility (R-V6, R-V7).

- **R10 — every existing feature flag still builds clean.**
  `cargo build --workspace --all-targets` (default features),
  plus `cargo build -p ui --features fixtures`,
  `cargo build -p ui --features live`,
  `cargo build -p agent --features in_process_cron` (T810), all
  remain green. Adding the unified binary must not flip any of those
  to red.

- **R11 — cancellation token plumbing.** A single
  `tokio_util::sync::CancellationToken` (or one root + scoped
  children) is the shutdown primitive. Agent tasks already model
  this for the funding poller and uptime heartbeat. Extend to
  every long-running task spawned by the unified binary so the
  iced-window-close pathway has a single thing to cancel.

- **R12 — observability of shutdown ordering.** Each lifecycle
  transition (`unified_started`, `bus_attached`, `cockpit_window_open`,
  `cockpit_window_closed`, `agent_tasks_cancelled`,
  `uptime_interval_closed`, `unified_exited`) emits a structured
  `tracing::info!` line so a tester or operator can read the JSON
  log and confirm the order. No new metrics; reuse the existing
  `kill_switch_trips_total` counter for kill-switch trips
  regardless of trigger source.

- **R13 — config sourcing.** The unified binary reads the agent's
  `config/agent.toml` for backend params (existing) plus whatever
  cockpit-side knobs the architect surfaces (likely none — the
  cockpit has no config file today). Open Question Q4 records the
  decision; default assumption is "single config file, agent's
  schema, cockpit reads from the same struct".

- **R14 — strategies panel still wires.** The `strategy_loaded`,
  `strategy_swapped`, `strategy_error` channels are already published
  by the agent's `run_strategy_watcher`
  ([crates/agent/src/watcher.rs:267,375,400](../../crates/agent/src/watcher.rs))
  and consumed by `ui::live::subscription`. The unified binary
  inherits both ends; no extra work, but the brief calls it out so
  the architect doesn't drop the watcher task by accident when
  picking the runtime topology (Q2).

- **R15 — no anchor regressions.** None of the locked
  `spec/anchors.toml` body-SHAs cover the `agent` or `ui` crates.
  Confirmed by a read of
  [spec/anchors.toml](../anchors.toml). The 11 entries cover
  backtest report rendering and `report-sample-*` artifacts, none
  of which this feature touches. The anchor gate stays green by
  construction; V5 verifies.

## Verification (V-items)

- **V1 — end-to-end smoke.** `cargo run --release --bin <unified> --
  --config config/agent.toml --mode paper` boots. Within 5s the
  cockpit window appears; within ~60s (one bar close on Binance
  spot 1m) the P&L and tape panels move from `Loading` to `Ready`
  as the agent publishes the first events. The latency badge
  updates from ticks. The strategies panel shows the SMA crossover
  row populated by the watcher's `StrategyLoaded` event.

- **V2 — kill-switch parity.**
  - **V2a — file-touch trip.** `touch ops/.halt` → cockpit halted
    banner appears within 1s; `kill_switch_trips_total` counter
    increments; T809 dual-write to `audit_memo` +
    `strategy_events.KillSwitchTripped` confirmed via
    `audit::query`; incident report spawned.
  - **V2b — cockpit-button trip.** Operator clicks Flatten & Halt,
    types the safety phrase, confirms → agent receives a
    `KillSwitch::trip(HaltReason::ManualOperator)` call → same
    dual-write + incident-spawn path → cockpit halted banner lights
    up via the resulting `mode` broadcast (closes the loop).
    Counter increments.

- **V3 — graceful shutdown timing.**
  - **V3a — Ctrl-C.** Send SIGINT → process exits in `< 2s`. The
    audit DB shows a `close_uptime_interval` row matching the boot
    UUID. No orphan tokio tasks (verified by tracing the
    `agent_tasks_cancelled` line followed by `unified_exited`).
  - **V3b — window close.** Click the iced window's close button
    → process exits in `< 2s`. Same uptime-close + no-orphans
    invariant.
  - **V3c — kill-switch then exit.** Trip kill switch, wait for
    halted banner, then close the window → process still exits
    cleanly with the close-uptime row written.

- **V4 — regression: existing test matrix stays green.**
  - `cargo test --workspace --all-targets` — PASS.
  - `cargo test -p ui --features fixtures` — PASS (43 tests).
  - `cargo test -p ui --features live` — PASS (53 tests, including
    the 3 `tests/live_subscription.rs` integration tests).
  - `cargo test -p agent` — PASS, including
    `tests/in_process_cron.rs` if the feature flag is exercised.
  - `cargo fmt --all -- --check` — PASS.
  - `cargo clippy --workspace --all-targets --all-features --
    -D warnings` — PASS.

- **V5 — anchor regression.** `scripts/verify_anchors.sh` →
  PASS 11/11. None of the anchored bytes change; this is plumbing
  outside the anchored crates (`backtest`, `audit`, `reports`).

- **V6 — backwards compat: cockpit fixtures.**
  `cargo run --bin cockpit --features fixtures` boots into the
  fake-steady-state layout exactly as it does today. No broken
  defaults, no compile errors with `--features fixtures` alone.

- **V7 — backwards compat: headless agent.**
  `cargo run --bin trading -- --config config/agent.toml` boots
  exactly as it does today (no GUI, JSON log on stdout, Prometheus
  on `:9100`). Operators who deploy headlessly (e.g. server-side,
  or in CI) keep their existing entry point.

- **V8 — feed-reconnect smoke.** Kill the network briefly while
  the unified binary is in `paper` mode → on reconnect, the
  Binance handler writes a `feed_reconnect` row to
  `strategy_events`; the audit query returns the row; the latency
  badge briefly drops to `STALE` then recovers. (T805 regression
  guard.)

- **V9 — uptime heartbeat smoke.** Run the unified binary for
  ≥ 90s → `agent_uptime` table contains: 1 open row, ≥ 2 heartbeat
  rows, 1 close row keyed on the same boot UUID. (T806 regression
  guard.)

- **V10 — Prometheus toggle.** With the new "disable Prometheus"
  config knob set, `curl :9100/metrics` returns connection refused
  and the agent log shows the listener was skipped. With the knob
  unset (default), `curl :9100/metrics` returns the same metrics
  surface as today.

## Backtest scenarios

_n/a — plumbing feature, no new backtest scenarios._

## Open questions for architect

The questions below are the decisions the architect must resolve
before development. Each one has a default the analyst recommends
in parentheses; the architect may override.

- **Q1 — binary name + crate placement.**
  Options:
  - **(a) `crates/agent/src/bin/<unified>.rs`** — re-uses the
    agent's `[bin] name = "trading"` slot; the new bin pulls in
    `ui` as a library dep. **Tradeoff:** `agent` would gain a `ui`
    dep, which today is one-way (`ui → agent`) — flipping that
    creates a cycle unless `ui` is split into a `ui-lib` (panels,
    state, live) and a `ui-bins` (cockpit, viewer) shape. Likely
    forces a small refactor.
  - **(b) `crates/ui/src/bin/<unified>.rs`** — re-uses the
    cockpit's bin slot; the new bin pulls in `agent` as a library
    dep, which is **already** how `--features live` works. No
    cycle, no refactor. **Tradeoff:** the agent's main-loop logic
    (config load, ledger open, observability bootstrap, kill
    switch, uptime, watcher, funding poller, feed) currently
    lives in `crates/agent/src/main.rs` — needs extraction into a
    library function (`agent::run(cfg, bus, kill_switch,
    cancel_token) -> JoinSet`) so the unified bin can call it
    instead of duplicating it.
  - **(c) New `crates/cockpit-live/`** — depends on both `agent`
    and `ui`. Cleanest separation, but adds a workspace member
    for one binary; likely overkill unless the architect wants
    to keep the option open for a future "headless trading-bot
    + remote viewer" topology.

  **Analyst default:** (b) + extract `agent::run(...)`. Lowest
  refactor cost. Bin name suggestion: `trading-cockpit`. Architect
  picks the final name.

- **Q2 — tokio runtime topology.**
  iced 0.14 owns the main thread (its `iced::run` /
  `iced::application(...).run()` blocks the calling thread and
  drives an internal event loop). The agent uses
  `#[tokio::main(flavor = "multi_thread")]` (implicit; `tokio.workspace`
  default). The two cannot both hold the main thread.
  Options:
  - **(a)** Build a `tokio::runtime::Runtime` explicitly, hand
    it to a side thread (`std::thread::spawn(move ||
    rt.block_on(agent::run(...)))`), then run iced on the main
    thread. Bus is `Arc<EventBus>` — thread-safe by construction
    (broadcast channels are `Send + Sync`).
  - **(b)** Use iced's `executor` config to bind iced to the
    tokio runtime (iced 0.14 supports providing a custom
    `Executor`). Keep one `tokio::main` and let iced run on a
    spawned task — but iced still needs the main thread on
    macOS / Linux for its windowing system.
  - **(c)** Use `iced::Subscription`'s internal pump to call into
    tokio via the existing `from_recipe`/`stream!` machinery
    (which `ui::live` already does); spawn the agent's tasks on
    a manually-constructed runtime hosted on a side thread.

  **Analyst default:** (a). It's the simplest and least clever:
  iced owns the main thread; tokio multi-thread runtime owns a
  worker thread pool; bus + cancellation token are the only
  shared state. macOS in particular *requires* GUI work on the
  main thread — option (a) respects that without contortion.

- **Q3 — shutdown ordering.**
  Two designs:
  - **(a) iced-led.** iced window close emits a final `Message`
    that triggers `cancel_token.cancel()`. The side-thread
    runtime sees cancellation, joins all agent tasks, writes
    the close-uptime row, then exits. Main thread joins the
    side thread and returns from `iced::run`. Process exits.
  - **(b) bus-led.** iced window close drops the `Arc<EventBus>`
    on the cockpit side; the agent side detects all-receivers-
    dropped (or a separate shutdown channel) and shuts down. The
    bus broadcast channels don't natively signal "all receivers
    dropped to the publisher" — would need a side `oneshot` or
    `Notify`.

  **Analyst default:** (a). One token, one cancel call, all
  tasks cooperative. Matches the existing pattern
  ([crates/agent/src/main.rs:123,316](../../crates/agent/src/main.rs)).

- **Q4 — config sourcing.**
  Options:
  - **(a)** Single `config/agent.toml` — cockpit has no
    settings-file footprint today; if the architect wants any
    new keys (`prometheus_enabled = true`, `cockpit.refresh_ms =
    250`, etc.) they go on the agent's `Config` struct.
  - **(b)** Two files — `config/agent.toml` + `config/cockpit.toml`,
    merged at startup. Less coupling but more files to maintain.

  **Analyst default:** (a). Agent's `Config` is the only config
  type today; adding a `[cockpit]` section to it (if and when
  any cockpit knobs land) keeps one source of truth. The R7
  Prometheus toggle goes on `[observability]`, not a new section.

- **Q5 — `in_process_cron` (T810) interaction.**
  The agent's `in_process_cron` feature is opt-in and
  forwards to `agent::cron::start`. In the unified binary should
  this be:
  - **(a)** Off by default (matches the agent today); the unified
    binary exposes the same `--features in_process_cron` switch
    on its bin spec, and operators who want it opt in.
  - **(b)** On by default in the unified binary because operators
    running an interactive cockpit are by definition not running
    a separate cron / launchd, and want everything in one place.

  **Analyst default:** (a). The whole point of the flag at T810
  was opt-in; flipping the default for the unified binary breaks
  the "default unchanged" contract documented at
  [architecture.md → architectural deltas summary](../architecture.md#v1-architectural-deltas-summary).

- **Q6 — kill-switch unification.**
  Today the agent owns a real `KillSwitch` (file-watch, audit
  dual-write, incident spawn). The cockpit owns a *UI-only*
  `KillState` machine that doesn't actually call into
  `KillSwitch::trip` — its "Flatten & Halt" button just sets
  `KillState::Flattening`. In the unified binary:
  - **(a)** Single shared `Arc<KillSwitch>` — cockpit's
    `Message::KillConfirmed` calls `kill_switch.trip(HaltReason::
    ManualOperator)`, which then triggers T809's dual-write +
    spawn + mode broadcast. Banner lights up via the same
    feedback loop as the file-watch trip. **Recommended.**
  - **(b)** Two distinct kill switches — UI button only sets
    UI state, agent only watches the file. Keeps the current
    asymmetry. **Not recommended** — it leaves R4 unsolved.

  **Analyst default:** (a). This is the right time to close the
  cockpit-button-not-actually-killing gap.

- **Q7 — keep the two-binary path alive?**
  Once the unified binary lands, the deferred two-binary path
  (separate `cockpit --features live` + `trading` agent) becomes
  vestigial. **Operational scenarios where two-binary still makes
  sense:** cockpit running on a separate workstation watching a
  remote agent over IPC. The product is locked single-operator /
  single-machine
  ([product.md → Project scope boundary](../product.md)),
  so this scenario is not in scope today. **Recommendation:**
  remove the standalone `cockpit --features live` codepath
  (which today only constructs an empty bus and sits in
  `Loading`) and update
  [v0-paper-sma.md lines 1338–1400 + 1437–1441](../v0-paper-sma/feature.md)
  with a "superseded by live-cockpit-unified" pointer. Architect
  may instead keep the path alive as a no-op for symmetry; the
  cost is one chunk of dead `cfg(feature = "live")` code in the
  cockpit bin.

- **Q8 — UI-designer touchpoints.**
  The cockpit view, panels, strings, and theme tokens are
  unchanged: `ui::live::subscription` already produces the right
  `Message`s; `state::update` already handles them; no new
  panel, no new copy. **Two minor possible exceptions:**
  (i) the `CONNECTION_AGENT_UNREACHABLE` string, currently
  unreachable, becomes reachable if `EventBus::new` panics at
  startup (architect can wire it; if not, the existing copy
  stays as future-proofing); (ii) the kill-switch button's
  hover-tooltip might want a new line "tripping this halts the
  trading agent and writes an incident report" — true today in
  spec but only true in code with R4. Architect: confirm `## UI`
  section is empty (no UI-designer round needed) or surface a
  one-line ask.

  **Analyst default:** no UI-designer round. R4's "button calls
  KillSwitch::trip" is plumbing, not new UX.

## Design

This Design section resolves the eight Open Questions, ratifies the
analyst's two findings, and locks the runtime topology + test strategy
+ risk register the developer + tester will work to. Task numbers
quoted here are forward references into
[spec/live-cockpit-unified/tasks.md](../tasks/live-cockpit-unified.md).
Anchor IDs (T9xx) are taken from the namespace freed by
[operator-success-reports tasks](../tasks/operator-success-reports.md)
which used T801–T817.

### Q-resolution summary

| Q  | Decision (one line) | Notes |
|----|---------------------|-------|
| Q1 | **(b) `crates/ui/src/bin/cockpit_live.rs`** + extract `pub async fn agent::run(RunHandles, CancellationToken) -> Result<()>` | Bin name `cockpit_live` (not analyst's `trading-cockpit`) — overrides analyst default; rationale below. |
| Q2 | **(a) iced on main, multi-thread tokio runtime hosted on a side thread** — matches analyst default. | macOS forces GUI on main thread; bus + `KillSwitch` shared via `Arc`. |
| Q3 | **iced-led, `tokio_util::sync::CancellationToken`** — matches analyst default. | Ctrl-C handler installed by the **iced** thread (`signal_hook::iterator`) bridges to the same token. 2 s wall-clock bound. |
| Q4 | **Single `config/agent.toml`** — matches analyst default. | New `[observability].prometheus_enabled: bool` (default `true`); no `[cockpit]` section in v1 (no UI knobs surface yet). |
| Q5 | **`in_process_cron` opt-in unchanged** — matches analyst default. | New bin specs `default = []`; operators opt in with `--features in_process_cron` on the cockpit_live bin too. |
| Q6 | **Single shared `Arc<KillSwitch>`** — matches analyst default. | Cockpit's `Message::KillConfirmed` calls `kill_switch.trip(HaltReason::ManualOperator)` via a new `Cockpit::kill_switch: Option<Arc<KillSwitch>>` field; preserves T809 dual-write end-to-end. |
| Q7 | **Remove `cockpit --features live` once `cockpit_live` lands** — matches analyst default. | Keep the headless `trading` bin and `cockpit --features fixtures`. The `--features live` path is dead-code-only today (empty bus); deletion is a net spec-honesty win. |
| Q8 | **Zero new UI surface** — matches analyst default. | `## UI` will read one line: "no new UI surface; existing live wiring is sufficient". The kill-button hover-tooltip update is a one-string-constant change tracked under T907 ([ui-designer]). |

**Bus-wiring scope: IN-SCOPE.** Analyst finding #1 (`Arc<EventBus>`
created at `crates/agent/src/main.rs:193` is not threaded through
data/exec/risk producers; only the watcher publishes today) is **part
of this feature, not a sibling feature.** R1 ("single binary that
runs both") is structurally incomplete unless the bus is actually
publishing — without it the operator boots the unified binary, sees
the iced window, and watches every panel sit in `Loading` forever.
That is the *exact failure mode* the feature exists to delete.
Scope split would only delay V1 by one round of architect→tester
ceremony for no architectural payoff. Concrete producer wiring is
specified in the **Bus producer wiring (six channels + three v0.5
strategy lifecycle channels)** subsection below; the developer tasks
T903a–T903d carry it.

### Q1 — binary placement, name, `agent::run` extraction

**Decision:** option (b) — new binary at
`crates/ui/src/bin/cockpit_live.rs`. Extract a public
`pub async fn agent::run(handles: RunHandles, cancel: CancellationToken) -> anyhow::Result<()>`
from `crates/agent/src/main.rs`. Both the headless `trading` bin and
the unified `cockpit_live` bin call into it.

**Rationale.** The `ui → agent` dep already exists (the `live`
feature on the `ui` crate pulls `agent` as an optional dep:
[crates/ui/Cargo.toml lines 32–37](../../crates/ui/Cargo.toml)).
Reusing it is a one-line `Cargo.toml` change. The reverse direction
(`agent → ui`) would require splitting the `ui` crate into `ui-lib`
+ `ui-bins` to avoid a cycle (because `crates/agent/src/main.rs`
currently lives in the `agent` crate, which `ui` imports). Option
(b) is therefore the lowest-refactor-cost path. Option (c) (new
`crates/cockpit-live/`) was rejected because the workspace already
has the right shape — adding a fourth bin-hosting crate for a
single binary is overhead without payoff, and the speculated
"headless trading-bot + remote viewer" topology is locked
out-of-scope by [product.md → Project scope boundary](../product.md).

**Bin name override:** `cockpit_live` instead of analyst's
`trading-cockpit`. Three reasons:
1. It mirrors the existing `cockpit` bin (same crate, same prefix)
   so `cargo run --bin cockpit_live` is parallel to `cargo run --bin
   cockpit` — the operator's muscle memory carries over.
2. The `_live` suffix is honest about *what changed* relative to the
   default `cockpit` bin (subscription is now wired to a real bus),
   and it does not collide with the existing `trading` agent bin name.
3. Avoids the hyphen-vs-underscore split (`trading-cockpit` would be
   the only hyphenated bin in the workspace; `viewer` and `cockpit`
   are unhyphenated).

**`agent::run` signature.** New module `crates/agent/src/runtime.rs`
exposes:

```rust
/// Subsystems handed to `agent::run` as already-constructed handles.
/// Both the headless `trading` bin and the unified `cockpit_live`
/// bin construct these and pass ownership in.
pub struct RunHandles {
    pub config: Arc<crate::config::Config>,
    pub ledger: Arc<audit::Ledger>,
    pub bus: Arc<crate::EventBus>,
    pub kill_switch: Arc<crate::KillSwitch>,
    pub registry: Arc<strategy::StrategyRegistry>,
    /// Boot UUID — used by `open_uptime_interval` /
    /// `close_uptime_interval`. Generated by the caller so the same
    /// id flows through tracing for both shutdown paths (R12).
    pub boot_id: String,
}

/// Run all agent tokio tasks (data feed, strategy watcher, funding
/// poller, uptime heartbeat, kill-switch file watcher, cron if
/// enabled) until `cancel` is tripped or the kill switch flips to
/// `Halted`. Returns Ok(()) on graceful shutdown.
///
/// Caller responsibilities:
/// - Construct `RunHandles` (config + ledger + bus + kill_switch +
///   registry + boot_id) before calling. The construction order
///   mirrors today's `crates/agent/src/main.rs` lines 50–195.
/// - Install a Ctrl-C handler that calls `cancel.cancel()` (the
///   headless `trading` bin does this; the unified `cockpit_live`
///   bin's iced shutdown handler does this).
/// - After `agent::run` returns, call `audit::journal::close_uptime_interval`
///   exactly once (R5). The function does NOT close the interval
///   itself, because the close write must observe the cancellation
///   AFTER all in-flight writes have flushed; the caller orders this.
pub async fn run(
    handles: RunHandles,
    cancel: tokio_util::sync::CancellationToken,
) -> anyhow::Result<()>;
```

The current `main.rs` collapses to ~70 lines:
1. parse CLI / load config / install tracing / install observability;
2. construct ledger, kill_switch, registry, bus, boot_id;
3. open the uptime interval;
4. install Ctrl-C handler that calls `cancel.cancel()`;
5. call `agent::run(handles, cancel).await`;
6. close the uptime interval; exit.

The cockpit_live bin reuses steps 1–3 + 6, replaces step 4 with the
iced `Subscription::run_with_id`-driven cancellation pump, and runs
step 5 on the side-thread runtime. Steps 4–6 are shared shutdown
plumbing factored into a new helper `agent::runtime::shutdown_writer(...)`
so the close-uptime write site has exactly one home (DRY across
both bins; preserves T806 invariant).

**Public API additions** (this feature):
- `agent::runtime::run(RunHandles, CancellationToken) -> Future<Output = anyhow::Result<()>>`
- `agent::runtime::RunHandles` struct (fields above)
- `agent::runtime::shutdown_writer(ledger, boot_id) -> Future<Output = ()>`
  — calls `audit::journal::close_uptime_interval` and warn-logs failures.
- Re-exports at the crate root: `pub use runtime::{run, RunHandles, shutdown_writer};`

No other public API additions. The bus, kill_switch, ledger,
registry, and config types stay shape-stable.

### Q2 — tokio + iced runtime topology

**Decision:** option (a) — iced owns the main thread; a multi-thread
tokio runtime is built explicitly and hosted on a side thread; the
`Arc<EventBus>` and `Arc<KillSwitch>` are constructed before either
runtime starts and shared via clone.

**Rationale.** macOS hard-requires GUI work on the main thread —
iced's `iced::application(...).run()` blocks the calling thread and
drives the windowing event loop. tokio cannot also own the main
thread, so the only contortion-free shape is: build a
`tokio::runtime::Builder::new_multi_thread().enable_all().build()?`,
stash the `Runtime` on a side `std::thread::spawn`, and let it
`block_on(agent::run(...))` until cancelled. The bus is a
`tokio::sync::broadcast` family — `Send + Sync` by construction — so
the iced subscription's `Recipe::stream(...)` polling pump (which
runs on iced's own internal thread pool, not the tokio runtime) can
hold receivers cloned from the same `Arc<EventBus>`.

**Rejected:** option (b) (custom `Executor`) — iced 0.14's `Executor`
trait is not designed to share a runtime across the GUI loop and
backend tasks; the GUI loop still needs the main thread on macOS,
so option (b) reduces to (a) plus glue. Option (c) (subscription-led
pump) — works but couples agent task lifecycle to the subscription's
poll cadence, which is brittle (an unmounted subscription would
silently kill the data feed). Option (a) decouples the two.

**Topology diagram:**

```
                ┌────────────────────────────────────────────┐
                │   main thread (macOS / Linux GUI thread)   │
                │                                            │
                │   ┌──────────────────────────────────┐     │
                │   │  iced::application(...).run()    │     │
                │   │   ├─ subscription:               │     │
                │   │   │    ui::live::subscription(   │     │
                │   │   │      Arc::clone(&bus))       │     │
                │   │   │   (Recipes hold broadcast    │     │
                │   │   │    receivers; iced internal  │     │
                │   │   │    pump drives them.)        │     │
                │   │   └─ on close → cancel.cancel()  │     │
                │   │       + side_thread.join()       │     │
                │   └──────────────────────────────────┘     │
                └──────────────┬─────────────────────────────┘
                               │ shared via Arc:
                               │   • Arc<EventBus>
                               │   • Arc<KillSwitch>
                               │   • CancellationToken (cheap clone)
                ┌──────────────▼─────────────────────────────┐
                │   side thread (std::thread::spawn)          │
                │                                            │
                │   tokio::runtime::Runtime (multi-thread)   │
                │     ├─ rt.block_on(agent::run(handles,     │
                │     │                          cancel))    │
                │     │     spawns:                          │
                │     │       • data feed task (publishes    │
                │     │         ticks, bars to bus)          │
                │     │       • paper engine task (publishes │
                │     │         fills, positions, pnl)       │
                │     │       • strategy watcher task        │
                │     │         (publishes strategy_*)       │
                │     │       • funding poller (optional)    │
                │     │       • uptime heartbeat task        │
                │     │       • kill-switch halt-file watcher│
                │     │       • cron scheduler (opt feature) │
                │     │       • mode-broadcast forwarder     │
                │     │         (kill_switch → bus.mode)     │
                │     └─ on cancel → JoinSet drains, returns │
                └────────────────────────────────────────────┘
```

The cancellation token is cheap to clone (`CancellationToken: Clone`)
and is shared three ways:
1. iced thread: drops it when the window closes / ctrl-c arrives;
2. side-thread runtime: every spawned task holds a child token via
   `cancel.child_token()` so cooperative shutdown is one
   `cancel.cancel()` call;
3. uptime heartbeat: same child token, so its 30 s tick aborts on
   shutdown before the close-interval write fires (matches today's
   `crates/agent/src/main.rs:123,316`).

The mode-broadcast forwarder (new task) subscribes to
`kill_switch.subscribe()` and forwards each `AgentMode` event into
`bus.publish_mode(...)`. Today that bridge does not exist —
`AgentMode` events go to `mode_rx` only inside `main.rs`'s shutdown
select. Without the forwarder, R4 cockpit-banner-after-trip cannot
fire. The forwarder is task **T905**.

### Q3 — shutdown ordering

**Decision:** iced-led, single `CancellationToken`, 2-second wall-clock
bound enforced by `tokio::time::timeout` wrapping the side-thread
join. Matches analyst default.

**Sequence (window-close path — V3b):**

1. iced emits its `Event::Window(WindowClosed)` event → caught by
   `Cockpit::subscription`'s window-event recipe.
2. `Message::ShutdownRequested` lands in `update`; calls
   `cancel.cancel()`.
3. iced's `update` returns `iced::Task::none()` and proceeds to its
   own teardown; the `iced::application(...).run()` call begins
   exiting on the main thread.
4. Side-thread runtime: every spawned task is cooperative on
   `cancel.cancelled()` (R11). The data feed exits its select arm,
   the strategy watcher drops its shutdown channel, the funding
   poller's child token fires, the uptime heartbeat tick is
   abandoned. All tasks tracked in a `tokio::task::JoinSet` so the
   runtime waits for them before `block_on` returns.
5. When `block_on` returns, the side thread calls
   `agent::runtime::shutdown_writer(ledger, boot_id).await` (which
   writes the T806 close-uptime row), then exits.
6. Main thread's `iced::run` returns. The bin's `main` joins the
   side thread (with a 2 s `std::thread::JoinHandle` deadline; on
   timeout we log `shutdown_deadline_exceeded` and exit anyway —
   force-abort).
7. Process exits.

**Ctrl-C path (V3a):** `tokio::signal::ctrl_c().await` runs **on the
side-thread runtime** (not the main thread; macOS routes SIGINT to
the process, both threads can `await` it). When ctrl_c fires, the
side thread calls `cancel.cancel()`. The iced thread observes the
cancellation token via a special `BusRecipe::Cancellation` recipe
that yields `Message::ShutdownRequested` when the token trips — at
which point the iced application calls `iced::window::close(...)`
and the V3a path collapses into the V3b path from step 3.

**Kill-switch-then-exit path (V3c):** kill switch trips →
`AgentMode::Halted` broadcast → mode-broadcast forwarder (T905)
publishes to `bus.mode` → cockpit's `stream_mode` yields
`Message::AgentHaltedExternally(reason)` → cockpit halted banner
appears. Operator then closes the window: V3b path executes. The
agent tasks are *already* in their cooperative-shutdown wind-down
because of the kill-switch trip (the existing `tokio::select!` in
the headless agent already breaks on `AgentMode::Halted`); the
unified binary inherits that behavior, so V3c finishes within the
same 2 s budget as V3b.

**Why not bus-led (option b).** broadcast channels do not natively
signal "all receivers dropped" back to the sender. We could bolt on
a `tokio::sync::Notify` for that, but it's strictly more moving
parts than option (a) and it conflates "subscription closed because
the cockpit is shutting down" with "subscription closed because the
operator clicked away from a panel" — which we do not want.

### Q4 — config sourcing

**Decision:** single `config/agent.toml`. Matches analyst default.

**Schema delta:** one new field on the existing `[observability]`
table:
```toml
[observability]
prometheus_listen   = "0.0.0.0:9100"
prometheus_enabled  = true   # NEW (R7) — defaults true; set false in
                             # the unified-binary case if the cockpit
                             # is on a laptop where binding :9100 is wrong
```
No new `[cockpit]` table — there are no cockpit-side knobs the
architect surfaces in v1. Theme, density, refresh cadence are all
compile-time constants in `crates/ui/src/theme.rs` today. Adding a
table for "future cockpit knobs that don't exist" violates YAGNI.
When a cockpit knob lands (e.g. the operator wants a non-Dark
theme), an additive `[cockpit] theme = "dark"` field on the existing
`Config` struct is a five-line patch — no migration is needed.

**Backwards compat:** `prometheus_enabled` defaults to `true` via
`#[serde(default = "default_true")]`, so existing
`config/agent.toml` files (without the field) keep their current
behavior. V10 verifies the toggle works in both directions.

### Q5 — `in_process_cron` (T810) interaction

**Decision:** opt-in unchanged. Matches analyst default.

The existing `agent` Cargo.toml feature `in_process_cron`
([crates/agent/Cargo.toml lines 16–20](../../crates/agent/Cargo.toml))
gates a runtime side-effect (the cron scheduler). The new
`cockpit_live` bin's Cargo.toml feature stanza inherits the same
gate by re-exporting it via the optional path:

```toml
# crates/ui/Cargo.toml — additive
[features]
in_process_cron = ["agent/in_process_cron"]
```

Then `cargo run --bin cockpit_live --features live,in_process_cron`
mirrors `cargo run --bin trading --features in_process_cron`. The
default `cargo run --bin cockpit_live --features live` skips cron.
T810's "default unchanged" contract holds: the feature is opt-in and
behaviorally identical between the two bins.

Flipping to "on by default" (option b) was rejected because it
silently re-introduces a behavior an operator may have explicitly
turned off via systemd/launchd in their deployment.

### Q6 — kill-switch unification

**Decision:** single shared `Arc<KillSwitch>`. Matches analyst
default. **This closes analyst finding #2.**

Confirmed by reading
[crates/ui/src/state.rs:397–402](../../crates/ui/src/state.rs):
`Message::KillConfirmed` only sets `model.kill = KillState::Flattening`
— it does **not** call into any agent-side trip mechanism. The UI
shows a flattening animation but the agent stays Running.

**Wiring:**
1. `Cockpit` model gains an optional field
   `kill_switch: Option<Arc<agent::KillSwitch>>` (Option because
   `--features fixtures` boot still passes None — it has no real
   agent to halt).
2. The `cockpit_live` bin constructs the `Arc<KillSwitch>` once,
   clones it into both `RunHandles.kill_switch` and
   `Cockpit { kill_switch: Some(...), .. }` at boot.
3. `Message::KillConfirmed`'s update arm changes from:
   ```rust
   model.kill = KillState::Flattening;
   ```
   to (under `#[cfg(feature = "live")]`, with non-live fallback
   keeping today's behavior):
   ```rust
   if let Some(ks) = model.kill_switch.as_ref() {
       ks.trip(agent::HaltReason::ManualOperator);
   }
   model.kill = KillState::Flattening;
   ```
   The trip flows through T809's existing dual-write (audit memo +
   `strategy_events.KillSwitchTripped` row + incident-report spawn)
   then comes back as a `mode` broadcast that the cockpit's
   `stream_mode` recipe picks up and renders as the halted banner.

**Audit dual-write invariant preserved.** Confirmed by reading
[crates/agent/src/kill_switch.rs:271–310](../../crates/agent/src/kill_switch.rs):
the `trip()` method is idempotent (`tripped.swap(true, SeqCst)`
short-circuits the second caller), broadcasts mode, dual-writes via
`audit::journal::kill_switch_tripped`, and spawns the incident
binary. Every one of those three side-effects runs whether the
caller is `spawn_halt_file_watcher` (file path) or
`Message::KillConfirmed` (cockpit path) because `trip()` does not
care who called it. T809's invariant is therefore preserved by
construction; **the trip is sticky, so duplicate trip calls (e.g.
operator clicks Flatten then `.halt` file appears) are silently
no-ops on the second call** — exactly the behavior
[crates/agent/src/kill_switch.rs:272–275](../../crates/agent/src/kill_switch.rs)
guarantees today.

The `mode` channel is **the** post-trip propagation path, so the
mode-broadcast forwarder task (T905) is what closes the
trip-button → cockpit-banner loop. Without it the trip writes audit
rows but the cockpit banner never updates — that's the gap the
forwarder fills.

### Q7 — keep two-binary path alive?

**Decision:** remove `cockpit --features live`. Matches analyst
default. Keep `trading` headless and `cockpit --features fixtures`.

Concrete deletions:
- `crates/ui/src/bin/cockpit.rs` lines 31–32, 45–46, 66–69, 96–101
  — the entire `#[cfg(feature = "live")]` arm that constructs an
  empty bus and calls `ui::live::subscription`.
- `crates/ui/Cargo.toml` `live` feature definition stays (the
  `cockpit_live` bin still depends on it). The `cockpit` bin no
  longer sets `live` itself.
- The `[features] live = [...]` spec line in
  [crates/ui/Cargo.toml line 60](../../crates/ui/Cargo.toml) stays
  unchanged — `cockpit_live` consumes it.
- Tests under `crates/ui/tests/live_subscription.rs` (3 tests)
  stay — they test the subscription module, not the cockpit bin.

The product is locked single-operator / single-machine
([product.md → Project scope boundary](../product.md)). The
deferred two-machine "remote viewer over IPC" topology stays
deferred; if it lands, a future `cockpit_remote` bin can be added
without touching this design.

**Backward-compat audit (V6, V7):**
- V6 — `cargo run --bin cockpit --features fixtures` is unchanged.
- V7 — `cargo run --bin trading -- --config config/agent.toml` is
  unchanged.
- Operators with workflow that runs `cargo run --bin cockpit
  --features live` get a crisp `error: feature flag combo no
  longer valid; use --bin cockpit_live instead` from a deprecation
  shim landed alongside this feature (T908 [ui-designer]). The
  shim is one line: `compile_error!` under
  `#[cfg(all(target = "cockpit", feature = "live"))]` in
  `crates/ui/src/bin/cockpit.rs`.

Also a deferred-screenshot sanity check: the v0/v0.5 manual
screenshot capture decisions in
[v0-paper-sma.md](../v0-paper-sma/feature.md) all use `--features fixtures`,
not `--features live` — confirmed by reading that brief. So
removing the live arm of the `cockpit` bin does not invalidate any
shipped screenshot or any screenshot the deferred-screenshot
backlog is tracking.

### Q8 — UI-designer touchpoints

**Decision:** zero new UI surface. The analyst's default holds. The
`## UI` section will read:

> "No new UI surface. `ui::live::subscription` and the existing
> Message taxonomy already cover every channel the unified binary
> publishes. The kill-button hover-tooltip gains one new line
> (T907) clarifying that pressing it halts the *real* agent — true
> in spec since v0, but only true in code after R4."

Two minor possibly-asks were considered and resolved:
- `CONNECTION_AGENT_UNREACHABLE` (currently unreachable string):
  stays in the catalog as future-proofing. The unified binary
  cannot trigger it because `EventBus::new` cannot panic
  (it's infallible — verified by reading the impl). If a future
  refactor makes `EventBus::new` fallible, the architect adds a
  bin-side `error::ErrorBoundary` in one patch; not blocking now.
- Kill-button tooltip line: tracked under T907 as a one-string
  edit to `crates/ui/src/strings.rs::KILL_HOVER_TOOLTIP`. Owner
  ui-designer; parallel-safe with developer tasks.

### Bus producer wiring (six channels + three v0.5 strategy lifecycle channels)

This subsection enumerates which agent producers must be wired so
finding #1 closes. Match against the
[bus.rs publisher API](../../crates/agent/src/bus.rs) (lines
116–166) and the
[dev-week2-broadcast-api](../reports/dev-week2-broadcast-api-2026-04-18.md)
six-channel spec.

| Channel | Producer (today) | Producer (after this feature) | Task |
|---------|------------------|-------------------------------|------|
| `fills` | none — paper engine creates `Fill` but doesn't publish | `crates/exec/src/paper.rs::PaperEngine::on_fill` calls `bus.publish_fill(fill.clone())` after `audit::post_fill` | T903a |
| `positions` | none — `Position` updated in paper engine but not published | `crates/exec/src/paper.rs::PaperEngine::on_fill` calls `bus.publish_position(pos.clone())` | T903a |
| `bars` | none — bar stream consumed by strategy, not published | `crates/agent/src/runtime.rs` (new) — bar stream `tap` task republishes each `Bar` to `bus.publish_bar(bar.clone())` before forwarding to strategy | T903b |
| `ticks` | none — tick stream consumed by data layer only | `crates/agent/src/runtime.rs` — tick stream `tap` task republishes each `Tick` to `bus.publish_tick(tick.clone())` | T903b |
| `pnl` | none — reconciler computes PnlSnapshot but doesn't publish | `crates/agent/src/reconciler.rs::ReconcilerTask::after_bar_close` calls `bus.publish_pnl(snap.clone())` | T903c |
| `mode` | only `kill_switch::trip` calls `mode_tx.send(Halted)` on its **own** internal sender (not the bus's `mode_tx`) | NEW T905 forwarder task subscribes to `kill_switch.subscribe()` and forwards each `AgentMode` to `bus.publish_mode(mode.clone())`. Sticky / idempotent like the trip itself. | T905 |
| `strategy_loaded` | already wired — `crates/agent/src/watcher.rs:400` | unchanged | — |
| `strategy_swapped` | already wired — `crates/agent/src/watcher.rs:375` | unchanged | — |
| `strategy_error` | already wired — `crates/agent/src/watcher.rs:267` | unchanged | — |

Critical implementation note: the `bus.mode_tx` and the
`kill_switch.mode_tx` are **two distinct broadcast::Sender<AgentMode>**
today. They have no relationship. The forwarder bridges them. This
is the cheapest correct fix — adding a "publish to bus" side-effect
to `KillSwitch::trip` would cross-contaminate a presently-pure
component (kill switch knows nothing about the bus); the forwarder
keeps that boundary clean.

The data-feed `tap` tasks (T903b) are intentionally lightweight —
they just `tx.send(item.clone())` against the bus and forward the
original item downstream. `Bar` and `Tick` are `Clone` (verified by
reading `trading_core::Bar` / `trading_core::Tick`); the clone is
~80 bytes (Bar) and ~64 bytes (Tick), so even at 8192/s tick
throughput the cost is bounded at ~500 KB/s of allocator pressure
— well inside the v0 latency budget. If profiling shows it
matters, a later refactor moves to `Arc<Tick>` on the bus; not
needed for v1.

### Crate map delta

**New files:**
- `crates/agent/src/runtime.rs` — `RunHandles` struct + `pub async
  fn run(...)` + `pub async fn shutdown_writer(...)`. ~200 LOC
  extracted from `crates/agent/src/main.rs`.
- `crates/ui/src/bin/cockpit_live.rs` — the unified binary. ~250
  LOC: builds the runtime + iced application, owns the
  `CancellationToken` + side-thread join. Mirrors the structure of
  `crates/ui/src/bin/cockpit.rs` but with the agent runtime
  hosted side-thread.

**Modified files (sketch — developer tasks for landing):**
- `crates/agent/src/main.rs` — collapses to ~70 LOC; calls into
  `agent::runtime::run`.
- `crates/agent/src/lib.rs` — adds `pub mod runtime;` and
  re-exports.
- `crates/agent/src/reconciler.rs` — `ReconcilerTask::after_bar_close`
  publishes `PnlSnapshot` (T903c).
- `crates/exec/src/paper.rs` — `PaperEngine::on_fill` publishes
  `Fill` and `Position` (T903a). Touches `paper::on_fill` only;
  not the matching engine logic.
- `crates/agent/src/config.rs` — adds `prometheus_enabled: bool`
  to `[observability]` (T901).
- `crates/agent/src/observability.rs` — `start_prometheus_exporter`
  becomes a no-op when `prometheus_enabled = false` (T901).
- `crates/ui/src/state.rs` — `Cockpit` gains `kill_switch:
  Option<Arc<agent::KillSwitch>>` field; `Message::KillConfirmed`
  arm calls `trip()` (T906).
- `crates/ui/src/bin/cockpit.rs` — removes `--features live` arm
  + adds the deprecation `compile_error!` (T908).
- `crates/ui/Cargo.toml` — adds `[[bin]] cockpit_live` entry +
  `in_process_cron` feature pass-through (T901).

**No new workspace member.** No new top-level dep beyond what
already exists in `Cargo.lock` — confirmed:
- `tokio_util` is in `Cargo.lock` and declared as a workspace dep
  in [Cargo.toml line 1 (workspace deps section)](../../Cargo.toml).
- `agent` is already in `crates/ui/Cargo.toml` under the `live`
  feature.
- No new system C dep; no new edition-2024-incompatible crate; no
  stdlib-name shadow (bin name `cockpit_live` is fine).

The library/crate compatibility checklist (architect.md section)
runs clean — every required item is satisfied by reusing existing
deps.

### Public API additions (full list)

In addition to the `agent::runtime::*` items above:

- `agent::config::ObservabilityConfig::prometheus_enabled: bool`
  — new struct field, `#[serde(default = "default_true")]`.
- `ui::state::Cockpit::kill_switch: Option<Arc<agent::KillSwitch>>`
  — new struct field, `#[cfg(feature = "live")]`. Ignored under
  `--features fixtures` boot; hot path is the
  `Message::KillConfirmed` arm.

No removals from the public API. The `cockpit` bin's `--features
live` removal is a *bin-spec* removal, not a library-API removal —
no consumer code breaks because no library code consumed it.

### Runtime topology — operator success report invariants

The five Wave-1 operator-success-reports landings must keep
working. Concrete assertions per invariant:

**T805 — feed_reconnect emission.** The Binance reconnect handler
(`crates/data/src/binance.rs:285–296` + `:411–422`) calls
`audit::journal::feed_reconnect` from inside the data tokio task.
Under the unified binary the data task runs on the side-thread
runtime, same as in the headless agent. Invariant: kill the network
mid-run, reconnect, observe a `strategy_events.FeedReconnect` row
within 2 s of reconnect. **Test:** V8 (manual smoke) +
`crates/data/tests/binance_reconnect_test.rs` (existing).

**T806 — uptime ledger open/heartbeat/close.** The
`agent::runtime::run` function does NOT itself open or close the
uptime interval — the caller does, both at entry and at the
`shutdown_writer` step. This makes the open/close a property of
the bin (which has the boot id and the ledger handle in scope), not
of the runtime function (which would have to thread `boot_id`
through). Invariant: under both shutdown paths (V3a, V3b, V3c),
exactly one open row + ≥ 2 heartbeat rows + exactly one close row
on the same boot UUID. **Test:** V9 (90 s smoke) + new
`crates/agent/tests/unified_uptime_test.rs` (T910) running the
unified binary as a subprocess via `assert_cmd::Command::cargo_bin`.

**T809 — kill-switch dual-write.** `KillSwitch::trip` is the *single*
trip method; it is called by (a) the file-watch task and (b) the
new cockpit `Message::KillConfirmed` arm. The dual-write logic in
[kill_switch.rs:283–298](../../crates/agent/src/kill_switch.rs)
runs unconditionally on the **first** trip (subsequent trips
short-circuit on `tripped.swap(true)`). Invariant: either trip path
produces exactly one `audit_memo` row, exactly one
`strategy_events.KillSwitchTripped` row, and exactly one incident
binary spawn. **Test:** V2a / V2b + reuse of the existing T809
integration test pattern (`crates/agent/tests/kill_switch_audit_test.rs`)
adapted for the cockpit-button trigger via a unit test that calls
`Message::KillConfirmed` directly with a `MockIncidentSpawner` —
new test `crates/ui/tests/kill_button_trips_kill_switch.rs` (T911).

**T810 — opt-in cron flag.** Default `cargo run --bin cockpit_live`
does not pull `tokio-cron-scheduler`; opt-in via
`--features in_process_cron`. **Test:** V4's
`cargo build -p ui --features in_process_cron` is part of the
matrix; new check `cargo build -p ui` (no in_process_cron) shows
no cron deps in the resulting binary.

### Test strategy — per V-item

| V | What's verified | Where it lives | Fixture / driver | Assertion |
|---|------------------|----------------|------------------|-----------|
| V1 | End-to-end smoke (window opens, panels populate from real bus) | manual + new `crates/ui/tests/cockpit_live_smoke.rs` | `assert_cmd` launches `cargo_bin("cockpit_live")` against `config/agent.research.toml` (research mode replay feed); test polls JSON log for `unified_started` + `bus_attached` + `cockpit_window_open` + first `FillReceived` within 30 s | tracing lines appear in order; subprocess returns 0 on SIGTERM |
| V2a | File-touch trip → halted banner | new `crates/ui/tests/cockpit_live_kill_file.rs` | subprocess + `tempdir().join(".halt")` write | log contains `KillSwitch tripped` + audit DB has `KillSwitchTripped` row + cockpit log has `AgentHaltedExternally` within 1 s |
| V2b | Cockpit-button trip → halted banner | unit test `crates/ui/tests/kill_button_trips_kill_switch.rs` (T911) | direct `state::update` call with `KillConfirmed` against a `Cockpit` carrying a `MockIncidentSpawner`-backed `KillSwitch`. **Plus** an integration variant under V_FINAL that drives a real iced subscription to confirm the round-trip via `bus.mode` | trip flag set; mode broadcast emitted; incident spawner recorded; UI state transitions to `KillState::Halted` |
| V3a | Ctrl-C → exit < 2 s + close-uptime row | new `crates/agent/tests/unified_uptime_test.rs` (T910) | subprocess; send SIGINT after 5 s; `wait_with_output` with 2 s timeout | exit code 0; `agent_uptime` table has matching `boot_id` with `stopped_at` set |
| V3b | Window close → exit < 2 s | manual (xdotool / pyautogui driving the iced window — flaky in CI; tracked as a manual gate) + a programmatic surrogate where the test sends `Message::ShutdownRequested` to the running `Cockpit` and asserts the cancel token observers propagate | manual + surrogate unit test in T910's file | within 2 s the side thread joins, close-uptime fires |
| V3c | Trip then close → clean exit | manual smoke; covered by the union of V2 + V3b | — | banner appears, then exit timing matches V3b |
| V4 | Existing test matrix stays green | `cargo test --workspace --all-targets` + the four cargo-test variants | existing | all PASS; clippy + fmt clean |
| V5 | Anchor regression | `scripts/verify_anchors.sh` | existing 11 anchors | PASS 11/11 — none of the 11 cover `agent` or `ui`, so this is a pass-by-construction (R15) |
| V6 | `cockpit --features fixtures` still boots | `cargo run --bin cockpit --features fixtures` smoke | existing | window opens with v1.5a steady-state fixture |
| V7 | `trading` headless still boots | `cargo run --bin trading -- --config config/agent.toml` smoke | existing | starts, log shows `agent running` |
| V8 | Feed-reconnect smoke | manual; with a programmatic surrogate in `crates/agent/tests/feed_reconnect_smoke.rs` (existing T805 test) | existing | `feed_reconnect` row appears |
| V9 | Uptime heartbeat smoke | `crates/agent/tests/unified_uptime_test.rs` (T910) | subprocess running for 90 s | 1 open + ≥ 2 heartbeat + 1 close on same boot id |
| V10 | Prometheus toggle | new `crates/agent/tests/prometheus_toggle_test.rs` (T912) | two subprocess invocations: one with `prometheus_enabled = true`, one false; `reqwest::get(":9100/metrics")` against each | enabled → 200 OK with metrics text; disabled → connection-refused error |

**V3 — graceful-shutdown timing requires a real binary launch.**
Confirmed: a unit test cannot exercise the side-thread + iced
shutdown handshake. `assert_cmd::Command::cargo_bin("cockpit_live")
.timeout(Duration::from_secs(10)).spawn()` then send SIGINT after a
5 s warm-up and `wait_with_output()` with a 2 s deadline gives a
deterministic timing assertion. The window-close path (V3b)
genuinely requires a humanoid driver in CI; we degrade to a manual
gate (operator runs the binary, clicks the X, observes < 2 s exit)
plus the programmatic surrogate that sends
`Message::ShutdownRequested` directly into `state::update` and
verifies the cancellation token propagates. The two together cover
the same property — surrogate proves "the message → cancel token
plumbing works", manual proves "the iced close button emits the
message". The manual gate is added to the V_FINAL row and the
operator-checklist deck.

### Risks + mitigations

| # | Risk | Mitigation |
|---|------|------------|
| 1 | Blocking iced's internal executor by accidentally calling agent code on the GUI thread (e.g. an `Arc<KillSwitch>::trip` that internally spawns a tokio task, but the spawn site is on the iced thread which has no tokio runtime in scope) | `KillSwitch::trip` already uses `tokio::spawn`; that requires a tokio runtime in scope. Wrap the cockpit's call site in `Handle::current().block_on(async { ks.trip(...) })` — but the cleaner fix is: the `Cockpit::kill_switch` field stores not the `Arc<KillSwitch>` directly but an `Arc<dyn Fn(HaltReason) + Send + Sync>` closure that captures the side-thread runtime's `Handle` and `spawn`s the trip on it. The `cockpit_live` bin constructs that closure. **Codified in T906 acceptance.** |
| 2 | Cockpit panel drift when bus is fully wired (existing `crates/ui/tests/live_subscription.rs` 53 tests assume specific channel shapes; new producers may emit at higher rates than tests expect) | Regression-run `cargo test -p ui --features live` is in V4 + per-task acceptance. Add a single new test `crates/ui/tests/live_subscription_full_bus.rs` (T911) that drives a fully populated bus and asserts no panel exceeds its `Loading` window past the first event. |
| 3 | Shutdown deadlock — a producer task ignores `cancel.cancelled()` and the side thread `block_on` never returns | 2 s wall-clock bound on the side-thread `JoinHandle::join` — on timeout, log `shutdown_deadline_exceeded` and exit anyway via `std::process::exit(0)`. The exit code stays 0 because the close-uptime row was written before the `block_on` re-enters; the `agent_uptime` schema tolerates a slightly-late close. |
| 4 | Config schema breakage — adding `prometheus_enabled` to `ObservabilityConfig` without a default would break existing `config/agent.toml` files | `#[serde(default = "default_true")]` keeps backward compat. T901 acceptance requires loading an unmodified pre-feature `config/agent.toml` and asserting the field defaults to `true`. |
| 5 | Developer regressions in `agent` headless mode (the extraction of `agent::run` could subtly change task spawn order, breaking the existing kill-switch test matrix) | Keep the headless `trading` bin in the CI test matrix (it already is). Add a single integration test `crates/agent/tests/agent_run_extraction_smoke.rs` (T902) that calls `agent::runtime::run(handles, cancel).await` directly with a `MockIncidentSpawner` and an in-memory ledger, sends a `cancel` after 1 s, asserts clean return. |
| 6 | Bus producer wiring (T903a/b/c) lands but a producer holds an extra `Arc<EventBus>` clone past shutdown, preventing the broadcast senders from dropping → `RecvError::Closed` never fires on the cockpit side → cockpit panels show stale data forever after window close | `RunHandles.bus: Arc<EventBus>` is dropped at the end of `agent::runtime::run`. Producers receive `bus: Arc<EventBus>` clones via `Arc::clone(&handles.bus)` at task spawn time; each task owns its clone; on `cancel.cancelled()` the task returns, dropping its clone. The reference count is bounded by the number of spawned tasks (~7) so cleanup is one cancel cascade. T903d (bus-drop test) verifies that after `agent::runtime::run` returns, `Arc::strong_count(&bus) == 1` — only the bin's outer reference remains. |

### Body-vs-front-matter discipline

The unified binary writes nothing that gets hashed by
`scripts/hash_report.py`. Audit DB rows have microsecond timestamps
(T806 already enforced this); JSON log output is informational and
not in the body-hash set; tracing lines are pure observability. The
spec/anchors.toml gate is unaffected by this feature (R15 — none
of the 11 anchors cover `agent` or `ui`).

The new `prometheus_enabled = true` field on
`agent::config::ObservabilityConfig` is a Config struct field, not a
report field, so the body-vs-front-matter rule does not apply. No
new artifact is created by this feature, so no new front-matter
schema is in scope.

## Implementation

### Wave 1 — T901 + T902 (developer, 2026-05-02)

**T901 — Config + observability `prometheus_enabled` toggle:**

- `crates/agent/src/config.rs:196-216` adds the `prometheus_enabled: bool`
  field to `ObservabilityConfig` with `#[serde(default = "default_true")]`.
- `crates/agent/src/observability.rs:106-123`'s
  `start_prometheus_exporter` short-circuits with one
  `prometheus_listener_disabled` info line when the toggle is `false`.
- `config/agent.toml:42-46` documents the field, leaves it commented to
  preserve default-true behavior on existing configs.
- New tests:
  `agent::config::tests::t901_prometheus_enabled_defaults_true_when_omitted`,
  `agent::config::tests::t901_prometheus_enabled_explicit_false_round_trips`,
  `agent::observability::tests::t901_disabled_skips_listener`.

**T902 — `agent::runtime::run` extraction + `RunHandles`:**

- New module `crates/agent/src/runtime.rs` (462 LOC) containing
  `pub struct RunHandles`, `pub async fn run(handles, cancel) ->
  Result<()>`, `pub async fn shutdown_writer(ledger, &boot_id)`. Module-doc
  spells out caller responsibilities (CLI parse → tracing → observability
  → ledger → kill_switch → registry → bus → boot_id → open_uptime →
  Ctrl-C → run → shutdown_writer).
- `crates/agent/src/lib.rs` re-exports `runtime::{run, RunHandles,
  shutdown_writer}`; `crates/agent/src/main.rs` slims to ~180 LOC of
  pure init + `agent::runtime::run(...).await?`.
- Task-spawn order inside `run()` mirrors the pre-extraction `main.rs`
  sequence — uptime heartbeat (T806), in-process cron (T810, gated),
  cost budget, strategy registry flush, strategy_watcher (with cancel
  bridge), funding poller + persist sidecar, data feed init,
  `select!` over cancel + `kill_switch.subscribe()`, JoinSet drain
  with 2 s wall-clock bound + force `abort_all` on overrun.
- **Bug fix in scope:** `crates/agent/src/watcher.rs:106-134` —
  the `notify` blocking thread originally used `for event in notify_rx`
  which never returned during runtime drop, pinning the tokio
  blocking-pool's shutdown indefinitely. Replaced with a
  `recv_timeout(200ms)` poll loop that observes the async sender's
  `is_closed()` so the blocking task drains before the runtime is
  dropped. Without this fix the T902 smoke test hung in
  `BlockingPool::shutdown` for >9 minutes.
- Operator-success-report invariants preserved by construction:
  T805 `feed_reconnect` writer untouched (`crates/audit/src/journal.rs:629`),
  T806 uptime open in `main.rs` + heartbeat in `runtime::run` + close in
  `shutdown_writer`, T809 `KillSwitch::with_audit` constructed in
  `main.rs` and dual-write happens inside `KillSwitch::trip`, T810
  `--features in_process_cron` builds clean (verified).
- Smoke verification: `./target/release/trading --config
  config/agent.toml --mode research`, SIGINT after 5 s — clean shutdown,
  one `agent_uptime` row per boot_id with non-NULL `stopped_at`.

### Wave 2 — T903b + T905 + T903c (developer-B, 2026-05-01)

**T903b — Data feed bar/tick taps (`agent::runtime`):**

- New `pub(crate) async fn spawn_feed_taps<S: MarketDataSource + ?Sized>(...)`
  helper at `crates/agent/src/runtime.rs:430-510`. Two tokio tasks, both
  pushed onto `run()`'s `JoinSet` so they participate in the existing
  cooperative-shutdown drain:
  - **bars tap** subscribes to `feed.subscribe_bars(symbol, tf)` and calls
    `bus.publish_bar(bar)` for each item; warn-logs stream errors and
    continues.
  - **ticks tap** subscribes to `feed.subscribe_trades(symbol)` and calls
    `bus.publish_tick(tick)` for each item; debug-logs stream errors
    (high-volume channel — log level intentionally lower).
- Both honor `cancel.child_token().cancelled()` and exit cleanly when the
  upstream stream ends (e.g. replay finished). If the initial subscribe
  fails (e.g. parquet root missing in research mode), the tap is
  warn-logged and skipped — runtime stays up.
- Helper is invoked from `run()` at `crates/agent/src/runtime.rs:317-365`,
  in both Research (replay feed) and Paper (Binance feed) arms. Symbol
  hardcoded to `BTCUSDT` / `Timeframe::OneMinute` to match the SMA
  crossover strategy in `config/agent.toml`; symbol/tf can become
  config-driven later without touching the helper.
- New unit test `runtime::tests::t903b_taps_publish_bars_and_ticks`
  (`crates/agent/src/runtime.rs:680-744`) drives the helper against a
  `data::FakeFeed` with 5 bars + 20 ticks; asserts `bus.bars()` and
  `bus.ticks()` subscribers receive every item within 2 s; then
  cancels and asserts the JoinSet drains cleanly.

**T905 — Mode-broadcast forwarder (`agent::runtime`):**

- New `pub(crate) fn spawn_mode_forwarder(...)` at
  `crates/agent/src/runtime.rs:513-540`. Subscribes to
  `kill_switch.subscribe()` (the kill-switch's *internal* mode channel)
  and forwards every received `AgentMode` event onto the bus's `mode`
  channel via `bus.publish_mode(...)`. Single writer to
  `bus.publish_mode(...)`; `KillSwitch` itself stays bus-agnostic
  (Q6 boundary preserved).
- Invoked from `run()` at `crates/agent/src/runtime.rs:373-378` after the
  data-feed init so the forwarder is part of the JoinSet drain on
  `cancel.cancel()`. Closes cleanly via `cancel.child_token()` plus the
  `RecvError::Closed` arm; lag is warn-logged.
- New unit test `runtime::tests::t905_kill_switch_trip_emits_to_bus_mode`
  (`crates/agent/src/runtime.rs:749-803`) trips the kill switch and
  asserts the bus's `mode` subscriber receives `AgentMode::Halted`
  within 500 ms; a second trip is asserted to NOT emit a second event
  (sticky-trip semantics on `KillSwitch::trip` short-circuit at
  `tripped.swap(true, SeqCst)`).
- Operator-success-report T809 dual-write invariant preserved by
  construction: `KillSwitch::trip` is unchanged; the forwarder is purely
  a downstream consumer of the same channel that today drives the
  headless agent's shutdown `select!`. Existing
  `tests/kill_switch_trip_writes_both.rs` integration test reruns clean.

**T903c — Reconciler publishes PnL snapshots (`agent::reconciler`):**

- `ReconcilerTask` gains a `bus: Option<Arc<EventBus>>` field at
  `crates/agent/src/reconciler.rs:64-73`; `with_bus(...)` builder helper
  at `:91-97` (`ReconcilerTask::new` signature unchanged — backward
  compat preserved for backtest callers).
- New `pub fn after_bar_close(&self) -> PnlSnapshot` at
  `crates/agent/src/reconciler.rs:99-124` builds a `PnlSnapshot` from
  the current `ReconcilerState` using `Money::from_decimal(...)` for
  every money field (cash / unrealized / realized / total_equity /
  daily_return). Determinism: 100% `rust_decimal::Decimal`; never
  `f64`. When a bus is wired, calls `bus.publish_pnl(snap.clone())`;
  otherwise just returns the snapshot for caller use.
- `ReconcilerState` gains two additive fields at `:35-46` —
  `realized_pnl: Decimal` and `cost_basis: Decimal` — used to compute
  `unrealized = position_qty * last_mark - cost_basis` (helper at
  `:55-60`). `daily_return` is populated as `Decimal::ZERO` pending a
  future baseline-from-ledger wire-up; documented inline.
- New unit test `reconciler::tests::t903c_after_bar_close_publishes_pnl`
  (`crates/agent/src/reconciler.rs:240-289`) constructs a reconciler
  with a real `Arc<EventBus>`, invokes `after_bar_close`, asserts:
  - returned snapshot has the expected `cash`, `unrealized = 5_000`,
    `realized = 123.45`, `total_equity = 130_000` Decimal values;
  - bus subscriber receives the same snapshot within 1 s;
  - a second reconciler with `bus = None` (the backtest path) returns
    the same snapshot without panicking on publish.
- Backtest report bytes unchanged: backtest callers go through
  `ReconcilerTask::new(...)` which leaves `bus = None`. R15 anchor
  invariant preserved by construction.

**Wave 2 verification:**

- `cargo test -p agent` — 30 lib + 23 integration tests green
  (`test result: ok. 30 passed; 0 failed`). Three new tests:
  `runtime::tests::t903b_taps_publish_bars_and_ticks`,
  `runtime::tests::t905_kill_switch_trip_emits_to_bus_mode`,
  `reconciler::tests::t903c_after_bar_close_publishes_pnl`. All ok.
- `cargo build -p agent` clean.
- `cargo clippy --workspace --tests -- -D warnings` clean.
- `cargo fmt --all -- --check` clean.
- Anchor gate (R15) — pass-by-construction; none of the 11 anchors
  cover `agent` and the backtest reconciler path leaves `bus = None`.

### Wave 2 — T904 (developer-D, 2026-05-01)

**T904 — `cockpit_live` bin skeleton (`crates/ui/src/bin/cockpit_live.rs`):**

- New file `crates/ui/src/bin/cockpit_live.rs` (464 LOC) — the unified
  agent + iced cockpit binary. Resolves R1: a single `cargo run` boots
  both the agent task graph and the iced window, sharing one
  `Arc<EventBus>` + one `Arc<KillSwitch>` + one `CancellationToken`.
- Runtime topology per Design Q2:
  - **Bootstrap path** uses a short-lived
    `tokio::runtime::Builder::new_current_thread()` runtime to drive
    `audit::Ledger::open` + `chart_of_accounts` + `open_uptime_interval`
    BEFORE the side-thread runtime exists. Avoids needing
    `#[tokio::main]` on `fn main()` (which would conflict with iced
    owning the main thread). The bootstrap runtime is dropped before
    the side-thread runtime starts.
  - **Side thread** is a named `std::thread::Builder::new().name("agent-runtime")`
    hosting `tokio::runtime::Builder::new_multi_thread().enable_all()`.
    It spawns a `tokio::signal::ctrl_c()` listener that trips the
    shared `cancel` (V3a — Ctrl-C path) then `block_on`s
    `agent::runtime::run(handles, cancel)` followed by
    `agent::runtime::shutdown_writer(ledger, &boot_id)` (V3b sequence
    step 5 — close-uptime row written on the same runtime that opened
    it).
  - **Main thread** runs `iced::application(boot, update, view).run()`
    with `subscription = ui::live::subscription(Arc::clone(&bus))`.
    The `AppState` carries `Arc<EventBus>` (for the subscription) and
    `Arc<KillSwitch>` (held only — T906 wires it into
    `Cockpit::kill_switch`).
- Shutdown invariants per Design Q3:
  - After `iced::run` returns, `main` calls `cancel.cancel()`
    (idempotent if Ctrl-C already fired) and joins the side thread
    with a 2 s wall-clock bound via `join_with_deadline()`. On timeout,
    logs `shutdown_deadline_exceeded` and `std::process::exit(0)` —
    Q3 explicitly accepts a missed close-uptime row over a hung
    process.
  - `join_with_deadline` polls `JoinHandle::is_finished()` at 10 ms
    cadence; chosen over a joiner-thread approach because the
    deadline path needs to *return* (so `main` can decide to log +
    force-exit), and a joiner thread would leak.
- Cargo plumbing in `crates/ui/Cargo.toml`:
  - New `[[bin]] cockpit_live` with `required-features = ["live"]` so
    a default `cargo build -p ui` does not pull the `agent` dep tree.
  - `live` feature now also pulls `audit`, `strategy`, `tokio-util`,
    `anyhow`, `clap`, `tracing-subscriber`, `uuid` — all optional
    deps gated behind `live` so the fixtures-only `cockpit` bin stays
    lean.
  - New `in_process_cron = ["live", "agent/in_process_cron"]`
    pass-through per Design Q5 — opt-in unchanged, mirrors the
    headless `trading` bin's gate.
- **Skeleton scope (T904 only) — what does NOT land here:**
  - `Cockpit::kill_switch` field + `Message::ShutdownRequested` variant
    + the recipe that bridges `cancel.cancelled()` → window close are
    owned by **T906** (ui-designer scope). The skeleton holds the
    `Arc<KillSwitch>` on `AppState._kill_switch` so T906 has a single
    named field to read from. Until T906 lands, window-close shutdown
    works through `iced::run`'s natural exit; Ctrl-C while the window
    is open gracefully shuts the agent side thread but does not
    auto-close the iced window.
  - End-to-end "panels actually populate" smoke is T911's gate, not
    T904's. T904 is "compiles, links, has the right shape" + clean
    build under both default and `--features in_process_cron`.
- Operator-success-report invariants preserved:
  - T805/T806/T809/T810 all flow through `agent::runtime::run` (Wave 1
    plumbed them; T904 just calls the function).
  - T806 close-uptime row written by `shutdown_writer` on the
    side-thread runtime — same write site as the headless `trading`
    bin.
  - T810 `--features in_process_cron` builds clean for both `trading`
    (`cargo build --release --bin trading --features agent/in_process_cron`)
    and `cockpit_live` (`cargo build --release --bin cockpit_live
    --features ui/in_process_cron`); skeleton verified.
- Verification:
  - `cargo build -p ui --features live --bin cockpit_live` clean.
  - `cargo build -p ui --features in_process_cron --bin cockpit_live` clean.
  - `cargo build --release --bin cockpit_live --features ui/live` clean.
  - `cargo build --release --bin cockpit_live --features ui/in_process_cron` clean.
  - `cargo test -p ui --features live` — 32 lib tests pass, 0 failed.
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean.
  - `cargo fmt --all -- --check` clean.
  - `cargo build --release --bin trading` (Wave 1 invariant) clean.
  - `cargo build --release --bin trading --features agent/in_process_cron` (T810 invariant) clean.
  - Anchor gate (R15) — pass-by-construction; T904 only adds files
    under `crates/ui/` (new `cockpit_live.rs` + additive Cargo.toml
    stanzas). No `crates/strategy/`, `crates/audit/`, `crates/exec/`,
    `crates/backtest/`, or report-rendering code touched, so the 11
    body-SHA-256 anchors in `spec/anchors.toml` cannot drift.

### Wave 3 — T903a-glue + T903d + T910 + T911 + T912 (developer, 2026-05-01)

**T903a-glue — agent-side bus-wiring closure (`crates/agent/src/bus.rs` + `crates/agent/src/runtime.rs`):**

- `crates/agent/src/bus.rs:269-280` — `impl exec::FillPublisher for EventBus`
  delegating to the existing `EventBus::publish_fill` / `publish_position`
  methods via `Fill::clone` / `Position::clone` (broadcast channels need
  owned values; the trait takes `&Fill` / `&Position` per the dep-graph
  rationale Dev A documented in `crates/exec/src/publisher.rs`).
- `crates/agent/src/runtime.rs:419-433` — new `pub fn paper_engine_publisher(bus: Arc<EventBus>) -> exec::PaperEnginePublisher`
  helper that builds the live-mode publisher in one allocation; the architect's
  Q6 boundary is preserved (`crates/exec/` still knows nothing about
  `crates/agent/`). Re-exported at the crate root via `crates/agent/src/lib.rs:20`.
- New tests:
  - `bus::tests::t903a_glue_event_bus_impls_fill_publisher` — coerces an
    `Arc<EventBus>` into an `Arc<dyn FillPublisher>` and asserts publish
    fans out on `bus.fills()` + `bus.positions()`.
  - `bus::tests::t903a_glue_paper_engine_publisher_routes_to_bus` — wires
    `PaperEnginePublisher::with_publisher(...)` against the bus and
    asserts `on_fill` produces one fill + one position event.
  - `runtime::tests::t903a_glue_paper_engine_publisher_routes_to_bus` —
    exercises the new `paper_engine_publisher` helper end-to-end.
- Operator-success-report invariants preserved by construction: T802 audit
  ordering is the runtime's contract (the impl is a thin pass-through);
  T809 `KillSwitch::trip` dual-write is independent of the new bus
  wiring; T805/T806/T810 unchanged.

**T903d — bus-drop test (`crates/agent/tests/bus_drops_on_shutdown.rs`):**

- New integration test `t903d_bus_strong_count_collapses_on_cancel`
  constructs `RunHandles` with a real `Arc<EventBus>`, holds an outer
  reference (`bus_outer`), awaits `agent::runtime::run` for 500 ms,
  trips `cancel.cancel()`, awaits `run`'s return inside 2 s, then
  asserts `Arc::strong_count(&bus_outer) == 1` — every spawned task's
  `Arc<EventBus>` clone has dropped on cancel.  Closes architect's
  Risk #6 (producer leaks an `Arc<EventBus>` clone past shutdown →
  cockpit panels show stale data forever).

**T910 — graceful-shutdown timing (`crates/agent/tests/unified_uptime_test.rs`):**

- New integration test `t910_v3_graceful_shutdown_within_two_seconds_with_close_uptime_row`
  constructs `RunHandles`, awaits `agent::runtime::run` under a
  `multi_thread` tokio runtime, sends `cancel.cancel()` after a 500 ms
  warm-up, asserts the future returns inside the architect's 2 s
  `SHUTDOWN_DEADLINE` (Q3), then writes the close-uptime row via
  `agent::runtime::shutdown_writer` and asserts the matching
  `agent_uptime` row has `stopped_at = Some(_)` (T806 R7.1).
- **Architect-design deviation:** the architect's V3a names "subprocess
  SIGINT" against the unified `cockpit_live` bin.  In the developer-agent
  sandbox we observed (a) tokio's lazy `ctrl_c()` handler registration
  races our `kill(2)` send (OS default-terminate wins, no close-uptime
  row); (b) the halt-file path used as a fallback trigger trips the
  watcher immediately on startup — `Path::exists()` returns true for a
  path the parent process can confirm does not exist (root cause:
  macOS sandbox file-system view).  Mitigation: drive `cancel.cancel()`
  directly — same primitive both the headless `trading` bin's Ctrl-C
  handler and the unified `cockpit_live` bin's window-close handler
  call.  The end-to-end SIGINT smoke is left to the tester's V_FINAL
  gate where a real terminal is in scope.  V9 (90 s heartbeat) is
  gated for the V_FINAL slot per architect's task spec.

**T911 — live-bus regression (`crates/ui/tests/live_subscription_full_bus.rs`):**

- New integration test file with two tests:
  - `t911_full_bus_drives_every_panel_out_of_loading` publishes 100 fills
    + 50 positions + 20 bars + 200 ticks + 5 PnL snapshots + 1 mode
    transition; reads one event per channel via the `ui::live::stream_*`
    recipes; calls `ui::state::update` to drive the `Cockpit`; asserts
    every panel exits `Loading` (`tape`, `positions`, `pnl`, latency
    badge `Known`, `last_bar_ts.is_some()`, mode `Halted` + `kill`
    `KillState::Halted`).
  - `t911_kill_button_round_trip_via_mode_forwarder` constructs a real
    audit-wired `Arc<KillSwitch>` against an in-memory ledger +
    `MockIncidentSpawner`, spawns the T905 `agent::runtime::spawn_mode_forwarder`,
    calls `kill_switch.trip(HaltReason::ManualOperator)`, asserts the
    cockpit's `stream_mode` recipe yields a `Halted` message inside 1 s
    and the cockpit's `kill` panel transitions to `KillState::Halted`,
    then asserts sticky-trip semantics (a second trip emits no
    duplicate event).
- Plumbing change: `crates/agent/src/runtime.rs::spawn_mode_forwarder`
  visibility raised from `pub(crate)` to `pub` so the V2b round-trip
  test can drive it from the ui test crate (additive-only).
- Existing 75 ui live tests still green; combined total is 77 ok.

**T912 — Prometheus toggle (`crates/agent/tests/prometheus_toggle_test.rs`):**

- New integration test file with three subtests:
  - `t912_disabled_skips_bind_via_public_api` passes a malformed listen
    string with `prometheus_enabled = false`; asserts the function
    short-circuits before parsing — proves the disabled branch is the
    FIRST thing checked.
  - `t912_enabled_attempts_parse` passes the same malformed listen
    string with `prometheus_enabled = true`; asserts the function
    returns Err — proves the toggle is bidirectional (a regression
    that silently disabled prometheus would surface as the malformed
    string being accepted).
  - `t912_runtime_with_prometheus_disabled_does_not_bind_9100` builds a
    `RunHandles` with `prometheus_enabled = false`, calls
    `start_prometheus_exporter` via the same public surface the
    `trading` / `cockpit_live` bins call at boot, runs
    `agent::runtime::run` for 200 ms, probes port 9100 via
    `TcpListener::bind`, asserts the runtime did not silently bind it.

**Wave 3 verification:**

- `cargo test --workspace --all-targets` — every test result line `ok`;
  zero failures across the workspace.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean.
- `cargo fmt --all -- --check` clean.
- Anchor gate (R15): pass-by-construction — Wave 3 only adds tests +
  one `impl` block on `EventBus` + one helper fn `paper_engine_publisher`
  that backtests don't touch.  No `crates/strategy/`, `crates/audit/`,
  `crates/backtest/`, or report-rendering code modified.
  `bash scripts/verify_anchors.sh` blocked by developer-agent sandbox
  (same gate as T903a / T903b / T904 notes); tester re-runs as the
  regression confirmation.

## Verification — links

_Tester fills this — link the test report(s) here once V1–V10
have been exercised._

## UI

_ui-designer fills this — likely a single line "no new UI surface;
existing live wiring is sufficient" once Q8 resolves. If R4's
button-hooks-to-real-kill-switch surfaces a tooltip ask, ui-designer
addresses it here._

## Changelog

- 2026-05-02 (tester, Final gate): bumped `status: in-progress -> shipped`,
  `owner: architect -> tester`. Test report
  `spec/archive/test-2026-05-02-1501-live-cockpit-unified-final.md (archived; see spec/archive/README.md)`
  records the V1-V10 verification matrix all VERIFIED, anchors PASS
  11/11 (twice — Phase 1 + Phase 4), full workspace test matrix PASS
  (`cargo test --workspace --all-targets`, `cargo test -p ui --features
  live` 78/78, `cargo test -p ui --features fixtures` 59/59), `cargo
  fmt --check` clean, `cargo clippy --workspace --all-targets
  --all-features -- -D warnings` clean. T_FINAL_LIVE_COCKPIT ticked in
  `spec/live-cockpit-unified/tasks.md`. T906's stitch integration test
  (`crates/ui/tests/cockpit_live_kill_button_writes_audit.rs`) is the
  load-bearing empirical proof that the cockpit kill button trips
  T809's audit dual-write end-to-end via the side-thread tokio runtime
  topology (option A). Operator-success-reports invariants (T802, T805,
  T806, T809, T810) preserved by construction and verified by their
  regression suites. T908's deprecation gate (`required-features =
  ["fixtures"]` + `cfg(all(feature="live", not(feature="fixtures")))`
  compile_error shim) works as designed: `cargo build --bin cockpit
  --features live` correctly fails with a redirect-to-cockpit_live
  cargo-level error while `--all-features` workspace builds remain
  clean. Tester smoke probe of `./target/release/trading --mode
  research` boots, runs for 3 s, exits cleanly on SIGINT with the T806
  close-uptime row written. VERDICT -> PASS; routing -> presenter.
- 2026-05-01 (architect): added `## Design` section resolving Q1–Q8.
  **Q1**: new bin `cockpit_live` at `crates/ui/src/bin/cockpit_live.rs`;
  extract `pub async fn agent::runtime::run(RunHandles, CancellationToken)`
  shared by both the headless `trading` bin and `cockpit_live`. Bin name
  `cockpit_live` overrides analyst's `trading-cockpit` to mirror the
  existing `cockpit` bin's prefix and the `_live` suffix carries the
  semantic. **Q2**: option (a) — iced on main thread, multi-thread tokio
  runtime hosted on a side thread, bus + kill_switch + cancel token
  shared via Arc clone. **Q3**: iced-led `CancellationToken` shutdown,
  2 s wall-clock bound enforced by `tokio::time::timeout` wrapping the
  side-thread join. **Q4**: single `config/agent.toml` with new
  `[observability].prometheus_enabled: bool` (default true); no
  `[cockpit]` section in v1. **Q5**: `in_process_cron` opt-in unchanged;
  the cockpit_live bin re-exports the gate. **Q6**: single shared
  `Arc<KillSwitch>`; cockpit `Message::KillConfirmed` calls
  `kill_switch.trip(HaltReason::ManualOperator)` via a closure that
  captures the side-thread tokio Handle. T809 dual-write invariant
  preserved by sticky-trip semantics. **Q7**: remove `cockpit
  --features live` empty-bus path; keep `trading` headless and `cockpit
  --features fixtures`. **Q8**: zero new UI surface; one tooltip line
  edit (T907 [ui-designer]). **Bus-wiring scope: in-scope** — analyst
  finding #1 (only the watcher publishes today) is closed by tasks
  T903a (paper engine publishes fills + positions), T903b (data feed
  taps publish bars + ticks), T903c (reconciler publishes pnl), T905
  (mode-broadcast forwarder bridges kill_switch.subscribe() to
  bus.publish_mode). Without those wires the unified-binary contract
  is structurally false (operator boots, every panel sits in
  `Loading` forever). **Analyst finding #2** (cockpit
  `Message::KillConfirmed` only sets `KillState::Flattening`)
  confirmed by re-reading `crates/ui/src/state.rs:397–402`; closed by
  Q6 + T906. Risks register: 6 entries (iced/tokio thread mixing,
  cockpit panel drift on full bus, shutdown deadlock, config schema
  break, headless-bin regression, bus-clone-leak past shutdown), each
  with a named mitigation tied to a task acceptance criterion. Test
  strategy maps each V-item to a fixture/test path; V3 (graceful
  shutdown timing) requires real-binary subprocess launch via
  `assert_cmd` — covered by T910. Operator-success-report invariants
  (T805, T806, T809, T810) preserved by construction; assertions
  documented per-invariant. Status `draft → in-progress`; owner
  `analyst → architect`.
- 2026-05-01 (analyst): initial draft. Promotes the deferred
  same-process unified binary from
  [v0-paper-sma.md → "What wired up"](../v0-paper-sma/feature.md) (lines
  1338–1400 + 1437–1441) into its own feature brief. Documents
  the operator gap (cockpit + agent never share a bus today),
  enumerates 15 R-items + 10 V-items, and lists 8 Open Questions
  for the architect (binary placement, runtime topology, shutdown
  ordering, config, in-process-cron interaction, kill-switch
  unification, two-binary-path retention, UI touchpoints).
  Cites operator-success-reports T805 / T806 / T809 / T810
  invariants the unified binary must preserve. No backtest
  scenarios — pure plumbing.
- 2026-05-01 (developer-B, Wave 2): T903b (data feed bar/tick taps in
  `agent::runtime::run`), T905 (mode-broadcast forwarder bridging
  `KillSwitch::subscribe()` → `bus.publish_mode`), and T903c (reconciler
  publishes `PnlSnapshot` via new `after_bar_close` + bus builder)
  landed. All three honest-tick conditions met: file:line + test
  command + green output. T809 dual-write invariant preserved —
  `KillSwitch::trip` unchanged; forwarder is downstream consumer only.
  Determinism intact: every money field uses `rust_decimal::Decimal`
  via `Money<Usdt>`; backtest reconciler path defaults `bus = None`
  so report bytes are unchanged (R15 anchor gate pass-by-construction).
  Wave 2 tests: 30 lib + 23 integration green; clippy + fmt clean.
- 2026-05-01 (developer-D, Wave 2): T904 (`cockpit_live` bin skeleton)
  landed. New file `crates/ui/src/bin/cockpit_live.rs` (464 LOC) +
  additive `crates/ui/Cargo.toml` stanzas (new `[[bin]] cockpit_live
  required-features = ["live"]`, expanded `live` feature pulling
  `audit`/`strategy`/`tokio-util`/`anyhow`/`clap`/`tracing-subscriber`/
  `uuid` as optional deps, new `in_process_cron = ["live",
  "agent/in_process_cron"]` pass-through per Q5). Runtime topology
  matches Design Q2: short-lived `current_thread` bootstrap runtime
  drives async ledger/uptime open before spawning a side-thread
  `multi_thread` runtime hosting `agent::runtime::run` + Ctrl-C
  listener + `shutdown_writer`; iced runs on the main thread with
  `ui::live::subscription(Arc<EventBus>)` wired through. Shutdown
  per Design Q3: post-`iced::run` `cancel.cancel()` + 2 s
  `join_with_deadline()` poll loop on the side thread (force
  `std::process::exit(0)` with `shutdown_deadline_exceeded` warning
  on overrun). All four operator-success-report invariants preserved
  (T805/T806/T809/T810 flow through `agent::runtime::run`); T810
  `in_process_cron` builds clean for both `trading` and `cockpit_live`
  bins. Both `Cockpit::kill_switch` field wiring and the
  `Message::ShutdownRequested` window-close → cancel bridge are
  deferred to T906 (ui-designer); skeleton holds the
  `Arc<KillSwitch>` on `AppState._kill_switch` so T906 has a single
  named field to read from. Determinism: no `SystemTime::now()` in
  any backtest replay path (this binary is not a backtest harness);
  no `f64` in money math (the binary touches no money math). Anchor
  gate (R15) — pass-by-construction; T904 only adds files under
  `crates/ui/`. Build/test matrix: `cargo build -p ui --features live
  --bin cockpit_live` clean; `cargo build -p ui --features
  in_process_cron --bin cockpit_live` clean; `cargo build --release
  --bin cockpit_live --features ui/live` clean; `cargo build
  --release --bin cockpit_live --features ui/in_process_cron` clean;
  `cargo test -p ui --features live` 32 lib tests pass;
  `cargo clippy --workspace --all-targets --all-features --
  -D warnings` clean; `cargo fmt --all -- --check` clean;
  `cargo build --release --bin trading` (Wave 1 invariant) clean.
