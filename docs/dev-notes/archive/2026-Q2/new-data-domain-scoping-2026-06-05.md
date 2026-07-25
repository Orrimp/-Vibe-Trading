---
slug: new-data-domain-scoping-2026-06-05
status: draft
owner: analyst
updated: 2026-06-05
tags: [new-data-domain, scoping, post-program, ohlcv-exhausted, perp-spot-basis, on-chain, microstructure, open-interest, options, dvol, non-crypto, data-feasibility, harness-reuse, robustness, go-no-go, basis-spike, basis-ic, orthogonality, premium-index, LIVE]
related:
  - spec/horizon-retest-robustness/presentations/horizon-retest-robustness-2026-06-05.md
  - docs/dev-notes/universe-method-diagnosis-2026-06-02.md
  - docs/dev-notes/robustness-decision-rule-2026-05-30.md
  - spec/carry-strategy/feature.md
  - spec/carry-funding-data-backfill/feature.md
  - spec/product.md
---

# New data-domain scoping — where alpha could come from once OHLCV is exhausted

> **Mandate (scoping + landscape, NO build).** The active-trading robustness
> program is exhaustively closed: four method families (x-sec momentum, MR, carry,
> TS-momentum) × three horizons (1h/4h/daily) × a 35-name universe spike all came
> back FAMILY-UNIFORM-FRAGILE — dominated by passive buy-and-hold net of fees on
> the OHLCV-only 10-symbol Binance universe (see the
> [program retrospective](../../../../spec/archive/presentations-2026-Q2.tar.gz)).
> The operator chose the **new-data-domain** fork. This note delivers a
> decision-grade landscape of candidate *structurally-different* signal sources,
> a recommended first domain to test, and a feature stub for go/no-go. Every
> feasibility claim below is grounded in the repo's existing fetchers/plumbing
> (inspected this session) or a real, named, cited endpoint — **no fabrication**
> (per the fabricated-"Sharpe 1.40" precedent). The `[[req]]` trace row is
> deliberately deferred until the operator greenlights.

---

## 0. TL;DR — the recommendation (with confidence)

**Test the PERP–SPOT BASIS (the perpetual mark-vs-spot premium) FIRST.** It is the
single highest expected-value domain on the board because it is the **only**
candidate where alpha-potential is genuinely *structurally-different from OHLCV*
**and** the data is **free, full-history, point-in-time, and reachable with the
fetcher the repo already owns** — the funding fetcher already calls the exact
Binance endpoint that returns `markPrice`; it just throws the field away today
(`fetch_binance_funding.rs:111`: "*markPrice is present but we do not store it*").
One additive column on a proven fetcher → a full 2023-2024 banked series → the
carry integration template (`funding_data.rs`) slots it into the existing
block-bootstrap sweep + frozen decision rule + BH control with **zero new
plumbing risk**.

| | Verdict |
|---|---|
| **Recommended first domain** | **Perp–spot basis** (perpetual premium-index vs spot, per-symbol time-series) |
| **Data source** | Binance `GET /fapi/v1/premiumIndex` (`markPrice`, `indexPrice`) — already hit by `crates/data/src/funding.rs`; full history via the funding-fetcher pattern |
| **Free / paid** | **FREE**, no auth, no history cap (unlike the 30-day-capped microstructure endpoints — see § 2) |
| **Harness fit** | **HIGH** — direct reuse of the carry `funding_data.rs` as-of loader + a new `ScoreSource` arm; no new backtest needed |
| **Cost to first verdict** | **~2-3 dev-days** + ~1 min compute (carry build is the line-for-line template) |
| **Honest prior of a ROBUST signal** | **LOW-to-MEDIUM** — the basis is funding's twin, and carry already failed; literature warns single-asset funding predictive power decays. Value is in **cheaply** testing the most-related structurally-new series before paying for harder domains. |

**Confidence in the *recommendation* (not in the signal): HIGH.** This is the
correct *first* probe regardless of outcome: it is the cheapest structurally-new
series to bank, and a FRAGILE result here is itself decision-grade — it
strengthens the "derivatives-positioning signals on these large-caps are dead"
read and routes the next dollar to a genuinely orthogonal domain (on-chain) with
eyes open, rather than paying CoinAPI/Glassnode money up front on a hunch.

**If-budget-tightens annotation:** if even ~2-3 dev-days is too much right now,
the strictly-cheaper fallback is a **pure-research basis spike** (§ 6, Option B):
fetch the premium series, compute the rank-IC / sign-persistence of basis →
forward-return the SAME way `universe_diag.rs` did for the ranking channel (~0.5-1
day, **no strategy code, no new ScoreSource**). If basis IC is ≈ 0 like price
rank IC was, you have killed the domain for almost nothing and never built the
sweep arm. This is the durable-cautious path and is named explicitly so the
operator has a clean cheaper lane.

**One-line operator framing:** *"Before we pay for exotic data, spend ~2 days
turning on a field our own fetcher already downloads and throws away — the gap
between the perp and spot price — and run it through the exact machine that ruled
out everything else. It is the cheapest honest shot at a signal the price bars
cannot express, and even a 'no' tells us to go on-chain next instead of guessing."*

---

## 1. The frame — what "structurally different" has to mean

The program's entire negative lived **inside OHLCV bars**: every axis ruled out
(method, universe, horizon) was a transform of open/high/low/close/volume. The
[universe-method diagnosis](universe-method-diagnosis-2026-06-02.md) § M4 showed
the decisive failure is information-theoretic — the cross-sectional **ranking
channel carries ≈ 0 forward information** (rank IC within ±0.07 of zero, no stable
sign, both years, both universe sizes). A new method on OHLCV cannot revive a dead
channel. **The only way to test a thesis this data cannot express is a signal
source that is not a function of the price bars.** That is the bar every candidate
below must clear: does it carry information *orthogonal to* OHLCV?

Two corollaries shape the whole landscape:

1. **The feasibility gate is the binding constraint, not the hypothesis.** Most
   plausible alpha hypotheses (on-chain flows, options skew, order-flow imbalance)
   are well-motivated. What separates them is whether we can get **clean,
   point-in-time, no-look-ahead, full-2-year-history** data **cheaply**. A domain
   we cannot back-test honestly on the banked 2023-2024 window is a non-starter
   regardless of how good the story is. This note therefore weights data-feasibility
   heavily, exactly as mandated.

2. **Harness reuse vs. structural rebuild is a ~10× cost fork.** The proven stack
   — `param_robustness_sweep` (block-bootstrap θ-surface + the frozen § 0 decision
   rule + the BH control + the anchor gate) — assumes a **per-symbol time-series
   the strategy reads at each bar's open** (price, or funding-as-of via
   `funding_data.rs`). Any domain that fits that shape (a per-symbol series joined
   as-of to the bar grid) is a ~2-3-day `ScoreSource` add. Any domain that needs a
   *different backtest* (intraday order-book replay, an options-portfolio P&L
   engine) is a multi-week structural build the program has never had. This fork
   dominates the cost column below.

---

## 2. The load-bearing feasibility finding — free-live ≠ free-historical

The mandate explicitly flags Binance's free order-book / open-interest /
long-short-ratio endpoints as a candidate. **I checked them, and there is a
decisive trap the operator must see:** these endpoints are free and unauthenticated
to *poll live*, but they **do not serve history** — they retain only the most
recent window:

| Binance free endpoint | Retains | Source |
|---|---|---|
| `/futures/data/openInterestHist` (open interest) | **latest 1 month only** | [Binance docs — Open Interest Statistics](https://developers.binance.com/docs/derivatives/usds-margined-futures/market-data/rest-api/Open-Interest-Statistics) |
| `/futures/data/globalLongShortAccountRatio` (L/S ratio) | **latest 30 days only** | [Binance docs — Long/Short Ratio](https://developers.binance.com/docs/derivatives/usds-margined-futures/market-data/rest-api/Long-Short-Ratio) |
| `/futures/data/topLongShort*Ratio`, `/takerlongshortRatio` | same 30-day window (same `futures/data` family) | (ditto) |
| **`/fapi/v1/fundingRate`** (funding) | **full history** (paginated by `fundingTime`) | already banked: `fetch_binance_funding.rs`, 2023-2024 |
| **`/fapi/v1/premiumIndex`** (mark/index/premium) | **full history-capable** (same family as funding; live-polled today by `funding.rs`) | `crates/data/src/funding.rs:180` |
| **`/fapi/v1/klines`, `/api/v3/klines`** (OHLCV) | **full history** | already banked: `fetch_binance_klines.rs`, 240 parquets |

**Consequence:** any backtest on the banked 2023-2024 window that needs open
interest or the long/short ratio **cannot be sourced from Binance for free** —
the history simply does not exist on the public endpoint. To get 2 years of point-
in-time OI/LSR you must **pay a third-party flat-file vendor** (CoinAPI is named
explicitly as the backfill route for exactly this gap —
[CoinAPI: Open Interest data](https://www.coinapi.io/blog/open-interest-data-api)).
That reclassifies "Binance microstructure" from FREE to **PAID-for-history** and
sharply lowers its expected value as a *first* probe.

The **funding** and **premium-index** endpoints are the exceptions: they are in
the `/fapi/v1/*` family (not `futures/data/*`) and serve full history. Funding is
already banked; the premium index is the **same shape** and reachable with the
**same fetcher**. This is precisely why the basis (built from the premium index)
is the cheapest structurally-new series — it sidesteps the 30-day trap entirely.

---

## 3. The candidate landscape (the comparison table)

Scored on the four mandated dimensions. **Cost** = dev-days to a *first robustness
verdict reusing the harness*. **Prior** = honest probability of finding a ROBUST
(per the frozen § 0 rule) signal — kept deliberately sober given the program's
uniform negative. **EV rank** = a qualitative alpha-potential × feasibility ÷ cost
ordering (1 = test first).

| # | Domain | Hypothesis (why orthogonal to OHLCV) | Data feasibility (the gate) | Harness fit | Cost (dev-days) | Prior | EV rank |
|---|---|---|---|---|---|---|---|
| **A** | **Perp–spot basis** (perpetual premium / mark-vs-spot) | Derivatives **positioning/leverage** pressure, not realized price. Basis = where leveraged demand sits; can lead spot when crowded. **Not a function of OHLCV.** | **FREE, full history, no cap.** Binance `/fapi/v1/premiumIndex` (`markPrice`,`indexPrice`); the funding fetcher already downloads `markPrice` and **discards it** (`fetch_binance_funding.rs:111`). One additive column. | **HIGH** — clone `funding_data.rs` as-of loader; new `ScoreSource` arm; **no new backtest**. | **~2-3** | **LOW-MED** — funding's twin; carry already FRAGILE; lit: single-asset funding predictive power **decays** ([Presto Research](https://www.prestolabs.io/research/can-funding-rate-predict-price-change)). | **1** |
| **B** | **On-chain** (exchange net-flows, active addresses, stablecoin supply) | **Settlement-layer truth** entirely outside any exchange's price tape: coins moving *to* exchanges (sell pressure), stablecoin mint/burn (dry powder), address activity (adoption). The most genuinely orthogonal domain. | **FREE-ish, full history, daily.** DeFiLlama (TVL, stablecoin supply, DEX vol — **no key**, full history); Dune free tier (queryable, slow, public); Glassnode/CryptoQuant free tiers are **daily + delayed** ([CoinMarketCap free-tier comparison](https://coinmarketcap.com/academy/article/best-free-crypto-api-in-2026-free-tier-comparison)). **Daily resolution** → matches a daily backtest only; point-in-time hygiene (revisions/reorg) needs care. | **MEDIUM** — needs a **new fetcher** (per-source schema) but the funding as-of-join template still applies on a **daily** bar grid. | **~5-8** (fetcher + per-metric schema + PIT hygiene) | **MED** — the strongest *orthogonality* story; but daily-only + 2yr window is thin, and free tiers are coarse/delayed. | **2** |
| **C** | **Open interest / long-short ratio** (derivatives crowding) | Crowding/leverage build-up; OI divergence from price flags fragile rallies; L/S ratio is a contrarian sentiment gauge. Orthogonal to OHLCV. | **PAID for history.** Binance free endpoints retain **only 30 days** (§ 2) → no 2023-2024 backfill without **CoinAPI/Coinglass paid flat files**. Live-free, history-paid. | **MEDIUM-HIGH** (once banked, same as-of join) — but blocked on a **paid data buy** first. | ~3-4 *after* a paid backfill (else ∞) | **LOW-MED** — same derivatives-positioning family as the basis/carry that already failed. | **4** |
| **D** | **Options / implied-vol surface** (Deribit DVOL, skew, term structure) | **Forward-looking** risk pricing: IV term structure & skew encode the market's expected distribution, not its realized path. Genuinely different information. | **FREE history for the *index*** — DVOL back to 2021-04 via CryptoDataDownload / Tardis free streams ([search](https://www.cryptodatadownload.com/data/deribit/)). **Full surface (skew/term) = PAID / heavy.** Only BTC + ETH have liquid options (universe shrinks to 2). | **LOW for a surface** (needs an options-aware backtest); **MEDIUM** if reduced to a **DVOL scalar regime filter** on BTC/ETH (fits as a per-symbol series). | ~3 (DVOL-scalar filter on BTC/ETH) … multi-week (full surface) | **MED** — vol-timing is a documented effect, but the project already retired a GARCH-σ forecaster bet; and 2-symbol universe is thin. | **3** |
| **E** | **Cross-exchange basis / dislocation** (Binance vs Coinbase/Kraken spot) | Venue dislocations / lead-lag; one venue's price can lead another's. Orthogonal to any single tape. | **FREE history, but needs new fetchers.** Repo's Coinbase/Kraken modules are **live WS only** (`wss://`), not historical REST; Coinbase `/products/{id}/candles` + Kraken `/0/public/OHLC` exist but are **uncoded**. | **MEDIUM** — two new kline fetchers (Binance fetcher is the template) + an aligned 2-venue join. | ~4-6 | **LOW** — arb is latency/HFT territory; on banked hourly bars the dislocation is mostly already closed → little for a slow strategy to harvest. | **5** |
| **F** | **Non-crypto universe** (equities / FX via Yahoo) | A different *asset class* may carry a trend/carry edge crypto large-caps lack; tests whether the **method or the asset** was dead. | **FREE, full history.** Yahoo `query1.finance.yahoo.com/v8/finance/chart` — fetcher exists (`fetch_yahoo_klines.rs`). **BUT** banked Yahoo is **crypto-USD only**; equities/FX must be fetched fresh, and the **sweep harness reads Binance-schema parquets only** — `RealDataBarSource` is hard-coded to `data/binance` + `Timeframe::OneHour`. | **LOW-MED** — fetch is easy, but the **harness needs a Yahoo-schema adapter** (the sweep has never ingested Yahoo OHLCV; only `universe_diag.rs` did). | ~4-5 (adapter + fetch) | **N/A to the crypto thesis** — answers a *different* question (is the *method* portable?), not "what signal does crypto carry beyond price?". Useful but off-mandate for "new signal *class*". | **6** |

**Reading the table.** The gate (column 3) does the discriminating, exactly as the
program's information-theoretic diagnosis predicted feasibility would. Three
domains are FREE + full-history + harness-shaped: **A (basis)**, **B (on-chain,
daily)**, and the *index-only* slice of **D (DVOL scalar)**. Of those, **A is the
cheapest by a wide margin** because the fetcher already downloads the raw field
and the carry loader is a line-for-line template — it is the only candidate with
**zero new plumbing risk**. **B has the best orthogonality story** but costs a new
fetcher + point-in-time hygiene + is daily-only (thin on a 2-year window). **C and
E are blocked on a paid buy or new fetchers** and sit in the same
derivatives-positioning family that already failed (C) or in HFT territory (E).
**F answers a different question** (method portability), not the mandated "new
signal class for crypto."

---

## 4. The recommended first domain — perp–spot basis (rationale)

**EV = alpha-potential × feasibility ÷ cost is maximized by A.** The argument:

### 4.1 Why the hypothesis is genuinely orthogonal to OHLCV
The perpetual **basis** (the premium index = how far the perp's mark price sits
above/below the spot index) is a direct readout of **leveraged positioning
pressure**, not of realized price. When leveraged longs crowd in, the perp trades
rich to spot (positive basis) and funding turns positive to pull it back; the
*level and change* of that premium is information about **who is positioned and how
hard** — which the price bars do not contain. This is a real economic channel
(the basis is the no-arbitrage tether between two instruments, and its deviations
are a positioning gauge), and it is the single most-cited derivatives signal after
funding itself. It clears the § 1 orthogonality bar: it is **not** a function of
OHLCV.

### 4.2 Why the feasibility is best-in-class (the decider)
- **The raw field is already being downloaded and discarded.** `fetch_binance_funding.rs:111`
  literally comments "*markPrice is present but we do not store it (advisory, not
  settlement price)*". The basis ingredient is one line away. The live poller
  `crates/data/src/funding.rs` already hits `/fapi/v1/premiumIndex` and parses the
  premium fields. **No new endpoint, no new auth, no vendor, no cost.**
- **Full history, no 30-day trap.** Unlike OI/LSR (§ 2), `/fapi/v1/premiumIndex`
  and the funding endpoint are full-history. We can bank a clean point-in-time
  2023-2024 series exactly as the funding backfill did (`carry-funding-data-backfill`
  is the precedent feature).
- **The integration template is already written and tested.** `funding_data.rs`
  is the exact shape: a parquet loader mirroring `RealDataBarSource`, a locked
  REVISION SHA gate, and a pure `as_of` forward-fill (`funding_as_of`) with a
  **no-look-ahead falsifier** already in its test module. A basis loader is a clone
  with one numeric column instead of `funding_rate`.

### 4.3 Why the cost is ~2-3 dev-days (harness-shaped)
The sweep already has the `SweepScoreSource` enum → `strategy::ScoreSource` dispatch
(`param_robustness_sweep.rs:1089-1104`). Carry added exactly one arm
(`Carry → FundingCarry`) + the funding sidecar loader + falsifiers + a 2-year
anchored surface. The basis is the **same delta**: a `Basis` arm + the premium
sidecar loader + the mandated day-1 baseline-divergence e2e + 2 anchored surfaces.
Compute is a rounding error (~1 min for both years, by the carry/horizon precedent).

### 4.4 Why this is the right *first* probe even with a sober prior
The honest prior is **LOW-to-MEDIUM**: the basis is funding's twin, the carry
family already came back FAMILY-UNIFORM-FRAGILE on both years, and the literature
is explicit that **single-asset funding/basis predictive power decays** and is
"*more useful cross-sectionally*" — but cross-sectional ranking is the very channel
§ M4 proved dead on this universe. So a robust basis edge is not the base case.
**That is exactly why it should be tested first and cheaply:** it is the
closest-to-already-built structurally-new series, and a FRAGILE result is
**decision-grade** — it would tighten the conclusion to "*derivatives-positioning
signals on these large-caps are dead, not just price*" and route the next (more
expensive) dollar to the genuinely orthogonal **on-chain** domain with full
justification, rather than spending CoinAPI/Glassnode money on a guess. Testing the
cheapest orthogonal series before the expensive ones is the durable-over-quick
sequencing: it buys the most information per dollar and de-risks the on-chain build.

---

## 5. The day-1 falsifiers (what would prove the basis signal real or dead)

Pre-registered now, before any number (the project's scientific-integrity
discipline + the frozen § 0 decision rule):

1. **Orthogonality check (the gate):** correlate the basis series against the
   contemporaneous OHLCV return/momentum signal. If basis ≈ a deterministic
   function of recent price (corr ≈ ±1), it carries **no new information** and the
   domain is dead on arrival — fail fast, before building the sweep arm.
2. **Basis IC ≈ 0 falsifier (the § 6 Option-B spike, run first if budget-tight):**
   compute the sign-persistence / rank-IC of basis → forward-return the SAME way
   `universe_diag.rs` did for price rank. If it is ≈ 0 with no stable sign (like
   price rank IC was), the domain is killed for ~0.5-1 day and **no `ScoreSource`
   is ever built**.
3. **Baseline-equity-divergence e2e (CLAUDE.md non-negotiable, day 1):** the basis
   overlay's output equity MUST diverge from the un-traded baseline by ≥ 1 bp when
   the basis decision variable is non-trivial — guards against a v3-vol-overlay-style
   no-op where the signal is computed but never applied. Pattern:
   `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`.
4. **No-look-ahead falsifier:** the as-of join must use only premium settled
   **at or before** each bar's open; future-shifting the basis series MUST change
   the result (direct clone of `funding_data.rs`'s `no_look_ahead_falsifier`).
5. **Frozen-rule verdict (void-if-not-`block-bootstrap-real`/`shared-index`):**
   score the 2-year basis θ-surface against
   [`robustness-decision-rule-2026-05-30.md`](robustness-decision-rule-2026-05-30.md)
   § 0 verbatim — the same weakest-link composite that scored all four families and
   three horizons. No goalpost-moving.
6. **BH-relative bar:** the basis surface's best cell must be read against the same
   buy-and-hold control every prior surface used; "beats nothing" = the program's
   recurring killer.

---

## 6. Feature stub — for operator go/no-go (NOT greenlit, NO build, NO trace row)

> Per the trace.toml ownership rule, the analyst creates the `[[req]]` row when a
> feature enters `proposed`. **This stub is deliberately NOT yet a `proposed` row**
> — the operator's go/no-go decides greenlight. On approval, the analyst creates
> the row + `spec/<slug>/feature.md`.

**Slug (proposed):** `perp-basis-signal-robustness`

**Why:** The OHLCV-only active-trading thesis is closed across method, universe,
and horizon. The cheapest *structurally-new* (non-OHLCV) series we can bank is the
perpetual **basis** — the funding fetcher already downloads the `markPrice`
ingredient and discards it. Testing it reuses the proven harness end-to-end and,
whatever the sign, is decision-grade: a robust edge is a new product direction; a
FRAGILE result retires the entire derivatives-positioning family (basis + funding)
and justifies paying for the orthogonal on-chain domain next.

**Scope — two options, operator picks the entry point:**

- **(Recommended) Option A — full harness arm (~2-3 dev-days + ~1 min compute).**
  Bank the 2023-2024 premium-index series via a `fetch_binance_premium` bin (clone
  of `fetch_binance_funding.rs`, add `mark_price`/`index_price` columns, own
  `REVISION.toml` under a new `data/binance-premium/` tree — leave `data/binance`,
  `data/binance-funding`, `data/yahoo` byte-immutable). Add a `BasisDataSource`
  loader (clone `funding_data.rs`, locked SHA). Add a `SweepScoreSource::Basis`
  arm + a small LOCKED θ-grid (basis lookback × entry/exit band) reusing
  `run_path` + `DistributionSummary` + `BlockBootstrapPathGen` + the BH control
  verbatim. Ship the **day-1 baseline-divergence e2e** + no-look-ahead +
  two-run-byte-identity falsifiers (the carry build is the line-for-line template).
  Emit 2 anchored surfaces (2023 + 2024) scored against the frozen § 0 rule.
  *This is the durable choice:* it produces the full anchored verdict in one pass
  and, if the basis is robust, the sizing arm is already production-shaped — no
  "now build the strategy" follow-on.

- **Option B — research spike first (~0.5-1 day, NO strategy code).** Fetch the
  premium series and compute orthogonality (falsifier 1) + basis IC / sign-
  persistence (falsifier 2) the same way `universe_diag.rs` did for price rank,
  BEFORE writing any `ScoreSource`. *This is the if-budget-tightens fallback:* if
  basis IC is ≈ 0 like price rank IC, the domain dies for almost nothing and Option
  A is never built. The carve-out: it cannot by itself *prove* a robust edge (only
  the sweep can), so a positive spike still spawns the Option-A follow-on. Pick B
  only if the ~2-3-day Option-A budget is genuinely unavailable this cycle.

**Data source (no fabrication):** Binance `GET /fapi/v1/premiumIndex`
(`markPrice`, `indexPrice`, premium) — full-history, free, no auth; already hit by
`crates/data/src/funding.rs:180`; the `markPrice` field is already parsed-then-
discarded by `crates/data/src/bin/fetch_binance_funding.rs:111`. Universe: the
ORIGINAL 10 large-caps under `data/binance` (pin `3a8b96c4…`) — keeps it directly
comparable to the four retired families and reuses the banked OHLCV with zero new
universe risk (the `universe-method-diagnosis` § S.5.2 rationale carries over).

**Harness integration:** direct reuse of `param_robustness_sweep` — new
`SweepScoreSource::Basis` → `strategy::ScoreSource::Basis` dispatch
(`param_robustness_sweep.rs:1089`), basis-as-of join via the `funding_data.rs`
template, scored by the frozen `robustness-decision-rule-2026-05-30.md` § 0
weakest-link composite + BH control. **No structurally-new backtest required.**

**Expected cost:** Option A ~2-3 dev-days + ~1 min compute; Option B ~0.5-1 day,
no compute beyond a read-only pass.

**Honest prior:** LOW-to-MEDIUM for a ROBUST verdict (basis is funding's twin;
carry already FRAGILE; single-asset funding/basis predictive power decays per the
literature). The value is the **cheap, decision-grade disambiguation**: it is the
last and cheapest member of the derivatives-positioning family to test, and its
result (either sign) directs the next, more-expensive domain choice with full
justification.

**Pre-condition / sequencing (durable-over-quick):** none blocking — the basis is
the cheapest first probe. If the operator wants maximum caution, run Option B's
orthogonality + IC spike inside the first day of Option A and abort to a research
note if falsifiers 1-2 fail (saves the sweep-arm cost). If the basis comes back
FRAGILE, the pre-registered next domain is **on-chain (B in § 3)** — the genuinely
orthogonal source — at which point the higher fetcher + PIT-hygiene cost is
justified by the basis result.

---

## 7. Assumptions & limits (challengeable by operator / architect)

1. **The basis is funding's twin, so the prior is honestly low.** The strongest
   case *against* testing it first is "carry already failed, the basis is the same
   family, skip to on-chain." The rebuttal is cost: the basis is ~2-3 days of
   already-templated work vs on-chain's ~5-8 days of new fetcher + PIT hygiene, and
   the *level/change of the premium* is a distinct quantity from the *funding rate*
   (funding is the basis's mean-reversion mechanism, not the basis itself) — so it
   is a genuine, if related, new test. If the operator judges the family-relatedness
   decisive, **skip straight to on-chain (B)** — that is a defensible call and is
   why B is ranked #2, not buried.
2. **Daily-only resolution caps on-chain (B).** Free on-chain tiers
   (Glassnode/CryptoQuant) are daily + delayed; DeFiLlama is daily. A 2-year daily
   window is ~730 points per series — thin for a robust block-bootstrap tail. If
   on-chain becomes the chosen domain, the backtest horizon must be daily (the
   horizon-retest already built the daily resampler + corrected annualization, so
   that plumbing exists), and point-in-time hygiene (no revised/back-filled values)
   needs an explicit guard.
3. **The 30-day microstructure cap (§ 2) is the single most important feasibility
   fact** and reclassifies OI/LSR from "free" to "paid-for-history." If the operator
   wants OI/LSR specifically, budget a CoinAPI/Coinglass flat-file buy first; do not
   assume the free live endpoints can be back-filled.
4. **Options (D) collapses the universe to BTC+ETH** (the only liquid crypto option
   chains) and the project already retired a GARCH-σ vol-forecaster bet — so a vol
   domain inherits that skepticism. The DVOL *index* is free-historical and could be
   a cheap regime *filter* (not a standalone alpha), but that is a different,
   smaller experiment than a signal class.
5. **Non-crypto (F) answers a different question** (is the *method* portable to
   another asset class?) than the mandated "what signal does crypto carry beyond
   price?". It is genuinely interesting and the Yahoo fetcher exists, but it needs a
   harness adapter (the sweep reads Binance-schema only) and is off the "new signal
   *class* for crypto" mandate — hence ranked last *for this mandate*, not deemed
   worthless.
6. **All priors are deliberately sober.** After a four-family × three-horizon
   uniform negative, the base rate for "this next thing finally works" is low. The
   recommendation is justified by **cheap, decision-grade information per dollar**,
   not by optimism about the basis specifically.

---

## Basis spike results

> **Spike mandate (operator-approved, ~0.5-1 day, RESEARCH only — no strategy
> build).** The § 6 Option-B research spike, run to decide cheaply whether the
> perp-spot basis is a LIVE orthogonal signal or funding's dead twin BEFORE
> committing the ~2-3-day Option-A harness build. Question (the falsifiers § 5.1-2):
> does the trailing **basis** (the perpetual premium index) predict forward
> returns, and is it orthogonal to (a) OHLCV momentum and (b) the funding/carry
> signal? Every number below traces to freshly-fetched, REVISION-pinned banked
> data — NO fabrication (per the fabricated-"Sharpe 1.40" precedent). Strict
> no-look-ahead on the basis as-of join, **proven** by a leak-check falsifier
> (B1-LEAK below). No anchored/immutable files touched; `crates/ui/`,
> `data/binance`, `data/binance-funding`, `data/yahoo` all byte-immutable.

### BS.0 TL;DR — the verdict: **LIVE** (MEDIUM-HIGH confidence)

**The perp-spot basis is NOT funding's dead twin. It carries a real, sign-stable,
cross-year-replicating CROSS-SECTIONAL REVERSAL signal that is largely orthogonal
to OHLCV momentum and only moderately redundant with funding.** This is the first
non-flat, decision-grade-positive result the post-OHLCV program has produced.

| Falsifier (pre-registered § 5) | Result | Pass/kill |
|---|---|---|
| **Basis IC ≈ 0?** (§ 5.2) — cross-sec rank-IC of trailing-basis → fwd-return | **NO — IC is ≠ 0, NEGATIVE, and GROWS with horizon** (L=60: −0.099/−0.081; L=168: −0.112/−0.069), **same sign both years** | **LIVE** (above ±0.03, stable sign) |
| **Basis ≈ a function of price?** (§ 5.1) — corr(basis, OHLCV-mom) | corr **+0.01…+0.23** at the signal-bearing 9h-168h horizons | **orthogonal** (not a price transform) |
| **Basis ≈ funding's twin?** (§ 5.1) — corr(basis, funding) | **+0.47 (2023) / +0.66 (2024)** level corr — moderate, **NOT ≈ +1** | **distinct quantity** (~25-55% shared variance) |
| **No-look-ahead** (§ 5.4) — causal trailing vs leaked contemporaneous B1 | causal ≠ leaked at **every** horizon; leaked flips POSITIVE where causal is NEGATIVE | **causal** (past-only; the predictive sign is not a leak artifact) |

> **The basis-rank reversal effect (high perp-premium names subsequently
> UNDERPERFORM) is real, past-only, sign-stable across 2023 AND 2024, peaks at
> −0.08 to −0.11 rank IC over the 2.5-day-to-1-week horizon, and is largely
> independent of both price momentum and the (already-FRAGILE) funding signal.**
> → **PROCEED to the Option-A full-harness build (`perp-basis-signal-robustness`),
> framed as a CROSS-SECTIONAL BASIS-REVERSAL sizing arm.** Do NOT skip to on-chain.

**Confidence MEDIUM-HIGH, not HIGH** — three honest caveats keep this below HIGH:
(i) the *magnitude* is modest (peak |IC|~0.10) and a raw rank-IC is the
*upper-bound* information content, not net-of-fee P&L — the cross-sectional rank
channel was already shown ≈0 for *price* (§ M4), so a −0.10 *basis* rank IC is a
genuine improvement but still must clear the BH bar + the frozen § 0 weakest-link
rule in the real sweep, which the spike cannot do; (ii) the signal is **reversal**
(negative IC), so the sizing arm shorts/underweights high-basis names — the
opposite of a carry/momentum long-tilt, and reversal edges are the most
fee-sensitive (frequent rebalancing); (iii) the long-horizon L=720 cells are
n=11 windows and sign-flip across years (2023 −0.195 vs 2024 +0.033) — **ignored
as noise**, exactly the M4/broader-universe L=720 caveat. The LIVE call rests on
the **L=9 → L=168 band**, where the sign is stable and n is adequate (51-974
windows).

### BS.1 What was fetched + the cheapest-valid acquisition decision

**Acquisition decision (the mandate's "cheapest valid way"):** I fetched the
**premium-index klines** (`GET /fapi/v1/premiumIndexKlines`), NOT the funding
endpoint's discarded `markPrice`. Rationale — this is *both* cheaper *and* the
only valid source on the hourly grid:

- The premium-index kline **close = `(markPrice − indexPrice) / indexPrice`
  already computed by Binance** and bucketed to `1h` — it IS the basis, natively
  aligned to the OHLCV `open_time` grid, **no separate index fetch, no manual
  division**.
- The funding endpoint (`fetch_binance_funding`) only exposes `markPrice` — **not
  `indexPrice`** — and only at the sparse 8h funding cadence. It **cannot
  reconstruct the basis** on the hourly grid. So the § 0 framing ("the fetcher
  already downloads the `markPrice` ingredient") was *half* the ingredient; the
  premium-index kline endpoint supplies the whole quantity directly and is the
  correct cheapest source. (Same `/fapi/v1/*` family → still free, unauth,
  full-history, no 30-day cap.)

**New fetcher:** `crates/data/src/bin/fetch_binance_premium.rs` (clone of
`fetch_binance_klines` pointed at `premiumIndexKlines`; writes `open_time`,
`close_time`, `basis_open/high/low/close` as signed decimal strings). 12 unit
tests (URL, pagination, out-of-window filter, **signed/negative-premium parse**,
parquet round-trip) — all green; clippy-clean under the canonical
`--workspace --all-targets --all-features` gate.

**New banked data:** `data/binance-basis/` — the **10 large-cap** symbols
(`ADAUSDT, AVAXUSDT, BNBUSDT, BTCUSDT, DOGEUSDT, DOTUSDT, ETHUSDT, LINKUSDT,
SOLUSDT, XRPUSDT` — the SAME `data/binance` pin `3a8b96c4…` universe, for direct
comparability) × 2023-2024 × `1h` = **240 month-parquets** (24/symbol, full
744/720/696 bar counts, 0 gaps). Self-contained
`data/binance-basis/REVISION.toml`, aggregate SHA
**`aa72409aa0f856960385a823bc61be1b8274e84f658439b65e5d1b1b1a48f1cd`**. The fetch
hit one Binance 429 mid-run (XRPUSDT 2024/04) — the fetcher errors hard on 429
(no back-off); resumed idempotently with `--sleep-ms 500`. `.gitignore` extended
with the `binance-basis` allow-rule (mirrors `binance-funding`/`binance-broaduni`:
parquets gitignored, REVISION tracked).

> **Reproduce** (read-only, ~1 s/yr; aligns 8 758 / 8 783 hourly returns, same
> depth as `universe_diag`):
> ```
> cargo run -p data --example basis_diag -- 2023
> cargo run -p data --example basis_diag -- 2024
> cargo run -p data --example basis_diag -- 2024 --leak-check   # no-look-ahead falsifier
> ```
> Probe: `crates/data/examples/basis_diag.rs` — reads OHLCV via the harness's own
> `ReplayFeed::merge_symbols` path (same reader as `realdata.rs`), joins the basis
> + funding parquets on the common hourly grid. No-look-ahead: the basis at the
> open of bar `t` is `basis_close[t-1]` (the premium-index close of bar `t` is
> only known at `t+1h`); trailing signals use `[t-L, t)` past bars only; forward
> returns use `[t, t+L)` future bars only; funding joined as-of (mirrors
> `funding_data.rs::funding_as_of`).

### BS.2 Basis IC — the core question (both years)

**(a) Cross-sectional rank-IC** — at each non-overlapping step, rank the 10 names
by trailing-mean basis over `[t-L, t)`, Spearman vs forward-L cum return `[t, t+L)`:

| Lookback L | 2023 rank IC | 2024 rank IC | replicates? |
|---|---|---|---|
| 3 (3h) | −0.0153 | −0.0023 | both ~0 (noise floor) |
| 9 (9h ≈ carry L) | −0.0324 | −0.0218 | **YES — negative, ~−0.02/−0.03** |
| 24 (1d) | −0.0313 | −0.0217 | **YES — negative, ~−0.02/−0.03** |
| **60 (2.5d)** | **−0.0992** | **−0.0806** | **YES — negative, ~−0.08/−0.10** |
| **168 (1wk)** | **−0.1123** | **−0.0690** | **YES — negative, ~−0.07/−0.11** |
| 720 (30d) | −0.1945 | +0.0325 | NO — n=11, sign-flips (noise, ignored) |

**Read:** unlike the *price* rank IC (§ M4: within ±0.07, no stable sign), the
*basis* rank IC is **consistently negative and grows monotonically in magnitude
from L=9 to L=168 in BOTH years**, reaching −0.08 to −0.11 over the 2.5-day-to-
1-week horizon — past the ±0.03 LIVE threshold with a **stable sign**. The
negative sign = a **reversal** effect: names whose perp trades richest to spot
(most crowded leveraged longs / highest basis) subsequently **underperform** the
cross-section. This is the economically-sensible direction (crowded longs mean-
revert) and is the cleanest non-zero cross-sectional IC the program has measured.

**(b) Per-asset time-series IC** (own trailing-basis vs own forward-return, pooled):

| Lookback L | 2023 pooled TS IC | 2024 pooled TS IC | read |
|---|---|---|---|
| 3 / 9 / 24 | −0.016 / −0.016 / +0.011 | +0.006 / +0.009 / +0.016 | ≈ 0, weak |
| 60 (2.5d) | +0.024 | +0.024 | weak-positive, replicates but tiny |
| 168 (1wk) | −0.055 | +0.054 | **sign-flips across years** |
| 720 (30d) | +0.177 | −0.214 | n=110, sign-flips (noise) |

**Read:** the time-series (own-basis → own-return) channel is **weak and not
sign-stable** (L=168 flips −0.055↔+0.054; L=720 flips +0.18↔−0.21). **The basis
signal is CROSS-SECTIONAL (relative basis rank), NOT time-series (own basis
level).** This is an important scoping result for the build: the Option-A arm
must be a **cross-sectional basis-rank** sizing rule (long low-basis / short-or-
underweight high-basis names), NOT a per-asset absolute-basis long/flat rule.

### BS.3 Orthogonality — vs OHLCV momentum and vs funding (both years)

corr of the trailing-basis signal `[t-L, t)` vs (a) OHLCV trailing-momentum
`[t-L, t)` and (b) as-of funding at `t`, pooled over names+windows:

| Lookback L | corr(basis, OHLCV-mom) 2023 / 2024 | corr(basis, funding) 2023 / 2024 |
|---|---|---|
| 3 (3h) | −0.018 / +0.010 | +0.551 / +0.713 |
| 9 (9h) | +0.025 / +0.032 | +0.652 / +0.750 |
| 24 (1d) | +0.052 / +0.070 | +0.626 / +0.714 |
| 60 (2.5d) | +0.170 / +0.127 | +0.533 / +0.673 |
| 168 (1wk) | +0.225 / +0.207 | +0.506 / +0.651 |
| 720 (30d) | +0.341 / +0.356 | +0.228 / +0.674 |

**(a) vs OHLCV momentum:** **LOW and orthogonal at the signal-bearing horizons.**
At L=9-24 (where the basis IC is cleanest) corr(basis, mom) is only **+0.02 to
+0.07**. It rises to +0.20-+0.36 at L=168-720 (long-window basis level
accumulates with realized return, as expected) — but even there the basis is
contributing distinct information, and the *predictive* basis signal is at the
shorter end where orthogonality is near-total. The basis is **not a price
transform** → it clears the § 1 / falsifier-1 orthogonality bar.

**(b) vs funding (the twin check):** **MODERATE, +0.47 to +0.75 level corr,
emphatically NOT ≈ +1.** B4 contemporaneous level corr (as-of `basis_close[t-1]`
↔ as-of funding): **+0.4728 (2023) / +0.6619 (2024)**, n≈87 600. Funding and the
basis share ~22-55% of variance (funding is, mechanically, a clamped+averaged 8h
settlement of the premium index plus an interest term — so positive correlation
is expected by construction) **but the hourly basis retains 45-78% distinct
variance** that funding's sparse clamped settlement discards. Combined with the
fact that funding came back FAMILY-UNIFORM-FRAGILE while the basis carries a
−0.10 cross-sectional reversal IC, **the basis is a genuinely different and
better-behaved signal than funding, not its redundant twin.** This is the
decisive split from the § 0 honest-prior worry ("basis ≈ funding ⇒ dead").

### BS.4 No-look-ahead falsifier (the integrity gate)

`basis_diag -- 2024 --leak-check` recomputes B1 with a deliberately **leaked**
(contemporaneous, `[t, t+L)`) basis signal and prints causal-vs-leaked:

| L | causal (trailing, past-only) | leaked (contemporaneous) | differ? |
|---|---|---|---|
| 3 | −0.0023 | −0.0214 | YES |
| 9 | −0.0218 | +0.0038 | YES |
| 24 | −0.0217 | **+0.0580** | YES |
| 60 | −0.0806 | **+0.0982** | YES |
| 168 | −0.0690 | +0.0201 | YES |
| 720 | +0.0325 | +0.0259 | YES |

**Read:** causal ≠ leaked at **every** horizon, and the leaked (look-ahead) basis
**flips POSITIVE** at L=24/60 (+0.058/+0.098) where the causal basis is NEGATIVE
(−0.022/−0.081). This is the signature of a true causal reversal: high basis
*over a window* co-moves with high *contemporaneous* return (positive leaked IC),
but high *trailing* basis predicts *subsequent* UNDER-performance (negative causal
IC). The −0.10 predictive signal is **past-only and not an alignment artifact** —
the join is causal, identical-in-spirit to `funding_data.rs::no_look_ahead_falsifier`.

### BS.5 Firmed recommendation — BUILD the basis arm (Option A), as cross-sectional reversal

1. **PROCEED to `perp-basis-signal-robustness` Option A (the full harness arm,
   ~2-3 dev-days + ~1 min compute), now with the spike's positive go-signal.** The
   spike has cleared all four pre-registered falsifiers: basis IC ≠ 0 (and sign-
   stable across years), orthogonal to OHLCV momentum, NOT a funding twin, and
   provably no-look-ahead. This is the durable choice (§ 0 framing): one pass
   produces the full anchored 2-year verdict, and if it clears the frozen § 0 rule
   the sizing arm is production-shaped — no "now build the strategy" follow-on.

2. **Frame the arm as CROSS-SECTIONAL BASIS-REVERSAL, not time-series.** BS.2(b)
   showed the own-asset time-series basis channel is weak/sign-unstable; the
   **cross-sectional rank** channel is where the −0.10 IC lives. The
   `SweepScoreSource::Basis` arm should rank the 10 names by trailing-mean basis
   and tilt **AGAINST** it (underweight/short high-basis, overweight low-basis) —
   the reversal direction the data shows. θ-grid: **basis lookback ∈ {24, 60, 168}
   bars** (the IC-bearing band; SKIP the n=11 L=720 noise cell) × an entry/exit
   band. Target universe: the **10 large-caps under `data/binance` pin `3a8b96c4…`**
   (the `data/binance-basis` pin `aa72409a…` is the matching basis side) — keeps it
   directly comparable to the four retired families.

3. **Mandatory honest framing for the architect/operator (durable-over-quick):**
   - The −0.10 IC is the **upper bound** on information content, NOT net P&L. The
     *price* rank channel was ≈0 and still produced FRAGILE sweeps; a basis rank IC
     of −0.10 is materially better but **must still clear the BH control + the
     frozen § 0 weakest-link composite** in the real block-bootstrap sweep. The
     spike CANNOT promise a ROBUST verdict — it promises the **first signal worth
     putting through the machine** since the program went negative.
   - It is a **reversal** signal → the sizing arm rebalances frequently and is the
     **most fee-sensitive** strategy class the program has built. The day-1
     baseline-divergence e2e + a fee-sweep (the frame-diagnostic's "FRAGILE even at
     0 bps?" test) are essential — if it dies at realistic Binance taker fees, that
     is the likely failure mode, and it should be tested first inside the build.
   - **If the basis sweep comes back FRAGILE** despite the −0.10 IC, that is itself
     decision-grade: it would mean the cross-sectional reversal information exists
     but is **un-harvestable net of fees on this universe/horizon**, which retires
     the entire derivatives-positioning family (price-rank + funding + basis) with
     finality and routes the next dollar to **on-chain (§ 3 domain B)** with full
     justification — the pre-registered next domain.

4. **Do NOT skip to on-chain.** The § 0 sober prior ("basis is funding's twin,
   skip it") is now **empirically refuted** by BS.3(b): the basis is only +0.47-
   +0.66 correlated with funding and carries a sign-stable reversal IC funding
   lacks. Spending ~2-3 already-templated dev-days to put the program's first
   non-zero signal through the proven harness dominates jumping to the ~5-8-day
   on-chain fetcher+PIT build on an untested hunch.

### BS.6 Spike artifacts & cleanup

- **New fetcher (KEEP):** `crates/data/src/bin/fetch_binance_premium.rs` — the
  premium-index → parquet downloader; 12 green unit tests; registered in
  `crates/data/Cargo.toml`. Needed by the Option-A build to (re-)bank the basis.
- **New banked data (KEEP):** `data/binance-basis/` (240 parquets) + its
  `REVISION.toml` (aggregate SHA `aa72409a…`). The basis side of the build's data
  pin. Parquets gitignored per the new `.gitignore` rule; only the manifest is
  tracked (mirrors `data/binance-funding`).
- **Probe (disposable, may keep):** `crates/data/examples/basis_diag.rs` — read-
  only re-runnable basis-IC/orthogonality/no-look-ahead diagnostic; clippy-clean;
  depends only on banked data + the public `ReplayFeed` API + the basis/funding
  parquet schemas. Keep as a re-runnable probe (mirrors `universe_diag.rs`) or
  delete — operator's call.
- **Untouched / byte-immutable:** `data/binance/REVISION.toml` (`3a8b96c4…`),
  `data/binance-funding/REVISION.toml` (`bf1ede44…`), `data/binance-broaduni/`
  (`518b4d40…`), `data/yahoo/` (pre-existing working-tree change, NOT mine), all
  `spec/*/reports/` anchors, `crates/ui/`.
- **`[[req]]` trace row:** still deferred — the spike was Option B (research only);
  on operator greenlight of the Option-A build the analyst creates the
  `REQ-BASIS-*` row + `spec/perp-basis-signal-robustness/feature.md` per the
  trace.toml ownership rule.

---

## Changelog

- 2026-06-05 (analyst, basis spike): ran the § 6 Option-B perp-spot-basis research
  spike. Acquisition decision: fetched `premiumIndexKlines` (the premium-index
  close = the basis, natively on the hourly grid) — NOT the funding endpoint's
  discarded `markPrice` (which lacks `indexPrice` and cannot reconstruct the basis).
  New fetcher `fetch_binance_premium.rs` (12 green tests, clippy-clean); banked the
  10 large-caps × 2023-2024 × 1h = 240 parquets into `data/binance-basis/` (new
  REVISION pin `aa72409a…`; existing pins untouched). New read-only probe
  `basis_diag.rs` (reuses the `ReplayFeed` reader; basis+funding as-of join,
  strict no-look-ahead proven by a leak-check falsifier). **VERDICT: LIVE
  (MEDIUM-HIGH).** The basis carries a cross-sectional REVERSAL rank IC of −0.08
  to −0.11 over L=60-168 bars, **negative and sign-stable in BOTH 2023 and 2024**
  (vs price rank IC ≈ 0 in § M4); it is orthogonal to OHLCV momentum (corr +0.02-
  +0.07 at the signal horizons) and only MODERATELY redundant with funding (level
  corr +0.47/+0.66, NOT ≈ +1) — refuting the § 0 "funding's dead twin" prior. The
  time-series (own-basis) channel is weak/sign-unstable, so the signal is
  cross-sectional, not time-series. Firmed recommendation: **PROCEED to the
  Option-A `perp-basis-signal-robustness` full-harness build, framed as a
  cross-sectional basis-reversal sizing arm** (θ-grid lookback {24,60,168}, tilt
  against high basis), with mandatory fee-sensitivity framing (reversal = most
  fee-exposed class) and the day-1 baseline-divergence e2e; do NOT skip to
  on-chain. The L=720 cells (n=11, sign-flip across years) flagged as noise and
  excluded from the verdict, per the M4/broader-universe discipline. `[[req]]` row
  deferred until operator greenlight of the build.
- 2026-06-05 (analyst, new-data-domain-scoping): scoped the post-OHLCV new-data-
  domain landscape per operator's strategic-fork choice. Inspected the repo's
  existing fetchers/plumbing (`fetch_binance_klines`, `fetch_binance_funding`,
  `fetch_yahoo_klines`, `funding.rs` live poller, `funding_data.rs` as-of loader,
  `param_robustness_sweep` ScoreSource dispatch, `RealDataBarSource` Binance-schema
  coupling). Decisive feasibility finding: Binance free `futures/data/*`
  microstructure endpoints (open interest, long/short ratio) retain **only the
  latest 30 days** — no 2023-2024 backfill without a PAID vendor (CoinAPI) — whereas
  the `/fapi/v1/*` funding + premium-index endpoints serve full history. RECOMMENDED
  first domain: **perp-spot basis** (premium index), because the funding fetcher
  already downloads-and-discards `markPrice` (fetch_binance_funding.rs:111), the
  carry `funding_data.rs` is a line-for-line integration template, and it slots into
  the proven block-bootstrap sweep + frozen § 0 decision rule + BH control with zero
  new plumbing — ~2-3 dev-days, honest prior LOW-MED (funding's twin; carry already
  FRAGILE; lit: single-asset funding predictive power decays). Comparison table
  ranks on-chain #2 (best orthogonality, ~5-8 days, daily-only), DVOL-scalar #3
  (free index history, BTC/ETH only), OI/LSR #4 (paid-for-history), cross-exchange
  #5 (new fetchers, HFT territory), non-crypto #6 (off-mandate, needs harness
  adapter). Feature stub `perp-basis-signal-robustness` (Option A full arm
  Recommended / Option B research-spike fallback) emitted for go/no-go; `[[req]]`
  row deferred until greenlight per trace.toml ownership rule. All citations real
  and named; no fabrication; no anchored/immutable files touched.
