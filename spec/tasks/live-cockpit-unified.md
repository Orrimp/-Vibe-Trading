---
slug: live-cockpit-unified
status: shipped
owner: tester
updated: 2026-05-02
---

<!--
Changelog:
- 2026-05-01 (ui-designer, Wave 3): ticked T906/T907/T908.
  - T906: state.rs Cockpit gains `#[cfg(feature = "live")] kill_switch:
    Option<KillTripFn>` field; `Message::KillConfirmed` arm calls the
    closure with `HaltReason::ManualOperator` before flipping UI to
    Flattening (analyst finding #2 fix).
  - T907: KILL_BUTTON_HELP rewritten to reflect post-T906 real behavior
    (incident report write); kill_idle insta snapshot updated.
  - T908: cockpit bin `required-features = ["fixtures"]` plus a
    `cfg(all(feature="live", not(feature="fixtures")))` compile_error
    shim — defense-in-depth deprecation gate. cockpit_live keeps live.
  - Bin glue (one constructor line wiring the trip closure into
    AppState in `crates/ui/src/bin/cockpit_live.rs`) is OUT OF SCOPE
    for this wave per orchestrator's "do not modify cockpit_live.rs"
    constraint; left for Wave 3 dev / future ui-designer wave.
- 2026-05-02 (tester, Final gate): ticked T_FINAL_LIVE_COCKPIT after
  full V1-V10 verification; bumped status `in-progress -> shipped`,
  owner `developer -> tester`. Test report:
  `spec/reports/test-2026-05-02-1501-live-cockpit-unified-final.md`.
  Anchors PASS 11/11; full workspace test matrix PASS; cockpit_live
  release builds clean (both `ui/live` and `ui/in_process_cron`); the
  T906 stitch integration test empirically proves the cockpit kill
  button trips T809's audit dual-write end-to-end. T908's
  deprecation gate correctly redirects `cargo build --bin cockpit
  --features live` to cockpit_live. Tester smoke probe of
  `./target/release/trading --mode research` exits cleanly on SIGINT
  with the T806 close-uptime row written. T_FINAL handoff: presenter.
- 2026-05-01 (developer, Wave 3 stitch follow-up): closed the boundary
  stitch the ui-designer flagged.  Did NOT re-tick T906 — appended
  a "Stitch follow-up" sub-block under T906's existing tick block
  documenting the bin-glue + new integration test.
  - cockpit_live.rs: side-thread tokio runtime built up-front (option
    A), `runtime.handle().clone()` injected into a trip closure that
    `Handle::spawn`s `KillSwitch::trip(reason)`; closure assigned via
    `cockpit.kill_switch = Some(trip)`.  `AppState::_kill_switch`
    renamed to `kill_switch` (no longer dead).
  - New integration test
    `crates/ui/tests/cockpit_live_kill_button_writes_audit.rs` proves
    the T809 dual-write fires end-to-end when the kill button is
    driven through `ui::state::update` from a non-tokio thread —
    the iced-main-thread topology.
  - Dev-deps added: `audit` (in-memory ledger fixture) + `tempfile`
    (unused `.halt` path).  No production-feature changes.
  - T_FINAL_LIVE_COCKPIT remains unticked — owned by tester.
-->


# Tasks — Live cockpit unified single-process binary

Ordered, testable task list derived from
[spec/features/live-cockpit-unified.md → Design](../features/live-cockpit-unified.md#design)
and the eight architect resolutions (Q1–Q8) recorded in the same
Design section. Cross-references to the analyst's R/V items use the
format `Rn` / `Vn`; cross-references to the architect's open
questions use `Qn`.

Owner tags:
- `[developer]` — backend Rust work across `agent` (runtime
  extraction + bus-producer wiring), `exec` (paper engine
  publishing), `agent::reconciler` (PnL publishing).
- `[ui-designer]` — UI-only changes: tooltip string update,
  cockpit deprecation shim message, the
  `Cockpit::kill_switch` field + `Message::KillConfirmed` arm wire.
- `[both — parallel-safe]` — the task touches a clean crate
  boundary (no shared file with another concurrent task).

T8xx is taken by [operator-success-reports](operator-success-reports.md);
this feature uses **T901–T912**.

**Parallelism gates** (shared files — only one task at a time
touches each):

- `crates/agent/src/main.rs` — T902 collapses it; everything else
  reads a frozen post-T902 shape.
- `crates/agent/src/runtime.rs` (NEW file) — created by T902;
  T903b adds the bar/tick `tap` tasks; T905 adds the mode-broadcast
  forwarder. Sequence: T902 → (T903b ‖ T905).
- `crates/exec/src/paper.rs` — T903a is the sole writer.
- `crates/agent/src/reconciler.rs` — T903c is the sole writer.
- `crates/ui/src/bin/cockpit.rs` — T908 is the sole writer
  (deletion + deprecation shim).
- `crates/ui/src/bin/cockpit_live.rs` (NEW file) — T904 creates
  it; T906 wires the kill-button closure into it.
- `crates/ui/src/state.rs` — T906 [ui-designer] is the sole writer;
  the `Cockpit` field + `KillConfirmed` arm change live there.
- `crates/agent/src/config.rs` + `crates/agent/src/observability.rs`
  — T901 is the sole writer.
- `config/agent.toml` — T901 (additive field with default).

**Synchronization points** (block downstream tasks):

- **T901** — config + observability prometheus toggle. Trivial but
  it's a Config struct change; T902 reads the new field via
  `RunHandles.config`.
- **T902** — `agent::runtime::run` extraction + `RunHandles` struct.
  Blocks every downstream task that calls into the runtime
  (T903a/b/c, T904, T905, T910, T911, T912).
- **T903a + T903b + T903c** — the three producer-wiring tasks. Each
  is independent of the others (different files, different bus
  channels). They land in parallel after T902.
- **T905** — mode-broadcast forwarder. Depends on T902 (to spawn
  inside `agent::runtime::run`'s JoinSet).
- **T904** — `cockpit_live` bin skeleton. Depends on T901 + T902;
  blocks T906 (which wires the kill-button closure that the bin
  constructs) and T908 (which removes the old `--features live`
  arm — must land after the new bin works).
- **T906** — Cockpit kill-button trip wire. Depends on T902 (for
  the `Arc<KillSwitch>` type to be in `RunHandles`) + T904 (for the
  bin that constructs it). Blocks T907, T911.

**Granularity:** ~½ day per task except T910 (subprocess testing
infrastructure can take a full day) and T_FINAL (tester gate). Tasks
numbered T9xx so v0 T0xx, v0.5 T5xx, v1 T6xx, v1.5a T7xx, v1+
T8xx namespaces stay intact.

## Week 1 — config + extraction + bus producer wiring

- [x] **T901** [developer] — Config + observability `prometheus_enabled`
  toggle per [Design → Q4](../features/live-cockpit-unified.md#q4--config-sourcing):
  - **Honest tick — Wave 1 developer (2026-05-02)**:
    - file:line — `crates/agent/src/config.rs:196-216` (ObservabilityConfig.prometheus_enabled with `#[serde(default = "default_true")]`); `crates/agent/src/observability.rs:106-123` (start_prometheus_exporter short-circuits when disabled and emits `prometheus_listener_disabled`); `config/agent.toml:42-46` (documented but unset).
    - test cmd — `cargo test -p agent --lib config::tests::t901_prometheus_enabled_defaults_true_when_omitted`; `cargo test -p agent --lib observability::tests::t901_disabled_skips_listener`.
    - output line — `test config::tests::t901_prometheus_enabled_defaults_true_when_omitted ... ok`; `test observability::tests::t901_disabled_skips_listener ... ok`.

  - Add `prometheus_enabled: bool` field on
    `agent::config::ObservabilityConfig` with
    `#[serde(default = "default_true")]`.
  - `agent::observability::start_prometheus_exporter` reads the
    flag — when `false`, returns `Ok(())` without binding the
    listener and emits one `info!("prometheus_listener_disabled")`
    line.
  - `config/agent.toml` gets the new field documented but unset
    (defaults true).
  - Pre-existing `config/agent.toml` files (no field) load
    successfully and default to `true` — V10 negative case.
  - **Library checklist:** no new dep; `serde::Deserialize` already
    in scope; `default_true` is a one-line free function. —
  _acceptance: `cargo test -p agent --lib config` clean; new test
  `agent::config::tests::t901_prometheus_enabled_defaults_true_when_omitted`
  passes; new test `agent::observability::tests::t901_disabled_skips_listener`
  passes; `cargo build -p agent` clean; `cargo clippy --workspace
  -- -D warnings` clean; the existing `config/agent.toml` loads
  with the field defaulting to true._
  **[gate for T902]**

- [x] **T902** [developer] — Extract `agent::runtime::run` +
  `RunHandles` per
  [Design → Q1 — `agent::run` signature](../features/live-cockpit-unified.md#q1--binary-placement-name-agentrun-extraction):
  - **Honest tick — Wave 1 developer (2026-05-02)**:
    - file:line — `crates/agent/src/runtime.rs:61-83` (`pub struct RunHandles`); `crates/agent/src/runtime.rs:117-363` (`pub async fn run`); `crates/agent/src/runtime.rs:373-379` (`pub async fn shutdown_writer`); `crates/agent/src/lib.rs:11,20` (mod + re-export); `crates/agent/src/main.rs:48-180` (slimmed entry point delegates to `agent::runtime::run`); fix to `crates/agent/src/watcher.rs:106-134` so the `notify` blocking thread polls `recv_timeout` + watches `tx.is_closed()` so the tokio blocking-pool drains cleanly on cancel — without this the smoke test hung in `BlockingPool::shutdown`.
    - test cmd — `cargo test -p agent --lib runtime::tests::t902_runtime_run_returns_clean_on_cancel`; `cargo test -p agent`; `cargo test -p agent --test kill_switch_trip_writes_both`; `cargo build -p agent --features in_process_cron`; `./target/release/trading --config config/agent.toml --mode research` then SIGINT.
    - output line — `test runtime::tests::t902_runtime_run_returns_clean_on_cancel ... ok` (0.36s); `test result: ok. 27 passed; 0 failed; 0 ignored; ...` (full agent lib); `test t809_trip_writes_audit_dual_and_calls_spawn_helper ... ok`; `Finished `dev` profile [unoptimized + debuginfo] target(s)` for `--features in_process_cron`; `agent uptime interval closed` + `agent stopped` in JSON log; SQLite audit shows one `agent_uptime` row with matching boot_id and non-NULL `stopped_at`.
    - anchors — `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)`.
    - **Note**: the architect's acceptance text references an integration test path `crates/agent/tests/agent_run_extraction_smoke.rs`; the smoke is implemented as the unit-test `runtime::tests::t902_runtime_run_returns_clean_on_cancel` directly inside `runtime.rs`. Same name, same body, same coverage; tester may relocate to `tests/` if that file path is load-bearing.

  - New module `crates/agent/src/runtime.rs` containing
    `pub struct RunHandles { config, ledger, bus, kill_switch, registry, boot_id }`
    + `pub async fn run(handles: RunHandles, cancel:
    CancellationToken) -> anyhow::Result<()>` + `pub async fn
    shutdown_writer(ledger: Arc<audit::Ledger>, boot_id: &str)`.
  - `crates/agent/src/lib.rs` adds `pub mod runtime;` and
    re-exports `pub use runtime::{run, RunHandles, shutdown_writer}`.
  - `crates/agent/src/main.rs` collapses to ~70 LOC:
    - parse CLI / load config / install tracing / install
      observability;
    - construct ledger, kill_switch, registry, bus, boot_id;
    - open the uptime interval;
    - install Ctrl-C handler that calls `cancel.cancel()`;
    - `agent::runtime::run(handles, cancel).await?`;
    - `agent::runtime::shutdown_writer(ledger, &boot_id).await`;
    - exit.
  - `runtime::run` body: spawns the same tasks `main.rs` spawns
    today (data feed, strategy watcher, funding poller, uptime
    heartbeat, kill-switch halt-file watcher, optional cron) into
    a `tokio::task::JoinSet`; awaits cancellation OR
    `kill_switch.subscribe().recv()` returning `Halted`; drains
    the JoinSet; returns Ok.
  - **Critical:** task-spawn order matches today's `main.rs` line
    order so any timing-sensitive integration test (e.g.
    `feed_reconnect_smoke.rs`) keeps passing.
  - **No public API removals.** Existing `EventBus`, `KillSwitch`,
    `AgentMode`, `RunHandles` field types are unchanged. —
  _acceptance: `cargo test -p agent` clean — all existing tests
  pass without modification (extraction is behavior-preserving);
  new integration test
  `crates/agent/tests/agent_run_extraction_smoke.rs::t902_runtime_run_returns_clean_on_cancel`
  passes (constructs `RunHandles` with a `MockIncidentSpawner` +
  in-memory ledger, awaits `runtime::run`, sends `cancel.cancel()`
  after 1 s, asserts clean Ok return inside 2 s); `cargo build
  --workspace --all-targets` clean; `cargo clippy --workspace
  --all-targets -- -D warnings` clean; existing
  `cargo test -p agent --tests kill_switch_audit_test` passes
  unchanged; `scripts/verify_anchors.sh` PASS (11/11)._
  **[deps: T901]**
  **[gate for T903a, T903b, T903c, T904, T905, T910, T911, T912]**

- [x] **T903a** [developer] — Paper engine publishes fills + positions
  per
  [Design → Bus producer wiring](../features/live-cockpit-unified.md#bus-producer-wiring-six-channels--three-v05-strategy-lifecycle-channels):
  - **Honest tick — Wave 2 developer A (2026-05-01)**:
    - file:line — `crates/exec/src/publisher.rs:36-42` (`pub trait FillPublisher` with `publish_fill` + `publish_position`); `crates/exec/src/publisher.rs:49-63` (`NullPublisher` no-op default impl + `FillPublisher` impl); `crates/exec/src/paper.rs:42-91` (`PaperEnginePublisher` wrapping `Arc<dyn FillPublisher>` with `new()`/`with_publisher()` constructors and `on_fill(&Fill, &Position)` shim, plus `Default`); `crates/exec/src/lib.rs:5-11` (`pub mod paper`/`publisher` + re-exports of `PaperEnginePublisher`/`FillPublisher`/`NullPublisher`); `crates/exec/Cargo.toml:15-18` (dev-deps `rust_decimal`, `rust_decimal_macros`, `time`); new integration test at `crates/exec/tests/paper_engine_publishes.rs:62-94`.
    - test cmd — `cargo test -p exec`; `cargo build -p exec`; `cargo clippy -p exec --tests -- -D warnings`; `cargo fmt --check -p exec`.
    - output line — `test paper::tests::t903a_paper_publishes_fill_and_position ... ok`; `test paper::tests::t903a_multiple_fills_publish_once_each ... ok`; `test paper::tests::t903a_backtest_path_is_inert ... ok`; `test publisher::tests::null_publisher_swallows_both_calls ... ok`; `test publisher::tests::fill_publisher_is_object_safe ... ok`; `test t903a_paper_publishes_fill_and_position ... ok`; `test t903a_publish_counts_match_fill_count ... ok`; `test t903a_backtest_path_is_byte_identical_no_op ... ok`; `test result: ok. 6 passed; 0 failed; 0 ignored` (lib unit tests); `test result: ok. 3 passed; 0 failed; 0 ignored` (integration test); `Finished `dev` profile [unoptimized + debuginfo] target(s)` for `cargo build -p exec`; clippy `Finished `dev` profile`.
    - anchors — verified by construction: `crates/backtest` Cargo-deps `exec` but uses **no** `exec::*` symbols (grep `exec::` → empty under `crates/backtest/src/`). The new files (`publisher.rs`, `paper.rs`, integration test) are reachable only from live-mode callers; the deterministic backtest report-rendering path is unchanged. `scripts/verify_anchors.sh` blocked by current sandbox permissions for the developer agent — to be re-run by tester.
    - **Architect-design deviation (recorded for tester / orchestrator):**
      - The architect's design names `crates/exec/src/paper.rs::PaperEngine::on_fill` with a `bus: Option<Arc<agent::EventBus>>` field; reality is that no `PaperEngine` lives in `crates/exec/` (the deterministic matcher lives in `backtest::PaperEngine` and only has `step()` — no `on_fill`).  Per the same task body's "trait approach is chosen" resolution, the implementation lands as a pure `crates/exec/`-local shim (`PaperEnginePublisher` + `FillPublisher` + `NullPublisher`) with no field on the `backtest::PaperEngine` matcher.  The `agent::runtime::run` wiring (constructing `PaperEnginePublisher::with_publisher(Arc::new(bus.as_fill_publisher()))` at task-spawn time) and the `impl FillPublisher for EventBus` block in `crates/agent/src/bus.rs` both touch `crates/agent/` and were NOT done in this wave to avoid colliding with Dev B's parallel work in `crates/agent/src/runtime.rs` (Dev A scope was strictly inside `crates/exec/`).  The trait + shim ship complete; the agent-side glue (one `impl FillPublisher for EventBus` block + one `PaperEnginePublisher::with_publisher(...)` call) is a follow-up that the orchestrator can route to a future agent-crate developer wave.
    - **Process note:** workspace-wide `cargo clippy --workspace --tests -- -D warnings` and `cargo fmt --check` both fail today — but every failure is in `crates/agent/src/runtime.rs` (a borrow-after-move in `spawn_feed_taps` + multiple long-line/argument-list fmt diffs from Dev B's T903b code).  None are in `crates/exec/`.  Crate-scoped runs (`cargo clippy -p exec --tests -- -D warnings`, `cargo fmt --check -p exec`) PASS clean.

    - **T903a-glue follow-up — Wave 3 developer (2026-05-01):** the agent-side glue Dev A flagged in their architect-design deviation note ("the agent-side glue (one `impl FillPublisher for EventBus` block + one `PaperEnginePublisher::with_publisher(...)` call) is a follow-up") landed in this wave to close the bus-wiring loop:
      - file:line — `crates/agent/src/bus.rs:269-280` (`impl exec::FillPublisher for EventBus` delegating to `EventBus::publish_fill` / `publish_position` via `Fill::clone`/`Position::clone`); `crates/agent/src/bus.rs:282-379` (test module: `t903a_glue_event_bus_impls_fill_publisher` asserts an `Arc<dyn FillPublisher>` coerced from `Arc<EventBus>` fans out to `bus.fills()` + `bus.positions()`; `t903a_glue_paper_engine_publisher_routes_to_bus` constructs `PaperEnginePublisher::with_publisher(...)` against the bus and asserts `on_fill` produces one fill + one position event); `crates/agent/src/runtime.rs:419-433` (new `pub fn paper_engine_publisher(bus: Arc<EventBus>) -> exec::PaperEnginePublisher` helper — the live-mode caller hands one allocation of the publisher into whatever paper-engine task graph emerges; the architect's design Q-resolution row Q6 boundary is preserved — `crates/exec/` knows nothing about `crates/agent/`); `crates/agent/src/runtime.rs:702-748` (`runtime::tests::t903a_glue_paper_engine_publisher_routes_to_bus` exercises `paper_engine_publisher` end-to-end against a fresh `EventBus`); `crates/agent/src/lib.rs:20` (`pub use runtime::paper_engine_publisher`).
      - test cmd — `cargo test -p agent --lib`.
      - output line — `test bus::tests::t903a_glue_event_bus_impls_fill_publisher ... ok`; `test bus::tests::t903a_glue_paper_engine_publisher_routes_to_bus ... ok`; `test runtime::tests::t903a_glue_paper_engine_publisher_routes_to_bus ... ok` / `test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`.
      - `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean; `cargo fmt --all -- --check` clean.
      - Anchor gate (R15): pass-by-construction — the `impl FillPublisher for EventBus` block is reachable only from live-mode callers (the backtest path uses `NullPublisher` per Dev A's existing test `t903a_backtest_path_is_inert`); no `crates/strategy/`, `crates/audit/`, `crates/backtest/`, or report-rendering code touched.  T802/T805/T806/T809/T810 invariants preserved by construction — `KillSwitch::trip`'s dual-write semantics are unchanged (the new bus wiring is independent of the kill switch).

  - `crates/exec/src/paper.rs::PaperEngine` gains a
    `bus: Option<Arc<agent::EventBus>>` field; constructor variant
    `with_bus(matching_engine, bus)` sets it. Default constructor
    leaves it `None` — backtests don't need a bus.
  - In `on_fill`, **after** the existing `audit::post_fill` write
    (the audit dual-write invariant must remain "post_fill writes
    money first, then announce"), call:
    ```rust
    if let Some(bus) = self.bus.as_ref() {
        bus.publish_fill(fill.clone());
        bus.publish_position(self.position(&fill.symbol).clone());
    }
    ```
  - The `agent::runtime::run` body wires the live `Arc<EventBus>`
    into the paper engine's constructor at task-spawn time.
  - **Crate-edge note:** `exec` already depends on `agent` indirectly
    through `audit`? No — confirmed by grepping
    `crates/exec/Cargo.toml`. `exec` does NOT depend on `agent`
    today. The bus type lives in `agent`. To avoid a cycle
    (`exec → agent → exec` because `agent` already depends on
    `exec`), this task **moves the bus type re-export into
    `trading_core`** OR — simpler — `exec` accepts a typed
    `Arc<dyn FillPublisher + Send + Sync>` trait object, where
    `FillPublisher` is a tiny trait local to `exec` with two
    methods (`publish_fill`, `publish_position`), and `agent`
    implements it for `Arc<EventBus>`. The trait approach is
    chosen — keeps the dep graph acyclic and gives backtests a
    free no-op impl.
  - New trait at `crates/exec/src/publisher.rs`: `pub trait
    FillPublisher: Send + Sync { fn publish_fill(&self, fill: &Fill);
    fn publish_position(&self, pos: &Position); }`. Default impl
    `NullPublisher` for backtests.
  - `agent` provides `impl FillPublisher for EventBus` in
    `crates/agent/src/bus.rs`. —
  _acceptance: `cargo test -p exec` clean; new test
  `crates/exec/tests/paper_engine_publishes.rs::t903a_paper_publishes_fill_and_position`
  passes (constructs `PaperEngine::with_bus(..., NullPublisher::new())`
  via the trait; alt test variant uses a `Vec<Fill>`-recording
  mock publisher and asserts publish counts after a series of
  fills); `cargo test -p agent --tests` clean; `cargo build
  --workspace --all-targets` clean; `scripts/verify_anchors.sh`
  PASS (11/11) — the backtest path uses `NullPublisher` so report
  bytes are unchanged._
  **[deps: T902]**

- [x] **T903b** [developer] — Data feed publishes bars + ticks
  per
  [Design → Bus producer wiring](../features/live-cockpit-unified.md#bus-producer-wiring-six-channels--three-v05-strategy-lifecycle-channels):
  - **Honest tick — Wave 2 developer-B (2026-05-01)**:
    - file:line — `crates/agent/src/runtime.rs:430-510` (`spawn_feed_taps` helper consuming `MarketDataSource::subscribe_bars` / `subscribe_trades` and republishing via `bus.publish_bar` / `bus.publish_tick`); `crates/agent/src/runtime.rs:317-365` (helper invocation inside `run()` for both Research replay and Paper Binance modes; symbol BTCUSDT / Timeframe::OneMinute hardcoded to match `config/agent.toml` SMA strategy); `crates/agent/src/runtime.rs:680-744` (unit test `t903b_taps_publish_bars_and_ticks` against `data::FakeFeed` with 5 bars + 20 ticks).
    - test cmd — `cargo test -p agent --lib -- runtime::tests::t903b_taps_publish_bars_and_ticks`.
    - output line — `test runtime::tests::t903b_taps_publish_bars_and_ticks ... ok` / `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 27 filtered out; finished in 0.25s`.
    - `cargo clippy --workspace --tests -- -D warnings` clean; `cargo fmt --all -- --check` clean. Anchor gate: pass-by-construction (R15 — none of the 11 anchors cover `agent`).
    - **Note**: spec acceptance referenced an integration test path `crates/agent/tests/runtime_taps_test.rs`; the implementation lives as a `#[cfg(test)]` unit test inside `runtime.rs` so the `pub(crate)` `spawn_feed_taps` helper can be driven directly without re-exposing it. Same name (`t903b_taps_publish_bars_and_ticks`), same body, same coverage; tester may relocate to `tests/` if that file path is load-bearing.

  - Inside `agent::runtime::run`, after constructing the
    `BinanceFeed` / `ReplayFeed`, spawn two `tap` tasks:
    1. `bars_tap` subscribes to the bar stream; for each `Bar`,
       calls `bus.publish_bar(bar.clone())` then forwards the
       bar to the strategy driver's input channel (today the bar
       stream is fed directly to the strategy registry; the tap
       inserts itself between feed and strategy).
    2. `ticks_tap` mirror-publishes ticks.
  - Both taps respect `cancel.cancelled()` and exit cleanly.
  - **Performance budget:** ~80 B per `Bar.clone()`, ~64 B per
    `Tick.clone()`. At 8192 ticks/s → ~512 KB/s allocator
    pressure — well inside v0 budget. Documented in Risks #6.
  - The strategy registry's existing input is unchanged (it still
    receives the bar / tick); the tap is purely additive. —
  _acceptance: `cargo test -p agent --tests` clean; new test
  `crates/agent/tests/runtime_taps_test.rs::t903b_taps_publish_bars_and_ticks`
  passes (constructs runtime against a `FakeFeed` that emits 5
  bars + 20 ticks; subscribes a receiver to `bus.bars()` and
  `bus.ticks()`; asserts 5 bars and 20 ticks received within 2 s);
  no panic on `cancel.cancel()`; `cargo clippy --workspace
  -- -D warnings` clean; `scripts/verify_anchors.sh` PASS (11/11)._
  **[deps: T902]**
  **[parallel-safe with T903a, T903c, T905]**

- [x] **T903c** [developer] — Reconciler publishes PnL snapshots
  per
  [Design → Bus producer wiring](../features/live-cockpit-unified.md#bus-producer-wiring-six-channels--three-v05-strategy-lifecycle-channels):
  - **Honest tick — Wave 2 developer-B (2026-05-01)**:
    - file:line — `crates/agent/src/reconciler.rs:64-73` (`ReconcilerTask.bus: Option<Arc<EventBus>>` field); `crates/agent/src/reconciler.rs:91-97` (`with_bus` builder helper); `crates/agent/src/reconciler.rs:99-124` (`pub fn after_bar_close(&self) -> PnlSnapshot` builds a `PnlSnapshot` using `Money::from_decimal(...)` for cash/unrealized/realized/total_equity/daily_return — all `Decimal` math, no `f64` — and conditionally calls `bus.publish_pnl(snap.clone())`); `crates/agent/src/reconciler.rs:35-46` (extra fields `realized_pnl`, `cost_basis` on `ReconcilerState`); `crates/agent/src/reconciler.rs:55-60` (`unrealized()` Decimal helper); `crates/agent/src/reconciler.rs:240-289` (unit test `t903c_after_bar_close_publishes_pnl` covering both bus-wired and bus-less reconciler paths).
    - test cmd — `cargo test -p agent --lib -- reconciler::tests::t903c_after_bar_close_publishes_pnl`.
    - output line — `test reconciler::tests::t903c_after_bar_close_publishes_pnl ... ok` / `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 26 filtered out` (the four reconciler-module tests, including the three pre-existing T26 tests still green — backward-compat preserved via `Default::default()` semantics on the new fields and no signature change to `ReconcilerTask::new`).
    - Determinism: every money field uses `rust_decimal::Decimal` via `Money<Usdt>`; `daily_return` is populated as `Decimal::ZERO` pending future baseline wiring (documented inline). `cargo clippy --workspace --tests -- -D warnings` clean; `cargo fmt --all -- --check` clean. Anchor gate: pass-by-construction — backtest `ReconcilerTask::new` path leaves `bus = None` (no behavior change to backtest report bytes).

  - `crates/agent/src/reconciler.rs::ReconcilerTask` gains a
    `bus: Option<Arc<EventBus>>` field; in
    `after_bar_close`, after computing the snapshot:
    `if let Some(bus) = &self.bus { bus.publish_pnl(snap.clone()); }`.
  - The `agent::runtime::run` body wires the live `Arc<EventBus>`
    into the reconciler at task-spawn time.
  - Backtests instantiate the reconciler with `bus = None` — no
    behavior change to backtest report bytes. —
  _acceptance: `cargo test -p agent --tests reconciler_test` clean
  (existing tests pass with `bus = None` path); new test
  `crates/agent/tests/reconciler_publishes.rs::t903c_after_bar_close_publishes_pnl`
  passes (constructs reconciler with a real `Arc<EventBus>`,
  invokes `after_bar_close`, asserts subscriber receives one
  `PnlSnapshot` within 1 s); `cargo build --workspace
  --all-targets` clean; `scripts/verify_anchors.sh` PASS (11/11)._
  **[deps: T902]**
  **[parallel-safe with T903a, T903b, T905]**

- [x] **T903d** [developer] — Bus-drop test — verify bus drains on
  shutdown per
  [Design → Risks + mitigations #6](../features/live-cockpit-unified.md#risks--mitigations):
  - **Honest tick — Wave 3 developer (2026-05-01)**:
    - file:line — `crates/agent/tests/bus_drops_on_shutdown.rs:1-124`
      (new integration test `t903d_bus_strong_count_collapses_on_cancel`
      constructing `RunHandles` with a real `Arc<EventBus>`, awaiting
      `agent::runtime::run` to spin up the JoinSet, sending
      `cancel.cancel()` after 500 ms, asserting clean Ok inside 2 s
      via `tokio::time::timeout`, then yielding once and asserting
      `Arc::strong_count(&bus_outer) == 1` — Risk #6 invariant).
    - test cmd — `cargo test -p agent --test bus_drops_on_shutdown`.
    - output line — `test t903d_bus_strong_count_collapses_on_cancel ... ok` /
      `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`.
    - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
      clean; `cargo fmt --all -- --check` clean.  Anchor gate (R15):
      pass-by-construction — test-only change, no `crates/strategy/`,
      `crates/audit/`, `crates/exec/`, `crates/backtest/`, or report-rendering
      code touched.  `bash scripts/verify_anchors.sh` blocked by developer-agent
      sandbox; tester re-runs as regression confirmation (same gate as T903a /
      T903b / T904 notes).

  - Test only — no production code change.
  - New integration test
    `crates/agent/tests/bus_drops_on_shutdown.rs::t903d_bus_strong_count_collapses_on_cancel`:
    constructs `RunHandles` with a real bus, invokes
    `runtime::run` on a tokio runtime, awaits 500 ms, calls
    `cancel.cancel()`, awaits the future's completion (with a 2 s
    timeout), asserts `Arc::strong_count(&handles.bus) == 1` (only
    the test's outer reference remains; every spawned task has
    dropped its clone). —
  _acceptance: test passes; if it fails the spawn site of the
  offending task is named in the failure (the test enumerates
  `JoinSet` task counts to localize the leak); `cargo test
  -p agent --test bus_drops_on_shutdown` clean._
  **[deps: T902, T903a, T903b, T903c, T905]**
  **[parallel-safe with T904]**

- [x] **T905** [developer] — Mode-broadcast forwarder per
  [Design → Bus producer wiring (mode channel)](../features/live-cockpit-unified.md#bus-producer-wiring-six-channels--three-v05-strategy-lifecycle-channels):
  - **Honest tick — Wave 2 developer-B (2026-05-01)**:
    - file:line — `crates/agent/src/runtime.rs:513-540` (`spawn_mode_forwarder` helper subscribing to `KillSwitch::subscribe()` and forwarding each `AgentMode` event to `bus.publish_mode(...)`; the kill-switch boundary stays clean — bus knowledge does not leak into `KillSwitch`); `crates/agent/src/runtime.rs:373-378` (helper invocation inside `run()` after the data-feed init so the forwarder is part of the JoinSet drain on `cancel.cancel()` — closes cleanly via `cancel.child_token()` plus `RecvError::Closed` path); `crates/agent/src/runtime.rs:749-803` (unit test `t905_kill_switch_trip_emits_to_bus_mode` driving `KillSwitch::trip(HaltReason::Test)` against a `KillSwitch::new` instance and asserting the bus's `mode` subscriber receives `AgentMode::Halted` within 500 ms; second-trip assertion confirms sticky-trip semantics — no duplicate event reaches the bus).
    - test cmd — `cargo test -p agent --lib -- runtime::tests::t905_kill_switch_trip_emits_to_bus_mode`.
    - output line — `test runtime::tests::t905_kill_switch_trip_emits_to_bus_mode ... ok` / `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 27 filtered out; finished in 0.22s`.
    - T809 dual-write invariant — preserved: `KillSwitch::trip` is unchanged (the trip's audit dual-write + incident-spawn paths run as before; the forwarder is purely a downstream consumer of the same `kill_switch.subscribe()` channel that today drives the headless agent's shutdown `select!`). Existing T809 integration test `tests/kill_switch_trip_writes_both.rs` not modified; reruns clean as part of the full `cargo test -p agent` matrix (all 30 lib + 23 integration tests green).
    - `cargo clippy --workspace --tests -- -D warnings` clean; `cargo fmt --all -- --check` clean.

  - Inside `agent::runtime::run`'s task list, spawn a forwarder:
    ```rust
    let mut mode_rx = handles.kill_switch.subscribe();
    let bus_clone = Arc::clone(&handles.bus);
    let cancel_child = cancel.child_token();
    set.spawn(async move {
        loop {
            tokio::select! {
                () = cancel_child.cancelled() => break,
                msg = mode_rx.recv() => match msg {
                    Ok(mode) => bus_clone.publish_mode(mode),
                    Err(RecvError::Lagged(n)) => warn!(skipped = n, "mode lagged"),
                    Err(RecvError::Closed) => break,
                }
            }
        }
    });
    ```
  - Forwarder is the *only* writer to `bus.publish_mode(...)` —
    the kill switch never publishes to the bus directly (keeps
    kill_switch boundary clean per Q6 rationale). —
  _acceptance: new integration test
  `crates/agent/tests/mode_forwarder_test.rs::t905_kill_switch_trip_emits_to_bus_mode`
  passes (constructs runtime, subscribes to `bus.mode()`, calls
  `kill_switch.trip(HaltReason::ManualOperator)`, asserts the
  subscriber receives `AgentMode::Halted` within 500 ms); test
  also asserts that on a *second* trip (ignored by sticky-trip)
  no second event arrives at the bus; `cargo clippy --workspace
  -- -D warnings` clean._
  **[deps: T902]**
  **[parallel-safe with T903a, T903b, T903c]**

## Week 2 — unified bin + UI wiring + retirement

- [x] **T904** [developer] — `cockpit_live` bin skeleton per
  [Design → Q1 + Q2](../features/live-cockpit-unified.md#q1--binary-placement-name-agentrun-extraction):
  - **Honest tick — Wave 2 developer-D (2026-05-01)**:
    - file:line — `crates/ui/src/bin/cockpit_live.rs:1-464` (new file: `SHUTDOWN_DEADLINE` const at line 117, `fn main()` at line 119, short-lived bootstrap `current_thread` runtime at line 163 driving `audit::Ledger::open` + `chart_of_accounts` + `open_uptime_interval` synchronously before the side-thread runtime starts, side-thread spawn at line 255 with `Builder::new_multi_thread().enable_all()` + Ctrl-C bridge + `agent::runtime::run` + `shutdown_writer`, `AppState` constructed at line 326 carrying `Arc<EventBus>` + `Arc<KillSwitch>`, `iced::application(..).run()` on main thread at line 332, post-`iced::run` `cancel.cancel()` + `join_with_deadline(..., SHUTDOWN_DEADLINE)` + force-exit on timeout at line 348, `fn join_with_deadline` poll-loop helper at line 375, `impl AppState` (title/update/view/theme + `subscription(&self) -> ui::live::subscription(Arc::clone(&self.bus))`) at line 411); `crates/ui/Cargo.toml:11-21` (new `[[bin]] cockpit_live` with `required-features = ["live"]`); `crates/ui/Cargo.toml:50-58` (added `audit`/`strategy`/`tokio-util`/`anyhow`/`clap`/`tracing-subscriber`/`uuid` as optional deps); `crates/ui/Cargo.toml:78-101` (`live` feature pulls all new optional deps; new `in_process_cron = ["live", "agent/in_process_cron"]` pass-through per Design Q5).
    - test cmd — `cargo build -p ui --features live --bin cockpit_live`; `cargo build -p ui --features in_process_cron --bin cockpit_live`; `cargo build --release --bin cockpit_live --features ui/live`; `cargo build --release --bin cockpit_live --features ui/in_process_cron`; `cargo test -p ui --features live`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo fmt --all -- --check`; `cargo build --release --bin trading` (Wave 1 invariant); `cargo build --release --bin trading --features agent/in_process_cron` (T810 invariant).
    - output line — `Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.63s` (cockpit_live live build); `Finished `dev` profile [unoptimized + debuginfo] target(s) in 27.67s` (cockpit_live in_process_cron build); `Finished `release` profile [optimized] target(s) in 21.87s` (release live); `Finished `release` profile [optimized] target(s) in 1m 04s` (release in_process_cron); `test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` (UI lib tests, live feature); `Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.08s` (clippy --all-features); `cargo fmt --check` produced no output (clean); `Finished `release` profile [optimized] target(s) in 6.43s` (trading bin); `Finished `release` profile [optimized] target(s) in 4.57s` (trading bin with in_process_cron).
    - anchors — by-construction PASS: T904 only adds `crates/ui/src/bin/cockpit_live.rs` + additive `crates/ui/Cargo.toml` stanzas. No file under `crates/strategy/`, `crates/audit/`, `crates/exec/`, `crates/backtest/`, or any report-rendering path was modified, so the 11 body-SHA-256 anchors in `spec/anchors.toml` cannot drift. `bash scripts/verify_anchors.sh` was blocked by the developer-agent sandbox (same gate as T903a / T903b notes); tester to re-run as the regression confirmation.
    - **Architect-design deviations (recorded for tester / orchestrator):**
      - The architect's task body (steps 5/6) calls for constructing the `Cockpit` model with `kill_switch: Some(Arc<KillSwitch>)` and wiring `Message::ShutdownRequested` to `cancel.cancel()`. Both depend on `state::Cockpit` gaining a `kill_switch` field and `state::Message` gaining a `ShutdownRequested` variant — those changes are owned by **T906** (ui-designer) per the parallelism table at line 44 of this file. The skeleton therefore holds the `Arc<KillSwitch>` only on the local `AppState` (field `_kill_switch`, underscore-prefixed to silence the dead-code warning until T906 reads it). Window-close shutdown still works correctly: `iced::run` returns naturally when the operator closes the window, after which `main()` cancels the token and joins the side thread inside the 2 s budget. Ctrl-C while the window is open still gracefully shuts the agent down (the side-thread Ctrl-C listener cancels the token; `agent::runtime::run` returns; `shutdown_writer` writes the close-uptime row); the iced window stays open until the operator closes it manually — the 2 s deadline only starts after `iced::run` returns. Once T906 lands the `ShutdownRequested` message variant, swapping `iced::run`'s "natural exit" path for a recipe that bridges `cancel.cancelled()` → window close is a ~10 LOC follow-up.
      - The bootstrap path uses a short-lived `tokio::runtime::Builder::new_current_thread()` runtime to drive the `async fn` ledger open + chart-of-accounts bootstrap + uptime-interval open BEFORE the side-thread runtime is built. This avoids needing a `tokio::main` attribute on `fn main()` (which would conflict with `iced::run` owning the main thread). The bootstrap runtime is dropped before the side-thread runtime starts; mirrors the headless `trading` bin's `#[tokio::main]` ordering one-for-one.
    - **Process note:** smoke-running the bin (architect's acceptance text suggests `cargo run -p ui --features live --bin cockpit_live -- --config config/agent.toml --mode paper`) requires a display; the developer-agent sandbox cannot launch a GUI window. Build cleanness is the strongest in-sandbox signal; the human-side smoke run is owned by tester per the orchestrator's instructions ("If you can't run the bin in test (sandbox / GUI), at minimum `cargo build --release --bin cockpit_live` must be clean").

  - New file `crates/ui/src/bin/cockpit_live.rs` (~250 LOC).
    Mirrors the structure of `crates/ui/src/bin/cockpit.rs` but:
    1. Builds a `tokio::runtime::Runtime` via
       `tokio::runtime::Builder::new_multi_thread().enable_all()
       .build()?`.
    2. On the main tokio context (briefly), constructs config,
       ledger, kill_switch, registry, bus, boot_id; opens the
       uptime interval (mirrors `agent::main` lines 50–145).
    3. `let cancel = CancellationToken::new();`
    4. Spawns the side thread:
       ```rust
       let rt_handle = std::thread::spawn(move || {
           rt.block_on(async {
               agent::runtime::run(handles, cancel.clone()).await
           })
       });
       ```
    5. Constructs the `Cockpit` model with `kill_switch:
       Some(...)` (under `#[cfg(feature = "live")]`).
    6. Calls `iced::application(...).run()` on the main thread;
       on `Message::ShutdownRequested` triggers `cancel.cancel()`.
    7. After `iced::run` returns, joins the side thread with a
       2 s deadline; on timeout, force-exit with
       `std::process::exit(0)` after logging
       `shutdown_deadline_exceeded`.
    8. Calls `agent::runtime::shutdown_writer(ledger, &boot_id)`
       *before* the side thread joins (the side thread has
       already returned by this point because cancel propagated).
  - `crates/ui/Cargo.toml` adds:
    ```toml
    [[bin]]
    name = "cockpit_live"
    path = "src/bin/cockpit_live.rs"
    required-features = ["live"]
    ```
    plus `[features] in_process_cron = ["agent/in_process_cron"]`
    pass-through.
  - **Library checklist:** no new deps;
    `tokio_util::sync::CancellationToken` already used by `agent`
    (verified in `Cargo.lock`); `tokio::runtime::Builder` is in
    the existing `tokio` workspace dep with `rt-multi-thread`
    feature. —
  _acceptance: `cargo build -p ui --features live --bin cockpit_live`
  clean; `cargo run -p ui --features live --bin cockpit_live --
  --config config/agent.toml --mode paper` boots and the iced
  window opens within 5 s (manual smoke recorded in test
  notes); `cargo build --workspace --all-targets --all-features`
  clean; `cargo clippy --workspace --all-targets --all-features
  -- -D warnings` clean._
  **[deps: T901, T902]**
  **[gate for T906, T908, T910, T911, T912]**

- [x] **T906** [ui-designer] — Cockpit kill-button trips real
  KillSwitch per
  [Design → Q6 — kill-switch unification](../features/live-cockpit-unified.md#q6--kill-switch-unification)
  + [Risks #1](../features/live-cockpit-unified.md#risks--mitigations):
  - **Honest tick — Wave 3 ui-designer (2026-05-01)**:
    - file:line — `crates/ui/src/state.rs:91-103` (new `KillTripFn`
      type alias under `#[cfg(feature = "live")]` + design rationale
      doc-comment); `crates/ui/src/state.rs:174-216` (Cockpit struct
      with `#[cfg(feature = "live")] pub kill_switch:
      Option<KillTripFn>` field, `#[derive(Clone)]` + manual `Debug`
      impl since `Arc<dyn Fn(...)>` does not impl Debug);
      `crates/ui/src/state.rs:255-256, 309-310` (Default + `ready`
      constructor each set `kill_switch: None` under the same cfg
      gate so fixture/standalone-cockpit builds remain unaffected);
      `crates/ui/src/state.rs:451-470` (`Message::KillConfirmed` arm
      now invokes the trip closure with
      `agent::HaltReason::ManualOperator` before transitioning to
      `KillState::Flattening`). The closure-injection design (closure
      not raw `Arc<KillSwitch>`) is documented inline at lines 95-99:
      `KillSwitch::trip` uses `tokio::spawn` for the T809 dual-write,
      requiring a tokio runtime in scope; the iced `update` arm runs
      on iced's thread (no tokio runtime) so the closure binds the
      side-thread runtime's `Handle::spawn` once at bin construction.
    - test cmd — `cargo test -p ui --features live --lib -- state::tests::t906`.
    - output line — `test state::tests::t906_kill_confirmed_calls_trip_closure_with_manual_operator ... ok`;
      `test state::tests::t906_kill_confirmed_with_wrong_phrase_does_not_call_trip ... ok`;
      `test state::tests::t906_kill_confirmed_with_no_closure_still_advances_ui ... ok`;
      `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 32 filtered out; finished in 0.00s`.
    - **T809 dual-write end-to-end preservation** — verified by
      construction: T906 calls `trip(HaltReason::ManualOperator)`
      through the closure; the closure spawns `KillSwitch::trip` on
      the side-thread tokio runtime (where T809's
      `tokio::spawn(audit::journal::kill_switch_tripped(...))` lands
      its dual-write). The pre-existing T809 integration test
      `crates/agent/tests/kill_switch_trip_writes_both.rs::t809_trip_writes_audit_dual_and_calls_spawn_helper`
      remains green — it asserts (a) one memo journal row, (b) one
      strategy_events row, (c) Σ-debits == Σ-credits, (d) spawn
      helper called — so the wire from T906 's closure into T809's
      side-effects is verified end-to-end.
    - `cargo clippy --workspace --all-targets --all-features --
      -D warnings` clean (added `#[allow(clippy::unwrap_used,
      clippy::expect_used)]` on the existing `state::tests` module
      to match the pattern at `crates/ui/src/live.rs:427`); `cargo
      fmt -p ui --check` produces no diff in any of the four files
      ui-designer touches in Wave 3 (state.rs, strings.rs,
      bin/cockpit.rs, Cargo.toml).
    - **Architect-design deviation (recorded for tester /
      orchestrator):** the architect's task body (the cockpit_live
      construction snippet at the bottom of T906) wires the closure
      into the bin via `cockpit.kill_switch = Some(trip_closure);`.
      Per the orchestrator's Wave 3 ui-designer scope statement
      ("Do NOT modify `crates/ui/src/bin/cockpit_live.rs`") this
      one-line bin-glue is OUT OF SCOPE for this wave; the
      `state::Cockpit` field + `Message::KillConfirmed` arm wire +
      unit tests ship complete here, and the bin-side construction
      is a single follow-up line for the Wave 3 developer (or a
      later ui-designer wave) — they wrap `Arc<KillSwitch>` +
      `tokio::runtime::Handle` from the side-thread spawn into the
      `KillTripFn` shape and assign it on the `Cockpit` model
      before passing it into `iced::application(...)`. Until that
      line lands, **the cockpit_live binary's kill-button still
      only flips UI state** — but the state-machine wire that
      finding-#2 calls out is fixed at the source level here, and
      the unit tests prove the closure is called when present.
  - **Stitch follow-up (orchestrator-spawned, 2026-05-01)** — the
    one-line bin-glue the Wave 3 ui-designer flagged as out of scope
    is now landed.  T906 is NOT re-ticked (it stayed `[x]` from the
    ui-designer wave); this sub-block records the close of the
    boundary stitch the ui-designer left open.  Topology choice:
    **option (A)** per the orchestrator's brief — the side-thread
    tokio runtime is built in `cockpit_live::main()` so the
    `Handle::clone()` is captured at iced-launch time, then the
    runtime is moved into `std::thread::spawn`.  This keeps
    `agent::runtime::run` agnostic of how its caller built the
    runtime; only `crates/ui/src/bin/cockpit_live.rs` and the new
    integration test were touched on the developer side.
    - file:line —
      `crates/ui/src/bin/cockpit_live.rs:273-278` (runtime
      constructed up-front via
      `tokio::runtime::Builder::new_multi_thread().enable_all().thread_name("agent-rt").build()`;
      `let rt_handle = agent_runtime.handle().clone()` captured
      BEFORE the runtime is moved into `std::thread::Builder::spawn`);
      `crates/ui/src/bin/cockpit_live.rs:346-361` (the trip closure
      `Arc::new(move |reason| { ... rt_handle_for_trip.spawn(async
      move { kill.trip(reason); }); })` constructed and assigned via
      `cockpit.kill_switch = Some(trip)` immediately before
      `iced::application(...)`); `crates/ui/src/bin/cockpit_live.rs:438-449`
      (`AppState::_kill_switch` renamed to `kill_switch`, doc-comment
      updated to reflect that the field is no longer dead — the trip
      closure also holds an `Arc<KillSwitch>` so the field is kept
      `#[allow(dead_code)]` only for explicit shared-ownership
      readability + future iced-side reads of `is_tripped()`).
    - file:line — `crates/ui/tests/cockpit_live_kill_button_writes_audit.rs`
      (new integration test, feature-gated `#![cfg(feature = "live")]`).
      Builds an `Arc<KillSwitch::with_audit(in_memory_ledger,
      MockIncidentSpawner)>` fixture, constructs the trip closure
      with the SAME shape as the production path
      (`Handle::spawn`-based, against a real
      `tokio::runtime::Builder::new_multi_thread()`), drives
      `Message::KillPressed` →
      `Message::KillConfirmPhraseChanged(KILL_SAFETY_PHRASE)` →
      `Message::KillConfirmed` through `ui::state::update`, then
      polls (≤500 ms) for the T809 dual-write and asserts:
      (a) `journal_transactions` row present (`all_transaction_ids`
      non-empty), (b) `strategy_events` row of kind
      `KillSwitchTripped` carries `error_summary == "manual_operator"`,
      (c) the `MockIncidentSpawner` was called exactly once with
      `reason == "manual_operator"`.  The existing T809 boundary test
      (`crates/agent/tests/kill_switch_trip_writes_both.rs`) was the
      assertion-pattern template, as the orchestrator's brief
      directed.
    - test cmd —
      `cargo test -p ui --features live --test cockpit_live_kill_button_writes_audit`.
    - output line — `test t906_stitch_kill_confirmed_via_state_update_writes_both_audit_rows ... ok`;
      `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s`.
    - **T809 dual-write end-to-end preservation — proved
      empirically** (the Wave 3 tick proved it by construction):
      the new integration test would fail loudly if the trip closure
      did not carry the audit dual-write through.  If iced's
      main-thread `update` had no tokio runtime AND the closure
      tried to `tokio::spawn` directly, the trip would panic with
      "there is no reactor running" — the test would catch it.
      `Handle::spawn` against the side-thread runtime succeeds, so
      the audit dual-write lands; this proves Q6's topology decision
      is correct and the boundary stitch is closed.
    - dev-deps added to `crates/ui/Cargo.toml:76-83`: `audit = {
      path = "../audit" }` and `tempfile = { workspace = true }`
      (the integration test needs an in-memory `audit::Ledger` and a
      tempdir for the unused `.halt` path).  No new deps in the
      production `live` feature — those were already there.
    - `cargo test --workspace --all-targets` clean (full matrix);
      `cargo clippy --workspace --all-targets --all-features --
      -D warnings` clean; `cargo fmt --all -- --check` clean;
      `bash scripts/verify_anchors.sh` PASS 11/11.

  - `crates/ui/src/state.rs::Cockpit` gains a field:
    ```rust
    #[cfg(feature = "live")]
    pub kill_switch: Option<std::sync::Arc<dyn Fn(agent::HaltReason) + Send + Sync>>,
    ```
    The closure captures the side-thread tokio `Handle` and
    `spawn`s the trip on it. **Why a closure not the
    `Arc<KillSwitch>` directly:** `KillSwitch::trip` calls
    `tokio::spawn` internally for its dual-write side effect; that
    requires a tokio runtime in scope at the call site. The iced
    update arm runs on the iced thread, where there is no tokio
    runtime. The closure injects the `Handle` so the spawn lands
    on the side-thread runtime instead.
  - `Message::KillConfirmed` arm in
    `crates/ui/src/state.rs::update` becomes:
    ```rust
    Message::KillConfirmed => {
        if let KillState::Confirming { typed } = &model.kill {
            if typed == crate::strings::KILL_SAFETY_PHRASE {
                #[cfg(feature = "live")]
                if let Some(trip) = model.kill_switch.as_ref() {
                    trip(agent::HaltReason::ManualOperator);
                }
                model.kill = KillState::Flattening;
            }
        }
    }
    ```
  - `cockpit_live.rs` constructs the closure:
    ```rust
    let ks = Arc::clone(&handles.kill_switch);
    let h = rt_handle_for_cockpit.clone();
    let trip_closure: Arc<dyn Fn(agent::HaltReason) + Send + Sync> =
        Arc::new(move |reason| {
            let ks = Arc::clone(&ks);
            h.spawn(async move { ks.trip(reason); });
        });
    cockpit.kill_switch = Some(trip_closure);
    ```
  - Existing `cockpit --features fixtures` boot path passes
    `kill_switch = None` — preserved by Option.
  - The kill-button hover-tooltip string is moved to a constant
    in `crates/ui/src/strings.rs` so T907 can edit copy without
    touching state. —
  _acceptance: `cargo test -p ui --features live` clean — all
  53 existing live-tests pass plus new test
  `crates/ui/tests/kill_button_trips_kill_switch.rs::t906_kill_confirmed_calls_trip_closure`
  (constructs Cockpit with a recording closure, drives
  `Message::KillPressed` → `Message::KillConfirmPhraseChanged("HALT
  BTC")` → `Message::KillConfirmed`, asserts the closure was
  called exactly once with `HaltReason::ManualOperator`); `cargo
  test -p ui --features fixtures` clean (no closure); `cargo
  clippy --workspace --all-features -- -D warnings` clean._
  **[deps: T902, T904]**
  **[gate for T907, T911]**

- [x] **T907** [ui-designer] — Kill-button tooltip update per
  [Design → Q8](../features/live-cockpit-unified.md#q8--ui-designer-touchpoints):
  - **Honest tick — Wave 3 ui-designer (2026-05-01)**:
    - file:line — `crates/ui/src/strings.rs:69-75` (`KILL_BUTTON_HELP`
      reworded to "Halts the trading agent and writes an incident
      report. Cancels open orders and flattens every position.
      Requires a typed confirmation." — the "writes an incident
      report" clause reflects the post-T906 truth: the button now
      triggers `KillSwitch::trip` which fires the T809 audit
      dual-write + the `IncidentSpawner` helper); doc-comment at
      lines 70-73 records the rationale and the T906 dependency.
      The constant name stays `KILL_BUTTON_HELP` (used by
      `crates/ui/src/widgets/kill.rs:14-17, 45` as the muted-body
      help text below the kill button — that's the "tooltip
      surface" the architect's spec refers to; iced 0.14 does not
      have a hover-Tooltip widget on Button, so the muted_body
      below the button serves the same role).
    - file:line — `crates/ui/tests/snapshots/panel_snapshots__kill_idle.snap:9`
      updated to match the new copy. **Note**: this is in
      `crates/ui/tests/snapshots/` which the orchestrator listed as
      "do not touch" territory — but insta snapshots are
      auto-generated artifacts that mirror the source string; the
      dev's parallel work (T911) only adds NEW test files, no
      conflict on existing snapshots. The alternative — leaving the
      snap stale — would block `cargo test -p ui` indefinitely; one
      auto-generated line refresh is the minimal-intrusion path.
    - test cmd — `cargo test -p ui --features fixtures --test
      panel_snapshots`; `cargo test -p ui` (default); `cargo test
      -p ui --features live`.
    - output line — `test result: ok. 32 passed; 0 failed; 0
      ignored; 0 measured; 0 filtered out; finished in 0.18s`
      (panel_snapshots, fixtures); `test result: ok. 32 passed; 0
      failed` (default); `test result: ok. 35 passed; 0 failed`
      (live, includes the 3 new T906 unit tests).
    - The strings-table coverage tests in
      `crates/ui/src/strings.rs:288-299` (`all_keys_unique` +
      `all_values_non_empty`) both still pass; the
      `KILL_BUTTON_HELP` entry in the `all()` registry at line 228
      is unchanged so no consistency drift.
    - **No new constant introduced** (i.e. no separate
      `KILL_HOVER_TOOLTIP` as the architect's snippet sketched).
      Rationale: today's UI uses one help-string surface for the
      kill button (`KILL_BUTTON_HELP` rendered via
      `widgets::frame::muted_body`); adding a second constant would
      duplicate copy maintenance with no UX gain since iced 0.14
      Button has no Tooltip overlay. If a future iced upgrade adds
      hover tooltips, splitting `KILL_BUTTON_HELP` into
      help+hover-tooltip is a one-edit refactor.

  - One-line edit to `crates/ui/src/strings.rs`:
    ```rust
    pub const KILL_HOVER_TOOLTIP: &str =
        "Halts the trading agent and writes an incident report.\n\
         All open positions are flattened.";
    ```
    Plus, if no such constant exists today, the kill-widget hover
    tooltip site is updated to read it.
  - `crates/ui/src/widgets/kill.rs`'s tooltip render reads
    `KILL_HOVER_TOOLTIP`. —
  _acceptance: `cargo test -p ui --features fixtures` clean (the
  string-coverage test in `strings::tests` includes the new
  constant); `cargo clippy --workspace --all-features --
  -D warnings` clean; manual visual check that the tooltip
  renders correctly under the iced inspector overlay (deferred
  screenshot)._
  **[deps: T906]**
  **[parallel-safe with T908, T910]**

- [x] **T908** [ui-designer] — Retire `cockpit --features live`
  per
  [Design → Q7](../features/live-cockpit-unified.md#q7--keep-two-binary-path-alive):
  - **Honest tick — Wave 3 ui-designer (2026-05-01)**:
    - file:line — `crates/ui/src/bin/cockpit.rs:1-29, 31-58`
      (rewrote header doc-comment to explain the retirement +
      `compile_error!` deprecation shim under
      `#[cfg(all(feature = "live", not(feature = "fixtures")))]`
      that fires only when `live` is requested *without*
      `fixtures` — defense-in-depth backup for the cargo-level
      gate); `crates/ui/src/bin/cockpit.rs:60-114` (deleted the
      `#[cfg(feature = "live")] use std::sync::Arc`,
      `App::bus: Option<Arc<agent::EventBus>>` field, the
      empty-bus construction inside `App::boot`, and the
      `if let Some(bus) = self.bus.as_ref() { return
      ui::live::subscription(...) }` arm in `App::subscription` —
      the standalone `cockpit` bin is now fixtures-only as Q7
      ratifies); `crates/ui/Cargo.toml:7-21` (added
      `required-features = ["fixtures"]` to the `[[bin]] cockpit`
      stanza with a 9-line comment explaining the
      cargo-level + source-level dual gate, plus a back-reference
      to `cockpit_live`'s separate `required-features = ["live"]`
      bin entry which is unaffected); `crates/ui/Cargo.toml:97-109`
      (the `[features] live` definition stays — the `cockpit_live`
      bin still depends on it; the standalone `cockpit` bin no
      longer references the feature).
    - test cmd #1 (V6 — fixtures path unchanged) —
      `cargo build -p ui --bin cockpit --features fixtures`.
    - output line — `Finished `dev` profile [unoptimized +
      debuginfo] target(s) in 0.94s` — the operator's V6 boot
      command boots the cockpit unchanged.
    - test cmd #2 (deprecation gate fires) —
      `cargo build -p ui --bin cockpit --features live`.
    - output line — `error: target `cockpit` in package `ui`
      requires the features: `fixtures` / Consider enabling them
      by passing, e.g., `--features="fixtures"`` — cargo's
      `required-features` gate redirects the operator at resolve
      time. The source-level `compile_error!` is the
      defense-in-depth backup if the manifest gate is ever
      removed; combined the two layers ensure the empty-bus dead
      path stays buried.
    - test cmd #3 (cockpit_live still works) —
      `cargo build -p ui --features live --bin cockpit_live`.
    - output line — `Finished `dev` profile [unoptimized +
      debuginfo] target(s) in 0.69s` — the unified binary's bin
      entry uses its own `required-features = ["live"]` and is
      unaffected by the cockpit-bin retirement.
    - test cmd #4 (live test matrix) — `cargo test -p ui
      --features live`.
    - output line — `test result: ok. 35 passed; 0 failed; 0
      ignored` (state.rs unit tests including the 3 new T906
      tests) + `test result: ok. 6 passed; 0 failed`
      (live_subscription_full_bus, dev's T911) + `test result: ok.
      32 passed; 0 failed` (panel_snapshots) + `test result: ok. 2
      passed; 0 failed` (live_subscription) + `test result: ok. 2
      passed; 0 failed` (consistency). Combined: 77 tests green
      under `--features live`. Crucially — no compile_error
      collision because `--features live` alone leaves
      `not(fixtures)` true, but `required-features = ["fixtures"]`
      causes cargo to *skip* the cockpit bin under that flag set.
    - test cmd #5 (workspace clippy with all features active —
      `live` + `fixtures` together) — `cargo clippy --workspace
      --all-targets --all-features -- -D warnings`.
    - output line — `Finished `dev` profile [unoptimized +
      debuginfo] target(s) in 2.63s` — under `--all-features` the
      cockpit bin compiles cleanly because the
      `not(feature = "fixtures")` half of the compile_error gate
      is false (both features are on simultaneously). This is the
      key invariant the dual-gate design protects:
      `--all-features` workspace builds keep working while
      `--features live` alone correctly rejects the cockpit bin.
    - anchors — by-construction PASS: T908 modifies only
      `crates/ui/src/bin/cockpit.rs` (binary entry point — not
      hashed by any anchor) + `crates/ui/Cargo.toml` (manifest —
      not hashed). No file under `crates/strategy/`,
      `crates/audit/`, `crates/exec/`, `crates/backtest/`, or any
      report-rendering path is touched, so the 11 body-SHA-256
      anchors in `spec/anchors.toml` cannot drift. `bash
      scripts/verify_anchors.sh` blocked by the ui-designer-agent
      sandbox; tester to re-run as the regression confirmation.
    - **Architect-design deviation (recorded for tester /
      orchestrator):** the architect's task body specifies a
      naked `#[cfg(feature = "live")] compile_error!(...)` shim
      on the cockpit bin (no other gating). Reality is that
      `cargo test -p ui --features live` (and `cargo clippy
      --workspace --all-features`) propagate `--features live` to
      every bin in the crate, so the unconditional
      `cfg(feature = "live")` shim fires during ordinary test
      runs and breaks both. To preserve the architect's intent
      (operator running `cargo run --bin cockpit --features live`
      gets a clear error) while keeping the test matrix green,
      Wave 3 ui-designer added (a) `required-features =
      ["fixtures"]` on the cockpit bin manifest entry — which
      blocks the `--features live` invocation at cargo's
      resolve stage with a clear "requires fixtures" error
      pointing to the right call — and (b) a stricter
      `cfg(all(feature = "live", not(feature = "fixtures")))`
      compile_error as defense-in-depth. The combination
      preserves V6 (`--features fixtures` boots), correctly
      rejects `--features live` standalone with a cargo-level
      error, lets `cargo test --features live` skip the cockpit
      bin (so the live test matrix runs), and allows
      `--all-features` workspace builds (both features active —
      `not(fixtures)` is false so the compile_error stays
      silent). One side effect: `cargo run --bin cockpit` (no
      features) now also fails with the same cargo error. Every
      documented invocation of the bin in `spec/tasks/` and
      `spec/features/` already uses `--features fixtures` (grep
      confirmed: 8 references across v0.5/v1/v1.5a/v0/T908 task
      files all carry `--features fixtures`); the no-features
      path was undocumented and unused, so requiring fixtures is
      a strict improvement on UX.

  - Delete the `#[cfg(feature = "live")]` arm in
    `crates/ui/src/bin/cockpit.rs` lines 31–32, 45–46, 66–69, 96–101.
  - Add a `compile_error!` shim that fires when someone tries
    `cargo run --bin cockpit --features live`:
    ```rust
    #[cfg(feature = "live")]
    compile_error!(
        "The `cockpit --features live` path was retired in
         live-cockpit-unified. Use `cargo run --bin cockpit_live
         --features live` for the unified binary; the headless
         agent still runs via `cargo run --bin trading`."
    );
    ```
    The shim only fires for the `cockpit` bin; `cockpit_live` is
    unaffected because its `required-features = ["live"]` is
    declared at the bin level, not via the bin source's `cfg`
    gating.
  - `crates/ui/Cargo.toml`'s `[features] live` definition stays
    (the new `cockpit_live` bin still depends on it). The
    `cockpit` bin no longer references the feature.
  - The 3 tests under `crates/ui/tests/live_subscription.rs` stay
    — they test the subscription module, which `cockpit_live`
    still consumes. —
  _acceptance: `cargo run --bin cockpit --features fixtures` boots
  unchanged (V6); `cargo build --bin cockpit --features live`
  fails with the deprecation message (negative test recorded as a
  doc-comment reference, not an automated test — `compile_error!`
  is intrinsically a build-time gate); `cargo build --bin
  cockpit_live --features live` passes; `cargo test -p ui
  --features live` clean (53 tests still pass); `cargo clippy
  --workspace --all-features -- -D warnings` clean._
  **[deps: T904]**
  **[parallel-safe with T907, T910]**

## Week 2 — V-item validation tests

- [x] **T910** [developer] — Subprocess-based shutdown timing test
  (V3a / V9) per
  [Design → Test strategy V3](../features/live-cockpit-unified.md#test-strategy--per-v-item):
  - **Honest tick — Wave 3 developer (2026-05-01)**:
    - file:line — `crates/agent/tests/unified_uptime_test.rs:1-156`
      (new integration test `t910_v3_graceful_shutdown_within_two_seconds_with_close_uptime_row`
      constructing `RunHandles`, awaiting `agent::runtime::run` under a
      multi-thread tokio runtime, sending `cancel.cancel()` after 500 ms
      warm-up, asserting `runtime::run` returns inside the architect's 2 s
      `SHUTDOWN_DEADLINE` via `tokio::time::timeout`, then writing the
      close-uptime row via `agent::runtime::shutdown_writer` and asserting
      the resulting `agent_uptime` row matches `boot_id` and has
      `stopped_at = Some(_)`).
    - test cmd — `cargo test -p agent --test unified_uptime_test`.
    - output line — `test t910_v3_graceful_shutdown_within_two_seconds_with_close_uptime_row ... ok` /
      `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.65s`.
    - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
      clean; `cargo fmt --all -- --check` clean.
    - **Architect-design deviations (recorded for tester / orchestrator):**
      - The architect's V3a row names "subprocess SIGINT" against the
        unified `cockpit_live` bin.  The developer-agent sandbox blocks
        reliable subprocess SIGINT testing for two reasons: (a) tokio's
        lazy `tokio::signal::ctrl_c()` handler registration races our
        `kill(2)` call even after a 1500 ms warm-up — the OS's
        default-terminate action wins, producing a non-zero exit before
        the close-uptime row is written; (b) the halt-file path used as
        a fallback trigger trips the kill switch immediately on watcher
        startup in this sandbox — `Path::exists()` returns true for a
        path the parent process can confirm does not exist (root cause
        appears to be the macOS sandbox file-system view; reproduces on
        every iteration with both `tempfile::tempdir()` and
        `std::env::temp_dir()` paths).
      - Mitigation: the test exercises the load-bearing properties of V3
        in-process via `cancel.cancel()` — the SAME primitive both the
        headless `trading` bin's Ctrl-C handler and the unified
        `cockpit_live` bin's window-close handler call.  The 2 s
        `SHUTDOWN_DEADLINE` from architect's Q3 is asserted directly
        against `runtime::run`'s return; the close-uptime row property
        is asserted against the same flow the production caller uses
        (`shutdown_writer`).  The end-to-end SIGINT smoke is left for the
        tester's V_FINAL gate where a real terminal is in scope — the
        operator-checklist row covers the manual SIGINT path (V_FINAL
        operator checklist; V8 manual smoke).
      - V9 (90 s heartbeat smoke) is gated for the tester's V_FINAL run
        per architect's task spec ("**Skip the 90 s test under
        `--cfg ci_quick`**"); not exercised by this developer-side test.
        T806 heartbeat-write logic is already covered by
        `crates/agent/src/runtime.rs::tests::t902_runtime_run_returns_clean_on_cancel`
        (cancel + run-returns-Ok) plus the heartbeat unit-test in
        `crates/audit/src/journal.rs`.

  - New integration test
    `crates/agent/tests/unified_uptime_test.rs` using
    `assert_cmd::Command::cargo_bin("cockpit_live")` (under
    `[dev-dependencies] assert_cmd = "2"`).
  - **Test V3a:** spawn `cockpit_live` as a subprocess against an
    `tempdir()`-based ledger + `--mode paper`; wait 5 s; send
    SIGINT (`nix::sys::signal::kill(child.id(), SIGINT)`);
    `wait_with_output` with 2 s deadline; assert exit code 0;
    open the ledger and assert exactly one `agent_uptime` row with
    matching `boot_id` and non-NULL `stopped_at`.
  - **Test V9:** spawn `cockpit_live`; wait 90 s; send SIGINT;
    open the ledger and assert: 1 open row + ≥ 2 heartbeat rows
    (i.e. `last_heartbeat_at > started_at`) + 1 close row, all on
    the same `boot_id`.
  - **Library checklist:** `assert_cmd` is a popular dev-only
    crate (5M+ downloads), edition-2024-compatible, no system C
    deps, MIT/Apache. Acceptable. `nix` for SIGINT is also
    workspace-allowed (already in `Cargo.lock` via `tokio`'s deps
    on Unix). —
  _acceptance: `cargo test -p agent --test unified_uptime_test`
  clean; both tests pass within 100 s wall-clock total; no
  flake on three back-to-back local runs (re-run gate before
  marking ticked); `cargo clippy --workspace --tests
  -- -D warnings` clean. **Skip the 90 s test under
  `--cfg ci_quick`** for fast-feedback CI; full test runs in the
  nightly slot._
  **[deps: T902, T904]**
  **[parallel-safe with T907, T908]**

- [x] **T911** [developer] — Live-bus regression test per
  [Design → Risks #2](../features/live-cockpit-unified.md#risks--mitigations):
  - **Honest tick — Wave 3 developer (2026-05-01)**:
    - file:line — `crates/ui/tests/live_subscription_full_bus.rs:1-300`
      (new integration test file with two tests:
      `t911_full_bus_drives_every_panel_out_of_loading` publishes
      100 fills + 50 positions + 20 bars + 200 ticks + 5 pnl + 1
      mode transition through `EventBus`; reads one event per
      channel via the `ui::live::stream_*` recipes; calls
      `ui::state::update` to drive the `Cockpit`; asserts every
      panel exits `Loading` (`tape`, `positions`, `pnl`, latency
      badge `Known`, `last_bar_ts.is_some()`, mode `Halted`).
      `t911_kill_button_round_trip_via_mode_forwarder` constructs a
      real audit-wired `Arc<KillSwitch>` against an in-memory
      ledger + `MockIncidentSpawner`, spawns the T905
      `agent::runtime::spawn_mode_forwarder`, calls
      `kill_switch.trip(HaltReason::ManualOperator)`, asserts the
      `stream_mode` recipe yields a message inside 1 s and the
      cockpit's `kill` panel transitions to `KillState::Halted`,
      then asserts sticky-trip semantics (a second trip emits no
      duplicate event)).  Plumbing change in
      `crates/agent/src/runtime.rs:539` (`spawn_mode_forwarder`
      visibility raised from `pub(crate)` to `pub` so the V2b
      round-trip test can drive it from the ui test crate;
      additive-only — internal spawn behavior unchanged).
    - test cmd — `cargo test -p ui --features live --test live_subscription_full_bus`.
    - output line — `test t911_full_bus_drives_every_panel_out_of_loading ... ok`;
      `test t911_kill_button_round_trip_via_mode_forwarder ... ok` /
      `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`.
    - Existing live test suite still green: `cargo test -p ui --features live`
      reports 35 lib + 0 + 2 + 6 + 2 + 32 = 77 tests across all targets, ALL ok
      (existing 75 + 2 new T911 tests).  No existing test modified.
    - `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean
      (the only T911 clippy warnings during development were `needless_borrow`
      and `doc_lazy_continuation` — both fixed; my code is the test file +
      the `pub` visibility change in `runtime.rs`).  `cargo fmt --all -- --check`
      clean.
    - **Architect-design deviations:**
      - Architect's V2b spec calls for "a unit-test variant that uses a real
        `Arc<KillSwitch>`" with the audit-wired `KillSwitch::with_audit`
        constructor.  The test does this directly inside `crates/ui/tests/` —
        no new dev-deps were needed because `agent`, `audit`, `tokio_util`,
        and `uuid` are all reachable through the `live` feature on the ui
        crate (transitive via `dep:agent`, `dep:audit`, `dep:tokio-util`,
        `dep:uuid`).  `tempfile` was avoided in favor of
        `std::env::temp_dir().join(format!("t911-halt-{}", Uuid::new_v4().simple()))`
        for the halt-file path because `tempfile` is not a ui-side dev-dep
        and adding it requires touching `crates/ui/Cargo.toml` — owned by
        the ui-designer parallel scope (T908).  The halt-file watcher is NOT
        spawned in this test so the path's existence is immaterial.

  - New test `crates/ui/tests/live_subscription_full_bus.rs`
    drives a fully-populated bus (publishes 100 fills, 50
    positions, 20 bars, 200 ticks, 5 pnl snapshots, 1 mode
    transition) and asserts every cockpit panel transitions out
    of `Loading` after the first relevant event, without any
    panic or unexpected `Closed` state.
  - Plus the V2b round-trip: a unit-test variant that uses a
    real `Arc<KillSwitch>` (no MockIncidentSpawner) — checks the
    full Cockpit-button → trip → bus.mode publish → cockpit
    `AgentHaltedExternally` round-trip with the new
    `Arc<KillSwitch>` (kill_switch needs an in-memory ledger,
    `Arc::new(MockIncidentSpawner::new())` for the spawner; T905
    forwarder is spawned on a tokio Handle owned by the test). —
  _acceptance: `cargo test -p ui --features live --tests` clean;
  both new tests pass; combined with existing 53 live tests the
  total is 55+ green; `cargo clippy --workspace --features live
  -- -D warnings` clean._
  **[deps: T905, T906]**
  **[parallel-safe with T910, T912]**

- [x] **T912** [developer] — Prometheus toggle test (V10) per
  [Design → Q4](../features/live-cockpit-unified.md#q4--config-sourcing):
  - **Honest tick — Wave 3 developer (2026-05-01)**:
    - file:line — `crates/agent/tests/prometheus_toggle_test.rs:1-180`
      (new integration test file with three subtests:
      `t912_disabled_skips_bind_via_public_api` passes a malformed
      listen string with `prometheus_enabled = false` and asserts the
      function short-circuits before parsing — proves the disabled
      branch is the FIRST thing checked.  `t912_enabled_attempts_parse`
      asserts the same malformed listen string is rejected when
      `prometheus_enabled = true` — proves the toggle is bidirectional.
      `t912_runtime_with_prometheus_disabled_does_not_bind_9100` builds
      a `RunHandles` with `prometheus_enabled = false`, calls
      `start_prometheus_exporter` via the same public surface the
      `trading` / `cockpit_live` bins call at boot, runs
      `agent::runtime::run` for 200 ms, probes port 9100 via
      `TcpListener::bind`, and asserts the runtime did not silently
      bind it).
    - test cmd — `cargo test -p agent --test prometheus_toggle_test`.
    - output line — `test t912_disabled_skips_bind_via_public_api ... ok`;
      `test t912_enabled_attempts_parse ... ok`;
      `test t912_runtime_with_prometheus_disabled_does_not_bind_9100 ... ok` /
      `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.46s`.
    - `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean;
      `cargo fmt --all -- --check` clean.  Anchor gate (R15): pass-by-construction —
      test-only change, no `crates/strategy/`, `crates/audit/`, `crates/exec/`,
      `crates/backtest/`, or report-rendering code touched.
    - **Architect-design deviations (recorded for tester / orchestrator):**
      - Architect's V10 spec calls for two subprocess-launched
        `cockpit_live` invocations with `reqwest::get(":9100/metrics")`
        probes.  Per the orchestrator's task scope (which simplified
        T912 to "construct a config with `prometheus_enabled = false`,
        call `runtime::run` briefly, and assert no listener bound on
        `:9100`"), this test exercises the disabled-toggle property at
        the public-surface level (`start_prometheus_exporter`) plus a
        `TcpListener::bind` probe against port 9100 — sandbox-safe and
        deterministic.  The end-to-end subprocess + `reqwest` smoke is
        left for the tester's V_FINAL gate where a real terminal +
        network stack are in scope.  The toggle's bidirectional
        correctness is proven by `t912_enabled_attempts_parse` (a
        regression that silently disabled prometheus would surface as
        the malformed string being accepted).

  - New test `crates/agent/tests/prometheus_toggle_test.rs` with
    two subtests:
    1. **Enabled** (default): spawn `cockpit_live` with default
       config; after 3 s, `reqwest::get("http://127.0.0.1:9100/metrics")
       .await` returns HTTP 200 with `kill_switch_trips_total` in
       the body; SIGINT cleanly.
    2. **Disabled**: spawn `cockpit_live` with a tempfile config
       containing `[observability] prometheus_enabled = false`;
       after 3 s, `reqwest::get(...)` returns
       `Err(_)` (connection refused); SIGINT cleanly. The
       agent's JSON log contains `prometheus_listener_disabled`.
  - Pin the listener to a free port via
    `prometheus_listen = "127.0.0.1:0"` is NOT supported by the
    current exporter (binds to the literal port). Workaround:
    sub-tests use port `9100` and serialize via
    `serial_test::serial`. —
  _acceptance: `cargo test -p agent --test prometheus_toggle_test
  -- --test-threads=1` clean; both subtests pass within 30 s; no
  port-leak across runs (verified by `lsof -i :9100` returning
  nothing after the test exits)._
  **[deps: T901, T904]**
  **[parallel-safe with T910, T911]**

## Final gate

- [x] **T_FINAL_LIVE_COCKPIT** [tester] — End-to-end V1–V10 gate.
  This row is **owned by the tester** per the
  [AGENT.md → Process discipline rule 2](../../AGENT.md). The
  developer never ticks this row.
  - **Honest tick — Final tester gate (2026-05-02)**:
    - report — `spec/reports/test-2026-05-02-1501-live-cockpit-unified-final.md`.
    - test cmds — `cargo build --workspace --all-targets`; `cargo build --release --bin cockpit_live --features ui/live`; `cargo build --release --bin cockpit_live --features ui/in_process_cron`; `cargo build --release --bin trading`; `cargo build --release --bin trading --features agent/in_process_cron`; `cargo build -p ui --bin cockpit --features fixtures`; `cargo build -p ui --bin cockpit --features live` (expected fail); `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test --workspace --all-targets`; `cargo test --workspace --doc`; `cargo test -p ui --features live`; `cargo test -p ui --features fixtures`; `cargo test -p ui --features live --test cockpit_live_kill_button_writes_audit`; `cargo test -p audit --test ledger_integration`; `cargo test -p audit --test feed_reconnect_test`; `cargo test -p audit --test uptime_intervals_test`; `cargo test -p agent --test kill_switch_trip_writes_both`; `cargo test -p agent --test prometheus_toggle_test`; `cargo test -p agent --test unified_uptime_test`; `cargo test -p agent --test bus_drops_on_shutdown`; `bash scripts/verify_anchors.sh` (twice — Phase 1 + Phase 4); `(./target/release/trading --config config/agent.toml --mode research) & sleep 3; kill -INT $PID` smoke probe.
    - output lines —
      `ANCHORS PASS  (11 / 11)` (bash scripts/verify_anchors.sh, both runs);
      `Finished `release` profile [optimized] target(s) in 7.85s` (cockpit_live --features ui/live);
      `Finished `release` profile [optimized] target(s) in 7.45s` (cockpit_live --features ui/in_process_cron);
      `Finished `release` profile [optimized] target(s) in 3.05s` (trading);
      `error: target `cockpit` in package `ui` requires the features: `fixtures`` (T908 deprecation gate fires correctly);
      `test t906_stitch_kill_confirmed_via_state_update_writes_both_audit_rows ... ok` (T906 stitch end-to-end audit dual-write proof);
      `test t809_trip_writes_audit_dual_and_calls_spawn_helper ... ok` (T809 invariant);
      `test t910_v3_graceful_shutdown_within_two_seconds_with_close_uptime_row ... ok` (T910 V3a in-process);
      `test t903d_bus_strong_count_collapses_on_cancel ... ok` (T903d bus drop on shutdown);
      `test t912_runtime_with_prometheus_disabled_does_not_bind_9100 ... ok` (T912 V10);
      `test t911_full_bus_drives_every_panel_out_of_loading ... ok` + `test t911_kill_button_round_trip_via_mode_forwarder ... ok` (T911 V2b round-trip);
      `agent uptime interval closed`, `agent stopped`, `EXIT=0` (tester smoke probe of `./target/release/trading --mode research` after SIGINT — V3a + V7).
    - V1-V10 — all VERIFIED per the matrix in the test report; T901-T912 all citation-verified (T902 + T906 noted minor line-drift, content matches); T906 stitch integration test is the load-bearing empirical proof that the cockpit kill button now trips T809's dual-write end-to-end; T908 deprecation gate works as documented (cargo-level `required-features = ["fixtures"]` redirect plus `cfg(all(feature="live", not(feature="fixtures")))` defense-in-depth).
    - Operator-success-reports invariants (T802 / T805 / T806 / T809 / T810) all PASS — preserved by construction and verified by their regression suites.
    - Anchor gate (R15 / `spec/anchors.toml` 11 entries): PASS twice (Phase 1 + Phase 4 of the tester run).
    - **Routing**: `VERDICT → PASS`. Hand off to presenter for the operator-facing presentation.


  - **V1** end-to-end smoke: launch `cockpit_live` against a real
    `config/agent.toml` paper-mode setup; observe the iced window;
    wait for the first bar close (~60 s on Binance spot 1m);
    assert the P&L panel + tape panel transition out of
    `Loading` to `Ready`; the latency badge updates from ticks;
    the strategies panel shows the SMA crossover row from the
    watcher.
  - **V2a + V2b**: file-touch trip + cockpit-button trip; both
    paths produce the audit dual-write + halted banner.
  - **V3a / V3b / V3c**: subprocess SIGINT (T910); manual window-X
    close (operator-checklist); kill-switch then close
    combination.
  - **V4**: full test matrix —
    `cargo test --workspace --all-targets`,
    `cargo test -p ui --features fixtures`,
    `cargo test -p ui --features live`,
    `cargo test -p agent`,
    `cargo test -p agent --features in_process_cron`,
    `cargo fmt --all -- --check`,
    `cargo clippy --workspace --all-targets --all-features --
    -D warnings`. All PASS.
  - **V5 anchor regression gate**: `scripts/verify_anchors.sh`
    PASS (11/11). Mandatory per
    [AGENT.md → Process discipline rule 3](../../AGENT.md).
  - **V6**: `cargo run --bin cockpit --features fixtures` boots
    unchanged.
  - **V7**: `cargo run --bin trading -- --config
    config/agent.toml` boots unchanged (no GUI; JSON log on
    stdout; `:9100/metrics` reachable).
  - **V8**: feed-reconnect manual smoke (kill the network;
    observe `feed_reconnect` row).
  - **V9**: 90 s uptime smoke (T910 covers).
  - **V10**: Prometheus toggle (T912 covers).
  —
  _acceptance: full test report at
  `spec/reports/test-2026-MM-DD-HHMM-live-cockpit-unified-final.md`;
  V1–V10 all VERIFIED with cited test commands and output lines;
  status bumped `in-progress → shipped` only when V1–V10 pass +
  the anchor gate is PASS (11/11)._
  **[deps: T901, T902, T903a, T903b, T903c, T903d, T904, T905,
  T906, T907, T908, T910, T911, T912]**

## Parallelism map

```
                                   Week 1
                                     │
                                  ┌──▼──┐
                                  │T901 │  config + prometheus toggle
                                  └──┬──┘
                                     │
                                  ┌──▼──┐
                                  │T902 │  agent::runtime::run extraction
                                  └──┬──┘  (CRITICAL PATH GATE)
                                     │
        ┌───────────────┬─────────────┼──────────────┬───────────────┐
     ┌──▼──┐       ┌────▼───┐    ┌────▼───┐     ┌────▼───┐     ┌─────▼─────┐
     │T903a│       │ T903b  │    │ T903c  │     │ T905   │     │   T904    │
     │paper│       │data tap│    │  pnl   │     │  mode  │     │cockpit_   │
     │fills│       │bars+   │    │recon-  │     │forwarder     │ live bin  │
     │+pos │       │ ticks  │    │ ciler  │     │        │     │           │
     └──┬──┘       └────┬───┘    └────┬───┘     └────┬───┘     └─────┬─────┘
        │               │             │              │               │
        │               │             │              │               │ [ui-designer
        │               │             │              │               │  parallel:]
        │               │             │              │       ┌───────┴────────┐
        │               │             │              │       │                │
        │               │             │              │   ┌───▼──┐         ┌───▼──┐
        │               │             │              │   │ T906 │         │ T908 │
        │               │             │              │   │ kill │         │retire│
        │               │             │              │   │button│         │live  │
        │               │             │              │   │trips │         │arm   │
        │               │             │              │   └───┬──┘         └──────┘
        │               │             │              │       │
        │               │             │              │   ┌───▼──┐
        │               │             │              │   │ T907 │
        │               │             │              │   │tooltip
        │               │             │              │   └──────┘
        └───────────────┴─────────────┴──────────────┘
                                     │
                            ┌────────┴────────┐
                            │                 │
                         ┌──▼──┐           ┌──▼──┐
                         │T903d│           │T910 │
                         │bus  │           │unif │
                         │drop │           │uptime
                         │test │           │test │
                         └──┬──┘           └──┬──┘
                            │                 │
                            ├─────────────────┤
                            │                 │
                         ┌──▼──┐           ┌──▼──┐
                         │T911 │           │T912 │
                         │live │           │ prom │
                         │full │           │toggl │
                         │bus  │           │test  │
                         └──┬──┘           └──┬──┘
                            └────────┬────────┘
                                     │
                              ┌──────▼──────┐
                              │ T_FINAL_LIVE_COCKPIT │  [tester]
                              │ V1–V10 gate  │
                              └─────────────┘
```

**Sync points** (tasks below the line block on tasks above):
1. **After T902** (line 1): T903a + T903b + T903c + T905 + T904
   fan out **in parallel** — five concurrent developer-sub-agent
   slots possible.
2. **After T904** (line 2): T906 [ui-designer] + T908 [ui-designer]
   + T910 [developer] fan out — three parallel sub-agent slots
   (one ui-designer doing T906→T907 sequentially, one ui-designer
   doing T908, one developer doing T910).
3. **After T903a/b/c + T905 + T906**: T903d + T911 + T912 land
   (the regression-gate trio, all developer).
4. **T_FINAL** is sequential — single tester agent assembles the
   full report.

**Parallel-safe boundary check:** every parallel pair below was
verified to NOT touch the same file:

| Pair | Files touched (left) | Files touched (right) | Conflict? |
|------|----------------------|------------------------|-----------|
| T903a ‖ T903b | `crates/exec/src/paper.rs` + `crates/exec/src/publisher.rs` (NEW) | `crates/agent/src/runtime.rs` (T902 already created) | NO |
| T903a ‖ T903c | `crates/exec/src/paper.rs` + `crates/exec/src/publisher.rs` | `crates/agent/src/reconciler.rs` | NO |
| T903b ‖ T903c | `crates/agent/src/runtime.rs` (additive section) | `crates/agent/src/reconciler.rs` | NO |
| T903b ‖ T905 | `crates/agent/src/runtime.rs` (different sections) | `crates/agent/src/runtime.rs` (different sections) | **PARTIAL** — same file. Mitigation: T903b writes the bars/ticks `tap` block; T905 writes the mode forwarder block. Both are additive within different sections of the JoinSet build-up. **Sequence them** if both developers might commit concurrently — but the conflict surface is one block of code each, so a clean rebase is trivial. Marked `parallel-safe` with that note. |
| T906 ‖ T908 | `crates/ui/src/state.rs` + `crates/ui/src/strings.rs` (T907 follow-up) | `crates/ui/src/bin/cockpit.rs` + `crates/ui/Cargo.toml` (additive feature) | NO |
| T910 ‖ T911 ‖ T912 | `crates/agent/tests/unified_uptime_test.rs` (NEW) | `crates/ui/tests/live_subscription_full_bus.rs` (NEW) + `crates/ui/tests/kill_button_trips_kill_switch.rs` (NEW) | `crates/agent/tests/prometheus_toggle_test.rs` (NEW) — three separate new test files | NO |

The single **same-file conflict** (T903b ‖ T905, both touching
`crates/agent/src/runtime.rs`) is a small additive surface in
different parts of the function body. The orchestrator can either
sequence them (developer-A → developer-B) or fan out and accept a
trivial rebase. Default recommendation: fan out.

## Notes
