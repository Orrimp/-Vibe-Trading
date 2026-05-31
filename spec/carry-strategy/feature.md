---
slug: carry-strategy
version: 0.1.0
status: draft
owner: analyst
priority: P2
updated: 2026-05-31
---

# Cross-sectional funding carry — the pre-registered rotation target after BOTH price families came back fragile — v0.1.0

> **The structurally-different bet.** Both price-based cross-sectional families
> are now retired on the robustness axis: momentum (FAMILY-UNIFORM-FRAGILE,
> path + parameter) and mean-reversion (FAMILY-UNIFORM-FRAGILE, parameter; all
> 6 θ-cells FRAGILE, best-shot g=3 p50 +0.007). The KILLER in both was
> **turnover / fee-bleed**: on the SAME resampled 2023 histories a passive
> equal-weight buy-and-hold of the same 10 coins earned **+1.74 Sharpe**
> (P(loss) 4.5%), and both active price families converted that capturable drift
> into a break-even-at-best loss machine through churn
> ([MR verdict](../cross-sectional-mean-reversion-strategy/feature.md#implementation);
> [momentum closure](../momentum-parameter-robustness-sweep/presentations/momentum-robustness-closure-2026-05-30.md)).
>
> **Carry is the pre-registered rotation runner-up**
> ([MR brief § recommendation](../cross-sectional-mean-reversion-strategy/feature.md#the-recommendation)):
> the most-INDEPENDENT return source (funding-based, non-trend), naturally
> LOW-turnover (funding settles 8h), and the best a-priori structural answer to
> the turnover-killer. Its **DATA is already BANKED**
> ([carry-funding-data-backfill](../carry-funding-data-backfill/feature.md),
> committed `ab815d5`: `data/binance-funding/`, 240 parquets, 10 symbols ×
> 2023-24, REVISION-pinned SHA `bf1ede44…`).
>
> **This brief is reversible DESIGN work** — it scopes the signal, the
> funding-data integration (the bulk of the new engineering), the θ-grid, and
> the day-1 BOTH-axes gate so the operator can make a decision-grade go/no-go.
> It commits NO code and triggers NO engine run. The operator confirms before
> the build. Per the orchestrator's scoping: no engine changes, no build, no
> experiments here.

---

## 0. Pre-registration & anti-cherry-pick (inherited verbatim, frozen now)

Carry is vetted under the **already-frozen** pre-registered decision rule
([`robustness-decision-rule-2026-05-30.md`](../dev-notes/robustness-decision-rule-2026-05-30.md) § 0)
— the same ruler that scored momentum AND mean-reversion. Nothing about the rule
is re-opened. Three commitments carry over verbatim:

1. **The bands are frozen.** p5 Sharpe ≥ +0.5 ROBUST / < 0 FRAGILE; prob-of-loss
   ≤ 15% ROBUST / > 35% FRAGILE; p95 MaxDD ≤ ~50% ROBUST / > ~70% FRAGILE; p50
   Sharpe ≥ 1.0 ROBUST; P(Sharpe>1) ≥ 60% ROBUST. Composite = **worst primary
   band wins** (weakest-link). Carry is scored against these, not the reverse.
2. **Anti-cherry-pick by construction.** The C3 θ-sweep reports the FULL surface
   + a family verdict and **crowns no argmax winner** (the FP-C3.5 renderer
   enforces this in code). A non-FRAGILE cell carries a `→ C5 deflation required`
   flag. A grid that picked argmax would inflate the false-ROBUST rate
   (`1 − 0.95^G`).
3. **Pre-flight void-if-fail.** Every carry report must print
   `generator: block-bootstrap-real` AND `bootstrap_mode: shared-index`, else the
   verdict is void (the tail is not a fair adversary otherwise).

**The buy-and-hold control (+1.74 Sharpe) is the bar carry must clear to matter.**
A family that does not beat simply holding the same coins net of fees on this
universe is not worth promoting, however internally "robust." Carry's honest
a-priori edge is precisely that, being low-turnover and non-trend, it has the
best structural shot at clearing that bar where the two price families could not.

---

## Why

### Why carry, and why now (the rotation is pre-registered, not invented here)

The robustness program has now run BOTH price-based cross-sectional families
through the harness and retired both:

| Family | Axis result | Killer |
|---|---|---|
| **Momentum** (top-K winners) | C2 FRAGILE (p50 ≈ −0.01, P(loss) 75.2%) + C3 FAMILY-UNIFORM-FRAGILE (6/6 cells) | turnover / fee-bleed (~5,343 trades/yr) |
| **Mean-reversion** (bottom-K losers) | C3 FAMILY-UNIFORM-FRAGILE (6/6 cells; best-shot g=3 p50 +0.007, P(loss) 42.5%) | turnover / fee-bleed (g=3 still 379,809 path-trades) |
| **Buy-and-hold** (passive) | p50 **+1.74**, P(loss) 4.5%, p95 MaxDD 51.2% | — (this is the bar) |

The lesson is now doubly-confirmed: **active *price-based* cross-sectional
trading on this 10-coin 1h universe loses to passive holding because of
fee-bleed.** The MR brief anticipated exactly this and named the rotation target:
*"Runner-up / fast-follow: carry, the moment a historical funding backfill lands
(it is the best structural answer to the turnover-killer)."* That backfill has
now landed (`carry-funding-data-backfill`, committed `ab815d5`). Carry is the
pre-registered next family — this brief makes it decision-grade.

### Why carry is the best a-priori shot at the FIRST non-fragile strategy

Two structural properties, both directly aimed at the killer that retired the
price families:

1. **Non-trend, genuinely independent return source.** Carry earns the **funding
   payment** (a cash settlement every 8h), not a price move. Its P&L is
   structurally decorrelated from the vol-adjusted-return signal that drives both
   momentum and MR. This is what makes it a true *rotation* rather than a third
   variant of the same bet — and it is the basis of the R-CARRY.6 divergence
   falsifier (the carry signal MUST select differently from a price-based signal,
   else it is not a genuinely different return source).
2. **Naturally LOW turnover.** Funding settles only **3×/day** (00:00, 08:00,
   16:00 UTC) and the cross-sectional funding *ranking* is far more persistent
   than a 1h price-momentum ranking (funding regimes — which side is crowded —
   move on multi-day timescales, not hourly). A carry strategy can rebalance on
   the funding cadence (8h) or slower, with a wide no-trade band, and still track
   its signal. This is the explicit structural dodge of the fee-bleed that
   momentum and MR could not escape even at their lowest-churn corners.

### The honest prior (what would make carry fragile too)

Carry is the best a-priori shot, NOT a guarantee. State the failure modes up
front so the harness verdict is read honestly:

- **Funding-rate mean-reversion / crowding decay.** The cross-sectional funding
  signal predicts the *next* funding payment only if today's high-funding names
  stay high-funding. If funding rates mean-revert fast (a crowded long unwinds,
  funding snaps back), the realized carry is much smaller than the signal, and
  the strategy pays fees to chase a rate that has already decayed. This is the
  carry analogue of the turnover-killer and is the single most likely way carry
  comes back FRAGILE.
- **Funding is small vs price vol.** An 8h funding rate is typically ~0.01%
  (≈0.03%/day, ≈11%/yr at the median). The price moves of the underlying perp
  dwarf that. If the carry strategy holds directional perp exposure (long the
  high-carry names), its P&L is dominated by *price* risk, not the funding it is
  trying to harvest — and then it is just a noisy long-only price bet that
  inherits the same drift the buy-and-hold control already captures more cheaply.
  **This is the load-bearing design question (Q-CARRY-2 below): is carry a
  market-neutral funding-harvest or a directional carry-tilt?** The answer
  materially changes both the signal and the integration.
- **The robustness axis judges resampled real 2023 history only** — it cannot
  speak to a funding regime 2023 never contained (inherited scope limit). And a
  block bootstrap of funding has a subtle methodological wrinkle the price
  families did not face (§ D-CARRY.2 — funding must be resampled with the SAME
  index sequence as returns, or price and funding decouple).

If carry ALSO comes back FAMILY-UNIFORM-FRAGILE, that is **again a methodology
win** (the machine has now cheaply ruled out the three most-cited crypto
cross-sectional families — momentum, MR, carry — on this universe) and the next
rotation is value (data-gated) or a regime/blended approach. The brief does not
overclaim a carry edge.

---

## Two real design problems (the scope)

This brief scopes two genuinely-hard problems. Problem 1 (the signal) has a
load-bearing **sign convention** that must be exactly right. Problem 2 (the
funding-data integration) is **the bulk of the new engineering** — materially
larger than MR's 1-line reuse — and has a methodological wrinkle (bootstrap
resampling destroys real calendar time) that the carry-data backfill brief did
not surface. Both are scoped honestly below.

### PROBLEM 1 — The carry SIGNAL (definition + the load-bearing sign convention)

#### R-CARRY.1 — Signal: cross-sectional funding rank (smoothed funding over a lookback)

The carry score for symbol `s` at time `t` is the **smoothed recent funding
rate**, ranked cross-sectionally:

```text
carry_score(s, t) = mean( funding_rate[s, τ] for τ in the last L funding settlements before t )
```

i.e. a trailing average of the 8h funding rate over a lookback `L` (in funding
settlements, e.g. L=3 = last 24h, L=9 = last 3 days, L=21 = last week). The
trailing mean is the carry analogue of momentum's `score_vol_adjusted_return`:
it smooths the noisy single-settlement rate into a persistent cross-sectional
signal. Symbols are then ranked by this score and the top/bottom-K selected per
the direction semantics in R-CARRY.2.

_Acceptance: the carry score is a pure function of the funding series over the
lookback; a unit test on a synthetic funding series confirms the trailing-mean
ranking selects the highest-average-funding names._

#### R-CARRY.2 — The SIGN CONVENTION (load-bearing — confirmed against Binance docs)

**This is the most error-prone part of the whole feature and must be exactly
right.** Confirmed from the Binance USDⓈ-M perpetual funding documentation:

> - **Funding rate POSITIVE → LONGS pay SHORTS.** (Longs are crowded; the perp
>   trades above index; longs compensate shorts.)
> - **Funding rate NEGATIVE → SHORTS pay LONGS.** (Shorts are crowded; the perp
>   trades below index; shorts compensate longs.)

**Therefore, to EARN funding you take the side that gets PAID:**

| Funding sign | Who pays | To earn the funding, hold | Perp position |
|---|---|---|---|
| **positive** (e.g. +0.01%) | longs → shorts | the **short** side | SHORT the perp |
| **negative** (e.g. −0.01%) | shorts → longs | the **long** side | LONG the perp |

So the **funding-harvest** direction is: **SHORT the high-positive-funding
names, LONG the high-negative-funding names** (the opposite sign of a naive
"long the high number"). The annualized carry earned per leg ≈ `−sign(funding) ×
|funding| × 3 × 365` (3 settlements/day) — the leading **minus** is the
load-bearing semantic: you earn by being on the paid side, which is the opposite
sign to the funding rate.

> **A naive sign error here silently inverts the entire strategy** (turns a
> funding-harvest into a funding-payer). R-CARRY.7 (the day-1 gate) includes a
> dedicated **sign-convention assertion test** that constructs a synthetic
> universe with a known-positive-funding symbol and asserts the carry strategy
> takes the SHORT (paid) side of it — the falsifier that the sign is harvested,
> not paid.

> **OPEN QUESTION Q-CARRY-2 (operator/architect — load-bearing).** v1 spot
> infrastructure is **long-only** (`k_short = 0` is enforced in the cross-
> sectional config loader; the `run_path` engine only opens `Side::Buy`
> positions — see § D-CARRY.1). A pure funding-harvest needs SHORT legs (to earn
> positive funding). **Two viable v0.1.0 framings, with the recommendation:**
>
> - **(a) DIRECTIONAL carry-tilt, long-only (Recommended for v0.1.0).** Long the
>   most-**negative**-funding names only (you LONG to earn negative funding —
>   the paid side is long there). Reuses the existing long-only `run_path` +
>   `top_k_long` sizing verbatim (no engine short-side work), so it is apples-to-
>   apples with the momentum/MR comparison AND fits the existing solvency-guarded
>   engine. Honest caveat: this is a *directional* bet (long perp exposure on the
>   negative-funding names), so its P&L carries price risk, not pure funding —
>   it tests "does tilting toward negative-funding names beat buy-and-hold," a
>   real and answerable question, but NOT a market-neutral funding harvest.
> - **(b) MARKET-NEUTRAL long/short funding harvest (the durable target, but
>   larger).** Long negative-funding, short positive-funding, dollar-neutral —
>   the textbook carry trade that isolates funding from price. Requires the
>   engine to support SHORT legs (new sizing path in `run_path`, new short-
>   solvency/margin accounting, `k_short > 0` un-gated in the config loader) AND
>   funding-cashflow accrual on both legs. This is a materially larger build
>   (§ D-CARRY size estimate) and would NOT be apples-to-apples with the long-only
>   momentum/MR anchors.
>
> **Analyst recommendation: ship (a) long-only directional carry-tilt at
> v0.1.0** as the cheap-and-apples-to-apples first read against the +1.74 bar,
> and treat (b) market-neutral as the v0.2.0 durable follow-on IF (a) shows a
> non-FRAGILE signal worth the short-side engineering. **Rationale for putting
> Recommended on (a) here (the durable-over-quick exception):** (b) is the more
> *complete* strategy, but the load-bearing scientific question — *"does a
> non-trend funding signal beat buy-and-hold on this universe?"* — is answerable
> by (a) at a fraction of the cost, and (a) keeps the engine comparison clean.
> Building the short-side engine BEFORE knowing whether the funding signal has
> any edge would be building durable infrastructure on an unvalidated premise —
> the opposite of durable. If (a) is FRAGILE, (b) almost certainly is too (same
> signal, the short leg just doubles the funding exposure), and the short-side
> engine is never owed. **This is the architect's M-T1 call to ratify** —
> flagged as the headline open question.

#### R-CARRY.3 — Rebalance cadence (turnover is the structural advantage — exploit it)

Carry's whole a-priori edge is low turnover. The default carry config MUST
exploit the 8h funding cadence:

- **Default rebalance = 8h (480 minutes)** — aligned to the funding settlement,
  the natural carry cadence (vs momentum's 60-min default). A faster rebalance
  earns nothing extra (funding only settles 3×/day) and only bleeds fees.
- **A wide no-trade band** (reuse the `drift_rebalance_threshold` lever) so small
  rank wiggles in the funding ranking do not trigger trades.
- **The C3 θ-grid spans the turnover axis** (§ D-CARRY.2-PROPOSED): from an 8h
  rebalance (the natural low-churn cadence) to a deliberately slower daily/weekly
  rebalance, × funding-lookback × K — to confirm carry's low-turnover prior and
  find whether the turnover-vs-Sharpe relationship differs from the price
  families (it SHOULD — that is the falsifiable claim).

_Acceptance: the default carry config rebalances on the 8h funding cadence; the
C3 grid includes a deliberately-slow (≥ daily) low-churn cell AND a faster cell
so the trades/yr column shows carry's turnover is structurally below the price
families' at comparable cells._

#### R-CARRY.4 — Can carry reuse the cross-sectional ranking path? (the reuse assessment)

**Partial reuse — more than zero, far less than MR's near-total reuse.** The
honest assessment, grounded in the code:

| Layer | MR reuse | Carry reuse | Why |
|---|---|---|---|
| **Selection** (`top_k_long`, descending top-K, alpha tie-break) | verbatim | **REUSE verbatim** | a ranked-score top-K is exactly what carry needs (rank by funding score) |
| **Sizing** (`run_path` long-only, solvency-guarded, equal-weight) | verbatim | **REUSE verbatim** (under framing (a)) | long-only directional carry-tilt = same sizing as momentum/MR |
| **The SCORE** | 1-line negation of the price score | **NEW data source** — funding, not price | this is the crux: MR's score was the *same input* (price) negated; carry's score is a *different input* (funding) that **the engine does not currently load** |
| **The HARNESS** (`run_path`, bootstrap) | verbatim (bars only) | **EXTENDED** — must carry funding alongside bars through the bootstrap | the bulk of the new engineering (PROBLEM 2) |

**The verdict: carry is NOT a `Direction` variant the way MR was.** MR worked as
a 1-line `Direction { Momentum, Reversion }` enum because momentum and MR consume
the **identical price input** and differ only in the sign of the ranking. Carry
consumes a **different input entirely** (funding), which the strategy cannot see
because the engine does not thread funding to the strategy. So carry needs:

1. a **new score source** in the cross-sectional strategy (a `ScoreSource {
   VolAdjustedReturn, FundingCarry }` enum on the config, analogous to but
   distinct from `Direction` — the funding score reads a funding series the
   strategy must be given access to), AND
2. the **funding-data integration** to give the strategy that access (PROBLEM 2).

This is the architect's M-T1 design decision (Q-CARRY-1 below): the cleanest seam
to surface funding to the `MomentumStrategy` (or a sibling strategy). The reuse of
`top_k_long` + the long-only sizing is real and worth ~40-50% of a from-scratch
strategy; the score-source + funding-threading is genuinely new.

> **OPEN QUESTION Q-CARRY-1 (architect M-T1).** HOW does the funding series reach
> the strategy's score computation? The `MomentumStrategy::on_bar` only receives a
> `&Bar` (OHLCV — no funding field). Three candidate seams, for the architect to
> rule on (analyst lean noted, not locked):
> - **(i)** extend `Bar` with `funding_rate: Option<Decimal>` (blast-radius across
>   the whole `Bar` struct + every bar constructor + the bootstrap + serde — high
>   blast radius, touches the anchored bootstrap output shape);
> - **(ii)** a parallel `funding_by_symbol_ts: BTreeMap<(Symbol, Timestamp),
>   Decimal>` injected into the strategy alongside the bar stream (lower blast
>   radius, mirrors `bars_override`; the carry-data brief's preferred option —
>   *"carry a parallel funding lookup keyed by (symbol, ts) injected alongside
>   bars_override in run_path"*) — **analyst lean**;
> - **(iii)** a new `CarryStrategy` struct that owns its funding series and
>   implements `Strategy` — but `run_path` is typed to concrete `MomentumStrategy`
>   (montecarlo.rs:79), so this forces `run_path` generic/`dyn` and risks all 87
>   anchors (the exact constraint that forced MR to be an enum-on-config, not a
>   struct — see [MR § D-MR.0](../cross-sectional-mean-reversion-strategy/feature.md#design)).
>   **The architect must weigh this against the anchor-preservation constraint.**

### PROBLEM 2 — The funding-data INTEGRATION (the bulk of the new engineering — sized honestly)

This is where carry diverges sharply from MR's 1-line reuse. The harness engine
loads ONLY OHLCV parquet today; carry needs the funding series loaded, aligned to
the price bars, carried through the bootstrap, and surfaced to the strategy. Each
sub-problem is scoped with an honest size estimate below.

#### R-CARRY.5 — Sub-problem A: LOAD the funding parquet (a `funding_root` loader)

`crates/backtest/src/realdata.rs` (`RealDataBarSource`) loads OHLCV via
`data::ReplayFeed::merge_symbols` and verifies the `data/binance/REVISION.toml`
pin. Carry needs a sibling path for `data/binance-funding/`:

- **A funding loader** mirroring `RealDataBarSource`: read
  `data/binance-funding/<SYM>/<YEAR>/<MM>.parquet` (schema: `symbol` Utf8,
  `funding_time` Int64 ms, `funding_rate` Utf8 decimal-string — confirmed from
  the backfill brief + the on-disk REVISION.toml), parse `funding_rate` via
  `rust_decimal::Decimal`, verify the `data/binance-funding/REVISION.toml` pin
  (SHA `bf1ede44…`) exactly as the OHLCV loader verifies its manifest.
- Output: a `Vec<FundingObs>`-compatible structure (the `core::FundingObs` type
  already exists — `symbol`, `funding_rate`, `funding_ts`, …) or a leaner
  `(Symbol, funding_time_ms, Decimal)` tuple stream.
- **The funding parquet is TINY**: 3 rows/day × ~365 days = ~1,095 rows/symbol-yr
  vs ~8,760 OHLCV bars/symbol-yr. Loading is cheap; the cost is the new module +
  its REVISION-verification + tests, not the I/O.

**Size: SMALL-MEDIUM (~0.5-1 day).** A near-mirror of the existing
`RealDataBarSource` + revision verifier, on a tiny dataset. Well-precedented by
the OHLCV loader and the `data::revision` module the funding REVISION.toml
already uses.

#### R-CARRY.6 — Sub-problem B: ALIGN funding (8h) to price bars (1h) — the cadence mismatch

Funding settles every 8h; bars are hourly. The strategy needs to know, at each
hourly bar, "what is the current/most-recent funding rate for this symbol?":

- **Forward-fill the 8h funding rate onto the 1h bar grid**: each hourly bar
  carries the **most-recent settled** funding rate (the rate from the last
  settlement at or before the bar's `open_ts`). This is a deterministic
  as-of-join (step function), the standard funding-to-bar alignment.
- **Information-timing discipline (no look-ahead):** the carry strategy may only
  use funding **settled at or before** the bar it trades on. The carry-data brief
  flagged this exact decision (*"trade on the bar that just settled vs the next
  bar (conservative)"*). Recommendation: use the funding settled at-or-before
  `open_ts` (information available at decision time) — the conservative,
  no-look-ahead choice. A look-ahead falsifier (R-CARRY.7) asserts no future
  funding leaks into a bar's score.

**Size: SMALL (~0.5 day) on real (un-bootstrapped) data.** A deterministic as-of
forward-fill. **BUT see Sub-problem C — under the bootstrap this becomes the hard
part.**

#### R-CARRY.7 — Sub-problem C: carry funding THROUGH the block bootstrap (the methodological crux)

**This is the load-bearing integration finding, and it is harder than the
carry-data backfill brief assumed.** Read from `crates/data/src/synth/bootstrap.rs`:

The C2/C3 robustness harness does NOT replay real bars. It runs each path on a
**block-bootstrap resampling** of the real returns:
`BlockBootstrapPathGen::generate` (1) builds per-symbol **log-return** series from
the real closes, (2) draws ONE shared **index sequence** `idx_seq` (stationary
bootstrap, geometric blocks, mean length L), (3) applies that index to ALL symbols
to reconstruct resampled price paths, and (4) **emits bars with SYNTHETIC
timestamps** (`epoch_2023() + i hours`) — the **real calendar time is discarded**.

**The consequence for carry:** the funding rate is a per-settlement value keyed to
**real calendar time** (`funding_time`). A naive "forward-fill real funding onto
the bootstrapped bars by timestamp" is **meaningless** — the bootstrapped bar at
synthetic hour `k` corresponds (via `idx_seq[k]`) to some *resampled* real return
index, NOT to synthetic-hour-`k` of real calendar time. **Price and funding would
decouple**: the bar's price came from real index `idx_seq[k]`, but the naive
funding fill would attach the funding from a *different* real time. That breaks the
co-movement the whole shared-index bootstrap exists to preserve.

**The correct design: funding must be resampled with the SAME index sequence as
the returns.** Concretely:

- Pre-compute a per-symbol **per-return-step funding series** on the REAL bar grid:
  `funding_at_return[s][k]` = the funding rate in force at real return-step `k`
  (the as-of forward-fill from R-CARRY.6, computed ONCE on the real data, length
  `T−1` to match the return series).
- When the bootstrap draws `idx_seq[k]`, the resampled funding for output bar `k`
  is `funding_at_return[s][idx_seq[k]]` — the **same index** that selected the
  return. Price and funding then move together exactly as the real contemporaneous
  pair did. This preserves the funding↔price co-movement under resampling, which
  is the entire point of the shared-index design (FP-C1.5).
- **Mechanically:** `BlockBootstrapPathGen` must either (a) gain an optional
  parallel `funding_by_symbol: Vec<Vec<Decimal>>` source aligned to the return
  series, resampled by the same `idx_seq` and emitted alongside `bars_by_symbol`
  in `GeneratedPath`; or (b) a parallel `FundingBootstrapPathGen` that takes the
  same `idx_seq`. **(a) is preferred** (one index draw, guaranteed identical
  resampling — a separate generator risks index drift, the cannot-silently-diverge
  discipline). This requires extending `GeneratedPath` with an optional
  `funding_by_symbol: Option<Vec<Vec<Decimal>>>` field and threading it through
  `merge_synthetic` → `bars_override`'s sibling → `run_path`.

> **This is the single biggest, least-precedented piece of the feature.** The
> carry-data backfill brief's consumption note (*"forward-fill funding onto bars
> by timestamp"*) is correct for a **real-replay** backtest but is **WRONG for the
> bootstrap harness** the robustness gate requires — it would silently decouple
> price and funding. The architect MUST design the funding-through-bootstrap path
> so the shared index governs both. This finding is the reason carry's integration
> is materially larger than MR's.

**Size: MEDIUM-LARGE (~2-3 days).** New `GeneratedPath` field + bootstrap
resampling of the parallel funding series by the shared index + threading through
`merge_synthetic`/`run_path` + the funding-cashflow accrual (Sub-problem D) +
determinism (the funding resampling must be byte-identical two-run, and must NOT
disturb the 87 existing anchors — the funding path is purely additive, gated on a
new `funding_source: Option<…>` that defaults `None` → momentum/MR runs are
byte-unchanged by construction, the same additive discipline MR used).

#### R-CARRY.8 — Sub-problem D: APPLY the funding cashflow in the engine (P&L accrual)

For carry to have a P&L distinct from a plain price tilt, the **funding payment
must hit the equity curve**. `run_path` (montecarlo.rs) marks-to-market and pushes
equity per bar (line 281). The funding accrual goes there:

- On each bar where a funding settlement occurs (every 8h boundary on the
  resampled grid), for each held position, accrue the funding cashflow:
  `cash += position_notional × funding_rate × side_factor`, where `side_factor`
  encodes the R-CARRY.2 sign (a LONG position earns when funding is negative, pays
  when positive). For framing (a) long-only, every leg is long, so the accrual is
  `−funding_rate × notional` (you earn the negative-funding names' funding, pay the
  positive ones').
- This is the **mechanism that makes carry ≠ a long-only price bet**: without the
  funding cashflow, framing (a) is literally just "long the negative-funding
  names" with no carry P&L, and the R-CARRY divergence test would be testing only
  a selection difference, not a return-source difference. **The funding accrual is
  non-negotiable for the feature to be "carry" at all.**

**Size: SMALL-MEDIUM (~0.5-1 day).** A per-bar cashflow injection at the existing
equity-update point, gated on the funding series being present. The CLAUDE.md
overlay-divergence discipline applies directly here (see § D-CARRY gate): an e2e
test must assert the funding accrual is non-zero / moves equity, or it is a
silent no-op exactly like the v3-vol-overlay bug.

#### Total integration size estimate (PROBLEM 2)

| Sub-problem | Size | Precedent |
|---|---|---|
| A — funding parquet loader (`funding_root`) | SMALL-MEDIUM (~0.5-1 day) | mirrors `RealDataBarSource` + `data::revision` |
| B — 8h→1h as-of forward-fill alignment | SMALL (~0.5 day) | standard funding-to-bar join |
| C — carry funding THROUGH the bootstrap (shared index) | **MEDIUM-LARGE (~2-3 days)** | **least-precedented; the crux** |
| D — funding-cashflow accrual in `run_path` | SMALL-MEDIUM (~0.5-1 day) | per-bar equity injection point exists |
| Signal (PROBLEM 1) — `ScoreSource::FundingCarry` + sign | SMALL-MEDIUM (~1 day) | reuses `top_k_long`; new score path |
| **TOTAL (framing (a), long-only directional)** | **~4.5-7.5 days** | vs MR's ~1-2 days |

**The honest headline: carry is roughly 3-5× the engineering of MR.** MR was a
1-line score negation on an input the engine already loaded. Carry needs a new
data source loaded, aligned, resampled-through-the-bootstrap (the hard part), AND
a new cashflow mechanism in the engine. The data being banked removes the
acquisition cost (~1-2 days saved) but NOT the integration cost. Framing (b)
market-neutral adds the short-side engine on top (~2-3 more days), which is why
(a) is the recommended v0.1.0 floor.

---

## Requirements summary (consolidated)

- **R-CARRY.1** — Signal = trailing-mean funding rank over a lookback L (funding
  settlements). Pure function of the funding series.
- **R-CARRY.2** — Sign convention (LOAD-BEARING): positive funding → longs pay
  shorts; to EARN funding hold the paid side; harvest direction is opposite-sign
  to the funding rate. A day-1 sign-assertion test is mandatory.
- **R-CARRY.3** — Default rebalance = 8h funding cadence + wide no-trade band;
  the C3 grid spans the turnover axis to confirm the low-churn prior.
- **R-CARRY.4** — Reuse `top_k_long` + long-only sizing verbatim; the score is a
  NEW `ScoreSource::FundingCarry` (NOT a `Direction` variant — different input).
- **R-CARRY.5** — Sub-A: funding parquet loader + REVISION pin (`bf1ede44…`).
- **R-CARRY.6** — Sub-B: 8h→1h as-of forward-fill, no look-ahead.
- **R-CARRY.7** — Sub-C: carry funding through the bootstrap via the SHARED index
  (the crux — additive, defaults-off so 87 anchors hold by construction).
- **R-CARRY.8** — Sub-D: funding-cashflow accrual in `run_path` (the mechanism
  that makes carry a distinct return source; e2e-divergence-gated per CLAUDE.md).
- **R-CARRY.9** — Day-1 BOTH-axes robustness gate (C2 + C3 + buy-and-hold
  control) under the frozen rule (§ Day-1 gate).
- **R-CARRY.10** — Divergence falsifier: carry selection/P&L ≠ price-based
  selection/P&L (genuinely different return source) + the funding-no-op falsifier.
- **R-CARRY.11** — Determinism & anchoring: byte-identical on the canonical box;
  the carry funding path is additive (defaults-off) so the 87 existing anchors are
  byte-unchanged; +1 carry θ-surface anchor (87→88) after the dev's anchored run
  (tester locks it; the grid + N are locked at design time per the MR precedent).
- **R-CARRY.12** — In-sample = 2023-FY (apples-to-apples with momentum/MR anchors);
  2024-FY OOS secondary, non-gating.

---

## Design
_architect fills this (M-T1). The analyst has flagged the load-bearing decisions
as Q-CARRY-1 (funding-to-strategy seam), Q-CARRY-2 (long-only directional vs
market-neutral — the headline), and the bootstrap-shared-index funding resampling
(§ D-CARRY.7-equivalent, the integration crux). The θ-grid below is PROPOSED, not
LOCKED — the architect locks it before the tester anchors, per the MR precedent._

### D-CARRY.0 — (architect) resolve Q-CARRY-2: long-only directional vs market-neutral
### D-CARRY.1 — (architect) resolve Q-CARRY-1: the funding-to-strategy seam
### D-CARRY.2-PROPOSED — the carry θ-grid (PROPOSED below; architect LOCKS)
### D-CARRY.7 — (architect) the funding-through-bootstrap shared-index design (the crux)

---

## Backtest Scenarios
_architect-ratifies. Primary anchored deliverable = the carry-C3 θ-surface;
carry-C2 single-config = optional higher-confidence tail read; 2023-FY in-sample;
2024-FY OOS secondary, mirroring the momentum/MR pattern._

1. **CARRY-C3 (PRIMARY, ANCHORED)** — `v1-carry-theta-surface-2023-block-bootstrap-real-fy`:
   the PROPOSED ~6-cell θ-grid spanning the turnover/lookback axes, N=200/cell,
   shared-index block-bootstrap of 2023-FY real Binance OHLCV **+ the
   shared-index-resampled funding series** (§ R-CARRY.7), 6 bps fees (2 slippage
   + 4 taker, inherited). ONE anchored θ-surface report. Per-cell
   FRAGILE/MARGINAL/ROBUST + family verdict + per-cell `→ C5` flags + the trades
   column (turnover legibility) + (carry-specific) a **realized-funding-harvested**
   column so the funding-vs-price P&L split is legible.
2. **Control (in the carry-C3 surface)** — buy-and-hold equal-weight under the same
   N paths + auto-L bootstrap (re-asserts the **+1.74 Sharpe bar carry must
   clear**). This row carries no verdict; the carry family verdict is read relative
   to it.
3. **CARRY-C2 (OPTIONAL fast-follow)** — `v1-carry-2023-block-bootstrap-real-fy-mc`:
   the single best-a-priori carry config (lowest-churn 8h-rebalance cell), N=500
   shared-index block-bootstrap, 2023-FY. The tighter-tail single-config read; ship
   only if the carry-C3 surface shows a non-FRAGILE cell worth the higher-confidence
   estimate. +1 anchor if run.
4. **OOS read (SECONDARY, non-gating)** — the same best-a-priori carry config on
   2024-FY (data present, REVISION-pinned) as out-of-history corroboration. SEPARATE
   run = SEPARATE anchor, NOT a primary gate. v0.2.0 fast-follow.

**θ-grid (PROPOSED — architect LOCKS before tester anchors).** Held constant
(mirroring the MR/momentum lock): `score_source = funding_carry`, `exposure_cap =
0.50`, `vol_floor` irrelevant for carry (funding score has no vol denominator —
the architect decides the carry-score floor), `size = equal_weight`, the 10-symbol
universe, year = 2023, N = 200, `ensemble_seed = 0xC0FFEE`, `fill_seed =
0xC0FFEE`, generator = `block-bootstrap-real`, revision `3a8b96c4…` (OHLCV) +
`bf1ede44…` (funding). Swept axes = funding-lookback × rebalance-cadence (the
turnover lever) × K:

| g | funding lookback (settlements) | rebalance | K | role / hypothesis | turnover |
|---|---|---|---|---|---|
| 0 | 9 (~3 days) | 8h (480m) | 3 | **baseline carry θ\*** (natural funding cadence) | low |
| 1 | 3 (~1 day) | 8h (480m) | 3 | short funding lookback — noisier signal | low-mid |
| 2 | 21 (~1 week) | 8h (480m) | 3 | long funding lookback — most persistent signal | low |
| 3 | 9 (~3 days) | 24h (1440m) | 5 | **deliberately-slow rebalance + wide K** (lowest-churn corner — carry's best structural shot) | **lowest** |
| 4 | 9 (~3 days) | 8h (480m) | 1 | narrow selection — top-1 carry name | low |
| 5 | 3 (~1 day) | 8h (480m) | 5 | shortest lookback + wide K — **the highest-churn carry extreme** (still far below the price families' churn — the falsifiable claim) | mid |

The ~6×200 envelope mirrors the C3/MR tractable shape (C3-measured ~20 min for
6×200 + control; carry adds the funding-resampling per path — the architect/dev
MUST re-validate the wall-clock budget before locking, per the C3 lesson:
wall-clock ≈ grid × N × per-path cost, and the funding resampling adds per-path
cost). N=200 is the proposed per-cell N (a hashed body field once locked).

---

## Day-1 BOTH-axes robustness gate + the divergence falsifier (NON-NEGOTIABLE)

Per CLAUDE.md (every strategy overlay/sizing-modifier ships a baseline-divergence
e2e test from day 1) and the closure-deck payoff (every future strategy vetted on
BOTH robustness axes from day 1), carry ships with the full C2+C3+control gate AND
the carry-specific falsifiers.

**The MANDATORY day-1 falsifiers (CLAUDE.md non-negotiable):**

1. **R-CARRY.10a — Carry-vs-price divergence (the headline anti-no-op; the genuinely-
   different-return-source falsifier).** A fast e2e test (small N, short synthetic
   bar + funding series, NO real data — about wiring) runs the SAME path through a
   carry (`score_source = funding_carry`) strategy and a price (`vol_adjusted_return`)
   strategy and asserts the two **selected-symbol sets differ on ≥ 1 rebalance**
   AND the **equity curves diverge by ≥ 1 bp**. This is the carry analogue of the
   MR R-MR.1 gate and the CLAUDE.md overlay-divergence gate. Construct the synthetic
   universe so the highest-funding names are NOT the highest-price-momentum names
   → guaranteed selection divergence.
2. **R-CARRY.10b — Funding-cashflow non-no-op (the CLAUDE.md v3-vol-overlay
   analogue, MANDATORY).** The carry P&L MUST come from the funding accrual, not
   just the selection. Force the funding accrual to zero (drop the cashflow in
   § R-CARRY.8) and assert the equity curve **collapses** to the no-funding case
   (Δ < ε) — proving the funding cashflow is load-bearing, not decorative. This is
   the EXACT pattern of `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`
   that CLAUDE.md mandates: *unit tests on the math layer + anchored backtests are
   NOT sufficient to catch a no-op where the value is computed but never applied.*
   **Both 10a and 10b ship in the test file.**
3. **R-CARRY.2 sign-assertion (the funding-harvest-not-payer falsifier).** A
   synthetic universe with a known-positive-funding symbol and a known-negative one;
   assert the long-only carry strategy (framing (a)) LONGS the negative-funding name
   (the paid side) and the funding accrual is POSITIVE (harvested) for it — and goes
   RED if the sign is flipped (proving the strategy harvests funding, not pays it).
4. **No-look-ahead falsifier (R-CARRY.6).** Assert a bar's carry score uses only
   funding settled at-or-before its `open_ts` — shifting the funding series one
   settlement into the future changes the score (proving the as-of-join is causal).
5. **Two-run byte-identity of the carry θ-surface body-SHA** (ADR-0051 D2/D3/§D6.4):
   run the small-N carry sweep twice at the same `ensemble_seed`; assert identical
   `report_body_hash`. Catches any unordered fold in the funding resampling or the
   carry renderer.

**Required for ship (gating the anchored run):**

6. **C2 path-robustness inside each cell + C3 parameter-robustness across cells**
   — the carry-C3 6-cell surface at N=200 delivers BOTH axes (C2 = the path
   distribution inside each cell; C3 = the parameter surface across cells), the same
   "both axes from one surface" structure the MR brief used.
7. **Buy-and-hold control row** on the same N paths + auto-L — re-asserts the +1.74
   bar carry must clear. The carry family verdict is read relative to it.
8. **Pre-flight void-if-fail** — both report headers print
   `generator: block-bootstrap-real` AND `bootstrap_mode: shared-index`.
9. **Anti-cherry-pick (FP-C3.5 reused)** — family-summary ∈ allowed values; any
   non-FRAGILE cell carries `→ C5 DEFLATION REQUIRED` (and IF a cell is non-FRAGILE,
   the C5 PBO/Deflated-Sharpe deflation pass is genuinely owed — unlike the
   uniform-negative momentum/MR results where C5 was moot).

Pattern references: `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`
(the CLAUDE.md no-op-overlay non-negotiable — directly applicable to R-CARRY.10b);
`crates/backtest/tests/mr_divergence_e2e.rs` (the MR sibling divergence gate
R-CARRY.10a mirrors); `crates/backtest/tests/param_sweep_e2e.rs` (the C3 θ-surface
two-run + anti-cherry-pick gates).

---

## Determinism & anchoring (inherits ADR-0051 § D6; additive — 87 anchors hold)

- **The carry funding path is purely ADDITIVE and defaults-OFF.** Every new seam
  (the `funding_root` loader, the `GeneratedPath.funding_by_symbol` field, the
  `run_path` funding accrual, the `ScoreSource` enum) is gated on a funding source
  being present, defaulting to `None`/absent → **every momentum and MR run is
  byte-identical by construction** (the same additive discipline MR used for the
  `direction` field). The 87 existing anchors (incl. the MR θ-surface #87, SHA
  `a708112e…`, and momentum #86, SHA `0dd989d9…`) are byte-unchanged.
- **SAME-paths seed rule (ADR-0051 D6.1) holds verbatim** — the carry family is
  varied at the strategy/config level (a `score_source` field + the funding data),
  NOT at the seed level. `path_seed_{g,j}` arithmetic is byte-identical. The funding
  resampling uses the SAME `idx_seq` the returns use (§ R-CARRY.7) — it is driven by
  the same single ChaCha20Rng draw, no new seed.
- **Anchor unit = ONE carry θ-surface report.** +1 anchor (87→88). Namespace
  decision for the architect: the carry θ-surface is a new report *shape* (it adds
  the realized-funding column) but the same robustness lane — recommend anchoring
  under the existing `mc-robustness-2026-06` namespace (the lane, not the family,
  defines the namespace), with `verify_anchors.sh` extended to search
  `spec/carry-strategy/reports/` (the one-line additive handler change C3 and MR
  both made). Scenario: `v1-carry-theta-surface-2023-block-bootstrap-real-fy`. The
  grid + N + score_source + funding-revision-SHA are hashed body fields (K3).
- **ADR action (architect):** a short **§ D6.6 amendment** to ADR-0051 stating the
  carry funding path is additive/defaults-off (87 anchors hold by construction) and
  the funding series is resampled by the shared `idx_seq` (so funding↔price
  co-movement is preserved under the bootstrap — a new but small extension of the
  D-C1.3 shared-index property to a second co-resampled series). **This is likely a
  real (small) ADR amendment, NOT a pure cross-reference** — the funding-through-
  bootstrap resampling is a genuinely new mechanism (the shared index now governs a
  second series), unlike MR's pure config-level variation. The architect rules on
  amendment-vs-new-ADR. No anchor in `spec/anchors.toml` is added by the architect
  (the tester locks the +1 carry anchor after the dev's anchored run; the grid + N
  are locked at design time).

---

## Recommendation summary

**Build carry as the pre-registered rotation target — it is the durable choice
and the best a-priori shot at the first non-fragile strategy on this universe.**
Both price families (momentum, MR) are conclusively retired on the turnover-killer;
carry is the structurally-different (non-trend, low-turnover, funding-based) bet
the rotation was pre-registered to reach, and its data is already banked.

**Recommended v0.1.0 scope: framing (a) — long-only DIRECTIONAL carry-tilt** (long
the most-negative-funding names, with the funding cashflow accrued), the cheap,
apples-to-apples first read against the +1.74 bar. It reuses `top_k_long` + the
long-only solvency-guarded `run_path` sizing verbatim and keeps the engine
comparison clean. The market-neutral long/short funding harvest (framing (b)) is
the v0.2.0 durable follow-on IF (a) shows a non-FRAGILE signal worth the short-side
engine — building the short-side engine before validating the funding signal would
be durable infrastructure on an unvalidated premise.

**The honest size: ~4.5-7.5 days (framing (a)), roughly 3-5× MR.** The data being
banked removes acquisition cost but NOT integration cost. The bulk is PROBLEM 2 —
and the load-bearing finding is that **funding must be resampled through the block
bootstrap by the SAME shared index as the returns** (§ R-CARRY.7), because the
bootstrap discards real calendar time; a naive timestamp forward-fill (as the
carry-data brief assumed) would silently decouple price and funding. That is the
single hardest, least-precedented piece and the reason carry is materially larger
than MR.

**If budget tightens (the if-budget-tightens fallback, NOT the recommendation):**
the cheapest scope that still answers the load-bearing question is a **single-config
carry-C2 path-robustness pass only** (the lowest-churn 8h cell, N=500, skip the C3
θ-surface), deferring the parameter axis to v0.2.0. This violates the spirit of the
day-1-both-axes gate (a single config answers "is this cell robust," not "is the
family robust") and should be a conscious operator downgrade. The integration work
(PROBLEM 2 sub-A..D) is required either way — there is no carry backtest without it
— so the savings is only the 5-cell θ-sweep, ~1 day. The recommended path (full C3
surface) is barely more expensive and is the durable, anti-cherry-pick-complete
deliverable.

**Honest prior restated:** carry SHOULD dodge the turnover-killer (it is the whole
point), but it can come back FRAGILE via funding-rate mean-reversion / crowding
decay, or by being a noisy directional price bet (framing (a)'s caveat) that the
+1.74 buy-and-hold already beats more cheaply. If carry is also
FAMILY-UNIFORM-FRAGILE, that is again a methodology win — the machine will have
cheaply ruled out the three most-cited crypto cross-sectional families on this
universe — and the rotation moves to value (data-gated) or a regime/blended track.

---

## Open questions (for the architect M-T1)

- **Q-CARRY-1 (seam):** how does the funding series reach the strategy's score
  computation — extend `Bar` (high blast radius, touches anchored bootstrap shape),
  a parallel `(Symbol, Timestamp) → funding` injection alongside `bars_override`
  (analyst lean — lowest blast radius), or a new `CarryStrategy` struct (forces
  `run_path` generic/`dyn`, risks the 87 anchors — the constraint that forced MR to
  be enum-on-config)? Weigh against anchor preservation.
- **Q-CARRY-2 (the headline — long-only vs market-neutral):** ratify framing (a)
  long-only directional carry-tilt for v0.1.0 (analyst Recommended) vs framing (b)
  market-neutral long/short funding harvest (the durable target, +2-3 days for the
  short-side engine). Materially changes signal + integration + apples-to-apples
  comparability.
- **Q-CARRY-3 (bootstrap funding resampling — the crux):** confirm the funding
  series is resampled by the SAME `idx_seq` as the returns (analyst design in
  § R-CARRY.7) via an optional `GeneratedPath.funding_by_symbol` field — vs any
  alternative the architect sees. This is the load-bearing integration decision.
- **Q-CARRY-4 (carry-score floor / normalization):** the funding score has no vol
  denominator (unlike `score_vol_adjusted_return`). Does the carry score need any
  normalization (e.g. by realized vol, to make it risk-adjusted) or is the raw
  trailing-mean funding the score? Affects the θ-grid `vol_floor`-equivalent.
- **Q-CARRY-5 (θ-grid lock):** ratify or revise the PROPOSED 6-cell carry grid and
  LOCK it before the tester anchors (per the MR precedent — the grid IS the hashed
  anchor input).

---

## Scope & honesty (no overclaim)

- This brief recommends a family and scopes the design + integration size; it
  commits no code and triggers no engine run. Reversible per the orchestrator's
  scoping. No engine changes, no build, no experiments were performed.
- The integration-size estimate (~4.5-7.5 days) is the analyst's honest read from
  the code (the bootstrap, `run_path`, `realdata.rs`, the funding parquet schema);
  the architect/developer should challenge it — in particular the § R-CARRY.7
  funding-through-bootstrap design is the riskiest piece and may be larger.
- The robustness axis judges **resampled real 2023-FY history** only — it cannot
  speak to a funding regime 2023 never contained (inherited scope limit).
- No alpha is claimed. This is uncertainty quantification of a candidate strategy,
  not prediction (inherited framing). The +1.74 buy-and-hold bar is the honest
  benchmark carry must clear to matter.
- The sign convention (R-CARRY.2) is confirmed against the Binance USDⓈ-M funding
  documentation; the developer MUST re-verify it against a sample of the banked
  funding data (a positive-funding period should correspond to a perp trading above
  its index) before locking the carry direction — a sign error silently inverts the
  strategy.

---

## Implementation
_developer fills this_

## Verification
_tester links to reports here_

## Changelog

- 2026-05-31 (analyst, rotation-scoping): drafted the carry-strategy feature brief
  as the **pre-registered rotation target** after BOTH price families
  (momentum + MR) came back FAMILY-UNIFORM-FRAGILE on the turnover-killer.
  **Signal (R-CARRY.1-2):** cross-sectional trailing-mean funding rank; LOAD-BEARING
  sign convention confirmed against Binance docs (positive funding → longs pay
  shorts; harvest the PAID side; harvest direction is opposite-sign to the rate) —
  a mandatory day-1 sign-assertion falsifier guards it. **Reuse assessment
  (R-CARRY.4):** carry is NOT a `Direction` variant like MR — momentum/MR share the
  *same price input* (1-line negation), carry needs a *different input* (funding)
  the engine does not load; reuses `top_k_long` + long-only sizing verbatim (~40-50%)
  but needs a new `ScoreSource::FundingCarry` + funding threading. **Integration
  (PROBLEM 2, R-CARRY.5-8) sized honestly at ~4.5-7.5 days (≈3-5× MR):** funding
  parquet loader (`funding_root`, SMALL-MED), 8h→1h as-of forward-fill (SMALL),
  **the crux — carry funding THROUGH the block bootstrap by the SAME shared index
  as the returns (MED-LARGE)** because `BlockBootstrapPathGen` discards real calendar
  time, so the carry-data brief's naive timestamp forward-fill would silently
  decouple price and funding, and funding-cashflow accrual in `run_path` (SMALL-MED,
  the mechanism that makes carry a distinct return source — CLAUDE.md no-op-overlay
  gated). **Headline open question Q-CARRY-2:** recommended framing (a) long-only
  directional carry-tilt for v0.1.0 (cheap, apples-to-apples vs the +1.74 bar) over
  framing (b) market-neutral long/short (the durable target but +2-3 days short-side
  engine — durable-over-quick exception: validate the signal before building the
  short-side infra). **Day-1 BOTH-axes gate + divergence falsifiers (R-CARRY.9-10):**
  carry-vs-price selection/equity divergence + the funding-cashflow non-no-op
  falsifier (the v3-vol-overlay analogue) + sign-assertion + no-look-ahead +
  two-run byte-identity, on top of the reused C2/C3/control/anti-cherry-pick.
  **Determinism (R-CARRY.11):** the funding path is additive/defaults-off → 87
  anchors hold by construction; +1 carry θ-surface anchor (87→88) under the
  `mc-robustness-2026-06` lane; likely a small real ADR-0051 § D6.6 amendment (the
  shared index now governs a second series — a genuinely new mechanism, unlike MR's
  pure config-level variation). PROPOSED 6-cell θ-grid spanning the
  lookback/rebalance(turnover)/K axes (architect LOCKS before anchoring). In-sample
  2023-FY apples-to-apples; 2024-FY OOS secondary. No code, no build, no engine run
  (reversible); no trace.toml / anchors.toml touch (orchestrator/tester own those).
