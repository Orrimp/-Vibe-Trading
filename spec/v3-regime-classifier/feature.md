---
slug: v3-regime-classifier
version: 0.1.0
status: in-progress
owner: developer
updated: 2026-05-28
predecessor: spec/dev-notes/strategy-reformulation-survey-2026-05-22.md (Candidate 2)
parent: v3-three-pick
priority: P2
promoted_2026_05_28: Queue → Active by operator under the v2.5 TCN re-investigation halt routing (TCN line correctly retired 2026-05-21; C1 v3-volatility-forecaster retired 2026-05-22 NEGATIVE-NET-DELTA; C5 v3-llm-forecaster shipped v0.1.0-PARTIAL 2026-05-22). C2 is the remaining v3 three-pick slot. M-A5 light-touch refresh per the 2026-05-22 deferred-milestone activation contract.
sibling_picks:
  - v3-volatility-forecaster (Candidate 1; RETIRED 2026-05-22 NEGATIVE-NET-DELTA)
  - v3-llm-forecaster (Candidate 5; shipped v0.1.0-PARTIAL 2026-05-22)
---

# v3 — Regime classifier (predict regime label, not μ)

> **M-A5 light-touch refresh 2026-05-28.** The 2026-05-22 spec-only
> design exploration (originally R1-R8 / H1-H6 / Q1-Q7 against the
> strategy-reformulation-survey § Candidate 2 framing) is **narrowed
> to the canonical M0 contract shape** (R1-R5 + R-NR + K1-K6 + H1-H4
> + Q1-Q5 + 4-cell verdict tree + cost framing) so the architect M-T1
> spawns on a single sharpened brief. Load-bearing 2026-05-22 findings
> are preserved verbatim in
> [`spec/dev-notes/strategy-reformulation-survey-2026-05-22.md § Candidate 2`](../dev-notes/strategy-reformulation-survey-2026-05-22.md)
> plus the [§ Historical context](#historical-context-2026-05-22-analyst-pass)
> block at the bottom of this brief.

## Why now

C2 was the last open slot in the v3 three-pick set
({C1 volatility + C2 regime + C5 LLM-as-forecaster}) from the
2026-05-22 strategy-reformulation survey. Activation gate fired:

- **C1 v3-volatility-forecaster** retired 2026-05-22 with REAL evidence
  (NEGATIVE-NET-DELTA = -0.021719 vs un-targeted realdata baseline,
  post-noop-fix). R-O1 routing → "(a) RETIRE C1 with REAL evidence".
- **C5 v3-llm-forecaster** shipped v0.1.0-PARTIAL 2026-05-22 (operator-
  approved). Sharpe-delta inconclusive at PARTIAL ship; standing-Q to
  re-evaluate at follow-on but the moat-aligned C5 has had its
  budget allocation.
- **2.5 TCN re-investigation halt 2026-05-28** — operator correctly
  halted re-litigating an already-retired DL paradigm (v2.5 retired
  2026-05-21 F4-F4-F4). C2 is the next non-DL strategy track in line.

Cumulative {C1 + C2 + C5} budget cap was ~16 weeks per the survey's
Q-BUDGET resolution; C1 ~1 week (incl. noop-fix) + C5 ~1 week (PARTIAL
ship) ≈ ~2 weeks consumed; **~14 weeks of the cumulative cap remain
unused** — C2 fits comfortably at ~3-4 weeks dev wall-clock per § Cost
framing below.

**Load-bearing seed (unchanged since 2026-05-22).**
[`crates/reflection/src/regime.rs`](../../crates/reflection/src/regime.rs)
ships a pure-fn 3-state BTC daily-close regime tagger
(`RegimeTag { Bull, Bear, Chop }`, `REGIME_THRESHOLD_RATIO = dec!(0.02)`,
`classify_regime(btc_closes, at)` fn signature) which 7+ downstream
test files + `crates/reflection/src/embedding.rs` lesson-card embedding
+ Phase F Memory/Models renderer fixture builders all depend on
byte-identically. C2 v0.1.0 **extends** this seed — it does not
reinvent. R1 (backward compatibility) is the non-negotiable invariant
governing every other decision in this brief.

## Scope (v0.1.0)

### R1 — Backward compatibility (load-bearing)

Every existing `RegimeTag` literal reference + `classify_regime`
callsite in `crates/reflection`, `crates/reports`, and downstream
tests MUST keep compiling byte-identical. The 70 anchored body-SHAs
in `spec/anchors.toml` MUST stay byte-identical (additive feature; no
strategy is rewired at v0.1.0 — see R3 default). Specifically:

- `RegimeTag` enum bytes stay identical (3 variants: Bull / Bear /
  Chop, exact order). New variants — if Q1 forces them — append at
  the end; never insert mid-enum.
- `Display` for `RegimeTag` keeps emitting `bull|bear|chop` lowercase.
- `REGIME_THRESHOLD_RATIO = dec!(0.02)` stays at ±2%. Any new hourly
  classifier uses a separate const.
- `classify_regime(btc_closes, at)` signature + behaviour
  byte-identical (T1802 test family in
  `crates/reflection/tests/regime_classifier.rs` must keep passing).
- `crates/reflection/src/embedding.rs` byte output for legacy 3-state
  `RegimeTag` stays identical (lesson-card embedding determinism is
  the load-bearing K-reg-4 risk).

### R2 — Regime taxonomy + hourly classifier

Define a regime taxonomy spanning at minimum **trending-up,
trending-down, and chop**; optionally adding **volatile-noise** and
**calm** axes per Q1. Implement an hourly-cadence classifier over a
(timestamp, close) series, generalising the existing daily 3-state
tagger to hourly cadence on USDT-pair OHLCV from the real-Binance
2023-2024 realdata window.

Classifier feature surface (architect M-T1 picks specific features
from this menu; analyst stays out of the model-class decision):

- Trailing 168-bar / 720-bar log-return mean + std (trending vs chop axis).
- Trailing 168-bar realised volatility (e.g. Parkinson HL-vol from
  existing `crates/forecast/src/vol.rs` plumbing — sibling of C1).
- Rolling autocorrelation at lag 1 / lag 24 (mean-reverting vs
  trending discriminator).
- Hurst exponent over 720-bar window (long-memory regime signal).
- ADX or trend-strength index (textbook regime feature).
- Optional GARCH(1,1) σ̂ from `crates/forecast/src/garch.rs` (C1 sibling
  reuse — gives the hourly classifier vol-regime info for free).

**The analyst explicitly does NOT pick the model class at M0.** That
is the architect's M-T1 Q-arch decision. Candidates surveyed in the
[2026-05-22 brief](../dev-notes/strategy-reformulation-survey-2026-05-22.md#candidate-2--regime-classification)
were: (a) HMM (Baum-Welch on log-returns); (b) small MLP / 1-D conv
classifier ~100k params; (c) ensemble; (d) rule-based threshold
extension of the existing seed. Q3 below surfaces this to operator.

### R3 — Strategy-selection contract

The classifier's regime tag feeds strategy selection in **one** of
three integration modes (Q4 below):

- **(a) Overlay-style multiplier per regime** — per-symbol position
  weight from v1 cross-sectional momentum is multiplied by a
  regime-dependent scalar at the strategy → executor handoff. Shape
  mirrors the v2.5 TCN overlay + C1 GARCH overlay pattern (with the
  load-bearing CLAUDE.md non-negotiable: an end-to-end equity-divergence
  test MUST ship from day 1 to catch the v3-vol-targeting no-op
  precedent — overlay equity ≠ un-targeted baseline by ≥ 1 bp when
  the regime tag is non-trivial).
- **(b) Strategy-switching gate** — dispatcher: in Bull/Bear → run v1
  momentum; in Chop → run a mean-reversion sibling (not built yet;
  v1.5 mean-reversion was queued but never shipped — this option
  introduces a prerequisite-feature dependency that compounds scope).
- **(c) Ensemble weighting** — regime-conditional weights over multiple
  strategies; cleanest seam with C5 LLM signals if a follow-on
  v3-llm-regime-ensemble lands, but adds architect surface that
  wouldn't ship in v0.1.0's 3-4 week budget alongside the classifier.

**Analyst-recommended R3 default: (a) overlay-style multiplier on
v1 momentum.** Smallest blast radius; mirrors the v3-volatility-forecaster
overlay pattern (now with the noop-fix end-to-end divergence gate in
place — CLAUDE.md non-negotiable line); doesn't introduce a
prerequisite-feature dependency (b); doesn't add an ensemble surface
(c). Defer (b) and (c) to v0.2.0+ follow-ons if H1/H2 clear.

### R4 — Verification regime cross-section

Backtest the regime-conditional overlay against the un-conditional v1
momentum baseline across scenarios pinned to real-Binance anchored
data, picking samples where regime structure is **visually evident**:

| Regime sample | Real-Binance window | Expected dominant tag | Anchored baseline |
|---|---|---|---|
| 2022 BTC bear-trending (proposal — not anchored yet) | Jan-Nov 2022 | trending-down | NOT-YET-ANCHORED; H1 needs operator-decide on whether to anchor a 2022 baseline or restrict v0.1.0 to 2023-2024 anchored window |
| 2023 H1 BTC consolidation | Jan-Jun 2023 (in `top10-2023-fy-momentum-realdata`) | chop | `top10-2023-fy-momentum-realdata` (anchor row 70-ish, `v2.6.0-realdata`) |
| 2023 H2 BTC trending-up | Jul-Dec 2023 (in `top10-2023-fy-momentum-realdata`) | trending-up | same anchor as above (H1/H2 split via tag emission per-bar) |
| 2024 alt rotation / volatile noise | Q1-Q4 2024 (in `top10-2024-fy-momentum-realdata`) | volatile-noise / mixed | `top10-2024-fy-momentum-realdata` (anchor row ~71, `v2.6.0-realdata`) |

**Analyst-recommended R4 v0.1.0 default: restrict to the 2 anchored
realdata baselines (top10-2023-fy + top10-2024-fy momentum-realdata).**
The 2022 bear window is unanchored — adding it would require a fresh
realdata window pin (architect would have to refresh `data/binance/`
REVISION.toml or operator-decide deferring to v0.2.0). H1 (classifier
identifies the 4 obvious regime-shifts) becomes a 2-sample test at
v0.1.0 (BTC H1 2023 chop + BTC H2 2023 trending-up are observable
within the single 2023-fy window via per-bar tag emission); the 2024
window gives the alt-rotation/volatile-noise sample. Three regime
samples on the existing anchor baselines — sufficient for an alpha
existence test, not yet a full regime-coverage audit.

### R5 — Non-regression

- **R-NR.1** — `bash scripts/verify_anchors.sh` PASS (70/70). Zero
  existing anchor SHA delta. New anchors at v0.1.0 ship are
  **additive** (Q5 below picks namespace `v2.7.0-regime` per
  analyst-recommended default).
- **R-NR.2** — `spec_lint.py` no NEW violation categories vs the
  73/3 baseline (this M0 pass authors spec files only — no library
  code touched).
- **R-NR.3** — `crates/reflection/src/regime.rs` `RegimeTag` /
  `REGIME_THRESHOLD_RATIO` / `classify_regime` byte-identical (R1
  invariant; architect M-T1 explicitly verifies the
  `patchtst_overlay_neutrality`-equivalent regime-overlay-neutrality
  guard ships in developer Wave A).
- **R-NR.4** — Zero new design tokens. Zero new `strings.rs` adds
  beyond classifier-label strings — at most 2-5 lowercase regime
  labels per Q1 outcome (e.g. `volatile`, `calm` if Q1=(b)),
  registered in `crates/ui/src/strings.rs` `all()` slice if a Phase F
  Trail UI surface is wired at v0.1.0 (R3 default = (a) overlay does
  not require UI surface, so the strings.rs adds are
  Q1-conditional, not Q1-mandatory).
- **R-NR.5** — `crates/reflection/src/embedding.rs` byte-output for
  legacy 3-state RegimeTag stays identical (K-reg-4 lesson-card
  embedding determinism — load-bearing for the 30 v2.5-chain anchor
  body-SHA invariants through memory-highlights renderer fixtures
  in `crates/reports/tests/`).
- **R-NR.6** — CLAUDE.md non-negotiable: the regime overlay strategy
  (R3 = (a) default) ships with a **baseline-equity-divergence
  end-to-end test from day 1** per
  [`crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`](../../crates/strategy/tests/vol_targeting_overlay_end_to_end.rs)
  precedent. Asserts overlay equity ≠ un-conditional baseline equity
  by ≥ 1 bp when the regime tag is non-trivial. Mandatory developer
  Wave deliverable; tester gates on it explicitly.

## K — Risk register / falsifiers

| K | Risk | Mitigation |
|---|---|---|
| **K1** | **Synthetic-data triviality.** Regime classification on synthetic GBM data is meaningless because synthetic GBM has no regime structure by construction (stationary Brownian motion). The classifier may train and test cleanly on synthetic but produce noise output on real-Binance bars. | R4 anchors against real-Binance only; no synthetic baseline for regime accuracy evaluation. Developer Wave A trains + evaluates on real-Binance realdata fixtures only. |
| **K2** | **Classifier flicker.** Classifier flips regime every bar — useless. Per the 2026-05-22 H6 register, target switch rate ≤ 10/week on liquid USDT pairs (≤ ~0.06 switches per bar). | R-NR.6 e2e divergence gate + new K2 test: `regime_switch_rate_under_threshold` in developer Wave A. Falsifies the feature if hourly switch rate > 20/week (2× the target). |
| **K3** | **Forecast latency budget violation.** Regime classification on every bar adds latency that may break the per-bar forecast budget. C1 GARCH overlay measured ~50µs/bar; the architecture has historically targeted a < 5ms p99 budget at the strategy → executor handoff. | Architect M-T1 measures classifier latency at the model-class decision (Q3); if HMM Baum-Welch / forward-pass cost > 1 ms/bar, downgrade to rule-based (Q3=(d)) or cache the regime tag at coarser cadence (e.g. update once per 24 bars). |
| **K4** | **Lesson-card embedding determinism break.** Any change to `RegimeTag` ordering OR `Display` output OR the underlying byte encoding in `crates/reflection/src/embedding.rs` breaks lesson-card retrieval determinism + the 30-anchor body-SHA invariant transitively through memory-highlights renderer fixtures in `crates/reports/tests/`. | R1 + R-NR.3 + R-NR.5 lock the invariant. Q1 default (a) (3-state extend) preserves byte-identity. Architect M-T1 explicitly verifies no `embedding.rs` byte drift in developer Wave A. |
| **K5** | **v1 baseline already implicitly captures trending.** The v1 cross-sectional momentum strategy's 20-bar lookback already implicitly trades "trending" — a regime overlay that turns OFF momentum in non-trending regimes might just reduce exposure to the period when momentum was going to be flat anyway (i.e. no net Sharpe lift, just less turnover). Inherited from 2026-05-22 K-reg-3. | R4 backtest reports compute per-regime decomposition (which periods is the regime-overlay outperforming?). If lift comes only from drawdown reduction in Chop regimes, that's a real but modest win — H2 ≥ +0.10 Sharpe-delta gate stays load-bearing for the SHIP decision. |
| **K6** | **v3-vol-targeting-no-op precedent recurrence.** The regime overlay could be implemented with the correct scale/multiplier logic but still be a no-op if the `Signal.quantity_scale` field (or equivalent) is not propagated to the executor. Reference: `spec/v3-volatility-forecaster-noop-fix v0.1.0` 2026-05-22; CLAUDE.md non-negotiable § "Every strategy overlay or sizing-modifier ships with a baseline-equity-divergence end-to-end test from day 1." | R-NR.6 e2e divergence test is **MANDATORY** at developer Wave A. Architect M-T1 explicitly cites ADR-0038 § D6 + the noop-fix precedent in the overlay's wiring contract. |

## H — Hypotheses

| H | Hypothesis | Confidence | Falsifier |
|---|---|---|---|
| **H1** | **Regime-aware strategy selection beats fixed strategy by ≥ +0.10 Sharpe-delta** on real-Binance hourly 2023+2024 against the un-conditional v1 momentum baseline (`top10-2023-fy-momentum-realdata` + `top10-2024-fy-momentum-realdata`). This is the survey's canonical alpha-unlock gate, inherited verbatim from C1 + C5. | Medium | R4 backtest report at developer Wave D shows joint Sharpe-delta. If < +0.05 → V-FAIL (H1 falsified). If [+0.05, +0.10) → V-MARGINAL (R-O2 routing). |
| **H2** | **Classifier accuracy ≥ 70%** on a held-out regime labeling task, where "ground truth" is per Q4 below. The 2026-05-22 brief's H1 carried forward. Falsifiable; cheap to evaluate. | Medium | Falsifies if < 50% on the held-out set (worse than random for a 3-state task). M-FINAL tester reports per-bar accuracy on the held-out window. |
| **H3** | **Hourly crypto exhibits 3-4 statistically distinguishable regimes** on the realdata window (e.g. for HMM model class: Baum-Welch likelihood monotone-increases in number of states up to 3-4 then plateaus). Inherited from 2026-05-22 H4. | Medium | Developer Wave A: if Q3 = HMM, the likelihood-vs-K curve is logged. Architect-decide whether 5+ states are explored. |
| **H4** | **Regime tag is stable enough for downstream consumption.** Switch rate ≤ 10/week on liquid USDT pairs in the realdata window (~0.06 switches/bar). Inherited from 2026-05-22 H6. K2 falsifier above is the load-bearing test. | Medium-high (the daily 7-day-lookback seed empirically averages ~3-5 regime transitions per year on BTC daily closes; the hourly equivalent with similar threshold tuning should land in the same neighborhood) | R-NR.6 e2e divergence gate confirms tag stability under realistic regime structure. K2 unit test asserts ≤ 20/week (2× the H4 target). |

## Operator-decide questions (Q1-Q5)

### Q1 — Regime taxonomy

How many regime states, and what do they represent?

| | Option | Pros | Cons |
|---|---|---|---|
| **(a)** | **Keep 3-state Bull/Bear/Chop (extend daily tagger to hourly cadence)** | Lesson-card embedding determinism preserved (K4); no enum-variant decisions; matches existing UI/embedding contracts byte-identically. | "Bull/Bear/Chop" doesn't capture "volatile-noise" as a regime — survey suggested {trending-up, trending-down, mean-reverting, volatile, calm}. |
| (b) | 4-state Bull/Bear/Volatile/Calm | Closer to textbook regime taxonomies; matches survey's suggestion. | New `RegimeTag::Volatile` + `RegimeTag::Calm` enum variants; ordinal encoding must append (not insert) to preserve K4 — adds embedding-determinism architect surface in Wave A. |
| (c) | 5-state Bull/Bear/Volatile/Calm/Chop | Most expressive. | Same K4 surface as (b); higher Q3 model-class burden (5-state HMM trains slower). |
| (d) | Continuous-valued regime score in [0, 1]^N (no discrete tag) | Avoids enum-variant lesson-card problem; richer signal. | Lesson-card system fundamentally consumes `RegimeTag` (discrete enum) — would need BOTH continuous regime + derived discrete tag for the legacy surface. 2× the surface. |
| (e) | HMM-derived hidden states (no human labels) | Most statistically honest; doesn't pre-impose taxonomy. | Human-readable interpretation of emergent states is post-hoc; strategy builder still has to map state-K to "buy momentum" / "flat" / etc. Forces Q3 = HMM. |

**Analyst-recommended Q1 = (a) keep 3-state + extend in-place.**
Lesson-card embedding determinism (K4) is load-bearing and the
simplest path is the safest. If H2 (accuracy ≥ 70%) fails on a 3-state
hourly classifier, Q1=(b) becomes the natural v0.2.0 follow-on brief.

### Q2 — Training data window

Where does the classifier train / fit?

| | Option | Rationale |
|---|---|---|
| **(a)** | **2023+2024 real-Binance hourly OHLCV (same substrate as v2.5 chain + C1 + C5; ~17,500 hourly bars on each of 10 USDT pairs)** | Matches existing realdata anchors; no new data sourcing; reuses `data/binance/REVISION.toml` lock SHA `3a8b96c4…` (preserved across C1 + C5). |
| (b) | Extend to 2022 (3-year window; ~26,000 bars per pair) | Adds the 2022 bear-trending regime sample. Requires fresh REVISION.toml pin + architect-side data acquisition cost. |
| (c) | Train/val split inside 2023-2024 (e.g. 2023 = train; 2024 = held-out val) | Cleanest H2 (accuracy) evaluation methodology; matches v2.5 chain BS-1/BS-2 split convention. |

**Analyst-recommended Q2 = (a) + (c).** Use the existing 2023+2024
anchored realdata at substrate level; split 2023 → train + 2024 → val
for H2 evaluation. Defer 2022 extension to v0.2.0 if H1 clears.

### Q3 — Model class

What model class does the architect lock at M-T1?

| | Option | Pros | Cons |
|---|---|---|---|
| **(a)** | **Statistical HMM (Baum-Welch on hourly log-returns + \|log-returns\| + Parkinson HL-vol)** | Smallest surface; trains in minutes; well-understood failure modes; tractable to test; sibling to C1 GARCH fitter (same `crates/forecast` infra). | HMM is a strong model assumption; emergent states may not map cleanly onto human-readable labels (K-reg-1 risk inherited from 2026-05-22). |
| (b) | Markov-switching regression (Hamilton 1989) | Textbook precedent; tractable; closed-form filter. | Higher model complexity than plain HMM; ≥ 2× the implementation surface. |
| (c) | Classifier ensemble (HMM + small MLP + rule) | Maximum robustness | Doubles scope; ~5-6 week budget at v0.1.0 instead of ~3-4. |
| (d) | Rule-based threshold extension (no learning; extends existing seed) | Cheapest; no training; pure-fn extension; ZERO new dependencies. Lesson-card embedding trivially stable. | Lowest ceiling — won't match HMM if regime structure is genuinely non-trivial. |
| (e) | LLM-as-classifier (route regime classification to crates/llm via C5 reuse) | Composes with C5 v3-llm-forecaster infra (RecordingProvider/ReplayProvider/cache already shipped). | LLM cost + cadence constraints; C5's H1 has not yet shipped final verdict — LLM-as-classifier inherits any C5 H1 uncertainty. |

**Analyst-recommended Q3 = (a) statistical HMM** unless Q1 = (e)
(in which case HMM is forced). **(d) is a credible cheap fallback**
if the budget tightens — rule-based hourly classifier can ship in ~1
week and is a clean H2 falsifier on its own. Architect M-T1 should
evaluate (a) vs (d) head-to-head if Q1 = (a).

### Q4 — Integration mode

How does the regime classifier feed the v1 momentum strategy?

| | Option | Pros | Cons |
|---|---|---|---|
| **(a)** | **Overlay-style multiplier (regime → per-symbol scalar applied to v1 momentum signals)** | Smallest blast radius; mirrors v3-volatility-forecaster + v2.5 TCN overlay patterns; reuses the post-noop-fix `Signal.quantity_scale` wiring contract; one new strategy builder. | Less expressive than dispatch/ensemble; can't run mean-reversion in Chop regimes (mean-reversion sibling doesn't exist anyway). |
| (b) | Strategy-switching dispatcher (different strategy per regime; e.g. v1 momentum in Bull/Bear + v1.5 MR in Chop) | Matches survey's framing for "regime-conditional dispatch". | Prerequisite v1.5 mean-reversion sibling doesn't exist; K-reg-2 (two-stage pipeline error compounding); compounds scope by ≥ 2× to ~5-7 weeks. |
| (c) | Regime-as-feature (regime tag becomes a column in feature window; v1 momentum + future strategies consume it as conditioning input) | Most flexible; cleanest seam with C5 LLM signals as a conditioning input. | No immediate strategy ships at v0.1.0; the feature is "just" a producer. |
| (d) | All three as opt-in builders | Max optionality | ~3× the architect surface; doesn't fit v0.1.0 budget. |

**Analyst-recommended Q4 = (a) overlay-style multiplier on v1 momentum.**
Smallest implementation; reuses the noop-fix-era wiring; tightest gate
against K6 (the v3-vol-targeting no-op precedent is fresh in
infra-memory; same gates apply verbatim). Defer (b) to v0.2.0 if v1.5
mean-reversion ships first; defer (c) to v0.2.0+ as the natural
follow-on if H1 clears and the ensemble surface earns a budget.

### Q5 — Scope at v0.1.0

What's the asset-coverage breadth at v0.1.0?

| | Option | Rationale |
|---|---|---|
| **(a)** | **Single asset (BTC-USDT) hourly classifier + overlay** | Tightest budget (~2-3 weeks dev); cleanest H2 accuracy evaluation; matches the existing daily seed's BTC focus. |
| **(b)** | **All 10 USDT pairs hourly classifier + overlay (same basket as v1 momentum)** | Matches v1 momentum's cross-sectional basket exactly — Sharpe-delta is computed against the same baseline (v1 momentum's anchored real-Binance scenarios). H1 evaluation is direct. |
| (c) | 3-asset (BTC + ETH + SOL) hourly | Compromise; mid-budget; partial cross-sectional coverage. |
| (d) | Cross-pair regime aggregation (single global regime tag derived from multi-pair classifier) | Cleanest "market-regime" framing; matches Hamilton-style macro regime literature. | Higher architect surface; per-pair lift attribution harder. |

**Analyst-recommended Q5 = (b) all 10 USDT pairs.** Matches v1 momentum's
basket exactly so the H1 +0.10 Sharpe-delta gate is computed against
the right baseline (BS-1 + BS-2 anchored realdata scenarios in R4).
Architect M-T1 confirms cross-pair regime training is computationally
tractable under the picked model class (Q3) — HMM trains on a single
pair's log-returns in minutes, so 10 pairs is still wall-clock-bounded.

## Pre-drawn 4-cell verdict tree (presenter inherits at M-P2)

| Cell | Condition | Route |
|---|---|---|
| **R-O1** | H1 ≥ +0.10 Sharpe-delta AND H2 ≥ 70% accuracy AND H4 switch-rate ≤ 10/week AND R-NR.1-6 green AND R-NR.6 divergence gate fires (overlay equity ≠ baseline by ≥ 1 bp) | **SHIP** v0.1.0. Operator approval block; close the v3 three-pick set (C1 retired, C5 partial-shipped, C2 ships). Spawn v0.2.0 follow-on briefs: Q4=(c) regime-as-feature ensemble + Q1=(b) 4-state extension + Q5=(d) cross-pair aggregation per operator-decide. |
| **R-O2** | H1 in [+0.05, +0.10) Sharpe-delta (V-MARGINAL) AND H2 + H4 + R-NR green | **SHIP-WITH-NARROWER-SCOPE** v0.1.0. Operator decides between (i) ship-as-V-MARGINAL with an honest sprint-review framing (mirrors the v3-volatility-forecaster v0.1.0 V-MARGINAL ship-with-caveats pattern) OR (ii) hold for re-tuning of Q3 hyperparameters (rule-based threshold for Q3=(d); HMM K-state for Q3=(a)) under a v0.1.1 patch follow-on. |
| **R-O3** | H1 < +0.05 Sharpe-delta (V-FAIL) AND R-NR green AND K2/K3/K4/K5/K6 all clear | **HOLD-FOR-OPERATOR-DECIDE.** No alpha unlocked at v0.1.0; classifier itself works (H2 + H4 green) but doesn't extract Sharpe-delta on this baseline. Operator picks between (i) v0.2.0 Q4=(c) ensemble follow-on (different integration mode might extract lift); (ii) v0.2.0 Q1=(b) 4-state taxonomy follow-on; (iii) full retirement — route to "all 3 v3 picks complete (C1 retired NEGATIVE-NET-DELTA, C5 partial, C2 V-FAIL)" closing meta-survey. |
| **R-O4** | Any of K1-K6 trip (synthetic-data triviality, flicker, latency budget violation, embedding break, K5 baseline-implicit-capture not falsified, K6 noop-fix precedent recurrence) OR R-NR.1-6 RED | **ABANDON** v0.1.0 at developer Wave A. Route back to analyst for K-specific root-cause; possible outcomes: feature deferral, scope narrowing (drop to Q5=(a) single-asset), or full retirement with what-not-to-chase dev-note. |

## Cost framing

| Phase | Effort | Notes |
|---|---|---|
| Analyst M-A5 refresh (this brief) | ~0.5 day | Light-touch update of 2026-05-22 brief to canonical M0 shape. |
| Operator-decide (Q1-Q5) | ~1 day | Q1 + Q3 + Q4 + Q5 are load-bearing decisions; Q2 close-to-default |
| Architect M-T1 | ~3-5 days | Lock Q-resolutions, decompose Waves A-E, write new sibling ADR (proposed ADR-0049 regime-classification-verdict-shape per 2026-05-22 brief; **NOT** ADR-0033 extension — ADR-0033 § D3 stays immutable). Verify `embedding.rs` byte-identity (K4 mitigation). |
| Developer Wave A (classifier core + overlay strategy) | ~5-7 days | Extend `regime.rs`; new strategy in `crates/strategy/` mirroring vol-target-overlay pattern; R-NR.6 mandatory e2e divergence test from day 1 |
| Developer Wave B (audit ledger + Trail UI) | ~2-3 days | `JournalEntry { kind: "regime_tag", … }` rows; optional Phase F Trail UI extension (Q1-conditional strings.rs adds) |
| Developer Wave C (backtest scenarios + R4 reports + Sharpe-comparison report) | ~3-5 days | 2 new scenarios at `v2.7.0-regime` anchor pin (Q5 default = (b) 10 pairs across 2023 + 2024 = 2 anchors total) |
| Developer Wave D (k-folds / val pass + H2 accuracy report) | ~2-3 days | Q2 = (c) 2023 train / 2024 val split. |
| Tester M-FINAL | ~3 days | Standard test-report.md; V-PASS / V-MARGINAL / V-FAIL verdict per new sibling ADR; R-NR.6 divergence gate; K2 switch-rate gate; 70 → 72 anchors PASS. |
| Presenter M-P | ~1 day | Sprint-review deck; live demo of classifier; operator approval block. |
| **Total wall-clock from activation** | **~3-4 weeks** | Survey budget was ~3-5 weeks per 2026-05-22 brief; this refresh tightens to ~3-4 weeks under Q3 = (a) HMM default; ~2-3 weeks under Q3 = (d) rule-based fallback. |

**Variance budget.** 1.5× wall-clock tripwire at ~5-6 weeks (per the
survey's Q-BUDGET resolution). Cumulative {C1 + C2 + C5} cap was ~16
weeks; ~14 weeks remain unused after C1 + C5 → comfortably within cap.

## Predecessor / parent chain

- **Parent**: v3-three-pick (umbrella for the 3-candidate set from
  the 2026-05-22 strategy-reformulation survey)
- **Predecessor**: `spec/dev-notes/strategy-reformulation-survey-2026-05-22.md § Candidate 2`
  (the analyst-pass framing); plus the 2026-05-22 spec-only design
  exploration captured in this brief's earlier version (now preserved
  in § Historical context below).
- **Siblings**: `v3-volatility-forecaster v0.1.0` (C1; RETIRED
  2026-05-22 NEGATIVE-NET-DELTA); `v3-llm-forecaster v0.1.0-PARTIAL`
  (C5; shipped 2026-05-22 partial-approved).
- **Successor (probable)**: `v3-regime-classifier-v0.2.0` follow-ons
  per R-O1 / R-O2 / R-O3 cells (Q1=(b) 4-state extension, Q4=(c)
  ensemble integration, Q5=(d) cross-pair aggregation).

## Cross-references

- 2026-05-22 strategy-reformulation survey — [`spec/dev-notes/strategy-reformulation-survey-2026-05-22.md`](../dev-notes/strategy-reformulation-survey-2026-05-22.md)
- C1 sibling brief — [`spec/v3-volatility-forecaster-noop-fix/feature.md`](../v3-volatility-forecaster-noop-fix/feature.md)
- C5 sibling brief — [`spec/v3-llm-forecaster/feature.md`](../v3-llm-forecaster/feature.md)
- Load-bearing seed — [`crates/reflection/src/regime.rs`](../../crates/reflection/src/regime.rs)
- v3-vol-targeting noop precedent (K6 reference) — [`spec/v3-volatility-forecaster-noop-fix/feature.md`](../v3-volatility-forecaster-noop-fix/feature.md)
- CLAUDE.md non-negotiable on overlay e2e divergence — [`crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`](../../crates/strategy/tests/vol_targeting_overlay_end_to_end.rs)
- Adjacent overlay precedents — [`crates/strategy/src/vol_targeting_overlay.rs`](../../crates/strategy/src/vol_targeting_overlay.rs), [`crates/strategy/src/vol_killswitch_overlay.rs`](../../crates/strategy/src/vol_killswitch_overlay.rs)
- Anchors file (target of additive R-NR.1) — [`spec/anchors.toml`](../anchors.toml)
- Trace row — `REQ-V3-REGIME-CLASSIFIER-001` in [`spec/trace.toml`](../trace.toml)
- Tasks — [`tasks.md`](tasks.md)

## Design

> Architect M-T1 closed 2026-05-28 (commit pending — this edit).
> Authoritative reference: [ADR-0049](../architecture/adr/0049-v3-regime-classifier-markov-switching-verdict-shape.md)
> — sibling to ADR-0038 (NOT extension); ADR-0033 § D3 STAYS IMMUTABLE.

### Architect decisions (resolution of operator-override interaction surface)

The M-OD overrides on Q1+Q3+Q4 (operator went bolder than analyst
defaults) opened three architect-load-bearing questions. Resolutions:

#### A. Q4 dispatcher prerequisite (no v1.5 price-MR exists for Chop/Volatile)

**RESOLVED — option (i) degenerate cash-hold strategy.** The dispatcher
routes Bull/Bear → v1 MomentumStrategy and Volatile/Calm → new
`CashHoldStrategy`. Existing positions are HELD on regime transition
into Volatile/Calm (cash-fallback is SUPPRESSION, not LIQUIDATION) —
natural exits via composed exit policy (ADR-0010). Forward-compatible
seam: v0.2.0 follow-on brief `v1.5-mean-reversion-for-regime-dispatcher`
swaps `CashHoldStrategy → MeanReversionStrategy` with zero dispatcher
rewire. Full contract: ADR-0049 § D3.

#### B. K4 lesson-card embedding determinism vs. Q1=(b) new variants

**RESOLVED — option (γ): preserve Chop, append Volatile + Calm.** The
RegimeTag enum keeps `Chop` as deprecated-but-preserved-for-K4 (existing
ordinal 2; legacy daily seed keeps emitting it byte-identically) +
appends `Volatile=3, Calm=4`. The new Markov-switching classifier emits
**only the 4 Q1=(b) variants**; the dispatcher routes only on the 4
Q1=(b) variants. Embedding vector grows by 2 one-hot slots but legacy
3-state fixtures emit byte-identical embeddings (Volatile/Calm slots
stay zero). Wave B's `regime_overlay_neutrality_4state.rs` gates the
K4 invariant. Escape hatch (no ADR amendment): versioned embedding
schema (`EmbeddingV1` / `EmbeddingV2`) if vector-length growth itself
breaks downstream byte-compare. Full contract: ADR-0049 § D2.
**Alternatives (α) remap and (β) full 5-state rejected** — see ADR-0049
§ D2.

#### C. Markov-switching {μ_s, σ²_s} 4-state prior specification

**RESOLVED — operator-set semantic priors lock regime identities;
Baum-Welch refines parameter values only (no post-hoc state-label
reassignment).** Hamilton 1989 regression-form mixture with 4 explicit
{μ_s, σ²_s} parameters per regime + 4×4 row-stochastic transition
matrix P:

| State    | μ_s prior (hourly log-return) | σ²_s prior                       |
|----------|-------------------------------|----------------------------------|
| Bull     | +1e-4 (≈+0.01%/h drift)       | Low — 25th-pctile realized var   |
| Bear     | −1e-4                         | Low — 25th-pctile                |
| Volatile | 0                             | High — 90th-pctile               |
| Calm     | 0                             | Low — 10th-pctile                |

EM convergence: Δ log-likelihood ≤ 1e-6 over 5 consecutive iters; max
200 iters; failure → V-REG-1. Per-pair fit on the 2023 train window
(Q2=(c) split); 2024 held-out for val + H2 accuracy gate. Full contract:
ADR-0049 § D1.

### Crate layout

| Component | Crate | New file? |
|-----------|-------|-----------|
| Markov-switching fitter + forward filter | `crates/forecast` | NEW `markov_switching.rs` (sibling to `garch.rs` / `vol.rs`) |
| RegimeClassifier trait + hourly impl | `crates/forecast` | extend `markov_switching.rs` |
| RegimeTag enum extension (Chop preserve + Volatile + Calm append) | `crates/reflection` | edit `regime.rs` (developer territory — architect only specs K4 contract) |
| Embedding K4 ordinal contract | `crates/reflection` | edit `embedding.rs:120-126` |
| Dispatcher | `crates/strategy` | NEW `regime_dispatcher.rs` (sibling to `vol_targeting_overlay.rs`) |
| CashHoldStrategy | `crates/strategy` | NEW `cash_hold.rs` |
| V-REG verdict bin | `crates/forecast` | NEW `bin/regime_verdict.rs` |
| T-REG Sharpe-comparison report bin | `crates/forecast` | extend existing `bin/sharpe_comparison.rs` with regime-dispatcher dispatch arm (ADR-0038 § D1.c precedent) |
| Audit ledger regime_tag row shape | `crates/audit` | extend `JournalEntry` enum (additive variant) |
| Trail UI surface | `crates/ui` | conditional column / modal (Wave D) |

### Wave decomposition (M-DEV)

5 sequential developer waves (Wave A → B → C → D → E) + 1 closing
gate-wave (F = e2e + harness). Wall-clock ~5-7 days per Wave A; ~2-5
days for subsequent waves.

**Wave A — Markov-switching core + forward filter**
- New `crates/forecast/src/markov_switching.rs`: 4-state regression
  with D1 operator-set priors; Baum-Welch EM refinement; forward
  filter emitting per-bar posterior `[p_Bull, p_Bear, p_Volatile, p_Calm]`.
- New `RegimeClassifier` trait (`classify(bars: &[Bar]) -> Vec<RegimePosterior>`)
  with `MarkovSwitchingClassifier` impl. Trait-based seam for v0.2.0+
  alternate model classes.
- Unit tests: convergence on synthetic 4-regime GBM-with-switch
  fixtures; per-state μ_s/σ²_s recovery within 10% of ground truth;
  K2 falsifier (`regime_switch_rate_under_threshold`); D6 falsifier
  (`dispatcher_confidence_gate_zero_when_uncertain` +
  `dispatcher_switches_when_confident`).
- **K1 mitigation:** trained on synthetic and on real-Binance 2023
  hourly fixtures; H3 likelihood-vs-K curve logged.

**Wave B — RegimeTag extension + K4 embedding contract**
- Extend `crates/reflection/src/regime.rs`: add `Volatile, Calm` enum
  variants (APPEND only; ordinals 3, 4). Display: `"volatile"`, `"calm"`.
- Extend `crates/reflection/src/embedding.rs:120-126`: add
  `RegimeTag::Volatile => 3, RegimeTag::Calm => 4` arms; embedding
  vector grows by 2 slots.
- New test `crates/reflection/tests/regime_overlay_neutrality_4state.rs`:
  re-runs ≥ 1 legacy 3-state lesson-card fixture (e.g.
  `embedding_determinism.rs` reference vector) and asserts byte-identity
  on the Bull/Bear/Chop slots. Falsifies the K4 invariant if it breaks.
- **Escape hatch (if K4 byte-identity breaks because vector-length grew
  even with zero-init Volatile/Calm slots):** promote to versioned
  embedding schema — legacy fixtures pin to `EmbeddingV1` (3-state,
  unchanged byte output); new classifier emits `EmbeddingV2` (5-slot).
  **No ADR amendment required** — declared in-scope in ADR-0049 § D2.

**Wave C — Strategy dispatcher + cash-fallback**
- New `crates/strategy/src/cash_hold.rs`: `CashHoldStrategy` emits
  `SignalKind::Hold` for every (symbol, bar). Existing positions HELD.
- New `crates/strategy/src/regime_dispatcher.rs`: stateful adapter
  wrapping `MomentumStrategy` + `CashHoldStrategy`. Routes per regime
  tag (D3 routing table); gates switches on `max_p ≥ 0.70` (D6).
- Unit tests: routing table coverage (4 regime variants × 2 strategies);
  D6 confidence gate; transition semantics (Bull→Volatile suppresses
  new signals, holds positions; Volatile→Bull resumes momentum).

**Wave D — Audit + Trail UI surface**
- `JournalEntry::RegimeTag { ts, symbol, regime: RegimeTag,
  max_confidence: Decimal }` — additive variant.
- Phase F Trail UI: regime-tag-per-bar column or per-symbol modal
  (architect default: column, since the dispatcher is already
  visible). Register `volatile`, `calm` strings in
  `crates/ui/src/strings.rs::all()`.
- R-NR.4 zero-new-design-tokens gate; spec-lint passes.

**Wave E — Backtest scenarios + anchors (D5 namespace)**
- 2 scenario equity-curve runs: `top10-2023-fy-regime-dispatcher-realdata`
  + `top10-2024-fy-regime-dispatcher-realdata` (Q5=(b) 10 pairs;
  Q2=(c) train/val split).
- New V-REG bin `crates/forecast/src/bin/regime_verdict.rs` emits
  `regime-verdict-bs1-realdata` (held-out 2024).
- Sharpe-comparison bin extension emits
  `sharpe-comparison-regime-dispatcher-bs1-realdata` (T-REG gate
  input).
- 4 new anchors added to `spec/anchors.toml` under namespace
  `v3.0.0-regime` (D5). 70 → 74 PASS at M-FINAL.

**Wave F — e2e divergence gate + tester harness**
- **Mandatory CLAUDE.md non-negotiable** (R-NR.6): new e2e test
  `crates/strategy/tests/regime_dispatcher_end_to_end.rs` (pattern
  copied from `vol_targeting_overlay_end_to_end.rs`) asserts dispatcher
  equity ≠ un-conditional v1 momentum baseline by ≥ 1 bp when the
  regime tag is non-trivial. K6 noop-fix precedent mitigation.
- Tester M-FINAL: V-REG + T-REG verdicts per ADR-0049 § D4 joint
  table; routes R-O1/R-O2/R-O3/R-O4 per feature.md § 4-cell verdict
  tree.

### Risk re-assessment after M-OD overrides

- **K3 (latency)** — Markov-switching forward filter is O(K² × T)
  per bar with K=4; should be < 1 ms/bar at hourly cadence on 10
  pairs. Wave A measures; if exceeded, fall back to cached regime at
  24-bar coarser cadence (per K3 mitigation in feature.md).
- **K-reg-2 (dispatcher two-stage error compounding)** — explicitly
  mitigated by D6 max-confidence gate. Compounds Wave A test surface
  but unlocks Q4=(b).
- **K4 (embedding determinism)** — Wave B contract above; escape
  hatch declared.
- **K6 (noop-fix precedent)** — Wave F e2e divergence gate is the
  load-bearing CLAUDE.md non-negotiable. Mandatory from day 1.

### Cross-references

- ADR-0049 — [`spec/architecture/adr/0049-v3-regime-classifier-markov-switching-verdict-shape.md`](../architecture/adr/0049-v3-regime-classifier-markov-switching-verdict-shape.md) (primary)
- ADR-0038 — V-VOL verdict shape sibling — [`spec/architecture/adr/0038-vol-forecast-verdict-shape.md`](../architecture/adr/0038-vol-forecast-verdict-shape.md)
- ADR-0033 — F-verdict IMMUTABLE — [`spec/architecture/adr/0033-tcn-alpha-investigation-report-shape.md`](../architecture/adr/0033-tcn-alpha-investigation-report-shape.md)
- Load-bearing seed — [`crates/reflection/src/regime.rs`](../../crates/reflection/src/regime.rs)
- K4 embedding ordinal site — [`crates/reflection/src/embedding.rs#L120-L126`](../../crates/reflection/src/embedding.rs)
- Noop-fix precedent (K6) — [`spec/v3-volatility-forecaster-noop-fix/feature.md`](../v3-volatility-forecaster-noop-fix/feature.md)
- e2e divergence test pattern (mandatory CLAUDE.md non-negotiable) — [`crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`](../../crates/strategy/tests/vol_targeting_overlay_end_to_end.rs)

## Implementation

_Developer Waves A-D populate after architect M-T1 close._

## Verification

_Tester M-FINAL populates after developer waves close. Required
gates: R-NR.6 e2e divergence (mandatory CLAUDE.md non-negotiable),
K2 switch-rate, H2 accuracy, H1 Sharpe-delta._

## Historical context (2026-05-22 analyst pass)

The 2026-05-22 spec-only design exploration authored a more granular
register: R1-R8 / H1-H6 / Q1-Q7 / K-reg-1..K-reg-6 + an 8-item
non-regression contract + an explicit deferred-milestone activation
contract. That register was authored when C1 was still in-flight and
C5 was a parallel-spec sibling. The M-A5 refresh of 2026-05-28 narrows
to canonical M0 shape (R1-R5 / H1-H4 / Q1-Q5 / K1-K6) because the
sibling lanes have settled — C1 RETIRED, C5 partial-shipped — so the
brief no longer needs to enumerate cross-sibling open-question rows
(those resolved as gates fired).

**Carry-forward findings from 2026-05-22 (verified unchanged at this
refresh):**

1. **Load-bearing seed unchanged.** `crates/reflection/src/regime.rs`
   was last touched 2026-05-22; `RegimeTag { Bull, Bear, Chop }` enum,
   `REGIME_THRESHOLD_RATIO = dec!(0.02)` const, `classify_regime`
   pure-fn signature all byte-identical to the 2026-05-22 reading.
   R1 invariant fully preserved.
2. **7+ downstream consumers unchanged.** `embedding.rs:24` consumer
   pattern unchanged; lesson-card embedding determinism intact (K4
   mitigation holds).
3. **The strategy crate has NOT acquired any regime-related code in
   the interim.** `grep -rn "RegimeTag\|classify_regime\|regime_tag"
   crates/strategy/src` → 0 hits. C2 v0.1.0 introduces the first
   strategy → regime tag consumer.
4. **C1 + C5 settled the cross-sibling uncertainty.** The 2026-05-22
   brief deferred Q-resolution pending C1 verdict; with C1 RETIRED
   NEGATIVE-NET-DELTA and C5 shipped partial-approved, the activation
   gate has fired per the 2026-05-22 brief's § Deferred milestones
   contract. M-A5 light-touch refresh is the correct workflow path.
5. **Q-mapping (2026-05-22 → 2026-05-28).** 2026-05-22 Q1 ↔ 2026-05-28
   Q1 (taxonomy; default carries (a) 3-state). 2026-05-22 Q2 ↔
   2026-05-28 Q3 (model class; default carries (a) HMM with (d)
   fallback). 2026-05-22 Q3 ↔ 2026-05-28 Q3 implicit-nowcasting (M-A5
   collapses Q3 lookback-horizon into Q3 model-class — nowcasting is
   the v0.1.0 default; forecast-mode is a v0.2.0 question). 2026-05-22
   Q4 ↔ 2026-05-28 Q4 (integration mode; default carries (a)
   overlay). 2026-05-22 Q5 absorbed into pre-drawn 4-cell verdict
   tree above. 2026-05-22 Q6 ↔ 2026-05-28 implicit anchor namespace
   (default `v2.7.0-regime` preserved). 2026-05-22 Q7 ↔ 2026-05-28
   implicit extend-in-place (default carries — codified in R1
   invariant directly).

The full 2026-05-22 R1-R8 / H1-H6 / Q1-Q7 / K-reg-1..6 register is
preserved in git history at commit `cf7015c` ancestry; the M-A5
refresh tightens to operator-actionable M0 shape and supersedes the
2026-05-22 register for architect M-T1 consumption.

## Changelog

- 2026-05-22 (analyst): spec-only design exploration authored under
  Q-SEQ HYBRID; full R1-R8 / H1-H6 / Q1-Q7 register; deferred-milestone
  activation contract. NO code commitment until activation gate.
- 2026-05-28 (analyst): **M-A5 light-touch refresh**. Promoted Queue →
  Active per operator routing (v2.5 TCN re-investigation halt; C1
  RETIRED; C5 partial-shipped). Narrowed to canonical M0 shape: R1-R5
  + R-NR + K1-K6 + H1-H4 + Q1-Q5 + 4-cell verdict tree + cost framing.
  Frontmatter `version 0.1.0` retained (no behaviour change — this is
  the same v0.1.0 brief, refreshed). `status: draft` (operator-decide
  M-OD opens Q1-Q5). Trace row `REQ-V3-REGIME-CLASSIFIER-001`
  flipped `draft → proposed`. Anchors baseline 70/70 PASS pre-spec
  confirmed.
- 2026-05-28 (operator): **M-OD closed at commit `6b47027`** —
  Q1=(b) 4-state Bull/Bear/Volatile/Calm (override; bolder than
  3-state default); Q2=(a)+(c) 2023 train / 2024 val on existing
  realdata (default); Q3=(b) Markov-switching regression (Hamilton 1989,
  override; bolder than HMM default); Q4=(b) strategy-switching
  dispatcher (override; bolder than overlay default); Q5=(b) all 10
  USDT pairs (default). Cost framing revised ~5-7 weeks (was ~3-4 wks).
- 2026-05-28 (architect): **M-T1 closed**. ADR-0049 authored
  ([`spec/architecture/adr/0049-v3-regime-classifier-markov-switching-verdict-shape.md`](../architecture/adr/0049-v3-regime-classifier-markov-switching-verdict-shape.md)
  — sibling to ADR-0038, NOT extension; ADR-0033 § D3 IMMUTABLE).
  Resolved three load-bearing questions: (A) dispatcher prerequisite =
  option (i) degenerate CashHoldStrategy for Volatile/Calm (positions
  HELD not LIQUIDATED; v0.2.0 v1.5-MR follow-on fills seam); (B) K4
  RegimeTag encoding = option (γ) preserve Chop + append Volatile=3,
  Calm=4 (new classifier emits only 4 Q1=(b) variants); (C)
  Markov-switching priors = operator-set semantic identities {Bull,
  Bear, Volatile, Calm} × {μ_s, σ²_s} priors per ADR-0049 § D1 table;
  Baum-Welch refines parameter values only (no post-hoc state-label
  reassignment). Anchor namespace bumped `v2.7.0-regime → v3.0.0-regime`
  per D5. 4 new anchors planned (70 → 74). 6-wave M-DEV decomposition
  (A-F). Trace row `REQ-V3-REGIME-CLASSIFIER-001` flipped
  `proposed → arch-done`. Frontmatter `owner: architect → developer`
  (no UI surface at v0.1.0 beyond Wave D Trail column extension —
  handled by developer not ui-designer per single-track scope).
