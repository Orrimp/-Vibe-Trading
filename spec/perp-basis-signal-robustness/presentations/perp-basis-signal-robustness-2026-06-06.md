---
slug: perp-basis-signal-robustness
mode: release
status: draft
owner: presenter
audience: human-operator
updated: 2026-06-06
generated: 2026-06-06T19:35:00Z
---

# Perp basis-reversal — release deck + derivatives-positioning family scorecard

> This deck closes the **perp-basis-signal-robustness** feature (VERDICT → PASS,
> HEAD `0a36cdf`) and reads it as a program retrospective. The headline is
> two-sided and worth holding both halves in your head at once: we found the
> **first live, orthogonal, causal signal** of the post-OHLCV program — and the
> **long-only vehicle we built for it is fragile even with zero fees**. The
> signal is real; the vehicle is wrong. Every number below traces to a committed
> report or a gate I ran live this session — nothing is computed here for the
> first time, and nothing is asserted without a source.

## TL;DR

The perp-basis-reversal signal is **real** — the first cross-sectional signal in
this whole program that actually carries forward information (rank-IC −0.08 to
−0.11, orthogonal to price, causal). But the **long-only** strategy we built to
trade it is **FRAGILE at every fee level we tested, including 0 bps gross**: it
carries full market beta and gets swamped by simply buying and holding the coins
(+1.74 Sharpe / 2023). Critically, **fees are not the killer** — the median
barely moves across the fee ladder, so this is a verdict on the *vehicle*, not
the signal. The derivatives-positioning family closes on the long-only verdict.
VERDICT PASS; anchors 99 → **107** (verified live `ANCHORS PASS (107 / 107)`).
The open fork — market-neutral long/short spread vs. route to on-chain — is teed
up for your call, not decided here.

## What changed

- **Pivoted to a brand-new data domain.** Every prior signal lived inside OHLCV
  price bars. This feature reached outside them for the first time, into
  **derivatives positioning**: the perpetual **basis** = (perp mark price −
  spot index price) / index price. A research spike found this signal is alive
  (the first non-dead one), and this build converted that spike into a hard
  robustness verdict.
- **Built the basis-reversal arm, anchor-neutral.** Added a `BasisReversal`
  score (`basis_reversal_score = −trailing_mean(basis)` — buy the lowest-basis
  names) to the existing cross-sectional strategy, plus a basis data loader and
  a `--taker-fee-bps` fee axis on the sweep binary. It slotted in defaults-off:
  all **99 pre-existing regression anchors stayed byte-identical** throughout.
- **Ran the fee-sweep — the load-bearing gate — and locked 8 new surfaces.**
  Eight anchored result surfaces ({0, 2, 5, 10} bps taker fee × {2023, 2024}),
  each a full block-bootstrap distribution over the 6-cell parameter grid.
  **Every one came back FAMILY-UNIFORM-FRAGILE.** Eight new byte-stable anchors
  locked (#100–#107); the gate is now 107/107.

## Why

The active-trading robustness program had already closed exhaustively negative:
four method families (cross-sectional momentum, mean-reversion, funding-carry,
time-series momentum) × three horizons (1h / 4h / daily) × a 35-name universe
spike, **all FAMILY-UNIFORM-FRAGILE**, all dominated by passive buy-and-hold net
of fees. The decisive failure was information-theoretic — the cross-sectional
*price*-ranking channel carries ≈ 0 forward information (rank-IC within ±0.07 of
zero, no stable sign). With every axis *inside* the price bars exhausted, the
only way to test a thesis this data cannot express is a genuinely different
signal class. You chose the **new-data-domain** fork, and a ~0.5-day research
spike on the perpetual basis — the cheapest structurally-new series, native to
the hourly grid — came back **LIVE**: a cross-sectional *reversal* signal where
high-basis names underperform, negative and sign-stable in both years,
orthogonal to price, and causal (past basis predicts future return; the leaked
contemporaneous version flips sign). After a parade of dead OHLCV signals, this
was the first with real cross-sectional information worth building on. See
[`feature.md`](../feature.md) "Why" and the
[basis-spike scoping note](../../dev-notes/new-data-domain-scoping-2026-06-05.md#basis-spike-results)
(§ BS.0–BS.6, VERDICT **LIVE / MEDIUM-HIGH**).

## What you can do now

| Action | Command |
|--------|---------|
| Reproduce an anchored basis-reversal surface (locked grid, N=200, 0 bps gross) | `cargo run -p backtest --features "candle realdata" --bin param_robustness_sweep -- --grid basis-tier1 --score-source basis-reversal --taker-fee-bps 0 --slippage-bps 2 --paths 200 --year 2023 --out-dir /tmp/basis-verify/` |
| Reproduce the realistic-fee surface (10 bps taker, 2024 tail-negative regime) | `cargo run -p backtest --features "candle realdata" --bin param_robustness_sweep -- --grid basis-tier1 --score-source basis-reversal --taker-fee-bps 10 --slippage-bps 2 --paths 200 --year 2024 --out-dir /tmp/basis-verify/` |
| Confirm every regression anchor is byte-stable (incl. the 8 new basis anchors) | `bash scripts/verify_anchors.sh` |
| Re-run the 6 day-1 basis falsifiers (divergence, sign, non-no-op, no-look-ahead, two-run) | `cargo test -p backtest --features "candle realdata" --test basis_divergence_e2e` |
| Re-read the spike that found the signal (the WHY behind the pivot) | open `spec/dev-notes/new-data-domain-scoping-2026-06-05.md` (§ BS.0–BS.6) |

## Live demo

A fresh basis-reversal sweep, run just now on real 2023 Binance basis + OHLCV
data, end to end through the loader, the as-of join, and the corrected fee path.
This is a **fast smoke at N=5 paths** (the anchored surfaces use N=200 — so the
per-cell numbers here are NOT the anchored figures; they exist only to prove the
binary genuinely produces the result live). What it demonstrates: the family
verdict, that the basis signal is load-bearing in the path, and the
buy-and-hold gap all reproduce on demand.

```
$ ./target/debug/param_robustness_sweep \
    --generator block-bootstrap-real --grid basis-tier1 \
    --score-source basis-reversal --taker-fee-bps 0 --slippage-bps 2 \
    --paths 5 --year 2023 --out-dir /tmp/basis-demo/
param_robustness_sweep DONE
  report:         /tmp/basis-demo/robustness-sweep-20260606-193321-v1-basis-reversal-fee00bps-theta-surface-2023-block-bootstrap-real-fy.md
  body_sha:       b7afb5a118cc2b92dae5012d856ce41b21a6569208ff292410487a277132f536
  wall_clock_s:   19.4
  n_cells:        6
  n_paths:        5
  family_verdict: FAMILY-UNIFORM-FRAGILE
  buyhold p50 Sharpe: 1.9262  P(loss): 0.0000  p95 MaxDD: 38.01%

  per-cell summary:
    g= 0 lookback=  60 k_long=3 drift=0.10 → FRAGILE | p50=0.0305 p5=0.0185 MaxDD_p95=68.2%
    g= 1 lookback=  24 k_long=3 drift=0.10 → FRAGILE | p50=0.0190 p5=-0.0430 MaxDD_p95=81.1%
    g= 2 lookback= 168 k_long=3 drift=0.10 → FRAGILE | p50=0.0491 p5=0.0287 MaxDD_p95=73.8%
    g= 3 lookback=  60 k_long=5 drift=0.10 → FRAGILE | p50=0.0214 p5=0.0071 MaxDD_p95=73.2%
    g= 4 lookback=  60 k_long=1 drift=0.10 → FRAGILE | p50=0.0201 p5=0.0023 MaxDD_p95=52.4%
    g= 5 lookback=  24 k_long=5 drift=0.10 → FRAGILE | p50=0.0135 p5=-0.0146 MaxDD_p95=76.1%
```

Notice: all 6 cells FRAGILE even at **0 bps gross**, with buy-and-hold p50
Sharpe ≈ +1.93 on this N=5 path-set while the best basis cell p50 is ≈ +0.05 — a
~40× gap, and several cells already show a negative worst-case (p5) tail. (Sharpe
= return per unit of risk; higher is better. "FRAGILE" = fails the frozen
decision rule, primarily because the worst-case path loses money / the median is
nowhere near the bar.) Full stdout saved at
[`artifacts/perp-basis-signal-robustness-2026-06-06/basis-reversal-smoke-n5-2023-fee0.txt`](artifacts/perp-basis-signal-robustness-2026-06-06/basis-reversal-smoke-n5-2023-fee0.txt).
The /tmp demo report was deleted after capture — no anchored directory was
touched.

## The headline — fees are NOT the killer (the fee-sweep surface)

This is the load-bearing table. It shows the **best** and **worst** parameter
cell at **every fee level**, both years. Read down any single year and watch the
median (p50): it **barely moves** as fees climb from 0 to 10 bps. If fees were
the killer, the median would collapse across the ladder — it doesn't. The signal
is being dominated by passive buy-and-hold *structurally*, not bled away by
transaction costs. (Source: the 8 anchored surfaces in
[`reports/`](../reports/), best cell = max-p50 cell (g2, lookback-168); worst
cell = min-p5 cell (g4, lookback-60, k_long=1). Cross-checked against the
[test report § 5](../reports/test-2026-06-06-1200-perp-basis-signal-robustness.md).)

| Taker fee | Year | Best cell (g2, L=168) p50 / p5 | Worst cell (g4, L=60, K=1) p50 / p5 | Family verdict |
|---|---|---:|---:|---|
| **0 bps** (gross) | 2023 | +0.049 / −0.043 | +0.020 / **−0.231** | FAMILY-UNIFORM-FRAGILE |
| 0 bps (gross) | 2024 | +0.051 / −0.010 | +0.027 / −0.001 | FAMILY-UNIFORM-FRAGILE |
| 2 bps | 2023 | +0.048 / −0.063 | +0.020 / −0.231 | FAMILY-UNIFORM-FRAGILE |
| 2 bps | 2024 | +0.050 / −0.010 | +0.026 / −0.001 | FAMILY-UNIFORM-FRAGILE |
| 5 bps | 2023 | +0.047 / −0.064 | +0.019 / −0.231 | FAMILY-UNIFORM-FRAGILE |
| 5 bps | 2024 | +0.049 / −0.011 | +0.026 / −0.002 | FAMILY-UNIFORM-FRAGILE |
| 10 bps | 2023 | +0.045 / −0.081 | +0.019 / −0.232 | FAMILY-UNIFORM-FRAGILE |
| 10 bps | 2024 | +0.047 / −0.015 | +0.026 / −0.001 | FAMILY-UNIFORM-FRAGILE |
| **Buy-and-hold (the bar)** | 2023 | **+1.735** / +0.124 | — | _passive — the bar to beat_ |
| **Buy-and-hold (the bar)** | 2024 | **+1.105** / — | — | _passive — the bar to beat_ |

> **P(Sharpe > 1) = 0.000 in every active cell of every surface** vs. buy-and-hold's
> 77.5% (2023) / 53.5% (2024). The best active median Sharpe (+0.05) is **34×–37×
> below** the buy-and-hold control's +1.735 median. The strategy *does* trade and
> *does* take risk (worst cell p95-MaxDD ≈ 83%, time-in-market is real) — it just
> never gets paid for it.

### Why the vehicle is wrong (the structural read)

The spike measured the −0.10 rank-IC on the **full long/short cross-sectional
spread** — long the lowest-basis names, short the highest. The v0.1.0 arm we
built is **long-only**: it captures only the long-low-basis *half* of that spread
while carrying **full market beta**. So it inherits the entire +1.74 buy-and-hold
return *as the bar it must beat*, while only harvesting one leg of a market-
neutral signal. A long-only tilt is structurally the wrong vehicle for a live
cross-sectional spread signal. **This is a decision-grade negative on the
long-only vehicle — NOT, on this evidence, a negative on the signal itself.**

## Cumulative program scorecard — every domain × family tested

The full arc of the search in one view. Every active-trading combination tested
across the entire program → FRAGILE. The basis row is the first that started from
a *live* signal — and still failed, because of the vehicle. On-chain is the next
genuinely-orthogonal domain and has **not yet been tested**. (Each active figure
= best-cell median Sharpe (p50); buy-and-hold is the bar each was read against.)

| Data domain | Method family | Horizon(s) | Signal alive? | Best-cell p50 Sharpe | Verdict |
|---|---|---|---|---:|---|
| OHLCV price | Cross-sectional momentum | 1h | No (rank-IC ≈ 0) | +0.014 | FAMILY-UNIFORM-FRAGILE |
| OHLCV price | Cross-sectional mean-reversion | 1h | No (rank-IC ≈ 0) | +0.007 | FAMILY-UNIFORM-FRAGILE |
| OHLCV price | Funding / carry | 1h / 4h / daily | No | +0.039 → +0.065 | FAMILY-UNIFORM-FRAGILE |
| OHLCV price | Time-series momentum | 1h / 4h / daily | No | +0.047 → +0.169 | FAMILY-UNIFORM-FRAGILE |
| OHLCV price | (35-name universe spike) | 1h | No (beta ↓, IC still ≈ 0) | — | universe exonerated; still FRAGILE |
| **Derivatives positioning** | **Basis-reversal (long-only v0.1.0)** | **1h** | **YES (rank-IC −0.08…−0.11)** | **+0.051** | **FAMILY-UNIFORM-FRAGILE even gross** |
| **On-chain** (net-flows, stablecoin supply) | — | daily | _unknown — strongest orthogonality prior_ | — | **NOT YET TESTED** |
| _Buy-and-hold (the bar, all OHLCV surfaces)_ | _passive_ | _all_ | _n/a_ | _**+1.74 (2023) / +1.10 (2024)**_ | _the bar to beat_ |

> Sources: OHLCV rows — the prior program decks
> ([time-series-momentum-robustness](../../time-series-momentum-robustness/presentations/time-series-momentum-robustness-2026-06-03.md),
> [horizon-retest-robustness](../../horizon-retest-robustness/presentations/horizon-retest-robustness-2026-06-05.md)).
> Basis row — this feature's 8 surfaces + [test report § 5](../reports/test-2026-06-06-1200-perp-basis-signal-robustness.md).
> On-chain row — ranked **#2 orthogonal domain** (EV-rank 2) in the
> [new-data-domain scoping note § 3](../../dev-notes/new-data-domain-scoping-2026-06-05.md);
> never built.
>
> **The single-sentence arc:** every OHLCV signal was *dead at the signal level*
> and FRAGILE as a strategy; the basis is the first signal that is *alive at the
> signal level* but FRAGILE **as a long-only strategy** — which is a different and
> more hopeful kind of failure, because the spread vehicle was never built.

## Numbers that matter

- **Anchors: 107 / 107 PASS** — verified live this session:
  `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (107 / 107)`. New: #100–#107
  (the 8 fee × year basis surfaces); all 99 pre-existing anchors byte-identical
  (transition 99 → 107, strictly additive).
- **Day-1 falsifiers: 6 passed / 0 failed** — `cargo test ... --test
  basis_divergence_e2e` → `test result: ok. 6 passed; 0 failed`
  (test report § 3). The load-bearing **sign guard** is confirmed RED-on-revert:
  flipping `−mean` → `+mean` at `momentum.rs:360` makes two `r_br2_*` unit tests
  panic with explicit "SIGN VIOLATION" messages; source restored byte-identical.
- **Fees are not the killer (the headline number):** best-cell p50 Sharpe moves
  only +0.049 → +0.045 (2023) and +0.051 → +0.047 (2024) across the **entire**
  0 → 10 bps fee ladder. A ~0.004 Sharpe move; the gap to buy-and-hold is ~1.69.
- **P(Sharpe > 1) = 0.000** in every active cell on every surface (vs.
  buy-and-hold 77.5% / 53.5%). Best active median +0.05 = **34×–37× below** the
  buy-and-hold +1.735 median.
- **The signal IS orthogonal + causal (why it's worth a spread vehicle):**
  rank-IC −0.08 to −0.11, sign-stable both years; corr to OHLCV momentum
  +0.01…+0.23 (orthogonal); causal trailing ≠ leaked contemporaneous at every
  horizon (no-look-ahead). Source: spike § BS.2–BS.4.
- **Wall-clock:** ~28–30 s per anchored surface (6 cells × N=200) on the canonical
  Apple-Silicon box; the live N=5 smoke above ran in 19.4 s. Compute is a rounding
  error.
- **Spec-lint:** `spec-lint: FAIL (94 violations in 2 categories)` — verified live:
  `python3 scripts/spec_lint.py`. This **matches the tester's PASS baseline exactly**
  (87 dead-link + 7 trace-broken-path, all pre-existing carry-over) and is an
  *improvement* over the audit-2026-06-01 baseline (95 violations / 3 categories).
  **Zero new violations** introduced by this feature; no regression since the
  tester's PASS.

## Verification matrix

V1..V6 map to the feature's `## Verification` gates. Each is VERIFIED with
one-line evidence.

| V-id | Description | Status | Evidence |
|------|-------------|--------|----------|
| V1 | The fee-sweep result (R-BR.LOAD): net-of-fee edge vs BH reported at each fee level; FRAGILE-on-fees verdict explicit | VERIFIED | test report § 5 fee-sweep table; the 8 anchored surfaces; fee-sweep table above (p50 flat across ladder, FRAGILE at all levels incl. 0 bps) |
| V2 | Day-1 falsifiers RED-on-revert (sign, baseline-divergence, non-no-op, no-look-ahead) | VERIFIED | test report § 3: 6/6 GREEN as written; `r_br2_*` sign guard RED-on-revert with explicit panic at `momentum.rs:360`; `git diff crates/` empty after restore |
| V3 | The 99 existing anchors byte-identical + new basis anchors locked | VERIFIED | `verify_anchors.sh` live → 107/107 (99 unchanged + 8 new #100–#107); transition 99 → 107 additive |
| V4 | Two-run byte-identity of the basis surface body-SHA (R-BR.7) | VERIFIED | test report § 3 `basis_two_run_byte_identity` PASS; the anchor gate is itself a determinism check |
| V5 | Pre-flight void-if-fail headers (`generator: block-bootstrap-real` + `bootstrap_mode: shared-index`) | VERIFIED | test report § 5 "void-if-fail headers" — confirmed present in all 8 reports; sample surface frontmatter `generator: block-bootstrap-real` / `bootstrap_mode: shared-index` |
| V6 | Frozen § 0 composite verdict read at the realistic fee level vs the BH control | VERIFIED | each surface "## Family verdict" line FAMILY-UNIFORM-FRAGILE; scored weakest-link vs frozen `robustness-decision-rule-2026-05-30.md` § 0 bands |

## Open decisions

There is exactly ONE decision in front of you for the approval block, and one
strategic fork framed for your awareness (deliberately NOT bundled into the
approval — one decision per tick).

**Decision — Ratify the close-out of the derivatives-positioning family on the
long-only verdict.** Approve that the **long-only** v0.1.0 basis-reversal arm is
FAMILY-UNIFORM-FRAGILE at every fee level including 0 bps gross — dominated by
passive buy-and-hold by 34×–37× on median Sharpe — and that this is a
decision-grade negative on the **long-only vehicle**, explicitly **not** on the
signal itself (the −0.10 IC is real, orthogonal, and causal). Approving this
closes the long-only derivatives-positioning arm and frees the next dollar for
the fork below.

### The strategic fork (FYI — frame only, do NOT decide here)

Where the next dollar goes. This is genuinely your call; the analyst is scoping
both in parallel and will bring a decision-grade recommendation. Presented
neutrally — no `(Recommended)` tag, because the analyst's scoping is the input
that should decide it, not this deck:

- **(a) Build the market-neutral long/short v0.2.0 basis spread** — the *correct*
  vehicle for a live spread signal. It removes the market-beta burden (no longer
  fighting the +1.74 BH bar) and captures **both** legs of the −0.10 IC (long
  low-basis, short high-basis), not the weaker half. **The cost to weigh:** the
  short leg has its own cost structure — perp funding paid on the shorts, borrow,
  and roughly **2× the fee drag** of a long-only book. The fee-sweep just showed
  fees aren't the long-only killer, but a 2×-fee short book changes that
  calculus, so the spread's net-of-cost edge is an open empirical question.
- **(b) Route to on-chain** — the analyst's **#2-ranked** orthogonal domain
  (settlement-layer flows, stablecoin supply, exchange net-flows; the strongest
  *orthogonality* story of all candidates). **The cost to weigh:** a new fetcher +
  per-source schema + point-in-time hygiene (~5–8 dev-days), and free tiers are
  daily-resolution only, so it would cap us to a daily backtest on a thin 2-year
  window.

**Decision pending — see analyst scoping. This deck does not pick a winner.** If
you approve with a steer toward (a) or (b), note it under Notes/feedback and the
orchestrator routes the next scoping to the analyst accordingly.

## Approval

The approval below gates ONLY the close-out decision above (ratify the long-only
basis-reversal arm as a decision-grade negative on the vehicle). All boxes ship
un-ticked; you are the only one who ticks.

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback
_empty until operator fills — note any steer on the strategic fork (a vs b) here_

## Feedback log
_empty — rejections/steers appended here and routed back to the named agent_

## Changelog
- 2026-06-06 (presenter): release deck for perp-basis-signal-robustness (VERDICT
  → PASS, HEAD `0a36cdf`). Two-sided headline: first LIVE/orthogonal/causal signal
  of the post-OHLCV program (rank-IC −0.08…−0.11) vs. long-only v0.1.0 vehicle
  FAMILY-UNIFORM-FRAGILE at every fee incl. 0 bps gross. Fee-sweep surface (8
  anchored surfaces, {0,2,5,10} bps × {2023,2024}) shows fees are NOT the killer
  (p50 flat across ladder). Structural read: long-only captures one leg of a
  market-neutral spread while carrying full beta → swamped by +1.74 BH bar.
  Cumulative program scorecard (every domain × family → all FRAGILE; basis-reversal
  long-only FRAGILE-even-gross; on-chain NOT YET TESTED). Anchors 99 → 107 (verified
  live `ANCHORS PASS (107/107)`); spec-lint live FAIL 94/2 = tester baseline (no
  regression). Live demo: N=5 basis-reversal smoke reproduced FAMILY-UNIFORM-FRAGILE.
  Strategic fork (market-neutral v0.2.0 spread vs on-chain) teed up, NOT decided.
