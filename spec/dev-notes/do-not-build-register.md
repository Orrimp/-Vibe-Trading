---
slug: do-not-build-register
status: reference
owner: analyst
updated: 2026-07-11
---

# Do-not-build register — the authoritative settled dead-ends

> **What this is.** The single authoritative "these are settled dead-ends, here is
> why, do NOT reopen" reference for this product (the single-coin crypto paper
> advisor). R3-4a of the v3 "prove it's done" close-out. Project memory shows the
> same off-track ideas — ML/forecasting, multi-coin, a return predictor in the
> ranking, automated alpha search, LLM-as-trader, on-chain/sentiment arms, live
> trading — get **re-proposed as "gaps" every few sessions**, each costing real
> cycles to re-litigate. When one of these surfaces again, point at the matching
> row here instead of re-deriving the rejection.
>
> **What this is NOT.** Not a gate change, not code, not a status change. It is a
> **consolidation + citation** of decisions already made — in `spec/product.md`
> (the IS/ISN'T boundary), the 900-paper research program (`research/SYNTHESIS.md`
> §3, `research/APPLICATIONS.md` "Dead ends / do-NOT-build"), `spec/v2/v2-analysis.md`
> §3 (OT-1..OT-10), `spec/dev-notes/post-v2-scoping-2026-07-09.md` §4, and dated
> operator decisions. Nothing here is new judgment; every row cites its source.

**How to read a row.** Each entry has: **the tempting idea** (the exact framing it
usually gets re-proposed as) · **the guardrail/evidence that kills it** (a product
principle OR a research finding OR a dated operator decision) · **"if re-proposed,
point here"** (a one-line rebuttal + citation).

**Scope note — this is a "settled-dead" register, not a "nothing may ever change"
decree.** Exactly one thing is genuinely still open (structural point-in-time/as-of
data) — see the final § *What IS still legitimately open*. Everything in the tables
below is closed.

---

## Group A — Alpha-chasing (the bright lines the thesis exists to hold)

The product's validated thesis (900 papers, 9 independent reviews, deep-read at
primary-source depth): **on the current deep-liquidity market era (2023+), no
active strategy robustly beats buy-and-hold net of costs on a single liquid
coin** — the modal outcome on every window the advisor can actually run. The
differentiator is **measured honesty, not asserted alpha.** Every idea in this
group is a way of quietly re-becoming the over-claiming alpha framework the
research exists to deflate.

**Why the era clause, and why it is a strength (efficiency migration).** The P2
corpus-expansion verdict re-run (`../v3/advisor-corpus-expansion/reports/backtest-2026-07-10-p2-verdict-rerun.md`,
tester PASS) ran this same frozen gate back across the older, thinner-liquidity
eras and found REAL, cost-annex-surviving active edges in the
early market (2017-18, 2020, 2021-22) — and measured their **decay to ~zero in
the current era** (2023-24: 0/1 ActiveWins; 2025-26: 2/10, both marginal). That
is the textbook efficiency-migration / anomaly-decay pattern the research corpus
*predicted* (McLean–Pontiff, `../../../research/SYNTHESIS.md` §62: decay largest
for the cheapest-to-arbitrage large caps = BTC/ETH). The machinery did not merely
fail to find alpha — it **positively detected real historical edges and then
detected the boundary where they died**, and reported it. That is a stronger
honesty claim than "we looked and found nothing," and it is the credibility story,
not a wobble. Qualified always by: **survivor-of-survivors** (the 2017-18 subset
is BTC/ETH/BNB — the three extreme eventual survivors of a top-10 now mostly
near-zero; those early edges were NOT knowably harvestable ex-ante) and **old-era
cost realism as a stated limit** (flat-8bps AND vol-scaled likely understate
2017-20 frictions — depth, impact, outages, withdrawal risk — so old-era crown
Sharpe margins are upper bounds; the direction is conservative for today's
verdict).

**This does NOT reopen Group A.** The old-era edges are unreachable in any window
the advisor runs (every lookback ends at "now"; there is no time machine), and
chasing them as live strategy is the *exact* alpha-chasing A-3 (automated search)
crossed with survivorship mining that this group forbids — "re-enable
donchian_floor because it won in 2020" is a settled dead-end, not a gap. Every
conclusion in the tables below stands unchanged: the forward advice is identical
(on the current era, hold), and nothing here is a licence to add alpha surface.
The finding sharpens the thesis's *scope*; it does not soften a single row.

| # | Tempting idea (usual re-proposal framing) | Guardrail / evidence that kills it | If re-proposed, point here |
|---|---|---|---|
| A-1 | **A return/direction predictor in the ranking** — "put a TSFM / deep-net / LSTM / LLM forecaster in the bake-off so we crown on predicted return, not just backtested rules." | **NO prediction in the ranking** is the product's one bright line (product.md D2; v2-analysis OT-4). Research is decisive: no gate-credible crypto return-alpha; the best peer-reviewed crypto economic test gives BTC ~1.0 Sharpe ≈ B&H; the "language" ablates out; **lower forecast MSE does not mean more profit** (price-level persistence trap); FinCast (the one model actually trained on crypto) reports only price-level MSE, never PnL/Sharpe/B&H. | A forecaster may ONLY feed a downstream **de-risk-only sizing overlay** (vol), NEVER the crown. `research/SYNTHESIS.md` §3–§4; `research/llms/application-llm-timeseries-foundation-models.md` §8; `research/deep-learning/application-forecasting-and-significance.md` §8; product.md D2; v2-analysis §3 OT-4. |
| A-2 | **Deep nets / DRL as the alpha engine** — "LSTM/GRU/Transformer/TCN/TFT or a deep-RL policy will find the edge simple rules miss." | Simple linear **≥** Transformers on a single series (shuffle test: DLinear degrades 27%, transformers ~0% — they never used time order); the deep edge is purely **cross-variate** → a single coin is exactly where deep models add nothing; DRL profits are overfitting; accuracy ≠ alpha. **Already TRIED-AND-RETIRED here** (the v2.5 4-phase DL programme TCN+PatchTST reached terminal F4, no +0.10 Sharpe-delta; XGBoost cheap-classifier foreclosed). | Retired by tested conclusion, not a backlog gap. `research/SYNTHESIS.md` §3; `research/deep-learning/application-forecasting-and-significance.md`; `spec/dev-notes/qlib-feature-gap-2026-06-17.md` rows #4–#6, #12–#15; product.md § Strategy library. |
| A-3 | **Automated alpha / parameter search** — "run a GA / genetic-programming / symbolic-regression / LLM-code-evolution loop to discover strategies or tune parameters automatically." | **NO automated search** — it is the product's own threat model. The single **highest-overfitting-risk idea in the 900-paper program**: industrialized data-snooping by construction; the in-sample winner is *negatively* correlated with OOS return. Our **FIXED pre-registered slates** are the standing defense. | Expected null on a single coin; allowed ONLY behind the full guard stack (walk-forward + once-only OOS + DSR/MinBTL budget-charging + pre-registration) and *expecting* a null — and even then the deliverable is the *protocol + null verdict*, not a winner. `research/evolution/application-automated-strategy-search.md` §1; `research/SYNTHESIS.md` §3; v2-analysis §3 OT-3; post-v2-scoping §4. |
| A-4 | **LLM-as-trader / multi-agent "debate"** — "let an LLM (or a bull/bear multi-agent panel) make or arbitrate the trade decision." | **Narration-only bright line** (F9/ADR-0064) + read-only reflection (ADR-0074). Every "LLM beats B&H" result is leakage / no-cost / single-window / passive style-harvesting (FINSABER: B&H beats FinMem/FinAgent on 3 of 4 stocks; KTD-Fin: negative selection alpha for 9/10 models). The multi-agent "debate" pattern is exactly the configuration the refutations target. | LLMs stay on the **narration + read-only-reflection** rail; at most a narration-*structuring* device (bull/bear prose), never a decision mechanism. `research/llms/application-llm-narration-and-agents.md` §8; `research/SYNTHESIS.md` §4; product.md § LLM role; v2-analysis §3 OT-5. |
| A-5 | **New signal primitives (ATR / VWAP / new indicators) added *to find edge*** — "add more indicators as bake-off arms to chase an edge the current four miss." | **NO alpha search; pre-registration only.** New primitives are fine as *pre-registered coverage* (an honest, expected-null falsifier slate — this is how OBV/Donchian/volume-breakout arms were added). Adding them *chasing alpha* is scope-creep toward search and adds arm surface for **~zero credibility gain post-v2** (the field is already modal-`BenchmarkWins` across long/combos/shorts/breakout/volume/OBV/IV/macro). | Only ever as pre-registered coverage, never alpha-chasing; a backlog one-liner at most, not a v3 theme. ATR/VWAP are confirmed **not built** and expected-null. v2-analysis §3 (new-primitive note), §5 (DROP/backlog); post-v2-scoping §4; product.md changelog (signal-library-expansion follow-ons). |

---

## Group B — Scope-expansion (breaks the single-coin, paper-only product boundary)

The product is deliberately **one coin, one budget, paper/sim only** (product.md
§ What this product IS / IS NOT). These ideas do not just add a feature — they
change *what the product is*, and each has an independent research or operator kill.

| # | Tempting idea (usual re-proposal framing) | Guardrail / evidence that kills it | If re-proposed, point here |
|---|---|---|---|
| B-1 | **Multi-coin / "rank many coins, pick the best" / a basket portfolio** — "the operator said *which coin* — so scan a universe and pick/allocate across the best coins." | **Single-coin only; NOT a multi-asset portfolio manager** (product.md § IS NOT). Single-coin *selection* the user already makes (pick XRP vs BTC, one at a time) is IN scope and unchanged; a cross-sectional rank-coins/basket is a **TRACK CHANGE**. The surviving factor edges (value/momentum/quality/low-risk) are **cross-sectional — they need a universe and are NOT harvestable on one coin**; worse, the diversification that would justify a basket **fails in crypto** (BTC–ETH ρ>0.85 in stress; "cash is the only real diversifier"). | A separate product track with its own gate calibration (cross-sectional N_eff, contagion-aware risk), explicitly named — NOT an "additive arm." `research/strategies/application-factor-replication-and-the-counter-thesis.md` §1; `research/risk-and-sizing/application-position-sizing-and-bet-sizing.md` §1; product.md § IS NOT; v2-analysis §3 OT-1; post-v2-scoping §4. |
| B-2 | **Live trading / real orders / margin / KYC** — "wire it to a real exchange so the €200 is real; add a live-exec client." | **PAPER / SIM ONLY** — standing operator constraint. Live execution was **built and then removed from `main` 2026-06-12** by explicit operator direction ("no live trading for a long time"); the whole `crates/exec/src/live/` block, secrets plumbing, and the live reconciler were reverted byte-exact. | Do NOT re-propose. Any maker/limit cost mode stays a *simulation assumption*, clearly labelled, never a step toward execution. `spec/dev-notes/live-trading-removed-2026-06-12.md`; product.md § IS NOT + § Constraints; project memory "no live trading"; v2-analysis §3 OT-2. |

---

## Group C — Infeasible / dishonest data (fails PIT, endogeneity, or Granger)

The product's whole credibility rests on an **honest** result. A signal that cannot
be reconstructed point-in-time, or that is endogenous to price, or that fails
Granger causality, silently manufactures phantom alpha. These are documented dead
ends, not "untried feeds."

| # | Tempting idea (usual re-proposal framing) | Guardrail / evidence that kills it | If re-proposed, point here |
|---|---|---|---|
| C-1 | **On-chain arms (MVRV / SOPR / exchange netflows) as bake-off signals** — "add on-chain valuation/flow signals; they're crypto-native alpha." | **PIT-infeasible / endogenous.** The pre-committed on-chain **hard-stop already fired 2026-06-08**: exchange net-flows are point-in-time-infeasible (CryptoQuant disclaims PIT accuracy; no free immutable past-only series), and the cleaner-PIT stablecoin-supply fallback is FRAGILE (sign flips year-over-year under the same live-bar). The real magnitude is tiny and sub-horizon ($100M inflow → +0.065% BTC *next hour*, inside round-trip costs). | Documented dead end; do NOT spend paid-feed budget. On-chain *valuation* (MVRV) is at most a future, **PIT-gated** research spike, never a v2/v3 arm. `spec/dev-notes/onchain-netflow-spike-2026-06-08.md`; `research/crypto-market-structure/application-exogenous-signal-arms.md` §8; `research/SYNTHESIS.md` §2 item 16; product.md § Why this is honest; v2-analysis §3 OT-6. |
| C-2 | **Sentiment arms (Fear & Greed / social-media / macro-as-return-signal)** — "add sentiment or macro features to the ranking for a prediction edge." | **Fail Granger causality and degrade crypto prediction.** Endogenous, non-PIT, style-harvesting. Macro is at best a *vol/regime* overlay — and even that fails OOS outside rate-cut regimes. In-scope data is only the structural PIT/as-of gap (§ open), never sentiment/macro-as-alpha. | Documented dead end. Macro is a *vol/regime overlay* candidate at most, never a return signal. `research/SYNTHESIS.md` §3; `research/crypto-market-structure/application-exogenous-signal-arms.md` §8; v2-analysis §3 OT-6; post-v2-scoping §4. |

---

## Group D — Execution-realism overreach (out-of-scale or out-of-horizon)

The product is a **€200 retail, daily-horizon paper tool**. Ideas here import
heavy machinery whose entire justification is a scale or a horizon this product
does not operate at — building them is effort for provably ~zero effect.

| # | Tempting idea (usual re-proposal framing) | Guardrail / evidence that kills it | If re-proposed, point here |
|---|---|---|---|
| D-1 | **Market-impact / VWAP-TWAP execution scheduling** — "add an impact model and schedule large orders to reduce slippage." | **Impact ≈ 0 at €200 retail scale** (confirmed on BTC). The execution-cost citations exist to *justify the simple fee+spread model*, not to build a heavy execution simulator. | Do-not-build at our scale; keep the simple fee+spread model. `research/backtesting/application-cost-and-impact-modeling.md` §8; `research/strategies/application-execution-and-sizing-rules.md` §8; v2-analysis §3 OT-9. |
| D-2 | **Order-book imbalance / depth / HFT microstructure overlays** — "trade on OB imbalance / depth signals for a microstructure edge." | Depth is **~31% spoofable even on Coinbase**; the edge dies on costs and is **out of our daily horizon**. Reported volume/OI/depth are fabricated on several venues (only Kraken/HTX reconcile). Non-goal: not HFT, not market-making. | Do-not-build. `research/crypto-market-structure/application-data-integrity.md` §6 G; `research/SYNTHESIS.md` §3; product.md § IS NOT; v2-analysis §3 OT-10. |
| D-3 | **Kelly / μ-driven "smart sizer" as a return tool** — "size up on high-conviction bars using Kelly / a drift estimate to boost returns." | **Quantitatively hopeless on a no-edge coin.** Kelly on a noisy μ̂ loses 27–48% of oracle return (recoverable ~1–3% at best); the skew hurdle is essentially never met on crypto; we are never in Kelly's asymptotic regime. | Keep **fixed-fraction + vol-only** sizing. A one-knob fractional-Kelly *shrink* dial is at most a gated experiment, expected ≈ null — "size down, control risk," never "size up for alpha." `research/risk-and-sizing/application-position-sizing-and-bet-sizing.md` §8; v2-analysis §3 OT-8; post-v2-scoping §4. |
| D-4 | **Generative synthetic test data (GAN / diffusion / VAE) for the gate** — "train a generator to produce more/worse test paths than the bootstrap." | Generators **structurally smooth tails** (a Gaussian latent prior cannot produce heavy tails) and overfit a single short path; Historical Simulation / GARCH tie-or-beat them on the VaR task we care about. | Keep the **model-free moving-block bootstrap** as the default. The one honest gap (can't invent a worse-than-seen crash) is best filled by a tail-stressed/EVT slice, research-only, never wired into the frozen gate. `research/data/application-synthetic-and-monte-carlo.md` §8; `research/SYNTHESIS.md` §2 item 19, §3; v2-analysis §3 OT-7. |

---

## Group E — Gate-tampering & anchor-churn (the FROZEN evidence base is load-bearing)

The frozen robustness gate + bands + B&H benchmark are **byte-frozen and
anchor-load-bearing** (119/119 anchored report body-SHAs). "Additive-only" is the
standing architecture posture — there is **no plugin architecture**; the seams are
the three additive registration points (arm / overlay / report-annex). These two
ideas look additive but are not.

| # | Tempting idea (usual re-proposal framing) | Guardrail / evidence that kills it | If re-proposed, point here |
|---|---|---|---|
| E-1 | **A silently-shipped DSR / PBO crown-eligibility veto** — "the scorecard already computes DSR; just have `rank_candidates` reject arms below 0.95 — it's additive." | **It is NOT additive-by-default.** A DSR/PBO crown-eligibility veto changes the FROZEN gate's *effective crowning behaviour*. The operator has **kept the scorecard report-only** (ratified v2 scoping 2026-06-28; re-confirmed 2026-07-01 on the P2-2 empirical CI; kept again 2026-07-09 in the v3 close-out — a lock-it-down posture). The `crown_clears_dsr` field is informational; `rank.rs` does not read it. | **Allowed ONLY** as an explicit operator decision + its own ADR + an anchor-impact assessment + a day-1 test proving the veto bites (the no-op-overlay precedent). It is NOT smuggled in as "additive." Full decision record + the exact four-step wiring bar: **`spec/dev-notes/dsr-report-only-decision-2026-07-09.md`** (R3-3b). Also v2-architecture §6 D3 / CX-3; post-v2-scoping §4. |
| E-2 | **A cost-model default bump (vs opt-in-forever)** — "make the vol-scaled spread / harder cost model the *default* so the gate is more honest." | A default cost-path change perturbs net returns ⇒ **re-emits every one of the 119 anchored report body-SHAs** via the ADR-0038 §D6 route — for **≈0 honesty gain at €200 scale** (the opt-in `VolScaledSpread` already exists; default `LinearBps` is conservative-enough and anchor-stable). | **DROP for the foreseeable.** New cost realism stays **opt-in / new-anchored**. Revisit ONLY if a coin is found where flat-bps mis-costs a *crownable* arm — then a deliberate versioned re-anchor with its own ADR. `research/backtesting/application-cost-and-impact-modeling.md` §5/§7; v2-analysis §4 CX-7, §5 (DROP); post-v2-scoping §4; ADR-0038 §D6. |

---

## What IS still legitimately open (so this register isn't read as "nothing may ever change")

~~Exactly **one** direction is genuinely open~~ **UPDATE 2026-07-11: that direction
SHIPPED** — the structural point-in-time / as-of discipline landed as remediation
**P3 / ADR-0086** (`scripts/check_no_raw_asof_join.sh` + explicit `publication_lag_ms`
on `PitSeries`; the DVOL/macro joins verified already-as-of and retrofitted with a
zero-divergence proof). The register's original one-open-item text is preserved below
for history:

A first-class structural point-in-time / as-of data discipline — a focused
as-of-join helper + a lint that makes look-ahead *impossible by construction*,
rather than re-proven per feature by hand. It strengthens the honest-negative-result moat (the most important claim the
product makes) and is the ONE qlib capability that is a real gap here rather than
TRIED-RETIRED or out-of-scope. It is modest (a helper + lint, **not** a new
database) and does not touch the frozen gate. See
`spec/dev-notes/qlib-feature-gap-2026-06-17.md` gap #1 and project memory
"qlib gap: only PIT data is worth it."

The current honest gap map (post-remediation, 2026-07-11) lives in
[`research-gap-analysis-2026-07-11.md`](research-gap-analysis-2026-07-11.md) — its one
build-candidate is a cross-run family-wise multiple-testing report-annex (online-FDR;
created by P2's own 32-run design), everything else stated-limit/leave.

Everything else the operator might reach for is settled-dead above. The honest v3
posture is **"prove it's done," not "do more"** — ship-readiness and the
workflow-spine last-mile, NOT new alpha surface
(`spec/dev-notes/post-v2-scoping-2026-07-09.md` §3).

---

## Changelog

- 2026-07-10 (analyst, P2 efficiency-migration ratification): era-scoped the
  Group A preamble's "validated thesis" sentence (universal → current-era 2023+)
  and added the efficiency-migration framing + a pointer to the P2 corpus-expansion
  verdict re-run (`../v3/advisor-corpus-expansion/reports/backtest-2026-07-10-p2-verdict-rerun.md`).
  The P2 re-run found real, cost-annex-surviving active edges in the
  older eras (2017-20) that decayed to ~zero by 2023+ — the corpus-predicted
  anomaly-decay pattern (SYNTHESIS §62). Added an explicit "this does NOT reopen
  Group A" paragraph: old-era edges are unreachable (all advisor lookbacks end at
  "now") and chasing them is the exact A-3 alpha-chasing + survivorship mining the
  group forbids — every table row stands unchanged; only the thesis *scope* is
  sharpened. Decision-support: `p2-wobble-thesis-analysis-2026-07-10.md` (operator
  ratified Option B). Docs-only; no gate/anchor/code touched; anchors 119/119 and
  spec-lint PASS(0).
- 2026-07-09 (analyst, R3-4a): created the authoritative do-not-build register —
  13 entries across 5 groups (A alpha-chasing ×5, B scope-expansion ×2,
  C infeasible-data ×2, D execution-overreach ×4, E gate-tampering ×2), each with
  the tempting framing + the killing guardrail/evidence + a cited one-line rebuttal.
  Consolidates `spec/dev-notes/post-v2-scoping-2026-07-09.md` §4, `spec/v2/v2-analysis.md`
  §3 (OT-1..OT-10), `research/SYNTHESIS.md` §3 + `research/APPLICATIONS.md`
  (do-NOT-build negative space), `spec/product.md` (IS/ISN'T), and the dated
  operator decisions (live-trading-removed 2026-06-12, DSR report-only 2026-07-09).
  Cross-references the R3-3b DSR decision doc for the gate-veto row. Read-only
  consolidation — no gate/anchor/code touched; anchors 119/119 and spec-lint PASS(0)
  unchanged.
