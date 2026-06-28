# Synthesis — 900 papers → a roadmap for our advisor

_Cross-topic distillation of the **complete** literature program: **900 papers
across 9 topic ledgers** (100 each), built over three rounds, then a **deep-read
pass** that upgraded the highest-value entries to full-text reads (primary PDFs via
`pdftotext`/ar5iv, not abstracts). The per-topic `knowledge.md` files are the detail;
this file is the **"what do we actually change in the app"** layer. Citations are
`topic[N]` → see `research/<topic>/papers.md`._

> **Our app:** Rust single-coin crypto **advisor** (paper/sim, not advice). Pick
> coin + budget → bake off strategies → rank under a FROZEN 1000-path
> moving-block-bootstrap gate (weakest-link verdict; buy-and-hold always the
> benchmark) → forward paper-trade. Thesis: **no active strategy robustly beats
> holding, net of costs.**

_Status: **complete + deep-read** (2026-06-28). The deep-read pass confirmed the
thesis at primary-source depth, turned the P0 gate-upgrade into exact codeable
formulas, and **corrected four round-3 claims** (flagged ⚠ below)._

---

## 1. The headline: nine independent reviews validate our thesis

Each topic, searched independently, converged on the same conclusion — active
single-asset trading does not robustly beat buy-and-hold net of costs. Full-text
reading only hardened it:

- **strategies:** the TA-efficacy arc, now read in full — Brock–Lakonishok–LeBaron
  "works in-sample" → Sullivan–Timmermann–White (best 5-day-MA 17.2%/yr IS, but the
  ex-ante trader gets 14.9% and a 1-day delay → Sharpe 0.34, p=0.26) → Bajgrowicz–Scaillet
  (16–70bps costs zero it; post-1962 nothing works even free, <5% of rules survive one
  rebalance) → Marshall (16/23 markets go to zero after snooping) → **Hudson–Urquhart:
  the best in-sample BTC rule goes negative OOS (Sharpe −0.05).** strategies[82][83][85][89][93].
- **backtesting:** the multiple-testing canon at full depth — DSR, PBO/CSCV, the
  False-Strategy Theorem, MinBTL, Harvey–Liu's nonlinear haircut. backtesting[1][2][3][29][80][87].
- **ml-trading:** Bysik–Ślepaczuk hourly BTC — frictionless XGBoost **+73.5%/yr → −64%/yr
  at 10bps**; after Holm, nothing beats B&H. Gu–Kelly–Xiu: ML's edge is cross-sectional,
  unavailable to one coin. ml-trading[89][13].
- **deep-learning:** "simple beats fancy," now evidence-backed — shuffling inputs degrades
  DLinear 27% but transformers ~0% (they never used time order); the deep edge is purely
  cross-variate → a single coin is where deep models add nothing. deep-learning[8][86][99].
- **data:** controlled comparison ranks **CPCV best / plain walk-forward worst** by PBO/DSR;
  GAN/diffusion generators **structurally smooth away tails** (a Gaussian latent prior cannot
  produce heavy tails). data[79][72][87].
- **evolution:** every "evolved alpha beats market" result rides a lever we're scoped out of;
  the honest papers land on "superb in-sample → unprofitable OOS net of costs." evolution[86][95].
- **llms:** FINSABER — **B&H beats FinMem/FinAgent on Sharpe on 3 of 4 stocks** (NFLX B&H
  0.62 vs FinAgent −0.42); KTD-Fin — **negative selection alpha for 9/10 models**; headline
  returns are passive style-harvesting. llms[86][100].
- **risk-and-sizing:** vol-targeting's Sharpe gain comes *only* from the negative return→vol
  leverage effect — **reversed in crypto** (γ=−0.261 crypto vs +0.115 equity). Risk tool,
  not Sharpe tool. risk-and-sizing[16][93].
- **crypto-market-structure:** the strongest external mirror of our gate — BTC, 27-fold
  walk-forward + bootstrap, +73%/yr → −64% at costs, cost-aware filter restores +65.4%
  (Sharpe 1.09 vs B&H 0.82) **yet Table 8: none significantly beats B&H after Holm.**
  crypto-market-structure[83].

⚠ **Correction (data):** round-3 logged McLean–Pontiff decay as "26%/58%." Full text gives
**~10% statistical/data-mining (statistically *insignificant*) + ~35% post-publication
crowding.** Subtler — but we crown the *max of many configs* (not one refereed hypothesis),
so the 10% is a floor, not a forecast, and the decay is **largest for the cheapest-to-arbitrage,
low-idiosyncratic-risk names = BTC/ETH/SOL** (our exact coins). data[94], strategies[92][93].

**Implication:** the frozen gate + benchmark is a *competitive advantage*. The roadmap
below hardens it and adds only honestly-gated experiments.

---

## 2. Prioritized roadmap

### P0 — Close the selection-bias gap in the gate (now an EXACT, codeable spec)

Five topics independently flagged the same hole, and the deep-read pass extracted the
**exact formulas** from the primary sources (Bailey–López de Prado, Harvey–Liu). Our
bootstrap tests each curve's robustness but does **not** correct for the multiple-testing
bias of crowning the **best of N** swept strategies. All inputs are already stored (the
per-strategy return matrix + N).

1. **Effective trial count `N_eff` — and a hard new mandate.** Use `N̄ = ρ̂+(1−ρ̂)·M`
   (M = configs, ρ̄ = mean pairwise return correlation), or ONC clustering / PCA.
   **Decisive caveat from full text:** when **M > T** (more configs than window bars —
   *our exact situation*), the correlation matrix is ill-conditioned and ρ̄ is itself
   overfit, so we **MUST dimension-reduce/cluster before estimating `N_eff`.** This was
   an open question in round 3; it is now a primary-source requirement.
   backtesting[1 App.3][86], strategies[32].
2. **Deflated Sharpe Ratio (exact):**
   `DSR = Z[(ŜR − SR₀)·√(T−1) / √(1 − γ̂₃·ŜR + ((γ̂₄−1)/4)·ŜR²)]`,
   threshold `SR₀ = √V[{ŜRₙ}]·((1−γ)·Z⁻¹[1−1/N_eff] + γ·Z⁻¹[1−1/(N_eff·e)])` (γ≈0.5772).
   **`V[{ŜRₙ}]` is the cross-trial dispersion of the baked-off configs' Sharpes — NOT the
   standard error of one Sharpe.** Crown only if `DSR ≥ 0.95` AND it beats B&H (B&H exempt).
   **Crypto kicker:** fat tails *shrink* the survivable trial budget — ŜR=2.5 clears at
   N=88 under Normal returns but only **N=46** at skew−3/kurt10. Heavy-tailed coins warrant
   *more* suspicion of large sweeps. evolution[98], strategies[32], backtesting[1][3].
3. **The haircut is nonlinear — "halve the Sharpe" is provably wrong.** At N=200 a genuine
   top-3 is cut **37% / 100% / 49%** (the middle one wiped). For the sub-0.4 net Sharpes a
   single coin realistically produces, the correct haircut is **>50% to near-total → the gate
   should crown almost nothing by construction.** backtesting[29][87], strategies[97].
4. **Composed-strategy penalty = a concrete critical-t ladder** (not a vague `n^k`):
   1-of-1 needs t≈2.1 (fat tails, not 1.96); 3-of-10 → t≈4; 3-of-20 → t≈5; 7-of-100 → t≈7;
   a k-signal combo reaches combined t≈√k·τ. **Threshold-tuning (our param search) inflates
   the bar MORE than equal-weighting** → tuned composed crowns get the highest hurdle.
   backtesting[80].
5. **Derive the crown threshold, don't hard-code it.** [40]'s **ORATIO odds-ratio** sets the
   bar from an explicit "a false 'beats-hold' is N× costlier than a miss" statement; the
   famous t=3.0 was, per Harvey, "never intended" as a universal cutoff. Make the cost
   asymmetry explicit and derive the DSR/t threshold from it. backtesting[40].
6. **PBO via CSCV** — model-free, from the same T×N matrix (S=16 → 12,780 splits, logit-rank,
   `PBO = ∫_{−∞}⁰ f(λ)`). Report PBO as a diagnostic and disqualify high values; the
   operating point is a calibration choice (≳0.5 = worse-than-coin-flip is clearly overfit;
   primary treatments use stricter bars — calibrate, report don't binary). backtesting[2], data[54].
7. **MinBTL pre-flight veto (cheap):** `MinBTL ≈ 2·ln(N)/SR²_target` years (loose bound);
   the exact form gives **5yr ⇒ ≤45 configs**, N=100 ⇒ 9.2yr, N=1000 ⇒ 13.8yr at SR=1.
   **Refuse to crown when the window is shorter than MinBTL(N).** ml-trading[95], evolution[29], backtesting[3].
8. **Studentize for power (SPA-style):** divide each config's excess-return-vs-hold by its
   bootstrap std (`z = w/σ̂`) before the max + a VAR(1)-calibrated block-size selector — a pure
   Reality-Check gate is too conservative to detect a real edge if one exists. backtesting[8].

- _Where:_ `crates/backtest/src/bakeoff/{robustness.rs,rank.rs}` + the ranking report. Surface a
  per-run **overfitting scorecard** (N_eff, DSR, PBO, MinBTL pass/fail) next to the verdict.
  **Additive — does not touch the FROZEN classifier bands.** Report the *deflated* statistic,
  not a binary.

9. **⚠ Design amendment — the forward paper-trade alone is insufficient.** The PBO paper [54]
   dismantles the single hold-out (high variance, blind to trial count). Keep the forward
   paper-trade (genuine unseen data), but **pair it with CSCV/PBO + DSR/MinBTL on the bake-off
   matrix.** This is the one place the deep read *amends* our design rather than endorsing it —
   converging with data[79] (walk-forward worst) and data[84] (don't trust one path).
10. **Data-driven block length** — Politis–White on each `(coin,window)` correlogram; the MBB
    variance estimate **= spectral density at zero = long-run variance**, so a too-short block
    under-estimates it → over-narrow CIs. ADR-0063 already cites Politis–White — confirm it's
    computed per-series and logged. backtesting[78], data[84].

### P1 — Test-data discipline (cheap, codify as gates)

11. **Per-report leakage audit:** shift-by-one-bar look-ahead on *every* indicator; in-sample-only
    fitting of any transform; R² on **returns, never price levels** (the frozen-LLM-on-ETH "win"
    is the price-level random-walk trap); selection/tuning leakage is **~40× preprocessing leakage**
    and inflates as `σ·√(2·ln K)` decaying only `O(1/√n)` → large-K/small-n (our case) = maximal
    inflation. data[80], llms[63].
12. **Validate the gate on synthetic no-alpha series** (GARCH/OU/Heston) — it must refuse to crown,
    and DSR/PBO must flag overfit picks. Standing regression test.
13. **Fee-sensitivity sweep + turnover penalty** as first-class ranking output. Costs are *the*
    decision variable. strategies, ml-trading[89].

### P1 — Honestly-gated candidate experiments (expected ≈ null, but worth proving)

14. **Vol-targeting overlay — reposition as a risk tool, loose & slow.** Full reads settled the
    mechanics: in crypto the return→vol leverage effect is reversed (γ=−0.261) so **promise
    drawdown/tail reduction, not Sharpe**, and measure each coin's return-vol correlation
    per-window. **Do NOT chase the target** — closing the feedback loop tightened vol tracking
    5.75× but blew turnover 93% → **1105%/yr**; open-loop is preferred for crypto. Use a slow
    EWMA (~126-day half-life) or HAR realized-vol, a no-trade band, **de-risk-only**, trigger on
    downside deviation, report Sortino/CVaR/median. risk-and-sizing[16][90][93].
15. **Drawdown overlay — the high-water-mark restart is load-bearing.** On BTC: drawdown
    modulation **with** restart held Sharpe 1.521 and cut max-DD 72%→20%; the **same controller
    without restart collapsed to Sharpe −0.043.** The multiplier is growth-optimal (model-free),
    not a hack. Honest cost: ~40% of B&H upside given back. Needs a day-1 baseline-divergence e2e
    (the v3-vol-overlay-noop precedent). risk-and-sizing[31][96].
16. **⚠ USDT exchange-inflow — DEMOTED on full reading (was "best new testable arm").** The real
    magnitude is **$100M inflow → +0.065% BTC / +0.11% ETH *next hour*** (inside round-trip costs);
    the daily horizon we trade is an appendix; the only economic test is **cost-free ETH options**,
    never spot net of fees. It would fail our daily bootstrap+cost gate → keep only as a weak
    vol-dampening hint. (The related "Tether→BTC" result is RDD-identified **2017 manipulation
    evidence**, not a forward signal.) Funding-sign froth is also weaker than billed (bidirectional
    Granger = endogeneity), and **open interest is fabricated on Bybit/OKX/Binance-inverse — only
    Kraken/HTX reconcile.** crypto-market-structure[68][30][66][91].
17. **Meta-labeling as a "trade-less" filter** — side = simple strategy; secondary classifier =
    whether-to-act/size on triple-barrier labels; requires avg-uniqueness sample weights +
    sequential bootstrap + purged/embargoed CV + Clustered-MDA (lifted AUC 0.716→0.779). Plausible
    win is cost-drag reduction, not return. ml-trading[2][94], data[53]. The implementable cousin:
    a **cost-aware execution filter** — act only when `|expected_move| > λ·c·|Δpos|` (λ=2.0, c=10bp);
    cut trades 98%, restored viability but **did not beat B&H** (Holm p=0.89–1.00). ml-trading[89].
18. **Regime-flat overlay with hysteresis** — jump-model bull/bear de-risk to cash, explicit
    switching penalty, OOS-CV params, detection-lag model; event-driven re-baking (not calendar).
    Needs its own day-1 divergence e2e. strategies, evolution[97].

### P2 — Generative synthetic test data (research-only)

19. **Keep the model-free moving-block bootstrap as the default.** Generators **structurally smooth
    tails** (a Gaussian latent prior cannot produce heavy tails) and overfit a single short path;
    Historical Simulation/GARCH tie-or-beat them on VaR. Our bootstrap's honest limit: it can't
    invent a *worse-than-seen* crash — best filled by a tail-stressed slice or an EVT generator,
    not a generic GAN/VAE. data[72][87], deep-learning.

### P2 — Narration & future structural edge

20. **Keep LLMs on the narration rail, off the alpha rail** (ADR-0064). Ground narration in the
    actual gated numbers (templated; free-form rationales hallucinate). llms.
21. **Funding-rate / basis carry** — highest-Sharpe crypto edge in the literature, market-neutral,
    non-predictive; needs perp+margin+funding + short support; stress-test, don't sell as free
    yield. strategies, crypto-market-structure[66].

---

## 3. What NOT to do (hype to avoid)

- **No deep nets / TSFMs as the alpha engine** — simple linear ≥ Transformers (shuffle test:
  DLinear −27%, transformers ~0%); random-init transformers perform like text-pretrained on time
  series; lower forecast MSE does **not** mean more profit. deep-learning[8][86], llms[73][82].
- **No un-budgeted factor/param/GP/LLM search** — automated alpha mining is industrialized
  data-snooping; charge the search budget against significance (DSR/MinBTL) and use a once-only OOS
  window. evolution[86][95], backtesting[80].
- **Never treat IC / accuracy / AUC / a single-window Sharpe as a verdict** — only equity-vs-hold
  net of costs over a path distribution counts; beware **drawdown-flattered ratio wins** (a
  low-return/low-DD strategy can top B&H on a drawdown-divided ratio while a paired t-test can't
  reject "no edge", p=0.24–0.92). deep-learning[99], ml-trading.
- **Don't add macro/social-media features expecting return gains** — they fail Granger causality and
  degrade crypto prediction; macro is at best a *vol/regime* overlay, and even that fails OOS outside
  rate-cut regimes. crypto-market-structure[52][74][95].
- **Don't trust reported volume / open-interest / order-book depth at face value** — fabricated on
  several venues; source derivatives metrics from reconcilable venues (Kraken/HTX). crypto-market-structure[90][91].

---

## 4. The LLM-on-financial-time-series verdict (the open question, answered at depth)

The program's headline research question — *has anyone trained an LLM/foundation model ON financial
time series, and does it beat a random walk on crypto?* — is now answered at full-text depth:

- **⚠ The "famous TSFMs are trained on finance" framing was overstated.** Full text: **Chronos and
  TimesFM have NO finance/crypto in-corpus**; Moirai's LOTSA is **0.10%** finance; Lag-Llama's
  "finance" is one daily FX set. The genuine "trained on crypto numbers" model is **FinCast**
  (1B-param MoE, 8.69% crypto, beats TimesFM ~2× on crypto MSE) — **but reports only price-level
  MSE, no return/PnL/Sharpe/B&H = the level-persistence trap.** Even the strongest constructive
  answer yields zero gate-credible crypto return-alpha. (Chronos's fixed-bin tokenizer is, per its
  authors, "theoretically infeasible to model a strong trend" — worst case for crypto.) llms[15][18][22][29].
- **The "language" ablates out** — controlled ablations show it; the method papers' own "no-LLM"
  baselines are under-powered/overfit. LLMTime: **GPT-4 < GPT-3** (RLHF degrades number calibration);
  its win is a seasonality/repetition bias crypto returns lack — use a *base*, not chat, model.
  llms[16][25][73].
- **The one forecastable numeric target is volatility** (for risk/sizing), not direction — and even
  that carries a calibration caveat. llms[19][83].

**Through-line:** *retrieve/explain yes, predict/trade no.* LLMs belong on the narration/research rail;
the frozen robustness gate decides what trades.

---

## 5. Map to the codebase

| Roadmap item | Touches | Nature |
|---|---|---|
| P0 N_eff (cluster-first when M>T) | `bakeoff/{robustness,rank}.rs` | additive; **do first** |
| P0 DSR crown rule (exact formula, >0.95 AND beats B&H) | `bakeoff/{robustness,rank}.rs` + report | additive; FROZEN bands untouched |
| P0 nonlinear haircut + composed critical-t ladder | `bakeoff/rank.rs` + report | additive |
| P0 ORATIO-derived threshold | `bakeoff/rank.rs` | additive; makes cost asymmetry explicit |
| P0 PBO via CSCV | `bakeoff/robustness.rs` + report | additive diagnostic |
| P0 MinBTL veto | `bakeoff/rank.rs` | additive; cheap pre-flight |
| P0 SPA studentization | `bakeoff/robustness.rs` | additive; restores power |
| P0 pair forward-trade with PBO/DSR | forward-plan + report | amendment (hold-out alone insufficient) |
| P0 data-driven block length | `bakeoff/bootstrap.rs` | confirm/expose (ADR-0063) |
| P1 leakage / no-alpha-gate / fee-sweep | new tests + ranking output | discipline gates |
| P1 vol overlay reposition (loose+slow, restart) | existing overlay + report | reframe + per-window ρ(ret,vol) |
| P1 meta-labeling / cost-aware filter / regime-flat | new candidate + day-1 divergence e2e | gated experiment |
| P2 generators (research-only) | research spike feeding bootstrap | keep bootstrap default |
| P2 LLM narration grounding | `agent::narration` (ADR-0064) | hardening |
| P2 funding/basis carry | perp+margin+funding engine (ADR-0051) | large, future |

---

## 6. Pointers

- Per-topic detail: `research/<topic>/knowledge.md` (themes, hold-up-vs-hype, paper map).
- Full ledgers (Title · Year · Source · % read · Summary · Relevance): `research/<topic>/papers.md` (100 each).
- Progress + resume protocol: `research/PROGRESS.md`, `research/README.md`.
- **Program complete + deep-read (2026-06-28): 900/900 papers; ~100 highest-value entries upgraded to
  full-text reads; four round-3 claims corrected (above).** The single highest-leverage next action is
  **P0** — the gate already stores everything DSR/PBO/MinBTL/N_eff need, and the deep-read pass turned
  the spec into exact, codeable formulas converged across four topics.
