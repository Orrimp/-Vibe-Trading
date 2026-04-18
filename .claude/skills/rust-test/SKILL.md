---
name: rust-test
description: Run the full Rust test suite and render a structured markdown report. Use after a build succeeds, on every PR, and whenever the tester agent is invoked. Produces a report at spec/reports/test-<timestamp>-<slug>.md using the template in this skill.
---

# rust-test

Canonical test-and-report pipeline.

## Procedure

1. Confirm the workspace builds (`rust-build` skill) — abort if not.

2. Run the full test matrix:

   ```bash
   cargo test --workspace --all-targets -- --nocapture
   cargo test --workspace --doc
   ```

3. If `proptest` or `quickcheck` crates are present, run them with a larger case budget:

   ```bash
   PROPTEST_CASES=1024 cargo test --workspace property_
   ```

4. Capture:
   - Pass/fail/ignored counts per crate.
   - Full output of any failing test (name + panic message + last 20 lines of output).
   - Wall-clock time.

5. Render the report by copying `templates/test-report.md` to
   `spec/reports/test-<YYYY-MM-DD-HHMM>-<slug>.md` and filling every section.
   Use the current UTC timestamp; derive `<slug>` from the active feature
   (read `spec/tasks/` for the most recently edited task file).

6. If backtests are in scope for this run, invoke the `backtest` skill and embed
   its metrics table in the "Backtest Results" section of the same report.

7. Print the report's verdict line and routing line as the last two lines of
   your response, exactly as written in the template.

## Template

See [templates/test-report.md](templates/test-report.md). It is the contract —
do not drop sections; if a section is empty, write `_n/a_` with a one-line
reason.

## Failure Modes

- Build failed → do not generate a report; emit `HANDOFF → developer` with the
  build error.
- Test binary panicked before any test ran → report it as a single failure
  in the "Environment" section and route to developer.
- Backtest data missing → fill the section with `_data unavailable: <reason>_`
  and route to analyst.
