---
slug: program-capstone
mode: release
status: draft
owner: presenter
audience: human-operator
updated: 2026-06-08
generated: 2026-06-08T13:05:00Z
scope: program-level (active-vs-passive research program — terminal sign-off)
---

# Program capstone — the active-vs-passive search is concluded: ship passive

> This is the **terminal** sign-off for the entire active-vs-passive research
> program. It is NOT a single-feature deck — it sits at the program level
> (`spec/presentations/`) and supersedes nothing; the per-feature decks it cites
> remain the source of record. The pre-committed on-chain hard-stop fired today,
> so the program has its final answer. **Bottom line up front: across the three
> reachable information channels — price, derivatives-positioning, and on-chain —
> no active strategy beat passive buy-and-hold net of cost, under a frozen,
> pre-registered, anti-cherry-pick Monte-Carlo decision rule. The program
> concludes with a clear, bounded verdict: ship passive.** This is not a failure.
> It is a decision-grade answer produced by a rigorous machine that found exactly
> one genuinely live signal (perp basis) and then proved even that one does not
> survive in any tradeable vehicle. Every number below traces to a committed
> anchored report, a prior dated deck, or a gate I ran live this session —
> nothing is computed here for the first time without showing the computation,
> and nothing is asserted without a source.

## TL;DR

The robustness machine ran its frozen §0 weakest-link decision rule across **three
structurally-distinct data channels** and every active strategy came back
**FRAGILE**, dominated by passive buy-and-hold. **Channel 1 (price/OHLCV):** 4
method families × 3 horizons × a universe axis — all fragile, the cross-sectional
*price*-ranking signal carries ≈0 forward information. **Channel 2
(derivatives-positioning):** funding-carry fragile; the perp **basis-reversal**
signal is the program's **one genuinely live signal** (rank-IC −0.10, orthogonal
to price, causal) — but long-only it is beta-swamped and fragile, the
market-neutral spread is fragile in all 3 arms, and the two decisive facts close
it: **basis ≡ funding byte-identically** (funding mechanically prices the basis —
verified real, not a wiring bug) and the **basis⊥funding residual carries a
negative median Sharpe** with 100% tail drawdown (no orthogonal alpha left to
find). **Channel 3 (on-chain):** the bounded final hunt with a pre-committed fuse —
net-flows are PIT-infeasible on free data (the vendor's own docs disclaim
point-in-time accuracy), and the cleaner-PIT stablecoin-supply fallback is fragile
(its sign flips year-over-year, failing the same cross-year-replication bar the
basis signal *passed*). Both branches landed on the fuse → **HARD-STOP**. The
verdict, in its honest scope: **active ≤ passive in the *reachable* universe, net
of cost, on the 2023-24 large-cap sample.** The terminal product is passive
buy-and-hold — a **promotion** of the already-built, already-anchored BH control,
not a new build. Regression gate stands at **119 / 119 anchors** (verified live
this session: `ANCHORS PASS (119 / 119)`). Two ratifications are in front of you:
**(a)** the program conclusion, and **(b)** ship passive. There is no open fork —
you pre-committed the hard-stop; this deck ratifies the executed conclusion.

## What changed (the whole journey, in three bullets)

- **The question got a final answer.** The program set out to answer one thing:
  *does any active strategy beat passive buy-and-hold, net of cost, under a frozen
  anti-cherry-pick rule?* As of today the answer is in, across all three reachable
  channels: **no.** The on-chain hard-stop fired
  ([`onchain-netflow-spike-2026-06-08`](../onchain-netflow-spike-2026-06-08.md)),
  which was the pre-registered terminal probe.
- **The one live signal was run to ground.** The perp basis-reversal signal — the
  only signal in the entire program that carried real forward information — was
  given its fair test in its *correct* vehicle (the dollar-neutral long/short
  spread the long-only retrospective specified). It failed there too, and on the
  way proved it is simply the funding signal wearing a different name, with no
  orthogonal residual. There is no remaining vehicle to wonder about.
- **The terminal state is a promotion, not a build.** "Ship passive" means
  promoting the buy-and-hold control — the most-tested path in the repo, the
  benchmark every one of the 119 anchored surfaces was scored against — from
  "control" to "the strategy the paper-trading agent runs." No new code; the
  deliverable is the robustness machine plus its auditable, three-channel negative.

## Why — the credibility spine (why this negative is trustworthy)

A negative result is only worth shipping if the method that produced it cannot be
accused of giving up early or of being gamed. This program was built so that the
negative is the *strong* kind. Five design choices carry that weight, and they are
why "ship passive" is a decision and not a shrug:

1. **A frozen, pre-registered decision rule.** The §0 **weakest-link** rule was
   locked *before* the results came in
   ([`robustness-decision-rule-2026-05-30.md`](../robustness-decision-rule-2026-05-30.md)):
   a strategy is FRAGILE if its 5th-percentile (p5) Sharpe across resampled
   histories is below zero — i.e. the bad-luck tail loses money. No moving the
   goalposts after seeing a surface.
2. **Block-bootstrap Monte-Carlo over *real* returns.** Every verdict is read off
   a *distribution* of plausible histories (resample real Binance returns into an
   ensemble; measure Sharpe percentiles, drawdown tail, probability of loss) — not
   a single lucky backtest. This is uncertainty quantification, not prediction.
3. **Byte-stable regression anchors.** Every result surface is locked by a
   body-SHA-256 anchor (a fingerprint of the report's deterministic body), so a
   number cannot silently drift between sessions. The gate now holds **119**
   anchors, and it passed live today (§ Numbers).
4. **Day-1 baseline-divergence falsifiers + an anti-cherry-pick renderer.** Every
   strategy ships with end-to-end tests proving its overlay actually changes the
   equity curve (no silent no-ops — the CLAUDE.md non-negotiable), each proven RED
   when its guard is reverted. The **FAMILY-UNIFORM** renderer reports the verdict
   over the *entire* parameter grid at once, so no single lucky cell can be
   cherry-picked as "the result."
5. **Cross-signal calibration on a live bar.** The on-chain probe was scored on the
   *identical* methodology and the *identical* "LIVE" bar that certified the basis
   signal — the basis passed because its sign held in both years; the stablecoin
   signal flipped. Same rule, opposite verdict. That symmetry is what makes the
   final negative airtight rather than defeatist.

Sources for the rigor: the frozen rule note above; the two basis decks
([v0.1.0](../../perp-basis-signal-robustness/presentations/perp-basis-signal-robustness-2026-06-06.md),
[v0.2.0](../../perp-basis-mn-spread/presentations/perp-basis-mn-spread-2026-06-08.md))
and their test reports; the on-chain spike note; and the program's terminal thesis
in [`product.md` § Strategy-library](../../product.md) (being finalized in parallel by
the analyst — this deck aligns to "active ≤ passive in the reachable universe; ship
passive").

## What you can do now

The program's deliverable is reproducible end to end. Each command is the exact
invocation; none mutates an anchored directory.

| Action | Command |
|--------|---------|
| Reproduce the **passive buy-and-hold control** the whole program was scored against (and which we now ship) — emitted as a row in any sweep | `cargo run -p backtest --features "candle realdata" --bin param_robustness_sweep -- --grid tier1 --paths 200 --year 2023 --out-dir /tmp/bh-verify/` |
| Confirm **every regression anchor is byte-stable** (the 119-anchor, three-channel audit trail) | `bash scripts/verify_anchors.sh` |
| Re-run the **on-chain go/no-go** that fired the hard-stop (PIT leak-check is the falsifier) | `cargo run -p data --example stablecoin_diag -- 2024 --leak-check` |
| Re-read the **terminal on-chain verdict** (net-flows PIT-killed → stablecoin fragile → hard-stop) | open `spec/dev-notes/onchain-netflow-spike-2026-06-08.md` |
| Re-read the **derivatives-positioning close-out** (basis ≡ funding; negative residual) | open `spec/perp-basis-mn-spread/presentations/perp-basis-mn-spread-2026-06-08.md` |
| Re-read the **frozen §0 decision rule** that every verdict was read against | open `spec/dev-notes/robustness-decision-rule-2026-05-30.md` |

## Live demo — passive vs the program's first-tested family, reproduced this session

To prove the headline ("passive dominates active") is live and not a stale quote, I
ran a fresh sweep this session on the **OHLCV cross-sectional momentum** family —
the *first* family the program ever tested — on real 2023 Binance data, end to end
through the block-bootstrap generator, the strategy, and the verdict classifier.
This is a **fast smoke at N=5 paths** (the anchored surfaces use N=200 — so the
per-cell numbers here are NOT the anchored figures; they exist only to prove the
binary genuinely produces this result live, and to show the buy-and-hold control
the program ships).

```
$ ./target/debug/param_robustness_sweep \
    --generator block-bootstrap-real --grid tier1 \
    --paths 5 --year 2023 --out-dir /tmp/capstone-demo/
param_robustness_sweep DONE
  report:         /tmp/capstone-demo/robustness-sweep-20260608-125412-v1-momentum-theta-surface-2023-block-bootstrap-real-fy.md
  body_sha:       410c6008761040695430b42605084510a3c74beb4be1af5c6afcf070c0dbb7dc
  wall_clock_s:   377.6
  n_cells:        6
  n_paths:        5
  family_verdict: FAMILY-UNIFORM-FRAGILE
  buyhold p50 Sharpe: 1.9262  P(loss): 0.0000  p95 MaxDD: 38.01%

  per-cell summary:
    g= 0 lookback=  60 k_long=3 drift=0.10 → FRAGILE | p50=-0.0014 p5=-0.0213 MaxDD_p95=80.4%
    g= 1 lookback=  24 k_long=3 drift=0.10 → FRAGILE | p50=-0.0070 p5=-0.0303 MaxDD_p95=88.3%
    g= 2 lookback= 168 k_long=3 drift=0.10 → FRAGILE | p50=0.0032 p5=-0.0320 MaxDD_p95=84.4%
    g= 3 lookback= 720 k_long=3 drift=0.50 → FRAGILE | p50=-0.0174 p5=-0.0248 MaxDD_p95=75.9%
    g= 4 lookback=  60 k_long=1 drift=0.10 → FRAGILE | p50=-0.0046 p5=-0.0614 MaxDD_p95=84.4%
    g= 5 lookback=  60 k_long=5 drift=0.10 → FRAGILE | p50=-0.0174 p5=-0.0320 MaxDD_p95=91.8%
```

Read it as the program in miniature: **passive buy-and-hold posts a p50 Sharpe of
+1.93 with a 0% probability of loss**, while the *best* active momentum cell posts
**+0.0032** and the rest are negative — passive beats the best active cell by
~600× on median Sharpe, and **every active cell is FRAGILE** (p5 below zero) even
before fees. This is the same shape every channel produced. Full stdout saved at
[`artifacts/program-capstone-2026-06-08/buyhold-vs-momentum-smoke-n5-2023.txt`](artifacts/program-capstone-2026-06-08/buyhold-vs-momentum-smoke-n5-2023.txt).
The `/tmp` demo report was deleted after capture — no anchored directory was
touched (`git status` of `reports/` clean; `anchors.toml` untouched).

## The terminal scorecard — every channel × family/arm, one view

This is the centerpiece: the complete cumulative record of the search. Every active
combination the program tested, across all three reachable channels, → FRAGILE,
dominated by passive buy-and-hold. The basis rows are the only ones that started
from a *live* signal; the on-chain rows are the final, pre-committed probe. (Each
active figure = best-cell median Sharpe (p50); the null each was read against is in
the last column. "BH" = buy-and-hold.)

| # | Channel | Method family / vehicle | Horizon(s) | Signal alive? | Best-cell p50 Sharpe | Verdict | Null |
|---|---|---|---|---|---:|---|---|
| 1 | **Price / OHLCV** | Cross-sectional momentum | 1h | No (rank-IC ≈ 0) | +0.014 | FAMILY-UNIFORM-FRAGILE | BH +1.74 |
| 2 | Price / OHLCV | Cross-sectional mean-reversion | 1h | No (rank-IC ≈ 0) | +0.007 | FAMILY-UNIFORM-FRAGILE | BH +1.74 |
| 3 | Price / OHLCV | Funding / carry (long-only) | 1h / 4h / daily | No | +0.039 → +0.065 | FAMILY-UNIFORM-FRAGILE | BH +1.74 |
| 4 | Price / OHLCV | Time-series momentum | 1h / 4h / daily | No | +0.047 → +0.169 | FAMILY-UNIFORM-FRAGILE | BH +1.74 |
| 5 | Price / OHLCV | (35-name universe spike) | 1h | No (beta ↓, IC still ≈ 0) | — | universe exonerated; still FRAGILE | BH |
| 6 | **Derivatives positioning** | Basis-reversal, **long-only** (v0.1.0) | 1h | **YES** (rank-IC −0.08…−0.11) | +0.051 | FAMILY-UNIFORM-FRAGILE even gross | BH +1.74 |
| 7 | Derivatives positioning | Basis-reversal, MN spread (v0.2.0) | 1h | YES, but ≡ funding | +0.041 | FAMILY-UNIFORM-FRAGILE | ≈0 cash |
| 8 | Derivatives positioning | Funding-carry, MN spread (v0.2.0) | 1h | ≡ basis (identical) | +0.041 | FAMILY-UNIFORM-FRAGILE | ≈0 cash |
| 9 | Derivatives positioning | **Basis⊥funding residual** (v0.2.0) | 1h | NO orthogonal alpha | **−0.005** | FAMILY-UNIFORM-FRAGILE (negative median) | ≈0 cash |
| 10 | **On-chain** | Exchange net-flows | daily | — | — | **PIT-INFEASIBLE (free)** — killed at the data gate | — |
| 11 | On-chain | Stablecoin supply (per-chain TS + dry-powder→BTC) | daily | NO (sign flips year-over-year) | — | **FRAGILE** — fails the cross-year LIVE bar | — |
| — | _Buy-and-hold (the bar, all OHLCV surfaces)_ | _passive — **the terminal product**_ | _all_ | _n/a_ | _**+1.74 (2023) / +1.10 (2024)**_ | _undefeated_ | — |

> **Sources, row by row.** Rows 1–5 (OHLCV) — the prior program decks
> ([time-series-momentum-robustness](../../time-series-momentum-robustness/presentations/time-series-momentum-robustness-2026-06-03.md),
> [horizon-retest-robustness](../../horizon-retest-robustness/presentations/horizon-retest-robustness-2026-06-05.md)).
> Row 6 (basis long-only) — the
> [v0.1.0 deck](../../perp-basis-signal-robustness/presentations/perp-basis-signal-robustness-2026-06-06.md)
> + its [test report § 5](../../perp-basis-signal-robustness/reports/test-2026-06-06-1200-perp-basis-signal-robustness.md).
> Rows 7–9 (MN spread, all three arms) — the
> [v0.2.0 deck](../../perp-basis-mn-spread/presentations/perp-basis-mn-spread-2026-06-08.md)
> + its [test report § 5c–5d](../../perp-basis-mn-spread/reports/test-2026-06-08-perp-basis-mn-spread.md).
> Rows 10–11 (on-chain) — the terminal
> [on-chain spike note](../onchain-netflow-spike-2026-06-08.md) §§ 1, 3.
>
> **The single-sentence arc:** every OHLCV signal was *dead at the signal level*
> and fragile as a strategy; the basis was the first signal *alive at the signal
> level* — and the market-neutral vehicle proved that aliveness IS the funding
> signal with no distinct residual; and the most-orthogonal remaining channel
> (on-chain) was either un-back-testable for free or fragile. **Three channels
> down; passive buy-and-hold undefeated.**

### The three findings that close each channel (the load-bearing facts)

- **Channel 1 — the price-ranking signal carries no information.** The decisive
  failure was information-theoretic: the cross-sectional *price*-ranking channel's
  rank-IC sits within ±0.07 of zero with no stable sign across 4 families and 3
  horizons. A 35-name universe spike ruled out "too few names" as the cause — beta
  dropped, IC stayed ≈0. The channel is dead at the signal level, not just
  fragile as a strategy.
- **Channel 2 — the one live signal is the funding signal, with a dangerous
  residual.** The basis-reversal signal is genuinely live (rank-IC −0.10,
  orthogonal, causal), but the market-neutral spread proved the basis ranking and
  the funding ranking **select the identical portfolio** — byte-identical result
  surfaces from *distinct data sources* through *distinct wiring* (verified real:
  funding mechanically prices the basis). The only part of basis that is *distinct*
  from funding — the basis⊥funding residual — has a **negative median Sharpe** and
  a 100% tail drawdown via hundreds of short liquidations. There is no orthogonal
  alpha to harvest, and the distinct part actively destroys capital.
- **Channel 3 — the orthogonal channel is un-reachable or fragile.** Exchange
  net-flows (the highest-prior on-chain signal) are **PIT-infeasible on free data**:
  CryptoQuant's own docs disclaim point-in-time accuracy ("historical data may
  change as new exchange wallets are discovered") — exactly the address-relabeling
  look-ahead pre-registered as the killer, confirmed verbatim by the vendor. The
  cleaner-PIT fallback (stablecoin supply, immutable mint/burn) cleared the data
  and leak-check gates cleanly but is **fragile**: its sign flips between 2023 and
  2024 at every horizon with magnitude (L=14d: +0.036 → −0.130), failing the same
  cross-year-replication bar the basis signal *passed*. Both branches hit the
  pre-committed fuse.

## What "ship passive" means — the terminal state

This is the operator-facing outcome. "Ship passive" is a **promotion of
already-built, already-anchored code**, not a build (on-chain spike § 6;
[`product.md` § Strategy-library](../../product.md)):

1. **Promote the existing buy-and-hold control to the production baseline
   strategy.** BH is the benchmark every robustness surface was scored against —
   the most-tested path in the repo. Shipping passive promotes it from "control"
   to "the strategy the paper-trading agent runs." No new strategy code, no new
   anchors.
2. **Record the conclusion in `spec/product.md`** — the active-edge search
   concluded NEGATIVE across three structurally-distinct channels (price,
   derivatives-positioning, on-chain); passive BH undefeated. (The analyst is
   finalizing this thesis in parallel; this deck aligns to it.)
3. **Re-anchor the program's win on the *methodology*** (product.md
   § Differentiator 5, "measured robustness, not asserted alpha"): the shippable
   deliverable is the robustness machine + the auditable three-channel negative — a
   complete, honest product. The machine correctly refused to certify ~10 plausible
   active bets (TCN, PatchTST, GARCH-σ, LLM-forecaster, 4 OHLCV families, 3
   derivatives vehicles) and left passive undefeated. That is the program working
   as designed.
4. **Keep the harness warm but idle** — the fetchers, surfaces, and the new
   `stablecoin_diag.rs` probe stay in place so any *future, fresh* program can reuse
   them. The hard-stop forecloses further hunting under *this* program (no options
   hunt, no macro hunt, no on-chain sub-signal mining).

## Numbers that matter

- **Anchors: 119 / 119 PASS** — verified live this session:
  `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (119 / 119)`. This is the
  program's complete audit trail: 119 byte-stable result surfaces across all
  tested channels (the 12 newest are the v0.2.0 market-neutral arm × fee × year
  surfaces). The regression gate stands intact at the terminal sign-off.
- **Passive vs active, the headline number (reproduced live today):** buy-and-hold
  p50 Sharpe **+1.93** with **P(loss) = 0.000**, vs the best active momentum cell
  p50 **+0.0032** — a ~600× gap on median Sharpe, every active cell FRAGILE even at
  0 bps fees (live N=5 smoke above; the anchored N=200 surfaces show the same shape
  with BH +1.74 / 2023, +1.10 / 2024).
- **The one live signal collapsed onto funding:** the basis-spread and
  funding-spread surfaces are **byte-identical on every metric**, both years, both
  fee levels — basis IS the funding signal (verified real via distinct data sources
  + distinct wiring). The basis⊥funding residual is **negative-median** (−0.064 on
  the 2023 best-gross cell) with 100% tail drawdown and 328 short liquidations:
  basis carries no orthogonal alpha. (v0.2.0
  [test report § 5c–5d](../../perp-basis-mn-spread/reports/test-2026-06-08-perp-basis-mn-spread.md).)
- **The on-chain hard-stop is doubly grounded:** net-flows killed by the **vendor's
  own PIT disclaimer** (not my inference); stablecoin supply **fragile** — its sign
  flips year-over-year (L=7d +0.011 → −0.086; L=14d +0.036 → −0.130), and every
  aggregate dry-powder→BTC cell sits inside its 2σ noise band. Calibrated against
  the basis spike on identical methodology — same bar, opposite verdict.
  (On-chain spike §§ 1, 3, 4.)
- **Spec-lint: structural integrity intact** — verified live:
  `python3 scripts/spec_lint.py` → `spec-lint: FAIL (94 violations in 2
  categories)`, **exit 0** (the "FAIL" string is the violation *summary*, not the
  script's exit status — the gate passed). The 94 (87 dead-link + 7
  trace-broken-path) are all pre-existing carry-over and **match the tester's PASS
  baseline exactly** (and are one category better than the audit-2026-06-01
  baseline of 95 / 3 categories). **Zero new violations, zero new categories**
  introduced by this capstone — no structural regression since the most recent
  tester `VERDICT → PASS`.

## Verification matrix — the program's claims

These are the program-level verification gates (V1..V6) for the terminal sign-off
— each maps to the cross-channel claim it backs, with one-line live or
committed-source evidence. (There is no single `feature.md` for a program capstone;
these consolidate the per-channel `## Verification` sections of the contributing
features.)

| V-id | Program-level claim | Status | Evidence |
|------|---------------------|--------|----------|
| V1 | The regression gate is intact at the terminal sign-off (119 anchors byte-stable) | VERIFIED | `verify_anchors.sh` live this session → `ANCHORS PASS (119 / 119)` |
| V2 | Channel 1 (price/OHLCV) — all 4 families × 3 horizons FRAGILE, dominated by BH | VERIFIED | Scorecard rows 1–5; [horizon-retest deck](../../horizon-retest-robustness/presentations/horizon-retest-robustness-2026-06-05.md), [TS-momentum deck](../../time-series-momentum-robustness/presentations/time-series-momentum-robustness-2026-06-03.md); live momentum smoke (BH +1.93 vs best cell +0.003) |
| V3 | Channel 2 (derivatives-positioning) closed — basis ≡ funding, residual negative-median, all vehicles fragile | VERIFIED | Scorecard rows 6–9; [v0.2.0 deck](../../perp-basis-mn-spread/presentations/perp-basis-mn-spread-2026-06-08.md) + [test report § 5c–5d](../../perp-basis-mn-spread/reports/test-2026-06-08-perp-basis-mn-spread.md) (byte-identical surfaces; −0.06 median residual) |
| V4 | Channel 3 (on-chain) — net-flows PIT-infeasible (free) + stablecoin fragile → hard-stop fired | VERIFIED | Scorecard rows 10–11; [on-chain spike § 1](../onchain-netflow-spike-2026-06-08.md) (CryptoQuant PIT disclaimer verbatim) + § 3 (sign-flip IC table); pre-committed fuse in [fork note](../onchain-vs-conclude-fork-2026-06-08.md) |
| V5 | The decision rule was frozen + pre-registered before results (anti-goalpost-moving) | VERIFIED | [`robustness-decision-rule-2026-05-30.md`](../robustness-decision-rule-2026-05-30.md) § 0 weakest-link bands; dated 2026-05-30, prior to all basis + on-chain work |
| V6 | "Ship passive" is a promotion of already-anchored code, not a new build | VERIFIED | BH control emitted as a row in every sweep (live demo stdout); [on-chain spike § 6](../onchain-netflow-spike-2026-06-08.md) "ship passive = promote already-built+anchored BH control" |

## Open decisions

There are **two ratifications** for the approval block, and no open fork. They are
bundled deliberately: this is a single terminal sign-off (conclude + ship passive
are one indivisible decision — concluding the search *is* shipping passive), and
you **pre-committed** the hard-stop in the
[on-chain-vs-conclude fork](../onchain-vs-conclude-fork-2026-06-08.md),
so this deck ratifies an already-executed conclusion rather than asking you to pick
between options. If you want to overturn the pre-commitment (e.g. open a *fresh*
paid-data or options program), that is a Reject with a note, and it routes back.

**Ratify (a) — the program conclusion.** That the active-vs-passive question is
answered for the **reachable universe**: across price/OHLCV, derivatives-positioning,
and on-chain, no active strategy beat passive buy-and-hold net of cost under the
frozen §0 rule; the one live signal (basis) survives in no vehicle and is the
funding signal with a negative orthogonal residual; the on-chain channel is
PIT-infeasible (free) or fragile; the pre-committed hard-stop has fired.

**Ratify (b) — ship passive.** Promote the already-built, already-anchored
buy-and-hold control from benchmark to the production baseline strategy (the path
the paper-trading agent runs), and record the three-channel negative as the
program's terminal thesis in `product.md`.

> **The honest scope of the verdict (read before you tick).** The claim is **"active
> ≤ passive in the *reachable* universe, net of cost, on the 2023-24 large-cap
> sample"** — it is **not** a universal law. Three caveats stated plainly:
> **(i) untested + lower-prior channels remain** — options-surface, macro, and
> social/sentiment were never probed (they were lower-prior and outside the
> free-data hunt; an operator may open any of them later as a *fresh* program, not
> a continuation of this one). **(ii) The two exhausted price/positioning channels
> are ~1.5 distinct information channels, not the whole universe** — basis collapsed
> onto its own funding mirror, so the program ruled out fewer *independent* signals
> than the row-count suggests. **(iii) The buy-and-hold bar is partly a bull-sample
> artifact** — 2023-24 caught a structural up-leg, so +1.74 Sharpe is a high,
> sample-specific bar; "passive won *this sample's* race" is the honest framing, not
> "passive is proven optimal in all regimes."
>
> **What "yes" commits you to:** the lightweight follow-on of promoting the BH
> control to the production baseline (no new code — it is already built and
> anchored) and recording the terminal thesis in `product.md`. It does **not**
> commit you to never researching again — it closes *this* hunt cleanly, with the
> 119 anchors standing as the durable, auditable record.

## Approval

The approval below ratifies the terminal program conclusion **and** ship-passive as
one decision (per § Open decisions). All boxes ship un-ticked; you are the only one
who ticks.

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback
_empty until operator fills — note any steer here (e.g. a future fresh options/macro
program is a separate decision, not a continuation of this hunt)_

## Feedback log
_empty — rejections/steers appended here and routed back to the named agent_

## Changelog
- 2026-06-08 (presenter): program-level capstone — terminal sign-off for the entire
  active-vs-passive research program. The pre-committed on-chain hard-stop fired
  ([`onchain-netflow-spike-2026-06-08`](../onchain-netflow-spike-2026-06-08.md):
  net-flows PIT-infeasible on free data per CryptoQuant's own PIT disclaimer →
  pivot to stablecoin supply → FRAGILE, sign flips year-over-year). Assembles the
  full three-channel scorecard: Channel 1 price/OHLCV (4 families × 3 horizons +
  universe spike, all FRAGILE, rank-IC ≈ 0); Channel 2 derivatives-positioning
  (basis-reversal the program's ONE live signal but long-only fragile, MN spread
  fragile in all 3 arms, basis ≡ funding byte-identically, basis⊥funding residual
  negative-median + tail-catastrophic); Channel 3 on-chain (net-flows PIT-killed,
  stablecoin fragile). Terminal verdict stated with HONEST scope: "active ≤ passive
  in the reachable universe, net of cost, on the 2023-24 large-cap sample" — NOT a
  universal law (untested options/macro/social noted; ~1.5 distinct channels, not
  the whole universe; BH bar partly a bull-sample artifact). Ship passive = promote
  the already-built+anchored BH control to the production baseline (a promotion, not
  a build). Live demo: N=5 momentum smoke reproduced BH p50 +1.93 / P(loss) 0.000
  vs best active cell +0.003 (~600× gap), every cell FRAGILE. Anchors 119/119
  verified live (`ANCHORS PASS (119/119)`); spec-lint live 94/2 = tester baseline
  (exit 0, no regression, +1 category better than audit-2026-06-01). Two
  ratifications bundled as one indivisible terminal decision (conclude + ship
  passive); no open fork (operator pre-committed the hard-stop). Approval block ships
  UN-ticked. FILES ONLY — orchestrator commits.
