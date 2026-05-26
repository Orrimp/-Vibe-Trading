---
slug: <feature-slug>
date: <YYYY-MM-DD>
owner: tester | operator
host: <hostname>
toolchain: <rustc --version output>
mutants_version: <cargo mutants --version>
prior_report: <path-to-previous-mutation-report-or-N/A>
---

# Mutation report — `<feature-slug>` — `<YYYY-MM-DD>`

## Run command

```bash
cargo mutants --package strategy --package exec --package audit --package risk \
  --jobs 8 --json target/mutants.json --output target/mutants 2>&1 | tee target/mutants.log
```

Wall-clock: `<MM:SS>`

## Per-crate score

| Crate | Total | Caught | Survived | Timed out | Unviable | Score |
|---|---|---|---|---|---|---|
| strategy | _N_ | _N_ | _N_ | _N_ | _N_ | _%_ |
| exec | _N_ | _N_ | _N_ | _N_ | _N_ | _%_ |
| audit | _N_ | _N_ | _N_ | _N_ | _N_ | _%_ |
| risk | _N_ | _N_ | _N_ | _N_ | _N_ | _%_ |
| **Hot-tier total** | _N_ | _N_ | _N_ | _N_ | _N_ | _%_ |

`Score = (caught + timed_out + unviable) / total`. Higher is better. Per
SKILL.md § "Interpreting the score": ≥90% strong, 75-89% acceptable, 50-74%
gap, <50% critical.

## Survived-mutation punch-list

For each survived mutant, one row:

| # | File:Line | Mutant | Why surviving (one-line hypothesis) | Triage owner |
|---|---|---|---|---|
| 1 | `crates/strategy/src/foo.rs:42` | `replace + with -` | No test exercises this branch with non-zero input | developer |
| 2 | … | … | … | … |

**Ranking**: order by perceived blast-radius (a survived mutant on the
audit-ledger write path > a survived mutant in a UI-only string formatter).

## Delta vs previous report (`<prior_report>`)

| | Δ caught | Δ survived | Δ score |
|---|---|---|---|
| strategy | _+N / -N_ | _+N / -N_ | _+%_ |
| exec | … | … | … |
| audit | … | … | … |
| risk | … | … | … |

If a row reports a new survivor in code shipped within the last 24h, the
operator should treat that as a regression signal and triage before any
further merge.

## Critical findings

Free-form. Examples:

- "Survived mutant at `strategy/vol_killswitch_overlay.rs:88` — `replace
  count += 1 with count -= 1` survives. This is the no-op pattern from
  bug-log #65; the killswitch counter increments but never affects
  output. e2e test confirms zero divergence."
- "Hot-tier crate `risk` has 0% mutation score because it has 0 tests.
  See bug-log entry to be filed."

## Flake candidates (state flipped between runs)

| File:Line | Run N-2 | Run N-1 | Run N | Probable cause |
|---|---|---|---|---|
| … | survived | caught | survived | timing-sensitive assert? |

Per SKILL.md § "Quarantine policy": flips between runs are candidate
flakes; confirm with `cargo mutants --regex '<file>:<line>' --jobs 1`.

## Verdict

- `PASS` — all crates ≥ 75% AND no NEW survived mutants vs prior report.
- `WARN` — at least one crate 50-74% OR at least one NEW survived mutant.
- `REGRESSION` — at least one crate < 50% OR critical-finding flagged.

## Routing

- `WARN` → orchestrator triages the punch-list; spawn developer for the
  top-3 highest-blast-radius survivors.
- `REGRESSION` → developer immediately; tester re-runs after the patch
  lands.
- `PASS` → operator-review only; no automatic spawn.
