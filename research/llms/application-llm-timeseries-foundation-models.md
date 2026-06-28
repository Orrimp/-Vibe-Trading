# Application — LLMs / foundation models trained ON numeric financial time series

_Decision doc for analyst & architect. Bucket **(b)** of the `llms` topic split:
foundation models / LLM-architectures trained directly on **numeric** series
(Chronos, TimesFM, Lag-Llama, MOMENT, Moirai, Time-LLM, GPT4TS, FinCast …) plus
the language-ablation studies. This **answers the operator's open question**:
*has anyone trained an LLM/foundation model on financial numbers, and does it beat
a random walk on crypto returns net of costs?* Companion file — bucket (a),
LLMs-on-text/agents —
[`application-llm-narration-and-agents.md`](application-llm-narration-and-agents.md).
Sources: `research/llms/knowledge.md`, `research/llms/papers.md` (cite `llms[N]`),
`research/SYNTHESIS.md`. No new papers._

> **Our app:** Rust single-coin crypto **advisor** (paper/sim, not advice, not
> live). Bake off EVERY strategy → rank under a FROZEN 1000-path moving-block-
> bootstrap gate (**buy-and-hold always the benchmark + exempt**) → forward plan →
> paper-trade. Thesis: **no active strategy robustly beats buy-and-hold net of
> costs.** LLMs/ML are **never a ranking input.**

> ## Bottom line (read this first)
> **YES**, foundation models trained on numeric series exist, are open, and are
> mature. **NO**, there is **no gate-credible evidence any of them beats a random
> walk / buy-and-hold on crypto *returns* net of costs.** The "language" prior
> **ablates out** of numeric forecasting. The one model genuinely trained on crypto
> (**FinCast** `llms[29]`) reports only **price-level MSE** — the
> **level-persistence trap** (tomorrow's price ≈ today's, which B&H already
> captures) — so even the strongest constructive case carries zero return-alpha
> evidence. **Volatility is the *only* defensible numeric target** (for risk/
> sizing), and even that is hedged by a calibration warning. **A TSFM must never
> enter the ranking.**

---

## 1. Summary of the research

**Two families — do not conflate them.**

**(i) Numeric-pretrained TSFMs** (learn from numbers; transformer/LLM
architecture; trained from scratch on time-series corpora). The famous ones:
Chronos `llms[15]` (Amazon; quantize into 4,094 bins, train T5 LMs), TimesFM
`llms[18]` (Google; patched decoder-only, 200M), Lag-Llama `llms[17]` (first open
TSFM; decoder-only on lags), MOMENT `llms[21]` (CMU; masked-patch encoder, general
analysis), Moirai `llms[22]` (Salesforce; any-variate multivariate), TimeGPT-1
`llms[24]` (Nixtla; first commercial, finance in-corpus, closed). Newer/catalog:
Time-MoE `llms[55]` (2.4B MoE), Sundial `llms[58]` (generative flow-matching),
Chronos-2 `llms[84]` (covariate-aware), TTM `llms[56]` (the anti-scale ~1M-param,
CPU-runnable model), and the deep baselines PatchTST `llms[53]`, iTransformer
`llms[54]`, N-HiTS `llms[57]`.

> **Decisive deep-read correction — "famous TSFMs are trained on finance" was
> overstated.** Chronos `llms[15]` and TimesFM `llms[18]` have **NO finance/crypto
> in-corpus** (web-traffic + synthetic). Moirai's LOTSA is **0.10%** finance
> `llms[22]`. Lag-Llama's "finance" is **one daily FX set** `llms[17]`. MOMENT's
> is daily Exchange rates only `llms[21]`. The genuine "trained on crypto numbers"
> answer is the *purpose-built* **FinCast** `llms[29]` (1B-param MoE, **8.69%
> crypto**, 1.78B points) — see §3.

**(ii) Text-LLM repurposing** (keep a language model, adapt the interface):
LLMTime `llms[16]` (digit-string encoding, zero-shot a frozen GPT-3/LLaMA-2),
Time-LLM `llms[23]` (reprogram a frozen Llama-7B), GPT4TS / "One Fits All"
`llms[61]` (freeze GPT-2's attention+FFN), UniTime `llms[62]`, TEMPO `llms[74]`.
Applied to crypto: a frozen-LLM-on-ETH study `llms[63]`.

**The skeptical evidence is the load-bearing part.**

- **The language prior ablates out — shown three ways.** `llms[25]` (NeurIPS-24
  Spotlight): remove the LLM, or swap it for a trivial attention layer, and
  accuracy is unchanged or *better*, at up to 3 orders of magnitude less compute.
  `llms[73]`: a from-scratch transformer on ~50M samples matches frozen GPT-2, and
  random-init matches text-pretrained (small-data overfitting masked the prior
  wins). `llms[80]`: zero-shot LLM forecasters are noise-sensitive and lose to
  simple models. `llms[78]`: LLMs hover near *random* on time-series reasoning. The
  method papers' OWN ablations claim the opposite (Time-LLM `llms[23]`: 14.7% MSE
  from the backbone; GPT4TS `llms[61]` Table 7: pretrained 0.427 vs random-init
  1.326) — **but that is the tell:** their in-house "no-LLM"/"random-init"
  baselines are left under-powered on small data, the exact overfitting `llms[73]`
  diagnoses. LLMTime `llms[16]` is the sharpest single datapoint: **GPT-4 is
  *worse* than GPT-3** (RLHF degrades number calibration; chat < base), and its win
  mechanism is an **Occam/repetition/seasonality bias — exactly what crypto
  returns lack.**
- **TSFMs are competitive-not-dominant, domain-bound, shock-fragile.** Off-the-
  shelf TSFMs don't transfer to finance `llms[27]`; zero-shot ability is tightly
  tied to pretraining domains `llms[26]`; Chronos/TimeGPT/Moirai match classical
  models in *stable* periods but **degrade under rapid shocks** `llms[20]` (crypto
  is shock-dominated — accurate when you don't need it, failing when you do).
  Chronos's `llms[15]` fixed-bin tokenizer is, per its authors, "theoretically
  infeasible to model a strong trend" — the worst case for trend-dominated crypto.
- **TSFMs are miscalibrated and not cost-justified.** "Beyond Accuracy" `llms[83]`
  finds calibration failures are *widespread* — models are confidently wrong — so
  even a vol-for-risk use inherits a calibration warning (a miscalibrated vol
  forecast on the risk rail can *increase* drawdown). Operationally, a complexity
  router beats deploying one universal TSFM `llms[66]` — small models usually win.
- **Forecasting error and trading profit DECOUPLE.** FinTSB `llms[82]` (regime-
  spanning, costs, includes Chronos/Time-MoE): **lower MSE ≠ more profit**, and
  XGBoost/LightGBM beat many deep/foundation models. FinTSBridge `llms[85]` is a
  companion finance-eval suite built for the same reason. This is the formal
  statement of why low forecast error tells us nothing about beating B&H.
- **The price-level trap is the recurring on-crypto offender.** The frozen-LLM-on-
  ETH "win" `llms[63]`, FinCast's crypto MSE `llms[29]`, the FinBERT-BiLSTM
  "98% accuracy / 0.019% MAPE" on BTC/ETH `llms[72]`, and TimesFM's Bitcoin Monash
  point `llms[18]` all post great *level* error against **no random-walk/B&H
  baseline** — meaningless for return direction.
- **Synthetic paths are not a stress-test substitute.** CTBench `llms[67]`:
  statistically realistic synthetic crypto series still **fail to support
  profitable trading** — supporting our resample-from-real-data bootstrap over
  generated paths (cf. `llms[44]`: LLM-simulated markets are unrealistically calm).

**The one positive: volatility.** Applied to *realized volatility*, a TSFM
(TimesFM) is only "a reasonable baseline" zero-shot and must be **incrementally
fine-tuned** to beat HAR/GARCH `llms[19]`; the win is on **volatility (forecastable,
persistent), not direction/PnL**. The balanced finance study `llms[43]` (TTM +
Chronos on yield/vol/spread) lands identically: TSFMs help on **volatility and
spread**, lose to specialized models on most tasks, and need finance-specific
pretraining. RiskLabs `llms[60]` (LLMs aimed deliberately at *risk*/variance)
reports success on vol — confirming the forecastable target is risk, not return.

---

## 2. Possible solutions / what can be done with this research

The honest menu is short, because the strong finding is negative on return-alpha.

1. **Answer the operator's question and close it.** The literature is now
   decisive: TSFMs exist, are open, and **do not beat a random walk / B&H on
   crypto returns net of costs** `llms[20][27][28][64][68]`. This file is the
   durable record of that answer.
2. **(The one constructive option) a gated VOLATILITY forecast feeding the SIZING
   overlay — never the ranking.** A *small* model (TTM `llms[56]`, PatchTST
   `llms[53]`, N-HiTS `llms[57]`, or the existing GARCH/HAR baselines) forecasts
   realized vol, which drives a **de-risk-only** sizing overlay. Vol is the one
   target with positive finance evidence `llms[19][43][60]` — but it inherits the
   calibration warning `llms[83]` and must still pass the cost-aware bootstrap-vs-
   B&H gate (and clear the existing GARCH/HAR baselines first).
3. **Keep model-free bootstrap as the stress-test substrate.** CTBench `llms[67]`
   and the calm-LLM-markets finding `llms[44]` say generated paths are not a safe
   substitute for resampling real history — confirms the current 1000-path design.
4. **Use the price-level trap as a standing test-data-discipline rule.** Any
   forecast claim must be measured on **returns, never price levels**, against a
   **random-walk/B&H baseline** `llms[63][72][18]`. This is a gate-discipline
   contribution regardless of whether any model is ever adopted.

---

## 3. Relevance for the project

**Honest verdict: no gate-credible return-alpha; vol-only — and even vol is a
"maybe," gated.**

- **Returns/alpha: NO.** No credible evidence any TSFM/LLM beats buy-and-hold on
  crypto returns net of costs. The forecastable part of price is the drift that
  B&H already captures; the un-forecastable part (the return) is where active
  timing would need to win, and that is exactly where these models are weakest
  `llms[20][28]`. On our own coins the careful tests confirm it: the peer-reviewed
  crypto TSFM study `llms[64]` gives **BTC Sharpe ~1.03 ≈ B&H** (the eye-popping
  ETH 4.29 is a *different* config under long/short with unstated costs — textbook
  error/profit decoupling `llms[82]`), and the careful crypto agent `llms[68]`
  comes out ≈ B&H.
- **FinCast `llms[29]` is the cleanest "yes, trained on crypto numbers" — and the
  cleanest illustration of why it doesn't help us.** A 1B-param MoE with 8.69%
  crypto beats generic TimesFM ~2× on crypto MSE (crypto_1day h=60: 0.2774 vs
  0.5730) — but it is **price-level point-MSE with NO return/PnL/Sharpe/B&H** (the
  level-persistence trap, where a random walk scores similarly), and it is a
  GPU-trained 1B model, the wrong shape for a lean local advisor. The strongest
  constructive answer still yields zero gate-credible crypto return-alpha.
- **Volatility: the one defensible numeric target — gated.** `llms[19][43][60]`
  give positive vol evidence; `llms[83]` warns it is miscalibrated. So *if*
  anything is ever bolted on, it is a vol forecast for **risk/sizing**, gated, and
  benchmarked against GARCH/HAR — not a return signal.
- **Architecture fit.** The codebase already has the right shape: `crates/forecast`
  exposes a **model-agnostic `ForecastProvider` trait** (deliberately narrower than
  `LlmProvider`) plus `vol.rs`, `garch.rs`, `markov_switching.rs`, and an
  `overlay.rs` combine seam, with `patchtst.rs`/`tcn.rs` behind a `candle` feature.
  This means a vol forecaster is a *contained* experiment behind an existing trait —
  it does **not** require touching the ranking.

---

## 4. Advantages for the project

- **The question is answered — a real planning advantage.** "Has anyone trained an
  LLM on financial numbers?" now has a sourced, durable answer: yes, and it does
  not beat holding crypto. This removes a recurring "should we adopt a TSFM?"
  distraction and lets the roadmap focus on the gate (SYNTHESIS P0).
- **A small, CPU-runnable vol option exists** if vol-for-sizing is ever pursued:
  TTM `llms[56]` (~1M params, CPU, exogenous-signal-aware) or the lighter deep
  baselines `llms[53][57]` — compatible with a lean local Rust advisor, no GPU/SaaS.
- **The price-level trap becomes a gate-discipline asset.** Codifying "measure
  returns vs a random-walk baseline, never price levels" `llms[63][72]` protects
  the advisor from a whole class of impressive-looking-but-meaningless results — a
  durable robustness win independent of any model.
- **Bootstrap-over-generators is externally validated.** CTBench `llms[67]` and
  `llms[44]` confirm the existing 1000-path moving-block bootstrap is the right
  stress-test substrate — no change needed, with citations.

---

## 5. Problems and challenges

- **The level-persistence trap (the defining methodological hazard here).** Price-
  level MSE/RMSE is trivially small because tomorrow's price ≈ today's; a random
  walk scores similarly. FinCast `llms[29]`, frozen-LLM-on-ETH `llms[63]`, FinBERT-
  BiLSTM `llms[72]`, TimesFM's Bitcoin point `llms[18]` all fall into it. **Any
  forecast must be scored on returns vs a random-walk/B&H baseline.**
- **Error/profit decoupling.** Lower forecast error does not mean more profit
  `llms[82]`; the best-accuracy config is not the best-Sharpe config `llms[64]`. A
  forecast that "looks accurate" can still lose to B&H net of costs.
- **Shock-fragility.** TSFMs degrade exactly during the regime breaks that matter
  for crypto `llms[20]` — accurate in calm, failing in crashes (where crash-
  avoidance would actually be valuable).
- **Miscalibration on the risk rail.** TSFMs are confidently wrong `llms[83]`; a
  miscalibrated vol forecast driving de-risking could *increase* drawdown — so even
  the one defensible use is hazardous without calibration checks.
- **The language prior is a mirage.** Reprogramming a chat LLM to forecast numbers
  (`llms[23][61][63]`) is debunked `llms[25][73]`; if any model is used, prefer a
  *small dedicated* numeric model or a *base* (not chat/RLHF) model `llms[16]`.
- **HARD CONSTRAINTS to respect:** **gate/bands FROZEN — a TSFM can NEVER be a
  ranking input** (it can only feed a *sizing/risk overlay*, downstream of the
  crown); USDT-denominated; **Decimal not f64** (a forecaster typically emits f64 —
  it must be confined to `crates/forecast`/the overlay and converted at the
  boundary, never injected into the Decimal money path); any overlay needs a **day-1
  baseline-equity-divergence e2e** (the v3-volatility-forecaster-noop precedent —
  unit tests + anchored reports are NOT sufficient to catch a no-op overlay);
  anchored SHAs byte-immutable (119/119); paper-only; lean/local — no GPU-bound 1B
  model `llms[29]`, no closed SaaS TSFM `llms[24][64]`.
- **Cost/complexity vs payoff.** A foundation model is rarely cost-justified vs a
  small/routed model `llms[66]`; for a single coin the multivariate machinery
  `llms[22][84]` is mostly unused.

---

## 6. Concrete next steps / candidate work items

Named, with codebase location and priority. The honest default is **do nothing on
return-forecasting**; the only constructive item is vol-for-sizing, gated.

- **P0 — Codify the "returns, not levels; vs random-walk baseline" leakage rule.**
  A standing test-data-discipline check (SYNTHESIS P1 item 11): every forecast/
  signal R² or error metric is computed on **returns**, never price levels, and any
  forecast claim must beat a random-walk/B&H baseline `llms[63][72][82]`. Location:
  test utility on the `crates/backtest`/`crates/audit` side. Cheap, high-leverage,
  needs no model. **(Aligns with bucket (a)'s honest-benchmark harness.)**
- **P1 — IF anything: a gated realized-volatility forecast feeding the SIZING
  overlay (never the ranking).** Use the existing `crates/forecast` seam: extend
  `vol.rs`/`garch.rs` (or add a small TTM/PatchTST-class model behind the existing
  `candle` feature, `patchtst.rs`/`tcn.rs`) to emit a realized-vol forecast that
  drives a **de-risk-only** sizing overlay via `overlay.rs` into `crates/risk`.
  Gates: (a) must clear the GARCH/HAR baseline `llms[19][43]`; (b) must pass a
  **calibration check** `llms[83]` before it can de-risk; (c) must pass the cost-
  aware 1000-path bootstrap-vs-B&H gate; (d) **day-1 baseline-equity-divergence
  e2e** (v3-vol-overlay-noop precedent). Honest expectation: marginal at best, and
  it sells **drawdown/tail reduction, not Sharpe** (cf. SYNTHESIS P1 vol-overlay
  reposition + the crypto leverage-effect reversal). Location: `crates/forecast/`
  + `crates/risk/` + a new candidate-strategy day-1 e2e.
- **P2 — Document FinCast `llms[29]` + the crypto-TSFM verdict in the forecast
  feature notes.** A short, sourced "we evaluated TSFMs for return-forecasting and
  rejected them; vol-only, gated" note in `spec/architecture/12-forecast-overlay.md`
  / the v2.5 forecast feature, so the decision is not re-litigated. Background/
  catalog (Chronos-2 `llms[84]`, Sundial `llms[58]`, TTM `llms[56]` as the
  "if we must, use a small model" references).

---

## 7. Open questions for analyst & architect

- **The single remaining empirical sub-question:** can a *small* TSFM/model
  (TTM/PatchTST) or the existing GARCH/HAR forecast crypto **realized volatility**
  well enough — and calibrated enough `llms[83]` — to drive a de-risk overlay that
  **passes the gate and beats GARCH/HAR**? This is the only open experiment with
  positive prior evidence `llms[19][43]`; everything else is answered-negative.
- Is the vol-for-sizing experiment worth the engineering + e2e cost given the
  honest expectation of a marginal, drawdown-only (not Sharpe) result, and given
  that a simple GARCH/HAR baseline may already capture most of it?
- If a forecaster is ever added, where exactly is the f64→Decimal boundary, so the
  forecast never contaminates the money path?
- Do we want the forecast overlay to consume **on-chain/funding covariates**
  (Chronos-2 / Moirai any-variate `llms[84][22]`) — or does that reintroduce the
  data-fabrication and cost-erosion problems flagged elsewhere in the program for a
  single coin?

---

## 8. What NOT to do / out of scope (load-bearing for this bucket)

- **Do NOT put a TSFM / foundation model / LLM-forecaster into the ranking.** It is
  not gate-credible on returns `llms[20][27][28][64][68]`; the gate/bands are FROZEN
  and the crown belongs to the bootstrap-vs-B&H verdict. A forecaster may *only*
  feed a downstream sizing/risk overlay, never the crown.
- **Do NOT adopt a TSFM to forecast crypto *returns/direction*.** Off-the-shelf
  doesn't transfer `llms[27]`, barely beats a random walk `llms[28]`, is shock-
  fragile `llms[20]`, miscalibrated `llms[83]`, and the only peer-reviewed crypto
  economic test gives BTC ~1.0 Sharpe ≈ B&H `llms[64]`.
- **Do NOT trust price-level MSE/RMSE** (incl. FinCast's crypto win `llms[29]`) as
  evidence of edge — demand returns + a random-walk baseline `llms[63][72]`.
- **Do NOT reach for a chat LLM to forecast numbers** — the language prior ablates
  out `llms[25][73]` and RLHF degrades number calibration (GPT-4 < GPT-3) `llms[16]`.
- **Do NOT replace the moving-block bootstrap with generative/synthetic paths** —
  statistical fidelity ≠ profitability `llms[67]`; LLM-simulated markets are
  unrealistically calm `llms[44]`.
- **Do NOT deploy a GPU-bound 1B model `llms[29]` or a closed SaaS TSFM
  `llms[24][64]`** — wrong shape for a lean, local, paper-only Rust advisor.
