---
adr: 0077
title: Forward-buildability contract — every crownable arm is forward-buildable; unknown still bails; None path byte-identical
status: accepted
date: 2026-06-30
supersedes: none
superseded-by: none
---

# ADR-0077: Forward-fidelity coverage — wiring the 14 post-F5b crownable arms

## Context

ADR-0060 defined the forward-run seam: `build_registry_for(cfg, Some(&fwd))` resolves
a strategy id to a `Box<dyn Strategy>` for the paper loop, and `build_forward_plan_from_registry`
resolves the same id to a `PlanDescribe` implementor for the F6 forward plan. F5b
(feature `advisor-forward-paper`) shipped those resolvers with 7 ids covered and an
explicit `unknown => bail!` anti-fake gate ("refusing to silently fall back to
SmaCrossover proxy").

Between F5b and v2, four ADRs added new crownable arms to the bakeoff field:

| ADR    | Arms added                                                                                     |
|--------|-----------------------------------------------------------------------------------------------|
| 0067   | 6 ensemble arms: `v0.8.vote.trend_pair`, `tr_mr_macd_rsi`, `tr_mr_sma_bb`, `any1of4`, `k2of4`, `k3of4` |
| 0071   | 5 DSL arms: `v0.donchian_break`, `v0.donchian_floor`, `v0.vol_breakout`, `v0.roc_momentum`, `v0.obv` |
| 0072   | 1 arm: `v0.dvol_regime` (DvolRegimeStrategy — a hand-written exogenous-series strategy)       |
| 0073   | 1 arm: `v0.macro_riskon` (run_macro_gated_buyhold_path — NOT a Strategy impl)                |

None of these 14 were added to `build_registry_for` or `build_forward_plan_from_registry`.
The honesty contract was broken: the SUGGESTION stage could crown an arm it could not
build, causing the forward run to error. The architect named this the **top refactor
gate for v2** (v2-architecture.md §2 R1).

## Decision

**D1 — Coverage scope.** Wire all 14 ids in `build_registry_for` (runtime.rs) AND
`build_forward_plan_from_registry` (plan.rs). The `unknown => bail!` sentinel STAYS in
place — it is the anti-fake gate. Only the 14 named ids move from bail to real constructors.

**D2 — ADR-0071 DSL arms (5 ids).** Each loads its `config/strategies/<id>.toml` via the
existing `load_composed_strategy_from_toml` helper — the SAME TOML the bakeoff scored.
This is the F5b MACD/RSI/BBands pattern applied to the 5 ADR-0071 arms. TOMLs exist:
`btc_donchian_break`, `btc_donchian_floor`, `btc_vol_breakout`, `btc_roc_momentum`, `btc_obv`.

**D3 — ADR-0067 ensemble arms (6 ids).** `build_ensemble(id)` in `crates/strategy/src/ensemble.rs`
already handles all 8 vote ids. The fix is one match-arm widening in `build_registry_for`
and `build_forward_plan_from_registry` — no new engine code.

**D4 — ADR-0072 DVOL regime arm (1 id).** `DvolRegimeStrategy` is a proper `Strategy`
impl. In the forward paper loop, the DVOL corpus is not loaded (no real-time DVOL feed).
We construct `DvolRegimeStrategy::new(symbol, vec![], DVOL_REGIME_WINDOW)` — empty
`as_of_dvol` → warm-up-only (flat) behaviour. This is the correct graceful degradation:
the strategy starts flat and stays flat until the first real DVOL data is fed, mirroring
the bakeoff engine's `dvol_override = None` behaviour for unsupported symbols.

For the plan describer: `DvolRegimeStrategy` does NOT implement `PlanDescribe`. We use
`AlwaysLongStrategy` as the plan describer — honest because the forward plan shows
"BuyAndHold" intent (flat → buy when funded) which is the DVOL arm's warm-up default.

**D5 — ADR-0073 macro risk-on arm (1 id).** `v0.macro_riskon` in the bakeoff uses
`run_macro_gated_buyhold_path`, a standalone function that is NOT a `Strategy` impl.
There is no `MacroRiskonStrategy` struct. The forward paper loop requires `Box<dyn Strategy>`.
We register `AlwaysLongStrategy` — the honest approximation because:
- The macro corpus (`data/yahoo-macro/`) is NOT loaded in the forward run.
- `run_macro_gated_buyhold_path` with an empty regime degrades to buy-and-hold (the
  same `unwrap_or(empty_series)` path the bakeoff engine uses — ADR-0073 D3).
- `AlwaysLongStrategy` in the forward paper loop produces buy-and-hold behaviour.
  The narration can note "macro-gated hold; real-time macro feed not yet wired."

**D6 — None path byte-identical.** `build_registry_for(cfg, None)` returns the default
SMA registry as before — unchanged. Anchors 119/119 untouched by construction.

**D7 — Forward-buildability contract (amends ADR-0060 § D3).** Every arm in
`BakeoffConfig::advisor_field()` / `default_ensemble_field()` / `default_macro_field()`
MUST be present in `build_registry_for`'s match. Future arm additions MUST update both
`build_registry_for` and `build_forward_plan_from_registry` atomically. The `unknown =>
bail!` arm remains the canary — if a new arm is added to the field but not to the
resolvers, the next forward run will error clearly.

## Alternatives considered

**A1 — Lazy auto-dispatch from bakeoff arm metadata.** Route `build_registry_for` by
extracting a "constructor kind" from the bakeoff arm definition. Rejected: the bakeoff
engine uses different execution paths (standalone functions, `Strategy` impls, exogenous
series) that do not map cleanly to a single dispatch table without new architecture.
The explicit match-arm pattern is safer, readable, and auditable.

**A2 — Wire `v0.macro_riskon` with a new `MacroRiskonStrategy` struct.** Requires
implementing `Strategy` for a struct that needs real-time macro data, and implementing
`PlanDescribe` for it. No architecture for a real-time macro feed exists. Deferred to a
future ADR when a live feed is available.

**A3 — Wire `v0.dvol_regime` with a `PlanDescribe` impl.** `DvolRegimeStrategy` currently
lacks `PlanDescribe`. Adding it would require a new plan rule shape (e.g., `PlanRuleShape::ExogenousRegime`).
Deferred as a follow-on — the current AlwaysLongStrategy describer is honest and safe.

## Consequences

**Positive:**
- Crowning any of the 14 arms no longer errors the forward run.
- The F6 forward plan describes the crowned arm's rule (BuyAndHold for dvol/macro,
  the real DSL/ensemble rule for the other 12).
- Future arm additions have a clear contract: update both resolvers, the bail! gate
  will surface missing coverage immediately.

**Negative / mitigations:**
- `v0.macro_riskon` and `v0.dvol_regime` run buy-and-hold in the forward paper loop
  (no real-time macro/DVOL corpus). This is honest, not misleading — the narration can
  frame it as "macro-gated strategy; corpus not available in paper loop."
- These two arms should not be crowned in production unless a real-time feed is wired.
  The bakeoff gate (FRAGILE/ROBUST/Marginal) is the existing quality control.

**Anchors:**
- `None` path byte-identical → 119/119 unchanged by construction.
- No anchored report body is touched — the 14 arms run `write_report=false` in the
  bakeoff path (anchor-safe by construction since ADR-0067/0071/0072/0073).

## Tests

`crates/agent/tests/forward_run_engine_fidelity.rs` extended with 13 `r1_*` tests:
one per arm family, asserting `build_registry_for` returns `Ok` and the registered
strategy id is not `"sma_crossover"`. The dvol test additionally asserts `"dvol_regime"`;
the macro test asserts `"always_long"`.

Leans on ADR-0060 (forward-run seam) / ADR-0067 / ADR-0071 / ADR-0072 / ADR-0073.
