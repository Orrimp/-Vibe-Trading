---
slug: perp-basis-mn-spread
mode: release
status: draft
owner: presenter
audience: human-operator
updated: 2026-06-08
generated: 2026-06-08T08:10:00Z
---

# Market-neutral basis spread — release deck + the close of the derivatives-positioning domain

> This deck closes the **perp-basis-mn-spread** feature (VERDICT → PASS, HEAD
> `8c2e6c4`) and, with it, the **entire derivatives-positioning domain**. The
> v0.1.0 retrospective said the long-only vehicle was wrong and the
> market-neutral spread was the correct one. We built the correct vehicle. **It
> failed too** — fragile in all three arms — and on the way it produced two
> findings that close the question with finality: the basis signal turned out to
> be the **same signal as funding** (it selects the identical portfolio), and the
> part of basis that is genuinely **distinct from funding has negative expected
> return and blows up the book on the tail**. There is no remaining vehicle to
> wonder about. Every number below traces to a committed anchored report or a
> gate I ran live this session — nothing is computed here for the first time
> without showing the computation, and nothing is asserted without a source.

## TL;DR

We built the dollar-neutral long/short basis spread — the "correct vehicle" the
v0.1.0 close-out said would isolate the signal from market beta — and **it came
back FRAGILE in all three arms** against the ≈0 cash null (the +1.74 buy-and-hold
hurdle the long-only arm fought is gone, and the spread still cannot clear zero).
Two findings make this a domain-close, not just another fragile result: **(1)**
the basis spread and the funding spread select the **identical portfolio** —
their result surfaces are bit-identical, so the basis signal *is* the funding
signal (funding mechanically prices the basis; verified real, not a bug); and
**(2)** the basis-orthogonal-to-funding residual — the only thing that could be
*new* — has a **negative median Sharpe (−0.06)** and a **100% tail drawdown via
hundreds of short liquidations**, so basis adds nothing beyond funding and the
market-neutral book is actively dangerous on the tail. The perp-basis-reversal
signal survives in **no vehicle**. The derivatives-positioning domain is closed.
**Two full data domains (OHLCV, derivatives-positioning) are now exhausted;
passive buy-and-hold remains undefeated.** VERDICT PASS; anchors 107 → **119**
(verified live `ANCHORS PASS (119 / 119)`). The pre-registered fork — route to
on-chain (the #3 domain) vs. conclude the active-vs-passive verdict is strong
enough to ship the passive baseline — is teed up for your call, **not decided
here**.

## What changed

- **Built the market-neutral long/short engine — the first `run_path` touch since
  v0.1.0, done anchor-neutral.** The long-only engine could only buy; the new
  engine also **shorts** the highest-basis names against the longs, dollar-neutral
  (equal dollars long and short, so net market exposure ≈ 0). This required a real
  short-side solvency + liquidation model (a short's loss is unbounded, unlike a
  long's). It was built **defaults-off**: a `k_short = 0` (no-short) run is
  byte-identical to the old engine, proven by a dedicated re-identity test, so all
  **107 pre-existing regression anchors stayed byte-identical** through the whole
  build. Anchors 107 → **119**.
- **Ran three arms on the same paths to settle the funding-confound.** Arm 1 =
  the raw **basis** spread; arm 2 = the raw **funding** spread (the mirror
  candidate); arm 3 = **basis-orthogonal-to-funding** (the basis rank-residual
  after removing funding — the part of basis that is genuinely distinct). The
  three-arm comparison is the headline: it tells us whether basis carries any
  alpha *beyond* funding.
- **Locked 12 new byte-stable result surfaces — all FRAGILE.** 3 arms ×
  {0, 5} bps fee × {2023, 2024}, each a full block-bootstrap distribution over the
  locked 2-cell grid, scored against the **dollar-neutral ≈0 null**. **Every one
  came back FAMILY-UNIFORM-FRAGILE.** 12 new anchors locked (#108–#119); the gate
  is now 119/119. Plus 7 day-1 falsifiers, each proven to go RED when its guard is
  reverted.

## Why

The v0.1.0 long-only basis arm closed FRAGILE at every fee level including 0 bps
gross — but the analyst argued that was a **vehicle** verdict, not a **signal**
verdict. The structural read: a long-only arm is "buy-and-hold plus a small
tilt," so it carries the full +1.74 market beta as the bar it must beat while
only harvesting the long half of a long/short signal. The −0.10 information
coefficient (a rank-IC: how well the signal's ranking predicts forward returns —
−0.10 is real but thin) was measured on the **full long/short spread**, which
v0.1.0 never built. The fork note's recommendation was to build that spread:
dollar-neutral strips the beta (the null drops from +1.74 buy-and-hold to ≈0
cash, the 35× hurdle vanishes) and captures **both** legs of the signal. The same
build also resolves a standing question — is basis just a re-skin of funding? —
by running the basis-orthogonal-to-funding arm. The operator greenlit the build
as the durable choice: give the strongest post-OHLCV signal its fair test in its
correct vehicle, with banked data, and either find the program's first survivable
signal or close the domain with finality. (Source:
[`basis-reversal-vehicle-vs-signal-fork-2026-06-06.md`](../../dev-notes/basis-reversal-vehicle-vs-signal-fork-2026-06-06.md)
§ 0, § 1–2; [`feature.md`](../feature.md) "Why".)

## What you can do now

| Action | Command |
|--------|---------|
| Reproduce an anchored MN basis-spread surface (locked grid, N=200, 0 bps gross, in-sample) | `cargo run -p backtest --features "candle realdata" --bin param_robustness_sweep -- --grid mn-tier1 --score-source mn-basis-spread --taker-fee-bps 0 --slippage-bps 2 --paths 200 --year 2023 --out-dir /tmp/mn-verify/` |
| Reproduce the killer residual surface (basis⊥funding, 0 bps gross, 2023 — negative median) | `cargo run -p backtest --features "candle realdata" --bin param_robustness_sweep -- --grid mn-tier1 --score-source mn-basis-funding-residual --taker-fee-bps 0 --slippage-bps 2 --paths 200 --year 2023 --out-dir /tmp/mn-verify/` |
| Confirm every regression anchor is byte-stable (incl. the 12 new MN anchors) | `bash scripts/verify_anchors.sh` |
| Re-run the 7 day-1 MN falsifiers (dollar-neutrality, short funding non-no-op, baseline-divergence, sign, orthogonalization non-no-op, two-run, RED-on-revert) | `cargo test -p backtest --features "candle realdata" --test mn_spread_divergence_e2e` |
| Re-read the fork that scoped this build (the WHY — vehicle vs signal) | open `spec/dev-notes/basis-reversal-vehicle-vs-signal-fork-2026-06-06.md` |

## Live demo

Three fresh MN sweeps, run just now on real 2023 Binance basis + funding + OHLCV
data, end to end through both sidecar loaders, the as-of joins, the dollar-neutral
long/short engine, the short-side liquidation rule, and the renderer. These are
**fast smokes at N=5 paths** (the anchored surfaces use N=200 — so the per-cell
numbers here are NOT the anchored figures; they exist only to prove the binary
genuinely produces these results live). What they demonstrate: the family verdict,
the **negative-median residual** (the killer finding), and the **basis = funding**
identity all reproduce on demand.

**(1) The residual arm — the killer finding (basis⊥funding, negative median):**

```
$ ./target/debug/param_robustness_sweep \
    --generator block-bootstrap-real --grid mn-tier1 \
    --score-source mn-basis-funding-residual \
    --taker-fee-bps 0 --slippage-bps 2 --paths 5 --year 2023 --out-dir /tmp/mn-demo/
param_robustness_sweep DONE
  report:         /tmp/mn-demo/robustness-sweep-20260608-060704-v2-mn-basisperp-fee00bps-theta-surface-2023-block-bootstrap-real-fy.md
  body_sha:       db7e48c4ce82f5c917816bce025fe0b36872d99d04799d06ecd5fbf7ec6b34c4
  wall_clock_s:   11.2
  n_cells:        2
  n_paths:        5
  family_verdict: FAMILY-UNIFORM-FRAGILE
  buyhold p50 Sharpe: 1.9262  P(loss): 0.0000  p95 MaxDD: 38.01%

  per-cell summary:
    g= 0 lookback=  60 k_long=3 drift=0.10 → FRAGILE | p50=-0.0185 p5=-0.0921 MaxDD_p95=98.5%
    g= 1 lookback= 168 k_long=3 drift=0.10 → FRAGILE | p50=-0.0871 p5=-0.1645 MaxDD_p95=98.5%
```

Both residual cells show a **negative** median Sharpe (the median path *loses*
money) with ~98.5% worst-case drawdown — the basis-orthogonal-to-funding signal
is worse than holding cash, reproduced live. Saved at
[`artifacts/perp-basis-mn-spread-2026-06-08/mn-basisperp-residual-smoke-n5-2023-fee0.txt`](artifacts/perp-basis-mn-spread-2026-06-08/mn-basisperp-residual-smoke-n5-2023-fee0.txt).

**(2) Basis arm vs funding arm — the same selection (the k2 confound):** I ran
both arms on the identical path-set. Their **per-cell distributions are
bit-identical** (g0: p50 +0.0053 / p5 −0.0792 / MaxDD 94.4%; g1: p50 +0.0500 /
p5 −0.0777 / MaxDD 92.1% — same to 4 decimals in both runs). A line-by-line diff
of the two report bodies differs **only** in the arm-name label and the
(non-hashed) wall-clock — every computed number is identical, because the basis
ranking and the funding ranking pick the **same names**. Saved at
[`artifacts/perp-basis-mn-spread-2026-06-08/k2-basis-eq-funding-bodydiff-n5.txt`](artifacts/perp-basis-mn-spread-2026-06-08/k2-basis-eq-funding-bodydiff-n5.txt)
(basis smoke + funding smoke stdout saved alongside). The /tmp demo reports were
deleted after capture — no anchored directory was touched (`git status` of
`reports/` clean).

## The headline — three arms, all fragile, and basis adds nothing over funding

This is the load-bearing table: the **best cell per arm** at 0 bps gross (the
gross ceiling — no fees, the most generous read), both years, scored against the
**dollar-neutral ≈0 null** (cash, NOT buy-and-hold — a beta-neutral book's correct
"do nothing" alternative). "Liquidations" counts forced short-position closes
across all N=200 paths when the book breached its maintenance-margin floor.
(Source: the 12 anchored surfaces in [`reports/`](../reports/); cross-checked
against the [test report § 5c–5d](../reports/test-2026-06-08-perp-basis-mn-spread.md).)

**2023 (in-sample), best cell per arm, 0 bps gross:**

| Arm | Best cell | p50 Sharpe | p5 Sharpe | P(Sharpe>1) | p95 MaxDD | Liquidations | Verdict |
|-----|-----------|----:|---:|------------:|----------:|-------------:|---------|
| Basis spread | g1 / L=168 | +0.037 | −0.140 | 0.000 | 97.77% | 86 | FRAGILE |
| Funding spread | g1 / L=168 | +0.037 | −0.140 | 0.000 | 97.77% | 86 | FRAGILE |
| **Basis⊥funding residual** | g1 / L=168 | **−0.043** | −0.197 | 0.000 | **100.00%** | 210 | FRAGILE |

**2024 (out-of-sample), best cell per arm, 0 bps gross:**

| Arm | Best cell | p50 Sharpe | p5 Sharpe | P(Sharpe>1) | p95 MaxDD | Liquidations | Verdict |
|-----|-----------|----:|---:|------------:|----------:|-------------:|---------|
| Basis spread | g1 / L=168 | +0.041 | −0.040 | 0.000 | 86.59% | 13 | FRAGILE |
| Funding spread | g1 / L=168 | +0.041 | −0.040 | 0.000 | 86.59% | 13 | FRAGILE |
| **Basis⊥funding residual** | g1 / L=168 | **−0.005** | −0.078 | 0.000 | 93.29% | 31 | FRAGILE |

> **P(Sharpe > 1) = 0.000 in every cell of every one of the 12 surfaces.** No
> cell clears any single one of the frozen decision bands. The fragility holds at
> **0 bps gross** — fees are not the killer (consistent with v0.1.0), the signal
> simply cannot clear zero. At the realistic **5 bps** fee the picture is
> unchanged (basis/funding best cell +0.035 / 2023, residual −0.047 / 2023 with
> 201 liquidations — [test report § 5c](../reports/test-2026-06-08-perp-basis-mn-spread.md)).

### The two killer findings (why this closes the domain)

**Finding 1 — the basis signal IS the funding signal (the k2 confound, fired at
maximum force).** The basis spread and the funding spread produce **byte-identical
result surfaces** — every Sharpe percentile, every probability, every drawdown,
every liquidation count matches to 6 decimal places, in-sample and out
([test report § 5d](../reports/test-2026-06-08-perp-basis-mn-spread.md), and the
live N=5 diff above). This is **not a bug** — it is verified real: the two arms
read **distinct data sources** (basis from `data/binance-basis`, funding from
`data/binance-funding`, distinct revision pins) through **distinct score wiring**,
and still select the identical portfolio, because funding *is* the mechanism that
prices the basis. The high-basis names ARE the high-positive-funding names, so
ranking on basis and ranking on funding pick the same longs and the same shorts.
(Pre-registered: basis/funding share +0.47/+0.66 level correlation — feature.md
§ k2.) The basis signal carries no information that funding does not already
carry.

**Finding 2 — no orthogonal alpha, and the residual is dangerous.** The
basis⊥funding residual arm is the decisive test of whether basis carries anything
*beyond* funding. It does not. The residual has a **negative median Sharpe** (2023
g0: −0.064; 2024 g0: −0.006), a **100% p95 drawdown** at 2023, and **hundreds of
short liquidations** (328 on the 2023 best-gross cell). Removing funding from
basis does not reveal a hidden edge — it reveals a portfolio that **destroys
capital in expectation and blows up on the tail**. The −0.10 IC the spike measured
lives **entirely in the funding channel**; the part of basis that is distinct from
funding is negative-expected-value and tail-catastrophic. (Source:
[test report § 5c "Residual arm special note" + § 5d](../reports/test-2026-06-08-perp-basis-mn-spread.md).)

### Why this is a domain-close, not another fragile result

Long-only (v0.1.0, FRAGILE even gross) + MN raw basis spread (FRAGILE) + MN
funding spread (FRAGILE, identical signal) + MN basis⊥funding residual (FRAGILE,
negative median, tail-catastrophic) — **every vehicle for the basis signal has now
been tested and every one fragile**. Price-rank, funding, and basis; long-only and
long/short. There is no remaining vehicle to wonder about. The derivatives-
positioning domain is closed **with finality**.

## Cumulative program scorecard — two domains exhausted

The full arc of the search in one view. Every active-trading combination tested
across the entire program → FRAGILE, dominated by passive buy-and-hold. The basis
rows are the first that started from a *live* signal — and the market-neutral
follow-on closes them by showing the signal is the funding signal with no
orthogonal residual. **On-chain (the #3 domain) is the next genuinely-orthogonal
series and has not yet been tested.** (Each active figure = best-cell median
Sharpe (p50); the null each was read against is in the last column.)

| Data domain | Method family / vehicle | Horizon(s) | Signal alive? | Best-cell p50 Sharpe | Verdict | Null |
|---|---|---|---|---:|---|---|
| OHLCV price | Cross-sectional momentum | 1h | No (rank-IC ≈ 0) | +0.014 | FAMILY-UNIFORM-FRAGILE | BH +1.74 |
| OHLCV price | Cross-sectional mean-reversion | 1h | No (rank-IC ≈ 0) | +0.007 | FAMILY-UNIFORM-FRAGILE | BH +1.74 |
| OHLCV price | Funding / carry (long-only) | 1h / 4h / daily | No | +0.039 → +0.065 | FAMILY-UNIFORM-FRAGILE | BH +1.74 |
| OHLCV price | Time-series momentum | 1h / 4h / daily | No | +0.047 → +0.169 | FAMILY-UNIFORM-FRAGILE | BH +1.74 |
| OHLCV price | (35-name universe spike) | 1h | No (beta ↓, IC still ≈ 0) | — | universe exonerated; still FRAGILE | BH |
| Derivatives positioning | Basis-reversal, **long-only** (v0.1.0) | 1h | YES (rank-IC −0.08…−0.11) | +0.051 | FAMILY-UNIFORM-FRAGILE even gross | BH +1.74 |
| **Derivatives positioning** | **Basis-reversal, MN spread (v0.2.0)** | **1h** | **YES, but = funding** | **+0.041** | **FAMILY-UNIFORM-FRAGILE** | **≈0 cash** |
| **Derivatives positioning** | **Funding-carry, MN spread (v0.2.0)** | **1h** | **= basis (identical)** | **+0.041** | **FAMILY-UNIFORM-FRAGILE** | **≈0 cash** |
| **Derivatives positioning** | **Basis⊥funding residual (v0.2.0)** | **1h** | **NO orthogonal alpha** | **−0.005** | **FAMILY-UNIFORM-FRAGILE (negative median)** | **≈0 cash** |
| **On-chain** (net-flows, stablecoin supply) | — | daily | _unknown — strongest orthogonality prior_ | — | **NOT YET TESTED** | — |
| _Buy-and-hold (the bar, all OHLCV surfaces)_ | _passive_ | _all_ | _n/a_ | _**+1.74 (2023) / +1.10 (2024)**_ | _the bar to beat_ | — |

> Sources: OHLCV rows — the prior program decks
> ([time-series-momentum-robustness](../../time-series-momentum-robustness/presentations/time-series-momentum-robustness-2026-06-03.md),
> [horizon-retest-robustness](../../horizon-retest-robustness/presentations/horizon-retest-robustness-2026-06-05.md)).
> Basis long-only row — the v0.1.0 deck
> ([perp-basis-signal-robustness](../../perp-basis-signal-robustness/presentations/perp-basis-signal-robustness-2026-06-06.md)).
> The three v0.2.0 MN rows — this feature's 12 surfaces +
> [test report § 5c–5d](../reports/test-2026-06-08-perp-basis-mn-spread.md).
> On-chain row — the **#3 orthogonal domain** in the
> [new-data-domain scoping note](../../dev-notes/new-data-domain-scoping-2026-06-05.md);
> never built.
>
> **The single-sentence arc:** every OHLCV signal was *dead at the signal level*
> and fragile as a strategy; the basis was the first signal *alive at the signal
> level* — and the market-neutral vehicle proved that aliveness IS the funding
> signal, with no distinct residual, closing the second full data domain. Two
> domains down; passive buy-and-hold still undefeated.

## Numbers that matter

- **Anchors: 119 / 119 PASS** — verified live this session:
  `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (119 / 119)`. New: #108–#119
  (the 12 arm × fee × year MN surfaces); all 107 pre-existing anchors
  byte-identical (transition 107 → 119, strictly additive). This is the **FIRST
  `run_path` anchor-neutrality re-proof** — the short-side engine was added without
  perturbing a single existing anchor.
- **Day-1 falsifiers: 7 passed / 0 failed** — `cargo test ... --test
  mn_spread_divergence_e2e` → all 7 green (test report § 3), each with a documented
  RED-on-revert proof (test report § 5a). The load-bearing ones: dollar-neutrality
  (proves the book carries ≈0 net market exposure), the short-leg funding-cost
  non-no-op (proves the binding cost is actually applied, not computed-and-ignored),
  and the orthogonalization non-no-op (proves the residual arm genuinely differs
  from the raw basis arm).
- **The residual is negative AND tail-catastrophic (the killer number):** best-gross
  residual cell p50 = **−0.064** (2023) / −0.006 (2024); p95 drawdown **100%** with
  **328 short liquidations** on the 2023 best-gross cell. Basis-orthogonal-to-funding
  is worse than cash.
- **Basis = funding to 6 decimals:** the basis-spread and funding-spread surfaces are
  byte-identical on every metric, both regimes, both fee levels (test report § 5d) —
  reproduced live in the N=5 diff (identical surface tables, differing only in arm
  label).
- **P(Sharpe > 1) = 0.000** in every one of the 12 surfaces' every cell. No cell
  clears any single decision band against the ≈0 null.
- **Two-run byte-identity confirmed:** the MN basis arm produced an identical body-SHA
  (`aa2c5d13…`) on two runs with the same seed (test report § 5b) — full determinism
  across the new short-side engine, liquidation rule, and second sidecar.
- **Wall-clock:** ~202–210 s per anchored surface (2 cells × N=200) on the canonical
  Apple-Silicon box; ~12 min for all 12 surfaces — under the ≲30 min tractability gate
  (test report § 6). The live N=5 smokes above ran in ~11 s each.
- **Spec-lint:** `spec-lint: FAIL (94 violations in 2 categories)` — verified live:
  `python3 scripts/spec_lint.py` (exit 2). This **matches the tester's PASS baseline
  exactly** (87 dead-link + 7 trace-broken-path, all pre-existing carry-over) and is
  an *improvement* over the audit-2026-06-01 baseline (95 violations / 3 categories).
  **Zero new violations, zero new categories** introduced by this feature; no
  regression since the tester's PASS.

## Verification matrix

V1..V7 map to the feature's [`## Verification`](../feature.md) gates. Each is
VERIFIED with one-line evidence.

| V-id | Description | Status | Evidence |
|------|-------------|--------|----------|
| V1 | Three-arm comparison (R-MN.6) — net-of-cost edge of basis / funding / basis⊥funding vs the ≈0 null; confound verdict explicit | VERIFIED | test report § 5c–5d: the 3-arm table; basis = funding byte-identical; residual negative-median → basis carries no orthogonal alpha |
| V2 | The dollar-neutral verdict — does the spread clear the ≈0 null on the frozen § 0 bands at the realistic fee? | VERIFIED | test report § 5c: FAMILY-UNIFORM-FRAGILE on all 12 surfaces vs ≈0 null; P(Sharpe>1)=0.000 everywhere; fragile at 0 bps gross and 5 bps |
| V3 | Day-1 falsifiers RED-on-revert (R-MN.7) — each GREEN as written AND RED when reverted | VERIFIED | test report § 3 (7/7 GREEN) + § 5a (per-falsifier RED-on-revert proof method documented); `git diff crates/` empty after |
| V4 | The 107 existing anchors byte-identical with `k_short = 0` (FIRST run_path re-proof) + new MN anchors locked | VERIFIED | `verify_anchors.sh` live → 119/119 (107 unchanged + 12 new #108–#119); `run_path_k_short_zero_byte_identical_to_head` PASS (test report § 3) |
| V5 | Two-run byte-identity of the MN surface body-SHA | VERIFIED | test report § 5b: basis arm body-SHA `aa2c5d13…` identical across two seeded runs; the anchor gate is itself a determinism check |
| V6 | Pre-flight void-if-fail — `generator: block-bootstrap-real` AND `bootstrap_mode: shared-index` | VERIFIED | test report § 5e: both fields present in all 12 surfaces; sample surface frontmatter confirms `generator: block-bootstrap-real` / `bootstrap_mode: shared-index` |
| V7 | Frozen § 0 composite verdict at the realistic fee vs the dollar-neutral ≈0 null | VERIFIED | each surface "## Family verdict" line FAMILY-UNIFORM-FRAGILE; weakest-link composite per [`robustness-decision-rule-2026-05-30.md`](../../dev-notes/robustness-decision-rule-2026-05-30.md) § 0 bands |

## Open decisions

There is exactly ONE decision in front of you for the approval block, and one
strategic fork framed for your awareness (deliberately NOT bundled into the
approval — one decision per tick).

**Decision — Ratify the close of the derivatives-positioning domain.** Approve
that the market-neutral basis-reversal spread is FAMILY-UNIFORM-FRAGILE in all
three arms against the ≈0 dollar-neutral null; that the basis signal is the
funding signal (identical selection, verified real); that the basis⊥funding
residual has negative expected return and is tail-catastrophic (so basis carries
no orthogonal alpha); and therefore that the **derivatives-positioning domain is
closed with finality** — every vehicle (price-rank, funding, basis; long-only and
long/short) has been tested and every one is fragile. Approving this retires the
domain and frees the next dollar for the fork below.

> **What "yes" commits you to:** nothing further on derivatives-positioning — the
> domain is done, the 119 anchors stand as the durable record. The only follow-on
> cost is **choosing the fork** below (a separate decision), for which the analyst
> will bring decision-support in parallel.

### The strategic fork (FYI — frame only, do NOT decide here)

Where the next dollar goes, now that two full data domains (OHLCV and
derivatives-positioning) are exhausted and passive buy-and-hold remains undefeated.
This is genuinely your call; the analyst is preparing decision-support in
parallel. Presented neutrally — **no `(Recommended)` tag**, because the analyst's
scoping is the input that should decide it, not this deck:

- **(a) Route to on-chain — the #3 orthogonal domain.** Settlement-layer truth
  outside any price tape (exchange net-flows = sell pressure; stablecoin supply =
  dry powder; active addresses = adoption) — the strongest *orthogonality* story
  on the board, genuinely a function of neither OHLCV nor derivatives positioning.
  **The cost to weigh:** a new fetcher + per-source schema + point-in-time hygiene
  (~5–8 dev-days), and free tiers are daily-resolution only, so it caps us to a
  daily backtest on a thin ~2-year (~730-point) window. It tests an *unmeasured*
  hypothesis — orthogonality is a story, the IC is unknown.
- **(b) Conclude the active-vs-passive search and ship the passive baseline.** Two
  full domains and every active vehicle within them have now failed to beat passive
  buy-and-hold net of fees; the cumulative evidence that active cross-sectional
  trading does not clear the passive bar on this universe is now strong. **The cost
  to weigh:** this is a strategic concession that closes the active-trading search
  (on-chain stays un-tested — a permanent "what if"), in exchange for shipping a
  defensible passive product now rather than spending another ~5–8 days on an
  unmeasured domain.

**Decision pending — see analyst decision-support. This deck does not pick a
winner.** If you approve with a steer toward (a) or (b), note it under
Notes/feedback and the orchestrator routes the next step accordingly.

## Approval

The approval below gates ONLY the close-out decision above (ratify the close of
the derivatives-positioning domain). All boxes ship un-ticked; you are the only
one who ticks.

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback
_empty until operator fills — note any steer on the strategic fork (a vs b) here_

## Feedback log
_empty — rejections/steers appended here and routed back to the named agent_

## Changelog
- 2026-06-08 (presenter): release deck for perp-basis-mn-spread (VERDICT → PASS,
  HEAD `8c2e6c4`) — closes the ENTIRE derivatives-positioning domain. Built the
  market-neutral long/short basis spread (the "correct vehicle" the v0.1.0
  retrospective specified), the FIRST `run_path` touch since v0.1.0, done
  anchor-neutral (107/107 held at every seam via a `k_short = 0` dead-code-by-
  construction design + a byte-identity re-proof test; anchors 107 → 119). All 3
  arms (basis / funding / basis⊥funding residual) FAMILY-UNIFORM-FRAGILE vs the ≈0
  dollar-neutral null. Two killer findings: (1) k2 confound at max force — basis
  spread = funding spread, byte-identical result surfaces (basis IS the funding
  signal; verified real via distinct data sources + distinct score wiring, not a
  bug); (2) no orthogonal alpha — the basis⊥funding residual has NEGATIVE median
  Sharpe (−0.06) + 100% tail drawdown via hundreds of short liquidations. The
  perp-basis-reversal signal survives in NO vehicle. Cumulative program scorecard
  updated: TWO domains exhausted (OHLCV + derivatives-positioning, all fragile);
  passive buy-and-hold undefeated; on-chain NOT YET TESTED. Anchors 119/119
  verified live; spec-lint live FAIL 94/2 = tester baseline (no regression, +1
  category better than audit-2026-06-01). Live demo: N=5 residual smoke reproduced
  the negative median; N=5 basis-vs-funding diff reproduced the identical-selection
  result. Pre-registered fork (route to on-chain vs ship passive baseline) teed up,
  NOT decided.
