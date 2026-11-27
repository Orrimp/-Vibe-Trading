---
name: verify-anchors
description: Verify the 119 locked backtest body-SHA-256 anchors in evidence/anchors.toml against the latest matching reports under evidence/**/reports/. Use as a hard gate before VERDICT → PASS in any tester run that touched strategy, audit, exec, or backtest code. Also use after any change that modifies report rendering. A single FAIL routes HANDOFF → developer with the body diff.
---

# verify-anchors

Single-command regression gate for the 119-anchor body-only SHA-256 contract.

## Procedure

1. **D7.1 — static drift-linter (sub-second, no engine execution):**

   ```bash
   python3 scripts/check_determinism_anchors.py
   ```

   Exit 0 = all non-cfg-gated `const ANCHOR` sites in `determinism.rs` match
   `evidence/anchors.toml` `v5-realdata-medium-2026-05` SHAs.
   Exit 1 = stale literal(s) detected → drift table on stderr → route to developer.

   Run this FIRST before the engine-executing steps below. It is fail-fast
   and sub-second (ADR-0045 § D7.1).

2. Run the file-anchor verifier:

   ```bash
   scripts/verify_anchors.sh
   ```

   Exit code 0 = all PASS. Non-zero = at least one FAIL or MISS.

3. On FAIL the script prints `expected`, `actual`, and the report file path.
   Compute the diff of the body bytes to localize the drift:

   ```bash
   # Compare two report bodies (front-matter stripped).
   diff <(awk '/^---$/{n++; next} n>=2' evidence/<slug>/reports/<old>.md) \
        <(awk '/^---$/{n++; next} n>=2' evidence/<slug>/reports/<new>.md)
   ```

4. On MISS (no report for a scenario) re-run the backtest first. A default
   run writes the git-ignored `backtest-runs/<feature>/reports/` (ADR-0104),
   which this gate never reads, so emitting into the corpus is explicit:

   ```bash
   cargo run --release --bin backtest -- --scenario <name> \
     --reports-dir evidence/<container>/<feature>/reports
   ```

   Then re-verify.

5. **D7.2 — engine-drift re-run gate (release mode; runs after verify_anchors.sh PASS):**

   ```bash
   # D7.2 (ADR-0045 § D7.2a, bug-log #229): the pins on the RELEASE binary cargo builds
   # for THIS command (`CARGO_BIN_EXE_backtest`); every spawn checks its profile;
   # `--show-output` prints the t229 line naming the binary the pins ran on.
   cargo test --release -p backtest --test determinism -- t229_ t622_ t717_ tt1_ --show-output
   ```

   Expected: `test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out`,
   and the captured line `t229: every non-builder spawn takes <ws>/target/release/backtest`.
   Measured 2026-11-25: libtest 5.4 s for the 18 tests, 9.6 s wall after an incremental
   compile; 118 s wall from a cold `target/` (2026-11-27, after `cargo clean`). The figures this
   step used to quote ("release ≈ sma 9s, momentum 26s") were debug numbers (ADR-0045 § D7
   Cost): until bug-log #229 this command spawned the debug binary.

   These tests re-execute the engine from scratch and assert the output body-SHA
   equals the locked in-test constant. They guard against "engine moved but nobody
   updated anchors.toml" (the scenario that D7.1 alone cannot catch).

   **Routing on FAIL:**
   - A `t229_spawned_backtest_is_this_profiles_build` failure, or any test failing with
     `backtest_bin()`'s panic (`bug-log #229: CARGO_BIN_EXE_backtest names no file` or
     `bug-log #229: this test runs from profile dir …`), means the binary the pins would
     spawn is not this command's build: read no t622/t717/tt1 result from this run;
     HANDOFF → developer to fix the harness, never the pins.
   - A failure of the other three `t229_` tests (static source scans) means the
     harness's wiring drifted (a new fixed path, spawn site or feature); the pins may
     still have run on this build. Fix the wiring before trusting the next run.
   - If the failing test's constant matches `anchors.toml` → engine moved since the
     last anchor lock → HANDOFF → developer (re-lock or investigate regression).
   - If the failing test's constant does NOT match `anchors.toml` → stale in-test
     constant → run `python3 scripts/check_determinism_anchors.py --write` (or
     manually re-lock per ADR-0045 § D6).

   **Scope:** this step runs ONLY in the `verify-anchors` skill gate (pre-VERDICT),
   NOT in every `cargo test`. CI (both legs) runs these tests in debug, on the debug
   binary; D7.2 runs them on the release binary before ship. Before the bug-log #229
   fix (2026-11-25) this command spawned whatever `target/debug/backtest` was on disk.

## Routing

- **All PASS** → tester proceeds with VERDICT line. Anchors are good. Do NOT
  run `scripts/prune_backtest_duplicates.sh` (see below).
- **FAIL** → `HANDOFF → developer` with the body diff. Most likely cause is
  a metadata field that should have been in YAML front-matter but leaked
  into the body (HF-1 / T715 pattern). The developer agent's body-vs-front-
  matter checklist exists for exactly this.
- **MISS** for a brand-new scenario the architect added → `HANDOFF →
  developer` to run it; once stable across two runs, append to
  `evidence/anchors.toml`.

## Post-PASS bookkeeping: RETIRED (bug-log #228)

Do NOT run `scripts/prune_backtest_duplicates.sh`. It was this section's
"mandatory" step, and it deleted committed anchored bodies: for each
`anchors.toml` row it kept one matching report and `rm -f`'d the others,
with no notion of namespaces, so a scenario's later rows lost the bodies
its earlier row did not keep (simulated over the committed tree: 155
tracked files, 31 of them bodies this gate resolves). Its `--dry-run` was
not a faithful preview. The script now refuses every invocation (exit 2).

Nothing replaces it. The corpus is byte-immutable (ADR-0038 § D6), so a
report is removed from `evidence/` only by an operator decision, by hand,
in a commit.

## When to invoke

Mandatory:
- Any tester run that touched `crates/strategy/`, `crates/audit/`,
  `crates/exec/`, `crates/backtest/`, or report rendering.
- Final tester gate before any version ship (T_FINAL_*).

Optional but cheap:
- After every developer fan-out merge, before declaring DONE.

## Adding a new anchor

Architect approves the new scenario; tester locks it once it is byte-
identical across two sequential `--release` runs at the same seed:

```bash
cargo run --release --bin backtest -- --scenario <name>  # run 1 -> backtest-runs/
cargo run --release --bin backtest -- --scenario <name>  # run 2 -> backtest-runs/
scripts/hash_report.py backtest-runs/<feature>/reports/backtest-*-<name>.md  # compare two bodies
# If hashes match -> append to evidence/anchors.toml under the new version, then emit
# the locked body into the corpus explicitly (ADR-0104 D2):
cargo run --release --bin backtest -- --scenario <name> \
  --reports-dir evidence/<container>/<feature>/reports
```

## What this skill does NOT do

- Does not run backtests. That is `backtest`.
- Does not modify reports. They are append-only.
- Does not auto-update `evidence/anchors.toml` on FAIL — that requires architect approval.
