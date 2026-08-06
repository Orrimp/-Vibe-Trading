# Story 1.18: horizon-retest-robustness

Status: done

<!-- Retro-generated 2026-07-25 (BMAD migration Phase 2, plan: docs/dev-notes/bmad-migration-plan-2026-07-24.md).
     spec/ remains authoritative until the Phase 5b cutover; this story is the BMAD-native registry entry. -->

## Story

As the operator of the Honest Advisor,
I want the coarser 4h/daily decision-cadence retest - the last untested OHLCV axis (verdict: FRAGILE),
so that the strategy/backtest engine is deterministic, real-data-grounded, and honest about what does not work.

## Acceptance Criteria

1. **Given** the built-and-verified state frozen at frontmatter `presenter-done` (2026-06-17 spec compression), **when** the remaining pipeline leg (presenter/operator close-out) is replayed or formally waived, **then** the delivered behaviour stands as recorded: the coarser 4h/daily decision-cadence retest - the last untested OHLCV axis (verdict: FRAGILE).
2. The standing floor holds: `verify_anchors` 119/119; `python3 scripts/spec_lint.py` PASS.

## Tasks / Subtasks

### Review Findings

<!-- bmad-code-review 2026-08-04 (burn-down 8 of 14; commits 643b6e3/d8f327c/948d4f8; layers: Blind 17, Edge 12, Auditor 12 raw — 41 deduped to 17). The heaviest story of the burn-down.
     Gates THIS session: `ANCHORS PASS (119 / 119)` (×3, incl. after the errata file) · `spec-lint: PASS (0 violations)`; independent leg: resample 11/11, horizon e2e 7/7.
     BOTH CRITICALS were found independently by two layers and verified AT SOURCE by the orchestrator. The annualization-siblings probe was closed before the review (leap-aware ppy verified) and correctly not redone.
     OPERATOR 2026-08-04: qualify the record NOW (errata issued) + apply every patch. -->

- [x] [Review][Decision→1-25 + ERRATA] **CRITICAL C1 (bug-log #72) — the carry×coarse-horizon surfaces measured a THROTTLED mechanism.** `BlockBootstrapPathGen` re-stamps every generated bar as `epoch_base + Duration::hours(i)` with `tf: Timeframe::OneHour` hardcoded regardless of the source cadence [crates/data/src/synth/bootstrap.rs:611,623]; funding settlement is detected as `hours_since_epoch % 8 == 0` [crates/backtest/src/scenarios/montecarlo.rs:493-501] — so it counts BARS, not hours: every 32 real hours at 4h, every 8 real DAYS at daily. Empirical fingerprint (g=0 2023, per path): **15,490 → 3,039 → 267**. Anchors #96-#99 assert "even at the native settlement cadence…" — never simulated. **The carry × coarse-horizon leg of the thesis closure is UNRESOLVED, not direction-preserved.** Errata ISSUED (operator-ratified AC4 escalation): `evidence/v1/horizon-retest-robustness/reports/ERRATA-2026-08-04.md`. Fix + re-derivation → 1-25.
- [x] [Review][Decision→1-25] **CRITICAL C2 (bug-log #71) — the per-symbol exposure cap is side-blind and silently rejects position-CLOSING sells.** `Order::new` tests `notional/equity > cap` with no `side` term and ignores the position snapshot [crates/core/src/order.rs:160-170]; the caller drops the `Err` with no else arm, no warn, no counter [montecarlo.rs:387-413]. When it fires, `held_symbols` believes the position closed while the engine's book keeps it open — every later decision runs off a false flat. Blast radius exceeds #67's: it changes WHICH ORDERS EXIST. **Process finding of record:** the 1-18 dev observed this exact behaviour, wrote it into a test comment verbatim ("leaving the physical position open forever"), and shipped a gentler fixture (`build_1h_up_down_bars_moderate`) so the cap would not trip — rather than filing it.
- [x] [Review][Decision→1-25] **H4 — daily carry samples only the 00:00-UTC settlement.** Daily buckets open at midnight, so the 08:00 and 16:00 rates never enter `funding_at_return`; at 4h each rate is forward-filled into two buckets, halving the documented "L settlements" memory. Grid docs say settlements; the code counts bars [funding_data.rs:384-436; resample.rs:294]. Anchor-impacting (#96-#99).
- [x] [Review][Patch] **H1 — `--horizon` is the 4th unguarded identity-forge axis.** The 1-17 full-tuple guard covers (grid, direction, selection_mode, score_source) but not horizon, so ≥4 combinations emit an existing anchor's scenario name INTO that anchor's own resolution directory (`--grid ts-4h` at default 1h → #90/#91; `--grid ts-tier1 --horizon 4h` → #92/#93; `--grid carry-4h --horizon daily` → #98/#99; `--grid carry-daily` at 1h → carry 1h) → anchors gate falsely RED from one misinvocation [sweep_harness.rs:434-459; bin:1411-1418]. Add `required_horizon()` as the 5th leg + extend the accept-set test.
- [x] [Review][Patch] **H2 (bug-log #70) — the R3 coverage gate compares COARSE expected against RAW loaded.** `expected_total = bar_count(coarse) * symbols` vs `loaded_count` of raw 1h bars → at daily the gate demanded 3,632 of 87,600: a corpus missing 95.9% of its hours would pass silently. The in-code comment and D-HR.2 both assert the opposite [bin:431,465-467; realdata.rs:236-241]. Orchestrator-verified. One-line unit fix + a horizon-invariance test.
- [x] [Review][Patch] **H3 — the burn-down's OWN 1-15 fix is inert in production.** The bin builds scenario names with an inline `format!` chain that never calls the re-exported `build_scenario_name` and contains ZERO `scenario_discriminator()` calls [bin:1813-1870] — so the 1-15 M2 grid-discriminator fix AND the L3 gbm-token fix never bite, while their test (asserting the library fn) passes green. Orchestrator-verified; this is the #66 vacuous class shipped BY the burn-down. Delete the inline copy, call the seam (byte-identical for every anchored lane — discriminator is `""` for all production grids), and prove it with literal expected-string assertions per anchored family.
- [x] [Review][Patch] M1 — `f_hr_4_baseline_divergence_4h` compares a 10%-sized TS run against a 100%-deployed buy-and-hold, so the ≥1bp AD-16 gate is met by the sizing artefact alone; a no-op TS rule passes. Re-point at a LIKE-SIZED always-long control.
- [x] [Review][Patch] M2 — `f_hr_4_no_look_ahead_coarse` substitutes input prices and asserts outputs differ (trivially true for any deterministic fold); its stated revert is unreachable. Rebuild as prefix-invariance: `resample(&bars[..k])` == the prefix of `resample(&bars)` for every complete bucket.
- [x] [Review][Patch] M3 — the `f_hr_2_*` annualization gates compare `compute_*_periodic` against a line-by-line copy of the same formula (can only fail if a function differs from itself); `f_hr_2_leap_year_scalars` never calls `Horizon::periods_per_year` — it passes 2196.0/366.0 as literals and asserts monotonicity, true of any increasing function. Keep the load-bearing ratio cross-checks; call the real fn; assert literal values.
- [x] [Review][Patch] M4 — `f_hr_5_two_run_byte_identity_{4h,daily}` asserts `f(x)==f(x)` in one process with no seed variation, no rayon, no renderer, and none of the four LOCKED grids — while its doc claims it "catches any unordered fold in the resampler, the grid, or the renderer". Drive the real chain or state honestly what it covers.
- [x] [Review][Patch] M5 — the four LOCKED grids hashed into #92-#99 have ZERO content-assertion test (they appear only in pairing-guard tests); and every falsifier runs at 0/0 bps while every anchored surface runs 4/2 bps, so the fee-bleed regime the verdicts hinge on is never exercised e2e. Add cell-tuple assertions + at least one anchored-fee falsifier variant.
- [x] [Review][Patch] M6 — `resample.rs` panics in library code (three `panic!`s in `emit()`, CLAUDE.md-forbidden), contradicts its own `# Panics` doc by silently falling back to `UNIX_EPOCH` on a bad timestamp, never re-checks symbol/venue across a bucket (interleaved symbols → silent cross-symbol corruption), has no sorted-input guard, and saturates `trade_count` to `u32::MAX` unlogged. Result-ify + guards.
- [x] [Review][Patch] M7 — partial buckets are byte-indistinguishable from complete ones (a daily bucket built from 1 of 24 hours renders identically, with under-summed volume and under-ranged high/low) [resample.rs:281-320]. Carry a bar count / warn loudly — do NOT drop partials (that would move the coarse source and the anchors).
- [x] [Review][Patch] M8 — leap-table inconsistency: `bars_per_year_1h`'s catch-all `_ => 8760` is not leap-aware while `Horizon::periods_per_year(year)` is, so `--year 2028 --horizon 4h` computes 2190 bars against ppy 2196 [bin:1451-1465 vs resample.rs:122-147].
- [x] [Review][Patch] L1 — doc/dead-code truth pass: `sweep_periods_per_year` is a pass-through whose doc states an unenforced rule; `sweep_harness.rs:265-266` cites `m2_*` tests that do not exist (grep: zero hits); `f_hr_1_*`'s doc claims a literal byte-value assertion it does not make; carry grid headers advertise a 3×3 cross-product but ship a 6-cell star (and g=3 moves two axes at once). Also add a comment at the hashed `| horizon |` row warning that "fixing" its ragged padding would break four anchors.
- [x] [Review][Patch] L2 — trace row `REQ-HORIZON-RETEST-ROBUSTNESS-001` still carries the superseded 6:1 / 1460-bucket arithmetic; the ship is 4:1 / 2190-2196 (the dev's own M-DEV-2 changelog corrected it). Story Dev Notes also point at the retired `spec/v1/...` path.
- [x] [Review][Defer] The three horizons reuse `ensemble_seed = fill_seed = 0xC0FFEE` over the same 2023/2024 source, so 1h/4h/daily are seed- and data-coupled re-cuts of ONE experiment — "three horizons agree" is not three independent tests. Not a defect; a stated-limit the re-lock's narrative should carry. Owner: 1-25 (narrative). Revisit: at 1-25 close.
- [x] [Review][Defer] Warmup is counted in BARS, so the forced-flat prefix is 8.2% at 1h but 24.7% at 4h (lookback 540 / 2190 bars) — σ is taken over all returns including the warmup zeros, inflating |Sharpe| exactly at the long-lookback 4h cells, and `time_in_market` is not comparable across horizons. An in-metric fix moves every #92-#99 Sharpe; a body disclosure is itself a hashed-body change. Owner: 1-25. Revisit: at 1-25 close.

Probes explicitly CLEAR: #68 axis-execution on all four new grids (every swept axis executed — incl. `rebalance_minutes_override: 120`, which fires every second bar exactly as documented); seed-collision (no new derivation); n_eff/block-length collapse (L = 204/80/9 → 43/27/41 independent blocks, comparable); volume-sum overflow; bucket-key arithmetic; empty/single-bar input; skip-visibility (no new gated tests). The bakeoff standing exemption holds — this diff does not touch `bakeoff/bootstrap.rs`.


- [ ] `horizon-retest-robustness` 0.2.0 - the base feature (presenter-done)

## Dev Notes

- Source feature folder: `spec/v1/horizon-retest-robustness/` - frontmatter status **`presenter-done`** (verbatim), version `0.2.0`, updated `2026-06-17`.
- Status mapping: `presenter-done` -> `review` (Phase-2 retro convention: shipped->done; retired/deprecated->retired; presenter/tester/dev-done->review; arch-done->ready-for-dev; candidate/draft/reserved->backlog; honest, never promoted).
- CHANGELOG index: CHANGELOG § Robustness program — CONCLUDED 2026-06-08 → ship passive.
- Provenance: `git log -- spec/v1/horizon-retest-robustness` (full narrative); reports under `evidence/v1/horizon-retest-robustness/reports/` where present (anchored report bodies are byte-immutable - ADR-0038 §D6).

### References

- Trace: `REQ-HORIZON-RETEST-ROBUSTNESS-001` (state=`shipped`)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 1 (Strategy & Backtest Engine (v0-v5 ladder + robustness program))

## Dev Agent Record

### Agent Model Used

Historical stub - implementation predates BMAD story tracking; see git history via the provenance pointer above.

### Debug Log References

### Completion Notes List

### File List

#### Review close-out (2026-08-04, orchestrator)

All 13 patches APPLIED by the dev subagent — the most rigorous agent run of the
burn-down: it MUTATION-TESTED its own rebuilds rather than asserting them.
Probe 1: disabling the TS exit (`SelectionMode::CrossSectionalTopK`) turns the
three rebuilt divergence gates RED, while a replica of the OLD BH-based gate
still PASSES with delta 32,308 against a threshold of 10 — the old form was met
**3,200× over by the sizing artefact alone, with the strategy entirely dead**.
Probe 2: a fold mutated to borrow the next bucket's close leaves the old
"causality" test green and turns the new prefix-invariance test RED. That is the
vacuity class measured, not argued.

Orchestrator additions + independent verification: fixed MY OWN clippy debt from
the 1-15 pass (`assert_eq!(.., true)` at param_sweep_e2e.rs — `--all-targets`
now clean); re-ran everything myself — horizon e2e **10/10**, resample **15/15**,
sweep bin **32/32**, param_sweep **13/13**, `ANCHORS PASS (119 / 119)`,
`spec-lint: PASS (0 violations)`. The H3 seam switch is proven byte-identical on
all 34 anchored scenario strings by tests asserting literals pasted from
`evidence/anchors.toml` — never re-derived from the code.

Disclosures of record: bug-log **#70** (FIXED — coverage gate compared coarse
expected against raw loaded; a corpus missing 95.9% of its hours would have
passed), **#71** (OPEN → 1-25 — exposure cap side-blind, blocks de-risking; the
dev softened a fixture around it), **#72** (OPEN → 1-25 — cosmetic 1h ladder made
funding accrual horizon-blind). Anchors #92-#99 → 1-25 inventory. **The carry ×
coarse-horizon leg of the thesis closure is UNRESOLVED pending re-lock** —
errata issued the same day at
`evidence/v1/horizon-retest-robustness/reports/ERRATA-2026-08-04.md` (the AC4
escalation, operator-ratified). The TS legs stand direction-preserved.
