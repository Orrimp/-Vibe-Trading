# Application — Volatility, Regimes & Sizing Overlays

> Decision doc for analyst + architect. Distilled from
> `research/crypto-market-structure/{knowledge.md,papers.md}` (100 entries) and
> `research/SYNTHESIS.md`. Citations are `crypto-market-structure[N]` → see
> `papers.md`. Cross-ref `research/risk-and-sizing/` for the sizing-overlay
> mechanics; this doc covers the **crypto-specific** vol structure and what it
> means for an overlay on our advisor.
>
> **Bottom line up front:** the single most-replicated finding across this entire
> topic is **volatility/regime is forecastable; direction is ~random-walk**
> [13][36][54][64][65][72][73][83][87][88]. That points to **risk-sizing / regime
> overlays, not return-timing bets**. But two hard caveats follow directly from
> the corpus: (1) in crypto the return→vol "leverage effect" is **contested / often
> reversed** (positive returns can raise future vol — retail chasing), so an
> overlay must **promise drawdown/tail reduction, NOT a Sharpe win**, and must NOT
> hard-code an equity-style asymmetry sign; (2) even a well-built vol overlay
> **rarely clears the bootstrap vs holding** — DVOL was already probed
> (`v0.dvol_regime`) and came back FRAGILE.

---

## 1. Summary of the research

**Vol is real, persistent, long-memory, multifractal, heavy-tailed — and
genuinely forecastable.** BTC/ETH return tails are inverse-cubic (mature-market-
like) [6][54]; vol clustering has power-law (long-range) autocorrelation [6];
realized vol is **multifractal**, so single-parameter "rough vol" (single Hurst)
models *mis-specify* it [5]. Forecastability is concrete: LightGBM with 69
features hits R²≈0.67, but the dominant predictors are just **lagged realized
variance (the HAR core) + volume + Google-search attention** — a simple HAR/EWMA
captures most of it; ML adds incremental R² at overfit risk [73]. Probabilistic
quantile forecasts (QRS on log-RV) give well-calibrated bands for sizing [73].
Classical ARIMA/GARCH give "meaningful" short-run *vol* forecasts but explicitly
**cannot forecast price levels beyond near-term** [88]; GARCH/EGARCH beat naive
historical/EMA [47].

**Crypto vol asymmetry runs the other way from equities — and its sign is
contested.** At high frequency crypto shows an **inverse leverage effect**
(positive returns raise future vol — retail buying-the-dip / chasing pumps),
*lower* persistence (transient regimes), and **jump-dominance** — so prefer a
*signed-semivariance, jump-aware, fast-decaying* HAR over an equity EGARCH [87].
But the sign is genuinely contested: [87] finds inverse, [88] finds standard
(negative-shock) EGARCH asymmetry, [47] finds **none**. **Do NOT hard-code an
asymmetry sign.** (This converges with `risk-and-sizing` SYNTHESIS: the
return→vol leverage effect is γ=−0.261 crypto vs +0.115 equity — *reversed* — so
vol-targeting is a risk tool, not a Sharpe tool.)

**Crypto options / DVOL carry VOLATILITY information, not DIRECTION — and DVOL is
biased high vs realized AND noisy in the wings.** IV slopes/skew forecast realized
*volatility* but **NOT returns** [65] — options trading is vol-informed, not
directionally-informed. There is a large positive variance risk premium (BVRP ≈
+14%/yr, ~7× the S&P's [45]) plus an extra clustered-jump premium [55], so implied
vol (DVOL) sits **above** subsequent realized vol — DVOL is **NOT an unbiased vol
forecast**; GARCH/HAR forecast realized vol as well or better [47][65][99]. Crypto
options markets are *thin*, so DVOL is distorted exactly in the **wings** (extreme
moneyness/maturities) where tail info matters [99]. Deribit quotes are *inverse*
options — consume DVOL as published, don't re-invert IV naively [49]. (This is the
ADR-0072 lesson, now corpus-grounded.)

**Regimes are detectable but non-persistent → switching is costly.** Bayesian
4-state NHHM separates bear/bull/calm and beats a random walk on forecast error,
but crypto regimes are **not persistent** (frequent state alternation, unlike
sticky FX regimes) [27] — so a regime-switch overlay trades frequently → high
turnover → cost drag, the exact failure mode in [10]. Efficiency itself is
dynamic (Adaptive Market Hypothesis), tied to liquidity, and **higher for large
caps** — so the coins we target are *harder* to beat [17][18][54].

**On-chain & cross-asset signals forecast vol regimes (not direction).** On-chain
tx-graph structure forecasts vol regimes [13]; exchange-inflow + whale transfers
forecast vol spikes (F1≈0.46) [36]; equity crash-risk spills into BTC **volatility
but NOT returns** [64]; macro event-risk (Fed/CPI/recession) forecasts crypto
**vol** [72]. All reinforce the same split: condition *size* on these, don't *time*
direction.

**Crashes are correlated, clustered, self-exciting — normality under-states tail
risk.** Cross-coin correlations surge to ~0.8–0.9 in crashes [28]; depeg/jump
events self- and cross-excite (Hawkes) [16][55]; perp liquidation cascades + ADL
dump into spot with slippage **5–10× normal** [33][37]; on BitMEX ~3.5% of *longs*
are force-liquidated *daily* at ~60× leverage [71]. The right tail machinery is
**GARCH + EVT + copula** (robust RVaR), not Gaussian VaR [71][84]; full-sample
correlation *understates* the stress coupling that hits when diversification is
most fragile, and crypto is a net *receiver* of cross-asset shocks [97]. A
**moving-block bootstrap (preserves clustering) is the right gate** — which we
already use.

---

## 2. Possible solutions / what can be done with this research

1. **A loose, slow, de-risk-only vol-targeting overlay** sized on a robust
   realized-vol estimate (HAR/EWMA, ~long half-life), with a no-trade band, NOT
   chasing the target. Promise drawdown/tail reduction; report
   Sortino/CVaR/median, not just Sharpe. (Mechanics in `risk-and-sizing` SYNTHESIS:
   open-loop preferred — closing the feedback loop blew turnover to ~1105%/yr.)
2. **A DVOL/skew regime overlay** as a *supplementary* risk-sizing input with the
   variance/jump-premium bias removed — never entry timing. **Already built
   (`v0.dvol_regime`) and FRAGILE** — the corpus explains why (biased high, noisy
   wings, vol-not-direction), and the honest next move is to leave it as a
   documented null and prefer a HAR/GARCH realized-vol baseline [73][87][99].
3. **A macro / equity risk-state vol overlay** (cut size into high macro-uncertainty
   / equity-crash-risk states) — `v0.macro_riskon` is built and FRAGILE [72][93]
   [96]; equity crash-risk → BTC vol is the cleanest cross-asset version [64].
4. **A regime-flat (bull/bear) overlay with hysteresis** to de-risk to cash, with
   an explicit switching penalty + detection-lag model, OOS-CV params, event-driven
   re-baking — tested through the gate [27]. (See `strategies`/`evolution` for the
   jump-model variant.)
5. **A froth/crash-warning de-risk-near-tops overlay** — LPPLS [12][58] or the more
   noise-robust TDA persistence-norm [98] — but only if its **full** signal history
   (misses + false alarms) clears the gate; neither has a published false-positive
   rate.
6. **Harden the gate's risk assumptions** to match crypto vol structure: heavy-tailed
   (inverse-cubic / q-Gaussian) tails, clustered self-exciting crashes, cascade
   slippage 5–10×, super-linear price impact — so the bootstrap doesn't implicitly
   assume thin tails + linear slippage [6][33][55][71][84].

---

## 3. Relevance for the project

- **Overlays are bake-off candidates, gated like everything else.** A vol/regime
  overlay is a sizing-modifier; per the CLAUDE.md non-negotiable it ships with a
  **day-1 baseline-equity-divergence e2e** and is ranked under the FROZEN gate vs
  buy-and-hold. The corpus says the honest prior is "reduces drawdown, does not
  beat hold on a path-distribution Sharpe test."
- **"Size on vol, don't bet on direction" is the durable, plausible framing.** It
  is the most-replicated result in our own asset class and the one an overlay can
  honestly act on. Direction-timing overlays contradict the corpus and the gate.
- **DVOL and macro arms are already FRAGILE — the corpus tells us why, which is
  itself product.** The advisor can say "we ingested implied vol and a macro
  risk-state, sized on them through our gate, and neither robustly beat holding,
  because the literature shows DVOL is premium-biased + vol-not-direction and the
  macro channel fails out-of-sample." That is traceable, plausible honesty.
- **The leverage-effect contest is a design constraint, not trivia.** Because the
  sign is contested [47][87][88], any overlay must measure each coin's
  return→vol correlation *per-window* and must not bake in an equity asymmetry —
  otherwise it is fitting a sign the data may not support.

---

## 4. Advantages for the project

- **Forecastable vol is a real, defensible lever** (R²≈0.67 [73]) — the one
  numeric target the corpus says is genuinely predictable, ideal for a *risk*
  overlay even when it can't beat hold.
- **A simple HAR/EWMA captures most of it** [73][87] — fits our "simple beats
  fancy" stance, no deep net needed, cheap to compute in `Decimal`.
- **Calibrated probabilistic vol bands** (QRS) give principled sizing inputs [73].
- **Crypto-specific corrections to equity priors** (inverse/contested leverage
  effect, low persistence, jump-dominance, reverse skew) keep us from importing a
  wrong equity vol model [8][45][47][87].
- **Right tail machinery already aligned** — our moving-block bootstrap matches the
  clustered, self-exciting, heavy-tailed reality [16][28][55][71][84], and the
  corpus tells us exactly how to harden the cost/slippage side (cascade 5–10×,
  super-linear impact).
- **Honest overlays as coverage** — DVOL + macro FRAGILE results are defensible
  lines in the advisor's honesty story.

---

## 5. Problems and challenges

- **An overlay can be a no-op and still pass unit tests + anchored reports** (the
  DVOL `v3-volatility-forecaster-noop-fix` lesson; CLAUDE.md non-negotiable). A
  `scale` computed but never *applied* does nothing. **Mandatory day-1
  baseline-equity-divergence e2e** proving the overlay's output equity diverges
  from the un-targeted baseline by ≥ a testable epsilon. Patterns:
  `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`,
  `crates/backtest/tests/dvol_regime_divergence_end_to_end.rs`.
- **Vol-targeting does NOT raise Sharpe in crypto** — its gain comes from the
  negative return→vol leverage effect, which is reversed/contested in crypto
  [47][87] (`risk-and-sizing` γ=−0.261). Promising a Sharpe win would be
  dishonest; promise drawdown/tail reduction and measure Sortino/CVaR.
- **Chasing the vol target explodes turnover** (~1105%/yr; `risk-and-sizing`
  SYNTHESIS) → cost drag kills it [10][83]. Use a slow EWMA + no-trade band +
  de-risk-only; open-loop, not closed-loop.
- **Regime non-persistence → switching cost** [27]; large-cap efficiency rises so
  there is less to exploit [17][18]. Frequent regime flips are the [10] failure
  mode.
- **DVOL is biased and noisy by construction** — premium-biased high [45][55],
  distorted in the wings [99], inverse-option subtlety [49]. Don't feed it raw as
  an unbiased forecast; prefer a HAR/GARCH realized-vol baseline.
- **Don't hard-code an asymmetry sign** — contested [47][87][88]; measure per-coin,
  per-window.
- **LPPLS/TDA crash detectors have no published false-positive rate** [58][98] —
  prone to researcher-degrees-of-freedom overfit; must be evaluated on the full
  signal history under the gate before trusting.
- **HARD CONSTRAINTS:** `Decimal` not `f64` (vol/scale are dimensionless, never
  enter P&L); if an overlay consumes an exogenous series (DVOL/macro) it goes
  through `core::pit::PitSeries` (ADR-0058) with a SHA-pinned corpus +
  `REVISION.toml` and a divergence e2e; `ui` must NOT depend on
  `strategy`/`exec`/`llm`; anchored SHAs byte-immutable (119/119); gate + bands
  FROZEN; paper-only.

---

## 6. Concrete next steps / candidate work items

| # | Item | Verdict | Where | Priority |
|---|------|---------|-------|----------|
| A | **Harden the gate's risk/cost assumptions to crypto vol structure**: heavy-tailed tails, clustered self-exciting crashes, cascade slippage 5–10×, super-linear size impact, widen effective spread in high-vol regimes. Additive; bands FROZEN. | **WORTH DOING** (highest-leverage; may flip marginal picks to FRAGILE) | `crates/backtest/src/bakeoff/{bootstrap.rs,rank.rs}` + cost model | **P1** |
| B | **Vol-targeting overlay repositioned as a risk tool** (loose, slow EWMA/HAR realized-vol, no-trade band, de-risk-only, open-loop), measured on Sortino/CVaR/median + drawdown, gated vs hold. Day-1 divergence e2e. | **PROBE-WORTHY** (expect drawdown reduction, NOT a Sharpe win / NOT a hold-beater) | `crates/strategy/src/vol_targeting_overlay.rs` (exists) + report | **P1** |
| C | **DVOL regime overlay** | **ALREADY BURNED — FRAGILE** (`v0.dvol_regime`). Leave as documented null; prefer HAR/GARCH realized-vol baseline [73][87][99]. Do NOT re-tune to chase a win. | `crates/strategy/src/dvol_regime.rs`, `crates/backtest/src/dvol_data.rs` | **done** |
| D | **Macro / equity risk-state vol overlay** | **ALREADY BURNED — FRAGILE** (`v0.macro_riskon`, ADR-0073). Equity-crash-risk→BTC-vol [64] is the cleanest variant; leave as documented null unless cost-hardening (A) changes the picture. | `crates/backtest/src/macro_regime.rs` | **done** |
| E | **Regime-flat (bull/bear) overlay with hysteresis** — jump-model de-risk-to-cash, explicit switching penalty, detection-lag, OOS-CV params, event-driven re-bake. Day-1 divergence e2e. | **PROBE-WORTHY** (expect cost drag from non-persistence [27]; honest prior FRAGILE) | new overlay in `crates/strategy/src/` + `crates/backtest/` scenario | **P2** |
| F | **HAR/EWMA realized-vol estimator as the primary vol input** for B/E (lagged RV + volume + attention features [73]; signed-semivariance, jump-aware, fast-decaying [87]; measure per-window return→vol sign — don't hard-code). | **WORTH DOING** (underpins every honest overlay; simple beats fancy) | `crates/forecast/` or `crates/strategy/` vol module | **P2** |
| G | **Froth/crash-warning de-risk overlay** (TDA persistence-norm > raw LPPLS [98][12][58]) | **PROBE-WORTHY but heavily caveated** — must prove a false-positive rate on the full signal history under the gate first; high overfit risk. | new overlay + scenario | **P3** |

---

## 7. Open questions for analyst & architect

1. **Does cost-hardening (A) belong in this round, and does it stay additive to the
   FROZEN gate?** It is the highest-leverage change and may flip marginal strategies
   to FRAGILE — but it touches the cost/slippage model feeding the gate. Confirm
   bands untouched. *(Recommended: yes — it is the durable correctness fix and makes
   every subsequent verdict more honest.)*
2. **Is a repositioned vol-targeting overlay (B) worth shipping if the honest prior
   is "reduces drawdown, doesn't beat hold"?** The advisor's value is honesty, and
   a drawdown-reducing-but-not-hold-beating overlay is a legitimate, traceable
   result — but it is not a "winner." Frame as risk tool or skip?
3. **Should DVOL/macro FRAGILE arms (C/D) be re-run after cost-hardening (A)?** The
   verdict could shift; but re-tuning to chase a win violates the pre-registered-null
   discipline. Re-run once, report, don't tune.
4. **Which crate owns the realized-vol estimator (F)** — `forecast` or `strategy`?
   It must not pull `ui` into a `strategy`/`exec` dependency.
5. **What false-positive-rate bar must a crash-warning overlay (G) clear** before it
   is even gate-eligible, given LPPLS/TDA have none published [58][98]?
6. **Per-coin, per-window leverage-effect sign**: where is the return→vol
   correlation measured and surfaced so an overlay never hard-codes an equity
   asymmetry [47][87][88]?

---

## 8. What NOT to do / out of scope

- **Do not sell any vol/regime overlay as a Sharpe-improver or hold-beater** — the
  corpus says it reduces drawdown at best [47][83][87].
- **Do not chase the vol target** (closed-loop) — turnover explodes; use slow
  open-loop + no-trade band, de-risk-only.
- **Do not feed DVOL raw as an unbiased vol forecast** — premium-biased, noisy
  wings, inverse-option; prefer HAR/GARCH realized-vol [45][49][99].
- **Do not hard-code an asymmetry sign** in the vol model — contested [47][87][88].
- **Do not use vol/regime/options signals to time DIRECTION** — they forecast vol,
  not returns [64][65][88].
- **Do not trust a crash-warning detector on cherry-picked successful calls** —
  require the full signal history (misses + false alarms) through the gate [58][98].
- **Do not assume thin tails / linear slippage / calm-period correlation** in the
  gate — crashes are heavy-tailed, clustered, self-exciting, 5–10× slippage, and
  correlations surge to ~0.9 [6][28][33][71][84][97].
