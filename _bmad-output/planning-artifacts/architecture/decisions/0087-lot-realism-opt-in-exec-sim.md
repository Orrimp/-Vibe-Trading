---
adr: 0087
title: Lot-size rounding + min-notional reject as an opt-in exec-sim mode (default unchanged)
status: accepted
date: 2026-07-10
supersedes: none
superseded-by: none
---

# ADR-0087: Lot-size rounding + min-notional reject as an opt-in exec-sim mode (default unchanged)

## Context

The product advertises a **€200 retail** scale (`spec/product.md`), but the
paper/sim fill path fills **any fractional quantity at any notional**. Real
spot venues enforce two `exchangeInfo` filters that the sim ignores:

- **`LOT_SIZE`** — quantity must be a multiple of a per-symbol `stepSize`
  (e.g. `0.00001 BTC`, `1 DOGE`); the venue floors the order to the step.
- **`NOTIONAL` / `MIN_NOTIONAL`** — order notional (`qty · price`) must be
  ≥ a per-symbol floor (~5–10 USDT on Binance spot); below it, the venue
  **rejects** the order.

At €200 with `fixed_fraction(0.1)` sizing (~€20 clips) the min-notional floor
is comfortably cleared, so this is **not an alpha-changing correction** — it is
a **realism/honesty gap**: (a) lot rounding shaves a few sats off every clip
(a low-price coin like DOGE rounds to whole units — coarse relative to €20),
and (b) a small-order reject path is entirely unmodeled, so the sim would
happily "fill" a €3 order that a real venue bounces.

**Grounding (code, at authoring):**
- The venue-filter vocabulary **already exists**: `data::SymbolInfo { min_qty,
  lot_size, min_notional }` (`crates/data/src/source.rs:8`), populated by the
  **live** `exchange_info()` fetch in `binance.rs:221` / `coinbase.rs:369` /
  `kraken.rs:368`. That path is a live REST call — unusable for the deterministic
  sim (no live calls; CLAUDE.md). We therefore mirror the **shape**, not the fetch.
- The fill chokepoint is `PaperEngine::step` (`crates/backtest/src/paper.rs:67`):
  **every** order → `Fill` for **both** the bake-off (`engine.rs:2501`, every
  `scenarios/*` runner) **and** the forward paper loop
  (`crates/agent/src/runtime.rs:2291`) funnels through this one `.step(&bar, orders)`.
- Quantity is born in `risk::FixedFractionSizer::compute_qty`
  (`crates/risk/src/sizing.rs:51`), which already composes a **budget-cap clamp
  as a `min` after the exposure-cap clamp** (F4, ADR precedent for "compose a
  Decimal-exact clamp into the sizer"). But the **Sell/close** leg does **not**
  go through the sizer (it uses `position.base_qty` directly at
  `engine.rs`/`runtime.rs`), so the sizer is **not** a both-legs chokepoint.

**Precedent (binding):** ADR-0081 (`SlippageModel::VolScaledSpread`, P1-6) —
a new **opt-in-forever** exec-sim mode added beside the default, with the default
byte-unchanged so the 119 anchored report body-SHAs hold **by construction**,
plus the CLAUDE.md **day-1 baseline-equity-divergence e2e** non-negotiable
(this IS a sizing-modifier — the overlay/modifier gate APPLIES).

## Decision

Add lot-size rounding + min-notional reject as a **new opt-in-forever exec-sim
mode**, applied at the `PaperEngine::step` seam, **OFF by default**, gated by a
new `#[serde(default)]` `Option<VenueFilterMode>` config field. **The default
(field absent / `None`) is byte-identical to today.**

### D1 — Seam: `PaperEngine::step` (the both-paths chokepoint)

Rounding + reject live **inside `PaperEngine::step`**, applied to each `order`
**before** the `Fill` is constructed:

1. Round `order.qty()` **down** to the symbol's `step_size` (Decimal-exact).
2. Compute post-round notional `= rounded_qty · fill_price`.
3. If `rounded_qty == 0` **or** `notional < min_notional` → **skip** this order
   (push **no** `Fill`; record a skip event — D4). Otherwise construct the
   `Fill` with `qty = rounded_qty`.

**Why this seam and not the sizer** (the two lessons that pick it):

- **F5b parity lesson** (the forward loop is a *separate site*): the bake-off and
  the forward loop are two independent call sites, but **both** call
  `engine.step(&bar, orders)`. Placing the rule in `step` means it is honored on
  BOTH paths from one edit — impossible to wire one and forget the other. The
  sizer (`compute_qty`) is bypassed by the Sell/close leg on both paths, so it
  cannot be the both-legs home.
- **v3-vol-overlay-noop lesson** (compute-but-never-apply is the failure mode):
  `step` is the sole place the `qty` that lands in `Fill` is finalized, and
  **every downstream cash/position update reads `fill.qty.get()`**
  (`engine.rs:2504–2521`, `runtime.rs:2298–2333` — both verified). Rewriting
  `fill.qty` therefore *provably* changes deployed capital and equity — there is
  no second copy of the quantity to fall out of sync.

A skipped order (no `Fill` pushed) is naturally absorbed by **both** callers,
which already loop `for fill in &fills { … }` under `if let Ok(fills) =
engine.step(…)` — a shorter `fills` vec is a valid, already-handled state. **A
min-notional skip is NOT a `MatchError`** (that enum is reserved for genuine fill
failures — `FillError`/`NoLiquidity`); the order is *dropped*, not *errored*.

### D2 — Config surface (mirrors ADR-0081 opt-in)

Extend `LatencySlippageSimConfig` (`crates/backtest/src/cli_types.rs:58` — the
existing exec-sim config that already houses `slippage_model`) with:

```rust
/// Opt-in venue-filter realism (ADR-0087). `None` (the serde default) =
/// no rounding, no reject — byte-identical to the pre-ADR-0087 fill path.
#[serde(default)]
pub venue_filter: Option<VenueFilterMode>,
```

`VenueFilterMode` is a small enum so future variants (e.g. a maker-rebate mode)
stay additive:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VenueFilterMode {
    /// Round qty down to step_size + reject sub-min-notional orders,
    /// using the checked-in static filter table (D3).
    LotSizeAndMinNotional,
}
```

`PaperEngine` gains an `Option<VenueFilterTable>` handle (constructed from the
mode by the runner/forward-loop wiring). `MatchConfig` is a `paper.rs`-local
struct; the table handle rides alongside it on the engine (not on the anchored
serde config), so no anchored serde surface changes. **`MatchConfig::default()`
and `LatencySlippageSimConfig::default()` are UNCHANGED**; `venue_filter`
defaults to `None`.

### D3 — Static filter table (checked-in constant; stated staleness)

The filter data is a **checked-in static table**, NOT a live `exchangeInfo`
fetch (no live calls — CLAUDE.md; determinism). It **reuses the existing
`data::SymbolInfo` shape** (`min_qty`, `lot_size`/`step_size`, `min_notional`)
rather than inventing a parallel vocabulary:

- New module `crates/cost/src/venue_filter.rs` (the `cost` crate is the exec-sim
  friction home, already owns `slippage.rs`; it does **not** depend on `data`,
  so we carry a tiny local `VenueFilter { step_size, min_notional }` record
  mirroring `SymbolInfo`'s three fields — no new dep edge).
- `pub fn venue_filter_for(symbol: &Symbol) -> Option<VenueFilter>` returns the
  snapshot for the **10 Binance USDT pairs** (the advisor corpus) **+ Coinbase
  `BTC-USD`** (the P2 second venue). Unknown symbols → `None` → the mode is a
  **no-op for that symbol** (never a panic, never a silent wrong number).
- Values are a **dated snapshot** (`SNAPSHOT_DATE` const in the module doc) of
  what the live fetch returns. **Staleness is a stated limit**: venues revise
  `stepSize`/`minNotional` occasionally; the table is a point-in-time capture,
  documented as such in the module header and the feature file. A refresh is a
  one-line table edit under this ADR (no re-emission owed — see D6). This is NOT
  a look-ahead concern (a filter is a static venue rule, not a time series), so
  ADR-0086's `PitSeries` machinery does not apply.

**Decimal discipline (ADR-0003):** `step_size` and `min_notional` are `Decimal`
literals; rounding is `(qty / step).floor() * step` computed **entirely in
`Decimal`** (`rust_decimal::Decimal::floor`) — **never `f64`**. Round-down only:
`floor`, never `round`/`ceil` — the user must never over-spend their budget.

### D4 — Reject/skip audit semantics (two homes, grounded)

A rejected order must leave an **auditable** trace. Grounding shows the advisor
bake-off and forward loop **do not write to the `audit::Ledger`** — they keep
cash/equity in **in-memory** `state`/`cash` (`engine.rs`, `runtime.rs`; the
ledger is threaded through the *live* agent runtime only, and live trading is
out of scope). So the skip record has two homes, by path:

1. **Primary (advisor sim path) — an in-memory tally surfaced in the result.**
   `PaperEngine::step` returns fills as today; the count of skipped orders is
   accumulated on the engine (`skipped_min_notional: u64`) and exposed via a
   `PaperEngine::sim_filter_stats()` accessor. The scenario/forward runner reads
   it post-loop and folds it into the run summary (and, where a report is
   emitted, a **report-body annex line** — but the advisor bake-off runs
   `write_report=false`, so **no anchored body moves**; D6). This is the
   determinism-safe, always-available record — analogous to how
   `SimulatedExecMetrics` already summarizes sim friction.

2. **Live-agent path (wiring reserved) — `AuditEvent::StrategyEvent`.** When/if
   the rule runs under the live agent (which *does* own a `Ledger`), the skip is
   recorded via the existing `strategy_events` table using the `StrategyEventWrite`
   mechanism (`crates/audit/src/journal.rs:1623`) with
   `kind = "min_notional_skip"` — the **same** pattern as the existing
   `rebalance_rejected` event (`journal.rs:1722`). This reuses the shipped
   6-digit-fractional-second audit-DB timestamp discipline (ADR-0004) and needs
   **no** new `AuditEvent` variant. **This wiring is specified but built only
   when a live-agent caller exists** — no live path ships here.

**€200 golden scenario, documented:** at €200 budget · `fixed_fraction(0.1)` ·
BTC/ETH majors, `notional ≈ €20 ≫ min_notional (~5–10 USDT)`, so **zero orders
are rejected**; lot rounding on BTC (`step 0.00001`) shaves < 1 basis point off
each clip. The mode is **honest-but-quiet** at the advertised scale — which is
exactly the point: it *proves* the golden path clears the filters, and it *bites*
only when the user picks a coarse-lot coin (DOGE) at a small budget (see D5).

### D5 — Day-1 baseline-equity-divergence e2e (CLAUDE.md non-negotiable)

This is a **sizing-modifier** (it changes deployed `qty`), so the overlay/modifier
gate **APPLIES**. Ship, from day 1, an end-to-end test asserting the opt-in
mode's terminal equity **diverges ≥ 1 bp** from the un-rounded baseline on a
corpus where rounding provably bites:

- **Corpus/config that makes rounding bite:** a **low-price coin — `DOGEUSDT`**
  (price ~€0.10–0.40, `step_size = 1` whole DOGE) at a **small budget (€50–200)**
  with `fixed_fraction(0.1)` → each clip is ~€5–20 = tens of DOGE, and flooring
  to whole DOGE discards a **material fraction of the last unit** on every trade.
  Over a multi-trade run the discarded-fraction + any sub-min-notional skips
  compound into a terminal-equity gap **≥ 1 bp** vs the un-targeted (un-rounded)
  baseline.
- **Test shape** (mirrors `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`):
  run the SAME strategy + bars twice — once `venue_filter = None` (baseline),
  once `venue_filter = Some(LotSizeAndMinNotional)` — assert
  `|equity_filtered − equity_baseline| / equity_baseline ≥ 1e-4`, AND assert the
  direction (filtered ≤ baseline — rounding down + rejects can only *reduce or
  hold* deployed capital, never increase it). A **negative control** on a
  high-price major at €200 asserts divergence ≈ 0 (filters don't bite → the mode
  is correctly inert there). New file:
  `crates/backtest/tests/lot_realism_divergence_end_to_end.rs`.

This test is the guard against the v3-vol-overlay-noop failure (a mode that
computes a rounded qty but never applies it would show **zero** divergence and
fail this test on day 1).

### D6 — Opt-in-forever contract (BINDING) → anchors 119/119 by construction

The 119 anchored body-SHAs (`spec/anchors.toml`) are byte-immutable
(ADR-0038 § D6). The lot-realism mode is **never reachable from any anchored CLI
path**:

- The anchored CLI (`param_robustness_sweep` et al.) constructs
  `MatchConfig` / `LatencySlippageSimConfig` via `default()` → `venue_filter =
  None` → the rounding/reject branch is **not taken** → `fill.qty` is
  byte-identical to today.
- The advisor bake-off runs `write_report=false` → **no anchor SHA is produced**
  even when the mode is enabled.
- The only callers that enable `LotSizeAndMinNotional` are opt-in,
  operator-configured (or the D5 e2e test).

**Enforcement (never delete):**
- `venue_filter_default_is_none` — asserts `LatencySlippageSimConfig::default()
  .venue_filter.is_none()` (mirrors ADR-0081's `default_is_linear_bps_8`).
- `paper_step_none_is_byte_identical` — asserts `PaperEngine::step` with
  `venue_filter = None` produces a `Fill` with `qty == order.qty()` unchanged
  (the byte-identity proof obligation: **default run ≡ pre-change run**).
- `bash scripts/verify_anchors.sh` → **119/119 before AND after** (the mechanical
  gate; this holds by construction because no anchored path takes the branch).

## Alternatives considered

- **Round in the sizer (`FixedFractionSizer::compute_qty`) — rejected.** Natural
  (the F4 budget clamp lives there), but the **Sell/close leg bypasses the sizer**
  on both the bake-off and forward paths (`engine.rs`/`runtime.rs` build the
  close order straight from `position.base_qty`), so the sizer is not a both-legs
  chokepoint. Rounding only the buy leg would leave dust on closes and violate the
  "one apply site" lesson. `PaperEngine::step` sees **every** order.
- **Live `exchangeInfo` fetch for the filter table — rejected.** Violates the
  no-live-calls constraint and breaks determinism (venues revise filters; a run's
  result would depend on wall-clock fetch timing). The static snapshot (D3) is the
  deterministic, offline-reproducible choice; staleness is a documented limit and
  the shipped `SymbolInfo`/`exchange_info()` path already exists for anyone who
  later wants a live refresher tool.
- **Default-on (bump the default to enabled) — rejected.** Would re-emit all 119
  anchored reports under ADR-0038 § D6b for a change that is ≈ 0 impact at the
  advertised €200/major scale (D4 golden scenario). Opt-in-forever, per the
  ADR-0081 precedent; revisit a default bump only if the product's advertised
  minimum budget/coin set changes such that rejects become common on the golden
  path.
- **New `AuditEvent::MinNotionalSkip` variant — rejected.** The existing
  `StrategyEvent` / `strategy_events` table already carries the `rebalance_rejected`
  precedent; a `kind="min_notional_skip"` string reuses it with zero schema churn
  (D4). Adding a first-class variant would touch the audit `tick.rs` enum for a
  live-only path that does not ship here.
- **Reject via `MatchError` — rejected.** A sub-min-notional order is a *normal*
  venue outcome (skip the order), not a *fault*; surfacing it as `Err(MatchError)`
  would abort the fill loop and conflate it with genuine fill failures. Dropping
  the order from the returned `fills` vec is the correct, already-handled semantics.

## Consequences

- `PaperEngine` gains an optional filter handle + a `skipped_min_notional`
  counter; `MatchConfig`/`LatencySlippageSimConfig` `Default` and serde bytes are
  **unchanged** (new field is `#[serde(default)] Option<…>` = `None`).
- Any code that constructs a `Fill` by assuming `fill.qty == order.qty()` is only
  affected when the mode is **explicitly enabled**; the default path is
  byte-identical (enforced by `paper_step_none_is_byte_identical`).
- The filter table is a maintenance surface: a venue revising `stepSize`/
  `minNotional` requires a one-line table edit + a `SNAPSHOT_DATE` bump under
  this ADR — **no anchor re-emission** (the table is off every anchored path).
- If this ADR is violated (default silently starts rounding), the two
  enforcement unit tests fail **and** `verify_anchors.sh` breaks — a loud,
  mechanical tripwire.
- The €200 golden path is now *provably* filter-clean (D4) and the small-budget /
  coarse-lot corner is now *provably* modeled (D5) — the honesty gap is closed
  without moving a single anchored byte.

## Changelog

- 2026-07-10 (architect): initial accept; remediation-plan P4, feature
  `advisor-lot-realism`. Numbering: took 0087 — 0086 was the last registered ADR;
  the sibling P5 (`advisor-handoff-export`) is a wording/export change and claims
  no ADR number (verified free on disk + in README at authoring).
