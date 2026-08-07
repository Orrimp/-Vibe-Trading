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

6. Save raw artifacts (equity curve CSV, trade log) under
   `evidence/<feature>/reports/artifacts/<run_id>/` so they survive for future diffs.

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

- All 119 PASS → run `scripts/prune_backtest_duplicates.sh` to collapse
  the just-written file into the canonical one-per-scenario set, then
  embed metrics in the test report and continue. (See
  `.Codex/skills/verify-anchors/SKILL.md` § "Post-PASS bookkeeping".)
- FAIL on a scenario you just re-ran → the body drifted. Diff the body
  bytes against the prior locked report (commands in
  `.Codex/skills/verify-anchors/SKILL.md`). Most likely cause: a
  run-varying field leaked from front-matter into the body. Do NOT
  prune on FAIL — keep the divergent file for the developer's diff.
- MISS on a brand-new scenario the architect added → that's expected;
  capture the SHA across two sequential runs and propose appending
  to `evidence/anchors.toml` (architect approves).

## Templates

See [templates/scenario.md](templates/scenario.md) for the scenario-definition
format used inside a feature's story (historically `feature.md`).
