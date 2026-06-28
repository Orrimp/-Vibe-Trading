# Application — Exogenous-Signal Arms (candidate testable arms vs documented dead ends)

> Decision doc for analyst + architect. Distilled from
> `research/crypto-market-structure/{knowledge.md,papers.md}` (100 entries) and
> `research/SYNTHESIS.md`. Citations are `crypto-market-structure[N]` → see
> `papers.md`. **This is not a literature dump** — it answers one question per
> candidate signal: *is it worth a pre-registered probe arm through our FROZEN
> robustness gate, or is it a documented dead end?*
>
> **Bottom line up front:** after 100 papers and a full-text deep-read pass on
> the most promising candidates, **no exogenous signal in the corpus clearly
> survives realistic costs at our daily horizon as return-timing alpha.** We have
> already burned **DVOL** (`v0.dvol_regime`) and **macro risk-on/off**
> (`v0.macro_riskon`, ADR-0073) — both came back FRAGILE → BenchmarkWins, the
> pre-registered valid null. The honest deliverable here is *coverage*: which
> arms are dead ends (don't spend the data-feed cost), which are expected-null
> (probe only if cheap, to extend honest coverage), and the *one* that is least-
> weak and reuses a seam we already own. The asset class is ours; the value we
> sell is **measured honesty + traceable, plausible trading.**

---

## 1. Summary of the research

The corpus repeatedly tests "exogenous signal X predicts crypto returns" and
repeatedly finds the same shape: a statistically detectable in-sample effect that
**(a) forecasts volatility, not direction; (b) is endogenous to price; or (c) is
real but too small / too short-horizon to survive round-trip costs at a retail
daily horizon.** The deep-read pass *demoted* the candidates that looked best in
abstract.

**Funding-rate / basis carry.** The single highest-Sharpe *structural* edge in
crypto — long spot / short perp to harvest funding (BTC net Sharpe ~1.9, ETH
~2.8; in-sample 7–10) — but it is **market-neutral and out of our long-only,
single-coin scope** [1][3][4][41][60]. It exists because retail crowds the long
side (negative convenience yield [4]) and is *risk-compensated*, not free:
segmented collateral (CME−IBIT wedge ~2.58%/yr [60]), liquidation/ADL risk
[37], and limits to arbitrage [4]. Its full-sample Sharpe has **fallen post-2024
and turned negative in 2025** [67] — even the structural edge is decaying. The
only long-only-accessible use is reading the funding **sign/level as a froth
gauge** (crowded longs), but the one direct test [66] finds **bidirectional**
Granger causality (funding↔price, p≈1e-15 and 5e-8) — funding partly *reflects*
price, an endogeneity trap — and reports **no net-of-cost timing strategy**.

**USDT exchange-inflow flows (the demoted ex-favorite).** Billed as our best
exogenous arm; the full text of [68] guts it for a daily long-only spot advisor.
The effect is **tiny and intraday (1–2h): $100M USDT inflow → +0.065% BTC /
+0.11% ETH next hour** — inside any realistic round-trip cost. The daily/weekly
horizon we trade is relegated to an **appendix**, and the only "economic"
validation is a **cost-free ETH options** trade, never spot net of fees. It is
genuinely exogenous (unlike price-derived Fear & Greed), but on its own numbers
it would almost certainly FAIL our daily gate. The related Tether→BTC result [30]
is **RDD-identified causal evidence of 2017 manipulation** (87 hours = 50% of the
rally), *not* a forward-tradeable signal — and a data-integrity caution about any
backtest window spanning 2017. The whale-inflow vol-spike paper [36], read in
full, also collapses: its "high-Sharpe" variants are in-market only 2–17% of the
time (high Sharpe on near-zero exposure), it has **no transaction costs anywhere**,
and where the strategies actually make money, drawdown is *worse* than buy-and-hold.

**On-chain valuation / cycle-timing (MVRV-Z, NUPL, CVDD, NVT, CVALUE).** The
boldest direct claim [40]: NUPL / MVRV-Z / CVDD threshold strategies beat
buy-and-hold *and* random entry over three cycles, MVRV-Z strongest, CVDD bottoms
at ~99% confidence. The deeper read confirms the **fatal gaps our gate exists to
catch: NO out-of-sample, NO walk-forward, NO transaction costs, only 3 cycles**
[40]. NVT is the on-chain "P/E" [70] and CVALUE = price-to-new-address is the
cross-sectional value factor [67] — both theory-motivated ratios to *test*, not
trust. Scope caveat: **~75% of BTC activity is off-chain**, so on-chain metrics
see a quarter of real flow, and **on-chain *demand* (not supply) carries the price
information** [78]. "Fundamental floors" are a trap: **miner/production cost is NOT
a floor — price drives cost** [48], so Puell/hash-ribbon signals are circular.

**Sentiment (Fear & Greed, social media).** Documented dead ends. The Crypto Fear
& Greed Index does **not** Granger-cause returns and gives no OOS gain — returns
cause sentiment [52]. Social-media sentiment (Twitter/Reddit) likewise shows **no
Granger causality to returns** [74]. A causal-Bayesian-network study finds
**technical indicators carry the causal signal while added external/social
features sometimes *hurt*** [95]. Extreme sentiment is a *liquidity-cost* regime
(spreads widen; intensity, not direction) with no net-of-cost edge [50].

**Macro risk-on/off (Fed / CPI / recession).** Macro event-risk repricing
forecasts crypto **volatility** [72]; hawkish policy and inflation surprises are
BTC **headwinds** [93][96]; BTC is **risk-on, not a hedge** [24][93][96], net
*receiver* of cross-asset (often FX/macro) shocks [97]. The strongest (monetary-
policy) channel **fails OOS outside the 2024–25 rate-cut window** [72] — a textbook
overfit. So macro is a *vol-sizing* input with known regime instability, not a
timing signal — and it is the one we already probed (`v0.macro_riskon`) → FRAGILE.

**Cross-venue / cross-chain / cross-country arbitrage.** Uniformly dead for
retail: deviations stay inside fee-defined no-arb bands and mean-revert fast [42];
triangular arb is net-unprofitable for the average sophisticated trader [26];
funding/spread arbs are 40%-profitable with 95% forced exits [51]; MEV / sandwich
/ cross-chain value accrues to searchers/builders [21][22][85][86]; cross-country
premiums measure capital controls [44]. ML *return* prediction collapses on costs;
even cost-aware doesn't beat hold by bootstrap [83].

---

## 2. Possible solutions / what can be done with this research

1. **Reuse the existing exogenous-arm seam** (already built and battle-tested):
   `crates/backtest/src/dvol_data.rs` and `macro_regime.rs` show the canonical
   pattern — a SHA-pinned gitignored parquet corpus + tracked `REVISION.toml`, a
   pure look-ahead-free as-of join through `trading_core::pit::PitSeries`
   (ADR-0058), `Decimal` at the seam, a `--features` gate, and a day-1
   baseline-equity-divergence e2e (`dvol_regime_divergence_end_to_end.rs`,
   `macro_regime_overlay_end_to_end.rs`). Any new arm is *additive* and slots
   into this seam — no new infrastructure required.

2. **Run any genuinely-new candidate as a pre-registered probe arm** through the
   FROZEN 1000-path moving-block bootstrap gate vs buy-and-hold, with the null
   (FRAGILE → BenchmarkWins) pre-registered. This is what we did for DVOL and
   macro; it is honest *coverage*, not a search for a win.

3. **Source derivatives metrics only from reconcilable venues.** If a froth arm
   (funding/OI) is ever built, funding from a leading CEX and **OI strictly from
   Kraken/HTX** — never Bybit/OKX/Binance-inverse, whose OI is *provably
   fabricated* [91].

4. **Document the dead ends as dead ends** in the spec so they are not
   re-proposed. The corpus is strong enough to justify *not* spending paid-feed
   budget on Fear & Greed, social sentiment, miner-cost floors, calendar/halving
   rules, or any cross-venue arb.

5. **Mine the carry/sentiment literature for a future structural arm, not a
   signal.** Funding/basis carry is the one documented high-Sharpe edge; it needs
   a perp + margin + short engine (ADR-0051) we don't have. The `basis_data.rs` /
   `funding_data.rs` / `basis_reversal_score` plumbing already exists as a seam —
   but as a *market-neutral* future direction, not a long-only return arm.

---

## 3. Relevance for the project

Our advisor's journey is: pick one coin + budget → bake off **every** strategy →
rank under the FROZEN gate (buy-and-hold always the benchmark + exempt) → forward
rule-based plan → watch it paper-trade. An exogenous-signal arm is just another
candidate in the bake-off — it must clear the same gate as an SMA cross.

- **The seam is the deliverable, not a win.** We have already proven the seam
  works end-to-end twice (DVOL, macro). The value of adding more arms is **honest
  coverage of our own asset class**: when the advisor says "no active strategy
  robustly beats holding," it is far more credible if we can show we *probed the
  exogenous signals the literature hyped and they failed our gate too.* That is
  exactly "traceable and plausible trading."
- **Expected-null is the honest prior.** The corpus + the two FRAGILE results say
  the default outcome of any new arm is BenchmarkWins. A new arm earns its place
  by (a) being genuinely exogenous (not price-derived), (b) reusing the seam
  cheaply, and (c) extending coverage of a signal a skeptical operator would ask
  about — *not* by being expected to win.
- **One arm is genuinely worth a probe; the rest are dead ends or already-burned.**
  See §6 — the **funding-sign froth arm** is the least-weak untested candidate and
  reuses the existing `basis_data`/`funding_data` seam, but expect FRAGILE.

---

## 4. Advantages for the project

- **This is OUR asset class, covered honestly.** 100 papers specific to crypto
  market structure means the advisor's "we tested this" claims are grounded in the
  exact venues, frictions, and signals a crypto retail user would raise.
- **A reusable, look-ahead-proof exogenous-arm seam** (`PitSeries` + SHA-pinned
  corpus + divergence e2e) that makes adding/retiring an arm cheap and auditable.
  The hard part (no-look-ahead as-of join, money-math discipline, feed integrity)
  is already solved and tested.
- **Pre-registered nulls turn "failures" into product.** Each FRAGILE arm is a
  defensible line in the advisor's honesty story rather than a dead end nobody can
  see. The DVOL and macro arms already demonstrate this.
- **Data-integrity discipline baked in.** Knowing OI is fabricated on Bybit/OKX
  [91] and that 2017 was manipulated [30] keeps us from building an arm on
  partly-invented numbers or fitting a manipulated regime.
- **Cost realism as a moat.** The corpus (esp. [83], a near-exact external mirror
  of our gate) repeatedly shows the edge dies on costs — our cost-aware gate is
  the competitive advantage, and exogenous-arm probes harden it.

---

## 5. Problems and challenges

- **PIT-infeasibility for on-chain valuation.** MVRV-Z / NUPL / realized-cap
  require a full on-chain feed (Glassnode/CryptoQuant — paid) *and* a clean
  as-of/point-in-time history. Realized-cap metrics are themselves revised and
  back-computed; constructing a look-ahead-free `PitSeries` for them is harder than
  for a daily DVOL or macro close, and a naive join would leak [40][78].
- **Paid feeds for the genuinely-exogenous signals.** USDT-inflow needs a paid
  CryptoQuant-style flow feed with clean exchange attribution [68][36]; on-chain
  valuation needs Glassnode [40]; Kalshi macro needs a Kalshi feed [72]. Spending
  this budget on a signal whose honest prior is FRAGILE is hard to justify.
- **Fabricated / manipulated data.** Open interest is provably fabricated on
  Bybit/OKX/Binance-inverse — only Kraken/HTX reconcile [91]; volume is >70% fake
  on unregulated CEXs [19]; visible depth is partly spoofable even on Coinbase
  [90]; the 2017 run was manipulated [30]. Any arm touching OI/volume/depth must
  restrict venues and widen effective-spread assumptions.
- **The no-op-stub trap (the DVOL lesson, codified).** Per the
  `v3-volatility-forecaster-noop-fix` precedent and CLAUDE.md non-negotiable,
  computing a `scale`/signal that is never *applied* passes unit tests and anchored
  reports while doing nothing. **Every arm ships with a day-1
  baseline-equity-divergence e2e** proving the arm's output equity diverges from
  the un-fed baseline by ≥ a testable epsilon when the signal is non-trivial.
  Pattern: `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`;
  precedent for arms: `dvol_regime_divergence_end_to_end.rs`,
  `macro_regime_overlay_end_to_end.rs`.
- **Endogeneity masquerading as signal.** Fear & Greed [52], social sentiment
  [74], and even funding [66] partly *reflect* price — a backtest that "works" may
  be re-trading lagged price. Demand a Granger / causality + OOS + net-of-cost test
  before trusting any "sentiment" feature.
- **Regime instability.** The strongest macro channel fails OOS outside the
  rate-cut window [72][93]; carry decays post-2024 [67]; efficiency rises with
  liquidity for large caps [17]. An arm fit on one regime may not generalize — the
  forward paper-trade and a fresh window are the discipline.
- **HARD CONSTRAINTS (must be named in any arm's design):**
  - **USDT-denominated; `Decimal` not `f64`** end-to-end (signals are dimensionless
    and never enter P&L — the `dvol_data.rs`/`basis_data.rs` pattern).
  - **Look-ahead-free as-of join** via `core::pit::PitSeries` (ADR-0058).
  - **SHA-pinned corpus**: gitignored parquets + tracked `REVISION.toml`, loader
    refuses to run on unverified data (`EXPECTED_*_REVISION_SHA`).
  - **Day-1 baseline-equity-divergence e2e** proving the arm FEEDS data.
  - **`ui` must NOT depend on `strategy`/`exec`/`llm`.**
  - **Anchored report SHAs byte-immutable (119/119); gate + bands FROZEN;
    paper-only.**

---

## 6. Concrete next steps / candidate work items

Ranked by honest promise. P0/P1/P2 = priority; each flagged **PROBE-WORTHY** or
**DEAD END / DO-NOT-BUILD** or **ALREADY BURNED**.

| # | Item | Verdict | Where | Priority |
|---|------|---------|-------|----------|
| A | **Funding-sign froth arm** (`v0.funding_froth`): de-risk / long-flat when 30-day-avg funding is extreme positive (crowded longs). Reuse the existing `basis_data.rs`/`funding_data.rs` seam + `basis_reversal_score`; funding from a reconcilable CEX. | **PROBE-WORTHY** (least-weak untested arm; *expect FRAGILE* — [66] finds funding↔price bidirectional + no net-of-cost edge). | `crates/backtest/src/funding_data.rs` (seam exists; `basis_divergence_e2e.rs` precedent) + new arm id in the registry | **P1** |
| B | **Document the dead ends in spec** so they are never re-proposed: Fear & Greed [52], social sentiment [74][95], miner-cost floor [48], halving/calendar [46][94], all cross-venue/cross-chain arb [26][42][51][86], DeFi LP yield [35][53][63]. | **DEAD END (codify)** | `spec/` dev-note / backlog "do-not-build" list | **P0** (cheap, prevents wasted feed budget) |
| C | **USDT exchange-inflow daily-horizon confirmatory probe** (`v0.usdt_inflow`): ONE honest daily probe to confirm the decay [68] predicts, then retire as a return arm; keep only as a vol-dampening hint. | **PROBE-WORTHY only as confirmatory null** (full text predicts FAIL at our horizon; ~0.065–0.11% per $100M, intraday, daily = appendix; needs paid feed). | exogenous seam (mirror `dvol_data.rs`) — **only if a flow feed is already on hand** | **P2** |
| D | **On-chain valuation overlay** (`v0.mvrv_threshold`): MVRV-Z / NUPL long-flat threshold through the gate — the [40] claim to *falsify*. | **PROBE-WORTHY but gated on PIT-feasibility** (high overfit risk: no OOS/walk-forward/costs, 3 cycles; paid Glassnode; PIT-hard). Do NOT build until a look-ahead-free realized-cap `PitSeries` is shown feasible. | exogenous seam + a new on-chain `PitSeries` loader | **P2** |
| E | **OI / leverage froth arm** | **DEAD END / DO-NOT-BUILD as specced** (OI provably fabricated on Bybit/OKX/Binance-inverse [91]; only Kraken/HTX reconcile). If ever attempted, Kraken/HTX OI ONLY — or drop OI and keep only the funding half (item A). | — | **P2 (blocked)** |
| F | **Sentiment / Fear & Greed contrarian arm** | **DEAD END / DO-NOT-BUILD** (endogenous, no Granger causality, no OOS [52][74][95]). | — | n/a |
| G | **Cost-model hardening from this corpus** (not an arm): widen effective spread in extreme-sentiment / high-vol / cascade regimes [9][33][50]; super-linear size penalty on crypto [6]; optionally time-of-day/funding-cadence spread [94]. | **WORTH DOING** (sharpens the gate; may flip marginal strategies to FRAGILE). | `crates/backtest/src/bakeoff/` cost model | **P1** |

---

## 7. Open questions for analyst & architect

1. **Is the funding-froth arm (A) worth the build given the expected-FRAGILE
   prior?** It reuses an existing seam and is the only untested arm with a
   theory-grounded, long-only-accessible read (crowded-longs froth). The value is
   coverage, not a win. Build it, or document funding-as-froth as a dead end on
   the strength of [66]'s bidirectionality? *(Recommended: build it — it is the
   durable, defensible coverage choice and the seam is cheap.)*
2. **What is the policy for paid feeds whose honest prior is FRAGILE?** Items C
   (CryptoQuant) and D (Glassnode) need budget for signals we expect to fail. Do
   we acquire a feed only opportunistically (already on hand), or never?
3. **PIT-feasibility of on-chain valuation.** Can realized-cap / MVRV be assembled
   into a genuine look-ahead-free `PitSeries` given the metrics are revised /
   back-computed? If not, item D is permanently blocked and should be a documented
   dead end, not a P2.
4. **Where does the "do-not-build" list live** so it is authoritative and survives
   spec compaction — backlog, a dev-note, or `spec/architecture.md`?
5. **Future structural arm (out of current scope):** is funding/basis carry
   (perp + margin + short, ADR-0051) on the long-term roadmap at all, given it is
   the one documented high-Sharpe edge but is decaying post-2024 [67] and is
   market-neutral (not single-coin)?
6. **Does cost-model hardening (G) belong in this round?** It is the highest-
   leverage non-arm change from this corpus and may flip marginal bake-off picks —
   but it touches the cost model feeding the FROZEN gate (additive, bands
   untouched). Confirm it stays additive.

---

## 8. What NOT to do / out of scope

- **Do not build a Fear & Greed or social-sentiment entry/exit rule** — endogenous
  to price, no Granger causality, no OOS edge [52][74][95].
- **Do not build a miner-cost / production-cost "floor" or buy-the-dip trigger** —
  price drives cost; it is circular [48]. Same for Puell/hash-ribbon.
- **Do not hard-code a halving or calendar/seasonality rule** — halving is weak,
  delayed, n≈2–3 [46]; intraday/weekly periodicity is arbitraged away [94].
- **Do not build any cross-venue / cross-chain / cross-country arbitrage arm** —
  fee-bounded, specialist-captured, retail-inaccessible [26][42][44][51][85][86].
- **Do not build an OI froth arm on aggregated or Bybit/OKX data** — provably
  fabricated [91]. Kraken/HTX only, or drop OI.
- **Do not model a DeFi LP-yield "strategy"** as a hold-beater — it loses to
  holding [35][53][63].
- **Do not treat USDT-inflow or any exogenous arm as standalone return-timing
  alpha** — at best a vol-dampening / risk-sizing hint, and the corpus says it
  fails our daily gate [68][36].
- **Do not promise carry/arbitrage edges** in the advisor — they are market-neutral
  and/or out of scope; the realistic single-coin baseline is buy-and-hold.
