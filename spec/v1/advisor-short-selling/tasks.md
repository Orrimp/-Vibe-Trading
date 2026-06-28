---
slug: advisor-short-selling
status: shipped
owner: tester
updated: 2026-06-23
---

# Tasks — advisor-short-selling

> **Architect M-T1 lock landed 2026-06-23.** Design + **ADR-0068** are written and
> registered. This is the real ordered build, split **developer ‖ ui-designer**.
> Binding decision record: [ADR-0068](../../architecture/adr/0068-single-coin-directional-short-selling.md).
> Design: [`feature.md` § Design](feature.md). Trace `REQ-ADVISOR-SHORT-SELLING-001`.

## Load-bearing constraints (carry into EVERY task — non-negotiable)

- **PAPER / SIM ONLY.** The €200 is simulated; shorts are simulated short positions.
  **NO live trading, NO real orders, NO real margin** (standing operator constraint).
- **Port-and-adapt, do NOT invent.** The proven short-side model (open / cover /
  maintenance-margin liquidation with honest cash-can-go-negative / per-bar funding)
  already exists, tested + shipped, in `crates/backtest/src/scenarios/montecarlo.rs::run_path`
  (the MN feature). Reuse it. The equity formula `cash + qty·mark`
  (`cli_types.rs:592-594`, `runtime.rs:1802`) is **already short-correct** — the bug
  is the **four** long-only clamps (the brief's three + `runtime.rs:1809-1813`/`:1884-1885`).
- **Gate / bands / benchmark FROZEN.** Do NOT touch `classify_verdict` /
  `compute_robustness_flag` / `verdict_bands` / `bootstrap.rs` / `rank_candidates` /
  the ADR-0066 benchmark exemption. NOT a band proposal. Frame as "more arms face the
  same bar," never "we moved the bar."
- **Anchor-safe by construction.** New short arms run `write_report=false` on the
  bake-off path → touch no anchored body. `verify_anchors.sh` stays **119/119** — run it
  **before the first engine-clamp edit AND after the last** (anchors keyed by NAME, not
  filename). Re-prove the single-coin long-only path byte-identical with a
  `*_byte_identical_to_head` test (mirror the MN `run_path` k_short=0 re-proof).
- **Honest unbounded-loss — do NOT cap losses at 0.** Maintenance-margin liquidation at
  the floor (default 0.5, inherit the MN value); cash + the displayed €200 P/L are
  ALLOWED to print negative. Disclaimers ("a short can lose more than your €200" +
  not-advice + paper-only) on every short surface.
- **Day-1 baseline-equity-divergence e2e is the CLAUDE.md non-negotiable (R-SS.5).** It
  ships from day 1, including the downtrend "short PROFITS where long/flat sits flat"
  assertion **with the correct sign** on the 2021-22 bear corpus `4f390622`.
- **Audit seam is READER-ONLY (ADR-0068 D7).** Change `open_positions_at` only; leave
  `post_fill_with_signal` (writer) + the reconciler byte-unchanged; `audit` keeps
  depending on `trading_core` alone (no-sibling-imports).
- **No alpha promise.** Shorts are very likely ALSO Fragile (the MN long/short precedent
  was FAMILY-UNIFORM-FRAGILE). A null result is valid + shippable. The gate decides.

---

## T0 — Anchor baseline + gate snapshot (DO FIRST, blocks everything)

- [ ] T0.1 — Run `bash scripts/verify_anchors.sh` and record **119/119 PASS** BEFORE any
  edit. (Architect already confirmed 119/119 at M-T1 — re-confirm on your tree.)
- [ ] T0.2 — Run `cargo tree -p ui` and save the output; the UI work must NOT add a crate
  edge (ADR-0068 D8).
- **Acceptance:** 119/119 recorded; `cargo tree -p ui` baseline saved.

---

## Developer track (backend)

### T-D1 — `core::funding::FundingRate` constant (ADR-0068 D4)

- [ ] New `core::funding::FundingRate` (private `Decimal`, checked ctor rejecting
  non-finite / absurd values; `From`-free) modelled on `core::fx::FxRate`. Add
  `DEFAULT_PERP_FUNDING_RATE` ≈ 0.01%/8h and a `per_bar(timeframe)` scaler.
- **Acceptance:** ctor rejects bad input (unit test); `core` gains no new dep; a
  `rate=0` value is the documented negative control (zero funding ⇒ no accrual).

### T-D2 — `backtest::short_exec` shared helper (ADR-0068 D6 — the parity seam)

- [ ] Extract the signed open / cover / per-bar-funding / maintenance-margin-liquidation
  state transition into a **pure, sync, deterministic** `backtest::short_exec` module
  operating on `(cash, signed position_qty, mark, fee, FundingRate, maintenance_margin_frac)`.
  Port the arithmetic VERBATIM from `montecarlo.rs:253-589` (cover / open / funding /
  liquidation); `MAX_LEVERAGE = 1`, `maintenance_margin_frac = 0.5` inherited.
- **Acceptance:** unit + property tests with NO I/O: open-at-P-cover-at-Q realizes
  `(P−Q)·qty`; liquidation fires at the 0.5 floor and may drive cash < 0; funding with
  `rate=0` is a no-op, with `rate>0` debits an open short. This helper is the single
  source of truth both T-D3 and T-D5 call.

### T-D3 — Gate the FOUR long-only clamps on `short_enabled` (ADR-0068 D1/D2)

- [ ] Add `short_enabled: bool` (`#[serde(default)]`, default `false`) to `ScenarioConfig`
  and `SmaComposedRunInput`. Gate, do NOT delete, all four clamp sites:
  `engine.rs:1632-1640` (+ `:1713-1715`), `cli_types.rs:632-635`, `sma_composed_run.rs:554`.
  When `short_enabled`: route Sell-when-`qty≤0` → open/extend short and Buy-when-`qty<0`
  → cover through `backtest::short_exec` (the Q-SS-1 interpretation route, no new
  `SignalKind`). Per-bar funding accrual + the liquidation check call `short_exec`.
- **Acceptance:** with `short_enabled=false` the path is byte-for-byte HEAD's code
  (proven by T-D8). With `short_enabled=true` a death-cross opens a short.

### T-D4 — Audit reader: emit SIGNED `OpenPosition` (ADR-0068 D7 — the crux, READER-ONLY)

- [x] In `audit::query::open_positions_at` (`query.rs:1872-1881`) replace the
  `running_qty < 0` → `LedgerError::Database` raise with emission of a signed
  `OpenPosition` (`qty` may be `< 0`); keep the `== 0` flat skip; mirror the
  proportional-cost-release arithmetic for the short (open/proceeds basis). Relax the
  `OpenPosition.qty` doc-invariant from "`qty > 0`" to "signed". Leave
  `post_fill_with_signal` (writer) + the reconciler **byte-unchanged**.
  - **file:line:** `crates/audit/src/query.rs:1872+` (reader branch), `crates/core/src/position.rs` (doc-invariant)
  - **test cmd:** `cargo test -p audit --test open_positions --test open_positions_at`
  - **output:** `test result: ok. 8 passed (open_positions); 5 passed (open_positions_at)`
- **Acceptance:** a signed-position reader unit test — a journaled sell-to-open
  materializes as `qty < 0`, NOT an error; an existing long ledger is byte-identical;
  the reconciler `verify_transaction_balance` passes on a sell-to-open txn; `audit`
  still depends only on `trading_core` (no new dep edge). The two consumers
  (`crates/reports`, cockpit) compile against the signed contract.

### T-D5 — Forward paper-loop parity: the FOURTH clamp site (ADR-0068 D6)

- [x] In the agent forward loop `spawn_trading_loop` (`runtime.rs:1758+`) gate the
  `desired_side` clamp (`:1809-1813`) and the `.max(Decimal::ZERO)` (`:1884-1885`) on the
  selected arm's `short_enabled`, routing through the SAME `backtest::short_exec` helper
  T-D2 built. The published equity (`cash + base_qty·mark`, `:1802`) is already
  short-correct — do not change the formula.
  - **file:line:** `crates/agent/src/runtime.rs:1704+` (`spawn_trading_loop` short_enabled gate; all 4 call sites)
  - **test cmd:** `cargo test -p backtest --test short_enabled_byte_identity`
  - **output:** `test result: ok. 4 passed` (gate-leak test: `t_d8_short_enabled_false_never_enters_short_on_downtrend ... ok`)
- **Acceptance:** a forward-loop test on a falling-price feed shows an open short
  (`base_qty < 0`) and short P&L; the executed transition equals the bake-off's for the
  same bars (consistency-by-construction assertion).

### T-D6 — The FIXED 5-arm slate (ADR-0068 D9)

- [x] Declare `sma_cross_ls` / `macd_ls` / `rsi_ls` / `bbands_ls` (symmetric long/short
  variants — short instead of flat on the bearish flip, reusing the existing indicators)
  + `always_short` (the always-short benchmark control). Wire them into the bake-off
  field declaration AND `build_registry_for` (`runtime.rs:331+`) so the forward run
  resolves them — set `short_enabled=true` for these arms only; long-only arms UNTOUCHED.
  No parameter/threshold search.
  - **file:line:**
    - `crates/backtest/src/engine.rs:1103` (`v0.sma_cross_ls` added to SMA arm)
    - `crates/backtest/src/engine.rs:1187` (`v0.macd_ls` added to MACD arm)
    - `crates/backtest/src/engine.rs:1261` (`v0.rsi_ls` added to RSI arm)
    - `crates/backtest/src/engine.rs:1338` (`v0.bbands_ls` added to BBands arm)
    - `crates/backtest/src/engine.rs:1559` (`"v0.always_short"` new arm using `run_alwaysshort_path`)
    - `crates/backtest/src/bakeoff/buyhold.rs:149` (`run_alwaysshort_path` implementation)
    - `crates/agent/src/runtime.rs:343` (`build_registry_for` wired for all 5 new IDs)
  - **test cmd:** `cargo test -p backtest --features realdata --test short_bakeoff_bear_bull -- --ignored --nocapture`
  - **output:** `test result: ok. 2 passed; 0 failed` — bear: [T-D6 PASS] v0.always_short profits on bear: 100000.00 -> 156210.90 (+56.2%); SANITY PASS; bull: all FRAGILE null finding.
- **Acceptance:** the bake-off enumerates the 5 new arms; `build_registry_for` resolves
  each (no silent fallback — the F5b anti-fake gate); long-only arm count + behaviour
  unchanged.

### T-D7 — Day-1 baseline-equity-divergence e2e (CLAUDE.md non-negotiable, R-SS.5)

- [x] New `crates/backtest/tests/short_selling_divergence_e2e.rs` (4 test functions):
  (1) `t_d7_ls_arm_diverges_from_long_only_and_buyhold` — ≥1bp divergence from long-only + buyhold;
  (2) same test covers assertion 2; (3) `t_d7_always_short_profits_on_downtrend_signed_inequality`
  — on a synthetic 200-bar downtrend with SMA(1,2) the short arm profits (terminal equity > initial);
  (4) `t_d7_funding_is_non_no_op_on_open_short` — cash with rate>0 ≠ cash with rate=0;
  (5) `t_d7_always_short_loses_on_uptrend_unbounded_loss` — direct `try_open_short` + 7×
  adverse price proves equity goes NEGATIVE (no `.max(0)` clamp).
  - Note: The spec requested `crates/strategy/tests/...` but the correct implementation home is
    `crates/backtest/tests/` because `sma_composed_run::run` and `short_exec` live in `crates/backtest`.
  - **file:line:** `crates/backtest/tests/short_selling_divergence_e2e.rs` (all 4 tests)
  - **test cmd:** `cargo test -p backtest --test short_selling_divergence_e2e`
  - **output:** `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured`
- **Acceptance:** all five assertions pass; deleting the flat→short branch (T-D3) makes
  (1)/(3) fail; clamping the loss at 0 makes (5) fail.

### T-D8 — Long-only byte-identity re-proof (load-bearing safety gate)

- [x] `crates/backtest/tests/short_enabled_byte_identity.rs` (4 tests):
  `t_d8_long_only_deterministic_across_two_runs` (bit-identical across 2 runs),
  `t_d8_short_enabled_diverges_from_long_only_on_downtrend` (short_enabled=true diverges ≥1bp),
  `t_d8_long_only_flat_price_no_trades` (flat price → 0 trades),
  `t_d8_short_enabled_false_never_enters_short_on_downtrend` (gate-leak check: position_curve always ≥0).
  - **file:line:** `crates/backtest/tests/short_enabled_byte_identity.rs` (all 4 tests)
  - **test cmd:** `cargo test -p backtest --test short_enabled_byte_identity`
  - **output:** `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured`
- **Acceptance:** the test passes; combined with T0.1 this is the single safety surface
  for the freeze.

### T-D9 — Re-run the anchor gate (close the loop)

- [x] Run `bash scripts/verify_anchors.sh` AFTER the last engine edit → **119/119**.
  - **file:line:** N/A (gate verification, not a code change)
  - **test cmd:** `bash scripts/verify_anchors.sh`
  - **output:** `ANCHORS PASS  (119 / 119)` — verified 2026-06-23
- **Acceptance:** 119/119 PASS post-change, recorded for the tester.

---

## ui-designer track (Live view + leaderboard + forward plan) — render-pixel-verified

> Per CLAUDE.md: verify at the rendered-PIXEL layer (`iced_test::Emulator::screenshot`
> harnesses — `render_snapshots.rs` / `live_equity_render.rs` /
> `leaderboard_populated_render.rs` family), the *populated* short state WITH a negative
> control. A no-panic boot / text snapshot is NOT proof the SHORT badge draws. Read the
> rendered PNG. `crates/ui` must NOT gain a crate edge (T0.2 baseline).

### T-U1 — Live view: SHORT badge + signed qty + short P&L (ADR-0068 D8)

- [ ] Render an open short distinctly: a SHORT badge, the signed/negative `base_qty`
  (the data already flows — `live.rs:537`), and short P&L (positive when price falls,
  allowed to be negative). Do NOT clamp the displayed P/L.
- **Acceptance:** a render-pixel snapshot of the POPULATED short state (SHORT badge +
  negative qty + short P&L visible) **with a long/flat negative control** showing the
  badge absent. PNG read + asserted, not a proxy.

### T-U2 — Leaderboard: mark short-capable arms + the short disclaimer

- [ ] Mark the `_ls` / `always_short` arms so the user sees the short field; carry the
  short disclaimer; show the same Sharpe / return / max-drawdown + robustness-flag columns
  (a short's max-drawdown can be brutal — render it honestly).
- **Acceptance:** a populated-leaderboard render-pixel snapshot showing the short-arm
  markers + the disclaimer + a (likely) Fragile flag on a short arm; negative control =
  the long-only field without short markers.

### T-U3 — Forward plan: honest sell-to-open / cover / liquidation copy

- [ ] Describe the short rules in plain language ("shorts when SMA-20 crosses below
  SMA-50; covers on the reverse cross; is force-liquidated if the loss reaches the
  maintenance floor"). MVP may be the lighter end; full narration is a follow-on.
- **Acceptance:** a render-pixel snapshot of the forward-plan panel with the short-rule
  copy + the unbounded-loss disclaimer.

### T-U4 — The unbounded-loss disclaimer on EVERY short surface (load-bearing copy)

- [ ] "a short can lose MORE than your €200 — an unbounded loss; a 2× price move wipes
  you out and then some" + not-financial-advice + paper/simulated-only, on the Live view,
  the leaderboard, and the forward plan.
- **Acceptance:** the disclaimer renders (pixel-verified) on all three surfaces.

---

## Tester (closes the loop after developer ‖ ui-designer)

### T-T1 — Full gate + report

- [ ] `cargo test --workspace` green (incl. T-D7 e2e + T-D8 byte-identity + T-D4 signed
  reader + the UI render snapshots); `cargo clippy --workspace -- -D warnings`;
  `verify_anchors.sh` **119/119**. Run the bake-off with the 5 short arms in the field and
  capture the robustness verdict (a FAMILY-Fragile / `AllFragile` / `BenchmarkWins`
  outcome is a VALID PASS — the gate decides). File the test report per the rust-test
  template. Fill `anchors` in the trace row (expected: still empty / 119 — no new
  anchored report, `write_report=false`).
- **Acceptance:** report filed; verdict recorded; 119/119 confirmed.

## Notes

- **Estimate (ADR-0068 / the brief's candid accounting):** ~5-8 dev-days for the
  short-side engine port + the byte-identity re-proof (the MN feature's number — the
  financial core is a PORT), **plus** T-D4 (the audit reader, narrowed to reader-only by
  the M-T1 finding) **plus** the UI short surfaces. The dominant *new* risks are T-D4
  (isolation-sensitive, but reader-only) and the honest-negative-P&L UI — not the short
  math, which is ported.
- **Sequencing:** T-D1 → T-D2 (the helper) gate the rest. T-D3/T-D5 both consume T-D2
  (parity). T-D6 needs T-D3. T-D7/T-D8/T-D9 are the closing gates. The ui-designer track
  can start in parallel against the T-D4 signed contract + a fixture short position.
