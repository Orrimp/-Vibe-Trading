---
adr: 0102
title: The verification envelope is two legs, and Linux is now unbuilt anywhere
status: accepted
date: 2026-11-10
supersedes: none
superseded-by: none
---

# ADR-0102: The verification envelope is two legs, and Linux is now unbuilt anywhere

## Context

On 2026-11-09 the operator asked for the Ubuntu CI leg removed and CI made faster. It was
done the same evening in commit `4777982f` — and **without an ADR**, which is what this
document repairs. AD-18 is explicit that a non-trivial decision is a numbered ADR plus its
Registry row *in the same commit*, and that divergence from the architecture spine is
either a spine update or a handoff, never both missing. This one diverged in the strongest
possible way: **AD-13's Rule reads "the 3-OS (Linux/Windows/macOS) matrix is active"**, and
from `4777982f` onward that sentence was simply false.

The decision itself is the operator's and is not re-litigated here. What needs recording is
what it *cost*, because the leg was not a third copy of the same steps. **Eleven steps
carried `if: matrix.os == 'ubuntu-latest'` and ran NOWHERE else** — the entire governance
block (`verify_anchors.sh`, `spec_lint.py`, and the five other gates, each with its
`--self-test`), the repository's **only** clippy invocation, the `--no-default-features`
build, the ADR-0097 figure reproduction, and all four feature-gated backtest/forecast test
steps. Deleting the leg as literally asked would have removed AD-2's only remote
enforcement while the matrix went on reporting green.

Measured per-step from run `42575316185`, the last all-green 3-OS run: ubuntu 51.1 min,
macOS 59.4, Windows 82.5.

## Decision

**D1 — The matrix is `{macos-latest, windows-latest}`, and macOS is the canonical leg.**
It runs the full `cargo test -p ui` including all 56 visual baselines, every repo-governance
gate, and every feature-gated cargo invocation whose subject is the *code* rather than the
operating system. Windows runs build + non-visual `cargo test -p ui` + the workspace tests.

**D2 — Nine steps were re-homed to macOS; three died, and the reasons live in `ci.yml`
itself, not only in a diff.** The three: `Install Linux system deps` (apt + xvfb — nothing
left to install for); `Test ui (headless, Linux)` (an xvfb smoke of winit/tiny-skia on X11 —
a Linux property with no Linux leg; it took no *test* with it, because
`crates/ui/src/lib.rs` declares `pub mod fixtures;` unconditionally, so `--features
fixtures` gated only the `[[bin]]` targets and macOS runs all 116 integration targets);
`Report corpus-gated skips (informational)` (`|| true` on both commands and no assertion —
**it could not fail**, and its premise was false besides, since `data/binance` has been
tracked since 2026-09-27). Bug-log `#172` loses its subject and is annotated
CLOSED-BY-DELETION: retired, not diagnosed.

**D3 — Linux is now built nowhere, and the guard on that is a comment.** This is the one
property genuinely lost. `crates/ui/Cargo.toml` carries `x11` + `wayland` on iced with
`default-features = false`; with both off, `softbuffer`'s Linux backends are never selected
and the build fails with **25 errors** (task `#11`, 2026-08-15, reproduced in a
`rust:latest` container). macOS uses the Cocoa backend, so the dev box cannot see it and
never could — the ubuntu leg was the only thing that ever compiled that dependency graph.
Dropping either feature now produces **a clean CI and a broken build for anyone on Linux**.
The Cargo.toml comment says so in those words, and is labelled as what it is: *"a comment,
not a gate."*

**D4 — Dependency caching, which this file had refused since its first commit.**
`Swatinem/rust-cache@v2`, `save-if` restricted to `refs/heads/main` so PR runs restore but
never evict main's entry. The measurement that justifies it: `Build ui (fixtures)` cost
17.80 min on macOS and 18.22 on Windows from cold, nearly all third-party dependency
compilation, because the workspace root optimizes **all** dependencies at `opt-level = 3`
even in dev. Removing the fastest leg saves runner-minutes, **not wall clock** — Windows
paces the matrix either way, so the wall-clock win has to come from the cache.

**D5 — Runs serialize per ref.** A `concurrency` group of
`${{ github.workflow }}-${{ github.ref }}`, with `cancel-in-progress` true **for pull
requests only**: a superseded PR run is pure waste, but a `main` run is the record — it
carries the governance verdict and it writes the cache. Named trade-off: GitHub keeps one
running and one pending run per group, so three quick pushes supersede the middle one
without a verdict of its own. Acceptable because the newest commit is still verified and
the skipped one is its ancestor; `workflow_dispatch` is the lever when a specific
intermediate commit needs its own run.

**D6 — The governance interpreter is pinned and asserted.** `spec_lint.py` and
`check_determinism_anchors.py` import `tomllib` (3.11+) and declare `requires-python` in a
PEP-723 header that bare `python3` does not read. On Ubuntu the image shipped 3.12 and it
never came up; on one leg carrying the whole constitutional block, an inherited runner-image
property is not good enough. `actions/setup-python@v5` pinned to `3.12` (the interpreter
`uv` selects locally, so the hook and the job agree), plus `assert sys.version_info >= (3,
11)` as the step's first line so the pin's *removal* fails on the version instead of on
`ModuleNotFoundError` three commands later (bug-log `#189`).

**D7 — What this does NOT change.** ADR-0057's visual baselines stay **macOS-canonical**
and keep their in-source `#![cfg(target_os="macos")]` gate; this ADR neither supersedes nor
amends it. ADR-0097 § D6 reads *"CI runs it on the ubuntu leg only"* on the ground that
reproducibility is a property of the data and the engine rather than of the operating
system — that ground is unchanged and the step simply moved to the canonical leg, but see
Consequences for what is still **pending** rather than verified.

## Alternatives considered

- **Keep three legs** — rejected: the operator asked for the leg gone, and runner-minutes
  on the only leg nobody was reading are the cheapest thing to give up.
- **Delete the eleven steps along with the leg, as literally asked** — rejected: it removes
  AD-2's only remote enforcement, the ADR-0082 triad lint, and all of clippy, while the
  matrix keeps reporting green. That is the defect shape this repository's bug-log is
  mostly about.
- **Re-home the governance block to Windows instead** — rejected: the gates are bash +
  Python (`verify_anchors.sh` is a shell script), and Windows is the leg that paces the
  matrix at 82.5 min, so a constitutional failure would cost an hour instead of seconds.
- **A build-only Linux leg (`cargo check -p ui --features fixtures`, no tests)** — rejected
  *for now*, and deliberately costed here rather than lost: this is the **only** mechanism
  that would restore the D3 guard, it would have caught the `#11` failure, and on run
  `42575316185` the whole ubuntu leg was 51.1 min of which `Install Linux system deps` was
  0.23 — a check-only leg is a small fraction of that. It is not added because the operator
  asked for Ubuntu removed, not reduced. A future reader who wants the x11/wayland features
  guarded by something other than a paragraph should start here.
- **`cancel-in-progress: true` on `main` as well** — rejected: it would cancel the run that
  holds the governance verdict and writes the cache, which is the opposite of what D4 is for.

## Consequences

- AD-13 is amended by this ADR: the envelope is two legs, macOS canonical. Every
  current-state "3-OS" claim in the planning artifacts is corrected in the same commit;
  **historical** references in the bug-log, in other ADRs' incident narratives, and in story
  files are left alone, because rewriting them would falsify the record.
- A Linux regression in the `ui` dependency graph is now invisible until a human builds on
  Linux. D3 names the exposure and the alternatives section names the fix.
- **Pending, not verified** — the honest state at the time of writing. Run `42762269706`
  (on `4777982f`) was read once while in progress: `Governance gates` **success on
  macos-latest**, clippy **success** — the first time clippy has ever run on macOS. Still
  `pending` on that run when it was read: `Check ui minimal build`, `Test workspace`,
  **`Reproduce the sample figure (story 6-12)`**, the three feature-gated backtest steps,
  `forecast candle-gated tests`, and both `Test ui` steps. The figure reproduction is the
  one to watch: it hashes a body produced by running the binary, it has **only ever run on
  Ubuntu**, and nothing has yet demonstrated that it reproduces the anchored SHA on macOS.
  Until that step is green, "the move cost nothing" is a claim this ADR does not make.

## Changelog

- 2026-11-10 — accepted. Records the 2026-11-09 operator-directed narrowing shipped in
  `4777982f`, adds D5/D6 from the 2026-11-10 follow-ups, and amends AD-13.
