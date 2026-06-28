# Application — Synthetic & Monte-Carlo Test Data (why we resample reality)

*Decision doc for analyst + architect. Distilled from `research/data/knowledge.md`
(primary) and the 100-entry `research/data/papers.md` ledger (cited `data[N]`), with
the cross-topic `research/SYNTHESIS.md` roadmap (P2 generators). This is the **"what
do we change in the app"** layer for the Monte-Carlo / synthetic-data-generation
strand. It does NOT add papers.*

> **Scope of this file:** the bootstrap family (moving-block / circular /
> stationary / tapered) and *why* block length is the load-bearing knob; the learned-
> generator zoo (GAN / diffusion / VAE / signature / copula) and the mechanistic
> branch (agent-based simulators); the decisive verdict that **generators
> structurally smooth tails** so the **model-free block bootstrap stays the
> default**; the one honest limit of resampling (can't invent a worse-than-seen
> crash) and how to fill it (tail-stressed slice / EVT, not a generic GAN). The
> leakage/CV strand and the PIT/labeling strand live in the two sibling files.

---

## 1. Summary of the research

The headline is unusually clean and consistent across the whole strand: **for a
single-coin, bar-level, risk-focused gate, resampling reality beats generating it.**

- **Dependent-data resampling is a design space, not one method.** The block-
  bootstrap family — moving (MBB), circular (CBB), stationary/random-length (SBB),
  non-overlapping (NBB), tapered (TBB) — plus model-based variants (sieve, residual,
  Markov) all preserve the serial dependence an i.i.d. bootstrap destroys. **Block
  length is the load-bearing tuning knob.** `data[1]`.
- **The MBB our gate runs has a name, a proof, and a precise reason block length
  matters.** Künsch (1989) is the foundational moving-block bootstrap; consistency
  requires **l → ∞ and l/n → 0**. The full-text deepening: the MBB variance estimate
  *is* the **spectral density of the returns at zero frequency = the long-run
  variance** (the sum of all autocovariances, not the lag-0 variance). So a block
  **too short to span the coin's correlation length under-estimates the long-run
  variance → over-narrow, over-optimistic confidence bands** — the gate would look
  more decisive than the data warrant. `data[84]`.
- **Block length should be data-driven, and the "optimal" can't be pinned down.**
  Politis-White's spectral plug-in sets it from the correlogram (stronger persistence
  ⇒ longer blocks) `data[18]`; selectors converge *slowly* (n^-1/6 to n^-1/3)
  `data[89]`; and the optimum **differs for quantile targets** (drawdown/CVaR) vs
  variance targets `data[88]`. The honest posture: pick a defensible length, document
  it, and **sensitivity-check** across nearby lengths and MBB↔stationary `data[47][100]`.
- **The whole generator zoo is mapped, and every branch is heavier, harder to
  validate, and less reproducible than block-resampling.** GAN (TimeGAN `data[76]`,
  Quant-GAN `data[5]`, Deep-Hedging market simulator `data[92]`), **signature**
  methods (Sig-WGAN `data[85]`, SOCK feature-matching `data[91]`), diffusion
  `data[11][65]`, **causally-constrained VAE** `data[86]`, copulas `data[33][69]`,
  and agent-based (ABIDES / RL-agent crypto ABM `data[6][34][71]`).
- **The decisive verdict for RISK is consistent — with a root cause.** A VaR-focused
  review of 14 models finds **Historical Simulation, GARCH, and one CWGAN are the top
  performers** `data[87]` — Historical Simulation is the non-block sibling of our
  bootstrap, i.e. *resampling reality wins for risk*. VAEs repeatedly **smooth away
  extremes** `data[50][61]`, and the rare-events survey gives the **structural
  reason: a Gaussian latent prior fundamentally cannot generate heavy tails** — the
  smoothing is architectural, not a tuning miss. `data[72]`.
- **Generators overfit the one path you have.** On a single short crypto history,
  adversarial training overfits the lone realization `data[91]`; even the most
  *theoretically principled* generator — the causal-Wasserstein-bounded TC-VAE — has
  a bound whose constant **C = 2(2^T − 1) blows up exponentially in path length T**,
  so the guarantee is near-vacuous for a realistic multi-hundred-bar window. `data[86]`.
- **The honest limit of our own method.** Plain Historical Simulation (and our block
  resample) is **capped by the worst historical scenario** — it can reshuffle the
  crashes the coin actually had but **never invent a worse-severity one**. `data[87]`.
- **Generic time-series augmentation is dangerous for finance.** Jitter / rotation /
  magnitude-warp inject physically meaningless artifacts (rotating a return series
  destroys the leverage/sign structure); only dependence-preserving block/permutation
  resampling is defensible, and augmentation only helps when there's signal to begin
  with. `data[12][22][59]`.

---

## 2. Possible solutions / what can be done with this research

1. **Keep the moving-block bootstrap as the gate default — it's already correct.**
   The gate's `compute_robustness_distribution` in
   `crates/backtest/src/bakeoff/bootstrap.rs` already draws 1000 MBB resamples and
   already chooses block length via `data::synth::block_length::politis_white_block_length`
   (Politis-White PWSD — *not* a magic constant). The research validates this exactly.
2. **Add a one-off block-scheme sensitivity check.** Re-run the weakest-link verdict
   with tapered + stationary/random-length blocks and across nearby lengths; confirm
   the verdict is invariant. If it flips, the block choice is doing hidden work and
   must be documented. `data[1][47][89][100]`.
3. **Consider a separate block scheme for tail/quantile statistics.** The block
   length tuned for Sharpe-style means may be sub-optimal for the drawdown/CVaR
   distribution; the hybrid-quantile theory is the right tool *if* we ever report
   bootstrap CIs specifically for drawdown/CVaR. `data[88]`.
4. **Fill the one honest gap with a tail-stressed slice, not a generator.** A
   deliberately tail-stressed resample (or, properly done, an EVT-augmented
   generator) addresses "worse-than-seen crash" without the tail-smoothing that GAN/
   VAE introduce. `data[72][87]`.
5. **Use generators (if ever) only research-only, with the full evaluation battery.**
   Distributional + temporal + stylized-fact + downstream-utility + TSTR ("train on
   synthetic, test on real"); prefer interpretable diffusion over GAN/VAE; demand a
   causal constraint (real-order, no look-ahead) and an explicit EVT-style tail check.
   `data[49][61][65][86]`.
6. **Use GBM/GARCH/OU synthetic *no-alpha* series to validate the gate.** The job
   here is the *opposite* of "fabricate data to test a strategy on" — it's
   "fabricate a *signal-free* series and confirm the gate refuses to crown." This is
   a legitimate, safe use of synthesis. `crates/data/src/synth/gbm.rs` already exists.

---

## 3. Relevance for the project

- **It validates the gate's core design at primary-source depth.** Our 1000-path MBB
  is Künsch's moving-block bootstrap applied to a coin's return series `data[84]`; it
  preserves the tails and volatility-cluster *positions* `data[13][90]` that decide
  whether a strategy survives a crisis; it can't fabricate dynamics the asset never
  had `data[4][30][91]`; and it's reproducible (deterministic, seeded — see the
  frozen ADR-0051 sub-seed rule in `bootstrap.rs`). For an advisor selling
  reproducible, auditable risk, that tradeoff is exactly right.
- **It tells us what *not* to build.** The strongest, most-cited message of the whole
  data folder is that learned generators understate exactly the crash risk crypto has
  `data[4][11][30][50][72]`, overfit a single short path `data[91]`, and add nothing
  without signal `data[22]`. So the "should we add a GAN/diffusion synthetic-data
  module?" question has a research-backed answer: **no, keep it research-only.**
- **It is honest about our one weakness.** Block resampling cannot invent a worse-
  than-seen crash `data[87]`. For a crypto advisor — where the next crash may exceed
  every historical one — this is a real limitation to *state*, and the principled fix
  is a tail-stressed slice / EVT, not a generic generator that would *understate*
  tails. Naming this limitation honestly is itself part of "traceable and plausible."
- **Expected-null, honestly.** None of this produces alpha. The durable value is
  protecting the verdict from a lucky single path and from a tail-smoothing test
  distribution. The block bootstrap is the credibility machine; the research confirms
  we built the right one.

---

## 4. Advantages for the project

- **Reproducibility.** The MBB is deterministic and seeded; a learned generator is
  training-fragile and non-reproducible. For an auditable advisor, deterministic
  resampling is a credibility asset.
- **Tail fidelity by construction.** Real blocks in real order inherit *this coin's*
  actual tail index, clustering, and (crypto-flipped) leverage sign — no Gaussian-
  prior tail-smoothing `data[42][43][44][72]`.
- **Causal by construction.** The block bootstrap never leaves the real-data manifold
  and needs no exponential-in-T robustness bound `data[86]` — it is leakage-free by
  design, which dovetails with the PIT discipline in the sibling file.
- **Honesty.** Stating the one real limit (can't invent a worse crash) and addressing
  it with a tail-stressed slice rather than a tail-smoothing generator is a stronger,
  more defensible position than claiming a generator gives us "unseen regimes."

---

## 5. Problems and challenges

- **HARD CONSTRAINT — gate/bands FROZEN; the block-length policy is part of the
  frozen contract.** The block-scheme/length sensitivity check is a **diagnostic**,
  not a change to the gate. Changing the *production* block scheme or selector would
  touch the FROZEN gate (ADR-0063 cites Politis-White) and needs an ADR plus re-
  baselining. Run the sensitivity check as a one-off; only promote a change through
  the frozen-gate process.
- **HARD CONSTRAINT — Decimal not f64.** The bootstrap already crosses into f64 for
  the statistical layer (`equity_to_log_returns_f64`, `returns_to_equity_decimal`
  with documented fallbacks). Any tail-stress or alternative-scheme work must keep the
  f64 island contained and round-trip back to Decimal for financial outputs.
- **HARD CONSTRAINT — determinism / anchor safety.** `compute_robustness_distribution`
  is a pure function of (equity, seed, paths); the `RobustnessMode::Bootstrap` arm is
  opt-in and never called by an anchored CLI report path. Any new scheme must preserve
  determinism (seeded `ChaCha20Rng`, the frozen `GOLDEN_GAMMA` sub-seed rule, the
  16-entry `SALT_TABLE`) or it breaks reproducibility — the whole point.
- **Generators are a tar pit of validation burden.** If ever scoped, a generator
  needs the full battery (distributional + temporal + stylized-fact + utility + TSTR
  + EVT tail check) `data[49][61]`, validation on known-ground-truth synthetic data
  *before* real data `data[4][32]`, and even then the causal-Wasserstein guarantee is
  near-vacuous for our path lengths `data[86]`. This is a research program, not a
  feature — P2 at best.
- **Tail-stress can lie in the other direction.** A hand-tuned "make it worse" slice
  can fabricate a crash the coin's dynamics would never produce, which is its own form
  of dishonesty. An EVT-grounded extrapolation is more defensible than an ad-hoc
  multiplier — but it is harder, and crypto-crash synthesis specifically is still an
  open problem `data[72]`.
- **Imputation is a leakage surface that overlaps this strand.** Never bootstrap over
  imputed bars as if they were real observations — it understates uncertainty
  `data[35]`. (See the PIT/labeling sibling file for the gap-handling policy.)

---

## 6. Concrete next steps / candidate work items

**P1 — Block-scheme / block-length sensitivity check (one-off diagnostic).**
- **What:** re-run the weakest-link verdict on a representative coin/window with
  (a) tapered and (b) stationary/random-length blocks, and across ±a few nearby
  lengths around the Politis-White value; confirm the verdict is invariant. Document
  the result as evidence the frozen block choice is not doing hidden work.
- **Where:** a one-off harness reading `crates/backtest/src/bakeoff/bootstrap.rs` +
  `crates/data/src/synth/block_length.rs` and `crates/data/src/synth/bootstrap.rs`
  (the SBB variant likely already lives in the synth bootstrap module). Output is a
  diagnostic report, *not* a gate change.
- **Priority rationale:** cheap, high-credibility; directly answers a standing open
  question. Diagnostic-only ⇒ no frozen-gate ADR needed.

**P1 — No-alpha-gate standing regression test (shared with the leakage/CV file).**
- **What:** run the gate on GBM/GARCH/OU no-alpha series; assert it refuses to crown
  and that DSR/PBO flag overfit picks. The safe, legitimate use of synthesis.
- **Where:** `crates/backtest/tests/` consuming `crates/data/src/synth/gbm.rs`
  (GBM exists; GARCH/OU may need adding to `synth/`).

**P2 — Tail-stressed "worse-than-seen-crash" slice (research spike).**
- **What:** prototype an EVT-grounded tail-stress resample as an *optional, clearly-
  labeled* stress lens beside the gate (never the default). Validate it preserves the
  coin's non-tail stylized facts while extending only the tail. Decide whether it
  adds credibility or just noise.
- **Where:** research spike feeding `crates/data/src/synth/`; do NOT wire into the
  FROZEN gate. `data[72][87]`.

**P2 — Quantile-aware block scheme for drawdown/CVaR CIs (research spike).**
- **What:** if we ever surface bootstrap CIs specifically for max-drawdown or CVaR,
  evaluate the hybrid-quantile block theory vs the variance-tuned length.
- **Where:** research spike; `data[88]`.

**P2 (or NEVER) — Learned generators.** Keep research-only. Only revisit if the
tail-stressed slice proves insufficient AND a concrete need for genuinely-unseen
regimes emerges (e.g. simulating a crash a young coin never experienced). Then demand
causal constraint + utility/TSTR + EVT tail check. `data[49][86][72]`.

---

## 7. Open questions for analyst & architect

1. **Does the verdict survive a block-scheme swap?** This is the one empirical
   question worth answering immediately — if the weakest-link verdict is invariant to
   tapered/stationary/nearby-length, the frozen MBB choice is vindicated; if it flips,
   the block choice is load-bearing and must be documented (and possibly re-examined
   through the frozen-gate ADR).
2. **Do we want a tail-stress lens at all?** It addresses our one honest weakness but
   risks fabricating an unrealistic crash. Is an EVT-grounded slice worth the
   complexity, or is "we state the limit honestly and don't pretend to cover it" the
   better product posture?
3. **Quantile block length:** do any of our reported tail statistics (the `p95`
   max-drawdown gate signal, any future CVaR surfacing) warrant a separate block
   scheme, or is the single Politis-White length adequate given we sensitivity-check?
4. **GARCH/OU in `synth/`:** do we extend `crates/data/src/synth/` with GARCH and OU
   generators for the no-alpha-gate test, or is GBM sufficient as the standing
   negative control?
5. **Where does `synth/bootstrap.rs` fit the sensitivity check?** It already contains
   bootstrap variants — does it expose tapered/stationary schemes the diagnostic can
   call directly, or do they need adding?

---

## 8. What NOT to do / effort & blast radius

- **Do NOT build a GAN/diffusion/VAE synthetic-data module for the gate.** The
  research is emphatic: they structurally smooth tails (Gaussian-prior root cause),
  overfit a single short path, and are training-fragile; Historical Simulation /
  GARCH tie-or-beat them on the exact (VaR/risk) task we care about. `data[72][87][50][91]`.
- **Do NOT import generic CV-style augmentation** (jitter/rotation/magnitude-warp) —
  physically meaningless for prices, injects artifacts, and "Reverse/sign-flip" hurt.
  `data[12][59]`.
- **Do NOT change the production block scheme casually.** The sensitivity check is a
  diagnostic; promoting any change touches the FROZEN gate and needs an ADR + re-
  baseline.
- **Do NOT bootstrap over imputed bars.** An imputed bar is not a real observation;
  resampling it understates uncertainty. `data[35]`.
- **Effort / blast radius:** the P1 sensitivity check is **low effort, zero gate
  blast radius** (diagnostic-only). The tail-stress and quantile-block items are
  **research spikes**, deliberately kept off the FROZEN gate. The default — keep the
  model-free MBB — is **zero new code** and is the research-backed correct choice.
