---
slug: v3-volatility-forecaster-rebaseline
version: 0.1.0
status: shipped
owner: tester
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

- [x] T-AR-1 — Lock the new scenario shape in
      `Scenario::from_name`: name = `top10-2023-fy-momentum-realdata`;
      `start_year = 2023`; `bar_count = 8760`; `strategy =
      ScenarioStrategy::Momentum { config_id: "top10_momentum_h1" }`;
      `initial_capital = dec!(100_000)`; `slippage_bps = 2`;
      `taker_fee_bps = 4`; `data_source =
      ScenarioDataSource::RealData`; `expected_revision_sha =
      Some("3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7".into())`;
      `#[cfg(feature = "realdata")]` gated. — **DONE 2026-05-22**:
      decomp.md § T-AR-1 locks the verbatim Rust block. Architect
      revised the brief's "after line 321" insertion-point hint to
      **immediately before `top10-2023-fy-patchtst-overlay-realdata`
      at line 546** — alphabetical placement within the existing
      realdata-cfg-gated cluster (lines 449-592) is the file's
      load-bearing documentation contract. Every non-strategy field
      verified by-value equal to the parent vol-target-realdata arm
      (lines 571-592) for apples-to-apples comparison; only `strategy`
      diverges (`Momentum { config_id: "top10_momentum_h1" }` per the
      un-targeted v1 baseline) — by design.

- [x] T-AR-2 — Lock the `sharpe_comparison.rs` patch shape. — **DONE
      2026-05-22 with CRITICAL CORRECTION to the brief's default**:
      hard-coded swap REJECTED (would mutate the body bytes of the
      parent `sharpe-comparison-vol-target-bs1-realdata` report and
      thus the parent anchor SHA `ef048366ac5433173016e937dce0871b4b8da368ad6d4b17621b29faacea2ab1` —
      violates ADR-0038 § D6 anchor-additive contract). Replaced
      with **NEW `ScenarioFamily::VolTargetRebaseline` enum variant**
      (~30 LoC additive) alongside the existing `VolTarget`; new
      dispatch arm clones the parent body verbatim except for (a)
      `vol_target_scenarios[0] = "top10-2023-fy-momentum-realdata"`,
      (b) `filename = "sharpe-comparison-vol-target-bs1-realbaseline-{today}.md"`,
      (c) calls into a sibling `render_vol_target_rebaseline` module
      with the 3 advisory string-literal swaps (sites 975 / 1049 /
      1082). Parent `VolTarget` arm stays byte-identical; parent
      anchor re-verifies on every CI run. The 20-LoC delta vs the
      original hard-coded approach is the non-negotiable cost of
      anchor-immutability. decomp.md § T-AR-2 + Wave B captures the
      full file:line patch set.

- [x] T-AR-3 — Decide whether to add a CLI flag for future re-baseline
      flexibility OR keep the hard-coded swap. — **DONE 2026-05-22**:
      REJECT `--baseline-scenario <name>` CLI flag. Replaced the
      brief's hard-coded-swap-vs-flag binary with a NEW-enum-variant
      shape (per T-AR-2): the new `--scenario vol-target-bs1-rebaseline`
      keyword is a discrete `ScenarioFamily` enum extension, not a
      parameterized flag. Rationale: (a) only one re-baseline pass is
      queued; (b) 1-day budget; (c) enum variants are the load-bearing
      precedent in this bin (`Tcn` / `VolTarget` already follow the
      pattern). decomp.md § T-AR-3 documents the cost/benefit reject.

- [x] T-AR-4 — Lock the anchor namespace block shape in
      `spec/anchors.toml`. — **DONE 2026-05-22**: new
      `[v3.0.0-volatility-rebaseline]` block (mirrors parent
      `[v3.0.0-volatility]` shape at lines 239-263 verbatim);
      preamble carries data_revision_sha callout, V-verdict
      carry-forward note (per H-rebase-2), T-classifier
      new-net-delta note (per ADR-0038 § D1.c), anchor-additive
      contract reference. Single anchor row:
      `sharpe-comparison-vol-target-bs1-realbaseline` (Q2=(a) default;
      baseline-backtest anchor NOT included — operator did not opt
      into Q2=(b)). N_new = +1; M-FINAL gate: `ANCHORS PASS  (34 /
      34)`. decomp.md § 6 quotes the block verbatim. Pre-feature
      baseline gate: `ANCHORS PASS  (33 / 33)` (architect's run
      2026-05-22).

## Developer (Waves A + B parallel-eligible; Wave C depends on both)

> Honest-tick rule: developer ticks the row only after running the
> invocation and quoting the literal output back into this file.

### Wave A — Add realdata baseline scenario (Day 1; parallel-eligible with Wave B)

- [x] T-D-N1 — `crates/backtest/src/main.rs` — insert
      `#[cfg(feature = "realdata")] "top10-2023-fy-momentum-realdata"
      => Ok(Self { ... })` arm immediately before line 546 per
      decomp.md § T-AR-1. ~25 LoC additive. Also added
      `bars_override`/`data_revision_sha` to `MomentumScenarioInput`
      (cli_types.rs:44-66), updated `momentum::run` to use
      `bars_override` (scenarios/momentum.rs:200-242), added realdata
      dispatch branch to main.rs is_momentum block, added
      `scenario_to_feature` entry. — file:line:
      `crates/backtest/src/main.rs:769`, `crates/backtest/src/cli_types.rs:44`,
      `crates/backtest/src/scenarios/momentum.rs:200`. — _accept:_
      `cargo build -p backtest --features realdata,candle` →
      `Finished 'dev' profile [unoptimized + debuginfo] target(s) in 6.77s`.

- [x] T-D-N2 — Run backtest end-to-end; emitted
      `backtest-20260522-095222-top10-2023-fy-momentum-realdata.md`
      under `spec/v3-volatility-forecaster-rebaseline/reports/`. —
      `cargo run -p backtest --release --features candle,realdata
      --bin backtest -- --scenario top10-2023-fy-momentum-realdata
      --seed 0xC0FFEE` →
      `Report written: spec/v3-volatility-forecaster-rebaseline/reports/backtest-20260522-095222-top10-2023-fy-momentum-realdata.md`.

- [x] T-D-N3 — Confirmed `data_revision_sha =
      3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7`
      in the new report frontmatter. — `grep '^data_revision_sha:'
      spec/v3-volatility-forecaster-rebaseline/reports/backtest-*-top10-2023-fy-momentum-realdata.md`
      → `data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7`.

- [x] T-D-N4 — Anchor-additive guard confirmed. — `bash
      scripts/verify_anchors.sh` → `ANCHORS PASS  (33 / 33)`.

### Wave B — Extend `sharpe_comparison.rs` (Day 1; parallel-eligible with Wave A)

- [x] T-D-N5 — `crates/forecast/src/bin/sharpe_comparison.rs:50-62`
      — additive `ScenarioFamily::VolTargetRebaseline` variant per
      decomp.md § T-AR-2. — file:line:
      `crates/forecast/src/bin/sharpe_comparison.rs:59`. —
      `cargo build -p forecast --bin sharpe_comparison --features candle`
      → `Finished 'dev' profile [unoptimized + debuginfo] target(s) in 3.99s`.

- [x] T-D-N6 — `sharpe_comparison.rs` — additive out-dir match arm
      `ScenarioFamily::VolTargetRebaseline =>
      PathBuf::from("spec/v3-volatility-forecaster-rebaseline/reports/")`.
      file:line: `crates/forecast/src/bin/sharpe_comparison.rs:1247-1249`.
      (Rolled into N5 build.)

- [x] T-D-N7 — `sharpe_comparison.rs` — new dispatch arm
      `if args.scenario == ScenarioFamily::VolTargetRebaseline { ... }`
      inserted before `VolTarget` arm; file:line:
      `crates/forecast/src/bin/sharpe_comparison.rs:1288`. (Rolled
      into N5 build.)

- [x] T-D-N8 — New `render_vol_target_rebaseline` sibling module
      (~250 LoC, no shared-extract needed — duplication is advisory
      strings only). file:line:
      `crates/forecast/src/bin/sharpe_comparison.rs:1210`. (Rolled
      into N5 build.)

- [x] T-D-N9 — Anchor-neutrality guard confirmed. —
      `cargo run -p forecast --release --features candle --bin
      sharpe_comparison -- --scenario vol-target-bs1 && bash
      scripts/verify_anchors.sh` →
      `ANCHORS PASS  (33 / 33)`. Parent anchor
      `ef048366ac5433173016e937dce0871b4b8da368ad6d4b17621b29faacea2ab1`
      verified byte-identical.

### Wave C — End-to-end + hand to tester (Day 1; depends on Wave A + B)

- [x] T-D-N10 — Run new sharpe-comparison end-to-end. —
      `cargo run -p forecast --release --features candle --bin
      sharpe_comparison -- --scenario vol-target-bs1-rebaseline` →
      `wrote spec/v3-volatility-forecaster-rebaseline/reports/sharpe-comparison-vol-target-bs1-realbaseline-20260522.md; T-classifier = T-VOL-NO-ALPHA`.

- [x] T-D-N11 — 2-run byte-identity (R5). Both runs produce body-SHA256
      = `d561fed564166f8c907cc9dda98fd2d56eb03333bd5aea16a0f6425924a2afb8`
      via `python3 scripts/hash_report.py
      spec/v3-volatility-forecaster-rebaseline/reports/sharpe-comparison-vol-target-bs1-realbaseline-20260522.md`.
      R5 PASS.

- [x] T-D-N12 — Spec hygiene gates. —
      `cargo fmt --check` → (no output, PASS);
      `cargo clippy --workspace --features candle,realdata -- -D warnings`
      → `Finished 'dev' profile [unoptimized + debuginfo] target(s) in 1.09s`;
      `cargo test --workspace --lib --features candle,realdata` →
      `test result: ok. 311 passed; 0 failed; 0 ignored; 0 measured;
      0 filtered out; finished in 0.52s`;
      `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (33 / 33)`.

## Tester (M-FINAL after Wave C; depends on T-D-N1..N12)

- [x] T-T-1 — Re-run all four cargo hygiene gates per T-D-N12 and
      quote each literal output line into
      `spec/v3-volatility-forecaster-rebaseline/reports/test-final-2026-05-22.md`.
      — `cargo fmt --check` → (no output, PASS);
      `cargo clippy --workspace --features candle,realdata -- -D warnings`
      → `Finished 'dev' profile [unoptimized + debuginfo] target(s) in 1.18s`;
      `cargo test --workspace --lib --features candle`
      → `test result: ok. 311 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s`;
      `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (33 / 33)` (pre-T-T2;
      parent anchor `ef048366ac5433173016e937dce0871b4b8da368ad6d4b17621b29faacea2ab1`
      verified byte-identical — T-AR-2 anchor-immutability contract holds). — **PASS 2026-05-22 (tester)**.

- [x] T-T-2 — Compute the new report body-SHA-256 via
      `python3 scripts/hash_report.py spec/v3-volatility-forecaster-rebaseline/reports/sharpe-comparison-vol-target-bs1-realbaseline-*.md`
      and write the new `[v3.0.0-volatility-rebaseline]` block in
      `spec/anchors.toml` per decomp.md § 6 verbatim shape (append
      after line 263). — _accept:_ `bash scripts/verify_anchors.sh`
      → `ANCHORS PASS  (34 / 34)`.
      — `python3 scripts/hash_report.py ...` →
      `d561fed564166f8c907cc9dda98fd2d56eb03333bd5aea16a0f6425924a2afb8`
      (matches developer's claim — 2-run byte-identity PASS R5 confirmed);
      anchors.toml `[v3.0.0-volatility-rebaseline]` block appended after line 263;
      `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (34 / 34)`. — **PASS 2026-05-22 (tester)**.

- [x] T-T-3 — Re-evaluate the joint advisory verdict cell against
      the routing table in `feature.md` § Routes. Record the verdict
      cell (R-O1 / R-O2 / R-O3 / R-O4) in the test report at
      `spec/v3-volatility-forecaster-rebaseline/reports/test-final-2026-05-22.md`.
      — net_delta = 0.000000 < +0.05 → T-VOL-NO-ALPHA + PASS → **R-O1**
      → (a) RETIRE C1; promote C2 or C5 from Queue → Active.
      feature.md § Verification updated with full hypothesis disposition,
      joint advisory verdict, architecture deviation note, and cross-refs.
      — **PASS 2026-05-22 (tester)**.

- [ ] T-T-4 — HANDOFF → presenter. Presenter inherits the 4-cell
      routing tree; the operator's next decision is mechanical given
      the verdict cell.

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
- 2026-05-22 (architect): M-T1 closed. T-AR-1..T-AR-4 ticked with
  decision rationale per row; decomp.md authored at
  `spec/v3-volatility-forecaster-rebaseline/decomp.md`. CRITICAL
  correction to T-AR-2 default — hard-coded swap rejected (would
  mutate parent anchor `ef048366...`); replaced with NEW
  `ScenarioFamily::VolTargetRebaseline` enum variant (preserves
  anchor-additive contract per ADR-0038 § D6). Wave A ∥ Wave B
  parallel-safe; Wave C M-FINAL depends on both. Anchor delta: +1
  at M-FINAL (33 → 34 PASS). Developer T-DEV-* rows expanded into
  T-D-N1..T-D-N12 with file:line + cargo invocation + expected
  literal triplets; tester T-T-1..T-T-4 unchanged in scope, anchor
  count delta locked at +1. Pre-feature baseline gate quoted
  literal: `ANCHORS PASS  (33 / 33)`. Frontmatter flipped: status
  `proposed → in-progress`, owner `analyst → architect`. HANDOFF →
  developer Wave A + Wave B parallel start.
- 2026-05-22 (developer): Waves A + B + C complete. T-D-N1..T-D-N12
  ticked with literal outputs. Architecture extension required beyond
  decomp.md estimate: MomentumScenarioInput gained `bars_override` +
  `data_revision_sha` fields (cli_types.rs); momentum::run updated to
  use bars_override; main.rs is_momentum dispatch extended for RealData
  path; report::momentum::write extended for data_revision_sha frontmatter;
  scenario_to_feature routing table updated. Wave B + C: new
  ScenarioFamily::VolTargetRebaseline enum variant + dispatch arm +
  render_vol_target_rebaseline sibling module (250 LoC, no shared-extract
  needed). All 33 parent anchors byte-identical confirmed. 2-run body-SHA256
  = d561fed564166f8c907cc9dda98fd2d56eb03333bd5aea16a0f6425924a2afb8.
  T-classifier = T-VOL-NO-ALPHA. HANDOFF → tester.
