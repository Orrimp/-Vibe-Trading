# Story 6.12: evidence-reproducibility-sample

Status: done

<!-- Created 2026-08-04 by the adversarial product review (PRD §13 Q7; operator
     decision 2026-08-04: BUILD). Finding 8: every pinned corpus is machine-local
     and gitignored, so no second party can reproduce a single real-data claim. -->

## Story

As anyone who is not the operator — a future maintainer, a reviewer, a sceptic,
or the operator on a new machine,
I want to reproduce at least one of this product's real-data figures from a fresh clone,
so that "honest" means verifiable-by-someone-else, not merely honestly-intended.

## Acceptance Criteria

1. **A committed sample corpus exists.** A small slice of real pinned data (one symbol,
   one bounded window — sized to stay comfortably inside a git repo, LFS if warranted) is
   committed with its own `REVISION.toml` following the sibling-corpus convention, and the
   `.gitignore` exception is added in the same style as `data/binance/`.
2. **One documented figure reproduces end to end.** A single command from a fresh clone
   produces a named figure (a backtest headline number or an equity point) that matches a
   committed expected value within a stated tolerance — the recipe, the command, and the
   expected value live together in a runbook.
3. **It is honest about what it is NOT.** The runbook states plainly that the sample
   reproduces *one* claim, that the full corpora remain machine-local, and that the 119
   anchored bodies are verifiable from the repo alone (they already are — anchors hash
   committed report bodies) while the *runs behind them* are not re-executable by a third
   party. Do not let a sample corpus imply full reproducibility.
4. **It cannot rot silently.** The reproduce path is exercised by CI (or by the anchors
   gate) so that a drift in the engine, the loader, or the sample makes it fail loudly
   rather than becoming a stale recipe in a document nobody runs.
5. Standing floor: anchors 119/119 before AND after (the sample corpus must not disturb any
   existing anchored resolution); spec-lint PASS; the pinned-revision discipline
   (ADR-0032/0040 family) applies to the sample exactly as to the full corpora.

## Tasks / Subtasks

- [x] Choose the slice and the figure — **done 2026-09-25**: `data/yahoo-sample/BTC-USD/1d/2024/` (12 daily parquets, 96 KB) reproducing `btc-yahoo-2024-1d-sma-cross`.
- [x] Commit the sample + `REVISION.toml` + `.gitignore` exception; loader pin check passes — **done 2026-09-25**.
- [x] Runbook recipe with the honest-limits section — **done 2026-09-25**, [`docs/runbooks/evidence-reproducibility-sample.md`](../../docs/runbooks/evidence-reproducibility-sample.md).
- [x] CI wiring so it cannot rot (AC4) — **done 2026-09-25**, ubuntu leg.

## Dev record (2026-09-25)

ADR-0097.

| AC | What shipped |
|---|---|
| AC1 | `data/yahoo-sample/BTC-USD/1d/2024/` — 12 daily parquets, 96 KB, own `REVISION.toml`, plain git. **AC1's `.gitignore` premise was wrong** — see below. |
| AC2 | `cargo test -p backtest --features yahoo --test reproducibility_sample_figure` reproduces `btc-yahoo-2024-1d-sma-cross` to body-SHA `076929bb…`, the value anchored 2026-05-28. Exact, not within a tolerance — see below. |
| AC3 | The runbook's "What this is NOT" section, at length: one claim only, corpora still machine-local, the anchored bodies were ALREADY verifiable and it is the runs that were not, and the `#93` caveat. |
| AC4 | CI, ubuntu leg. Two tests: the corpus's manifest verifies (no feature flag, runs everywhere) and the figure reproduces (`yahoo` feature). |
| AC5 | anchors 119/119 before and after; spec-lint PASS; the sample's pin verifies through `read_and_verify_revision_manifest` exactly as the full corpora do. |

### Three places the evidence contradicted the ACs

1. **AC1's `.gitignore` premise is false.** It says to add the exception "in the same style
   as `data/binance/`". That style commits only `REVISION.toml` — `git ls-files data/`
   returns 13 files, all manifests, **zero** parquets. Following it literally would have
   committed no data at all. D1 uses a new pattern and the `.gitignore` comment explains
   why this corpus is the one exception.
2. **AC1's "LFS if warranted" is not warranted** at 96 KB, and `.gitattributes` has no
   `*.parquet` rule today. Plain git.
3. **AC2's "within a stated tolerance" does not fit.** The anchored body SHA is exact and
   already existed; inventing a float tolerance would be a weaker claim than the one
   available for free. The human-readable figure ($104560.08) is quoted in the runbook.

### A prerequisite had to ship first

Bug-log **#106**, found while scoping this: `run_yahoo_sma` did not pin the wall-clock it
prints into the hashed body, so the anchor was reproducible only on hardware fast enough
to finish inside 50 ms. Fixed in `bb3be261`. **Without it this gate would have been flaky
by construction** — which is why the scoping pass came before the build.

### The caveat that makes this story more useful than it was written

Measured 2026-09-24 (bug-log `#93`): several OTHER anchored scenarios **no longer
reproduce their own committed bodies**. The `btc-2023-1m-*` family emits hourly bars over
two years where the evidence records minute bars over one — 525601 → 17544 bars, final
equity $47290.03 → $107381.95. This gate is green precisely because it covers the scenario
that still holds, and the runbook says so rather than letting a green sample imply a
healthy corpus. Resolving `#93` belongs to story 1-26.
- [ ] Wire the reproduce check into CI so it cannot rot.

## Dev Notes

- Origin: product review 2026-08-04 finding 8 / PRD §13 Q7, operator decision BUILD.
- Companion to `docs/runbooks/corpus-restore.md`, which solves a different problem: that
  runbook restores the operator's own machine; this story lets a *second party* verify
  something. Cross-link both ways.
- Sizing discipline: this repo already carries LFS-tracked forecast checkpoints, so large
  binaries are not unprecedented — but the sample must stay small enough that a clone is
  not punished. Prefer one symbol × one month over anything broader.
- The existing pin machinery (`data::revision`, hardened by the story-1-12 review with
  completeness-at-emit and pinned-SHA skip) applies unchanged; do not fork it for the
  sample.

### References

- ADR: [`0097-one-figure-reproduces-from-a-fresh-clone.md`](../planning-artifacts/architecture/decisions/0097-one-figure-reproduces-from-a-fresh-clone.md)
- Runbook: [`docs/runbooks/evidence-reproducibility-sample.md`](../../docs/runbooks/evidence-reproducibility-sample.md)
- Trace: `REQ-EVIDENCE-REPRODUCIBILITY-SAMPLE-001` (state=`scoped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 6 (Remediation, Infra & Governance)
