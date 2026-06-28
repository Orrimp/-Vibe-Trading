# Application — Data Integrity (which venues/metrics to trust; widen the spread)

> Decision doc for analyst + architect. Distilled from
> `research/crypto-market-structure/{knowledge.md,papers.md}` (100 entries) and
> `research/SYNTHESIS.md`. Citations are `crypto-market-structure[N]` → see
> `papers.md`.
>
> **Bottom line up front:** in crypto, **reported volume, open interest, AND
> visible order-book depth are all partly fictional** [19][20][90][91]. This is a
> first-class hazard for an honest advisor: a backtest run on faked liquidity, or
> a signal built on a fabricated metric, manufactures phantom alpha. Two durable
> consequences: **(1) source price + derivatives metrics only from reconcilable /
> regulated venues** (Kraken/HTX reconcile OI; only-Kraken/HTX clean — Bybit/OKX/
> Binance-inverse fabricate OI), and **(2) widen effective-spread / slippage
> assumptions** in the cost model so we never assume mid-price fills on overstated
> depth. This is the cheapest, highest-leverage credibility upgrade in the corpus.

---

## 1. Summary of the research

**Volume is heavily fabricated.** Wash trading is **>70% of reported volume on
unregulated CEXs** (trillions/yr), detectable because authentic trading obeys
Benford's-Law first-digit, trade-size roundness, and power-law size-tail
regularities that fakes violate; regulated exchanges match, unregulated ones don't
[19]. On DEXs it is pervasive too: **>30% of tokens** wash-traded on both IDEX and
EtherDelta, with 10% of EtherDelta tokens *almost exclusively* wash-traded [20]. A
microstructure detector shows wash trading inflates not just volume but the
**variance** of liquidity (liquidity "diffusion") — a more robust screen than raw
volume [57]. Coordinated **pump-and-dumps** target small/low-liquidity/low-price
coins (100 coins <$20M cap; one group did $82.3M in six minutes), remain pervasive
(604 target tickers across 14 exchanges), and are concentrated in a few Telegram
groups at predictable times [31][32].

**Open interest is provably fabricated on the highest-volume perp venues.** [91]
applies a hard accounting identity (|ΔOI| over any interval cannot exceed traded
volume; any excess = proof of misreporting) to seven exchanges. Findings (and they
*deteriorated* over 2023): **Bybit is egregious** — OI cannot be reconciled on
*any* sub-period (wrong every day, ~99% of hours, >70% of minutes), with implied
missing volume "$156–213B, greater than Binance's actual volume"; **OKX** also
large; **Binance inverse perps start showing misreporting**; only **HTX and Kraken
(and BitMEX to a degree) reconcile** on essentially every sub-period. The authors
demand OI be reported "with the same rigor as proof of reserves" and note
misreported OI "could easily be exploited to manipulate market participants'
perception."

**Visible depth is partly strategic/fake — even on Coinbase.** ~**31% of large
orders could spoof** Coinbase BTC/ETH books; posting *distance* matters as much as
size [90]. Spoofing/layering was rife around the LUNA crash, and statistical-
physics detection beats Z-score anomaly detection [89]. So order-book depth and
imbalance overstate true liquidity.

**Historical price itself can be manipulated.** The 2017 BTC run was materially
shaped by Tether-issuance flows: the **87 hours of largest combined BTC+Tether
flows (<1% of the sample) are associated with 50% of the rise**, causally
identified by a round-number-discontinuity instrument (RDD/IV), with a placebo
test finding no movement absent the flow events [30]. CFTC later found Tether
fully backed only 27.6% of the time in 2016–2018. Any backtest window spanning
2017 is partly fitting a manipulated regime.

**Where price discovery actually lives (which feed to trust).** CEX leads DEX (one-
way, zero reverse causality) [42][51]; **Binance/Huobi lead** cross-venue price
formation [42]; for BTC, **CME futures lead spot**; for ETH, **CEX leads** Uniswap
[69]; and **BTC leads ETH** at the microstructure level [90]. Cross-venue
deviations stay inside fee-defined no-arb bands and mean-revert fast [42] — not a
retail edge, and a reason to anchor on a deep major-venue USD/USDT feed. Cross-
country premiums (Kimchi, etc.) measure **capital controls**, not value — a
constrained-currency feed carries a premium unrelated to global price [44].

**Effective costs are higher than mid, especially in stress.** CEX round-trip cost
is low and triangular deviations <5 bps for major pairs in calm [25], but adverse
selection widens the *effective* spread when it matters: spreads widen in extreme-
sentiment regimes [50] and in toxic/informed-flow regimes [9][59]; liquidation
cascades blow execution slippage to **5–10× normal** [33][37]; and crypto liquidity
has predictable intraday/weekly periodicity (funding cadence + algos) so some
windows are reliably worse [94]. The right tail-risk machinery is GARCH+EVT+copula,
not Gaussian — naive risk models **understate** crypto tail risk [71][84].

---

## 2. Possible solutions / what can be done with this research

1. **A venue/metric trust policy, codified:** prefer regulated/reconcilable venues
   for price (deep major-venue USD/USDT, e.g. Binance/Coinbase/Kraken — Binance
   flagged relatively honest on volume [19], Huobi/HTX flagged for wash trading on
   *volume* yet *reconcilable* on OI [19][91], so the trust map differs by metric);
   **OI strictly from Kraken/HTX** [91]; never aggregated or Bybit/OKX OI.
2. **A universe screen** that excludes small/low-liquidity/DEX-only coins where
   volume and depth are largely fictional and P&D-targeted [19][20][31][32], and
   flags algorithmic-stablecoin-adjacent assets [15][43].
3. **A cost-model upgrade** that (a) never assumes mid-price fills on visible depth
   (depth is partly spoofable [90]); (b) widens effective spread in extreme-
   sentiment / high-vol / toxic-flow regimes [9][50][59]; (c) blows out slippage
   in cascade regimes (5–10× [33]); (d) optionally conditions spread on
   time-of-day / funding cadence [94].
4. **A data-quality screen** beyond raw volume: liquidity-diffusion variance signature
   [57]; Benford/roundness/size-tail tests [19]; OI-vs-volume reconciliation [91].
5. **Backtest-window hygiene**: treat the 2017 episode as a partly-manipulated
   regime [30]; be wary of edges discovered there; prefer recent windows for
   calibration (efficiency rises over time [17][54]).
6. **Feed-source guidance**: anchor on the price-discovery leader (CME for BTC, CEX
   for ETH [69]; Binance/Huobi cross-venue lead [42]); we trade CEX spot so we lag
   the futures-led "true" price slightly — acceptable, but don't expect to exploit
   the lead.

---

## 3. Relevance for the project

- **Honest backtesting depends on honest inputs.** Our thesis ("no active strategy
  robustly beats holding net of costs") is only credible if the data feeding the
  bake-off is clean. A volume-confirmation strategy (OBV is a registered arm,
  `v0.obv`) or any liquidity estimate built on faked volume produces phantom alpha
  — exactly the failure our gate exists to prevent, but the gate can't fix garbage
  inputs.
- **"Traceable & plausible" requires a stated trust map.** When the advisor reports
  a verdict, it should be able to say *which venue's price/volume it used and why*,
  and that it widened costs for known fabrication. That is the measured-honesty
  product.
- **Cost realism is the moat — and depth is fake, so mid-price fills are too
  optimistic.** [83] (our near-exact external mirror) shows a +73% paper edge → −64%
  at 10 bps; the corpus here says realistic costs are *higher* than naive spread
  because visible depth overstates liquidity [90] and stress widens it 5–10× [33].
  Under-costing would flatter strategies past the gate dishonestly.
- **Universe screening is a real risk control**, not a nicety — small caps are
  wash-traded, P&D-targeted, fictional-volume assets [19][20][31][32].

---

## 4. Advantages for the project

- **A cheap, high-leverage credibility upgrade.** Codifying a venue/metric trust map
  + widening effective spread costs almost nothing and directly hardens every
  verdict — the best honesty-per-effort change in the corpus.
- **Concrete, citable venue rankings** for the trust map: Kraken/HTX reconcile OI;
  Bybit/OKX/Binance-inverse fabricate it [91]; Binance relatively honest on volume,
  unregulated CEXs >70% fake [19]; Coinbase depth ~31% spoofable [90].
- **A robust data-quality screen** (liquidity-diffusion variance [57], OI
  reconciliation [91], Benford [19]) that catches fabrication raw-volume filters
  miss.
- **Defensible universe rule** grounded in P&D/wash-trading evidence — keeps the
  advisor on large, liquid, honest coins where the backtest is valid.
- **Backtest-window discipline** (2017 manipulation [30]) prevents fitting a
  manipulated regime and inflating an edge.

---

## 5. Problems and challenges

- **The trust map differs by metric.** A venue can be honest on one metric and
  fabricated on another (HTX: flagged for *volume* wash trading [19] yet *reconcilable*
  on *OI* [91]). The policy must be metric-specific, not a single venue allow-list.
- **Fabrication is moving and deteriorating.** [91] shows OI misreporting *worsened*
  over 2023 and spread to Binance inverse perps — so a one-time trust map goes
  stale; reconciliation should be periodic, not assumed.
- **Depth/imbalance signals are structurally compromised.** ~31% spoofability [90]
  means any order-book-imbalance feature is partly reading fake liquidity — caution
  for any microstructure overlay (also out of our daily horizon anyway [9][79]).
- **We cannot trade the price-discovery leader.** CME leads BTC spot [69] but we
  trade CEX spot; we inherit a slight lag and must not assume we can exploit the
  lead [42].
- **Over-widening costs can be its own bias.** Too-pessimistic spreads would fail
  legitimate strategies; the widening must be calibrated/regime-conditioned, not a
  blanket penalty — and it touches the cost model feeding the FROZEN gate (must stay
  additive, bands untouched).
- **Stablecoin denominator is not always $1.** USDT/USDC peg stability is
  time-varying [81], held by a thin/centralized arbitrage layer (USDT ~6 redeemers/
  month [82][100]), with a $0.99 break-the-buck threshold [62] — so a USDT-denominated
  price can itself wobble in stress; the cost/risk model should not assume a rock-solid
  denominator.
- **HARD CONSTRAINTS:** USDT-denominated, `Decimal` not `f64`; if a data-quality or
  peg-stress series is ingested as an exogenous input it goes through
  `core::pit::PitSeries` (ADR-0058) + SHA-pinned corpus + `REVISION.toml` + a
  divergence e2e; `ui` must NOT depend on `strategy`/`exec`/`llm`; anchored SHAs
  byte-immutable (119/119); gate + bands FROZEN; paper-only.

---

## 6. Concrete next steps / candidate work items

| # | Item | Verdict | Where | Priority |
|---|------|---------|-------|----------|
| A | **Widen effective-spread / slippage in the cost model**: no mid-price fills (depth partly fake [90]); widen in extreme-sentiment/high-vol/toxic-flow [9][50][59]; 5–10× in cascade regimes [33]; optional time-of-day/funding-cadence [94]. Additive; bands FROZEN. | **WORTH DOING** (cheapest, highest-leverage honesty upgrade; may flip marginal picks to FRAGILE) | `crates/backtest/src/bakeoff/` cost/slippage model | **P1** |
| B | **Codify a metric-specific venue trust map** (price: deep major-venue USD/USDT; OI: Kraken/HTX only; never Bybit/OKX/aggregated OI; volume: distrust unregulated). | **WORTH DOING** (cheap, directly supports "traceable" claims) | `spec/` dev-note + data-source config in `crates/data/` | **P1** |
| C | **Universe screen** excluding small/low-liquidity/DEX-only coins (wash-traded, P&D-targeted [19][20][31][32]); flag algo-stablecoin-adjacent [15][43]. | **WORTH DOING** (real risk control) | advisor coin-selection / `crates/data/` | **P1** |
| D | **Backtest-window hygiene note**: flag 2017 as partly-manipulated [30]; prefer recent windows for calibration [17][54]. | **WORTH DOING (codify)** | `spec/` dev-note + bake-off window docs | **P2** |
| E | **Data-quality screen beyond volume**: liquidity-diffusion variance [57] + OI-vs-volume reconciliation [91] + Benford/roundness [19], run periodically (fabrication deteriorates [91]). | **WORTH DOING** | `crates/data/` validation pass | **P2** |
| F | **Stablecoin-peg stress monitor** (flag USDT/USDC < ~$0.99 [62], or rising pair-stablecoin vol/redemption friction [81][82][100]) as an exogenous tail-risk circuit-breaker — through `PitSeries` + SHA-pinned corpus + divergence e2e. | **PROBE-WORTHY** (tail-risk context, not a return signal; honest prior = rarely binds) | exogenous seam (mirror `dvol_data.rs`) | **P3** |
| G | **Order-book-imbalance / depth-based overlays** | **DEAD END / DO-NOT-BUILD** (depth ~31% spoofable [90]; sub-second edge dies on costs [9][79]; out of daily horizon) | — | n/a |

---

## 7. Open questions for analyst & architect

1. **Does the cost-model widening (A) belong in this round, and does it stay
   additive to the FROZEN gate?** It is the highest-leverage honesty change and may
   flip marginal strategies to FRAGILE, but it feeds the gate — confirm bands
   untouched. *(Recommended: yes — it is the durable correctness fix; under-costing
   on fake depth is a silent dishonesty.)*
2. **What is the canonical price feed / venue per coin**, and is it the same one the
   live paper-trade uses? The trust map must match the data the bake-off *and* the
   forward plan consume (the F5 forward-fidelity precedent: don't let the two
   diverge).
3. **How often is the venue trust map / OI reconciliation refreshed**, given
   fabrication deteriorates over time [91]?
4. **Where does the universe screen live** so it gates both bake-off and advice, and
   what are the concrete liquidity/market-cap/venue thresholds [19][31]?
5. **Is a stablecoin-peg stress monitor (F) worth the exogenous-arm build** for a
   tail event that rarely binds, or is it documentation-only context?
6. **Does widening cost in stress regimes interact with the vol-regime overlays**
   (vol-and-overlays doc §6)? They touch the same cost model — sequence to avoid
   double-counting.

---

## 8. What NOT to do / out of scope

- **Do not assume mid-price fills** — visible depth is partly spoofable [90].
- **Do not trust reported volume / OI / depth at face value** — all partly
  fabricated [19][20][90][91]; OI especially on Bybit/OKX/Binance-inverse.
- **Do not build volume-confirmation or OI/depth-based signals on unreconciled
  venues** — phantom alpha on faked liquidity [19][20][91].
- **Do not advise on small/low-liquidity/DEX-only coins** — wash-traded,
  P&D-targeted, fictional volume [19][20][31][32].
- **Do not treat large cross-venue/cross-country price gaps as alpha** — fee-bounded
  / capital-control premiums, not mispricings [42][44].
- **Do not trust edges discovered on the 2017 window** without acknowledging it was
  manipulated [30].
- **Do not assume the USDT/USDC denominator is always $1** — peg stability is
  time-varying and held by a thin arbitrage layer [62][81][82][100].
