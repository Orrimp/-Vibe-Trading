# Knowledge — Strategies (quant / algorithmic trading)

Synthesized findings for our single-coin crypto **advisor** (paper/sim only).
Updated every ~5 papers and at end of run. Focus: which strategy ideas are worth
testing in our advisor, and which are known to overfit / not survive costs.

## Key themes

1. **Two flavours of momentum.** *Time-series* (own past return predicts own
   future return) [1] vs *cross-sectional* (rank a universe, long winners/short
   losers) [2]. Our single-coin advisor lives in the time-series world; the
   cross-sectional literature is mostly background for us.
2. **Volatility scaling is the recurring load-bearing idea.** It shows up as the
   driver of TSM's Sharpe [1], as inverse-vol (1/σ) weights in risk parity [6],
   and as the vol term in market-making spreads [5]. Across the literature,
   *sizing by volatility* is more robust than *forecasting returns*.
3. **Mean-reversion via bands / z-scores.** Pairs trading [4] opens at ±2σ
   spread divergence and closes on convergence — the same logic transfers to a
   single coin's deviation from its own moving average (Bollinger-style).
4. **Every documented edge decays / is cost-sensitive.** TSM weakens post-2009
   [1]; cross-sectional momentum has "momentum crashes" [2]; pairs-trading
   profits have steadily declined as capital crowded in [4]. All seminal results
   are reported gross or with optimistic costs.
5. **Execution & market-making theory is mostly background for €200 paper-sim**
   [3][5] — useful for *cost modelling* (fees + spread dominate; impact is
   negligible at our size), not as deployable strategies.
6. **Crypto-specific evidence cuts both ways.** Crypto time-series momentum and
   investor-attention are real predictors [12]; trend factors look robust
   cross-sectionally [10]. BUT the cleanest *single-coin BTC* test shows technical
   rules beat buy-hold **gross** and **collapse net of realistic fees** [11] —
   the canonical confirmation of our thesis on our own asset class.
7. **The most attractive crypto edge is structural, not predictive.** Funding-rate
   / basis carry (long spot + short perp) earns a near-market-neutral ~6+ Sharpe
   carry independent of price direction [9] — a fundamentally different lever from
   the directional trend/mean-reversion set, but it needs a perp+margin model.
8. **Combine negatively-correlated signals.** Value vs momentum are negatively
   correlated and combine to a higher Sharpe [7]; the single-coin analogue is
   trend (wins in trends) vs mean-reversion (wins in ranges) → a regime-blend /
   ensemble is a concrete experiment [7][10].
9. **Horizon determines the sign of autocorrelation.** Trend over months,
   reversal over years [2][13]; a momentum window mismatched to the regime
   inverts. Test multiple horizons; don't assume a trend persists.
10. **Regime-switching = de-risk into bear markets, but watch turnover & latency.**
    A jump-model regime detector that goes to cash in bear regimes cuts drawdowns
    and raises Sharpe — but ONLY because it switches rarely; the over-switching
    HMM version *underperforms* buy-hold [17]. Detection lags the turn by ~2 weeks.
11. **The most-hyped sizing overlay (vol targeting) is in-sample-fragile.** [21]
    shows big in-sample Sharpe gains from inverse-vol scaling; [22] shows they
    **largely vanish out-of-sample / in real time** due to unstable fitted
    parameters. The honest expectation: vol targeting trims variance/drawdown,
    not net Sharpe. Pre-register fixed scaling; judge only out-of-sample.
12. **Beware mis-attributing why a strategy "works."** Contrarian profits blamed
    on overreaction were largely a cross-asset lead-lag artifact [19]; an apparent
    single-coin edge may be a microstructure/statistical artifact that dies under
    costs or out-of-sample. Don't trust the story, trust the gated result.
13. **Anomaly decay is quantified, not folklore.** Published cross-sectional
    predictors lose ~26% out-of-sample and ~58% post-publication [25]; the
    famous "MA rules beat the market" result was data-snooping and failed
    out-of-sample after 1986 [30]; stat-arb halved post-2010 [16]. Every TA rule
    we test is maximally public → expect the largest decay class.
14. **Negative skew = hidden insurance/carry; it is a risk, not free yield.**
    Short-vol / variance-risk-premium strategies earn Sharpe ~1.26 but with
    extreme negative skew (ζ*≈−4.6) — slow gains wiped out by tail events [23].
    Crypto funding-carry has the same profile [9][18]. Trend-following is the
    opposite (positive skew). Rank on left-tail / skew, not just mean Sharpe.
15. **Simple/equal weights beat optimized weights (the combination puzzle).**
    Equal-weighted forecast/strategy blends routinely beat estimated "optimal"
    weights because estimation noise overwhelms the bias gain [29]. Default any
    ensemble to fixed/equal weights; learned weights are an overfitting hazard.
16. **Structural edges (leverage constraints, carry) are more durable than
    predictive ones.** BAB exists because investors are leverage-constrained [24],
    carry because of frictions [18] — these are "who-can-do-what" premia, not
    forecasts, and decay less than data-mined predictors [25][30].
17. **Intraday reversal exists but the spread widens exactly when you trade.**
    Stocks reverse ~1–2% after extreme 60-min moves, but the bid-ask spread
    widens during the spike, gutting the net edge (1.6% gross → 0.44% net) [27].
    Cost models must make spread *state-dependent* (wider on volatile bars).
18. **Calendar/seasonality anomalies are cheap to test and likely noise on
    crypto.** Day-of-week/turn-of-month effects had equity-microstructure causes
    (settlement, institutional flows) that don't exist for 24/7 crypto [26];
    treat any apparent calendar edge as a multiple-testing artifact.
19. **Overlay sign is regime-dependent — stops help trends, hurt ranges.** A
    stop-loss adds value ONLY under positive serial correlation/momentum and
    *destroys* return under mean-reversion [38]. Pair any stop/exit overlay with
    a trend filter; never apply it family-blind. Same logic as the long/flat
    de-risk: useful in trends, harmful in chop.
20. **Volatility is a compound-return drag (geometric ≈ arithmetic − ½σ²).** This
    identity [35] is the rigorous reason to (a) report *geometric/compound*
    return, and (b) value vol-reduction overlays for lifting realized growth even
    without raising the mean — and it's why a single buy-and-hold coin (zero
    diversification return) is the honest single-asset ceiling.
21. **The durable crypto edges are scarce and mostly structural/valuation, not
    predictive.** Cross-sectional crypto factors reduce to ~market/size/momentum
    and even those decay OOS [41]; the one single-coin beats-buy-hold claim with a
    clean control is on-chain *valuation* (MVRV/NUPL/CVDD) [40] — a valuation
    signal, not TA — but rests on only 3 cycles. Among directional families,
    time-series momentum is the most academically supported [12][41].
22. **Many "ML alpha" results are ML risk-managing a structural edge, not
    forecasting price.** VIX-futures NN times entry/exit of the roll-yield carry
    [37]; that is the defensible role for ML — overlay timing/risk control on a
    known structural premium — vs the overfit-prone role of pure price prediction.
23. **The momentum turning point is the fragile moment — detect regime breaks.**
    Time-series momentum's worst losses come right at trend reversals; adding a
    changepoint detector to flip toward fast reversion improved Sharpe ~1/3
    (~2/3 in 2015–2020) [56]. Short-horizon trend is the *convex/positive-skew*
    diversifier (low corr to market), long-horizon trend ≈ just market beta
    (82% correlated, redundant) [52][36]. De-risk fast on detected breaks.
24. **Predictable, mechanical *flow* edges decay too.** The S&P 500 index-inclusion
    pop shrank from ~7.4% (1990s) to <1% as arbitrageurs supplied the predictable
    index-fund demand [54]. Even non-forecasting, structural/flow edges get
    arbitraged away once crowded — crypto's analogues (listings, ETF inflows,
    index-product rebalances) likely offered transient, decaying edges.
25. **Crypto calendar effects are an aggregation artifact, not a signal.** The
    "Monday effect" collapses into a single Sunday 23:00 UTC window (US retail
    re-entry) and vanishes under intraday fixed effects; the BTC Monday effect
    decayed to nothing post-2015 [55][26]. The only robust pattern is *lower
    weekend volume/liquidity* — a cost-model input (wider weekend spreads), not
    a return signal.
26. **Crypto's risk structure is the *opposite* of equities in two ways.** Bitcoin
    shows an **inverse leverage effect** (vol rises with rising price, not falling)
    and its risk premium is **upside-driven** (large positive returns supply ~39%
    of it) vs equities where ~80% of the premium is crash-insurance [58]. So
    equity-calibrated vol-targeting/skew intuitions can mis-time crypto, and
    holding BTC pays you largely for bearing *upside* jump/vol risk (~0.8 Sharpe).
27. **Negative-skew carry/vol-risk-premium harvesting is a whole family — and
    crypto has its own.** Dispersion (sell index vol / buy component vol) earns
    ~15% p.a. but with a −43% crash drawdown [53]; the Bitcoin variance risk
    premium is ~14%/yr [58]. All are insurance sales (smooth then tail loss),
    the same profile as funding carry [9] and short-vol [23][46]. Rank on skew.
28. **Nearness to the 52-week high is a single-asset-computable momentum signal.**
    It subsumes past-return momentum and — unlike JT momentum — does NOT reverse
    long-run; mechanism is behavioral anchoring to the high [60]. Computable on
    one coin (price / 52w-high), so a directly testable long/flat candidate.
29. **Rigorous mean-reversion = model the deviation as OU, trade an s-score.** The
    Avellaneda–Lee s-score (residual deviation in σ units, enter ≈ ±1.25σ, exit
    ≈ 0) [62] is the operational form of our Bollinger/z-score candidate and refines
    [39]; require the residual to be OU-stationary first [33]. Like all such edges
    it decayed post-2003 as it crowded [16][25].
30. **Many big "gross" edges are payments for liquidity provision / illiquidity,
    not alpha.** Weekly short-term reversal (~1.7%/week gross [66]), the crypto
    illiquidity premium [33], and the low-vol/lottery anomaly [63] all pay you for
    bearing immediacy or illiquidity risk — they survive ONLY on liquid names and
    you *pay* them as spread on thin ones. Run reversion on BTC/ETH, net the spread.
31. **More volatility ≠ more return (the low-vol anomaly).** Low-vol/low-beta assets
    earn higher risk-adjusted returns than high-vol ones across 90 years and every
    sector [63] — the empirical cousin of inverse-vol sizing [21][6][8]: cutting risk
    costs little expected return. Don't assume a high-vol regime/coin pays more.
32. **Riskless crypto arbitrage is gone net of costs/latency — so directional bar
    edges certainly are.** Triangular arbitrage yields 0.1–0.5%/cycle, lasts
    fractions of a second, needs 15–50 ms infra, and is arbitraged away / not
    retail-exploitable [64]. If even a *riskless* edge can't beat fees, a
    statistical bar-level directional edge beating buy-and-hold is even less likely.
33. **Maker vs taker fees are a first-class cost lever.** The optimal crypto
    execution policy is to post limit (maker) orders to dodge taker fees [65] — a
    real, large difference our cost model could distinguish (at the cost of fill/
    queue risk). Execution optimization shaves costs; it can't make a losing
    strategy win.
34. **The Bitcoin halving is an overfitting trap (n≈4), and may just be the macro
    liquidity cycle.** Halving event-studies are mixed; the scarcity narrative
    doesn't reliably materialize, and M2 money-supply growth (~0.78 corr, ~90-day
    lag) drove 2020–2023 more than the halving [67]. With ~4 events it cannot clear
    an honest gate; macro-liquidity trend is the more defensible regime variable.
35. **Independent walk-forward replication of our exact design reaches our exact
    thesis.** EMA-crossover on crypto with double-out-of-sample WFO beat buy-and-hold
    in-sample but collapsed to ≈buy-and-hold out-of-sample (worse after 0.1% costs),
    and beat *random* parameter picks only 8–13.7% of the time [68]. The robust win
    was a 50% drawdown cut from blending the active sleeve with buy-and-hold.
36. **Crypto coins move together; lead-lag is barely tradeable.** BTC-ETH causality
    is bi-directional and mostly contemporaneous — no exploitable fixed lag beyond
    1–2 days [69]. A multi-coin basket diversifies *less* than its count suggests
    (high pairwise correlation, especially in stress [53]).
37. **Momentum crashes are conditional, forecastable, and fixed by *risk* scaling.**
    Momentum's worst losses cluster in high-vol post-bear rebounds (loser-leg beta
    >3, winner <0.5 → conditional negative beta); dynamic vol-scaling ~doubles
    Sharpe/alpha [70]. The fix is sizing, not signal — the strongest "sizing >
    signal" datapoint. Crypto's V-shaped recoveries are exactly this danger state.
38. **Liquidity is dynamic (resilience), so spread/depth should worsen transiently
    after volatility.** Obizhaeva–Wang [71]: optimal execution depends on how fast
    the book *recovers*, not static spread — formal backing for a state-dependent
    cost model (wider spread / thinner depth right after big moves [27] and on
    weekends [55]). Impact still negligible at €200; fees + dynamic spread dominate.
39. **Costs destroy 55–90% of even peer-reviewed mean-reversion edges, and the
    fanciest method is the most cost-exposed.** Distance/cointegration/copula pairs
    go 91/85/43 bps gross → 38/33/5 bps net; copula (the "smartest") is gutted to
    5 bps [72]. Simpler cointegration ≈ distance and wins in turbulence — complexity
    rarely pays net (the combination-puzzle lesson [29] again).
40. **Crypto has real, persistent bull/bear/calm regimes (n>>1) — unlike calendar
    effects.** Bayesian HMMs find 3–4 persistent crypto regimes with distinct
    return/vol signatures [73]; bull/bear are persistent, sideways is the switch-
    prone buffer. A long-in-bull / flat-in-bear overlay operates on real structure
    — but detection lags and must be gated net of switching costs [17].
41. **The crypto carry trade — the most-hyped structural crypto edge — just decayed
    into NEGATIVE.** Funding carry Sharpe 6.45 (2020–2025) → 4.06 (2024) → negative
    (2025); only ~40% of top opportunities positive after costs [74]. Our entire
    thesis playing out in real time on a crypto-native, non-directional edge. Show
    the decay, never the headline Sharpe.
42. **Trend-following only hedges *trending* crises, not V-shaped ones.** "Crisis
    alpha" is real on average but intermittent — trend gets whipsawed by sudden
    V-shaped crashes and choppy markets [75][70]. Crypto crashes are often fast/
    V-shaped → a single-coin trend overlay gives inconsistent crash protection;
    don't oversell it as insurance.
43. **Crypto lacks a clean quality/value factor; its supported directional edges are
    momentum + (structural) carry.** Quality (profitability/growth/safety/payout)
    [76] has no clean crypto analogue; the crypto-factor literature [41][12] finds
    value/quality weak and momentum + size the robust ones. On-chain "fundamentals"
    are the only analogue and are thin [40][77].
44. **On-chain / exchange flows are a real but weak, short-horizon, reverse-
    causality-prone feature.** USDT inflows predict higher returns / lower vol at
    1–2h with correlation <0.3 [77]; it's a *flow* signal not a regime signal, and
    likely uneconomic at retail bar frequency. The second data-driven feature class
    to gate honestly (after MVRV [40]).
45. **PEAD is the rare anomaly that survived publication — because it's hard to
    arbitrage.** Post-earnings drift persisted for decades (the "anomalous anomaly")
    precisely because costs/limits-to-arbitrage protect it [78]. Flip side for us:
    liquid-coin price edges have *no* such protection → they decay [25][68]. Crypto
    event-drift would need an event feed + hard multiple-testing gate.
46. **Bitcoin spot ETFs (Jan 2024) are a structural break: more liquidity, lasting
    demand, BTC decoupling from alts.** ETF inflows improved BTC liquidity (tighter
    spreads — good for our cost model) and have a cointegrated lasting price impact
    [80], decoupling BTC from the alt market [69]. Treat pre/post-2024 as different
    regimes in long windows; the launch itself is an un-backtestable n=1 event.
47. **The turn-of-month effect is payday/pension cash flows — absent in 24/7 crypto.**
    Ariel's [-1:+8] TOM window captures most equity monthly return via month-end
    institutional cash deployment [81]; crypto lacks that payroll/rebalance cycle,
    so a TOM overlay is a data-mining candidate (though growing institutional flows
    [80] could *eventually* import some — unestablished). Don't ship without a
    mechanism + OOS survival.

## Methods / findings that hold up (and which don't)

**Hold up (robust, repeatedly confirmed):**
- Time-series momentum exists across asset classes over ~12m lookbacks, *gross
  of costs* and *diversified across many instruments* [1].
- Volatility-scaled position sizing improves risk-adjusted returns [1][6].
- ERC/risk-parity existence, uniqueness, and the 1/σ special case are
  mathematical facts, not fitted results [6].

- Crypto time-series momentum and investor-attention predictability are
  documented in peer-reviewed work [12]; crypto trend factors survive costs
  *cross-sectionally* [10].
- Funding-rate/basis carry in crypto perps is a real, high-Sharpe structural
  edge (market-neutral) [9], underpinned by the general carry theory [18].
- Trend-following has positive returns in *every decade since 1880* and acts as a
  crisis hedge (8/10 worst 60/40 drawdowns) — but that smoothness is driven by
  diversification across ~60 markets we can't replicate on one coin [14].
- Regime-switching de-risking (jump model → cash in bear regimes) cuts drawdowns
  and lifts Sharpe out-of-sample *when switching is penalized* and params are
  picked by cross-validation [17].
- Fama-French: a few factors explain ~90% of diversified return variance vs ~70%
  for CAPM [15] — most single-asset "edge" is idiosyncratic noise.
- The square-root impact law (impact ∝ σ√(Q/V), schedule-independent) is one of
  the most robust empirical regularities — but negligible at retail size [20].

**Don't hold up / decay / need caveats:**
- Naive momentum and pairs-trading profitability **decays over time** and shrinks
  under realistic costs [2][4].
- Mean-variance optimization is fragile (needs expected-return + full-covariance
  estimates); risk parity is preferred precisely because it sidesteps return
  forecasts [6].
- Single-asset, single-window backtests of any of these are prone to overfitting
  the chosen lookback (the J/K grid lesson from [2]).
- **Single-coin BTC technical rules do NOT survive realistic transaction costs**
  — they beat buy-hold gross but lose at 2–4% round-trip fees [11]. This is the
  most directly applicable negative result we have.
- The headline crypto-momentum magnitudes (e.g. 11%/week) are **gross,
  cross-sectional, long-short** spreads over a wide universe — they do NOT
  transfer to a single coin net of costs [12].
- **Volatility-managed portfolios largely FAIL out-of-sample / in real time**
  [22] — the in-sample Sharpe gains [21] rest on unstable fitted scaling. Best
  case is fixed-weight vol scaling; expect lower net Sharpe than buy-hold.
- Statistical-arbitrage / pairs returns roughly **halved post-2010** (15.6% →
  6.4%) as the trade crowded [16] — the best-quantified edge-decay datapoint.
- A reported strategy edge is often **mis-attributed** — contrarian "overreaction"
  profits were mostly lead-lag cross-effects [19]; assume an artifact until the
  gate proves otherwise.
- **Technical (MA-crossover / breakout) rules do not survive out-of-sample.** The
  canonical BLL study looked strong in-sample but was shown to be data-snooping
  and failed out-of-sample after the publication period [30].
- **Published anomalies decay ~26% OOS / ~58% post-publication** [25] — a
  quantified law, larger for high-in-sample-return and illiquid predictors.
- **A headline backtest can be driven by a handful of extreme observations** —
  a 49% stat-arb return collapsed to 10.7% after removing two outlier stocks
  [28]; sub-period stability and leave-out-extremes stress are mandatory.
- **Short-vol / variance-risk-premium "edges" hide tail risk** (negative skew);
  high Sharpe + tiny vol is a red flag, not a green one [23].
- **Optimized ensemble weights underperform equal weights** out-of-sample due to
  estimation noise (the forecast-combination puzzle) [29].
- **Intraday-reversal net edge is largely eaten by state-dependent spread** that
  widens during the volatility spike you are trying to fade [27].
- **Calendar/seasonality effects on crypto are probable noise** — their equity
  causes (settlement, institutional flow cycles) don't exist on a 24/7 coin [26].

## Actionable takeaways for our advisor

1. **Bake off a vol-scaled time-series-momentum rule** (sign of trailing N-period
   return, position size ∝ 1/realized-vol) as a first-class candidate vs
   buy-and-hold [1]. Expect it to *underperform* net of costs on one coin — that
   is the honest hypothesis, and the bake-off should confirm or refute it.
2. **Sizing > signal.** Treat volatility-targeting / inverse-vol throttle as the
   primary lever; entry rules are secondary [1][6]. This aligns with our v3
   vol-overlay work — and the noop-fix precedent means we must e2e-test that the
   scale is actually applied.
3. **Mean-reversion band strategy** (z-score on deviation from a moving average,
   enter at ±kσ, exit at 0) is worth baking off [4], but cost-per-round-trip will
   hurt — count the spread on every entry/exit.
4. **Cost realism = fees + bid-ask spread, not impact** at €200 [3][5]. Don't
   over-engineer market-impact models; do make spread/fees bite on every trade.
5. **Treat any in-sample edge as a decay candidate.** The literature's strongest
   message [2][4] is that crowded edges fade — our 1000-path bootstrap robustness
   gate + buy-hold benchmark is the right defence; keep it frozen.
6. **Make fees/spread the headline decision variable.** [11] shows BTC technical
   rules flip from winning to losing between ~0% and 2–4% round-trip cost. The
   bake-off MUST report post-cost-vs-buy-hold and run a **fee-sensitivity sweep**;
   penalize turnover so short-window crossovers don't win on gross numbers.
7. **Prefer an ensemble/composed trend signal over one tuned SMA** [10] — multiple
   lookbacks blended is more robust than a single fitted window; reuse the
   composed-family sweep engine.
8. **Flag funding-rate / basis carry as the top future feature** [9] — a
   market-neutral, non-predictive crypto edge. Needs a perp+margin+funding model
   and short-side support; not deployable in the current long/flat spot sim, but
   the highest-Sharpe idea in the literature for crypto specifically.
9. **Consider attention/sentiment as a later feature, gated hard** [12] — Google
   /social proxies predict crypto returns in-sample; treat as an experiment behind
   the robustness gate, not a promised edge.
10. **Bake off a regime-flat overlay with hysteresis** [17] — a bull/bear detector
    that moves the single coin to cash in bear regimes, with an explicit
    switching penalty (the JM beat the HMM purely by switching less) and
    hyperparameters chosen by out-of-sample CV. Model the ~2-week detection lag.
11. **Be explicit that vol-targeting probably won't raise net Sharpe** [21][22] —
    ship it (if at all) as a *drawdown/variance* tool with fixed pre-registered
    parameters, validated out-of-sample, e2e-tested for actual application, and
    measured net of its extra turnover. Do not market it as an alpha source.
12. **Use the edge-decay statistics in the presenter narrative** [11][16] — "even
    a peer-reviewed stat-arb edge halved post-2010" and "BTC technical rules lose
    money at 2–4% fees" are the crispest ways to explain to an operator *why* the
    advisor keeps recommending buy-and-hold.
13. **Treat the parameter sweep as multiple hypotheses (deflate the winner)** [30]
    [25]. The best-of-N in-sample config is upward-biased; correct for the number
    of configs tried (White's-reality-check / deflated-Sharpe mindset) and reserve
    genuine out-of-sample data the sweep never saw. MA-crossover/breakout
    candidates are the *most* likely in-sample mirages.
14. **Default ensembles to equal / fixed weights, not learned weights** [29].
    The combination puzzle says estimated optimal weights underperform OOS;
    learned weights must beat equal-weight by a wide gated margin to justify the
    extra parameters. This validates the composed-family blend over a single
    fitted strategy.
15. **Rank on left-tail / skew, not just mean Sharpe** [23]. A candidate with high
    Sharpe and strongly negative skew (smooth then crashes — the short-vol/carry
    profile) should be penalized by the weakest-link gate; report bootstrap
    left-tail quantiles and skewness alongside Sharpe.
16. **Make spread state-dependent (wider on volatile/extreme bars)** [27]. A flat
    spread overstates the net edge of any reversal/dip-buying rule, because the
    real spread widens exactly when such rules want to trade.
17. **Add explicit robustness stresses to the bake-off** [28]: sub-period split
    (does it hold in both halves?) and leave-out-top-k-contributions (does the
    edge survive removing its best few trades?). A Sharpe that halves when two
    trades are removed is not robust.
18. **Don't ship a calendar/seasonality overlay without hard gating** [26]. On
    24/7 crypto these effects lack their equity causes; any apparent day-of-week
    /turn-of-month edge is a prime multiple-testing artifact — pre-register and
    demand out-of-sample survival.
19. **Adopt a t > 3 significance hurdle, not t > 2, for declaring an edge** [42].
    The multiple-testing literature raised the bar because most published factors
    are false positives; our sweep tests far more configs than they did. This is
    the t-space partner of the Deflated Sharpe Ratio [32].
20. **Gate stop-loss / exit overlays by a trend filter** [38]. Apply trailing/vol
    stops only to the trend candidate (positive serial correlation → stops help);
    do NOT apply them to mean-reversion (stops actively hurt). Prefer slower/wider
    stops; e2e-test divergence; net the extra round-trip costs.
21. **Treat any Sharpe ≫ 2 over < 2 years of crypto as a gate failure until
    proven** [44][28]. Eye-popping backtest Sharpes almost always reflect thin
    samples, survivorship in selection, or unmodeled slippage — the exact failure
    the frozen bootstrap + DSR + sub-period stress exist to catch.
22. **On-chain valuation (MVRV-Z/NUPL/CVDD) is the top data-driven feature to
    evaluate** [40] — a crypto "value" signal distinct from price TA, with an
    honest random-entry control — BUT gate hard: only 3 cycles (effective n≈3),
    so deflate heavily and check for realized-value lookahead. Needs an on-chain
    feed.
23. **For mean-reversion, tune a bounded entry interval and couple exit to stop**
    [39], and first verify the coin's MA-deviation is OU-stationary; if it isn't
    (drift/regime), mean-reversion is the wrong family for that coin [33].
24. **Report compound/geometric return; value vol-reduction via the ½σ² drag**
    [35]. A single buy-and-hold coin earns zero diversification/rebalancing
    return, so it is the honest single-asset ceiling — the strongest argument for
    a *future multi-coin rebalanced* mode being the one place a structural
    (non-predictive) edge over holding plausibly exists.
25. **Match each strategy family to its theoretically-correct horizon band** [49].
    Reversion belongs at very-short and very-long scales, trend at the
    intermediate (days–weeks) scale; do NOT sweep every family over every
    lookback (manufactures multiple-testing [42] and fits families in regimes
    where they shouldn't work). Test horizon-by-horizon; expect sign-instability
    across mismatched horizons.
26. **Treat rebalancing/trading frequency as a cost-penalized parameter with an
    optimum** [51][35]. Even the growth-optimal (Kelly) strategy goes bankrupt if
    it rebalances too often under realistic costs — the formal backbone of
    "penalize turnover" and of why low-turnover/hold is the honest single-coin
    default. Any adaptive frequency [48] must net costs explicitly.
27. **Never rank on forecast accuracy — rank on net-of-cost equity vs buy-hold**
    [50]. A low price-MAPE (or high directional accuracy) is not an edge; many
    "ML predicts crypto" papers never backtest with costs. Convert every signal
    to positions → net equity → robustness gate; weight external claims by
    whether they actually backtested net of costs.
28. **Add a changepoint / turning-point filter to the trend candidate** [56]. The
    most fragile moment for momentum is the trend reversal — a simple regime-break
    or volatility-break filter that sizes down / flips right after a detected break
    is the defensible (non-LSTM) version of the deep-momentum-network result.
    Judge it on drawdown and net-vs-buy-hold, not relative-to-plain-momentum.
29. **Test a short-horizon trend overlay separately for its convexity, not return**
    [52][36]. Long-horizon trend ≈ buy-and-hold (redundant with market beta); the
    short-horizon sleeve is the convex crisis-diversifier. Bake them off separately
    and reward the short sleeve's positive-skew/drawdown profile, expecting the
    long sleeve to roughly tie buy-and-hold.
30. **Measure cost as implementation shortfall vs the decision-bar (arrival) price,
    treated as a distribution** [57]. Account each simulated trade's cost against
    the arrival price (spread + slippage + delay), and treat the cost itself as a
    state-dependent random draw [27], not a constant — the same distributional
    discipline our return-bootstrap already uses.
31. **Bake off a "distance-from-52-week-high" long/flat rule** [60]. Price / 52w-high
    is a single-coin-computable momentum signal that (in equities) subsumes
    past-return momentum and doesn't reverse long-run; cheap to add, test whether
    the time-series use holds on one coin vs buy-hold, expect momentum-style
    turning-point fragility [56] and public-signal decay [25].
32. **Respect crypto's inverse leverage effect in any vol overlay** [58]. Crypto
    vol can spike in *rallies*, not just crashes, so an equity-calibrated
    "de-risk when vol rises" overlay [21][22] may cut exposure during up-moves
    and mis-time the coin. Calibrate vol behavior to crypto, and note holding BTC
    is being paid (~0.8 Sharpe) largely for upside vol/jump risk — a high bar.
33. **Treat predictable crypto *flow* events as transient, decaying edges** [54].
    Exchange listings, ETF-driven inflows, and index-product rebalances are the
    crypto analogues of the index-inclusion effect that decayed from ~7.4% to <1%;
    any flow-front-running idea should be assumed to shrink as it becomes
    anticipated, and gated accordingly.
34. **Do NOT ship a calendar/time-of-day overlay; the one real pattern is a cost
    input** [55]. Crypto day-of-week effects are aggregation artifacts (Sunday
    23:00 UTC US re-entry) that vanish under intraday controls and decayed post-2015.
    The only durable fact is lower weekend liquidity → wider weekend spreads in the
    cost model, not a tradeable return.
35. **Adopt double-out-of-sample / walk-forward evaluation and report the
    beat-random-parameter bootstrap fraction** [68]. Optimize on a window, evaluate
    ONCE on truly unseen data; report what fraction of bootstrap iterations the
    crowned config beats *random* parameter picks (the crypto EMA study got only
    8–13.7%). This is the operational form of the DSR/t>3 selection discipline [32][42].
36. **Offer an active+buy-and-hold blend, not pure active** [68][47][21]. The one
    robust win across the honest studies is that blending the active sleeve with a
    buy-and-hold core cuts drawdown ~50% with little return give-up — a better
    default product than an all-in active strategy.
37. **Build the mean-reversion candidate as an OU s-score on a detrended residual**
    [62][39]. Define the signal as deviation-in-σ of the residual after removing a
    slow trend (or a BTC-beta for an alt), enter ≈ ±1.25σ / exit ≈ 0, and require
    OU-stationarity before trusting it; restrict it to liquid coins [33][66] and net
    a state-dependent spread [27].
38. **Scope out HFT/latency arbitrage explicitly** [64]. Triangular and cross-rate
    arbitrage need millisecond infrastructure and are gone net of fees — outside our
    honest, retail, bar-frequency design. Use as a presenter point on why retail
    bar-level edges don't survive.
39. **Consider a maker/taker-aware cost model option** [65]. Posting limit (maker)
    orders genuinely costs less than crossing (taker), so a strategy that can wait
    pays less — but model the fill/queue risk; default to taker-style crossing for
    conservatism.
40. **Do NOT ship a halving-timing overlay; prefer macro-liquidity trend if
    anything** [67]. n≈4 events cannot clear the gate, the scarcity narrative
    doesn't materialize, and M2 liquidity (not the halving) drove the last cycle —
    so a halving rule risks spuriously capturing the macro cycle and failing when
    they decouple.

## Open questions / things worth testing in our app

- Does a vol-scaled TSM rule beat buy-and-hold on BTC/ETH net of costs over
  multiple windows, or does it confirm the "no active strategy wins" thesis? [1]
- What lookback N is most *stable* (not most profitable) across windows — i.e.
  flat performance across the J/K-style grid rather than a single peak? [1][2]
- Does inventory-skew sizing (AS-style, exposure pushed toward neutral by vol)
  beat fixed sizing or simple vol-targeting on one coin? [5]
- For a future multi-coin advisor: does ERC/risk-parity across a small basket
  beat single-coin buy-hold net of rebalancing costs? [6]
- At what all-in round-trip cost does each baked-off strategy cross from
  beating to losing vs buy-hold on our coin? (Replicate [11]'s fee sweep.)
- Does a blended multi-lookback trend signal beat the best single SMA on one coin
  *out-of-sample*, or does the blend just overfit differently? [10]
- Is funding-rate carry worth adding a perp+margin engine for — does the
  simulated net carry (after perp fees + funding frequency + liquidation buffer)
  still beat spot buy-hold? [9]
- Do attention/sentiment proxies add anything over price-only signals once passed
  through the robustness gate? [12]
- Does a regime-flat overlay (de-risk to cash in bear regimes, with a switching
  penalty) beat single-coin buy-hold net of costs, or does crypto's faster/noisier
  regimes + detection latency erase the benefit? [17]
- Does our vol-overlay's equity actually diverge from baseline AND survive costs
  out-of-sample — or does it replicate the [22] in-sample-only mirage? [21][22]
- Does a trend (wins in trends) + mean-reversion (wins in ranges) ensemble beat
  either standalone on one coin, exploiting their negative correlation? [7][13]
- Does a changepoint/regime-break filter on the trend candidate reduce drawdown
  at turning points enough to beat buy-hold net of costs, or just add turnover? [56]
- Does a short-horizon trend overlay deliver measurable crisis-convexity (positive
  skew / smaller drawdown) on one coin, while the long-horizon version just ties
  buy-and-hold? [52][36]
- Does a single-coin "distance-from-52-week-high" long/flat signal beat buy-hold
  out-of-sample, or does the cross-sectional anchoring effect fail to transfer to
  a pure time-series use? [60]
- Does crypto's inverse leverage effect [58] break an equity-style vol-targeting
  overlay (de-risking during rallies)? Compare a crypto-calibrated vol overlay vs
  the naive equity one.
- If we ever ingest macro/on-chain regime variables, does a trend-on-fundamentals
  signal [61][40] beat a price-only trend signal through the gate?

## Paper map (claim → supporting [N])

- Time-series momentum exists & vol-scaling drives its Sharpe → [1]
- Cross-sectional momentum (winners−losers ~1%/mo); long-horizon reversal → [2]
- Optimal execution = E[cost]+λVar; perm/temp impact; front-loading → [3]
- Pairs trading = ±2σ distance-method mean reversion; profits have decayed → [4]
- Market-making optimal spread & inventory-skew; vol widens spread → [5]
- Risk parity / ERC; 1/σ weights under equal correlation; avoids return forecasts → [6]
- Value & momentum negatively correlated → combine for higher Sharpe → [7]
- Kelly f*=μ/σ²; use fractional Kelly; vol is estimable, return isn't → [8]
- Perpetual funding mechanism; cash-and-carry basis trade (high Sharpe) → [9]
- Crypto trend factor (ML over 28 signals) beats momentum, survives costs (cross-sec) → [10]
- Single-coin BTC technical rules beat B&H gross, LOSE net of 2–4% fees → [11]
- Crypto returns = distinct class; strong TS momentum + attention predictors → [12]
- Long-horizon (3–5y) reversal coexists with short-horizon momentum (overreaction) → [13]
- Trend-following positive every decade since 1880; crisis hedge; multi-horizon+vol-scaled → [14]
- Few factors (mkt/size/value) explain ~90% of variance; rest is noise → [15]
- Pairs/stat-arb returns halved post-2010 (15.6%→6.4%); 5 method families → [16]
- Regime-switching jump model de-risks to cash; cuts drawdown, low turnover, OOS-CV-tuned → [17]
- Carry = "return if price doesn't move"; universal predictor; crashes in stress → [18]
- Contrarian profits ≠ overreaction; mostly lead-lag cross-effects (attribution caution) → [19]
- Square-root impact law: impact ∝ σ√(Q/V), schedule-independent, concave → [20]
- Vol-managed portfolios: big IN-SAMPLE Sharpe gains from inverse-vol scaling → [21]
- Vol-managed portfolios FAIL out-of-sample/real-time (unstable params); fixed-weight best → [22]
- Risk premia share negative-skew signature; short-vol Sharpe 1.26 but tail-risky → [23]
- Betting-against-beta = leverage-constraint structural premium; "bad beta" refinement → [24]
- Anomaly decay: ~26% OOS, ~58% post-publication; larger for high-return/illiquid → [25]
- Day-of-week effect lives in scaling structure (Monday); calendar effects subtle → [26]
- Intraday reversal after extreme moves real but spread widens during spike (1.6%→0.44%) → [27]
- Graph-clustering stat-arb 49% return → 10.7% after removing 2 outlier stocks (overfit) → [28]
- Forecast-combination puzzle: equal weights beat estimated optimal weights (est. noise) → [29]
- Technical MA/breakout rules (BLL): in-sample strong, data-snooping, fail OOS post-1986 → [30]
- Market intraday momentum: first half-hour predicts last half-hour, stronger on vol/news days → [31]
- Deflated Sharpe Ratio: deflate the best-of-N backtest winner for selection bias + non-normality → [32]
- Crypto momentum exists ONLY in liquid coins; illiquid coins mean-revert; liquidity premium = spread → [33]
- Opening-range-breakout day trading "profitable" only under zero-spread + 4× leverage assumptions → [34]
- Diversification return / rebalancing premium ≈ ½[Σwᵢσᵢ²−σp²]; geometric ≈ arithmetic − ½σ² → [35]
- Trend horizons: medium (60/125d) redundant noise; short+long "barbell" beats dense grid → [36]
- VIX-futures roll-yield = short-vol carry (contango ~80%); ML times entry/exit of structural edge → [37]
- Stop-losses help ONLY under momentum/positive serial correlation; hurt under mean-reversion → [38]
- Optimal OU mean-reversion: entry is bounded interval above stop; higher stop → lower take-profit → [39]
- On-chain BTC metrics (MVRV-Z/NUPL/CVDD) beat buy-hold + random-entry, but only 3 cycles (thin) → [40]
- Crypto 3-factor model (market/size/momentum); size effect disappears OOS; value weak → [41]
- Multiple-testing hurdle: a real factor needs t > 3 (not 2); most of the factor zoo is false → [42]
- 447 anomalies replicated: 64% fail at 5%, 85% fail at t>3; microcaps manufactured most → [43]
- Crypto cointegration pairs claim Sharpe ~8 over 13 months — thin sample, no OOS/decay test (skeptic) → [44]
- Ensemble RL (PPO/A2C/DDPG, pick best by validation Sharpe) Sharpe 1.30 > agents/DJIA; turbulence de-risk → [45]
- Volmageddon: crowded short-vol lost ~95% in a day via rebalancing feedback loop; carry tail risk → [46]
- Dual momentum: relative mom adds return, absolute (time-series) mom cuts drawdown/vol (de-risk switch) → [47]
- Crypto vol-scaled trend-following (adaptive sizing) claims risk-adjusted alpha vs buy-hold → [48]
- Time-scale hierarchy: reversion at very-short + very-long horizons, trend in the middle (days–months) → [49]
- Crypto Twitter-sentiment "prediction" reports MAPE only — NO backtest/costs (forecast ≠ profit) → [50]
- Frequency-based log-optimal (Kelly) portfolio: too-frequent rebalancing under costs → bankruptcy → [51]
- CTA replication: long-horizon trend ≈ market beta (82% corr, redundant), short-horizon trend = convex diversifier (24% corr); fees gut the benchmark (0.03 vs 0.49 Sharpe) → [52]
- Dispersion trading: sell index vol / buy component vol; ~15% p.a. but −43% crash drawdown (negative-skew correlation-risk premium) → [53]
- The S&P 500 index-inclusion effect decayed ~7.4% (1990s) → <1% (mechanical flow edge arbitraged away) → [54]
- Crypto day-of-week effect is an aggregation artifact (Sunday 23:00 UTC US re-entry); vanishes intraday; decayed post-2015 → [55]
- Slow Momentum with Fast Reversion: changepoint detection on a deep momentum net improves Sharpe ~1/3 (~2/3 in 2015–2020); turning points are the fragile moment → [56]
- Bayesian TCA: rank execution by implementation shortfall (arrival-price slippage) as a noisy latent variable, not point estimates → [57]
- Bitcoin risk premia: BP ~66%/yr, variance risk premium ~14%/yr; inverse leverage effect; upside-driven premium (vs equities' crash-driven) → [58]
- Option-implied vol skew negatively predicts returns; structural sources = business cyclicality + default risk → [59]
- 52-week-high momentum subsumes past-return momentum, doesn't reverse long-run; behavioral anchoring; single-coin-computable → [60]
- Macro momentum: trend on macro fundamentals (growth/inflation/policy) across assets; crisis-diversifier; underreaction to slow info → [61]
- Avellaneda–Lee stat-arb: PCA/ETF residuals as OU; s-score (deviation in σ) entry ±1.25 / exit 0; Sharpe ~1.44 (decayed post-2003) → [62]
- Low-volatility anomaly: low-vol/low-beta earn higher risk-adjusted returns (contra CAPM); leverage constraints + lottery/skew preference → [63]
- Crypto triangular arbitrage: 0.1–0.5%/cycle, sub-second, needs 15–50 ms; arbitraged away, not retail-exploitable net of fees → [64]
- Crypto limit-order RL: PPO learns to post maker orders to dodge taker fees; queue imbalance predicts short-term moves → [65]
- Short-term (weekly) reversal: ~1.7%/week gross (Lehmann) = liquidity-provision premium; survives only large-cap net of spread → [66]
- Bitcoin halving event-study: mixed CARs, scarcity narrative unconfirmed; n≈4; M2 liquidity drove 2020–2023 more than halving → [67]
- Walk-forward EMA on crypto: in-sample beats B&H, OOS ≈ B&H (worse after 0.1% cost); beats random params only 8–13.7%; blend cuts DD 50% → [68]
- BTC-ETH lead-lag: bi-directional, mostly contemporaneous, barely tradeable; coins co-move (less diversification than count) → [69]
- Momentum crashes: post-bear-rebound, high-vol, conditional negative beta (loser β>3); dynamic vol-scaling ~doubles Sharpe → [70]
- Obizhaeva–Wang: optimal execution depends on book *resilience* (recovery speed), not static spread; transient impact; discrete trades → [71]
- Pairs distance/cointegration/copula: 91/85/43 bps gross → 38/33/5 bps net; copula most cost-exposed; cointegration best in turbulence → [72]
- Bayesian HMM crypto: 3–4 persistent bull/bear/calm regimes; sideways = switch-prone buffer; regime structure is real → [73]
- Crypto carry trade: Sharpe 6.45 (2020–2025) → 4.06 (2024) → negative (2025); ~40% positive after costs (real-time decay) → [74]
- CTA crisis alpha "myth or reality?": real on average but intermittent; trend hedges trending crises, whipsawed by V-shaped/choppy → [75]
- Quality Minus Junk: long quality (profit/growth/safety/payout) / short junk; modest price impact → high risk-adj returns; no clean crypto analogue → [76]
- On-chain flows: USDT inflows predict higher returns/lower vol at 1–2h, corr <0.3; flow not regime signal; reverse-causality debated → [77]
- PEAD: drift after SUE-decile earnings surprises; underreaction; survived publication (hard to arbitrage = "anomalous anomaly") → [78]
- Crypto intraday: momentum AND reversal coexist, regime/jump/FOMC-conditional; timing beats B&H intraday but reversion turnover cost-prohibitive → [79]
- Bitcoin spot ETFs (Jan 2024): improved liquidity, lasting cointegrated demand, BTC decoupling from alts (structural break) → [80]
- Turn-of-month effect: Ariel [-1:+8] window captures most equity monthly return via payday/pension cash flows; absent in 24/7 crypto → [81]
