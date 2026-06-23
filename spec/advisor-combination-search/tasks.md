---
slug: advisor-combination-search
status: in-progress
owner: developer
updated: 2026-06-23
---

# Tasks — advisor-combination-search

> **Architect-ratified, developer-ready.** Design + decisions are in
> [`feature.md` § Design](feature.md) and [ADR-0067](../architecture/adr/0067-pre-registered-combination-slate-expansion.md).
> All six analyst OQs are resolved in the Design; this list is the ordered build.
> Trace `REQ-ADVISOR-COMBINATION-SEARCH-001`.

## Load-bearing constraints (carry into EVERY task — non-negotiable)

- **Pre-registration is the overfit-safety contract.** The slate is the FIXED 6
  arms in the table below — **NO search**, no id parser, no continuous knob.
- **Robustness bands FROZEN.** Do NOT touch `classify_verdict` /
  `compute_robustness_flag` / `verdict_bands` (`crates/backtest/src/bakeoff/robustness.rs`)
  or `bootstrap.rs` — byte-UNCHANGED (ADR-0059 §D4 / ADR-0063 §D4). NOT a B2/B3
  band proposal. Frame as "more candidates face the same bar."
- **Reuse-only.** `EnsembleStrategy` / `VoteMethod` / `arbitrate` / `build_member` /
  `run_bakeoff` / `rank_candidates` + the ADR-0066 benchmark exemption: VERBATIM.
  The 6 vote arms need ZERO new arbitration math.
- **Anchor-safe by construction.** New `v0.8.vote.*` ids run `write_report=false`
  on the `RobustnessMode::Bootstrap` advisor path. Run `scripts/verify_anchors.sh`
  **FIRST (before the first seam, expect 119/119) and AFTER the last** — any
  non-119 = STOP-and-route-back-to-architect. Anchors are keyed by NAME not
  filename; do not edit any `spec/*/reports/` body, `anchors.toml`, or
  `REVISION.toml`.
- **`BenchmarkWins` / `AllFragile` reachability UNCHANGED** (ADR-0066). A null
  all-Fragile field → `BenchmarkWins` must stay reachable with the 6 new arms in.
- **Day-1 baseline-equity-divergence e2e** (CLAUDE.md non-negotiable) for the new
  arms — T3, written FAIL-before / PASS-after, lands WITH the wiring (T1/T2).

## The FROZEN v1 slate (the only membership — do not adjust)

| id | VoteMethod | members |
|---|---|---|
| `v0.8.vote.trend_pair` | `Unanimous { n: 2 }` | `[v0.5.macd, v0.sma]` (predicted-null control) |
| `v0.8.vote.tr_mr_macd_rsi` | `Unanimous { n: 2 }` | `[v0.5.macd, v0.5.rsi]` |
| `v0.8.vote.tr_mr_sma_bb` | `Unanimous { n: 2 }` | `[v0.sma, v0.5.bbands]` |
| `v0.8.vote.any1of4` | `Majority { k: 1, n: 4 }` | `[v0.sma, v0.5.macd, v0.5.rsi, v0.5.bbands]` |
| `v0.8.vote.k2of4` | `Majority { k: 2, n: 4 }` | `[v0.sma, v0.5.macd, v0.5.rsi, v0.5.bbands]` |
| `v0.8.vote.k3of4` | `Majority { k: 3, n: 4 }` | `[v0.sma, v0.5.macd, v0.5.rsi, v0.5.bbands]` |

Member id → builder mapping already exists in `build_member` (`v0.sma`,
`v0.5.macd`, `v0.5.rsi`, `v0.5.bbands` are all handled) — no `build_member` edit.

---

## Tasks

- [ ] **T0 — Anchor baseline.** Run `scripts/verify_anchors.sh` and confirm
      **119/119** BEFORE any edit. Record the number.
      _acceptance: `verify_anchors.sh` prints 119/119; if not, STOP and route back._

- [ ] **T1 — Register the 6 arms in `build_ensemble`** (`crates/strategy/src/ensemble.rs`).
      Add 6 literal `match` arms mirroring the existing `v0.8.vote.majority` /
      `v0.8.vote.unanimous` arms — each builds its members via `build_member` and
      constructs `EnsembleStrategy::new(id, <VoteMethod from the table>, member_ids, members)`.
      Update the `build_ensemble` doc comment to list all 8 pre-registered ids.
      No new arbitration code, no `arbitrate` edit, no `member_id_to_rule_shape`
      edit (it already covers the 4 base ids).
      _acceptance: `build_ensemble("v0.8.vote.<arm>")` returns `Ok` for all 6 new
      ids and `Err(UnknownId)` for an unregistered id; `cargo build -p strategy`
      clean; `arbitrate`/`VoteMethod`/`bootstrap.rs`/`robustness.rs` git-diff EMPTY._

- [ ] **T2 — Widen the engine dispatch + the field list.**
      (a) `crates/backtest/src/engine.rs` (~line 1527): widen the `run_scenario`
      match pattern `"v0.8.vote.majority" | "v0.8.vote.unanimous"` to alternate the
      6 new ids (the arm BODY is unchanged — it already calls
      `strategy::build_ensemble(strategy_str)` generically). Update the arm's
      doc-comment id list.
      (b) `crates/backtest/src/bakeoff/mod.rs` `default_ensemble_field()`: add the
      6 new `StrategyId`s (now returns 8). This is the SINGLE source of truth —
      `advisor_field()` (`crates/ui/src/leaderboard/runner.rs:53`) picks them up
      automatically; do NOT add a second list anywhere.
      (c) Move the lockstep field-count test: `runner.rs:238-242`
      `cfg.request.field.len()` assertion `6 → 12`, and extend its
      `ids.contains(...)` set to include the 6 new ids (keep the
      `!ids.contains("v0.buyhold")` assertion). Update its message to "4 rule
      engines + 8 vote ensembles".
      _acceptance: `cargo build -p backtest -p ui` clean; `advisor_field().len() == 12`;
      a unit test confirms all 8 ensemble ids are in `default_ensemble_field()`;
      the moved runner test passes; `default_field()` is UNCHANGED (still 4 singles)._

- [ ] **T3 — Day-1 divergence e2e** — new file
      `crates/strategy/tests/combination_slate_divergence_end_to_end.rs`, modelled
      on `crates/strategy/tests/ensemble_vote_divergence_end_to_end.rs`. Reuse its
      `run_strategy_equity` + `sine_bars` harness. For **each of the 6 new arms**:
      (a) build the arm with **`SmaCrossover` members at distinct parameter pairs**
      (the vote-mechanics proxy — TOML members don't fire on synthetic bars, per
      the F8 precedent) and assert its final equity diverges from **at least one
      member curve by ≥ 1 bp** of initial capital (no silent passthrough);
      (b) assert the arm's equity diverges from **buy-and-hold (always-long) by ≥
      1 bp** AND that no two new arms produce identical curves on the same series
      (no accidental duplicate); plus a **factory smoke test** asserting each real
      `build_ensemble("v0.8.vote.<arm>")` (real 4 base TOMLs) builds `Ok` without
      panic. Write it FAIL-before/PASS-after: aliasing any arm to an existing id
      must break it.
      _acceptance: the test FAILS if any new arm's `match` arm is deleted or
      aliased to an existing arm's `(method, members)`; PASSES on the real wiring;
      `cargo test -p strategy --test combination_slate_divergence_end_to_end` green._

- [ ] **T4 — Build + validate + RE-VERIFY ANCHORS.** `cargo build --workspace`,
      `cargo clippy --workspace -- -D warnings`, `cargo test -p strategy -p backtest -p ui`
      (the existing F8 + ensemble + rank tests must stay green — they assert
      `BenchmarkWins`/`AllFragile` reachability), then run
      `scripts/verify_anchors.sh` again.
      _acceptance: clippy clean; all named test suites green; `verify_anchors.sh`
      still **119/119 byte-identical** (any non-119 = STOP-and-route-back)._

- [ ] **T5 — Tester: real-data 13-arm bake-off.** Run the advisor bake-off on
      BTCUSDT H1-2024 (`BinanceCache`, `RobustnessMode::Bootstrap{paths:1000}`,
      `LAB_DEFAULT_SEED`) over the live `advisor_field()` (now 13 arms incl.
      buy-and-hold). Produce a tester report under
      `spec/advisor-combination-search/reports/` that (1) **records the
      pre-registered prediction up front** (most/all Fragile → `BenchmarkWins`;
      `trend_pair` control shows little p5 lift; `Unanimous{n:2}` mixed pairs sit
      mostly Flat — see feature § Backtest Scenarios), (2) tabulates **all 13 arms**
      (flag, p5 Sharpe, p50 Sharpe, total-return, max-DD, trade_count), (3) reports
      the run-level `RecommendationOutcome` + crowned arm, (4) states whether the
      prediction HELD. Report the WHOLE slate, win or lose; a null all-Fragile
      result is a PASS-worthy honest finding.
      _acceptance: report exists with the up-front prediction + the 13-row table +
      the outcome + a held/refuted verdict; `verify_anchors.sh` unaffected (the
      advisor path wrote no anchored body)._

- [ ] **T6 — ui-designer: leaderboard 13-row render-snapshot (OQ-6, pixel proof).**
      Extend `ui::fixtures::fake_bakeoff_report_mirror()` (`crates/ui/src/fixtures.rs:1256`)
      to **13 `LeaderRow`s** (4 singles + 8 ensembles + buy-and-hold), keeping a
      crowned `★ best` row + ≥1 Fragile ensemble row + the correct `ranked`/`crowned`
      indices. Add a guard in `crates/ui/tests/leaderboard_populated_render.rs`
      asserting the 13-row table PAINTS (crowned ACCENT teal + always-negative
      Max-DD clay across rows + a healthy foreground-text floor), with the existing
      `Empty` negative control still painting no table. Verify at the RENDERED-PIXEL
      layer (read the PNG); a passing model state is NOT proof.
      _acceptance: the populated 13-row guard passes on macOS (writes
      `/tmp/leaderboard_populated_render.png`), the empty negative control still
      paints no table, and the existing render guards stay green._

- [ ] **T7 — ui-designer: ensemble-rule honest description + arm-count note.**
      (a) Add one crowned-combination forward-plan fixture (e.g. `tr_mr_macd_rsi`
      or `k2of4`) and a guard (extend `crates/ui/tests/forward_f6_ensemble_named_render.rs`
      or a sibling) asserting its RULES band paints the honest named-member
      brace-list ("Holds while at least k of {…} agree…") via the existing
      `PlanRuleShape::Ensemble` + `member_id_to_rule_shape` path (no new render
      code) — strict-exceedance vs the single-strategy SMA negative control.
      (b) Surface the field arm-count in the leaderboard header context copy
      (OQ-2) so a longer bake-off is self-explanatory; verify it paints.
      _acceptance: the crowned-combination plan's named-member rule paints (pixel
      guard, with negative control); the arm-count note renders; no `String`
      crosses the agent→ui seam (`cargo tree -p ui` unchanged)._

## Notes

- **Sequencing:** T0 → (T1, T2, T3 land together — wiring + its day-1 gate) → T4
  (build/validate/re-anchor) → T5 (tester) ‖ T6, T7 (ui-designer, parallel with
  the tester since they touch fixtures + render harnesses, not the engine).
- **Out of scope (do NOT build):** weighted/inverse-vol/regime blends (v0.2 of
  this feature, OQ-4); a combination-SEARCH engine (guarded follow-on, R5); new
  signal TYPES + short-selling (sibling backlog items). See
  [`../backlog.md`](../backlog.md).
- **If `verify_anchors.sh` ever returns non-119:** STOP. The new arms must touch
  no anchored body — a non-119 means a wiring mistake (e.g. an arm accidentally
  ran `write_report=true`), not a band change. Route back to the architect.
