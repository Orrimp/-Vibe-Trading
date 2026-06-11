---
title: Strategy reformulation survey — post v2.5 DL retirement
date: 2026-05-22
authors: [analyst]
status: survey
tags: [survey, strategy, post-v25, next-direction, operator-decide, volatility, regime, horizon, features, llm, hmm]
related:
  - spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md
  - spec/v25-tcn-horizon-bump-or-retire/feature.md
  - spec/v25a-patchtst-overlay/feature.md
  - spec/product.md
  - spec/backlog.md
  - spec/architecture/12-forecast-overlay.md
---

# Strategy reformulation survey — post v2.5 DL retirement

> **This is a SURVEY, not a feature pass.** No new `[[req]]` rows, no
> Queue→Active promotion, no feature folder, no code. The analyst
> tabulates seven candidate research directions with cost / EV /
> reuse / hypothesis framing so the operator can decide which lane(s)
> get the freed multi-week budget (~3-5 weeks of compute and
> analyst/architect/dev/tester bandwidth previously earmarked for
> v2.5b vanilla decoder Transformer + v2.6 forecast bake-off; both
> deprecated 2026-05-22).
>
> Companion to
> [`v25-dl-journey-retrospective-2026-05-22.md`](v25-dl-journey-retrospective-2026-05-22.md) —
> the retrospective is the "what NOT to chase"; this survey is the
> "what COULD next chase."

## Why now

**Budget freed.** Operator routing (a) at the v25a-patchtst-overlay
v0.1.0 presenter on 2026-05-22 retired the entire 4-phase DL forecast
overlay umbrella (v2.5 TCN + v2.5a PatchTST shipped F4-F4-F4; v2.5b
vanilla Transformer + v2.6 bake-off deprecated without shipping). The
~3-5 weeks of compute + human bandwidth that had been earmarked is now
unallocated.

**Evidence base.** Joint F4-F4-F4 across 3 model checkpoints / 2 model
families / 2 horizons established that v2.5-era DL approaches —
predicting **μ** (1h or 24h log-return) over the 5-feature OHLCV
window — do not clear the +0.10 Sharpe-delta unlock threshold on the
v1 cross-sectional momentum baseline. The retrospective's
"what NOT to chase" enumerates the falsified hypothesis space; this
survey's "what COULD next chase" framing samples the orthogonal
research landscape.

**Infrastructure leverage.** Across the v2.5 journey we built a
substantial scaffold that is **task-agnostic** — `crates/forecast` +
`crates/replay-cache` + `crates/audit` + `crates/strategy` overlay
composition + `crates/reflection` + `crates/llm` + the realdata
Binance pipeline. None of these are wedded to the μ-prediction task;
the same plumbing can carry vol forecasts, regime labels, longer
horizons, richer features, or LLM-emitted signals. The marginal cost
of any new direction below is "design + train + integrate"; not
"build from scratch."

**Decision shape.** The operator must pick (i) which candidate(s)
to fund, (ii) in which order, and (iii) with which budget cap.
Picking nothing — i.e. pausing strategy work and shifting bandwidth
elsewhere (UI, ops, infra, paper-trade-live) — is also a valid
operator-decide; the survey doesn't force a strategy spend.

## Candidate directions (7-row table)

> Cost units below: **analyst** = analyst pass time; **architect** =
> architecture lock; **dev** = developer impl; **tester** = test +
> verification; **compute** = wall-clock for training / backtests if
> applicable. Wall-clock estimates are **brief-time** ranges, not
> commitments; each candidate carries its own variance budget at
> feature-pass time.

### Candidate 1 — Volatility forecasting (predict σ, not μ)

**Scope summary.** Replace the next-bar log-return target with a
realized-volatility target. Train a small model (GARCH baseline →
small DL refinement) to emit σ̂_{t+h} instead of r̂_{t+h}. Downstream
consumer is **not** an overlay on momentum direction; it's either
(a) a **vol-targeting overlay** that scales position size inversely
to forecasted σ, or (b) a **kill-switch trigger** that flat-lines
exposure when σ̂ crosses a regime-shift threshold.

**Reuse of existing infrastructure.**
- `crates/forecast/src/features.rs` 5-feature window → reuses verbatim;
  target derivation flips from `(close_{t+1}/close_t).ln()` to e.g.
  Parkinson estimator `σ̂ = sqrt((1/4*ln2) * ln(high/low)^2)` or
  realized-vol over a rolling 1h-of-1min bars (not currently bootstrapped
  — see Q-DATA below) or simply the rolling std of past log-returns.
- `crates/forecast` `ForecastProvider` trait → reuses; emit a new
  `VolForecast { sigma_hat, confidence }` enum variant alongside
  `Direction`. Architect lock.
- `crates/strategy` overlay composition → **needs new shape**.
  Vol-targeting is **risk-level**, not signal-level — different from
  the v2.5 overlay pattern. ADR-amendment territory; architect-decide
  whether vol forecasts compose at risk or signal layer.
- `crates/audit` + `crates/replay-cache` → reuses verbatim.
- `data/binance/` realdata → reuses 10 USDT hourly OHLCV; **no extra
  data sourcing** for Parkinson / GARCH baselines (range-based vol
  computable from existing high/low). Realized-vol from intraday
  1-minute bars **would** require new data sourcing.
- `crates/forecast/checkpoints/anchors/` TCN + PatchTST checkpoints →
  inference-time forward pass can be re-used as a **feature**
  (model emits forecast, feed into a vol model as a feature) — but
  speculative; not central to this candidate.

**Cost estimate.**
- Analyst pass: 1-2 weeks (literature on GARCH(1,1) + range-based
  estimators + DL vol forecasting; data-availability decisions).
- Architect lock: 3-5 days (overlay-pattern decision: risk vs signal
  layer; ADR amendment for vol-overlay shape).
- Dev impl: 2-3 weeks (GARCH baseline in pure Rust — likely via
  `rust-quant` or hand-rolled; small DL refinement if GARCH baseline
  shows promise).
- Tester: 1 week.
- Compute: small. GARCH(1,1) fits in seconds; even a small DL vol
  forecaster trains in hours, not days.
- **Total wall-clock: ~4-6 weeks.** Compute budget tiny vs v2.5.

**Risk + reversibility.**
- K-vol-1: vol forecasting is **easier than direction forecasting**
  (well-known result in quant literature); a baseline GARCH(1,1)
  will likely beat naïve constant-σ. But the question isn't "can
  we forecast σ" — it's "does a vol-targeting overlay extract +0.10
  Sharpe-delta vs the v1 cross-sectional momentum baseline." Those
  are different bars. Crypto vol clustering is strong; momentum-on-
  10-pairs may already implicitly vol-balance via position weighting.
- K-vol-2: ADR amendment for risk-layer overlay is non-trivial and
  affects all future strategies, not just this one. Reversible but
  consequential.
- Reversibility: HIGH. ADR amendment is the only sticky decision;
  the rest is additive (new forecaster, new strategy builder, new
  anchors under a new version pin).

**Hypothesis.** A vol-targeting overlay applied to v1 cross-sectional
momentum (scale per-symbol position by `1/σ̂` clipped to e.g.
[0.5×, 2×] vs baseline weights) extracts ≥+0.10 Sharpe-delta vs the
v1 baseline on BS-1 / BS-2 realdata scenarios. Sharpe rises because
high-vol regimes get smaller positions; drawdown bands narrow.

**Success vs F4 evidence.** Sharpe-delta ≥ +0.10 on BS-1 realdata
backtest with vol overlay; turnover / fee impact bounded; drawdown
strictly improves (since we're explicitly downsizing in high-vol
regimes). F4-equivalent failure: vol-targeting Sharpe-delta < +0.05
AND/OR drawdown gets worse (the vol forecast leaks future info or
fee-drags too hard).

**Prior probability of clearing +0.10 Sharpe-delta: MEDIUM-HIGH.**
- Vol clustering on crypto hourly bars is empirically strong
  (autocorr of |r| > 0.3 at lag 1-24); a GARCH baseline will
  predict σ much better than constant.
- Vol-targeting overlays are **textbook practice** in
  cross-sectional momentum (Asness/Frazzini/Pedersen "Quality minus
  Junk", "Betting Against Beta" volatility-scaling) — the Sharpe
  improvement from vol-targeting on the underlying baseline is
  often 0.1-0.3 in equity factor research.
- Hourly crypto is **not** equities; transaction costs may eat the
  benefit if turnover from vol-rebalancing is daily-or-more frequent.
  This is a load-bearing empirical question.

**Why this differs from v2.5 F4.** v2.5 was "can DL predict direction
on hourly crypto OHLCV." Vol-targeting answers "can we use the
vol-clustering signal that empirically exists." Different hypothesis,
different task framing, different load-bearing evidence.

---

### Candidate 2 — Regime classification (HMM, kernel, or small classifier)

**Scope summary.** Predict a regime label (e.g.
{trending-up, trending-down, mean-reverting, volatile, calm}) over the
realdata window. Strategy responds with **regime-conditional position
sizing** — momentum in trending regimes, mean-reversion in MR regimes,
flatten in volatile regimes. Conceptually orthogonal to v2.5's
direction-prediction failure: regime classification doesn't require
predicting next-bar μ; it requires labeling current state.

**Reuse of existing infrastructure.**
- `crates/reflection/src/regime.rs` already has a **3-state BTC
  daily-close regime tagger** (`Bull | Bear | Chop` over a ±2%
  threshold; pure-fn, no I/O). **This is a load-bearing seed.**
  Extending to hourly cadence + multi-symbol + finer-grained tags
  is additive.
- `crates/forecast/src/features.rs` 5-feature window → reusable
  as classifier input. Target derivation flips from log-return to a
  regime label.
- HMM / kernel approaches: pure-Rust crates exist
  (`hmm` / `linfa-hmm` / hand-rolled). No new ML framework.
- `data/binance/` realdata → reuses verbatim.
- `crates/strategy` → needs a new **regime-conditional dispatch**
  strategy builder (mux between {momentum, mean-reversion, flat}
  conditional on regime tag). Architect-lock the shape.
- `crates/audit` → reuses; emit `JournalEntry { kind: "regime_tag", … }`
  rows alongside forecast rows.

**Cost estimate.**
- Analyst pass: 1-2 weeks (HMM baseline vs kernel methods vs small
  classifier; regime taxonomy decision — 3 / 4 / 5 tags?
  hourly vs daily?).
- Architect lock: 3-5 days (regime-conditional strategy dispatch
  pattern; ADR for the dispatch shape since it's a new strategy
  pattern; backtest scenario shape).
- Dev impl: 2-3 weeks (HMM training; classifier; regime-conditional
  strategy builder).
- Tester: 1 week.
- Compute: tiny. HMM Baum-Welch fits in minutes; small classifier
  trains in hours.
- **Total wall-clock: ~4-6 weeks.** Compute budget small.

**Risk + reversibility.**
- K-reg-1: regime classification is well-studied but **regime
  taxonomies are subjective**. A 5-state HMM that the model finds
  via Baum-Welch may not map cleanly onto "trending / MR / volatile"
  human labels. The strategy needs to consume the **emitted state
  probabilities directly**, not human-readable labels.
- K-reg-2: regime-classification + regime-conditional strategy is a
  **two-stage pipeline** with compounding error; each stage adds noise.
- K-reg-3: the v1 cross-sectional momentum baseline already
  implicitly captures "trending" via its 20-bar lookback. A regime
  overlay that turns OFF momentum in non-trending regimes might just
  reduce exposure to the period when momentum was going to be flat
  anyway — i.e. no net Sharpe lift, just less turnover. Hard to
  beat the baseline if it already implicitly conditions on regime.
- Reversibility: HIGH. Additive strategy builder; additive
  classifier checkpoint; new anchor version pin.

**Hypothesis.** A regime-conditional strategy that {runs v1 momentum
in trending regimes, flatlines in volatile, runs mean-reversion in
chop} extracts ≥+0.10 Sharpe-delta vs v1 baseline on BS-1 / BS-2.

**Success vs F4 evidence.** Sharpe-delta ≥ +0.10 on realdata
backtest; regime tag distribution roughly matches operator's prior
on "% of time market is trending vs MR vs volatile"; turnover impact
bounded. F4-equivalent failure: Sharpe-delta < +0.05; regime tags
flicker too fast (e.g. >10 regime switches per week) implying the
classifier is over-fitting to noise.

**Prior probability of clearing +0.10 Sharpe-delta: MEDIUM.**
- Conceptually orthogonal to v2.5; uses signal-existence (regime
  clustering) different from "direction predictability."
- Crypto markets DO have clear regime shifts (Mar 2020 crash,
  Jan-Nov 2021 bull, May-Jul 2022 deleveraging, Q4 2023 ETF rally).
  An HMM with realdata 2-year window should find them.
- But regime-conditional dispatch is **complex** to get right — many
  papers show small or no Sharpe lift after fees/turnover, because
  the strategy is now switching between two strategies and paying
  twice the bid-ask.

**Why this differs from v2.5 F4.** v2.5 asked "predict the future."
Regime classification asks "label the present" — fundamentally
easier task with established evidence (HMM works on financial time
series). The Sharpe-lift question is downstream of classification
quality.

---

### Candidate 3 — Longer-horizon trend signal (168h / 1 week)

**Scope summary.** Re-test the v2.5 PatchTST scaffold (or v2.5 TCN)
with `target_horizon_bars = 168` (weekly log-return target on hourly
bars). Operator surfaced "168h trend" in the retrospective's
"what COULD next chase" section. Same architecture-paradigm test
but at a horizon where momentum signals are historically more robust.

**Reuse of existing infrastructure.**
- `crates/forecast/src/features.rs` already supports
  `target_horizon_bars` parameter (the v25-tcn-horizon-bump scaffold
  generalized it). **Zero scaffold change.**
- `crates/forecast/checkpoints/anchors/` → both TCN and PatchTST
  weights are available as inference-only baselines (frozen at 1h /
  24h horizons); cannot reuse weights for 168h target, but can
  reuse the training scaffold + topology.
- PatchTST scaffold (`crates/forecast/src/patchtst.rs` + train_patchtst)
  → reuses verbatim; only `target_horizon_bars` flag changes.
- TCN scaffold → reuses verbatim with the same flag.
- All `_realdata` scenarios + audit + replay-cache → reuses verbatim.

**Cost estimate.**
- Analyst pass: 3-5 days (small; the scaffold is already proven, the
  hypothesis is narrow).
- Architect lock: 1-2 days (no new ADR; just a horizon-bump scoping
  decision plus a sample-count / overlap-fraction sub-decision —
  168h non-overlapping over 2 years of hourly data is ~104 samples
  per symbol = **catastrophically small**; 168h **overlapping** is
  ~17k samples per symbol but with autocorrelation ~0.99 between
  adjacent targets).
- Dev impl: 1-2 days (only the target-horizon CLI flag + retrain).
- Compute: 5-10 days wall-clock per checkpoint (PatchTST at 168h
  has same epoch count, similar sample count; TCN at 168h same).
  **Sequential, Apple Silicon Metal bottleneck** — same as v2.5
  multi-week retrain.
- Tester: 3-5 days.
- **Total wall-clock: ~2-3 weeks.** Compute moderate.

**Risk + reversibility.**
- K-horiz-1: **strong prior this also F4s**. The v25a-patchtst-overlay
  brief explicitly contemplated 168h horizon under Q4 and the
  analyst's read was "log-return std scales as ~3-4× per √24
  horizon increase; 168h std would be ~0.06-0.10, which is a much
  better SNR but the **effective sample count drops dramatically**
  unless we use heavily-autocorrelated overlapping targets, which
  may not give the model genuinely independent signal."
- K-horiz-2: weekly trend signals on crypto are well-studied in
  *daily* literature; hourly-cadence prediction of weekly trends
  may not be the right granularity match — daily bars at weekly
  horizon would be more natural, but that's a different data shape
  (10 symbols × 730 days = 7,300 samples total — very small for DL).
- K-horiz-3: **paradigm-test exhausted**. Joint TCN-F4 + PatchTST-F4
  is strong evidence the *architecture-family axis* is not the bound;
  horizon-bump on the same architecture family is unlikely to flip
  the verdict. **This is the lowest-prior candidate in the survey.**
- Reversibility: HIGH. Additive checkpoint; new anchor version.

**Hypothesis.** PatchTST (or TCN) trained at 168h horizon extracts
≥+0.10 Sharpe-delta vs v1 momentum baseline. Equivalently:
F-verdict per immutable ADR-0033 § D3 is **not F4** on a
forecast-distribution report against the new checkpoint.

**Success vs F4 evidence.** Same F-verdict + Sharpe-delta gates as
v2.5a. F4-equivalent: same as v2.5a (most likely outcome).

**Prior probability of clearing +0.10 Sharpe-delta: LOW.**
- Direct extrapolation from joint F4-F4-F4: same model families on
  same data with longer horizon **and** smaller effective sample
  count is unlikely to clear the bar. The argument is "we've tested
  2 of 2 architectures at 2 of N horizons; the simplest hypothesis
  is the task framing (μ-prediction over OHLCV) is wrong, not the
  horizon."
- Information-theoretic argument: we already have evidence that
  TCN+PatchTST cannot extract direction information on this data
  at 1h or 24h. The amount of direction-information at 168h is
  bounded by the autocorrelation structure; if anything, it's
  smaller per-sample (longer-horizon returns are noisier).

**Why this differs from v2.5 F4.** Doesn't, much. This is the
v2.5-paradigm "one more horizon" test. Cheapest of the 7
candidates if operator wants closure on horizon-axis exhaustion;
arguably **lowest EV per dollar** in the survey.

---

### Candidate 4 — Crypto-specific features (funding rate, OI, perp basis)

**Scope summary.** Augment the 5-feature OHLCV window with crypto-
specific features the v2.5 setup missed: perp **funding rate**
(8-hourly settlement on Binance perps), **open interest** (OI), **perp
basis** (spot vs perp price difference), optionally **CVD / aggressor
ratio** from trade ticks. The hypothesis is: the v2.5 5-feature input
was structurally signal-poor; adding domain-specific features may
unlock alpha that no architecture could extract from OHLCV alone.

**Reuse of existing infrastructure.**
- `crates/forecast/src/features.rs` 5-feature window → **needs
  extension**. The `FEATURE_DIM` constant is load-bearing; growing
  it changes the model input shape, which means **retraining all
  v2.5 checkpoints** if we want apples-to-apples (currently retired,
  so this is moot — but new checkpoints will use the new feature shape).
- `crates/data/src/bin/fetch_binance_klines.rs` → **needs new
  bootstraper**. Funding rate + OI are separate REST endpoints
  (`/fapi/v1/fundingRate`, `/fapi/v1/openInterestHist`). Perp basis
  needs both spot and perp price series.
- TCN + PatchTST scaffolds → **adaptable** by changing input channel
  count; topology essentially unchanged.
- All other plumbing (overlay, audit, replay-cache, backtest, anchors)
  → reuses.

**Cost estimate.**
- Analyst pass: 1-2 weeks (feature engineering decisions; data
  sourcing decisions for each feature; signal-quality literature
  review).
- Data bootstrap: 1-2 weeks (analyst + dev). Funding rate is
  available; OI is available; perp basis requires aligning spot
  and perp series. **Dev-grade work, non-trivial but bounded.**
- Architect lock: 3-5 days.
- Dev impl: 1-2 weeks (extend feature module; rebuild any forecaster
  to consume new input shape).
- Compute: same as v2.5 if we retrain a DL model; minimal if we
  augment a non-DL approach.
- Tester: 1 week.
- **Total wall-clock: ~5-8 weeks** (data bootstrap is the long
  tail; ML retraining adds wall-clock on top).

**Risk + reversibility.**
- K-feat-1: **feature engineering is research-grade open-ended.**
  Funding rate is a known signal on crypto (positive funding ⇒
  crowded long positioning ⇒ contrarian short prior), but the
  signal-extraction model is non-trivial. OI is similar. We may
  spend weeks bootstrapping data only to find the marginal signal
  is too weak to clear +0.10 Sharpe-delta.
- K-feat-2: **out-of-scope reach risk**. Combining this with any
  of candidates 1, 2, 3, 6, 7 is tempting ("better features +
  better target"); orchestration says "one variable at a time"
  to keep evidence clean.
- K-feat-3: data quality of historical funding rate + OI varies
  by venue; Binance is reliable; cross-venue augmentation adds
  scope.
- Reversibility: MEDIUM. New data bootstrap is sticky (we'll keep
  the data even if the model fails); feature module growth is
  additive but the FEATURE_DIM-load-bearing constant means
  existing checkpoints can't consume new features. New version
  pin needed.

**Hypothesis.** Adding funding rate + OI + perp basis to the
feature window unlocks a directional signal that the model can
extract at 1h or 24h horizon, clearing +0.10 Sharpe-delta.

**Success vs F4 evidence.** Same Sharpe-delta gate; additional
diagnostic: feature-importance analysis (e.g. permutation importance
on a small XGBoost baseline) should show non-trivial weight on the
new features. F4-equivalent: Sharpe-delta < +0.05 AND/OR feature
importance shows the new features are not load-bearing.

**Prior probability of clearing +0.10 Sharpe-delta: MEDIUM.**
- Funding rate is a documented signal on crypto (multiple papers,
  practitioner write-ups); its independent Sharpe contribution
  is plausibly 0.1-0.4 standalone on perp basis trades.
- BUT: we're not trading perps in this product (v2 success metric
  is spot paper-trading per product.md). Funding rate as a signal
  on **spot** momentum is a weaker case — it's a sentiment proxy
  rather than a direct execution signal.
- OI is widely cited but its direct alpha contribution after
  controlling for momentum is modest.

**Why this differs from v2.5 F4.** Different signal-axis. v2.5
asked "can architecture extract alpha from 5-feature OHLCV";
this asks "is the 5-feature OHLCV simply insufficient input?"
Independent hypothesis.

---

### Candidate 5 — Reflection-memory-as-forecaster (v2 LLM signal)

**Scope summary.** Use the v2 LLM agent infrastructure
(`crates/llm` shipped 2026-05-13) + the reflection-memory system
(`crates/reflection` Phase F shipped 2026-05-21, surfaces Memory
+ Models screens) to **produce a forecast-equivalent signal via
LLM debate over context + lesson cards**. Concretely: at each
decision tick, the v2 LLM analyst pipeline reads (a) recent
realdata window features, (b) top-K relevant lesson cards from
the reflection store, and produces a typed
`{rating: STRONG_SELL..STRONG_BUY, confidence: 0..1, horizon: …}`
that an overlay strategy consumes. Conceptually adjacent to
`v2x-trading-state-bus` (Queue § Process) — the operator-decide
trigger for the v2 LLM state-bus pattern would unlock this.

**Reuse of existing infrastructure.**
- `crates/llm` → reuses entire stack (Anthropic provider,
  prompt caching, redaction, budgeted, retry, recording, replay).
  Provider trait + factory is shipped.
- `crates/reflection` → reuses; `lesson_cards` store is queryable
  via `crates/reflection/src/query.rs::list_recent_lesson_cards`
  per the Phase F ship. Embeddings live at
  `crates/reflection/src/embedding.rs`.
- `crates/audit` → reuses; LLM responses already audit-ledger-
  emitted with cost telemetry.
- `data/binance/` realdata → reuses as input context for the LLM
  analyst.
- `crates/strategy` → needs new `with_llm_overlay` builder. Composed
  at signal-level (same as v2.5 TCN overlay pattern).

**Cost estimate.**
- Analyst pass: 2-3 weeks (prompt design, lesson-card retrieval
  shape, LLM-debate orchestration, evaluation methodology;
  literature on LLM-for-quant; reading list at
  [`dev-notes/v25-dl-reading-list-2026-05-16.md`](v25-dl-reading-list-2026-05-16.md)
  is partially relevant but mostly DL-focused).
- Architect lock: 1-2 weeks (overlay shape; LLM cost-budget
  envelope; deterministic-replay contract for LLM calls; new ADR
  for LLM-as-forecaster).
- Dev impl: 2-3 weeks (LLM overlay strategy; replay-cache
  extension if needed for backtest-time LLM determinism;
  audit-row shape).
- Tester: 1-2 weeks (including a deterministic backtest gate that
  pins LLM responses via replay).
- Compute: zero CPU. **LLM cost** is the variable axis — at v1
  ceiling ($80/month per product.md cost ladder) a backtest with
  N=10,000 LLM-decision-ticks at $0.001 each = $10. **Bounded by
  the cost ladder.**
- **Total wall-clock: ~5-9 weeks.** LLM cost ~$10-100 per backtest
  iteration depending on volume + caching effectiveness.

**Risk + reversibility.**
- K-llm-1: **deterministic-replay contract is load-bearing.**
  Backtests must be reproducible; LLM calls must be replay-cached
  pinned to a model SHA + prompt SHA. The replay-cache exists
  but its use for LLM-decision-replay in backtest is unproven
  at scale.
- K-llm-2: **cost-blow-up risk** — naive prompting at every bar
  on 10 symbols × 8760 bars/year = 87k LLM calls/year. With
  prompt caching this collapses to ~10k unique prompts/year but
  still $100+/backtest run. Cost-bound at architect lock; possibly
  N-bar batching (LLM only fires every N=24h or on regime change).
- K-llm-3: **evidence base is thin.** TradingAgents-style projects
  show LLMs *can* produce intelligible trading reasoning; whether
  that translates to risk-adjusted alpha on a benchmark like the
  v1 cross-sectional momentum baseline is **largely unevaluated**
  in published literature. Novel territory; high variance EV.
- K-llm-4: **memory feedback loop is novel** — the lesson-card
  retrieval is unproven for forecast-tick consumption. Currently
  Phase F surfaces it via UI; using it as an LLM-input requires
  the embedding-retrieval path to be backtest-deterministic.
- Reversibility: HIGH. Additive strategy builder; if it doesn't
  work, retire the LLM-overlay and keep the v2 LLM infrastructure
  for the assistant slot.

**Hypothesis.** A v2 LLM analyst+debate pipeline reading 5-feature
context + top-K relevant lesson cards produces a directional rating
that, used as an overlay signal on v1 momentum, extracts ≥+0.10
Sharpe-delta vs v1 baseline on BS-1 / BS-2 realdata scenarios.

**Success vs F4 evidence.** Sharpe-delta ≥ +0.10; lesson-card
retrieval shows interpretable patterns (e.g. specific lessons fire
on specific symbols / regimes); LLM cost stays inside the per-
backtest budget cap. F4-equivalent: Sharpe-delta < +0.05; OR LLM
ratings show low correlation with future returns; OR
deterministic-replay fails to reproduce.

**Prior probability of clearing +0.10 Sharpe-delta: LOW-MEDIUM.**
- This is the **most novel** candidate in the survey. Few
  precedents; publication landscape is largely anecdotal /
  benchmark-task (FinBench, BloombergGPT) without strict
  out-of-sample evaluation against real trading baselines.
- The reflection-memory innovation is **genuinely differentiated**
  per product.md § Differentiator ("(2) + (4)"); building a
  signal-source on top of it is on-strategy.
- HIGH variance: could be 0 Sharpe-delta (LLM produces
  noise-equivalent ratings) or 0.2+ (lesson-card-aware analyst
  finds patterns humans + DL missed).

**Why this differs from v2.5 F4.** Different signal-source axis
entirely: not "DL on OHLCV"; rather "LLM reasoning over
context + persistent memory." Information-theoretically distinct
hypothesis — taps signal sources DL can't (e.g. cross-pair
correlations, regime-shift narratives, lesson-of-similar-past).

---

### Candidate 6 — Non-DL approaches (HMM, kernel methods, statistical filters)

**Scope summary.** Independent of regime classification (candidate 2):
**predict direction with non-DL methods** — XGBoost, gradient-boosted
trees, kernel ridge regression, Kalman filter, ARIMA-with-features,
GAM, ESN (echo-state networks). Targeting the same μ-prediction task
v2.5 attempted but with **lower-variance / less-overparameterized**
methods. Often perform surprisingly well on hourly crypto OHLCV
because the signal-to-noise floor favors low-capacity models that
don't overfit.

**Reuse of existing infrastructure.**
- `crates/forecast/src/features.rs` 5-feature window → reuses
  verbatim (or extends, see candidate 4 combinability).
- `crates/forecast` `ForecastProvider` trait → reuses; new
  `XgboostForecaster` / `KalmanForecaster` impl.
- `crates/strategy` overlay → reuses (same signal-level overlay
  pattern as v2.5).
- `crates/audit` + `crates/replay-cache` → reuses.
- Pure-Rust ML crates: `linfa`, `smartcore`, `rust-quant`, a hand-
  rolled XGBoost (or a thin FFI wrapper if pure-Rust XGBoost is
  insufficient — adds Python sidecar risk per CLAUDE.md).

**Cost estimate.**
- Analyst pass: 1-2 weeks (which method? XGBoost is the
  industry default for tabular forecasting; Kalman filter is
  classical; kernel ridge is a quick baseline).
- Architect lock: 3-5 days.
- Dev impl: 2-3 weeks (XGBoost via `smartcore` or similar;
  feature engineering; train + serialize).
- Compute: tiny. XGBoost trains in minutes; Kalman filter in
  seconds.
- Tester: 1 week.
- **Total wall-clock: ~4-6 weeks.** Compute negligible.

**Risk + reversibility.**
- K-non-1: **prior literature evidence on non-DL beating DL is
  mixed**. Some quant literature finds gradient-boosted trees
  outperform DL on cross-sectional features; some shows the opposite
  on time-series. Hourly crypto is genuinely under-studied —
  could go either way.
- K-non-2: **same task framing as v2.5** — predicting μ over OHLCV
  features. The joint F4-F4-F4 evidence doesn't directly apply
  (different model class), but it does argue weakly that the
  signal-in-data is small. A non-DL approach may still F4-equivalent
  because the underlying signal is just weak, regardless of model
  capacity.
- K-non-3: pure-Rust XGBoost ecosystem is less mature than
  Python's; alternative is `lightgbm`-FFI or hand-rolled. Adds
  ecosystem risk.
- Reversibility: HIGH. Additive forecaster; new anchor version.

**Hypothesis.** A non-DL forecaster (XGBoost preferred, fallback to
Kalman / kernel ridge) trained on the 5-feature OHLCV window with
next-1h-or-24h log-return target extracts ≥+0.10 Sharpe-delta vs v1
baseline on BS-1 / BS-2 realdata.

**Success vs F4 evidence.** Same F-verdict + Sharpe-delta gates as
v2.5. F4-equivalent failure mode: same as v2.5 (signal genuinely
absent; lower-capacity model can't surface what higher-capacity
DL couldn't).

**Prior probability of clearing +0.10 Sharpe-delta: LOW-MEDIUM.**
- Same task framing as v2.5; same data; same horizon options. The
  joint F4-F4-F4 evidence is **partially predictive** that
  non-DL also F4s here (the task-shape may be the wrong shape,
  not the model-class).
- BUT: there is a non-trivial prior that **classical methods
  underfit-by-design** in a way that suits low-SNR data — i.e.
  they don't overfit to noise the way deep models can. The
  alpha-investigation found `r_hat` was inside ε for most
  samples (F4 trigger); a Kalman filter forced to emit small,
  bounded predictions might be more honest about the signal
  ceiling than a high-capacity TCN.
- Cheap to falsify; the cost asymmetry is favorable even given
  the low-medium prior.

**Why this differs from v2.5 F4.** Different model-class axis.
Cheap-to-falsify orthogonal test — if it F4s, we've explored a
3rd model class on this task and have stronger evidence the task
framing is wrong. If it doesn't F4, we have a non-DL forecaster
that beat DL on this specific task — a substantive finding.

---

### Candidate 7 — Strategy-side reformulation entirely

**Scope summary.** Accept the F4-F4-F4 evidence chain at face value:
**the signal-overlay-on-momentum task shape is wrong**, not just
the model. Reformulate the strategy architecture entirely —
e.g. (i) **multi-strategy ensemble** running v1 momentum + a
mean-reversion sibling + a vol-targeting layer all simultaneously,
weighted by regime / Sharpe-history; (ii) **regime-switching
master strategy** that dynamically allocates capital among 2-3
named strategies; (iii) **risk-parity portfolio construction** at
the symbol level rather than equal-weighted top-N momentum;
(iv) **alpha-blending framework** (factor-model style) where each
candidate signal (momentum, vol-targeting, regime, LLM) gets a
small allocation rather than competing for the same overlay slot.

**Reuse of existing infrastructure.**
- `crates/strategy` overlay composition → **architectural redesign
  needed**. Current pattern is single-baseline-with-overlay; this
  candidate's shape is multi-baseline-ensemble or master-dispatch.
  ADR-level work.
- v1 cross-sectional momentum baseline → reuses as ONE input in
  an ensemble; not the singular truth.
- `crates/risk` → may need redesign for portfolio-level risk
  vs strategy-level risk allocation.
- Mean-reversion strategy (v1.5 in product.md roadmap, never
  shipped) → may need to be built as part of this candidate.
- Vol-targeting layer (candidate 1) → may be a building block.
- Regime classifier (candidate 2) → may be a building block.

**Cost estimate.**
- Analyst pass: 3-4 weeks (broad scope; multiple ADR amendments;
  prerequisite scoping for v1.5 mean-reversion if not yet shipped).
- Architect lock: 2-3 weeks (substantial — new strategy registry
  shape; new risk allocation layer; ADR-amendment-heavy).
- Dev impl: 3-5 weeks (assumes some of candidates 1-2-6 land as
  building blocks first).
- Tester: 2-3 weeks.
- Compute: small per-piece; total depends on building blocks.
- **Total wall-clock: ~6-10 weeks**, possibly more if v1.5
  mean-reversion is a prerequisite that hasn't shipped.

**Risk + reversibility.**
- K-strat-1: **scope is largest of any candidate**. Operator's
  read in the retrospective was "bigger scope (~6-10 weeks);
  operator may not want this depth." Honest read of the survey:
  this is the highest-EV-on-success candidate but also the
  highest-spend.
- K-strat-2: **prerequisites compound** — multi-strategy ensemble
  benefits from having candidates 1, 2, 6 land first as building
  blocks. Spawning this without building blocks risks
  reinventing the building blocks inside the ensemble.
- K-strat-3: **architectural reversibility is LOW**. ADR
  amendments touching strategy registry shape are sticky;
  walking them back is expensive.
- Reversibility: LOWER than candidates 1-6.

**Hypothesis.** A multi-strategy ensemble that allocates among
{v1 momentum, mean-reversion sibling, vol-targeting} via regime-
conditional weights extracts ≥+0.20 Sharpe-delta vs v1 baseline
on BS-1 / BS-2 realdata. (Note: bar is higher because the spend
is higher — operator should demand bigger alpha if budget is
bigger.)

**Success vs F4 evidence.** Sharpe-delta ≥ +0.20; portfolio-level
drawdown narrower than v1 baseline; turnover bounded. F4-equivalent
failure: Sharpe-delta < +0.10 (i.e. the ensemble doesn't beat
candidates 1 or 2 standalone — meaning the architectural overhead
ate the alpha).

**Prior probability of clearing +0.10 Sharpe-delta: MEDIUM-HIGH.**
- Ensemble methods empirically beat individual strategies in most
  quant literature; the question is bar-height (above +0.10 is
  the survey's gate; above +0.20 is this candidate's stretch).
- The orchestration cost is real and eats alpha.
- Strong synergy with other candidates as building blocks.

**Why this differs from v2.5 F4.** This is the operator's
"task-framing was wrong" hypothesis — if true, reformulating the
overall strategy architecture is the right response. v2.5 asked
"is this overlay shape extractable"; this candidate asks
"is the overlay shape the right architectural choice at all?"

---

## Tabulated summary

| # | Direction | Total wall-clock | Compute | Reuse | Prior of +0.10 | Reversibility | Independence from v2.5 F4 |
|---|-----------|------------------|---------|-------|----------------|---------------|---------------------------|
| 1 | Volatility forecasting | ~4-6w | small | HIGH | MEDIUM-HIGH | HIGH | HIGH (different task) |
| 2 | Regime classification | ~4-6w | tiny | HIGH (regime.rs seed) | MEDIUM | HIGH | HIGH (different task) |
| 3 | 168h horizon retest | ~2-3w | moderate | HIGHEST (scaffold proven) | LOW | HIGH | LOW (same axis as F4) |
| 4 | Crypto-specific features | ~5-8w | moderate | MEDIUM (FEATURE_DIM-load-bearing) | MEDIUM | MEDIUM | MEDIUM (different signal-axis) |
| 5 | Reflection-memory + LLM | ~5-9w | LLM-cost-bound | HIGH | LOW-MEDIUM (high variance) | HIGH | HIGHEST (different signal-source entirely) |
| 6 | Non-DL approaches | ~4-6w | tiny | HIGH | LOW-MEDIUM | HIGH | LOW-MEDIUM (same task framing, different model) |
| 7 | Strategy-side reformulation | ~6-10w+ | building-block-bound | LOWER (ADR-heavy) | MEDIUM-HIGH | LOWER | HIGHEST (task-shape itself reformulated) |

**Per-candidate cost-effectiveness ranking** (Sharpe-delta-EV per
wall-clock-week, brief-time-estimated; analyst's read only):

1. **Candidate 1 (volatility)** — highest EV/week. Strong prior +
   strong infrastructure reuse + small compute + textbook precedent.
2. **Candidate 2 (regime)** — close second. Strong prior + already
   has `crates/reflection/src/regime.rs` seed.
3. **Candidate 6 (non-DL)** — third on cheap orthogonal evidence.
   Cheap-to-falsify; high information value even on F4 outcome.
4. **Candidate 5 (LLM)** — fourth on novelty + product
   differentiation. High variance EV; differentiator-aligned per
   product.md (memory + audit).
5. **Candidate 4 (features)** — fifth on data-bootstrap-tail.
   Strong potential signal but the long tail of data sourcing
   reduces EV/week.
6. **Candidate 7 (reformulation)** — sixth on operator-readiness.
   Highest stretch EV but highest spend and lowest reversibility.
   Best as a follow-on after 1-2 building blocks land.
7. **Candidate 3 (168h)** — seventh on lowest prior. Cheapest to
   falsify if operator wants horizon-axis closure; otherwise
   skip.

## Cross-cutting considerations

### Infrastructure leverage (constant across candidates)

The following crates and assets are **task-agnostic** and carry
forward to every candidate above:

- `crates/forecast/src/features.rs` 5-feature window + iterator —
  candidates 1-3-6 reuse verbatim; candidates 4-5 extend.
- `crates/forecast` `ForecastProvider` trait — new impls land
  alongside `TcnSyncForecaster` + `PatchTstSyncForecaster`.
- `crates/forecast/checkpoints/anchors/` — TCN + PatchTST
  checkpoints are **inference-capable for free**; candidates that
  want to use frozen forecast outputs as features (or as an
  ensemble baseline in candidate 7) get them at zero retraining
  cost.
- `crates/strategy` signal-level overlay pattern —
  candidates 1-3-5-6 reuse; candidate 7 extends.
- `crates/audit` ledger + `crates/reflection` lesson-card store —
  all candidates reuse for ledger emission and lesson-card
  retrieval; candidate 5 is the only one that **drives**
  reflection consumption.
- `crates/replay-cache` — strict-replay determinism for
  backtests; load-bearing for candidate 5 (LLM-call replay).
- `crates/llm` — only candidate 5 consumes directly; available
  for candidates 2 / 6 if they want LLM-emitted feature
  augmentation.
- `data/binance/` 10-USDT-pair hourly OHLCV 2023+2024 — all
  candidates reuse; candidate 4 needs additional bootstrap.
- Realdata backtest scenarios (`top10-2023-fy-*`, `top10-2024-fy-*`)
  — all candidates reuse as evaluation surface.

### Data availability

The current realdata is **10 USDT pairs hourly OHLCV 2023+2024** — a
~76k-window dataset per checkpoint span. This supports:

- All candidates' baseline (vol forecast, regime, 168h, non-DL, LLM).
- Candidate 4 needs additional bootstrap (funding, OI, perp basis;
  ~1-2 weeks dev work).
- Candidates 1/2 may benefit from finer intraday data (1m bars for
  realized-vol; tick-level for microstructure features) — out-of-
  scope today; operator-decide on whether to expand the data
  fidelity ladder per product.md "Universe & data fidelity ladder."

### Anchor implications

All candidates are **anchor-additive**. The 30 currently-locked
anchors across the v2.5 chain stay byte-identical (verified
invariant per the K4 patchtst_overlay_neutrality test + the
tcn_byte_identity test). New anchors land under new version pins:

- Candidate 1: `v2.7.0-volatility` or similar.
- Candidate 2: `v2.7.0-regime` or similar.
- Candidate 3: `v2.7.0-horizon-168h`.
- Candidate 4: `v2.7.0-features-extended`.
- Candidate 5: `v2.7.0-llm-overlay`.
- Candidate 6: `v2.7.0-nondl` or e.g. `v2.7.0-xgboost`.
- Candidate 7: bigger version bump (`v3.0.0-ensemble` style).

### ADR impact

- **No candidate violates** the immutable ADR-0033 § D3 F-verdict
  algorithm — but candidates 1, 2, 7 may amend it for new task
  shapes (vol F-verdict, regime F-verdict, multi-strategy
  evaluation).
- ADR-0028 (candle ML framework) extends additively for any DL
  candidate (1 if vol-DL; 6 if non-DL needs a non-candle crate).
- ADR-0029 (TCN checkpoint provenance) extends for any new
  checkpoint kind.
- ADR-0035 (post-training σ_train recalibration) applies to any
  new DL forecaster.
- ADR-0036 (PatchTST training contract) is precedent for any new
  Transformer-shape forecaster.
- **New ADRs likely:** candidate 1 risk-level overlay (vs
  signal-level); candidate 2 regime-conditional strategy
  dispatch; candidate 5 LLM-as-forecaster determinism contract;
  candidate 7 multi-strategy registry shape.

### Strategy lifecycle gates

Per product.md "Strategy lifecycle — promotion gates", any candidate
that lands a backtest-positive result still has to clear:

- `research → paper` gate: Sharpe > 1.0 on 2y OOS data + no tester
  fatals. Today's BS-1 + BS-2 cover 2 years; the candidate's gate
  for promotion is well-defined.
- `paper → live` gate: 30 days paper without risk-limit breach +
  cost-within-budget. **Out of scope for the candidate-pick decision**;
  this is downstream.

Operator may choose to defer all candidates to focus on paper-trade-live
infrastructure instead — also a valid resolution of the freed budget.

### Compounding effects

Several candidates **compound well as building blocks** for
candidate 7 (strategy reformulation). If operator picks more than
one, the sequencing matters:

- Vol-targeting (1) is a clean stand-alone deliverable AND a
  building block for ensemble (7).
- Regime classification (2) is a clean stand-alone deliverable
  AND a building block for ensemble (7).
- Non-DL forecaster (6) is a clean stand-alone deliverable that
  feeds into an ensemble (7) as one model among many.
- LLM overlay (5) is a stand-alone deliverable and the v2 LLM-
  strategy roadmap continuation; less obvious as a building
  block but possible.
- Candidates 3, 4 are mostly stand-alone — feature-extended
  scaffold (4) could later be used by all DL-based candidates.

## Recommended sequencing (if operator picks more than one)

Two operator-friendly sequencing options:

### Sequence A — "incremental + cheap-first"

1. **Candidate 1 (volatility)** — ~4-6 weeks. Most-likely-positive,
   smallest infrastructure risk, textbook precedent. Buys real
   alpha on the v1 baseline with high prior probability.
2. **Candidate 2 (regime)** — ~4-6 weeks. Layered on top of
   candidate 1's vol forecaster; regime classifier may use vol
   as a feature. Combined with vol-targeting, plausibly clears
   +0.20 Sharpe-delta total.
3. **Candidate 6 (non-DL) OR Candidate 5 (LLM)** —
   ~4-6 weeks. Operator-decide on whether the next-orthogonal
   evidence is cheap-non-DL or novel-LLM. Non-DL is cheaper to
   falsify; LLM is more aligned with the product.md "memory + audit"
   differentiator.
4. **Candidate 7 (strategy reformulation)** — ~6-10 weeks if 1-2-6
   landed positively; the building blocks compose into an ensemble
   shape. Skip if 1-2 alone delivered the alpha.

Total: **~18-28 weeks** for a 4-step sequence. Operator can ship
between any two and reconsider.

### Sequence B — "highest-EV-first single-bet"

1. **Candidate 5 (LLM)** — ~5-9 weeks. Highest novelty + most
   product-differentiated. If LLM-as-forecaster works, it's the
   v2.5-and-beyond story; if it doesn't, we have a baseline
   evaluation of the differentiator's signal-extraction value.
2. **Fallback to Sequence A** if candidate 5 F4s.

### Sequence C — "horizon-axis closure first"

1. **Candidate 3 (168h)** — ~2-3 weeks. Cheap-to-falsify closure
   on the horizon-axis exhaustion question. If F4 (likely), all
   remaining μ-prediction candidates have lower priors.
2. Sequence A from there.

## Open questions for operator-decide

> The analyst doesn't pick the next direction; the operator does.
> These questions surface what the operator should answer when they
> issue the next "next" directive.

### Q-PICK — Which candidate(s) get funded?

Operator picks a subset of {1, 2, 3, 4, 5, 6, 7} or **none**
(reallocate budget to paper-trade-live infra / UI / ops). No analyst
default; the decision is a function of operator's risk appetite,
budget tolerance, and product-strategy priorities.

### Q-SEQ — If more than one candidate, what's the sequencing?

Options:
- (A) incremental cheap-first (1 → 2 → {6 or 5} → 7).
- (B) highest-EV single-bet (5 standalone, fall back to A).
- (C) horizon-axis closure first (3 → A).
- (D) operator-defined other sequence.

### Q-BUDGET — What's the budget cap per candidate?

Operator may want to cap each candidate's spend so a single bet
doesn't consume the entire freed budget. Suggested defaults
(analyst-honest):

- Per-candidate cap: ~6-8 weeks wall-clock (operator-decide override).
- Hard tripwire: any candidate exceeding 1.5× its brief-time
  wall-clock estimate triggers an operator-decide pause.
- Cumulative cap: operator-decide; analyst suggests **≤16 weeks
  cumulative across all picked candidates before strategic
  re-review**.

### Q-DATA — Should the data-fidelity ladder be expanded?

Some candidates benefit from finer-grained data than the current
hourly OHLCV:

- Candidate 1 (vol): 1-minute bars for realized-vol estimation.
- Candidate 2 (regime): tick-level for microstructure features.
- Candidate 4 (features): funding rate, OI, perp basis from
  Binance perp endpoints.

Operator-decide whether to expand the data scope alongside any
candidate or hold the ladder at hourly OHLCV.

### Q-DEFER — Defer strategy work entirely?

Operator may choose to **not** pick any of {1..7} and instead
allocate the freed budget to:

- Paper-trade-live continuous-operation infrastructure (v3 success
  criterion per product.md).
- UI polish / cockpit features (already shipped Phase F; lumen
  Phase 6 assistant slot is gated on v2 LLM regardless of strategy
  picks).
- Process / tooling (e.g. `v2x-trading-state-bus` from Queue §
  Process).
- Ops + observability + cost-monitoring.

Strategy work is not the only useful thing the freed budget can fund.

### Q-PROCESS — Multi-survey or pick-first?

Operator may want a second survey-pass focused on a single
candidate (e.g. "deep-dive candidate 1: GARCH vs realized-vol vs
range-based estimator literature review before locking the
analyst pass") rather than going directly from this survey →
analyst-pass → feature folder.

If yes, queue a follow-on analyst-deep-dive on the chosen
candidate.

## Analyst's recommended top pick (and 2 alternates)

> The operator decides; the analyst's call is information-bearing,
> not authoritative.

### Top pick: **Candidate 1 (volatility forecasting)**

**Why.** Highest cost-effectiveness in the survey: strong prior
(GARCH baseline + textbook vol-targeting precedent give MEDIUM-HIGH
probability of clearing +0.10 Sharpe-delta), smallest compute
budget (GARCH fits in seconds; small DL refinement in hours, not
days), highest infrastructure reuse (5-feature window + features.rs
+ overlay composition all transfer with minor extension), and
genuinely orthogonal to the v2.5 F4 evidence (predicting σ is a
different signal than predicting μ; the v2.5 evidence chain
doesn't bound this prior).

**Risk-honest framing.** The only real risk is the
**risk-level-vs-signal-level overlay** ADR amendment; vol-targeting
is naturally risk-level (scaling position size) rather than
signal-level (modulating direction). The architect lock would
need an ADR for this overlay shape. Otherwise the candidate is
near-textbook.

**Concretely.** A 6-week feature-pass:
1. Weeks 1-2: analyst — GARCH(1,1) baseline vs range-based
   (Parkinson) vs rolling-std; data-availability decisions on
   whether to bootstrap 1-min realized-vol or use Parkinson on
   hourly highs/lows; cost / hypothesis brief.
2. Week 3: architect — ADR for risk-level overlay shape; decomp.md.
3. Weeks 4-5: dev — implement vol forecaster + vol-targeting
   overlay; integrate with v1 momentum baseline; realdata backtest.
4. Week 6: tester — full verification; presenter; operator approval.

### Alternative #1: **Candidate 2 (regime classification)**

**Why.** Close runner-up. Already has a 3-state BTC regime tagger
seed at `crates/reflection/src/regime.rs` (`Bull | Bear | Chop`
with ±2% threshold). Extending to hourly cadence + multi-symbol +
HMM-or-classifier refinement is well-bounded. Sharpe-lift via
regime-conditional dispatch is a textbook idea. Compute is tiny.
**The regime tagger as a building block compounds with candidate 1
and candidate 7.**

**When alt #1 wins.** Operator's prior is "regime structure is
load-bearing on crypto and we should use it explicitly" rather
than "vol clustering is the load-bearing signal."

### Alternative #2: **Candidate 5 (LLM-as-forecaster)**

**Why.** Highest novelty + best product-differentiation alignment.
The reflection-memory + LLM-debate-as-forecast-source is **on-
strategy with product.md's "moat = (2) + (4)"** call — persistent
memory and audit-ledger are exactly the assets a LLM-overlay
strategy compounds. If it works, it's the v2.5-and-beyond story
and lifts the entire product narrative beyond F4-F4-F4 DL
exhaustion. If it doesn't work, we have a baseline evaluation
of the differentiator's signal-extraction value — which is
information-bearing regardless.

**When alt #2 wins.** Operator's prior is "DL-on-OHLCV may be
exhausted, but LLM-reasoning-over-context+memory is a different
signal class altogether and we should go big on the
product-differentiated bet" — i.e. willing to absorb LOW-MEDIUM
prior in exchange for stretch-EV.

---

## Handoff envelope

```toml
[handoff]
from        = "analyst"
to          = "operator"
feature     = "strategy-reformulation-survey"
trace_refs  = []  # no new [[req]] rows — survey only
verdict     = "READY-FOR-STRATEGIC-DECIDE"
priority    = "high"  # the next "next" directive needs Q-PICK answered
notes       = """
Seven candidate directions tabulated; cost / EV / reuse / hypothesis
per candidate. Analyst's top pick is Candidate 1 (volatility); alts
are Candidate 2 (regime) and Candidate 5 (LLM-as-forecaster). No code,
no feature folder, no new [[req]] rows opened. Survey is exhaustive of
the retrospective's "what COULD usefully chase" enumeration.
"""

[inputs]
spec_files = [
  "spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md",
  "spec/v25-tcn-horizon-bump-or-retire/feature.md",
  "spec/v25a-patchtst-overlay/feature.md",
  "spec/product.md",
  "spec/backlog.md",
  "spec/architecture/12-forecast-overlay.md",
  "crates/forecast/src/features.rs",
  "crates/reflection/src/regime.rs",
]
brief = "none — orchestrator did not pass a brief; greenfield survey"

[outputs]
spec_files = [
  "spec/dev-notes/strategy-reformulation-survey-2026-05-22.md",
]
trace_rows_opened = []   # NONE — survey only
trace_rows_updated = []  # NONE — survey only
feature_folders_created = []  # NONE — survey only

[open_questions]
items = [
  "Q-PICK: which candidate(s) of {1..7} (or NONE) get the freed multi-week budget?",
  "Q-SEQ: if more than one, sequencing — incremental-cheap-first / single-EV-bet / horizon-closure-first / other?",
  "Q-BUDGET: per-candidate and cumulative budget caps?",
  "Q-DATA: expand the data-fidelity ladder (1m bars / tick / funding+OI+perp-basis) for any candidate?",
  "Q-DEFER: defer strategy work entirely and reallocate to paper-trade-live infra / UI / ops?",
  "Q-PROCESS: multi-survey deep-dive on the chosen candidate before feature pass, or go direct to analyst pass?",
]

[assumptions]
items = [
  "Freed budget is ~3-5 weeks of compute + analyst/architect/dev/tester bandwidth per the retrospective.",
  "The 30 v2.5-chain anchors stay byte-identical regardless of candidate; all are anchor-additive.",
  "ADR-0033 § D3 F-verdict algorithm stays immutable; new task shapes (vol, regime) may add new F-verdict shapes alongside it, not replace.",
  "Apple Silicon Metal stays the only compute substrate; no new external compute (cloud GPU, Python sidecar) is in scope for any candidate without explicit operator-decide.",
  "Realdata pipeline (10 USDT pairs hourly 2023+2024) is the evaluation substrate for all candidates' Sharpe-delta gates.",
  "The +0.10 Sharpe-delta vs v1 baseline gate from v25-tcn-overlay § success criterion stays the canonical alpha-unlock threshold for any candidate's backtest verdict.",
]
```

HANDOFF → operator-decide (Q-PICK + Q-SEQ + Q-BUDGET + Q-DATA + Q-DEFER + Q-PROCESS)

Open questions: see Q-PICK, Q-SEQ, Q-BUDGET, Q-DATA, Q-DEFER, Q-PROCESS above. The operator's next "next" directive should answer at least Q-PICK; the rest can autoapprove to analyst defaults.
