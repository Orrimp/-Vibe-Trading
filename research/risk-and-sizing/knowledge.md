# Knowledge — Risk & Position Sizing

*Synthesis of the `risk-and-sizing` ledger (`papers.md`). Updated every ~5 papers.*
*Context: our app is a single-coin crypto paper advisor; buy-and-hold is the
benchmark; validated thesis is "no active strategy robustly beats holding net of
costs." This topic asks: given a signal, HOW MUCH to hold, and which sizing/risk
rules survive out-of-sample.*

## Key themes

1. **Sizing is a real lever, but a double-edged one.** Growth-optimal (Kelly) sizing
   genuinely maximizes long-run wealth growth [4][6], but full Kelly is far too
   aggressive — huge bets, deep drawdowns, finite-horizon ruin risk [3][6]. The
   deployable form is *fractional* Kelly (½ or less).
2. **The in-sample vs out-of-sample gulf for volatility timing — and the resolution.**
   Vol-managed portfolios look great in spanning regressions [1] but a real-time
   investor gets no robust Sharpe gain [2]. The reconciliation [16]: vol targeting
   is a reliable RISK-shaping tool (thinner tails, lower max drawdown, less
   vol-of-vol — near-universal across 60+ assets) but an unreliable RETURN/Sharpe
   tool (Sharpe gain shows up ONLY for "risk assets" with a strong leverage effect,
   i.e. negative return-vol correlation: equities/credit, not bonds/FX/commodities).
   This is THE cautionary tale for our overlay and mirrors our whole product thesis:
   expect drawdown/variance benefit, not net Sharpe.
3. **Estimation error dominates, and it's asymmetric.** Errors in the MEAN return
   are ~20× as damaging as errors in covariances and ~10× as damaging as errors in
   variances [6]. Sizing rules that lean on μ̂ (Kelly) are fragile exactly where our
   single-coin μ̂ is weakest; sizing rules that lean only on σ/covariance are far
   more forgiving.
4. **Vol-targeting ≈ a crude Kelly.** Kelly fraction f ≈ μ/σ² [4]: the 1/σ² term IS
   inverse-variance scaling. So our vol-targeting overlay is Kelly with μ held
   constant — its de-risking-in-high-vol behavior is the well-founded half; the
   lever-up-in-low-vol half is the dangerous half.
5. **Risk-parity / ERC is a multi-asset tool.** Equal-risk-contribution [7]
   diversifies risk across assets; largely N/A to a single coin, but its
   tail-risk (Expected Shortfall) and bootstrap-de-noising ideas transfer.
6. **Drawdown control = scale position by the cushion (distance to the floor).**
   The recurring, robust, implementable idea across [10][12][13]: hold risky
   exposure proportional to how far current equity is above a high-water-mark
   floor; de-risk to zero as you approach the max-drawdown limit. This GUARANTEES
   (idealized) the drawdown the user asks for and naturally becomes a stop-loss at
   the limit — but it COSTS terminal return (you de-risk into recoveries).
7. **Stop-losses only help under momentum, not random walk** [8]: under IID returns
   a stop ALWAYS lowers expected return; it earns a "stopping premium" only when
   losses predict more losses. Even optimally-tuned stops beat the no-stop baseline
   in barely >50% of assets [9].
8. **Gap risk breaks every floor guarantee on a jumpy asset.** CPPI / drawdown
   controllers assume continuous trading; a crypto jump larger than the cushion
   between rebalances breaches the floor [11][13]. Discrete bars + jumps ⇒ the
   "probability-one" max-drawdown promise is only approximate.
9. **The "leverage effect" is the hidden requirement for vol-timing Sharpe gains.**
   Vol targeting injects implicit momentum/crash-avoidance ONLY when returns and
   volatility are negatively correlated [16]; without that asymmetry it just reshapes
   risk. So whether our overlay can lift Sharpe is a per-coin empirical question
   about its return-vol correlation — not a given.
10. **Sizing, not signal, drives celebrated strategies.** Much of time-series
    momentum's performance comes from the inverse-vol position SIZING, not the trend
    signal [14] — the sizing-side echo of our "sizing > signal" theme. But the
    crisis-alpha needs cross-asset diversification a single coin can't get.
11. **We can only de-risk, never lever.** Long-only €200 spot ⇒ the academically
    optimal "lever a low-vol asset up to target" (Kelly/vol-target/BAB) is off the
    table [4][15][16]; only the defensive half (cut exposure, hold cash) is available.
12. **Risk of ruin falls ~exponentially as you cut per-bet size** [17]; even a
    positive-edge strategy is ruined if bets are too large — the math behind
    fractional-Kelly + a hard position-size floor.
13. **Drawdown-CONSTRAINED Kelly dominates plain fractional Kelly.** Instead of an
    ad-hoc "½ Kelly," impose a convex bound on drawdown PROBABILITY and maximize
    growth subject to it — one risk-aversion parameter, and it beats fractional Kelly
    at equal drawdown risk OR equal growth [30]. The rigorous (model-independent)
    constrained-growth optimum is literally a transform of the unconstrained Kelly
    portfolio [31] — so "scale the Kelly position by a drawdown-state function"
    ([10][12][13][25]) is the correct solution, not a hack.
14. **CVaR (Expected Shortfall) is the right tail-risk OBJECTIVE/metric; VaR is not.**
    VaR is non-convex and not sub-additive (diversification can raise it); CVaR is
    coherent and minimizable by a convex/LP auxiliary function [34]. For us CVaR is a
    trivial readout from the bootstrap loss distribution — report it instead of VaR.
15. **Regime-gated de-risking is the most promising "active" risk tool — with the
    usual caveat.** A persistence-penalized regime model (statistical jump model)
    that goes to cash in detected bear regimes HALVED equity-index drawdowns
    (S&P −55%→−27%) and even raised Sharpe, net of costs and delay [35]. But it was
    on equity indices (persistent crash regimes), the penalty was CV-tuned (overfit
    surface), and it still retimes the market — replicate under our gate, expect the
    drawdown benefit to be more robust than the Sharpe benefit.
16. **HRP / risk parity work BECAUSE they avoid covariance inversion and μ̂.**
    "Markowitz's curse": inverting an ill-conditioned covariance matrix makes
    MVO unstable and overfit; HRP's clustering + inverse-variance allocation beats
    even min-variance OOS [32][33]. Multi-asset (N/A to one coin) but the strongest
    cross-asset endorsement of estimation-light, variance-based, inversion-free sizing.
17. **Robustness ↔ conservatism: size for the worst-case distribution, not the
    point estimate.** Distributionally-robust Kelly maximizes worst-case growth over
    an ambiguity set and is provably long-run optimal under uncertainty [42]; adding
    Knightian uncertainty / fat tails on top of variance pushes the optimal fraction
    DOWN further [43]. Both give a rigorous backing for our "shrink hard, the coin's
    distribution is unknowable" instinct — and a wide ambiguity set on a no-edge coin
    drives the bet toward zero (≈ just hold / don't actively size).
18. **Coherent DRAWDOWN risk measures exist and are optimizable.** CDaR (mean of the
    worst (1−β) drawdowns) is the drawdown analogue of CVaR, LP-formulable, with max-
    and average-drawdown as limiting cases [41]. Expected max drawdown of a process
    grows log T / √T / linear-T for positive / zero / negative drift [39] — so on a
    near-zero-drift coin, drawdown GROWS with the holding horizon from randomness
    alone; a large MDD is not by itself evidence of a bad strategy, and the horizon
    must always be stated.
19. **Crypto's fat left tail is partly a LEVERAGE-CYCLE artifact.** Deleveraging /
    margin spirals [37] and crypto liquidation cascades produce fundamental-free
    crashes and the gap/jump risk that breaks floor guarantees [11][13]. The
    UNLEVERED holder is the structurally-advantaged "natural buyer" who survives the
    spiral while levered holders are force-liquidated at the bottom — a deep
    vindication of our no-leverage design. System-wide leverage/funding stress is a
    candidate regime de-risk signal.
20. **Retail value is partly DEBIASING, not just returns.** The disposition effect
    (sell winners early, hold losers) is empirically present in Bitcoin [40]; a
    rules-based sizing/exit plan beats the user's own biased behavior even if it does
    not beat buy-and-hold on return — a legitimate reason to offer disciplined plans.
21. **Be conservative on the INPUTS, not just the multiplier.** Concrete recipe:
    feed a LOWER-QUANTILE (pessimistic) estimate of edge/probability AND an
    UPPER-QUANTILE estimate of vol into any sizing rule [45][19]; the quantile level
    is an interpretable conservatism dial. A chance-constraint ("only size up if
    positive expected return at high confidence") is a significance gate before
    sizing — aligned with our FROZEN gate.
22. **Stops: pair, don't solo; and they're mostly cost.** A trailing stop ALONE is
    provably suboptimal — pair it with a profit-take (an upper exit) [44]; the
    triple-barrier (upper+lower+time) [38] is the disciplined form. But the value of
    any stop still hinges on momentum/mean-reversion structure [8][44]; on a
    near-random-walk coin expect drawdown reduction at a return cost.
23. **Optimal-f = aggressive Kelly cousin; same verdict.** Vince's optimal-f / TWR
    maximizes growth and has well-posed unique optima [47] but punishing drawdowns;
    the deployable form is the drawdown-corrected fraction [25]. Don't expose raw
    optimal-f to a retail user.
24. **Vol scaling can LOWER net Sharpe, not just fail to raise it.** On futures
    momentum, vol scaling raised raw return (15.3% vs 11.5%) but cut Sharpe
    (0.39 vs ~0.59), and the extra turnover's costs make it worse net [48] — the
    cost-aware twin of [2][28]. Only low-turnover / conditional vol targeting is
    cost-survivable.
25. **Report MEDIAN, not mean; we are never in Kelly's asymptotic regime.** Triple
    Kelly's Monte-Carlo mean wealth was 940× but its MEDIAN was 0.017× (near-certain
    wipeout) [55] — overbetting's average is a rare lottery path masking typical ruin.
    And Kelly's superiority only shows up after ~10k–40k trades ("the long run had to
    be really long") — a single coin over a few years has too few trades, so the
    growth guarantee is unavailable and finite-sample ruin risk dominates → shrink
    hard or don't size on μ. Our bootstrap already reports the distribution; size for
    the median investor.
26. **Vol targeting DOES control crypto risk — as a small, cash-diluted sleeve.**
    A Boyd-group study folds crypto in as a 10% sleeve diluted with cash to a vol
    target (EWMA/GARCH(1,1)) and gets good risk-adjusted results [49] — but the win
    rests on DIVERSIFICATION + a SMALL allocation, neither of which a single 100%-crypto
    coin has. Endorses dilute-with-cash-to-a-vol-target as our (no-leverage) risk lever;
    does NOT promise beating buy-and-hold on one coin.
27. **Don't build RL / heavy-ML sizing.** Learning a good Kelly-objective sizing
    policy took ~2M steps ≈ 8,000 years of daily data even on clean simulation with a
    KNOWN optimum [50]; on one noisy non-stationary coin it can only overfit. Prefer
    closed-form, parameter-light sizing. Better vol POINT forecasts also don't
    guarantee better TAIL (VaR/ES) forecasts [51] — measure the tail directly, don't
    over-engineer σ̂.
28. **Skew is a first-class risk metric; distrust smooth-Sharpe negative-skew.**
    Most "risk premia" are paid for NEGATIVE skewness (insurance-selling: steady gains,
    rare catastrophic loss); Sharpe rises ~linearly with negative skew [54]. A smooth
    high-Sharpe backtest with negative skew is likely an unexploded tail bomb (martingale/
    averaging-down/short-vol-like) — our gate must surface it; report SKEW. The
    desirable exception is TREND/loss-cutting, which has POSITIVE skew [54] — aligning
    with "trend beats puts as a tail hedge" [53].
29. **Tail hedging: puts BLEED, trend is the better-value hedge.** Rolling OTM puts
    earned −0.61%/yr (Sharpe −0.61) over 35 yrs from the volatility risk premium, while
    multi-asset trend earned +8.7%/yr (Sharpe 0.84) with −0.08 equity correlation —
    AQR prefers Trend over Put [53]. For our spot, no-options app the realistic "tail
    hedge" is the defensive de-risking of a trend/regime rule, judged on LONG-RUN net
    cost (the bleed dominates), not crash payoff alone.

## Methods / findings that hold up (and which don't)

**Holds up:**
- Fractional Kelly as the practical sizing rule; "never bet more than full Kelly"
  is a hard ceiling (2× Kelly drives long-run growth to the risk-free rate) [6].
- Inverse-variance / vol scaling reliably REDUCES portfolio variance and drawdown
  (the mechanical effect is real) [1][2][4].
- Mean-estimation error is the dominant risk in any μ-dependent sizing [6].
- Bootstrap / resampling for honest, tail-aware risk estimates [7] (mirrors our gate).
- Expected Shortfall (CVaR) over plain σ for fat-tailed assets [7].

- Drawdown control via "scale by the cushion" / risk-aversion ramp — reliably
  caps realized drawdown (idealized markets) [10][12][13]; one paper even reports
  "little or no sacrifice of mean-variance efficiency" in a multi-asset HMM
  setting [12] (treat as best-case, verify for single coin).

**Does NOT hold up / fragile:**
- Vol-managed portfolios delivering a robust *Sharpe* improvement out-of-sample —
  the spanning-regression alpha is not implementable in real time and OOS
  combination strategies underperform simple holding in ~72/103 cases [2].
- Full Kelly as a deployable strategy (too aggressive; ruin risk) [3][6].
- The lever-up half of vol-targeting for a long-only, no-leverage retail account —
  Moreira–Muir's own gains need 400–864% leverage at the 99th pct [2].
- Stop-losses as a return enhancer — value-destroying under a random walk, and
  even optimized stops help in barely >50% of assets [8][9].
- Floor/drawdown GUARANTEES on a gapping asset — broken by jumps between
  rebalances [11][13].

## Actionable takeaways for our advisor

1. **Treat the vol-targeting overlay as a drawdown/variance tool, not a Sharpe
   engine.** Validate it by DIRECT comparison vs the un-targeted baseline equity
   (our mandatory baseline-divergence e2e test), never by a regression alpha that
   can smuggle in future info [2]. Expect Sharpe gains to be fragile; expect
   variance/drawdown reduction to be real.
2. **Never size at full Kelly from a single-coin μ̂.** If we ever offer Kelly-style
   sizing, shrink hard (½ or ¼) and cap it; better, prefer vol-only sizing because
   μ-errors cost ~20× more than σ-errors [6]. A principled dial: map fractional
   Kelly to a negative-power-utility δ (½K ↔ δ=−1, ¼K ↔ δ=−3) [6].
3. **Honor the no-leverage reality.** A €200 spot advisor can de-risk (sell to cash)
   but cannot lever up; so only the *defensive* half of vol-targeting is available
   [2][4]. Frame the overlay accordingly.
4. **Report tail risk (ES/CVaR), not just σ or Sharpe** [7]; crypto is fat-tailed
   and negative-skewed, which also inflates Sharpe-estimator variance.
5. **Watch for regime breaks ("structural instability").** The OOS failure of
   vol-timing was driven by parameter breaks [2]; our crypto windows are regime-shifty,
   so any trained sizing parameter goes stale — argues for simple, slow-moving rules.

## Open questions / things worth testing in our app

- Does our vol-targeting overlay's equity curve actually have LOWER drawdown/variance
  than buy-and-hold on real coin windows, even when Sharpe is unchanged? (Direct test.)
- Half-Kelly vs quarter-Kelly vs fixed-fraction vs vol-target: bake them off under the
  FROZEN gate on the same (coin,window) and compare net-of-cost terminal wealth +
  max drawdown. Hypothesis: defensive sizing cuts drawdown but not net Sharpe.
- If we size by f ≈ μ̂/σ̂², how badly does plugging a noisy single-coin μ̂ blow up vs
  setting μ̂=0 (pure vol scaling)? (Estimation-error stress test, per [6].)
- Is the exact log-normal Kelly fraction [4] (more conservative than μ/σ²) materially
  safer on crypto's high-σ regime than the μ/σ² approximation?

## Paper map (claim → supporting [N])

- Vol-managed portfolios show big in-sample alphas / Sharpe gains → [1]
- ...but no robust OUT-OF-SAMPLE Sharpe gain for a real-time investor → [2]
- Vol-timing's gains need extreme (400–864%) leverage / concentrated in momentum → [2]
- Full Kelly maximizes long-run growth but is too aggressive (drawdowns, ruin) → [3][6]
- Kelly fraction f ≈ μ/σ² (inverse-variance scaling ≈ crude vol targeting) → [4]
- Exact log-normal Kelly is more conservative than μ/σ² as vol rises → [4]
- Kelly is highly sensitive to estimation error in μ → [4][5][6]
- Mean errors ~20× costlier than covariance errors, ~10× variance errors → [6]
- Never bet more than full Kelly; 2× Kelly → risk-free growth → [6]
- Fractional Kelly ↔ negative-power utility (½K=δ−1, ¼K=δ−3) → [6]
- Options / convex payoffs add robustness to Kelly estimation risk → [5]
- ERC/risk-parity sits between min-variance and equal-weight; multi-asset tool → [7]
- Expected Shortfall + bootstrap for honest tail-aware risk on non-Gaussian data → [7]
- Stop-losses lower expected return under random walk; help only under momentum → [8]
- Even optimized (drawdown-distribution-fit) stops beat no-stop in barely >50% → [9]
- Drawdown control: hold risky exposure ∝ cushion (equity − floor) → [10][12][13]
- Risk-aversion ramp γ_t = γ_0·D^max/(D^max−D_t) controls drawdown convexly → [12]
- Modulator M(k)=(d_max−d(k))/(1−d(k)) guarantees max drawdown (idealized) → [13]
- Drawdown control caps drawdown but costs return (TSLA: 5% DD/1.005× vs 22.5%/1.136×) → [13]
- Drawdown control with "little/no" mean-variance cost (multi-asset HMM) → [12]
- CPPI beats VaR-based insurance under regime-switching; simple rule wins → [11]
- Gap risk: a jump > cushion between rebalances breaches the floor → [11][13]
- Vol targeting lifts Sharpe ONLY for risk assets (equity/credit), not bonds/FX/cmdty → [16]
- Vol targeting reduces tails + max drawdown + vol-of-vol UNIVERSALLY (all 60+ assets) → [16]
- Mechanism = leverage effect (negative return-vol correlation injects momentum) → [16]
- Vol scaling (1/σ) has lower turnover/cost than variance scaling (1/σ²) → [16]
- Time-series momentum: inverse-vol SIZING drives much of the performance → [14]
- TSMOM is "crisis alpha" (best in extreme markets) but needs cross-asset diversification → [14]
- Leverage-constrained investors overweight high-beta → high beta = low alpha (BAB) → [15]
- Risk of ruin: general closed form for arbitrary (fat-tailed) payoff distributions → [17]
- Risk of ruin shrinks ~exponentially with smaller per-bet fraction → [17]
