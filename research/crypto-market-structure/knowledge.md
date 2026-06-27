# Knowledge — Crypto Market Structure

Synthesis of the `crypto-market-structure` ledger. Updated incrementally.
Our app: a long-only, single-coin, paper-sim crypto advisor that bakes off
strategies and ranks them under a frozen 1000-path moving-block-bootstrap
robustness gate, with buy-and-hold as the permanent benchmark. Validated thesis:
*no active strategy robustly beats holding, net of costs.* Read every takeaway
through that lens.

## Key themes (so far, [1]–[10])

1. **Funding / basis carry is the single highest-Sharpe *structural* edge in crypto.**
   Three independent venues — an academic working paper [1], a CMU/NBER paper [3],
   and BIS [4] — agree: longing spot while shorting the perp/future to harvest the
   funding/basis yields high Sharpe (BTC ~1.9 net of high fees [1]; in-sample 7–10
   [3]). BUT it is **market-neutral and needs a perp/short/margin engine** our
   long-only advisor does not have. The edge exists because retail crowds the *long*
   side (negative convenience yield [4]); it is risk-compensated, not free
   (limits-to-arbitrage, liquidation risk, segmentation).

2. **Carry/funding level is a directional *sentiment* proxy.** Very high positive
   funding/carry = aggressive leveraged-long crowd = froth. This is the part of the
   carry literature that *could* inform a long-only holder (a caution overlay),
   even though the arb trade itself is out of scope. [3][4]

3. **Crypto volatility is real, persistent, long-memory, multifractal, and
   heavy-tailed.** BTC/ETH return tails are inverse-cubic (mature-market-like) [6];
   vol clustering has power-law (long-range) autocorrelation [6]; realized vol is
   multifractal, so single-parameter "rough vol" models mis-specify it [5]. Use
   HAR-type / regime / multiscale vol models, heavy-tailed risk assumptions, and
   never trust Gaussian VaR.

4. **Crypto's option-implied risk asymmetry is the *opposite* of equities.**
   Bitcoin shows a forward/reverse vol skew (OTM calls richer) — commodity-/upside-
   demand-like, not equity crash-fear-like [8]. Directional info shows up in OTM
   option demand / net buying pressure [7].

5. **Microstructure signals exist statistically but die on costs.** Order-flow
   imbalance, spread, VWAP-deviation are the robust cross-asset LOB features [9],
   but at realistic fees high-frequency microstructure strategies have deeply
   negative net Sharpe (turnover 100–200×/day) [10]. Gradient boosting overfits;
   OLS edge is statistically insignificant [10].

## Methods / findings that hold up (and which don't)

- **Holds up:** funding/basis carry as a high-Sharpe market-neutral edge [1][3][4];
  long-memory vol clustering [5][6]; heavy (inverse-cubic) tails for large caps [6];
  order-flow-imbalance / spread / VWAP-deviation as the dominant microstructure
  features [9]; purged walk-forward validation as the honest evaluation discipline
  [10].
- **Doesn't hold up / cautions:** rough-volatility (single Hurst) models for BTC [5];
  high-frequency microstructure alpha net of costs [10]; gradient-boosting on
  microstructure features (overfits) [10]; cross-asset model transfer in crypto
  (weak — idiosyncratic per coin) [10]; treating cash-and-carry as "risk-free"
  (segmentation + liquidation risk) [4].

## Actionable takeaways for our advisor

1. **The biggest documented crypto edge (carry) is out of our long-only scope** —
   confirm we are honest that the advisor cannot capture it without a perp/short/
   funding engine. Logging this prevents a future "why don't we beat hold?" rabbit-
   hole: the structural edge is market-neutral, not directional. [1][3][4]
2. **Cost realism is decisive.** [10] is a clean external replication of our thesis:
   statistical edge → economic failure once realistic fees + turnover apply. Keep
   the cost model central; consider widening effective spread in volatile/crash
   regimes (adverse selection is real [9]).
3. **Use heavy-tailed, long-memory-aware risk assumptions** in the robustness gate
   for large caps; super-linear price impact means size penalties should be harsher
   on crypto (and much harsher on small caps) than on equities [6].
4. **Funding-rate as a regime/sentiment overlay** (not a trade): persistently high
   funding/basis = leveraged-long froth = candidate caution flag for a buy-and-hold
   horizon. Testable in our app if we ingest a funding feed. [3][4]

## Open questions / things worth testing in our app

- Does a simple **funding-rate caution overlay** (de-risk when 30-day-avg funding is
  extreme) improve risk-adjusted buy-and-hold? (needs a funding data feed)
- Does conditioning entries/exits on a **realized-vol regime** (HAR/multiscale)
  survive our robustness gate net of costs, or does it overfit like microstructure?
- Are large-cap **heavy-tailed VaR** assumptions already reflected in our bootstrap,
  or do we implicitly assume thinner tails?

## Paper map (claim → supporting [N])

- Funding/basis carry is the top structural edge (market-neutral): [1] [3] [4]
- No-arb perp pricing formula (F=S(1+r/κ)) + realistic fee tiers: [1]
- Carry driven by retail leverage demand / negative convenience yield: [3] [4]
- Carry is risk-compensated, limits-to-arbitrage severe in crypto: [4]
- Vol is long-memory / multifractal; rough-vol mis-specified: [5] [6]
- Large-cap crypto has inverse-cubic (heavy) tails, mature-market-like: [6]
- Super-linear volume→price impact (worse than equities): [6]
- Bitcoin reverse/forward vol skew (commodity-like, upside demand): [7] [8]
- Microstructure features (OFI/spread/VWAP-dev) robust but weak: [9] [10]
- HFT microstructure alpha fails net of costs (validates our thesis): [10]
