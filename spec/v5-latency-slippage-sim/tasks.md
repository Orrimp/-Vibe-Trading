---
slug: v5-latency-slippage-sim
status: draft
owner: architect
updated: 2026-05-26
priority: P1
---

# v5-latency-slippage-sim — tasks

> Per AGENT.md, task IDs use these prefixes: **T-A** analyst,
> **T-OD** operator-decide, **T-AR** architect, **T-D-N** developer
> (Waves A-E), **T-T** tester, **T-P** presenter.

## M0 — Analyst

_owner: analyst_

- [x] **T-A1** (2026-05-26) — `spec/v5-latency-slippage-sim/feature.md`
  v0.1.0 authored with R1-R7 + R-NR.1-6 + K1-K7 + H1-H4 + Q1-Q5 +
  verdict tree + cost framing.
- [x] **T-A2** (2026-05-26) — tasks.md scaffolded (this file).
- [x] **T-A3** (2026-05-26) — Active row appended to
  [`spec/backlog.md`](../backlog.md).
- [x] **T-A4** (2026-05-26) — trace row `REQ-V5-LATENCY-SLIPPAGE-001`
  appended at END of [`spec/trace.toml`](../trace.toml) in `proposed`
  state.
- [x] **T-A5** (2026-05-26) — Gates verified: `bash scripts/verify_anchors.sh`
  PASS (34/34); `scripts/spec_lint.py` no new violation categories.

## M-OD — Operator decides (Q1-Q5)

_owner: operator. AskUserQuestion-routed by orchestrator OR standing-
Autoapprove via the operator's session-level directive._

All 5 Qs are standing-Autoapprove-eligible at analyst-recommended
defaults per the feature.md § Operator-decide table.

- [ ] **T-OD1** — Q1 latency model. Default: (b) uniform jitter range.
- [ ] **T-OD2** — Q2 slippage model. Default: (a) linear bps.
- [ ] **T-OD3** — Q3 audit row shape. Default: (b) NEW
  `AuditEvent::SimulatedExecMetrics` variant. **Load-bearing — locks
  the audit schema.**
- [ ] **T-OD4** — Q4 scope. Default: (a) backtest-only.
- [ ] **T-OD5** — Q5 anchor migration timing. Default: (a) defer to
  v0.2.0.
- [ ] **T-OD6** — Frontmatter flip `status: draft → in-progress`;
  `owner: analyst → architect`. trace.toml state flip
  `proposed → in-progress`.

## M-T1 — Architect

_owner: architect (post-operator-decide)._

- [ ] **T-AR-1** — Author
  [`spec/architecture/adr/0043-simulated-latency-and-slippage.md`](../architecture/adr/0043-simulated-latency-and-slippage.md).
  5 standard sections + Changelog. Lock D1-D5 sub-decisions per
  feature.md.
  _Acceptance_: ADR registry entry + cross-link in feature.md
  § Cross-references.
- [ ] **T-AR-2** — Lock the `LatencySlippageSimConfig` exact shape +
  the `MatchingEngine::apply_latency` signature + the
  `cost::apply_slippage` signature. Acceptance: signatures committed
  in feature.md § R1-R3 are byte-identical to the developer's
  implementation.
- [ ] **T-AR-3** — RNG sub-stream contract. Document HOW the
  `ChaCha20Rng` keying off `(scenario_seed, order_id)` is constructed.
  Cite the existing scenario seed plumbing in
  `crates/backtest/src/scenarios/momentum.rs`.
- [ ] **T-AR-4** — Audit-event variant exact shape (matches D4 / Q3).
  Acceptance: variant added to `AuditEvent` enum docs in ADR-0043
  § Decision.
- [ ] **T-AR-5** — Frontmatter flip `owner: architect → developer`;
  trace.toml `arch` column populated with ADR-0043 + this tasks.md.

## M-DEV — Developer execution

_owner: developer. Wave-parallelizable per ADR-0043 § Decision._

### Wave A — Configuration toggle (default-zero noop) [CRITICAL — blocks B-E]

**This is the FIRST developer task per the operator's directive: the
default config MUST act as a noop so existing anchors pass.**

- [ ] **T-D-N1** — Add `LatencySlippageSimConfig` to
  `crates/backtest/src/cli_types.rs` (or wherever `ScenarioConfig`
  lives — grep for the struct). Derive `Default` returning all zeros.
  Plumb through `ScenarioConfig` as a non-optional field with default
  value applied at all call sites.
  - Owner: developer • Milestone: M-DEV • Depends on: T-AR-5 • Blocks: T-D-N2..N9
  - File:line: `crates/backtest/src/cli_types.rs`, `crates/backtest/src/engine.rs::run_scenario`
  - Test cmd: `cargo test -p backtest --lib`
  - Expected: existing tests pass; new test
    `latency_slippage_sim_config_default_is_noop` confirms
    `LatencySlippageSimConfig::default() == { 0, 0, 0 }`.

- [ ] **T-D-N2** — **CRITICAL ANCHOR GATE**: run all 34 anchored
  scenarios with the new default-zero config. Confirm byte-identical
  output.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N1 • Blocks: T-D-N3
  - File:line: N/A (verification only)
  - Test cmd: `bash scripts/verify_anchors.sh`
  - Expected: `ANCHORS PASS (34 / 34)`. **If this fails, Wave A is
    incomplete — do NOT proceed to Wave B.**

### Wave B — Latency simulation in `crates/exec`

- [ ] **T-D-N3** — Implement `MatchingEngine::apply_latency` per R2.
  Seeded RNG sub-stream keyed on `(scenario_seed, order_id)`.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N2 • Blocks: T-D-N7
  - File:line: `crates/exec/src/matching.rs` (or wherever
    `MatchingEngine` lives — grep for `trait MatchingEngine`)
  - Test cmd: `cargo test -p exec --lib apply_latency`
  - Expected: 4 unit tests pass — `noop_at_zero`,
    `fixed_at_min_eq_max`, `jitter_uniform_distribution`,
    `deterministic_across_runs`.

- [ ] **T-D-N4** — Re-verify anchors: `bash scripts/verify_anchors.sh`
  must still PASS (34/34). At noop default, the latency apply is a
  no-op timestamp pass-through.

### Wave C — Slippage simulation in `crates/cost`

- [ ] **T-D-N5** — Implement `cost::apply_slippage` per R3. Linear
  bps model.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N2 (Wave A
    blocks; Wave C parallel-safe with B) • Blocks: T-D-N7
  - File:line: `crates/cost/src/slippage.rs` (NEW) +
    `crates/cost/src/lib.rs` (`pub mod slippage;`)
  - Test cmd: `cargo test -p cost --lib slippage`
  - Expected: 5 unit tests — `noop_at_zero_bps`, `buy_increases_price`,
    `sell_decreases_price`, `sign_symmetry`, `decimal_precision`.

- [ ] **T-D-N6** — Re-verify anchors: 34/34 PASS.

### Wave D — Audit-ledger integration

- [ ] **T-D-N7** — Add `AuditEvent::SimulatedExecMetrics` variant per
  R4. Skip-when-zero guard.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N3 + T-D-N5
    • Blocks: T-D-N8
  - File:line: `crates/audit/src/journal.rs` (or wherever `AuditEvent`
    enum is defined — grep `enum AuditEvent`)
  - Test cmd: `cargo test -p audit --lib simulated_exec_metrics`
  - Expected: 3 unit tests — `variant_serializes_round_trip`,
    `skip_when_zero_emits_nothing`, `dual_write_lands_in_both_tables`.

### Wave E — e2e divergence test + perf bench + non-regression

- [ ] **T-D-N8** — **CLAUDE.md non-negotiable**: baseline-equity-
  divergence e2e test per R5.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N7 • Blocks: T-D-N10
  - File:line: NEW `crates/strategy/tests/latency_slippage_sim_e2e.rs`
  - Test cmd: `cargo test -p strategy --test latency_slippage_sim_e2e`
  - Expected: 3 tests — `noop_byte_identical_to_baseline`,
    `enabled_diverges_by_at_least_1bp`, `enabled_audit_metrics_recorded`.
  - Pattern reference: `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`

- [ ] **T-D-N9** — Criterion bench per R7.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N7 (parallel
    with N8) • Blocks: T-D-N10
  - File:line: NEW `crates/exec/benches/latency_slippage.rs`
  - Test cmd: `cargo bench -p exec --bench latency_slippage`
  - Expected: 3 micro-benches print P99 — all under their R7 budgets.

- [ ] **T-D-N10** — Throughput regression bench per R-NR.4.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N9 • Blocks: M-FINAL entry
  - File:line: NEW `crates/exec/benches/throughput_with_sim.rs`
  - Test cmd: `cargo bench -p exec --bench throughput_with_sim`
  - Expected: noop-vs-enabled delta < 1% on the momentum 8760-bar
    scenario.

- [ ] **T-D-N11** — Final anchor + workspace gate.
  - Owner: developer • Milestone: M-DEV • Depends on: T-D-N10 • Blocks: M-FINAL
  - Test cmd: `bash scripts/verify_anchors.sh` && `cargo test --workspace`
  - Expected: 34/34 PASS; workspace test count delta = `+ (T-D-N1 + N3 + N5 + N7 + N8 + N9 + N10 tests)`.

## M-FINAL — Tester verification

_owner: tester._

Standard 11-row tester gate mirroring cockpit-activity-status-bar /
reflection-memory-trader-wiring M-FINAL:

- [ ] **T-T-1** — `bash scripts/verify_anchors.sh` → 34/34 PASS
  (R-NR.1)
- [ ] **T-T-2** — `cargo test --workspace --no-fail-fast` (R-NR.2)
- [ ] **T-T-3** — `cargo bench -p exec --bench latency_slippage` —
  record baseline (R7)
- [ ] **T-T-4** — `cargo bench -p exec --bench throughput_with_sim` —
  confirm < 1% regression (R-NR.4)
- [ ] **T-T-5** — `cargo test -p strategy --test latency_slippage_sim_e2e`
  — 3/3 PASS (R5)
- [ ] **T-T-6** — `cargo clippy --workspace --all-targets -- -D warnings`
  — green on touched crates
- [ ] **T-T-7** — `cargo fmt --check`
- [ ] **T-T-8** — `cockpit-smoke` against live binary — 0 panics
- [ ] **T-T-9** — Author
  `spec/v5-latency-slippage-sim/reports/test-final-<YYYY-MM-DD>-v5-latency-slippage-sim.md`
  per rust-test SKILL template (9-row matrix incl. the divergence
  test + criterion baselines + perf gate)
- [ ] **T-T-10** — Populate trace row `tests` + `anchors` columns.
- [ ] **T-T-11** — Flip trace state `in-progress → passed`.

## M-PRESENTER — Sprint-review deck

_owner: presenter. Runs only after VERDICT → PASS._

- [ ] **T-P-1** — Author
  `spec/v5-latency-slippage-sim/presentations/v5-latency-slippage-sim-<YYYY-MM-DD>.md`
  per present-results SKILL. TL;DR / what changed / 4-wave summary /
  verification matrix (incl. e2e divergence numbers + criterion
  baselines) / risk register / open questions (Q5 anchor migration
  for v0.2.0) / approval block.
- [ ] **T-P-2** — Capture before/after screenshots (optional — code
  refactor; no UI change). May skip per cockpit-activity precedent.
- [ ] **T-P-3** — Operator review. Capture verdict cell on the 4-cell
  routing tree (ship / hold / regression / critical).
- [ ] **T-P-4** — On operator approval, flip feature.md frontmatter
  `status: draft → shipped` + `shipped: <YYYY-MM-DD>`. Move backlog
  row Active → Recent. Spawn `v0.2.0-anchor-migration` brief if
  R-O1 picked.

## Notes

- **Parallelism map**:
  - Wave A (config plumbing) blocks all subsequent waves.
  - Wave B (latency, `crates/exec`) parallel with Wave C (slippage,
    `crates/cost`) — different crates, no file overlap.
  - Wave D (audit) depends on B + C (uses both fields).
  - Wave E (tests + benches) depends on D.
  - M-FINAL standard.
- **Anchor risk profile**: ZERO at v0.1.0 ship (default zeros + R-NR.1
  hard gate). The v0.2.0 anchor-migration brief is where anchors
  re-emit with non-zero values — a separate operator decision.
- **CRITICAL gates**: T-D-N2 (post-Wave-A anchor check) + T-D-N8
  (CLAUDE.md non-negotiable e2e test). Failure of either stops the
  wave and triggers an architect re-spawn.
- **K5 sequencing constraint**: vol-killswitch-overlay-noop-fix Bug #65
  Q4=(p3) developer is in flight as of 2026-05-26. Both briefs touch
  `crates/strategy/` + cost-modifier semantics. Sequence: Bug #65
  lands first; this brief's developer rebases.
