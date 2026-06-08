---
slug: perp-basis-signal-robustness
version: 0.1.0
status: presenter-done
owner: architect → developer
priority: P1
updated: 2026-06-08
---

# Perp-spot basis reversal — the FIRST LIVE signal of the post-OHLCV program: a cross-sectional basis-reversal arm, gated on whether it survives realistic taker fees — v0.1.0

> **The first non-flat result.** The active-trading robustness program closed
> exhaustively negative: four method families (x-sec momentum, MR, carry, TS
> absolute momentum) × three horizons (1h / 4h / daily) × a 35-name universe spike
> all came back FAMILY-UNIFORM-FRAGILE, dominated by passive buy-and-hold net of
> fees. The decisive failure was information-theoretic — the cross-sectional
> *price*-ranking channel carries ≈ 0 forward information (rank IC within ±0.07 of
> zero, no stable sign). The operator then chose the **new-data-domain** fork and a
> ~0.5-day research spike on the perpetual **basis** (the perp mark-vs-spot premium)
> came back **LIVE** — the program's first decision-grade-positive signal. This
> brief formalizes the build to convert that spike into a **robustness VERDICT**.
> Per the
> [basis spike](../dev-notes/new-data-domain-scoping-2026-06-05.md#basis-spike-results)
> (§ BS.0-BS.6, VERDICT **LIVE / MEDIUM-HIGH**).
>
> **This brief is analyst-altitude only.** It scopes the WHY, the requirements, the
> day-1 falsifiers, the backtest scenarios, and the framed design questions. It
> commits NO code, writes NO Design section, and authors NO tasks.md — the
> architect's M-T1 owns those next. The carry feature
> ([`carry-strategy`](../carry-strategy/feature.md)) is the line-for-line precedent
> (a non-OHLCV sidecar series + a `ScoreSource` arm + an as-of join through the
> bootstrap) and is referenced throughout.

---

## 0. Pre-registration & anti-cherry-pick (inherited verbatim, frozen now)

The basis arm is vetted under the **already-frozen** pre-registered decision rule
([`robustness-decision-rule-2026-05-30.md`](../dev-notes/robustness-decision-rule-2026-05-30.md) § 0)
— the same ruler that scored all four retired families across all three horizons.
Nothing about the rule is re-opened. The commitments carry over verbatim:

1. **The bands are frozen.** p5 Sharpe ≥ +0.5 ROBUST / < 0 FRAGILE; prob-of-loss
   ≤ 15% ROBUST / > 35% FRAGILE; p95 MaxDD ≤ ~50% ROBUST / > ~70% FRAGILE; p50
   Sharpe ≥ 1.0 ROBUST; P(Sharpe>1) ≥ 60% ROBUST. Composite = **worst primary band
   wins** (weakest-link). The basis surface is scored against these, not the
   reverse.
2. **Anti-cherry-pick by construction.** The θ-sweep reports the FULL surface + a
   family verdict and **crowns no argmax winner** (the FP-C3.5 renderer enforces
   this in code). A non-FRAGILE cell carries a `→ C5 deflation required` flag. A
   grid that picked argmax would inflate the false-ROBUST rate (`1 − 0.95^G`).
3. **Pre-flight void-if-fail.** Every basis report must print
   `generator: block-bootstrap-real` AND `bootstrap_mode: shared-index`, else the
   verdict is void.

**The buy-and-hold control is the bar the basis arm must clear to matter** —
+1.74 Sharpe (2023) / +1.10 Sharpe (2024), the same control every prior surface was
read against. A signal that does not beat simply holding the same coins net of fees
on this universe is not worth promoting, however internally "robust." **For a
reversal arm the binding question is whether ANY positive net-of-fee edge survives
at all** — see § The fee-sweep, the load-bearing gate.

---

## Why

### Why the basis, and why now (the spike found the first LIVE signal — not a hunch)

The post-OHLCV program asked: once the price bars are exhausted, where can a signal
this data cannot express come from? The basis spike (§ BS.0-BS.6) answered it for
the cheapest structurally-new series — the perpetual premium index
`(markPrice − indexPrice) / indexPrice`, natively on the hourly grid — and the
answer is **LIVE**, clearing all four pre-registered falsifiers:

| Spike falsifier (pre-registered § 5) | Result | Verdict |
|---|---|---|
| **Basis IC ≈ 0?** — cross-sec rank-IC of trailing basis → fwd-return | **NO** — IC is ≠ 0, **NEGATIVE**, and **grows with horizon** (L=60: −0.099/−0.081; L=168: −0.112/−0.069), same sign **both years** | **LIVE** (past ±0.03, sign-stable) |
| **Basis ≈ a price transform?** — corr(basis, OHLCV-mom) | +0.02 … +0.07 at the signal-bearing L=9-24 horizons | **orthogonal** (not OHLCV) |
| **Basis ≈ funding's dead twin?** — corr(basis, funding) | level corr **+0.47 (2023) / +0.66 (2024)** — moderate, **NOT ≈ +1** | **distinct** (~22-55% shared variance; the hourly basis retains 45-78% funding discards) |
| **No-look-ahead** — causal trailing vs leaked contemporaneous | causal ≠ leaked at every horizon; leaked **flips POSITIVE** where causal is NEGATIVE | **causal** (the predictive sign is past-only, not a leak artifact) |

This is materially different from every prior family:

- **It is the first non-zero cross-sectional IC the program has measured.** The
  *price* rank channel was ≈ 0 (§ M4, rank IC within ±0.07, no stable sign); the
  *basis* rank channel is −0.08 to −0.11 over the 2.5-day-to-1-week horizon,
  **negative and sign-stable in BOTH 2023 and 2024**. The ranking channel is dead
  for price but alive for the basis.
- **It is orthogonal to OHLCV** (corr +0.02-+0.07 at the signal horizons) — a
  genuinely different information source, not a fourth re-skin of the price bars.
- **It is only MODERATELY redundant with funding** (+0.47/+0.66), refuting the
  scoping note's honest-prior worry ("the basis is funding's dead twin, skip it").
  Funding came back FAMILY-UNIFORM-FRAGILE; the basis carries a sign-stable reversal
  IC funding lacks. The basis is a genuinely different and better-behaved signal.

The economic channel is sensible: the basis is a direct readout of **leveraged
positioning pressure**. When leveraged longs crowd in, the perp trades rich to spot
(high positive basis); those crowded-long names subsequently **underperform** the
cross-section as the crowd unwinds. The negative IC IS that reversal. The load-
bearing direction is therefore to tilt **AGAINST** the basis: high-basis →
underweight/short, low-basis → overweight.

### What we are actually buying with this build (the honest framing)

The spike's |IC| ~ 0.10 is the **GROSS information ceiling, NOT net P&L.** Three
honest caveats keep the spike at MEDIUM-HIGH, not HIGH, and they define exactly
what this build must resolve:

1. **A raw rank IC is an upper bound, not a tradable edge.** The price rank channel
   was ≈ 0 and *still* produced FRAGILE sweeps once the engine, the BH control, and
   the frozen § 0 weakest-link composite were applied. A −0.10 basis rank IC is
   materially better, but it MUST still clear the BH bar and the frozen rule in the
   real block-bootstrap sweep — which the spike could not do. **This build is the
   first time the program's first non-zero signal goes through the machine.**
2. **It is a REVERSAL signal → it is the most fee-sensitive class the program has
   built.** A reversal arm rebalances frequently (it constantly chases the names
   that just moved against their basis). The price families died on the
   turnover-killer (fee-bleed converted a +1.74 capturable drift into a break-even-
   at-best loss machine). A reversal arm is *structurally more exposed* to that
   exact killer than the momentum/MR families were. **This is the make-or-break.**
3. **The verdict is decision-grade either way.** A ROBUST result is a new product
   direction — the first survivable active signal on this universe. A FRAGILE
   result (the likely failure mode — the reversal information exists but is
   un-harvestable net of fees) **retires the entire derivatives-positioning family
   with finality** (price-rank + funding + basis all dead net of fees) and routes
   the next dollar to the genuinely-orthogonal **on-chain** domain (§ 3 domain B of
   the scoping note) with full justification — the pre-registered next domain.

So we are buying a **clean, anchored, decision-grade answer to one question:**
*does the basis-reversal signal beat passive buy-and-hold net of realistic Binance
taker fees, or does it die — and which?* The build is the carry pattern (the data is
already fetched and pinned, the sidecar-loader template is written), so the cost is
integration, not invention.

### The honest prior (what would make the basis arm FRAGILE too)

The basis arm is the best a-priori shot the post-OHLCV program has had, NOT a
guarantee. State the failure modes up front so the verdict is read honestly:

- **Fee-bleed (the single most likely killer).** A −0.10 gross IC is a thin edge;
  a reversal arm's turnover can easily consume it. If the signal dies at realistic
  Binance taker fees (the spike explicitly flagged this as the likely failure mode),
  that is the verdict. **The build front-loads this as the fee-sweep gate (below).**
- **Reversal-signal decay / crowding already-unwound.** The cross-sectional basis
  rank predicts the *next* relative move only if today's high-basis names stay
  crowded long enough to mean-revert on the strategy's cadence. If the crowd unwinds
  faster than the rebalance can capture, realized P&L is far below the signal — the
  basis analogue of the funding-mean-reversion killer that retired carry.
- **The robustness axis judges resampled real 2023/2024 history only** — it cannot
  speak to a basis regime those two years never contained (inherited scope limit).

If the basis arm ALSO comes back FAMILY-UNIFORM-FRAGILE despite the −0.10 IC, that
is **again a methodology win** (the machine will have cheaply ruled out the last and
strongest member of the derivatives-positioning family) and the pre-registered next
domain is **on-chain**. The brief does not overclaim a basis edge.

---

## Requirements

> **Naming:** requirements are tagged **R-BR.\*** (BR = Basis-Reversal). Each maps
> to a carry R-CARRY.\* sibling where the carry build is the precedent, so the
> architect can lift the proven design. The load-bearing SIGN (R-BR.2) and the
> FEE-SWEEP (R-BR.LOAD) are called out as the two things that must be exactly right.

### R-BR.1 — Signal: cross-sectional basis rank (trailing-mean basis over a lookback)

_(carry sibling: R-CARRY.1)_ The basis score for symbol `s` at time `t` is the
**trailing-mean basis** over a lookback `L` (in bars), ranked cross-sectionally:

```text
basis_score(s, t) = mean( basis[s, τ] for τ in the last L bars STRICTLY before t )
```

where `basis[s, τ]` is the premium-index close `(markPrice − indexPrice)/indexPrice`
at bar `τ` (the `basis_close` column of `data/binance-basis`). The trailing mean
smooths the noisy single-bar premium into a persistent cross-sectional signal — the
basis analogue of momentum's `score_vol_adjusted_return`. Names are ranked by this
score and the top/bottom-K selected per the SIGN in R-BR.2.

**The signal is CROSS-SECTIONAL (relative basis rank), NOT time-series.** The spike
(BS.2b) showed the own-asset time-series basis channel is weak and **sign-unstable**
across years (L=168 flips −0.055 ↔ +0.054); the cross-sectional rank channel is
where the −0.10 IC lives. The arm ranks the 10 names by trailing-mean basis — it is
NOT a per-asset absolute-basis long/flat rule.

_Acceptance: the basis score is a pure function of the basis series over the
lookback; a unit test on a synthetic basis series confirms the trailing-mean ranking
orders names by their average basis._

### R-BR.2 — The SIGN CONVENTION (LOAD-BEARING — tilt AGAINST the basis)

**This is the most error-prone part of the feature and must be exactly right.** The
spike measured a **NEGATIVE** cross-sectional rank IC (BS.2a): names with the
**highest** trailing basis subsequently **UNDERPERFORM** the cross-section. This is
a **reversal** effect (crowded-long perps mean-revert). Therefore:

| Trailing basis rank | Subsequent relative return | The arm holds | Position |
|---|---|---|---|
| **HIGH** (richest perp premium, crowded long) | UNDER-performs | the **bottom** of the rank | underweight / SHORT |
| **LOW** (cheapest perp premium) | OUT-performs | the **top** of the rank | overweight / LONG |

So the basis-reversal direction is: **tilt AGAINST the basis — short/underweight the
high-basis names, long/overweight the low-basis names.** Mechanically (mirroring the
carry sign trick D-CARRY.1 where the sign lives in the score, not in `Direction`),
the natural implementation is a score of **`−trailing_mean(basis)`** so the
**lowest-basis** name floats to the TOP of the unchanged descending `top_k_long`
selector — then a long-tilt on the low-basis names IS the reversal.

> **A naive sign error here silently inverts the entire strategy** — it would turn a
> basis-*reversal* arm into a basis-*momentum* payer (long the crowded-long names),
> the opposite of what the spike found, and would chase precisely the names that
> underperform. **R-BR.2 (the SIGN-assertion falsifier, day 1) is mandatory:**
> construct a synthetic universe with a known-high-basis name and assert the arm
> takes the UNDERWEIGHT/bottom (or short, depending on the framing the architect
> ratifies) side of it — RED on a sign flip.

> **OPEN QUESTION Q-BR-2 (operator/architect — load-bearing framing).** Carry
> faced the identical fork (R-CARRY.2 / Q-CARRY-2) and shipped **long-only** at
> v0.1.0 because the v1 engine is long-only (`run_path` opens only `Side::Buy`;
> `k_short = 0` enforced in the config loader). The reversal arm has the SAME
> constraint and a structurally-equivalent answer:
>
> - **(a) Long-only basis-reversal tilt (Recommended for v0.1.0 — the durable
>   choice, exactly as carry ratified).** Long the **lowest-basis** names only (the
>   reversal-favored leg: cheapest perp premium → outperforms). Reuses the long-only
>   solvency-guarded `run_path` + `top_k_long` sizing verbatim → apples-to-apples
>   with the four retired families AND the carry #88/#89 anchors. Honest caveat: a
>   long-only reversal arm captures only the long leg of the cross-sectional spread,
>   so its edge is the weaker half of the full long/short reversal — but it answers
>   the load-bearing question *"does tilting toward low-basis names beat buy-and-hold
>   net of fees?"* cleanly and at a fraction of the cost.
> - **(b) Market-neutral long-low/short-high basis reversal (the v0.2.0 durable
>   follow-on, larger).** Long low-basis, short high-basis, dollar-neutral — the full
>   cross-sectional reversal that isolates the basis spread from market beta and
>   captures BOTH legs (where the spike's full −0.10 IC lives). Requires the short-
>   side engine (new short-sizing path, short-solvency accounting, `k_short > 0`
>   un-gated) — a materially larger build that would NOT be apples-to-apples with the
>   long-only family.
>
> **Analyst recommendation: ship (a) long-only at v0.1.0** as the cheap, apples-to-
> apples first read against the +1.74/+1.10 BH bar, and treat (b) as the v0.2.0
> durable follow-on IFF (a) shows a non-FRAGILE, fee-surviving cell worth the short-
> side engineering. **Rationale for putting Recommended on (a) (the durable-over-
> quick exception, identical to carry's):** the load-bearing scientific question —
> *"does the basis-reversal signal beat buy-and-hold net of realistic fees?"* — is
> answerable by (a) at a fraction of (b)'s cost, and (a) keeps the engine comparison
> clean. Building the short-side engine BEFORE knowing whether the signal survives
> fees would be durable infrastructure on an unvalidated premise — the opposite of
> durable. And for a REVERSAL arm the fee verdict is the gating risk: if (a) dies on
> fees (the likely outcome), (b) — which rebalances both legs and doubles the
> turnover-exposed surface — almost certainly dies too, so the short-side engine is
> never owed. **This is the architect's M-T1 call to ratify.**
> *(If-budget-tightens: there is no cheaper-than-(a) framing that still produces an
> anchored verdict; (a) IS the floor. The only sub-(a) downgrade is dropping the
> θ-sweep to a single config — see § Backtest Scenarios — which is NOT recommended,
> it forfeits the parameter axis.)*

### R-BR.LOAD — THE FEE-SWEEP (the make-or-break gate; REGRESSION-blocked)

**This is the load-bearing requirement of the entire feature.** The spike's
|IC| ~ 0.10 is the GROSS information ceiling; a reversal arm rebalances often, so
**fees are the make-or-break, not a nice-to-have.** The price families died on
exactly this (turnover/fee-bleed), and a reversal arm is the *most* fee-exposed
class the program has built. The build MUST therefore **front-load a fee-sweep on
day 1**:

- **Sweep the taker fee across a range that brackets free → realistic → punitive:**
  e.g. **{0, 2, 5, 10} bps** taker fee (the exact ladder is an architect open
  question — Q-BR-4 — but it MUST include 0 bps as the gross-edge ceiling and a
  realistic Binance taker level, ~5 bps, as the decision point). The current sweep
  hardcodes `slippage_bps: 2` + `taker_fee_bps: 4` per path
  (`param_robustness_sweep.rs:2409-2410`); the fee level must become a swept /
  parameterized axis for this arm.
- **Show, per fee level, whether the arm's best cell still beats the BH control and
  clears the frozen § 0 rule.** The deliverable is a **fee-sensitivity curve**: the
  net-of-fee edge as a function of taker fee, read against the (fee-invariant) BH
  bar.
- **IF IT DIES NET OF REALISTIC FEES, THAT IS THE VERDICT** — a decision-grade
  negative that retires the derivatives-positioning family (per § Why caveat 3). The
  fee-sweep is the GATE that distinguishes "the signal is real AND harvestable"
  (ROBUST, fee-surviving) from "the signal is real but un-harvestable" (FRAGILE on
  fees) — and the latter is the most likely outcome the spike itself flagged.

This is **not** a robustness band by itself; it is the axis along which the §-0
verdict is read for a reversal arm. A basis arm that clears every § 0 band at 0 bps
but loses to BH at 5 bps is **FRAGILE-on-fees** and is NOT promoted. The architect
designs the exact mechanism (a `--taker-fee-bps` sweep axis vs a fixed pin + a
separate fee-sensitivity report — Q-BR-4) but the gate itself is non-negotiable.

_Acceptance: the anchored deliverable includes a per-fee-level net-of-fee read of
the best cell vs the BH control at minimum at {0, 5} bps (the gross ceiling and the
realistic decision point); the FRAGILE-on-fees case is an explicit, reported
verdict, not a silent omission._

### R-BR.3 — Basis sidecar loader (mirror `funding_data.rs`)

_(carry sibling: R-CARRY.5)_ A **basis-data source** mirroring the proven
`crates/backtest/src/funding_data.rs` (`FundingDataSource`):

- Load `data/binance-basis/<SYM>/<YEAR>/<MM>.parquet` (schema confirmed on disk:
  `open_time` Int64 ms, `close_time` Int64 ms, `basis_open/high/low/close` Utf8
  signed decimal strings — `basis_close` is the basis at each bar).
- Parse `basis_close` via `rust_decimal::Decimal` (never `f64` — ADR-0003).
- **Verify the `data/binance-basis/REVISION.toml` pin** against the locked aggregate
  SHA **`aa72409aa0f856960385a823bc61be1b8274e84f658439b65e5d1b1b1a48f1cd`** exactly
  as the funding loader verifies `bf1ede44…` (a `RevisionMismatch` error rejects
  runs on unverified data).
- The basis parquet is the SAME size class as OHLCV (~8,760 bars/symbol-yr, hourly)
  — unlike funding's sparse 8h cadence. **This is a simplification vs carry:** the
  basis is natively on the 1h bar grid, so the 8h→1h forward-fill (carry's
  Sub-problem B) is a trivial identity at 1h. The as-of join still applies for
  no-look-ahead (R-BR.5).

_Acceptance: the loader reads the 10-symbol × 2-year basis tree, rejects a SHA
mismatch, and yields a `Decimal` basis series per symbol on the hourly grid; a unit
test pins the schema + the signed/negative-basis parse (negative basis = perp below
spot)._

### R-BR.4 — Universe + data pins (the SAME 10 large-caps, for direct comparability)

_(carry sibling: R-CARRY.12)_ The universe is the **ORIGINAL 10 large-caps** under
`data/binance` (`ADAUSDT, AVAXUSDT, BNBUSDT, BTCUSDT, DOGEUSDT, DOTUSDT, ETHUSDT,
LINKUSDT, SOLUSDT, XRPUSDT`), pin **`3a8b96c4…`** (OHLCV) + the matching basis side
`data/binance-basis` pin **`aa72409a…`**. Keeping the universe identical to the four
retired families + carry makes the basis result **directly comparable** and reuses
the banked OHLCV with zero new universe risk. In-sample = 2023-FY (apples-to-apples
with the #86-#91 anchors); 2024-FY is the harder/fairer regime and is RUN + READ on
day 1 (the carry/horizon E1 precedent: both regimes gating, not 2024-as-afterthought).

### R-BR.5 — As-of basis join, strict no-look-ahead

_(carry sibling: R-CARRY.6)_ The basis at the open of bar `t` must use only the
basis **known at or before** `t`'s open. Per the spike's proven convention (BS.1):
**the basis at the open of bar `t` is `basis_close[t-1]`** (the premium-index close
of bar `t` is only known at `t+1h`); trailing signals use `[t−L, t)` past bars only.
This is the basis analogue of `funding_data.rs::funding_as_of` and was proven causal
in the spike by the `--leak-check` falsifier (BS.4: causal ≠ leaked at every
horizon; the leaked signal flips POSITIVE where the causal one is NEGATIVE).

_Acceptance: a no-look-ahead falsifier (R-BR.5, day 1) asserts a bar's basis score
uses only basis settled at-or-before its `open_ts`; shifting the basis series one bar
into the future changes the result — RED on revert (proving the join is causal)._

### R-BR.6 — Basis through the block bootstrap (mirror the carry shared-index crux)

_(carry sibling: R-CARRY.7 — THE crux)_ The robustness harness does NOT replay real
bars; it runs each path on a **shared-index block-bootstrap** resampling of the real
returns, emitting **synthetic timestamps** — real calendar time is discarded. So a
naive "forward-fill real basis onto the bootstrapped bars by timestamp" would
**decouple basis from price** (the bar's price came from real index `idx_seq[k]`, but
the naive basis fill would attach basis from a *different* real time).

**The correct design is the carry mechanism, already built and proven:** the basis
must be resampled with the **SAME `idx_seq`** as the returns. Carry solved this with
`GeneratedPath.funding_by_symbol: Option<Vec<Vec<Decimal>>>` gathered by the same
`idx_seq` in the existing reconstruction loop (D-CARRY.7, "~15-line additive change,
ZERO new RNG draws, byte-identical anchors by construction"). **The basis is the
same shape** — a per-symbol per-return-step series co-resampled by the shared index.
The architect decides whether to **reuse the existing `funding_by_symbol` channel**
(the basis rides the same `Option<Vec<Vec<Decimal>>>` field — the cheapest path,
since basis and funding are never used simultaneously in v0.1.0) **or add a sibling
`basis_by_symbol` field** (Q-BR-3). Either way the shared-index co-resample is the
proven D6.6 mechanism, not new methodology.

_Acceptance: the basis is resampled by the same `idx_seq` as the returns (the carry
D-CARRY.7 mechanism); a two-run byte-identity test (R-BR.7) confirms zero new RNG /
no unordered fold; the additive/defaults-off gate keeps the 99 anchors byte-identical
(R-BR.8)._

### R-BR.7 — Day-1 falsifiers (each RED-on-revert; modeled on carry R-CARRY.2/6/10a/10b)

_(carry siblings: R-CARRY.10a/10b/2/6 + two-run identity)_ Per CLAUDE.md (every
sizing-modifier ships a baseline-equity-divergence e2e from day 1) and the carry
precedent, the basis arm ships these falsifiers **on day 1, before the anchored run**:

1. **The FEE-SWEEP gate (R-BR.LOAD).** The fee-sensitivity read is produced and the
   FRAGILE-on-fees case is an explicit reported verdict. (This is the headline gate,
   not a unit test — see R-BR.LOAD.)
2. **SIGN-assertion (the basis-reversal-not-momentum falsifier).** A synthetic
   universe with a known-high-basis name; assert the arm UNDERWEIGHTS / shorts it
   (the reversal side) and a **sign flip → a basis-MOMENTUM payer → RED**. This is
   the load-bearing guard on R-BR.2.
3. **Baseline-equity-divergence e2e (CLAUDE.md non-negotiable).** The basis arm's
   output equity MUST diverge from the un-tilted baseline equity by ≥ 1 bp when the
   basis decision variable is non-trivial — guards against a v3-vol-overlay-style
   no-op where the signal is computed but never applied. Pattern:
   `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`;
   carry sibling: `crates/backtest/tests/carry_divergence_e2e.rs::r_carry_10a_*`.
4. **Basis-signal-non-no-op.** Force the basis signal to a constant (no
   cross-sectional dispersion) and assert the arm's selection/equity **collapses to
   the baseline** (Δ < ε) — proving the basis is load-bearing, not decorative
   (the carry R-CARRY.10b analogue).
5. **No-look-ahead (R-BR.5).** The basis as-of join is past-only; shifting the basis
   one bar into the future changes the result — RED on revert (carry R-CARRY.6).
6. **Two-run byte-identity.** Run the small-N basis sweep twice at the same
   `ensemble_seed`; assert identical `report_body_hash` — catches any unordered fold
   in the basis resampling or the renderer (carry `carry_two_run_byte_identity`).

### R-BR.8 — Determinism & anchoring: the 99 anchors hold byte-identical (additive/defaults-off)

_(carry sibling: R-CARRY.11)_ Every new seam (the basis loader, the basis-through-
bootstrap co-resample, the basis `ScoreSource` arm, the fee axis) is **additive and
defaults-off**, gated on a basis source being present — so **every momentum / MR /
carry / TS / horizon run is byte-identical by construction**, exactly as carry's
`funding_override` and TS's `SelectionMode` were. **The 99 existing anchors
(`spec/anchors.toml`) stay byte-identical** — this is a hard gate, NOT a goal:

- The `ScoreSource` default is `VolAdjustedReturn` (serde-default); a new basis arm
  must NOT change the default-path serialization.
- The fee axis must default to the existing `slippage_bps: 2` / `taker_fee_bps: 4`
  so non-basis runs are unchanged; the fee-sweep is opt-in for the basis arm.
- The shared-index basis co-resample consumes ZERO new RNG draws (the carry D6.6
  proof transfers: the gather rides the already-materialized `idx_seq`).

**Anchor unit = the basis θ × fee surfaces.** New basis anchors are added **after**
the developer's anchored run (the tester locks them at the M-TEST PASS gate, per the
carry/TS/horizon precedent); the grid + N + fee ladder are locked at design time by
the architect (the MR/carry/TS precedent — the grid is a hashed body field). The
existing 99 anchors are NOT re-locked. **Anchored report files in `spec/*/reports/`
remain byte-immutable** (ADR-0038 § D6).

### R-BR.9 — Decimal money; no f64 in the signal or the P&L

_(inherited)_ The basis (`basis_close`), the trailing-mean score, the rank, and any
sizing/cashflow stay `rust_decimal::Decimal` end-to-end (ADR-0003). No `f64` in the
money path. (Note: a reversal arm v0.1.0 long-only does NOT need a funding-style
cashflow accrual — the basis is a *selection/sizing* signal, not a cash settlement —
so carry's Sub-problem D does NOT transfer. The architect confirms whether the basis
arm has any cashflow component or is purely a selection tilt — Q-BR-1.)

---

## Requirements summary (consolidated)

- **R-BR.1** — Signal = trailing-mean basis rank over a lookback L (bars), pure
  function of the basis series; CROSS-SECTIONAL (relative rank), not time-series.
- **R-BR.2** — SIGN (LOAD-BEARING): tilt AGAINST the basis (high basis →
  underweight/short; low basis → overweight/long); a day-1 sign-assertion is
  mandatory (a sign flip = a basis-momentum payer → RED).
- **R-BR.LOAD** — THE FEE-SWEEP gate (make-or-break, REGRESSION-blocked): sweep the
  taker fee ({0, 2, 5, 10} bps proposed), show whether the edge survives realistic
  Binance fees; if it dies net of fees, that IS the verdict.
- **R-BR.3** — Basis sidecar loader mirroring `funding_data.rs` (load
  `data/binance-basis`, pin `aa72409a…`, Decimal parse, REVISION-verify).
- **R-BR.4** — The SAME 10 large-caps (OHLCV pin `3a8b96c4…` + basis pin
  `aa72409a…`); 2023 in-sample + 2024 gating, both regimes day 1.
- **R-BR.5** — As-of basis join (`basis_close[t-1]` at open of `t`), strict
  no-look-ahead, proven causal by the spike's leak-check.
- **R-BR.6** — Basis through the block bootstrap via the SAME `idx_seq` (the carry
  D-CARRY.7 / ADR-0051 § D6.6 shared-index co-resample — proven, additive).
- **R-BR.7** — Day-1 falsifiers (each RED-on-revert): fee-sweep gate, sign-assertion,
  baseline-equity-divergence e2e, basis-signal-non-no-op, no-look-ahead, two-run
  identity.
- **R-BR.8** — Additive/defaults-off → the 99 existing anchors hold byte-identical;
  new basis anchors locked by the tester after the anchored run.
- **R-BR.9** — Decimal money throughout; no f64 in the signal or P&L.

---

## Design

_Architect M-T1 (2026-06-05). All six OQs (Q-BR-1..6) resolved + justified below.
Carry (`carry-strategy`) is the line-for-line precedent — the basis arm is the
**cheaper sibling** (no funding-style cashflow; native-1h basis = no cadence
forward-fill). The headline new work is the **fee-sweep axis** (D-BR.LOAD), modeled
on the proven `--horizon` axis (D-HR.5 — a CLI-driven, body-hashed, namespace-routing
parameter). The 99 anchors hold byte-identical by construction (D-BR.8). The SIGN is
load-bearing and lives in ONE place (D-BR.1). ADR-0051 § D6.9 amendment registers the
basis namespace + the fee axis._

> **True-size re-assessment (the M-T1 mandate).** The analyst's "cheaper than carry's
> ~4.5–7.5 d" prior **HOLDS and tightens to ~3–5 d**. Three of carry's sub-problems
> shrink or vanish: (B) the 8h→1h forward-fill is an **identity** (basis is native 1h);
> (D) the funding-cashflow accrual **does not transfer** (basis is a selection signal —
> Q-BR-1); (C) the bootstrap co-resample is **reused, not rebuilt** (Q-BR-3 — the basis
> rides the existing `funding_by_symbol` channel). The genuinely-new work is the basis
> loader (D-BR.3, a near-mirror of `funding_data.rs`) + the **fee-sweep axis** (D-BR.LOAD,
> the one new parameter). No intractability; PROCEED to build (gated on operator go).

### D-BR.0 — Q-BR-2 RESOLVED: ship framing (a) long-only basis-reversal tilt (v0.1.0)

**RATIFIED: framing (a) — long-only basis-reversal tilt.** v0.1.0 longs the
**lowest-basis** names (the reversal-favored leg: cheapest perp premium → outperforms),
sized by the unchanged `top_k_long`. The market-neutral long-low/short-high reversal
(framing (b)) is deferred to v0.2.0 IFF (a) shows a non-FRAGILE, fee-surviving cell
worth the short-side engine. This is the exact transfer of carry's D-CARRY.0 ruling,
for the same three reasons:

1. **Reuses the solvency-guarded long-only engine verbatim.** `run_path`
   (`montecarlo.rs:92`) only ever opens `Side::Buy` (line 204) under the Bug-B
   solvency cap; `top_k_long` (`selector.rs`) is exactly the ranked top-K the basis arm
   needs. The config loader enforces `k_short = 0` (`config.rs:288`). Framing (b)
   requires a new short-sizing path + short-solvency accounting + `k_short > 0` un-gated
   — a materially larger build on an **unvalidated** premise.
2. **Apples-to-apples with the 99 anchors + the four retired families + carry #88/#89.**
   All families run the identical long-only engine on the identical resampled paths; any
   difference is the SIGNAL, not the engine. A long/short engine breaks that comparison.
3. **The fee verdict is the gating risk — (a) answers it cleanly.** For a REVERSAL arm,
   if (a) dies on fees (the likely outcome the spike flagged), (b) — which rebalances
   BOTH legs and doubles the turnover-exposed surface — almost certainly dies too, so the
   short-side engine is never owed. Building it first would be durable infrastructure on
   an unvalidated premise — the opposite of durable.

**Honest caveat carried into the verdict (NOT suppressed):** framing (a) captures only
the **long-low-basis leg** of the cross-sectional reversal spread; the spike's full
−0.10 IC lives in the full long/short cross-section. The verdict must be read as *"does
the long-low-basis tilt beat BH net of fees,"* NOT *"does the full reversal spread
work."* This is the deliberate apples-to-apples / fee-verdict-first scope (Assumption 2).

### D-BR.1 — Q-BR-1 RESOLVED: a single `ScoreSource::BasisReversal` with the SIGN baked in (NO cashflow)

**RATIFIED: option (ii) — add ONE new `ScoreSource::BasisReversal` arm to the existing
`ScoreSource` enum** (`crates/strategy/src/cross_sectional/config.rs:48`, sibling to
`VolAdjustedReturn` (default) and `FundingCarry`), with the load-bearing minus baked
**into the score** (`basis_score = −trailing_mean(basis)`). This is the carry pattern
exactly (D-CARRY.1): the reversal sign lives in **one auditable place**, guarded by the
day-1 sign-assertion falsifier (D-BR.7 #2). Option (i) — `ScoreSource::Basis` +
`Direction::Reversion` — is REJECTED:

| Option | Decision | Why |
|---|---|---|
| (i) `ScoreSource::Basis` + `Direction::Reversion` to express the sign | **REJECTED** | Splits the load-bearing sign across **two** fields (the score source AND the direction enum). The reversal is then `score = +trailing_mean(basis)` then `Direction::Reversion` negates it — two places to get wrong, two places to audit. A reader must hold both in their head to know which names the arm holds. Carry already proved the "sign-in-the-score" idiom is the safer one. |
| (ii) single `ScoreSource::BasisReversal`, `−trailing_mean(basis)` baked in, `Direction::Momentum` (identity) | **RATIFIED** | The minus is in ONE place (`basis_reversal_score`, the carry-twin of `carry_score`). The lowest-basis name floats to the TOP of the unchanged descending `top_k_long`; a long-tilt on it IS the reversal. The name `BasisReversal` (not `Basis`) makes the sign self-documenting — there is no sign-neutral "basis" arm to confuse it with. Guarded by the sign-assertion falsifier (RED on a flip → a basis-MOMENTUM payer). |

**Cashflow: NONE. Carry's Sub-problem D does NOT transfer (R-BR.9 confirmed).** The
basis is a **selection/sizing signal**, not a cash settlement. There is no "basis
payment" that hits the equity curve — the arm earns (or loses) purely by holding the
**price** of the low-basis names it selects. Mechanically: the funding-cashflow accrual
block in `run_path` (`montecarlo.rs:307-359`, gated on `funding_override.is_some()`) is
**NOT entered** for the basis arm, because the basis arm threads its sidecar **only to
the strategy's score** (via the `funding_map` injection seam — see D-BR.3), and leaves
the `run_path` accrual path untouched. Concretely:

- The basis sidecar IS injected into the strategy's `funding_map` (reused as the basis
  lookup — D-BR.3) so `basis_reversal_score` can read it.
- The basis sidecar is **NOT** passed as `TcnScenarioInput.funding_override` to
  `run_path` for accrual. The accrual gate stays `None` for the basis arm → the
  `cash += notional × (−rate)` block is never entered → the basis arm's P&L is pure
  price-of-selection, exactly as intended for a selection signal.

> **This is the one place the basis design diverges from a literal carry clone.** Carry
> passes its sidecar to BOTH the score AND the accrual; the basis arm passes it ONLY to
> the score. The developer must wire the strategy-side injection **without** wiring the
> `run_path` accrual side. The D-BR.7 #4 basis-non-no-op falsifier (force the basis to a
> constant → selection collapses to baseline) is the guard that the score is load-bearing;
> the absence of a cashflow accrual is the design, not a bug — there is no
> "basis-cashflow non-no-op" test because there is no basis cashflow.

**The score (R-BR.1).** `basis_reversal_score(s, t) = −mean( basis[s, τ] for τ in the
last L bars STRICTLY before t )`, computed from the injected basis lookup keyed by
`(Symbol, open_ts)` on the synthetic bar grid. It is the basis twin of `carry_score`
(`momentum.rs:305`) — a trailing ring over the sidecar series — EXCEPT the ring counts
**price bars** (the basis is native 1h), not 8h settlements, so the ring is a plain
`L`-bar window. The minus is the R-BR.2 sign. Returns `None` until the ring holds ≥ L
bars (warm-up, excluded from the rank — identical to a warming-up momentum score).

### D-BR.LOAD — Q-BR-4 (the load-bearing half): the FEE-SWEEP mechanism = a `--taker-fee-bps` axis (one surface per fee level)

**RATIFIED: mechanism (i) — a swept `--taker-fee-bps` axis producing ONE anchored
surface per fee level**, modeled line-for-line on the proven `--horizon` axis (D-HR.5):
a CLI flag, defaulted to the legacy value (anchor-neutral), threaded into the per-path
`TcnScenarioInput`, and rendered as a **hashed body field** gated to non-default values.
Mechanism (ii) — a fixed pin + a separate best-cell fee-sensitivity report — is REJECTED
as a downgrade for a reversal arm, where the fee axis IS the verdict axis (below).

**Why (i) over (ii).** For a reversal arm, the fee-sweep is not a sensitivity sidebar —
it is the dimension along which the §-0 verdict is read (R-BR.LOAD). Reading the full
surface at each fee level (i) lets the verdict say *"the family survives to N bps"* with
the SAME anti-cherry-pick discipline (FP-C3.5, full surface, no argmax crown) applied at
every fee. Mechanism (ii) reads the fee axis at only the **best-θ** cell — which is
exactly the argmax the frozen § 0 rule forbids crowning, and would smuggle a cherry-pick
into the load-bearing gate. (i) is the durable, anti-cherry-pick-complete choice.

**The mechanism (the seam, precise).** The fee is hardcoded today at
`param_robustness_sweep.rs:2409-2410` inside `run_one_path_with_config`
(`slippage_bps: 2, taker_fee_bps: 4`). The change is additive:

1. **New CLI flag** `--taker-fee-bps <u32>` on `Args`, **default `4`** (the legacy
   taker value). `--slippage-bps <u32>` is ALSO added, **default `2`** (legacy slippage),
   so the fee point is fully specified; the fee LADDER sweeps `taker_fee_bps` (slippage
   held at its default unless the operator overrides). Mirrors `--horizon` defaulting to
   `1h`.
2. **Thread it** into `run_one_path_with_config` (a new `taker_fee_bps: u32` /
   `slippage_bps: u32` param, replacing the two hardcoded literals at 2409-2410). For
   momentum/MR/carry/TS/horizon runs the caller passes the defaults `2`/`4` → the
   `MatchConfig` is byte-identical → **the 99 anchors are byte-unchanged** (the literals
   `slippage_bps: 2, taker_fee_bps: 4` become `args.slippage_bps`/`args.taker_fee_bps`
   which default to `2`/`4`).
3. **Render it as a hashed body field, GATED to the basis arm + non-default fees.** Add a
   `| taker_fee_bps | {n} |` (and `| slippage_bps |`) row to the report body **only when
   `score_source == BasisReversal`** (the same gating idiom as the horizon row, which
   renders only when `is_horizon_run`). This keeps every existing 1h/non-basis body-SHA
   byte-identical while making the fee level part of the basis anchor's hashed identity
   (so the {0,2,5,10}-bps surfaces are four DISTINCT anchors, not four files that collide
   on the same SHA).

> **Anchor-neutrality proof for the fee axis.** The 99 existing anchors are produced by
> runs that pass `taker_fee_bps = 4, slippage_bps = 2` (the defaults) and are NOT the
> basis arm, so (a) their `MatchConfig` is the same literal as before, and (b) the new
> fee-body-row is gated `score_source == BasisReversal` → never rendered for them. Both
> the engine input AND the report body are byte-identical for every non-basis run. This
> is the identical discipline the `--horizon` axis used to add a body row without
> touching the 91 anchors it inherited.

### D-BR.2-LOCKED — Q-BR-4 (the grid half): the basis θ × fee surface — LOCKED

**LOCKED** (per the MR/carry/TS/horizon precedent — the grid + N + fee ladder are hashed
body fields, K3; changing any = a different surface = a different SHA). Held constant
across every cell: `score_source = basis_reversal`, `direction = momentum` (identity; the
sign lives in the score), `selection_mode = cross_sectional_top_k`, `exposure_cap = 0.50`,
`size = equal_weight`, `k_short = 0`, the 10-symbol universe, `ensemble_seed = 0xC0FFEE`,
`fill_seed = 0xC0FFEE`, generator = `block-bootstrap-real`, `bootstrap_mode =
shared-index`, revisions `3a8b96c4…` (OHLCV) + `aa72409a…` (basis), `N = 200`. **No
`vol_floor` cell** — the basis score has no vol denominator (the carry Q-CARRY-4 ruling
transfers: raw trailing-mean basis, no vol-normalization in v0.1.0); `vol_floor` stays at
its config default and is inert.

**The θ-axis (signal lookback) — LOCKED to the spike's signal-bearing band** (BS.2a; the
analyst's `{24, 60, 168}` ratified unchanged; **L=720 SKIPPED** as noise: n=11,
sign-flips across years). The lookback unit is **price BARS** (the basis is native 1h, so
1 lookback unit = 1 hour), passed literally as the strategy's `lookback_minutes` field
(reinterpreted as a bar count, exactly as carry reinterprets it as settlements):

| g | lookback L (bars) | K | drift | rebalance | role / hypothesis | turnover |
|---|---|---|---|---|---|---|
| 0 | 60 (2.5d) | 3 | 0.10 | 8h (480m) | **baseline basis θ\*** — the IC peak (−0.099/−0.081), low-churn cadence | low |
| 1 | 24 (1d) | 3 | 0.10 | 8h (480m) | short lookback — faster, more fee-exposed (IC −0.031/−0.022) | low-mid |
| 2 | 168 (1wk) | 3 | 0.10 | 8h (480m) | long lookback — IC peak / lowest-turnover corner (−0.112/−0.069) | low |
| 3 | 60 (2.5d) | 5 | 0.10 | 24h (1440m) | **deliberately-slow rebalance + wide K** (lowest-churn corner — the reversal arm's best fee shot) | **lowest** |
| 4 | 60 (2.5d) | 1 | 0.10 | 8h (480m) | narrow selection — top-1 lowest-basis name | low |
| 5 | 24 (1d) | 5 | 0.10 | 8h (480m) | shortest lookback + wide K — **highest-churn extreme** (the reversal fee-trap stress) | mid |

> **Why these cells (the reversal-fee diagnostic thesis).** A reversal arm's fee exposure
> is dominated by **turnover**, so the cadence/K axis is the most diagnostic. g0 is the
> IC-peak baseline at the natural low-churn cadence; g3 is the lowest-churn corner (the
> arm's best structural shot at surviving fees); g5 is the highest-churn extreme (the
> fee-trap stress). g1/g2 span the lookback (faster-vs-persistent). g4 narrows selection.
> The **rebalance default = 8h (480m)** mirrors carry's reasoning (a wide cadence reduces
> turnover bleed), with g3 deliberately slowed to 24h — the reversal arm's analogue of
> carry's lowest-churn corner. **The rebalance cadence is the basis arm's primary fee
> lever, and the grid spans it explicitly.**

**The fee-axis — the load-bearing gate (R-BR.LOAD):** taker fee ∈ **{0, 2, 5, 10} bps**
(LOCKED). `0` = the gross-edge ceiling (does the signal survive AT ALL with zero
friction?); `2` = a low-fee reference; `5` = the realistic Binance taker decision point
(the verdict fee); `10` = the punitive stress. The {0, 5} read is the R-BR.LOAD-acceptance
minimum; all four are run. Slippage held at the default `2` bps across the ladder (the fee
LADDER sweeps the taker leg only — the analyst's `taker_fee_bps` axis).

**N = 200/cell** (the carry/MR/TS/horizon tractable shape; the developer re-validates the
wall-clock before locking, per the C3 lesson — see D-BR.WALLCLOCK).

### D-BR.WALLCLOCK — the |L|×|K|×|fee|×N×regime budget is TRACTABLE (the C3-lesson gate)

The grid is a **6-cell θ-grid** (the table above; L×K×cadence are folded into the 6
cells, NOT a full cross-product) **× 4 fee levels × 2 regimes (2023, 2024)**. The fee
axis multiplies the SURFACE count, NOT the per-surface cell count — each fee level is a
separate sweep invocation that re-runs the 6×200 grid:

```
per surface  = 6 cells × 200 paths            = 1,200 backtests  (+ buy-and-hold control)
per regime   = 4 fee levels × 1,200            = 4,800 backtests
both regimes = 2 × 4,800                       = 9,600 backtests  → 8 anchored surfaces
```

**Per-path cost ≈ the carry/TS per-path cost** (same engine, same N, same universe; the
basis gather is the SAME O(n_bars) `Vec<usize>` index-gather as the funding gather, which
the carry M-DEV-7 measured as negligible). Carry's N=3 smoke ran in 1.7s for 18 paths →
~0.094 s/path; the carry N=200 surface extrapolated to **~2 min** and ran well within the
≲30-min gate. So:

```
per surface  ≈ 1,200 × 0.094 s   ≈ 113 s ≈ ~2 min   (matches the carry M-DEV-7 measurement)
per regime   ≈ 4 × 2 min          ≈ 8 min
both regimes ≈ 2 × 8 min          ≈ 16 min          ← TRACTABLE (under the ≲30-min gate)
```

**VERDICT: TRACTABLE.** ~16 min wall-clock for the full 8-surface fee × grid × regime
deliverable — comparable to the carry/horizon runs and under the gate. **No re-scope of
the fee axis is needed** — the analyst's economy fallback (read the fee axis at only the
best-θ cell) is NOT invoked; the full 6-cell surface is run at each fee level (the durable,
anti-cherry-pick-complete choice). The developer MUST re-confirm the per-path cost on the
canonical box at the small-N smoke before launching the 8 surfaces (the mandatory C3-lesson
gate: `wall-clock ≈ grid × N × fee × regime × per-path cost`); if the smoke shows a
material per-path regression, the daily-style economy (drop to {0, 5} bps only — the
R-BR.LOAD minimum) is the documented fallback.

### D-BR.3 — Q-BR-3 RESOLVED: the basis sidecar loader + threading (reuse the `funding_by_symbol` co-resample channel)

**RATIFIED: REUSE the existing `funding_by_symbol` co-resample channel for the basis**
(the cheapest path — basis and funding are never used simultaneously in v0.1.0), NOT a
sibling `basis_by_symbol` field. The basis rides the already-built
`GeneratedPath.funding_by_symbol: Option<Vec<Vec<Option<Decimal>>>>` +
`BlockBootstrapPathGen::with_funding` + the `TcnScenarioInput.funding_override` →
`MomentumStrategy::with_funding` → `funding_map` chain (D-CARRY.7, ADR-0051 § D6.6),
which is **already a generic per-symbol per-return-step `Option<Decimal>` series**. The
basis IS that shape. Rationale + the rejected alternative:

| Option | Decision | Why |
|---|---|---|
| **Reuse `funding_by_symbol` channel** | **RATIFIED** | The channel is already a generic "co-resampled `Option<Decimal>` sidecar by the shared `idx_seq`" — it is not funding-specific in its plumbing, only in its NAME. The basis is the same `Option<Decimal>`-per-(symbol,return-step) shape. ZERO new bootstrap field, ZERO new `GeneratedPath`/`TcnScenarioInput` field → **smallest anchor blast radius** (no new `Option` field to default-`None` across ~15 construction sites). Basis + funding mutually exclusive in v0.1.0 (the basis arm is its own `ScoreSource`; carry is a different `ScoreSource`) → no collision. The field is `funding_override`/`funding_by_symbol` by name but carries the basis value when the basis arm runs. |
| Add a sibling `basis_by_symbol` field | **REJECTED for v0.1.0** | Cleaner separation, but +1 `Option` field on `GeneratedPath` + `BlockBootstrapPathGen` + `TcnScenarioInput` + `MomentumStrategy` = ~15 new construction sites to default-`None`, each an anchor-risk surface (every one MUST be `None` for the 99 to hold). More bytes, more risk, for a v0.1.0 where the two series never coexist. Revisit in v0.2.0 IF basis + funding are ever combined (a market-neutral basis arm with a funding overlay) — then the sibling field earns its keep. |

> **A naming-clarity note for the developer (NOT a code change).** The reused field is
> named `funding_*`. To prevent a future reader thinking the basis arm accrues funding,
> the developer adds a one-line doc-comment at the basis injection site: *"the basis arm
> reuses the `funding_by_symbol` co-resample channel as a generic sidecar carrier — the
> value is the BASIS, not funding, and is consumed ONLY by `basis_reversal_score`, NEVER
> by the `run_path` accrual (which stays gated `None` for the basis arm — D-BR.1)."* This
> is the single most-confusable point in the whole feature; the comment is mandatory.

**The basis loader (R-BR.3) — a near-mirror of `funding_data.rs`.** New module
`crates/backtest/src/basis_data.rs`, `#[cfg(feature = "realdata")]`, mirroring
`FundingDataSource` (`funding_data.rs`):

- `pub const EXPECTED_BASIS_REVISION_SHA: &str = "aa72409aa0f856960385a823bc61be1b8274e84f658439b65e5d1b1b1a48f1cd";`
  (verified on disk: `data/binance-basis/REVISION.toml`).
- `BasisRow { symbol: Symbol, open_time_ms: i64, basis_close: Decimal }` (the basis twin
  of `FundingRow`). Schema on disk: `open_time` Int64 ms, `close_time` Int64 ms,
  `basis_open/high/low/close` Utf8 **signed** decimal strings — read `open_time` +
  `basis_close`, parse `basis_close` via `Decimal::from_str` (never f64 — ADR-0003; the
  parse MUST handle the leading `-` for negative basis = perp below spot).
- `BasisDataSource::load(span, scenario)` — the SAME 6-step load+verify+parse as
  `FundingDataSource::load`: REVISION.toml existence, per-file SHA verify, aggregate SHA
  verified against `EXPECTED_BASIS_REVISION_SHA`, polars `scan_parquet`, Decimal parse,
  span filter, sort `(open_time_ms ASC, symbol ASC)`. A `BasisDataError` enum mirrors
  `FundingDataError` (`RevisionMismatch` rejects unverified data).
- `files_for_span` — **identical** to `RealDataBarSource`/`FundingDataSource`
  (`<SYM>/<YEAR>/<MM>.parquet`).

> **The basis is the SAME size class as OHLCV (~8,760 bars/symbol-yr, hourly), unlike
> funding's sparse 8h cadence — and that is a SIMPLIFICATION, not a cost.** Carry's
> Sub-problem B (the 8h→1h forward-fill) is an **identity** at 1h: the basis is already on
> the bar grid, so there is no forward-fill step — the as-of join is just "the basis at
> the open of bar t is `basis_close[t-1]`" (R-BR.5).

### D-BR.5 — Q-BR-3 (the as-of join): `basis_close[t-1]` at the open of bar t (strict no-look-ahead)

The basis at the open of bar `t` is **`basis_close[t-1]`** (the premium-index close of
bar `t` is only known at `t+1h`), per the spike's proven convention (BS.1). This is the
basis analogue of `funding_data.rs::funding_as_of`, BUT simpler because the basis is on
the bar grid: instead of a forward-fill from a sparse 8h grid, it is a **one-bar lag** of
the dense 1h series. Implementation:

- A pure function `basis_as_of(basis_by_bar, bar_open_ts_ms)` in `basis_data.rs` mirroring
  `funding_as_of` — for each bar `t`, return the basis settled **at or before** the
  PRIOR bar (i.e. `basis_close` of the most-recent bar whose `close_time ≤ t.open_ts`,
  which on the aligned 1h grid is `basis_close[t-1]`). The same `partition_point`
  binary-search structure as `funding_as_of` works verbatim (it already returns the
  rightmost settlement ≤ the query ts; here the "settlements" are the dense per-bar
  `basis_close` values keyed by `close_time`, so the rightmost `close_time ≤ open_ts` is
  exactly `basis_close[t-1]`).
- `build_basis_at_return(...)` — the basis twin of `build_funding_at_return`: produces the
  `T-1`-length `basis_at_return[s][k]` array the bootstrap co-resamples (the basis in
  force at real return-step `k` = the as-of basis at the open of source bar `k`). Reused
  verbatim in shape; only the input series changes.

_Day-1 falsifier (D-BR.7 #5): shifting the basis series one bar into the FUTURE changes
the result — RED on revert (proving the as-of join is causal). This is the basis twin of
`funding_data.rs::no_look_ahead_falsifier`, and re-asserts the spike's leak-check (BS.4:
the leaked basis flips POSITIVE where the causal basis is NEGATIVE)._

### D-BR.6 — Q-BR-5 RESOLVED: `run_path` stays CONCRETE (the basis is a `ScoreSource` arm, not a Strategy struct)

**CONFIRMED — straight transfer of the carry/TS ruling (ADR-0051 § D6.5.2 / D6.6.2).**
`run_path` (`montecarlo.rs:92`) is typed to the concrete `strategy::MomentumStrategy` —
the basis arm is a `ScoreSource::BasisReversal` value on `CrossSectionalMomentumConfig`,
NOT a new `Strategy` struct. A sibling `BasisStrategy` struct is REJECTED (it would force
`run_path` generic/`dyn`, re-touching both `run_path` call-sites and risking all 99
anchors — the exact trap that forced MR/carry/TS to be config-on-enum). The basis arm
flows through the existing `score_source` fork in `MomentumStrategy::on_bar`
(`momentum.rs:372`) as a third match arm (alongside `VolAdjustedReturn` and
`FundingCarry`), byte-untouching the existing two. **`run_path`, `PaperEngine`, and the
bootstrap are byte-UNTOUCHED** (the basis rides the existing `funding_*` channels and the
existing `score_source` fork; the only `run_path`-adjacent change is the additive
`taker_fee_bps`/`slippage_bps` parameterization at 2409-2410, which is defaults-`2`/`4`
for every non-basis caller — D-BR.LOAD).

### D-BR.8 — Determinism & anchoring: the 99 anchors hold byte-identical (additive/defaults-off)

Every new seam is **additive and defaults-off**, gated on the basis `ScoreSource` being
selected — so every momentum / MR / carry / TS / horizon run is byte-identical by
construction. **The 99 existing anchors (`spec/anchors.toml`) stay byte-identical — a hard
gate, NOT a goal.** The four seams + their neutrality argument:

1. **`ScoreSource::BasisReversal` enum arm** — serde-default stays `VolAdjustedReturn`;
   adding a third variant does NOT change the default-path serialization (the carry
   `FundingCarry` precedent proved this — the enum already has 2 variants and the 99
   anchors hold). The config-hash gains a `BasisReversal` discriminant only when selected.
2. **The basis loader + co-resample** — reuses the `funding_by_symbol` channel
   (D-BR.3); ZERO new `GeneratedPath`/`bootstrap`/`TcnScenarioInput` fields. The
   co-resample consumes ZERO new RNG draws (the carry D6.6 proof transfers verbatim: the
   basis gather rides the already-materialized `idx_seq` — `basis_at_return[s][idx_seq[k]]`
   at the same index that picks the return). When the basis arm is NOT running, the
   `funding_at_return` is `None` and the bootstrap takes the byte-identical pre-carry path.
3. **The fee axis** — `--taker-fee-bps`/`--slippage-bps` default to `4`/`2` (the legacy
   literals at 2409-2410); the fee body-row is gated `score_source == BasisReversal`. Both
   the engine input AND the report body are byte-identical for every non-basis run
   (D-BR.LOAD anchor-neutrality proof).
4. **The basis score fork** — a third match arm in `on_bar`; the `VolAdjustedReturn` and
   `FundingCarry` arms are byte-untouched.

**Anchor unit = the basis θ × fee surfaces.** New basis anchors are added **after** the
developer's anchored run (the tester locks them at the M-TEST PASS gate, per the
carry/TS/horizon precedent); the grid + N + fee ladder are locked HERE at design time
(D-BR.2-LOCKED). The existing 99 anchors are NOT re-locked. **Anchored report files in
`spec/*/reports/` remain byte-immutable** (ADR-0038 § D6).

### D-BR.9 — Q-BR-6 RESOLVED: a new `perp-basis-signal-robustness` namespace + the additive `verify_anchors.sh` handler

**RATIFIED: a NEW `perp-basis-signal-robustness` anchor namespace** (the horizon-retest
D6.8 precedent — a new program phase, the FIRST LIVE signal, gets its own namespace), NOT
the existing `mc-robustness-2026-06` lane. Rationale: the basis arm is a distinct
experiment axis (a structurally-new data domain + a swept fee axis), exactly as the
horizon retest was a distinct axis (a new cadence). A new namespace keeps the basis
surfaces' provenance legible and avoids overloading the `mc-robustness-2026-06` lane's
semantics.

**`verify_anchors.sh` handler (additive one-liner — the carry/MR/horizon precedent).** Add
a new `elif [[ "$version" == "perp-basis-signal-robustness" ]]` branch (after the
`horizon-retest-robustness` branch at line 162) that searches
`spec/perp-basis-signal-robustness/reports/` for `robustness-*-${scenario}.md`. This is
the identical additive handler shape the horizon namespace used (lines 162-169). No
existing branch is touched → the 99 anchors resolve through their existing branches
byte-identically.

**Scenario names (the fee level is in the name AND the hashed body).** Per fee level + per
regime, to make the four-fee × two-regime surfaces distinct anchors:

```
v1-basis-reversal-fee{NN}bps-theta-surface-{year}-block-bootstrap-real-fy
```

e.g. `v1-basis-reversal-fee05bps-theta-surface-2023-block-bootstrap-real-fy` (fee=5 bps,
2023). The fee level is a two-digit zero-padded `{NN}` (`00`/`02`/`05`/`10`). Up to **8
anchors** (4 fee × 2 regime), locked by the tester at M-TEST PASS (the durable choice;
the tester may elect a subset — minimum the {0, 5}-bps × {2023, 2024} = 4 surfaces per
the R-BR.LOAD acceptance). The slug routes the report to
`spec/perp-basis-signal-robustness/reports/` (the effective-out-dir logic at line 3044
gains a `score_source == BasisReversal` branch, mirroring the carry/TS/horizon branches).

### D-BR.10 — the ADR amendment: ADR-0051 § D6.9 (registered atomically)

The basis arm is the **5th anchor-additive instance** after MR=D6.5 / carry=D6.6 /
TS=D6.7 / horizon=D6.8. It warrants an **amendment to ADR-0051, NOT a new ADR** (the same
ruling as carry/TS/horizon — it inherits the determinism + anchoring contract and extends
it additively). § D6.9 records: (1) the basis is a 3rd `ScoreSource` arm
(`BasisReversal`), the sign baked into the score (NO cashflow — the basis is a selection
signal, so carry's accrual does NOT transfer); (2) the basis **reuses** the
`funding_by_symbol` co-resample channel (basis + funding mutually exclusive in v0.1.0) —
no new field, smallest blast radius, the co-resample consuming ZERO new RNG draws (D6.6
proof transfers); (3) the NEW `--taker-fee-bps`/`--slippage-bps` axis (defaults `4`/`2` →
99 anchors byte-identical; fee body-row gated to the basis arm), modeled on the `--horizon`
D6.8 axis; (4) NEW namespace `perp-basis-signal-robustness` + up to 8 anchors (4 fee × 2
regime); LOCKED 6-cell grid (§ D-BR.2-LOCKED) + N + fee ladder {0,2,5,10} are hashed body
fields (K3); (5) `run_path`/`PaperEngine`/`bootstrap` byte-UNTOUCHED. Registered
atomically in `spec/architecture/adr/README.md` (the § Registry row 0051 summary +
frontmatter `updated:` bumped in the same edit pass; `adr_registry_check.py --pre-commit`
must PASS) per the architect.md § ADR registry contract.

---

## Backtest Scenarios

_**Architect-RATIFIED + LOCKED (M-T1, 2026-06-05).** The grid (§ D-BR.2-LOCKED, 6 cells),
the fee ladder ({0,2,5,10} bps, § D-BR.LOAD), and N (200/cell) are LOCKED hashed body
fields (the MR/carry/TS/horizon precedent). Primary anchored deliverable = the
basis-reversal θ × fee surface on 2023-FY at each fee level (+ up to 8 anchors across
4 fee × 2 regime). The day-1 robustness gate runs BOTH 2023 AND 2024 (the carry/horizon
E1 precedent: 2024 is gating, not an afterthought). Wall-clock ~16 min for all 8 surfaces
(§ D-BR.WALLCLOCK — TRACTABLE, under the ≲30-min gate). The analyst's proposed surface
shape is ratified below with the LOCKED specifics._

> **The locked grid + fee ladder + N live in § D-BR.2-LOCKED / § D-BR.LOAD (the Design).**
> This section ratifies the analyst's surface PLAN against those locks. The grid is
> 6 cells (L∈{24,60,168} bars × K/cadence folded into the 6 cells, SKIP L=720) × the fee
> ladder {0,2,5,10} bps × {2023, 2024}. N=200/cell. The fee axis produces ONE surface per
> fee level (mechanism (i), § D-BR.LOAD); the {0, 5} bps × {2023, 2024} = 4 surfaces are
> the R-BR.LOAD-acceptance minimum, all 8 are the durable choice.

The primary anchored deliverable is a **θ × fee surface** — the basis-reversal
signal swept over its signal-bearing lookback band × the sizing/K axis, AT EACH
taker-fee level, on 2023 + 2024 vs the BH control.

**The θ-axis (signal lookback) — LOCKED to the spike's signal-bearing band:**
lookback `L ∈ {24, 60, 168}` bars. This is the band where the spike measured a
sign-stable, adequate-`n` IC (BS.2a):

| L (bars) | spike rank IC 2023 / 2024 | role |
|---|---|---|
| 24 (1d) | −0.031 / −0.022 | short end of the signal band — faster, more fee-exposed |
| 60 (2.5d) | **−0.099 / −0.081** | the IC peak — the strongest cross-sectional signal |
| 168 (1wk) | **−0.112 / −0.069** | the IC peak / lowest-turnover corner |

> **SKIP L = 720** (30d): the spike flagged it as **noise** (n = 11 windows,
> sign-flips across years: 2023 −0.195 vs 2024 +0.033), exactly the M4 / broader-
> universe L=720 discipline. It is excluded from the grid, NOT swept.

**The fee-axis — the load-bearing gate (R-BR.LOAD):** taker fee ∈ **{0, 2, 5, 10}
bps** (proposed; the architect locks the exact ladder — Q-BR-4). 0 bps = the gross-
edge ceiling; ~5 bps = the realistic Binance taker decision point; 10 bps = the
punitive stress. The deliverable shows the net-of-fee edge as a function of fee.

**The sizing/K axis:** mirror the carry/MR θ-grid shape (K ∈ a small locked set;
rebalance cadence as the turnover lever — a reversal arm's fee exposure is dominated
by turnover, so the cadence axis is especially diagnostic here). The architect locks
the exact cells.

**N:** N = 200/cell on 2023 + 2024 (the carry/MR/TS tractable shape; the architect
re-validates the wall-clock before locking, per the C3 lesson `wall-clock ≈ grid × N
× per-path cost` — note the fee axis multiplies the cell count, so the architect must
confirm `|L| × |K| × |fee| × N × 2 regimes` is tractable; if not, the fee axis can be
read at the best-θ cell rather than the full cross-product — an architect economy
call).

**Plan to anchor the surfaces** (the tester locks them at M-TEST PASS):

1. **BASIS-θ×FEE (PRIMARY, ANCHORED)** — the basis-reversal θ-surface × the fee
   ladder, N=200/cell, shared-index block-bootstrap of 2023-FY real OHLCV **+ the
   shared-index-resampled basis series** (R-BR.6). Per-cell FRAGILE/MARGINAL/ROBUST
   + family verdict + per-cell `→ C5` flags + the trades column (turnover
   legibility, the reversal-fee story) + **the net-of-fee edge vs BH at each fee
   level** (R-BR.LOAD).
2. **Control (in the surface)** — buy-and-hold equal-weight under the same N paths +
   auto-L bootstrap (re-asserts the **+1.74 (2023) / +1.10 (2024)** bar the basis
   arm must clear). Fee-invariant; carries no verdict.
3. **2024-FY surface** — the SAME locked grid + fee ladder on 2024-FY (banked, pinned)
   as the harder/fairer multi-regime gate (the carry/horizon E1 precedent: 2024 is
   gating, not an afterthought). Anchored if the tester elects (the durable choice).

> **Namespace (RESOLVED — § D-BR.9):** a new `perp-basis-signal-robustness` anchor
> namespace (the horizon-retest precedent — a new program phase gets its own namespace),
> with `verify_anchors.sh` extended (an additive `elif` branch after the
> `horizon-retest-robustness` branch, line 162) to search
> `spec/perp-basis-signal-robustness/reports/` for `robustness-*-${scenario}.md`. Scenario
> names carry the fee level: `v1-basis-reversal-fee{NN}bps-theta-surface-{year}-block-bootstrap-real-fy`
> (`{NN}` ∈ {00,02,05,10}).

---

## Verification
_Tester links reports here after the M-TEST gate. The gates the tester must clear:_

1. **The FEE-SWEEP result (R-BR.LOAD) — the headline.** The fee-sensitivity read is
   produced; the best cell's net-of-fee edge vs the BH control is reported at each
   fee level; the FRAGILE-on-fees verdict (if it dies at realistic fees) is explicit.
   This is the decision-grade output of the whole feature.
2. **The day-1 falsifiers RED-on-revert (R-BR.7).** The sign-assertion, baseline-
   equity-divergence e2e, basis-signal-non-no-op, and no-look-ahead falsifiers each
   go GREEN as written AND RED when their guard is reverted (genuine guards, not
   no-ops — the carry/TS/horizon precedent).
3. **The 99 existing anchors byte-identical** (`verify_anchors.sh` 99/99) + the new
   basis anchors locked.
4. **Two-run byte-identity** of the basis surface body-SHA (R-BR.7).
5. **Pre-flight void-if-fail** — the basis report headers print
   `generator: block-bootstrap-real` AND `bootstrap_mode: shared-index`.
6. **The frozen § 0 composite verdict** read per
   [`robustness-decision-rule-2026-05-30.md`](../dev-notes/robustness-decision-rule-2026-05-30.md)
   § 0 (weakest-link), at the realistic fee level, against the BH control.

---

## Open design questions for the architect (M-T1)

> Framed, not resolved. The carry feature resolved the structurally-identical set
> (Q-CARRY-1..5) — the architect should lift those rulings where they transfer.

- **Q-BR-1 — `ScoreSource::Basis` + `Direction::Reversion`, or a single
  `ScoreSource::BasisReversal` with the sign baked in?** Carry baked the sign into
  the score (`−trailing_mean(funding)` + `Direction::Momentum`) so the reversal sign
  lived in one auditable place. The basis reversal is the same shape — a
  `−trailing_mean(basis)` score keeps the unchanged `top_k_long` selector. Decide
  whether to (i) add `ScoreSource::Basis` and reuse the existing `Direction`
  Momentum/Reversion enum to express the sign, or (ii) add a single
  `ScoreSource::BasisReversal` with the minus baked in (the carry pattern — one
  place for the load-bearing sign). Also confirm whether the basis arm has ANY
  cashflow component or is purely a selection/sizing tilt (R-BR.9 note: carry's
  funding-cashflow Sub-problem D does NOT obviously transfer — the basis is a
  positioning signal, not a cash settlement).
- **Q-BR-2 — long-only (a) vs market-neutral (b)?** (The load-bearing framing,
  R-BR.2.) Analyst recommends (a) long-only at v0.1.0 (apples-to-apples, fee-verdict-
  first); (b) market-neutral is the v0.2.0 follow-on IFF (a) survives fees. The
  architect's M-T1 ratifies (the carry Q-CARRY-2 / D-CARRY.0 precedent).
- **Q-BR-3 — how does the basis sidecar load + thread?** Mirror the carry funding
  seam: a basis-as-of map injected alongside `bars_override` (carry's seam (ii),
  `with_funding` / `funding_override` / `GeneratedPath.funding_by_symbol`),
  `run_path` stays CONCRETE (the binding constraint that forced carry/MR/TS to be
  config-on-enum, not a new struct — ADR-0051 § D6.5.2). Decide whether the basis
  **reuses the existing `funding_by_symbol` co-resample channel** (cheapest — basis
  and funding are never used together in v0.1.0) **or adds a sibling
  `basis_by_symbol` field** (cleaner separation, +1 `Option` field). Either way the
  shared-index co-resample is the proven D6.6 mechanism.
- **Q-BR-4 — the exact θ-grid + the fee-sweep mechanism.** Lock the lookback band
  ({24, 60, 168} proposed — SKIP 720), the K/cadence axis, and N. AND decide the
  **fee-sweep mechanism**: (i) a new `--taker-fee-bps` sweep axis threaded into
  `TcnScenarioInput` (currently hardcoded `slippage_bps: 2` / `taker_fee_bps: 4` at
  `param_robustness_sweep.rs:2409`) producing one surface per fee level, vs (ii) a
  fixed realistic-fee pin for the anchored θ-surface + a SEPARATE fee-sensitivity
  report that re-runs the best cell across the fee ladder. (i) is more complete
  (every cell × every fee); (ii) is cheaper (the fee axis only at the best-θ cell).
  Whichever is chosen, the {0, 5} bps read (gross ceiling + realistic decision point)
  is mandatory (R-BR.LOAD acceptance). Confirm the wall-clock is tractable for the
  chosen cross-product.
- **Q-BR-5 — does the basis arm reuse `run_path` concrete? (Yes — the carry
  pattern.)** Confirm `run_path` stays typed to concrete `MomentumStrategy` (the
  basis is a `ScoreSource` arm on the config, not a new `Strategy` struct), so the
  basis path does not force `run_path` generic/`dyn` and does not risk the 99
  anchors. This is expected to be a straight transfer of the carry/TS ruling.
- **Q-BR-6 — anchor namespace + `verify_anchors.sh` handler.** New
  `perp-basis-signal-robustness` namespace (the horizon-retest precedent) vs the
  existing `mc-robustness-2026-06` lane. Recommend a new namespace (a new program
  phase — the first LIVE signal) + the additive `verify_anchors.sh` handler.

---

## Assumptions & limits (challengeable by operator / architect)

1. **The fee-sweep is the gating risk, and FRAGILE-on-fees is the likely outcome.**
   The spike itself flagged this (BS.5 §3): a reversal arm is the most fee-sensitive
   class, and the −0.10 gross IC is thin. The build is justified by the **decision-
   grade answer either way**, not by optimism about a positive verdict. If it dies
   on fees, that retires the derivatives-positioning family and routes to on-chain —
   a clean, pre-registered next step.
2. **Long-only (framing a) captures only the long leg of the reversal spread.** The
   spike's full −0.10 IC lives in the full long/short cross-section; the long-only
   v0.1.0 arm tests the weaker (long-low-basis) half. This is the deliberate apples-
   to-apples / fee-verdict-first scope; (b) market-neutral is the v0.2.0 follow-on if
   (a) survives fees. The verdict must be read as "does the long-low-basis tilt beat
   BH net of fees," not "does the full reversal spread work."
3. **The basis is natively 1h, so carry's hardest sub-problems shrink.** The 8h→1h
   forward-fill (carry Sub-problem B) is an identity at 1h, and there is likely no
   funding-style cashflow accrual (carry Sub-problem D) — the basis is a selection
   signal. So the basis build is expected to be **cheaper than carry's ~4.5-7.5 d**,
   not larger — the dominant new work is the loader (R-BR.3, near-mirror of
   `funding_data.rs`), the shared-index co-resample (R-BR.6, the proven D6.6
   mechanism — possibly reusing the funding channel), and the **fee-sweep mechanism**
   (R-BR.LOAD, the one genuinely new axis). The architect's M-T1 owns the true-size
   re-assessment.
4. **The robustness axis judges resampled real 2023/2024 only.** It cannot speak to
   a basis regime those two years never contained (inherited scope limit). A ROBUST
   verdict is "robust to resampled 2023/2024 history net of fees," not "robust to all
   future basis regimes."
5. **The spike is MEDIUM-HIGH, not HIGH.** The magnitude is modest (peak |IC| ~0.10),
   a raw rank IC is the upper bound not net P&L, and the L=720 cells were noise. The
   LIVE call rests on the L=9 → L=168 band where the sign is stable and n is adequate.
   This build is the honest test of whether that gross signal survives the machine +
   fees.

---

## Changelog

- 2026-06-05 (architect, M-T1): resolved Q-BR-1..6 + wrote the Design (D-BR.0..10) +
  authored `tasks.md` (M-DEV-0..8 + M-TEST) + the ADR-0051 § D6.9 amendment (registry row
  + frontmatter atomic; `adr_registry_check.py` PASS). Status proposed → **arch-done**.
  **Q-BR-2: framing (a) long-only basis-reversal tilt** (long the lowest-basis names;
  reuses the solvency-guarded long-only `run_path` + `top_k_long`; apples-to-apples with
  the 99 anchors + carry #88/#89; (b) market-neutral deferred to v0.2.0 on validation).
  **Q-BR-1: a single `ScoreSource::BasisReversal` arm** with the SIGN baked into the score
  (`−trailing_mean(basis)`, one auditable place; option (i) ScoreSource::Basis+Reversion
  REJECTED — splits the sign across two fields) + **NO cashflow** (the basis is a selection
  signal — carry's accrual Sub-D does NOT transfer; the `run_path` accrual stays gated
  `None` for the basis arm). **Q-BR-4 (LOAD-BEARING): the FEE-SWEEP = a `--taker-fee-bps`
  axis (mechanism (i), one anchored surface per fee level)** modeled on the proven
  `--horizon` axis (defaults `4`/`2` → 99 anchors byte-identical; fee body-row gated to
  the basis arm); mechanism (ii) fixed-pin + best-cell report REJECTED (reads the fee axis
  at the argmax the frozen § 0 rule forbids crowning). The fee site is the hardcoded
  `slippage_bps: 2`/`taker_fee_bps: 4` at `param_robustness_sweep.rs:2409-2410`. **Q-BR-4
  grid LOCKED (§ D-BR.2-LOCKED):** 6 cells (L∈{24,60,168} BARS × K/cadence; rebalance
  default 8h, g3 slowed to 24h — the reversal arm's lowest-churn fee shot; SKIP L=720
  noise), N=200, fee ladder {0,2,5,10} bps. **Wall-clock ~16 min for all 8 surfaces
  (4 fee × 2 regime) — TRACTABLE, under the ≲30-min gate** (§ D-BR.WALLCLOCK; the basis
  gather is the SAME negligible O(n_bars) index-gather carry M-DEV-7 measured). **Q-BR-3:
  REUSE the existing `funding_by_symbol` co-resample channel** for the basis (cheapest —
  basis + funding mutually exclusive in v0.1.0; ZERO new bootstrap/`GeneratedPath`/
  `TcnScenarioInput` field; a sibling `basis_by_symbol` field REJECTED for v0.1.0 as
  +bytes/+anchor-risk); the co-resample consumes ZERO new RNG draws (D6.6 proof transfers).
  Basis loader = a near-mirror of `funding_data.rs` (new `basis_data.rs`, pin
  `aa72409a…`, Decimal parse of signed `basis_close`, REVISION-verify); the 8h→1h
  forward-fill is an IDENTITY at 1h (the as-of join is `basis_close[t-1]`, a one-bar lag).
  **Q-BR-5: `run_path` stays CONCRETE** (the basis is a 3rd `ScoreSource` arm, not a
  Strategy struct — the D6.5.2 trap avoided; `run_path`/`PaperEngine`/`bootstrap`
  byte-UNTOUCHED). **Q-BR-6: a NEW `perp-basis-signal-robustness` namespace** (the horizon
  D6.8 precedent) + the additive `verify_anchors.sh` `elif` handler; scenario
  `v1-basis-reversal-fee{NN}bps-theta-surface-{year}-block-bootstrap-real-fy`; up to 8
  anchors (4 fee × 2 regime) locked by the tester at M-TEST PASS ({0,5} bps × {2023,2024}
  = 4 the minimum). **True-size: ~3–5 d** (cheaper than carry — B is an identity, D
  vanishes, C is reused). M-DEV-8 (the anchored 8-surface fee × grid × regime run) is the
  deliverable; the day-1 falsifiers (sign-assertion / baseline-divergence e2e /
  basis-non-no-op / no-look-ahead / two-run identity) land BEFORE it. No code, no build,
  no engine run; no `anchors.toml` touch (the tester locks the basis anchors post-run).
  developer M-DEV next; M-DEV-2 (the fee-axis + the basis loader anchor-neutrality gate,
  99/99) lands FIRST.
- 2026-06-05 (analyst, perp-basis-signal-robustness): authored the feature brief +
  opened the trace row `REQ-PERP-BASIS-SIGNAL-ROBUSTNESS-001` (state `proposed`,
  operator-greenlit). Transcribes the basis spike
  ([new-data-domain-scoping-2026-06-05.md § BS.0-BS.6](../dev-notes/new-data-domain-scoping-2026-06-05.md#basis-spike-results),
  VERDICT **LIVE / MEDIUM-HIGH**). Why = the first non-flat post-OHLCV signal — a
  cross-sectional basis-reversal (high-basis/crowded-long names underperform; rank IC
  −0.08 to −0.11 over L=60-168, sign-stable both years), orthogonal to OHLCV (corr
  +0.02-+0.07), only MODERATELY redundant with funding (+0.47/+0.66, NOT ≈ +1),
  proven causal. Requirements R-BR.1..9 + the load-bearing **R-BR.LOAD fee-sweep
  gate** (a reversal arm is the most fee-sensitive class — if it dies net of
  realistic Binance taker fees, that IS the verdict, retiring the derivatives-
  positioning family). LOAD-BEARING SIGN (R-BR.2): tilt AGAINST the basis; a sign
  flip = a basis-momentum payer → RED. Carry (`carry-strategy`) is the line-for-line
  precedent (ScoreSource arm + sidecar loader + shared-index co-resample + as-of join
  + day-1 falsifiers). Additive/defaults-off → the 99 existing anchors hold byte-
  identical. θ-band LOCKED to the spike's signal-bearing lookback {24, 60, 168} (SKIP
  L=720 noise) × the sizing/K axis × the fee ladder {0,2,5,10} bps, N=200 on 2023 +
  2024 vs the BH control (+1.74 / +1.10). 6 framed design questions (Q-BR-1..6) for
  the architect M-T1 (ScoreSource::Basis+Reversion vs BasisReversal; long-only vs
  market-neutral; sidecar threading; θ-grid + fee mechanism; run_path concrete;
  namespace). No Design section, no tasks.md, no code authored by the analyst.

## Implementation

_Developer pass 1 of 2 (M-DEV-0..3), 2026-06-06._

### Scope: M-DEV-0..3 (basis-signal foundation)

This pass implements the anchor-sensitive signal foundation: the basis loader, the as-of
join, and the `ScoreSource::BasisReversal` arm. M-DEV-4..9 (fee axis, sweep wiring,
integration falsifiers, anchored run) are the next developer pass.

### Files changed / created

| File | Change | Task |
|---|---|---|
| `crates/backtest/src/basis_data.rs` | New (675 lines) — `BasisDataSource`, `BasisRow`, `LoadedBasis`, `BasisDataError`, `basis_as_of`, `build_basis_at_return`, 12 tests | M-DEV-1, M-DEV-2 |
| `crates/backtest/src/lib.rs` | +6 lines — `pub mod basis_data` registration (realdata-gated) | M-DEV-1 |
| `crates/strategy/src/cross_sectional/config.rs` | +27 lines — `ScoreSource::BasisReversal` variant + 3 config-hash tests | M-DEV-3 |
| `crates/strategy/src/cross_sectional/momentum.rs` | +200 lines — `basis_reversal_score`, `all_warmed` BasisReversal arm, `on_bar` BasisReversal arm, mandatory channel-reuse doc-comment, 4 tests (sign-assertion × 2, no-look-ahead, plus helper) | M-DEV-3 |

### Deviations from spec

None. The implementation matches the architecture spec (D-BR.0..3, D-BR.5) exactly:
- `BasisRow` uses `open_time_ms` as the key (basis parquet schema uses `open_time`, not
  `close_time`), as confirmed on disk. The as-of join is keyed on `open_time_ms` with
  `≤` semantics — this implements `basis_close[t-1]` on the 1h grid (D-BR.5).
- `basis_reversal_score` reuses `funding_rings` as the per-symbol bar-count ring (D-BR.3
  channel reuse). The mandatory doc-comment is in place at both the method and the
  `on_bar` arm call site.
- The `BasisReversal` arm in `all_warmed` mirrors `FundingCarry` exactly (ring.len() ≥
  funding_lookback). This is correct: both use the same `funding_rings` field.

### Gate results (all 4 gates PASS)

1. **Clippy (canonical):** `cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | grep -E "^\s+-->" | grep -v "crates/ui/" | sort -u` → **EMPTY** (zero non-UI warnings)
2. **fmt:** `cargo fmt --all -- --check` → **clean**
3. **Anchors:** `bash scripts/verify_anchors.sh` → **99/99 PASS**
4. **Tests:**
   - `cargo test -p backtest --features "candle realdata" --lib basis_data` → **11 passed; 0 failed; 1 ignored**
   - `cargo test -p strategy --lib` → **156 passed; 0 failed; 0 ignored**
   - Sign-assertion (`r_br2_sign_assertion_longs_low_basis_name` + `r_br2_basis_reversal_score_low_basis_outscores_high_basis`) → **2 passed**
   - No-look-ahead (`r_br5_no_look_ahead_strategy_level`) → **1 passed**
   - Config hash (`m_dev3_*`) → **3 passed**

### Non-negotiables verification

- `run_path` / `PaperEngine` / `bootstrap` / `montecarlo.rs` — **NOT TOUCHED** (confirmed by `git diff --name-only`)
- No `.unwrap()` in library code (all in `#[cfg(test)]` blocks)
- No `f64` in the basis parse or the score
- No `SystemTime::now()` / `thread_rng()` added
- The SIGN (`−trailing_mean`) is in ONE place (`basis_reversal_score`, line ~277 in `momentum.rs`)
- `crates/ui/` — NOT TOUCHED
- `data/yahoo/REVISION.toml` — NOT TOUCHED by this pass

- 2026-06-08 (orchestrator): status `arch-done` → `presenter-done` (spec-hygiene wind-down, audit-2026-06-08 § Status drift). The lagging mirror is corrected to the actual pipeline state: M-TEST VERDICT PASS (`0a36cdf`), presenter release-retrospective deck (long-only close-out, `f9e3302`); anchors 99→107. trace.toml (the source of truth) was already correct. Frontmatter-only edit; anchors 119/119 unperturbed.
