---
slug: v3-volatility-forecaster-rebaseline
version: 0.1.0
status: proposed
owner: analyst
updated: 2026-05-22
parent: v3-volatility-forecaster
parent_version: 0.1.0
---

# Tasks — v3-volatility-forecaster-rebaseline

Tight, 1-day-scoped task list. Analyst owns T-A1..T-A3 (closed at this
pass). Operator-decide T-OD1..T-OD3 carry analyst-recommended defaults
under standing Autoapprove. Architect, developer, tester rows are stubs
the next agents own.

## Analyst (this pass)

- [x] T-A1 — Author `feature.md` with frontmatter (slug, version 0.1.0,
      status: proposed, owner: analyst, updated 2026-05-22, parent:
      v3-volatility-forecaster, parent_version: 0.1.0,
      parent_disposition: shipped-with-MODEL-BROKEN-NO-ALPHA-advisory).
      Body sections: Why / Scope / Out-of-scope / Investigation
      findings / Requirements R1–R5 / Risks K-rebase-1..4 / Hypotheses
      H-rebase-1..2 / Routes (4-cell verdict×determinism table) /
      Operator-decide Q1..Q3 / References / Verification stub /
      Changelog. — _acceptance: file exists at
      `spec/v3-volatility-forecaster-rebaseline/feature.md`; frontmatter
      validates against spec-update skill contract; investigation
      findings cite exact `crates/backtest/src/main.rs` and
      `crates/forecast/src/bin/sharpe_comparison.rs` file:line ranges._

- [x] T-A2 — Investigate the three open questions surfaced in the brief:
      (i) what baseline does the parent sharpe-comparison use (answer:
      hard-coded `top10-2023-1h-momentum`, synthetic; cited at
      `crates/forecast/src/bin/sharpe_comparison.rs:1293` + 3 string-
      literal sites at 975 / 1049 / 1082); (ii) does a realdata
      un-targeted v1 momentum scenario exist (answer: no — only
      `top10-2023-1h-momentum` and `top10-2024-h1-momentum` are
      registered as momentum, both `data_source: Synthetic`); (iii) is
      there an anchored realdata momentum report we can reuse (answer:
      no — all `-realdata` reports under
      `spec/backtest-real-binance-data/reports/` are overlay variants;
      `spec/v25-tcn-alpha-investigation/reports/sharpe-comparison-realdata-20260519.md`
      compares overlay vs overlay-real-weights, not vs un-targeted; the
      ADR-0038 § D7 reference to a real-data momentum baseline report
      is aspirational and the file does not exist). Findings embedded
      verbatim in `feature.md` § Investigation findings. — _acceptance:
      Q1 forced to default (b) by finding #3; defaults for Q2 and Q3
      are clean (no constraint conflict)._

- [x] T-A3 — Register the rebaseline pass across spec hygiene surfaces:
      add `REQ-V3-VOL-FORECASTER-REBASELINE-001` to `spec/trace.toml`
      with `state: proposed`, `parent: REQ-V3-VOL-FORECASTER-001`,
      `feature: v3-volatility-forecaster-rebaseline`, empty
      `crates`/`tests`/`anchors` (next agents fill); add an Active
      block to `spec/backlog.md` summarizing the (b) routing pick,
      Q1–Q3 defaults, +1 anchor projection, ~1 day budget, four-route
      tree; emit handoff envelope per AGENT.md § Communication contract
      pointing at the new spec files. — _acceptance: trace.toml row
      exists with the canonical id; backlog Active block exists with
      the standard analyst HTML comment shape; orchestrator can
      identify the (b)-routed pass without re-investigation._

## Operator-decide (standing Autoapprove applies to defaults)

- [x] T-OD1 — **Q1: real-data baseline scenario.** Default **(b)**:
      introduce `top10-2023-fy-momentum-realdata` in
      `Scenario::from_name` (~25 LoC additive, mirrors existing
      `-realdata` pattern at lines 450-475; uses
      `ScenarioStrategy::Momentum { config_id: "top10_momentum_h1" }`
      and `data_source: ScenarioDataSource::RealData` and pinned
      `expected_revision_sha = 3a8b96c43f…`). Option (a) is
      structurally rejected — see T-A2 finding #3. — **Resolved 2026-05-22 → (b)** by orchestrator under operator's standing Autoapprove (prior session); option (a) anchored-report-reuse path was structurally closed by the analyst's T-A2 investigation.

- [x] T-OD2 — **Q2: anchor naming + namespace.** Default **(a)**:
      anchor `sharpe-comparison-vol-target-bs1-realbaseline` under
      NEW `[v3.0.0-volatility-rebaseline]` namespace block; N_new = +1.
      Existing 3 `[v3.0.0-volatility]` anchors stay byte-immutable per
      ADR-0038 § D6. — **Resolved 2026-05-22 → (a)** by orchestrator under standing Autoapprove.

- [x] T-OD3 — **Q3: deliverable path.** Default **(a)**:
      `spec/v3-volatility-forecaster-rebaseline/reports/sharpe-comparison-vol-target-bs1-realbaseline-<YYYYMMDD>.md`.
      Keeps the rebaseline cohort cleanly separated from the parent's
      `[v3.0.0-volatility]` evidence. — **Resolved 2026-05-22 → (a)** by orchestrator under standing Autoapprove.

## Architect (M-T1; spawned after T-OD1..3 close)

- [ ] T-AR-1 — Lock the new scenario shape in
      `Scenario::from_name`: name = `top10-2023-fy-momentum-realdata`;
      `start_year = 2023`; `bar_count = 8760`; `strategy =
      ScenarioStrategy::Momentum { config_id: "top10_momentum_h1" }`;
      `initial_capital = dec!(100_000)`; `slippage_bps = 2`;
      `taker_fee_bps = 4`; `data_source =
      ScenarioDataSource::RealData`; `expected_revision_sha =
      Some("3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7".into())`;
      `#[cfg(feature = "realdata")]` gated. Cite the exact insertion
      point at `crates/backtest/src/main.rs` (recommended: after the
      synthetic `top10-2024-h1-momentum` arm at line 321, alongside
      the other `-realdata` momentum block). — _acceptance: decomp.md
      Wave A lists the scenario insertion as additive (no refactor)
      and reviewers can apply the patch without consulting the
      analyst's findings._

- [ ] T-AR-2 — Lock the `sharpe_comparison.rs` patch shape. Three
      surface changes: (i) swap the hard-coded
      `vol_target_scenarios` array entry at line 1293 from
      `"top10-2023-1h-momentum"` to
      `"top10-2023-fy-momentum-realdata"`; (ii) update the three
      string-literal advisory lines at 975 / 1049 / 1082 to read
      "real-data" (NOT "synthetic GBM"); (iii) update the report
      filename template at line 1327 from
      `"sharpe-comparison-vol-target-bs1-realdata-{today}.md"` to
      `"sharpe-comparison-vol-target-bs1-realbaseline-{today}.md"`
      AND the output dir mapping at line 1243 to
      `spec/v3-volatility-forecaster-rebaseline/reports/`. — _accept:
      decomp.md Wave B captures all 6 patch sites with file:line._

- [ ] T-AR-3 — Decide whether to add a CLI flag (e.g.
      `--baseline-scenario <name>`) for future re-baseline
      flexibility OR keep the hard-coded swap (cheapest). Analyst
      recommendation: **hard-coded swap** — adding a CLI flag is
      scope creep against the 1-day budget. Architect ratifies at
      M-T1. — _acceptance: decomp.md § Scope explicitly rejects the
      CLI flag for this pass._

- [ ] T-AR-4 — Lock the anchor namespace block shape in
      `spec/anchors.toml`. New block header preamble mirrors the
      existing `[v3.0.0-volatility]` block at line 239: 1-line title
      ("v3.0.0-volatility-rebaseline sharpe-comparison re-baseline
      pass, T-T2 / M-FINAL 2026-05-23+"), the data_revision_sha
      callout, the V-verdict carry-forward note, the T-classifier
      new-net-delta callout, the anchor-additive contract reference.
      — _acceptance: decomp.md § Anchors Wave shows the block
      verbatim._

## Developer (spawned after architect M-T1)

- [ ] T-DEV-1 (Wave A) — Add `top10-2023-fy-momentum-realdata`
      scenario per T-AR-1. Run `cargo run -p backtest --release
      --features candle,realdata --bin backtest -- --scenario
      top10-2023-fy-momentum-realdata --seed 0xC0FFEE` to verify
      backtest succeeds end-to-end and emits a report. Use the
      copy-pasteable watch recipe below to monitor wall-clock if the
      run exceeds 2 minutes.

- [ ] T-DEV-2 (Wave B) — Apply T-AR-2 patches to `sharpe_comparison.rs`.
      Run `cargo run -p forecast --release --features candle,realdata
      --bin sharpe_comparison -- --scenario vol-target-bs1` and verify
      a new report lands under
      `spec/v3-volatility-forecaster-rebaseline/reports/sharpe-comparison-vol-target-bs1-realbaseline-<YYYYMMDD>.md`.

- [ ] T-DEV-3 — Run 2-run byte-identity determinism check (R5). Two
      clean tempdir runs, hash both bodies via
      `scripts/hash_report.py`, assert match.

- [ ] T-DEV-4 — Emit `cargo fmt --check`, `cargo clippy --workspace
      --features candle,realdata -- -D warnings`, `cargo test
      --workspace --lib --features candle,realdata` PASS before
      handing off to tester.

## Tester (M-FINAL after developer waves)

- [ ] T-T-1 — Run T-T1 cargo gates per parent feature's R11.x
      contracts: `cargo fmt --check`, `cargo clippy -- -D warnings`,
      `cargo test --workspace --lib --features candle,realdata`.

- [ ] T-T-2 — Compute the new report body-SHA-256 via
      `scripts/hash_report.py` and write the new
      `[v3.0.0-volatility-rebaseline]` block in
      `spec/anchors.toml`. Confirm anchor count goes 33 → 34 PASS
      (or 33 → 35 PASS if the operator opted in on T-OD2 (b)).

- [ ] T-T-3 — Re-evaluate the joint advisory verdict cell against
      the routing table in `feature.md` § Routes. Record the verdict
      cell (R-O1 / R-O2 / R-O3 / R-O4) in the test report. Emit the
      report at `spec/v3-volatility-forecaster-rebaseline/reports/test-final-<YYYY-MM-DD>.md`.

- [ ] T-T-4 — HANDOFF → presenter. Presenter inherits the 4-cell
      routing tree; the operator's next decision is mechanical
      given the verdict cell.

## Notes / Watch recipes

If any cargo / backtest / sharpe-comparison run exceeds 2 minutes,
emit this copy-pasteable watch block (per repo memory directive):

```sh
# Watch the new realdata baseline backtest progress
watch -n 5 'ls -lah spec/v3-volatility-forecaster-rebaseline/reports/ 2>/dev/null | tail -20; echo "---"; pgrep -f "backtest|sharpe_comparison" | xargs -I {} ps -p {} -o pid,pcpu,etime,comm 2>/dev/null'
```

## Routes (pre-drawn for presenter — mirrors `feature.md` § Routes)

The four possible end-states of this 1-day re-baseline pass — keyed by
T-classifier outcome × determinism gate — pre-draw the routing tree
the presenter inherits at the next deck:

| Outcome | T-classifier | Determinism | Next feature |
|---------|--------------|-------------|--------------|
| **R-O1** | T-VOL-NO-ALPHA | PASS | (a) RETIRE C1 → promote C2 (`v3-regime-classifier`) or C5 (`v3-llm-forecaster`) |
| **R-O2** | T-VOL-MARGINAL | PASS | (d) v0.1.1 GARCH refit + return |
| **R-O3** | T-VOL-ALPHA-UNLOCKED | PASS | (c) DEBUG V3 — spawn `v3-garch-calibration-tune` |
| **R-O4** | (any) | FAIL | Route back to developer for determinism fix; if iteration overflows, escalate to operator |

Standing Autoapprove from the 2026-05-22 prior session applies to
Q1–Q3 defaults; the post-(b) route decision is the operator's next
mechanical call.

## Changelog

- 2026-05-22 (analyst): tasks scaffolded at v0.1.0 / status=proposed.
  T-A1..T-A3 ticked (analyst work closed). T-OD1..T-OD3 carry
  defaults under standing Autoapprove. Architect / developer / tester
  rows are stubs; the next agents own them.
