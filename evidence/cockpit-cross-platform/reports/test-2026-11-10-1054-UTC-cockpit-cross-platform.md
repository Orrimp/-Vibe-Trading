---
title: Test Report
feature: cockpit-cross-platform
run_id: 2026-11-10-0654-UTC
commit: 4777982fa2f99a6791f1e18b7f333595a88c6059
agent: orchestrator
verdict: PASS
---

# Test Report — cockpit-cross-platform — 2026-11-10 06:54 UTC

## 1. Scope

- **Feature / change under test:** the two-leg CI verification envelope — does the cockpit
  build and test green on every platform the matrix still covers? This is the closing
  evidence for story `6-9-cockpit-cross-platform`, whose AC1 was amended on 2026-11-10 by
  **ADR-0102** to *"both lanes of `{macos-latest, windows-latest}` green on a push to `main`"*
  after the operator's removal of the Ubuntu leg made the original wording
  (*"the Linux/Windows lanes go green"*) permanently unsatisfiable.
- **Spec refs:** `_bmad-output/implementation-artifacts/6-9-cockpit-cross-platform.md`;
  `_bmad-output/planning-artifacts/architecture/decisions/0102-the-verification-envelope-is-two-legs.md`;
  AD-13 in `_bmad-output/planning-artifacts/architecture.md`.
- **Commit SHA:** `4777982fa2f99a6791f1e18b7f333595a88c6059`
- **Rust toolchain:** `1.94.1`, pinned in `rust-toolchain.toml` per ADR-0091 — the pin is the
  reason this is reproducible at all; CI's `Install Rust stable` step honours it.
- **OS / arch:** `macos-latest` and `windows-latest` GitHub-hosted runners.
- **Provenance of this report:** it reports a **CI run**, GitHub Actions
  `42762269706`, read with `gh run view --json jobs` (logs are HTTP 403 from the dev box —
  see `docs/dev-notes/`-recorded constraint; per-step conclusions need no credential). It is
  **not** a local `cargo test` sweep, and nothing below is a local measurement unless it says so.

## 2. Static Analysis

| Check              | Result | Notes                       |
|--------------------|--------|-----------------------------|
| `cargo fmt --check`| _not run_ | Not a step in `ci.yml`. Honest gap, not a pass. |
| `cargo clippy`     | **PASS** | Step 6, `macos-latest`. **The first time clippy has ever run on macOS** — it was an Ubuntu-only step until 2026-11-09. Skipped on Windows by design (`matrix.os == 'macos-latest'`). |
| `cargo audit`      | _not run_ | Not a step in `ci.yml`. |
| `cargo deny`       | _not run_ | Not a step in `ci.yml`. |

Governance gates (step 3, macOS): **PASS** — `verify_anchors.sh` 119/119, `spec_lint.py`
plus its self-tests, and the five other constitutional gates each with `--self-test`.

## 3. Unit & Integration Tests

Reported per CI **step**, which is the unit this run actually measures; a crate×test matrix
would be a shape this evidence does not have.

| Step | macos-latest | windows-latest |
|------|--------------|----------------|
| Build ui (fixtures) | **success** | **success** |
| Check ui minimal build (`--no-default-features --features fixtures`) | **success** | skipped (canonical leg only) |
| Test workspace (non-ui crates) | **success** | **success** |
| Reproduce the sample figure (story 6-12 / ADR-0097) | **success** | skipped |
| backtest yahoo-gated ticker-flag tests (`#184`) | **success** | skipped |
| backtest realdata-gated p2 verdict rerun (`#186`) | **success** | skipped |
| backtest realdata-gated threshold-sweep read-only guard (`#186`) | **success** | skipped |
| forecast candle-gated tests (`#163`) | **success** | skipped |
| Test ui — full suite incl. 56 visual baselines | **success** | skipped (`#![cfg(target_os="macos")]`) |
| Test ui — non-visual | skipped | **success** |
| **Job totals** | **19/19 steps, 0 failures** | **0 failures** |

### Failing Tests

_none_ — both jobs concluded `success`; `gh run view --json jobs` reports zero steps with
conclusion `failure`, `cancelled` or `timed_out` on either leg.

## 4. Property / Fuzz Tests

_n/a_ — this run exercises a build-and-test matrix, not a property suite.

## 5. Backtest Results

_n/a as a performance measurement_, and deliberately so: nothing in this change touches
strategy logic, and no metric moved. One backtest-adjacent **determinism** gate is
load-bearing here and is reported instead:

**`Reproduce the sample figure` (step 10) — PASS, and this was the run's real risk.** The gate
runs the binary over the committed 96 KB `data/yahoo-sample/` slice and hashes the report body
with `scripts/hash_report.py`, the same function `verify_anchors.sh` uses, asserting it
reproduces an anchored body-SHA locked on 2026-05-28. It had **only ever run on Ubuntu**. Its
ADR (0097 § D6) justifies one leg on the ground that reproducibility is a property of the data
and the engine rather than of the operating system — a reasonable argument that had never been
tested, because no second OS had executed it. It now has.

## 6. Benchmarks

_No benchmark suite ran._ One wall-clock measurement is worth recording, because ADR-0102 § D4
promised it and the number does **not** flatter the change:

| leg | this run | last cold 3-OS run (`42575316185`) | Δ |
|---|---:|---:|---:|
| `macos-latest` | **74.5 min** | 59.4 min | **+15.1** |
| `windows-latest` | **86.4 min** | 82.5 min | **+3.9** |

**Both legs were SLOWER than the cold baseline, on the first run with caching enabled.** Read
honestly, that is what a cold cache predicts — the run has nothing to restore and must still
pay the `Post Cache` save — and for macOS most of the delta is *work that moved onto the leg*:
the nine re-homed steps cost ≈12.9 min on Ubuntu by the per-step figures recorded in `ci.yml`
(governance 0.12 + clippy 4.58 + minimal 1.25 + figure 5.22 + yahoo 0.02 + forecast 1.72, plus
two realdata steps whose Ubuntu cost is unknown because they are new). So macOS's +15.1 is
roughly the re-homed work and **not** evidence the cache hurt; Windows's +3.9 on an unchanged
step list is the clearest look at the save cost. **The cache's wall-clock claim remains
unproven either way — the measurement that settles it is the NEXT run on `main`**, which is
the first that can restore. Nothing here should be cited as a cache win.

## 7. Environment / Infrastructure Issues

**The scope limitation is the finding, not an aside: Linux was not tested, and cannot be.**
This run covers two platforms because the matrix covers two. `crates/ui/Cargo.toml`'s `x11` +
`wayland` iced features remain load-bearing for a Linux build (25 errors without them, task
`#11`), macOS uses the Cocoa backend so the dev box cannot see a regression, and the dropped
Ubuntu leg was the only thing that ever compiled that dependency graph (ADR-0102 § D3). A
green verdict below is therefore **not** a statement that the cockpit works on Linux.

No flakes, no infra outages, no data gaps. The pinned `data/binance` corpus and the LFS
checkpoint anchors both resolved (`lfs: true` on checkout).

## 8. Verdict

**`PASS`** — for the envelope that exists, and explicitly not for the one the story originally
named. Both legs of the live matrix concluded `success` with zero failing steps; every one of
the nine steps re-homed from Ubuntu on 2026-11-09 is green on the leg that now owns it,
including the three that had never run anywhere but Ubuntu — clippy, the minimal
`--no-default-features` build, and the ADR-0097 figure reproduction. The governance block,
the 56 visual baselines and anchors 119/119 all hold. What this run does not and cannot say is
anything about Linux; story `6-9` closes as **`shipped-partial`** for exactly that reason,
with Linux portability priced as a separate backlog item owning ADR-0102's costed build-only
`cargo check` leg.

## 9. Routing

`VERDICT → PASS` — ready to ship, scope as stated in § 7 and § 8.
