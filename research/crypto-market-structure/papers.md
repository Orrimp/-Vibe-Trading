# Papers — Crypto Market Structure

Ledger for the `crypto-market-structure` topic. One entry per paper, appended
immediately after reading. Source of truth for resume + dedup.

Scope: perpetual-futures funding rates & funding-rate arbitrage; spot-perp basis
& cash-and-carry; crypto volatility (realized/implied, DVOL), vol clustering &
jumps; crypto liquidity & order-book microstructure; on-chain metrics & their
predictive value; stablecoins & their pegs; crypto market regimes & contagion;
Bitcoin/Ether market efficiency; crypto market manipulation & wash trading; MEV;
cross-exchange arbitrage; crypto-equity/macro correlation.

---

### [1] Fundamentals of Perpetual Futures
- **Authors / Venue:** Songrun He, Asaf Manela, Omri Ross, Victor von Wachter / arXiv working paper (latest v 2024)
- **Year:** 2022
- **Source:** arXiv:2212.06888
- **% read:** 75%
- **Summary:** Derives no-arbitrage prices for crypto perpetual futures in frictionless markets and arbitrage *bounds* under trading costs. Core relation: F_t = S_t(1 + r/κ), where the funding rate is proportional to the perp–spot gap and κ (≈1095) scales the 8-hour payment cadence. Empirically, perp-price deviations from the no-arb benchmark are large (mean-absolute ≈ 60–100%/yr across tokens), comove across coins, and shrink over time — far larger than in traditional FX. The implied cash-and-carry arbitrage (long spot / short perp to harvest funding) yields high annualized Sharpe ratios even net of high fees: BTC 1.92, ETH 2.82, BNB 5.44, DOGE 4.42, ADA 3.12 (BTC reaches 3.94 at zero fees). Three fee tiers tested: 2.25/4.5/6.75 bps spot, 0.18/0.72/1.44 bps futures.
- **Relevance to our system:** Foundational. This is the canonical evidence that funding/basis carry is the highest-Sharpe *structural* edge in crypto — but it is a market-neutral (long-spot/short-perp) trade, NOT a directional single-coin bet. Our advisor is long-only single-coin vs buy-and-hold, so this edge is **out of reach without a perp/short/margin engine**. Documents the exact no-arb formula and realistic fee tiers we'd need to model funding. Also a caution: the deviations comove across coins (systematic factor), so the "arb" carries real risk, not free money.

### [2] Designing Funding Rates for Perpetual Futures in Cryptocurrency Markets
- **Authors / Venue:** Jaehyun Kim, Hyungbin Park / arXiv
- **Year:** 2025
- **Source:** arXiv:2506.08573
- **% read:** 25%
- **Summary:** A mathematical-finance treatment of how a perpetual-future *issuer* should design the funding-rate function so the perp price tracks its target (spot) value. Uses path-dependent infinite-horizon BSDEs plus arbitrage-pricing theory to prove existence/uniqueness of the funding mechanism and to construct replicating portfolios that let issuers hedge. Contrasts the standard (instantaneous gap) funding rule with a path-dependent alternative and analyses long-run price-alignment behaviour. Primarily a theory/design paper; little empirical validation.
- **Relevance to our system:** Background/theory only. Explains *why* funding exists and how it is engineered to peg perp↔spot — useful if we ever build a funding/perp simulation engine, but no directly testable directional signal for a long-only single-coin advisor. The path-dependent vs instantaneous distinction matters for accurately modelling realized funding cash-flows if we later add a perp leg.

### [3] The Crypto Carry Trade
- **Authors / Venue:** Nicolas Christin, Bryan R. Routledge, Kyle Soska, Ariel Zetlin-Jones / CMU working paper (NBER Big Data conf.)
- **Year:** 2022
- **Source:** https://www.andrew.cmu.edu/user/azj/files/CarryTrade.v1.0.pdf
- **% read:** 60%
- **Summary:** Studies perpetual-futures contracts on a large exchange and uses the funding-rate design to measure whether demand pressure sits on the long or short side. Constructs a carry trade — short the perp, hedge long with spot — and finds it "unusually profitable," with in-sample annualized Sharpe ratios of 7–10. Interprets the long perp as an expensive way to be long crypto: traders pay a premium for leveraged long-side exposure, so demand is dominantly long. Frames the result as evidence on the financialization of crypto and the risks unique to crypto exchanges. The high Sharpe is in-sample and reflects compensation for exchange/counterparty/liquidation risk.
- **Relevance to our system:** Confirms the same structural edge as [1] from an independent venue: funding/basis carry (short perp + long spot) is the dominant high-Sharpe crypto edge, and it exists because retail crowds the *long* side. For our advisor this is doubly relevant: (a) the edge is market-neutral and needs a perp/short engine we don't have; (b) the *direction* of crowding (long-side demand premium) is a candidate sentiment signal — persistently positive funding = crowded longs = a possible mean-reversion/caution flag for a long-only holder. The 7–10 Sharpe is in-sample; treat as upper bound, not a forecast.

### [4] Crypto Carry
- **Authors / Venue:** Maik Schmeling (Goethe Univ. & CEPR), Andreas Schrimpf (BIS & CEPR), Karamfil Todorov (BIS) / BIS Working Paper No 1087
- **Year:** 2023 (rev. Oct 2025)
- **Source:** https://www.bis.org/publ/work1087.pdf (BIS WP 1087)
- **% read:** 60%
- **Summary:** Analyses crypto carry = the futures–spot gap, organized via F_t = S_t·e^((r_t+u_t−y_t)(T−t)). Documents avg annualized carry ≈ 7% p.a. across exchanges Apr-2019→Jul-2024, occasionally >40% p.a., with strong time variation. Shows observable fundamentals (rate differentials r, storage u≈0) cannot explain the level/fluctuations, implying a large and *negative* convenience yield y<0 — investors prefer being long the futures over long spot (opposite of commodities). Traces carry to two forces: (i) demand from smaller trend-chasing investors seeking leveraged exposure, and (ii) limited arbitrage capital due to regulatory/margin frictions. Concludes the "risk-free" cash-and-carry is far from free: structural limits to arbitrage are especially severe in crypto and amplify price inefficiencies.
- **Relevance to our system:** The most authoritative (BIS) statement of the same edge as [1]/[3], plus the cleanest economic mechanism: a *negative convenience yield* driven by retail leverage demand + limits to arbitrage. Two takeaways for our advisor: (1) carry/funding is a real but risk-compensated edge requiring a perp/futures+margin engine — out of our long-only scope; (2) carry level is a directional sentiment proxy: very high carry = aggressive leveraged-long crowd = froth, a candidate caution overlay for a buy-and-hold horizon. The 7% baseline (vs eye-catching 40%+ spikes) is the honest, regime-conditional number.

### [5] Multifractality in Bitcoin Realized Volatility: Implications for Rough Volatility Modelling
- **Authors / Venue:** Milan Pontiggia / arXiv
- **Year:** 2025
- **Source:** arXiv:2507.00575
- **% read:** 50%
- **Summary:** Tests whether Bitcoin realized volatility (1-min Bitstamp data, 2017–2024) is "rough" (fractional, single Hurst exponent) or multifractal. Three independent diagnostics — MF-DFA, log-log moment scaling, wavelet leaders — converge on a pronounced *multifractal* structure; shuffling the series collapses the multifractal signature, proving it comes from temporal correlations, not heavy tails. Critically, the Cont–Das normalized p-variation statistic never crosses zero in any year/frequency, so *no valid roughness index* can be estimated. Conclusion: rough-volatility models are structurally misaligned with Bitcoin's empirical volatility.
- **Relevance to our system:** Methodological caution for any volatility forecaster we bolt on (our DVOL/vol-targeting probes). Bitcoin vol is multifractal with long-range dependence — a single-parameter rough/fractional model will mis-specify it. If we model realized vol, prefer HAR-type or regime/multiscale specs over rough-vol. Also reinforces that vol clustering is real and persistent (exploitable for risk sizing), but its scaling is complex — don't over-trust a tidy closed-form. Background-to-moderate relevance: shapes *how* we'd forecast vol, not a trading signal itself.

### [6] What is Mature and What is Still Emerging in the Cryptocurrency Market?
- **Authors / Venue:** Stanisław Drożdż, Jarosław Kwapień, Marcin Wątorek / Entropy (MDPI); arXiv:2305.05751
- **Year:** 2023
- **Source:** arXiv:2305.05751
- **% read:** 50%
- **Summary:** Econophysics study of which "financial stylized facts" of mature markets crypto already satisfies. Finds the largest-cap coins (BTC, ETH) now closely match mature markets: return-distribution tails moved from near-Lévy (α≈2, 2012–13) to the universal inverse-cubic power law (α≈3) by 2014–15 and have held since; volatility clustering shows power-law (long-range) autocorrelation comparable in range to stocks/Forex; multifractality is present in both returns and inter-transaction times, including bivariate BTC–ETH multiscaling. Smaller-cap coins are deficient on these properties and less cross-correlated. Notably, volume→price impact R(V)~V^α scales with α≳1 — much stronger price impact than mature equity markets.
- **Relevance to our system:** Directly supports our single-coin focus on large caps: BTC/ETH behave like mature assets (inverse-cubic tails, persistent vol clustering), so risk models calibrated on equities transfer reasonably. Two concrete cautions: (1) price impact is super-linear and stronger than equities — our cost/slippage model must penalize size more aggressively on crypto, especially for smaller coins; (2) heavy (inverse-cubic) tails mean Gaussian VaR understates risk — use heavy-tailed assumptions in the robustness gate. Confirms vol clustering is exploitable for risk sizing but is long-memory.

### [7] Net Buying Pressure and the Information in Bitcoin Option Trades
- **Authors / Venue:** Carol Alexander, Jun Deng, Jianfen Feng, Huning Wan / arXiv (rev. 2022)
- **Year:** 2021
- **Source:** arXiv:2109.02776
- **% read:** 50%
- **Summary:** Uses tick-level Deribit options data to test whether option demand (net buying pressure = delta-weighted buyer- minus seller-initiated trades) moves implied vol, à la Bollen–Whaley. Finds support for the limits-to-arbitrage hypothesis: market makers cannot hedge perfectly, so their supply curves slope up and net demand shifts implied vol (and partially reverses on rebalancing). ATM option prices are driven mainly by volatility traders; OTM options are driven jointly by volatility traders and directionally-informed traders. Deribit's information-aggregation efficiency is improving but differs from mature US/Asian options markets.
- **Relevance to our system:** Moderate. The headline edge (net-buying-pressure → IV moves that partially reverse) is an options-market-maker trade we cannot run (no options engine, long-only). But the *directional* content of OTM option demand is a candidate sentiment feature: informed directional flow shows up in OTM call/put skew. If we ever ingest a Deribit options feed, option skew / net buying pressure could be a leading directional signal for the underlying coin. Confirms crypto options markets are less efficient than equities — so signals there may be noisier. Background-to-moderate.

### [8] Implied Volatility Estimation of Bitcoin Options and the Stylized Facts of Option Pricing
- **Authors / Venue:** Noshaba Zulfiqar, Saqib Gulzar / Financial Innovation
- **Year:** 2021
- **Source:** DOI 10.1186/s40854-021-00280-y (PMC8418903)
- **% read:** 50%
- **Summary:** Estimates implied volatility from Bitcoin options and characterizes the IV surface. Finds a distinctive *forward (reverse) volatility skew* — IV is lower at low strikes and higher at high strikes, i.e., OTM calls carry richer IV than OTM puts — which sharpens near expiry and tends to a more symmetric smile. This pattern is commodity-like, leading the authors to classify Bitcoin with the commodity asset class rather than equities (equities show the opposite, a downward "smirk" where OTM puts are richer from crash fear). Numerically, Newton–Raphson converges faster than bisection for ATM/OTM IV inversion.
- **Relevance to our system:** Moderate/structural. The reverse skew is a key behavioral marker: crypto's tail demand is skewed toward upside (FOMO/leveraged-long), opposite to equities' crash-protection bias — consistent with the carry papers [3][4]. For a long-only single-coin advisor this means: (1) crypto's option-implied risk asymmetry differs from equities, so equity-derived tail/skew intuition is misleading; (2) if we ever add an options/IV feed, monitor skew flips (toward put-richness) as a regime-change / fear signal. No directly tradable signal in our current engine.

### [9] Explainable Patterns in Cryptocurrency Microstructure
- **Authors / Venue:** Bartosz Bieganowski, Robert Ślepaczuk / Univ. of Warsaw; arXiv
- **Year:** 2025
- **Source:** arXiv:2602.00776
- **% read:** 50%
- **Summary:** Builds explainable ML models on limit-order-book microstructure across five coins (rank 1–100). The same features dominate predictions regardless of market cap: order-flow imbalance (monotone, diminishing at extremes), bid-ask spread (wider spread = less predictability = adverse-selection risk), and VWAP-to-mid deviation (asymmetric short-term pressure then microstructure reversion). The Oct-10-2025 flash crash is a natural experiment: taker (liquidity-demanding) strategies profited from directional prediction while maker (passive) strategies took catastrophic losses being filled on collapsing bids — validating that the spread compensates for trading against informed flow. Out-of-sample taker annualized returns are modest: BTC 13%, LTC 7%, ETC 5.8%, ENJ 4.1%, ROSE 7.0% (significant only for ETC/ENJ/ROSE); maker returns weak/negative.
- **Relevance to our system:** Confirms order-flow imbalance + spread + VWAP-deviation are the robust, cross-asset microstructure features — but the edges are small and require *taker* (cost-heavy) execution at high frequency. For our long-only daily-ish advisor this is mostly out of horizon, yet the lesson transfers: passive/maker assumptions are dangerous in crash regimes (relevant to how we model fills/slippage in the robustness gate). Reinforces that adverse selection is real — our cost model should widen effective spread in volatile regimes, not assume mid-price fills.

### [10] Microstructure Alpha: Hierarchical Learning and Cross-Asset Transfer in Cryptocurrency Markets
- **Authors / Venue:** Edson Pindza / Frontiers in Blockchain
- **Year:** 2026
- **Source:** DOI 10.3389/fbloc.2026.1811716
- **% read:** 50%
- **Summary:** Tests whether classical microstructure signals (Corwin–Schultz spread, realized vol, trade intensity, VPIN, Kyle's λ, Amihud illiquidity, depth/order-flow imbalance, momentum) predict 5-min crypto returns, using purged walk-forward validation and hierarchical/transfer learning. Signals carry "genuine but weak information content." OLS adds only +1.23% R² over random walk (not significant); LightGBM *overfits* badly (−10.94% R²). Transfer learning works within an asset across venues (spot↔futures) but poorly across coins (idiosyncratic microstructure). Decisive economic result: with Binance fees, all strategies have deeply negative net Sharpe (−31 to −52 spot; −11 to −18 futures) because turnover hits 124–204×/day — costs overwhelm the edge by orders of magnitude.
- **Relevance to our system:** Strong direct support for our app's core thesis — "no active strategy robustly beats holding net of costs." A textbook example of statistical-edge-but-economic-failure: weak R², gradient boosting overfits, and realistic fees turn it deeply negative. Three takeaways: (1) always evaluate net of realistic round-trip costs (we already do); (2) walk-forward + purging is the right discipline (mirrors our bootstrap gate); (3) cross-asset transfer is weak in crypto — a model tuned on one coin won't generalize, reinforcing our single-coin, per-coin bakeoff design. Also a caution against trusting boosted-tree backtests without a turnover/cost reality check.

### [11] The Predictive Power of the Blockchain Transaction Networks: Towards a New Generation of Network Science Market Indicators
- **Authors / Venue:** M. Grande, F. Borondo, J. Borondo / arXiv
- **Year:** 2024
- **Source:** arXiv:2401.01379
- **% read:** 50%
- **Summary:** Tests whether *network-science properties* of the Ethereum transaction graph add predictive value beyond technical analysis (TA) and social-media trends. Builds two XGBoost classifiers of next-period log-return direction (up/flat/down, ±1% threshold): a Base Model (TA + Twitter/GoogleTrends) and a Full Model that adds network properties (size, degree, connectivity of the tx network). The Full Model anticipates 46% more rises and 19% more falls than the Base Model, and network variables have a significant effect even after controlling for TA and social media. Concludes blockchain-network complexity carries non-redundant forecasting information.
- **Relevance to our system:** A genuine candidate signal — on-chain transaction-network features carry information *orthogonal* to price-based TA. BUT the evaluation is directional-accuracy on an XGBoost classifier; there's no net-of-cost economic backtest and no robustness gate, so per [10]'s lesson treat the "46% more rises" as optimistic in-sample lift, not tradable alpha. For our advisor: if we add an on-chain feed, network-activity features are the most theory-motivated thing to test as a regime/confirmation overlay — but must pass the same bootstrap + cost gate as everything else (likely the deciding factor).

### [12] Are Bitcoin Bubbles Predictable? Combining a Generalized Metcalfe's Law and the Log-Periodic Power Law Singularity (LPPLS) Model
- **Authors / Venue:** Spencer Wheatley, Didier Sornette, Tobias Huber, Max Reppen, Robert N. Gantner / Royal Society Open Science
- **Year:** 2019
- **Source:** DOI 10.1098/rsos.180538 (PMC6599809)
- **% read:** 50%
- **Summary:** Combines a generalized Metcalfe's Law (fundamental value from active-user network growth, fitted exponent β≈1.69 not 2.0) with the Log-Periodic Power Law Singularity (LPPLS) bubble model. When both signal coincide, it flags a bubble + impending correction. Documents four BTC bubbles (2012–2017) and shows LPPLS gives ex-ante warning, with crash-time confidence intervals bracketing realized turning points ~2 weeks ahead. In March 2018 the Metcalfe fundamental value was ~$22–44B vs ~$170B market cap — a ~4× overvaluation flag.
- **Relevance to our system:** The most credible "fundamental valuation + bubble timing" framework for a single coin, from a top group. Two testable ideas for our advisor: (1) a Metcalfe-style fundamental-value band (network users → fair value) as an over/undervaluation overlay for buy-and-hold; (2) LPPLS as a froth/crash-warning detector to de-risk near tops. Caveats: LPPLS is notoriously sensitive to fitting windows and has a mixed live track record; both need our robustness gate + an active-address data feed before trusting. Still, this is exactly the kind of theory-grounded, single-coin, regime-overlay signal worth testing rather than dismissing.

### [13] Inferring Short-Term Volatility Indicators from the Bitcoin Blockchain
- **Authors / Venue:** Nino Antulov-Fantulin, Dijana Tolic, Matija Piskorec, Zhang Ce, Irena Vodenska / ETH Zurich et al.; arXiv
- **Year:** 2018
- **Source:** arXiv:1809.07856
- **% read:** 50%
- **Summary:** Studies whether early-warning indicators (EWIs) for periods of *extreme* Bitcoin price volatility (1–10 day horizon) can be inferred from daily Bitcoin transaction graphs (no price data as input). Constructs low-dimensional representations of the tx graphs (2012–2017) via non-negative matrix decomposition and learns to combine them to predict extreme-volatility events. Finds the NMF-based EWI contains more predictive information than SVD or a scalar total-transaction-volume feature. Demonstrates on-chain money-flow structure carries volatility-forecasting signal independent of price.
- **Relevance to our system:** Supports the idea that on-chain features can forecast *volatility regimes* (not direction) — directly relevant to a vol-targeting / risk-sizing overlay or a DVOL-style probe. More credible than direction-prediction because volatility is more forecastable than returns. For our advisor: if we ingest on-chain data, transaction-graph structure is a candidate input to a regime detector feeding position-sizing. Still must clear our robustness gate and a hold-out split; 2018 sample is small and pre-dates the mature-market regime [6].

### [14] Tracing Stablecoin Contagion during the USDC Depeg after the Silicon Valley Bank Collapse
- **Authors / Venue:** Krongtum Sankaewtong, Stefan Kitzler, Bernhard Haslhofer, Yuichi Ikeda / arXiv
- **Year:** 2026
- **Source:** arXiv:2606.07442
- **% read:** 25%
- **Summary:** Uses high-granularity on-chain transaction data to reconstruct contagion pathways during the March 2023 USDC depeg (USDC briefly fell to ~$0.87 after SVB held part of its reserves). Identifies a *bifurcated* contagion: USDT/WBTC/WETH acted as liquidity-absorption channels (large trade volumes) while USDC-linked assets (incl. DAI, which used USDC as collateral) showed immediate price drops and surging transaction counts. Documents flight-to-quality: users mass-reallocated from single-coin to multi-coin stablecoin portfolios, and activity across major stablecoins became strongly synchronized in the crisis window.
- **Relevance to our system:** Stablecoin-peg risk is a contagion/regime hazard for any single-coin holder. Two takeaways: (1) stablecoin depegs are sudden, exogenous (off-chain banking shock), and synchronize the whole market — a tail-risk source our robustness gate's heavy-tail/regime assumptions should respect; (2) if a coin we advise on depends on a stablecoin (collateral, trading pair), peg stress is a contagion channel to monitor. Mostly background for a long-only single-coin advisor, but flags that "exogenous, fast, synchronized" tail events exist that no price-only backtest window may contain.

### [15] Stability Anchors and Risk Amplifiers: Tail Spillovers Across Stablecoin Designs
- **Authors / Venue:** Wenbin Wu, Can Liu / arXiv
- **Year:** 2026
- **Source:** arXiv:2602.18820
- **% read:** 25%
- **Summary:** Uses quantile-connectedness analysis (tail-focused, à la Diebold–Yilmaz at extreme quantiles) to measure how shocks transmit across stablecoin designs. Finds a clear hierarchy: fiat-backed stablecoins act as *stability anchors* (resilient in tail events), crypto-collateralized are intermediate, and algorithmic stablecoins are *risk amplifiers* that intensify contagion. Directionally, stress originates in algorithmic/crypto-collateralized designs and spreads outward to fiat-backed systems; tail spillovers are asymmetric and intensify with market volatility.
- **Relevance to our system:** Background/risk-context for a single-coin advisor. The actionable nugget: design type predicts contagion risk — algorithmic stablecoins (e.g., the Terra/UST archetype) are systemic amplifiers, so a coin tied to one is far riskier in tails than one tied to a fiat-backed stablecoin. If our advisor's coin universe ever includes algorithmic-stablecoin-adjacent assets, that's a structural red flag for the robustness/tail assessment. No direct trading signal; informs tail-risk weighting and universe screening.

### [16] A Multivariate Hawkes Process Model for Stablecoin-Cryptocurrency Depegging Event Dynamics
- **Authors / Venue:** Connor Oxenhorn / arXiv
- **Year:** 2022
- **Source:** arXiv:2205.06338
- **% read:** 40%
- **Summary:** Proposes modeling stablecoin depegging events as a multivariate *mutually-exciting Hawkes process*: a depeg raises the near-term probability of further depegs (self-excitation) and of disruption/price-jump events in a linked cryptocurrency (cross-excitation, assumed roughly one-sided). Motivated by the fact that USDT was involved in >70% of BTC transactions (Fall 2021), so depeg ripples propagate into BTC microstructure. Demonstrates the framework on a USDT–BTC numerical example. Primarily a modeling/definitional paper, not a large empirical study.
- **Relevance to our system:** Methodologically useful for tail-risk modeling. The key idea — depeg/jump events *cluster* (self- and cross-exciting) rather than arriving independently — means our robustness gate should not assume i.i.d. shocks; a moving-block bootstrap (which preserves local clustering) is more appropriate than a plain i.i.d. resample, which we already use. Also flags that stablecoin stress cross-excites the dominant trading-pair coin (USDT→BTC). Background/methods relevance; reinforces clustered-tail modeling rather than providing a signal.

### [17] On the Evolution of Cryptocurrency Market Efficiency
- **Authors / Venue:** Akihiko Noda / Applied Economics Letters; arXiv
- **Year:** 2019
- **Source:** arXiv:1904.09403
- **% read:** 50%
- **Summary:** Tests the Adaptive Market Hypothesis (AMH) on Bitcoin and Ethereum using a GLS-based *time-varying* degree-of-efficiency measure that avoids the sample-size dependence of rolling-window methods. Finds market efficiency varies over time (not constant), supporting AMH over strict EMH: profit opportunities emerge and dissipate as conditions change. Bitcoin is more efficient than Ethereum across most of the sample, and higher-liquidity periods show evolving (improving) efficiency. Return predictability is therefore time-varying, not a fixed property.
- **Relevance to our system:** Core to our thesis but with nuance. EMH-like efficiency is the *baseline* (why buy-and-hold is hard to beat), but AMH says windows of predictability open and close — so a strategy that worked in one regime may fail in another. This argues for (a) per-window bakeoffs (which we do) rather than one global "best" strategy, and (b) skepticism that any single strategy stays robust across regimes — consistent with our "no active strategy robustly beats hold" finding. Practically: efficiency rises with liquidity, so large-cap coins are *harder* to beat than small caps — set expectations accordingly per coin.

### [18] Market Efficiency, Liquidity, and Multifractality of Bitcoin: A Dynamic Study
- **Authors / Venue:** Tetsuya Takaishi, Takanori Adachi / Hiroshima Univ. of Economics & Tokyo Metropolitan Univ.; arXiv (Applied Economics Letters)
- **Year:** 2019
- **Source:** arXiv:1902.09253
- **% read:** 50%
- **Summary:** Studies the joint dynamics of Bitcoin's market efficiency, liquidity, and multifractality. Before 2013, liquidity was low and the Hurst exponent was <0.5 (anti-persistent → inefficient); after 2013, as liquidity grew, the Hurst exponent rose toward ~0.5 (improving efficiency), though it dipped significantly below 0.5 in several sub-periods (intermittent anti-persistence). Uses the generalized Hurst exponent to quantify multifractality and finds the multifractal degree is related to market (in)efficiency in a *non-linear* way — more multifractality ≈ more deviation from efficiency. The literature it reviews is notably divergent (some find efficiency, some inefficiency), underscoring regime/time dependence.
- **Relevance to our system:** Reinforces [17]: efficiency is dynamic and tied to liquidity, and the literature genuinely disagrees — a caution against any backtest that assumes stationary efficiency. The multifractality↔inefficiency link suggests a *measurable* state variable: when Bitcoin's generalized-Hurst/multifractal degree is elevated, the market is less efficient and possibly more strategy-exploitable. This is a candidate regime indicator to test in our app (does conditioning on a Hurst/efficiency state improve a strategy vs hold?), but with low priority — these effects are subtle and historically intermittent.

### [19] Crypto Wash Trading
- **Authors / Venue:** Lin William Cong, Xi Li, Ke Tang, Yang Yang / Management Science; arXiv
- **Year:** 2021
- **Source:** arXiv:2108.10984 (Management Science 2023)
- **% read:** 50%
- **Summary:** Introduces systematic statistical tests for fabricated ("wash") trading across 29 exchanges, exploiting regularities of authentic trading: Benford's-Law first-significant-digit distributions, trade-size roundness, and power-law transaction-size tails. Regulated exchanges match these natural patterns; unregulated ones violate all three. Estimates wash trading averages >70% of reported volume on unregulated exchanges (trillions of USD/yr). Documents that fake volume inflates exchange rankings, temporarily distorts prices, and correlates with exchange age, user base, and regulatory status.
- **Relevance to our system:** Critical data-integrity warning for our advisor. If volume is >70% fake on some venues, any strategy or signal that uses *volume* (VWAP, volume-confirmation, OBV, liquidity estimates) can be poisoned — and even price can be temporarily distorted. Concrete takeaways: (1) source price/volume data from regulated/honest venues (the paper flags Binance as relatively honest, Huobi as worst per related work); (2) be wary of volume-based features in the bakeoff — test robustness to volume noise; (3) our cost/liquidity model should not trust reported volume at face value. This is exactly the kind of test-data-discipline issue our project prioritizes.

### [20] Detecting and Quantifying Wash Trading on Decentralized Cryptocurrency Exchanges
- **Authors / Venue:** Friedhelm Victor, Andrea Marie Weintraud / TU Berlin; WWW '21 (ACM)
- **Year:** 2021
- **Source:** arXiv:2102.07001 (WWW 2021)
- **% read:** 50%
- **Summary:** First systematic on-chain analysis of wash trading on two LOB-based DEXs (IDEX, EtherDelta). Because every DEX trade is on-ledger with account labels, they directly identify self-trades and two-account round-trip structures meeting the legal definition of wash trading. Finds a lower bound of $159M wash-traded; on *both* exchanges >30% of all tokens experienced wash trading, and on EtherDelta 10% of tokens were *almost exclusively* wash traded. Predominant structures are one- or two-account loops, though complex multi-account forms also occur.
- **Relevance to our system:** Complements [19] with direct (not statistical-inference) evidence and extends the warning to DEXs: a large share of tokens have heavily manipulated volume. For our advisor: (1) avoid low-cap/DEX-only tokens where volume is essentially fictional — they will look more liquid/active than they are and break cost/slippage assumptions; (2) volume-based signals are unreliable on such assets; (3) prefer large, regulated-venue coins for both data quality and backtest validity. Strengthens the universe-screening + test-data-discipline takeaway.

### [21] Flash Boys 2.0: Frontrunning, Transaction Reordering, and Consensus Instability in Decentralized Exchanges
- **Authors / Venue:** Philip Daian, Steven Goldfeder, Tyler Kell, Yunqi Li, Xueyuan Zhao, Iddo Bentov, Lorenz Breidenbach, Ari Juels / IEEE S&P; arXiv
- **Year:** 2019
- **Source:** arXiv:1904.05234
- **% read:** 60%
- **Summary:** The paper that coined Miner Extractable Value (MEV) — the Ether miners can extract by reordering/inserting transactions. Documents three vectors on DEXs: frontrunning (bots outbid pending trades), transaction reordering by miners, and pure-revenue atomic arbitrage across DEXs. Studies priority gas auctions (PGAs) — continuous-time, all-pay auctions where arbitrage bots bid up gas; median winning bots capture ~65% of opportunity value. Establishes a >$6M lower bound on pure-revenue arbitrage and shows high-MEV regimes make forking rational (undercutting and "time-bandit" attacks), threatening consensus-layer security.
- **Relevance to our system:** Mostly background for a CEX-focused long-only advisor, but two real implications: (1) on-chain/DEX execution carries an adversarial-ordering tax (frontrunning, sandwiching) that a naive backtest ignores — if we ever model DEX fills, slippage must include MEV extraction, not just spread; (2) MEV is a structural reason on-chain arbitrage profits accrue to bots/miners, not to ordinary participants — reinforcing that the retail-accessible edge is small. Cross-DEX arbitrage exists but is captured by latency/ordering specialists, not a strategy our advisor could realistically run.

### [22] SoK: The Evolution of Maximal Extractable Value, From Miners to Cross-Chain
- **Authors / Venue:** Davide Mancino, Hasret Ozan Sevim / arXiv (Systematization of Knowledge)
- **Year:** 2026
- **Source:** arXiv:2603.07716
- **% read:** 40%
- **Summary:** Systematizes MEV across three eras: (1) miner extraction (validators directly reorder), (2) searcher economy (independent bots + Flashbots-style auctions), (3) proposer-builder separation (PBS — builders construct blocks and bid to proposers, fragmenting extraction). Classifies the three primary MEV types — arbitrage, sandwich attacks, liquidations — and surveys mitigations: threshold encryption / commit-reveal, MEV-burn, order-flow auctions (OFA), cross-domain solutions, and privacy-preserving ordering. Tracks extraction rates across Ethereum, L2s, and cross-chain settings.
- **Relevance to our system:** Background. Confirms MEV is now an institutionalized, specialist-captured value stream (searchers/builders), reinforcing that cross-exchange/cross-chain arbitrage is not a retail-accessible edge. The taxonomy (arbitrage/sandwich/liquidation) is useful if we ever model on-chain execution costs. For our CEX long-only advisor, the practical takeaway is unchanged: avoid assuming we can harvest on-chain arbitrage; if simulating DEX trades, add an MEV/sandwich slippage component. Liquidation-MEV also flags that leveraged positions get hunted — another reason our long-only no-leverage stance reduces hidden costs.

### [23] The Impact of Bitcoin ETF Approval on Bitcoin's Hedging Properties Against Traditional Assets
- **Authors / Venue:** Yihan Hong, Hengxiang Feng, Yinghan Wang, Boxuan Li / Olin Business School, WUSTL; arXiv
- **Year:** 2024
- **Source:** arXiv:2512.12815
- **% read:** 50%
- **Summary:** Tests how the January 2024 US spot-Bitcoin-ETF approval changed BTC's relationship with traditional assets, using forward rolling correlations, Chow structural-break tests, and ARMA-DCC-GARCH. Finds a clear structural break: BTC–S&P 500 correlation, previously volatile/trending down and idiosyncratic, shifted to a persistent positive level post-approval (Chow p≈0.0000 on 30- and 60-day windows). Correlation with gold stayed ~0 (no "digital gold" strengthening); USD-index correlation stayed negative/negligible. Conclusion: post-ETF, Bitcoin behaves as a "risk-on" high-growth-tech proxy, not a defensive diversifier, and its downside-protection value has diminished.
- **Relevance to our system:** Important regime context for a single-coin BTC advisor. Post-2024, BTC moves with equities, so (1) macro/equity risk-off events now hit BTC harder and more in-sync — relevant to drawdown modeling and to whether a buy-and-hold thesis should incorporate equity-regime awareness; (2) "diversification benefit" claims are stale post-ETF. A candidate overlay: condition BTC risk on equity-vol / risk-on-off state. Also a structural-break caution: relationships are non-stationary, so a backtest spanning the ETF break mixes two regimes — our per-window bakeoff helps, but long windows may blend incompatible correlation regimes.

### [24] Grandpa, Grandpa, Tell Me the One About Bitcoin Being a Safe Haven: Evidence from the COVID-19 Pandemics
- **Authors / Venue:** Ladislav Kristoufek / Charles University & Czech Academy of Sciences; arXiv (Elsevier)
- **Year:** 2020
- **Source:** arXiv:2004.00047
- **% read:** 50%
- **Summary:** Tests Bitcoin's "safe haven" claim during the COVID-19 March-2020 crash — the first severe global market distress in Bitcoin's liquid life. Uses quantile correlations of Bitcoin vs S&P 500 and VIX, benchmarked against gold. Finds the Bitcoin-safe-haven story "unsubstantiated and far-fetched": during the crisis Bitcoin's correlation with equities did *not* fall (a safe haven requires lower/non-positive correlation in turbulence), whereas gold behaved as a clear safe haven. Notes Bitcoin only reached stable >$100M daily volume by ~2016, so this was the first real stress test.
- **Relevance to our system:** Direct evidence that Bitcoin is *not* a crisis hedge — it sells off with risk assets when stress hits. For a long-only single-coin BTC advisor: do not assume diversification/defensive behavior; drawdowns will coincide with broad risk-off. This predates the ETF break [23] yet reaches the same conclusion (BTC = risk-on), making the "BTC is risk-on, not safe haven" finding robust across two very different periods (2020 COVID, 2024 post-ETF). Reinforces heavy-tail, correlated-crash modeling in the robustness gate and tempers any "hold through anything" narrative.

### [25] On the Quality of Cryptocurrency Markets: Centralized Versus Decentralized Exchanges
- **Authors / Venue:** Andrea Barbon, Angelo Ranaldo / Univ. of St. Gallen; arXiv
- **Year:** 2024
- **Source:** arXiv:2112.07386
- **% read:** 50%
- **Summary:** Compares market quality (transaction costs + price efficiency) of CEXs vs DEXs. Transaction costs are trade-size dependent: CEXs dominate for small/medium trades (<$10k); DEXs (esp. Uniswap v3) become cheaper for large trades (~22 bps for a $1M stablecoin trade). Price efficiency: CEXs show triangular-no-arbitrage deviations <5 bps, while DEXs are far wider (Uniswap v2 ~10–30 bps, v3 ~5–50 bps). Provides causal evidence that gas fees directly worsen DEX price efficiency. Uniswap v3 cut transaction costs ~58% and improved price efficiency ~75% vs v2.
- **Relevance to our system:** Directly informs cost-model and venue-choice assumptions for our advisor. Concrete numbers we can borrow: CEX round-trip cost is low and price deviations <5 bps for major pairs — so for a long-only CEX strategy on a large coin, modest spread/fee assumptions are realistic. DEX execution is materially worse for small trades (gas + wider deviations), so retail-sized advice should assume CEX execution. Cross-venue deviations exist but are small (<5 bps CEX) and arbitraged away fast — not a retail edge. Useful calibration for the bakeoff's fill/slippage model.

### [26] Who Are the Arbitrageurs? Empirical Evidence from Bitcoin Traders in the Mt. Gox Exchange Platform
- **Authors / Venue:** Pietro Saggese, Alessandro Belmonte, Nicola Dimitri, Angelo Facchini, Rainer Böhme / IEEE Trans.; arXiv
- **Year:** 2021
- **Source:** arXiv:2109.10958
- **% read:** 50%
- **Summary:** Uses leaked Mt. Gox account-level data (2011–2014) to identify 440 triangular-arbitrage participants and characterize who profits. Arbitrageurs are few and sophisticated. Crucially: "expert users are on average *non-profitable* once transaction costs are accounted for, while [only] skilled investors conduct arbitrage at a positive and statistically significant premium." Winners used order-splitting and non-aggressive execution, reacted quickly to exogenous official-rate moves, and exploited small, fast price movements rather than large deviations. Transaction costs and execution timing are the binding frictions.
- **Relevance to our system:** A clean historical confirmation of our core thesis from the arbitrage angle: even a textbook "risk-free" edge (triangular arbitrage) is net-unprofitable for the *average* sophisticated participant once realistic costs are included — only a skilled minority with execution edges win. For a retail long-only advisor this strongly implies we should not promise arbitrage-like edges; the realistic baseline is buy-and-hold. Methodologically, it underscores that backtests must include transaction costs *and* execution realism (order-splitting, non-aggressive fills) or they will manufacture phantom arbitrage profits.

### [27] Exploring the Predictability of Cryptocurrencies via Bayesian Hidden Markov Models
- **Authors / Venue:** Constandina Koki, Stefanos Leonardos, Georgios Piliouras / Research in International Business and Finance; arXiv
- **Year:** 2020
- **Source:** arXiv:2011.03741
- **% read:** 50%
- **Summary:** Models BTC, ETH, XRP returns with Bayesian (MCMC) Hidden Markov Models where returns depend on unobserved regimes and transition probabilities depend on predictors (Non-Homogeneous HMM). For Bitcoin a 4-state NHHM separates bear (state 1: negative returns, high vol), bull (positive returns, low vol), and calm regimes. The 4-state NHHM gives the best one-step-ahead forecasts across all three coins vs Random Walk, AR, and "kitchen-sink" benchmarks (by CRPS and MSE). Key structural finding: crypto regimes are *not persistent* — frequent state alternations, unlike conventional FX where regimes are sticky.
- **Relevance to our system:** A credible regime-detection framework and a sober caveat. The actionable idea: a bull/bear/calm regime state could gate position sizing or strategy selection in our advisor. BUT (a) the paper itself does no transaction-cost / economic-profitability test, so statistical forecast gains may not survive our cost gate (cf. [10]); and (b) the non-persistence of crypto regimes means a regime-switch strategy will trade frequently → high turnover → cost drag, the exact failure mode in [10]. So: worth testing a regime overlay, but expect costs and rapid switching to erode it. Good motivation for testing regime features inside (not instead of) our robustness+cost gate.

### [28] Complex Network Analysis of Cryptocurrency Market During Crashes
- **Authors / Venue:** Kundan Mukhia, Anish Rai, S. R. Luwang, Md Nurujjaman, Sushovan Majhi, Chittaranjan Hens / arXiv
- **Year:** 2024
- **Source:** arXiv:2405.05642
- **% read:** 50%
- **Summary:** Builds correlation networks across many coins and measures how topology changes through three crashes (2017-18, 2018-19, 2019-20/COVID). Cross-coin correlations surge in crashes: mean partial correlation 0.290→0.769 (2017-18), 0.304→0.898 (2018-19), 0.312→0.339 (2019-20). Network degree-density and clustering rise while average path length falls, i.e., the market becomes densely interconnected with rapid information flow — consistent with an "uninformed synchronized panic sell-off." Diversification across coins collapses exactly when it's needed.
- **Relevance to our system:** Strong tail-risk evidence directly relevant to risk modeling. Even though our advisor is single-coin, this shows crypto-wide contagion: in a crash, *all* coins move together (corr → ~0.8–0.9), so a single-coin drawdown is part of a market-wide event with no crypto-internal hedge. Implications: (1) the robustness gate must model correlated, clustered crash regimes (moving-block bootstrap is appropriate; i.i.d. is not); (2) "switch to another coin to diversify" is illusory in a crash; (3) crash drawdowns are deeper and more synchronized than a calm-period backtest suggests — size and stop assumptions should reflect this.

### [29] Detecting Network Instability via Multiscale Detrended Cross-Correlations and MST Topology
- **Authors / Venue:** Jose De Leon Miranda, Marina Dolfin, George Kapetanios, Leone Leonida / arXiv (econ.EM)
- **Year:** 2026
- **Source:** arXiv:2602.10174
- **% read:** 30%
- **Summary:** Proposes an asset-agnostic framework combining Multiscale Detrended Cross-Correlations (MDCC, frequency-dependent dependence) with Minimum Spanning Tree (MST) topology to monitor market interconnectedness and detect emerging systemic instability. Argues that structural changes in MST topology (node centrality, clustering shifts) can precede market dislocations, offering early-warning signals. Methodological/framework paper applicable to financial networks broadly, including crypto.
- **Relevance to our system:** Background/methods. The transferable idea: MST/network-topology shifts as an *early-warning* state variable for instability — a candidate crash-warning overlay if we ever build a multi-coin correlation monitor. For a single-coin advisor this is lower priority (needs a cross-asset panel), but it complements [28]'s finding that crashes are preceded/accompanied by densifying correlation structure. Treat as a possible regime/early-warning input, not a standalone signal; would still require validation in our cost/robustness gate.

### [30] Is Bitcoin Really Un-Tethered?
- **Authors / Venue:** John M. Griffin, Amin Shams / The Journal of Finance (orig. SSRN working paper 2018)
- **Year:** 2020
- **Source:** SSRN abstract_id=3195066; DOI 10.1111/jofi.12903
- **% read:** 25% (abstract + widely-quoted findings via secondary sources; full text paywalled — not fetched directly)
- **Summary:** The landmark crypto-manipulation study. Analyzing blockchain flows Mar-2017→Mar-2018, the authors find Tether (USDT) issuance was used to buy Bitcoin specifically after price declines, propping up the market: purchases timed to follow downturns produced sizable Bitcoin price increases, and they attribute roughly *half* of Bitcoin's 2017 rise to the hours just after large Tether transactions. The flows trace to one large Bitfinex account. The pattern is consistent with supply-driven manipulation — Tether printed without full USD backing, then used to buy Bitcoin (the CFTC later found Tether was fully backed only 27.6% of the time in 2016–2018).
- **Relevance to our system:** A foundational caution on historical data integrity and exogenous price drivers. The 2017 bull run — a window any long backtest will include — was materially shaped by manipulation, not organic demand. Implications: (1) a strategy that "worked" across 2017 may be fitting a manipulated regime that won't recur; (2) stablecoin-issuance flows are an exogenous price driver outside any price-only model; (3) reinforces survivorship/regime caution in backtest windows. For a single-coin advisor, it argues for skepticism of edges discovered on the 2017 episode and for awareness that crypto prices can be moved by off-model actors.

### [31] Pump and Dumps in the Bitcoin Era: Real Time Detection of Cryptocurrency Market Manipulations
- **Authors / Venue:** Massimo La Morgia, Alessandro Mei, Francesco Sassi, Julinda Stefa / Sapienza Univ. Rome; arXiv (ICCCN)
- **Year:** 2020
- **Source:** arXiv:2005.06610
- **% read:** 50%
- **Summary:** Empirically maps coordinated pump-and-dump (P&D) operations run via Telegram/Discord. Documents >100 active groups (Jul-2017→Jan-2019), 343 P&D operations across 44 exchanges; the Big Pump Signal group alone generated $82.3M volume in six minutes. Targets are deliberately low-liquidity coins (100 coins <$20M cap; 99 traded <$0.40). Builds a real-time detector keyed on "rush orders" (instant market orders); a random forest hits ~92% precision / 91.4% recall and flags a pump within 5 seconds. Example: SingularDTV pumped $0.0354→$0.0924 in minutes; arbitrage bots replicated the move across exchanges.
- **Relevance to our system:** Strong universe-screening signal for our advisor. P&D manipulation overwhelmingly hits *small-cap, low-liquidity, low-price* coins — exactly the assets our advisor should avoid or flag. Implications: (1) restrict the advisable universe to large, liquid coins where P&D is impractical; (2) a backtest on a small coin may be fitting engineered pump spikes, not a real edge — phantom alpha; (3) volume/price spikes in small caps are often manipulation, not signal. Reinforces the test-data-discipline + universe-quality theme alongside the wash-trading papers [19][20].

### [32] PumpSense: Real-Time Detection and Target Extraction of Crypto Pump-and-Dumps on Telegram
- **Authors / Venue:** Ahmed Mahrous, Roberto Di Pietro / arXiv
- **Year:** 2026
- **Source:** arXiv:2605.09431
- **% read:** 40%
- **Summary:** A two-stage P&D detector: a LightGBM screen over Telegram message windows (9.4 µs/sample) followed by LLM-based (GPT/Gemini/DeepSeek) extraction of the target coin + exchange. Detection F1 0.79–0.83; joint coin+exchange extraction accuracy 0.90–0.96. Dataset: 283,017 messages from 39 Telegram groups with 2,246 pump announcements. Findings: 10 groups account for ~70% of pumps; ~70% of announcements cluster in 15:00–17:00 UTC; ~1% of messages are actual pumps; targets span 604 tickers across 14 exchanges. Detects pumps at message time — before the price anomaly that market-based detectors need.
- **Relevance to our system:** Confirms and updates [31]: coordinated P&D remains pervasive (604 target tickers, billions in volume) and concentrated in a few groups with predictable timing. For our advisor the actionable points are the same — avoid small/altcoin targets and distrust their volume/price spikes — plus a note that LLMs are now the SOTA for parsing manipulation chatter (a possible future data-quality filter, not a trading signal). Background-to-moderate; mainly hardens the small-cap-avoidance universe rule.

### [33] Slippage-at-Risk (SaR): A Forward-Looking Liquidity Risk Framework for Perpetual Futures Exchanges
- **Authors / Venue:** Otar Sepper / arXiv
- **Year:** 2026
- **Source:** arXiv:2603.09164
- **% read:** 50%
- **Summary:** Proposes Slippage-at-Risk, a forward-looking liquidity-risk metric for perpetual-futures markets that estimates how far execution price deviates from mid under stressed/cascade conditions (vs backward-looking VaR). Models the liquidation-cascade spiral: price shock → liquidations of undercapitalized positions → market makers withdraw → subsequent liquidations execute far below bankruptcy price → contagion to the next cohort. Drivers: leverage concentration (clustered liquidation prices), funding-rate spikes incentivizing peak risk-taking, shallow depth, and correlated margins. Reports execution slippage can be 5–10× worse than the bankruptcy price in extreme cascades; argues static-margin/Gaussian risk models miss these non-linear feedback loops.
- **Relevance to our system:** Even as a long-only spot advisor, this matters for *modeling crash dynamics correctly*. Liquidation cascades in the leveraged perp market spill into spot price (forced selling), producing the deep, fast, self-reinforcing drawdowns our robustness gate must respect — slippage 5–10× normal, depth evaporating. Concrete takeaways: (1) our fill/slippage model should blow out in high-vol/cascade regimes, not stay linear; (2) clustered leverage + funding spikes are a froth/instability signal (ties to carry sentiment [3][4]); (3) reinforces heavy-tailed, non-i.i.d. crash modeling. Also a reason to stay long-only/no-leverage: we avoid being the liquidated cohort.

### [34] A Trend Factor for the Cross Section of Cryptocurrency Returns (CTREND)
- **Authors / Venue:** Christian Fieberg, Gerrit Liedtke, Thorsten Poddig, Thomas Walker, Adam Zaremba / Journal of Financial and Quantitative Analysis
- **Year:** 2024 (publ. 2025, JFQA 60(7))
- **Source:** DOI 10.1017/S0022109024000747 (open access)
- **% read:** 60%
- **Summary:** Proposes CTREND, a trend factor that aggregates 28 technical signals (momentum oscillators, moving averages of multiple horizons, volume- and volatility-based measures) via machine learning into one signal, on 3,000+ coins (2015–2022). A long-short value-weighted quintile strategy (buy highest-expected-return coins, short lowest) earns 3.87% per week. The effect is exceptionally robust (holds across subperiods/market states; survives 55,296 alternative implementations), *survives transaction costs*, persists in big and liquid coins, and renders the standard cross-sectional momentum factor insignificant. A 3-factor model (market, size, CTREND) outperforms competing crypto asset-pricing models.
- **Relevance to our system:** The strongest "trend works in crypto" evidence — and an important nuance for our advisor. Crucially it is a *cross-sectional long-short* factor (rank many coins, long winners / short losers); the headline 3.87%/week is a market-neutral spread, not a single-coin long-only return, and requires shorting — out of our scope. BUT the underlying message — that *aggregated multi-horizon trend* (not a single MA/oscillator) carries robust, cost-surviving information even in liquid coins — is directly testable in our bakeoff: rather than picking one MA strategy, test an ensemble/aggregate of multiple trend signals as the long/flat decision. This is the most promising lead for a long-only trend overlay that might actually survive our gate. Caveat: long-only capture of a long-short factor is typically far weaker, and the short leg often carries much of the alpha.
