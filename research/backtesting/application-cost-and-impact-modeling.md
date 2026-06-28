# Application — Cost & Impact Modeling (realistic friction, capacity, live-vs-backtest decay)

_Decision doc for analyst + architect. Source: `research/backtesting/knowledge.md`
(synthesis) + `research/backtesting/papers.md` (100-entry ledger; cited
`backtesting[N]`) + `research/SYNTHESIS.md` (cross-topic roadmap). This is the
**realistic-cost / market-impact / decay** half of the backtesting research; the
multiple-testing / selection-bias half is in
`application-overfitting-and-multiple-testing.md`._

> **One-line thesis of this doc:** for a **€200-budget single-coin spot advisor**,
> market *impact* is genuinely ≈ 0 — three independent papers (equities, Bitcoin,
> theory) confirm a fixed **fee + spread** is the correct small-order limit. The
> durable value here is therefore NOT an impact model; it is **(a) cost realism
> that bites on TURNOVER so churny crowns can't fake a win, (b) state-aware
> (vol-scaled) spreads, and (c) framing live-vs-backtest DECAY as the expected
> case.** All of it structurally *favours holding* — i.e. it hardens the thesis.

---

## 1. Summary of the research

- **Market impact follows a universal square-root law,** `Impact ≈ Y·σ·√(Q/V)`
  (Q = order size, V = daily volume, σ = vol), with the participation exponent ≈0.5
  (sublinear). Confirmed across stocks, asset classes, eras `backtesting[13]`, and
  — decisively for us — **on Bitcoin specifically** via ~1M metaorders
  `backtesting[95]`. A fresh theory derives it from price diffusivity alone
  `backtesting[41]`.
- **Impact is dynamic and DECAYING, not a static per-trade charge.** Almgren–Chriss
  permanent+temporary `backtesting[27]`; Obizhaeva–Wang LOB resilience
  `backtesting[55]`; the Bouchaud propagator (decaying kernel reconciling persistent
  order flow with diffusive prices) `backtesting[56]`. Charging a *permanent* impact
  per trade over-states cost; charging *zero* understates it; the honest at-size
  middle is a transient kernel.
- **All of the above vanish at retail scale.** For €200 on a liquid coin the
  participation rate Q/V ≈ 0, so impact ≈ 0 — `backtesting[13][41][55][95]` agree a
  fixed fee + spread is defensible. Impact only bites at size, and a **√→LINEAR
  crossover for *informed* trades** `backtesting[41]` means the more "alpha" a
  strategy claims, the *worse* its execution-cost scaling — a phantom-alpha guard.
- **Costs are STRONGLY turnover-dependent, and paper returns overstate realized.**
  AQR on ~$1T of live executions `backtesting[82]`: short-term reversal
  (high-turnover) is crushed by costs even at modest scale; low-turnover styles
  survive. Patton–Weller `backtesting[83]`: after implementation costs, funds earn
  low returns to value and ~zero to momentum — "what you see is not what you get."
- **Transaction costs alone can offset ALL apparent technical-rule profit.**
  Bajgrowicz–Scaillet on the DJIA `backtesting[24]`: modest costs zero out the
  in-sample edge of TA rules entirely — the single most on-thesis empirical result.
- **Crypto-specific cost realism:** spreads on liquid majors are tiny in calm
  markets (sub-bp to a few bp) but **blow out 2–3× in volatility/stress**;
  a credible crypto backtest needs taker fees + spread + liquidity/vol-scaled
  slippage + delay + conservative fills `backtesting[47]`.
- **The simulation ENGINE itself moves results — entirely via the cost model.** At
  zero cost five engines agree exactly; with costs, divergence scales with turnover
  (up to 3.71% for high-turnover rotation; cost-intensity↔spread correlation 0.93)
  `backtesting[22]`. Intra-candle fill ordering (stop vs target in the same bar) is
  ambiguous on OHLC data and must be decided conservatively `backtesting[23]`.
- **Live-vs-backtest decay is quantified and predictable.** McLean–Pontiff
  `backtesting[57]`: ~26% OOS drop (data-mining upper bound) + further ~32%
  post-publication (arbitrage) — *though the SYNTHESIS deep-read corrected the split
  to ~10% statistical + ~35% crowding; see §5*. Falck–Rej–Thesmar `backtesting[58]`:
  **overfitting proxies (signal complexity, outlier-sensitivity) predict decay
  better than arbitrage** — most decay is "never as real as the backtest said."

---

## 2. Possible solutions / what this research makes available

1. **A defensible fixed fee + spread retail cost floor** — justified explicitly
   (not by hand-wave) by the √-law's near-independence from schedule at Q/V≈0, on
   Bitcoin specifically. `backtesting[13][41][95]`.
2. **State-aware (volatility-scaled) spread** — widen 2–3× in high-vol regimes;
   disproportionately penalizes the over-trading crowns whose edge is most fragile
   exactly when costs spike. `backtesting[47][22]`.
3. **Turnover as a first-class ranking output** — surface each crown's turnover
   next to its net edge; give high-turnover crowns extra skepticism (the cost
   killer). `backtesting[82][83]`.
4. **A fee-sensitivity sweep** — re-rank across a small grid of cost assumptions;
   a crown whose "beats hold" verdict flips on plausible fees is not robust
   (the spec-curve discipline applied to costs). `backtesting[24][47]`.
5. **Conservative intra-candle fill semantics** — pin a deterministic worst-case
   order (e.g. stop before target in the same bar) and a one-bar execution delay;
   "model-candle" unit tests. `backtesting[23]`.
6. **A zero-cost engine cross-check** — a second reference path must match to the
   penny at zero cost; with costs, agree within a documented tolerance.
   `backtesting[22]`.
7. **A transient-impact kernel** — *only* if a future "scale to $X / capacity"
   feature enters scope: charge a decaying kernel, not a static or permanent
   per-trade impact. `backtesting[55][56]`.
8. **Decay-aware ranking + expectations** — a complexity penalty and an
   outlier-sensitivity (drop-top-k-days) metric to down-rank fragile winners;
   frame the forward run to *expect* underperformance vs the in-sample crown.
   `backtesting[57][58]`.

---

## 3. Relevance for the project

- **We are squarely in the regime where the simple model is correct.** €200 spot on
  a liquid coin ⇒ impact ≈ 0 ⇒ fixed taker-fee + spread is not a shortcut, it is the
  *right* model — and `backtesting[95]` lets us say so *on BTC specifically*, not by
  analogy. This is a credibility win: we can defend the cost model with an on-asset
  citation in the "traceable & plausible" report.
- **Cost realism is the deciding filter for the thesis.** Because costs bite on
  turnover and buy-and-hold is the lowest-turnover strategy possible, realistic
  costs *structurally favour holding* — `backtesting[24][82][83]` are independent
  confirmations of "no active strategy robustly beats holding **net**." The cost
  model is not a detail; it is *why* the thesis holds.
- **Turnover reporting is the single most useful actionable add here.** It makes the
  cost story visible and auditable: the operator sees that the "edge" of a churny
  RSI/Bollinger crown is an artifact of trading frequency, not skill.
- **Decay framing protects the product's honesty.** The forward paper-trade *should*
  underperform the in-sample crown — `backtesting[57][58]` make that the expected
  case, not a bug. Saying so up front is the honest, credibility-building move.
- **Expected-null, again.** None of this produces alpha. It *removes phantom alpha*
  and *explains* the null — which is exactly the product's job.

---

## 4. Advantages for the project

- **Defensible, on-asset cost assumption** — `backtesting[95]` justifies fee+spread
  for BTC at retail scale; we cite rather than assert.
- **Phantom-alpha removal** — a non-trivial cost floor + turnover sensitivity stop
  churny crowns faking a win at unrealistic friction `backtesting[24][22]`.
- **Engine-quirk robustness** — a zero-cost cross-check + conservative intra-candle
  semantics make net-return magnitudes (which feed the buy-and-hold comparison)
  reproducible and defensible `backtesting[22][23]`.
- **Honest expectations** — decay framing pre-empts the "but it worked in the
  backtest!" objection and turns it into a *feature* of the methodology
  `backtesting[57][58][83]`.
- **Future-proofing** — the transient-kernel result is the correct thing to reach
  for *if* capacity ever enters scope, so we won't reach for a wrong (static or
  permanent) impact model later `backtesting[55][56]`.
- **Cheap.** Turnover, fee-sweep, drop-top-k-days, and a complexity count are all
  computable from artifacts we already store.

---

## 5. Problems and challenges (incl. which HARD CONSTRAINTS they bump)

- **The calibration tightrope (both directions).** Too-cheap costs ⇒ phantom alpha;
  **punitively-high costs ⇒ a rigged, too-easy "hold wins" that is also
  dishonest.** AQR `backtesting[82]` warns real costs are an order of magnitude
  *smaller* than naive quote-based estimates — so the honest target is a
  *calibrated* cost, defended with sources, neither too cheap nor punitive. This is
  the central risk of any cost change.
- **State-aware spread vs the FROZEN gate / determinism.** A vol-scaled spread
  changes net returns, hence the gate's inputs. This is a **cost-model change, not a
  gate-band change** — the frozen bands in `crates/backtest/src/bakeoff/robustness.rs`
  stay untouched — but it *does* shift every verdict, so it needs the same
  pre-commitment discipline as the gate and a clear changelog/ADR. Architect must
  confirm a cost-spec change is in-bounds vs "frozen gate."
- **The day-1 baseline-divergence e2e constraint (hard rule).** Any cost change is a
  *sizing/overlay-adjacent* modifier to realized equity. Per the
  v3-vol-overlay-noop precedent, a cost-spec change that is *computed but never
  applied* is a real failure mode. **A vol-scaled spread or turnover penalty MUST
  ship with a day-1 e2e asserting the cost-adjusted equity diverges from the
  flat-cost baseline by ≥ a testable epsilon when turnover is non-trivial.**
- **Decimal-vs-f64 constraint.** Costs are *money* ⇒ **`Decimal`, never f64.** Fees,
  spreads, and slippage must be applied in the Decimal accounting path, not the f64
  stats path. This is the most error-prone part of any cost change and the place to
  be strictest.
- **Anchor-safety constraint.** Changing the cost model changes net returns ⇒ every
  anchored backtest report's numbers change ⇒ body-SHAs break. This is the **largest
  blast-radius risk in this whole doc.** A cost-model change must follow the
  ADR-0038 anchor-additive / re-emission protocol and run `verify_anchors.sh` before
  and after; expect to re-anchor reports deliberately, not accidentally.
- **⚠ Decay-number correction (carry into any copy).** The round-3 "26%/58%" framing
  was corrected by the SYNTHESIS deep-read to **~10% statistical (statistically
  *insignificant*) + ~35% post-publication crowding**, and the decay is *largest for
  the cheapest-to-arbitrage, low-idiosyncratic-risk names = BTC/ETH/SOL* — our exact
  coins. We crown the *max of many configs*, so the ~10% is a **floor, not a
  forecast.** Do not quote "26%" as gospel. `backtesting[57]`, SYNTHESIS §1.
- **Impact model is out of scope at our scale — resist over-engineering.** Building
  a √-law or transient-kernel impact model for a €200 advisor would be wasted effort
  and false precision; the citations exist to *justify ignoring* impact, not to
  build it `backtesting[13][41][55][95]`. Only revisit if capacity enters scope.
- **ui-layering / paper-only constraints** are unaffected — cost logic lives in the
  backtest/exec accounting path, the UI renders results.

---

## 6. Concrete next steps / candidate work items

Ordered by value-per-effort. Note the anchor/Decimal/e2e blast radius is higher
here than in the multiple-testing doc — these touch realized equity.

| # | Candidate | Where | Priority | Notes / constraints |
|---|---|---|---|---|
| A | **Turnover as a first-class KPI** next to net edge | `bakeoff/` KPIs + report | **P0** | Pure reporting; no equity change ⇒ no anchor break. Highest credibility-per-effort. `backtesting[82][83]` |
| B | **Fee-sensitivity sweep** (re-rank across a small cost grid; flag verdict flips) | `bakeoff/rank.rs` + report | **P1** | Spec-curve for costs. If it re-runs the gate it touches numbers ⇒ anchor care. `backtesting[24][47]` |
| C | **Outlier-sensitivity (drop-top-k-days) + complexity penalty** | `bakeoff/rank.rs` | **P1** | Decay predictors as down-rank signals; report-side. `backtesting[58]` |
| D | **Document + test the intra-candle fill semantics** (worst-case order, 1-bar delay) | engine + tests | **P1** | "Model-candle" unit tests; conservative + deterministic. `backtesting[23]` |
| E | **State-aware (vol-scaled) spread** (2–3× in high-vol) | cost spec (Decimal path) | **P2** | Changes equity ⇒ **needs day-1 divergence e2e + ADR + re-anchor**. Calibrate, don't punish. `backtesting[47][22]` |
| F | **Zero-cost engine cross-check** (penny-match at 0 cost; tolerance with costs) | test harness | **P2** | Validates net-return magnitudes that feed the B&H comparison. `backtesting[22]` |
| G | **Explicit "well-known signal" decay note** in the report | report copy | **P2** | Frame forward underperformance as expected (corrected magnitude, §5). `backtesting[57][58]` |
| H | **Transient-impact kernel** | new module | **P3 / deferred** | ONLY if capacity/"scale to $X" enters scope. Out of scope today. `backtesting[55][56]` |

**Already correct — do NOT change:**

- **The fixed fee + spread retail model is the right one** for €200 spot — confirmed
  on BTC `backtesting[95]`. Do not add an impact model at current scale.

**Highest-value single item:** **A — turnover as a first-class KPI.** It is pure
reporting (zero anchor/Decimal/e2e risk), it directly surfaces the cost mechanism
that decides most bake-off verdicts, and it makes the thesis ("costs favour
holding") *visible and auditable* — the maximum credibility-per-line-of-code here.
Defer the equity-touching cost changes (E/F) until the analyst rules on the
calibration target and the architect sizes the re-anchor cost.

---

## 7. Open questions for analyst & architect

1. **What is the calibrated cost target, and who owns the number?** Taker fee +
   spread + (small) slippage for which venue / tier? The honest-vs-rigged line is a
   *product* decision (`backtesting[82]` says real costs are smaller than naive
   estimates). **Analyst call**, documented with sources.
2. **Is a vol-scaled spread (item E) worth the re-anchor blast radius now**, or do
   we ship turnover reporting + fee-sweep first and treat the constant calibrated
   spread as "good enough" given impact≈0? (Recommendation: defer E.) `backtesting[47]`
3. **Does a cost-spec change count as in-bounds vs "the FROZEN gate"?** The bands
   don't change, but every verdict's inputs do. **Architect call** + ADR.
4. **Re-anchor plan.** A cost change re-emits ~all backtest report SHAs. Deliberate
   ADR-0038 re-emission vs scope-limit the change to *new* report fields only?
   **Architect + `verify_anchors.sh`.**
5. **Forward-run framing.** Do we add explicit copy that the forward paper-trade is
   expected to underperform the in-sample crown (corrected decay magnitude), so the
   operator reads it as methodology, not failure? **Analyst/UX call.** `backtesting[57]`
6. **Fee-sensitivity operating point.** What cost-grid width counts as "plausible,"
   and is a verdict that flips within it auto-downgraded or just flagged?
   `backtesting[24]`

---

## 8. What NOT to do / out of scope

- **Do NOT build a market-impact model** (√-law, Almgren–Chriss, propagator,
  transient kernel) for a €200 retail advisor. Impact ≈ 0; the citations exist to
  *justify the simple model*, not to build a complex one. Revisit only if capacity
  enters scope. `backtesting[13][41][55][95]`
- **Do NOT manufacture a too-easy "hold wins" with punitive costs.** Calibrate to
  reality (smaller than naive estimates) `backtesting[82]`.
- **Do NOT change the cost model without** (a) a day-1 baseline-divergence e2e,
  (b) Decimal-path application (never f64 money), and (c) a deliberate anchor
  re-emission plan. All three are hard constraints.
- **Do NOT quote the "26%/58%" decay split as fact** — use the corrected ~10%
  statistical + ~35% crowding, and treat it as a floor since we crown the max of
  many configs. §5.
- **Do NOT touch the frozen gate bands or the buy-and-hold-exempt rule** — cost
  changes are upstream of the gate, not changes to it.
