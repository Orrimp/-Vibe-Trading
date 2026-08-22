# Story 1.25: harness-fill-correctness-relock

Status: in-progress

<!-- 2026-08-19: the CODE half of AC1-AC3 is landed and pushed; AC4 (regeneration) and
     AC5 (band re-examination) are NOT started and are gated on two operator rulings
     plus a compute window. Deliberately NOT `ready-for-review`: reviewing now would
     assess a story whose defining deliverable — the re-lock — has not run. -->

<!-- Created 2026-07-31 by the 1-14 code-review decision (operator: new critical story + program).
     CRITICAL priority. Runs as ONE re-lock program with 1-24-pwsd-fidelity-relock (same
     namespace ceremony, one re-run budget, one errata). Disclosure of record: bug-log #67. -->

## Story

As the operator of the Honest Advisor,
I want the research-harness lanes' fill arithmetic corrected (orders priced at their own symbol's bar, never the trigger bar's) and the affected anchored evidence formally regenerated under a re-lock,
so that the C2/C3 research verdicts rest on real execution arithmetic — with the migration honest, the old rows frozen as history, and the verdict re-derivation loud.

## Acceptance Criteria

1. **Fill-symbol correctness**: a batch stepped through the harness lanes prices each order at ITS symbol's bar for that timestamp (or defers the order to that symbol's bar); `PaperEngine::step` either enforces `order.symbol == bar.symbol` (typed reject) or the harness callers route per-symbol — architect chooses the seam with the LIVE cockpit paths regression-proven byte-identical (the Lab/advisor flows step single-symbol batches; prove no behavior change there via the existing suites + a dedicated same-bytes test).
2. **Both lanes**: `scenarios/montecarlo.rs::run_path` AND `threshold_sweep.rs::run_cell` fixed (run_cell additionally gains the Bug-B solvency guard it never received); the FROZEN gate files stay byte-untouched (identity test discharged).
3. **The riders land in the same regeneration**: √8760 annualization (or a formal ratification of √8575 with the doc corrected to match), hashed-body verdict vocabulary aligned to the frozen 5-signal rule, sentinel-zero pooling policy, negative-final Calmar guard, slippage-aware solvency pre-flight, FILL_SEED domain separation, real portfolio-exposure-cap enforcement decision (enforce or delete the decorative limit) — each either fixed or explicitly ratified-as-is in the story record.
4. **Formal re-lock per the ADR-0038/0045 §D6 family**: affected reports regenerated under a NEW namespace; old rows byte-frozen; an errata/decision note records per-scenario old-vs-new headline numbers and the RE-DERIVED verdicts. The C2 FRAGILE and the sweep FAMILY-UNIFORM-FRAGILE conclusions are re-stated from clean arithmetic — whichever way they land. Any flip that would touch the era-qualified thesis's supporting narrative escalates to the operator BEFORE publication (AD-19 spirit).
5. **Band re-examination (product-review finding 2, 2026-08-04):** `classify_verdict` /
   `verdict_bands` thresholds were calibrated against the very surfaces this story
   regenerates. After the re-derivation, the story must state explicitly — with numbers —
   whether the frozen bands would classify the CLEAN surfaces the same way they classified
   the contaminated ones. The gate stays byte-frozen either way (AD-1); the deliverable is
   the ANSWER, recorded in the errata, plus an operator escalation if the clean numbers sit
   near a band edge. A frozen gate whose calibration is never re-examined is an assumption,
   not a guarantee.
6. Standing floor: anchors green (old + new rows); spec-lint PASS; advisor-gate independence re-proven (bakeoff/bootstrap.rs resamples returns — assert its inputs/outputs unchanged by this story).

## Tasks / Subtasks

- [x] Architect: fill-correctness seam decision + re-lock plan — **DONE 2026-08-16**, `docs/dev-notes/1-25-architect-seam-and-relock-plan-2026-08-16.md`. **Seam RATIFIED by the operator: ENGINE GUARD** (`PaperEngine::step` typed-rejects `order.symbol != bar.symbol`) — the signature already carries that precondition, it is provably a no-op on every live/agent caller (all pass single-symbol batches: `runtime.rs:2280/:2310/:2385`), and it converts a convention into an invariant a future harness lane cannot silently break. Silent *deferral* inside the engine was explicitly REJECTED (it reorders execution — a behaviour change disguised as a fix, unprovable byte-identical). Inventory: **34 anchors (#86-#119) = 29% of the corpus**. Compute budget deliberately NOT estimated (no runtime is recorded anywhere in the corpus); one measurement run authorised instead.
- [~] Dev: fixes per AC1-AC3 + the same-bytes live-path proof — **CODE HALF DONE 2026-08-17/19.**
  - [x] **#67** engine guard (`MatchError::SymbolMismatch`, checked before pricing so a mismatched
        batch cannot partially fill) + per-symbol fill routing in `run_path` (`last_bar_by_symbol`,
        the fill-side twin of the `mark_prices` lookup the SIZING path already used — #67 was a
        divergence between sizing and filling inside ONE block). AC1's same-bytes live-path proof:
        `paper_step_symbol_guard_is_noop_on_single_symbol_batches`, pinned to LITERAL arithmetic so
        it cannot degenerate into a tautology. Measured: the bug fabricated ~1% of equity
        (`buy@1000/mark@1100 → 101 000` vs a correct flat `100 000`, both to the digit).
        Commit `11acd12`.
  - [x] **#75** score/accrual channel separation — `funding_override` is now accrual-ONLY;
        `run_path` no longer overwrites the caller's score map. Only `MnBasisSpread` was corrupted
        (the other two MN arms pass the same map on both channels), which is why `mn-basis` differed
        from `mn-funding` in NO number while the `mn-basisperp` control differed in every one.
        Commit `936134c`.
  - [x] **#71** side-aware exposure cap on RESULTING exposure. Second defect found while fixing it,
        not in the original write-up: **the old cap was evadable by splitting** — at 0.39 of equity a
        top-up ending at 0.44 passed on its own 0.05 notional. AC3's binding test added (7 cases).
        Commit `7884f52`.
  - [x] **#76** residual direction — `rank(funding) − rank(basis)`. AC3's literal-value direction
        gate landed by converting the pre-existing `characterization_bug76_…` test (which pinned the
        BUG and carried an instruction to invert it on fix) into
        `direction_gate_bug76_…_longs_the_lowest_basis_name`, plus the assertion that matters: the
        residual arm and the plain `BasisReversal` arm must now AGREE on the sign of the basis axis.
        Commit `7884f52`.
  - [x] **#72 / #73** — code halves already fixed 2026-08-04 (`fix(carry)`); verified still in place.
        Regeneration is what remains, and that is AC4.
  - [x] **#68 + #69 — WIRED AND BINDING 2026-08-23.** `run_path` builds a signed target vector at each
        rebalance boundary and calls `size_portfolio_target`; a breach skips the whole rebalance,
        increments `PathRunResult.portfolio_breaches`, and logs (ADR-0089 D1/D2). The sizer was first
        extended to signed weights with a GROSS cap (D7), which is what made the ruling implementable.
        Three RED-proven gates in `crates/backtest/tests/portfolio_controls_bind.rs`: neutering the
        cap, the band, or #94's delta sizing turns exactly one of them red.
        **Four findings recorded with the fix, none of them cosmetic:**
        1. **bug-log #94** — the sizer sized a resize order to the whole TARGET, not the delta. First
           fixture through the resize path lost **74 % of equity** (`min_cash_seen` 43.8 / 100 000).
           It also DISABLED the drift band: an overshooting order leaves the leg outside the band every
           bar, so the band could never hold anything. Fixing it first is what let #68's gate be
           RED-proven at all.
        2. **The gate is the REBALANCE BOUNDARY, not `!signals.is_empty()`.** Signals are a delta, so a
           full book emits none; a signal-gated rebalance would have left the band nearly as inert as
           #68 found it. `MomentumStrategy::last_rebalance_ts()` added for this.
        3. **ADR-0089's "turnover falls" is WRONG and is now corrected in the ADR.** The old code could
           not resize a held leg at all (`Buy if current_qty <= 0`), so the band bounds NEW behaviour
           rather than suppressing old. Net direction is an empirical question for 1-26.
        4. **bug-log #95** — `portfolio_exposure_cap` is declared at **9** sites and read at **1**
           workspace-wide. `run_path` is now wired and it is the lane behind **all 34** inventory
           anchors (#86-#119 are θ-surfaces from `param_robustness_sweep`); **eight other lanes still
           declare a cap they cannot enforce**. Needs a ruling. This also corrects the architect note:
           `scenarios::threshold_sweep::run_cell` is the candle-gated TCN τ/ε sweep and produces NONE
           of the inventory anchors, so D1 was dischargeable on `run_path` alone.
  - [x] **#69** — *(superseded by the entry above)* **RULED 2026-08-19: WIRE IT.** The operator chose to make the limit real rather than
        strike the claim: call `size_portfolio_target` on the harness lanes so the declared `0.50`
        actually binds, with a binding test per AC3. Changes results — absorbed by the re-lock, which
        regenerates these surfaces anyway. The hashed bodies' `exposure_cap=0.50` becomes true rather
        than aspirational. *(prior state, for the record:)* enforce-or-delete — Source-confirmed: `size_portfolio_target`
        is the sole enforcer and has **zero production callers** (definition + its own tests + 3 sites in
        `agent/tests/v1_rebalance_reject.rs`). Sweep scenarios set `Some(0.50)`, it is printed into hashed
        bodies, nothing reads it. Annotated at the declaration; commit `ae62de8`.
  - [ ] **#68 + #69 — UNITS RULED 2026-08-22: `exposure_cap` MEANS GROSS (Σ |notional|).** ADR-0089 D7.
        The anchored MN surfaces **did** breach their declared limit (6 legs × 0.10 = **0.60 gross vs a
        hashed 0.50**), so bug-log #69's reading is now official rather than a candidate. And
        `size_portfolio_target` **cannot implement the ruling as written** — it caps
        `total_long_notional`, the long-only measure that was explicitly rejected — so it must be
        extended to signed weights with a gross cap, or replaced. 1-26's errata owes the per-scenario
        non-compliance record.
  - [ ] *(prior)* **#68 + #69 are ONE defect — RE-RULED 2026-08-19: WIRE `size_portfolio_target` FULLY.**
        Found while implementing the earlier rulings: `risk::size_portfolio_target` implements **both**
        controls — the portfolio cap at `portfolio.rs:189` (`if let Some(portfolio_cap) =
        limits.portfolio_exposure_cap`) **and** the drift hold band at `:110`
        (`relative_drift > drift_threshold`). Neither parameter is inert by design; they are inert
        because **the single function that consumes both has no production caller**. The hold band is
        not missing — it is written, unit-tested, and never invoked.
        **This SUPERSEDES the "drop the drift axis" ruling taken earlier the same day.** That ruling
        rested on "implementing a hold band would be NEW strategy behaviour in a feature-complete
        project" — a premise that is false: the behaviour already exists. The operator re-ruled to
        wire the function fully and accept both controls, with binding tests for each per AC3. The
        drift axis therefore becomes **real**, not removed, and the grid's three advertised axes all
        become genuine.
        *(superseded ruling, kept for the record:)* DROP THE AXIS — Remove `drift_rebalance_threshold` from the
        Tier-1 grid and correct every surface that presents drift as an explored dimension. No
        information is lost — there was none, the axis did nothing. Chosen over implementing a hold
        band because that would be NEW strategy behaviour in a project declared feature-complete, and
        the grid would then need cells that actually vary drift to be worth exploring (54 of 58 sit at
        0.10). *(prior state, for the record:)* implement-or-drop — `drift_rebalance_threshold` is swept,
        **range-validated**, copied to `momentum.rs:194` — and read nowhere. One of the three advertised
        Tier-1 grid axes has no consumer. Annotated at the declaration; commit `ae62de8`.
- [ ] Re-run + re-lock + errata + verdict re-derivation (AC4) — **RULED 2026-08-19: SPLIT OUT.**
  The regeneration moves to its own story so the eight code fixes can be reviewed while a multi-day
  compute window is scheduled. The split line falls BEFORE regeneration (plan §6) so no partial corpus
  is ever produced. 1-25 therefore closes on the CODE deliverable; the re-lock story owns AC4 + AC5.
  **Ordering constraint:** the re-lock cannot start until #69 (wire) and #68 (drop) land — both change
  results, so regenerating first would produce a corpus obsolete on arrival.
  Budget carried over, MEASURED not estimated:
  Budget MEASURED 2026-08-16 rather than estimated: one θ-surface = **1087 s (18.1 min)**, build 8.65 s
  (one-time), ~12.4× parallelism already in use. **34 surfaces ⇒ ≈10.3 h sequential, and that is a
  FLOOR** — the measured lane is long-only momentum (the cheap end); the MN family runs ~2× the order
  traffic and the basis-reversal family hits 60k–318k trades/200 paths. Plan a multi-day window;
  15–20 h is the realistic figure. `--out-dir` is MANDATORY (its default points INTO the anchored
  corpus — nearly written there on 2026-08-16, caught mid-compute).
- [ ] Review: old rows intact, new rows complete, verdict-delta table honest, advisor-gate independence proof (AC5).

## Dev Notes

- Origin: 1-14 review Critical (Blind Hunter; orchestrator-verified at paper.rs:118-136 + montecarlo.rs:274+ + run_cell) — full evidence in `1-14-strategy-robustness-harness.md` § Review Findings; disclosure bug-log #67.
- **Advisor gate PROVEN unaffected** (verified 2026-07-31): `bakeoff/bootstrap.rs` resamples log-returns from candidate equity curves — no fill re-execution. Crowns/verdicts/ship-passive rest on it, not on the harness lanes.
- Blast radius: research-evidence class only (C2 monte_carlo lane, C3 threshold-sweep lanes and their anchored namespaces).
- **Inventory extension (1-15 review, 2026-07-31):** anchor #86 (`v1-momentum-theta-surface-…`, SHA `0dd989d9…`) confirmed contaminated via run_path→PaperEngine on all 6 active cells (BUYHOLD row clean — pure mark-to-market; this bin never calls run_cell, so the contamination has TWO routes: run_path for C2/C3-sweep, run_cell for the older threshold lanes). NEW riders for AC3: (i) cross-frequency Sharpe comparison — θ-cell curves per MERGED bar vs BUYHOLD hourly, both annualized hourly (~√10 deflation on the cell axis of the same table); (ii) BUYHOLD frictionless-vs-fee'd-cells asymmetry (behavioral half); (iii) per-cell trade counts into the θ-surface table (the hashed conclusion asserts a turnover mechanism its body can't evidence); (iv) the `{:.6}` format-placeholder prose + hard-coded `held_constant` line (body hygiene at regeneration). FP-C3.3's full-sweep two-run identity + the p50-vs-real-path interpretive field ride the re-verification suite. **Extension (1-18 review, 2026-08-04) — the heaviest yet:** anchors **#92-#99** (8 horizon θ-surfaces) join the inventory via the same run_path chain. THREE new items, two of them CRITICAL and one a claim-qualification already issued:
  - **bug-log #73 (CRITICAL, code FIXED 2026-08-04) — found by questioning the review's own framing:** funding accrued once per SYMBOL-BAR, not once per settlement (the accrual block sat inside a loop over multi-symbol merged bars with no dedup). Measured: one position, universe 2/3/4 → −5/−7/−9 units. The anchored universe is 10 symbols, so **every carry lane over-accrued by ~an order of magnitude — including the 1h anchors #88/#89, which the 1-18 review had called the correct reference.** They join the inventory. **This withdraws the "~1/4 and ~1/24" magnitudes published in #72 and in the errata §1**: both sides of those ratios were contaminated, so the corrected numbers can only come from THIS re-run. Code fix + gates are in; the anchored bodies still need regeneration here.
  - **bug-log #72 (CRITICAL, cadence half code-FIXED 2026-08-04):** the bootstrap's cosmetic 1h timestamp ladder makes funding settlement fire every 8 BARS, so carry-4h harvested ~1/4 and carry-daily ~1/24 of the true funding (per-path g=0: 15,490 → 3,039 → 267). Anchors #96-#99 measured a throttled mechanism, and their hashed bodies assert a "native settlement cadence" they never ran. **The carry × coarse-horizon leg of the thesis closure is UNRESOLVED, not direction-preserved** — errata already issued at `evidence/v1/horizon-retest-robustness/reports/ERRATA-2026-08-04.md` (operator-ratified escalation under AC4, 2026-08-04). The fix must make generated paths carry their true cadence (or every time-derived rule take it explicitly), then RE-DERIVE the carry-coarse verdicts.
  - **bug-log #71 (CRITICAL):** `Order::new`'s exposure cap is side-blind and rejects position-CLOSING sells silently (no else arm, no warn, no counter) — the strategy's held_symbols diverges from the engine's book and every later decision runs off a false flat. Blast radius is larger than #67's: it changes WHICH ORDERS EXIST. AC3 gains: the cap must be evaluated against RESULTING exposure with the side considered, plus a binding test.
  - **H4 (anchor-impacting):** daily carry samples only the 00:00-UTC settlement — the 08:00 and 16:00 rates never enter the score ring (bucket opens are midnight-aligned), so the daily carry score is a biased subsample, not a daily mean; at 4h each rate is forward-filled into two buckets, halving the documented "L settlements" memory. Grid docs say settlements; the code counts bars.
  - Body-hygiene riders for the regeneration: the `rebalance_minutes=60` row printed unqualified on 4h/daily surfaces; the ragged `| horizon |` column padding (fixing the alignment would break four anchors — leave until re-lock); the anchors.toml "every cell has p5 < 0" justification, which is false for #97 g4 and #99 g4 (verdicts still correct via weakest-link).
**⚠ NEW CONSUMPTION SURFACE for the √8575 rider (2-18 review, 2026-08-12) — a re-sync obligation this story does not currently track.** The √8575-vs-√8760 item has so far been scoped to anchored report bodies. It also reaches the **cockpit's Baseline screen**, which is the operator-facing comparator for the whole era-qualified "active ≤ passive" claim. Chain, orchestrator-verified: `crates/backtest/examples/passive_baseline_equity.rs` → `compute_sharpe_hourly` (`stats/mod.rs`, `SQRT_HPY = 92.601_295_098_46`, whose square is **8574.9999**, not 8760 — the function's own doc says `sqrt(24*365)` and the characterization repeats it; **both statements are false**) → `passive-baseline-characterization.md` §7.1 → the hardcoded const in `crates/ui/src/baseline/loader.rs` → the screen. Correcting the constant moves Sharpe **1.8417 → 1.8615** (2023) and **0.8925 → 0.9021** (2024), Sortino likewise (~1.07% understatement throughout; Calmar sits on a *different* basis again, `years = (n−1)/8760`). **AC3 must add: when the annualization is ratified or fixed, re-sync the cockpit const in the same pass.** Nothing will trip otherwise — the test that the design named "the re-sync trigger" asserts the const against literals in its own file and never reads the characterization (2-18 finding F4). This panel is **anchor-impacting: NO** (the BH CSVs are non-anchored); it is a downstream-consumer obligation, not a new contaminated body. **Do NOT route the Baseline surface into the #67/#69/#71/#72/#73 inventory** — orchestrator-verified at the producing code that `build_buyhold_curve` fixes quantities at bar 0 and computes `Σ qty × close`, never constructing an `Order`, never entering `PaperEngine::step`, applying no fee and no funding. It is genuinely clean.

**⚠ THIS STORY NOW BLOCKS STORY 1-21's CLOSURE.** The 1-21 code review returned **FAIL** (2026-08-11, burn-down 10/14) — the first of the burn-down that could not be flipped to `done`. Its triad is deliberately unflipped because closing it would assert a delivered result its evidence does not support. 1-21 closes only after this re-lock fixes #75 + #76 and re-runs the MN arms; the re-derived verdicts, not the 1-21 review, decide what the market-neutral basis spread actually shows.

**Extension (1-21 review, 2026-08-11) — anchors #108-#119 (12 MN θ-surfaces) join the inventory, carrying a NEW CRITICAL of their own.** Same `param_robustness_sweep → run_path → PaperEngine::step` chain. They inherit **#67** (all cells — and at ~2× the order traffic of any prior lane: 6 legs plus buy-to-covers; BUYHOLD rows clean), **#69** (all 12 bodies assert `exposure_cap=0.50` while the 6-leg book runs ~60% gross — the declared limit is violated by construction on every MN path, an escalation over the long-only lanes' 30%), **#71** (the FIRST lane with deliberate Sell traffic, plus a **new mirror-image instance**: a rising short's buy-to-cover is sized at full short notional, so it can exceed the per-symbol cap and be silently rejected — leaving the liquidation path, which bypasses `Order::new` entirely, as the only exit; a mechanistic candidate for the 97.8-100% p95 MaxDD and the 86-328 liquidation counts), the **√8575** rider (1h lane, ~1.06%), and **#73** — **VERIFIED entered, not assumed**: `funding_override` is `Some` for all three MN arms, so with a 10-symbol universe the funding over-accrued by ~10× *on the very leg that is this feature's binding cost* (R-MN.3) and its k3 kill-criterion. **VERIFIED CLEAR of #72** (`Horizon::OneHour` only ⇒ `bar_span_hours = 1`).
  - **NEW CRITICAL — bug-log #76 (anchor-impacting, #116-#119):** the basis⊥funding **residual** arm ranks the basis axis **inverted** vs its own spec — rank 1 = lowest basis, `top_k_long` takes the **highest** residual, so the arm longs the **highest** basis while the doc says "Long = highest residual (low-basis relative to funding)". Verified at source. **Together with #75 this is the load-bearing consequence for the whole family: NO anchored MN surface tested the basis in its documented direction** (#108-#111 never loaded it, #112-#115 are funding by design, #116-#119 loaded it backwards). The recorded reading — "residual carries no orthogonal alpha, domain closed" — cannot be taken off these surfaces; a negative median from an inverted arm is *consistent with* an edge in the intended direction. That is NOT a claim an edge exists (costs, the #73 10× over-accrual, the liquidation regime and noise all live in the same number) — the honest status is **unknown pending a correctly-signed re-run**. The re-lock owes a **literal-value direction gate** for the residual arm, in the shape of the long-only arm's two sign guards; every existing residual test asserts difference, which the inverse satisfies just as well.
  - **NEW CRITICAL — bug-log #75 (anchor-impacting, specific to this family):** `run_path` overwrites the pre-injected SCORE map with the ACCRUAL map, so **#108-#111 (`mn-basis`) are duplicate funding runs — the market-neutral basis arm never ran.** Confirmed with a control: `mn-basisperp` (basis on a different field) differs from `mn-funding` in every number, while `mn-basis` differs in none. The k2 kill-criterion, the R-MN.6 three-arm headline, and the "domain CLOSED with finality" claim all rest on the artifact. The re-lock must fix the field collision (separate score/accrual channels), re-run the arm, and re-derive k2 + the closure language in the **era-qualified** form.
  - Also owed at this family's regeneration: the pre-registered **dollar-neutral ≈0 null is absent from all 12 bodies** — every one prints the BUYHOLD control the frozen-§0 change was meant to retire, so the verdicts are stated against a control that is not in the artifact; **no `trades` and no funding-cost column**, so R-MN.3's "net-of-cost edge at each fee level" is not derivable from any body; **k1's "0 bps gross" was never run** (all six fee00 surfaces carry `slippage_bps = 2`); the **`git_commit:` frontmatter on all 12 stamps `18334c9`**, a commit that does not contain the harness that produced them (frontmatter is hashed → anchor-impacting); and the **supersession item repeats here** — all 12 bodies close with "…removes directional beta but not fee-bleed from short-leg turnover", the same falsified fee-bleed mechanism as #100-#107, asserted on six ZERO-fee surfaces.
**Extension (1-20 review, 2026-08-11) — anchors #100-#107 (8 basis-reversal θ-surfaces) join the inventory.** Same `param_robustness_sweep → run_one_path_with_config → run_path → PaperEngine::step` chain; they inherit **#67** (all 6 active cells; BUYHOLD row clean), the **√8575-vs-√8760** rider (1h lane), **#69** (all 8 hashed bodies assert `exposure_cap=0.50`), and **#71** — which bites hardest here, this being the heaviest Sell-traffic lane of any (60k-318k trades/200 paths). **VERIFIED CLEAR of #72/#73** (not assumed): `funding_override_non_mn` is gated on `is_carry`, so `funding_map_for_accrual` is `None` for the basis lane and the entire 2026-08-04 accrual block is never entered — the basis map reaches the *score* only, via the `montecarlo.rs` preserve-branch. Three new decision items, two anchor-impacting:
  - **Basis publication-lag ruling (ADR-0086 correction, 2026-08-11) — ANCHOR-IMPACTING, needs a decision here.** The as-of join keys `basis_close` (a kline **close** value) on that kline's **`open_time`**, so the value read at bar `t` is realized at `t+1h`; the grounded lag is `3_600_000`, not the declared `0`. It is not a look-ahead under the harness's signal-at-close/fill-at-close convention, but the margin is **zero, not the hour the docs claimed**. ADR-0086's table + the code docs are corrected (claims only); **the join was deliberately NOT changed** because declaring the real lag shifts every basis score one bar and re-prices #100-#107 **plus the MN surfaces that consume this join — which, per bug-log #75, is ONLY #116-#119 (MN-BasisPerp).** ⚠ **CORRECTION 2026-08-11 (1-21 review):** an earlier version of this bullet said #108-#111 (MN-Basis) consume the basis join. **They do not** — the #75 overwrite means those four surfaces never saw the basis at all; they are duplicate funding runs. #112-#115 (MN-Funding) are settlement-keyed and were always clear. Once #75 is fixed and the mn-basis arm actually runs, #108-#111 WILL consume the join and re-enter this bullet's scope. 1-25 rules: declare `0` with the corrected written justification, or declare `3_600_000` and re-lock. Nothing downstream may keep asserting the fictitious margin either way.
  - **The 1-14 solvency-cap removal rests on a precondition THIS story falsified — re-verify before the re-lock.** `montecarlo.rs`'s surviving comment justifies deleting the `min(target_notional, cash)` cap by asserting "with **any positive taker fee** a cash-capped buy always failed the pre-flight … **all anchored lanes and the harness drivers use `taker_fee_bps = 4`**." Both halves are now false: the fee is a swept axis, and anchors **#100/#101 are locked at fee = 0**, where the pre-flight degenerates to `cash < notional` — precisely the case the removed cap covered. The skip-vs-skip byte-identity proof therefore does not hold on the zero-fee lane, which skips buys the cap would have downsized. Nothing is internally inconsistent (the anchors were locked *after* the removal); the defects of record are a **false surviving invariant** plus **unverified behaviour on a lane that did not exist when the proof was written**. Fix the comment regardless; restoring the cap is anchor-impacting and is 1-25's call.
  - **A hardcoded causal conclusion is frozen into all 8 bodies, including the two where it is impossible.** The fee-bleed sentence ("the fee-bleed from reversal-arm turnover consumes the gross −0.10 IC edge at this fee level") is emitted unconditionally by `sweep_harness.rs` — so the **0 bps** surfaces assert a fee-bleed mechanism on a lane with no fee to bleed, contradicted by those anchors' own comments. **Worse, orchestrator measurement across all 8 bodies refutes the claim at every fee level, not just at zero**: the best cell's p50 Sharpe moves only 0.048645 → 0.045192 (2023) and 0.050617 → 0.047626 (2024) across the whole 0→10 bps ladder — **~7% of the signal**, nowhere near "consumes the gross −0.10 IC edge." **The conclusion was FORMALLY FALSIFIED by this project, from these very surfaces, on the day they were locked — and the falsification was never propagated back.** `docs/dev-notes/archive/2026-Q2/basis-reversal-vehicle-vs-signal-fork-2026-06-06.md` (same date as the report timestamps) states *"Fees are demonstrably NOT the killer"*, *"the R-BR.LOAD fee-sweep … **falsified the fee-bleed hypothesis**"*, *"the killer is BETA, not fees"* — the reading carried into `REQ-PERP-BASIS-MN-SPREAD-001`'s trace row and into the whole rationale for building 1-21. No errata was ever issued against the bodies. Orchestrator measurement reproduces the adjudication to five decimals (best-cell p50 0.048645→0.045192 across 0→10bps in 2023, 0.050617→0.047626 in 2024 — **~0.003 Sharpe against a ~1.69 gap to passive**). **NOTE — unlike #90/#91, no new column is owed: this family already ships per-cell `trades`, so the turnover claim was checkable against its own body and was not checked.** At regeneration the sentence must be made fee-conditional and restated to the adjudication's finding — this closes a corpus contradiction standing since 2026-06-06, it does not require new analysis. Body-hygiene rider: the `{:.6}`/`{:.2}` format-placeholder prose defect (already listed for #86's family) is present in these bodies too.
  - Body-hygiene rider: **measure before enforcing a basis staleness bound.** `PitSeries::as_of_value` forward-fills with no max-age and the strategy ring never shrinks once warm, so a mid-year data gap freezes a stale basis as a fully-selectable constant with no warn (contrast the loud warn review 1-17 added on the TS arm). Enforcing a bound is anchor-impacting **iff the shipped corpus has gaps** — measure the gap distribution first, then choose enforce-vs-warn.
**Extension (1-17 review, 2026-08-04):** anchors #90/#91 (TS 2023+2024) join the inventory — identical chain, BUYHOLD clean, same riders; #92-#99 (1-18 horizon surfaces) expected at the 1-18 review. **The decorative-cap rider is UPGRADED to bug-log #69**: portfolio_exposure_cap inert engine-wide, D-TSM.2's safety premise false, TS surfaces ran ~90-100% gross vs the hashed 0.50 claim with alphabetical cash rationing — AC3 now includes enforce-or-delete + a BINDING test for every declared risk limit; AC4's re-derivation includes the corrected exposure description + an EXPLICIT re-affirmation of the active-trading-thesis closure (which stands direction-preserved-pending-re-lock per the 1-17 audit). Narrative corrections at regeneration also cover: the hysteresis/"band" mechanism (single threshold both ways — no band exists), the missing trades/turnover column vs the fee-bleed conclusion, the tim warmup-skew legend, and the cross-year same-seed correlation caveat. **Extension (1-16 review, 2026-08-03):** anchor #87 (MR θ-surface, SHA `a708112e…`) joins #86 — identical chain, BUYHOLD clean, same riders; plus the #68 drift-axis ratify-or-fix (implement the hold band or drop the axis + correct the narrative at regeneration; a drift-only cell pair becomes a mandatory per-axis divergence probe in the re-verification suite).
- Do-not-build register: not implicated (gate-credibility maintenance; no new alpha surface).
- Direction honesty: Blind's assessment — cleaning the noise plausibly shows MORE fragility, not less — is a hypothesis, not a result; AC4's escalation clause is the guard.

### References

- Trace: `REQ-HARNESS-FILL-CORRECTNESS-001` (state=`scoped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 1 (Strategy & Backtest Engine (v0-v5 ladder + robustness program))
