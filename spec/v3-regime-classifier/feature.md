---
slug: v3-regime-classifier
status: draft
owner: analyst
updated: 2026-05-22
version: 0.1.0
predecessor: spec/dev-notes/strategy-reformulation-survey-2026-05-22.md (Candidate 2)
parent: none
sibling_picks:
  - v3-volatility-forecast (Candidate 1; ships first under HYBRID sequencing)
  - v3-llm-as-forecaster   (Candidate 5; parallel analyst pass)
---

# v3 — Regime classifier (predict regime label, not μ)

> **Spec-only design exploration.** Per operator-decide 2026-05-22 under
> the strategy-reformulation-survey resolution: Q-PICK = {C1 + C2 + C5},
> Q-BUDGET ≈ 6-8 weeks total, **Q-SEQ = HYBRID**. Candidate 1
> (volatility) builds first. **This Candidate-2 analyst pass produces a
> full design brief but NO code commitment until C1 ships its verdict OR
> operator explicitly promotes this feature.** Architect M-T1 + developer
> waves are DEFERRED — see `## Deferred milestones` below for the
> activation contract.

## Provenance + load-bearing finding

This brief is the analyst pass for **Candidate 2 (regime classification)**
in the
[strategy-reformulation survey](../dev-notes/strategy-reformulation-survey-2026-05-22.md)
of 2026-05-22, which retired the v2.5/v2.5a DL forecast-overlay umbrella
on joint F4-F4-F4 evidence and freed a ~3-5 week budget for orthogonal
research directions. Operator routing at survey close picked C1 + C2 +
C5 with hybrid sequencing — C1 builds first; C2 and C5 produce analyst
briefs in parallel and queue for activation only after C1's verdict
lands.

**Load-bearing finding (the seed).**
[`crates/reflection/src/regime.rs`](../../crates/reflection/src/regime.rs)
already ships a **pure-function 3-state BTC daily-close regime tagger**
that this feature extends. Verbatim contract today:

- `pub enum RegimeTag { Bull, Bear, Chop }` — `Display` emits
  `bull|bear|chop` lowercase (body-byte stable; downstream lesson-card
  embeddings + memory-screen renderer depend on this stability).
- `pub const REGIME_THRESHOLD_RATIO: Decimal = dec!(0.02)` — ±2%
  threshold pinned analyst-strawman; boundary at exactly ±2% maps to
  `Chop` (strict inequality, R1.3).
- `pub fn classify_regime(btc_closes: &[(Timestamp, Decimal)], at: Timestamp) -> Result<RegimeTag, RegimeError>`
  — trailing 7-day return on a BTC daily-close series; `Decimal` only,
  no `f64`; no I/O; no clock.
- `RegimeError` variants: `NoCloseAtTimestamp`, `NoCloseAtMinus7d`,
  `ZeroReferenceClose` — exhaustive over the failure surface.

**Live consumers of `RegimeTag` today** (verified by grep
2026-05-22 — every one of these MUST keep compiling byte-identical
after this feature lands):

- `crates/reflection/src/embedding.rs:24` — embeds `RegimeTag` into
  lesson-card vectors (lesson-card recall depends on this stable
  ordinal encoding).
- `crates/reflection/src/store/*` — `LessonCard.entry_regime` +
  `LessonCard.exit_regime` fields; persisted on disk.
- `crates/reflection/tests/store_smoke.rs`,
  `tests/post_mortem_generate_card.rs`,
  `tests/embedding_determinism.rs`,
  `tests/store_top_k_determinism.rs`,
  `tests/regime_classifier.rs` (T1802 — boundary case + Bull/Bear/Chop
  + determinism gate) — `RegimeTag` literal references; **byte-identity
  invariant** under any extension this feature lands.
- `crates/reports/tests/memory_highlights_with_lessons.rs`,
  `tests/body_no_volatile_metadata.rs`,
  `tests/report_scenarios_with_lessons.rs`,
  `tests/fixtures/build_reflection_store_{7d,90d}.rs` — `RegimeTag`
  literal references in fixture builders (Phase F Memory/Models
  renderer pipeline). The 30-anchor v2.5-chain anchor body-SHA invariant
  intersects here through the memory-highlights report renderer.

This is **the cheapest possible seed** — the type, the const, the
function signature, the test fixtures, the lesson-card embedding shape
all exist. The feature extends; it does not replace.

## Why

The strategy-reformulation survey of 2026-05-22 surfaced regime
classification as the analyst's #2 cost-effectiveness pick (close
behind C1 vol). Reasoning carried forward here verbatim:

1. **Conceptually orthogonal to v2.5 F4 evidence.** v2.5 asked
   "predict the future μ"; regime classification asks "label the
   present state". Fundamentally easier task with established
   evidence (HMM works on financial time series at multiple
   horizons; literature spans 30+ years from Hamilton 1989 onward).
2. **Crypto markets DO exhibit clear regime shifts.** Mar 2020 crash,
   Jan-Nov 2021 bull, May-Jul 2022 deleveraging, Q4 2023 ETF rally are
   not subtle. An HMM (or even a simpler statistical classifier) on
   realdata 2-year window should identify them with high confidence.
3. **The seed already exists.** `crates/reflection/src/regime.rs`'s
   3-state Bull/Bear/Chop tagger gives the entire downstream stack
   (lesson cards, memory screen, embedding) a stable contract; this
   feature extends rather than reinvents.
4. **Compounding value.** A working regime classifier composes with
   C1 (vol forecasts) into a regime-conditional vol overlay; it
   composes with C5 (LLM-as-forecaster) as a contextual feature in
   the LLM prompt; it composes with v1 momentum as the dispatch
   layer. Strong building-block for the survey's Candidate 7
   (strategy reformulation), if the operator ever picks it.
5. **Cost is small.** ~3-5 weeks per survey estimate. Compute is
   tiny (HMM Baum-Welch fits in minutes; small classifier trains
   in hours, not days — Apple Silicon Metal is non-load-bearing).

## Quantitative context

**Regime structure on hourly crypto bars.**
- Daily Bitcoin log-return std across 2023+2024 realdata: ~0.02-0.04
  (well-separated from the ±2% threshold the existing tagger uses).
- 7-day rolling return std: ~0.05-0.10 (clear regime separation
  when threshold-tagged).
- Empirically observed regime durations on BTC daily closes
  2020-2024: trending regimes 30-180 days; chop regimes 14-60 days;
  rare sharp transitions (mostly via 1-2 day price gaps).
- **Hourly cadence is materially different from daily.** The existing
  seed operates at daily cadence (7-day lookback on daily closes,
  ≤7 samples per call); hourly regime detection sees ~168 samples
  in a 7-day window — different SNR, different transition rate, and
  different empirical regime-duration prior. This is a load-bearing
  Q1/Q2/Q3 decision below.

**Why classification is easier than μ-prediction (the F4 escape
hatch).** The v2.5 chain showed `r_hat` was inside ε for most
samples (F4 trigger per ADR-0033 § D3). The information-theoretic
argument: predicting the sign of `r_{t+1}` requires extracting
~1 bit per hourly sample from a high-noise channel. Predicting
the regime state — "is the market currently trending up, trending
down, or chopping?" — requires extracting ~log₂(3) ≈ 1.58 bits over
a multi-day window (hundreds-to-thousands of samples integrated).
The integration window is the SNR amplifier; the classification
target is the lower-bandwidth ask. **This is why textbook regime
classifiers (HMM-on-returns) routinely identify regime structure
on financial time series where μ-prediction fails — and it's the
load-bearing prior for H1 (≥70% accuracy) below.**

**Why this isn't free.** The K-reg-3 risk from the survey applies
verbatim: the v1 cross-sectional momentum baseline already implicitly
captures "trending" via its 20-bar lookback. A regime overlay that
turns OFF momentum in non-trending regimes might just reduce exposure
to the period when momentum was going to be flat anyway — i.e. no net
Sharpe lift, just less turnover. The H2 question (does
regime-conditional sizing extract Sharpe-delta) is genuinely open
and is **the load-bearing empirical question this feature would
answer**.

## Disposition of the existing `crates/reflection/src/regime.rs`

This is the **highest-leverage decision in the brief** because every
downstream consumer in `crates/reflection`, `crates/reports`, and the
Phase F UI screens already imports `RegimeTag` and `classify_regime`
by name. Three options surfaced (analyst-recommended default in
**bold**):

- **Q7 (a) — Extend in-place (DEFAULT).** Keep `RegimeTag` enum +
  `REGIME_THRESHOLD_RATIO` const + `classify_regime` function exactly
  as today. Add new public API alongside: a new
  `RegimeClassifier` trait, new variants of `RegimeTag` only if Q1
  resolution forces it (see Q1 below), new functions
  `classify_hourly_regime`, `predict_regime_horizon`, etc.
  **Pro:** zero downstream breakage; 7+ test files keep passing
  byte-identical; lesson-card embeddings stay stable; Phase F
  renderer untouched. **Con:** module grows; the daily-only
  contract becomes one of many contracts in the same file.
- Q7 (b) — Sibling file `crates/reflection/src/regime_classifier.rs`.
  New trait + new types in a sibling module; existing `regime.rs`
  unchanged. **Pro:** even cleaner separation; the daily 3-state
  tagger is named "daily 3-state tagger" forever. **Con:** two
  files where one would do; future engineer has to remember both.
- Q7 (c) — New crate `crates/regime/`. **Pro:** strongest module
  hygiene. **Con:** new dependency edge into `crates/reflection`
  (and possibly `crates/strategy`, `crates/forecast`, `crates/audit`);
  workspace `Cargo.toml` changes; CI metadata cascade. Heaviest
  option. Almost certainly not warranted at v0.1.0.

**Analyst-recommended Q7 = (a) extend in-place.** Default unless the
Q1 taxonomy decision forces enum-variant additions that would muddy
the daily 3-state contract (in which case Q7 = (b) sibling becomes
the fallback to preserve daily-tagger byte-identity for
lesson-card embedding determinism).

## Requirements (R)

- **R1 — Backward compatibility (load-bearing).** Every existing
  `RegimeTag` literal reference + `classify_regime` callsite in
  `crates/reflection`, `crates/reports`, and downstream tests
  MUST keep compiling byte-identical. The
  `crates/reflection/tests/regime_classifier.rs` T1802 boundary
  test MUST keep passing byte-identical. The 30 v2.5-chain anchor
  body-SHAs MUST stay byte-identical (verified by the existing
  K4 patchtst_overlay_neutrality test). This is the
  **non-negotiable invariant** of the feature.
- **R2 — Hourly-cadence classifier.** A new function (or trait) that
  classifies regime at hourly cadence on a (timestamp, close) series
  spanning the realdata 2023+2024 window. Output type compatible
  with `RegimeTag` per Q1 resolution.
- **R3 — Multi-symbol generalisation.** The existing
  `classify_regime` operates on BTC daily closes only. R3 generalises
  to any USDT-pair in the 10-symbol realdata universe, optionally
  with a cross-symbol regime aggregation (Q1.d sub-option).
- **R4 — Strategy consumer.** At least one new strategy builder that
  consumes the classifier output and modulates the v1
  cross-sectional momentum baseline. Shape per Q4 below.
- **R5 — Audit emission.** Each regime prediction lands a
  `JournalEntry { kind: "regime_tag", … }` row in the audit ledger;
  Trail UI surface gets a new column or modal for regime
  visualisation (additive; Phase F Trail surface is the natural
  consumer).
- **R6 — Backtest verification.** Two realdata backtests
  (BS-1-equivalent + BS-2-equivalent) report regime-conditional
  strategy vs regime-blind baseline; verdict per Q5 below.
- **R7 — Anchor surface.** New anchors land under a new version pin
  per Q6; existing 30 anchors stay byte-identical.
- **R8 — Reflection consumption.** Lesson-card recall (Phase F
  Memory screen) sees the new hourly regime tags emerging in lesson
  cards over time as the regime-conditional strategy runs. The
  lesson-card embedding shape stays stable (R1); the strategy-to-
  lesson-card emission path is the only behavioural change.

## Hypotheses (H)

- **H1 (classifier-accuracy).** A statistical or DL regime classifier
  achieves **≥70% accuracy** on a held-out regime labeling task,
  where "ground truth" is defined per Q5.4 below (analyst-recommended
  default: human-pinned labels on the 4 obvious 2023-2024 regime
  shifts × small operator-validated label set). Falsifiable; cheap
  to evaluate.
- **H2 (Sharpe-lift).** Regime-conditional position sizing on v1
  momentum extracts **≥+0.10 Sharpe-delta** vs the un-conditional v1
  baseline on BS-1 + BS-2 realdata scenarios. The survey's gate.
- **H3 (cost realism).** A 3-5 week ship is feasible per survey.
  Falsified if the analyst-architect-developer-tester sequence
  exceeds the 1.5× wall-clock tripwire (~7.5 weeks).
- **H4 (regime structure on hourly crypto).** Hourly crypto exhibits
  **3-4 statistically distinguishable regimes** (Baum-Welch likelihood
  monotone-increases in number of states up to 3-4 then plateaus;
  cross-pair correlation in regime tags ≥0.5 on liquid USDT pairs).
- **H5 (compounding-with-C1).** IF C1 (vol forecast) ships positively,
  THEN a regime-x-vol composition (regime-conditional vol-targeting
  thresholds) extracts **≥+0.05 additional Sharpe-delta** above C1
  alone. Speculative; load-bearing only if Q-SEQ promotes both.
- **H6 (paint-the-tape risk).** The classifier doesn't flicker — i.e.
  fewer than **10 regime switches per week** on liquid USDT pairs in
  the realdata window. Falsified if the classifier emits
  bouncing-around predictions (turnover blows up; the strategy can't
  capture the regime).

## Open questions (Q) — operator-decide

### Q1 — Regime taxonomy

How many regime states, and what do they represent?

- **Q1 (a) — Keep 3-state Bull/Bear/Chop (extend in-place).** Hourly
  cadence on the existing tagger; same ±2% threshold or a new
  hourly-tuned threshold; same lookback or longer. **Pro:** maximum
  compatibility with existing lesson cards + Phase F UI; one less
  enum-variant decision. **Con:** "Bull/Bear/Chop" doesn't capture
  "volatile" as a regime — the survey explicitly suggested
  {trending-up, trending-down, mean-reverting, volatile, calm}.
- **Q1 (b) — 4-state Bull/Bear/Volatile/Calm.** Adds a volatility-
  orthogonal axis. **Pro:** matches survey suggestion; closer to
  textbook regime taxonomies. **Con:** new enum variants
  `RegimeTag::Volatile` + `RegimeTag::Calm` would force the
  lesson-card embedding contract (R1) to either grow (the new tags
  appear in cards going forward, lesson-card retrieval is
  forward-compatible but old cards stay 3-state). Lesson-card
  embedding determinism is preserved IF the ordinal encoding
  uses additive variants (Bull=0, Bear=1, Chop=2, Volatile=3,
  Calm=4) rather than reordering. Architect-decide.
- **Q1 (c) — Continuous-valued regime score.** Output a regime
  vector in `[0, 1]^N` instead of a discrete tag. **Pro:** avoids
  the lesson-card-embedding compatibility problem entirely; richer
  signal for downstream consumption. **Con:** the lesson-card system
  fundamentally consumes `RegimeTag` (discrete enum); we'd need
  both a continuous regime + a derived discrete tag for the legacy
  surface. Twice the surface area.
- **Q1 (d) — HMM-derived hidden states.** Let Baum-Welch find K
  states without human labels; the strategy consumes the emitted
  state probabilities. **Pro:** most statistically honest; doesn't
  pre-impose taxonomy. **Con:** human-readable interpretation of
  emergent states is post-hoc; the strategy builder still has to
  map "state-0" → "buy momentum" / "state-2" → "flat" etc.

**Analyst-recommended default: Q1 = (a) keep 3-state + extend
in-place.** Reasoning: lesson-card embedding determinism (R1) is
load-bearing; the simplest extension that preserves it is most
likely to ship on the survey's 3-5 week budget. If H1 (accuracy)
fails on a 3-state hourly classifier, Q1=(b) or Q1=(d) become
the natural follow-on briefs. **No autoapprove — operator should
explicitly answer Q1 before architect M-T1 (which itself is
DEFERRED).**

### Q2 — Classifier architecture

How is the regime classifier itself built?

- **Q2 (a) — Statistical (HMM, kernel methods, no DL).** Baum-Welch
  on hourly log-returns + |log-returns|; HMM with 3-4 states; emission
  distributions Gaussian per state. Pure-Rust crates (`hmm` /
  `linfa-hmm` / hand-rolled). **Pro:** smallest surface; trains in
  minutes; well-understood failure modes; tractable to test.
  **Con:** HMM is a strong model assumption; emergent states may
  not map cleanly onto human-readable regime labels (K-reg-1 risk).
- **Q2 (b) — Small DL classifier (~100k params).** Logistic regression
  on engineered features OR a small MLP / 1-D conv classifier on the
  5-feature OHLCV window. Trained on hand-labeled or HMM-derived
  ground truth. **Pro:** familiar tooling (candle); composes with
  v2.5 forecast scaffold for free; small enough that v2.5 F4 evidence
  does not directly transfer. **Con:** more degrees of freedom; needs
  ground truth (introduces K-reg-1 dependency upstream).
- **Q2 (c) — Ensemble of (a) + (b).** Train both; pick the higher-
  accuracy one OR vote. **Pro:** maximum robustness. **Con:** doubles
  scope; survey 3-5 week budget unlikely to accommodate.
- **Q2 (d) — Rule-based extension of existing `regime.rs` (no
  learning).** Generalise the ±2% threshold over a parametric lookback
  to hourly cadence; tune the threshold + lookback on the realdata
  window via a grid sweep. **Pro:** cheapest; no training; pure-fn
  extension; minimal new surface; ZERO new dependencies. Lesson-card
  embedding stays trivially stable. **Con:** lowest ceiling — won't
  match an HMM if the regime structure is genuinely non-trivial.

**Analyst-recommended default: Q2 = (a) statistical HMM** unless
Q1 = (d) (in which case HMM is forced anyway). Reasoning: matches
the literature precedent (Hamilton 1989, regime-switching models
in finance are HMM-by-default), trains cheaply, fits the 3-5 week
budget, and the existing seed in `crates/reflection/src/regime.rs`
gives us the rule-based comparison baseline for free (Q2.d becomes
the "is the HMM actually better than the rule?" sanity check).
**Q2 = (d) is a credible fallback** if the budget tightens or H1
fails to justify HMM — the rule-based hourly classifier can ship
in ~1 week and is a clean H1 falsifier on its own. Architect M-T1
should evaluate (a) vs (d) head-to-head if Q1 = (a).

### Q3 — Lookback / horizon

Predict the current regime from past N bars OR predict the regime
over the next N bars OR both?

- **Q3 (a) — Predict current regime from past N bars (nowcasting).**
  Symmetrical to the existing daily tagger (7-day trailing return →
  Bull/Bear/Chop). Hourly equivalent: 168-bar (7-day) or 720-bar
  (30-day) trailing window → current regime. **Pro:** trivial
  evaluation (the "ground truth" at time t is computable from data
  up to t with no leakage); textbook HMM uses this shape.
- **Q3 (b) — Predict regime over next N bars (forecasting).** Given
  features up to t, predict the regime at t+N. **Pro:** more
  immediately useful for strategy positioning. **Con:** harder task;
  shares failure modes with v2.5 (predicting the future has bounded
  SNR); H1 prior should drop.
- **Q3 (c) — Both (nowcasting + forecasting).** **Pro:** most
  flexibility downstream. **Con:** doubles scope.

**Analyst-recommended default: Q3 = (a) nowcasting only at v0.1.0.**
Reasoning: the survey's H1 (regime-conditional sizing extracts +0.10
Sharpe-delta) does not require regime prediction in the future tense —
it requires accurate **labeling of the current state** so the
strategy can switch behaviors as the regime moves. Nowcasting is
the strictly-cheaper task. Q3 = (b) (forecast next-N regime) is a
natural v0.2.0 follow-on if H2 (Sharpe-lift) clears on nowcasting.
**No autoapprove — operator confirms.**

### Q4 — Strategy consumer shape

How does the strategy consume the regime tag?

- **Q4 (a) — Regime-conditional position sizing on existing
  strategies.** v1 momentum's per-symbol position weight is
  multiplied by a regime-dependent scalar (e.g. 1.0× in Bull/Bear,
  0.0× in Chop, OR 1.0× in Bull, -1.0× in Bear, 0.0× in Chop).
  **Pro:** clean overlay; v2.5 overlay pattern transfers verbatim;
  minimal architect surface.
- **Q4 (b) — Regime-switching strategy (different strategy active
  per regime).** Dispatcher: in Bull/Bear → run v1 momentum; in
  Chop → run a mean-reversion sibling (which doesn't exist yet —
  v1.5 mean-reversion was queued but never shipped). **Pro:** matches
  the survey's regime-conditional-dispatch framing. **Con:**
  prerequisite work (mean-reversion sibling) compounds scope;
  K-reg-2 (two-stage pipeline error compounding) applies.
- **Q4 (c) — Regime-as-feature feeding into another strategy.**
  The regime tag becomes a column in the feature window; downstream
  strategies (v1 momentum, future strategies) consume it as a
  conditioning input. **Pro:** most flexible; cleanest seam with the
  C1 vol-targeting overlay + C5 LLM overlay if they ship. **Con:**
  no immediate strategy ships; the feature is "just" a producer.
- **Q4 (d) — All three as opt-in builders.** Compose at architect-
  lock time.

**Analyst-recommended default: Q4 = (a) regime-conditional position
sizing on v1 momentum AT v0.1.0**, with explicit deferral of Q4 (b)
to v0.2.0 if v1.5 mean-reversion is built first. Q4 (c) is the
**right long-term shape** (composes with C1 and C5) but adds
architect surface that wouldn't ship in 3-5 weeks alongside Q4 (a)
— defer to v0.3.0 if H2 clears. **No autoapprove.**

### Q5 — Verdict shape

What's the success bar at backtest time?

- **Q5.1 — Classifier accuracy (H1).** ≥70% accuracy against the
  Q5.4 ground-truth label set. **Falsifies H1 if <50%** (worse than
  random for 3-state).
- **Q5.2 — Sharpe-delta vs regime-blind baseline (H2).** Same
  +0.10 Sharpe-delta bar as the v2.5 chain (survey's canonical
  alpha-unlock threshold). **Falsifies H2 if <+0.05.** Adopts the
  ADR-0033-equivalent verdict tree:
  - **V-PASS** — Sharpe-delta ≥ +0.10 AND drawdown not worse AND
    turnover bounded.
  - **V-MARGINAL** — Sharpe-delta in [+0.05, +0.10).
  - **V-FAIL** — Sharpe-delta < +0.05.
- **Q5.3 — Regime stability (H6).** Regime-switch rate ≤10/week on
  liquid USDT pairs. **Falsifies H6 if flicker rate >20/week.**
- **Q5.4 — Ground truth definition (operator-decide).**
  - **(a) Human-pinned labels** on the 4 obvious 2023-2024 regime
    shifts. Cheapest; operator-aligned. Analyst-recommended.
  - **(b) Forward-return-based** labels (regime at t = sign of
    return over t+1..t+N). Mechanical; no human-in-the-loop;
    introduces a look-ahead-bias-by-construction in the ground
    truth, which is fine for classifier evaluation but means the
    classifier accuracy is a self-consistency metric not a
    "predictive power" metric.
  - **(c) HMM-derived labels** (Q1 = (d) automatically forces this).

**Analyst proposes a new ADR (NOT ADR-0033 extension).** ADR-0033 is
the immutable F-verdict tree for μ-prediction forecasters (the
"are forecasts inside ε" question); regime classification is a
different task and the verdict tree needs different leaves.
Architect-decide whether the new ADR is a sibling to ADR-0033 (e.g.
`ADR-0037 regime-classification-verdict-shape`) or whether ADR-0033
gets a § E (regime-classification verdict tree) addendum — the
analyst's mild preference is **sibling ADR** to preserve ADR-0033
§ D3 immutability.

### Q6 — Anchor strategy

Where do new anchors land?

- **Q6 (a) — Under `v3.0.0-regime` version pin.** New
  `regime-classifier-bs1-realdata` + `regime-classifier-bs2-realdata`
  anchors (forecast-distribution-equivalent for regime tag
  predictions over the realdata window). Sibling
  `regime-overlay-momentum-bs{1,2}-realdata` Sharpe-comparison
  anchors. **Pro:** stays additive; clean version naming.
  **Con:** `v3.0.0` jumps a major version — the survey suggests
  `v2.7.0-regime` for sibling-style; `v3.0.0` is the survey's
  Candidate 7 (ensemble) reserved bump. **Analyst recommends
  `v2.7.0-regime` to leave `v3.0.0` for the survey's Candidate 7.**

**Analyst-recommended default: Q6 = `v2.7.0-regime` version pin.**
Architect can override to `v3.0.0-regime` if survey Candidate 7 is
explicitly retired or merged into this feature's scope.

### Q7 — Existing `crates/reflection/src/regime.rs` disposition

Covered above in `## Disposition of the existing
crates/reflection/src/regime.rs` section. **Default: Q7 = (a) extend
in-place**, with Q7 = (b) sibling-file fallback if Q1 forces
enum-variant additions that would muddy the daily-tagger contract.

## Known risks (K)

- **K-reg-1 — Regime taxonomy is subjective.** Inherited from the
  survey. A 4-state HMM may not map onto human "trending / MR /
  volatile" labels. Strategy must consume emitted state probabilities
  directly, not human labels. **Mitigation:** Q1 default = (a)
  keep-3-state preserves human readability; if H1 forces (d) HMM-
  derived, the strategy contract becomes opaque-states.
- **K-reg-2 — Two-stage pipeline error compounding.** Inherited from
  the survey. Classifier error multiplied by strategy mis-allocation
  error. **Mitigation:** Q4 default = (a) is single-stage (regime →
  position size scalar), not two-stage (regime → strategy-of-strategies).
- **K-reg-3 — v1 baseline already implicitly captures trending.**
  Inherited from the survey. **Mitigation:** the analyst-pass
  verdict needs to compute Sharpe-delta on BS-1 + BS-2 with
  per-regime decomposition (which periods is regime-overlay
  outperforming?); if the lift comes only from drawdown reduction in
  Chop regimes, that's a real but modest win.
- **K-reg-4 (new) — Lesson-card embedding determinism.** R1 invariant.
  ANY change to `RegimeTag` ordering OR `Display` output OR the
  underlying byte encoding in `crates/reflection/src/embedding.rs`
  breaks lesson-card retrieval determinism + the 30-anchor body-SHA
  invariant transitively (through memory-highlights renderer fixtures
  in `crates/reports/tests/`). **Mitigation:** Q1 default = (a) +
  Q7 default = (a) jointly guarantee zero contract change.
  Architect M-T1 explicitly verifies no `embedding.rs` byte drift.
- **K-reg-5 (new) — Apple Silicon Metal compute determinism.**
  ADR-0035 § D1 (post-training σ_train recalibration cross-phase)
  applies if Q2 = (b) DL classifier. HMM (Q2 = (a)) trains on CPU
  only — Metal determinism non-load-bearing. **Mitigation:** Q2
  default = (a) sidesteps.
- **K-reg-6 (new) — Backtest determinism with regime classifier in
  the loop.** The regime classifier emits a tag at every hourly bar
  during backtest; the audit row format + the replay-cache
  determinism guarantees must extend to the new tag-emission path.
  **Mitigation:** the existing audit ledger + replay-cache already
  carry forecast-tag rows (v2.5 TCN + v2.5a PatchTST); the regime-tag
  row is symmetrical; should be additive.

## Non-regression contract (locked at this analyst pass)

1. `crates/reflection/src/regime.rs` `RegimeTag` enum bytes stay
   identical (3 variants, exact order Bull / Bear / Chop). New
   variants — if Q1 forces them — append at the end of the enum;
   never insert mid-enum.
2. `Display` for `RegimeTag` keeps emitting `bull|bear|chop` lowercase
   (body-byte stable; lesson-card serialization depends on this).
   New variants land their own lowercase Display strings (e.g.
   `volatile|calm`).
3. `REGIME_THRESHOLD_RATIO = dec!(0.02)` const stays at exactly
   ±2% (existing daily tagger contract). New hourly classifier uses
   a separate const; never re-uses or mutates this one.
4. `classify_regime(btc_closes, at)` function signature + behavior
   stays byte-identical (T1802 test family must keep passing).
5. `crates/reflection/src/embedding.rs` byte-output for legacy
   3-state RegimeTag stays identical. Embedding for new variants
   (if any) extends additively.
6. The 30 v2.5-chain anchor body-SHAs in `spec/anchors.toml` stay
   byte-identical (verified by existing `patchtst_overlay_neutrality`
   K4 test).
7. All 7+ existing test files importing `RegimeTag` keep passing
   byte-identical (none rebuild as part of this feature's verification).
8. Phase F Memory + Models UI renderer (`crates/reports/...` + UI
   surface) sees no fixture rebuild from this feature; new lesson
   cards emerging from the new strategy ARE new content over time
   (R8) but no fixture regeneration is required at ship.

## Deferred milestones (per Q-SEQ HYBRID)

Per operator-decide 2026-05-22:
> "C1 builds first; this analyst pass produces a spec-only design
> exploration. NO code commitment until C1 ships its verdict OR
> operator promotes this feature."

The following milestones are **DEFERRED — no work past M-A4 (this
analyst pass) until activation gate fires**:

- **M-T1 — Architect lock.** Decomp into Waves A-X; ADRs (new
  regime-verdict ADR + possibly classifier-training ADR if Q2 = (b));
  parallelism map. **Activation gate:** C1 ships AND (operator
  routing = promote-C2 OR Sharpe-delta on C1 ≥ +0.10 triggers
  auto-progression on remaining survey picks).
- **M-D1..M-Dn — Developer waves.** Crate extensions; classifier
  training; strategy builder; backtest scenarios. **Activation gate:**
  M-T1 ships.
- **M-F — Tester gate.** Standard test-report.md per the contract.
- **M-FINAL — Anchor lock + presenter.** Standard operator approval.

**Activation contract.** When the activation gate fires, the
orchestrator re-spawns analyst for an M-A5 (light-touch refresh —
re-read survey, re-read this brief, confirm no drift in
`crates/reflection/src/regime.rs` since 2026-05-22, surface any new
Q's from intervening ships) before architect M-T1 spawns. Q-PROCESS
note: this analyst pass runs **in parallel** with the C5 LLM-as-
forecaster analyst pass; C1 ships as a sibling **without** waiting
for either C2 or C5 analyst-pass completion.

## Cost estimate (carried from survey)

- Analyst pass (this): **completed 2026-05-22** (~1 day vs 1-2 weeks
  survey estimate — light because the seed
  `crates/reflection/src/regime.rs` already exists; not greenfield).
  Cost: ~1 analyst day.
- Architect lock (DEFERRED): 3-5 days when activation gate fires.
- Dev impl (DEFERRED): 2-3 weeks. HMM training (minutes), classifier
  evaluation (hours), strategy builder (3-5 days), backtest
  integration + 2 realdata scenarios (3-5 days).
- Tester (DEFERRED): 1 week.
- Compute (DEFERRED): tiny. HMM Baum-Welch in minutes; small
  classifier in hours.
- **Total wall-clock from activation: ~4-6 weeks** (matches survey
  estimate verbatim).

**Variance budget.** Per the survey's Q-BUDGET resolution, this
candidate's hard tripwire is 1.5× the brief-time estimate ≈ ~9
weeks; cumulative budget across {C1 + C2 + C5} cap is ~16 weeks
operator-suggested.

## Sequencing dependencies

- **Prerequisite: C1 (volatility) ships its verdict.** Per Q-SEQ
  hybrid. C1 verdict determines:
  - If C1 PASS (Sharpe-delta ≥ +0.10): the +0.10 bar is achievable on
    this data; C2's H2 prior (+0.10 from regime-conditional sizing)
    holds at MEDIUM. H5 (regime × vol composition) becomes
    activatable.
  - If C1 V-MARGINAL: H2 prior weakens to MEDIUM-LOW; operator may
    re-evaluate whether to fund C2 at all (Q-PICK revisit).
  - If C1 V-FAIL: H2 prior drops to LOW; operator likely defers C2
    until a more orthogonal candidate (C5) provides separate
    evidence.
- **Sibling: C5 (LLM-as-forecaster) analyst pass runs in parallel.**
  Q-PROCESS = 3 analysts in parallel. No dependency between C2 and C5
  analyst passes; both queue equally for operator routing
  post-C1-ship.

## Handoff envelope

See bottom of `tasks.md` for the standard TOML envelope. This brief
hands off to **operator-decide AFTER C1 ships**, not to architect or
developer at this pass.
