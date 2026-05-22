---
slug: v3-volatility-forecaster-noop-fix
status: in-progress
owner: architect
updated: 2026-05-22
priority: P0
---

# v3-volatility-forecaster-noop-fix — tasks

> Per AGENT.md, task IDs use these prefixes: **T-A** analyst,
> **T-OD** operator-decide, **T-AR** architect, **T-D-N** developer
> (Wave A then B then …), **T-T** tester, **T-P** presenter.
> Architect refines the developer waves at M-T1.

## T-A — Analyst (closed at HANDOFF)

- [x] **T-A1** — Author `feature.md` v0.1.0 with R1-R6 + K1-K3 +
  H1-H2 + Q1-Q3 + 4-cell route table.
- [x] **T-A2** — Investigate TCN overlay for the same no-op pattern.
  Finding: structurally different — TCN's dampen-to-Hold semantic
  mutates `Signal.kind` (load-bearing field the executor reads);
  no parallel bug. Documented in § Investigation findings.
- [x] **T-A3** — Enumerate affected anchors (4 rows: 3 in
  `[v3.0.0-volatility]`, 1 in `[v3.0.0-volatility-rebaseline]`).
  Three expected to change post-fix; `vol-verdict-bs1-realdata`
  audit-pending (GARCH-only path may or may not cite overlay
  equity). Architect confirms at M-T1.
- [x] **T-A4** — Add `REQ-V3-VOL-FORECASTER-NOOP-FIX-001` row to
  `spec/trace.toml` with parent `REQ-V3-VOL-FORECASTER-001`,
  state `proposed`.
- [x] **T-A5** — Append Active block to `spec/backlog.md` flagging
  P0.
- [x] **T-A6** — Amendment block to `spec/v3-volatility-forecaster/feature.md
  § Verification` noting "INVALIDATED 2026-05-22 — see
  v3-volatility-forecaster-noop-fix".
- [x] **T-A7** — Amendment block to
  `spec/v3-volatility-forecaster-rebaseline/feature.md § Verification`
  similarly.
- [x] **T-A8** — Author dev-note
  `spec/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md`
  capturing the caveman-probe diagnostic chain.

## T-OD — Operator-decide (Q1..Q3)

Standing Autoapprove from operator's 2026-05-22 prior session
applies to the analyst-recommended defaults. Orchestrator may
auto-tick all three.

- [x] **T-OD1** — Resolve **Q1** (fix shape). Default: **(ii)**
  defaulted `Strategy::quantity_scale(&self, symbol) → f64` trait
  method (minimum blast radius). — **Resolved 2026-05-22 → (ii)** by orchestrator under operator's standing Autoapprove (K2 min-blast-radius rationale; Signal serialization in audit ledger / journal is the larger blast-radius risk under (i)).
- [x] **T-OD2** — Resolve **Q2** (anchor re-emission protocol).
  Default: **(a)** re-emit in-place under existing namespaces +
  ADR-0038 § D6 amendment subsection documenting the wiring-bug-
  fix re-emission protocol. — **Resolved 2026-05-22 → (a)** by orchestrator under standing Autoapprove (precedent-setting; ADR amendment makes the protocol explicit for any future demonstrated wiring-bug discoveries).
- [x] **T-OD3** — Resolve **Q3** (TCN overlay co-investigation scope).
  Default: **(b)** vol-target-only fix; TCN audit deferred (no
  evidence of a parallel bug per T-A2). — **Resolved 2026-05-22 → (b)** by orchestrator under standing Autoapprove (T-A2 investigation confirmed TCN overlay mutates load-bearing `Signal.kind`; no parallel bug; scope stays tight).

## T-AR — Architect (M-T1) — CLOSED 2026-05-22

Architect lock per [`decomp.md`](decomp.md). All file:line citations,
the wave shape, the ADR amendment text, and the forensic-gate
protocol live there; this list ticks the rows + cites the load-bearing
output.

- [x] **T-AR-1** — Q1=(ii) locked: defaulted `Strategy::quantity_scale(&self, _symbol: &Symbol) -> f64 { 1.0 }` at [`crates/strategy/src/traits.rs:8-15`](../../crates/strategy/src/traits.rs) (+7 LoC inside trait + 1-line import addition). Receiver `&self` (read-only accessor; scale cached during `on_bar`); parameter `&Symbol` (already in scope at the call site, no clone). 9 existing `impl Strategy` blocks auto-inherit `1.0` without code change. Sizing-pipeline call site identified at [`crates/backtest/src/scenarios/garch_vol_target_overlay.rs:262-265`](../../crates/backtest/src/scenarios/garch_vol_target_overlay.rs) (Buy arm only; Sell arm closes the full position and scale does NOT apply — documented in decomp.md § T-AR-2). VolTargetingOverlay gains `scale_cache: BTreeMap<Symbol, f64>` field; populates in `on_bar`; reads in `quantity_scale` override. Misleading "diagnostic only" inline comment at lines 315-317 of `vol_targeting_overlay.rs` removed. See [`decomp.md § T-AR-1`](decomp.md), [`§ T-AR-2`](decomp.md), [`§ T-AR-3`](decomp.md).
- [x] **T-AR-2** — `vol-verdict-bs1-realdata` audit closed: body is GARCH-only. Sections (Checkpoint table + Per-symbol QLIKE table + Aggregate statistics + Verdict + Notes) cite `qlike_garch`, `qlike_constant`, `mean_sigma_hat`, `mean_sigma_realized`, `calibration_ratio`, `improvement_pct`, `qlike_dispersion`, `mean_calibration_ratio`, `n_symbols_improving`, `verdict.label`, `verdict.evidence`, `verdict.routes_to` — **none load the overlay or any equity curve**. Anchor SHA `99c2189210d2091aebf199a5fc1cc8a448d14da6911130e3d6ebb163e686cd21` stays byte-identical post-fix. Walk of [`crates/forecast/src/bin/vol_verdict.rs:428-587`](../../crates/forecast/src/bin/vol_verdict.rs) in [`decomp.md § T-AR-5`](decomp.md). Final anchor delta: **3 SHAs re-emit, 1 GARCH-only row stays byte-identical**.
- [x] **T-AR-3** — ADR-0038 § D6.b amendment subsection drafted (~35 lines, 5-clause protocol: enumerate + cite + would-have-caught test + architect sign-off + negative invariant). Lands verbatim at developer T-D-N14 via spec-update at end of § D6 (after the existing "Anchor count progression" block at line 606 of `0038-vol-forecast-verdict-shape.md`, before `## Alternatives considered`). Text in [`decomp.md § T-AR-7`](decomp.md).
- [x] **T-AR-4** — TCN overlay re-audit under Q1=(ii): no adjacent break. The defaulted trait method auto-inherits `1.0`; TCN overlay's `Signal { kind: modulated_kind, ..sig }` spread is unchanged; TCN scenarios (`tcn_overlay.rs`, `tcn_overlay_weights.rs`) do NOT call `quantity_scale` in their sizing pipelines (per decomp.md § T-AR-2 scenario-by-scenario table); all 8 TCN anchors stay byte-identical. Q3=(b) confirmed; no follow-on filed. See [`decomp.md § T-AR-8`](decomp.md).
- [x] **T-AR-5** — Developer waves decomposed in [`decomp.md § T-AR-6`](decomp.md). **Wave A** (sequential, ~80-150 LoC, ~45-75 min): T-D-N1 trait method add → T-D-N2 overlay refactor → T-D-N3a forensic-gate pre-fix FAIL run → T-D-N3b post-fix PASS → T-D-N4 sizing-pipeline hook → T-D-N5 R6 unit tests → T-D-N6 workspace gate. **Wave B** (sequential after A, ~5 min): T-D-N7 + T-D-N8 (top10-2023-fy-vol-target-overlay-realdata re-emit + 2-run determinism) → T-D-N9 (sharpe-comparison realdata) → T-D-N10 (sharpe-comparison realbaseline) → T-D-N11 (determinism re-confirm) → T-D-N12 (lock SHAs in anchors.toml) → T-D-N13 (verify_anchors.sh `ANCHORS PASS (34 / 34)`). **Wave C** (parallel-safe with B, ~15 min): T-D-N14 ADR-0038 § D6.b amendment → T-D-N15 trace.toml state flip → T-D-N16 feature.md § Design append. Worst-case wall-clock: <3 hours total dev work.
- [x] **T-AR-6** — Forensic-gate protocol locked in [`decomp.md § T-AR-4`](decomp.md) + [`§ T-AR-6 Wave A T-D-N3a/3b`](decomp.md). The R2 e2e test (`overlay_quantity_scale_reflects_computed_factor` in new file `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`) is run against current main BEFORE the fix lands. Expected pre-fix output: `test result: FAILED. 0 passed; 1 failed; ...` with the literal panic `'vol-target overlay produced scale=1 after 5 on_bar calls — expected ≠ 1.0 (no-op signature)'`. Developer captures this verbatim into Wave A status update at T-D-N3a. Expected post-fix output (T-D-N3b): `test result: ok. 1 passed; 0 failed; ...`. If pre-fix run PASSES, the test is wrong (false negative) and Wave A halts pending architect re-audit.
- [x] **T-AR-7** — Frontmatter flipped: `status: proposed → in-progress`, `owner: analyst → architect`. Developer T-D-N16 flips `owner: architect → developer` at Wave A start. Baseline gate captured in decomp.md § Baseline gate: `ANCHORS PASS  (34 / 34)` quoted verbatim from `bash scripts/verify_anchors.sh` at M-T1 open (2026-05-22, pre-fix).

## T-D-N — Developer (Waves stubbed; architect refines at T-AR-5)

### Wave A — Wire-up fix + tests

- [ ] **T-D-N1** — Implement the Q1-locked fix at the strategy → executor
  handoff.
- [ ] **T-D-N2** — Implement R6 unit test: `scale != 1.0` causes
  returned Signal (or queried `Strategy::quantity_scale`) to carry
  the computed scale.
- [ ] **T-D-N3** — Implement R6 integration test at the engine
  boundary: two fills under `compute_scale → 1.7` differ in
  quantity from two fills under `scale = 1.0`.
- [ ] **T-D-N4** — Implement R2 end-to-end test:
  `vol_targeting_overlay::overlay_changes_equity_vs_untargeted_baseline`.
  Synthetic-or-fixture data stream with a rigged sigma_hat sequence.
  Assert equity divergence ≥ 1 bp + trade-count ≈ identical.
- [ ] **T-D-N5** — Run `cargo test --workspace --features candle`;
  confirm green. Run `cargo clippy --workspace --features candle -D
  warnings`; confirm green.

### Wave B — Anchor re-emission + ADR amendment

- [ ] **T-D-N6** — Re-run `top10-2023-fy-vol-target-overlay-realdata`
  backtest; emit new report; capture new body-SHA-256. Determinism:
  2-run byte-identity per R11.9 / R11.10 carry-forward.
- [ ] **T-D-N7** — Re-run `sharpe-comparison-vol-target-bs1-realdata`;
  emit new report; capture new body-SHA-256. Re-evaluate T-classifier
  on the new net_delta.
- [ ] **T-D-N8** — Re-run `sharpe-comparison-vol-target-bs1-realbaseline`
  (rebaseline-namespace anchor); emit new report; capture new
  body-SHA-256.
- [ ] **T-D-N9** — (Conditional on T-AR-2 finding) Re-run
  `vol-verdict-bs1-realdata` if its body cites overlay equity;
  otherwise leave byte-identical.
- [ ] **T-D-N10** — Update `spec/anchors.toml` with the new SHAs
  under existing namespace blocks (Q2=(a) default). Add a comment
  block referencing this feature and the dev-note.
- [ ] **T-D-N11** — Land the ADR-0038 § D6 amendment subsection
  (T-AR-3 text). Spec-update via the skill.
- [ ] **T-D-N12** — Run `scripts/verify_anchors.sh`; confirm
  ANCHORS PASS (34 / 34) with 3-4 fresh + 30-31 unchanged.

## T-T — Tester (M-FINAL)

- [ ] **T-T1** — cargo fmt --check PASS; cargo clippy --workspace
  --features candle -D warnings PASS; cargo test --workspace --lib
  --features candle PASS.
- [ ] **T-T2** — Re-run `scripts/verify_anchors.sh`; confirm
  ANCHORS PASS (34 / 34). Verify the affected rows show the new
  SHAs; verify the un-changed rows show their original SHAs
  (negative invariant on 30-31 rows). Capture the diff in the
  test report.
- [ ] **T-T3** — Re-run R2 + R6 regression tests; capture PASS.
  Verify (via local revert experiment) that the tests FAIL under
  pre-fix code; capture this verification in the test report as
  evidence the gate is meaningful. Authored test report at
  `spec/v3-volatility-forecaster-noop-fix/reports/test-final-<date>.md`.
- [ ] **T-T4** — Record new T-classifier + V-verdict (likely
  V3 unchanged + new T-cell) in feature.md § Verification.
- [ ] **T-T5** — Flip parent + rebaseline feature.md § Verification
  amendment block's TBD verdict-cell cross-reference to the new
  Verification block's verdict (R-O1 / R-O2 / R-O3).

## T-P — Presenter (M-PRESENTER)

- [ ] **T-P1** — Assemble
  `spec/v3-volatility-forecaster-noop-fix/presentations/v3-volatility-forecaster-noop-fix-<YYYY-MM-DD>.md`.
  Carry the 4-cell route table; recommend the (a) RETIRE-C1 (R-O1) /
  (a) RETIRE or (d) refit (R-O2) / reopen + V3 repair (R-O3) decision
  per the verdict cell. Operator ticks approval.

## Watch recipes (per MEMORY.md)

If any backtest / anchor re-emission runs >2 min, the developer
emits a copy-pasteable `watch -n 2 '<probe>'` block in the wave-
status update so the orchestrator can stream progress without
polling. Expected wall-clock per scenario: ~40s (carry-forward
from rebaseline pass).

## Changelog

- 2026-05-22 (analyst): tasks skeleton authored; T-A1..T-A8 ticked
  at HANDOFF; T-OD1..T-OD3 carry standing-Autoapprove defaults
  (Q1=(ii), Q2=(a), Q3=(b)); T-AR / T-D-N / T-T / T-P stubs left
  for architect / developer / tester / presenter refinement.
