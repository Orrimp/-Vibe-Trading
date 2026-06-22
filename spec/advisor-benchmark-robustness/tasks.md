---
slug: advisor-benchmark-robustness
status: in-progress
owner: developer
updated: 2026-06-22
---

# Tasks — B1 benchmark exemption from the `AllFragile` outcome

Normative design: **ADR-0066**
(`spec/architecture/adr/0066-benchmark-exempt-from-allfragile.md`). The classifier
(`classify_verdict` / `compute_robustness_flag` / `verdict_bands`) is **byte-frozen** —
do NOT touch it. All edits below are in `crates/backtest/src/bakeoff/rank.rs` (engine)
and `crates/ui/` (copy/render). Run `scripts/verify_anchors.sh` → **119/119** before T1
and after T-final; any non-119 is STOP-and-route-back.

Developer (T1–T6) and ui-designer (U1–U4) run **in parallel** — they share no files
(rank.rs vs ui strings/screens).

## Developer — engine seam + the day-1 gate (`crates/backtest`)

- [ ] **T0 — Anchor baseline.** Run `scripts/verify_anchors.sh`; confirm **119/119**
  before touching anything. — _acceptance: "ANCHORS PASS (119/119)" captured._

- [ ] **T1 — D1: `all_fragile` → `all_active_fragile` (rank.rs:60-62).** Change the
  all-fragile determination to range over non-benchmark arms only
  (`candidates.iter().filter(|c| !c.is_benchmark).all(|c| c.robustness ==
  Some(RobustnessFlag::Fragile))`). Keep the variable name honest (`all_active_fragile`).
  The outcome branch (rank.rs:64-70) `AllFragile` arm now reads the active-only flag. —
  _acceptance: `AllFragile` fires iff all ACTIVE arms are Fragile and no benchmark is
  crownable; the existing `t63`/`t64`/`t66`/`determinism`/`empty` tests still pass._

- [ ] **T2 — D2: `is_eligible` benchmark-always-eligible (rank.rs:124-126).** Return
  `c.is_benchmark || c.robustness != Some(RobustnessFlag::Fragile)`. This is the
  **required second edit** — without it the crown lands on a Fragile active arm and the
  outcome falls through to `ActiveWins` on a Fragile crown (ADR-0066 § D2). The
  active-arm anti-overfit lock is unchanged (a Fragile *active* arm stays ineligible). —
  _acceptance: with all active arms Fragile + benchmark top-Sharpe, the benchmark sorts
  to `order[0]` and `crowned.is_benchmark == true`._

- [ ] **T3 — D5: amend `rank.rs::t65_all_fragile` (rank.rs:297-325).** Its current 2-arm
  fixture (`v0.sma` Fragile @ Sharpe 2.0 + `v0.buyhold` Fragile @ Sharpe 1.0) has the
  active arm out-Sharping the benchmark → after T1+T2 the **active arm is crowned** and
  the field is genuinely all-fragile ⇒ the corrected expectation **stays `AllFragile`**
  (no crownable benchmark by Sharpe). Update the test's doc comment to state WHY it is
  still `AllFragile` under the new semantics (active arm out-Sharpes the benchmark), and
  keep the `AllCandidatesFragile` reason assertion. — _acceptance: `t65_all_fragile`
  passes with an updated comment that reads as the corrected semantics, not a silent
  expectation flip._

- [ ] **T4 — D5: the day-1 BenchmarkWins-reachability e2e (FAIL-before / PASS-after).**
  Add the reachability regression the ADR-0063 § D7 / R4.4 promised but never
  implemented. Recommended home: extend `crates/backtest/tests/robustness_bootstrap_bites.rs`
  (its doc comment line 9 already *declares* this intent — make a test body assert it),
  OR add a `rank`-level case. Assert, on a field where **all ACTIVE arms are Fragile**
  AND the **benchmark is the top-Sharpe arm** (e.g. benchmark Sharpe 1.0 Fragile + one
  active arm Sharpe 0.5 Fragile): `outcome == BenchmarkWins` AND
  `reasons.contains(BenchmarkUndefeated)` AND `candidates[crowned].is_benchmark == true`.
  Add the **residual dual**: a field where all arms are Fragile AND an active arm
  out-Sharpes the benchmark → `outcome == AllFragile` (the row-3 case; this is what `t65`
  now covers, so the dual can cross-reference it). This is a pure-`rank_candidates`
  assertion — construct `CandidateResult`s with explicit flags (the `make_candidate` /
  `t65` pattern), no corpus, no bootstrap. — _acceptance: the BenchmarkWins case FAILS on
  pre-T1/T2 code (returns `AllFragile`) and PASSES after; the residual `AllFragile` case
  passes both ways._

- [ ] **T5 — Determinism + freeze guard.** Confirm (a) `rank_candidates` is still pure /
  total — no f64 arithmetic introduced (only `total_cmp` / `Decimal::cmp`); the existing
  `determinism_same_input_same_output` test still passes; (b) you did **not** edit
  `robustness.rs` (`classify_verdict` / `verdict_bands`) or `bootstrap.rs` — `git diff
  --stat` shows changes confined to `rank.rs` + the test file(s). — _acceptance: `git
  diff --name-only` lists only `crates/backtest/src/bakeoff/rank.rs` +
  `crates/backtest/tests/robustness_bootstrap_bites.rs` (and this tasks/feature doc); no
  classifier file touched._

- [ ] **T6 — Anchor + build close-out.** `scripts/verify_anchors.sh` → **119/119**;
  `cargo build -p backtest`; `cargo test -p backtest --lib rank` + the bites test green;
  `cargo clippy -p backtest -- -D warnings`. — _acceptance: 119/119 + all rank/bites
  tests green + clippy clean._

## ui-designer — honest copy + render proof (`crates/ui`)

> All copy frames the result as **"benchmark-is-not-a-candidate"** and the negative
> result honestly — never "we relaxed the gate," never asserting alpha. The benchmark
> is the **baseline**, not a co-fragile loser. The absolute Fragile flag stays visible
> on every row (including the benchmark's, per ADR-0066 § D3 — the flag is still
> displayed; it is just no longer crown-disqualifying).

- [ ] **U1 — `BenchmarkWins` honest recommendation copy.** The headline/recommendation
  for `RecommendationOutcome::BenchmarkWins` reads "**nothing active cleared the
  robustness bar — holding is the least-bad on this window**" (or equivalent), with the
  not-advice + simulated-budget framing intact. The benchmark row renders as the
  **baseline** (visually distinct, the "vs just holding" framing — `Recommendation.benchmark_kpis`
  already carries this). — _acceptance: the `BenchmarkWins` recommendation block names
  holding as the least-bad, shows the benchmark as the baseline, and never implies an
  active strategy was robust._

- [ ] **U2 — `AllFragile` honest copy (the residual).** When the outcome is genuinely
  `AllFragile` (row-3: nothing active robust AND holding not even best-by-Sharpe), the
  copy reads "**nothing active cleared the robustness bar**" without nihilism — it is a
  ranking + least-bad surface, not "everything is hopeless." — _acceptance: the
  `AllFragile` copy is distinguishable from `BenchmarkWins` and does not read as "do
  nothing."_

- [ ] **U3 — Unanimous-vote "sat in cash" state.** The `v0.8.vote.unanimous` 0-trades
  arm renders as "**sat in cash — consensus never reached**" (0 trades) rather than a
  silent Sharpe-0 Fragile loser indistinguishable from a strategy that traded and lost.
  (Honest-but-mis-presented per the analyst § 1.4.) — _acceptance: a 0-trade
  unanimous-vote arm shows the "sat in cash / consensus never reached" copy, not a bare
  Sharpe-0 row._

- [ ] **U4 — Render-proof PNG (CLAUDE.md iced pixel rule).** A render-layer screenshot
  test (the `leaderboard_populated_render.rs` / `Emulator::screenshot` pattern,
  macOS-canonical per ADR-0057 § D2) covering the **populated** `BenchmarkWins` state:
  the honest recommendation copy painted, the benchmark-as-baseline row, the benchmark's
  Fragile badge still visible (informational), and the unanimous-vote "sat in cash" row
  — **plus a negative control** (e.g. an `ActiveWins` or empty state) so a passing proxy
  is not mistaken for proof. Read the PNG; confirm it draws. — _acceptance: the
  `BenchmarkWins` populated render PNG shows the honest copy + baseline framing + the
  "sat in cash" row, with a negative control; `cargo tree -p ui` unchanged._

## Notes

- **Why two `rank.rs` edits, not one** (ADR-0066 § D2): edit-1-alone (`all_active_fragile`
  without benchmark crown-eligibility) is a *worse* bug — a Fragile benchmark still
  partitions ineligible, the crown lands on a Fragile active arm, and the outcome is
  `ActiveWins` on a Fragile crown. Both edits are mandatory.
- **The `t65` amendment is expected** (ADR-0066 § D5): it is a deliberate, ADR-logged
  unit-test behaviour *clarification* (the fixture still asserts `AllFragile`, but for the
  corrected reason — active arm out-Sharpes the benchmark), plus a **new** sibling case
  for the `BenchmarkWins` path. Not a regression.
- **Anchor discipline:** B1 is anchor-safe by construction (ADR-0066 § D4), but the gate
  is mechanical — run `scripts/verify_anchors.sh` first and last. Do NOT touch any
  `spec/*/reports/` file, `anchors.toml`, or `data/*/REVISION.toml`.
- **UI ⊥ engine:** the ui-designer tasks consume the existing `BakeoffReport` /
  `RecommendationOutcome` mirror — no new seam, no `strategy`/`llm` type crosses the
  view line, `cargo tree -p ui` unchanged.
