# Applications — research → next big steps (analyst + architect entry point)

_Prepared 2026-06-28 from the completed 900-paper research program (9 topics ×
100, deep-read). For each research folder, a per-topic **application doc**
translates the literature into project decisions: **Summary · Possible solutions ·
Relevance · Advantages · Problems & challenges · Concrete next steps (with
codebase locations + P0/P1/P2) · Open questions · What-not-to-do.**_

> **Goal these serve (unchanged):** a **framework for trading with traceable and
> plausible trading** — the single-coin crypto advisor (paper/sim, not-advice)
> that bakes off every strategy, ranks under a FROZEN bootstrap robustness gate
> (FRAGILE can't crown; buy-and-hold always the benchmark), and sells **measured
> honesty, not asserted alpha.** Validated thesis: no active strategy robustly
> beats holding net of costs.
>
> **These are PREPARATION, not decisions.** The analyst + architect read them to
> decide what becomes a feature. Every "next step" is grounded in the real
> codebase and named additive/anchor-safe where it touches the frozen gate.

---

## The 21 application docs (by folder)

| Folder | Doc | One-line |
|---|---|---|
| **backtesting** | [overfitting & multiple-testing](backtesting/application-overfitting-and-multiple-testing.md) | the P0 selection-bias scorecard (DSR/PBO/MinBTL/N_eff) — with the `MAX_SWEEP_CONFIGS=24` reality-check |
| | [cost & impact modeling](backtesting/application-cost-and-impact-modeling.md) | realistic fees/slippage/turnover/decay; costs are the decision variable |
| **data** | [splits, leakage & CV](data/application-splits-leakage-cv.md) | purged/embargoed CPCV; forward paper-trade alone is insufficient → pair with PBO/DSR |
| | [synthetic & Monte-Carlo](data/application-synthetic-and-monte-carlo.md) | generators smooth tails → keep the model-free block bootstrap; generators research-only |
| | [PIT, labeling & stationarity](data/application-pit-labeling-stationarity.md) | as-of `PitSeries` discipline, triple-barrier labels, frac-diff; the F5 skew fix |
| **strategies** | [TA-efficacy & selection-bias](strategies/application-ta-efficacy-and-selection-bias.md) | the data-snooping arc = external validation of our thesis + the DSR recipe |
| | [execution & sizing rules](strategies/application-execution-and-sizing-rules.md) | VWAP/IS background; how rule families inform the forward-plan + "trade-less" |
| | [factor replication & the counter-thesis](strategies/application-factor-replication-and-the-counter-thesis.md) | governs claim language: "rarely robustly beats hold, we TEST it" — not "TA never works" |
| **ml-trading** | [LdP pipeline & meta-labeling](ml-trading/application-ldp-pipeline-and-meta-labeling.md) | triple-barrier, meta-labeling as a cost-drag filter, MinBTL/SR₀ veto |
| | [classical ML & baselines](ml-trading/application-classical-ml-and-baselines.md) | what-not-to-chase + how to test honestly (baseline-vs, purged CV, costs) |
| **deep-learning** | [forecasting & significance](deep-learning/application-forecasting-and-significance.md) | simple ≥ Transformers; the significance lessons that motivate the DSR layer |
| | [deep-RL & hedging](deep-learning/application-deep-rl-and-hedging.md) | mostly avoid; input-parsimony as a robustness property |
| **evolution** | [automated strategy search](evolution/application-automated-strategy-search.md) | the highest-overfitting-risk idea — footgun; mostly "don't, here's how if" |
| | [anti-overfitting & search discipline](evolution/application-anti-overfitting-and-search-discipline.md) | the durable export: exact DSR + MinBTL formulas feeding P0 |
| **llms** | [narration & agents](llms/application-llm-narration-and-agents.md) | LLMs on the narration rail (F9) + reflection surface; never the ranking |
| | [time-series foundation models](llms/application-llm-timeseries-foundation-models.md) | answers the open question: no crypto return-alpha; language ablates out; vol-only |
| **risk-and-sizing** | [vol-targeting & drawdown overlays](risk-and-sizing/application-vol-targeting-and-drawdown-overlays.md) | the most actionable: reposition the vol overlay as risk-not-Sharpe; drawdown+restart |
| | [position & bet sizing](risk-and-sizing/application-position-sizing-and-bet-sizing.md) | Kelly is hopeless on a no-edge coin; "shrink hard, control risk, hold" |
| **crypto-market-structure** | [exogenous-signal arms](crypto-market-structure/application-exogenous-signal-arms.md) | candidate-vs-dead-end triage; no signal clearly survives costs at our horizon |
| | [volatility, regimes & overlays](crypto-market-structure/application-volatility-regimes-and-overlays.md) | vol forecastable, direction random-walk → overlays not return-timing |
| | [data integrity](crypto-market-structure/application-data-integrity.md) | fabricated volume/OI/depth → which venues/metrics to trust |

---

## Convergent priorities (what nearly every folder pointed at)

### P0 — the selection-bias / overfitting scorecard (surfaced #1 in 6 of 9 folders)

The bootstrap gate tests **each curve's** robustness but does **not** correct for
the multiple-testing bias of crowning the **best of N** swept strategies. The fix
is an **additive, report-first scorecard** next to the verdict in
`crates/backtest/src/bakeoff/{rank.rs,robustness.rs}` — **N_eff → DSR → MinBTL →
(later) PBO** — every input already computed each run. It is the literal
embodiment of "traceable & plausible": each crown ships an auditable "we tried
N_eff effective strategies; here's the deflated confidence; here's why it
was/wasn't crowned."

**Two code-grounded reality-checks the agents found (override the SYNTHESIS's
scarier framing):**
1. **`MAX_SWEEP_CONFIGS = 24`** (`bakeoff/sweep.rs:62`) → raw N is tiny, N_eff is
   single-digit, the haircut is **modest not "gut everything,"** and the SYNTHESIS's
   "M>T ⇒ must-cluster-first" mandate **likely does not apply to us** (T ≫ 24).
   **MinBTL bites hardest at our scale** — the cheapest honest "can't crown with
   confidence" veto.
2. **Two SYNTHESIS P0 items are already done:** Politis–White block length is
   computed per-series (`bootstrap.rs:139`) and the resample is already circular
   (`bootstrap.rs:265`). The only gap is *logging* the block length.

**Ship order:** MinBTL + DSR + N_eff (closed-form, report-only) first — no plumbing,
no frozen-band/anchor touch. PBO/CSCV + BBC-CV need an **enabler**: capture the
per-config bar-return matrix in the sweep (`CandidateResult` stores only
per-candidate equity today). _Don't overscope — see each backtesting/data/evolution
doc._

### P1 — concrete, mostly-risk-shaping builds

- **Drawdown-control overlay** (new — none exists): cushion multiplier + **high-water-mark
  restart** (BTC: max-DD 72%→20% holding Sharpe 1.52; collapses to −0.04 *without*
  the restart). `crates/strategy/src/drawdown_control_overlay.rs` + day-1 divergence e2e.
- **Reposition the shipped vol-targeting overlay** as a **risk tool, not a Sharpe
  tool** (crypto inverse leverage effect → no Sharpe gain): loose+slow EWMA, no-trade
  band, de-risk-only. `crates/strategy/src/vol_targeting_overlay.rs`.
- **Harden the F9 narration faithfulness check** (verbatim-number match + extend the
  banned-phrase list to prediction/causation verbs). `crates/agent/src/narration.rs`.
- **Cost-model hardening + a metric-specific venue-trust map** (Kraken/HTX-only OI,
  no mid-price fills on spoofable depth, wider effective spread). `crates/cost`, gate.
- **Funding-sign froth arm** (`v0.funding_froth`) — the one genuinely probe-worthy
  exogenous arm; reuses the existing `basis_data`/`funding_data` seam; **expect
  FRAGILE** → honest coverage, not a win.
- **Meta-labeling / cost-aware "trade-less" filter** — plausible win is cost-drag
  reduction, **not return** (doesn't beat B&H after Holm).

### Correctness fixes (no new capability, just honesty)

- **F5 forward-fidelity skew** (re-flagged independently): the forward paper-trade
  runs an **SMA proxy** for non-SMA crowned picks → the forward number measures a
  *different* strategy than the one crowned. Fix = reuse the bake-off's
  ComposedStrategy-from-TOML in `build_registry_for`.
- **"Returns, not price levels; vs a random-walk baseline"** as a standing
  test-data-discipline check — inoculates against the entire price-level-trap class.

### Dead ends / do-NOT-build (the honest negative space)

- **Automated alpha search** (GA/GP/symbolic-regression/LLM-code-evolution) — industrialized
  data-snooping; only behind walk-forward + complexity-penalty + budget-charged-significance
  + pre-registration; expected null on a single coin.
- **A TSFM/LLM in the ranking** — no gate-credible crypto return-alpha; the "language"
  ablates out; vol is the only defensible numeric target (and only for the *sizing* overlay).
- **On-chain (MVRV/SOPR/netflows) + sentiment (Fear & Greed) exogenous arms** —
  PIT-infeasible / endogenous / fail Granger; documented dead ends.
- **Deep nets / DRL as the alpha engine** — simple linear ≥ Transformers; DRL profits
  are overfitting; accuracy ≠ alpha.

---

## How to use this

1. Start here → open the per-folder docs for the areas you're scoping.
2. The cross-topic distillation + the exact P0 formulas live in
   [SYNTHESIS.md](SYNTHESIS.md); the full per-topic detail in each
   `research/<topic>/knowledge.md`; the 900-paper ledgers in `research/<topic>/papers.md`.
3. Each doc's §6 (next steps) + §7 (open questions) are written to be lifted
   straight into analyst scoping / architect ADRs. The recurring architect call:
   *does a DSR/PBO crown-eligibility predicate count as "additive" to the FROZEN
   gate, or ship report-only?* (raised in several docs).
