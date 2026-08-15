# Session handoff — 2026-08-15

**Everything below is uncommitted.** The operator was away from keyboard and commits were withheld
by instruction. Nothing here has been pushed; the tree is green and reviewable as-is.

**Gate state at handoff:** `ANCHORS PASS (119 / 119)` · `spec-lint: PASS (0 violations)` ·
FROZEN AD-1 files (`robustness.rs`, `rank.rs`, `bootstrap.rs`) byte-untouched · `backtest --lib`
225 passed / 0 failed.

---

## 1. What changed in the tree

| file | what |
|---|---|
| `crates/agent/src/runtime.rs` | **#80 forward half** — short legs now route through the engine (+210/−93) |
| `crates/agent/tests/short_long_friction_parity_forward_e2e.rs` | **new**, 4 assertions, non-vacuous |
| `crates/backtest/src/bakeoff/mod.rs` | `dvol_arm_compiled()` + extended arm-count predicate (#81) |
| `crates/ui/src/leaderboard/runner.rs` | arm-count honesty fix + test derives its count (#81) |
| `.github/workflows/ci.yml` | **#84 fix** — `shell: bash` restores `pipefail` |
| `CHANGELOG.md`, `trace.toml`, `advisor-end-to-end-demo.md` | claim corrections |
| `docs/dev-notes/bug-log.md` | disclosures #74–#91 |
| `docs/dev-notes/{codebase-overview,reachability-map,claims-ledger,feature-reachability-audit}-2026-08-15.md` | **new** — the coverage artifacts |

---

## 2. Fixed this session

- **#84 — the one that mattered most.** GitHub's default `run:` shell is `bash -e {0}`, *without*
  `pipefail`, so `verify_anchors.sh | tail -1` exited with **tail's** status — always 0. AD-2's only
  *remote* enforcement had been inert since CI activation. One line (`shell: bash`), plus a
  load-bearing comment so it is not "tidied" away.
- **#80 — both halves now closed.** The forward paper loop (the path that runs the operator's actual
  plan, not merely the ranking bake-off) bypassed the engine on short legs exactly as the ranking
  side had. Now sized via `plan_open_short`, stepped inline through `engine.step`, accounted in the
  one existing per-fill block. Measured: slippage **+41.4 %**, total friction **+10.75 %**, aggregate
  rate now exactly 2 bps; long-only control byte-identical to the last digit.
  **Structural result: `try_open_short`/`try_cover_short` have zero production call sites
  workspace-wide** — the self-accounting seam is gone, not merely patched twice.
- **#79** (config value never reaching the engine) and **#81's honesty half** (arm counts shown to
  the operator counted arms that cannot run).

## 2b. Bug-log decisions worked (2026-08-15, second pass)

| # | outcome |
|---|---|
| **#86** | **FIXED** — wrote the missing `0079-*.md` (reconstruction from primary sources, labelled), then landed the bidirectional registry check `(d)`, RED-proven. Also fixed a defect *inside the gate*: `--self-test` exercised a near-identical COPY of the production function, so the lint's tests never guarded the lint's code. |
| **#90** | **OPTION 1 APPLIED** — carve-out documented at source + a caller-census gate (`liquidation_carve_out_census.rs`), RED-proven, with a >100-files non-vacuity assert. A fill-tape gate can never see this path, so the caller set is the only guardable thing. |
| **#89** | **PARTLY FIXED** — the predicted tautology existed: `t24_deterministic_across_runs` used the SAME seed twice. RED-proven vacuous (mutated to 999_999, still passed), replaced with an assertion of the real invariant — fills are seed-INDEPENDENT — which is falsifiable the moment anyone wires the RNG. |
| **#87** | **HALF-FIXED** — silent `let _ = ledger;` off-path now warns loudly with the rebuild command. Not test-gated (no log-capture harness exists; said so rather than implying otherwise). |
| **#85** | **INTERIM APPLIED** — ⚠️ NOT ENFORCED annotations on both fields + both live config files; zero behavioural change. One evidence leg **corrected**: the runbook citing them as kill-switch triggers is in `docs/archive/` (frozen history), not live. |
| **#83** | **STAGED for ruling** — [decision memo](decision-83-frozen-gate-fails-open-2026-08-15.md). Much easier than it looked: the correct treatment already exists in `sweep.rs`, whose comment asserts agreement with the leaderboard that is **false**. One file, one function. |
| **#92** | **NEW** — the documented `--no-default-features` build does not compile. **Blocks #91**, whose fix cannot be compile-verified until it does. |

## 3. Still awaiting an operator decision (after the second pass)

| # | what is left for you | why it needs you |
|---|---|---|
| **#83** | apply the `is_eligible` patch — **allow-list** form recommended | AD-1 ruling: `rank.rs` is byte-frozen and this changes crown eligibility. Everything else is prepared — [memo](decision-83-frozen-gate-fails-open-2026-08-15.md) |
| **#85** | **wire** the two loss stops, or delete them | wiring makes runs start halting — a behaviour change. Infrastructure is ready: `KillSwitch::trip` exists, the loop already computes `cur_equity`; `HaltReason` just has no variant for either |
| **#89** | wire the RNG, or delete the field + `seed` param | deletion touches one production caller + several tests. The misleading half (a tautological test implying seedability was verified) is already gone |
| **#87** | wire `agent/forecast-audit-tick` into a shipping build, or delete the documented flag | the flag still cannot work in any build; it now fails loudly instead of silently |
| **#90** | options 2 (emit a liquidation `Fill`) or 3 (engine-route it) | option 3 changes *what* is liquidated via a feedback loop, so it needs its own blast-radius measurement. Option 1 has landed |
| **#92** | delete the `--no-default-features` claim, fix the build, or fix **and** add a CI leg | only the third closes the family; the first two just change which side of "declared vs executed" is true |
| **#91** | — | **blocked on #92**; fix is 3 lines but cannot be compile-verified until that build works |
| **#81 cap.** | — | **blocked**: enabling `backtest/yahoo` before the emission-cadence defect yields a working-but-wrong arm |
| **1-25** | architect seam decision + compute budget | 20 anchored surfaces, four routed defects |

---

## 4. The finding behind the findings

Every defect this session — without exception — was the same sentence: **something is declared in one
place and not executed in another, and nothing compares the two.** A config field and its consumer. A
feature and its enabler. A strategy's name and its behaviour. A gate's pipe and its exit status. A
seed and its reader.

That is why reading the code crate-by-crate would have found none of them, and why mapping
*reachability* found twelve. It is also why the compiler cannot help: **rustc does not report unused
`pub` items in a library crate**, so in a 17-crate `pub`-heavy workspace the language's own detector
for this exact defect class is blind by construction.

The project has unusually strong machinery for **freezing** things — 119 byte-immutable anchors, a
frozen decision gate, atomic ADR registration, a story↔trace↔CHANGELOG triad lint. It has none for
**connecting** them. Nothing checks that a declared capability is reachable, that a configured limit
is read, or that a named gate can fail. That gap is the one worth closing, and it is cheap to close:
#84 was one line, #81's honesty half mirrored a guard that already existed, and the arm-count fix was
a second match arm.

**The cfg-reachability audit is now CLOSED** — all 24 features carry a verdict, and its last open
candidate resolved *against* the alarming reading (see #91). Two of its findings were downgraded on
verification, which is the point of verifying: an audit that only ever escalates is not measuring.

**The sharpest single discriminator**, worth adopting as a rule: for a `cfg(feature = …)` capability,
the question is not "is the feature enabled?" but **"does the `cfg(not(...))` arm `bail!`, or does it
return a plausible value?"** `backtest/candle` is off everywhere and nearly harmless because all three
off-arms bail with the rebuild command. `backtest/realdata` returns a bare `None` — and that
difference is the entire severity gap between a non-issue and #81.

## 5. Corrections I made to my own work

Recorded because the pattern is the point, not the individual slips: an over-broad anchor claim
(narrowed — some rows are settlement-keyed), a wrong "owes a trades column" rider (the family already
ships one), a `bar_span_hours` regression I introduced and shipped uncompiled for two weeks because
verification ran `--features backtest/realdata` without `candle` (**`--all-targets` selects targets,
not features** — now in the playbook), a mis-named failing suite, an overstated #82 where I asserted
reachability without tracing the caller graph, and an incomplete #81 predicate whose fix would have
introduced a double-subtraction. One sub-agent report was also corrected (it placed two sibling gates
in the wrong crate).

## 6. Known coverage gaps — stated plainly

I have **never seen this cockpit render**; every UI conclusion is via harnesses. The **full workspace
suite has never completed** in my hands. **CI has been red throughout** and its logs need repo-admin
to read (403). I have read essentially **none** of `reports`, `llm`, `cost`, `forecast`, `trader`,
`exec`, `risk`, and only fragments of `ui` (249 files), `audit`, `agent`. The 900-paper `research/`
tree and the vendored `iced_tiny_skia` fork are unexamined. The reachability sweep ran while the tree
moved under it; early items were not all re-confirmed against the final state.
