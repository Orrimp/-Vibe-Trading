# Synthesis — 117 papers → a roadmap for our advisor

_Cross-topic distillation of batch 1 (117 papers across 5 topic ledgers). The
per-topic `knowledge.md` files are the detail; this file is the **"what do we
actually change in the app"** layer. Citations are `topic[N]` → see
`research/<topic>/papers.md`._

> **Our app:** Rust single-coin crypto **advisor** (paper/sim, not advice). Pick
> coin + budget → bake off strategies → rank under a FROZEN 1000-path
> moving-block-bootstrap gate (weakest-link verdict; buy-and-hold always the
> benchmark) → forward paper-trade. Thesis: **no active strategy robustly beats
> holding, net of costs.**

---

## 1. The headline: five independent reviews validate our thesis

Each topic, searched independently, converged on the same conclusion — active
single-asset trading does not robustly beat buy-and-hold net of costs:

- **strategies:** the cleanest single-coin BTC study — technical rules beat hold
  *gross*, **all lose at 2–4% round-trip fees** (strategies[11]); seminal edges
  (momentum, pairs) decay and are cost-sensitive (strategies[2][4][16]).
- **backtesting:** Bajgrowicz–Scaillet (FDR-corrected, real prices → no robust
  net edge, can't pick the future-best rule) and LookAheadBench (agents fail once
  look-ahead is removed) (backtesting[24][15]).
- **ml-trading:** Gu–Kelly–Xiu — ML's real edge is **cross-sectional and tiny**
  (~0.4% monthly R²), unavailable to one coin; a no-intelligence baseline beats
  published AI traders under honest testing (ml-trading[1][13]).
- **deep-learning:** DLinear beats Transformers; DRL trading profits are
  "false positives due to overfitting"; the spectacular crypto numbers are
  bull-market/leakage/short-window artifacts (deep-learning[8][18][10][21][23]).
- **llm-and-evolution:** StockBench — 14 frontier LLMs barely edge a cost-free
  buy-and-hold and **all underperform it in downturns**; the double-OOS crypto
  study finds optimized params **lose to *random* params 87–92% of the time**
  (llm-and-evolution[22][15]).

**Implication:** the frozen gate + benchmark is a *competitive advantage*, not a
limitation. The roadmap below hardens it and adds only honestly-gated experiments.

---

## 2. Prioritized roadmap

### P0 — Close the selection-bias gap in the gate (highest leverage)

Three topics independently flagged the same hole: our bootstrap tests **each
curve's** robustness but does **not** correct for the multiple-testing bias of
crowning the **best of N** swept strategies. The corrections need only inputs we
already store (N tried + per-strategy return series):

1. **Deflated / Probabilistic Sharpe Ratio** haircut on the crowned strategy —
   subtract the expected max Sharpe over N noise trials (≈ √(2 ln N)·σ_SR),
   skew/kurtosis-aware (apt for crypto fat tails). backtesting[1][3], ml-trading[20].
2. **Probability of Backtest Overfitting (PBO) via CSCV** — model-free, needs only
   the per-strategy return matrix; usable as a **disqualification filter** (improved
   crypto OOS survival in crashes). backtesting[2][21], ml-trading[7], deep-learning[18].
3. **Confirm the gate is Reality-Check-style** (best-vs-benchmark over ALL configs,
   not a single-strategy test) and consider **Hansen SPA / Romano–Wolf StepM** for
   correlation-aware power (Bonferroni over-penalizes correlated configs).
   backtesting[6][7][8].
   - _Where:_ `crates/backtest/src/bakeoff/{robustness.rs,rank.rs}` + the ranking
     report. Surface a per-run **overfitting scorecard** (N, DSR, PBO) next to the
     verdict. This is additive — does not touch the FROZEN classifier bands.

4. **Make the bootstrap block length data-driven** — use the Politis–White corrected
   selector on each `(coin,window)` correlogram; too-short blocks bias verdicts toward
   "looks robust." backtesting[10][20], ml-trading. _Note:_ ADR-0063 already cites
   Politis–White block length — **confirm it's computed per-series and log it** in the
   report (the action may be "expose + verify," not "add").

### P1 — Test-data discipline (cheap, codify as gates)

5. **Per-report Seven-Sins / leakage audit:** a shift-by-one-bar look-ahead test on
   *every* indicator; in-sample-only fitting of any transform; R² on **returns, never
   price levels**; survivorship (dead coins) if we ever go multi-coin; an audited cost
   spec. backtesting[17][15][14], ml-trading[5][16].
6. **Validate the gate on synthetic no-alpha series** (GARCH/OU/Heston) — it must
   refuse to crown a winner, and DSR/PBO must flag overfit picks. A standing
   regression test. backtesting[11][19], ml-trading[14].
7. **Fee-sensitivity sweep + turnover penalty** as first-class ranking output — report
   the round-trip cost at which each strategy flips from beating to losing vs hold
   (replicate strategies[11]'s fee sweep). Costs are *the* decision variable.

### P1 — Honestly-gated candidate experiments (expected ≈ null, but worth proving)

8. **Meta-labeling as a "trade-less" filter** — the single best ML experiment. Keep a
   simple strategy as the *side*; add a small interpretable classifier (tree/logit)
   deciding *whether to act*, triple-barrier-labeled, CPCV-validated, gated vs hold net
   of costs. Cannot underperform "always act" if gated; plausible win is **cost-drag
   reduction**, not return. ml-trading[2][15].
9. **B&H + crowned-strategy blend (risk overlay)** — the one recurring *positive* in the
   evolution/LLM literature: a hold+active blend cut drawdown ~50% in the honest crypto
   study. A drawdown tool, not alpha; needs its own baseline-divergence e2e test (per
   CLAUDE.md, the v3-vol-overlay-noop precedent). llm-and-evolution[15], strategies.
10. **Regime-flat overlay with hysteresis** — a jump-model bull/bear detector that
    de-risks the coin to cash in bear regimes, with an explicit **switching penalty**
    (the jump model beat the HMM purely by switching less) + OOS-CV params + a ~2-week
    detection-lag model. Maps onto our existing long/flat decision. strategies[17],
    ml-trading[6].

### P2 — Generative synthetic test data (the one DL lane for us)

11. **Diffusion-based market-path generator** to enrich the moving-block bootstrap with
    novel, stylized-fact-preserving regimes (fat tails, vol clustering). Diffusion is
    preferred over GANs (no mode collapse); validate with TimeGAN's discriminative +
    TSTR scores; **hard guardrail: must reproduce our coin's stylized facts and must NOT
    leak the held-out test path.** deep-learning[16][9][19].

### P2 — Narration & future structural edge

12. **Keep LLMs on the narration rail, off the alpha rail** — our existing "why this
    one" seam (ADR-0064) is the *correct* use. Ground narration in the **actual gated
    numbers** (constrain to templated explanation of real metrics; LLM free-form
    rationales hallucinate); a small local open LLM (FinGPT/LoRA-style) is the realistic
    substrate; add a lightweight risk-audit (provenance/recency/prompt-injection) for any
    news layer. llm-and-evolution[1][22][23][9].
13. **Funding-rate / basis carry is the highest-Sharpe crypto edge in the literature** —
    market-neutral (long spot / short perp), ~6 Sharpe, non-predictive. Aligns with our
    existing perp-basis work (ADR-0051 §D6.9/D6.10); needs a perp+margin+funding model
    and short support to deploy. Stress-test it (crashes in liquidity/vol stress), don't
    sell it as free yield. strategies[9][18].

---

## 3. What NOT to do (hype to avoid)

- **No deep nets as the alpha engine** — DLinear ≥ Transformers; classical ML +
  features ≥ deep learning on tabular finance; deep RL overfits one regime.
  deep-learning[8], ml-trading[23].
- **No un-budgeted factor/param/GP/LLM search** — automated alpha mining is
  industrialized data-snooping; charge the search budget against significance and use a
  once-only OOS window, or expect the gate to (correctly) reject it.
  llm-and-evolution[10][11][12][21].
- **Never treat IC / accuracy / a single-window Sharpe as a verdict** — only
  equity-vs-hold net of costs over a path distribution counts. ml-trading[12], deep-learning[13][23].
- **Don't add macro/social-media features expecting gains** — they degraded crypto
  prediction; keep features small + technical, regularize (shrinkage) when combining.
  ml-trading[17][10][18].

These directly validate the **expected-null** framing of our fresh-channel probes
(ADR-0072 DVOL, ADR-0073 macro): honest coverage, not asserted alpha.

---

## 4. Map to the codebase

| Roadmap item | Touches | Nature |
|---|---|---|
| P0 DSR/PBO/SPA scorecard | `bakeoff/{robustness,rank}.rs` + report | additive; FROZEN bands untouched |
| P0 data-driven block length | `bakeoff/bootstrap.rs` | confirm/expose (ADR-0063 already cites Politis–White) |
| P1 leakage/seven-sins audit | new tests + report | discipline gates |
| P1 synthetic no-alpha gate test | `bakeoff` tests | standing regression |
| P1 fee-sweep + turnover | ranking output | reporting |
| P1 meta-labeling filter | new candidate + composed-sweep engine (ADR-0069) | gated experiment |
| P1 B&H+active blend / regime-flat | new overlay + day-1 divergence e2e | gated experiment |
| P2 diffusion test-data gen | new tool feeding the bootstrap | research spike |
| P2 LLM narration grounding | `agent::narration` (ADR-0064) | hardening |
| P2 funding/basis carry | perp+margin+funding engine (ADR-0051 §D6.x) | large, future |

---

## 5. Pointers

- Per-topic detail: `research/<topic>/knowledge.md` (themes, hold-up-vs-hype, paper map).
- Full ledgers (Title · Year · Source · % read · Summary · Relevance):
  `research/<topic>/papers.md`.
- Progress + resume protocol: `research/PROGRESS.md`, `research/README.md`.
- Suggested next batches (depth gaps the agents flagged): RAG-for-finance specifics;
  post-2025 honest LLM benchmarks (LiveTradeBench, CryptoBench); PDF-text deep reads of
  the ~18 papers logged at abstract level due to paywalls/binary PDFs.
