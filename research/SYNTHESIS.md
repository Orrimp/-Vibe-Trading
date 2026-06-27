# Synthesis — 900 papers → a roadmap for our advisor

_Cross-topic distillation of the **complete** literature program: **900 papers
across 9 topic ledgers** (100 each), built over three resumable rounds. The
per-topic `knowledge.md` files are the detail; this file is the **"what do we
actually change in the app"** layer. Citations are `topic[N]` → see
`research/<topic>/papers.md`._

> **Our app:** Rust single-coin crypto **advisor** (paper/sim, not advice). Pick
> coin + budget → bake off strategies → rank under a FROZEN 1000-path
> moving-block-bootstrap gate (weakest-link verdict; buy-and-hold always the
> benchmark) → forward paper-trade. Thesis: **no active strategy robustly beats
> holding, net of costs.**

_Lineage: batch 1 (117 papers, 5 topics) → split `llm-and-evolution` into
`evolution` + `llms`, added `data`, `risk-and-sizing`, `crypto-market-structure`
→ rounds 2 & 3 extended every topic to 100. Program complete 2026-06-27._

---

## 1. The headline: nine independent reviews validate our thesis

Each topic, searched independently by a separate agent, converged on the same
conclusion — active single-asset trading does not robustly beat buy-and-hold net
of costs. Round 3 added the decisive **on-our-own-asset, with-our-own-method**
replications:

- **strategies:** the 30-year TA-efficacy arc — Brock–Lakonishok–LeBaron "works
  in-sample" → Sullivan–Timmermann–White "dies OOS after data-snooping correction"
  → Bajgrowicz–Scaillet "erased by costs, and the future-best rule was never
  selectable ex ante," confirmed across 49 markets and **on Bitcoin specifically**
  (in-sample-best rule goes negative OOS). strategies[82][83][85][89][93].
- **backtesting:** the multiple-testing canon (DSR, PBO/CSCV, False Strategy
  Theorem, MinBTL) — crowning the best of N swept strategies is overfitting unless
  deflated; published predictors lose 26% OOS / 58% post-publication.
  backtesting[1][2][86][87].
- **ml-trading:** Bysik–Ślepaczuk hourly BTC, 27-fold walk-forward — frictionless
  XGBoost **+73.5%/yr → −64%/yr at 10bps**; after Holm correction **nothing
  significantly beats buy-and-hold**. Gu–Kelly–Xiu: ML's real edge is
  cross-sectional and tiny, unavailable to one coin. ml-trading[89][13].
- **deep-learning:** simple beats fancy (N-HiTS/DLinear ≥ Transformers); DRL
  profits are "false positives due to overfitting"; LSTM-ARIMA *looks* like it
  beats hold but a paired t-test can't reject no-edge. deep-learning[86][99][18].
- **data:** controlled comparison ranks **CPCV best / plain walk-forward worst**
  by PBO and Deflated-Sharpe — independent validation of multi-path resampling;
  GAN/diffusion generators smooth away the tails. data[79][87].
- **evolution:** every "evolved alpha beats market" result rides a lever we're
  scoped out of (HFT latency, cross-sectional breadth, leverage, carry); the
  honest papers land on "superb in-sample → unprofitable OOS net of costs."
  evolution[86][95].
- **llms:** FINSABER (20yr, bias-controlled, costs) — LLM-agent advantages
  "vanish… no significant alpha (all p > 0.34)"; KTD-Fin (identifier-masked) —
  **negative** selection alpha for 9/10 models; headline returns are passive
  style-harvesting. llms[86][100].
- **risk-and-sizing:** vol-targeting's Sharpe gain comes *only* from the negative
  return→vol leverage effect — which is **absent/reversed in crypto** (positive
  returns drive crypto vol). The overlay is a risk tool, not a Sharpe tool.
  risk-and-sizing[16][93].
- **crypto-market-structure:** the strongest external mirror of our gate — BTC,
  27-fold walk-forward + bootstrap vs B&H, +73%/yr → −64% at costs, cost-aware
  filter restores +65% but **bootstrap shows no significant Sharpe outperformance
  over buy-and-hold.** crypto-market-structure[83].

**Implication:** the frozen gate + benchmark is a *competitive advantage*, not a
limitation. The roadmap below hardens it and adds only honestly-gated experiments.

---

## 2. Prioritized roadmap

### P0 — Close the selection-bias gap in the gate (now with a concrete, implementation-ready spec)

**Five topics independently flagged the same hole and, in round 3, handed us the
exact recipe.** Our bootstrap tests **each curve's** robustness but does **not**
correct for the multiple-testing bias of crowning the **best of N** swept
strategies. The corrections need only inputs we already store (the per-strategy
return matrix + N tried):

1. **N must be the EFFECTIVE trial count, not the raw config count.** Cluster the
   candidates' return-correlation matrix (ONC / correlation-distance clustering)
   and use the **cluster count** as `N_eff`. Critical for us: SMA-50≈SMA-51
   redundancy means raw N would over-deflate punitively; composed strategies
   inflate toward `(single-signal count)^k`. **Implement this first** — it makes
   the haircut correct rather than punitive. backtesting[86], strategies[100].
2. **Deflated Sharpe Ratio crown rule.** `DSR = PSR(E[max SR])` using the crown's
   own skew/kurtosis (crypto fat tails bite); threshold
   `E[max SR] ≈ √V_SR·[(1−γ)·Z⁻¹(1−1/N_eff) + γ·Z⁻¹(1−1/(N_eff·e))]`.
   **Crown only if `DSR > 0.95` AND it beats buy-and-hold** (B&H exempt, per
   ADR-0066). backtesting[1][3], evolution[98], data[73].
3. **Probability of Backtest Overfitting (PBO) via CSCV** — model-free, from the
   same return matrix with purge+embargo; **down-rank `PBO > 0.5`** as a
   disqualification filter. backtesting[2][9], data[79].
4. **MinBTL veto (new, cheap).** Bailey–Borwein–LdP–Zhu give an analytical
   minimum-backtest-length as a function of N tried; below it the best in-sample
   Sharpe is a guaranteed artifact (and under serial correlation — crypto has it —
   overfit picks have *negative* expected OOS). We know N, so compute MinBTL and
   **refuse to crown when our window is shorter.** ml-trading[95], backtesting[86].
5. **Studentize for power (SPA-style).** Divide each config's excess-return-vs-hold
   by its bootstrap std before the max — a pure Reality-Check gate is too
   conservative to detect a real edge if one exists. Crypto's mild inefficiency
   means the gate must stay *able* to find an edge, with costs + B&H as the
   deciding filters. backtesting[93][99].

- _Where:_ `crates/backtest/src/bakeoff/{robustness.rs,rank.rs}` + the ranking
  report. Surface a per-run **overfitting scorecard** (N_eff, DSR, PBO, MinBTL
  pass/fail) next to the verdict. **Additive — does not touch the FROZEN classifier
  bands.** Report the *deflated* statistic, not a binary: a high-deflated-t crown
  (t≫3–4 after deflating by N_eff) is the rare genuine winner.

6. **Data-driven bootstrap block length** — Politis–White corrected selector on each
   `(coin,window)` correlogram; too-short blocks bias verdicts toward "looks
   robust." ADR-0063 already cites Politis–White — **confirm it's computed
   per-series and logged** (action may be "expose + verify," not "add").
   backtesting[78][89], data[84][88][100].

### P1 — Test-data discipline (cheap, codify as gates)

7. **Per-report leakage audit:** shift-by-one-bar look-ahead test on *every*
   indicator; in-sample-only fitting of any transform; R² on **returns, never price
   levels** (the frozen-LLM-on-ETH "win" is the price-level random-walk trap);
   audited cost spec. backtesting, ml-trading, data[67][68], llms[63].
8. **Validate the gate on synthetic no-alpha series** (GARCH/OU/Heston) — it must
   refuse to crown, and DSR/PBO must flag overfit picks. Standing regression test.
   data, backtesting.
9. **Fee-sensitivity sweep + turnover penalty** as first-class ranking output —
   report the round-trip cost at which each strategy flips from beating to losing vs
   hold. Costs are *the* decision variable. strategies, ml-trading[89].

### P1 — Honestly-gated candidate experiments (expected ≈ null, but worth proving)

10. **Vol-targeting overlay — reposition as a risk tool, with a crypto caveat.** We
    already ship it; the literature says: in crypto the return→vol leverage effect
    is absent/reversed, so **don't promise Sharpe — promise drawdown/tail/vol-of-vol
    reduction** (universal), and measure each coin's return-vol correlation
    per-window. Mechanics: EWMA λ≈0.94 or HAR realized-vol for σ̂ (better point-vol
    ≠ better tail); **no-trade band / state-gated trigger** (continuous re-sizing
    goes net-negative on turnover); trigger on **downside deviation**, report
    Sortino/CVaR/median. risk-and-sizing[16][66][85][93][96].
11. **USDT exchange-inflow flows — the most credible NEW testable exogenous arm.**
    Stablecoin "dry powder" arriving at exchanges positively predicts BTC/ETH
    returns and lowers vol — genuinely exogenous/demand-side, unlike Fear&Greed and
    social sentiment (which fail Granger causality). Worth a probe through the gate
    (caveats: intraday horizon, paid on-chain feed). crypto-market-structure[68][100].
12. **Meta-labeling as a "trade-less" filter** — keep a simple strategy as the
    *side*; add a small interpretable classifier deciding *whether to act*,
    triple-barrier-labeled, CPCV-validated, gated vs hold net of costs. Plausible
    win is cost-drag reduction, not return. ml-trading[2], data[66][70][74].
13. **Regime-flat overlay with hysteresis** — jump-model bull/bear detector that
    de-risks to cash in bear regimes, with an explicit **switching penalty** + OOS-CV
    params + detection-lag model. Maps onto our long/flat decision; needs its own
    day-1 baseline-divergence e2e (the v3-vol-overlay-noop precedent). strategies,
    evolution[97] (event-driven re-baking, not calendar-driven).

### P2 — Generative synthetic test data (research-only)

14. **Treat generators as research, not as the gate.** Round-3 verdict: GAN/diffusion
    market generators **smooth away the tails** and overfit a single short path;
    Historical Simulation/GARCH tie-or-beat them on VaR. **Keep the model-free
    moving-block bootstrap as the default** (it can't invent dynamics the data never
    showed); explore diffusion only with a hard guardrail — must reproduce the coin's
    stylized facts AND must NOT leak the held-out test path. data[87][91], deep-learning.

### P2 — Narration & future structural edge

15. **Keep LLMs on the narration rail, off the alpha rail** (ADR-0064 is the correct
    use). Ground narration in the **actual gated numbers** (templated explanation of
    real metrics; free-form rationales hallucinate). llms.
16. **Funding-rate / basis carry** is the highest-Sharpe crypto edge in the
    literature — market-neutral (long spot / short perp), non-predictive; aligns with
    perp-basis work (ADR-0051). Needs perp+margin+funding model + short support;
    stress-test it, don't sell it as free yield. strategies, crypto-market-structure[4][66].

---

## 3. What NOT to do (hype to avoid)

- **No deep nets / TSFMs as the alpha engine** — simple linear ≥ Transformers;
  random-init transformers perform like text-pretrained ones on time series; lower
  forecast MSE does **not** mean more profit. deep-learning[86], llms[73][82].
- **No un-budgeted factor/param/GP/LLM search** — automated alpha mining is
  industrialized data-snooping; charge the search budget against significance
  (DSR/MinBTL) and use a once-only OOS window, or expect the gate to reject it.
  evolution[86][95], backtesting[86].
- **Never treat IC / accuracy / AUC / a single-window Sharpe as a verdict** — only
  equity-vs-hold net of costs over a path distribution counts. ml-trading, deep-learning[89].
- **Don't add macro/social-media features expecting return gains** — they degrade
  crypto prediction and fail Granger causality; keep features small + technical.
  crypto-market-structure[52][74][95]. (Macro is at best a *vol/regime* overlay,
  and even that fails OOS outside rate-cut regimes.)
- **Don't trust reported volume / open-interest / order-book depth at face value** —
  all are partly fabricated on some venues; widen effective-spread assumptions and
  source derivatives metrics from reconcilable venues. crypto-market-structure[90][91].

These validate the **expected-null** framing of our fresh-channel probes (ADR-0072
DVOL, ADR-0073 macro): honest coverage, not asserted alpha.

---

## 4. The LLM-on-financial-time-series verdict (the open question, answered)

The program's headline research question — *has anyone trained an LLM/foundation
model ON financial time series (numeric, not text), and does it beat a random walk
on crypto?* — is now answered across `llms` (bucket b):

- **TSFMs exist and ARE trained on numeric series** (Chronos, TimesFM, Lag-Llama,
  MOMENT, Moirai, Time-LLM, GPT4TS) — but on crypto **returns** they land at
  **≈ buy-and-hold** (BTC Sharpe ≈ 1.0; the eye-popping ETH 4.29 is long/short,
  no costs). llms[64][68].
- **The "language" ablates out** three independent ways: random-init transformers
  perform like text-pretrained ones; zero-shot LLM forecasters lose to simple
  models; LLMs hover near *random* on time-series reasoning. llms[25][73][78][80].
- **The one genuinely forecastable numeric target is volatility** (for risk/sizing),
  not direction — and even that carries a calibration warning. llms[19][43][83].

**Through-line:** *retrieve/explain yes, predict/trade no.* LLMs belong on the
narration/research rail; the frozen robustness gate decides what trades.

---

## 5. Map to the codebase

| Roadmap item | Touches | Nature |
|---|---|---|
| P0 N_eff via ONC clustering | `bakeoff/{robustness,rank}.rs` | additive; **do first** |
| P0 DSR crown rule (>0.95 AND beats B&H) | `bakeoff/{robustness,rank}.rs` + report | additive; FROZEN bands untouched |
| P0 PBO via CSCV | `bakeoff/robustness.rs` + report | additive disqualification filter |
| P0 MinBTL veto | `bakeoff/rank.rs` | additive; cheap |
| P0 SPA studentization | `bakeoff/robustness.rs` | additive; restores power |
| P0 data-driven block length | `bakeoff/bootstrap.rs` | confirm/expose (ADR-0063) |
| P1 leakage / no-alpha-gate / fee-sweep | new tests + ranking output | discipline gates |
| P1 vol overlay reposition | existing overlay + report | reframe + per-window ρ(ret,vol) |
| P1 USDT-inflow arm | new exogenous arm (ADR-0072/0073 pattern) | gated probe |
| P1 meta-labeling / regime-flat | new candidate + day-1 divergence e2e | gated experiment |
| P2 diffusion test-data | research spike feeding bootstrap | research-only |
| P2 LLM narration grounding | `agent::narration` (ADR-0064) | hardening |
| P2 funding/basis carry | perp+margin+funding engine (ADR-0051) | large, future |

---

## 6. Pointers

- Per-topic detail: `research/<topic>/knowledge.md` (themes, hold-up-vs-hype, paper map).
- Full ledgers (Title · Year · Source · % read · Summary · Relevance):
  `research/<topic>/papers.md` (100 each).
- Progress + resume protocol: `research/PROGRESS.md`, `research/README.md`.
- **Program complete: 900/900 papers across 9 topics.** The single highest-leverage
  next action is implementing **P0** — the gate already stores everything DSR/PBO/
  MinBTL need; this is the upgrade five independent reviews converged on.
