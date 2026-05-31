---
slug: cross-sectional-mean-reversion-strategy
version: 0.1.0
status: dev-done
owner: developer → tester
priority: P2
updated: 2026-05-31
---

# Cross-sectional mean-reversion — the first PIVOT family through the robustness harness — v0.1.0

> **The PIVOT after FAMILY-UNIFORM-FRAGILE.** v1 cross-sectional momentum is
> retired on the robustness axis — fragile across both its history (C2: p50
> Sharpe ≈ −0.01, P(loss) 75.2%, P(Sharpe>1)=0) AND its whole parameter family
> (C3: all 6 θ-cells FRAGILE). The killer was **turnover / fee-bleed**: on the
> SAME resampled 2023 histories a passive equal-weight buy-and-hold of the same
> 10 coins earned **+1.74 Sharpe** (P(loss) 4.5%), so the drift was right there
> to capture — momentum specifically converted it into a loss machine through
> ~5,343 trades/yr of churn
> ([adversarial-review § 2](../dev-notes/robustness-verdict-adversarial-review-2026-05-30.md);
> [closure deck](../momentum-parameter-robustness-sweep/presentations/momentum-robustness-closure-2026-05-30.md)).
>
> This brief recommends **cross-sectional mean-reversion** as the first family
> to vet next, and frames the data-availability decision that gates the
> alternatives. **It is reversible DESIGN work** — the operator confirms before
> any code is written. It exists to make the pivot a decision-grade choice, not
> to pre-commit to building.

---

## 0. Pre-registration & anti-cherry-pick (inherited, frozen now)

This family is vetted under the **already-frozen** pre-registered decision rule
([`robustness-decision-rule-2026-05-30.md`](../dev-notes/robustness-decision-rule-2026-05-30.md) § 0)
— the same ruler that scored momentum. Nothing about the rule is re-opened for
this family. Three commitments carry over verbatim and are restated here so the
build inherits them from day 1:

1. **The bands are frozen.** p5 Sharpe ≥ +0.5 ROBUST / < 0 FRAGILE; prob-of-loss
   ≤ 15% ROBUST / > 35% FRAGILE; p95 MaxDD ≤ ~50% ROBUST / > ~70% FRAGILE; p50
   Sharpe ≥ 1.0 ROBUST; P(Sharpe>1) ≥ 60% ROBUST. Composite = **worst primary
   band wins** (weakest-link). MR is scored against these, not the reverse.
2. **Anti-cherry-pick by construction.** The C3 θ-sweep for MR reports the FULL
   surface + a family verdict and **crowns no argmax winner** (FP-C3.5 enforces
   this in code). A non-FRAGILE cell carries a `→ C5 deflation required` flag —
   a 6-cell grid that picked argmax would inflate the false-ROBUST rate.
3. **Pre-flight void-if-fail.** Every MR report must print
   `generator: block-bootstrap-real` AND `bootstrap_mode: shared-index`, else
   the verdict is void (the tail is not a fair adversary otherwise).

**The buy-and-hold control (+1.74 Sharpe) is the bar MR must clear to matter.**
A family that does not beat simply holding the same coins net of fees on this
universe is not worth promoting, however internally "robust." That is the
sharpest scientific question the pivot exists to answer
([closure deck § neutral framing](../momentum-parameter-robustness-sweep/presentations/momentum-robustness-closure-2026-05-30.md)).

---

## Why

### Why a pivot at all, and why THIS family first

The momentum arc produced a complete, trusted two-axis robustness machine
(C1 bootstrap generator + C2 path-robustness + C3 parameter-robustness) and a
frozen decision rule. The machine is family-agnostic; the only open question is
**which family to put through it next**. Four candidates were teed up:
mean-reversion, carry, breakout, cross-sectional value. They were weighed
against **three axes**, with **data availability assessed first** because it can
be decisive.

### Axis 1 — DATA AVAILABILITY (the load-bearing finding, grounded in code)

The harness real-data path is `crates/backtest/src/realdata.rs`
(`RealDataBarSource`), which loads the revision-pinned parquets under
`data/binance/` via `data::ReplayFeed::merge_symbols` and produces a
`Vec<trading_core::Bar>`. **What that path can deliver per family was determined
by inspecting the loader, the `Bar` struct, and the parquet column footer — not
by assumption:**

| Field source | What's on disk | Verdict |
|---|---|---|
| **Binance OHLCV parquet** (`data/binance/<SYM>/<YEAR>/<MM>.parquet`) | columns: `open_time, open, high, low, close, volume, trade_count, close_time` (read directly from the parquet footer); `Bar` struct = OHLCV + counts + venue, **no funding / basis / fundamental field** | **price-only** |
| Universe | 10 USDT pairs (`BTCUSDT ETHUSDT BNBUSDT SOLUSDT XRPUSDT ADAUSDT DOGEUSDT AVAXUSDT` + 2), revision SHA `3a8b96c4…`, **both 2023-FY and 2024-FY present** (12 months each) | runnable in-sample (2023) + an out-of-history second year (2024) |
| **Funding rate** | type `FundingObs` + table `funding_rates` (migration 003) + a **live forward-only poller** `data::funding::FundingPoller` hitting `fapi.binance.com/fapi/v1/premiumIndex` hourly into SQLite — but **no historical funding parquet on disk**, and the harness reads parquet bars, not the SQLite ledger | **infra half-built; NO historical backfill** |
| **On-chain value proxy** (NVT, MVRV, realized-cap) | **nothing** — no type, no table, no fetcher, no data; grep for `nvt/mvrv/realized_cap/on-chain` returns zero non-comment hits | **absent end-to-end** |

**Per-family data verdict:**

- **Mean-reversion → price-only → RUNNABLE NOW** on the exact existing
  data/engine. No new ingestion.
- **Breakout → price-only → RUNNABLE NOW.** (Range high/low over a window from
  the same OHLCV.)
- **Carry → needs historical funding-rate series.** We have the *live* poller
  and the *schema*, but **not a historical funding backfill in the parquet
  format the harness consumes.** Acquisition path exists and is well-precedented
  (see § Carry acquisition path below) but is **net-new work before a single
  carry backtest can run.**
- **Cross-sectional value → needs an on-chain value proxy that does not exist
  anywhere in the repo.** Acquisition is a larger, vendor-dependent lift
  (third-party API: Glassnode / CoinMetrics / Santiment; daily granularity at
  best). **Furthest from runnable.**

### Axis 2 — the TURNOVER-KILLER lesson

Momentum died on turnover. Ranked by independence-from-trend and turnover
profile:

| Family | Return source | Turnover profile | Relation to the killer |
|---|---|---|---|
| **Carry** | funding/basis — **non-price, most independent** of momentum | low (funding settles 3×/day; rebalance can be slow) | best *structural* dodge — but data-gated |
| **Cross-sectional value** | fundamentals — non-trend | **lowest** (value moves slowly; weekly/monthly rebalance) | good dodge — but no data |
| **Mean-reversion** | counter-trend price | **turnover-heavy by default** (fast signals flip often) — BUT a **slow-rebalanced** variant dodges most of the fee-bleed | *shares* the killer unless explicitly slowed; this brief mitigates it head-on |
| **Breakout** | trend-adjacent price | moderate–high | trend-adjacent → inherits the buy-and-hold-dominates risk that just killed momentum |

The honest tension: the two families that best dodge the turnover-killer (carry,
value) are exactly the two we lack data for; the two that are runnable-now
(mean-reversion, breakout) are both price-based and at risk of the same
fee-bleed. **The resolution is to pick the runnable-now family that can be
explicitly engineered for LOW turnover** — and that is mean-reversion with a
slow rebalance + wide no-trade band, NOT breakout (which is trend-adjacent and
shares momentum's losing prior).

### Axis 3 — robustness prospects + definability in crypto

- **Mean-reversion is the natural counter-hypothesis.** If trend-following is a
  cost-bleed machine on 1h crypto, the inverse is the single most informative
  next test. It is cleanly definable on our universe: it **reuses the exact
  cross-sectional ranking, inverted** — `crates/strategy/src/cross_sectional/`
  already has `score_vol_adjusted_return` and a `top_k_long` selector that
  stable-sorts **descending**; cross-sectional MR is the **bottom-K** (buy the
  biggest recent losers, expecting reversion). The plumbing is ~90% reuse.
- **Robust-edge plausibility:** short-horizon reversal is one of the most
  documented effects in crypto (overreaction / liquidation-cascade snapback).
  Whether it survives the harness net of fees is precisely the open question —
  and unlike momentum, the prior is not already-known-negative.
- **Definability caveat:** MR's edge tends to live at *short* horizons where
  turnover is highest, which is exactly where fees bit momentum. The g=3
  momentum result (1-month lookback + wide hold-band) showed the low-churn lever
  *materially* helps the cost signals (P(loss) 18.5%, the only near-ROBUST
  cell). The MR θ-grid must therefore deliberately span the turnover axis to
  find whether *any* MR cell escapes the fee trap — the same hypothesis-aimed
  design that made C3 conclusive.

### The recommendation

**Pick: cross-sectional mean-reversion, runnable NOW, with turnover treated as a
first-class design axis.** Runner-up / fast-follow: **carry**, the moment a
historical funding backfill lands (it is the best *structural* answer to the
turnover-killer and the funding infra is already half-built).

This is the **durable** choice, not merely the cheap one: it answers the
load-bearing scientific question the whole pivot exists to ask — *"is there ANY
active family that beats buy-and-hold net of fees on this universe?"* — with the
**most direct counter-hypothesis to the strategy we just retired**, on data we
already trust, while the data-gated families are unblocked in parallel. It does
not spawn a follow-on cleanup brief: the C2/C3 harness, the decision rule, and
the cross-sectional plumbing are all already built, so the marginal cost is the
inverted signal + two harness runs.

**If budget tightens** (fallback, NOT the recommendation): the cheapest possible
scope is a *single-config* MR C2 path-robustness pass only (skip the C3
θ-surface), deferring the parameter axis to a v0.2.0. This violates the spirit
of the day-1-both-axes gate (§ Day-1 robustness gate) and should be a conscious
operator downgrade, not a default — a single config cannot answer "is the
*family* robust," only "is *this cell* robust," which is exactly the trap C3 was
built to avoid.

---

## Requirements

### R-MR.1 — Signal (cross-sectional reversal)

The MR score is the **negation of the v1 momentum score**, reusing the existing,
anchored `features::cross_sectional::score_vol_adjusted_return`:

```text
score_mr(s, t) = − [ ln(close[t] / close[t−n]) / realized_vol(close[s], n) ]
```

i.e. the symbol with the **most negative** vol-adjusted recent return ranks
highest (buy the biggest recent losers). Concretely this is the existing
`top_k_long` selector applied to **negated scores** (or an ascending sort) —
**bottom-K of the momentum ranking**. No new feature-math crate is needed; the
inversion is a one-line sign flip at the ranking boundary. _Acceptance: an
e2e divergence test asserts the MR equity curve diverges from the v1-momentum
equity curve on the same 2023 path by ≥ 1 bp (they are sign-inverted selections;
identical curves would prove the inversion is a no-op — the CLAUDE.md
overlay/modifier divergence-gate discipline applied to a sibling family)._

### R-MR.2 — Sizing

Reuse v1's long-only sizing exactly so the comparison is apples-to-apples:
equal-weight across the K selected names, `notional = equity × fraction` per leg
with the **solvency guard already fixed in `montecarlo::run_path`** (Bug B fix,
v0.1.1 — cash can never go negative; buys skip when cash < notional + fee). No
new sizing code. Long-only v0.1.0 (no shorting the winners) to keep the first
pivot a clean single-variable change from momentum. _Acceptance: reuses the
production `run_path` sizing path verbatim; no new sizing module._

### R-MR.3 — Rebalance cadence (turnover is a FIRST-CLASS axis — the whole point)

The default MR config MUST be a **slow rebalance with a wide no-trade band** to
attack the fee-bleed that killed momentum, mirroring the g=3 low-churn corner
that was momentum's only near-ROBUST cell:

- a **drift / no-trade band** (reuse the C3 `drift_rebalance_threshold` lever):
  only rebalance a leg when its weight drifts past the band, so small rank
  wiggles do not trigger trades;
- the **C3 θ-grid for MR spans the turnover axis explicitly** — short vs long
  lookback × narrow vs wide band × K — so the sweep can find whether *any* MR
  cell escapes the fee trap, not just the default.

_Acceptance: the MR C3 grid includes at least one deliberately-low-turnover cell
(long lookback + wide band) AND one deliberately-high-turnover cell (short
lookback + narrow band); the per-cell report prints trades/yr so the
turnover-vs-Sharpe relationship is legible._

### R-MR.4 — Universe & data

Same 10 USDT pairs, same revision-pinned `data/binance/` parquets, same hourly
bars, same 6 bps round-trip fees as momentum. **No new ingestion.** In-sample =
2023-FY (matches the momentum anchors for direct comparability); an
out-of-history check on 2024-FY is available (data present) and recommended as a
secondary read. _Acceptance: the MR scenario loads via the unchanged
`RealDataBarSource`; revision SHA `3a8b96c4…` resolves; ≥ 99.5% bar coverage._

### R-MR.5 — Day-1 robustness gate (BOTH axes — non-negotiable, the entire point)

Per the closure deck's lasting payoff, every future strategy is vetted on
**both** robustness axes **from day one**. MR ships with:

- **C2 path-robustness:** one N=500 shared-index block-bootstrap pass over
  2023-FY real returns → one anchored distribution-summary report
  (Sharpe p5/p50/p95, prob-of-loss, P(Sharpe>1), p95 MaxDD), scored against the
  frozen bands.
- **C3 parameter-robustness:** one θ-surface sweep (a hypothesis-aimed grid,
  ~6 cells × N=200 to fit the ~20-min compute budget) → one anchored θ-surface
  + family verdict (FAMILY-UNIFORM-FRAGILE / mixed / FAMILY-ROBUST), no argmax
  winner crowned.
- **Buy-and-hold control** re-asserted on the same paths (the +1.74 reference)
  so the MR verdict is read *relative to passively holding*.

_Acceptance: two new anchored reports (one C2-style, one C3 θ-surface) under a
new MR namespace; both print the void-if-fail pre-flight header; the family
verdict line is one of the allowed values; the buy-and-hold control matches the
+1.74 reference to sampling noise._

### R-MR.6 — Determinism & anchoring (inherited)

Byte-identical on the Apple-Silicon canonical box (ADR-0051 § D5 / § D6.1
SAME-paths seeding); two-run byte-identity unit-proven; reports body-SHA-locked
into the regression gate as new additive anchors (ADR-0038 § D6 additive
contract — no existing anchor disturbed). _Acceptance: `verify_anchors.sh`
passes at N+2 anchors with all prior anchors byte-identical._

---

## Design

> **M-T1 BINDING DESIGN (architect, 2026-05-31).** Q1 is RESOLVED in § D-MR.0
> (the analyst's shared-ranking-path lean is RATIFIED, and the architecture
> makes it near-forced — see the `run_path` signature finding). The signal
> inversion is specified exactly in § D-MR.1 (negate at the score-cache
> boundary — ONE line, the anchored `top_k_long` + `score_vol_adjusted_return`
> reused verbatim). The MR θ-grid is LOCKED in § D-MR.2-LOCKED (it IS the
> anchor input — frozen before the tester anchors). The day-1 BOTH-axes gate +
> the R-MR.1 divergence falsifier are specified in § D-MR.5-GATE. The
> determinism/anchoring contract reuses ADR-0051 § D6 with a one-paragraph
> § D6.5 cross-reference amendment (§ D-MR.6). This is design-only — reversible
> until the dev build per the operator's pivot delegation.

### D-MR.0 — Q1 RESOLVED: `Direction { Momentum, Reversion }` on the shared config (NOT a separate struct)

**Decision: add a `direction: Direction` field to `CrossSectionalMomentumConfig`
(default `Momentum`); the existing `MomentumStrategy` consumes it at the single
score-cache boundary. NO separate `CrossSectionalMeanReversionStrategy` struct.**
The analyst's lean is ratified, and the architecture makes it the only
low-blast-radius choice — this is a forced move, not a preference:

**The load-bearing constraint (verified at HEAD).**
`crates/backtest/src/scenarios/montecarlo.rs::run_path` (line 79) is typed
`mut strategy: strategy::MomentumStrategy` — a **concrete type, not a trait
object**. The C2 driver (`monte_carlo.rs:876`), the C3 driver
(`param_robustness_sweep.rs:1077`), and the C3 e2e gate all call it with a
concrete `MomentumStrategy`. A *separate* MR struct would force `run_path` to
become generic (`run_path<S: Strategy>`) or take `Box<dyn Strategy>` — which
**touches the C2-anchored `run_path` and risks all 86 anchors** (R-MR.6
forbids this). Keeping MR as a `MomentumStrategy` instance whose selection is
inverted keeps `run_path`'s signature **byte-identical** → the 86 anchors hold
by construction, exactly as the C3 D6.1 SAME-paths argument keeps them by
construction.

| Criterion | `Direction` on config (CHOSEN) | Separate MR struct (REJECTED) |
|---|---|---|
| `run_path` signature | **unchanged** (`MomentumStrategy`) — 86 anchors hold by construction | must go generic / `dyn` → touches C2-anchored code → anchor risk |
| Inversion provably 1 flip | **yes** — `direction` gates one `if` at the score-cache write; R-MR.1 divergence test is meaningful | two code paths → plumbing can silently diverge (the failure R-MR.1 guards against) |
| Ranking path shared | **yes** — `top_k_long` + `score_vol_adjusted_return` reused verbatim, anchored, untouched | risk of a forked selector drifting from the momentum one |
| Sweep-bin reuse | **yes** — `cell_config` + `run_one_path_with_config` gain one `direction` arg; everything else verbatim | new constructor + new injection seam |
| Config-loader blast | one new optional field + one enum + one validation line | a whole new `kind` + loader + schema |

The durable choice (per the analyst's framing) is the one where the two
families **cannot silently diverge in plumbing** — that is `Direction` on the
shared config. The MR θ-grid is therefore a **sibling `const`** next to
`TIER1_GRID` (NOT a generalised direction-keyed grid — that would over-couple
the two families' anchor inputs); see § D-MR.2-LOCKED.

**Naming note (no rename churn).** The struct stays `MomentumStrategy` and the
config stays `CrossSectionalMomentumConfig` at v0.1.0 — renaming to
`CrossSectional*` would ripple across the registry, the C2/C3 drivers, and the
anchored config hash for zero functional gain. The `kind` field stays
`cross_sectional_momentum` (a `direction` field disambiguates the family). A
cosmetic rename is a deferred, anchor-neutral follow-on if ever wanted. This is
the same "don't touch the anchored driver for a cosmetic win" reasoning C3 used
to reject refactoring `run_one_path`.

### D-MR.1 — The signal inversion (EXACT spec — negate at the score-cache boundary)

MR = the **negation of the v1 vol-adjusted-return score**. The inversion point
is **ONE line** in `MomentumStrategy::on_bar`
(`crates/strategy/src/cross_sectional/momentum.rs:198-201`), where the freshly
computed score is written into `self.scores`:

```rust
// momentum.rs on_bar, after `score_vol_adjusted_return(...)`:
let score = self.histories.get(&bar.symbol).and_then(|rb| {
    score_vol_adjusted_return(rb, self.lookback_minutes, self.vol_floor).ok()
});
// D-MR.1: invert at the cache boundary. Momentum stores +score; Reversion stores −score.
let score = match self.direction {
    Direction::Momentum  => score,
    Direction::Reversion => score.map(|s| -s),   // bottom-K losers float to the top
};
self.scores.insert(bar.symbol.clone(), score);
```

Why this exact point, and why it is provably a no-op-free single-variable change:

- **`top_k_long` is reused VERBATIM** (it stable-sorts **descending** and takes
  the first K — `selector.rs:51,55`). Negating the score makes the
  most-negative-return symbol (the biggest recent loser) have the **largest**
  negated score, so the unchanged descending top-K selects the **bottom-K of the
  momentum ranking** — exactly R-MR.1's "buy the biggest recent losers." No
  `bottom_k_long`, no second sort path, no selector edit. (A `bottom_k_long`
  variant was considered and REJECTED: it duplicates the anchored sort logic and
  creates two code paths to keep in sync — the opposite of the
  cannot-silently-diverge goal.)
- **`score_vol_adjusted_return` is reused VERBATIM** (anchored feature math,
  `crates/features/src/cross_sectional.rs:49`; pure `Decimal`). The negation is
  applied to its `Decimal` output, NOT inside it — the feature crate is
  untouched, so its tests + any anchors that depend on it are unaffected.
- **Tie-break stays alphabetical and deterministic.** `top_k_long`'s stable sort
  over BTreeMap-ordered entries preserves the alphabetical tie-break on equal
  (negated) scores — identical determinism to momentum.
- **Sizing, rebalance, drift, solvency-guard: all unchanged.** The only behavioral
  difference between a `Momentum` and a `Reversion` strategy at the same θ is
  *which* K symbols are selected. This is what makes the R-MR.1 divergence test
  meaningful (§ D-MR.5-GATE): identical equity curves would prove the sign flip
  never took effect.

`Direction` enum (new, in `cross_sectional/config.rs`):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    #[default] Momentum,   // top-K winners (v1 behavior — the serde default)
    Reversion,             // bottom-K losers (cross-sectional MR)
}
```
- **Naming collision note (developer):** `core::forecast::Direction { Up, Down,
  Flat }` already exists (a *forecast* direction, unrelated). The new
  `cross_sectional::Direction { Momentum, Reversion }` is a distinct type in the
  `strategy::cross_sectional` namespace — do NOT import or unify with the forecast
  one. The `cross_sectional` module imports no `Direction` symbol today, so there
  is no ambiguity as long as the new enum stays module-local (verified at HEAD).
- Config field: `#[serde(default)] pub direction: Direction` → **every existing
  momentum TOML and every existing struct literal deserializes/compiles to
  `Momentum` unchanged** (no anchor disturbance, no existing-test breakage). The
  config hash (`momentum.rs:227 compute_config_hash`) MUST append
  `;direction={direction:?}` so a Momentum-vs-Reversion config at the same θ
  produces a **different** strategy hash (K3 discipline — the family is a hashed
  input). NB: this changes the *strategy* config hash, NOT any report body-SHA;
  the 86 report anchors are unaffected (they hash report bodies, not the
  in-memory config hash). Validation: `Direction` is a closed enum, no new error
  code needed.

### D-MR.2-LOCKED — The MR θ-grid (6 cells × N=200, turnover-axis-spanning) — ANCHOR INPUT, FROZEN

> **LOCKED 2026-05-31 (architect M-T1).** This exact 6-cell list is the hashed
> body field for the MR θ-surface anchor (ADR-0051 § D6.3, inherited). Changing
> it = a different surface = a different SHA. It mirrors the C3 6-cell tractable
> envelope (the proven ~20 min @ N=200 shape — C3 measured 1217 s for 6×200) and
> is **deliberately aimed at the turnover axis**, because the C3 evidence is that
> turnover is the lever that moved the only near-robust momentum cell (g=3:
> 1mo × wide band → P(loss) 18.5%, the sole sub-coin-flip cell).
>
> **Held constant across every cell (identical to C3 except `direction`):**
> `direction = Reversion`, `rebalance_minutes = 60`, `exposure_cap = 0.50`,
> `vol_floor = 0.000001`, `size = equal_weight`, `k_short = 0`, the 10-symbol
> universe, year = 2023, N = 200, `ensemble_seed = 0xC0FFEE`,
> `fill_seed = 0xC0FFEE`, generator = `block-bootstrap-real`, revision
> `3a8b96c4…`.

The swept axes are `lookback_minutes` (signal horizon), `k_long` (selection
breadth), and `drift_rebalance_threshold` (the no-trade hold band = the turnover
/ exit lever). The grid **spans turnover** — from a deliberately-high-churn
corner (short lookback + narrow band) to a deliberately-low-churn corner (long
lookback + wide band) — so the surface answers "does *any* MR cell escape the fee
trap," the R-MR.3 acceptance question.

| g | lookback_minutes | k_long | drift_rebalance_threshold | role / hypothesis | turnover |
|---|---|---|---|---|---|
| 0 | 60 | 3 | 0.10 | **baseline MR θ\*** (mirror of the C2/C3 shipped θ, direction-flipped; the apples-to-apples MR-vs-momentum cell) | mid |
| 1 | 24 | 3 | 0.10 | **short lookback + narrow band — deliberately HIGH churn** (R-MR.3 high-turnover cell; the classic short-horizon MR where fees bit momentum hardest) | **high** |
| 2 | 168 | 3 | 0.10 | 1w lookback horizon | mid |
| 3 | 720 | 5 | 0.50 | **1mo lookback + wide band — deliberately LOW churn** (R-MR.3 low-turnover cell; the best a-priori robustness shot, mirrors C3 g=3's escape corner) | **low** |
| 4 | 720 | 3 | 0.30 | long lookback + medium band (low-churn diagonal, narrower selection) | low–mid |
| 5 | 24 | 5 | 0.10 | short lookback + wide selection — **the maximal-churn extreme** (most legs flipping fastest; confirms the fee trap if MR shares it) | **highest** |

**Design notes on the lock:**
- **R-MR.3 acceptance is satisfied by construction:** g=1 and g=5 are the
  deliberately-high-turnover cells; g=3 and g=4 are the deliberately-low-turnover
  cells. The per-cell report prints trades/yr-equivalent (the surface carries the
  total trade count per cell — already plumbed via `IndexedPathMetrics.trades` →
  `CellResult.total_trades`), so the turnover-vs-Sharpe relationship is legible.
  **Add a `trades` column to the MR θ-surface table** (the C3 renderer logs
  total_trades but does not table it — the MR renderer surfaces it, because
  turnover legibility is R-MR.3's whole point; this is an additive renderer
  change, body-SHA-isolated to the new MR namespace).
- **g=0 is a free correctness probe** of the inversion plumbing: it is the C2/C3
  baseline θ with `direction = Reversion`. It MUST diverge from the C3 g=0
  momentum cell (the R-MR.1 family-divergence assertion at the surface level — see
  § D-MR.5-GATE); if g=0 MR reproduces the C3 g=0 momentum numbers, the inversion
  is a no-op and the surface is void. (Contrast the C3 g=0, which had to *match*
  the C2 anchor; the MR g=0 must *differ* from the momentum g=0 — the inversion is
  the variable.)
- **Bounded + hypothesis-aimed, NOT a dense cube.** 6 cells = baseline + 2
  high-churn + 2 low-churn + 1 mid, aimed at the turnover axis the C3 evidence
  localized. This is the C3 tractability lesson (14×500 intractable → 6×200)
  applied verbatim; the budget is spent on the corners the prior says matter.
- **Wall-clock budget (validated against C3):** C3 measured **1217.1 s (~20 min)**
  for 6 cells × N=200 + the buy-and-hold control on the canonical box. The MR
  sweep is the same shape (6×200 + control) → **~20 min expected**. The dev MUST
  validate the budget before locking (the C3 § Implementation lesson:
  wall-clock ≈ grid × N × per-path cost). N=200 is the locked per-cell N; N stays
  a hashed body field.
- **`rebalance_minutes` held at 60.** Co-moves with lookback for turnover; a full
  cross would explode the grid (C3 precedent). A Tier-2 refine MAY add it iff
  Tier-1 surfaces a non-FRAGILE low-churn MR cell.

### D-MR.3 — Harness reuse map (what is reused verbatim vs genuinely new)

The reuse story is **even stronger than C3's ~85%** because MR reuses the entire
C3 sweep machinery (not just the C2 harness) — C3 already built the outer
θ-loop, the config-injection seam, the verdict classifier, and the θ-surface
renderer. MR's marginal new code is the `direction` field + the inversion line +
a sibling grid const + threading `direction` through two existing functions.

| Component | Status | Source |
|---|---|---|
| `run_path` (per-path engine loop) | **REUSE VERBATIM — do NOT touch (anchor-load-bearing)** | `montecarlo.rs:76` |
| `DistributionSummary` + `compute_*` + `PathMetrics` | **REUSE VERBATIM** | `backtest::stats` |
| C1 `BlockBootstrapPathGen` (shared-index, auto-L) | **REUSE VERBATIM** | `data::synth` |
| `top_k_long` selector (descending top-K) | **REUSE VERBATIM** (negated score → bottom-K) | `selector.rs:25` |
| `score_vol_adjusted_return` (feature math) | **REUSE VERBATIM** (negate its `Decimal` output) | `features::cross_sectional:49` |
| Per-path D1 seeding (`derive_path_seed`) | **REUSE VERBATIM** (SAME-paths, ADR-0051 D6.1) | `param_robustness_sweep.rs:409` |
| `ParamRobustnessVerdict` classifier (5-signal weakest-link) | **REUSE VERBATIM** (frozen § 0 bands) | `param_robustness_sweep.rs:138` |
| θ-surface renderer (FM/body split, sort-by-g) | **REUSE** (+ one additive `trades` column + MR labels) | `param_robustness_sweep.rs:638` |
| Buy-and-hold control row | **REUSE VERBATIM** (same passive equal-weight; the +1.74 bar) | `param_robustness_sweep.rs:506` |
| `run_one_path_with_config` glue | **REUSE** (+ one `direction` arg threaded to `cell_config`) | `param_robustness_sweep.rs:1007` |
| **`Direction` enum + config field + config-hash append** | **NEW (small)** | `cross_sectional/config.rs` + `momentum.rs:227` |
| **Score-inversion line in `on_bar`** | **NEW (1 line + 1 field on the struct)** | `momentum.rs:198-201` |
| **`MR_TIER1_GRID` sibling const (6 cells, § D-MR.2-LOCKED)** | **NEW (data only)** | MR sweep bin |
| **R-MR.1 family-divergence e2e test** | **NEW** | MR e2e test file |

**The build seam — the cleanest realization (T-MR-A1):** the MR sweep is run by
the **same `param_robustness_sweep` bin** with a new `--direction reversion`
flag + a `--grid mr-tier1` enum value selecting `MR_TIER1_GRID`, OR a dedicated
`bin/mr_robustness_sweep.rs` sibling. **Architect recommends the FLAG-on-the-
existing-bin path** (`--direction {momentum,reversion}` default `momentum`):
it threads `direction` into `cell_config` (one arg) and `run_one_path_with_config`
(one arg, passed to `MomentumStrategy::from_config` via the config), reuses the
entire renderer/classifier/control verbatim, and keeps a single sweep driver to
maintain. The momentum θ-surface anchor (#86) is **unaffected** because the
default `--direction momentum` + `--grid tier1` reproduces the exact existing run
(the new flag defaults preserve the existing code path — verify via the two-run
identity gate). The developer MAY instead fork a thin `mr_robustness_sweep.rs` if
they judge the flag-threading touches too much of the C3-anchored bin; either is
acceptable provided the momentum #86 anchor stays byte-identical (the binding
constraint, R-MR.6). _Recommendation: flag-on-existing-bin — one driver, less
drift._

### D-MR.4 — L is family-independent and printed once (inherits D6.1.4)

The auto-selected block length L is computed by Politis–White on the **source
series'** universe-average |log-return| (`data::synth::bootstrap`), which depends
only on the resampled real data — NOT on the strategy direction. So across the
entire MR θ-grid (and identically to the momentum grid at the same year/seed/
revision) **L is constant**, and the MR sweep at year=2023 sees the **same L** as
the C3 momentum sweep. This is a property of SAME-paths (ADR-0051 D6.1.4): the MR
family is varied at the strategy/config level, the source paths are byte-identical
to C3's, so L is identical. Printed once in the surface header (a shared input),
never per-row.

### D-MR.5-GATE — The day-1 BOTH-axes gate + the R-MR.1 divergence falsifier (NON-NEGOTIABLE)

Per the closure-deck payoff (every future strategy vetted on BOTH robustness
axes from day 1) and CLAUDE.md's sibling-family divergence discipline, MR ships
with the full C2+C3+control gate AND the R-MR.1 family-divergence falsifier.

**The two MANDATORY day-1 gates (CLAUDE.md non-negotiable):**

1. **R-MR.1 — MR-vs-momentum divergence (the headline anti-no-op; the CLAUDE.md
   sibling-family analogue of the overlay divergence gate).** A fast e2e test
   (small N, short synthetic bar series, NO real data — about wiring, not tail
   numbers) runs the **same path** through a `Momentum` strategy and a `Reversion`
   strategy at the **same θ** and asserts the two equity curves **diverge by
   ≥ 1 bp** (or, the robust signal: the **selected symbol sets differ** on ≥ 1
   rebalance, equivalently `|final_equity_mr − final_equity_mom| ≥ ε`). This is
   the exact CLAUDE.md pattern of
   `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`, adapted to a
   sibling family.
   - **(a) real case — PASS when wired:** Momentum and Reversion selections differ
     → curves diverge ≥ ε. (On a 2-symbol or 3-symbol synthetic universe with
     distinct trends, momentum and reversion pick opposite names → guaranteed
     divergence when K < universe size.)
   - **(b) degenerate case — MUST go RED (the falsification dry-run, FP-C2.1
     discipline):** force the inversion line to a no-op (e.g. `Reversion => score`
     — drop the negation) and assert the divergence check now FAILS (curves
     identical, Δ < ε). This proves the gate **detects** an inversion no-op — the
     sign flip is provably load-bearing, not decorative. **Both (a) and (b) ship
     in the test file.** This is the falsifier the whole pivot's integrity rests
     on: it goes RED if MR is silently running momentum.
   - **Test-design note for the developer:** the C3 e2e `make_config`
     (`param_sweep_e2e.rs:59`) uses a struct literal — adding the `direction`
     field means updating that literal (set `direction: Direction::Momentum` to
     preserve existing behavior). Choose a synthetic universe with K strictly less
     than the symbol count so momentum's top-K and reversion's bottom-K are
     **disjoint** (the cleanest divergence). The strategy must reach at least one
     rebalance after warmup for the selection to differ.

2. **Two-run byte-identity of the MR θ-surface body-SHA (FP-MR.3 / ADR-0051
   D2/D3/§ D6.4):** run the whole small-N MR sweep twice at the same
   `ensemble_seed`; assert identical `report_body_hash`. Catches any unordered
   fold sneaking into the outer θ-loop or the MR renderer.

**Required for ship (not the day-1 blocker pair, but gating the anchored run):**

3. **C2 path-robustness (the MR-C2 scenario):** the single best-a-priori MR config
   (g=3: 1mo × wide band) over N=500 shared-index block-bootstrap of 2023-FY real
   returns → scored against the frozen bands. *(Optional at v0.1.0: the MR-C3
   θ-surface already runs g=3 at N=200; a standalone N=500 MR-C2 pass on g=3 is the
   higher-confidence tail read and is the analyst's "single-config C2" floor. The
   architect's recommendation: ship the MR-C3 6-cell surface as the primary BOTH-
   axes deliverable; the MR-C2 N=500 single-config pass is a fast-follow if the
   surface shows a non-FRAGILE g=3 worth a tighter tail estimate. The day-1 gate is
   the surface + the divergence falsifier.)*
4. **C3 parameter-robustness (the MR-C3 θ-surface):** the LOCKED 6-cell grid
   (§ D-MR.2-LOCKED) at N=200 → ONE anchored θ-surface + family verdict
   (`FAMILY-UNIFORM-FRAGILE` / `FAMILY-HAS-NON-FRAGILE-CELLS`), NO argmax winner
   crowned (the C3 anti-cherry-pick renderer, reused verbatim — FP-C3.5 analogue).
5. **Buy-and-hold control row** on the same N paths + auto-L (re-asserts the +1.74
   reference; the C3 control reused verbatim). **This is the bar MR must clear to
   matter:** the family verdict is read *relative to* the +1.74 buy-and-hold
   Sharpe. A FAMILY-UNIFORM-FRAGILE MR that does not beat passive holding is the
   methodology-win-and-rotate-to-carry outcome the brief anticipates.
6. **Pre-flight void-if-fail:** both report headers print
   `generator: block-bootstrap-real` AND `bootstrap_mode: shared-index` (inherited
   C2/C3 discipline; the verdict is void otherwise).
7. **Anti-cherry-pick (FP-C3.5 analogue):** the family-summary line is one of the
   two allowed values; every non-FRAGILE cell carries `→ C5 DEFLATION REQUIRED`.
   Reused verbatim from the C3 renderer.

Pattern references: `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`
(CLAUDE.md single-variable divergence non-negotiable),
`crates/backtest/tests/param_sweep_e2e.rs` (the C3 θ-surface two-run + divergence
gate this mirrors).

### D-MR.6 — Determinism + anchoring (reuse ADR-0051 § D6; one-paragraph § D6.5 cross-ref amendment)

MR adds **NO new determinism mechanism.** The MR family axis is varied at the
**strategy/config level** (a `direction` field), NOT at the seed level — exactly
as the C3 θ-axis is varied at the config level under D6.1. Therefore:

- **The SAME-paths seed rule (D6.1) holds verbatim:**
  `path_seed_{g,j} = ensemble_seed.wrapping_add(j * 0x9E37_79B9)` for all cells —
  byte-identical to the C2/C3 D1 seed. The MR family is the *empty change* on the
  seed axis (the strongest form of "extends D1 without changing determinism"): the
  86 existing anchors hold by construction because no seed arithmetic changes.
- **Anchor unit = ONE MR θ-surface report (D6.3 extended):** +1 anchor. **New
  namespace decision:** anchor under the **existing `mc-robustness-2026-06`
  namespace** (the MR θ-surface is the same report *shape* as the momentum one,
  same lane, same anchor mechanism — a new namespace would fragment the lane for
  no determinism gain). Scenario name:
  `v1-mr-theta-surface-2023-block-bootstrap-real-fy` (mirrors the momentum
  `v1-momentum-theta-surface-…` name with `mr` substituted). Anchor count
  86 → 87. The MR grid def + N + `direction` + buy-hold flag are hashed body
  fields (K3). `scripts/verify_anchors.sh` already searches
  `spec/momentum-parameter-robustness-sweep/reports/` for the
  `mc-robustness-2026-06` namespace; the MR report lives under
  `spec/cross-sectional-mean-reversion-strategy/reports/`, so the verify script's
  namespace handler MUST be extended to also search that dir (a one-line additive
  change the tester/dev makes — the same pattern C3 used when it added its reports
  dir to the handler).
- **D2/D3/D5 inherited verbatim:** per-cell reduction unchanged (index-order
  mean/two-pass-std + `total_cmp` + type-7 linear pct + NaN-absent); FM/body split;
  `{:.6}` ratios / `{:.2}%` drawdowns; rows sorted by g before render;
  Apple-Silicon canonical-box scope.

**ADR action (D-MR.6 → ADR-0051):** write a short **§ D6.5 amendment** stating
that the strategy-family axis (Momentum vs Reversion) is the second instance of
the "vary at config level, seed untouched ⇒ determinism unchanged by
construction" pattern D6.1 established for the θ-axis — so the MR θ-surface
inherits D6.1/D6.3/D6.4 with no new seed mechanism, and the MR surface anchors
under `mc-robustness-2026-06` (86→87). This is a **cross-reference amendment, not
a new ADR** (the seed idiom, FM/body split, fixed-precision, and one-report
anchor unit are all reused verbatim — the architect's `analyst-defaults` cheap-
and-correct exception, identical to the C3 § D6 amendment rationale). Registered
atomically in the ADR registry README per the architect.md ADR-registry contract.
**No anchor in `spec/anchors.toml` is added by the architect** (the tester locks
the +1 MR θ-surface anchor after the dev's anchored run; the grid+N are locked
HERE, before the tester anchors — R-MR.6).

---

## Backtest Scenarios
_architect-ratified (M-T1, 2026-05-31). Primary anchored deliverable = the MR-C3
θ-surface; MR-C2 is an optional higher-confidence tail read; OOS is secondary._

1. **MR-C3 (PRIMARY, ANCHORED)** — `v1-mr-theta-surface-2023-block-bootstrap-real-fy`:
   the LOCKED 6-cell θ-grid (§ D-MR.2-LOCKED) spanning the turnover axis (R-MR.3),
   N=200/cell, shared-index block-bootstrap of 2023-FY real Binance, 6 bps fees
   (2 slippage + 4 taker, inherited verbatim from the C3 `TcnScenarioInput`).
   ONE anchored θ-surface report under `mc-robustness-2026-06` (86→87). Per-cell
   FRAGILE/MARGINAL/ROBUST + family verdict + per-cell `→ C5` flags + the trades
   column (R-MR.3 turnover legibility). ~20 min wall-clock (C3-measured shape).
2. **Control (in the MR-C3 surface)** — buy-and-hold equal-weight under the same N
   paths + auto-L bootstrap (re-asserts the **+1.74 Sharpe bar MR must clear**).
   This row carries no verdict; the MR family verdict is read relative to it.
3. **MR-C2 (OPTIONAL fast-follow)** — `v1-mr-2023-block-bootstrap-real-fy-mc`: the
   single best-a-priori MR config (g=3: 1mo lookback + wide band, k_long 5),
   N=500 shared-index block-bootstrap, 2023-FY. The tighter-tail single-config
   path-robustness read. Ship only if the MR-C3 surface shows a non-FRAGILE g=3
   worth the higher-confidence estimate; +1 anchor if run. (The day-1 BOTH-axes
   gate is satisfied by the MR-C3 surface, which already runs g=3 at N=200 — the
   "both axes" are C2 path-robustness *inside each cell* + C3 parameter-robustness
   *across cells*; the C3 surface delivers both.)
4. **OOS read (SECONDARY, non-gating)** — the same g=3 MR config on 2024-FY (data
   present on disk) as out-of-history corroboration. SEPARATE run = SEPARATE
   anchor (identical shape), NOT a primary gate. v0.2.0 fast-follow, mirrors the
   C3 +2024 pattern.

---

## Implementation

**Developer (2026-05-31):** Code complete + anchored θ-surface run complete.

### MR Family Verdict: FAMILY-UNIFORM-FRAGILE

**The headline scientific finding:** Cross-sectional mean-reversion is structurally
fragile across the entire LOCKED 6-cell Tier-1 grid (lookback × k_long ×
drift_rebalance_threshold). Zero cells cleared even the FRAGILE/MARGINAL boundary.
The MR family does NOT beat the +1.74 buy-and-hold Sharpe benchmark net of fees
on the 2023 real Binance universe.

**Key surface numbers (6×200, block-bootstrap-real, 2023-FY):**

| g | role | p50 Sharpe | P(loss) | p95 MaxDD | trades | verdict |
|---|------|-----------|---------|-----------|--------|---------|
| 0 | baseline MR θ* (60-min lookback) | -0.013 | 70.5% | 91.7% | 1,347,505 | FRAGILE |
| 1 | high-churn (24-min lb) | -0.017 | 84.0% | 91.6% | 1,963,647 | FRAGILE |
| 2 | 1w lookback | -0.005 | 57.5% | 90.5% | 799,300 | FRAGILE |
| 3 | LOW-CHURN: 1mo × wide band (best shot) | +0.007 | 42.5% | 85.5% | 379,809 | FRAGILE |
| 4 | long lb + medium band | -0.004 | 56.0% | 88.8% | 329,321 | FRAGILE |
| 5 | maximal-churn (24-min + k=5) | -0.010 | 72.0% | 93.2% | 2,082,234 | FRAGILE |

Buy-and-hold control: p50 Sharpe = **+1.74**, P(loss) = 4.5%, p95 MaxDD = 51.2%.

**g=3 (1mo lookback + wide band) is the nearest cell** at p50=+0.007 (barely above zero)
and the lowest P(loss) at 42.5% — confirming the C3 momentum lesson that slow
rebalance helps, but even the best-a-priori low-churn MR cell is still decisively
FRAGILE. No cell approached the MARGINAL threshold (p5 Sharpe ≥ 0 required).

**R-MR.1 surface-level inversion confirmed:** g=0 MR p50=-0.013 vs g=0 momentum
p50=-0.008. Surfaces differ — the inversion is provably not a no-op at the surface level
(in addition to the day-1 e2e falsifier `r_mr_1a_momentum_vs_reversion_diverge`).

**R2 binding-constraint verified:** `--direction momentum` reproduces momentum anchor
#86 SHA `0dd989d9dc6f81a8...` byte-identical. The `--direction` flag did not leak
into the momentum path.

### Implementation notes

- `Direction` enum + `direction` field on `CrossSectionalMomentumConfig`:
  `crates/strategy/src/cross_sectional/config.rs`
- Score inversion (1 line at the cache-write boundary):
  `crates/strategy/src/cross_sectional/momentum.rs` (on_bar, after `score_vol_adjusted_return`)
- `MR_TIER1_GRID` sibling const + `--direction` / `--grid mr-tier1` flags:
  `crates/backtest/src/bin/param_robustness_sweep.rs`
- R-MR.1 divergence falsifier (both (a) and (b)):
  `crates/backtest/tests/mr_divergence_e2e.rs` (5/5 PASS)
- `verify_anchors.sh` MR directory handler: already present (lines 146-149)
- `cargo fmt` applied: clean
- `cargo clippy -D warnings`: clean

### Anchored deliverables

- MR θ-surface report (#87): `spec/cross-sectional-mean-reversion-strategy/reports/robustness-sweep-20260531-153647-v1-mr-theta-surface-2023-block-bootstrap-real-fy.md`
- Body SHA: `a708112e1e8ddd4e48360b1e9f83d927c65d3d0f05be984e362c76f20be7bb4a`
- `verify_anchors.sh`: 87/87 PASS (all 86 prior anchors byte-identical)
- Wall-clock: 3087.3s (concurrent with R2 verify run → ~2× expected; single run expected ~1800s)

Lesson carried from C3: **validate the compute budget before locking the grid** — wall-clock ≈ grid × N × per-path cost; the C3 14-cell × N=500 design was intractable (~1 h) and was re-scoped to 6 × 200 (~20 min). The MR C3 grid inherits the 6 × N=200 envelope. Running TWO concurrent sweeps on the same machine approximately doubled each sweep's wall-clock (both competed for ~12 cores).

---

## Verification
_tester links to reports here_

Day-1 falsification probes (mirroring the momentum FP-C3.x family):

- **MR divergence gate** (R-MR.1) — MR equity ≠ momentum equity on the same path
  by ≥ 1 bp (proves the inversion is not a no-op; the CLAUDE.md sibling-family
  analogue of the overlay divergence gate).
- **θ-injection real** (FP-C3.1 analogue) — different MR params → different
  results; mutation twin collapses to |Δtrades|=0 under forced-same-config.
- **Anti-cherry-pick** (FP-C3.5 analogue) — family summary ∈ allowed values; any
  non-FRAGILE cell carries `→ C5 deflation required`.
- **Pre-flight** — `generator: block-bootstrap-real` AND
  `bootstrap_mode: shared-index` in both report headers.
- **Two-run byte-identity** + **anchor additivity** (no prior anchor disturbed).

---

## Carry acquisition path (the fast-follow, documented now so it is not lost)

If/when the operator wants the runner-up unblocked, the carry data path is
**well-precedented** and partly pre-built — this is the Yahoo-recipe analogue:

1. **Historical funding backfill tool.** Mirror `fetch_binance_klines` (which
   downloads klines → per-symbol-month parquet) with a `fetch_binance_funding`
   sibling hitting Binance's **historical** funding endpoint
   (`GET /fapi/v1/fundingRate?symbol=…&startTime=…&endTime=…`, paginated;
   funding settles 3×/day so volume is tiny vs hourly klines). Write
   `data/binance-funding/<SYM>/<YEAR>.parquet` with columns
   `funding_time, funding_rate`, REVISION.toml-pinned exactly like the OHLCV
   data. The live `BinanceFundingClient` already parses the sibling
   `premiumIndex` JSON, so the parse logic is mostly reusable.
2. **Harness bar enrichment.** Carry needs the funding series aligned to bars;
   either extend `Bar` with an optional funding field (blast-radius across the
   `Bar` struct — architect call) or carry a parallel funding lookup keyed by
   `(symbol, ts)` injected alongside `bars_override` in `run_path`. The latter is
   lower blast-radius and preferred for a first carry spike.
3. **Cost:** estimated a **small spike** (~1–2 days) for the backfill tool +
   revision pin, then carry is a first-class harness family. This is the clean
   fast-follow once MR's verdict is in.

**This brief does NOT commit the carry work** — it documents the path so the
runner-up is a known, scoped option rather than a vague pointer.

---

## Scope & honesty (no overclaim)

- This brief recommends a family and frames the data decision; it commits no
  code and triggers no engine run. Reversible per the orchestrator's scoping.
- MR is **price-based and therefore at structural risk of the same fee-bleed
  that killed momentum.** The mitigation (slow rebalance + wide band + a
  turnover-spanning θ-grid) is a *hypothesis*, not a guarantee. It is entirely
  possible the harness returns FAMILY-UNIFORM-FRAGILE for MR too — in which case
  that is *also* a methodology win (the machine ruled out a second family cheaply)
  and the pivot rotates to carry.
- The robustness axis judges **resampled real 2023 history** only — it cannot
  speak to regimes 2023 never contained. (Inherited scope limit.)
- No alpha is claimed from synthetic data; this is uncertainty quantification of
  a candidate strategy, not prediction (inherited framing).

---

## Changelog

- 2026-05-31 (architect, M-T1): **arch-done.** Resolved Q1 (§ D-MR.0): RATIFIED
  the analyst's shared-ranking-path lean as a **`Direction { Momentum, Reversion }`
  field on `CrossSectionalMomentumConfig`** (NOT a separate struct) — and showed
  it is near-FORCED: `montecarlo::run_path` (line 79) takes a **concrete
  `MomentumStrategy`**, so a separate MR struct would force `run_path` generic/
  `dyn` and risk all 86 anchors; keeping MR a direction-flipped `MomentumStrategy`
  keeps `run_path` byte-identical → anchors hold by construction. Specified the
  EXACT signal inversion (§ D-MR.1): **ONE line at the score-cache boundary
  (`momentum.rs:198-201`) negating the `Decimal` output of the anchored
  `score_vol_adjusted_return`** — the unchanged descending `top_k_long` then
  selects the **bottom-K losers**; `score_vol_adjusted_return` + `top_k_long`
  reused VERBATIM (no `bottom_k_long`, no feature-crate edit). LOCKED the MR
  θ-grid (§ D-MR.2-LOCKED): **6 cells × N=200**, deliberately spanning the
  turnover axis (g=1/g=5 high-churn, g=3/g=4 low-churn, g=0 baseline MR θ\*),
  mirroring the C3 tractable 6×200 envelope (~20 min C3-measured) — this IS the
  anchor input, frozen. Reuse map (§ D-MR.3): MR reuses the **entire C3 sweep
  machinery** (outer θ-loop, config-injection, verdict classifier, θ-surface
  renderer, buy-and-hold control) verbatim; new = the `Direction` enum + the
  1-line inversion + the `MR_TIER1_GRID` sibling const + a `--direction` flag on
  the existing `param_robustness_sweep` bin (recommended) threading `direction`
  through `cell_config`/`run_one_path_with_config`. Specified the day-1 BOTH-axes
  gate + the **R-MR.1 family-divergence falsifier** (§ D-MR.5-GATE): the headline
  anti-no-op asserts MR ≠ momentum equity on the same path by ≥ 1 bp, tested on
  BOTH the real case (PASS) and the degenerate inversion-no-op (RED-on-revert) —
  the CLAUDE.md sibling-family analogue of the overlay divergence gate; plus
  two-run byte-identity; plus the reused C3 anti-cherry-pick + void-if-fail +
  control. Stated the **+1.74 buy-and-hold Sharpe as the bar MR must clear to
  matter.** Determinism/anchoring (§ D-MR.6): the family axis is varied at the
  config level (NOT the seed level) ⇒ ADR-0051 D6.1 SAME-paths holds verbatim, 86
  anchors unchanged by construction; +1 MR θ-surface anchor under the existing
  `mc-robustness-2026-06` namespace (86→87, scenario
  `v1-mr-theta-surface-2023-block-bootstrap-real-fy`); written as a short ADR-0051
  **§ D6.5 cross-reference amendment** (NOT a new ADR — the cheap-and-correct
  `analyst-defaults` exception). Ratified the backtest scenarios (MR-C3 surface =
  PRIMARY anchored; MR-C2 N=500 single-config = optional fast-follow; 2023-FY
  in-sample apples-to-apples; 2024-FY OOS secondary non-gating). No code written
  (design-only, reversible until dev build); no trace.toml / anchors.toml touch
  (orchestrator/tester own those).
- 2026-05-31 (analyst, pivot-scoping): drafted the first-pivot brief after the
  momentum FAMILY-UNIFORM-FRAGILE closure. **Recommendation: cross-sectional
  mean-reversion** (runnable NOW on existing price-only Binance OHLCV — confirmed
  by reading `realdata.rs`, the `Bar` struct, and the parquet column footer:
  columns are OHLCV-only, no funding/basis/fundamental field), with **carry as
  the data-gated fast-follow**. Data-availability verdict (load-bearing): MR &
  breakout = price-only RUNNABLE NOW; carry = funding infra half-built (live
  `FundingPoller` + `funding_rates` table exist, but NO historical funding
  parquet backfill — scoped a `fetch_binance_funding` acquisition path); value =
  on-chain proxy absent end-to-end (largest lift). Chose MR over breakout on the
  turnover-killer lesson (breakout is trend-adjacent → shares momentum's losing
  prior; MR is the direct counter-hypothesis and can be engineered low-turnover).
  Defined signal (negated v1 score = bottom-K of the existing `top_k_long`
  ranking), sizing (reuse v1 long-only + the solvency-guarded `run_path`),
  rebalance (slow + wide no-trade band, turnover as a first-class θ-axis), and
  the day-1 BOTH-axes robustness gate (C2 + C3 + buy-and-hold control) under the
  frozen decision rule. Noted prior MR plumbing exists (`vol_meanreversion.rs`,
  `pairs/mean_reversion.rs`, shipped `v15a-mean-reversion-pairs` anchors).
