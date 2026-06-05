---
slug: perp-basis-signal-robustness
version: 0.1.0
status: proposed
owner: analyst → architect
priority: P1
updated: 2026-06-05
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
_Architect M-T1 fills this. Do NOT author above this line as the analyst._

---

## Backtest Scenarios
_Architect ratifies + LOCKS the exact grids, the fee ladder, and N before the tester
anchors. The analyst proposes the surface shape below; the architect's M-T1 locks
the hashed body fields (the MR/carry/TS/horizon precedent)._

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

> **Namespace (architect decides):** a new `perp-basis-signal-robustness` anchor
> namespace (the horizon-retest precedent — a new program phase gets its own
> namespace), with `verify_anchors.sh` extended to search
> `spec/perp-basis-signal-robustness/reports/` (the additive one-liner carry / MR /
> horizon all made).

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
