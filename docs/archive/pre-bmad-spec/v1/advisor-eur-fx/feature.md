---
slug: advisor-eur-fx
status: shipped
owner: tester
updated: 2026-06-22
---

# F7 — Honest EUR→USDT budget conversion (the real EUR/USD rate)

> One-line: the operator enters a budget **in euros**; today the engine silently
> treats `€200` as `200 USDT` ("FX not modelled"). F7 applies a **real EUR/USD
> rate** at the single budget-conversion boundary so the simulated budget that
> flows into F4 sizing — and the "€200 ≈ …" labels the operator reads — is
> honest. This is the **last v0.2 roadmap item** and resolves product § D4
> (the deferred "fixed EUR→USD rate" refinement).

## Why

The product is a **paper decision-support tool**: pick a coin + **€200** → bake
off all strategies → rank → plan → paper-trade your simulated €200
([`../product.md`](../../product.md) § What this product IS). The operator's budget
is denominated in **euros** (the user is a European retail investor), but the
engine is **USDT-denominated end to end** — `ForwardRunConfig.budget`,
`FixedFractionSizer.budget_cap`, and `BacktestKpis.final_equity` are all
`Money<Usdt>`, and **no `Eur` currency type exists** in `crates/core` (only
`Usdt`/`Btc`/`Eth` — verified 2026-06-22, `crates/core/src/asset.rs`).

Today that mismatch is papered over by a **1:1 collapse**: the UI takes the
parsed euro amount and stamps it as `Money<Usdt>` with no conversion, then
labels it honestly-but-coarsely as **"€200 ≈ 200 USDT — FX not modelled."** That
was the right MVP call (product § D4 option (a)) — EUR/USD has hovered ~1.05–1.10,
the *relative* strategy ranking is FX-invariant anyway (a constant scalar on the
budget cannot change which strategy wins), and it kept the engine untouched. But
the label literally tells the operator the tool is lying to them by ~5–10%. F7
is the named v0.2 upgrade (product § D4 option (b), "a reasonable v0.2
refinement"): convert `€200 × rate = ~$216 USDT` at the budget boundary and
replace the "FX not modelled" hedge with an honest **"€200 ≈ $216 (at 1.08
EUR/USD, ⟨source/as-of⟩)."**

This is a **small, surgical, honesty feature**, not new machinery. It touches
exactly one arithmetic seam plus three display strings.

## What F7 IS (the honest scope)

Apply a real EUR/USD rate at the **budget-conversion boundary** — the single
point where the operator's euro budget becomes the `Money<Usdt>` that the engine
sizes against:

```
€200  ──×rate──▶  $216 USDT  ──▶  F4 FixedFractionSizer.budget_cap
                              ──▶  F5 forward-paper starting capital
                              ──▶  the "€200 ≈ $216 (at R EUR/USD)" display
```

- **The conversion:** `usdt_budget = eur_budget × eur_usd_rate`. One multiply.
- **Where it flows:** the converted `Money<Usdt>` is *exactly* what F4/F5 already
  consume today — F7 changes the **value**, not the plumbing.
- **The display:** every "€200 ≈ 200 USDT (FX not modelled)" label becomes
  "€200 ≈ $⟨converted⟩ (at ⟨rate⟩ EUR/USD, ⟨source/as-of⟩)" — input panel + the
  F6 plan + the Live forward note.

## What F7 IS NOT (explicit NON-goals)

- **NOT FX trading / an FX strategy / an FX leg.** F7 is a one-time
  budget-unit conversion at input, not a position, not a tradeable pair, not a
  hedge. The bake-off field is unchanged; no new strategy arm.
- **NOT an FX prediction / forecast.** The rate is a *known input as of the run*,
  not a predicted future rate. Paper-only stays paper-only; the not-advice and
  not-a-prediction stance is untouched.
- **Does NOT affect the bake-off RANKING.** The ranking is budget-independent
  (risk-adjusted return per `rank_candidates`; a scalar on the budget is
  FX-invariant). F7 affects **sizing** (the €→USDT amount F4 deploys) and
  **display** only. The robustness gate, the buy-and-hold benchmark, and the
  119/119 anchored backtest body-SHAs are all untouched — F7 reads no anchored
  scenario and writes no report.
- **NOT a first-class `Eur` currency + FX-PnL plumbing through the ledger** (product
  § D4 option (c)). That remains rejected for a paper tool — P/L is displayed in
  USDT (with the euro budget as the labelled reference), not reconciled in EUR
  through the double-entry ledger. F7 converts the budget *into* the engine's
  unit; it does not make the engine bilingual.
- **NOT a multi-currency feature.** EUR→USDT only (the one budget currency the
  product's single European operator uses).

## Ground truth verified against code (2026-06-22)

| Claim | Verified |
|---|---|
| **No FX/forex source exists anywhere** | `grep -rni "eur.usd\|forex\|fx.rate\|exchange.rate\|fiat" crates/` → **zero** matches. The fetchers are crypto-only: `fetch_binance_klines`, `fetch_binance_funding`, `fetch_binance_premium`, `fetch_yahoo_klines`. **F7 needs a new (tiny) rate source — be honest about that.** |
| **No `Eur` currency type** | `crates/core/src/asset.rs` defines `Usdt`/`Btc`/`Eth` only; `Money<C: Currency>`. The budget is `Money<Usdt>` from the UI boundary inward. |
| **The 1:1 collapse seam** | `crates/ui/src/bin/cockpit_live.rs:1431-1437`: `budget_eur()` returns the euro `Decimal`, the **very next line** `Money::<Usdt>::from_decimal(budget_decimal)` stamps it 1:1. **That adjacency IS the conversion boundary.** F7 inserts one multiply between them. |
| **`budget_eur()` already names the unit** | `crates/ui/src/leaderboard/state.rs:589` — `pub fn budget_eur(&self) -> Option<Decimal>` ("euros") + `parse_budget` (accepts a leading `€`). The function name already asserts euros; F7 makes the downstream stamp honour it. |
| **The "FX not modelled" literals** | `crates/ui/src/strings.rs`: `LEADERBOARD_BUDGET_HINT` (2442), `FORWARD_PLAN_BUDGET_LINE` (2716), `LIVE_FORWARD_FX_NOTE` (2828) — all hard-code "€200 ≈ 200 USDT — FX not modelled". These three strings (+ their render harnesses) are the display surface F7 rewrites. |
| **F4 consumes a plain `Money<Usdt>`** | `crates/risk/sizing.rs` `FixedFractionSizer::with_budget_cap(fraction, budget)`; `crates/agent/src/plan.rs:147` `let budget_cap: Money<Usdt> = cfg.budget;`. F4 is unit-agnostic — it caps at whatever USDT value it's handed. F7 hands it the converted value; **no F4 change**. |
| **Determinism today** | The bake-off + the F4 divergence e2e + the render harnesses are deterministic (fixed seeds, fixed budget literals). **A LIVE fetched rate would break this** (network + non-determinism + a failure mode). A **configured/static rate is a deterministic constant** — drop-in safe. This is the core tension behind the operator fork below. |

## Requirements

### R1 — A single EUR/USD rate, available at the budget-conversion boundary
A `Decimal` EUR/USD rate (USDT per 1 EUR) must be reachable at the seam where
`budget_eur()` → `Money<Usdt>` happens. **It carries provenance** — a source
label and an as-of marker — so the display can be honest ("at 1.08 EUR/USD,
config 2026-06-22"). Default value and provenance are the operator fork (§ Open
questions OQ-OP-1); the analyst recommendation is a **configurable static rate
with a config default** (deterministic, no network, honest "as configured").

### R2 — Convert at the boundary; ranking untouched
`usdt_budget = eur_budget × rate` applied at the `ForwardRunConfig` construction
seam (`cockpit_live.rs:1431-1437`) and the symmetric F6-plan/Live display path.
The bake-off `run_bakeoff` / `rank_candidates` path is **not** touched (it never
reads the budget). The converted value flows unchanged into F4
`with_budget_cap`, F5 starting capital, and the F6 projected sizing.

### R3 — Honest display in all three surfaces
Replace "€200 ≈ 200 USDT — FX not modelled" with "€200 ≈ $⟨converted⟩ (at ⟨R⟩
EUR/USD, ⟨source/as-of⟩)" in: the bake-off input hint (`LEADERBOARD_BUDGET_HINT`),
the forward-plan budget line (`FORWARD_PLAN_BUDGET_LINE`), and the Live forward
note (`LIVE_FORWARD_FX_NOTE`). The hard-cap framing ("never deploys more than
your budget") and the not-advice disclaimer are preserved verbatim. The converted
USDT figure shown MUST equal the value F4 actually sizes against (display ↔ engine
agreement — the F6 anti-drift discipline).

### R4 — Determinism preserved for tests + anchors
The conversion must be deterministic under test (a fixed rate → a fixed converted
budget → reproducible sizing + reproducible render PNGs). If the operator ever
chooses a live-fetched rate (OQ-OP-1 option b), the tests + the render harnesses
MUST inject a **fixed fake rate** through a rate-source seam (the `crates/data`
fetcher-mock discipline — every external I/O behind a trait), never hit the
network. The 119/119 anchored backtest body-SHAs stay byte-identical (F7 reads no
anchored scenario; the anchored CLI path never sees the budget).

### R5 — Honest fallback when a rate is unavailable
- **If configured (recommended):** show the configured rate + its provenance;
  there is no "unavailable" state (a constant is always present).
- **If live-fetched (the fork's option b):** a fetch failure (network, timeout,
  stale) MUST fall back to a sane labelled default ("≈ $216 at 1.08 EUR/USD —
  *fallback rate, live FX unavailable*") and never block the bake-off or the
  forward run. Honesty over a hard error.

## Design

**Full decision record: [ADR-0065](../../../_bmad-output/planning-artifacts/architecture/decisions/0065-eur-usdt-budget-conversion-seam.md)**
(EUR→USDT budget-conversion seam — configurable static FX rate). Architecture
changelog: [`../architecture.md`](../../architecture.md) § Changelog 2026-06-22.

The four architect-bound open questions resolve as follows.

### OQ-AR-1 — `FxRate` type + home (a `core` type, no new `ui` edge)

A small value object `FxRate { rate: Decimal, source: SmolStr, as_of: SmolStr }`
in a NEW `crates/core/src/fx.rs` module (re-exported from `core::lib`). `rate` is
**private** with a checked ctor (`FxRate::new(..) -> Result<_, FxRateError>`
rejecting `rate ≤ 0`) plus an infallible `FxRate::config(rate)` that stamps
`source = "config"`. `as_of`/`source` are provenance **labels** (strings), not
clock reads.

- **Home = `crates/core`** (the ADR-0058 § D2 precedent): `core` is the base
  crate every consumer depends on, it already carries `rust_decimal` + `smol_str`
  + `serde` (**zero new dependency**), and because `ui` already imports
  `trading_core`, consuming a new `core` type adds **no new `ui` edge** —
  `cargo tree -p ui` unchanged is a hard gate. Homing in `crates/data` (the
  future-fetcher home) is REJECTED — it would force a `ui → data` edge.
- The rate VALUE comes from config: the advisor config carries
  `eur_usd_rate: Option<Decimal>` (`#[serde(default)]`, additive) and the cockpit
  binary resolves `FxRate::config(cfg.eur_usd_rate.unwrap_or(DEFAULT_EUR_USD_RATE))`
  at the seam, where `pub const DEFAULT_EUR_USD_RATE: Decimal = dec!(1.08)` lives
  in `core::fx`.

### OQ-AR-2 — ONE conversion fn, ONE value, two readers (no drift)

One pure method — the ONLY EUR→USDT arithmetic in the codebase:

```rust
impl FxRate {
    pub fn convert_eur_to_usdt(&self, eur: Decimal) -> Money<Usdt> {
        Money::<Usdt>::from_decimal(eur * self.rate)
    }
}
```

A `BudgetConversion { eur: Decimal, rate: FxRate, usdt: Money<Usdt> }` carrier
(also `core::fx`) computes `usdt` **once** in its ctor. At
`cockpit_live.rs:1431-1437` the `Money::<Usdt>::from_decimal(budget_decimal)`
**1:1 stamp is replaced** by `BudgetConversion::new(budget_decimal, fx)`; the
ENGINE reads `conversion.usdt()` → `ForwardRunConfig.budget`, the DISPLAY reads
`conversion.usdt()` / `.eur()` / `.rate()`. One converted value, one conversion
call → **engine and display cannot drift** (the F6 / ADR-0062 anti-drift
discipline). **EUR-at-input = a labelled `Decimal`, NOT a `core::Eur` marker**
(rejected — a first-class EUR currency + ledger FX-PnL is over-scoped for one
input multiply); F4/F5/bake-off stay unit-agnostic and byte-unchanged.

### OQ-AR-3 — Determinism + anchor-safety (by construction)

A config constant is a deterministic input → reproducible sizing + render PNGs;
the `as_of`/`source` labels add no clock non-determinism. **The anchored
CLI/headless path never reads the rate** — bake-off, `run_scenario`, and the
sweep bins take `Money<Usdt>` directly; the EUR→USDT conversion lives ONLY at the
cockpit UI input boundary. `verify_anchors.sh` stays **119/119 by construction**
(no anchored scenario, no report body, no `anchors.toml` SHA). The future
live-rate option is a NOTED-not-built `RateSource` trait + a `crates/data`
fetcher behind a fake seam (every test/render injects a fixed rate, never the
network — the ADR-0061 mock precedent); it reuses `DEFAULT_EUR_USD_RATE` as its
fallback (a strict superset → zero rework; its own ADR).

### OQ-AR-4 — The day-1 conversion-applied gate (CLAUDE.md non-negotiable)

F7 modifies the budget the F4 sizing modifier consumes, so per the
`v3-volatility-forecaster-noop` precedent the gate is REQUIRED (not N/A). A
dedicated **`crates/core/tests/eur_fx_conversion_applied.rs`**, FAIL-before /
PASS-after against a 1:1 stub:

1. `FxRate::config(dec!(1.08)).convert_eur_to_usdt(dec!(200)) == dec!(216)` and is
   **strictly ≠ `dec!(200)`** (the no-op guard; `rate = 1.0` is the negative
   control — converted **==** 1:1, by design).
2. The converted value is the one F4 caps against: feed `conversion.usdt()` into
   `FixedFractionSizer::with_budget_cap(fraction, conversion.usdt())` and assert
   the effective cap reflects 216, not 200 (closes the "computed-then-dropped"
   hole — the rate is *applied*, not stamped 1:1).
3. Display ↔ engine agreement: the formatted display figure is byte-identical to
   the `Money<Usdt>` fed to F4 (same `BudgetConversion`).

The render-layer PNG (input panel + F6 plan showing "€200 ≈ $216 (at 1.08
EUR/USD)" with a `rate = 1.0` negative control rendering "€200 ≈ $200") is the
tester's pixel-floor per the CLAUDE.md iced render rule.

### Display

The three "FX not modelled" literals (`LEADERBOARD_BUDGET_HINT:2442`,
`FORWARD_PLAN_BUDGET_LINE:2716`, `LIVE_FORWARD_FX_NOTE:2828`) become the honest
"€X ≈ $Y (at R EUR/USD, source as-of)", driven by the SAME `BudgetConversion`;
the hard-cap framing and not-advice disclaimer are preserved verbatim.

### Task split

F7 is small + UI-centric — see [`tasks.md`](tasks.md). It ships as **one
developer** (no dev ‖ ui-designer split): the only UI-surface change is three
literal rewrites filled from a value the same developer threads through the seam,
so a parallel ui-designer would have nothing independent to build. The pixel
verification is the tester's render-layer floor, not a design task.

## Backtest Scenarios

**None — by design.** F7 introduces NO new anchored backtest scenario. It is a
budget-unit conversion at the input boundary + a display change; it runs no
strategy, produces no equity curve of its own, and reads no anchored corpus. The
bake-off RUNS the existing (anchored) strategies but F7 changes only the
downstream sizing scalar, not any anchored scenario. `verify_anchors.sh` stays
**119/119 by construction**.

**CLAUDE.md day-1 baseline-equity-divergence e2e gate — APPLICABLE, scoped.** F7
**modifies the budget that the F4 sizing modifier consumes** — it is adjacent to
a sizing modifier. The honest reading: F4 already carries the canonical
divergence e2e (`budget_sizing_divergence_end_to_end.rs`, the
budget-cap-vs-uncapped gate). F7's correctness claim is narrower and arithmetic —
*"a non-unit rate produces a different USDT budget than the 1:1 collapse, and that
different budget reaches F4."* The required gate is therefore an **e2e/unit
assertion that `convert(€200, rate=1.08) ≠ convert(€200, rate=1.0)` AND the
converted value is the one F4 sizes against** (i.e. the rate is *applied*, not
computed-and-dropped — the exact `v3-volatility-forecaster-noop-fix` failure mode
the non-negotiable exists to catch). The architect should decide whether this
rides as an extension of the F4 e2e or a dedicated `eur_fx_conversion_*` test;
the analyst flags it as **REQUIRED, not N/A** — a budget conversion that computes
a rate but stamps 1:1 anyway is precisely the silent no-op the gate guards.

## Implementation

Implemented 2026-06-22 (developer, one-developer feature per `tasks.md`).

### New files

- **`crates/core/src/fx.rs`** — all FX primitives:
  - `FxRate { rate: Decimal (private), source: SmolStr, as_of: SmolStr }` with checked `new()` + infallible `config()` (uses `debug_assert!`, no `.expect()`)
  - `convert_eur_to_usdt(&self, eur: Decimal) -> Money<Usdt>` — the ONLY EUR→USDT multiply in the codebase (`eur * self.rate`)
  - `BudgetConversion { eur, rate, usdt }` — computes `usdt` once in ctor; `fx_note()` extracts `FxNote` for display threading
  - `FxNote { eur, usdt, rate, source, as_of }` — lightweight display carrier (a `core` type, no new `ui` edge)
  - `DEFAULT_EUR_USD_RATE: Decimal = dec!(1.08)` — the config fallback constant
- **`crates/core/tests/eur_fx_conversion_applied.rs`** — 6 FAIL-FIRST gate tests (day-1 non-negotiable per CLAUDE.md):
  - Proves `rate=1.08 → €200 = $216 ≠ $200` (no-op guard)
  - Proves the converted value reaches `FixedFractionSizer::with_budget_cap` (cap=216, not 200)
  - Proves engine and display read the same `BudgetConversion` (no drift)
  - Written to FAIL against a 1:1 stub, PASS once `convert_eur_to_usdt` uses `self.rate`
- **`crates/ui/tests/eur_fx_budget_render.rs`** — 2 macOS pixel-layer render tests (T8):
  - `fx_budget_hint_with_108_rate_paints_form_foreground` — FORM band ≥2000 foreground px (PNG: `/tmp/eur_fx_budget_render.png`)
  - `fx_budget_hint_unit_rate_negative_control` — structural anti-tautology (both rates render a form)

### Modified files

- **`crates/core/src/lib.rs`** — `pub mod fx;` + `pub use fx::{BudgetConversion, DEFAULT_EUR_USD_RATE, FxNote, FxRate, FxRateError}`
- **`crates/core/Cargo.toml`** — `risk` dev-dependency for gate test
- **`crates/agent/src/config.rs`** — `AdvisorConfig { eur_usd_rate: Option<Decimal>, eur_usd_rate_as_of: Option<String> }` + `pub advisor: AdvisorConfig` on `Config`
- **`crates/ui/src/state.rs`** — `Message::ForwardPaperTradeStarted(Money<Usdt>, Option<FxNote>)`; `Cockpit.forward_fx: Option<FxNote>`; `Cockpit.advisor_eur_usd_rate: Decimal`
- **`crates/ui/src/bin/cockpit_live.rs`** — seam: `BudgetConversion::new(budget_eur, fx).usdt()` replaces 1:1 stamp; `FxNote` emitted alongside the budget; `AppState` carries `advisor_eur_usd_rate`
- **`crates/ui/src/strings.rs`** — three `_FMT` variants with `{eur}/{usdt}/{rate}/{source}` placeholders
- **`crates/ui/src/widgets/num.rs`** — `fmt_rate`, `fmt_usdt_plain`, `fmt_eur_plain`
- **`crates/ui/src/widgets/bakeoff_input.rs`** — `view()` extended with `eur_usd_rate: Decimal`; hint built from `BudgetConversion`
- **`crates/ui/src/screens/leaderboard.rs`** — passes `model.advisor_eur_usd_rate` to `bakeoff_input::view`
- **`crates/ui/src/gallery/routes.rs`** — passes `DEFAULT_EUR_USD_RATE` to `bakeoff_input::view`
- **`crates/ui/src/screens/forward_plan.rs`** — `FxNote` threaded through `result_body<'a>`/`ready_pane<'a>`/`sizing_block<'a>`
- **`crates/ui/src/screens/live.rs`** — `build_forward_pnl_block` with `fx_note: Option<&FxNote>`; honest label from note, fallback to `DEFAULT_EUR_USD_RATE`
- **`crates/ui/tests/live_forward_pnl_render.rs`** — updated `ForwardPaperTradeStarted(budget, None)` call site (7/7 PASS)
- **`crates/ui/src/fixtures.rs`** — `fake_cockpit_leaderboard_with_fx_rate` fixture

### Verification results

| Gate | Result |
|---|---|
| Day-1 conversion-applied gate (6 tests) | `test result: ok. 6 passed; 0 failed` |
| Live forward PnL render (7 tests) | `test result: ok. 7 passed; 0 failed` |
| FX budget render (2 macOS pixel tests) | `test result: ok. 2 passed; 0 failed; finished in 7.05s` |
| `cargo clippy --workspace --all-targets -- -D warnings` | CLEAN |
| `cargo tree -p ui` | UNCHANGED (no new edge) |
| `bash scripts/verify_anchors.sh` | 119/119 PASS |
| Grep guard (single EUR→USDT multiply) | ONE location: `crates/core/src/fx.rs::convert_eur_to_usdt` |

## Verification
_tester links to reports here. Expected floor: (1) the conversion-applied gate
(R4/§ Backtest Scenarios — converted ≠ 1:1 AND reaches F4); (2) a render-layer
PNG of the input panel + the F6 plan showing the honest "€200 ≈ $X (at R)" label
with a 1:1-rate negative control (per the CLAUDE.md iced pixel rule); (3)
`verify_anchors.sh` 119/119; (4) `cargo tree -p ui` unchanged (no new ui edge)._

## Open questions

### Operator product-fork (the one real decision) — must answer before architect locks

**OQ-OP-1 — The rate SOURCE.** This is the load-bearing fork; everything else
follows from it.

- **(a) Configurable static rate with a sane config default (Recommended —
  durable).** The rate is a config value (e.g. `advisor.eur_usd_rate = 1.08`,
  operator-editable), defaulting to a recent EUR/USD level, carried with a
  source/as-of label ("config, 2026-06-22"). **Deterministic** (a constant →
  reproducible sizing + reproducible render PNGs → tests + anchors unaffected),
  **zero new network surface** (no forex API, no DNS, no rate-limit, no outage
  mode), **zero new failure mode**, and **honest** ("€200 ≈ $216 at 1.08 EUR/USD,
  as configured"). For a **paper, single-operator, simulated-budget** tool whose
  ranking is FX-invariant and whose budget is a round €200, a configured rate is
  the *correct* fidelity — a live forex feed buys ~1% display precision at the
  cost of determinism, a network dependency, and a fallback state. This is the
  durable choice: it carries forward across versions without amendment, and it
  matches the existing "configured universe / config-driven" product posture.
  **Why Recommended (durable, not merely cheap):** a live feed would *force* a
  rate-source trait + a mock + a fallback-rate constant *anyway* (R4/R5) — i.e.
  option (b) is a strict superset that still needs (a)'s constant as its fallback.
  Shipping (a) first is not a carve-out that spawns rework; it is the stable core
  that (b) would extend if ever wanted. The architect can PROVE (b) cannot be
  done without (a)'s constant, so (a) is the load-bearing primitive regardless.

- **(b) Live-fetched rate from a forex API (fallback — defer to v0.3 if ever).**
  Honest "live as-of now", but: adds a **network dependency** (a new external I/O,
  a new fetcher, a new key/endpoint), **breaks determinism** (the rate moves every
  run → sizing + render PNGs are no longer reproducible unless mocked → R4 forces a
  rate-source trait + a fake-rate seam through every test/render), introduces a
  **failure mode** (timeout / stale / rate-limited → R5 fallback-to-default-rate +
  a "live FX unavailable" label), and is **overkill for a paper budget** (≈1%
  precision on a round €200 the user picked arbitrarily). Reasonable only if the
  operator specifically wants the displayed P/L to track real-time EUR. *This is a
  clean v0.3 upgrade on top of (a)* — (a)'s config value becomes (b)'s fallback —
  so choosing (a) now does not foreclose it.

- **(c) Derived from a EURUSD-like pair in the corpus (rejected — no such data).**
  Verified: the corpus is crypto-only (Binance klines/funding/premium + Yahoo).
  There is **no EURUSD or fiat-FX series** to derive from. Not an option without
  first ingesting forex data — which collapses into (b). Reject.

**Recommendation: (a) — configurable static rate with a config default + a
source/as-of label.** Durable, deterministic, zero new network/failure surface,
and the correct fidelity for a paper simulated budget; (b) is the clean v0.3
upgrade if the operator ever wants a live-tracking display, layered on (a)'s
value as its fallback. *If-budget-tightens:* (a) is already the cheapest path
**and** the durable one — no separate fallback needed (the rare case where the
durable choice is also the smallest; (b) only ADDS cost).

### Architect-bound (the conversion seam — lock in the F7 design after OQ-OP-1)

These are design questions, not product forks — the architect resolves them once
the operator answers OQ-OP-1.

- **OQ-AR-1 — Where the rate type + provenance lives, and how it reaches the
  boundary as a `core` type.** The conversion happens UI-side
  (`cockpit_live.rs:1431-1437`) but the rate's *home* must respect the layering
  (`ui` imports no `strategy`/`exec`/`forecast`/`llm`; and for option (b), a
  fetcher belongs in `crates/data`, not `ui`). Candidate: a small `core`-typed
  `FxRate { rate: Decimal, source: &str, as_of: ... }` (a value object, like the
  `ForwardRunConfig`/`ForwardPlan` `core`-only-fields mirror discipline) read by
  the cockpit binary at the seam. Confirm the home crate + that no new `ui` edge
  appears (`cargo tree -p ui` unchanged is a verification gate).

- **OQ-AR-2 — The exact arithmetic seam + display↔engine agreement.** Confirm the
  single multiply lands at `cockpit_live.rs:1431-1437` (between `budget_eur()` and
  `from_decimal`) AND the symmetric F6-plan + Live display path reads the **same**
  converted value the engine sizes against (R3 — the F6 anti-drift discipline:
  the "$X" the operator reads must equal what F4 caps at). One conversion
  function, one source of truth, two readers (engine + display).

- **OQ-AR-3 — The determinism / test-injection contract (R4).** For option (a)
  this is trivial (a config constant). For option (b), lock the rate-source trait
  + fake-rate seam so every test + render harness injects a fixed rate (the
  `crates/data` HttpKlineFetcher-vs-mock precedent) and never hits the network —
  and the anchored CLI path never reads a live rate. Either way, define the
  fixed-rate fixture the conversion-applied gate (§ Backtest Scenarios) uses.

- **OQ-AR-4 — The conversion-applied (no-op-guard) test shape.** Decide whether
  the R4 "converted ≠ 1:1 AND reaches F4" assertion extends the existing F4
  `budget_sizing_divergence_end_to_end.rs` or ships as a dedicated
  `eur_fx_conversion_applied` test. The analyst's constraint: it MUST prove the
  rate is *applied* to the value F4 consumes (not computed-then-dropped — the
  `v3-vol-overlay-noop` failure mode), per the CLAUDE.md non-negotiable.

## Changelog

- 2026-06-22 (architect, F7 design — ADR-0065): filled the `## Design` section +
  authored [ADR-0065](../../../_bmad-output/planning-artifacts/architecture/decisions/0065-eur-usdt-budget-conversion-seam.md)
  (registered in the ADR README + architecture.md § Changelog). Resolved
  OQ-AR-1..4 to the operator-LOCKED **configurable static rate**: (OQ-AR-1) a
  `FxRate {rate,source,as_of}` value object in a new `crates/core::fx` (the
  ADR-0058 home-in-`core` precedent — zero new dep, no new `ui` edge,
  `cargo tree -p ui` unchanged; rate VALUE from advisor config
  `.unwrap_or(DEFAULT_EUR_USD_RATE=dec!(1.08))`); (OQ-AR-2) ONE pure
  `convert_eur_to_usdt(eur:Decimal)->Money<Usdt>` + a `BudgetConversion{eur,rate,
  usdt}` carrier computed once so the engine (`ForwardRunConfig.budget`) and the
  display read the SAME value (the ADR-0062 anti-drift discipline), EUR kept a
  labelled `Decimal` at input — no `core::Eur` (F4 stays unit-agnostic +
  byte-unchanged), the seam REPLACES the 1:1 `from_decimal` stamp at
  `cockpit_live.rs:1431-1437`; (OQ-AR-3) deterministic + anchor-safe BY
  construction (the anchored CLI is USDT-denominated + never reads the rate →
  119/119), the v0.3 live `RateSource`+fake-seam NOTED-not-built; (OQ-AR-4) the
  DAY-1 conversion-applied gate `crates/core/tests/eur_fx_conversion_applied.rs`
  (converted≠1:1 AND it's the value F4 caps against AND display==engine;
  FAIL-before/PASS-after per the `v3-vol-overlay-noop` non-negotiable). The three
  "FX not modelled" literals → honest "€X ≈ $Y (at R EUR/USD, source as-of)".
  Task split = **one developer** (small + UI-centric; no independent ui-designer
  surface — the three literals are filled from a value the developer threads
  through the seam). Wrote [`tasks.md`](tasks.md). No code; no anchored content
  touched; `status: draft → in-progress`, owner analyst → architect.
- 2026-06-22 (analyst, F7 scoping — NEW feature): authored the F7 brief — apply a
  **real EUR/USD rate** at the single budget-conversion boundary so the simulated
  euro budget that flows into F4 sizing + the "€200 ≈ …" display is honest,
  resolving product § D4's deferred "fixed EUR→USD rate (v0.2 refinement)". Scoped
  the **honest reality**: verified against code (2026-06-22) that **no FX/forex
  source exists** (crypto-only fetchers; no `Eur` currency type — `crates/core/src/asset.rs`)
  so F7 needs a new tiny rate source, and pinned the **1:1-collapse seam** at
  `crates/ui/src/bin/cockpit_live.rs:1431-1437` (where `budget_eur()` → `Money<Usdt>::from_decimal`
  stamps euros as USDT 1:1 — F7 inserts one multiply there) + the three "FX not
  modelled" display literals (`LEADERBOARD_BUDGET_HINT`/`FORWARD_PLAN_BUDGET_LINE`/`LIVE_FORWARD_FX_NOTE`
  in `crates/ui/src/strings.rs`). Locked the **NON-goals** (NOT FX trading /
  prediction / a first-class `Eur` currency + ledger FX-PnL; ranking
  FX-invariant + untouched; paper-only; 119/119 anchors untouched). Surfaced the
  **one operator product-fork (OQ-OP-1, rate SOURCE: configurable static default
  [Recommended — durable, deterministic, zero new network/failure surface] vs
  live-fetched [v0.3 upgrade, breaks determinism + adds a network dep/failure
  mode] vs derived [rejected — no corpus FX series])** with the durable-AND-cheap
  recommendation, and split the **architect-bound seam questions** (OQ-AR-1 rate
  home + `core` type + no new `ui` edge; OQ-AR-2 the single multiply + display↔engine
  agreement; OQ-AR-3 determinism/test-injection; OQ-AR-4 the conversion-applied
  no-op-guard test per the CLAUDE.md `v3-vol-overlay-noop` non-negotiable). Flagged
  the **day-1 equity-divergence gate as APPLICABLE-not-N/A** (F7 modifies the
  budget the F4 sizing modifier consumes → a "converted ≠ 1:1 AND reaches F4" gate
  is REQUIRED). No engine code; no anchored content touched; trace row
  `REQ-ADVISOR-EUR-FX-001`.
