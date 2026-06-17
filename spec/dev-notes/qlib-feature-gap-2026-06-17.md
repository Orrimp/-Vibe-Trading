# qlib feature-gap — scope-aware comparison (2026-06-17)

**Author:** analyst · **Type:** dev-note (research) · **Status:** informational

**Question asked:** What does Microsoft's [qlib](https://github.com/microsoft/qlib)
offer that THIS project lacks, and which gaps are genuinely worth considering vs.
deliberately excluded?

**One-paragraph answer up front.** Most of qlib's surface area is the
*alpha-prediction* machinery — a model zoo (LightGBM/XGBoost/CatBoost + LSTM/GRU/
ALSTM/Transformer/TCN/TFT/TabNet/…), alpha-factor mining (Alpha158/Alpha360),
supervised forecasting workflow, online rolling/retraining, RL execution, and
meta-learning (DDG-DA). This project **ran that thesis and concluded it loses**:
across price/OHLCV + derivatives-positioning + on-chain, no active strategy beat
passive buy-and-hold net of cost under a frozen block-bootstrap robustness rule
(`spec/product.md`, terminal verdict 2026-06-08). So qlib's headline capabilities
are not "todo gaps" here — they are **TRIED-AND-RETIRED** or **OUT-OF-SCOPE by
thesis / by Rust-vs-Python architecture**. The honest residue is *three or four*
genuinely-scope-fitting gaps, all in service of the surviving thesis ("measured
robustness, not asserted alpha" + cockpit + paper-trading), and even those are
modest, not foundational.

---

## Comparison table

Verdict legend: **HAVE** · **PARTIAL** · **MISSING** · **OOS-DECISION**
(deliberately excluded) · **OOS-THESIS** (excluded because we tested the thesis
and it failed) · **OOS-ARCH** (excluded because it's a Python-ecosystem feature
incompatible with a Rust single-binary) · **TRIED-RETIRED**.

| # | qlib capability | Verdict | Evidence / one-line note |
|---|---|---|---|
| 1 | **Data layer** (compact columnar store, expression+dataset caching, offline/online modes) | **HAVE** | Parquet corpus + pinned Binance/Yahoo data, deterministic seeds, cached run reports. CHANGELOG: `backtest-real-binance-data`, `binance-corpus-expansion`, `lab-yahoo-realdata`. |
| 2 | **Point-in-Time (PIT) database** (as-of, no look-ahead) | **PARTIAL** | We enforce PIT *per-feature, ad-hoc* via day-1 falsifiers + leak-checks (on-chain spike PIT leak-check distinguished causal vs look-ahead join). There is **no general as-of store**; PIT is proven case-by-case, not structurally guaranteed. See "genuinely-relevant gaps" #1. |
| 3 | **Alpha-factor expression engine** (Alpha158/Alpha360, formulaic factors) | **MISSING / OOS-THESIS-leaning** | We have a hand-rolled indicator suite (SMA/EMA/MACD/RSI/Bollinger/ATR/OBV/VWAP/CVD) and composed-strategy TOML assemblies (`v0.5`), but **no symbolic factor-expression DSL**. As an *alpha-mining* tool this is OOS-THESIS. As a *robustness-testing breadth* tool it is a real (small) gap — see #2. |
| 4 | **Model zoo — GBDT** (LightGBM, XGBoost, CatBoost) | **TRIED-RETIRED / OOS-THESIS** | `v3 XGBoost cheap-classifier` foreclosed when the OHLCV channel was exhausted (CHANGELOG "Retired research lines"). Prediction-as-alpha lost; not relitigated. |
| 5 | **Model zoo — Deep Learning** (LSTM/GRU/ALSTM/Transformer/Localformer/TCN/ADARNN/TFT/TabNet/…) | **TRIED-RETIRED** | The v2.5 4-phase DL programme (TCN + PatchTST + planned Transformer + v2.6 bake-off) reached terminal **F4** across two model families, no +0.10 Sharpe-delta on hourly OHLCV. RETIRED 2026-05-22 (`spec/product.md` § Empirical basis). |
| 6 | **Supervised forecasting workflow** (train → predict → score a return label) | **TRIED-RETIRED / OOS-THESIS** | This *is* the retired bet: "can a model predict the next return?" The product explicitly re-anchored to "does the strategy survive resampled histories?" (`spec/product.md` § Vision). |
| 7 | **`qrun` / YAML workflow management** | **OOS-ARCH** | Python config-driven pipeline runner. We are Rust single-binary; equivalent role filled by the Lab (`lab-run-save-compare`) + `cargo run --bin` + TOML strategy assemblies. No reason to import a YAML DSL. |
| 8 | **Backtesting engine** | **HAVE** | Deterministic backtest engine with v5 latency+slippage realism (`slippage_bps: 8`, sqrt market impact, ADR-0043/0045), byte-SHA-256 anchored reports. Arguably *more* friction-honest than qlib's default. |
| 9 | **Portfolio generation strategies** (TopkDropout, EnhancedIndexing) | **PARTIAL** | We have cross-sectional top-N momentum (`v1`) and a passive equal/cap-weight buy-and-hold baseline with documented rebalance cadence (`passive-baseline.md`). We do **not** have a generic portfolio-construction layer with turnover/holding controls as a reusable component. Mostly OOS — see #3 note. |
| 10 | **Portfolio optimization / risk model** (mean-variance, factor risk, planning-based opt) | **MISSING** | No covariance/factor-risk-model optimizer. RustQuant is wired for risk *metrics* (`spec/architecture.md`) but there is no MVO/risk-parity weighting layer. Candidate gap — see #3; judged LOW value given the passive verdict. |
| 11 | **Order execution / nested multi-level execution** (high-freq executor nesting) | **OOS-DECISION** | Non-goal: "Ultra-HFT sub-millisecond execution" + "no real-money execution" (`spec/product.md` Non-goals). We model fills at bar granularity with a friction model; nested HFT execution is explicitly out. |
| 12 | **RL execution / `qlib.rl`** (TWAP/PPO/OPDS order placement) | **OOS-DECISION + TRIED-RETIRED-adjacent** | Two reasons: (a) it's execution-microstructure RL → OOS by the HFT/live-exec non-goals; (b) strategy-level RL (`v3 RL policy`) was on the ladder but never reached promotion and the active-edge search concluded before it; prediction/learning-as-alpha is the retired thesis. |
| 13 | **Online serving + rolling / automatic retraining** | **OOS-THESIS (mostly)** | Online model rolling exists to keep a *predictive* model fresh in production. We ship passive + paper-trade; there is no production predictive model to roll. A *deterministic* param/route learning loop is a designed future seam (core pillar 3) but is NOT model-retraining. |
| 14 | **High-frequency trading support** (1-min/tick infra, HFT examples) | **OOS-DECISION** | Non-goal (Ultra-HFT). We ingest 1m bars + 1s aggregated trades (`v1.5b`) for signal, not for sub-second execution. |
| 15 | **Meta-learning (DDG-DA)** (forecast distribution shift, adapt the learner) | **OOS-THESIS** | DDG-DA adapts a *predictor* to non-stationarity. With no shipped predictor, there's nothing to meta-adapt. Our analogue of "handle non-stationarity" is the block-bootstrap robustness rule + cross-year sign-persistence live-bar, which *measures* fragility instead of adapting to it. |
| 16 | **Report / analysis** (graphical backtest reports, risk analysis) | **HAVE / exceeds** | Operator success reports (Sharpe/Sortino/Calmar/maxDD), cockpit Reports viewer, equity/drawdown rendering, FAMILY-UNIFORM anti-cherry-pick renderer, robustness distribution summaries. CHANGELOG `operator-success-reports`, `cockpit-reports-viewer`, `strategy-robustness-harness`. |
| 17 | **Reflection / persistent memory** | **HAVE (qlib lacks this)** | Reverse gap — `reflection-memory` lesson-card store with retrieval at decision time is a differentiator qlib has no equivalent of. Noted for completeness. |
| 18 | **Monte-Carlo robustness layer** (block-bootstrap, distribution-valued verdict) | **HAVE (qlib lacks this)** | Reverse gap — the project's epistemic core (core pillar 2). qlib scores point estimates / IC; it has no pre-registered block-bootstrap fragility gate. |

---

## Genuinely-relevant gaps, ranked

Only items that advance THIS project's *surviving* goals (measured robustness +
cockpit + paper-trading), not items that re-import the retired alpha thesis.

1. **A first-class point-in-time / as-of data discipline (table row #2).**
   *Worth it?* **Yes, modest-high — the one structurally-worthwhile gap.** Today
   PIT-cleanliness is re-proven per feature by hand (leak-checks, day-1
   falsifiers). qlib bakes it into the data layer so look-ahead is impossible by
   construction. For a project whose entire credibility rests on an *honest
   negative result*, a structural "you cannot join future data" guarantee
   hardens the most important claim we make. Likely a focused as-of-join helper +
   a lint, **not** a new database. Scope-fitting; would strengthen the moat.

2. **An alpha-factor *expression* engine — repurposed for robustness breadth, not prediction (row #3).**
   *Worth it?* **Marginal / optional.** A small formulaic-factor DSL would let us
   sweep *many* candidate signals through the robustness harness cheaply, which is
   in-thesis (we test signals to *falsify* them, not to trade them). But the
   active search is **CONCLUDED**; building factor-mining breadth now risks
   re-opening a closed question and importing scope we rejected. Only justified if
   the operator opens a *fresh* program on an untested channel (options/macro/
   social) — then a factor DSL pays off. Park it.

3. **Portfolio optimization / risk-model weighting layer (rows #9–#10).**
   *Worth it?* **Low, given the verdict.** MVO / factor-risk weighting matters
   when you have multiple alpha sleeves to combine. We ship passive buy-and-hold
   with a simple equal/cap-weight rebalance; there are no surviving active sleeves
   to optimize across. Revisit only if a future fresh program produces ≥2
   robust signals. Not worth building speculatively.

4. **(Honest non-gap) RL / online-rolling / meta-learning / model zoo.**
   *Worth it?* **No.** These are the heart of qlib and the heart of what we
   *deliberately retired or scoped out*. Listing them as "missing" would be
   dishonest: they are absent **by tested conclusion** (prediction-as-alpha lost)
   or **by non-goal** (no live exec, no HFT). No action.

---

## Bottom line (honest)

**We deliberately don't do most of qlib because we tested that thesis and it
failed** — qlib is an alpha-prediction research platform (model zoo + factor
mining + supervised forecasting + RL + online rolling + meta-learning), and this
project's terminal verdict is that active alpha-prediction does not beat passive
on its reachable data, so those capabilities are TRIED-AND-RETIRED or
out-of-scope by thesis, not backlog gaps. Layered on top, qlib's Python/`qrun`/
HFT-execution machinery is architecturally out of scope for a Rust single-binary,
no-live-trading research+cockpit agent. The genuinely scope-fitting residue is
small: **(1) a structural point-in-time / as-of data guarantee** is the one
clearly worthwhile hardening (it strengthens the honest-negative-result moat);
**(2) a factor-expression DSL** is a *maybe*, justified only if a fresh
untested-channel program is opened; **(3) portfolio/risk-model optimization** is
low-value until ≥2 robust sleeves exist. Conversely, this project *has* two things
qlib lacks entirely — a block-bootstrap robustness gate and a persistent
reflection-memory loop — which is the whole point: it optimizes for *measured
robustness*, not for breadth of predictors.
