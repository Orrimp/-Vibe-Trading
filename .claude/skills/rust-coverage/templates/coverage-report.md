---
slug: <feature-slug>
date: <YYYY-MM-DD>
owner: tester | operator
host: <hostname>
toolchain: <rustc --version output>
llvm_cov_version: <cargo llvm-cov --version>
prior_report: <path-to-previous-coverage-report-or-N/A>
---

# Coverage report — `<feature-slug>` — `<YYYY-MM-DD>`

## Run command

```bash
cargo llvm-cov --workspace --html --output-dir target/coverage
cargo llvm-cov --workspace --json --output-path target/coverage/coverage.json
```

Wall-clock: `<MM:SS>` cold / `<MM:SS>` warm

## Per-crate line coverage

Tier targets per SKILL.md frontmatter:

| Tier | Crates | v0.2 floor (informational at v0.1) |
|---|---|---|
| Hot | strategy / exec / audit / risk | 80% |
| Warm | backtest / forecast / data / replay-cache | 60% |
| Cold | everything else | none |

| Crate | Tier | Line % | Branch % | Functions % | vs target |
|---|---|---|---|---|---|
| strategy | Hot | _%_ | _%_ | _%_ | _OK / GAP_ |
| exec | Hot | _%_ | _%_ | _%_ | _OK / GAP_ |
| audit | Hot | _%_ | _%_ | _%_ | _OK / GAP_ |
| risk | Hot | _%_ | _%_ | _%_ | _OK / GAP_ |
| backtest | Warm | _%_ | _%_ | _%_ | _OK / GAP_ |
| forecast | Warm | _%_ | _%_ | _%_ | _OK / GAP_ |
| data | Warm | _%_ | _%_ | _%_ | _OK / GAP_ |
| replay-cache | Warm | _%_ | _%_ | _%_ | _OK / GAP_ |
| core | Cold | _%_ | _%_ | _%_ | _info_ |
| ui | Cold | _%_ | _%_ | _%_ | _info_ |
| agent | Cold | _%_ | _%_ | _%_ | _info_ |
| _other crates_ | Cold | _%_ | _%_ | _%_ | _info_ |
| **Workspace** | — | _%_ | _%_ | _%_ | — |

## Top-10 uncovered files (by line count)

| # | File | Lines | Uncovered | % | Tier | Notes |
|---|---|---|---|---|---|---|
| 1 | `crates/_/src/_.rs` | _N_ | _N_ | _%_ | _hot/warm/cold_ | _one-line context_ |
| … | … | … | … | … | … | … |

## Delta vs previous report (`<prior_report>`)

| Crate | Δ line | Δ branch | Δ fn |
|---|---|---|---|
| strategy | _+%_ | _+%_ | _+%_ |
| … | … | … | … |

If a crate dropped > 2% line coverage in the last 24h, the operator
should triage — likely a new untested module landed.

## Critical findings

Free-form. Examples:

- "`crates/strategy/src/vol_killswitch_overlay.rs` shows 100% line
  coverage but the e2e test (#[ignore]'d per bug-log #65) detects a
  no-op. Coverage is not detection — see rust-mutants for mutation
  score on the same file."
- "`crates/risk/src/kill_switch.rs` has 0% coverage — `risk` crate has
  no tests in workspace. Critical."

## Verdict (informational at v0.1; gating at v0.2)

- `OK` — all tier-targeted crates ≥ floor (v0.2+) OR all numbers
  documented (v0.1).
- `GAP` — at least one Hot/Warm crate below its target (v0.2+) OR a
  > 2% regression vs prior report.

## Routing

v0.1: report-only; operator reviews at their cadence.
v0.2+: `GAP` → orchestrator spawns developer with the top-3 uncovered
hot files as the punch-list.
