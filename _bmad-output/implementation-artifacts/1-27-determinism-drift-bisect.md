# Story 1.27: determinism-drift-bisect

Status: in-progress

<!-- Created 2026-09-25 by the operator's AC7 ruling on 1-26. 1-26 measured that its re-lock
     moved all 34 inventory surfaces and moved these four gates NOT AT ALL, which is the
     confirming measurement that the two movements are disjoint. Disclosure of record:
     bug-log #93 (the gate that can see drift) and #109 (why AC7 could not discharge it). -->

## Story

As the operator of the Honest Advisor,
I want the four `#[ignore]`d determinism gates' divergence traced to its actual cause by bisect,
so that the only gate in the repo that can observe code-vs-evidence drift is green because the
drift is understood — not because someone re-pinned it.

## Context — why this is its own story

`crates/backtest/tests/determinism.rs` carries four gates that **re-run** a scenario from real data
and compare its body-SHA against a pin. They are the **only** gate that can observe code-vs-evidence
drift: `scripts/verify_anchors.sh` hashes committed bodies and never re-runs, which is why it
reported 119/119 green while the code that produced those bodies had moved underneath them.

They are RED, and they are **right** to be. Story 1-26's AC7 asked for their pins to be re-derived
from its 34 regenerated surfaces; that turned out to be impossible, because **none of the four
scenarios is among the 34** (bug-log `#109`). The file said so itself at
`crates/backtest/tests/determinism.rs:722`, before anyone tried.

**Measured 2026-09-25, after the 1-26 re-lock, at `2d63ddef`** — all four still RED with hashes
*unchanged* across three measurements spanning the ADR-0089 D1 sizer wiring **and** the entire
34-surface re-lock:

| gate | pinned | produced 2026-08-22 / 08-23 / 09-25 |
|---|---|---|
| `top10-2023-1h-momentum` | `0f6f6eb8…` | `b655e5e7…` — byte-identical all three times |
| `top10-2024-h1-momentum` | `78976062…` | `37ce69e9…` |
| `top10-2023-fy-tcn-overlay` | `1460fcc7…` | `64f51802…` |
| `top10-2024-fy-tcn-overlay` | `b8e9186b…` | `908b66e1…` |

That stability is the finding that makes this tractable: **the divergence is deterministic and
reproducible**, so it is bisectable. It is not drift-in-progress.

## Acceptance Criteria

1. **Bisect to a first bad commit.** `git bisect` against `83378c5` — already known to be on the
   bad side, so the divergence predates the #67/#71/#75/#76 harness fixes. The lanes are
   `scenarios/momentum.rs` and `scenarios/tcn_overlay.rs`; `run_path` is **not** on their path
   (bug-log `#95`), so the 1-26 fixes are excluded by construction and must not be re-litigated here.
2. **Name the cause in one sentence, with `file:line`.** Not "something in this commit" — the
   specific change, and what it does to the body.
3. **Classify it.** Either (a) a defect, in which case it is fixed and the gates go green against
   their EXISTING pins; or (b) an intended behaviour change that was never re-locked, in which case
   the pins are re-derived under the ADR-0038 § D6.b protocol **with its 5 steps and its negative
   invariant**, exactly as story 1-26 did for its 34.
4. **`#[ignore]` is removed in the same commit as the resolution**, so the flip is visible in the
   diff that causes it.
5. **Do NOT re-baseline to current output to make the gates pass.** That converts a truthful
   regression gate into a rubber stamp — bug-log `#77`'s exact failure mode. A pin only moves
   through AC3 branch (b), with the reason written down.
6. **Report whether other lanes share the cause.** Bug-log `#95` lists eight lanes still declaring a
   `portfolio_exposure_cap` they cannot enforce. If the bisected cause touches them, say so; if it
   does not, say that too, with the check that was run.
7. Standing floor: anchors green before AND after; spec-lint PASS; the FROZEN gate files
   byte-untouched (AD-1).

## Investigation log — 2026-09-25

Recorded as it was measured, so the next reader inherits the exclusions rather than re-deriving them.

### The drift, reproduced and diffed

`t717_top10_2023_momentum` reproduces **exactly** outside the test harness: same tempdir shape, same
`--scenario … --seed 0xC0FFEE`, same `b655e5e7…`. Diffing the produced body against the anchored one
(`evidence/v5-latency-slippage-sim-v0.3.0-full-path-wiring/reports/backtest-20260527-181341-…`) shows
**8 changed lines, all in the summary table** — the θ/table structure is untouched:

| field | anchored | produced |
|---|---|---|
| Trades | 4809 | **592** |
| Buys / Sells | 2406 / 2403 | 296 / **296** |
| Max drawdown | 87.63 % | **14.34 %** |
| Final equity | $50 922.49 | $87 606.01 |
| Total fees | $3 640.60 | $2 255.39 |

An 8.1× turnover collapse with symmetric buys/sells and a drawdown that falls by 73 points.

### Excluded, each with its evidence

- **`crates/strategy/src/momentum.rs` — byte-unchanged** across the whole window. Signal generation
  is not the cause.
- **The only change in the lane's own file is a no-op here.** `scenarios/momentum.rs` gained exactly
  two lines: `sim_slippage_cost(…, &sig.symbol)`. In `sim.rs` the `symbol` argument is read **only**
  by the `SquareRoot` arm; the `Linear { bps }` arm ignores it, and this synthetic scenario runs
  `Linear{bps:8}` (`main.rs`: *"Synthetic: Linear{bps:8} fallback + no V map"*).
- **`#71`'s side-aware exposure cap — excluded by its own anchor note.** `Order::new` now caps the
  *resulting* signed position, but its comment states the arithmetic is byte-identical when
  `position_snapshot` is the empty placeholder. This lane passes exactly that
  (`Position::empty(sig.symbol.clone())`). Consistent with the measurement: the hashes did not move
  across `7884f52`.
- **`crates/risk` is not on this lane's path.** The loop calls `Order::new` + `engine.step`; it never
  calls `size_portfolio_target`.
- **"The pin was silently re-pointed at the sqrt-impact namespace" — checked and FALSE.** There is no
  `v5-sqrt-impact-2026-05` row for any of the four scenarios, and
  `evidence/v5-latency-slippage-sim-v0.5.0-square-root-market-impact/reports/` contains **0 files**.

That leaves the matching layer: `crates/backtest/src/engine.rs` (+2315) and `paper.rs` (+412).

### The boundary probe changed the question

Probing `e74204a9^` (2026-05-28) produced **`3b60ef07…`** — which is this scenario's
**`noop-baseline`** anchor row, *not* the pinned `0f6f6eb8…`. And `3b60ef07…` is precisely the SHA
the test's own doc-comment calls the *"stale noop-baseline SHA"* that was *"replaced with canonical
8-bps SHA"*.

`git log -S` dates that replacement: **`f089533e` (2026-05-31) — "engine-drift fix COMPLETE: re-lock
14 in-test anchors + close the regression-gate blind-spot"**.

So the timeline is: ≤05-28 the default binary produced the noop-baseline body → an engine-drift fix
landed → 05-31 the pins were re-locked to `0f6f6eb8…` → at some later point the output moved again to
`b655e5e7…`. **The known-good boundary is therefore `f089533e`, not the start of the candidate
window**, and the search space is the candidates dated after it.

**Probe (decisive): `f089533e` itself — MEASURED `0f6f6eb8…`.** The 2026-05-31 re-lock was honest:
the default invocation reproduced the pin the day it was set. The "pin was never reproducible"
hypothesis is **falsified**, and this is a real code drift with a proven known-good boundary.

### CAUSE FOUND — `11acd126` (2026-08-16), the `#67` fix

Bisected over the 26 path-filtered candidates in (2026-05-31, 2026-08-15], then narrowed:

| commit | date | produces | |
|---|---|---|---|
| `b0aeb172` | 2026-06-25 | `0f6f6eb8…` | GOOD |
| `452ce026` | 2026-07-25 | `0f6f6eb8…` | GOOD |
| `b1d96e72` | 2026-08-07 | `0f6f6eb8…` | GOOD |
| `cd7a5c7a` | 2026-08-14 | `0f6f6eb8…` | GOOD |
| `83378c59` | 2026-08-15 | `0f6f6eb8…` | **GOOD** — falsifies this story's own cited boundary |
| `b3332d35` | 2026-08-15 | `0f6f6eb8…` | GOOD |
| **`11acd126`** | **2026-08-16** | **`b655e5e7…`** | **BAD — first bad, and it is today's hash** |

`11acd126` is *"fix(1-25,#67): the harness was booking a ~1% gain for buying one symbol at another
symbol's price — engine guard + per-symbol fill routing"*.

**This resolves AC3 to branch (b), and it retracts the premise this story was written on.** The
pinned bodies are `#67`-contaminated; the gates are red because the engine became **correct**. There
is no defect to fix.

The reasoning error that hid it for six weeks: every artefact excluded `#67` because these four do
not go through `run_path`. True, and irrelevant — `#67`'s fix landed in `engine.rs` (+20) and
`paper.rs` (+98), the engine **below** every lane. The `t622_*` gates corroborate the mechanism from
the other side: single-symbol, stayed green throughout, because "one symbol at another symbol's
price" needs two symbols.

Full write-up and the widened-scope question: bug-log **`#111`**.

**Superseded hypothesis, kept so the correction is legible:**
- If it yields `0f6f6eb8…`, the 2026-05-31 re-lock was honest and the drift is a later commit —
  bisect the ~27 candidates between 2026-06-01 and 2026-08-15.
- If it yields anything else, the re-lock itself pinned a body the default invocation never produced,
  and this is a *pin* defect rather than an engine drift — which would also explain why no code fix
  has ever made these four green.

### Cost correction

This story's Dev Notes claimed "14 s per run — gated on thinking, not compute". That is right for the
**test run** and wrong for the **historical builds**: a commit from May pulls a full workspace
dependency rebuild (minutes each), and cargo keeps every old dependency version, so `target` grew to
60 GB during the first probe. The probe script now carries a disk brake (`cargo clean` below 30 GB
free). Budget the bisect as ~5 builds, not as a 14 s loop.

## Tasks / Subtasks

- [x] Reproduce all four RED locally and record the hashes. Done 2026-09-25; `t717_top10_2023_momentum`
      also reproduces outside the harness, which is what made the diff below possible.
- [x] Diff the produced body against the anchored one — 8 lines, all summary-table (see log above).
- [x] Exclude the obvious suspects with evidence rather than reasoning (see log above).
- [ ] Probe `f089533e` — the commit that re-locked the pins. Decisive between "engine drifted later"
      and "the re-lock pinned a body the default invocation never produced".
- [ ] Bisect the candidates dated after `f089533e`. `scripts/relock`-style driver + probe live in the
      session scratchpad; budget ~5 full builds.
- [ ] Name the cause (AC2) and classify it (AC3).
- [ ] Resolve per the branch taken; remove `#[ignore]` in the same commit (AC4).
- [ ] Check the `#95` lanes for the same cause (AC6).

## Dev Notes

- **Do not conflate this with the 1-26 re-lock.** 1-26 moved 34 θ-surfaces on `run_path`; these four
  run through lanes `run_path` never touches, and the measurement above proves the two movements are
  disjoint. Re-deriving one set of pins says nothing about the other.
- The four gates run in **14 s** once compiled. This is a cheap story gated on thinking, not compute.
- `scripts/callers.sh <symbol>` (CodeGraph ∪ grep) is the house tool for "who calls X" — `codegraph
  callers` alone undercounts by ~19 % here.

### References

- Predecessor: `1-26-harness-relock-regeneration` (AC7 could not discharge this — bug-log `#109`).
- Bug-log: `#93` (the gate that can see drift), `#95` (the eight unenforced-cap lanes), `#77` (why
  re-pinning is forbidden).
- Protocol if AC3 branch (b): ADR-0038 § D6.b, worked example in
  [`docs/dev-notes/1-26-d6b-re-emission-2026-09-25.md`](../../docs/dev-notes/1-26-d6b-re-emission-2026-09-25.md).
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 1.
