---
slug: advisor-forward-fidelity-coverage
status: shipped
owner: operator
version: 0.1.0
updated: 2026-07-01
---

# Forward-Fidelity Coverage (R1) — 14 missing arms wired

**Design:** [`v2-architecture.md`](../v2-architecture.md) §2 R1.
**Owed ADR:** ADR-0077 (amends ADR-0060 forward-run seam contract).

## Problem

`build_registry_for` (`crates/agent/src/runtime.rs`) routed only 7 strategy ids.
The crownable advisor field has ~32 ids; the 14 added after F5b (ADR-0067/0071/0072/0073)
hit the `unknown => bail!` arm. If the bakeoff crowned any of them the forward
run errored — not silently proxied (the anti-fake gate works), but refused to proceed.

## What shipped

### 14 arms wired — mapping

| Strategy id | Family | Constructor |
|---|---|---|
| `v0.donchian_break` | ADR-0071 DSL | `load_composed_strategy_from_toml("btc_donchian_break")` |
| `v0.donchian_floor` | ADR-0071 DSL | `load_composed_strategy_from_toml("btc_donchian_floor")` |
| `v0.vol_breakout` | ADR-0071 DSL | `load_composed_strategy_from_toml("btc_vol_breakout")` |
| `v0.roc_momentum` | ADR-0071 DSL | `load_composed_strategy_from_toml("btc_roc_momentum")` |
| `v0.obv` | ADR-0071 DSL | `load_composed_strategy_from_toml("btc_obv")` |
| `v0.8.vote.trend_pair` | ADR-0067 ensemble | `build_ensemble(id)` |
| `v0.8.vote.tr_mr_macd_rsi` | ADR-0067 ensemble | `build_ensemble(id)` |
| `v0.8.vote.tr_mr_sma_bb` | ADR-0067 ensemble | `build_ensemble(id)` |
| `v0.8.vote.any1of4` | ADR-0067 ensemble | `build_ensemble(id)` |
| `v0.8.vote.k2of4` | ADR-0067 ensemble | `build_ensemble(id)` |
| `v0.8.vote.k3of4` | ADR-0067 ensemble | `build_ensemble(id)` |
| `v0.dvol_regime` | ADR-0072 DVOL | `DvolRegimeStrategy::new(symbol, vec![], DVOL_REGIME_WINDOW)` |
| `v0.macro_riskon` | ADR-0073 macro | `AlwaysLongStrategy::new()` (graceful degradation) |

**Notes:**
- ADR-0071 (5 DSL arms): load the exact same TOML the bakeoff scored — byte-identical strategy.
- ADR-0067 (6 ensemble arms): `build_ensemble` already knew all 8 ids; one match-arm widening.
- ADR-0072 DVOL: `DvolRegimeStrategy` is a proper `Strategy` impl; empty `as_of_dvol` →
  warm-up-only (flat) in the forward run (no DVOL corpus loaded). Honest degradation.
- ADR-0073 macro: `run_macro_gated_buyhold_path` is NOT a `Strategy` impl. `AlwaysLongStrategy`
  is the honest proxy — the macro gating degrades to buy-and-hold when the macro corpus is absent
  (same `unwrap_or(empty_series)` pattern the bakeoff engine uses for ADR-0073 D3).

### `build_forward_plan_from_registry` (plan.rs) — same 14 ids

All 14 mirrored in `plan.rs` so the F6 forward plan describes the real crowned rules.
For dvol/macro the plan describer is `AlwaysLongStrategy` (BuyAndHold rule kind) — honest
since neither has a `PlanDescribe` impl and the forward plan shows "no active signal yet".

### Anti-fake gate STAYS

The `unknown => bail!` sentinel remains. Only the 14 named ids moved from bail to a real
constructor. Any new arm NOT in this list will still error at forward-run time (correct
behaviour — forces explicit wiring).

### Tests

- `crates/agent/tests/forward_run_engine_fidelity.rs` extended: 13 new `r1_*` tests
  (14 arms mapped, 1 test per id). Each asserts `build_registry_for` returns `Ok` and
  the registered id is not `"sma_crossover"`. The dvol test additionally asserts `"dvol_regime"`;
  the macro test asserts `"always_long"`.
- Existing 8 tests unchanged: `f5b_*` suite still green.

## Not in this increment

- A forward `DvolRegimeStrategy` with a live DVOL feed — needs architecture for real-time
  DVOL data in the paper loop (deferred, no scope).
- A `Strategy` impl wrapper for `v0.macro_riskon` with live macro data — same constraint.
- Forward-plan narrative for dvol/macro (both show "BuyAndHold" plan until corpus is wired).

## ADR

ADR-0077: `_bmad-output/planning-artifacts/architecture/decisions/0077-forward-fidelity-coverage.md`

## Changelog

- 2026-06-30 (developer): wired 14 missing arms in `build_registry_for` + mirrored in
  `build_forward_plan_from_registry`; 13 R1 forward-buildability tests added;
  ADR-0077 written + registered; anchors 119/119 unchanged.
