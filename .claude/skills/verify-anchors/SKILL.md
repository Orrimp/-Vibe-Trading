---
name: verify-anchors
description: Verify the 9 locked backtest body-SHA-256 anchors in spec/anchors.toml against the latest matching reports under spec/reports/. Use as a hard gate before VERDICT → PASS in any tester run that touched strategy, audit, exec, or backtest code. Also use after any change that modifies report rendering. A single FAIL routes HANDOFF → developer with the body diff.
---

# verify-anchors

Single-command regression gate for the 9-anchor body-only SHA-256 contract.

## Procedure

1. Run the verifier:

   ```bash
   scripts/verify_anchors.sh
   ```

   Exit code 0 = all 9 PASS. Non-zero = at least one FAIL or MISS.

2. On FAIL the script prints `expected`, `actual`, and the report file path.
   Compute the diff of the body bytes to localize the drift:

   ```bash
   # Compare two report bodies (front-matter stripped).
   diff <(awk '/^---$/{n++; next} n>=2' spec/<slug>/reports/<old>.md) \
        <(awk '/^---$/{n++; next} n>=2' spec/<slug>/reports/<new>.md)
   ```

3. On MISS (no report for a scenario) re-run the backtest first:

   ```bash
   cargo run --release --bin backtest -- --scenario <name>
   ```

   Then re-verify.

## Routing

- **All PASS** → run `scripts/prune_backtest_duplicates.sh` (see below),
  then tester proceeds with VERDICT line. Anchors are good.
- **FAIL** → `HANDOFF → developer` with the body diff. Most likely cause is
  a metadata field that should have been in YAML front-matter but leaked
  into the body (HF-1 / T715 pattern). The developer agent's body-vs-front-
  matter checklist exists for exactly this.
- **MISS** for a brand-new scenario the architect added → `HANDOFF →
  developer` to run it; once stable across two runs, append to
  `spec/anchors.toml`.

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

Without this step `spec/*/reports/` accumulates duplicate runs every time
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
scripts/hash_report.py spec/<feature>/reports/backtest-*-<name>.md  # compare two bodies
# If hashes match -> append to spec/anchors.toml under the new version.
```

## What this skill does NOT do

- Does not run backtests. That is `backtest`.
- Does not modify reports. They are append-only.
- Does not auto-update `spec/anchors.toml` on FAIL — that requires architect approval.
