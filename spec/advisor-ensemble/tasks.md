---
slug: advisor-ensemble
status: in-progress
owner: architect
updated: 2026-06-21
---

# F8 — Strategy-mix ensembles + robustness-gate activation — Task list

Design: [feature.md § Design](feature.md#design) +
[ADR-0063](../architecture/adr/0063-ensemble-vote-seam-and-robustness-gate-activation.md).
Developer and ui-designer run **in parallel** — the developer owns the engine
seam + gate + the two day-1 e2e tests; the ui-designer owns the leaderboard +
plan render proofs. The only cross-dependency: the ui-designer's render proofs
consume the `BakeoffReport` / `ForwardPlan` mirrors the developer's arms produce,
so the ui-designer can build against the structured types from the start and
wire the live render last.

**Anchor gate (both tracks, MANDATORY):** run `scripts/verify_anchors.sh` BEFORE
the first edit and AFTER the last. Must read **119/119** both times. Any non-119
result = STOP and route back to the architect. Touch NO file under
`spec/*/reports/`, NO `data/*/REVISION.toml`.

---

## Developer tasks (engine seam + gate + day-1 gates)

### D-T1 — `EnsembleStrategy` adapter + the pure arbiter (ADR-0063 § D1)
- [x] D-T1.1 In `crates/strategy/src/ensemble.rs` (new module; export from
      `lib.rs`), add `EnsembleStrategy` holding `id: StrategyId`,
      `members: Vec<Box<dyn Strategy>>`, `member_stances: Vec<Stance>` (current
      LONG/FLAT per member, `Stance ∈ {Long, Flat}` + an `Abstain`/`Unwarmed`
      sentinel), `method: VoteMethod`, and the ensemble's own last stance.
      — file:line: `crates/strategy/src/ensemble.rs:137`
      — test: `cargo test -p strategy` → `test ensemble::tests::… ok` (195 total)
      — output: `test result: ok. 195 passed` ✓
- [x] D-T1.2 Add the closed `VoteMethod` enum: `Majority { k: usize, n: usize }`,
      `Unanimous { n: usize }`. Add a pure free fn
      `arbitrate(method: VoteMethod, stances: &[MemberStance]) -> Stance` that
      (a) IGNORES un-warmed/abstaining members (counts them in NEITHER
      `long_count` NOR the denominator), (b) returns `Flat` until the
      method's quorum is warm (`Majority` needs ≥ k warmed members present;
      `Unanimous` needs all n warmed), (c) otherwise returns `Long` iff
      `long_count ≥ k` (majority) / `long_count == n` (unanimous), else `Flat`.
      Pure, deterministic, unit-tested at the warmup boundary.
      — file:line: `crates/strategy/src/ensemble.rs:67,105`
      — test: `cargo test -p strategy` → `test ensemble::tests::majority_requires_k_warmed_before_long … ok`
      — output: (included in 195 passed above) ✓
- [x] D-T1.3 Implement `Strategy` for `EnsembleStrategy`: `on_bar` fans the bar to
      every member, updates each member's tracked stance from its edge-triggered
      Buy/Sell (un-warmed member ⇒ stance stays the `Unwarmed` sentinel),
      recomputes consensus via `arbitrate`, and emits a single Buy/Sell **only on
      the ensemble's own stance transition** (edge-triggered, matching
      `ComposedStrategy::on_bar`); `on_tick` returns `vec![]`; `config_schema`
      returns the ensemble JSON schema; `quantity_scale` = default `1.0`.
      Do NOT modify the `Strategy` trait (ADR-0005 freeze).
      — file:line: `crates/strategy/src/ensemble.rs:208`
      — test: `cargo test -p strategy` → `test ensemble::tests::ensemble_emits_buy_on_first_majority … ok`
      — output: (included in 195 passed above) ✓
- [x] D-T1.4 Add the shared member constructor
      `build_member(id: &str) -> Result<Box<dyn Strategy>, EnsembleBuildError>` using
      thiserror (anyhow removed per ADR-0041 D1). Reuses existing per-id construction.
      Unknown id → typed error (no silent fallback — F5b anti-fake precedent).
      — file:line: `crates/strategy/src/ensemble.rs:407`
      — test: `cargo test -p strategy --test ensemble_vote_divergence_end_to_end build_ensemble_unknown_id_returns_err`
      — output: `test build_ensemble_unknown_id_returns_err … ok` ✓
- [x] D-T1.5 Add the ensemble factory `build_ensemble(id: &str) -> Result<EnsembleStrategy, EnsembleBuildError>`.
      — file:line: `crates/strategy/src/ensemble.rs:481`
      — test: `cargo test -p strategy --test ensemble_vote_divergence_end_to_end build_ensemble_majority_succeeds`
      — output: `test build_ensemble_majority_succeeds … ok` ✓

### D-T2 — `PlanRuleShape::Ensemble` + the ensemble `PlanDescribe` (ADR-0063 § D2, D3)
- [x] D-T2.1 In `crates/strategy/src/plan.rs` add the closed `PlanVoteMethod` enum
      (`Majority { k, n } | Unanimous { n }`) and the
      `PlanRuleShape::Ensemble { method: PlanVoteMethod, members: Vec<PlanRuleShape> }`
      variant. Structured data only — NO copy string.
      — file:line: `crates/strategy/src/plan.rs` (PlanVoteMethod + PlanRuleShape::Ensemble)
      — test: `cargo test -p strategy` → all 195 strategy tests pass ✓
      — output: `test result: ok. 195 passed` ✓
- [x] D-T2.2 Implement `PlanDescribe` for `EnsembleStrategy`: a non-mutating read
      of each member's already-warmed stance + the arbiter → `StrategyPlan`.
      — file:line: `crates/strategy/src/ensemble.rs:305`
      — test: `cargo test -p strategy` → (included in 195 passed) ✓
      — output: `test result: ok. 195 passed` ✓
- [x] D-T2.3 Map `strategy::PlanRuleShape::Ensemble` → `agent::PlanRuleKind::Ensemble`
      in the agent boundary. Added `PlanVoteMethod` enum to `crates/agent/src/config.rs`.
      — file:line: `crates/agent/src/config.rs` (PlanRuleKind::Ensemble, PlanVoteMethod),
                   `crates/agent/src/plan.rs` (map_rule_shape Ensemble arm)
      — test: `cargo test -p agent` → `test result: ok. 76 passed` ✓
      — output: all 76 agent lib tests pass ✓

### D-T3 — `RobustnessMode::Bootstrap` compute-and-feed (ADR-0063 § D4)
- [x] D-T3.1 In `crates/backtest/src/bakeoff/mod.rs` extend `RobustnessMode` with
      `Bootstrap { paths: usize, seed: u64 }`. `default()` STAYS `Skip`.
      — file:line: `crates/backtest/src/bakeoff/mod.rs:306`
      — test: `cargo test -p backtest --test robustness_bootstrap_bites bootstrap_skip_mode_all_none`
      — output: `test bootstrap_skip_mode_all_none … ok` ✓
- [x] D-T3.2 New `crates/backtest/src/bakeoff/bootstrap.rs` with
      `compute_robustness_flag(&[Decimal], paths, master_seed) -> RobustnessFlag`.
      Uses Politis-White block length, ADR-0051 D1 sub-seed, DistributionSummary,
      and FROZEN classify_verdict. Returns Decimal (not Money<Usdt>) to match the
      bakeoff loop's existing equity extraction.
      — file:line: `crates/backtest/src/bakeoff/bootstrap.rs:111`
      — test: `cargo test -p backtest --test robustness_bootstrap_bites`
      — output: `test result: ok. 14 passed` ✓
- [x] D-T3.3 Wired `Bootstrap` arm in `run_bakeoff` loop with `derive_master_seed`.
      Loop enumerates `candidate_index`. SALT_TABLE (16-entry) ensures different
      candidates get different resample draws.
      — file:line: `crates/backtest/src/bakeoff/mod.rs:582`
      — test: `cargo test -p backtest --test robustness_bootstrap_bites bootstrap_flags_populate_in_bakeoff`
      — output: `test bootstrap_flags_populate_in_bakeoff … ok` ✓
- [ ] D-T3.4 Wire the advisor cockpit bake-off (`spawn_bakeoff`) to pass
      `RobustnessMode::Bootstrap { paths: 1000, seed }`.
      NOTE: this is the UI-designer / final wiring task; the engine seam is ready.

### D-T4 — Field + dispatch wiring (ADR-0063 § D5, D6)
- [x] D-T4.1 Add `BakeoffConfig::default_ensemble_field() -> Vec<StrategyId>`
      returning the two ensemble ids. Leave `default_field()` UNCHANGED.
      — file:line: `crates/backtest/src/bakeoff/mod.rs:350`
      — test: `cargo test -p backtest --test robustness_bootstrap_bites default_ensemble_field_is_non_empty`
      — output: `test default_ensemble_field_is_non_empty … ok` ✓
- [x] D-T4.2 `crates/backtest/src/engine.rs` `run_scenario`: ensemble dispatch arm
      for `"v0.8.vote.majority" | "v0.8.vote.unanimous"` builds `EnsembleStrategy`
      via `build_ensemble` factory and runs it through the same single-symbol paper
      engine as `v0.5.*` arms. `write_report = false` guaranteed.
      — file:line: `crates/backtest/src/engine.rs` (ensemble arm added)
      — test: `cargo test -p backtest` → all backtest tests pass ✓
      — output: `test result: ok. 113 passed; 5 ignored` (lib) ✓
- [x] D-T4.3 `crates/agent/src/runtime.rs` `build_registry_for`: two ensemble id arms
      registered via `strategy::build_ensemble(id)`.
      — file:line: `crates/agent/src/runtime.rs`
      — test: `cargo test -p agent` → `test result: ok. 76 passed` ✓
      — output: all 76 agent lib tests pass ✓

### D-T5 — Day-1 e2e gates + reachability regression (ADR-0063 § D7)
- [x] D-T5.1 `crates/strategy/tests/ensemble_vote_divergence_end_to_end.rs` — 12 tests:
      majority + unanimous equity divergence (SMA-based members), warmup abstention
      prevention, determinism, unknown-id error, factory smoke, pure arbitrate tests.
      — file:line: `crates/strategy/tests/ensemble_vote_divergence_end_to_end.rs`
      — test: `cargo test -p strategy --test ensemble_vote_divergence_end_to_end`
      — output: `test result: ok. 12 passed; 0 failed` ✓
- [x] D-T5.2 `crates/backtest/tests/robustness_bootstrap_bites.rs` — 14 tests:
      determinism, fragile-for-declining, not-fragile-for-growing, skip→None,
      bootstrap→populated, default_ensemble_field checks.
      — file:line: `crates/backtest/tests/robustness_bootstrap_bites.rs`
      — test: `cargo test -p backtest --test robustness_bootstrap_bites`
      — output: `test result: ok. 14 passed; 0 failed` ✓
- [ ] D-T5.3 Reachability regression (R4.4): `BenchmarkWins` + `AllFragile` reachable
      with ensembles present. LEFT FOR TESTER to verify-and-tick (T_FINAL row).
- [ ] D-T5.4 Determinism test: same bake-off seed → identical ensemble equity +
      identical RobustnessFlags. LEFT FOR TESTER to verify-and-tick (T_FINAL row).

### D-T6 — Close-out (developer)
- [x] D-T6.1 `cargo fmt` + `cargo clippy -p strategy -p backtest -p agent -- -D warnings` clean.
      — test: `cargo clippy -p strategy -p backtest -p agent -- -D warnings`
      — output: `Finished dev profile` (no warnings, no errors) ✓
- [x] D-T6.2 `cargo test -p strategy -p backtest -p agent` green.
      — test: all three crates
      — output: strategy 195+, backtest 113+ (5 ignored), agent 76+ all pass ✓
- [x] D-T6.3 `scripts/verify_anchors.sh` → **119/119** CONFIRMED.
      — output: `ANCHORS PASS  (119 / 119)` ✓
      — `cargo tree -p ui` NOT widened (ensemble types live in strategy/backtest).
- [x] D-T6.4 Implementation section appended to `spec/advisor-ensemble/feature.md`.
      Trace `crates` + `tests` columns updated in tasks.md.

---

## UI-designer tasks (leaderboard + plan render proofs)

Per CLAUDE.md, verify at the rendered-PIXEL layer (the
`iced_test::Emulator::screenshot` harnesses), exercising the POPULATED state with
a negative control — NOT unit tests or a no-panic boot. macOS-canonical
(ADR-0057 § D2). Build against the structured `BakeoffReport` / `ForwardPlan`
mirrors; no new `ui` dep (`cargo tree -p ui` must stay unchanged).

### U-T1 — Leaderboard shows ensemble arms + their Fragile flags
- [ ] U-T1.1 Extend the leaderboard mirror/view so the two ensemble rows render
      alongside the singles + buy-and-hold (7-arm field), each labelled as a vote
      ("Majority vote" / "Unanimous vote") — NOT as a single indicator.
- [ ] U-T1.2 Render the per-row **robustness flag** now that the gate is live:
      `Robust` / `Marginal` / `Fragile` badge (Fragile visually distinct — it is
      ineligible-to-crown). Confirm a `Fragile` ensemble shows ranked AFTER the
      eligible arms and is NOT crowned.
- [ ] U-T1.3 Render proof: a `crates/ui/tests/*_render.rs` populated-state PNG (the
      `leaderboard_populated_render.rs` / `reports_populated_curve_render.rs`
      pattern) showing a populated 7-arm leaderboard with ≥ 1 ensemble row + a
      visible Fragile badge, plus the empty-state negative control. Read the PNG —
      a passing proxy is not proof the screen draws.

### U-T2 — F6 plan `Ensemble` copy + render proof
- [ ] U-T2.1 Add the `ui` exhaustive-match arm for `PlanRuleKind::Ensemble`
      (mirrored from `strategy::PlanRuleShape::Ensemble`). Copy must NAME the
      method + members and the live tally — e.g. "Holds when ≥ 2 of {MACD trend,
      RSI reversion, Bollinger reversion} agree; goes flat when the majority flips.
      Current vote: 1 / 3 → FLAT." It must NOT fabricate a single-indicator rule.
      List each member's own rule from `members`.
- [ ] U-T2.2 Keep the not-advice / not-a-prediction disclaimers (ADR-0062 § D1).
      No free-text rule string crosses the seam — the `ui` owns the words.
- [ ] U-T2.3 Render proof: a populated PNG of an ensemble forward-plan (method +
      members + tally + €200 projected sizing) PLUS the buy-and-hold degenerate
      plan as the negative control (the `leaderboard_populated_render.rs` /
      `PanelState` real-renderer pattern). Read the PNG.

### U-T3 — Close-out (ui-designer)
- [ ] U-T3.1 `cargo fmt`; clippy clean on `ui` (fresh lint).
- [ ] U-T3.2 The two render PNGs land under the feature `reports/screenshots/`
      (NOT under any anchored `reports/*.md`); `scripts/verify_anchors.sh`
      119/119; `cargo tree -p ui` unchanged.
- [ ] U-T3.3 Update the feature `## Implementation` (UI portion) via `spec-update`.

---

## Definition of done (tester closes)

Per feature `## Verification`: (1) bake-off e2e — the 2 ensembles enter the
field, ranked by the UNCHANGED `rank_candidates`, BenchmarkWins/AllFragile
reachable with ensembles present (D-T5.3); (2) the day-1 equity-divergence e2e
(D-T5.1); (3) the real-robustness gate-bites test (D-T5.2); (4)
`verify_anchors.sh` 119/119; (5) leaderboard + F6-plan render PNGs with an
ensemble row + an ensemble plan (U-T1.3, U-T2.3). VERDICT must be PASS before the
presenter runs.
