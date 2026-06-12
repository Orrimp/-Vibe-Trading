---
name: rust-mutants
description: >
  Run cargo-mutants over the workspace's Hot-tier crates (strategy/exec/audit/risk)
  and render a structured mutation-score report. Non-gating warning mode at v0.1
  (operator reviews output; no threshold gate). Recommended cadence: nightly on
  `main` per analyst Q1 default (a) "adopt as nightly with operator punch-list".
  Produces a report at spec/<slug>/reports/mutation-<YYYY-MM-DD>-<slug>.md using
  the template in this skill.
version: 0.1.0
phase: warning-only
gating: false
scope_tier: hot   # strategy / exec / audit / risk only — workspace-wide is multi-hour
cadence: nightly  # operator-decide Q from spec/dev-notes/archive/2026-Q2/testing-strategy-review-2026-05-25.md
roadmap:
  v0.1: skill ships, runs locally, prints survived-mutation punch-list; no gate
  v0.2: extend to warm-tier crates (backtest / forecast); compare to baseline
  v0.3: CI / pre-commit gate enforcement (NOT this version)
references:
  - spec/dev-notes/archive/2026-Q2/testing-framework-audit-2026-05-25.md § Recommendations
  - spec/dev-notes/archive/2026-Q2/testing-strategy-review-2026-05-25.md § Q1 nightly cargo-mutants
  - spec/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md (the exact case this would catch)
  - spec/bug-log.md #65 vol_killswitch_overlay no-op (second exact case)
---

# rust-mutants

Mutation testing via `cargo-mutants` — answers the question "does my test
suite actually detect bugs?" Complement to `rust-coverage` (which only says
"what code did my tests exercise"). A high mutation score means the tests
will fail if the code changes its behavior; a low score means the tests pass
even when the code is broken.

Cross-references:
- Architect recommendation in
  [`spec/dev-notes/archive/2026-Q2/testing-framework-audit-2026-05-25.md`](../../../spec/dev-notes/archive/2026-Q2/testing-framework-audit-2026-05-25.md).
- Analyst Q1 in
  [`spec/dev-notes/archive/2026-Q2/testing-strategy-review-2026-05-25.md`](../../../spec/dev-notes/archive/2026-Q2/testing-strategy-review-2026-05-25.md):
  *"Nightly cargo-mutants on Hot tier (strategy/exec/audit/risk) — single
  highest-leverage change; would have caught the v3 no-op."*
- The exact bug class this catches:
  [`spec/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md`](../../../spec/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md)
  + [`spec/bug-log.md`](../../../spec/bug-log.md) `#65 vol_killswitch_overlay no-op`.

## Procedure

### 1. Install (one-time)

```bash
cargo install cargo-mutants --locked
cargo mutants --version  # confirm >= 25.0
```

`cargo-mutants` is a host dev-tool, NOT a workspace dependency. Do **not**
add it to `Cargo.toml`. Installs ~30 MB into `~/.cargo/bin/`.

### 2. Run — Hot tier only at v0.1

```bash
cargo mutants \
  --package strategy \
  --package exec \
  --package audit \
  --package risk \
  --jobs 8 \
  --json target/mutants.json \
  --output target/mutants 2>&1 | tee target/mutants.log
```

| Flag | Why |
|---|---|
| `--package` × 4 | Hot tier scope. Workspace-wide is multi-hour; out of scope at v0.1. |
| `--jobs 8` | Parallel mutant runs. Tune to `nproc - 2` if your host has fewer cores. |
| `--json` + `--output` | Machine-parseable + human-readable artifacts in `target/mutants/`. |
| `tee target/mutants.log` | Capture stderr too — useful when a mutant times out or panics. |

**Expected wall-clock**: 30-90 min for the 4 Hot-tier crates depending on
test-suite size. Run nightly via `cron` / `launchd` / `systemd timer` (per
analyst Q1 default — see `cadence: nightly` in this skill's frontmatter).

### 3. Render the report

Copy `templates/mutation-report.md` (in this skill dir) to
`spec/<feature>/reports/mutation-<YYYY-MM-DD>-<feature>.md` and fill every
section. The template enforces:

- Mutation score per crate: `(caught + timed_out + unviable) / total`.
  Caught = test failed when the mutant was introduced (good). Survived =
  tests passed despite the mutant (bad — actionable).
- **Survived-mutation punch-list** with `file:line` refs. This is the
  primary operator deliverable.
- **Newly-survived mutations vs previous baseline** (diff against the
  most recent prior report in the same `spec/<feature>/reports/` dir).
- **Timeout / unviable counts** — these are NOT failures but worth
  tracking; a sudden spike means a flaky test or a build break.

### 4. Failure modes

| Failure mode | Cause | Action |
|---|---|---|
| `--jobs N` crashes with OOM | Too many parallel cargo builds | Lower `--jobs` to 4 or 2. |
| Mutants timing out | Slow tests dominating | Add `--timeout 120` (default 60s). |
| `error: package not found` | Crate name typo | `cargo mutants --list-packages` to inventory. |
| Survived count surges between runs | Recently-landed code reduced coverage of a hot path | Inspect the new survivors first; they're the regression signal. |
| Report shows 0 caught | Probably running outside the workspace root | `cd` to repo root before invoking. |

### 5. Interpreting the score

| Score (caught / total) | Interpretation |
|---|---|
| ≥ 90% | Strong test suite; new code rarely lands a no-op. |
| 75–89% | Acceptable for v0.1 warning mode; surface punch-list to operator. |
| 50–74% | Coverage exists but isn't catching real bugs. The v3-vol-overlay no-op precedent fits this range. |
| < 50% | Critical gap. Tests are likely structural only, not behavioral. |

**Note**: the score is per-crate. A workspace-average hides crate-level
disasters. Always render the per-crate breakdown.

### 6. Quarantine policy

`cargo-mutants` is non-deterministic in two ways: parallel scheduling
affects which mutant ran when, and the test suite itself may have flakes.
v0.1 quarantine rule:

- A mutant that survived in **3 consecutive nightly runs** is a confirmed
  survivor. File a bug-log entry and a follow-up brief; do not treat as
  flakey.
- A mutant that flips state-of-survival between runs is a candidate flake.
  Run that single mutant 5× sequentially with `cargo mutants --regex
  '<file>:<line>' --jobs 1` to confirm.

## Cadence

| Cadence | Rationale |
|---|---|
| **Nightly on `main`** (default per analyst Q1) | Catches regression-class no-ops within 24h. Cost: ~1 hr/night of host CPU. |
| **Pre-merge on feature branches** | Out of scope at v0.1 — too slow for a PR gate. Revisit at v0.3 once mutation-budget is bounded. |
| **On-demand by tester agent** | When investigating a suspected no-op (per the `v3-vol-overlay-noop-discovery` pattern) — run with `--regex` filter to scope to a single file. |

## Operator-decide questions still open

- **Q-MUT-1**: Scheduling mechanism — host cron vs GitHub Actions vs a
  scheduled-tasks skill (`mcp__scheduled-tasks__create_scheduled_task`)?
  Analyst-recommended: **host cron at 02:00 local** (simplest; survives
  the host being offline; resumable).
- **Q-MUT-2**: Notification policy when survived-count > threshold —
  email? Stdout-only? Spawn a task chip via `mcp__ccd_session__spawn_task`?
  Analyst-recommended: **stdout-only at v0.1** (operator reads the
  report at their cadence; no push notifications).
- **Q-MUT-3**: Warm-tier expansion timing — at v0.2 (after 2-week soak)
  or sooner if Hot-tier reveals critical gaps? Analyst-recommended:
  **at v0.2 strictly** to avoid scope creep.

## Template

See [`templates/mutation-report.md`](templates/mutation-report.md). It is the
contract — do not drop sections; if a section is empty, write `_n/a_` with a
one-line reason.

## Failure modes

- `cargo-mutants` not installed → install per § 1; do NOT proceed with a
  stub report.
- Workspace build fails before mutation begins → fix the build first; mutation
  testing on a broken workspace is meaningless.
- Hot-tier crate has zero tests → mutation score is undefined; surface this
  as a critical finding in the report, NOT a 0% score.
