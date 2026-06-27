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
  combination strategies underperform simple holding in ~72/103 cases [2]; on crypto
  the leverage effect that would drive a Sharpe gain is absent/reversed [93][75].
- Full Kelly as a deployable strategy (too aggressive; ruin risk) [3][6]; and Kelly's
  asymptotic optimality [100] is unreachable in a few-year, few-trade single-coin window [55].
- The lever-up half of vol-targeting for a long-only, no-leverage retail account —
  Moreira–Muir's own gains need 400–864% leverage at the 99th pct [2].
- Stop-losses as a return enhancer — value-destroying under a random walk, and
  even optimized stops help in barely >50% of assets [8][9].
- Floor/drawdown GUARANTEES on a gapping asset — broken by jumps between
  rebalances [11][13][46]; Bitcoin's daily 99% ES ≈ −22%, realized MDD 76% [83][89].
- Optimized / ML / RL sizing and elaborate vol models — beaten by simple rules OOS
  ([18][32][50][51]); high backtested Sharpes (e.g. 2.4–5.7 in [91][99][94]) are the
  multiple-testing illusion the Deflated Sharpe [69] + PBO [70] exist to deflate.
- Implied volatility as a primary sizing input for crypto — distorted by thin options
  liquidity [67]; useful only combined, at weekly+ horizons [95].

**New (Round 3) — holds up:**
- Selection-bias correction (Deflated Sharpe [69], PBO/CSCV [70]) as a rigorous overfit gate.
- HAR-RV [66] / EWMA [85] as cheap, robust vol forecasts; GARCH-EVT for fat-tail risk [88].
- Coherent tail measures (ES/CVaR/CDaR/EVaR/spectral) over VaR [82][34][41][64][98].
- Drawdown modulation + restart, cost-aware, crypto-tested [13][96].
- "Sizing carries celebrated strategies": strip vol-scaling from TSMOM ⇒ ≈ buy-and-hold [86][87].

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
6. **Add the selection-bias correction to the gate (Round 3 priority).** Compute the
   Deflated Sharpe Ratio [69] of the crowned bake-off pick (deflate by effective #trials +
   skew/kurtosis; require DSR ≥ 0.95) and estimate the Probability of Backtest Overfitting
   via CSCV [70]. This is the Sharpe-side complement to our moving-block bootstrap and the
   exact upgrade the research program flagged. Our many-strategy×parameter bake-off is the
   textbook False Strategy Theorem setup — the crown is inflated without this.
7. **Do NOT expect a Sharpe gain from the vol overlay on crypto — the leverage effect is
   absent/reversed.** [93][75] show crypto vol rises after UP moves (or shows no asymmetry),
   so the [16] Sharpe mechanism doesn't operate. Ship the overlay as a drawdown/tail tool,
   measure the coin's return-vol correlation per-window, and frame honestly.
8. **Use a parameter-light vol estimator.** EWMA λ≈0.94 [85] or a HAR-style multi-horizon
   realized-vol blend [66]; don't over-engineer σ̂ (better point-vol ≠ better tail [51]);
   implied vol only as a *combined* weekly-horizon signal [95], never the primary input.
9. **Prototype drawdown control as modulation + restart.** The [13] cushion multiplier
   M(k)=(d_max−d(k))/(1−d(k)) PLUS a high-water-mark restart [96] (shown to improve perf
   net of costs on crypto) — de-risk toward the operator's floor but allow recovery. Offer
   static (CPPI-like) vs ratcheting (TIPP-like [72]) floor; disclose the floor is probabilistic.
10. **Make rebalancing cost-survivable.** Implement any vol-target/drawdown overlay with a
    no-trade band [61] (width ∝ cost & vol) or a conditional/state-gated trigger [28], not
    continuous re-sizing — the turnover bleed is what flips vol scaling net-negative [48][28].
11. **Trigger de-risk on DOWNSIDE volatility, not total vol** [59][35] (Sortino-style
    semi-deviation), a cleaner crash signal; and report Sortino/Calmar + CVaR/ES + skew +
    median terminal wealth, not just Sharpe [54][55][84] — these surface the overlay's real
    (risk-shaping) benefit and crypto's asymmetry.
12. **If a stop is added, attach it only to a TREND pick, tune it to volatility (ATR-based),
    pair it with a profit-take, and bootstrap-test significance** [8][9][44][57][94] — never
    a fixed-% stop on buy-and-hold; expect drawdown reduction at a return cost.

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
- Wire the Deflated Sharpe Ratio [69] + PBO/CSCV [70] into the bake-off: what is the DSR /
  PBO of a typical crowned single-coin pick? Hypothesis: many crowns fail DSR ≥ 0.95 (i.e.
  the raw Sharpe edge over buy-and-hold doesn't survive deflation). This is the highest-value
  next experiment.
- Does triggering the vol overlay on DOWNSIDE deviation [59] (Sortino-style) cut drawdown
  more than total-vol scaling on the same (coin,window), net of costs? (Direct test.)
- Per-coin, is the realized leverage effect positive (equity-like) or inverse [93]? Does a
  positive-leverage-effect coin/window actually get a Sharpe bump from the overlay while an
  inverse one doesn't? (Confirm the [93] mechanism on our data before promising anything.)
- Drawdown modulation + restart [13][96] vs a static CPPI-style floor vs buy-and-hold on real
  coin windows with our cost model: does the restart actually improve net-of-cost terminal
  wealth, and how often does the floor get breached by gaps (gap-risk frequency)?
- Does a no-trade band [61] (width ∝ cost & vol) make our vol overlay cost-survivable where
  continuous re-sizing is net-negative [48][28]? What band width is the break-even on a
  high-cost crypto coin?

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
- Optimal vs naive (1/N): no optimized rule beats 1/N OOS; needs ~3000–6000 mo of data → [18]
- Vol-managed (put-writing) Kelly×VIX hybrid = dynamic fractional Kelly; best drawdown control → [22]
- ERC exists & unique, sits between min-var and 1/N, uses covariance only (no μ) → [23]
- Max drawdown is a model-free, path-aware risk measure (permutation changes it) → [24]
- Risk-averse optimal-f: shrink the growth fraction by the CURRENT drawdown → [25]
- Low-volatility anomaly: low-vol/low-beta had HIGHER return + smaller drawdown (1968–2008) → [26]
- Low-vol anomaly extends to crypto (cross-sectional) → [27]
- CONDITIONAL vol targeting (only de-risk in extreme-vol states) = low-turnover, robust → [28]
- More signals/horizons often add redundancy not diversification; simpler is better → [29]
- Risk-constrained Kelly (convex drawdown-prob bound) DOMINATES plain fractional Kelly → [30]
- Drawdown-constrained growth optimum = a transform of the unconstrained Kelly portfolio → [31]
- HRP beats min-variance OOS by avoiding covariance inversion ("Markowitz's curse") → [32][33]
- CVaR is coherent & LP-optimizable; VaR is non-convex & non-subadditive → [34]
- Statistical-jump-model regime de-risk HALVED equity drawdowns net of costs → [35]
- Deep-hedging minimizes CVaR with frictions (tail-risk objective is active research) → [36]
- Leverage cycle / margin spirals → fundamental-free crashes; unlevered holder survives → [37][56]
- Meta-labeling: decouple SIDE (signal) from SIZE (confidence); triple-barrier labels → [38]
- E[max drawdown] grows log T / √T / linear-T for positive / zero / negative drift → [39]
- Disposition effect present in Bitcoin → advisor value is partly debiasing → [40]
- CDaR: coherent drawdown risk measure, mean of worst (1−β) drawdowns, LP-form → [41]
- Distributionally-robust Kelly: size for the worst-case distribution in an ambiguity set → [42]
- Knightian uncertainty + fat tails push the optimal Kelly fraction DOWN further → [43]
- A trailing stop ALONE is suboptimal; pair it with a profit-take → [44]
- Conservative sizing recipe: plug a LOWER-QUANTILE edge / UPPER-QUANTILE vol into the rule → [45]
- Jump-aware CPPI keeps some exposure after a breach to dodge the "miss the rebound" trap → [46]
- Multivariate optimal-f (Vince TWR) has a unique well-posed optimum → [47]
- Vol scaling can LOWER net Sharpe (0.39 vs 0.59) once turnover costs count → [48]
- Crypto vol-targeting works as a small cash-diluted sleeve (EWMA/GARCH σ̂) → [49]
- Don't build RL sizing: ~2M steps ≈ 8000 yrs of daily data even on clean sim → [50]
- Better point-vol forecast ≠ better TAIL (VaR/ES) forecast → measure tail directly → [51]
- Crypto HRP is OOS-robust, esp. on tail-risk-adjusted return → [52]
- Tail hedging: rolling puts BLEED (−0.61%/yr); trend is the better-value hedge → [53]
- Sharpe rises ~linearly with NEGATIVE skew (most premia = insurance-selling); trend is +skew → [54]
- Kelly: mean wealth ≫ median (Triple-Kelly 940× mean / 0.017× median); report medians → [55]
- Stop-loss = trend rule; interior optimal threshold; tune to vol, evaluate at modest freq → [57]
- Skewness term enters optimal allocation once returns are non-Gaussian → [58]
- DOWNSIDE-vol scaling beats TOTAL-vol scaling (cleaner de-risk trigger) → [59]
- LPPL/crash-prediction de-risk signals are tempting but overfit-prone → [60]
- No-trade band (width ∝ cost & vol) cuts rebalancing turnover ~50% → [61]
- Block-bootstrap test: CPPI/synthetic-put beat naive stop-loss; fancy variants add nothing → [62]
- Mutual non-dominance: B&H doesn't stochastically dominate insurance, nor vice versa → [63]
- EVaR ≥ CVaR ≥ VaR: most conservative coherent tail measure, KL-ball dual (robustness) → [64]
- Liquidity/funding shocks forecast crypto vol (de-risk signal beyond realized vol) → [65]

### Round 3 additions — vol-forecasting for sizing
- HAR-RV (daily+weekly+monthly realized vol, OLS) is the parameter-light vol-forecast workhorse → [66]
- EWMA (λ=0.94 daily) is the cheap one-parameter conditional-vol recipe (RiskMetrics) → [85]
- GARCH/EGARCH beat HIST/EMA AND implied vol for crypto vol; crypto shows NO asymmetry (EGARCH) → [75]
- Stochastic-vol + Student-t forecasts crypto vol best (density, fat tails) but heavier to fit → [68]
- Crypto implied vol is distorted by thin options liquidity (unreliable in the tails) → [67]
- Implied vol beats models at 7–15 day horizons, loses at 1 day; COMBINE, don't replace → [95]
- GARCH-EVT/POT-GPD is the rigorous fat-tail-AND-vol-clustering risk estimator → [88]
- Bitcoin tail is heavy & TIME-VARYING: daily 99% VaR ≈ −13%, ES ≈ −22% → [89]
- Shrink the inputs: Ledoit–Wolf for covariance; single-asset analogue = shrink μ̂ → 0 → [77]

### Round 3 additions — Kelly / sizing
- Breiman (1961) rigorously proved Kelly's two optimality results — but they're ASYMPTOTIC & known-edge → [100]
- Thorp: f* = μ/σ² for stocks; "bet LESS than the formula" (fractional, edge-first) → [80]
- Crypto's INVERSE leverage effect (up-moves drive vol) ⇒ vol-targeting Sharpe gain absent/reversed → [93]
- Bitcoin leverage effect is present-but-regime-dependent (tension [78] vs [75]) → [78]

### Round 3 additions — drawdown control
- Drawdown-modulation + RESTART (re-base high-water mark) improves perf NET of costs, on crypto → [96]
- TIPP (ratcheting floor) = best downside protection but sacrifices upside capture vs CPPI → [72]

### Round 3 additions — tail risk / measures
- Coherent-measure axioms (Artzner): subadditivity fails for VaR → use ES → [82]
- Spectral risk measures = risk-aversion-weighted blend of ES (encode operator preference) → [98]
- Sortino ratio (downside deviation / MAR) rewards loss-asymmetry; better than Sharpe for crypto → [84]
- Crypto tail risk is non-stationary & enormous (76.4% Terra/FTX drawdown); calibrate adaptively → [83]
- Cash/stablecoin = volatility DAMPENER not hedge; crypto crashes together (BTC–ETH ρ>0.85) → [79]

### Round 3 additions — selection-bias correction (THE gate upgrade)
- Deflated Sharpe Ratio: deflate crowned Sharpe by #trials + skew/kurtosis; require DSR ≥ 0.95 → [69]
- Probability of Backtest Overfitting (CSCV): estimate P(IS-best underperforms OOS-median) → [70]
- NCO/MCOS: two instability sources (noise + signal-magnified inversion); Monte-Carlo the optimizer → [71]
- Markowitz (1952): the in-sample-optimal frontier whose OOS fragility motivates our skepticism → [92]

### Round 3 additions — sizing > signal evidence
- Strip vol-scaling from TSMOM ⇒ performance ≈ buy-and-hold (sizing carries the load) → [86]
- Crypto vol-scaled TSMOM beats B&H on risk-adjusted return + downside risk (but it's the SIZING) → [87]
- Vol management mitigates crypto-momentum CRASHES (tail benefit, not edge) → [97]
- Feedback-control vol targeting (single-asset) > open-loop forecast-and-divide (turnover/leverage) → [90]
- Variance risk premium predicts equity returns; crypto VRP is 7× larger but non-standard → [74][76]
- Risk-aware RL reward (penalize downside) avoids "reward hacking" → score drawdown, not just Sharpe → [81]
- ATR-scaled asymmetric exits (1×ATR stop / 2×ATR target) — sensible, but overfit-prone in sweeps → [94]
- Crypto trend-following with vol-regime-calibrated trailing stops (distrust the headline Sharpe) → [91][99]
- Crypto Monte-Carlo / simulation-based tail metrics mirror our bootstrap gate → [79]
- Regime-/tail-dependent CVaR strategies: no universal winner; drawdown reduction in stress is robust → [73]

## Round 3 synthesis — what changed and what's portable

**The single most important new finding for our overlay: crypto's leverage effect runs the
WRONG way.** [16] established that volatility targeting lifts Sharpe ONLY through the
leverage effect (negative return→volatility correlation, which injects implicit
crash-avoidance). [93] (Brini–Lenz, high-frequency panel) finds crypto has an INVERSE
leverage effect — *positive* returns drive volatility higher — and [75] (EGARCH) finds no
asymmetry at all, while [78] finds a present-but-regime-dependent one. The honest synthesis:
the equity mechanism that makes vol-targeting Sharpe-accretive is **absent, reversed, or
unstable in crypto**. Therefore our vol overlay should be shipped as a **drawdown/tail/
vol-of-vol reduction tool** (the benefit [16] found is universal regardless of the leverage
effect) and we should **not promise a Sharpe gain** — if anything, de-risking after the
up-moves that precede crypto vol spikes could slightly *cost* return. This is the strongest
single-coin-crypto-specific confirmation of our whole "risk-shaping, not edge" thesis.

**The gate upgrade is now fully specified.** [69] Deflated Sharpe Ratio + [70] PBO (CSCV) are
exactly the selection-bias correction the research program flagged. Our bake-off crowns the
best-Sharpe strategy across many strategy×parameter trials on one coin+window — the textbook
False Strategy Theorem setup. The concrete plan: (a) compute SR₀ (expected max Sharpe under
the multiple-testing null) from the *effective* number of independent strategy trials and
require the crowned pick's **Deflated Sharpe ≥ 0.95**, with the skew/kurtosis terms naturally
penalizing crypto's fat-tailed negative-skew strategies; (b) run **CSCV** across the
strategy×time return matrix to estimate the **probability the crown is overfit** and surface
it (or gate on it) — a real OOS check that needs no separate hold-out (precious given short
crypto histories). Together with our moving-block bootstrap (path robustness) these attack
selection bias + path fragility simultaneously.

**Vol-input choice is settled toward simple.** HAR-RV [66] (daily+weekly+monthly realized vol,
OLS) and EWMA/RiskMetrics λ=0.94 [85] are the parameter-light workhorses; GARCH/EGARCH [75]
are fine if we want conditional vol; stochastic-vol/DL [68][51] forecast points slightly
better but **don't reliably improve the TAIL numbers that govern de-risking** [51], and
implied vol is **unreliable for crypto** (thin options) except as a *combined* signal at
weekly+ horizons [95][67]. On daily bars we approximate realized variance with squared
returns or a Garman–Klass/Parkinson range. Shrink the inputs [77]: with no robust edge, μ̂→0,
so sizing defaults to vol-only.

**Drawdown control has a crypto-tested, cost-aware recipe.** The drawdown-modulation
controller [13] guarantees (idealized) a max-drawdown floor but locks out at the floor; the
**restart mechanism** [96] re-bases the high-water mark so the position can recover, and is
shown to improve performance **net of transaction costs on cryptocurrency** — the single most
deployable drawdown-overlay upgrade found. Offer the operator a static floor (CPPI-like, more
upside) vs a ratcheting floor (TIPP-like [72], protects profits, costs upside), disclose the
trade, and treat the floor as **probabilistic, not guaranteed** (gap risk [46][89] is real:
Bitcoin's daily 99% ES ≈ −22% and it has drawn down 76% [83]).

**Sizing > signal got its cleanest proof.** [86] (Kim–Tse–Wald): strip the vol-scaling out of
time-series momentum and its performance collapses to ≈ buy-and-hold — the apparent "edge" is
the *sizing*, not the signal. [87] confirms the same for crypto TSMOM. But the honest
flip-side for us: on a SINGLE coin there's no cross-asset diversification (the thing that made
TSMOM's vol-scaling actually profitable across 58 instruments), so even the sizing benefit is
mostly **risk-shaping, not return** — exactly our thesis.
