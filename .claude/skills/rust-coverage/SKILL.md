---
name: rust-coverage
description: >
  Run cargo-llvm-cov over the workspace and render a structured per-crate
  coverage report. Non-gating warning mode at v0.1 (operator reviews output;
  no threshold gate). Invoked on demand by the operator or the tester agent.
  Produces a report at spec/<slug>/reports/coverage-<YYYY-MM-DD>-<slug>.md
  using the template in this skill.
version: 0.1.0
phase: warning-only
gating: false
tier_targets:
  hot:  80   # strategy / exec / audit / risk  — floor at v0.2 soak
  warm: 60   # backtest / forecast / data / replay-cache — floor at v0.2 soak
  cold: none # all other crates — informational only at v0.1
roadmap:
  v0.1: skill ships, runs locally, prints findings; no threshold gate
  v0.2: 2-week soak complete; per-crate floors set from observed baseline data
  v0.3: CI / pre-commit gate enforcement (NOT this version)
references:
  - docs/dev-notes/archive/2026-Q2/testing-framework-audit-2026-05-25.md § R2
  - docs/dev-notes/archive/2026-Q2/testing-strategy-review-2026-05-25.md § §3 Coverage in this project's context
---

# rust-coverage

Source-based line and branch coverage via `cargo-llvm-cov`.

Cross-references:
- Architect recommendation: `docs/dev-notes/archive/2026-Q2/testing-framework-audit-2026-05-25.md § R2`
  ("Adopt `cargo-llvm-cov` in non-gating warning mode, 2-week soak, then
  per-crate floors — HIGH ROI")
- Analyst strategic framing: `docs/dev-notes/archive/2026-Q2/testing-strategy-review-2026-05-25.md § §3`
  ("Code coverage in context — Coverage is the floor, mutation testing is the
  ceiling. They answer different questions.")

**v0.1 — non-gating, warning mode.** No threshold gate is enforced. The skill
runs, reports, and routes to the operator for review. Per the architect's
sequencing, gating enforcement lands in v0.3 after a 2-week data soak (v0.2).

## Pre-flight

Confirm the workspace builds (`rust-build` skill) before proceeding. Coverage
instrumentation requires a successful compile; abort and route to developer if
the build is red.

## Installation

`cargo-llvm-cov` is a cargo sub-command, not a workspace crate dependency.
Install once globally:

```bash
cargo install cargo-llvm-cov --locked
```

Pin the version in your local environment notes. At the time of v0.1 authoring
(2026-05-26), the current stable release is `0.6.x`. Confirm `llvm-tools-preview`
is installed via rustup (see Failure Modes below).

## Procedure

### Step 1 — HTML report (human-readable)

```bash
cargo llvm-cov \
  --workspace \
  --all-features \
  --html \
  --output-dir target/coverage \
  2>/tmp/llvm-cov-run.log
```

Open `target/coverage/html/index.html` in a browser to browse by file.

Long-running warm: approximately 5 minutes on a warm build with Apple M-series.
Cold workspace (first run after `cargo clean`): approximately 25-35 minutes.

Watch recipe for the operator (emit this block when starting the coverage run):

```
watch -n 15 '
PID=$(pgrep -f "cargo-llvm-cov" | head -1)
[ -z "$PID" ] && echo "cargo-llvm-cov not running" && exit
ELAPSED=$(ps -o etime= -p $PID 2>/dev/null | awk "{gsub(/^ +/,\"\"); n=split(\$0,a,/[-:]/); if(n==2)print a[1]*60+a[2]; else if(n==3)print a[1]*3600+a[2]*60+a[3]; else if(n==4)print a[1]*86400+a[2]*3600+a[3]*60+a[4]}")
LINES=$(wc -l < /tmp/llvm-cov-run.log 2>/dev/null || echo 0)
echo "elapsed=${ELAPSED}s | log lines processed=$LINES | target/coverage/ exists=$([ -d target/coverage ] && echo yes || echo no)"
'
```

### Step 2 — JSON report (machine-parseable, for per-crate aggregation)

```bash
cargo llvm-cov \
  --workspace \
  --all-features \
  --json \
  --output-path target/coverage/coverage.json \
  2>>/tmp/llvm-cov-run.log
```

The JSON output follows the LLVM coverage export format. The `data[0].files`
array contains per-file summaries with `lines.count`, `lines.covered`,
`branches.count`, and `branches.covered` fields.

### Step 3 — Per-crate aggregation

Parse `target/coverage/coverage.json` to produce the per-crate summary table.
Group files by the crate they belong to (match their path prefix against
`crates/<name>/`). Sum `lines.count` and `lines.covered` across all files in
the crate; compute `pct = 100.0 * covered / count`.

Sort the resulting table: Hot-tier crates first (strategy, exec, audit, risk),
then Warm-tier (backtest, forecast, data, replay-cache), then Cold-tier
(everything else). Within each tier, sort by `pct` ascending (lowest coverage
first — the most actionable entries rise to the top).

### Step 4 — Top-10 uncovered files

From `target/coverage/coverage.json`, sort the `data[0].files` array by
`lines_uncovered` descending (where `lines_uncovered = lines.count - lines.covered`).
Take the top 10. These are the files where adding tests would recover the most
coverage points per unit of effort.

### Step 5 — Diff vs previous report

If a previous coverage report exists under `spec/<slug>/reports/coverage-*.md`,
extract its per-crate percentage column and compute the delta for each crate.
Annotate with:
- `+` prefix for crates that improved
- `-` prefix for crates that regressed (highlight these in bold in the report)
- `new` for crates that appear for the first time

If no previous report exists, write `_first run — no baseline to diff_`.

### Step 6 — Render report

Copy `templates/coverage-report.md` into
`spec/<slug>/reports/coverage-<YYYY-MM-DD>-<slug>.md` and fill every section.
Use UTC date only (no time component — coverage runs are not time-sensitive at
the minute level). Derive `<slug>` from the active feature (the most recently
edited `spec/*/tasks.md`).

## Tier classification

Per the architect's R2 recommendation and analyst's §3 strategic framing:

| Tier | Crates | v0.1 mode | v0.2+ floor |
|------|--------|-----------|-------------|
| **Hot** | `strategy`, `exec`, `audit`, `risk` | Warning — observe | 80% lines (architect R2 default; analyst recommends 90% — operator to decide at v0.2) |
| **Warm** | `backtest`, `forecast`, `data`, `replay-cache` | Warning — observe | 60% lines |
| **Cold** | `core`, `reflection`, `llm`, `agent`, `features`, `cost`, `models`, `reports`, `ui` | Informational | No floor (operator-decide at v0.2 with 2-week data) |

Note: the analyst's §3 Q3 default is tiered option (a) — Hot 90% + branch-level /
Warm 80% / Cool 60%. The architect's R2 defaults are slightly lower (85% for
load-bearing crates, 70% for others). The operator decides at v0.2 after the
soak. At v0.1 these tier definitions are documentation only, not enforcement.

**Branch coverage on Hot-tier crates** is the strategic priority per
`testing-strategy-review-2026-05-25.md § §3`: "Coverage is the floor, mutation
testing is the ceiling." For `strategy` and `exec`, both arms of every
`Signal/SignalKind` match must be hit to catch no-op wiring bugs of the v3
vol-targeting class. The `--branch` flag on `cargo llvm-cov` enables branch
reporting; include it in the JSON output step when available.

## Failure modes

| Symptom | Cause | Fix |
|---------|-------|-----|
| `error: no such sub-command: llvm-cov` | `cargo-llvm-cov` not installed | Run `cargo install cargo-llvm-cov --locked` |
| `error: component 'llvm-tools-preview' is not installed` | Missing rustup component | Run `rustup component add llvm-tools-preview` |
| OOM during instrumented build | Debug+coverage instrumentation is memory-heavy | Run with `CARGO_PROFILE_DEV_OPT_LEVEL=0` or limit parallelism with `--jobs 4` |
| `no coverage data found` for a crate | The crate has no tests at all | Note in the report; the crate needs test authoring, not a tooling fix |
| Coverage numbers differ between runs | Non-determinism in test execution order | Use `--test-threads=1` to serialise; flag in the report |
| Proc-macro crates show 0% | LLVM instrumentation does not cross proc-macro boundaries | Exclude via `--exclude <crate>` and note in the report |

## Cost

- Cold workspace (no prior build): approximately 25-35 minutes on Apple M-series.
- Warm rebuild (incremental, only changed crates re-instrumented): approximately 5 minutes.
- Do NOT run during a PR loop or blocking tester gate at v0.1. Run on demand or
  nightly after the workspace is already built.

## Output

The report is self-contained: per-crate table, top-10 uncovered files, tier
commentary, and diff-vs-previous. Route it to the operator for review. At v0.1
no automated routing decision is required; the operator decides where to invest
test authoring effort next.
