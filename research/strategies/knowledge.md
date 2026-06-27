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
