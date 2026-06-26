---
slug: advisor-options-impliedvol-probe
version: 0.1.0
status: proposed
owner: analyst
priority: P2
updated: 2026-06-26
related:
  - spec/dev-notes/onchain-netflow-spike-2026-06-08.md
  - spec/dev-notes/onchain-vs-conclude-fork-2026-06-08.md
  - spec/perp-basis-mn-spread/feature.md
  - spec/product.md
  - spec/backlog.md
---

# Options / implied-vol probe — bring the Deribit DVOL channel into the bake-off as a new exogenous arm class (honest coverage of the vol channel, null-is-valid)

> **Mandate (analyst scope, FILES ONLY — feature.md only; orchestrator reconciles
> trace.toml + product.md).** Scope a NEW fresh-channel feature for the Single-Coin
> Investment Advisor (paper): an **options / implied-volatility probe** that brings the
> UNTESTED orthogonal channel named in [backlog.md § Future fresh program](../backlog.md)
> ("options/implied-vol (Deribit DVOL)") into the bake-off as a new arm class, scored by
> the IDENTICAL frozen robustness gate (`classify_verdict`, 5-signal weakest-link
> bootstrap; FRAGILE = ineligible to crown) and benchmarked against buy-and-hold. The
> active-edge hunt CONCLUDED 2026-06-08 ("ship passive") across price/OHLCV +
> derivatives-positioning + on-chain; options/IV is the honest "we ALSO checked the vol
> channel." **The deliverable is HONEST COVERAGE, not an alpha claim — a null (FRAGILE)
> result is the expected, valid, shippable outcome.** Every claim below traces to source,
> data-vendor docs inspected this session, or the on-chain precedent; nothing is
> fabricated or synthesized.

---

## 0. TL;DR — the verdict: FEASIBLE (PIT-honest, unlike on-chain), build it as a 2-symbol probe, expect FRAGILE

**The data-PIT-feasibility crux comes out the OTHER way from on-chain: Deribit DVOL is a
free, no-auth, immutable, past-only daily series that can be fetched and pinned exactly
like the existing Binance/Yahoo/basis corpora.** This is the load-bearing finding and it
is the difference between this probe and the on-chain one:

- **On-chain net-flows DIED at the PIT gate** because the vendor (CryptoQuant) **itself
  disclaims** point-in-time accuracy — historical values are *mutable*, silently
  rewritten as exchange wallets are retroactively discovered and relabeled
  ([onchain-netflow-spike-2026-06-08 § 1.1](../dev-notes/onchain-netflow-spike-2026-06-08.md)).
  That address-relabeling look-ahead is unfixable on free data.
- **DVOL has NO such mutation mechanism.** It is computed **live, in real time, from the
  Deribit option order book** via a variance-swap construction (a deterministic function
  of the bids/asks/trades/mark on the chain *at that instant*; the methodology even pins
  the fallback to "mark price from 1 min ago"). A closed DVOL candle is a **recording of
  what the index printed live**, not a backfilled estimate. Structurally this is the
  **same PIT class as the perp basis** (`(mark−index)/index`, computed live from the
  perp's own book — already certified PIT-clean and shipped in this repo,
  [`basis_data.rs`](../../crates/backtest/src/basis_data.rs)) and as OHLCV itself. There
  is no "relabeling" analogue: the option chain on 2023-05-01 is what it was; nothing
  discovered later rewrites that day's DVOL print.

→ **VERDICT: FEASIBLE and buildable.** This is NOT a forced build on bad data — the PIT
gate genuinely passes (evidence in § 1, including three independent free historical
mirrors that corroborate immutability). So the spec carries the full design (§ 2–§ 6).

**But the honest framing is load-bearing and points at a likely null:**

1. **Universe collapses to 2 symbols (BTC + ETH).** DVOL exists ONLY for BTC and ETH —
   the only crypto with option chains liquid enough to compute a stable variance-swap
   index (§ 1.4, corroborated by Deribit/CryptoDataDownload/TheBlock). There is NO DVOL
   for SOL/ADA/XRP/etc. This caps the probe to a **time-series long/flat regime arm on
   BTC and ETH**, not the 10-name cross-section the harness runs natively — the same
   thin-universe reality the on-chain stablecoin probe hit (4 names there, 2 here).
2. **The program already retired a realized-vol bet** (the v3 volatility-forecaster /
   GARCH-σ overlay came back NO-ALPHA). An *implied*-vol regime signal is genuinely
   different information (forward-looking option-market expectations, not a backward GARCH
   estimate), but it **inherits skepticism** — the prior on a survivable vol-regime edge
   is **LOW-to-MEDIUM**, deliberately not inflated.
3. **A FRAGILE result is the expected, valid, shippable outcome.** Per the frozen gate,
   FRAGILE = ineligible to crown, buy-and-hold stays the benchmark, and the honest record
   becomes "we ALSO tested the forward-looking option-implied-vol channel; it too does not
   survive." That CLOSES the vol channel with the same finality the on-chain probe gave
   its channel — which is the durable deliverable, not an alpha claim.

**Recommendation: BUILD the probe (spike-first lane available), as a 2-symbol BTC+ETH
DVOL-regime arm + a day-1 divergence gate, with NO parameter search (one pre-registered
signal). Confidence in the FEASIBILITY verdict: HIGH. Confidence the signal will be
ROBUST: LOW** (the honest prior). The value is the *coverage*, scored on the same machine
and the same bar that found / rejected every prior channel.

**If-budget-tightens annotation (the named cheaper lane):** run the **DVOL research spike
first** — a read-only `dvol_diag.rs` probe (clone of [`basis_diag.rs`](../../crates/data/examples/basis_diag.rs)
/ `stablecoin_diag.rs`) that fetches the BTC+ETH daily DVOL series, banks it under a new
REVISION pin, and computes the regime signal's rank-IC / cross-year sign-persistence /
leak-check vs forward return BEFORE building any `SweepFamily`/`ScoreSource`/arm. ~1–2
dev-days; kills or greenlights the channel for almost nothing, exactly as the basis and
stablecoin spikes did. Gate the full arm build on the spike showing a non-zero,
sign-stable daily IC. This is the path to default to if the ~4–6-day full build is not
affordable this cycle.

---

## 1. DATA-PIT-FEASIBILITY — the crux, assessed FIRST (the load-bearing section)

The on-chain probe established the discipline: **the feasibility / PIT gate runs FIRST and
is binding — design no signal on data you cannot back-test honestly.** Options/IV carries
the same risk, so it gets the same gate first. Unlike on-chain, it passes.

### 1.1 The free, no-auth, public endpoint exists

| Gate | Result | Evidence |
|---|---|---|
| **Free?** | **YES** — no payment, no API key. | Deribit `public/get_volatility_index_data` is under the `/public/` namespace ("no authentication needed"; docs.deribit.com llms.txt / OpenAPI, inspected this session). A free CSV mirror also exists (CryptoDataDownload, "we list both [BTC, ETH] for free"). |
| **Endpoint contract** | `GET public/get_volatility_index_data` → "volatility index chart data formatted as **candles**" (OHLC + timestamp). Params: `currency` (BTC/ETH), `start_timestamp`, `end_timestamp`, `resolution`. | Deribit API reference, inspected this session. |
| **Daily resolution?** | **YES** — sub-daily candles (e.g. 1h/12h) fold to a daily close on the same daily grid the stablecoin/basis probes used; the free CSV mirror serves `[Daily]` BTC+ETH OHLC directly. | CryptoDataDownload Deribit page ("Daily OHLC candles"). |
| **History depth?** | **2021-04-01 → present** (the `deribit_volatility_index` stream start), comfortably covering the program's 2023–2024 robustness window with margin. (Free CSV mirror starts ~2022-09; Deribit's own API reaches back to 2021-04 — either covers 2023–2024.) | Deribit Insights (DVOL launched Mar 2021); CryptoDataDownload; Tardis.dev. |

### 1.2 The PIT / immutability argument — DVOL is PAST-ONLY by CONSTRUCTION (THE gate)

This is the section that decided the on-chain verdict, and it is where DVOL **diverges**
from net-flows. The on-chain kill was the **vendor's own disclaimer** that history mutates
(address relabeling). DVOL has the opposite property, by construction:

- **DVOL is a deterministic function of the live option chain at print time.** Deribit
  computes it from the implied-vol smile across the two expiries bracketing 30 days, via
  the variance-swap methodology, using the order book's bids/asks (fallback: last-minute
  trades, then mark-price-from-1-min-ago). The value at 2023-05-01T00:00Z is whatever that
  formula returned over *that day's* book. **Nothing learned later changes it** — there is
  no "we reclassified an address" event that could retroactively rewrite a past option
  price. (Contrast: net-flow's entire mutation source is exactly such reclassification.)
- **It is the SAME PIT class as the basis, which this repo already certified clean.** The
  basis (`(markPrice − indexPrice)/indexPrice`) is also computed live from a Deribit-style
  perp book and is shipped as a PIT-clean exogenous series with a no-look-ahead falsifier
  ([`basis_data.rs` § Invariant (no look-ahead)](../../crates/backtest/src/basis_data.rs),
  the `basis_as_of` as-of join + `no_look_ahead_falsifier` test). DVOL is an option-book
  analogue of that exact construction. If the basis is PIT-clean, DVOL is PIT-clean by the
  identical argument.
- **Three independent free mirrors corroborate immutability.** The same historical DVOL
  OHLC series is served by Deribit (`get_volatility_index_data`), CryptoDataDownload (free
  CSV), Glassnode (`derivatives.DvolOhlc`), TheBlock, and TradingView. If DVOL values were
  silently revised after the fact, these independently-collected mirrors would disagree or
  carry a revision disclaimer; they serve a single consistent OHLC history and **none
  carries a PIT/mutability disclaimer** (the conspicuous contrast with CryptoQuant, which
  *does*). Convergent independent recordings of the same printed series is the empirical
  signature of an immutable past-only feed.

> **The one residual PIT caveat (honest, and not disqualifying).** As with the
> stablecoin probe (§ 1.3 there), I can only fetch the series *once* (today), so I cannot
> longitudinally prove a 2023 DVOL value is byte-identical to what the API served in 2023.
> But the **construction** (a live order-book function with no relabeling substrate) makes
> retroactive rewriting structurally implausible — categorically unlike net-flow, where
> the vendor *confirms* rewriting happens. The day-1 leak-check falsifier (§ 5, the
> required e2e gate) closes the *join* side (no future DVOL leaks into a past bar);
> structural immutability + multi-mirror corroboration closes the *source* side. Residual
> risk: LOW. This is a genuine FEASIBLE, not a feasibility we are squinting to assert.

### 1.3 Pinning — identical to the existing corpora

The banked series rides the **exact REVISION-manifest pattern** the repo already uses for
every external corpus (`data/binance/`, `data/yahoo/`, `data/binance-basis/`,
`data/defillama-stablecoins/`): fetch once → write daily parquets → compute an aggregate
SHA-256 → pin it in `data/<corpus>/REVISION.toml` (parquets gitignored, manifest tracked).
The loader verifies the on-disk aggregate SHA against a locked constant and **refuses to
run on unverified data** — the pattern in `basis_data.rs` (`EXPECTED_BASIS_REVISION_SHA`,
`BasisDataError::RevisionMismatch`). So DVOL is pinnable to the same immutability standard
as the price corpus. Proposed corpus dir: `data/deribit-dvol/` (new; gitignored parquets +
tracked `REVISION.toml`). **This is the cleanest precedent match in the whole probe** —
the stablecoin manifest (`data/defillama-stablecoins/REVISION.toml`) is a drop-in template
(forward-dated daily snapshot, `day_key Int64 / value Float64` schema).

### 1.4 The buildable universe — a load-bearing scoping reality (2 symbols)

DVOL exists **only for BTC and ETH** — the only crypto whose option chains are liquid
enough for a stable variance-swap index. There is no SOL/ADA/XRP/DOGE/etc. DVOL (no liquid
option market → no index to compute). Verified this session across Deribit, CryptoDataDownload
("we list both: BTC and ETH"), and TheBlock.

**Consequence (mirrors the on-chain stablecoin universe reality § 2 there):** the probe
supports at most a **2-name universe**. A 2-wide cross-sectional rank-IC is meaningless
(rank noise dominates), so the **honest framing is a per-symbol TIME-SERIES regime arm**:
for each of BTC and ETH independently, does the trailing DVOL regime (level/trend) lead
that symbol's own forward return? This is a **long/flat market-timing arm on 2 symbols**,
not the 10-name cross-section the bake-off runs natively. The thin universe (i) lowers the
probe's power to detect a weak edge and (ii) caps the eventual strategy's breadth — both
are reported, not hidden, and both lean the expectation toward the FRAGILE / null outcome.

---

## 2. IF feasible — the hypothesis + the PRE-REGISTERED signal (it is feasible)

**No parameter search. One fixed, pre-registered signal, declared before any backtest**
(per the anti-cherry-pick discipline — the gate crowns no argmax winner; a searched signal
would void the honest-coverage claim).

### 2.1 The decorrelation rationale (why this channel is orthogonal)

Implied vol is **forward-looking option-market information**: the market's priced
expectation of future 30-day realized vol, extracted from option prices. It is
**structurally orthogonal** to the coin's own realized price/volume tape (the OHLCV
channel that came back fragile) and to leveraged-positioning pressure (the basis/funding
channel that came back fragile, basis ≡ funding). DVOL reads a *different quantity* — the
risk the option market is *pricing*, not the price the spot market *printed*. It is the
one channel on the board (with on-chain now closed) that is forward-looking rather than a
transform of the realized past. That orthogonality is the entire reason to spend the
coverage dollar — though orthogonality is only valuable if a replicating signal exists,
which the probe will test, not assume.

### 2.2 The pre-registered signal: a DVOL vol-regime long/flat filter

**Hypothesis (pre-registered):** crypto exhibits a *calm-regime risk premium* — when
implied vol is LOW / falling (the option market prices a calm forward regime), holding the
coin earns the drift with low realized risk; when implied vol SPIKES (the option market
prices fear / a stress regime), forward returns are more likely negative / drawdown-heavy,
so step to cash. This is the classic "risk-off when vol spikes" / vol-target-style regime
filter, applied with IMPLIED (forward-looking) rather than realized (backward) vol — the
distinction that makes it genuinely new versus the retired GARCH-σ bet.

**Concrete, FIXED rule (per symbol s ∈ {BTC, ETH}, daily grid, strictly causal):**

- `dvol_t` = the as-of DVOL daily close for symbol `s`, available at bar `t`'s open
  (the close of the prior completed DVOL day — the `basis_as_of` "at-or-before" convention,
  strict no-look-ahead).
- Regime classifier (the pre-registered form — ONE rule, NOT a grid):
  - **Calm / RISK-ON (hold the coin, weight = 1):** `dvol_t` is below its own trailing
    median over a FIXED lookback `W = 30d` AND not rising sharply.
  - **Stress / RISK-OFF (step to cash, weight = 0):** `dvol_t ≥ trailing-median(W)` (vol
    elevated relative to its own recent regime).
- The signal is a {0, 1} long/flat weight on each symbol's own spot bars. Buy-and-hold
  (always weight 1) is the benchmark; the arm only diverges by going flat in stress
  regimes.

**Pre-registration of the parameters (locked BEFORE backtest, no search):** `W = 30d`
trailing median, daily rebalance, threshold = the trailing median itself (a self-normalizing,
parameter-light cut — deliberately chosen so there is NOTHING to tune; "below its own
30-day median" has no free knob to argmax over). The 30d window matches DVOL's own 30-day
forward horizon (the index *is* a 30-day vol gauge), so it is theory-motivated, not fit.
The architect may challenge `W` or the median-vs-quantile cut as an M-T1 lock decision
(§ 7 open decisions), but the **probe ships with exactly one rule** — any sensitivity sweep
is a SEPARATE, explicitly-labeled robustness check, never a crowning search.

> **Alternative signal considered and explicitly NOT chosen (recorded for the architect):**
> a *vol-risk-premium* arm (implied DVOL minus trailing realized vol → harvest when IV
> richly overprices RV). It is a coherent second hypothesis but (a) needs a realized-vol
> estimator that re-introduces exactly the GARCH-σ machinery the program retired, muddying
> the "this is a NEW channel" claim, and (b) is a relative-value/carry construction closer
> to the retired funding-carry family. The regime long/flat filter is cleaner, more
> obviously orthogonal, and parameter-light. The VRP arm is named as a possible follow-on
> IFF the regime arm shows life (it will not, on the honest prior). **Do not build both —
> one pre-registered signal.**

---

## 3. The seam — reuse the proven exogenous-series path (NOT a new seam)

**Reuse, decisively.** The repo already has the exact seam an exogenous IV-regime arm
needs, built and shipped for the perp-basis feature. DVOL rides it almost unchanged.

### 3.1 The exogenous-series as-of seam (basis_data.rs) — the load-bearing reuse

[`crates/backtest/src/basis_data.rs`](../../crates/backtest/src/basis_data.rs) is a
SECOND (exogenous) series — the basis — joined to the coin's bars as-of with strict
no-look-ahead, REVISION-pinned, and explicitly designed as a **generic sidecar carrier**:

> *"The basis arm reuses the `funding_by_symbol`/`funding_map` channel as a generic
> sidecar carrier — the value is the BASIS, not funding, and is consumed ONLY by
> `basis_reversal_score`, NEVER by the `run_path` accrual"* — `basis_data.rs` § D-BR.3.

This is precisely the seam DVOL needs: a daily exogenous series (DVOL) carried through the
existing sidecar channel, consumed ONLY by a new `dvol_regime_score`, and NEVER touching
the funding/accrual path. The reusable parts, verbatim from the basis build:

- **`basis_as_of(series, bar_open_ts_ms) -> Vec<Option<Decimal>>`** — the at-or-before
  as-of join with `None` warm-up (`basis_data.rs:403`). A `dvol_as_of` is a near-identical
  clone (same `PitSeries::as_of_value` core, ADR-0058).
- **The `no_look_ahead_falsifier` unit test** (`basis_data.rs:555`) — future-shift the
  series, assert the join result changes. Clones directly to DVOL.
- **The REVISION-pin loader** (`BasisDataSource::load`, `EXPECTED_BASIS_REVISION_SHA`,
  `RevisionMismatch`) — clones to a `DvolDataSource` against `data/deribit-dvol/`.
- **The fetcher template:** [`crates/data/src/bin/fetch_binance_premium.rs`](../../crates/data/src/bin/fetch_binance_premium.rs)
  (`PremiumFetcher` trait + `HttpPremiumFetcher` + `MockFetcher` + paginator +
  `write_revision_manifest`) is the shape for a new `fetch_deribit_dvol` bin (every
  external I/O behind a trait so tests fake it — CLAUDE.md). The `dvol_diag.rs` spike
  (§ 0 if-budget lane) clones `basis_diag.rs` for the read-only IC pass before any of this.

### 3.2 NOT the multi-symbol cross_sectional path

[`crates/strategy/src/cross_sectional/`](../../crates/strategy/src/cross_sectional/)
consumes multi-symbol bars for a *cross-sectional rank*. With only 2 DVOL symbols, the
cross-section is meaningless (§ 1.4) — so the arm is a **per-symbol time-series regime
filter**, NOT a cross-sectional rank. The cross_sectional `ScoreSource` enum
(`config.rs:56`) is still the registration point (a new `ScoreSource::DvolRegime` variant
feeding the new `dvol_regime_score`), but the *selection mode* is per-symbol long/flat, not
top-k cross-sectional. This mirrors the on-chain stablecoin probe's "honest framing is
time-series, not the 10-name cross-section" conclusion.

### 3.3 The bake-off arm-registration seam

A new bake-off arm is a new `SweepFamily` variant ([`crates/backtest/src/bakeoff/sweep.rs:77`](../../crates/backtest/src/bakeoff/sweep.rs))
that maps to the new `ScoreSource::DvolRegime`. Because the signal is parameter-light (one
pre-registered rule, no grid), the `SweepGrid` for this family is a **single cell** (or a
tiny named sensitivity set the architect may choose to expose as a labeled robustness
check, NOT a crowning sweep). The arm then flows through the IDENTICAL `classify_verdict`
gate and is benchmarked against buy-and-hold like every other arm.

**Net seam read:** ~80% reuse. New code = a `fetch_deribit_dvol` bin, a `DvolDataSource`
(clone of `BasisDataSource`), a `dvol_as_of` (clone of `basis_as_of`), a `dvol_regime_score`
(the one pre-registered rule), a `ScoreSource::DvolRegime` + `SweepFamily` wiring, the
day-1 divergence e2e test (§ 5), and 2 anchored surfaces (BTC, ETH — added, not mutated).

---

## 4. Anchor safety + the frozen gate (unchanged)

- **`write_report=false` → anchor-safe.** The probe runs with report-writing OFF for any
  exploratory pass, so it touches NO `spec/*/reports/` file. The existing **119/119**
  anchors (verified PASS this session, both before and after this scope) stay green. The
  probe ADDS up to 2 new anchored surfaces (BTC-DVOL, ETH-DVOL theta/regime surfaces) at
  build time — **additive, never mutating** an existing anchor (ADR-0038 § D6
  anchor-additive contract).
- **The robustness gate + bands are FROZEN.** `classify_verdict` (the 5-signal
  weakest-link bootstrap) is unchanged; FRAGILE = ineligible to crown; buy-and-hold remains
  the benchmark. The DVOL arm is scored by the **identical** machine and bar that scored
  price, positioning, and on-chain — that identity is the entire point (honest, calibrated
  coverage), and it is what makes a FRAGILE verdict here decision-grade.
- **Existing arms byte-identical.** The DVOL arm is purely additive: a new `SweepFamily` /
  `ScoreSource` variant + a new sidecar series. It does not alter the SMA/MACD/RSI/Bollinger/
  buy-hold arms or their banked surfaces. (Architect to confirm the enum-variant addition is
  non-perturbing to existing arms' serialized output.)

---

## 5. Day-1 baseline-equity-divergence gate (MANDATORY — CLAUDE.md non-negotiable)

Per the CLAUDE.md non-negotiable + the
[`v3-volatility-forecaster-noop-fix` precedent](../dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md):
**every strategy overlay or sizing-modifier ships with a baseline-equity-divergence e2e
test from day 1** — unit tests on the regime math + anchored surfaces are NOT sufficient to
catch a no-op arm where `dvol_t` is computed but never actually flattens the position. This
risk is ACUTE here (a regime filter is exactly the "scale computed but not applied" shape
the v3 vol overlay failed on).

**Required e2e test** (pattern: [`crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`](../../crates/strategy/tests/vol_targeting_overlay_end_to_end.rs)):
the DVOL-regime arm's output equity MUST diverge from the un-filtered buy-and-hold baseline
equity by **≥ 1 bp** (testable epsilon) over a span where the DVOL series is non-trivial
(i.e. contains at least one stress→calm transition that flips the {0,1} weight). Concretely:
construct a fixture where DVOL crosses its 30d median at least once; assert
`|equity_dvol_arm − equity_buyhold| ≥ 1 bp` at the end. If the arm is a no-op (regime
computed, weight never applied), the equities stay identical and the test FAILS — catching
the v3-class bug on day 1.

**Plus the no-look-ahead leak-check** (the on-chain probe's load-bearing gate): a
falsifier that future-shifts the DVOL series and asserts the arm's decisions/equity change
(proves the as-of join is causal — no future DVOL leaks into a past bar). Direct clone of
`basis_data.rs::no_look_ahead_falsifier`, lifted to the arm/equity level.

---

## 6. Reuse-vs-new — explicit ledger

| Component | Reuse / New | Source |
|---|---|---|
| Exogenous as-of join | **REUSE** (clone) | `basis_as_of` / `PitSeries` (`basis_data.rs:403`) |
| No-look-ahead falsifier | **REUSE** (clone) | `basis_data.rs:555` |
| REVISION-pin loader + mismatch guard | **REUSE** (clone) | `BasisDataSource::load`, `EXPECTED_BASIS_REVISION_SHA` |
| Corpus pinning pattern | **REUSE** | `data/defillama-stablecoins/REVISION.toml` (drop-in template) |
| Fetcher (trait + http + mock + paginator + manifest) | **NEW** (template clone) | `fetch_binance_premium.rs` `PremiumFetcher` |
| Diag spike (read-only IC pass) | **NEW** (template clone) | `basis_diag.rs` / `stablecoin_diag.rs` |
| `DvolDataSource` | **NEW** (clone of `BasisDataSource`) | — |
| `dvol_regime_score` (the one signal) | **NEW** (small, parameter-light) | — |
| `ScoreSource::DvolRegime` + `SweepFamily` wiring | **NEW** (enum variant) | `cross_sectional/config.rs:56`, `bakeoff/sweep.rs:77` |
| Frozen `classify_verdict` gate + BH benchmark | **REUSE** (unchanged) | — |
| Day-1 divergence e2e + leak-check | **NEW** (mandatory) | `vol_targeting_overlay_end_to_end.rs` pattern |

**Engineering size (honest):** ~**4–6 dev-days** to a first robustness verdict (fetcher +
PIT-pinned banked BTC+ETH series + `DvolDataSource` + `dvol_regime_score` + `ScoreSource`/
`SweepFamily` wiring + the mandatory day-1 divergence e2e + leak-check + 2 added anchored
surfaces). The **spike-first lane is ~1–2 dev-days** (read-only `dvol_diag.rs` IC pass,
gates the full build). Smaller than on-chain's ~5–8 because the seam reuse is higher (the
basis exogenous-series path is a near-exact fit) and there is exactly ONE parameter-light
signal (no grid, no PIT-relabeling guard to invent — DVOL is clean by construction).

---

## 7. Open decisions for the architect / operator

1. **Spike-first vs full build?** Recommend **spike-first** (`dvol_diag.rs`, ~1–2 days) to
   read the regime signal's cross-year IC / sign-persistence BEFORE the ~4–6-day arm build,
   exactly as the basis and stablecoin spikes did. Gate the full arm on a non-zero,
   sign-stable daily IC. (Operator may greenlight the full build directly if they want the
   anchored coverage surfaces regardless of the spike read — a defensible "coverage is the
   deliverable, build it either way" call. **Recommended (durable): spike-first** — it
   reaches the same place faster if the signal is dead, and catches the upside if alive.)
2. **`W = 30d` lock + median-vs-quantile cut** — M-T1 architect lock. 30d is
   theory-motivated (matches DVOL's own 30-day horizon) and parameter-light; the architect
   confirms the exact cut (median vs e.g. 33rd percentile) and the "not rising sharply"
   clause precision. Whatever is chosen is LOCKED pre-backtest (no search).
3. **Source: Deribit API vs CryptoDataDownload CSV** for the banked corpus. Deribit
   `get_volatility_index_data` reaches back to 2021-04 (full window margin); the free CSV
   mirror is a simpler fetch but starts ~2022-09 (still covers 2023–2024). Recommend
   **Deribit API as primary** (longer history, canonical source) with the CSV mirror as a
   corroboration/fallback. Architect picks; both are free + immutable.
4. **2-symbol surface shape** — confirm the arm registers as a per-symbol time-series
   long/flat (NOT cross-sectional top-k) given the 2-name universe, and that 2 anchored
   surfaces (BTC, ETH) is the right granularity vs 1 pooled surface.
5. **DVOL-as-overlay vs DVOL-as-standalone-arm?** This spec scopes a **standalone arm**
   (DVOL-regime long/flat, benchmarked vs BH) for clean honest coverage. An alternative is
   a DVOL **regime overlay** on the existing crowned arm (flatten the crowned strategy in
   stress regimes). The standalone arm is the cleaner coverage test (does the IV channel
   carry its OWN edge); the overlay is a different, larger experiment. Recommend
   **standalone arm** for this probe; the overlay is a possible follow-on. (Operator decide
   if they specifically want the overlay framing instead.)
6. **Null-result framing on ship.** Confirm that a FRAGILE verdict ships as the
   product.md "we also checked the forward-looking vol channel; it too does not survive →
   buy-and-hold undefeated across FOUR channels (price + positioning + on-chain + options/IV)"
   record — the honest-coverage deliverable, NOT a failure. (This is the expected outcome on
   the LOW prior; the orchestrator lands the product.md/trace.toml reconciliation, not this
   analyst.)

---

## 8. Assumptions & limits (challengeable by architect / operator)

1. **The PIT verdict rests on CONSTRUCTION + multi-mirror corroboration, not a vendor PIT
   guarantee.** Deribit's docs do not *explicitly* state "DVOL history is immutable" (just
   as they don't state the opposite). The FEASIBLE call rests on (a) DVOL being a live
   order-book function with no relabeling substrate — structurally past-only like the basis
   this repo already certified, and (b) three independent free mirrors serving a consistent
   un-disclaimed history. This is materially stronger than the on-chain stablecoin PIT case
   (which rested on forward-recording alone) and CATEGORICALLY stronger than net-flow
   (which the vendor *disclaims*). If an architect uncovers a Deribit revision/restatement
   policy for DVOL, the verdict flips — but nothing found this session suggests one.
2. **The 2-symbol universe is an inherent, unfixable limit** (no liquid altcoin options →
   no altcoin DVOL). It lowers detection power and caps strategy breadth. This is the single
   biggest reason the honest prior is LOW and the expected outcome is FRAGILE. It is not a
   build gap; it is the channel's nature.
3. **The prior on a ROBUST IV-regime edge is LOW-to-MEDIUM, deliberately not inflated.**
   The channel is genuinely orthogonal (forward-looking), but: the program retired a
   realized-vol bet; vol-regime timing is heavily competed and public; and 2 names × ~730
   daily points is thin for the frozen gate's tail percentiles. The probe is justified by
   *bounded coverage-per-dollar toward closing the vol channel honestly*, NOT by optimism.
4. **A FRAGILE result is the success condition for "honest coverage."** Per the mandate, a
   null is the expected, valid, shippable outcome — it closes the vol channel with the same
   finality the on-chain probe closed its channel, removing a "but we never checked options"
   asterisk from the "active ≤ passive in the reachable universe" conclusion. The probe is
   NOT predicated on finding alpha; finding none is a complete deliverable.
5. **No parameter search — one pre-registered signal.** Any sensitivity analysis is a
   separate, explicitly-labeled robustness check, never a crowning argmax. A searched DVOL
   signal would void the honest-coverage claim (it would be the cherry-pick the frozen gate
   was built to prevent).
6. **`write_report=false` for exploration keeps anchors green; the 2 added surfaces are
   additive** (ADR-0038 § D6), built only at the build stage, never mutating an existing
   anchor. 119/119 verified PASS at scope time.

---

## Changelog

- 2026-06-26 (analyst, fresh-channel scope): scoped the options/implied-vol probe as a NEW
  bake-off arm class (Deribit DVOL). **DATA-PIT-FEASIBILITY VERDICT: FEASIBLE** — the crux
  comes out OPPOSITE to on-chain. DVOL is free + no-auth (`public/get_volatility_index_data`,
  under `/public/`; free CryptoDataDownload CSV mirror), daily OHLC, history from 2021-04
  (covers 2023–2024). **PIT-clean by CONSTRUCTION:** DVOL is a deterministic live
  order-book function (variance-swap on the option chain at print time) with NO relabeling
  substrate — the SAME PIT class as the perp basis this repo already certified clean
  (`basis_data.rs`), and categorically unlike CryptoQuant net-flow which the vendor itself
  DISCLAIMS as mutable (the exact thing that killed on-chain). Three independent free
  mirrors (Deribit/CryptoDataDownload/Glassnode/TheBlock/TradingView) serve a consistent
  un-disclaimed history → corroborates immutability. Pinnable via the existing
  REVISION-manifest pattern (`data/defillama-stablecoins/REVISION.toml` is a drop-in
  template) → `data/deribit-dvol/`. SEAM: ~80% REUSE of the proven exogenous-series path
  (`basis_data.rs` `basis_as_of` + no-look-ahead falsifier + REVISION loader; sidecar-carrier
  D-BR.3; `fetch_binance_premium.rs` fetcher template), NOT a new seam and NOT the
  cross_sectional rank (2-symbol universe → per-symbol time-series long/flat). PRE-REGISTERED
  SIGNAL (no search): a DVOL vol-regime long/flat filter — hold when implied vol < trailing
  30d median (calm), step to cash when elevated (stress); W=30d locked (matches DVOL's own
  30-day horizon), self-normalizing median cut (nothing to tune). HONEST FRAMING (load-bearing):
  universe collapses to BTC+ETH only (no liquid altcoin options → no DVOL); program retired
  a realized-vol/GARCH-σ bet → inherits skepticism; prior on a ROBUST edge LOW-to-MEDIUM; a
  FRAGILE/null result is the EXPECTED, valid, shippable outcome (honest coverage, NOT an
  alpha claim). Day-1 baseline-equity-divergence e2e gate MANDATORY (≥1bp vs BH over a
  regime-flip span — catches the v3 vol-overlay no-op class) + no-look-ahead leak-check.
  Anchor-safe (write_report=false; 119/119 verified PASS before+after; ≤2 surfaces added,
  additive). Frozen `classify_verdict` gate + BH benchmark UNCHANGED. Engineering size
  ~4–6 dev-days full / ~1–2 spike-first (`dvol_diag.rs`). RECOMMENDATION: BUILD (spike-first
  lane available; durable=spike-first). NO product code, NO git, NO trace.toml/product.md
  touched (orchestrator reconciles; sibling analyst scoping in parallel). status=proposed,
  owner=analyst, v0.1.0.
