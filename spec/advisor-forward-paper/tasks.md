---
slug: advisor-forward-paper
status: draft
owner: architect
updated: 2026-06-20
version: 0.1.0
---

# Tasks — budget-aware sizing (F4) + forward paper-trade (F5)

Ordered for a developer. **F4 first with its day-1 e2e** (F5 depends on the
budget cap existing), **then F5**. Each task names the file, the change, and the
acceptance check. Gates run per the `rust-build` / `rust-validate` /
`rust-test` skills.

Design source: [`feature.md`](feature.md) § Design. ADR:
[`../architecture/adr/0060-budget-aware-sizing-and-forward-paper-run-seam.md`](../architecture/adr/0060-budget-aware-sizing-and-forward-paper-run-seam.md).

Legend: `M-DEV-*` developer, `M-TEST-*` tester.

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

- [ ] **M-DEV-F5.1 — `ForwardRunConfig` + the budget arg on the loop.**
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

- [ ] **M-DEV-F5.2 — `build_registry_for` (the widened injection seam).**
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

- [ ] **M-DEV-F5.3 — Thread the selection through `runtime::run`.**
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

- [ ] **M-DEV-F5.4 — `cockpit_live` constructs `ForwardRunConfig` from the
  selection.** `crates/ui/src/bin/cockpit_live.rs`: build a `ForwardRunConfig`
  from (i) the leaderboard's crowned/picked `LeaderRow` strategy id
  (`StrategyId`), (ii) the bake-off `symbol`, (iii) the F3 budget (`Decimal` →
  `Money<Usdt>`), defaulting to the crowned pick (OQ-2). Set
  `RunHandles.forward = Some(...)`. The bridge uses `core` types only.
  - **AC:** `cargo tree -p ui` is **unchanged** (no new `ui → strategy`/`exec`/
    `forecast`/`llm` edge — the invariant gate); the binary compiles and a
    headless smoke run starts a forward loop on the selected coin/strategy with
    the budget.

- [ ] **M-DEV-F5.5 — Live €200 P/L framing.** `crates/ui/src/live.rs` (+ strings):
  render running **P/L = equity − budget** and **P/L% = (equity − budget) /
  budget** off the existing equity/PnL subscription, with the "€200 ≈ 200 USDT
  (FX not modelled)" label (product § D4) and the persistent not-advice +
  simulated-budget disclaimer (product § D5). No engine type crosses into iced
  state.
  - **AC:** the P/L value + sign render in the Live view; no new dependency.

- [ ] **M-DEV-F5.6 — Render-layer guard for the €200 P/L surface.** Add a
  macOS-gated `iced_test::screenshot` test (the `live_equity_render.rs` /
  `reports_populated_curve_render.rs` precedent) rendering the REAL Live €200
  P/L surface with a POPULATED budget-equity fixture (non-zero P/L) **and a
  NEGATIVE CONTROL** (flat-at-budget → zero P/L, no sentiment colour). Eyeball
  the PNG; assert the P/L value + its sign colour paint.
  - **AC:** PNG shows the P/L + sentiment colour in the populated case and not in
    the control. (Per CLAUDE.md: a passing proxy is not proof the screen draws —
    read the rendered PNG.)

- [ ] **M-DEV-F5.7 — F5 gate sweep.** `cargo test -p agent`, `cargo test -p ui`,
  forced `cargo clippy -p agent -p ui -- -D warnings`, `cargo fmt --check`,
  `scripts/verify_anchors.sh` → 119/119, `cargo tree -p ui` unchanged.

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

## Definition of done (MVP-closing)

- F4 ships **with** its day-1 baseline-equity-divergence e2e (FAIL-before /
  PASS-after proven by the tester) — the CLAUDE.md non-negotiable.
- A forward paper run of the **crowned (default) or user-picked** strategy runs
  real-time on the selected coin with €200 budget sizing; the Live view shows
  running €200 P/L, verified at the render layer with a negative control.
- The budget hard cap is enforced (`compute_qty` never returns qty·price >
  budget); paper-only; no real orders.
- `scripts/verify_anchors.sh` → 119/119 byte-identical; `cargo tree -p ui`
  unchanged; the full lib/integration/UI-render suite green.
- OQ-1 (real-time-only vs replay preview) and OQ-2 (default-to-crowned) carry
  their recommended defaults; the operator confirms or overrides — neither is a
  build gate.
