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

4. On MISS (no report for a scenario) re-run the backtest first:

   ```bash
   cargo run --release --bin backtest -- --scenario <name>
   ```

   Then re-verify.

5. **D7.2 — engine-drift re-run gate (release mode; runs after verify_anchors.sh PASS):**

   ```bash
   # D7.2: synthetic re-run determinism tests in RELEASE (ADR-0045 § D7.2).
   # Release is mandatory — debug is multi-minute; release ≈ sma 9s, momentum 26s.
   cargo test --release -p backtest --test determinism -- t622_ t717_ tt1_
   ```

   These tests re-execute the engine from scratch and assert the output body-SHA
   equals the locked in-test constant. They guard against "engine moved but nobody
   updated anchors.toml" (the scenario that D7.1 alone cannot catch).

   **Routing on FAIL:**
   - If the failing test's constant matches `anchors.toml` → engine moved since the
     last anchor lock → HANDOFF → developer (re-lock or investigate regression).
   - If the failing test's constant does NOT match `anchors.toml` → stale in-test
     constant → run `python3 scripts/check_determinism_anchors.py --write` (or
     manually re-lock per ADR-0045 § D6).

   **Scope:** this step runs ONLY in the `verify-anchors` skill gate (pre-VERDICT),
   NOT in every `cargo test`. The full `cargo test --workspace` already includes
   these tests in debug; D7.2 ensures they also run in release before ship.

## Routing

- **All PASS** → run `scripts/prune_backtest_duplicates.sh` (see below),
  then tester proceeds with VERDICT line. Anchors are good.
- **FAIL** → `HANDOFF → developer` with the body diff. Most likely cause is
  a metadata field that should have been in YAML front-matter but leaked
  into the body (HF-1 / T715 pattern). The developer agent's body-vs-front-
  matter checklist exists for exactly this.
- **MISS** for a brand-new scenario the architect added → `HANDOFF →
  developer` to run it; once stable across two runs, append to
  `evidence/anchors.toml`.

## Post-PASS bookkeeping (mandatory)

After `verify_anchors.sh` exits 0, run:

```bash
scripts/prune_backtest_duplicates.sh
```

For each anchored scenario this keeps exactly one report on disk — the
oldest run whose body-SHA matches the locked anchor — and deletes every
other matching `backtest-*-<scenario>.md`. Idempotent: a run that
produced an identical body to the existing canonical file leaves the
canonical file's timestamp untouched (option C semantics — the
filename's timestamp records when the current canonical body was first
produced, not when last verified). Stale runs from before an anchor
update are also removed; their content lives in git history. Run with
`--dry-run` to preview.

Without this step `evidence/*/reports/` accumulates duplicate runs every time
the tester touches strategy code; with it the directory steady-states
at one file per anchored scenario.

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
cargo run --release --bin backtest -- --scenario <name>  # run 1
cargo run --release --bin backtest -- --scenario <name>  # run 2
scripts/hash_report.py evidence/<feature>/reports/backtest-*-<name>.md  # compare two bodies
# If hashes match -> append to evidence/anchors.toml under the new version.
```

## What this skill does NOT do

- Does not run backtests. That is `backtest`.
- Does not modify reports. They are append-only.
- Does not auto-update `evidence/anchors.toml` on FAIL — that requires architect approval.
