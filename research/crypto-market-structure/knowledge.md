# Knowledge — Crypto Market Structure

Synthesis of the `crypto-market-structure` ledger. Updated incrementally.
Our app: a long-only, single-coin, paper-sim crypto advisor that bakes off
strategies and ranks them under a frozen 1000-path moving-block-bootstrap
robustness gate, with buy-and-hold as the permanent benchmark. Validated thesis:
*no active strategy robustly beats holding, net of costs.* Read every takeaway
through that lens.

Coverage so far: [1]–[65]. (Round 2 complete; target 100.)

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
   though the arb trade itself is out of scope [3][4].

3. **Crypto "passive yield" sources are short-vol / short-tail trades in disguise.**
   AMM liquidity provision (impermanent loss) is a continuous adverse-selection bleed:
   LVR = (σ²/8)·pool-value per unit time for a constant-product pool [35]. Funding
   carry is compensation for crowded-long + liquidation risk [1][3][4]. None is free
   money; each is risk-compensation that our heavy-tailed regimes would punish.

4. **Crypto volatility is real, persistent, long-memory, multifractal, heavy-tailed.**
   BTC/ETH return tails are inverse-cubic (mature-market-like) [6]; vol clustering has
   power-law (long-range) autocorrelation [6]; realized vol is multifractal, so
   single-parameter "rough vol" models mis-specify it [5]. Use HAR / regime /
   multiscale vol models, heavy-tailed risk, never Gaussian VaR.

5. **Crypto options / DVOL carry VOLATILITY information, not DIRECTION — and DVOL is
   biased high vs realized.** Bitcoin shows a forward/reverse vol skew (OTM calls richer),
   commodity-/upside-demand-like, opposite to equities [8]; Bitcoin lacks the equity
   "leverage effect" (symmetric conditional-vol response) [47]. Critically, IV slopes
   forecast realized *volatility* but NOT *returns* — options trading is vol-informed,
   not directionally-informed [65]. There is a large positive variance risk premium
   (BVRP ≈ +14%/yr, ~7× the S&P's [45]) plus an extra clustered-jump premium [55], so
   implied vol (DVOL) sits *above* subsequent realized vol — DVOL is NOT an unbiased
   vol forecast; GARCH can forecast realized vol as well or better [47][65]. Deribit
   quotes are *inverse* options (handle IV inversion carefully) [49]. ADR-0072 lesson:
   use DVOL/skew as a risk-sizing/vol input, never as a return-timing signal.

6. **Microstructure & cross-venue/arbitrage edges exist statistically but die on costs
   / are captured by specialists.** OFI, spread, VWAP-deviation are the robust LOB
   features [9], but HFT microstructure strategies have deeply negative net Sharpe
   (turnover 100–200×/day) [10]; gradient boosting overfits [10]. Cross-exchange
   deviations stay inside fee-defined no-arb bands and mean-revert fast [42]; even
   triangular arbitrage is net-unprofitable for the average sophisticated trader [26];
   MEV/sandwich/liquidation value accrues to searchers/builders, not retail [21][22].
   Cross-country premiums measure capital controls, not free arb [44].

7. **On-chain metrics carry orthogonal information — strongest for VOLATILITY/REGIME,
   most-hyped for direction/cycle-timing.** Tx-network features add direction info
   beyond TA [11]; tx-graph structure forecasts vol regimes [13]; exchange-inflow
   flows + whale transfers forecast vol spikes (F1≈0.46, caught >61% of spikes) [36].
   The boldest claim: NUPL / MVRV-Z / CVDD threshold strategies beat buy-and-hold
   risk-adjusted over 3 cycles, MVRV-Z strongest [40] — but with very few independent
   cycles and no robustness gate, this is a hypothesis to falsify, not an established
   edge. Metcalfe-fundamental + LPPLS flags bubbles/tops ex-ante [12][58] (LPPLS needs
   multi-timescale fitting, prone to false alarms). CAUTION on "fundamental floors":
   **miner/production cost is NOT a price floor — price drives cost, not the reverse**
   [48], so miner-cost / Puell / hash-ribbon signals are price-derived (circular), not
   independent valuation. The recurring meta-theme across on-chain, sentiment, and
   options data: **volatility/regime is forecastable; direction is ~random-walk** —
   build risk-sizing overlays, not return-timing bets.

7b. **"Sentiment signals" are mostly endogenous or cost-effects, not alpha.** The Crypto
   Fear & Greed Index does NOT Granger-cause returns and gives no OOS gain — returns
   cause sentiment, so FGI is lagged price [52]. Extreme sentiment is a *liquidity-cost*
   regime (spreads widen; intensity, not direction) with no reported net-of-cost edge
   [50]. Funding/carry level remains the one structurally-grounded sentiment proxy
   (crowded-long froth) [3][4], but harvesting it needs a perp engine.

7c. **DeFi "passive yield" loses to holding.** Three independent results — LVR theory
   [35], the Uniswap-v3 IL study (~49.5% of LPs negative; IL > fees) [53], and an
   LVR-vs-fees measurement (fees < arb losses in most large pools) [63] — agree that
   AMM liquidity provision is, in aggregate, a losing trade vs simply holding. This
   extends the ship-passive thesis to DeFi yield.

8. **Crypto crashes are correlated, clustered, self-exciting, and amplified by the
   leverage layer.** Cross-coin correlations surge to ~0.8–0.9 in crashes — divers/-
   ification collapses when needed [28]; depeg/jump events self- and cross-excite
   (Hawkes) [16]; perp liquidation cascades + ADL dump into spot with slippage 5–10×
   normal [33][37]; DeFi liquidations over-sell collateral and can run toxic spirals
   [38][39]; non-custodial stablecoins have an endogenous tip-into-instability regime
   [43]. Moving-block bootstrap (preserves clustering) is the right gate; i.i.d./
   Gaussian is wrong.

9. **Bitcoin is risk-ON, not a safe haven — and increasingly equity-correlated.** It
   sold off WITH equities in COVID (no crisis-hedge value) [24], and post-Jan-2024 ETF
   approval its S&P 500 correlation jumped to a persistent positive level (structural
   break) [23], peaking ~0.87 in 2024 [56]. Gold correlation stayed ~0. Equity crash
   risk spills into BTC *volatility* but not BTC *returns* [64] — again the vol-vs-
   direction split. Drawdowns coincide with broad risk-off; diversification benefit is
   stale [23][24][56].

10. **Market efficiency is dynamic (Adaptive Market Hypothesis), tied to liquidity,
    and rises for large caps.** Predictability windows open and close [17][18]; BTC
    more efficient than ETH [17]; more liquidity → more efficiency → harder to beat
    [17][18]. So large-cap coins are *harder* to beat than small caps — set per-coin
    expectations, and don't assume a strategy stays robust across regimes.

11. **Data integrity is a first-class hazard.** Wash trading is >70% of reported volume
    on unregulated CEXs [19] and pervasive on DEXs (>30% of tokens) [20]; coordinated
    pump-and-dumps target small/low-liquidity coins [31][32]; 2017's BTC run was
    materially shaped by Tether-issuance flows [30]. Volume-based signals and small-cap
    backtests are easily poisoned.

## Methods / findings that hold up (and which don't)

- **Holds up:** funding/basis carry as a high-Sharpe market-neutral edge, uncorrelated
  with HODL [1][3][4][41][60]; LVR = (σ²/8)·value as the AMM adverse-selection cost [35]
  (empirically: LP'ing loses to holding [53][63]); long-memory/multifractal vol
  clustering [5][6][54]; heavy (inverse-cubic / q-Gaussian) tails for large caps [6][54];
  OFI/spread/VWAP-deviation as dominant microstructure features [9]; purged walk-forward
  validation [10]; correlated/clustered/self-exciting crash behaviour [16][28][33][55];
  BTC = risk-on, equity-correlated, not safe haven [23][24][56][64]; dynamic (AMH)
  efficiency tied to liquidity, improving over time [17][18][54]; on-chain & options &
  cross-asset signals forecasting *volatility* regimes (not direction) [13][36][64][65];
  large positive Bitcoin variance risk premium (DVOL > realized) [45][55]; GARCH/EGARCH
  as a solid crypto vol baseline (no leverage effect) [47]; stablecoin runs with a
  $0.99 break-the-buck threshold + flight-to-safety [61][62].
- **Doesn't hold up / cautions:** rough-volatility (single Hurst) for BTC [5]; HFT
  microstructure alpha net of costs [10]; gradient boosting on microstructure (overfits)
  [10]; cross-asset model transfer in crypto (idiosyncratic) [10]; "risk-free"
  cash-and-carry (segmentation + liquidation/ADL risk) [4][37][60]; cross-exchange &
  cross-country & funding premiums as exploitable arb (fee bands / capital controls /
  forced exits) [42][44][51]; triangular arb for the average trader (net-negative after
  costs) [26]; on-chain *cycle-timing* outperformance [40] — plausible but unproven under
  a robustness gate; **miner/production cost as a price floor** (price drives cost) [48];
  **halving as a reliable calendar edge** (delayed, ~1/5 of move, n≈2-3, 2020 inconclusive)
  [46]; **Fear & Greed contrarian as alpha** (endogenous to price, no OOS gain) [50][52];
  **option skew/DVOL as a direction signal** (it forecasts vol, not returns) [65].

## Actionable takeaways for our advisor

1. **The biggest documented crypto edges (carry, LP yield) are out of our long-only
   scope and/or are disguised short-vol/short-tail risk.** Be honest that the advisor
   cannot capture carry without a perp/short/funding engine [1][3][4][41]; and that
   AMM "yield" is an LVR bleed [35]. Logging this prevents a "why don't we beat hold?"
   rabbit-hole — the structural edge is market-neutral, not directional.
2. **Cost realism is decisive.** [10] is a clean external replication of our thesis:
   statistical edge → economic failure once realistic fees + turnover apply. Keep the
   cost model central; widen effective spread in volatile/crash regimes (adverse
   selection [9]; cascade slippage 5–10× [33]).
3. **Use heavy-tailed, long-memory, correlated-crash assumptions in the robustness gate.**
   Moving-block bootstrap is appropriate; crashes are clustered, self-exciting, and
   synchronize all coins [16][28][33][43]. Super-linear price impact → harsher size
   penalties on crypto, much harsher on small caps [6].
4. **The single most promising long-only test: an on-chain valuation overlay (MVRV-Z /
   NUPL / CVDD) run through OUR gate vs buy-and-hold.** [40] claims it beats hold
   risk-adjusted; our job is to falsify that under the frozen robustness+cost gate. If
   anything survives, this — plus a funding/basis *sentiment* caution overlay [3][4] —
   is the leading candidate for a real edge. Both need data feeds (on-chain; funding).
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
- Is a **stablecoin-peg stress monitor** (flag when USDT/USDC trade < ~$0.99) a useful
  exogenous tail-risk circuit-breaker for a coin priced against that stablecoin? [61][62]

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
- Trend/momentum works cross-sectionally (long-short, survives costs): [34]
- META-THEME: volatility/regime is forecastable; direction is ~random-walk: [13] [36] [54] [64] [65]
- META-THEME: crypto "passive yield" (carry, LP fees) = disguised short-vol/short-tail risk: [1][3][4][35][41][53][60][63]
