# `#129` — the threshold-sweep lane, measured in three states (2026-09-27)

## Provenance

Every SHA and number below was produced on this machine today. The runner is
`scratchpad/ts/run.sh`: it built `target/release/threshold_sweep` with
`--features candle,realdata`, ran each scenario with `--out-dir` pointing at a scratch
directory (never `evidence/`), hashed the emitted body with `scripts/hash_report.py`, and
swapped `crates/backtest/src/scenarios/threshold_sweep.rs` between the pre-fix and post-fix
source under an `EXIT` trap that restores the post-fix copy. Nothing in `evidence/` was
written. Raw results: `scratchpad/ts/results.tsv`.

**The ruling this executes** (operator, 2026-09-27): fix `run_cell` — per-symbol routing plus
the Bug-B solvency guard — and re-lock its anchors under ADR-0038 § D6.b. **The fix is done and
landed. The re-lock is NOT, and this note is the reason.**

## What was measured

| state | what it is | bs1 body SHA | bs2 body SHA |
|-------|------------|--------------|--------------|
| **ANCHOR** | locked 2026-05-21 at `447c0432`, before the `#67` engine fix | `551cc2ab3df85bff…` | `755bc3801359f199…` |
| **PRE-FIX** | HEAD (`46edb01c`) with `run_cell` untouched | `1dc4855f59e86b4f…` | `811f0c01aec94786…` |
| **POST-FIX** | HEAD (`8fd887f4`) with the `#129` fix | `04564535ea419357…` | `f7008296b23dedc1…` |

Six distinct digests for two scenarios. Two findings follow, and only one of them is about
my fix.

### 1. This lane stopped reproducing BEFORE today, and nothing noticed

`PRE-FIX ≠ ANCHOR` on both scenarios. The lane has been silently non-reproducing since some
commit between 2026-05-21 and now, while `verify_anchors.sh` reported 119/119 throughout —
because that gate hashes committed bodies and never re-runs a producer.

This is the **third** such family. Story 1-26 re-locked 34 θ-surfaces; story 1-27 found 15
more and bisected them to `11acd126` (the `#67` fix); these 2 are a third pair, and they were
outside both inventories. The cause here is **not yet bisected** and must not be assumed to
be `#67` — `#125` is the precedent for why: there, a drift that resembled `#67`'s signature
had a different cause, and folding it in would have destroyed the evidence.

### 2. The verdict had already flipped — before my fix

| state | verdict | v1 Sharpe (ann.) | best cell Sharpe |
|---|---|---|---|
| ANCHOR   | `T-MARGINAL`        | +0.003098 | +0.018254 |
| PRE-FIX  | `T-ALPHA-UNLOCKED`  | −0.328302 | +0.354852 |
| POST-FIX | `T-ALPHA-UNLOCKED`  | −0.693194 | +1.155769 |

The verdict rule is mechanical: `T-ALPHA-UNLOCKED ⇔ max_cell_sharpe − v1_sharpe ≥ +0.10`
(`bin/threshold_sweep.rs:228-235`, `max_sharpe_delta` at `:352`). Anchor:
`0.018254 − 0.003098 = 0.0152` → MARGINAL. Post-fix: `1.155769 − (−0.693194) = 1.8490`.

**Read the composition before reading the label.** Of the 1.8338 by which that delta grew,
**0.6963 — 38 % — comes from the CONTROL getting worse**, not from the strategy getting
better. A label that says alpha was unlocked, where more than a third of the movement is the
baseline deteriorating, is a label about the comparison and not about an edge.

And the FROZEN gate is not involved at all: neither `bin/threshold_sweep.rs` nor
`scenarios/threshold_sweep.rs` references `classify_verdict`, `verdict_bands`,
`compute_robustness_flag` or `rank_candidates` (grepped). `T-*` is this one report's private
vocabulary — a point estimate over a 9×5 grid with no bootstrap, no DSR, no multiple-testing
correction, and 45 cells to select the maximum from. It is not a gate-crowned edge and cannot
become one by being re-emitted.

## What I could NOT establish, and why that matters

`POST-FIX ≠ PRE-FIX`, so the fix does change results. **Which half of it does, I cannot show.**
The fix has exactly two behavioural effects — per-symbol fill routing, and the Bug-B solvency
pre-flight — and I have no measurement separating them:

- **The logger is silent.** Both new branches emit `tracing::warn!`, and the run produced
  **zero log lines of any level** (`grep -cE 'INFO|WARN|DEBUG'` → 0): no subscriber is
  installed in this bin and `RUST_LOG` was unset. I first read "0 refusals" as evidence that
  the silent-drop path was never taken. It is not evidence of anything — and worse, the
  post-fix code *cannot* log a refusal, because it never hands a foreign bar to the engine.
  An absence of output from a logger that was never installed is the exact shape this whole
  week's bug-log is about, and I nearly entered it as proof.
- **The report renders no order or trade counts.** So the artefact cannot answer it either.
  `#110` added a `trades` column to the MN surfaces for precisely this reason; this family
  never got one.

## Consequence: the code lands, the anchors do not move

The code fix is correct on its own merits, independent of the numbers: it adopts the ratified
`#67` seam (`last_bar_by_symbol`, the fill-side twin of `mark_prices`, mirroring
`montecarlo::run_path`) and it converts a silent `if let` drop into a reported refusal. Tests:
282 lib + 13 + 2 integration green; clippy clean on a forced re-lint.

The re-lock does **not** land, for three reasons, any one of which would be sufficient:

1. **AD-19.** A verdict flip that touches the thesis's supporting narrative escalates to the
   operator *before* publication. `T-ALPHA-UNLOCKED` on a retired research line is exactly that,
   and the operator's ruling was made before this flip was known.
2. **Re-pinning a number whose cause is unnamed is re-baselining** — bug-log `#77`. The PRE-FIX
   drift is unexplained; it needs a bisect, as story 1-27 did for its 15 rows.
3. **The old verdict is cited in artefacts a re-emission cannot silently change**:
   `trace.toml:372` (live), **ADR-0036:45 (accepted)**, and
   `evidence/v1/v25-tcn-horizon-bump-or-retire/reports/test-final-2026-05-21.md:75` — a body
   inside `evidence/`, which cannot be edited to match without breaking its own bytes. That is
   the same knot `#110` hit, and it is not dev's to untie.

## Next step

A bisect story for the PRE-FIX drift, in the shape of 1-27: find the commit, name the
mechanism at a `file:line`, then re-emit under § D6.b with the verdict flip stated in the
errata rather than discovered by a reader. Two instrumentation riders belong to it, because
without them the next measurement will be as blind as this one: install a subscriber (or state
that this bin has none), and render per-cell order/fill counts so the report can answer what
its own arithmetic did.
