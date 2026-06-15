---
slug: simple-strategy-bear-survey
mode: release
status: approved-shipped
audience: human-operator
updated: 2026-06-15
generated: 2026-06-15T00:00:00Z
---

# Simple-strategy bear-market survey — does ANY simple strategy beat passive in a real deep bear? — release

## TL;DR

We stress-tested ship-passive against the deepest bear we have (2021-22, the whole
universe down in 2022): **40 apparent winners surfaced, the best "beating" a −94%
crash by +97 percentage points — and path-resampling proved every one of them
fragile (luck, not edge). The deepest/widest bear evidence FIRMS ship-passive.**

## What changed

- Re-ran the four shipped simple strategies (SMA, MACD, RSI, BBands) over the
  just-shipped 2021-22 bear corpus (10 large-caps, hourly) and asked one
  pre-registered yes/no question: **does any of them show a path-robust edge in a
  real market-wide bear, or does this firm ship-passive?** Answer: **firms
  ship-passive — 16 of 16 tested candidates FRAGILE, none robust.**
- Added one re-runnable analysis harness (un-anchored, `#[ignore]`d — does not run
  in the default test suite, does not touch production code or anchors) and one
  `findings` dev-note recording the confirmed numbers.
- Amended the passive-baseline runbook with a dated 2026-06-15 BEAR-SURVEY callout
  — this is the operator-facing change you sign off on.

## Why

We just shipped two things that set this up: the 2021-22 bear corpus
(`data/binance-2122/`), and the overfit-guard, which showed the lone 2024
"down-market hedge" was path-luck on two hand-picked alt-coin dips. That finding
was honest but narrow — the corpus simply did not contain a real, market-wide
bear. Now it does (2022: BTC ≈ −64%, SOL ≈ −94%, AVAX ≈ −90%). The sharp,
pre-registered follow-up: across the *whole* deep bear universe, does ANY simple
strategy show an edge that holds up to re-ordering the same bars — or does
ship-passive firm? A robust survivor here would have been the most credible
non-passive signal the program has ever produced and would have **reopened** the
active-vs-passive question for a v0.2.0 trend-following product line. The whole
point of testing on the deepest bear is that a survivor *there* should most change
your mind. (See [`feature.md`](../feature.md) § The single sharp question.)

## What you can do now

This is an analysis finding, not a shipped runtime feature. "Doing" = re-running
the harness to reproduce the verdict, and reading the recorded conclusion.

| Action | Command |
|--------|---------|
| Reproduce the full two-stage survey (Stage 1 + Stage 2, ~2.3 min) | `cargo test -p backtest --release --test realdata_simple_strategy_bear_survey -- --ignored --nocapture` |
| Watch the long (>2 min) Stage-2 release run | `watch -n 30 'tail -n 20 /tmp/bear-A.log'` |
| Read the confirmed numbers + intuition + scope cap | `spec/dev-notes/analysis-2026-06-15-simple-strategy-bear-survey.md` |
| Read the runbook callout you are approving | `spec/runbooks/passive-baseline.md` (§ Real-data validation → BEAR-SURVEY 2026-06-15) |

## Live demo

The harness `--nocapture` stdout IS the deliverable (un-anchored analysis tooling;
this feature ships no binary). Per the run constraint on this deck, the output
below is quoted verbatim from the tester PASS report
([`test-2026-06-15-1200-simple-strategy-bear-survey.md`](../reports/test-2026-06-15-1200-simple-strategy-bear-survey.md)),
**Stage 2 — block-bootstrap results (N=500 per candidate)**, byte-identical across
two consecutive runs:

```
Cell                          Strategy   sharpe p5/p25/p50/p75/p95         prob_loss  P(sh>0)  VERDICT
SOLUSDT · 2022                RSI        -0.888/-0.122/0.430/1.041/1.948   0.310      0.690    FRAGILE
AVAXUSDT · 2022               RSI        -0.966/-0.186/0.424/1.089/1.848   0.312      0.688    FRAGILE
SOLUSDT · 2022                MACD       -2.182/-1.410/-0.871/-0.370/0.452 0.868      0.132    FRAGILE
SOLUSDT · 2022                BBands     -3.100/-2.302/-1.797/-1.210/-0.451 0.986     0.014    FRAGILE
AVAXUSDT · 2022               BBands     -2.800/-1.937/-1.313/-0.711/0.112 0.930      0.070    FRAGILE
SOLUSDT · 2022                SMA 20/50  -2.514/-1.590/-1.042/-0.483/0.305 0.890      0.110    FRAGILE
AVAXUSDT · 2022               MACD       -2.115/-1.290/-0.754/-0.208/0.438 0.836      0.164    FRAGILE
DOTUSDT · 2022                RSI        -1.474/-0.577/-0.055/0.379/1.041  0.534      0.466    FRAGILE
AVAXUSDT · 2022               SMA 20/50  -2.562/-1.756/-1.183/-0.532/0.453 0.880      0.120    FRAGILE
DOTUSDT · 2022                BBands     -1.148/-0.256/0.284/0.843/1.689   0.374      0.626    FRAGILE
ADAUSDT · 2022                MACD       -1.781/-0.668/0.027/0.682/1.744   0.482      0.518    FRAGILE
ADAUSDT · 2022                RSI        -1.219/-0.467/-0.031/0.527/1.240  0.512      0.488    FRAGILE
DOTUSDT · 2022                MACD       -2.799/-1.962/-1.520/-1.054/-0.370 0.984     0.016    FRAGILE
ADAUSDT · 2022                BBands     -2.821/-2.201/-1.759/-1.312/-0.613 0.994     0.006    FRAGILE
LINKUSDT · 2022               RSI        -1.118/-0.256/0.396/0.959/2.000   0.350      0.650    FRAGILE
ADAUSDT · 2022                SMA 20/50  -2.848/-1.985/-1.367/-0.796/0.055 0.942      0.058    FRAGILE
SOLUSDT · 2021 (up-contrast)  SMA 20/50  +0.439/1.428/2.059/2.660/3.485   0.012      0.988    MARGINAL
```

**Read this:** the frozen rule is "p5 Sharpe < 0 ⇒ FRAGILE" (p5 = the worst-5%
ordering; Sharpe = return per unit of risk; a negative Sharpe tail means the worst
re-orderings lose money). Every one of the 16 bear candidates has a negative p5 —
**all FRAGILE.** The headline winner, **SOL-2022 RSI** (the cell that "beat" a
−94.2% buy-and-hold by +97 pp on the one realized path), still has **p5 = −0.888**
and loses in ~31% of resampled orderings. The last row is the calibration proof:
the up-market control (SOL-2021 SMA, a real bull leg) lands **MARGINAL with a
POSITIVE tail (p5 +0.439)** — the test is reading regime direction, not rejecting
everything.

## Stage 1 — the hook (apparent winners, before path-testing)

On point returns these look like genuine bear alpha. **40 cells beat B&H by ≥ 10 pp
while B&H was negative — all from 2022.** Top by margin (full 80-cell table in the
tester report § 5):

| Rank | Cell | Strategy | B&H% | Strat% | Margin |
|---|---|---|---|---|---|
| 1 | SOLUSDT · 2022 | RSI | −94.2% | +2.8% | **+97.0 pp** |
| 2 | AVAXUSDT · 2022 | RSI | −90.2% | +1.7% | +91.9 pp |
| 3 | SOLUSDT · 2022 | MACD | −94.2% | −2.9% | +91.2 pp |
| 4 | SOLUSDT · 2022 | BBands | −94.2% | −4.9% | +89.2 pp |

This is precisely the trap the two-stage method exists to catch: in a −90% crash,
*almost any* strategy that happened to sit out the worst of it on the one historical
ordering looks heroic on point returns. Stage 1 deliberately concludes nothing; it
only selects the top-16 candidates for the real test (Stage 2 above).

## Screenshots

_n/a — non-UI feature. The deliverable is the harness `--nocapture` stdout (quoted
in Live demo) + the `findings` dev-note; there is no UI surface and no anchored
report (UN-ANCHORED per [`feature.md`](../feature.md) § Anchoring / D-BS.4)._

## Verification

V-ids map to the feature's acceptance criteria (AC-BS.1–AC-BS.9). Evidence cites
the tester PASS report
[`test-2026-06-15-1200-simple-strategy-bear-survey.md`](../reports/test-2026-06-15-1200-simple-strategy-bear-survey.md)
(verdict PASS, commit `4585cf9`).

| V-id | Description | Status | Evidence |
|------|-------------|--------|----------|
| AC-BS.1 | Stage 1 prints the full 80-cell point-survey table over `data/binance-2122/` | VERIFIED | test report § 5 — 20 rows (10 sym × 2 yr) printed, all cells ≥ 8747 bars, no thin-cell warnings |
| AC-BS.2 | Candidate set is explicit (predicate + threshold printed, count within cap) | VERIFIED | § 5 — predicate `bh_pct < 0 AND margin ≥ 10.0 pp`; 40 qualifying → top-16 kept (24 dropped); 16 ≤ cap=16 |
| AC-BS.3 | Stage 2 prints N, p5/p25/p50/p75/p95, prob_loss, dd, P(Sharpe>0), § 0 verdict per candidate | VERIFIED | § 5 Stage-2 table — 16 candidate rows + contrast row, all fields present (quoted in Live demo) |
| AC-BS.4 | SKIP-safe + `#[ignore]`d (does not run in / fail the default suite) | VERIFIED | § 3 — harness `#[ignore]`d, not in default count; default suite 8 passed / 0 failed |
| AC-BS.5 | Two consecutive `--release --ignored` runs are byte-identical (determinism) | VERIFIED | § 6 — `diff` of A vs B = **empty diff, PASS** (all 80 + 40 + 16 + contrast rows identical) |
| AC-BS.6 | Negative control: no mean-reverter ROBUST; up-market contrast discriminates | VERIFIED | § 7 — 9/16 candidates RSI/BBands, all FRAGILE; contrast SOL-2021 SMA p5 +0.439 MARGINAL vs all-negative bear p5 |
| AC-BS.7 | `findings` dev-note states per-candidate p5/prob_loss + folds into passive thesis | VERIFIED | [`analysis-2026-06-15-simple-strategy-bear-survey.md`](../../dev-notes/analysis-2026-06-15-simple-strategy-bear-survey.md) — authored, status `findings` |
| AC-BS.8 | Corpus + anchors untouched; no `spec/*/reports/` anchored file written | VERIFIED | § 9–10 — `git diff` empty on corpus/shipped harnesses; `verify_anchors.sh` = **ANCHORS PASS (119/119)** (quoted from test report § 10); no new `anchors.toml` row |
| AC-BS.9 | spec-lint = 70 zero-new; clippy clean; no `.unwrap()` outside test | VERIFIED | § 2 clippy zero warnings; § 11 spec-lint 70 (all pre-existing). Re-confirmed live for this deck (see Numbers) |

## Numbers that matter

- **Stage-2 verdicts: 16/16 candidates FRAGILE, 0 ROBUST, 0 MARGINAL** (bear cells).
  Up-market contrast: 1 MARGINAL (positive tail). Decisive null result.
- **Headline cell SOL-2022 RSI:** apparent margin +97.0 pp vs a −94.2% B&H →
  bootstrap **p5 Sharpe −0.888, prob_loss 0.310**. The biggest apparent edge in the
  corpus is path-luck.
- **Calibration proof — SOL-2021 SMA (bull leg):** p5 **+0.439** (positive),
  prob_loss 0.012, P(Sharpe>0) 0.988 → MARGINAL. Test discriminates regime; the
  all-FRAGILE bear result is signal, not a constant.
- **Stage 1:** 80 single-path backtests; 40 qualifiers (all 2022); top-16 advanced.
- **Bootstrap:** N=500 paths/candidate; block length Auto = 200–210 bars (no L≤1
  i.i.d. degeneration). Determinism: two runs byte-identical (empty diff).
- **Tests:** default `backtest` suite 8 passed / 0 failed; workspace 192 passed / 0
  failed / 5 ignored. Clippy zero warnings.
- **Anchors:** 119/119 PASS — un-anchored feature, no new row (quoted from tester
  report; not re-run on this deck — no builds run while assembling a presentation).
- **spec-lint:** 70 violations (65 dead-link + 5 trace-broken-path), all
  pre-existing, **zero new** — re-confirmed live while writing this deck:
  `spec-lint: FAIL (70 violations in 2 categories)`.

## Triangulation (why this is more than a re-run)

Three independent lines now point the same way:

1. **Corpus survey** (2023-24, 10 symbols): passive dominates the 18/20 up cells.
2. **Overfit-guard** (2026-06-15): the 2 hand-picked 2024 "hedge" cells are
   path-fragile.
3. **Bear-survey** (this, 2026-06-15): across a whole deep-bear universe — 40
   apparent winners, the single most spectacular +97 pp margin — none is
   path-robust.

This generalizes the overfit-guard from "the one 2024 hedge wasn't real" to "across
a market-wide bear, no apparent winner is robust." The 2026-06-08 terminal
ship-passive verdict is not reopened; it survived a deliberate stress-test on the
deepest bear we have.

## Honest caveat (the scope cap, stated at the level the evidence supports)

A null result **firms** ship-passive on the strongest available bear evidence; it
does **NOT** prove "no strategy can ever beat passive in a bear market." This is
still **hourly bars, shipped-default params, 4 simple strategies, 10 large-caps,
and the specific 2021-22 window** — not the space of all strategies. A per-symbol-
year FRAGILE verdict is a statement about path-resampling within that symbol's
2021/2022, not a cross-sectional "trend-following can never hedge a bear" claim (the
cap is symmetric — a ROBUST verdict would have been capped the same way). A ROBUST
survivor *would* have reopened the question; none appeared, so the question stays
closed and no v0.2.0 trend-following product line is greenlit off this evidence.

## Open decisions

1. **Approve this finding + the passive-baseline BEAR-SURVEY callout (→ ship), or
   route back.** A "yes" ratifies that the 2026-06-08 terminal ship-passive verdict
   stands on the strongest available bear evidence, and that the dated callout in
   `spec/runbooks/passive-baseline.md` § Real-data validation is the record. **No
   follow-up cost** is committed: no anchors to re-lock (UN-ANCHORED), no manual
   capture, no deferred question — the reopen path was the only thing a different
   result would have triggered, and it did not fire.

## Approval

- [x] Approved — ship — operator, 2026-06-15
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback
<empty until operator fills>

## Changelog
- 2026-06-15 (presenter): initial release-mode deck. Assembled from the tester PASS
  report (commit `4585cf9`), the `findings` dev-note, and the passive-baseline
  BEAR-SURVEY callout. Stage-2 verdict table quoted verbatim (16/16 FRAGILE incl.
  the +97 pp SOL-2022 RSI at p5 −0.888); up-market contrast SOL-2021 SMA MARGINAL
  (p5 +0.439) cited as calibration proof. Verification matrix maps AC-BS.1–9 to
  tester evidence. spec-lint re-confirmed 70 zero-new. Approval block ships
  UN-ticked. Awaiting operator approval.
