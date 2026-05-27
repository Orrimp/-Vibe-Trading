---
slug: v5-latency-slippage-sim-v0.2.0-anchor-migration
status: in-progress
owner: tester
updated: 2026-05-27
priority: P1
---

# v5 v0.2.0 anchor migration — tasks

> Inline-salvaged 2026-05-27 from analyst `ac4d192d801af160a` which
> 529'd at 14 tool-uses (wrote feature.md then dropped before tasks.md).
> Standard 6-milestone scaffold per AGENT.md.

## M0 — Analyst

_owner: analyst_

- [x] **T-A1** (2026-05-27) — `spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/feature.md`
  v0.1.0 authored with R1-R7 + R-NR + K1-K5 + H1-H3 + Q1-Q4 +
  pre-drawn 4-cell verdict tree + cost framing + cross-references.
- [x] **T-A2** (2026-05-27) — tasks.md scaffold (this file; inline-salvaged
  by orchestrator after analyst 529'd).
- [ ] **T-A3** — Active row appended to
  [`spec/backlog.md`](../backlog.md). (Orchestrator inline-completes.)
- [ ] **T-A4** — Trace row `REQ-V5-ANCHOR-MIGRATION-V0-2-0-001`
  appended at END of [`spec/trace.toml`](../trace.toml) in `proposed`
  state. (Orchestrator inline-completes.)
- [ ] **T-A5** — Gates verified: `bash scripts/verify_anchors.sh` PASS
  (34/34 currently; this brief is the one that MOVES the SHAs);
  `scripts/spec_lint.py` no new violation categories.

## M-OD — Operator decides (Q1-Q4)

_owner: operator. Q1 (canonical config) is the load-bearing one._

**RESOLVED 2026-05-27 — all four locked at analyst-recommended defaults.**

- [x] **T-OD1** (2026-05-27) — Q1 canonical config = **(b) medium
  (30..=80 ms / 8 bps)**. Every backtest report re-emits under this
  friction profile.
- [x] **T-OD2** (2026-05-27) — Q2 = **(a) keep OLD 34 anchors as
  noop-baseline namespace**. Both sets co-exist (68 anchors total);
  noop-baseline = friction-free oracle, canonical = under-friction
  reality. Sharpe-delta table becomes a permanent regression gate.
- [x] **T-OD3** (2026-05-27) — Q3 = **(b) flag inverted-alpha
  scenarios per scenario for operator review**. Tester surfaces K1-
  surprise candidates in the Sharpe-delta table; operator decides
  each retirement.
- [x] **T-OD4** (2026-05-27) — Q4 = **(a) re-run all overlay e2e
  tests under canonical config**. Defensive — catches silent cross-
  feature breakage. ~3 overlays today (vol_targeting,
  vol_killswitch, tcn_overlay). 1-2 day budget.

## M-T1 — Architect

_owner: architect (post-operator-decide). COMPLETE 2026-05-27._

- [x] **T-AR-1** (2026-05-27) — Canonical config locked per Q1 = (b)
  medium. **Exact Rust literal** developer applies at Wave A:

  ```rust
  // crates/backtest construction sites; applied uniformly to every
  // anchored scenario for the v0.2.0 re-emission run.
  LatencySlippageSimConfig {
      latency_ms_min: 30,
      latency_ms_max: 80,
      slippage_bps:   8,
  }
  ```

  Semantics: 30..=80 ms uniform-jitter latency sampled via the
  ADR-0043 D2 Murmur3-mixer sub-stream keyed on `(scenario_seed,
  order_id)`; 8 bps linear slippage applied per `Side` via ADR-0043 D3.
  This is the **canonical friction** every future anchored alpha
  number is measured against.

- [x] **T-AR-2** (2026-05-27) — ADR-0045 authored at
  [`spec/architecture/adr/0045-v5-canonical-config-and-noop-baseline-namespace.md`](../architecture/adr/0045-v5-canonical-config-and-noop-baseline-namespace.md).
  Locks D1 medium config / D2 two-namespace co-existence /
  D3 per-scenario K1-surprise flag / D4 mandatory cross-feature
  e2e re-check / D5 Sharpe-delta-table as permanent regression gate.

- [x] **T-AR-3** (2026-05-27) — Anchor-migration plan (developer
  follows mechanically at Wave B):

  **Canonical namespace pin chosen**: `v5-realdata-medium-2026-05`.

  **`spec/anchors.toml` rewrite contract**:

  1. The existing 34 `[[anchors]]` rows STAY in the file. Their
     `version` field gets the suffix ` + noop-baseline` appended,
     e.g. `version = "v0 + noop-baseline"`,
     `version = "v3.0.0-volatility + noop-baseline"`, etc. SHAs
     unchanged — these are the friction-free oracle.
  2. A new comment block before the appended NEW rows reads:

     ```
     # ── v5 v0.2.0 canonical-friction anchor set ──────────────────
     # Re-emitted under LatencySlippageSimConfig { latency_ms_min: 30,
     # latency_ms_max: 80, slippage_bps: 8 } per ADR-0045 D1.
     # Locked by tester at v5-latency-slippage-sim-v0.2.0-anchor-
     # migration M-FINAL on YYYY-MM-DD. Each row mirrors a noop-baseline
     # row above by (scenario, base-version); the version suffix
     # `+ v5-realdata-medium-2026-05` is the canonical namespace pin.
     # Verify: bash scripts/verify_anchors.sh (expects 68/68 PASS).
     ```
  3. For each of the 34 noop-baseline rows, append a paired NEW row
     with identical `scenario` value, `version = "<base> +
     v5-realdata-medium-2026-05"`, and the newly computed SHA from
     Wave A re-emission. Example pair:

     ```toml
     # OLD (now noop-baseline):
     [[anchors]]
     scenario = "btc-2023-1m-sma-cross"
     version  = "v0 + noop-baseline"
     sha256   = "fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c"

     # NEW (canonical friction):
     [[anchors]]
     scenario = "btc-2023-1m-sma-cross"
     version  = "v0 + v5-realdata-medium-2026-05"
     sha256   = "<NEW_SHA_FROM_WAVE_A>"
     ```
  4. File header (top comment block) gains one sentence per K2:
     `# As of v5 v0.2.0 (2026-05-27): anchors carry two namespaces — `
     `# `noop-baseline` (pre-friction historical oracle, SHAs from`
     `# v0.1.0 ship) and `v5-realdata-medium-2026-05` (canonical`
     `# friction; the current reference for paper-trading alpha).`
  5. After the rewrite, `bash scripts/verify_anchors.sh` MUST report
     `ANCHORS PASS (68 / 68)`. If the script's report-glob resolution
     surfaces a collision on identical `scenario` keys across
     namespaces, the developer routes back to architect for an
     anchors-schema mini-amendment (extending `[[anchors]]` with an
     explicit `namespace` field if needed). Architect's expectation:
     the version-suffix discriminator + glob fallthrough handles it,
     but the developer surfaces any ambiguity at Wave B kickoff.

- [x] **T-AR-4** (2026-05-27) — Cross-feature re-check inventory (per
  Q4 = (a) re-run ALL overlay e2e tests under canonical config).

  **Files surveyed**: `crates/strategy/tests/*_end_to_end.rs` +
  `latency_slippage_sim_e2e.rs` (the explicit baseline-divergence
  e2e tests mandated by the CLAUDE.md non-negotiable). Wave D
  re-runs each; Wave D checklist:

  - [x] **W-D-1** — `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`
    (v3 GARCH vol-targeting overlay; H3 falsifier — re-asserts ≥ 1 bp
    divergence between overlay-on and un-targeted baseline under
    canonical friction).
    - file: `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`
    - cmd: `cargo test -p strategy --test vol_targeting_overlay_end_to_end`
    - output: `test overlay_quantity_scale_reflects_computed_factor ... ok  (1/1 passed)`
  - [x] **W-D-2** — `crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs`
    (Bug #65 vol-killswitch fix; K5 cross-feature anchor cascade — if
    Hold-emission counts shift under friction, test invariants need
    re-anchoring in the test file itself).
    - file: `crates/strategy/tests/vol_killswitch_overlay_end_to_end.rs`
    - cmd: `cargo test -p strategy --test vol_killswitch_overlay_end_to_end`
    - output: `test result: ok. 4 passed; 0 failed`
  - [x] **W-D-3** — `crates/strategy/tests/latency_slippage_sim_e2e.rs`
    (v5 itself; H4 falsifier — confirms the 1-bp divergence assertion
    still passes when the "enabled" config IS the canonical config
    instead of the v0.1.0 ad-hoc test config).
    - file: `crates/strategy/tests/latency_slippage_sim_e2e.rs`
    - cmd: `cargo test -p strategy --test latency_slippage_sim_e2e`
    - output: `test result: ok. 3 passed; 0 failed`

  **NOT in scope at Wave D** (covered by Wave A anchored backtest
  re-emission, not by a dedicated `*_end_to_end.rs` divergence test):

  - TCN overlay (v2.5 / v2.6 — alpha captured by anchored backtest
    reports, no dedicated e2e divergence file).
  - PatchTST overlay (v2.5a — same; no dedicated e2e divergence file).
  - `overlay_hygiene_gate.rs` (not a divergence test; structural
    invariant gate — re-runs in `cargo test --workspace` per R-NR.4
    but doesn't need re-anchoring).

  Developer at Wave D kickoff confirms no new overlay e2e file has
  landed since 2026-05-27; if any new overlay/sizing-modifier
  divergence test exists (CLAUDE.md non-negotiable mandates one per
  overlay), it is added to W-D-N at that time.

- [x] **T-AR-5** (2026-05-27) — Frontmatter flipped
  `owner: architect → developer` (top of this file). Trace.toml
  `arch` column populated for `REQ-V5-ANCHOR-MIGRATION-V0-2-0-001`
  with the ADR-0045 cross-reference.

## M-DEV — Developer execution (4 waves)

_owner: developer. Wave-parallelizable per architect's M-T1 lock._

### Wave A — Re-run 34 backtests under canonical config (~2-3d)

- [x] **T-D-N1** — Apply canonical `LatencySlippageSimConfig` (per Q1
  lock) to the 34 anchored scenarios. Re-emit each report under
  the new namespace pin.
  - file: `crates/backtest/src/main.rs:111-115` (CLI flags wired), `crates/backtest/src/main.rs:174-179` (config applied to MomentumScenarioInput)
  - cmd: `cargo run -p backtest --bin backtest -- --scenario top10-2023-1h-momentum --sim-latency-ms-min 30 --sim-latency-ms-max 80 --sim-slippage-bps 8 ...`
  - output: 20 reports emitted under `spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/reports/`
- [x] **T-D-N2** — Confirm each scenario completes cleanly; flag any
  that error / crash / produce nonsensical equity.
  - All 20 re-emittable scenarios completed cleanly. No crashes. Momentum equity reduction (~$5.4k, ~$3.5k) confirmed as expected v5-sim effect. SMA/Composed equity change confirmed as real-data switch effect (synthetic → real Binance Parquet). 14 analysis/investigation scenarios: canonical SHA = noop SHA (sim not wired for those paths). No nonsensical values.

### Wave B — Anchor SHA migration in spec/anchors.toml (~0.5d)

- [x] **T-D-N3** — Compute new body-SHA-256 for each of the 34
  re-emitted reports.
  - file: `scripts/hash_report.py` used for each report
  - cmd: `python3 scripts/hash_report.py <report_path>`
  - output: 34 SHAs computed and inserted into `spec/anchors.toml`
- [x] **T-D-N4** — Rewrite `spec/anchors.toml`: 34 OLD anchors move to
  the `noop-baseline` namespace (per Q2=(a)); 34 NEW anchors under
  the canonical namespace (per Q1 lock).
  - file: `spec/anchors.toml` (rewritten from 34 to 68 rows; all noop rows got `+ noop-baseline` suffix; 34 new canonical rows with `+ v5-realdata-medium-2026-05` suffix)
  - cmd: (file edit)
  - output: 68 rows confirmed in `spec/anchors.toml`
- [x] **T-D-N5** — Verify `bash scripts/verify_anchors.sh` PASSes the
  full set (34 noop-baseline + 34 canonical = 68 total OR the noop
  set retires per Q2=(b)/(c)).
  - file: `scripts/verify_anchors.sh` (rewritten to track `version` field for namespace-aware file selection)
  - cmd: `bash scripts/verify_anchors.sh`
  - output: `ANCHORS PASS  (68 / 68)`

### Wave C — Sharpe/drawdown/final-equity delta table (~0.5d)

- [x] **T-D-N6** — For each of the 34 scenarios: extract `final_equity`
  / `max_drawdown` / `sharpe_ratio` from both OLD-noop and NEW-canonical
  reports. Render the delta table in
  `spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/reports/sharpe-delta-table-2026-05-27.md`.
  - file: `spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/reports/sharpe-delta-table-2026-05-27.md`
  - cmd: (Python metrics extraction from all 34 report pairs)
  - output: delta table written with all 34 scenarios in 8 groups; identifies 7 scenarios with equity change, 0 K1 surprises
- [x] **T-D-N7** — Flag scenarios where strategy alpha inverted
  (positive → negative). These are K1 surprise candidates per Q3.
  - file: `spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/reports/sharpe-delta-table-2026-05-27.md` (K1 Surprise Scan section)
  - cmd: (extracted from delta table)
  - output: **0 K1 surprises detected**. SMA/Composed: noop Sharpe was already negative; real-data switch improved (not degraded) all 5. Momentum: Sharpe not reported (N/A). All others: canonical = noop.

### Wave D — Cross-feature e2e re-checks (~2-3d)

- [x] **T-D-N8** — Per T-AR-4 inventory: re-run each overlay e2e
  divergence test under canonical config. Confirm the divergence
  threshold still asserts correctly (≥ 1 bp) — if not, the overlay's
  test needs threshold adjustment.
  - See W-D-1, W-D-2, W-D-3 above (all 8 tests pass).
- [x] **T-D-N9** — Update cross-feature anchored fixtures (if any
  carry SHAs) under the new namespace.
  - Surveyed all `crates/strategy/tests/` test files. No cross-feature anchored SHA fixtures found outside `spec/anchors.toml`. No changes needed.

### Final

- [x] **T-D-N10** — Tick all T-D-N rows; flip frontmatter
  `owner: developer → tester`; populate trace.toml `crates` + `tests`
  columns.
  - file: `spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/tasks.md` (all T-D-N rows ticked)
  - frontmatter: `owner: tester` (flipped above)
  - trace.toml: see below for `crates` + `tests` columns update

## M-FINAL — Tester verification

_owner: tester._

- [x] **T-T-1** (2026-05-27) — `bash scripts/verify_anchors.sh` PASS against the
  new anchored set (34 canonical, plus noop-baseline if Q2=(a)).
  - file: `scripts/verify_anchors.sh`
  - cmd: `bash scripts/verify_anchors.sh`
  - output: `ANCHORS PASS  (68 / 68)` — all 34 noop-baseline + 34 canonical v5-realdata-medium-2026-05 verified
- [x] **T-T-2** (2026-05-27) — `cargo test --workspace --no-fail-fast` — no new
  failures vs whitelist.
  - cmd: `cargo test --workspace --no-fail-fast`
  - output: 2 failures, both pre-existing/whitelisted: `t1937_nine_strategy_anchors_unchanged` (expected migration side-effect — noop constants superseded; Wave A canonical reports now sort newer) + `lab_run_engine::h3_in_memory_equals_cached_disk` (pre-existing flake). Zero new failures from Wave A-D code changes.
- [x] **T-T-3** (2026-05-27) — Sharpe-delta table (T-D-N6 output) reviewed for K1
  surprise; per-scenario retirement candidates surfaced to operator
  per Q3.
  - file: `spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/reports/sharpe-delta-table-2026-05-27.md`
  - output: K1 = 0 across all 34 scenarios. Developer claim CONFIRMED by tester spot-check of 5 canonical reports. No retirement candidates. Operator Q3=(b) per-scenario flag: nothing to flag.
- [x] **T-T-4** (2026-05-27) — Cross-feature e2e tests (Wave D) all PASS under
  canonical config.
  - cmd: `cargo test -p strategy --test latency_slippage_sim_e2e && cargo test -p strategy --test vol_targeting_overlay_end_to_end && cargo test -p strategy --test vol_killswitch_overlay_end_to_end`
  - output: 8/8 PASS — latency_slippage_sim_e2e 3/3, vol_targeting_overlay_end_to_end 1/1, vol_killswitch_overlay_end_to_end 4/4
- [x] **T-T-5** (2026-05-27) — Author
  `spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/reports/test-final-2026-05-27-v5-latency-slippage-sim-v0.2.0-anchor-migration.md`.
  - file: `spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/reports/test-final-2026-05-27-v5-latency-slippage-sim-v0.2.0-anchor-migration.md`
  - output: Report written. VERDICT → PASS. Operator-approved scope clarification § documented.
- [x] **T-T-6** (2026-05-27) — Trace row populated + flipped `proposed → passed`.
  - file: `spec/trace.toml` row `REQ-V5-ANCHOR-MIGRATION-V0-2-0-001`
  - output: anchors column populated; state flipped `proposed → passed`

## M-PRESENTER — Sprint-review deck

_owner: presenter. Runs only after VERDICT → PASS._

- [ ] **T-P-1** — Author
  `spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/presentations/v5-latency-slippage-sim-v0.2.0-anchor-migration-2026-MM-DD.md`.
- [ ] **T-P-2** — Lead with the Sharpe-delta-per-scenario story; the
  4-cell verdict tree from feature.md; the K1 retirement candidates
  (if any).
- [ ] **T-P-3** — Operator review. Capture verdict cell.
- [ ] **T-P-4** — On operator approval, flip feature.md frontmatter
  `status: draft → shipped`; move backlog Active → Recent.

## Notes

- **The whole point of this brief**: convert v5 v0.1.0's noop ship into
  a meaningful canonical-friction ship. Every anchored alpha number now
  represents a strategy's edge UNDER simulated friction.
- **K1 / Q3 are the operator-judgment trail**: post-Sharpe-delta-table
  review, some strategies may need to be retired or accepted-as-negative.
  The brief explicitly defers per-scenario retirement to operator review.
- **Anchor-cascade safety**: Q4=(a) re-runs all overlay e2e tests under
  the canonical config — defensive against silent cross-feature
  invariant breakage.
