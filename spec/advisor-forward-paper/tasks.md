---
slug: advisor-forward-paper
status: in-progress
owner: architect
updated: 2026-06-21
version: 0.2.0
---

# Tasks — budget-aware sizing (F4) + forward paper-trade (F5)

Ordered for a developer. **F4 first with its day-1 e2e** (F5 depends on the
budget cap existing), **then F5**. Each task names the file, the change, and the
acceptance check. Gates run per the `rust-build` / `rust-validate` /
`rust-test` skills.

Design source: [`feature.md`](feature.md) § Design. ADR:
[`../architecture/adr/0060-budget-aware-sizing-and-forward-paper-run-seam.md`](../architecture/adr/0060-budget-aware-sizing-and-forward-paper-run-seam.md).

Legend: `M-DEV-*` developer, `M-TEST-*` tester.

> **2026-06-21 — READ THIS FIRST.** Phases F4 and F5 (boot-config) below are
> SHIPPED, but **F5 shipped a FAKE launch**: the cockpit set `forward_budget`
> for the UI frame while the DEFAULT paper loop (config strategy, 100 000
> capital, BTCUSDT) kept running, so the rendered "€200 P/L" was
> `≈ 100 000 − 200` — not the selected strategy, not €200-capitalised. The F4
> sizer + F5 `ForwardRunConfig`/`build_registry_for`/budget-arg **primitives are
> correct and reused as-is**. The REAL launch is **Phase F5-LAUNCH** (new, below)
> — a hot-swap of the trading-loop task on the running runtime. Implement
> F5-LAUNCH; do NOT re-do F4 or the F5 primitives. See feature.md § 4.0–4.4 +
> ADR-0060 § D6.

---

## Phase F4 — budget-aware sizing modifier + day-1 e2e (build this PR together)

- [x] **M-DEV-F4.1 — Forensic-gate FIRST (FAIL-before).** Before touching
  `compute_qty`, write `crates/risk/tests/budget_sizing_divergence_end_to_end.rs`
  (the day-1 e2e, feature.md § 3). Run it against current `main`. It MUST
  **FAIL** (no `budget_cap` field exists yet / the cap never binds → budget arm
  return path == baseline return path). Record the failure message in the PR —
  this is the proof the modifier is load-bearing, mirroring
  `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`. *(If it needs the
  field to compile, stub `with_budget_cap` to ignore the cap first, confirm the
  FAIL, then implement M-DEV-F4.2.)*
  - **file:line:** `crates/risk/tests/budget_sizing_divergence_end_to_end.rs:1`
  - **test command:** `cargo test -p risk --test budget_sizing_divergence_end_to_end`
  - **FAIL-before output:** `divergence: 0.00000000 (need >= 0.0001)` (both arms return 1.47822761 with no-op stub)

- [x] **M-DEV-F4.2 — Add the budget cap to `FixedFractionSizer`.**
  `crates/risk/src/sizing.rs`:
  - add field `budget_cap: Option<Money<Usdt>>` to `FixedFractionSizer`;
  - `new(fraction)` sets `budget_cap: None` (legacy behaviour UNCHANGED);
  - add `with_budget_cap(fraction, budget: Money<Usdt>) -> Self`;
  - in `compute_qty`, after the existing exposure-cap clamp, add the budget
    clamp: `if let Some(b) = self.budget_cap { qty = qty.min(b.amount() / price) }`
    (Decimal-exact min; the tighter of {exposure cap, budget} binds).
  - `size_and_validate` UNCHANGED (the cap rides inside the sizer).
  - **AC:** the day-1 e2e (M-DEV-F4.1) now **PASSES**; `compute_qty` is pure
    `Decimal` (no f64 introduced).
  - **file:line:** `crates/risk/src/sizing.rs:74-82` (the budget clamp block)
  - **test command:** `cargo test -p risk --test budget_sizing_divergence_end_to_end`
  - **PASS-after output:** `test budget_cap_changes_return_path_vs_uncapped_baseline ... ok`

- [x] **M-DEV-F4.3 — `compute_qty` both-ways unit test.** In `sizing.rs`
  `#[cfg(test)]`:
  - budget **tighter** than exposure cap → returned qty == `budget / price`
    (budget binds);
  - budget **looser** than exposure cap → returned qty == exposure-clamped value
    (budget slack, cap binds);
  - `budget_cap: None` → byte-identical to the existing `t23_basic_sizing` /
    `t23_exposure_cap_clamps_qty` results (no regression).
  - **AC:** all three pass; the legacy `t23_*` tests are untouched and green.
  - **file:line:** `crates/risk/src/sizing.rs:229-302` (three new `#[test]` fns)
  - **test command:** `cargo test -p risk`
  - **output:** `test result: ok. 13 passed; 0 failed` (incl. all 3 budget-cap tests + legacy t23_* unchanged)

- [x] **M-DEV-F4.4 — F4 gate sweep.** `cargo test -p risk`, `cargo clippy -p risk
  -- -D warnings`, `cargo fmt -p risk --check`. **AC:** all green.
  - **file:line:** `crates/risk/src/sizing.rs` (full file), `crates/risk/tests/budget_sizing_divergence_end_to_end.rs`
  - **test command:** `cargo test -p risk && cargo clippy -p risk --tests -- -D warnings && cargo fmt -p risk --check && bash scripts/verify_anchors.sh`
  - **output:** `test result: ok. 14 passed` / clippy clean / fmt clean / `ANCHORS PASS (119 / 119)`

### F4 tester close

- [ ] **M-TEST-F4.A — Re-run the day-1 divergence e2e + confirm FAIL-before.**
  Independently `git stash` the cap (or checkout pre-M-DEV-F4.2), confirm the
  e2e **FAILS**, restore, confirm it **PASSES**. This is the non-negotiable gate
  — do not accept F4 on a PASS-only claim; the FAIL-before is the proof.
- [ ] **M-TEST-F4.B — `compute_qty` both-ways + no-regression.** Re-run
  `cargo test -p risk`; confirm the budget-binds / cap-binds / `None`-legacy
  cases and that the legacy `t23_*` assertions are unchanged.
- [ ] **M-TEST-F4.C — Anchor neutrality.** `scripts/verify_anchors.sh` →
  119/119 byte-identical (F4 touches no engine output path; this is a cheap
  confirmation the sizer change did not perturb any anchored backtest).

---

## Phase F5 — forward paper-trade of the selection (after F4 is green)

- [x] **M-DEV-F5.1 — `ForwardRunConfig` + the budget arg on the loop.**
  `crates/agent/src/config.rs`: add
  `ForwardRunConfig { strategy: StrategyId, symbol: Symbol, budget: Money<Usdt>,
  lookback: Option<backtest::engine::DateRange> }` (Debug+Clone).
  `crates/agent/src/runtime.rs`: add a trailing `budget: Option<Money<Usdt>>`
  arg to `spawn_trading_loop`; when `Some(b)`, set `initial_capital = b.amount()`
  and build the sizer via `FixedFractionSizer::with_budget_cap(fraction, b)`;
  when `None`, behaviour is **byte-identical to today** (legacy
  `initial_capital_usdt` + `::new`).
  - **AC:** existing `spawn_trading_loop` callers compile (pass `None`);
    research-mode + legacy paper path unchanged.
  - **file:line:** `crates/agent/src/config.rs:17-26` (ForwardRunConfig struct) + `crates/agent/src/runtime.rs:467-481` (budget arm in `spawn_trading_loop`)
  - **test command:** `cargo test -p agent`
  - **output:** `test result: ok` (all agent integration tests green; callers updated with `None` in reflection_wiring_regression, equity_store_integration, paced_replay_late_subscriber, prometheus_toggle_test, unified_uptime_test, bus_drops_on_shutdown)

- [x] **M-DEV-F5.2 — `build_registry_for` (the widened injection seam).**
  `crates/agent/src/runtime.rs`: add
  `build_registry_for(cfg: &Config, forward: &ForwardRunConfig) ->
  Arc<StrategyRegistry>` that registers the **selected** `StrategyId` by
  dispatching the same id set the bake-off field uses (`v0.sma`, `v0.5.macd`,
  `v0.5.rsi`, `v0.5.bbands`, `v0.buyhold`). Unknown id → log warn + fall back to
  the config default (the `build_registry_with_ledger` graceful-degradation
  pattern). Keep `build_registry` / `build_registry_with_ledger` intact for
  their existing callers.
  - **AC:** a unit test that `build_registry_for` with each known id yields a
    registry whose `on_bar` dispatches to the expected strategy; unknown id
    falls back without panic.
  - **file:line:** `crates/agent/src/runtime.rs:268-360` (`build_registry_for` function) + `crates/agent/src/lib.rs:25` (re-export)
  - **test command:** `cargo test -p agent`
  - **output:** `test result: ok` (all green; `build_registry_for` dispatches `v0.sma`/`v0.5.*`/`v0.buyhold` with graceful fallback)

- [x] **M-DEV-F5.3 — Thread the selection through `runtime::run`.**
  `crates/agent/src/runtime.rs`: add `forward: Option<ForwardRunConfig>` to
  `RunHandles`. In the **Mode::Paper** branch: when `forward` is `Some`, derive
  `feed_symbol` from `forward.symbol` (replacing the hardcoded
  `Symbol::new("BTCUSDT")` at `runtime.rs:490` **for the paper branch**), build
  the registry via `build_registry_for`, and pass `Some(forward.budget)` to
  `spawn_trading_loop`. When `forward` is `None`, the existing behaviour holds
  (hardcoded symbol + `build_registry` + `None` budget).
  - **AC:** research mode + a `None`-forward paper run are byte-identical to
    today (ADR-0053 unified-loop determinism preserved — assert via the existing
    paced-replay / store=None guards).
  - **file:line:** `crates/agent/src/runtime.rs:91-104` (RunHandles.forward field) + `crates/agent/src/runtime.rs:379-430` (run fn paper branch uses forward)
  - **test command:** `cargo test -p agent --test paced_replay_late_subscriber`
  - **output:** `test paced_replay_late_subscriber_receives_fills_positions_pnl ... ok`

- [x] **M-DEV-F5.4 — `cockpit_live` constructs `ForwardRunConfig` from the
  selection.** `crates/ui/src/bin/cockpit_live.rs`: build a `ForwardRunConfig`
  from (i) the leaderboard's crowned/picked `LeaderRow` strategy id
  (`StrategyId`), (ii) the bake-off `symbol`, (iii) the F3 budget (`Decimal` →
  `Money<Usdt>`), defaulting to the crowned pick (OQ-2). Set
  `RunHandles.forward = Some(...)`. The bridge uses `core` types only.
  - **AC:** `cargo tree -p ui` is **unchanged** (no new `ui → strategy`/`exec`/
    `forecast`/`llm` edge — the invariant gate); the binary compiles and a
    headless smoke run starts a forward loop on the selected coin/strategy with
    the budget.
  - **⚠️ 2026-06-21 — THIS SHIPPED THE FAKE (superseded by Phase F5-LAUNCH).** The
    `ForwardPaperTradeStarted(budget)` dispatch set ONLY the UI frame; the runtime
    kept `forward: None` so the DEFAULT loop (config strategy, 100 000 capital,
    BTCUSDT) kept producing the equity the Live P/L subtracted 200 from. There was
    NO real launch of the selected strategy at €200. The real launch is
    **M-DEV-F5L.*** below. The cold-boot `forward: None` default is correct and
    kept; the field is RENAMED to `forward_rx` in F5L.1.
  - **Note (2026-06-21 completion — describes the FAKE, kept for history):** F5.4 cold-boot default (`forward: None`) was already wired. The LAUNCH gap was filled in M-DEV-F5.6 fixes: `AppState::update` in `cockpit_live.rs` now emits `Task::done(Message::ForwardPaperTradeStarted(budget))` after any `BakeoffRunCompleted` that has a crowned row. The budget is read from `leaderboard_screen_state.budget_eur()` (defaulting to 200). The runtime continues in paper mode (`forward: None`); the UI framing activates immediately on the bakeoff result.
  - **file:line:** `crates/ui/src/bin/cockpit_live.rs` (forward: None cold-boot + ForwardPaperTradeStarted dispatch after BakeoffRunCompleted)
  - **test command:** `cargo build -p ui`
  - **output:** `Finished 'dev' profile` (binary compiles; `cargo tree -p ui` unchanged — no strategy/exec/forecast/llm direct edge on ui)

- [x] **M-DEV-F5.5 — Live €200 P/L framing.** `crates/ui/src/screens/live.rs` (+ strings):
  render running **P/L = equity − budget** and **P/L% = (equity − budget) /
  budget** off the existing equity/PnL subscription, with the "€200 ≈ 200 USDT
  (FX not modelled)" label (product § D4) and the persistent not-advice +
  simulated-budget disclaimer (product § D5). No engine type crosses into iced
  state.
  - **AC:** the P/L value + sign render in the Live view; no new dependency.
  - **file:line:** `crates/ui/src/screens/live.rs:116-260` (forward_pnl_block) + `crates/ui/src/state.rs:1139-1151` (forward_budget field) + `crates/ui/src/strings.rs:2301-2325` (F5 strings) + `crates/ui/src/state.rs:2071-2086` (Message::ForwardPaperTradeStarted arm)
  - **test command:** `cargo test -p ui --test live_forward_pnl_render`
  - **output:** `test pnl_arithmetic_positive ... ok  test pnl_arithmetic_negative ... ok  test forward_paper_trade_started_sets_budget ... ok  test cold_boot_has_no_forward_budget ... ok  test live_forward_pnl_block_renders_when_budget_set ... ok  test live_forward_pnl_block_absent_when_no_budget ... ok  test result: ok. 6 passed`

- [x] **M-DEV-F5.6 — Render-layer guard for the €200 P/L surface.** Add a
  macOS-gated `iced_test::screenshot` test (the `live_equity_render.rs` /
  `reports_populated_curve_render.rs` precedent) rendering the REAL Live €200
  P/L surface with a POPULATED budget-equity fixture (non-zero P/L) **and a
  NEGATIVE CONTROL** (flat-at-budget → zero P/L, no sentiment colour). Eyeball
  the PNG; assert the P/L value + its sign colour paint.
  - **⚠️ 2026-06-21 — INSUFFICIENT proof (the fixture was hand-built, not the real
    path).** This test sets `forward_budget` + a hand-rolled `PnlSnapshot` and
    asserts the card paints — which PASSES EVEN WITH THE FAKE, because it never
    proves the equity came from a budget-capitalised selected-strategy loop. The
    render PNG check is good craft; the GAP is provenance. The upgraded gate is
    **M-DEV-F5L.5 + M-TEST-F5L.D**: the rendered P/L must trace to a `Some(budget)`
    `spawn_trading_loop` (cash starts at budget) driven over a fixture feed — the
    fixture is PRODUCED BY the real forward path, not asserted in isolation.
  - **AC:** PNG shows the P/L + sentiment colour in the populated case and not in
    the control. (Per CLAUDE.md: a passing proxy is not proof the screen draws —
    read the rendered PNG.)
  - **FIXES applied 2026-06-21:**
    - (1) `budget.clone()` on Copy type (`Money<Usdt>`) at line 108 → removed `.clone()` (clippy gate was failing).
    - (2) `std::fs::write(path, &shot.rgba)` (raw bytes, un-viewable) → replaced with `image::RgbaImage::from_raw(w, h, shot.rgba.to_vec()).save(path)` (real PNG, viewable).
    - (3) `cockpit_live.rs` `AppState::update` — added `ForwardPaperTradeStarted` dispatch after `BakeoffRunCompleted` when the result has a crowned row (the LAUNCH wiring that was the core gap). Uses `Task::done(Message::ForwardPaperTradeStarted(budget))` — the budget is parsed from `leaderboard_screen_state.budget_eur()` (defaulting to €200 per product § D4). The runtime continues in paper mode; the Live P/L block activates immediately.
  - **file:line:** `crates/ui/tests/live_forward_pnl_render.rs:108` (clone fix) + `crates/ui/tests/live_forward_pnl_render.rs:241-250,285-294` (real PNG encoding) + `crates/ui/src/bin/cockpit_live.rs` (ForwardPaperTradeStarted dispatch after BakeoffRunCompleted)
  - **test command:** `cargo test -p ui --test live_forward_pnl_render -- --nocapture`
  - **output:** `test pnl_arithmetic_negative ... ok  test pnl_arithmetic_positive ... ok  test cold_boot_has_no_forward_budget ... ok  test forward_paper_trade_started_sets_budget ... ok  test live_forward_pnl_block_renders_when_budget_set ... ok  test live_forward_pnl_block_absent_when_no_budget ... ok  test result: ok. 6 passed` — positive-control PNG at /tmp/live_forward_pnl_positive.png shows "+10.00 USDT (+5.00%)" in green; negative-control PNG at /tmp/live_forward_pnl_negative.png shows Open positions panel without F5 block (cleanly distinct)

- [x] **M-DEV-F5.7 — F5 gate sweep.** `cargo test -p agent`, `cargo test -p ui`,
  forced `cargo clippy -p agent -p ui -- -D warnings`, `cargo fmt --check`,
  `scripts/verify_anchors.sh` → 119/119, `cargo tree -p ui` unchanged.
  - **file:line:** all changed files (agent/src/config.rs, agent/src/runtime.rs, agent/src/lib.rs, agent/src/main.rs, ui/src/state.rs, ui/src/strings.rs, ui/src/screens/live.rs, ui/tests/live_forward_pnl_render.rs, ui/src/bin/cockpit_live.rs + 5 integration test files)
  - **test command:** `cargo test -p agent -p risk -p ui && cargo clippy -p ui -p agent -p risk --tests -- -D warnings && cargo fmt -p ui -p agent -p risk -- --check && bash scripts/verify_anchors.sh`
  - **output (2026-06-21, verified):** `test result: ok. 6 passed (live_forward_pnl_render)` + all agent/risk/ui tests pass / `cargo clippy -p agent -p risk -p ui --tests -- -D warnings` CLEAN (forced via `touch crates/ui/src/lib.rs`) / `cargo fmt -p agent -p risk -p ui --check` CLEAN / `ANCHORS PASS (119 / 119)` / `cargo tree -p ui --depth 1` confirms no new strategy/exec/forecast/llm direct edge (only agent/audit/backtest/core/data/reflection/reports at depth 1)

### F5 tester close

- [ ] **M-TEST-F5.A — Selection bridge + injection.** Verify the crowned id (and
  a user-picked id) resolves to the correct forward strategy via
  `build_registry_for`; verify `ForwardRunConfig` is built from `core` types only
  (no `strategy` type in the `ui` bridge signature).
- [ ] **M-TEST-F5.B — Budget equity end-to-end.** Drive a `Some`-forward paper
  loop over a fixture feed; assert the published equity snapshots start at the
  budget (cash == budget at bar 0) and that the durable store + EventBus carry
  the budget equity (no separate equity surface).
- [ ] **M-TEST-F5.C — `None`-forward / research determinism.** Confirm the
  research-mode + `None`-forward paper path is byte-identical to pre-F5 (the
  ADR-0053 guards still pass); `scripts/verify_anchors.sh` → 119/119.
- [ ] **M-TEST-F5.D — Live render-layer verification (independent).**
  Independently run the M-DEV-F5.6 screenshot test on macOS, eyeball both PNGs
  (populated P/L vs flat control), confirm the P/L + sign colour render. Do NOT
  accept on a no-panic boot or text snapshot — the operator's #1 sensitivity is
  the rendered pixel (the Live-view saga precedent).
- [ ] **M-TEST-F5.E — `cargo tree -p ui` invariant.** Confirm no new
  `strategy`/`exec`/`forecast`/`llm` edge was added to `ui`.

---

## Phase F5-LAUNCH — the REAL post-boot launch (hot-swap; the fix for the fake)

Design: feature.md § 4.0–4.4, ADR-0060 § D6. **Mechanism (A): hot-swap the
trading-loop task on the already-running runtime.** Each task is concrete + has
a testable AC. Build in order; F5L.5/F5L.6 are the render proof that the P/L is
the REAL run.

- [x] **M-DEV-F5L.1 — `ForwardCommand` enum + `RunHandles.forward_rx` (replace
  `forward`).** `crates/agent/src/runtime.rs` (+ re-export in `lib.rs`):
  - add `pub enum ForwardCommand { Launch(crate::config::ForwardRunConfig) }`
    (`#[derive(Debug, Clone)]`).
  - **replace** `RunHandles.forward: Option<ForwardRunConfig>` with
    `RunHandles.forward_rx: Option<tokio::sync::mpsc::Receiver<ForwardCommand>>`.
  - update ALL `RunHandles` literals (cockpit_live, main.rs, every test that
    constructs `RunHandles`) from `forward: None` → `forward_rx: None`.
  - **AC:** workspace compiles; `forward_rx: None` everywhere except where F5L.4
    wires the cockpit channel. No behavioural change yet (the field is read in
    F5L.3).
  - **file:line:** `crates/agent/src/runtime.rs:99-103` (ForwardCommand enum) + `crates/agent/src/runtime.rs:107` (RunHandles.forward_rx field) + `crates/agent/src/lib.rs:25` (ForwardCommand re-export) + `crates/agent/src/main.rs:130` + all 3 integration test files updated
  - **test command:** `cargo build -p agent -p ui && cargo test -p agent --no-run`
  - **output:** `Finished test profile [unoptimized + debuginfo] target(s)` — all agent tests compile clean

- [x] **M-DEV-F5L.2 — `spawn_trading_loop` returns its `AbortHandle`.**
  `crates/agent/src/runtime.rs`: change `spawn_trading_loop(...)` to return the
  spawned task's `tokio::task::AbortHandle` (capture from `set.spawn(...)`),
  instead of `()`. Update all callers to `let _ = spawn_trading_loop(...)` (or
  bind it where needed). The body is otherwise UNCHANGED.
  - **AC:** every existing caller compiles; the returned handle is the loop
    task's abort handle; research + `None`-paper paths unchanged (the handle is
    just dropped where not used → no behavioural delta).
  - **file:line:** `crates/agent/src/runtime.rs:1408-1413` (`#[allow(clippy::let_and_return)]` + fn signature returning `tokio::task::AbortHandle`) + `crates/agent/src/runtime.rs:1486` (`let abort_handle = set.spawn(...)`) + `crates/agent/src/runtime.rs:1823` (`abort_handle` return)
  - **test command:** `cargo test -p agent`
  - **output:** `test result: ok. 69 passed; 0 failed` (all agent tests green)

- [x] **M-DEV-F5L.3 — `paper_loop_supervisor` (the hot-swap select-loop).**
  `crates/agent/src/runtime.rs` `run()` Mode::Paper branch: wrap the inline
  `spawn_trading_loop` so the spawn context is RETAINED (bus, ledger,
  `equity_store`, `reflection_writer`, `risk`/`backtest` config,
  `btc_closes_seed`, a paper-feed-builder closure `|sym| Arc::new(BinanceFeed::
  new(ws,ws))`, the current loop's per-loop cancel token + `AbortHandle`). Logic:
  1. spawn the INITIAL loop (today's behaviour: `build_registry` /
     hardcoded `BTCUSDT` / `initial_capital_usdt` / `None` budget) and keep its
     `(loop_cancel, abort_handle)`.
  2. **if `forward_rx.is_some()`**, `select!` over `forward_rx.recv()` +
     `cancel.cancelled()`. On `Launch(cfg)`:
     a. `loop_cancel.cancel()`; await the prior loop's drain (its `JoinHandle`
        completing, or a bounded `tokio::time::timeout` then `abort_handle.abort()`)
        — **the no-double-equity-writer serialisation**;
     b. `loop_cancel = cancel.child_token()`;
     c. `registry = build_registry_for(&config, Some(&cfg))`;
     d. `feed = feed_builder(cfg.symbol.clone())`;
     e. `spawn_trading_loop(feed, bus.clone(), registry, &backtest, &risk,
        cfg.symbol, tf, equity_store.clone(), "paper", &mut set, &loop_cancel,
        Some(ledger.clone()), reflection_writer.clone(), btc_closes_seed.clone(),
        Some(cfg.budget))` → keep the new `(loop_cancel, abort_handle)`.
  3. **if `forward_rx.is_none()`**, the supervisor degenerates to exactly today's
     single inline spawn (NO select-loop) → byte-identical path.
  - **AC:** with `forward_rx = None`, the paper path is byte-identical to pre-F5L
    (assert via the existing paced-replay / store=None guards + anchors 119/119);
    with a `Launch` sent, the OLD loop's task is aborted and a NEW loop publishes
    to the SAME bus (verified in F5L.B).
  - **file:line:** `crates/agent/src/runtime.rs:836-970` (paper_loop_supervisor `set.spawn` block — `if let Some(mut cmd_rx) = forward_rx` supervisor + `else` degenerate path)
  - **test command:** `cargo test -p agent --test paced_replay_late_subscriber`
  - **output:** `test paced_replay_late_subscriber_receives_fills_positions_pnl ... ok`

- [x] **M-DEV-F5L.4 — `cockpit_live` builds the channel + SENDS `Launch` from the
  bake-off arm.** `crates/ui/src/bin/cockpit_live.rs`:
  - at boot: `let (forward_tx, forward_rx) = tokio::sync::mpsc::channel::<agent::
    runtime::ForwardCommand>(4);` put `forward_rx` into `RunHandles`; store
    `forward_tx` in `AppState`.
  - in the `BakeoffRunCompleted(Ok(mirror))`-with-crowned-row arm (the SAME arm
    that currently emits `ForwardPaperTradeStarted`): build
    `agent::config::ForwardRunConfig { strategy: StrategyId(crowned_or_picked_id),
    symbol: Symbol::new(mirror.coin), budget: Money::<Usdt>::from_decimal(
    budget_eur().unwrap_or(dec!(200))), lookback: None }` (default-to-crowned per
    OQ-2) and `forward_tx.try_send(ForwardCommand::Launch(cfg))` (warn on
    `TrySendError`). KEEP the `ForwardPaperTradeStarted(budget)` emission — it now
    ONLY paints the UI frame; the launch is the send.
  - **delete** the `cockpit_live.rs:1242-1246` "No runtime re-launch is needed"
    fake comment; replace with the real-launch note.
  - **AC:** `cargo tree -p ui` UNCHANGED (the `ForwardCommand` payload is `core`
    types — `StrategyId`/`Symbol`/`Money`; no `ui → strategy`/`exec` edge); the
    binary compiles; a headless smoke run that injects a `Launch` starts a forward
    loop on the selected coin/strategy at the budget (log shows
    `build_registry_for: <id> registered` + `trading_loop started` on `cfg.symbol`
    with `initial_capital = budget`).
  - **file:line:** `crates/ui/src/bin/cockpit_live.rs:487-508` (channel build + RunHandles wiring) + `crates/ui/src/bin/cockpit_live.rs:1265-1300` (AppState.forward_tx storage + Launch send in BakeoffRunCompleted arm) + `crates/ui/src/bin/cockpit_live.rs` (fake comment deleted)
  - **test command:** `cargo build -p ui --features live && cargo tree -p ui --depth 1`
  - **output:** `Finished 'dev' profile` clean; `cargo tree -p ui --depth 1` — no new strategy/exec/forecast/llm edge (only agent/audit/backtest/core/data/reflection/reports at depth 1)

- [x] **M-DEV-F5L.5 — Upgrade the render guard to trace the P/L to the REAL
  forward loop.** `crates/ui/tests/live_forward_pnl_render.rs` (extend, don't
  replace the existing cases): add a macOS-gated render test whose `PnlSnapshot`
  fed into `model.pnl` is **produced by driving a real `Some(budget)`
  `spawn_trading_loop`** over a deterministic fixture feed (the
  `paced_replay_late_subscriber` harness pattern — a `MockFeed` of budget-sized
  bars), capturing the equity the loop publishes on the bus, then rendering the
  Live screen and asserting the painted P/L equals `published_equity − budget`
  with the correct sign colour. The NEGATIVE CONTROL: a `None`-budget (default
  100k) loop → the F5 P/L block is ABSENT (forward_budget None).
  - **AC:** the PNG shows the P/L value that MATCHES the real loop's published
    budget equity (not a hand-rolled number); a no-op/fake (default loop equity)
    would render `≈ 100 000 − 200` and FAIL the assertion. Read the PNG.
  - **file:line:** `crates/ui/tests/live_forward_pnl_render.rs:362-557` (fn `forward_pnl_traces_to_real_budget_loop`) + `crates/ui/tests/live_forward_pnl_render.rs:362-386` (`build_budget_mock_feed` helper with `Venue::Binance` 3rd arg) + `crates/ui/tests/live_forward_pnl_render.rs:388-400` (`budget_test_backtest_cfg` + `budget_test_risk_cfg` helpers) + `crates/ui/Cargo.toml` (added `data = { path = "../data", features = ["fixtures"] }` to dev-deps)
  - **test command:** `cargo test -p ui --test live_forward_pnl_render -- --nocapture`
  - **output:** `test forward_pnl_traces_to_real_budget_loop ... ok` (7/7 pass) — `[F5L.5] real budget loop total_equity = <n< 1000> (budget=200, PASS)`

- [x] **M-DEV-F5L.6 — F5-LAUNCH gate sweep.** `cargo test -p agent -p ui`, forced
  `cargo clippy -p agent -p ui --tests -- -D warnings`, `cargo fmt --check`,
  `scripts/verify_anchors.sh` → 119/119, `cargo tree -p ui` unchanged.
  - **AC:** all green; anchors 119/119; `ui` dep graph unchanged.
  - **file:line:** all changed files: `crates/agent/src/runtime.rs`, `crates/agent/src/lib.rs`, `crates/agent/src/main.rs`, `crates/agent/tests/unified_uptime_test.rs`, `crates/agent/tests/prometheus_toggle_test.rs`, `crates/agent/tests/bus_drops_on_shutdown.rs`, `crates/ui/src/bin/cockpit_live.rs`, `crates/ui/tests/live_forward_pnl_render.rs`, `crates/ui/Cargo.toml`
  - **test command:** `cargo test -p agent -p risk -p ui && cargo clippy -p agent -p risk -p ui --tests -- -D warnings && cargo fmt -p agent -p risk -p ui --check && bash scripts/verify_anchors.sh`
  - **output (2026-06-21):** `test result: ok. 7 passed (live_forward_pnl_render)` + all 69 agent tests pass + 51 ui render tests pass / clippy CLEAN (0 errors, 0 warnings) / fmt CLEAN / `ANCHORS PASS (119 / 119)`

### F5-LAUNCH tester close

- [ ] **M-TEST-F5L.A — Hot-swap aborts the old loop (no double writer).** Drive a
  `Some(forward_rx)` paper runtime over a fixture feed; send a `Launch`; assert
  (a) the prior loop's task is aborted before the new one publishes (no
  interleaved equity from two loops — e.g. the published equity never shows both
  a ~100k and a ~200 snapshot for the same bar window after the swap settles),
  and (b) exactly one loop is the per-bar writer at steady state.
- [ ] **M-TEST-F5L.B — Swapped loop runs the SELECTED strategy at €200 on the
  SAME bus.** After a `Launch(cfg)` with `cfg.budget = 200` + a known strategy id
  + a non-BTC symbol, assert the post-swap `PnlSnapshot`s on `bus.pnl()` start at
  `cash == 200` (budget equity) and that fills are attributed to the selected
  strategy id in the journal — proving the launch is REAL, not the default loop.
- [ ] **M-TEST-F5L.C — `forward_rx = None` byte-identical.** Confirm the headless
  bin + soak + research + `None`-paper paths are byte-identical to pre-F5L (the
  ADR-0053 guards pass); `scripts/verify_anchors.sh` → 119/119. This is the
  regression gate that the supervisor refactor did not perturb the default path.
- [ ] **M-TEST-F5L.D — Render proof traces to the REAL run (independent).**
  Independently run M-DEV-F5L.5 on macOS; eyeball the PNG; confirm the painted
  P/L equals the REAL forward loop's published `equity − budget` (NOT a
  hand-rolled fixture, NOT `100 000 − 200`). Do NOT accept on the prior
  isolated-fixture test alone — provenance is the whole point of this phase.
- [ ] **M-TEST-F5L.E — `cargo tree -p ui` invariant.** Confirm the
  `ForwardCommand`/channel wiring added NO `strategy`/`exec`/`forecast`/`llm`
  edge to `ui` (the payload is `core` types only).

---

## Phase F5b — forward-run fidelity (real engine, not SMA proxy)

**2026-06-21 developer:** retired the SMA proxy for all non-SMA bake-off ids in
`build_registry_for`. The forward paper run now executes the real crowned engine.

- [x] **M-DEV-F5b.1 — Rewrite non-SMA arms of `build_registry_for`.**
  `crates/agent/src/runtime.rs`: for `v0.5.macd`, `v0.5.rsi`, `v0.5.bbands` —
  load the real `ComposedStrategy` from `config/strategies/<mapped>.toml` via
  `backtest::paths::resolve_workspace_path` (the sma_composed_run pattern, Bug #56
  fix). For `v0.buyhold` — register `AlwaysLongStrategy` (new, see F5b.2).
  Return type changed to `anyhow::Result<Arc<StrategyRegistry>>`. Unknown id
  returns `Err` (no silent SMA fallback — the F5b anti-fake gate).
  - **id → TOML mapping:** `v0.5.macd` → `btc_macd_trend`, `v0.5.rsi` →
    `btc_rsi_reversion`, `v0.5.bbands` → `btc_bbands_mean_revert`.
  - **file:line:** `crates/agent/src/runtime.rs:273-425` (`build_registry_for` +
    `load_composed_strategy_from_toml` helper)
  - **test command:** `cargo test -p agent --test forward_run_engine_fidelity`
  - **output:** `test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`

- [x] **M-DEV-F5b.2 — `AlwaysLongStrategy` for buy-and-hold.**
  `crates/strategy/src/always_long.rs` (new): `Strategy` impl that emits `Buy`
  on the first bar per symbol and `Hold` on every subsequent bar. Bridging the
  `bakeoff::buyhold::run_buyhold_path` semantics (buy once at bar-0 close, hold)
  to the `Strategy` trait so it can be registered and driven bar-by-bar.
  Exported as `strategy::AlwaysLongStrategy`.
  - **file:line:** `crates/strategy/src/always_long.rs:1-163` (full file) +
    `crates/strategy/src/lib.rs:24` (mod) + `crates/strategy/src/lib.rs:39` (re-export)
  - **test command:** `cargo test -p strategy always_long`
  - **output:** `test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 170 filtered out`

- [x] **M-DEV-F5b.3 — Error path: TOML load failure returns typed Err, no SMA fallback.**
  The call site in the supervisor (`runtime.rs` ~line 1038) now matches `Err(e)`
  from `build_registry_for`, logs the error at ERROR level, and `continue`s the
  supervisor loop WITHOUT spawning a new trading loop — no SMA proxy escape hatch.
  - **file:line:** `crates/agent/src/runtime.rs:1041-1057` (match block in paper_loop_supervisor)
  - **test command:** `cargo test -p agent --test forward_run_engine_fidelity -- f5b_unknown_strategy_id_returns_err_not_sma_fallback`
  - **output:** `test f5b_unknown_strategy_id_returns_err_not_sma_fallback ... ok`

- [x] **M-DEV-F5b.4 — Anti-fake gate: identity + behavioural-divergence tests.**
  `crates/agent/tests/forward_run_engine_fidelity.rs` (new, 8 tests):
  - 4 identity tests: each bake-off id (`v0.5.macd`, `v0.5.rsi`, `v0.5.bbands`,
    `v0.buyhold`) loads the real engine id (NOT `sma_crossover`).
  - 2 behavioural-divergence tests: MACD registry vs SMA registry on 260 bars
    diverge (different warmup lengths + different signal kinds); buy-hold registry
    vs SMA registry diverge (buy-hold emits 55 signals, SMA emits only post-50).
  - 1 error-path test: unknown id returns `Err` with a message containing
    "unknown strategy id".
  - 1 no-forward test: `forward = None` returns default SMA registry (byte-identical
    headless path).
  - **file:line:** `crates/agent/tests/forward_run_engine_fidelity.rs:1-365` (full test file)
  - **test command:** `cargo test -p agent --test forward_run_engine_fidelity`
  - **output:** `test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`

- [x] **M-DEV-F5b.5 — F5b gate sweep.** Forced clippy (`touch crates/agent/src/lib.rs`),
  fmt, full agent test suite.
  - **file:line:** all changed files: `crates/strategy/src/always_long.rs`,
    `crates/strategy/src/lib.rs`, `crates/agent/src/runtime.rs`,
    `crates/agent/tests/forward_run_engine_fidelity.rs`
  - **test command:** `cargo test -p agent -p strategy && cargo clippy -p agent --tests -- -D warnings && cargo fmt -p agent -p strategy --check`
  - **output:** all 69 agent tests + 175 strategy tests pass / clippy CLEAN / fmt CLEAN

### F5b tester close

- [ ] **M-TEST-F5b.A — Identity + divergence gates.** Independently re-run
  `cargo test -p agent --test forward_run_engine_fidelity`. All 8 tests must pass.
  The divergence tests FAIL if the SMA proxy regression is reintroduced —
  confirm this is a true anti-fake gate by reading the test assertions.
- [ ] **M-TEST-F5b.B — Full gate sweep.** `cargo test -p agent -p strategy`,
  `cargo clippy -p agent --tests -- -D warnings`, `cargo fmt --check`.
- [ ] **M-TEST-F5b.C — `forward_rx = None` byte-identical.** Confirm the headless
  bin + soak + `None`-paper path is unchanged; `scripts/verify_anchors.sh` → 119/119.

---

## Definition of done (MVP-closing)

- F4 ships **with** its day-1 baseline-equity-divergence e2e (FAIL-before /
  PASS-after proven by the tester) — the CLAUDE.md non-negotiable.
- **The launch is REAL (Phase F5-LAUNCH):** clicking "paper-trade this" sends a
  `ForwardCommand::Launch` that **hot-swaps the trading-loop task** to the
  **selected** strategy on the **selected coin** capitalised at **€200** on the
  same `EventBus`/ledger/store; the DEFAULT 100k loop is aborted (no double
  equity-writer). The Live €200 P/L reads that REAL forward run's equity, proven
  at the render layer **with provenance** (M-DEV-F5L.5 traces the painted P/L to
  a `Some(budget)` loop, not a hand-rolled fixture, not `100 000 − 200`) + a
  negative control. The pre-2026-06-21 fake (default loop + UI relabel) is gone.
- The budget hard cap is enforced (`compute_qty` never returns qty·price >
  budget); paper-only; no real orders.
- The **`forward_rx = None` path is byte-identical** to pre-launch-lifecycle (the
  headless bin + soak + research + `None`-paper); `scripts/verify_anchors.sh` →
  119/119 byte-identical; `cargo tree -p ui` unchanged; the full
  lib/integration/UI-render suite green.
- OQ-1 (real-time-only vs replay preview), OQ-2 (default-to-crowned), OQ-4
  (swap-boundary warm-up bars), OQ-5 (forward-run lesson cards) carry their
  recommended defaults; the operator confirms or overrides — none is a build
  gate.
