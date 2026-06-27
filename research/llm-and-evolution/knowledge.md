# Knowledge — LLM Applications + Evolutionary / Genetic Methods in Trading

_Synthesis of the `llm-and-evolution` ledger (23 papers). Payoff focus: where
LLMs genuinely help OUR advisor (the "why this one" narration seam,
news/context, research assistance) vs. where they're hype; and whether
evolutionary / symbolic-regression alpha-mining is just industrialized
overfitting that our FROZEN robustness gate (1000-path moving-block bootstrap
vs buy-and-hold, net of costs, weakest-link verdict) would correctly reject._

_Status: COMPLETE for batch 1 — 23 papers (LLM side [1-9,18-23] + evolution
side [10-17] + LLM-evolution hybrids [10,16,20,21]). Both halves covered._

---

## Executive verdict (the one-paragraph answer to the task)

**LLMs add genuine value to our advisor only on the *language* rail — narration,
news/context ingestion, and research assistance — and add nothing reliable on
the *alpha* rail.** This is not opinion; it is the consistent finding across the
flagship finance LLMs ([1][5] make zero PnL claims), the holistic benchmark
([7]: LLMs strong at text, weak at forecasting), and the cleanest
contamination-free reality check ([22] StockBench: 14 frontier models barely
edge a 0.4% buy-and-hold in a benign window *without costs*, and **all**
underperform B&H in downturns). **Evolutionary / genetic / symbolic-regression
alpha mining is, statistically, industrialized data-snooping**: from the
foundational GP study ([11], 1999 — evolved rules don't beat B&H out-of-sample
after costs) to modern LLM/RL/GP miners ([10][12][16][21]), the search is made
fancier but the multiple-testing problem that dooms it is unchanged, and the
honest crypto replication ([15]) shows optimized params lose to *random* params
87-92% of the time and merely match B&H. **Our FROZEN gate is the correct,
time-tested defense and should expect to reject essentially all of this output.**

---

## Key themes

### LLM side
1. **"Finance LLM" ≠ "profitable LLM."** Flagship finance LLMs validate
   *language* tasks (sentiment, NER, QA), not returns. BloombergGPT [5] and
   FinGPT [1] make **no PnL claims**; FinBen [7] (which *adds* trading/
   forecasting tasks) concludes LLMs are strong at text, weak at forecasting.
2. **LLM trading-agent papers report implausibly good results on tiny,
   favorable test beds.** FinMem [2] (5 hand-picked tickers, one bear window),
   TradingAgents [3] (3 stocks, 5 months, Sharpe ~8, MDD <1%), FinAgent [4]
   (6 assets, single window). These trip *every* alarm our gate is built for.
3. **Look-ahead / memorization contamination is the dominant threat.** LLM
   training cutoffs overlap test windows in [2][3][4]; GPT recalls historical
   prices/headline dates. [6] shows the effect is real and tangled with a
   "name-distraction" effect, and offers anonymization + post-cutoff testing.
   [22] is the gold standard: it *deliberately* tests post-cutoff to kill leakage.
4. **Transaction costs are routinely hand-waved** ([3][4][14][22] none/vague) —
   yet costs are what kill active strategies in our validated thesis.
5. **Stock-tuned LLM agents do NOT transfer to crypto.** FinAgent [4]
   *under*performs baselines on ETHUSD and admits its tools are stock-specialized
   — a direct caution for our single-coin crypto advisor.
6. **Even the careful pro-signal evidence is fragile.** Lopez-Lira & Tang [9]:
   the big hit-rate is on the *non-tradable* initial reaction; the tradable
   drift lives in *illiquid small-caps* and **decays as LLM adoption rises**.
7. **The best reality check confirms the thesis.** StockBench [22]: frontier
   LLMs barely beat a 0.4% B&H in a flat/up window (no costs), and **all** lose
   to B&H in downturns. QA skill ≠ trading skill.

### Evolution side
8. **Automated alpha mining is industrialized data-snooping.** Whether the
   generator is GP [11], RL [12], LLM [10], or LLM+MCTS [21], an iterative
   search that selects factors/rules by performance over the dataset
   manufactures in-sample winners. The 1999 foundational GP study [11] found
   evolved rules **don't beat buy-and-hold out-of-sample after costs**; modern
   papers re-run it with fancier search and *drop the honest controls*.
9. **Modern miners report IC/accuracy, not PnL-net-of-costs.** AlphaGen [12]
   doubles IC (0.045→0.085) — still a weak correlation — with no costs, no
   quantitative B&H comparison. IC is a *screen*, never a *verdict*.
10. **The honest controls are known and old.** Allen-Karjalainen [11] used
    train/select/**test** splits + costs in 1999. The flashy papers
    ([3][4][10][12][14][21]) routinely omit held-out test, costs, and/or
    multiple-testing correction.
11. **Our gate is independently reinvented by the honest papers.** The
    double-out-of-sample crypto study [15] uses a 1000-path bootstrap vs random
    params, costs throughout, once-only OOS — and concludes optimized active
    params do NOT beat B&H and lose to *random* params 87-92% of the time. That
    is our thesis + our gate, arrived at externally.
12. **The crypto GA papers bracket the spectrum:** [14] (no costs, no OOS, no
    B&H, +550% — the anti-pattern) vs [15] (rigorous, honest, matches B&H).
    Same method family, opposite credibility — *methodology is everything*.
13. **Even careful neuroevolution loses.** NEAT over multi-indicator inputs on
    22 years of S&P 500 [17] **underperforms B&H by ~9 pts before costs**.
14. **The alpha-factor paradigm is cross-sectional and institutional.** Alpha101
    [13]: industrial alphas are individually *weak*, short-horizon (~0.6-6.4 day
    holds), low-correlation (~16%), useful only *combined across hundreds* with
    heavy turnover — a regime structurally inapplicable to single-coin retail.

### Cross-cutting
15. **The one recurring *positive*: B&H + active as a portfolio cuts drawdown**
    ([15], echoed by [13]'s diversification framing). Risk management, not alpha
    — and a testable advisor feature.
16. **LLMs introduce non-PnL risks the backtest gate can't catch** ([23]):
    hallucinated rationales, stale/false news, adversarial prompt manipulation.
    Needs a separate risk-aware audit layer.

## Methods / findings that hold up (and which don't)

**Hold up (genuine value):**
- LLM *language* competence on finance text — sentiment, extraction, QA,
  summarization — is real and benchmark-validated ([1][5][7][18]).
- **Tool-augmentation** ([4][20]): an LLM that *calls* vetted indicators/
  strategies, or orchestrates tools, rather than inventing signals.
- **Human-in-the-loop research assistance** ([20]): LLM interprets intent +
  RAG over literature, human judges — a manual analogue of our gate.
- **Interpretability of symbolic formulas** ([16][21]): readable logic aids
  *explanation* (our narration goal), independent of any alpha claim.
- **Honest evaluation methodology** ([11][15][22]): train/select/test splits,
  double-out-of-sample, post-cutoff windows, bootstrap-vs-random, costs always.
- **Factor-decay awareness → periodic re-evaluation** ([16]): a crowned edge is
  not stationary; re-baking cadence is worth considering.
- **B&H + active portfolio for drawdown reduction** ([15]).

**Don't hold up (hype):**
- "LLM/GP/RL agent beats buy-and-hold" — every spectacular instance is small-N,
  single-window, cost-free, and/or cutoff-overlapping ([2][3][4][12][14]).
- Sharpe ~8 / sub-1% drawdown ([3]) or +550% scalping with no costs ([14]) are
  contamination/overfitting signatures, not edges.
- IC/accuracy as a stand-in for tradeable PnL ([12][16][21]).
- Automated alpha mining as "discovery" — it's multiple testing ([10][11][12][21]).

## Actionable takeaways for our advisor

1. **Keep LLMs on the narration/context rail, off the alpha rail.** Our existing
   "why this one" seam is the *correct* use. The robustness gate — never an LLM —
   decides what to trade. ([1][5][7][22] all support this boundary.)
2. **Ground all LLM narration in the actual gated numbers.** Per [23], free-form
   LLM rationales hallucinate; constrain narration to templated explanation of
   the crowned strategy's real metrics. Consider a bull/bear/risk framing
   borrowed from the multi-role pattern ([3]) and human-in-the-loop ([20]).
3. **If we add an LLM news-sentiment feature, gate it AND guard contamination.**
   Test only on post-training-cutoff data and/or anonymize tickers ([6][9][22]);
   model transaction costs; expect a fragile, decaying, cost-sensitive signal
   ([9]) — likely rejected by the gate, which is fine.
4. **Never run an un-budgeted factor/param search.** Any automated search
   (GP/RL/LLM) is a multiple-testing engine ([10][11][12][21]); if we ever do
   it, charge the search budget against significance (deflated Sharpe / PBO —
   see backtesting topic), use a once-only OOS window ([15]), and run survivors
   through the cost-aware bootstrap-vs-B&H gate. Expect ~null results.
5. **Treat IC/accuracy as a screen, never a verdict** ([12][16][21]) — only
   equity-vs-B&H net of costs counts ([11][15] are the precedents).
6. **Don't import stock-trained agents/factors wholesale for crypto** ([4][13]).
7. **Consider two genuinely useful borrowings:** (a) a B&H+active *risk* overlay
   that cuts drawdown ([15]); (b) a periodic *re-bake* cadence acknowledging
   factor/edge decay ([16]).
8. **A small open finance LLM (FinGPT/FinLlama-style, LoRA)** is the realistic,
   cheap, local substrate for narration/sentiment ([1][18]) — the closed 50B
   BloombergGPT [5] is irrelevant to us.
9. **Add a lightweight LLM risk-audit** for the narration/news layer
   (provenance, recency, no-hallucination, prompt-injection resistance) per [23].

## Open questions / things worth testing in our app

- Does an LLM-generated "bull/bear/risk" narration around our crowned pick
  measurably improve operator *decision quality*? (UX experiment, not PnL.)
- Does a **B&H + crowned-strategy blend** reduce drawdown on our coins without
  killing return, the way [15] reports? (Directly runnable in our engine as a
  risk feature; would need its own baseline-divergence e2e test per CLAUDE.md.)
- Is there a **re-bake cadence** ([16]) at which a previously-crowned pick should
  be re-evaluated as its edge decays? Does our forward paper-run already show decay?
- If we ever trial an LLM news-sentiment feature: build it post-cutoff +
  anonymized ([6][9]); does it survive our gate net of costs? (Strong prior: no.)
- Can a small local open LLM ([1][18]) narrate at acceptable cost/latency?
- Would running a deliberately-contaminated vs post-cutoff backtest on our own
  data (à la [22]) be a useful *demonstration* to operators of why the gate matters?

## Paper map (claim → supporting [N])

- Finance LLMs validate language, not PnL → [1] FinGPT, [5] BloombergGPT, [7] FinBen
- LLMs strong at text, weak at forecasting → [7] FinBen, [8] FinSeer (~54% acc), [19] survey
- LLM trading agents over-fit tiny favorable test beds → [2] FinMem, [3] TradingAgents, [4] FinAgent
- Look-ahead / memorization contaminates LLM backtests → [6] Glasserman-Lin, [9] Lopez-Lira; clean test → [22] StockBench
- LLM "win" over B&H is tiny, cost-free, and inverts in downturns → [22] StockBench
- Careful pro-signal evidence is fragile/decaying/illiquid → [9] Lopez-Lira & Tang
- Costs hand-waved in agent/GA papers → [3], [4], [14], [22]
- Stock→crypto transfer fails → [4] FinAgent
- Automated alpha mining = industrialized data-snooping → [10] EFS, [11] Allen-Karjalainen, [12] AlphaGen, [21] Alpha Jungle
- Foundational GP rules don't beat B&H OOS after costs → [11] Allen-Karjalainen (1999)
- Miners report IC/accuracy not PnL-net-of-costs → [12] AlphaGen, [16] AlphaForge, [21] Alpha Jungle
- Honest crypto replication: optimized params ≈ B&H, lose to random → [15] double-OOS
- Crypto GA anti-pattern (+550%, no costs/OOS/B&H) → [14] CGA-Agent
- Neuroevolution underperforms B&H even before costs → [17] NEAT
- Industrial alphas are weak/short/cross-sectional/institutional → [13] Alpha101
- B&H + active reduces drawdown (risk, not alpha) → [15], [13]
- Factor/edge decay → dynamic re-weighting / re-bake → [16] AlphaForge
- Reusable LLM patterns (tool-aug, debate, layered memory, human-in-loop, RAG) → [4], [3], [2], [20]
- LLMs add non-PnL risks (hallucination, stale data, prompt-injection) → [23] risk-audit
- Field's own verdict: "modest outcomes so far," no claim AI beats passive → [19] survey
