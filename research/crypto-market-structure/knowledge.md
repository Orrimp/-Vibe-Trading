# Knowledge — Crypto Market Structure

Synthesis of the `crypto-market-structure` ledger. Updated incrementally.
Our app: a long-only, single-coin, paper-sim crypto advisor that bakes off
strategies and ranks them under a frozen 1000-path moving-block-bootstrap
robustness gate, with buy-and-hold as the permanent benchmark. Validated thesis:
*no active strategy robustly beats holding, net of costs.* Read every takeaway
through that lens.

Coverage so far: [1]–[100]. (Round 3 complete; **target 100 reached**.)

## TL;DR — candidate exogenous-signal arms vs dead ends (for the impatient)

After 100 papers, the picture is consistent and our thesis survives. The few things
worth a *probe* in our pipeline (all as **risk-sizing / regime overlays, tested
through the bootstrap+cost gate vs buy-and-hold — none as standalone return-timing
alpha**):

- **CANDIDATE — USDT exchange-inflow flows.** Stablecoin "dry powder" arriving at
  exchanges positively predicts BTC/ETH returns and lowers vol [68]; Tether flows
  arrive after downturns and (claimed) predict positive subsequent BTC returns [100/30].
  The one genuinely *exogenous, demand-side* on-chain signal that isn't price-derived.
  Caveats: intraday horizon, paid feed, no robustness gate yet.
- **CANDIDATE — macro risk-on/off (Fed/CPI/recession) as a VOL overlay.** Macro
  event-risk repricing forecasts crypto *volatility* [72]; hawkish policy + inflation
  surprises are BTC headwinds [93][96]; BTC is risk-on, not a hedge [93][96]. Feeds our
  macro-risk-on probe arm — but the strongest (monetary-policy) channel *fails OOS*
  outside the rate-cut window [72], so expect regime instability.
- **CANDIDATE — funding-rate sign/level as a froth/sentiment gauge.** Extreme positive
  funding = crowded longs [4][66]; the one directional read of funding accessible to a
  long-only holder (we can't harvest carry). Source from reconcilable CEX venues —
  funding *and* open interest are misquoted on some exchanges [91].
- **CANDIDATE (weak) — on-chain valuation (MVRV-Z/NUPL) + Metcalfe/CVALUE, and a
  TDA/LPPLS froth detector.** Still the [40] hypothesis to falsify; TDA is a more
  noise-robust crash-warning input than raw LPPLS [98] but unproven on false-positives.
- **DEAD ENDS confirmed:** social-media sentiment & Fear&Greed (endogenous to price,
  no Granger causality, no OOS [52][74]); cross-venue/cross-chain arbitrage (specialist-
  captured, friction-bounded [85][86][91]); ML *return* prediction without cost-aware
  execution (collapses on costs; even cost-aware doesn't beat hold by bootstrap [83]);
  Bitcoin as inflation/crisis hedge (it's risk-on [93][96]); DeFi LP yield (loses to
  holding [53][63]); calendar/intraday seasonality as alpha (arbitraged away [94]).

**Headline:** the strongest, most current external replication of our entire project is
[83] — same asset (BTC), same method (walk-forward + bootstrap vs buy-and-hold), same
verdict: cost-aware ML restores returns but shows **no statistically significant Sharpe
outperformance over buy-and-hold**. The whole topic converges on one rule:
**volatility/regime is forecastable; direction is ~random-walk → build risk-sizing
overlays, not return-timing bets.**

## Key themes

1. **Funding / basis carry is the single highest-Sharpe *structural* edge in crypto
   — but it is market-neutral and out of our long-only scope.** Independent venues
   agree: long spot / short perp to harvest funding yields high Sharpe (BTC ~1.9 net
   of high fees [1]; in-sample 7–10 [3][4]); it is uncorrelated with HODL, a genuine
   diversifier [41]. It exists because retail crowds the *long* side (negative
   convenience yield [4]) and is risk-compensated, not free (limits-to-arbitrage,
   liquidation/ADL risk [4][37]). Capturing it needs a perp/short/margin engine we
   don't have.

2. **Carry/funding level is a directional *sentiment* proxy.** Persistently high
   positive funding/carry = aggressive leveraged-long crowd = froth — the part of the
   carry literature that *could* inform a long-only holder as a caution overlay,
   though the arb trade itself is out of scope [3][4]. Early evidence supports a causal
   funding↔price link and funding-as-trend-gauge, but with no robust net-of-cost timing
   strategy [66]. **Sourcing caveat:** funding rate *and* open interest are systematically
   **misquoted** by some major exchanges (Bybit/OKX worst; Kraken/HTX most reconcilable),
   so build any funding/OI overlay from reconcilable venues and treat OI levels skeptically
   [91]. Carry's full-sample Sharpe (~6.5) has fallen post-2024 and turned negative in 2025
   per recent backtests — even the structural edge is regime-dependent and decaying [67].

3. **Crypto "passive yield" sources are short-vol / short-tail trades in disguise.**
   AMM liquidity provision (impermanent loss) is a continuous adverse-selection bleed:
   LVR = (σ²/8)·pool-value per unit time for a constant-product pool [35]. Funding
   carry is compensation for crowded-long + liquidation risk [1][3][4]. None is free
   money; each is risk-compensation that our heavy-tailed regimes would punish.

4. **Crypto volatility is real, persistent, long-memory, multifractal, heavy-tailed —
   and genuinely forecastable.** BTC/ETH return tails are inverse-cubic (mature-market-
   like) [6]; vol clustering has power-law (long-range) autocorrelation [6]; realized vol
   is multifractal, so single-parameter "rough vol" models mis-specify it [5]. Use HAR /
   regime / multiscale vol models, heavy-tailed risk, never Gaussian VaR. **Forecastability
   is real:** LightGBM with 69 features hits R²≈0.67, but the dominant predictors are just
   *lagged realized variance (the HAR core) + volume + Google-search attention* — a simple
   HAR/EWMA captures most of it; ML adds incremental R² at overfit risk [73]. Contra equity
   priors: at high frequency crypto shows an **inverse leverage effect** (positive returns
   raise future vol — retail chasing/buying-the-dip), *lower* persistence (transient regimes),
   and **jump-dominance** [87] — so prefer *signed-semivariance, jump-aware, fast-decaying*
   HAR over an equity EGARCH. **The leverage-effect SIGN is genuinely contested** — [87]
   finds inverse, [88] finds standard (negative-shock) EGARCH asymmetry, [47] finds none —
   so do NOT hard-code an asymmetry sign. Probabilistic/quantile vol forecasts (QRS on
   log-RV) give well-calibrated bands for sizing [73]. Classical ARIMA/GARCH give
   "meaningful" short-run vol forecasts but explicitly **cannot forecast price levels
   beyond near-term** [88], and macro prediction-market signals (Kalshi) add vol-forecast
   info orthogonal to HAR and to the options surface [72].

5. **Crypto options / DVOL carry VOLATILITY information, not DIRECTION — and DVOL is
   biased high vs realized AND noisy in the wings.** Bitcoin shows a forward/reverse vol
   skew (OTM calls richer), commodity-/upside-demand-like, opposite to equities [8];
   Bitcoin lacks the equity "leverage effect" (symmetric conditional-vol response) [47].
   Critically, IV slopes forecast realized *volatility* but NOT *returns* — options trading
   is vol-informed, not directionally-informed [65]. There is a large positive variance risk
   premium (BVRP ≈ +14%/yr, ~7× the S&P's [45]) plus an extra clustered-jump premium [55],
   so implied vol (DVOL) sits *above* subsequent realized vol — DVOL is NOT an unbiased vol
   forecast; GARCH/HAR can forecast realized vol as well or better [47][65][99]. **New
   caveat:** crypto options markets are *thin*, so implied vol (incl. DVOL) is distorted at
   extreme moneyness/maturities — DVOL is noisy exactly in the wings where tail info matters
   [99]. Deribit quotes are *inverse* options (handle IV inversion carefully; consume DVOL
   as published) [49], and the same Deribit options imply a large, time-varying, venue-
   specific USD "risk-free" rate (~0–20%+) [92]. **ADR-0072 lesson:** use DVOL/skew as a
   *noisy* risk-sizing/regime input with the variance/jump premium in mind, never as a
   return-timing signal, and prefer a HAR/GARCH realized-vol baseline as the primary
   estimate [73][87][99].

6. **Microstructure & cross-venue/arbitrage edges exist statistically but die on costs
   / are captured by specialists.** OFI, spread, VWAP-deviation are the robust LOB
   features [9], but HFT microstructure strategies have deeply negative net Sharpe
   (turnover 100–200×/day) [10]; gradient boosting overfits [10]. At LOB scale, **simple
   models (XGBoost/logit) match deep nets and handcrafted order-imbalance features match
   CNN-learned ones — "better inputs > deeper model," with a modest 42–71% accuracy
   ceiling and limited alpha after costs** [79]. Order-*flow* (event) representations
   generalize across regimes far better than order-book *level* snapshots — prefer
   change/flow features for robustness [80]. Cross-exchange deviations stay inside
   fee-defined no-arb bands and mean-revert fast [42]; even triangular arbitrage is
   net-unprofitable for the average sophisticated trader [26]; MEV/sandwich/liquidation
   value accrues to searchers/builders, not retail [21][22], including **cross-chain
   arbitrage** (specialist-captured, bridge/capital barriers, retail-inaccessible [86])
   and **sandwiching** (DEX execution tax; even "private" routing isn't safe; one bot =
   ⅔ of private front-runs [85]). Cross-country premiums measure capital controls [44].
   **Most-current direct rebuttal:** cost-aware ML on hourly BTC restores returns only
   via a trade-suppression filter, yet shows **no statistically significant Sharpe
   outperformance over buy-and-hold** by bootstrap — execution discipline matters more
   than the model, and the edge still doesn't beat holding [83].

7. **On-chain metrics carry orthogonal information — strongest for VOLATILITY/REGIME and
   DEMAND-SIDE flows, most-hyped for direction/cycle-timing.** Tx-network features add
   direction info beyond TA [11]; tx-graph structure forecasts vol regimes [13]; exchange-
   inflow flows + whale transfers forecast vol spikes (F1≈0.46, caught >61% of spikes) [36].
   **The cleanest exogenous on-chain *direction* signal: stablecoin (USDT) exchange-inflows
   positively predict BTC/ETH returns and lower vol** ("dry powder" arriving to buy) [68],
   echoed by the claim that Tether flows arrive after downturns and predict positive
   subsequent BTC returns [100/30] — unlike Fear&Greed [52] this is genuinely exogenous,
   not price-derived. **Important scope caveat:** ~75% of BTC activity is *off-chain*, so
   on-chain metrics see only a quarter of real flow; and *on-chain DEMAND* (active addresses,
   inflows, tx demand), not on-chain supply, is what carries price information [78] — prioritize
   demand-side metrics. The boldest claim: NUPL / MVRV-Z / CVDD threshold strategies beat
   buy-and-hold risk-adjusted over 3 cycles, MVRV-Z strongest [40] — but with very few
   independent cycles and no robustness gate, a hypothesis to falsify. The cross-sectional
   crypto value factor is **price-to-new-address (CVALUE)** [67] and NVT is the on-chain
   "P/E" [70] — both theory-motivated valuation ratios to *test*, not trust. Metcalfe-
   fundamental + LPPLS flags bubbles/tops ex-ante [12][58]; **topological data analysis
   (TDA persistence-norm) is a more noise-robust crash-warning input than raw LPPLS, tested
   on hourly BTC** [98] (still no false-positive-rate / OOS validation — same caveat). CAUTION
   on "fundamental floors": **miner/production cost is NOT a price floor — price drives cost**
   [48], so miner-cost / Puell / hash-ribbon signals are price-derived (circular). The
   recurring meta-theme across on-chain, sentiment, and options data: **volatility/regime is
   forecastable; direction is ~random-walk** — build risk-sizing overlays, not return-timing
   bets.

7b. **"Sentiment signals" are mostly endogenous or cost-effects, not alpha.** The Crypto
   Fear & Greed Index does NOT Granger-cause returns and gives no OOS gain — returns
   cause sentiment, so FGI is lagged price [52]. Social-media sentiment (Twitter/Reddit)
   likewise shows **no Granger causality to returns** (returns→sentiment), improves only
   coarse directional accuracy not return magnitude, and reports no net-of-cost result [74]
   — the same endogeneity trap. A causal-Bayesian-network study finds **technical indicators
   carry the causal signal while added external/social features sometimes *hurt*** (no
   monotonic improvement, and no OOS/cost test) [95] — supporting parsimonious price-based
   strategies over feature-stacking. Extreme sentiment is a *liquidity-cost* regime (spreads
   widen; intensity, not direction) with no reported net-of-cost edge [50]. Funding/carry
   level remains the one structurally-grounded sentiment proxy (crowded-long froth) [3][4][66],
   but harvesting it needs a perp engine.

7c. **DeFi "passive yield" loses to holding, and crypto bears no intrinsic yield.** Three
   independent results — LVR theory [35], the Uniswap-v3 IL study (~49.5% of LPs negative;
   IL > fees) [53], and an LVR-vs-fees measurement (fees < arb losses in most large pools)
   [63] — agree that AMM liquidity provision is, in aggregate, a losing trade vs simply
   holding. BTC/ETH also bear **~zero intrinsic own-currency yield** (derivatives don't even
   fully price ETH's ~2% staking) [92], so buy-and-hold's *entire* return is price
   appreciation (no carry cushion), and every crypto "yield" is risk-compensated short-vol/
   credit [35][45][53][92]. This extends the ship-passive thesis to DeFi yield.

8. **Crypto crashes are correlated, clustered, self-exciting, and amplified by the
   leverage layer.** Cross-coin correlations surge to ~0.8–0.9 in crashes — divers/-
   ification collapses when needed [28]; herding (consensus-driven co-movement) is the
   behavioral mechanism, concentrating in stress and now propagated *into* crypto via the
   ETF complex [75][76]; depeg/jump events self- and cross-excite (Hawkes) [16]; perp
   liquidation cascades + ADL dump into spot with slippage 5–10× normal [33][37]; on BitMEX
   ~3.5% of *long* positions are force-liquidated *daily* at ~60× leverage [71]; DeFi
   liquidations over-sell collateral and can run toxic spirals [38][39]; non-custodial
   stablecoins have an endogenous tip-into-instability regime [43]. **Normality badly
   under-states crypto tail risk** — GARCH+EVT+copula (heavy-tailed, robust RVaR) is the
   right machinery [71][84]; full-sample correlation *understates* the stress-state coupling
   that hits exactly when diversification is most fragile, and crypto is a net *receiver* of
   cross-asset shocks (often imported from FX/macro stress) [97]. Moving-block bootstrap
   (preserves clustering) is the right gate; i.i.d./Gaussian is wrong.

9. **Bitcoin is risk-ON, not a safe haven / hedge — and increasingly equity- and macro-
   correlated.** It sold off WITH equities in COVID (no crisis-hedge value) [24], and
   post-Jan-2024 ETF approval its S&P 500 correlation jumped to a persistent positive level
   (structural break) [23], peaking ~0.87 in 2024 [56]. Gold correlation stayed ~0. Equity
   crash risk spills into BTC *volatility* but not BTC *returns* [64] — again the vol-vs-
   direction split. **Bitcoin is NOT an inflation hedge** (declines ~24 bps per SD of
   inflation surprise [96]) and **NOT a hedge against central banking** (highest returns in
   *dovish* regimes, underperforms on hawkish narratives — a "barometer of global liquidity,"
   no countercyclical protection [93]). Macro event-risk (Fed/CPI/recession repricing)
   forecasts crypto *vol* and is a BTC return headwind when tightening [72][93][96]. A
   monetary-policy-expectations index claims a 3–5-week *leading* relation to BTC returns
   [93], but the monetary channel **fails OOS outside the 2024–25 rate-cut window** [72] —
   regime-dependent. Drawdowns coincide with broad risk-off; diversification benefit is
   stale [23][24][56][97].

10. **Market efficiency is dynamic (Adaptive Market Hypothesis), tied to liquidity,
    and rises for large caps.** Predictability windows open and close [17][18]; BTC
    more efficient than ETH [17]; more liquidity → more efficiency → harder to beat
    [17][18]. So large-cap coins are *harder* to beat than small caps — set per-coin
    expectations, and don't assume a strategy stays robust across regimes.

12. **Cross-sectional factors (size/momentum/value/trend) work — but they are LONG-SHORT,
    multi-coin, out of our single-coin long-only scope, and weakening.** The canonical
    crypto factor model is C-4: market, size (small-caps underperform), momentum (~2-week
    winners continue), value (price-to-new-address) [67]; magnitudes *decline post-2020*.
    Aggregated multi-horizon **trend** (CTREND, 28 signals) earns ~3.87%/week long-short
    and survives costs in liquid coins [34]; momentum is significant and cost-survivable
    (IR 1.27 at 100 bps) **specifically in liquid coins**, while illiquid coins mean-revert
    [77]. ML cross-section adds economic gains but the simplest methods win (OLS > trees/NN),
    edges concentrate in hard-to-trade small/illiquid coins, and complexity barely helps.
    **For us:** the headline returns are long-short spreads (need shorting + a multi-coin
    book), so not directly usable; BUT the transferable lesson is that an **aggregated
    multi-horizon trend at a ~2-week lookback on liquid BTC/ETH** is the most credible
    candidate for a long-only trend overlay to test in the bakeoff — expecting far weaker
    capture than the long-short spread, and only in the *liquid* coins we already target.

11. **Data integrity is a first-class hazard — and it's not just volume.** Wash trading is
    >70% of reported volume on unregulated CEXs [19] and pervasive on DEXs (>30% of tokens)
    [20]; coordinated pump-and-dumps target small/low-liquidity coins [31][32]; 2017's BTC
    run was materially shaped by Tether-issuance flows [30]. Beyond volume: **open interest
    is systematically misquoted** by major exchanges (Bybit/OKX worst; Kraken/HTX
    reconcilable) [91]; **order-book depth is partly fake** — ~31% of large orders could
    *spoof* even Coinbase BTC/ETH books [90], and spoofing/layering was rife around the LUNA
    crash [89]. So volume, OI, *and* visible depth are all partly fictional → widen effective-
    spread assumptions, distrust venue-reported size, prefer clean major-venue feeds, and
    avoid small caps. (Data-source corollary: CEX leads price discovery and **BTC leads
    ETH** [69][90].)

## Methods / findings that hold up (and which don't)

- **Holds up:** funding/basis carry as a high-Sharpe market-neutral edge, uncorrelated
  with HODL [1][3][4][41][60] (but Sharpe falling post-2024, negative 2025 [67]); LVR =
  (σ²/8)·value as the AMM adverse-selection cost [35] (empirically: LP'ing loses to holding
  [53][63]); crypto bears ~zero intrinsic yield [92]; long-memory/multifractal vol
  clustering [5][6][54]; heavy (inverse-cubic / q-Gaussian) tails for large caps [6][54],
  with normality badly under-stating tail risk → use EVT/GARCH/copula [71][84]; vol is
  forecastable (HAR core: lagged RV + volume + attention; R²~0.67) [73][87]; OFI/spread/
  VWAP-deviation as dominant microstructure features, "better inputs > deeper model,"
  simple ≈ deep [9][79]; order-flow > order-book-level representations for regime robustness
  [80]; purged/walk-forward validation [10][83]; correlated/clustered/self-exciting crash
  behaviour [16][28][33][55][71]; herding propagated into crypto via ETFs [75][76]; BTC =
  risk-on, equity/macro-correlated, NOT a safe haven / inflation / central-banking hedge
  [23][24][56][64][93][96]; crypto a net *receiver* of cross-asset shocks; full-sample
  correlation understates stress coupling [97]; dynamic (AMH) efficiency tied to liquidity
  [17][18][54]; on-chain & options & macro signals forecasting *volatility* regimes (not
  direction) [13][36][64][65][72][73]; USDT exchange-inflows predicting returns/vol
  (demand-side, exogenous) [68][100]; cross-sectional size/momentum/value/trend factors
  (long-short, liquid coins) [34][67][77]; CEX leads price discovery, BTC leads ETH
  [69][90]; large positive Bitcoin variance risk premium (DVOL > realized) [45][55];
  stablecoin runs with $0.99 break-the-buck threshold, flight-to-safety, peg held by a
  thin/centralized arbitrageur layer [61][62][82][100].
- **Doesn't hold up / cautions:** rough-volatility (single Hurst) for BTC [5]; HFT
  microstructure alpha net of costs [10][79]; gradient boosting on microstructure /
  ML on *returns* (overfits; collapses on costs; even cost-aware doesn't beat hold by
  bootstrap) [10][83]; cross-asset model transfer in crypto (idiosyncratic) [10]; the
  leverage-effect *sign* (inverse [87] vs standard [88] vs none [47] — contested, don't
  hard-code); DVOL as an unbiased/clean vol forecast (premium-biased + thin-wing noise;
  HAR/GARCH ≥ DVOL) [45][55][99]; "risk-free" cash-and-carry (segmentation + liquidation/
  ADL risk) [4][37][60]; cross-exchange / cross-chain / cross-country / funding premiums as
  exploitable arb (fee bands / bridges / capital controls / forced exits) [42][44][51][86];
  triangular arb for the average trader [26]; on-chain *cycle-timing* outperformance [40] —
  unproven under a robustness gate; on-chain metrics as complete demand picture (~75%
  off-chain) [78]; **miner/production cost as a price floor** (price drives cost) [48];
  **halving as a reliable calendar edge** (delayed, ~1/5 of move, n≈2-3) [46]; **intraday/
  calendar seasonality as alpha** (arbitraged away; cost/execution hygiene only) [94];
  **Fear & Greed / social sentiment as alpha** (endogenous, no Granger causality, no OOS)
  [50][52][74][95]; **Bitcoin as inflation / crisis / central-banking hedge** (it's
  risk-on) [24][93][96]; **option skew/DVOL as a direction signal** (forecasts vol, not
  returns) [65]; reported volume / open interest / order-book depth as honest (all partly
  fake) [19][20][90][91].

## Actionable takeaways for our advisor

1. **The biggest documented crypto edges (carry, LP yield) are out of our long-only
   scope and/or are disguised short-vol/short-tail risk.** Be honest that the advisor
   cannot capture carry without a perp/short/funding engine [1][3][4][41]; and that
   AMM "yield" is an LVR bleed [35]. Logging this prevents a "why don't we beat hold?"
   rabbit-hole — the structural edge is market-neutral, not directional.
2. **Cost realism is decisive — and now externally replicated on our exact problem.** [10]
   and especially **[83]** are clean external replications of our thesis: statistical edge →
   economic failure once realistic fees + turnover apply. [83] is the closest mirror of our
   project (BTC, walk-forward + bootstrap vs buy-and-hold): a +73% paper edge becomes −64%
   at 10 bps; a cost-aware *trade-suppression filter* (act only on strong signals — a design
   pattern our bakeoff strategies should use) restores +65%, but **bootstrap shows NO
   significant Sharpe outperformance over buy-and-hold.** Keep the cost model central; widen
   effective spread in volatile/crash/extreme-sentiment regimes (adverse selection [9][50];
   cascade slippage 5–10× [33]; fake depth [90]).
3. **Use heavy-tailed, long-memory, correlated-crash assumptions in the robustness gate.**
   Moving-block bootstrap is appropriate; crashes are clustered, self-exciting, and
   synchronize all coins [16][28][33][43]. Super-linear price impact → harsher size
   penalties on crypto, much harsher on small caps [6].
4. **Candidate exogenous-signal arms to PROBE (all as risk-sizing/regime overlays through
   OUR gate vs buy-and-hold, none as standalone alpha), ranked by promise:**
   (a) **USDT exchange-inflow flows** [68][100] — the cleanest *exogenous, demand-side*
   signal (stablecoin dry powder → buying); not price-derived like FGI. Test as a buy/hold
   confirmation; caveats: intraday horizon, paid feed.
   (b) **Macro risk-on/off (Fed/CPI/recession) as a VOL overlay** [72][93][96] — feeds our
   existing macro-risk-on probe arm; cut size into high macro-uncertainty / hawkish episodes.
   Caveat: the monetary channel *fails OOS* outside rate-cut regimes [72] — expect instability.
   (c) **Funding-rate sign/level as a froth gauge** [4][66] — de-risk when 30-day-avg funding
   is extreme positive (crowded longs). Source from reconcilable CEX venues (funding/OI are
   misquoted [91]).
   (d) **On-chain valuation (MVRV-Z / NUPL / CVALUE / NVT)** [40][67][70] — the [40] claim to
   *falsify*; few cycles, high overfit risk.
   (e) **TDA/LPPLS froth-crash detector** [12][58][98] — TDA persistence-norm is more
   noise-robust than raw LPPLS; must prove it doesn't false-alarm.
   Each needs a data feed; each must clear the frozen robustness+cost gate. Default
   expectation given the other 95 papers: most won't survive net of costs — that's the point.
5. **Universe screening is a real risk control.** Restrict to large, liquid, regulated-
   venue coins: small caps are P&D/wash-trading targets with fictional volume
   [19][20][31][32], algorithmic-stablecoin-adjacent assets carry contagion fragility
   [15][43], and post-ETF BTC is equity-correlated risk-on [23][24][56].
6. **ADR-0072 DVOL probe: use it as a VOLATILITY input, never a direction signal, and
   correct for the variance/jump premium.** Crypto IV slopes/skew forecast realized vol
   but not returns [65]; DVOL is biased *high* vs realized by a large positive variance
   risk premium (~14%/yr [45]) plus a clustered-jump premium [55], so DVOL over-states
   future realized vol — don't feed it raw as an unbiased forecast (GARCH/EGARCH may do
   as well [47]). Deribit quotes are inverse options; consume DVOL as published, don't
   re-invert IV naively [49]. A DVOL/skew overlay belongs in position-sizing, not entry
   timing.
7. **Don't build spurious "fundamental floors" or calendar rules.** Miner/production cost
   is NOT a floor (price → cost) [48]; the halving is a weak, delayed, n≈2-3 effect, not
   a reliable buy trigger [46]; Fear & Greed is lagged price with no OOS edge [50][52].
   These are tempting overlays that the evidence says won't survive an honest gate.

## Open questions / things worth testing in our app

- Does a **MVRV-Z / NUPL threshold long-flat overlay** survive our robustness+cost gate
  vs buy-and-hold, or does it overfit on ~3 cycles like we'd expect? [40] (needs on-chain feed)
- Does a **funding-rate / basis caution overlay** (de-risk when 30-day-avg funding is
  extreme) improve risk-adjusted hold? [3][4] (needs funding feed)
- Does conditioning size on an **on-chain-/exchange-inflow vol-spike forecast** reduce
  drawdown without killing return? [13][36] (vol is more forecastable than direction)
- Does an **equity-regime (risk-on/off) overlay** help post-ETF, now that BTC is
  S&P-correlated and equity crash-risk raises BTC *vol*? [23][24][56][64]
- Are our heavy-tailed VaR / cascade-slippage assumptions actually reflected in the
  bootstrap, or do we implicitly assume thinner tails + linear slippage? [6][33][55]
- If we wire the **DVOL probe (ADR-0072)**, does a DVOL-driven vol-targeting overlay
  (de-risk when DVOL is high), with the variance-premium bias removed, reduce drawdown
  net of costs vs buy-and-hold — and does it beat a plain GARCH vol estimate? [45][47][65]
- Should the cost model **widen effective spread in extreme-sentiment / high-DVOL
  regimes** (adverse selection + liquidity withdrawal), and does that flip any
  marginal strategy from "passes" to "fails" the gate? [9][33][50]
- Is a **stablecoin-peg stress monitor** (flag when USDT/USDC trade < ~$0.99, or when
  pair-stablecoin volatility/redemption-friction rises) a useful exogenous tail-risk
  circuit-breaker for a coin priced against that stablecoin? [61][62][81][82][100]
- Does a **USDT-exchange-inflow overlay** (scale exposure up when stablecoin dry powder
  flows to exchanges) survive our gate vs hold, or is it an intraday-only effect that
  decays at our daily horizon? [68][100] (needs on-chain flow feed)
- Does a **macro risk-on/off overlay** (de-risk on hawkish-policy / inflation-surprise /
  high-macro-uncertainty states) reduce drawdown net of costs — and does it survive OUTSIDE
  the regime it was fit on (the monetary channel failed OOS in [72])? [72][93][96]
- Does an **aggregated multi-horizon trend overlay at ~2-week lookback** (CTREND-style,
  not a single MA) beat buy-and-hold net of costs on liquid BTC/ETH — the long-only shadow
  of the long-short trend/momentum factor? [34][67][77]
- Should our cost model **condition effective spread on time-of-day / day-of-week** (funding-
  cadence + algo periodicity) for rebalances, and does it change any gate verdict? [94]
- Do our derivatives-sourced overlays (funding/OI) read from **reconcilable venues**
  (Kraken/HTX) rather than the misquoting ones (Bybit/OKX)? [91]

## Paper map (claim → supporting [N])

- Funding/basis carry = top structural edge (market-neutral, uncorrelated w/ HODL): [1] [3] [4] [41] [60]
- No-arb perp pricing formula (F=S(1+r/κ)) + realistic fee tiers: [1]
- Carry driven by retail leverage demand / negative convenience yield: [3] [4]
- Carry/funding risk-compensated; limits-to-arbitrage severe (segmented margin); ADL hunts winners: [4] [37] [60]
- Carry persists because of segmented collateral (CME−IBIT wedge ~2.58%/yr): [60]
- AMM impermanent loss = LVR = (σ²/8)·value adverse-selection bleed: [35]
- AMM liquidity provision empirically LOSES to holding (fees < IL/LVR): [53] [63]
- Vol is long-memory / multifractal / q-Gaussian; rough-vol mis-specified: [5] [6] [54]
- Large-cap crypto has inverse-cubic (heavy) tails, mature-market-like: [6] [54]
- Super-linear volume→price impact (worse than equities): [6]
- Bitcoin reverse/forward vol skew (commodity-like, upside demand): [7] [8]
- Bitcoin lacks the equity leverage effect (symmetric conditional-vol response): [47]
- GARCH/EGARCH is a solid crypto vol baseline (beats naive hist/EMA): [47]
- IV slopes/skew forecast VOLATILITY but NOT returns (vol-informed trading): [65]
- Large positive Bitcoin variance risk premium (DVOL > realized, ~14%/yr): [45]
- Extra clustered-jump (Hawkes) premium in crypto options; DVOL biased high: [55]
- Deribit = inverse options (>80% share); handle IV inversion carefully: [49]
- Microstructure features (OFI/spread/VWAP-dev) robust but weak: [9] [10]
- HFT microstructure alpha fails net of costs (validates our thesis): [10]
- Cross-exchange deviations bounded by fee bands, mean-revert fast: [42]
- Binance/Huobi/CEX lead price discovery (CEX→DEX one-way): [42] [51]
- Funding/spread arbs mostly unprofitable after costs+reversals (40% hit, 95% forced exit): [51]
- Triangular arb net-unprofitable for average trader: [26]
- MEV/arbitrage value captured by searchers/builders/latency not retail: [21] [22] [63]
- Cross-country BTC premiums measure capital controls, not arb: [44]
- On-chain network/tx features add info beyond price (direction): [11]
- On-chain tx-graph / exchange-inflow / whale flows forecast VOL regimes/spikes: [13] [36]
- On-chain NUPL/MVRV-Z/CVDD timing beats hold (UNPROVEN under our gate): [40]
- Metcalfe fundamental + LPPLS flags bubbles/tops (multi-timescale, false-alarm-prone): [12] [58]
- Miner/production cost is NOT a price floor (price → cost): [48]
- Bitcoin halving = weak/delayed/inconsistent effect (n≈2-3), not a calendar edge: [46]
- Fear & Greed is endogenous to price (returns→sentiment), no OOS edge: [50] [52]
- Extreme sentiment = wider spreads (liquidity cost), not a directional edge: [50]
- Crashes synchronize all coins (corr → 0.8–0.9); equity-correlated post-ETF (~0.87): [28] [56]
- Equity crash risk spills to BTC VOL (not returns): [64]
- Depeg/jump events self- & cross-excite (Hawkes / endogenous instability): [16] [43] [55]
- Perp liquidation cascades + ADL dump into spot, slippage 5–10×: [33] [37]
- DeFi liquidations over-sell collateral; toxic spirals: [38] [39]
- Non-custodial stablecoins have endogenous deleveraging-spiral regime: [43]
- Stablecoin runs: $0.99 break-the-buck threshold + flight-to-safety; demand-side arb holds peg: [61] [62]
- BTC = risk-on, not safe haven; equity-correlated post-ETF (~0.87 in 2024): [23] [24] [56]
- Market efficiency dynamic (AMH), tied to liquidity, improving / higher for large caps: [17] [18] [54]
- Wash trading >70% on unregulated CEXs / pervasive on DEXs; excess liquidity-variance signature: [19] [20] [57]
- Pump-and-dumps target small/low-liquidity coins: [31] [32]
- 2017 BTC run shaped by Tether-issuance flows (data-integrity caution): [30]
- Trend/momentum works cross-sectionally (long-short, survives costs in liquid coins): [34] [77]
- Crypto C-4 factor model (market/size/momentum/value=price-to-new-address; weakening post-2020): [67]
- Momentum survives costs ONLY in liquid coins; illiquid coins mean-revert: [77]
- ML cross-section: simplest methods win (OLS > trees/NN); edges in hard-to-trade coins: [67]
- Funding rate as sentiment/positioning gauge (causal link, no robust net-of-cost timing): [66]
- Carry Sharpe falling post-2024, negative in 2025 (structural edge decaying): [67]
- Open interest systematically misquoted (Bybit/OKX worst; Kraken/HTX reconcilable): [91]
- USDT exchange-inflows predict +BTC/ETH returns & lower vol (exogenous demand signal): [68]
- Tether flows arrive after downturns, predict +subsequent BTC returns; peg arb centralized: [100] [30]
- ~75% of BTC activity off-chain; on-chain DEMAND (not supply) carries price info: [78]
- NVT ratio = on-chain "P/E" valuation/connectedness metric (to test, not trust): [70]
- Vol forecastable R²~0.67; dominant features = lagged RV + volume + attention (HAR core): [73]
- Crypto inverse leverage effect / low persistence / jump-dominance (signed-semivariance HAR): [87]
- Leverage-effect SIGN contested (inverse [87] vs standard [88] vs none [47]) — don't hard-code
- Classical ARIMA/GARCH: meaningful short-run VOL forecast, price unpredictable: [88]
- DVOL/implied vol noisy in thin-option wings; HAR/GARCH ≥ DVOL for realized vol: [99]
- Crypto bears ~zero intrinsic own-currency yield (ETH staking not priced in derivatives): [92]
- Crypto USD "risk-free" rate large/time-varying/venue-specific (~0–20%+ from Deribit): [92]
- "Better inputs > deeper model"; simple ≈ deep at LOB scale; ceiling 42–71%, dies on costs: [79]
- Order-flow (event) representations generalize across regimes better than LOB-level snapshots: [80]
- Cost-aware ML on BTC restores returns but NO Sharpe outperformance vs hold (bootstrap): [83]
- Social-media sentiment: no Granger causality to returns (returns→sentiment), no net-of-cost: [74]
- Causal Bayesian net: TA carries signal, external/social features can hurt (no OOS/cost): [95]
- Herding = consensus co-movement; now propagated INTO crypto via ETF complex: [75] [76]
- BitMEX ~3.5% of longs force-liquidated DAILY at ~60× leverage; normality under-sizes margin: [71]
- GARCH+EVT+copula / robust RVaR the right tail machinery; normality under-states tail risk: [71] [84]
- Full-sample correlation understates stress coupling; crypto a net RECEIVER of shocks: [97]
- Bitcoin NOT an inflation hedge (−24 bps per SD inflation surprise): [96]
- Bitcoin NOT a central-banking hedge; risk-on, "barometer of global liquidity": [93]
- Macro event-risk (Fed/CPI/recession) forecasts crypto VOL; monetary channel fails OOS: [72]
- CEX leads price discovery; BTC leads ETH; futures lead BTC spot: [69] [90]
- Sandwiching = DEX execution tax (even private routing unsafe; one bot = ⅔ front-runs): [85]
- Cross-chain arbitrage specialist-captured, retail-inaccessible (bridge/capital barriers): [86]
- ~31% of large orders could spoof Coinbase BTC/ETH books (depth partly fake): [90]
- Spoofing/layering rife around LUNA crash; stat-physics detection > Z-score: [89]
- TDA persistence-norm detects bubbles (links to LPPLS); more noise-robust; tested on BTC: [98]
- Stablecoin peg held by thin/CENTRALIZED arbitrageur layer (~6 Tether redeemers/month): [82] [100]
- Stablecoin peg stability time-varying (USDT distribution shifts across regimes): [81]
- Intraday/weekly periodicity in crypto vol/liquidity (funding-cadence/algo); arbitraged away: [94]
- META-THEME: volatility/regime is forecastable; direction is ~random-walk: [13] [36] [54] [64] [65] [72] [73] [83] [87] [88]
- META-THEME: crypto "passive yield" (carry, LP fees) = disguised short-vol/short-tail risk; coin bears no intrinsic yield: [1][3][4][35][41][53][60][63][92]
- META-THEME: costs kill the edge; even cost-aware ML doesn't beat buy-and-hold (our thesis, replicated): [10] [26] [51] [83]
- META-THEME: reported volume / open interest / order-book depth are all partly fake → distrust venue size: [19] [20] [90] [91]
