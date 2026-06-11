---
slug: strategy-robustness-monte-carlo-direction-2026-05-29
date: 2026-05-29
authors: analyst
status: proposed
tags: [strategy, robustness, monte-carlo, synthetic-data, sensitivity, learning-loop, llm-demotion, direction]
related:
  - spec/dev-notes/strategic-reset-2026-05-23.md
  - spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md
  - spec/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md
  - spec/dev-notes/monte-carlo-robustness-architecture-readiness-2026-05-29.md
  - spec/product.md
---

# Strategy robustness via Monte-Carlo synthetic data — strategic direction

> **What this is.** A READ-ONLY research + scoping pass naming the project's
> next honest limit and proposing a direction to address it: move from
> **fixed, parametric strategies evaluated on a single deterministic
> historical path** toward **Monte-Carlo-validated, sensitivity-tested,
> learning-adaptive strategies**, with the LLM explicitly demoted to a
> SUPPORT pillar (explanation / narration / tie-break) rather than the
> alpha engine.
>
> **What this is NOT.** Not a feature folder, not a trace.toml row, not a
> backlog edit. The § 6 candidate features are *sketches* for the operator
> to choose from next session, gated on the architect's parallel readiness
> audit (`monte-carlo-robustness-architecture-readiness-2026-05-29.md`).
> Output is this one dev-note.

---

## § 0. The framing in one paragraph

The README sells "honest about its limits" — it retired four DL/vol/regime
research lines rather than overclaim. The next honest limit, named here:
**every shipped strategy is a fixed parametric rule, and every strategy is
judged by ONE number per scenario computed on ONE deterministic historical
price path.** There is no measure of *robustness* — no distribution of
outcomes under perturbed parameters, resampled returns, or alternative
synthetic realizations. A strategy that scores Sharpe 1.4 on the single
2023-FY path and a strategy that scores 1.4-but-only-on-that-exact-path are
indistinguishable today. Monte-Carlo synthetic-data evaluation is the
standard quant answer to this, and — critically — it is a **different bet
than the retired DL-forecasting-alpha track** (see § 2.6). This note argues
the cheap, defensible first step is **stationary block bootstrap**, that the
output is a **distribution-valued robustness report** (§ 3), and that the
project's single highest-leverage honest gap is the **half-closed learning
loop** (§ 4): reflection memory is written on every trade but read by exactly
one strategy, into a prompt, on a path whose verdict is deferred indefinitely.

---

## § 1. The limit, named precisely

### 1.1 Which strategies are parametric, and what params each exposes

Read of `crates/strategy/src/`. Every shipped strategy is a deterministic
parametric rule. The params are loaded once from TOML (`config/strategies/`)
and **never swept, perturbed, or varied within a run**.

| Strategy | Source | Tunable params (fixed at load) | Param count |
|---|---|---|---|
| `SmaCrossover` | `sma_crossover.rs` | `fast_len` (default 20), `slow_len` (default 50) | 2 |
| `MomentumStrategy` (v1, load-bearing baseline) | `cross_sectional/config.rs` | `lookback_minutes` (60), `rebalance_minutes` (60), `k_long` (3), `exposure_cap` (0.50), `drift_rebalance_threshold` (0.10), `vol_floor` (1e-6), `universe` (top-10) | 6 numeric + universe |
| `MeanReversionPairsStrategy` (v1.5a) | `pairs/config.rs` | z-score entry/exit thresholds, lookback, pair set | ~4 |
| `ComposedStrategy` (v0.5) | `composed/` | arbitrary TOML AST of indicators + thresholds | unbounded |
| `VolTargetingOverlay` / `VolKillSwitchOverlay` (v3, retired) | `vol_*` | GARCH params, `target_vol`, `scale_clamp_{min,max}`, `threshold_multiplier` | ~4 each |
| `RegimeDispatcher` (v3-regime, draft) | `regime_dispatcher.rs` | min-fit bars, regime thresholds, route map | ~5 |
| `TcnOverlayMomentumStrategy` / `PatchTstOverlay...` (retired) | `tcn_*` / `patchtst_*` | τ (forecast threshold), ε (dampen epsilon), horizon | ~3 |

The v1 momentum baseline alone has a **~7-dimensional parameter space**
(`lookback × rebalance × k_long × exposure_cap × drift × vol_floor ×
universe-membership`). Today exactly **one point** in that space is
evaluated (`top10_momentum_h1.toml`). We do not know whether Sharpe is a
broad plateau around that point (robust) or a sharp peak (fragile / curve-fit).
That distinction is the entire robustness question — and "stable plateaus
indicate robustness; sharp peaks signal fragility" is the textbook read of a
parameter-sensitivity surface ([Build Alpha](https://www.buildalpha.com/robustness-testing-guide/),
[LuxAlgo](https://www.luxalgo.com/blog/what-is-overfitting-in-trading-strategies/)).

### 1.2 How strategies are evaluated today — single deterministic path → one number

- The backtest CLI takes `--scenario <name> --seed <hex>` and produces ONE
  report under `spec/<feature>/reports/backtest-<stamp>-<scenario>.md`,
  body-SHA-256-anchored in `spec/anchors.toml` (168 anchor-related lines).
- The price path is either (a) real Binance OHLCV for the scenario's calendar
  year via `--features realdata`, or (b) a single seeded synthetic path from
  `synthetic_bars()` (Box-Muller GBM, ChaCha20, fixed drift `1.9e-6`/min,
  fixed vol `1.1e-3`/min). Either way it is **one path**.
- The seed (`0xC0FFEE` canonical) drives only **fill tie-breaking and the
  synthetic fixture** — it does NOT generate an ensemble. Re-running the same
  scenario at the same seed is byte-identical by design (that is the anchor
  contract); re-running at a *different* seed changes only the synthetic
  fixture's single path, and no shipped scenario does this systematically.

The result the operator sees per scenario is a point estimate: one Sharpe,
one Sortino, one max-drawdown, one final-equity. The strategic-reset note
records the load-bearing example — **v1 momentum has 73% max-DD on 2023-FY
real Binance data** (`strategic-reset-2026-05-23.md` § 4.2). That is one
number on one path. We do not know its 5th/50th/95th percentile across
plausible alternative 2023s, nor across ±10% parameter perturbations.

### 1.3 The critique, made rigorous (not hand-wavy)

Three concrete deficiencies, each falsifiable:

1. **No path-distribution.** A backtest reports `f(strategy, params,
   the_one_2023_path)`. Robustness needs the distribution of
   `f(strategy, params, P_i)` over an ensemble `{P_i}` of plausible paths.
   Today `N = 1`.
2. **No parameter-distribution.** A backtest reports `f(strategy, θ*, path)`
   at a single `θ*`. Sensitivity needs `f(strategy, θ, path)` over a
   neighborhood of `θ*`. Today the neighborhood is a singleton.
3. **No overfit-adjusted metric.** With a 1-path, 1-θ evaluation there is no
   way to compute a Probability of Backtest Overfitting (PBO) or a Deflated
   Sharpe Ratio — the standard López de Prado guards
   ([Bailey, Borwein, López de Prado, Zhu](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2326253)).
   The TCN threshold-tuning ship (`strategic-reset` decision #7) swept τ × ε
   on a single real path and reported a marginal +0.018/+0.045 — with no
   deflation for the size of the search. We literally cannot say whether
   that marginal edge would survive multiple-testing correction.

The operator's phrase **"check whenever the strategy will behave different
with different inputs"** decomposes precisely into deficiencies (1)+(2): vary
the *path inputs* (resample / re-simulate) and vary the *parameter inputs*
(sweep / perturb), and measure how the output number moves.

---

## § 2. Monte-Carlo synthetic-data methods (the research core)

Survey of the real techniques, ranked by **defensibility-per-implementation-
cost for THIS project** (crypto OHLCV, Rust, deterministic-anchor constraint).
Each row: what it does, pros, cons, when-to-use. Sources cited inline.

### 2.1 Ranked comparison table

| Rank | Method | What it generates | Preserves | Pros | Cons | Calibration burden | Crypto fit |
|---|---|---|---|---|---|---|---|
| **1** | **Stationary block bootstrap** (Politis–Romano 1994) | Resampled return series, concatenating random-geometric-length blocks of the *real* historical returns | Autocorrelation, fat tails, vol clustering (within block length) — **empirically, because it reuses real returns** | No distributional assumption; preserves real higher moments; one tunable (mean block length); cheapest defensible method | Cannot produce regimes absent from history; block boundaries can break very-long-memory; needs ≥1 real series to resample | Low — one parameter (expected block length); auto-selectable ([Politis–White 2004](https://public.econ.duke.edu/~ap172/Politis_White_2004.pdf)) | **Best.** Reuses real crypto returns so fat tails + clustering come for free |
| **2** | **Combinatorial Purged CV / walk-forward** (López de Prado) | Not synthetic paths — multiple train/test *splits* of the real path with purging + embargo | The real data; defends against leakage + overfit | Directly yields PBO + Deflated Sharpe; the gold standard for "did I overfit"; no generative model to mis-specify | Bounded by the one historical realization (no tail-event synthesis); more about *splitting* than *simulating* | Low–medium (window + embargo lengths) | Strong — complements bootstrap; orthogonal guard |
| **3** | **GBM / parametric path simulation** (calibrated drift/vol) | Fully synthetic lognormal price paths | Mean drift + constant vol only | Trivial to implement (**already in tree** — `synthetic_bars()`); unlimited paths; analytically clean | **Wrong for crypto**: Gaussian returns → no fat tails, no vol clustering; systematically *understates* tail/drawdown risk ([ScienceDirect](https://www.sciencedirect.com/topics/engineering/geometric-brownian-motion), [Diversification.com](https://diversification.com/term/geometric-brownian-motion)) | Low (μ, σ) | **Weak.** Will produce optimistic drawdown tails; usable only as a smoke-test baseline |
| **4** | **Merton jump-diffusion / Hawkes-jump GBM** | GBM + Poisson(/self-exciting) jump component | Drift + vol + fat tails + (Hawkes) clustering | Directly addresses GBM's fat-tail failure; tractable extension ([DEV / Murtazin](https://dev.to/ayratmurtazin/volatility-clustering-with-merton-hawkes-jump-diffusion-simulations-in-python-4ibh)) | More params to calibrate (jump intensity, size dist); calibration is itself an overfit surface | Medium–high | Medium — better tails than GBM but adds a calibration burden |
| **5** | **GARCH-simulated paths** | Synthetic returns from a fitted GARCH(1,1) (or EGARCH) process | Vol clustering explicitly; fat tails if Student-t innovations | Captures the #1 crypto stylized fact (clustering) generatively; unlimited paths | **Project-tainted** — see § 2.6; calibration is the hard part; mis-specified GARCH propagates bias into every path | High | Medium — *as a generator* it is defensible; the retirement was of GARCH-as-**forecaster**, a different role |
| **6** | **Regime-switching synthetic generator** (HMM / Markov-switching) | Paths that switch between fitted regime models (bull/bear/chop) | Regime structure + per-regime moments | Can synthesize regime *sequences* absent from the single historical path | **Project-tainted** — § 2.6; double calibration (regimes + per-regime dynamics); highest overfit surface | Very high | Low–medium — most powerful, least defensible first; revisit only if bootstrap proves insufficient |

### 2.2 Method 1 — stationary block bootstrap (the recommended first bet)

The stationary bootstrap resamples blocks of consecutive real returns with
**random (geometric-distributed) block lengths**, producing a new series that
is itself stationary and — because it splices real return blocks — inherits
the real fat tails and within-block volatility clustering without any
distributional assumption ([Politis–Romano 1994](https://www.tandfonline.com/doi/abs/10.1080/01621459.1994.10476870);
[Politis impact survey](https://mathweb.ucsd.edu/~politis/impactBOOT.pdf)).
It is "less sensitive to block-size misspecification than the moving-blocks
bootstrap" and "has become a standard tool in financial time series analysis
and backtesting." The single tunable (expected block length) can be
auto-selected ([Politis–White 2004](https://public.econ.duke.edu/~ap172/Politis_White_2004.pdf)).

**Why first:** lowest calibration risk (one parameter, auto-selectable), no
generative model to mis-specify, and it directly answers "is my Sharpe a
property of the strategy or an artifact of this exact 2023 ordering?" by
re-ordering real return blocks. It is the method least likely to repeat the
retired-track failure mode (overfitting a generative model and mistaking
model artifacts for alpha).

### 2.3 Method 2 — Combinatorial Purged CV / walk-forward (the overfit guard)

Distinct from synthetic-path methods: CPCV systematically constructs many
train/test splits, *purges* overlapping samples, and *embargoes* a buffer to
kill look-ahead leakage, then reports a **distribution** of performance across
splits plus a Probability of Backtest Overfitting and a Deflated Sharpe Ratio
([López de Prado, SSRN 3104847](https://www.smallake.kr/wp-content/uploads/2018/07/SSRN-id3104847.pdf);
[Quant Beckman with-code](https://www.quantbeckman.com/p/with-code-combinatorial-purged-cross);
[Wikipedia: Purged CV](https://en.wikipedia.org/wiki/Purged_cross-validation)).
"CPCV produces a distribution of performance metrics enabling more rigorous
statistical inference." This is the orthogonal complement to the bootstrap:
the bootstrap perturbs *paths*; CPCV perturbs *the train/test partition* of
the real path. The TCN threshold-tuning ship needed exactly this guard and
did not have it.

### 2.4 Method 3 — GBM (already in tree, demoted to smoke-test)

`crates/backtest/src/main.rs:951` already contains a Box-Muller GBM generator
(`synthetic_bars()`). It is honest to acknowledge GBM's documented failure for
crypto: Gaussian returns produce **no fat tails and no volatility clustering**,
so GBM-derived drawdown and VaR are systematically optimistic
([ScienceDirect overview](https://www.sciencedirect.com/topics/engineering/geometric-brownian-motion);
[arXiv 2601.14272 on crypto VaR](https://arxiv.org/pdf/2601.14272)). Recommendation:
keep GBM only as a *baseline smoke-test* (does the harness run N paths at all?),
never as the robustness verdict source.

### 2.5 Methods 4–6 — jumps, GARCH-gen, regime-switching (deferred)

Jump-diffusion and GARCH-with-Student-t innovations *do* fix GBM's tails and
clustering, and a regime-switching generator can synthesize regime sequences
the single 2023 path never contained. All three are deferred to a v0.2+
because each adds a **calibration surface that is itself an overfit risk** —
exactly the trap the retired tracks fell into. The discipline: prove the
bootstrap harness produces decision-grade distributions first; add a
generative model only if the bootstrap's inability to synthesize
out-of-history regimes becomes the binding limitation.

### 2.6 Honest cross-reference: why this is NOT the retired DL/vol/regime bet

The project retired four lines (`v25-dl-journey-retrospective`,
`v3-vol-overlay-noop-discovery`). Two methods above (GARCH-gen, regime-gen)
name techniques whose *forecasting* cousins were retired. The distinction is
load-bearing and must be stated plainly:

| Retired line | What it did | Why retired | How the MC bet differs |
|---|---|---|---|
| TCN / PatchTST forecasters | Predict next-period return → trade the prediction | No +0.10 Sharpe-delta on real OHLCV; F4 across 2 model families | MC does **not predict** anything. It evaluates an *existing* strategy across many realizations. No alpha is claimed from the synthetic data |
| `v3-volatility-forecaster` (GARCH) | Forecast σ → size positions by it | MODEL-BROKEN / NO-ALPHA / negative net-delta after noop-fix | GARCH-as-**generator** (§ 2.5) produces *paths to test on*, not a *sizing signal to trade*. Different role; the retirement was of the trading-signal role |
| `v3-regime-classifier` | Classify regime → route strategy | Draft; never promoted; commodity | Regime-**generator** would synthesize test paths, not gate live trades. Still deferred (rank 6) precisely because of overfit risk |

**The categorical difference:** the retired lines tried to *extract alpha
from a model's predictions*. The Monte-Carlo robustness bet tries to *measure
the variance of an already-shipped strategy's outcome* under input
perturbation. One hunts signal; the other quantifies uncertainty. The first
repeatedly failed; the second is standard practice
([PickMyTrade 2026 robustness guide](https://blog.pickmytrade.io/trading-strategy-robustness-testing-2026-guide/),
[Build Alpha](https://www.buildalpha.com/robustness-testing-guide/)). They
share vocabulary ("GARCH", "regime") but not epistemics.

---

## § 3. From point-estimate to distribution: the robustness harness

### 3.1 What "evaluate across N Monte-Carlo paths" produces

Today: `backtest(strategy, θ*, path) → { sharpe, sortino, max_dd, equity }`
(scalars). Proposed: `robustness(strategy, θ*, {P_1..P_N}) → distribution`.

| Output dimension | Point-estimate today | Distribution under N paths |
|---|---|---|
| Sharpe | `1.40` | p5 / p25 / **median** / p75 / p95 (e.g. `[0.3, 0.9, 1.4, 1.8, 2.4]`) |
| Max drawdown | `73%` (one path) | drawdown **tail**: median MaxDD + p95 MaxDD (the number that should gate `paper→live`) |
| Probability of loss | undefined | `P(final_equity < initial)` across the ensemble |
| Probability of positive Sharpe (PPSR) | undefined | fraction of paths with Sharpe > 0 (and > 1.0 for the gate) |
| Overfit risk | undefined | PBO (target < 15% per [PickMyTrade]) + Deflated Sharpe |

### 3.2 The three perturbation axes (mapping the operator's "different inputs")

A robustness run is the cross-product of three perturbation axes; each maps to
a phrase in the operator's intent:

1. **Path resampling** — bootstrap / re-simulate the price series. Answers
   "would this strategy have worked on a *plausibly different* market?"
2. **Parameter sweep** — vary `θ` over a grid around `θ*`. Produces a
   **sensitivity surface** (heatmap). Answers "is `θ*` a plateau or a peak?"
   The robustness read: "look for parameter areas that all show similar
   performance — if the best is 20, ±10% should be about as good"
   ([Build Alpha](https://www.buildalpha.com/robustness-testing-guide/)).
3. **Input perturbation** — jitter slippage_bps / taker_fee / fill-price-mode
   / start-price within plausible bounds. Answers "is the edge an artifact of
   my exact cost assumptions?"

### 3.3 Sketch of a robustness report (new artifact)

```
# Robustness Report — <strategy> @ <θ*> — <date>
Ensemble: N=500 stationary-bootstrap paths (block_len=auto), seed=0xROBUST01
Base path: top10-2023-fy-momentum-realdata

## Sharpe distribution        p5    p25   p50   p75   p95
                              0.31  0.88  1.40  1.81  2.39
## MaxDrawdown distribution   p50: 41%   p95: 78%   worst: 91%
## P(final_equity < start):   0.18
## P(Sharpe > 1.0) (PPSR):    0.61
## PBO (CPCV):                0.22   [FLAG: > 0.15 overfit threshold]
## Deflated Sharpe:           0.74

## Parameter sensitivity (lookback × k_long), median Sharpe heatmap
            k=2    k=3    k=4    k=5
 lb=30      0.9    1.1    1.0    0.8
 lb=60      1.2    1.4*   1.3    1.1     (* = shipped θ*)
 lb=90      1.1    1.3    1.2    1.0
 → plateau around (60, 3): ROBUST. No sharp peak.

## LLM narration (support pillar — § 5)
"Median Sharpe 1.40 holds across 500 resamples but the p5 of 0.31 and an
 18% probability of net loss indicate the 2023 result is path-favourable.
 The (lookback, k_long) plateau is reassuringly flat..."
```

Two things land here that the project has never produced: a **drawdown tail**
(the number the `paper→live` gate should actually use, per `strategic-reset`
§ 4.2) and a **plateau-vs-peak verdict** on the parameter grid.

---

## § 4. The learning loop — does it close? (the highest-leverage gap)

### 4.1 What is wired today

Read of `crates/reflection/` + every consumer of `retrieve_top_k` across the
workspace. The write side is fully closed; the read side is a single thin,
unverified path.

| Loop stage | Status | Evidence |
|---|---|---|
| **Write** lesson card per closed trade | **Closed** | `ReflectionWriterTap` in `exec::PaperEnginePublisher`; one `LessonCard` per closed trade with 32-dim deterministic embedding + regime tags |
| **Store + retrieve** `top_k(query, k)` | **Closed (mechanically)** | `SqliteReflectionStore`; `retrieve_top_k` works; `reports/render/memory_highlights.rs` reads cards for the operator report |
| **READ by a decision path** | **Single consumer, deferred verdict** | `crates/trader/src/llm_forecaster/` is the *only* decision-path consumer (ADR-0041 D2). It injects top-K cards into the **LLM prompt**. Whether that moves equity is the v3-llm-forecaster **Wave D verdict — deferred indefinitely** (no `ANTHROPIC_API_KEY`) |
| **ACT on a lesson** (change strategy/param selection) | **NOT WIRED** | No deterministic strategy reads lessons. Enforced by a gate test (`crates/reflection/tests/no_strategy_caller.rs::t1809`): the strategy crate is structurally **forbidden** from consuming reflection retrieval |

### 4.2 The gap between "record a lesson" and "act on a lesson"

There are **two** distinct gaps, and only naming both is honest:

- **Gap A (verification gap):** the one consumer that *does* read lessons
  (the LLM forecaster) feeds them to a prompt whose alpha verdict is deferred.
  Per `strategic-reset` § 2.5, "no production trade has been causally linked
  to lesson-card retrieval." So even the single read path is **unverified
  telemetry** until Wave D runs.
- **Gap B (architecture gap):** *no deterministic strategy can read lessons
  at all* — it is forbidden by the t1809 gate. The entire learning loop is
  routed through one LLM strategy. If the LLM path is noise-equivalent (the
  `strategic-reset` § 4.5 prior says this is the likely Wave D outcome), then
  the framework has **no learning channel left**, because the deterministic
  strategies are walled off from the memory by construction.

**Blunt statement:** today reflection memory is **write-mostly telemetry**.
It is read by exactly one strategy, into a prompt, on an unverified path, and
every other strategy is architecturally barred from it. "The framework
learns" is currently false in any load-bearing sense.

### 4.3 What "the framework learns" would concretely mean

Three operationalizations, increasing in ambition. None requires an LLM —
this is the key reframe (§ 5):

1. **Lesson-informed parameter selection (deterministic).** Before a run,
   query reflection for past outcomes of *this strategy in this regime* and
   nudge `θ` toward the params that historically won (e.g. shrink `exposure_cap`
   if recent same-regime cards are losses). A closed loop: outcome → lesson →
   future sizing. Today the t1809 gate forbids the strategy crate from doing
   this; it would need a sanctioned seam (a `trader`- or `agent`-level
   pre-run selector that reads lessons and *configures* the strategy, keeping
   the strategy crate itself consumer-free).
2. **Regime-conditioned strategy routing fed by past outcomes.** The
   `RegimeDispatcher` currently routes Bull/Bear → Momentum, Calm/Volatile →
   CashHold using **live classification only** (confirmed: it does not read
   reflection). A learning version would route based on *which strategy
   historically performed best in the detected regime*, retrieved from
   reflection. This closes the loop at the routing layer.
3. **Robustness-feedback (the § 3 connection).** A robustness run's
   distribution per parameter cell IS a lesson. "θ=(60,3) had median Sharpe
   1.4 but θ=(60,2) had a tighter p5" is exactly the kind of durable lesson
   the reflection store should hold and the next run should consult. The
   learning loop and the robustness harness are the same loop viewed twice.

This is the single highest-leverage honest gap because (a) it is the named
moat (`product.md` § Differentiator: persistent reflection memory), (b) it is
currently unverified AND architecturally walled off, and (c) it does not
depend on alpha-hunting — it is pure uncertainty-reduction + adaptation.

---

## § 5. LLM as support pillar (the reframe)

### 5.1 The demotion, argued from evidence

The evidence base for demoting the LLM from alpha-engine to support pillar is
already on disk:

- The v2.5 DL forecaster track (the numeric alpha-engine bet) is **retired**
  across two model families (`v25-dl-journey-retrospective`).
- `v3-volatility-forecaster` (numeric sizing-signal bet) is **retired**
  NO-ALPHA / negative net-delta.
- `v3-llm-forecaster` (the LLM-as-alpha-engine bet) is **shipped-partial**;
  its load-bearing verdict (Wave D) is deferred, and the `strategic-reset`
  § 4.5 prior rates it LOW-MEDIUM to clear the +0.10 Sharpe-delta gate.

Three alpha-engine bets; two retired, one unverified-and-priced-low. The
honest read: **alpha-engine-by-prediction has not paid off, regardless of
whether the predictor is a TCN, a GARCH, or an LLM.**

### 5.2 The reframed pillar stack

```
CORE (alpha + safety, all deterministic):
  1. Quantitative strategy (momentum / pairs / composed) — the edge candidate
  2. Monte-Carlo robustness layer (§ 2-3) — quantifies whether the edge is real
  3. Learning loop (§ 4) — adapts param/route selection from past outcomes
  4. Risk envelope + auditable double-entry ledger — the validated moat (D.1)

SUPPORT (the LLM pillar — explanation, not decision):
  - Regime narration: turn the detected regime + robustness distribution into
    a human sentence ("path-favourable; p5 Sharpe 0.31")
  - Lesson summarization: distill clusters of LessonCards into review-ready
    rules (product.md § "periodic distillation", currently deferred)
  - Human-readable robustness-report explanation (the § 3.3 narration block)
  - Tie-break ONLY: when two strategies/params are statistically
    indistinguishable on the robustness distribution, the LLM may break the
    tie with a narrated rationale — bounded, auditable, never the primary gate
```

The LLM stops being a signal source and becomes the **explanation and
narration layer over a quantitative core**. This aligns with the validated
moat: the audit ledger (component 4) is operationally proven
(`strategic-reset` § 3.2, the noop-fix incident); the LLM is the surface that
makes the quantitative robustness story legible, not the thing that generates
returns.

---

## § 6. Candidate features (SKETCH ONLY — do not promote)

Ranked by leverage. Each: one-line scope, rough dev-day estimate, dependency
on the architect's parallel readiness audit
(`monte-carlo-robustness-architecture-readiness-2026-05-29.md` — fires in
parallel; these estimates are PRELIMINARY pending its findings). **None of
these is a trace.toml row yet.**

| Rank | Candidate | One-line scope | Rough est. | Architect-audit dependency |
|---|---|---|---|---|
| **C1** | **Path-ensemble generator in `crates/data`** | Stationary block bootstrap (Politis–Romano) over real return series → `N` seeded synthetic paths; GBM smoke-test as fallback | 4-6 dev-days | Where does it live (`data` vs new `mc` crate)? Does it reuse `synthetic_bars()` plumbing? Seeded-determinism contract — **the § 8 tension** |
| **C2** | **Robustness-harness backtest mode** | `--robustness --paths N` flag → runs the ensemble, emits the § 3.3 distribution report (Sharpe percentiles, DD tail, P(loss), PPSR) | 5-8 dev-days | How a multi-path run coexists with the single-path anchor gate (§ 8); report-render reuse |
| **C3** | **Parameter-sweep runner** | Grid over `θ` neighborhood → sensitivity heatmap + plateau-vs-peak verdict (the § 3.2 axis 2) | 3-5 dev-days | Reuse of `threshold_sweep.rs` `run_cell` pattern (already exists for τ×ε); generalize to arbitrary `θ` |
| **C4** | **Reflection-feedback decision seam** | Sanctioned pre-run selector (in `trader`/`agent`, NOT `strategy`) that reads `top_k` lessons and *configures* the strategy — closes Gap B without breaking the t1809 wall | 5-8 dev-days | The hard one: where the seam lives so the strategy crate stays consumer-free; interaction with ADR-0041 layering |
| **C5** | **CPCV / Deflated-Sharpe overfit guard** | Combinatorial purged CV split runner → PBO + Deflated Sharpe over the real path | 4-6 dev-days | Orthogonal to C1; may live beside C2; pure-analysis, no live-trade path |

**Recommended first slice (durable):** C1 + C2 together as a v0.1.0 —
"bootstrap path generator + robustness report mode." C1 alone produces paths
nothing consumes; C2 alone has nothing to run. They are the minimum coherent
unit. C3 (sweep) is a fast follow that reuses the existing `threshold_sweep`
machinery. C4 (learning seam) is the highest-leverage but highest-architecture-
risk — it should follow the architect's readiness verdict on the t1809 wall.

---

## § 7. Operator-decide questions

Per the 2026-05-28 durable-over-quick contract, the `(Recommended)` tag is on
the **most durable** option, with an explicit if-budget-tightens fallback
named. Cost framing quotes both wall-clock and rework risk.

### Q-MC-1: Which synthetic method ships first?

- **(a) Stationary block bootstrap (Recommended)** — ~4-6 dev-days now; zero
  follow-on calibration-debt. Reuses real crypto returns so fat tails +
  clustering are free; one auto-selectable parameter; the method least likely
  to repeat the retired-track overfit failure. Durable because no generative
  model can become mis-specified later.
- **(b) GBM (already in tree)** — ~1 dev-day now (wire `synthetic_bars` into a
  loop) **+ a v0.2.0 cleanup commitment** to replace optimistic Gaussian tails
  before any `paper→live` gate trusts the drawdown number. Fallback only if
  budget is tight and the operator accepts a smoke-test-quality first pass.
- **(c) GARCH / regime generative** — defer; § 2.5/§ 2.6 overfit risk;
  re-propose only if bootstrap proves insufficient.

*If budget tightens:* take (b) as a 1-day smoke-test BUT annotate the
`paper→live` gate that its drawdown tail is GBM-optimistic until (a) lands.

### Q-MC-2: Robustness distribution — new report type, or extend the existing report?

- **(a) New `robustness-*.md` report type with its own anchor scheme
  (Recommended)** — ~2-3 extra dev-days vs extending; durable because a
  distribution report is structurally different from a single-path report
  (percentile tables, heatmaps, PBO) and forcing it into the single-path
  template spawns a v0.2.0 refactor. Pairs cleanly with the § 8 anchor
  decision (anchor the *distribution summary*, not each path).
- **(b) Extend the existing single-path report** — ~1 day now **+ a v0.2.0
  refactor commitment** when the percentile/heatmap content outgrows the
  template. Fallback if the operator wants the fastest possible first look.

### Q-MC-3: Close the learning loop now, or after robustness lands?

- **(a) Robustness first (C1+C2), learning seam (C4) second (Recommended)** —
  the robustness harness *produces the lessons* the learning loop should
  consume (§ 4.3 item 3). Building C4 before C2 means the loop has nothing
  high-quality to learn from. Sequencing robustness-then-learning means C4
  consumes a real distribution, not a hypothetical one. ~9-14 dev-days total,
  correctly ordered, no rework.
- **(b) Learning seam first** — risks building the adaptation channel before
  there is verified signal to adapt on, repeating the `strategic-reset` § 4.1
  "build the visibility layer before testing the moat half" anti-pattern.
  Fallback only if the operator weights the moat-narrative (component 2
  visibility) over robustness rigor.

*Cross-link:* this interacts with `strategic-reset` M1 (Wave D). If Wave D
runs and is noise-equivalent, C4 (deterministic learning seam) becomes MORE
important, not less — it is the learning channel that does not depend on the
LLM (§ 4.2 Gap B).

### Q-MC-4: Does the LLM-as-support reframe get ratified into `product.md`?

- **(a) Ratify now (Recommended)** — amend `product.md` § LLM strategy +
  § Differentiator to state the core = quantitative + robustness + learning,
  LLM = support. ~0.5 day analyst. Durable because it stops future sessions
  re-proposing LLM-as-alpha-engine (three such bets have now under-delivered).
- **(b) Defer ratification until Wave D verdict** — keeps `product.md` honest
  about an open question, but risks another alpha-engine-by-LLM proposal in
  the interim. Fallback if the operator wants Wave D evidence in hand first.

---

## § 8. The hard tension (flag for architect)

**Monte Carlo is stochastic (many paths); the project's entire quality system
is byte-identical anchored single-path backtests.** This is a genuine
collision and the architect's readiness audit must resolve it. Naming it, not
solving it:

- **The anchor contract** (`spec/anchors.toml`, ADR-0038 § D6): every shipped
  backtest report has a body-SHA-256 that must be byte-reproducible. The
  `verify_anchors.sh` gate is load-bearing and a non-negotiable
  (CLAUDE.md). A multi-path robustness run produces a *distribution*, and the
  naive read is "distributions aren't byte-identical → can't anchor."
- **But determinism is recoverable.** The codebase already uses seeded
  `ChaCha20Rng` everywhere a path is synthesized (`synthetic_bars()`,
  `PaperEngine::new(config, seed)`). A robustness run is deterministic IF the
  ensemble is generated from a single master seed → a fixed, reproducible
  **set** of N paths. The candidate resolution (for the architect to
  confirm/refute): **seed the ensemble from one master seed → byte-identical
  path SET → anchor the *distribution summary* (the percentile table + PBO),
  not each path.** Same seed → same 500 paths → same percentiles → same
  body-SHA.
- **The subtle risk:** the `strategic-reset` § 6.2 meta-lesson — "byte-identity
  across structurally-DIFFERENT runs is the no-op signature." A robustness
  report's anchor must be sensitive to the strategy/params (different strategy
  → different distribution → different SHA) while being invariant to
  re-runs (same seed → same distribution → same SHA). The anchor must hash the
  *summary statistics*, and the harness must include a baseline-divergence
  guard (per the CLAUDE.md non-negotiable for overlays) so a robustness run
  that secretly collapses to a single path is caught.

**Handoff to architect:** the questions are (1) does seeded-ensemble →
distribution-summary-anchor satisfy the ADR-0038 byte-identity contract? (2)
where does the path-set seed live in the determinism hierarchy
(`scenario_seed` is currently used for fill tie-breaks AND the synthetic
fixture — does a robustness `ensemble_seed` need to be orthogonal)? (3) does
the existing `threshold_sweep::run_cell` "load-once-use-N-times" pattern
generalize to the path ensemble? These are deferred to
`monte-carlo-robustness-architecture-readiness-2026-05-29.md`; this note only
names the collision and sketches the seeded-set resolution.

---

## § 9. Assumptions (challengeable by architect/operator)

1. `product.md` § Differentiator (persistent reflection memory) + the
   `strategic-reset` half-validated-moat finding are the canonical priors;
   this note extends, does not contradict, them.
2. The strategy crate's structural ban on reflection retrieval (t1809 gate) is
   a deliberate ADR-0041 layering rule, not an accident — closing Gap B means
   adding a sanctioned seam *outside* the strategy crate, not deleting the gate.
3. Stationary block bootstrap is the lowest-overfit-risk synthetic method for
   crypto because it resamples real returns; this is the literature consensus,
   not a project-specific claim.
4. The deterministic-anchor contract is non-negotiable; any robustness harness
   must reduce to a byte-reproducible distribution summary under a master seed
   (the § 8 resolution), or the architect must reject the approach.
5. The architect's parallel readiness audit owns the crate-placement, seam, and
   anchor-mechanism decisions; § 6 dev-day estimates are preliminary and will
   be re-priced against its findings.
6. No alpha is claimed from synthetic data — the bet is uncertainty
   quantification + adaptation, categorically distinct from the retired
   prediction bets (§ 2.6).

---

## Changelog

- 2026-05-29 (analyst): initial direction note. Names the limit (fixed
  parametric strategies on single deterministic paths), surveys 6 Monte-Carlo
  synthetic-data methods (bootstrap ranked #1, GBM demoted, GARCH/regime-gen
  deferred with the retired-line distinction made explicit), defines the
  distribution-valued robustness report, identifies the half-closed learning
  loop as the highest-leverage gap (write-mostly telemetry; one LLM consumer;
  deterministic strategies architecturally walled off by t1809), reframes the
  LLM as a support pillar, sketches 5 candidate features (no promotion), poses
  4 operator-decide questions (durable-biased), and hands the stochastic-vs-
  anchor tension to the architect with a seeded-ensemble resolution sketch.
