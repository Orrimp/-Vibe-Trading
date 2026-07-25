---
adr: 0065
title: EUR→USDT budget-conversion seam (configurable static FX rate)
status: accepted
date: 2026-06-22
supersedes: none
superseded-by: none
---

# EUR→USDT budget-conversion seam (configurable static FX rate)

## Context

The single-coin advisor's MVP closed with the operator entering a budget **in
euros** (the user is a European retail investor) while the engine is
**USDT-denominated end to end** — `ForwardRunConfig.budget`,
`FixedFractionSizer.budget_cap`, and `BacktestKpis.final_equity` are all
`Money<Usdt>`, and **no `Eur` currency type exists** in `crates/core`
(`asset.rs` defines `Usdt`/`Btc`/`Eth` only — verified 2026-06-22).

Today that mismatch is papered over by a **1:1 collapse** at
`crates/ui/src/bin/cockpit_live.rs:1431-1437`: the parsed euro `Decimal`
(`LeaderboardScreenState::budget_eur()`, `leaderboard/state.rs:589`) is stamped
straight into `Money::<Usdt>::from_decimal(budget_decimal)` with no conversion,
and the three display literals (`LEADERBOARD_BUDGET_HINT`,
`FORWARD_PLAN_BUDGET_LINE`, `LIVE_FORWARD_FX_NOTE` in `ui/src/strings.rs`) say
**"€200 ≈ 200 USDT — FX not modelled."** The label literally tells the operator
the tool is off by ~5–10 %.

F7 (the last v0.2 roadmap item; product § D4 option (b)) makes the conversion
honest: `usdt_budget = eur_budget × rate` at the single budget-conversion
boundary, with the labels rewritten to **"€200 ≈ $216 (at 1.08 EUR/USD,
⟨source/as-of⟩)."**

The analyst verified (`spec/advisor-eur-fx/feature.md` § Ground truth) that **no
FX/forex source exists anywhere** (`grep eur.usd|forex|fx.rate|exchange.rate` →
zero; the fetchers are crypto-only), so F7 introduces a *new, tiny* rate value.
The product is paper-only, single-operator, the budget is a round €200 the user
picked arbitrarily, and the bake-off ranking is **FX-invariant** (a constant
scalar on the budget cannot change which strategy wins).

**Operator decision (LOCKED):** the rate source is a **configurable static
rate** — a config value (default `~1.08`, operator-editable) carried with a
source/as-of provenance label. A live-fetched rate is an explicit **v0.3
upgrade** layered on this, not part of F7. This ADR designs the static path as
the stable core.

## Decision

### D1 — `FxRate` is a small value object in `crates/core` (`core::fx`)

A new `crates/core/src/fx.rs` module exports:

```rust
pub struct FxRate {
    rate: Decimal,        // USDT per 1 EUR (e.g. 1.08); private + validated > 0 at ctor
    source: SmolStr,      // provenance label, e.g. "config"
    as_of: SmolStr,       // as-of label, e.g. "2026-06-22" (a LABEL, not a clock read)
}
```

- **Home = `crates/core`** (the base crate — `backtest`/`risk`/`agent`/`ui`/`data`
  all depend on it; it depends on none of them). `core` already carries
  `rust_decimal` + `smol_str` + `serde` (verified — **zero new dependency**, no
  new dep edge). This is the ADR-0058 § D2 "home the primitive in `core`"
  precedent (the same reason `PitSeries` went there): the rate must reach a
  UI-side seam **as a `core` type** so that **`cargo tree -p ui` is unchanged by
  construction** — `ui` already imports `trading_core`, so consuming a new `core`
  type adds no edge. Homing it in `crates/data` (the natural home for a *future*
  fetcher, D6) is **REJECTED for the static value** — it would force a `ui → data`
  edge that does not exist today, the exact layering breach OQ-AR-1 guards.

- `rate` is **private** with a checked ctor (`FxRate::new(rate, source, as_of) ->
  Result<FxRate, FxRateError>`, rejecting `rate ≤ 0`) plus an infallible
  `FxRate::config(rate)` convenience that stamps `source = "config"`. There is no
  public mutable rate field — a constructed `FxRate` is always valid.

- **The rate VALUE comes from config.** A `pub const DEFAULT_EUR_USD_RATE:
  Decimal = dec!(1.08)` lives in `core::fx`; the advisor config surface
  (`AdvisorConfig` / the existing cockpit config) carries an optional
  `eur_usd_rate: Option<Decimal>` + `eur_usd_rate_as_of: Option<String>`
  (`#[serde(default)]`, additive — anchor-irrelevant, it is a UI-input config not
  a scenario config). The cockpit binary resolves
  `FxRate::config(cfg.eur_usd_rate.unwrap_or(DEFAULT_EUR_USD_RATE))` at the seam.
  `DEFAULT_EUR_USD_RATE` is **also the fallback constant a future live fetcher
  (D6) would degrade to** — so the static path is not a carve-out, it is the
  primitive the live path reuses (the OQ-OP-1 "(b) is a strict superset of (a)"
  proof).

### D2 — ONE conversion function, ONE converted value, two readers (no drift)

The single source of truth is one pure method on `FxRate`:

```rust
impl FxRate {
    /// usdt = eur × rate. The ONLY EUR→USDT arithmetic in the codebase.
    pub fn convert_eur_to_usdt(&self, eur: Decimal) -> Money<Usdt> {
        Money::<Usdt>::from_decimal(eur * self.rate)
    }
}
```

- **EUR-at-input representation: a labelled `Decimal`, NOT a new `core::Eur`
  marker.** `budget_eur()` already returns `Option<Decimal>` and already names
  the unit; the conversion fn takes that `Decimal` and emits `Money<Usdt>`. A
  first-class `Eur` currency + `Money<Eur>` is **REJECTED** (product § D4 option
  (c)): it would force an `Eur: Currency` impl, an `Eur`-arm everywhere `Asset`
  is matched, and tempt FX-PnL through the ledger — all for a value that exists
  for exactly one multiply at one input boundary. EUR stays a labelled input
  scalar; F4/F5 stay **unit-agnostic** (they keep consuming `Money<Usdt>`,
  byte-unchanged). This is the lighter option the brief recommends.

- **The seam.** At `cockpit_live.rs:1431-1437`, the line
  `Money::<Usdt>::from_decimal(budget_decimal)` is **replaced** by a single
  conversion that produces a small carrier read by BOTH the engine and the
  display:

  ```rust
  let fx = FxRate::config(resolved_rate);              // from config / default
  let conversion = BudgetConversion::new(budget_decimal, fx); // {eur, rate, usdt}
  let budget: Money<Usdt> = conversion.usdt();         // → ForwardRunConfig.budget (ENGINE)
  // conversion.eur(), conversion.rate(), conversion.usdt() → the display string (DISPLAY)
  ```

  `BudgetConversion` (also `core::fx`) holds `{ eur: Decimal, rate: FxRate, usdt:
  Money<Usdt> }` where `usdt` is computed ONCE via `rate.convert_eur_to_usdt(eur)`
  in its ctor. **The engine reads `conversion.usdt()`; the display reads
  `conversion.usdt()` (and `.eur()`, `.rate()`).** They cannot drift because
  there is one converted value and one conversion call — the F6 anti-drift
  discipline (ADR-0062: structured data, one boundary, two readers of the same
  value). The "$216" the operator reads is *definitionally* the `Money<Usdt>` F4
  caps against.

- **Display surfaces share the SAME `BudgetConversion`.** The three literals
  (`LEADERBOARD_BUDGET_HINT`, `FORWARD_PLAN_BUDGET_LINE`, `LIVE_FORWARD_FX_NOTE`)
  become format strings filled from `conversion` (the F6/Live plan path carries
  the same converted figure forward — symmetric with how `forward_budget` already
  flows). The hard-cap framing ("never deploys more than your budget") and the
  not-advice disclaimer are preserved verbatim (R3).

### D3 — Determinism + anchor-safety (R4, by construction)

- A config constant is a **deterministic input**: a fixed rate → a fixed
  converted budget → reproducible F4 sizing → reproducible render PNGs.
  `as_of`/`source` are **labels** (not `Timestamp::now()` reads) so they introduce
  no clock non-determinism.

- **The anchored CLI / headless path NEVER reads the rate.** The bake-off
  (`backtest::bakeoff`), `run_scenario`, and the sweep bins all take
  `Money<Usdt>` budgets/capital directly; the EUR→USDT conversion is an
  **advisor-input concern that lives only at the cockpit UI boundary**
  (`cockpit_live.rs`). No anchored scenario, no anchored report, and no
  `anchors.toml` SHA reads or writes the FX rate. `verify_anchors.sh` stays
  **119/119 by construction** — F7 reads no anchored corpus and writes no report
  body (confirmed: the F7 surface is `core::fx` + the cockpit binary seam + three
  `strings.rs` literals + a `core` test; none touch `spec/*/reports/`).

- **`cargo tree -p ui` unchanged** is a hard verification gate (D1 homes the type
  in `core`, already a `ui` dependency).

### D4 — THE DAY-1 CONVERSION-APPLIED GATE (CLAUDE.md non-negotiable — APPLICABLE)

F7 **modifies the budget the F4 sizing modifier consumes**, so per the
`v3-volatility-forecaster-noop-fix` (2026-05-22) precedent — a `scale` computed
but never applied — there MUST be a day-1 e2e proving the rate is **applied**,
not computed-then-stamped-1:1. This is the exact failure mode the non-negotiable
exists to catch (a budget that computes a rate but stamps 1:1 anyway is a silent
no-op).

**Gate = a dedicated `crates/core/tests/eur_fx_conversion_applied.rs`** (homed
with `FxRate`; the conversion arithmetic is the unit under test, mirroring how the
F4 gate lives in `crates/risk`). It is **FAIL-before / PASS-after** against a 1:1
stub (`convert_eur_to_usdt` returning `from_decimal(eur)`, ignoring `rate`):

1. **Converted ≠ 1:1 (the no-op guard).**
   `FxRate::config(dec!(1.08)).convert_eur_to_usdt(dec!(200))` equals
   `dec!(216)` (`200 × 1.08`) and is **strictly ≠** `dec!(200)` (the 1:1 value).
   Under the stub this FAILS (`216 != 200` is the asserted gap; the stub returns
   `200`). A `rate == 1.0` arm is the negative control (converted **==** 1:1, by
   design).

2. **The converted value is the one F4 sizes against (reaches the engine).**
   Build `BudgetConversion::new(dec!(200), FxRate::config(dec!(1.08)))`, assert
   `conversion.usdt() == Money::<Usdt>::from_decimal(dec!(216))`, then feed
   `conversion.usdt()` into `FixedFractionSizer::with_budget_cap(fraction,
   conversion.usdt())` and assert the sizer's effective cap reflects 216 (not
   200) — i.e. the converted budget, not the raw EUR, is what bounds deployed
   notional. This closes the "computed-then-dropped" hole: the value that bites
   F4 is provably the converted one.

3. **Display ↔ engine agreement.** Assert the figure the display formatter
   renders (`conversion.usdt()` formatted) is **byte-identical** to the
   `Money<Usdt>` value fed to F4 — they read the same `BudgetConversion`, so this
   is structurally true; the test pins it against regression.

The render-layer PNG (input panel + F6 plan showing "€200 ≈ $216 (at 1.08
EUR/USD)" with a **rate = 1.0 negative control** rendering "€200 ≈ $200") is the
tester's pixel-floor (CLAUDE.md iced render rule) and is owned by Verification,
not this gate.

### D5 — Honest display, fallback trivial under D1

- The three literals (R3) are rewritten to "€⟨eur⟩ ≈ $⟨usdt⟩ (at ⟨rate⟩ EUR/USD,
  ⟨source⟩ ⟨as_of⟩)", driven by the SAME `BudgetConversion`. Hard-cap +
  not-advice copy preserved verbatim.
- A configured rate is **always present** — there is no "unavailable" state (R5),
  so no fallback UI is needed for the static path. (The live path's
  fetch-failure → fallback-to-`DEFAULT_EUR_USD_RATE` + "live FX unavailable" label
  is a D6 concern, not built.)

### D6 — v0.3 live-rate upgrade (NOTED, NOT BUILT)

If the operator ever wants a live-tracking display, the upgrade layers cleanly on
this ADR and is recorded here so the seam is upgrade-ready:

- A `RateSource` trait (`fn rate(&self) -> Result<FxRate, RateError>`) with a
  `StaticRateSource(FxRate)` (wrapping D1's config value — **the fallback**) and a
  future `data::FxFetcher` impl (the fetcher belongs in `crates/data` per
  ADR-0061, NOT `ui`). Every test + render harness injects a **fixed fake rate**
  through this trait (the `crates/data` `HttpKlineFetcher`-vs-mock precedent,
  ADR-0061 § D1) — **never the network** (R4). The anchored CLI path still never
  reads a rate.
- This is a **strict superset** of D1: the live path *needs* D1's
  `DEFAULT_EUR_USD_RATE` as its fallback constant (R5), so shipping the static
  path first spawns **zero rework**. Building it now is **out of F7 scope** (no
  network/FX fetcher per the operator constraint); it requires its own ADR (a new
  external I/O + a failure-mode contract).

## Alternatives considered

- **First-class `core::Eur` currency + `Money<Eur>` + ledger FX-PnL** (product
  § D4 option (c)) — REJECTED. Makes the engine bilingual for one input multiply;
  tempts EUR reconciliation through the double-entry ledger; forces an `Eur` arm
  at every `Asset`/`Currency` match site. F7 converts the budget *into* the
  engine's unit; it does not teach the engine a second language.
- **Home `FxRate` in `crates/data` or a config struct in `ui`** — REJECTED.
  `data` forces a `ui → data` edge (the `cargo tree -p ui` gate fails); a `ui`-local
  type means the value object cannot be reused by a future `data` fetcher and
  re-litigates the home at v0.3. `core` is the only home that is already a
  dependency of every consumer AND of a future fetcher.
- **Live-fetched rate now (OQ-OP-1 (b))** — DEFERRED to v0.3 (D6). Buys ~1 %
  display precision on a round paper €200 at the cost of determinism, a network
  dependency, a new fetcher, and a fallback state — and still needs (a)'s
  constant as its fallback.
- **Derive from a EURUSD-like corpus pair (OQ-OP-1 (c))** — REJECTED. No fiat-FX
  series exists in the crypto-only corpus; ingesting one collapses into (b).
- **Convert inside F4 / the engine** — REJECTED. The engine is unit-agnostic and
  must stay so; converting at the boundary keeps F4/F5/bake-off byte-unchanged
  and keeps the conversion in exactly one place (D2).

## Consequences

- **Positive:** one new `core::fx` value object (no new dep), one pure conversion
  fn, one converted value both engine and display read (no drift), F4/F5/bake-off
  byte-unchanged, anchors 119/119 by construction, `cargo tree -p ui` unchanged,
  the v0.3 live path is a documented strict superset with zero rework. The "FX not
  modelled" hedge becomes an honest, deterministic figure.
- **Negative / accepted:** the rate is a configured constant, not live — a stale
  config value will be ~1 % off real EUR/USD until the operator edits it. For a
  paper, single-operator, FX-invariant-ranking tool this is the correct fidelity
  (the brief's OQ-OP-1 rationale); the honest label states the rate and its as-of
  so the operator is never misled.
- **Frozen surfaces honoured:** `FixedFractionSizer` (ADR-0060) consumes
  `Money<Usdt>` unchanged; `ForwardRunConfig` (ADR-0060 § D3) carries the same
  `Money<Usdt>` budget field; the bake-off ranking (ADR-0059) never reads the
  budget; the F6 plan (ADR-0062) and Live view read the shared converted figure.
  No `anchors.toml` SHA touched; no `spec/anchors.toml` anchor mutated (no ADR for
  anchor mutation needed — F7 mutates none).
- Leans on ADR-0058 § D2 (home the primitive in `core`), ADR-0060 § D1/D3 (F4
  sizing + `ForwardRunConfig` budget shape), ADR-0061 § D1 (the future
  fetcher-in-`data` + mock-seam precedent), ADR-0062 (one-boundary, two-readers
  anti-drift discipline), ADR-0003 (Decimal money math), ADR-0023/0041 (ui
  layering — no new `ui` edge).

## Changelog

- 2026-06-22 (architect): ADR-0065 filed — EUR→USDT budget-conversion seam for
  feature `advisor-eur-fx` (single-coin-advisor pivot F7, the last v0.2 item).
  Operator-LOCKED to a **configurable static rate** (default `1.08`, config-edit,
  provenance-labelled); live fetch is a v0.3 upgrade (D6) layered on this
  constant. Resolves OQ-AR-1..4: D1 `FxRate {rate,source,as_of}` value object in
  `crates/core::fx` (no new dep, no new `ui` edge — the ADR-0058 home-in-`core`
  precedent) with the rate value from advisor config + `DEFAULT_EUR_USD_RATE`;
  D2 ONE pure `FxRate::convert_eur_to_usdt(eur:Decimal)->Money<Usdt>` + a
  `BudgetConversion{eur,rate,usdt}` carrier computed once so the ENGINE
  (`ForwardRunConfig.budget`) and the DISPLAY read the SAME converted value (the
  F6 anti-drift discipline), EUR kept as a labelled `Decimal` at input (no
  `core::Eur` — F4 stays unit-agnostic); D3 deterministic + anchor-safe by
  construction (anchored CLI is USDT-denominated, never reads the rate → 119/119,
  `cargo tree -p ui` unchanged); D4 the day-1 CONVERSION-APPLIED gate
  (`crates/core/tests/eur_fx_conversion_applied.rs`, FAIL-before/PASS-after:
  converted ≠ 1:1 AND the converted value is the one F4 caps against AND display
  == engine) per the `v3-vol-overlay-noop` non-negotiable. D5 the three "FX not
  modelled" literals → honest "€X ≈ $Y (at R EUR/USD, source as-of)". D6 the
  v0.3 `RateSource` trait + fake seam NOTED-not-built. No engine code; no anchored
  content touched.
