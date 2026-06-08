---
slug: onchain-vs-conclude-fork-2026-06-08
status: draft
owner: analyst
updated: 2026-06-08
tags: [strategic-fork, on-chain, conclude, ship-passive, active-vs-passive, two-domains-exhausted, ohlcv-exhausted, derivatives-positioning-closed, funding-confound, basis-equals-funding, monte-carlo, robustness, pre-registration, evidentiary-threshold, durable-over-quick, go-no-go, exchange-netflows, stablecoin-supply, prior, point-in-time, daily-resolution, passive-baseline, program-thesis]
related:
  - spec/perp-basis-mn-spread/feature.md
  - spec/perp-basis-mn-spread/reports/test-2026-06-08-perp-basis-mn-spread.md
  - spec/dev-notes/basis-reversal-vehicle-vs-signal-fork-2026-06-06.md
  - spec/dev-notes/new-data-domain-scoping-2026-06-05.md
  - spec/dev-notes/robustness-decision-rule-2026-05-30.md
  - spec/horizon-retest-robustness/presentations/horizon-retest-robustness-2026-06-05.md
  - spec/product.md
  - spec/backlog.md
---

# On-chain vs conclude — the two-domains-exhausted strategic fork

> **Mandate (analyst decision-support, FILES ONLY — orchestrator commits).** The
> `perp-basis-mn-spread` market-neutral v0.2.0 feature closed PASS (tester
> [test report](../perp-basis-mn-spread/reports/test-2026-06-08-perp-basis-mn-spread.md),
> HEAD `8c2e6c4`) with science verdict **FAMILY-UNIFORM-FRAGILE in all 3 arms** and,
> with it, the ENTIRE derivatives-positioning domain. The research program has now
> exhausted TWO full data domains (OHLCV/price + derivatives-positioning) with uniform
> negatives under the frozen block-bootstrap Monte-Carlo § 0 decision rule. The
> pre-registration said: route to on-chain IFF the MN spread is fragile. It is. But
> after TWO exhausted domains, the operator faces a genuine fork that pre-registration
> should NOT auto-resolve: **keep hunting (on-chain, the #3 domain) vs conclude the
> active-vs-passive search and ship the passive baseline.** This note is the rigorous,
> intellectually-honest decision-support for that fork — NOT an on-chain feature scope
> (that is a follow-up IFF the operator picks on-chain). Every number traces to the
> anchored MN test report, the two prior fork notes, the program retrospective, or
> source inspected this session. No manufactured optimism.

---

## 0. TL;DR — the verdict, the prior, and the recommendation

**The "passive wins" verdict is now STRONG but DOMAIN-LIMITED, and that distinction is
the whole decision.** Two domains, ~6 method families, 3 horizons, a universe axis, and
now a full long/short market-neutral vehicle — all FRAGILE under a pre-registered,
anti-cherry-pick, p5-Sharpe-<0 Monte-Carlo rule that no result has ever gamed. What this
licenses is precise and load-bearing: **"no harvestable edge exists in PRICE or
DERIVATIVES-POSITIONING data on these large-caps net of cost."** What it does NOT
license: **"no edge exists anywhere."** Both exhausted domains are *transforms of the
same two underlying quantities* — realized price and leveraged-positioning pressure
(and the MN report proved basis ≡ funding *byte-identically*, collapsing the two
"domains" closer to ONE information source than the count "2 of N" suggests). On-chain is
the first candidate that reads a *third, structurally-disjoint* quantity: settlement-layer
flows that exist whether or not anyone has traded.

**Recommendation: ON-CHAIN (keep hunting) — ONE more domain, then conclude.** The
`(Recommended)` tag goes on the durable choice, and here the durable = intellectually-
honest choice is **continue-with-a-high-prior-and-a-hard-stop**, NOT conclude-now. The
defense (§ 5): concluding after two domains that are near-degenerate in information space
(price + a positioning signal that turned out to BE its own funding mirror) would close
the active-vs-passive question on evidence that never tested a genuinely orthogonal
source — an un-durable "we'll wonder forever about on-chain" close. On-chain is the
*single* domain on the board with (a) a genuinely-different information channel, (b) a
real (not inflated) prior, and (c) a bounded, knowable cost. **But the recommendation is
explicitly the LAST hunt:** it ships with a pre-committed hard-stop — if on-chain comes
back FRAGILE under the same frozen rule, the program concludes and ships passive *with
no further domain hunt* (§ 4 names the stop precisely). This is "keep hunting" with a
fuse, not an open-ended search.

**Confidence: MEDIUM.** The honest case for concluding-now is real and is given full
weight in § 4 — two negatives is enough evidence that the base rate for "the next thing
finally works" is low, and on-chain's daily resolution + point-in-time hygiene burden are
genuine headwinds. The recommendation rests on on-chain being **categorically different**
(not just the next item in a list), on its cost being **bounded and one-time**, and on
the **fuse** that caps the downside of being wrong to one more ~5-8 dev-day spike.

**Highest-prior on-chain signal to test first (if operator picks on-chain):
EXCHANGE NET-FLOWS** (native coins moving onto / off exchange addresses) — the single
on-chain series with the clearest causal link to forward price (coins to exchanges =
intent to sell = supply pressure), the strongest orthogonality to both exhausted domains,
and free full-history daily availability. Rationale + the cheaper-fallback acquisition
lane in § 5.2. **This is a pointer, not a scope** — the full on-chain feature brief is
authored only on operator greenlight.

**If-budget-tightens annotation (the named cheaper lane):** the strictly-cheaper path
that is NOT the conclude-now path is an **on-chain research spike first** (mirror the
basis spike that preceded `perp-basis-signal-robustness`): fetch ONE series (exchange
net-flows for BTC+ETH from a free full-history source), compute its daily rank-IC /
sign-persistence vs forward return the way `basis_diag.rs` did, BEFORE building any
`ScoreSource` or fetcher-hardening. ~1-2 dev-days, kills or greenlights the domain for
almost nothing. The full harness build is gated on the spike showing a non-zero,
sign-stable IC. This preserves the on-chain hunt while spending the least to learn
whether it is alive — and it is the path I would default to if the ~5-8-day full build is
not affordable this cycle.

---

## 1. How strong is "passive wins" now — what the two-domain negative does and does NOT license

### 1.1 The evidence, stated precisely

The program has run, all under the **same frozen § 0 weakest-link Monte-Carlo composite**
(`robustness-decision-rule-2026-05-30.md`; block-bootstrap-real / shared-index; p5 Sharpe
< 0 → FRAGILE; anti-cherry-pick FP-C3.5 full-surface no-argmax-crown):

| Domain | Families / vehicles tested | Horizons | Verdict |
|---|---|---|---|
| **1. OHLCV / price** | x-sec momentum, mean-reversion, funding-carry¹, TS-momentum (4 families) | 1h / 4h / daily | ALL FAMILY-UNIFORM-FRAGILE; dominated by passive BH (+1.74/+1.10) net of fees |
| **2. Derivatives-positioning** | funding-carry, basis-reversal long-only (v0.1.0), MN basis-spread, MN funding-spread, MN basis⊥funding residual | hourly | ALL FRAGILE; basis ≡ funding byte-identical; residual NEGATIVE median Sharpe + 100% tail-DD |

¹ Funding-carry appears in both rows because it was first tested as an OHLCV-adjacent
sizing tilt and then re-tested as a derivatives-positioning MN spread. This is itself a
tell — see § 1.3.

This is a **large, anti-fragile body of negative evidence.** It is not one unlucky
backtest. The decision rule was pre-registered before any result, it crowns no winner,
it voids if the generator is wrong, and it has been applied identically across every
family. Passive buy-and-hold remains undefeated across the entire program. **As a
falsification exercise, this is exemplary** — the machine has done exactly what a
robustness program is supposed to do: refuse to certify edges that do not survive
resampled histories.

### 1.2 What the evidence DOES license

- **"On these 10 large-cap crypto perps/spot, in the 2023-2024 window, there is no
  cross-sectional or time-series edge in PRICE data or DERIVATIVES-POSITIONING data that
  survives the frozen robustness rule net of realistic cost."** This is strongly
  supported. Six families, multiple horizons, both a long-only and a full long/short
  beta-stripped vehicle, all fail. The vehicle objection (raised and resolved at the
  v0.1.0 fork) is now closed: the MN arm stripped beta and benchmarked against the
  *correct* ≈0 null, and STILL failed at 0bps gross. There is no "we used the wrong
  vehicle" escape left for these two domains.
- **"The derivatives-positioning domain carries no alpha orthogonal to funding."** The MN
  report's k2 finding is decisive and goes beyond "fragile": mn-basis and mn-funding
  produced **byte-identical surfaces** (every metric to 6 d.p., 12 anchored surfaces),
  and the basis⊥funding residual — the purpose-built test of orthogonal alpha — showed
  **negative median Sharpe** (2023 g0: −0.064; 2024 g0: −0.006) and **100% p95 tail
  drawdown** with short liquidations. The basis is not a *related* signal to funding; on
  this universe it IS funding. That is a stronger negative than "fragile" — it is "the
  information was never distinct."
- **"Passive buy-and-hold is the correct baseline to ship for these two domains."** If
  the question were narrowly "price + positioning," the answer is settled: ship passive.

### 1.3 What the evidence does NOT license (the load-bearing limit)

- **It does NOT license "no edge exists anywhere in the reachable universe."** Both
  exhausted domains are functions of a *small set of underlying quantities*: domain 1 is
  every transform of OHLCV (realized price/volume); domain 2 is leveraged-positioning
  pressure (basis/funding/the perp premium). The universe-method diagnosis already
  established the price ranking channel carries ≈0 forward information
  (`universe-method-diagnosis-2026-06-02.md` § M4). The MN report just established the
  positioning channel collapses onto funding. **So "two domains" overstates the
  information diversity tested.** In information space we have tested ~1.5 distinct
  channels, not 2 — price, and a positioning signal that turned out to be its own funding
  mirror. The conclusion "active never beats passive" would require ruling out channels
  these data *cannot express*: settlement-layer flows, forward-looking option-implied
  distributions, cross-asset/macro state, real-time sentiment. **None of those is a
  transform of price or positioning.** The two-domain negative is silent on them.
- **It does NOT license treating on-chain as "just the 3rd of N — same base rate as
  domains 1 and 2."** The base-rate-pessimism argument (§ 4) is the strongest case for
  concluding, and it is real — but it implicitly models the domains as exchangeable draws
  from one urn. They are not. On-chain is the first draw from a *different urn* (a channel
  that exists independent of the price tape). The prior for on-chain is not "two failures
  ⇒ the third fails too"; it is "two failures *in price/positioning* ⇒ the third, which is
  neither, is informed by them only weakly."
- **It does NOT make passive "proven optimal" — only "undefeated by what we have tested."**
  Passive winning is partly a *benchmark artifact*: BH on 2023-2024 BTC/large-caps caught
  a structural bull leg (+1.74 Sharpe is a very high bar that reflects the sample period,
  not a law). A different regime sample could lower the BH bar and change which side of
  the ledger an active arm lands on. This is a known scope limit of the whole program
  (the robustness axis judges resampled 2023/2024 only) and it cuts BOTH ways — it
  weakens "active is dead" AND it weakens "passive is great"; it argues for honesty about
  what "passive wins" means (it won *this sample's* horse race), not for more hunting per
  se.

**Net read:** the verdict is **strong-for-its-scope and the scope is price + positioning.**
"Passive wins" is the correct, well-evidenced answer to "is there an edge in the data we
have tested." It is NOT yet the answer to "is there an edge in the data we can reach." The
gap between those two questions is exactly the size of the remaining untested domains —
and on-chain is the largest, most-orthogonal piece of that gap.

---

## 2. On-chain — the prior, honestly (not inflated)

### 2.1 Why on-chain is structurally different (the genuine case)

On-chain metrics read the **blockchain settlement layer** — state that exists whether or
not anyone is trading on an exchange. This is the orthogonality the two exhausted domains
structurally lack:

- **Exchange net-flows** — native coins moving *onto* exchange-controlled addresses
  (intent to sell → supply pressure) vs *off* (intent to hold/custody → supply
  withdrawal). This is a *flow of the asset itself*, measured on-chain, with a direct
  causal economic story toward forward price. It is **not** derivable from OHLCV (price
  does not tell you who is moving coins where) and **not** derivable from the perp basis
  (positioning in the derivative is a different population from on-chain holders moving
  spot coins).
- **Stablecoin supply (mint/burn)** — net issuance of USDT/USDC/DAI = "dry powder"
  entering or leaving the crypto system. Rising aggregate stablecoin supply has a
  plausible lead on buying capacity. Orthogonal to both domains (it is a liability of an
  issuer, not a price or a basis).
- **Miner / validator flows** — issuance + miner/validator selling. A structural supply
  source with its own cadence (halvings, staking unlocks), unrelated to price transforms.
- **Active-address / network-growth** — adoption proxies (Metcalfe-style). Weakest causal
  link of the four (slow-moving, noisy), but genuinely exogenous to the price tape.

The orthogonality is not a *story* in the way the basis's was before its spike — it is
structural: these series are measured on a different substrate (the chain) from a
different population (on-chain holders / issuers / miners) than either price or perp
positioning. **This is the strongest orthogonality case the program has had.** The basis
"orthogonality" turned out to be moderate (+0.47/+0.66 corr with funding, and ultimately
byte-identical selections); on-chain flows have no such mechanical tether to OHLCV or
funding.

### 2.2 The realistic cost (honest — daily resolution + PIT hygiene are the headwinds)

The cost is real and I will not minimize it:

- **Resolution is DAILY, not hourly.** Free on-chain tiers (DeFiLlama, Glassnode/
  CryptoQuant free) are daily and often delayed. A 2-year daily window is **~730
  points/series** — *thin* for a robust block-bootstrap tail. The two exhausted domains
  ran on **8,760 points/yr** (hourly); on-chain has ~12× less data per series. This
  directly weakens the statistical power of the § 0 rule's tail percentiles (p5/p95 on
  ~730 points are noisier). **Mitigation:** the daily resampler + corrected annualization
  already exist (`crates/backtest/src/resample.rs`, confirmed this session — built during
  the horizon-retest), so the *plumbing* for a daily backtest is in place; the *data
  thinness* is an inherent limit, not a build gap.
- **Point-in-time hygiene is genuinely hard.** On-chain data is **revised and
  back-filled** — providers re-label addresses (an address reclassified as "exchange"
  retroactively changes historical net-flow), re-org adjustments, and methodology
  changes mean *today's* value for a past date may differ from what was knowable then.
  Getting a clean no-look-ahead series is materially harder than the funding/basis as-of
  join (which was a clean settled-at-or-before-bar-open forward-fill). This is the single
  biggest *new* engineering risk and the reason the cost is ~5-8 dev-days not ~2-3.
- **New fetcher + per-source schema.** No on-chain plumbing exists in the repo
  (confirmed: zero DeFiLlama/Glassnode/CryptoQuant references in `crates/`; the only
  fetchers are `fetch_binance_{klines,funding,premium}` + `fetch_yahoo_klines`). Each
  on-chain source has its own schema; the funding as-of-join *template* applies on the
  daily grid but the fetcher and PIT guard are new work.

**Honest cost: ~5-8 dev-days to a first robustness verdict** (fetcher + PIT-clean banked
series + daily `ScoreSource` arm + day-1 falsifiers incl. a PIT/no-look-ahead guard + 2
anchored surfaces). The if-budget-tightens spike (§ 0, § 5.3) is ~1-2 dev-days for a
research-only IC read on one series.

### 2.3 Is the prior high enough to justify the cost after two negatives?

**Honest prior of a ROBUST on-chain signal: LOW-to-MEDIUM** — and I am deliberately not
inflating it past MEDIUM. The case for it being above the floor:

- It is the **first genuinely-orthogonal channel** (§ 2.1) — the two negatives inform it
  only weakly because it is not a price/positioning transform.
- Exchange net-flows in particular have a **documented, causal, widely-cited** link to
  forward price (the "exchange reserve / netflow" literature is one of the most-referenced
  on-chain signals after realized cap). This is a real economic mechanism, not a
  data-mined artifact.

The case for keeping it at LOW-to-MEDIUM (not higher):

- **The base rate is genuinely low after two negatives** (§ 4). "The next thing finally
  works" has failed repeatedly across this program (TCN, PatchTST, GARCH-σ, LLM-forecaster,
  4 OHLCV families, 3 derivatives vehicles). Bayesian honesty: each prior negative lowers
  the prior on the next bet, even an orthogonal one.
- **Daily resolution + PIT hygiene degrade even a real edge.** A signal that is real at
  daily cadence may be un-harvestable net of (a) the thin-tail noise penalty and (b) any
  PIT contamination that survives the guard. The basis had a clean −0.10 IC at the spike
  stage and STILL came back fragile through the harness; an on-chain signal that has to
  clear the same harness on 12× less data starts further back.
- **Crypto on-chain alpha is heavily competed.** Exchange-flow signals are public and
  watched by every desk; the edge, if any, is likely small and fast-decaying — which the
  thin daily window is poorly suited to capture.

**Verdict on the prior-vs-cost question:** the prior is **high enough to justify ONE
bounded probe, but not high enough to justify an open-ended on-chain program.** That is
exactly the shape of the § 5 recommendation: one on-chain hunt (spike-first if budget is
tight), with a pre-committed hard-stop. The prior does NOT clear the bar for "keep mining
on-chain across many sub-signals if the first fails" — one orthogonal channel, one fair
test, then conclude.

---

## 3. Is on-chain the LAST reasonable domain, or are there others? (the remaining map)

On-chain is **not** the only untested domain — but it ranks **first among the remaining**.
The remaining map, scored on orthogonality (to the two exhausted domains), data-feasibility
(free + full-history + harness-shaped), and honest prior:

| Rank | Remaining domain | Orthogonality | Data feasibility | Honest prior | Why this rank |
|---|---|---|---|---|---|
| **1** | **On-chain** (exchange net-flows, stablecoin supply, miner/validator flows, active addresses) | **Highest** — different substrate, different population, not a price/positioning transform | Free, full-history, but **daily** + PIT-hygiene-hard (new fetcher) | **LOW-MED** | Most orthogonal remaining channel; real causal story (net-flows); bounded one-time cost. The natural next dollar. |
| 2 | **Options / implied-vol surface** (Deribit DVOL, skew, term structure) | High — *forward-looking* risk pricing, genuinely different information from realized price | DVOL **index** free-historical; full surface PAID/heavy. **Universe collapses to BTC+ETH** (only liquid option chains) | MED-but-narrow | Genuinely orthogonal (forward-looking), but (a) 2-symbol universe is thin for a cross-sectional rule, (b) the program already retired a GARCH-σ vol bet (inherits skepticism), (c) full surface needs an options-aware backtest the harness lacks. A DVOL *scalar regime filter* is a smaller, different experiment than a signal class. |
| 3 | **Cross-asset / macro** (DXY, US 10Y, SPX correlation, risk-on/off state) | Medium-high — exogenous to crypto entirely | Free (Yahoo/Stooq), full-history, **but** the sweep harness reads Binance-schema parquets only → needs a Yahoo-schema adapter | LOW-MED | Genuinely exogenous, but (a) crypto's macro beta is regime-dependent and unstable, (b) needs a harness adapter the sweep has never had, (c) answers "does macro state time crypto?" — a different question from "what crypto-native signal beats passive?". Defensible #3, not #1. |
| 4 | **Sentiment / social** (crypto-Twitter, Reddit, funding-as-sentiment, news flow) | Medium — reflects crowd state, partially endogenous to price | **Hard**: clean full-history social data is paid/scrape-fragile; severe survivorship + look-ahead risk (deleted tweets, revised sentiment models); no clean PIT story | LOW | Most-hyped, least-feasible-to-backtest-honestly. The look-ahead/survivorship hazards are worse than on-chain's PIT problem. The LLM-as-sentiment-analyst is already scoped as a *support* role (product.md § Pillar stack), not an alpha source — re-litigating it as alpha contradicts the ratified reframe. Low prior, high build risk. |
| 5 | **OI / long-short-ratio** (derivatives crowding) | Low — **same derivatives-positioning family that just closed** | **Paid-for-history** (Binance free retains 30 days only; needs CoinAPI/Coinglass) | LOW | In the *just-retired* domain (§ 1.2 closed it with finality), AND blocked on a paid data buy. Effectively excluded by the domain-2 closure. |
| 6 | **Cross-exchange basis / dislocation** | Low-for-slow — arb is latency/HFT territory | Free but needs new fetchers; on hourly bars the dislocation is mostly already closed | LOW | HFT territory; off the project's non-HFT scope (product.md non-goals). Little for a slow strategy to harvest. |
| 7 | **Non-crypto universe** (equities/FX trend/carry) | N/A to the crypto thesis | Free (Yahoo) but needs harness adapter | OFF-MANDATE | Answers "is the *method* portable to another asset class?", not "what signal does crypto carry beyond price?". Useful but a different program. |

**Reading the map.** Two domains genuinely contend for "next": **on-chain (#1)** and
**options/DVOL (#2)**. On-chain wins because: (i) its orthogonality is higher and cleaner
(a different substrate vs a forward-looking transform of the same underlying); (ii) it
preserves the full 10-name cross-sectional universe (the harness's native shape), where
DVOL collapses to 2 symbols; (iii) its prior rests on a *documented causal mechanism*
(net-flows → supply pressure), where DVOL inherits the retired-vol-bet skepticism. The
others are excluded by domain-2 closure (#5), HFT scope (#6), off-mandate (#7), feasibility
(#4 social), or are a narrower/different experiment (#3 macro, #2-as-scalar-filter).

**So: on-chain is the BEST next bet among the remaining, but it is NOT the last reasonable
domain** — options/DVOL and cross-asset/macro remain as genuinely-different #2 and #3.
This matters for the conclude-vs-continue call: concluding now forecloses not just
on-chain but options and macro too. The honest framing is **"3 genuinely-orthogonal
domains remain untested (on-chain, options, macro); on-chain is the strongest; the fork is
whether to test the strongest-remaining-orthogonal-channel ONCE or stop now."** It is NOT
"on-chain is the last gasp."

---

## 4. The honest meta-call — at what point does the program conclude "active ≤ passive, ship it"?

This is the section that must NOT manufacture optimism. The case for **concluding NOW** is
genuine and strong, and the operator deserves it stated at full strength.

### 4.1 The honest case FOR concluding now

1. **The base rate is brutally low.** Across this whole program, "the next predictive bet
   finally works" has failed every time: TCN (F4), PatchTST (F4), GARCH-σ (NO-ALPHA),
   LLM-forecaster (deferred LOW-MED), 4 OHLCV families (all fragile), basis long-only
   (fragile), MN basis/funding/residual (fragile, residual negative). That is ~10
   consecutive negatives. A rational prior on bet #11 being the winner is **low**, and
   each negative compounds it. "One more domain will be different" is *exactly* the
   sentence that has been wrong ~10 times.
2. **Passive is genuinely excellent on this data.** BH +1.74/+1.10 Sharpe is not a weak
   benchmark to beat — it is a strong one. An active edge has to be *very* good to justify
   the operational complexity (a live long/short book, funding accrual, liquidation risk,
   rebalancing cost) over just *holding*. The program's own evidence says: holding wins.
3. **On-chain's headwinds are real** (§ 2.2): 12× less data per series, PIT hygiene that
   is genuinely hard to get right, a heavily-competed signal space. Even a *real* on-chain
   edge may not survive the same harness on daily data.
4. **Research has a cost** — every domain spike is ~5-8 dev-days that could instead harden
   the passive baseline into a shippable product (the program's actual terminal state per
   product.md § Project scope boundary is continuous paper-trading on real data, which a
   passive baseline satisfies *today*).
5. **The methodology is the actual win, and it is already banked.** The durable
   deliverable of this program was never "find alpha" — it was "build a machine that
   refuses to certify fake alpha." That machine exists, is anchored (119 anchors), and has
   proven itself by killing ~10 plausible-looking bets. Concluding now *crystallizes* that
   win instead of risking it on bet #11.

**This case is strong enough that "conclude now" is a fully defensible operator choice. I
am not recommending against it because it is wrong — I am recommending against it because
the alternative is marginally MORE durable, by a specific argument (§ 5).**

### 4.2 The honest case AGAINST concluding now (why one more is durable)

1. **We have tested ~1.5 channels, not 2** (§ 1.3). The "10 consecutive negatives"
   framing is real but partly an *illusion of diversity*: most of those 10 were
   predictive bets on the SAME OHLCV substrate (TCN/PatchTST/GARCH/LLM all forecast price
   from price), and the derivatives bets collapsed onto funding. The number of *distinct
   information channels* ruled out is small. Concluding "active ≤ passive everywhere" on
   ~1.5 channels is a wider claim than the evidence supports.
2. **On-chain is categorically different, not "the next item."** The base-rate argument
   (4.1.1) treats domains as exchangeable. On-chain is the first non-exchangeable draw
   (§ 2.1). Its prior is not "10 failures ⇒ #11 fails" — it is "10 failures *in price/
   positioning* ⇒ #11, which is neither, inherits little." Whether the operator finds this
   decisive is the crux of the fork.
3. **The cost of being wrong is bounded and small.** If we hunt on-chain and it fails, we
   lose ~5-8 dev-days (or ~1-2 with the spike-first lane) and then conclude — having
   *also* closed the most-orthogonal remaining channel with finality, which makes the
   eventual "ship passive" conclusion *stronger* and the "but what about on-chain?" regret
   *zero*. The downside of one more hunt is one spike; the downside of premature conclusion
   is a permanent open question on the program's best-remaining orthogonal channel.
4. **The pre-registration anticipated exactly this.** Every prior fork note named on-chain
   as the pre-registered next domain *conditional on derivatives closing*. Derivatives
   closed. Honoring a pre-registration that was written before the result (and is
   therefore immune to post-hoc "we're tired, let's stop") is itself a methodology virtue.
   The bar for *overriding* a pre-registration should be high; "two negatives" was
   *anticipated* by the pre-registration, not a surprise that invalidates it.

### 4.3 Where the line actually is

**The program concludes "active ≤ passive in the reachable universe — ship passive, stop
burning research" when the genuinely-orthogonal channels are exhausted, not when the
price/positioning transforms are.** Concretely, the conclusion is fully earned after:

- ✅ Price/OHLCV (done — exhausted)
- ✅ Derivatives-positioning (done — exhausted, basis ≡ funding)
- ⬜ **On-chain** (the most-orthogonal remaining — UNTESTED)
- (options/DVOL and cross-asset/macro remain, but are narrower/adapter-blocked; on-chain
  is the cleanest test of "is there ANY orthogonal channel")

**We are ONE channel short of the honest conclusion.** On-chain is the channel whose
absence would make "active is dead everywhere" an overclaim. Test it once. If it fails
under the frozen rule, the conclusion is earned with no dangling orthogonal channel, and
the program ships passive with a clean conscience. **That is the hard-stop:**

> **Pre-committed hard-stop:** if the on-chain probe (spike or full build) comes back
> FRAGILE under the frozen § 0 rule on the dollar-neutral / BH-appropriate null, the
> program CONCLUDES. Ship passive. No options-domain hunt, no macro-domain hunt, no
> on-chain sub-signal mining. The active-vs-passive question is answered NEGATIVE for the
> reachable universe, and that answer is *durable* because the most-orthogonal channel was
> given its fair test. (The operator may always *later* re-open options/macro as a fresh
> program, but it is not a continuation of this hunt.)

This converts "keep hunting" from an open-ended money-pit into a **single bounded final
experiment with a pre-registered stop** — which is the durable shape.

---

## 5. Recommendation — ON-CHAIN (one bounded hunt, then conclude), with the durable-choice defense

**Recommendation: route to ON-CHAIN — the `(Recommended)` durable choice.** Test the
most-orthogonal remaining channel ONCE, spike-first if budget is tight, with the § 4.3
pre-committed hard-stop. The fallback (conclude now / ship passive) is named and fully
defensible, but it is the *cheaper* choice, not the *more-durable* one — and per the
operator's durable-over-quick lens the Recommended tag belongs on the durable choice.

### 5.1 Why ON-CHAIN is the durable choice (the defense)

The durable-over-quick rule says the Recommended tag goes on the choice whose decision
"carries forward without amendment" — here, the choice that answers the active-vs-passive
question *correctly and permanently*. Concluding now is cheaper (zero more dev-days) but
**leaves the program's single best-remaining orthogonal channel untested**, which means
the conclusion "active ≤ passive in the reachable universe" carries an asterisk forever:
*we never tested the one channel that isn't a price/positioning transform.* That asterisk
is the un-durable part. Routing to on-chain — once, with a hard-stop — removes the
asterisk: either on-chain is the first survivable signal (a new product), or it fails and
the conclusion becomes airtight (the most-orthogonal channel was tested and also failed →
ship passive with finality and zero regret). **Both outcomes are more durable than
concluding on ~1.5 channels.** The on-chain hunt is not "more optimism" — it is "buy the
one piece of evidence that makes the eventual conclusion (whichever way it lands)
permanent."

Crucially, the durable choice here is **bounded**, which is what separates it from
money-pit "keep hunting": the § 4.3 hard-stop caps the hunt at one channel. This is NOT a
recommendation to mine on-chain sub-signals indefinitely, nor to then proceed to options
and macro. It is "spend one more bounded spike on the highest-prior orthogonal channel,
then conclude either way." That is durable AND finite.

**If-budget-tightens (the named cheaper-than-full-build lane that is NOT conclude-now):**
run the **on-chain research spike first** (§ 5.3) — ~1-2 dev-days, one series, research-only
IC read, no fetcher-hardening, no `ScoreSource`. Gate the full ~5-8-day harness build on
the spike showing a non-zero sign-stable daily IC. If even the spike is unaffordable this
cycle, *then* conclude-now becomes the pragmatic call — but note that conclude-now and
spike-first reach the same place if the signal is dead (spike kills it for ~1-2 days and
the program concludes), while spike-first ALSO catches the upside if the signal is alive.
So spike-first dominates conclude-now on information-per-dollar unless ~1-2 dev-days is
genuinely unavailable.

### 5.2 The single highest-prior on-chain signal to test first: EXCHANGE NET-FLOWS

If the operator picks on-chain, the first signal to test is **exchange net-flows** (native
coins moving onto vs off exchange-controlled addresses), for these reasons:

- **Clearest causal link to forward price.** Coins moving *to* exchanges = intent to sell
  = imminent supply pressure; coins moving *off* = custody/hold = supply withdrawal. This
  is the most direct on-chain → price mechanism, and the most-cited on-chain signal after
  realized cap. The hypothesis is economically legible, not data-mined.
- **Strongest orthogonality to both exhausted domains.** It is a flow of the *spot asset
  itself*, measured on-chain, from a *different population* (on-chain holders) than perp
  positioning. No mechanical tether to OHLCV or funding (unlike the basis, which was
  +0.47/+0.66 funding-correlated and ultimately byte-identical).
- **Free full-history daily availability.** Aggregate exchange-flow / exchange-reserve
  series are available free at daily resolution (e.g. CryptoQuant free tier exposes
  exchange reserve/netflow for BTC+ETH; Glassnode free tier similar; the exact source +
  PIT caveats get pinned in the feature brief on greenlight — NOT here, to avoid
  fabricating an endpoint contract). For the cheaper *spike* lane, even a coarse
  BTC+ETH-only daily netflow series is enough to read the IC.
- **Fits the harness on the daily grid.** A cross-sectional rank of names by trailing
  exchange-netflow → forward return is the *same shape* as the basis-rank arm, on the
  daily resampler that already exists (`resample.rs`). The build is a daily clone of the
  basis arm with a new (PIT-guarded) sidecar.

**The cheaper-fallback acquisition note (durable-cautious):** mirror the basis spike
exactly — fetch netflow for BTC+ETH from a free source, compute daily rank-IC /
sign-persistence vs forward return via a `netflow_diag.rs` probe (clone of `basis_diag.rs`),
WITH a deliberate PIT/no-look-ahead leak-check falsifier (the on-chain revision problem
makes this falsifier MORE load-bearing than it was for basis). If the IC is ≈0 / sign-
unstable like price rank IC was, the domain dies for ~1-2 dev-days and no harness arm is
built. **This is a pointer for the follow-up brief, not a scope** — the full on-chain
feature brief (fetcher contract, exact source + endpoint, PIT guard design, θ-grid, the
2-arm-vs-3-arm question, anchor plan) is authored by the analyst ONLY on operator
greenlight of the on-chain route.

> **Honest caveat on net-flows specifically:** exchange-flow address labeling is the
> *hardest* PIT problem in on-chain data (an address reclassified as "exchange" today
> retroactively rewrites history). The spike's leak-check falsifier is therefore the gate:
> if a clean past-only netflow series cannot be constructed (the provider only serves
> *revised* values), the signal is un-backtestable honestly and the domain closes on a
> *feasibility* verdict, not a signal verdict — which is itself decision-grade and routes
> straight to the hard-stop conclusion. Stablecoin-supply (cleaner PIT: mint/burn events
> are on-chain and immutable) is the natural second-choice if net-flow PIT proves
> intractable.

### 5.3 What "ship passive" concretely means (the fallback, named precisely)

If the operator picks conclude-now (or if on-chain later fails the hard-stop), "ship
passive" means, concretely for this codebase:

1. **Adopt buy-and-hold (the existing BH control) as the production baseline strategy.**
   The BH control is already implemented and anchored throughout the sweep (it has been
   the benchmark every surface was scored against). "Shipping passive" is promoting the
   *already-built, already-validated* BH path from "control" to "the strategy the paper-
   trading agent runs." This is a *small* change, not a new build — the passive baseline
   is the most-tested code path in the repo.
2. **Update `spec/product.md` § Strategy library / success metrics** to record that the
   active-edge search concluded NEGATIVE for the reachable universe and the terminal
   strategy is passive (this note already drafts the thesis update — see § 6 and the
   product.md changelog entry landed alongside this note).
3. **Re-anchor the program's terminal state on the METHODOLOGY, not the alpha.** Per
   product.md § Differentiator (5) "measured robustness, not asserted alpha," the shippable
   win is the robustness machine + the auditable ledger + the honest negative result. The
   operator-success-reports surface (already specced) narrates "we tested N channels,
   passive won, here is the equity curve vs BH" — which is a *complete, honest product*.
4. **Keep the harness warm but idle.** The Monte-Carlo robustness harness, the anchored
   surfaces, and the fetchers stay in place so any *future* domain (options, macro, or a
   re-opened on-chain) can be tested without rebuild — but no further domain is pursued
   under THIS program.

"Ship passive" is therefore **cheap and fully-specified** — it is a promotion of existing
validated code plus a thesis-doc update, not a build. This is precisely why it is the
*cheaper* choice; the recommendation routes to on-chain not because passive is hard to
ship but because the active-vs-passive *question* is one orthogonal channel short of a
durable answer.

---

## 6. Program-thesis implication (product.md update — landed alongside this note)

The two-domain negative DOES warrant a product.md thesis update — not a reversal, but a
sharpening. The product's epistemic core (§ Pillar stack, ratified 2026-05-30) is
"measured robustness, not asserted alpha." The two-domain negative is the **strongest
possible vindication** of that core: the machine refused to certify ~10 plausible bets and
left passive undefeated. The thesis update (landed in product.md this session) records:

- The active-edge search has exhausted TWO structurally-distinct data domains
  (price/OHLCV + derivatives-positioning) with uniform FRAGILE verdicts under the frozen
  rule; passive BH remains undefeated.
- This is scoped honestly: it licenses "no harvestable edge in price/positioning data on
  these large-caps net of cost," NOT "no edge anywhere." On-chain (the most-orthogonal
  remaining channel) is the pre-registered next-and-final domain probe; a FRAGILE on-chain
  result concludes the active-vs-passive search and ships passive.
- The strategy-library roadmap gains an explicit terminal note: the realistic terminal
  strategy for this project may be **passive buy-and-hold**, and that is a *successful*
  outcome of the robustness program (the machine correctly identified that active edges do
  not survive on the reachable data), not a failure.

This does not touch the locked (2)+(4) moat, the LLM-as-support reframe, or any anchored
content. It sharpens the strategy-library section and adds the two-domains-exhausted
finding to the empirical record, consistent with the existing demotion-of-prediction-bets
table.

---

## 7. Assumptions & limits (challengeable by operator / architect)

1. **The prior on on-chain is LOW-to-MEDIUM and deliberately not inflated.** The
   recommendation does NOT rest on optimism that on-chain will be ROBUST — it rests on
   on-chain being the one *bounded* purchase that makes the eventual conclusion (either
   way) durable. If the operator weights the base-rate pessimism (§ 4.1) above the
   orthogonality-diversity argument (§ 4.2), conclude-now is a fully defensible choice and
   I would not call it wrong — only marginally less durable.
2. **"~1.5 channels not 2" is the load-bearing claim** and is challengeable. The counter
   is "two domains is two domains; stop splitting hairs." My defense: the MN report's
   byte-identical basis ≡ funding finding is *direct evidence* that the positioning domain
   collapsed toward the funding channel, and the M4 diagnosis is direct evidence the price
   ranking channel is ≈0-information. The diversity-of-channels-tested really is lower than
   the domain count suggests. But an operator who treats domain-count as the right unit
   would reasonably lean conclude-now.
3. **On-chain PIT hygiene may make net-flows un-backtestable honestly** (§ 5.2 caveat). If
   so, the spike returns a *feasibility* kill (not a signal kill), which still routes to
   the hard-stop conclusion — so the spike is decision-grade either way. Stablecoin-supply
   is the cleaner-PIT fallback signal within the on-chain domain.
4. **The hard-stop is a commitment, and its value depends on honoring it.** The
   recommendation is "on-chain ONCE, then conclude" — NOT "on-chain, then options, then
   macro." If the operator/program treats a FRAGILE on-chain result as license to keep
   hunting options/macro, the bounded-durable shape collapses into a money-pit and the
   conclude-now critics were right. The discipline is: the hard-stop is real.
5. **Passive "winning" is partly a 2023-2024-sample artifact** (§ 1.3). A different regime
   could lower the BH bar. This is a known whole-program scope limit and is NOT resolved by
   on-chain (which is judged on the same window). It is a reason to state "passive won
   *this sample's* race" honestly, not a reason to hunt more.
6. **All priors are sober post-~10-negatives.** The base rate for "this next thing works"
   is low and I have not pretended otherwise. The on-chain recommendation is justified by
   *bounded information-per-dollar toward a durable conclusion*, not by a high probability
   of finding alpha.

---

## Changelog

- 2026-06-08 (analyst, on-chain-vs-conclude fork): adjudicated the two-domains-exhausted
  strategic fork after `perp-basis-mn-spread` closed PASS / FAMILY-UNIFORM-FRAGILE in all
  3 arms (HEAD `8c2e6c4`), retiring the derivatives-positioning domain with finality
  (k2: mn-basis ≡ mn-funding byte-identical; basis⊥funding residual NEGATIVE median Sharpe
  + 100% tail-DD → no orthogonal alpha). STRENGTH-OF-VERDICT read: the two-domain negative
  is STRONG-but-DOMAIN-LIMITED — it licenses "no harvestable edge in PRICE or
  DERIVATIVES-POSITIONING data on these large-caps net of cost," NOT "no edge anywhere";
  in information space the program has tested ~1.5 distinct channels (price + a positioning
  signal that collapsed onto its own funding mirror), not 2. ON-CHAIN PRIOR: LOW-to-MEDIUM,
  deliberately not inflated — first genuinely-orthogonal channel (different substrate +
  population, not a price/positioning transform; exchange net-flows have a documented
  causal link to forward price), but headwinds are real (DAILY ~730 pts/yr = 12× less data
  than the hourly domains; PIT hygiene hard — on-chain revisions/address-relabeling
  retroactively rewrite history; heavily-competed signal). Honest cost ~5-8 dev-days
  (no on-chain plumbing exists — confirmed zero DeFiLlama/Glassnode/CryptoQuant refs in
  crates/; daily resampler `resample.rs` DOES exist). REMAINING-DOMAIN MAP: on-chain ranks
  #1 among remaining (options/DVOL #2 — orthogonal but 2-symbol + retired-vol skepticism;
  cross-asset/macro #3 — exogenous but adapter-blocked + unstable beta; social #4 —
  feasibility-blocked; OI/LSR #5 — in the just-closed domain + paid; cross-exchange #6 —
  HFT; non-crypto #7 — off-mandate). On-chain is the BEST next bet but NOT the last domain.
  META-CALL: the honest conclusion ("active ≤ passive, ship passive, stop") is earned when
  the genuinely-ORTHOGONAL channels are exhausted, not the price/positioning transforms —
  the program is ONE orthogonal channel short. RECOMMENDATION: route to ON-CHAIN (the
  durable choice; `(Recommended)` per durable-over-quick) — test the most-orthogonal
  remaining channel ONCE, spike-first if budget tight, with a PRE-COMMITTED HARD-STOP
  (FRAGILE on-chain under the frozen rule → CONCLUDE + ship passive, no further domain
  hunt). Conclude-now is the named, fully-defensible CHEAPER fallback (zero dev-days; ship
  passive = promote the already-built+anchored BH control to production + a thesis-doc
  update — a promotion, not a build). Durable defense: concluding on ~1.5 channels leaves
  the best-remaining orthogonal channel untested → permanent asterisk on "active is dead";
  one bounded on-chain hunt removes the asterisk either way (ROBUST = first product;
  FRAGILE = airtight conclusion, zero regret). Highest-prior first on-chain signal:
  EXCHANGE NET-FLOWS (clearest causal price link + strongest orthogonality + free daily
  history), with a spike-first/leak-check acquisition lane (PIT falsifier is THE gate;
  stablecoin-supply = cleaner-PIT fallback if net-flow labeling is intractable). product.md
  thesis update landed alongside (two-domains-exhausted record + passive-may-be-terminal
  note + on-chain-as-final-probe). Backlog updated. NO on-chain feature brief authored
  (deferred to operator greenlight per trace.toml ownership rule); NO `[[req]]` row; NO
  code; NO commit; NO anchored-report edits.
