---
name: backtest
description: Run a historical backtest for a strategy feature and emit a metrics block for embedding into the test report. Use when a code review verifies a strategy change, when the analyst persona needs baseline performance, or whenever a feature's story (or its archived pre-BMAD feature brief) specifies a backtest scenario.
---

# backtest

## Inputs

- `feature_slug` — the feature's slug (story file `_bmad-output/implementation-artifacts/{epic}-{story}-<slug>.md`; pre-BMAD briefs archived at `docs/archive/pre-bmad-spec/**/<slug>/feature.md`)
- `scenario` — section within that story/brief, e.g. `btc-2023-regime`
- `baseline` — optional report path to diff against; defaults to most recent
  `evidence/<slug>/reports/test-*-<slug>.md` with a Backtest section.

## Procedure

1. Resolve the scenario: read the feature's story → `## Backtest Scenarios` block
   (for shipped pre-BMAD features the block lives in the archived
   `docs/archive/pre-bmad-spec/**/<slug>/feature.md`).
   Each scenario defines universe, period, data source, fees, and entry config.

2. Invoke the workspace's backtest binary (the architect defines this; default
   assumption `cargo run --release --bin backtest -- --scenario <scenario>`).
   Abort if the binary does not exist and route to architect/developer.

3. Capture the JSON metrics the binary emits. Validate it includes at minimum:
   `total_return, cagr, sharpe, sortino, max_drawdown, hit_rate, turnover,
   trades, avg_trade_pnl`.

4. Load the baseline report (if any) and compute deltas per metric.

5. Produce a markdown block matching the "Backtest Results" section of
   `rust-test/templates/test-report.md`. Embed it into the caller's report.

6. Raw artifacts land beside the report: with no `--reports-dir` the binary
   writes `backtest-runs/<feature>/reports/backtest-<stamp>-<scenario>.md` and its
   equity CSV under `backtest-runs/<feature>/reports/artifacts/<report-stem>/`
   (git-ignored, ADR-0104), never under `evidence/`. Run from the workspace root:
   a scenario that declares a baseline reads it from `./evidence` and refuses to
   run without it.

## Failure Modes

- Missing market data → `HANDOFF → analyst` with the data gap described.
- Backtest binary crash → `HANDOFF → developer` with stack trace.
- All metrics present but Sharpe < baseline − tolerance → flag as
  `REGRESSION` in the verdict; do not auto-route — the tester decides.

## Body-vs-front-matter discipline (HARD RULE)

The 119-anchor regression gate hashes the body of each report only —
the leading YAML front-matter is excluded. Use `scripts/hash_report.py`
to compute the canonical hash; never re-type a hash function inline.

Run-varying fields MUST go in the front-matter:

```yaml
---
scenario:       <name>
seed:           0xC0FFEE
generated:      <RFC3339 timestamp>     # excluded from hash
wall_clock_s:   <float>                  # excluded from hash
host:           <hostname>               # excluded from hash
agent_pid:      <int>                    # excluded from hash
git_commit:     <sha>                    # excluded from hash
binary_version: <semver>                 # excluded from hash
data_source:    <description>            # excluded if it can shift
run_id:         <uuid>                   # excluded from hash
---
```

The body holds: scenario params, metric values, ledger lines, equity
curve series — the deterministic outputs of the run. If you add a new
field that may differ between two equivalent runs, put it in the
front-matter or you will break an anchor.

## Anchor verification (after every run)

After the binary writes a report, run the regression gate:

```bash
scripts/verify_anchors.sh
```

A default run's report is in `backtest-runs/`, outside `evidence/`, so the
gate never sees it (ADR-0104): a PASS here says the committed corpus is
intact, not that the fresh run reproduces it. Compare a fresh body with
`python3 scripts/hash_report.py <report>` against its `anchors.toml` row.

- All 119 PASS → embed metrics in the test report and continue. A default
  run added nothing under `evidence/`, so there is nothing to prune; do NOT
  run `scripts/prune_backtest_duplicates.sh`, which is retired because it
  deleted committed anchored bodies (bug-log #228).
- FAIL on a scenario you just emitted into `evidence/` with `--reports-dir`
  → the body drifted. Diff the body
  bytes against the prior locked report (commands in
  `.claude/skills/verify-anchors/SKILL.md`). Most likely cause: a
  run-varying field leaked from front-matter into the body. Do NOT
  prune on FAIL — keep the divergent file for the developer's diff.
- MISS on a brand-new scenario the architect added → that's expected;
  run it twice into `backtest-runs/` (the default), compare the two bodies
  with `python3 scripts/hash_report.py`, and propose appending the SHA to
  `evidence/anchors.toml` (architect approves). Only then emit the report
  into the corpus with `--reports-dir evidence/<container>/<feature>/reports`.

## Templates

See [templates/scenario.md](templates/scenario.md) for the scenario-definition
format used inside a feature's story (historically `feature.md`).
