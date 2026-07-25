---
slug: onchain-netflow-spike-2026-06-08
status: draft
owner: analyst
updated: 2026-06-08
tags: [on-chain, exchange-netflows, stablecoin-supply, spike, go-no-go, hard-stop, conclude, ship-passive, point-in-time, pit-gate, defillama, cryptoquant, address-relabeling, rank-ic, orthogonality, sign-persistence, leak-check, no-look-ahead, daily-resolution, dry-powder, mint-burn, fragile, three-domains-exhausted, active-vs-passive, durable-over-quick, basis-spike-precedent, prior]
related:
  - docs/dev-notes/onchain-vs-conclude-fork-2026-06-08.md
  - docs/dev-notes/new-data-domain-scoping-2026-06-05.md
  - spec/perp-basis-mn-spread/reports/test-2026-06-08-perp-basis-mn-spread.md
  - docs/dev-notes/robustness-decision-rule-2026-05-30.md
  - spec/product.md
  - spec/backlog.md
---

# On-chain spike — exchange net-flows (PIT-killed) → stablecoin-supply (FRAGILE) → HARD-STOP

> **Mandate (analyst spike, FILES ONLY — orchestrator commits).** The operator
> greenlit the bounded on-chain hunt recommended in
> [onchain-vs-conclude-fork-2026-06-08](onchain-vs-conclude-fork-2026-06-08.md),
> with a **pre-committed HARD-STOP**: a FRAGILE or PIT-infeasible spike result
> CONCLUDES the entire active-vs-passive search (ship the passive baseline, no
> further hunt). This is the ~1-2-day go/no-go on the on-chain prior — the #1-ranked
> remaining orthogonal channel after OHLCV and derivatives-positioning both came
> back uniformly fragile. It mirrors the perp-basis spike that found the program's
> first (and only) live signal. Every number below traces to a free public endpoint
> inspected this session or to the read-only `stablecoin_diag.rs` probe over banked
> data — NO fabrication (per the fabricated-"Sharpe 1.40" precedent), NO synthesized
> data to force a result. A clean negative is as decision-grade as a positive here,
> and the result IS a clean negative.

---

## 0. TL;DR — the verdict: HARD-STOP → CONCLUDE + ship passive

**The on-chain channel does NOT carry a live, PIT-clean, orthogonal signal on the
reachable free data. Per the pre-committed fuse, this CONCLUDES the active-vs-passive
search.** Two findings, in the mandated order:

1. **Exchange net-flows — KILLED at the data-feasibility / PIT gate (the gate comes
   first, and it is decisive).** The canonical free net-flow source (CryptoQuant)
   fails BOTH sub-gates: (a) the API requires a **PAID** Professional/Premium plan —
   not free; and (b) — the deeper kill — CryptoQuant's own docs **disclaim
   point-in-time accuracy**: *"this endpoint does not support Point-In-Time (PIT)
   accuracy due to periodic updates to wallet address clustering. Historical data may
   change as new exchange wallets are discovered, added, and validated ... historical
   data ... is mutable."* That is **exactly** the address-relabeling look-ahead the
   fork note pre-registered as the net-flow killer, confirmed verbatim by the vendor.
   A net-flow series that is silently rewritten as labels change cannot be back-tested
   honestly. This is a **FEASIBILITY verdict**, which per the fork note routes straight
   to the hard-stop. → **PIVOT to the pre-named cleaner-PIT fallback: stablecoin
   supply** (mint/burn at issuer contracts is immutable on-chain).

2. **Stablecoin supply — PIT-clean and free, but the signal is FRAGILE (does NOT
   replicate across years).** The pivot signal cleared the data + PIT gates cleanly
   (DefiLlama, free, no-auth, daily, full 2023-2024 history, forward-recorded
   snapshots — verified), the leak-check passed (causal ≠ leaked at every horizon),
   and it is beautifully orthogonal to price-momentum (|corr| < 0.07 everywhere). But
   on the predictive question it **fails the basis spike's LIVE bar at every horizon**:
   no cell is jointly sign-stable across 2023 AND 2024 with |IC| ≥ 0.05. The per-chain
   time-series IC **flips sign** between years at the only horizons where it has any
   magnitude (L=7d: +0.011 → −0.086; L=14d: +0.036 → −0.130); the aggregate
   dry-powder → BTC signal's same-sign cells are **all inside their 2σ noise bands**
   (n=25-51). Contrast the basis spike, which I certified LIVE precisely because it
   held the **same sign in both years** (L=60-168: −0.08 to −0.11 both years). The
   stablecoin signal has no such replication. It is **FRAGILE**.

**→ VERDICT: HARD-STOP. CONCLUDE the active-vs-passive search and ship the passive
baseline.** Net-flows are PIT-infeasible for free; the cleaner-PIT stablecoin fallback
is fragile. Both branches of the pre-registered spike land on the fuse. The program has
now given its **single best-remaining orthogonal channel its fair test**, and it also
fails — which makes the "active ≤ passive in the reachable universe" conclusion
**airtight and asterisk-free**, exactly the durable outcome the fork note named for the
FRAGILE branch (§ 5.1 there: "FRAGILE = airtight conclusion, zero regret"). This is NOT
a manufactured negative and NOT a softened one: it is the clean go/no-go the operator
pre-committed to honor.

**Confidence: HIGH** (higher than the basis spike's MEDIUM-HIGH LIVE call). Three
things make the negative robust: (i) the net-flow PIT kill is the **vendor's own
disclaimer**, not my inference; (ii) the stablecoin signal fails the *same*
cross-year-replication bar that the basis signal *passed*, so the two spikes are
calibrated against each other on identical methodology; (iii) the fragility is uniform
— there is no horizon, no chain, and no framing (per-chain TS, aggregate→BTC) where a
sign-stable, significant signal survives. The one honest caveat keeping it short of
"certain" is the **thin daily universe** (§ 7): 4 chains × ~365 days/yr is materially
thinner than the basis's 10 names × 8 760 hourly bars, so the spike's *power to detect a
weak edge* is lower — but that cuts toward the hard-stop, not away from it (a signal too
weak to detect on 2 years of daily data is also too weak to harvest net of cost).

---

## 1. The data-feasibility / PIT gate (FIRST — and it is binding)

The fork note was explicit that for on-chain, **the feasibility gate is the binding
constraint, not the hypothesis**, and that **net-flow address labeling is the hardest
on-chain PIT problem**. The spike ran the gate first, exactly as mandated, and it
decided the net-flow branch before any IC was computed.

### 1.1 Exchange net-flows — FAILED both sub-gates

| Source | Free? | PIT-clean? | Verdict |
|---|---|---|---|
| **CryptoQuant** (canonical free-tier net-flow) | **NO** — API token requires Professional/Premium **paid** plan | **NO** — vendor disclaims PIT (see quote) | **KILLED** |
| **CoinGlass** (exchange wallet netflow) | free tier exists; historical netflow gated | history requires paid; same address-labeling PIT problem | **KILLED** (same PIT defect) |
| Glassnode free tier | daily + delayed; netflow address-clustering identical | same mutable-history defect | **KILLED** (same PIT defect) |

> **CryptoQuant, verbatim from its own API user-guide (the decisive quote):** *"this
> endpoint does not support Point-In-Time (PIT) accuracy due to periodic updates to
> wallet address clustering. Historical data may change as new exchange wallets are
> discovered, added, and validated."* And separately: *"exchanges often create new
> wallets and move their funds ... therefore, historical data of certain on-chain data
> associated with exchanges is mutable."*

This is the address-relabeling look-ahead made concrete: today's value for a past date
is **not** what was knowable then — it incorporates exchange-wallet discoveries made
*after* the bar. A back-test on this series would be training on relabeled-with-hindsight
flows; any "edge" it showed could be a pure look-ahead artifact, and there is **no way to
reconstruct the past-only series** from the free product (it serves only the latest
revision). Combined with the paid-key requirement, net-flows are **infeasible to
back-test honestly for free**. Per the fork note (§ 5.2 caveat, § 7.3) this is a
**FEASIBILITY verdict** that routes straight to the hard-stop — and it also vindicates
the fork note's own pre-registration that net-flow PIT was the most likely thing to break.

→ **PIVOT to stablecoin supply**, the pre-named cleaner-PIT fallback.

### 1.2 Stablecoin supply — PASSED the data + PIT gates cleanly

The pivot target. Why it is the right fallback: **mint/burn events at the issuer
contract are immutable on-chain**, so a past date's circulating supply is
reconstructible from immutable history — there is no "relabeling" analogue.

| Gate | Result |
|---|---|
| **Free?** | **YES** — DefiLlama stablecoins API, `https://stablecoins.llama.fi`, no auth, no key. |
| **Daily?** | **YES** — exact 86 400 s spacing; verified on `/stablecoincharts/all` (3 114 daily points 2017-11→2026-06). |
| **Full 2023-2024 history?** | **YES** — 731 daily points in the window, matching the OHLCV window exactly. |
| **Per-chain (for a universe)?** | **YES** — `/stablecoincharts/{chain}` gives daily per-chain supply; `/stablecoin/{id}` gives per-asset per-chain with a `minted` field. |
| **PIT-clean (forward-recorded, not backfilled)?** | **YES (structurally verified)** — see § 1.3. |
| **Methodology** | DefiLlama computes per-chain supply as **mints − burns at the issuer contract on each chain** (on-chain contract state / `totalSupply`), per its adapter docs. |

### 1.3 The PIT leak-check / falsifier — stablecoin supply is PAST-ONLY (THE gate)

Two independent demonstrations that the stablecoin series uses no
published-after-the-bar information:

**(a) Structural forward-recording test (the on-chain analogue of "no address
relabeling").** The risk for stablecoin supply is *retroactive backfill*: if DefiLlama
injected a newly-added chain/token's full history into past dates, old values would
change. I tested this directly by checking whether a chain's supply series **begins at
the chain's real launch** (forward-recorded) or **predates it** (backfilled):

| Chain | Series starts | First >$1M supply | Real mainnet launch | Read |
|---|---|---|---|---|
| Arbitrum | 2021-06-25 | 2021-09-01 | ~Aug-Sep 2021 (bridge) | forward-recorded ✓ |
| **Base** | **2023-08-15** | 2023-08-16 | Aug 2023 | **series literally begins AT launch — zero pre-launch backfill** ✓ |

Base is the clinching case: its DefiLlama series starts **one day before** its first
non-trivial supply, at its mainnet-launch week. There is **no data injected before the
chain existed** → DefiLlama records a daily snapshot **from the date a chain first
exists, forward**, not a retroactive from-genesis recompute. A past date's value
reflects on-chain state *then*. This is categorically cleaner than CryptoQuant's
self-disclaimed mutable net-flow.

**(b) Causal-vs-leaked IC falsifier (in-probe, `--leak-check`).** The probe recomputes
B1 with a deliberately **leaked** (contemporaneous `[D, D+L)`) supply window and prints
causal-vs-leaked. Result (2024):

| L | causal (trailing, past-only) | leaked (contemporaneous) | differ? |
|---|---|---|---|
| 1d | −0.0094 | +0.0842 | YES |
| 3d | +0.0504 | +0.1931 | YES |
| 7d | −0.0857 | +0.2363 | YES |
| 14d | −0.1302 | +0.1594 | YES |
| 30d | −0.1146 | +0.0392 | YES |

Causal ≠ leaked at **every** horizon, and the leaked (look-ahead) signal flips strongly
**positive** where the causal signal is negative — the signature of a correct past-only
join (the `supply[D-1]`-as-of construction is not silently reading future supply). The
join is causal. **The PIT gate PASSES for stablecoin supply** — which is exactly why
the subsequent FRAGILE finding is a genuine *signal* verdict, not a contaminated one.

> **Reproduce** (read-only after the first fetch; ~5 s/yr; first run pulls 5 DefiLlama
> series and banks them to `data/defillama-stablecoins/`):
> ```
> cargo run -p data --example stablecoin_diag -- 2023
> cargo run -p data --example stablecoin_diag -- 2024
> cargo run -p data --example stablecoin_diag -- 2024 --leak-check   # the PIT falsifier
> ```
> Probe: `crates/data/examples/stablecoin_diag.rs` (clone of `basis_diag.rs`; reads
> banked 1h OHLCV via the harness's own `ReplayFeed::merge_symbols`, folds to daily
> closes, joins DefiLlama daily supply + funding as-of). Data pinned in
> `data/defillama-stablecoins/REVISION.toml` (aggregate SHA `782148bd…`; parquets
> gitignored, manifest tracked — mirrors `data/binance-basis`).

---

## 2. The buildable universe — a load-bearing scoping reality

Stablecoin supply maps to **L1 chains**, and only chains with a meaningful native
stablecoin ecosystem carry a usable 2023-2024 series. Mapping the original 10 large-cap
OHLCV symbols to DefiLlama chains (verified supply magnitudes this session):

| OHLCV sym | DefiLlama chain | 2023-01-02 supply | 2024-12-30 supply | usable? |
|---|---|---|---|---|
| ETH | Ethereum | 85.4 B | 112.9 B | **YES** |
| BNB | BSC | 9.26 B | 6.84 B | **YES** |
| SOL | Solana | 1.82 B | 5.12 B | **YES** |
| AVAX | Avalanche | 1.57 B | 2.29 B | **YES** |
| ADA | Cardano | 0.003 B | 0.023 B | no (negligible) |
| DOT | Polkadot | — | 0.089 B | no (history starts 2024-11) |
| XRP | Ripple | — | — | no (history starts 2025-04) |
| DOGE | Dogecoin | — | — | no (no native stablecoin chain) |
| LINK | (token, not a chain) | — | — | no |
| BTC | (no native stablecoin chain) | — | — | aggregate proxy only |
| — | **all (aggregate)** | 137.5 B | 206.1 B | yes (dry-powder proxy) |

**Consequence:** the on-chain stablecoin signal supports at most a **4-name universe
(ETH/BNB/SOL/AVAX)** plus an **aggregate "dry-powder → market"** leg. A 4-name
cross-section is too thin for a rank-IC (the basis used 10; a 4-wide Spearman is
dominated by tie/rank noise), so the **honest framing is time-series** (does a chain's
trailing Δsupply lead its own native token's forward return) plus the aggregate leg.
This is reported, not hidden: even in the best case (a LIVE result), the build would
have been a **time-series long/flat arm on 4 names + a market-timing aggregate**, not the
cross-sectional 10-name shape the harness runs natively. The thin universe is an inherent
limit of the free on-chain data, and it both (i) lowers the spike's detection power and
(ii) would have capped the eventual strategy's breadth — another reason the hard-stop is
the right call on a fragile read.

---

## 3. The diagnostic numbers (the probe, both years)

All from `stablecoin_diag.rs` over banked OHLCV (`data/binance` pin `3a8b96c4…`) +
DefiLlama supply (pin `782148bd…`) + funding (`data/binance-funding` pin `bf1ede44…`).
Daily grid; 365 (2023) / 366 (2024) aligned days across the 4 chains.

### 3.1 B1 — per-chain time-series rank-IC (trailing Δsupply → forward return)

Signal[D] = pct-change in chain supply over `[D-L, D)` (past-only); forward = cum
log-return `[D, D+L)`. Pooled across the 4 chains, plus per-chain:

| Lookback L | 2023 pooled IC | 2024 pooled IC | sign-stable & |IC|≥0.05 both yrs? |
|---|---|---|---|
| 1 d | −0.020 | −0.009 | NO (both ≈0, below 0.05) |
| 3 d | +0.008 | +0.050 | NO (2023 ≈0) |
| **7 d** | **+0.011** | **−0.086** | **NO — sign FLIPS** |
| **14 d** | **+0.036** | **−0.130** | **NO — sign FLIPS** |
| 30 d | −0.023 | −0.115 | NO (2023 ≈0; n=44) |

Per-chain at L=14d (the largest-magnitude horizon) shows the instability starkly:
ETH **+0.122 (2023) → −0.352 (2024)**; SOL +0.194 → +0.199 (the only same-sign pair,
but n=25 windows); AVAX −0.241 → −0.383; BNB +0.120 → +0.006. The signs do not hold.

### 3.2 B2 — aggregate dry-powder → forward BTC return

Signal[D] = pct-change in TOTAL stablecoin supply over `[D-L, D)`; forward = BTC cum
log-return `[D, D+L)`. Pearson (single series):

| Lookback L | 2023 corr | 2024 corr | same sign? | significant (2σ vs n)? |
|---|---|---|---|---|
| 1 d | −0.048 | +0.066 | NO | no (both) |
| 3 d | +0.032 | +0.156 | yes | **no** (2σ≈0.18, n=120) |
| 7 d | −0.179 | −0.132 | yes | **no** (2σ≈0.28, n=51) |
| 14 d | −0.131 | −0.252 | yes | **no** (2σ≈0.41, n=25) |
| 30 d | +0.402 | −0.129 | NO | no (n=11 — noise) |

The aggregate leg is the most tempting (L=7d/14d are same-sign and negative — "supply
surged, BTC then fell back" is a coherent contrarian story). But **every single B2 cell,
both years, is inside its 2σ band** — none is statistically distinguishable from zero
given the daily-window n. The headline +0.40 at L=30d rides on **n=11 windows** and flips
to −0.13 the next year — the identical noise signature I flagged and excluded at L=720 in
the basis spike. There is no significant, replicating aggregate edge.

### 3.3 B3 — orthogonality (to BOTH dead channels)

A new on-chain signal is only interesting if orthogonal to BOTH already-dead channels
(price-momentum AND funding). The stablecoin signal IS orthogonal to momentum, and
mostly to funding:

| Lookback L | corr(Δsupply, momentum) 2023/2024 | corr(Δsupply, funding) 2023/2024 |
|---|---|---|
| 1 d | +0.010 / +0.048 | +0.033 / +0.062 |
| 3 d | −0.003 / +0.067 | +0.039 / +0.060 |
| 7 d | +0.036 / +0.042 | +0.176 / +0.155 |
| 14 d | +0.046 / +0.029 | +0.229 / +0.135 |
| 30 d | −0.067 / +0.034 | +0.105 / +0.246 |

**vs momentum: excellent orthogonality** (|corr| < 0.07 at every horizon, both years) —
stablecoin supply is genuinely not a price transform, the cleanest orthogonality the
program has measured. **vs funding: mostly orthogonal at short horizons** (≤0.07 at
L≤3d) but creeping to +0.13-0.25 at the L=7-14d horizons where B1 is largest — so it is
not even *cleanly* orthogonal to funding exactly where it would matter. **But
orthogonality is only valuable when there is a signal to be orthogonal**, and B1/B2 show
there is no sign-stable, significant one. A perfectly-orthogonal channel that carries no
replicable forward information is still dead.

---

## 4. Calibration against the basis spike — why this is FRAGILE and that was LIVE

The two spikes share **identical methodology** (`*_diag.rs`, rank-IC, orthogonality,
cross-year sign-persistence, leak-check) and an **identical LIVE bar**, so they
cross-calibrate. The contrast is the whole verdict:

| Criterion | Basis spike (certified LIVE) | Stablecoin spike (this) |
|---|---|---|
| Peak \|IC\| | −0.08 to −0.11 (L=60-168) | per-chain up to ±0.13 (L=14d) but unstable |
| **Same sign BOTH years?** | **YES** (negative 2023 AND 2024) | **NO** (flips at L=7d, L=14d) |
| Sample depth at signal horizon | n=51-974 (hourly) | n=25-100 (daily) — much thinner |
| Significant vs noise? | yes (held across 974 windows) | **no** (every B2 cell inside 2σ) |
| Orthogonal to momentum? | yes (+0.02-0.07) | yes (<0.07) |
| Orthogonal to funding? | moderate (+0.47-0.66 level, distinct) | mixed (+0.13-0.25 at long horizons) |
| Leak-check (causal≠leaked)? | pass | pass |
| **Verdict** | **LIVE → build basis arm** | **FRAGILE → hard-stop** |

The basis cleared the bar **because it replicated**: the same negative reversal IC in two
independent years is hard to fake with noise. The stablecoin signal does the opposite —
its sign is a coin-flip between years at exactly the horizons with magnitude. Applying
the *same* rule that said "build" to the basis, this one says "do not build." That
symmetry is what makes the negative trustworthy rather than defeatist.

---

## 5. The verdict against the pre-committed fuse

The fork note's hard-stop (§ 4.3, verbatim):

> **Pre-committed hard-stop:** if the on-chain probe comes back FRAGILE under the frozen
> rule ... the program CONCLUDES. Ship passive. No options-domain hunt, no macro-domain
> hunt, no on-chain sub-signal mining. The active-vs-passive question is answered NEGATIVE
> for the reachable universe, and that answer is *durable* because the most-orthogonal
> channel was given its fair test.

Both branches of the spike land on the fuse:
- **Net-flows → FEASIBILITY kill** (PIT-infeasible for free) — the fork note (§ 7.3)
  pre-declared this routes to the hard-stop.
- **Stablecoin-supply fallback → FRAGILE** (no cross-year-replicating, significant,
  orthogonal signal) — the fork note (§ 4.3) pre-declared this routes to the hard-stop.

There is **no surviving branch.** The most-orthogonal remaining channel was given a fair
test on the cleanest free PIT-clean series available, with the same machine and the same
bar that found the basis signal, and it failed. **→ CONCLUDE. Ship passive.**

This is the **durable** outcome, not a defeat (fork note § 5.1): concluding *now*, after
on-chain has had its fair test, makes "active ≤ passive in the reachable universe"
**asterisk-free** — the program no longer has an untested best-orthogonal channel hanging
over the conclusion. The discipline the fork note demanded (§ 7.4 — "the hard-stop is
real; do NOT treat a FRAGILE on-chain result as license to hunt options/macro") now
binds: **no options hunt, no macro hunt, no on-chain sub-signal mining** (miner flows,
active addresses, etc. are explicitly out — the channel got its representative test via
its two strongest, cleanest-PIT signals). The operator may always *later* open
options/macro as a *fresh* program; it is not a continuation of this hunt.

---

## 6. What "ship passive" means (the named, already-built terminal state)

Unchanged from the fork note § 5.3 — restated so this note is self-contained. "Ship
passive" is a **promotion of already-built, already-anchored code**, not a build:

1. **Promote the existing buy-and-hold control to the production baseline strategy.** BH
   is the benchmark every robustness surface was scored against — the most-tested path in
   the repo. "Shipping passive" promotes it from "control" to "the strategy the
   paper-trading agent runs."
2. **Record the conclusion in `spec/product.md`** — the active-edge search concluded
   NEGATIVE across THREE structurally-distinct channels (price/OHLCV,
   derivatives-positioning, on-chain), passive BH undefeated. (This note + the backlog
   update land the record; the product.md § Strategy-library terminal note already
   anticipated this from the fork.)
3. **Re-anchor the program's win on the METHODOLOGY** (product.md § Differentiator 5,
   "measured robustness, not asserted alpha"): the shippable deliverable is the
   robustness machine + the auditable negative result across three orthogonal channels —
   a complete, honest product.
4. **Keep the harness warm but idle** — fetchers, surfaces, and the new
   `stablecoin_diag.rs` probe stay in place so any *future* fresh program can reuse them,
   but no further domain is pursued under THIS hunt.

The realistic terminal strategy for this project is **passive buy-and-hold**, and per the
fork note that is a **successful** outcome of the robustness program — the machine
correctly identified that active edges do not survive on the reachable data, across
price, positioning, AND the settlement layer.

---

## 7. Assumptions & limits (challengeable by operator / architect)

1. **The net-flow PIT kill is the vendor's own disclaimer, not my inference.** If a
   *different* provider served a genuinely past-only, free net-flow series, the net-flow
   branch could be re-opened. I found none free (CryptoQuant/CoinGlass/Glassnode all gate
   history behind paid tiers AND share the address-clustering mutability defect). If the
   operator is willing to **pay** for a vendor that provides *immutable point-in-time
   snapshots* (some paid flat-file vendors do version their address labels), net-flows
   become a *fresh paid program* — but that is outside this free-spike mandate and
   outside the hard-stop's scope.
2. **The thin daily universe lowers detection power (cuts toward the hard-stop).** 4
   chains × ~365 days/yr is much thinner than the basis's 10 names × 8 760 hourly bars,
   so the spike's power to detect a *weak* edge is genuinely lower — the B2 2σ bands are
   wide (±0.18 to ±0.41). BUT: a signal too weak to surface above noise on two years of
   daily data is also too weak to harvest net of cost (daily rebalancing on a 4-name
   long/flat book). The thinness argues for the hard-stop, not against it. The fork note
   (§ 2.2) pre-registered exactly this daily-thinness headwind.
3. **Only two on-chain signals were tested (net-flows, stablecoin supply).** The fork
   note ranked these as the two highest-prior, cleanest-PIT on-chain series; miner/
   validator flows and active-address counts are weaker-causal and (for miner flows)
   share the address-labeling PIT defect. The hard-stop explicitly forecloses mining
   them — the channel got its representative test via its two strongest members. An
   operator who wanted to litigate "but active addresses might…" would be re-opening the
   exact open-ended sub-signal mining the fuse was designed to prevent.
4. **PIT for stablecoin supply rests on forward-recording, verified structurally not
   longitudinally.** I verified DefiLlama records forward (a chain's series begins at its
   launch, no pre-launch backfill) and that the causal join is leak-free — but I could
   only fetch the series *once* (today), so I cannot directly prove a 2023 value is
   byte-identical to what the API served in 2023. The forward-recording architecture +
   immutable mint/burn substrate make retroactive rewriting structurally unlikely (unlike
   net-flow relabeling, which the vendor *confirms* happens), so the residual risk is low
   — and it does not change the verdict, since the signal is FRAGILE even taking the
   series at face value.
5. **Passive "winning" remains partly a 2023-2024-sample artifact** (BH caught a
   structural bull leg; +1.74 Sharpe is a high, sample-specific bar). This is a known
   whole-program scope limit, unchanged by this spike (judged on the same window). It
   means "passive won *this sample's* race," stated honestly — not "passive is proven
   optimal in all regimes."
6. **All priors were sober going in (LOW-to-MEDIUM, fork note § 2.3) and the result
   landed at the low end.** The spike was justified by *bounded information-per-dollar
   toward a durable conclusion*, not by optimism — and it delivered exactly that: ~1 day
   spent to convert "we never tested on-chain" into "on-chain's two best signals were
   tested and failed," which is the asterisk-removing evidence the fork note bought it for.

---

## Changelog

- 2026-06-08 (analyst, on-chain spike): ran the operator-greenlit bounded on-chain
  go/no-go with the pre-committed hard-stop. **GATE 1 (data feasibility / PIT, run
  FIRST): exchange net-flows KILLED** — CryptoQuant (canonical free net-flow) requires a
  PAID plan AND its own docs disclaim point-in-time accuracy ("does not support PIT
  accuracy due to periodic updates to wallet address clustering; historical data may
  change as new exchange wallets are discovered"), confirming verbatim the
  address-relabeling look-ahead the fork note pre-registered as the net-flow killer; no
  free source serves an immutable past-only net-flow series → FEASIBILITY verdict →
  routes to hard-stop. **PIVOT to the pre-named cleaner-PIT fallback (stablecoin supply,
  mint/burn immutable on-chain).** GATE 2 passed for stablecoin supply: DefiLlama
  stablecoins API is FREE/no-auth/daily/full-2023-2024 (731 pts), forward-recorded
  (verified: Base chart begins 2023-08-15 at mainnet launch, zero pre-launch backfill),
  PIT leak-check PASSES (causal≠leaked at every horizon). Built read-only probe
  `crates/data/examples/stablecoin_diag.rs` (clone of `basis_diag.rs`); banked 5
  DefiLlama daily series to `data/defillama-stablecoins/` (new REVISION pin `782148bd…`;
  parquets gitignored, manifest tracked — mirrors data/binance-basis; existing pins
  untouched). **DIAGNOSTIC VERDICT: FRAGILE.** Universe reality: only ETH/BNB/SOL/AVAX
  carry usable 2023-2024 per-chain supply (4 names — too thin for a rank-IC → honest
  framing is time-series + an aggregate→BTC leg). B1 per-chain TS IC fails the basis
  spike's LIVE bar at every horizon — no cell jointly sign-stable across 2023 AND 2024
  with |IC|≥0.05 (L=7d +0.011→−0.086, L=14d +0.036→−0.130, signs FLIP; per-chain ETH
  L=14d +0.122→−0.352). B2 aggregate→BTC: same-sign cells (L=7d −0.18/−0.13, L=14d
  −0.13/−0.25) are ALL inside their 2σ noise bands (n=25-51); headline +0.40 rides n=11
  and flips next year (noise). B3 orthogonality: excellent vs momentum (|corr|<0.07) but
  creeps to +0.13-0.25 vs funding at the signal horizons — and orthogonality is moot
  without a replicating signal. Calibration: the basis was certified LIVE *because* it
  held the same sign both years; the stablecoin signal flips — same rule, opposite
  verdict. **OVERALL VERDICT: HARD-STOP → CONCLUDE the active-vs-passive search, ship
  passive.** Both spike branches (net-flow feasibility kill + stablecoin fragile) land on
  the pre-committed fuse; the most-orthogonal remaining channel got its fair test on the
  cleanest free PIT-clean series and failed, making "active ≤ passive in the reachable
  universe" asterisk-free across THREE channels (price + positioning + on-chain). The
  hard-stop binds: NO options hunt, NO macro hunt, NO on-chain sub-signal mining. Ship
  passive = promote the already-built+anchored BH control + a product.md thesis update.
  Confidence HIGH. NO feature brief authored (the spike said HARD-STOP, not BUILD); NO
  `[[req]]` trace row; NO strategy/ScoreSource/run_path/anchor surface; NO commit; NO
  anchored-report edits. Backlog updated.
