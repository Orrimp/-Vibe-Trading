# Story 1.27: determinism-drift-bisect

Status: ready-for-dev

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

## Tasks / Subtasks

- [ ] Reproduce all four RED locally and record the hashes (a 14 s run once the test target is built:
      `cargo test -p backtest --test determinism --release -- --ignored top10_`).
- [ ] Bisect. The cheap probe is one gate, not four — `t717_top10_2023_momentum` is the fastest.
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
