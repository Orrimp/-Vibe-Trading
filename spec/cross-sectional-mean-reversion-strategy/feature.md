---
slug: cross-sectional-mean-reversion-strategy
version: 0.1.0
status: draft
owner: analyst
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
_architect fills this — proposed shape below for the architect to ratify or reject_

Proposed (non-binding) shape, to minimise blast radius:

- **New strategy:** `crates/strategy/src/cross_sectional/` gains a
  `mean_reversion` mode (or a thin `CrossSectionalMeanReversionStrategy` wrapping
  the existing momentum scorer with a negated ranking). The cleanest M-T1 lock
  is a `Direction { Momentum, Reversion }` enum on the existing config rather
  than a forked strategy, so the inversion is provably a single sign flip and the
  divergence test (R-MR.1) is meaningful. **Architect decides** enum-on-config vs
  separate-struct — the durable choice is the one that keeps the two families
  sharing one tested ranking path so they can never silently diverge in plumbing.
- **Harness reuse:** the MR strategy is injected into the **unchanged**
  `montecarlo::run_path` (it already accepts a caller-supplied strategy) and the
  **unchanged** `param_robustness_sweep` bin (parametrise the grid + strategy
  constructor). No new harness machinery — this is the closure deck's
  "implement a new `Strategy` and run it through both bins" path.
- **Open architecture question for M-T1:** is MR a new `[[req]]`-level strategy
  family, or a `Direction` variant of the existing cross-sectional req? This
  determines whether the MR θ-grid is a sibling `const` next to `TIER1_GRID` or a
  generalised grid keyed by direction. (Analyst leans: shared ranking path,
  separate grid const — see Open questions.)

---

## Backtest Scenarios
_analyst + architect fill using the backtest/scenario template — outline below_

1. **MR-C2** — `mr-2023-block-bootstrap-real-fy-mc`: single best-a-priori MR
   config (long lookback + wide band), N=500 shared-index block-bootstrap,
   2023-FY real Binance, 6 bps fees. Primary path-robustness verdict.
2. **MR-C3** — `mr-theta-surface-2023-block-bootstrap-real-fy`: ~6-cell θ-grid
   spanning the turnover axis (R-MR.3), N=200 each. Family verdict.
3. **Control** — buy-and-hold under the same auto-L bootstrap (re-asserts +1.74).
4. **OOS read (secondary)** — the same MR-C2 config on 2024-FY (data present) as
   an out-of-history corroboration, NOT a primary gate.

---

## Implementation
_developer fills this_

Lesson carried from C3 (`momentum-parameter-robustness-sweep` § What this cost):
**validate the compute budget before locking the grid** — wall-clock ≈ grid × N
× per-path cost; the C3 14-cell × N=500 design was intractable (~1 h) and was
re-scoped to 6 × 200 (~20 min). The MR C3 grid inherits the 6 × N=200 envelope.

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
