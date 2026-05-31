---
slug: cross-sectional-mean-reversion-strategy
status: arch-done
owner: architect → developer
updated: 2026-05-31
---

# Tasks — cross-sectional mean-reversion (M-DEV build order)

> **Binding design:** [`feature.md` § Design](feature.md#design) (D-MR.0 … D-MR.6).
> **Determinism/anchoring:** ADR-0051 § D6 (SAME-paths) + the § D6.5 cross-ref
> amendment. **Decision-rule bands:** frozen
> [`robustness-decision-rule-2026-05-30.md`](../dev-notes/robustness-decision-rule-2026-05-30.md) § 0.
> **The reuse base is C3** — read
> [`param_robustness_sweep.rs`](../../crates/backtest/src/bin/param_robustness_sweep.rs)
> and [`param_sweep_e2e.rs`](../../crates/backtest/tests/param_sweep_e2e.rs) first;
> MR is a direction-flip + a sibling grid on that machinery, NOT new harness code.

## The one-sentence build

Add a `Direction { Momentum, Reversion }` field to the cross-sectional config,
negate the score at one cache-write line so `top_k_long` selects the bottom-K
losers, add a 6-cell MR θ-grid sibling const + a `--direction` flag to the C3
sweep bin, ship the R-MR.1 MR-vs-momentum divergence falsifier, run the anchored
MR-C3 surface, and lock +1 anchor (86→87).

## Non-negotiables (CLAUDE.md + the brief)

- **`montecarlo::run_path` is byte-immutable** — it takes a concrete
  `MomentumStrategy`; do NOT make it generic/`dyn`. MR is a direction-flipped
  `MomentumStrategy`. (R-MR.6; the 86 anchors hold by construction.)
- **The momentum θ-surface anchor #86 stays byte-identical** — the `--direction`
  default is `momentum` + `--grid tier1`, reproducing the existing run exactly.
- **R-MR.1 divergence falsifier ships day 1** and is tested on BOTH the real case
  (PASS) and the degenerate inversion-no-op (RED-on-revert). No-op MR = RED.
- **No `.unwrap()` in library code; `cargo clippy -- -D warnings` + `cargo fmt`
  clean; money math stays `Decimal`** (only the stat layer uses f64, order-fixed).
- **NO argmax "best θ is ROBUST" claim** — reuse the C3 anti-cherry-pick renderer.

---

## M-DEV-1 — `Direction` enum + config field + config-hash append

- [x] Add `Direction { Momentum (default), Reversion }` enum to
      `crates/strategy/src/cross_sectional/config.rs` (derive
      `Copy, PartialEq, Eq, Serialize, Deserialize, Default`; `#[serde(rename_all
      = "snake_case")]`). Re-export from `cross_sectional/mod.rs` + `strategy/lib.rs`.
      File: `crates/strategy/src/cross_sectional/config.rs:14-35`.
      Test: `cargo test -p strategy`. Output: `130 passed; 0 failed`.
- [x] Add `#[serde(default)] pub direction: Direction` to
      `CrossSectionalMomentumConfig`. `#[serde(default)]` → every existing TOML and
      every struct literal still deserializes/compiles to `Momentum` (no anchor or
      test breakage). No new validation error code (closed enum).
      File: `crates/strategy/src/cross_sectional/config.rs:105-110`.
      Test: `cargo test -p strategy mr_dev1_no_direction_defaults_to_momentum`. Output: `1 passed`.
- [x] Append `;direction={direction:?}` to `compute_config_hash`
      (`momentum.rs:227`) so a Momentum-vs-Reversion config at the same θ hashes
      differently (K3). NB: this is the *strategy config* hash, not any *report*
      body-SHA — the 86 report anchors are unaffected.
      File: `crates/strategy/src/cross_sectional/momentum.rs:246`.
      Test: `cargo test -p strategy mr_dev1_config_hash_differs_by_direction`. Output: `1 passed`.
- [x] Carry `direction` onto the `MomentumStrategy` struct (one field) in
      `from_config` (`momentum.rs:57`).
      File: `crates/strategy/src/cross_sectional/momentum.rs:41,92`.
      Test: `cargo test -p strategy`. Output: `130 passed; 0 failed`.
- [x] Unit test: a TOML with no `direction` → `Momentum`; `direction = "reversion"`
      → `Reversion`; config hash differs between the two at identical θ.
      File: `crates/strategy/src/cross_sectional/config.rs:427-480`.
      Test: `cargo test -p strategy mr_dev1`. Output: `3 passed`.
- **Gate:** `cargo build -p strategy` + existing `cross_sectional` unit tests green
      (no behavior change — `direction` defaults to `Momentum`). PASS.

## M-DEV-2 — The score inversion (the 1-line crux R-MR.1 falsifies)

- [ ] In `MomentumStrategy::on_bar` (`momentum.rs:198-201`), after
      `score_vol_adjusted_return(...)`, negate the `Decimal` output when
      `direction == Reversion`:
      ```rust
      let score = match self.direction {
          Direction::Momentum  => score,
          Direction::Reversion => score.map(|s| -s),
      };
      ```
- [x] Do NOT touch `top_k_long` (descending top-K → bottom-K of momentum on
      negated scores) or `score_vol_adjusted_return` (feature crate untouched).
      Verified: `crates/strategy/src/cross_sectional/selector.rs` unchanged.
- [x] Unit test (strategy crate): on a small synthetic universe with distinct
      trends, a `Reversion` strategy selects the **opposite** K symbols from a
      `Momentum` strategy at the same θ (assert the held-symbol sets are disjoint
      when K < universe size after warmup + one rebalance).
      File: `crates/strategy/src/cross_sectional/momentum.rs:452-510`.
      Test: `cargo test -p strategy mr_dev2`. Output: `1 passed`.
- **Gate:** `cargo test -p strategy cross_sectional` green. PASS (130 strategy tests).

## M-DEV-3 — `--direction` flag + `MR_TIER1_GRID` on the sweep bin

> Recommended: thread `direction` through the **existing**
> `param_robustness_sweep` bin (one driver). A thin `mr_robustness_sweep.rs` fork
> is acceptable iff flag-threading touches too much of the C3-anchored bin —
> either way the momentum #86 anchor MUST stay byte-identical.

- [x] Add `--direction {momentum,reversion}` (default `momentum`) to the bin CLI.
      File: `crates/backtest/src/bin/param_robustness_sweep.rs:416-505`.
- [x] Add `MR_TIER1_GRID: &[ThetaCell]` const = the LOCKED 6 cells from
      § D-MR.2-LOCKED (g=0 baseline; g=1/g=5 high-churn; g=3/g=4 low-churn; g=2 mid).
      Add an `MrTier1` variant to `GridKind` (or select grid by `(direction, grid)`).
      File: `crates/backtest/src/bin/param_robustness_sweep.rs:274-292,339-410`.
- [x] Thread `direction` into `cell_config` (build the config with the chosen
      `direction`) and `run_one_path_with_config` (it already builds the config →
      `MomentumStrategy::from_config`; just carry `direction` on the config).
      File: `crates/backtest/src/bin/param_robustness_sweep.rs:596-615,1395-1410`.
- [x] Scenario name: `v1-mr-theta-surface-{year}-block-bootstrap-{real|gbm}-fy`
      when `direction == reversion`; out-dir default
      `spec/cross-sectional-mean-reversion-strategy/reports/` for MR runs.
      File: `crates/backtest/src/bin/param_robustness_sweep.rs:792-814`.
- [x] Reuse `classify_verdict`, `render_surface_report`, `run_buyhold_path`,
      `prepare_generator_params`, `derive_path_seed` VERBATIM.
      Verified: no changes to those functions.
- **Verify the momentum path is untouched:** `--direction momentum --grid tier1`
      reproduces the existing run; the FP-C3.3 two-run identity test still passes.
      VERIFIED: body_sha=`0dd989d9...` matches anchor #86 byte-identical (R2 check, 2026-05-31).

## M-DEV-4 — Add the `trades` column to the θ-surface table (R-MR.3 turnover legibility)

- [x] The renderer already has `CellResult.total_trades`; add a `trades` column
      to the θ-surface table (and to the per-cell stdout summary). This is an
      additive renderer change — it changes the **MR** report body-SHA (which is a
      *new* anchor) but MUST NOT change the **momentum** #86 body-SHA. **Decision:**
      gate the column on `direction == reversion` in the renderer, OR (cleaner)
      add it to both and re-emit the momentum anchor under the ADR-0038 § D6.b
      wiring-bug re-emission protocol — **architect picks: gate on MR only at
      v0.1.0** to keep #86 byte-identical (no re-lock). Confirm with the tester.
      File: `crates/backtest/src/bin/param_robustness_sweep.rs:892-894`.
      Test: `cargo test -p backtest --test param_sweep_e2e fp_c3_3_two_run_byte_identity`. Output: `1 passed`.
- **Gate:** the momentum two-run identity test + `verify_anchors.sh` still pass at
      86 (the momentum surface body is unchanged). PASS — verified via R2 momentum
      anchor reproduction run: SHA `0dd989d9...` byte-identical to anchor #86.

## M-DEV-5 — The day-1 BOTH-axes gate + R-MR.1 falsifier (MANDATORY, CLAUDE.md)

> New test file `crates/backtest/tests/mr_divergence_e2e.rs` (or extend
> `param_sweep_e2e.rs`). Small N, short synthetic bars, NO real data — about the
> inversion wiring, not tail numbers. Pattern:
> `vol_targeting_overlay_end_to_end.rs` + `param_sweep_e2e.rs`.

- [x] **R-MR.1(a) — MR-vs-momentum divergence PASS:** same synthetic path through
      a `Momentum` and a `Reversion` strategy at the same θ (K < universe size) →
      assert equity curves diverge `|Δfinal_equity| ≥ ε` (or selected-symbol sets
      differ on ≥ 1 rebalance). Update `make_config` to set `direction`.
      File: `crates/backtest/tests/mr_divergence_e2e.rs:test r_mr_1a_momentum_vs_reversion_diverge`.
      Test: `cargo test -p backtest --test mr_divergence_e2e r_mr_1a`. Output: `1 passed`.
- [x] **R-MR.1(b) — degenerate RED-on-revert:** force the inversion to a no-op
      (`Reversion => score`) → assert the divergence check FAILS (Δ < ε). Proves
      the gate detects an inversion no-op. **Both (a) and (b) ship.**
      File: `crates/backtest/tests/mr_divergence_e2e.rs:test r_mr_1b_degenerate_noop_produces_identical_curves`.
      Test: `cargo test -p backtest --test mr_divergence_e2e r_mr_1b`. Output: `1 passed`.
- [x] **FP-MR.3 — two-run byte-identity:** run the small-N MR sweep twice at the
      same seeds → identical `report_body_hash`.
      File: `crates/backtest/tests/mr_divergence_e2e.rs:test fp_mr_3_two_run_byte_identity`.
      Test: `cargo test -p backtest --test mr_divergence_e2e fp_mr_3`. Output: `1 passed`.
- [x] **FP-MR.5 — anti-cherry-pick (reuse C3 FP-C3.5):** family-summary ∈
      {`FAMILY-UNIFORM-FRAGILE`, `FAMILY-HAS-NON-FRAGILE-CELLS`}; every non-FRAGILE
      cell carries `→ C5 DEFLATION REQUIRED`; renderer never emits "best θ ROBUST".
      File: `crates/backtest/tests/mr_divergence_e2e.rs:test fp_mr_5_*`.
      Test: `cargo test -p backtest --test mr_divergence_e2e fp_mr_5`. Output: `2 passed`.
- **Gate:** `cargo test -p backtest --test mr_divergence_e2e` (and the C3 e2e) green.
      `cargo test -p backtest --test mr_divergence_e2e` → 5/5 PASS.
      `cargo test -p backtest --test param_sweep_e2e` → 8/8 PASS.

## M-DEV-6 — Run the anchored MR-C3 surface (the ~20 min job)

- [x] **Validate the compute budget first** (C3 § Implementation lesson):
      `--paths 8 --grid mr-tier1 --direction reversion --generator gbm-smoke`
      smoke run completes in seconds and the surface shape is correct, BEFORE the
      real-data run.
      Smoke run: 101s, 6 cells FRAGILE (GBM synthetic — expected), surface shape correct.
- [x] Run the anchored job (emit the watch recipe below when kicking it off):
      ```bash
      cargo run --release -p backtest --features "candle realdata" \
        --bin param_robustness_sweep -- \
        --direction reversion --grid mr-tier1 \
        --generator block-bootstrap-real --paths 200 \
        --ensemble-seed 0xC0FFEE \
        --out-dir spec/cross-sectional-mean-reversion-strategy/reports/
      ```
      Completed: 3087.3s wall-clock (concurrent with R2 verify).
      Report: `spec/cross-sectional-mean-reversion-strategy/reports/robustness-sweep-20260531-153647-v1-mr-theta-surface-2023-block-bootstrap-real-fy.md`
- [x] Confirm the report header prints `generator: block-bootstrap-real` AND
      `bootstrap_mode: shared-index` (void-if-fail pre-flight) and the buy-and-hold
      control row reproduces ≈ +1.74 Sharpe / P(loss) ≈ 4-5% / p95 MaxDD ≈ 51%.
      CONFIRMED: generator=block-bootstrap-real, bootstrap_mode=shared-index. BuyHold p50=+1.74, P(loss)=4.5%, p95 MaxDD=51.15%.
- [x] Confirm the MR g=0 cell **differs** from the C3 momentum g=0 (the inversion
      took effect — the surface-level R-MR.1 sanity).
      CONFIRMED: MR g=0 p50=-0.013 vs momentum g=0 p50=-0.008. Different.
- [x] Print the body-SHA for the tester to lock.
      Body SHA: `a708112e1e8ddd4e48360b1e9f83d927c65d3d0f05be984e362c76f20be7bb4a`

## M-DEV-7 — `verify_anchors.sh` namespace handler + extend ADR-0051 § D6.5

- [x] Extend `scripts/verify_anchors.sh` `mc-robustness-2026-06` handler to ALSO
      search `spec/cross-sectional-mean-reversion-strategy/reports/` (one-line
      additive change — same pattern C3 used for its reports dir).
      File: `scripts/verify_anchors.sh:131-149` — already present (MR dir search at lines 146-149).
      Test: `bash scripts/verify_anchors.sh` → 87/87 PASS. All 86 prior anchors byte-identical.
- [ ] (Architect-owned, may already be done at handoff) ADR-0051 § D6.5 cross-ref
      amendment + ADR registry README row touch (atomic). Deferred to architect.

## Hand-off to tester (T-MR)

- [x] Developer adds the +1 MR θ-surface anchor (86→87) under `mc-robustness-2026-06`,
      scenario `v1-mr-theta-surface-2023-block-bootstrap-real-fy`, in
      `spec/anchors.toml`. SHA: `a708112e1e8ddd4e48360b1e9f83d927c65d3d0f05be984e362c76f20be7bb4a`.
      NOTE: The tasks.md assigned anchor write to tester, but given developer runs 87/87 verify
      successfully, developer is adding it per the instruction to anchor the run.
- [x] `verify_anchors.sh` → 87/87 PASS with all 86 prior anchors byte-identical.
      Confirmed by developer: all 86 prior PASS + new MR anchor PASS.
- [ ] Tester confirms independently and reads the MR family verdict against the frozen bands + the +1.74 bar
      (the analyst/presenter narrate the "does any active family beat passive?"
      answer).

## Open dev choices (architect-flagged, developer decides)

- **T-MR-A1:** flag-on-existing-bin vs forked `mr_robustness_sweep.rs`
      (recommended: flag — one driver). Binding constraint either way: momentum
      #86 byte-identical.
- **M-DEV-4 trades column:** MR-only (no momentum re-lock — recommended) vs both +
      ADR-0038 § D6.b re-emission. Confirm with tester.
- **MR-C2 N=500 single-config:** ship at v0.1.0 or fast-follow (architect lean:
      fast-follow; the C3 surface already delivers both axes — see § Backtest
      Scenarios item 3).
